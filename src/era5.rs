//! Clima passado: a reanálise ERA5 do ECMWF (Copernicus Climate Change Service), pela API
//! histórica do Open-Meteo (`archive-api.open-meteo.com`, sem login, com CORS).
//!
//! - **Por que o Open-Meteo e não o cubo ERA5 do RustGIS:** o cubo do catálogo do RustGIS (o
//!   ARCO-ERA5 do Google, em Zarr) guarda uma hora do globo inteiro por pedaço: ler uma área exige
//!   baixar o globo a cada hora, cerca de 2 s por hora de dado (23 minutos para um mês). O
//!   Open-Meteo serve os mesmos dados do ERA5 já resumidos por dia, por ponto.
//! - **Os pontos** são os nós da grade de 0,25° do ERA5 que cercam a área, como na previsão
//!   ([`crate::tempo::pontos`]). A API gratuita conta o custo de um pedido em pontos × dias / 14
//!   × variáveis / 10 e aceita até 600 por minuto: o plugin gasta até [`ORCAMENTO`] num clique, e
//!   num período longo a grade de pontos fica mais aberta (30 anos cabem em 6 pontos).
//! - **A temperatura** vem ajustada à altitude de cada ponto, num modelo de elevação de 90 m.
//! - **Os dias** são os do horário de Brasília. O ERA5 sai com uns 5 dias de atraso: um fim mais
//!   recente que o último dia com dados dá um erro com esse dia.

use crate::datas::Data;
use crate::tempo::Temperatura;
use serde_json::Value;

/// O dado do ERA5.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Variavel {
    Chuva,
    Temperatura(Temperatura),
    Umidade,
}

/// Como resumir a chuva do período.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Resumo {
    /// A chuva somada no período (mm).
    Total,
    /// A média por ano (mm por ano).
    PorAno,
}

/// As escolhas da janela do resumo da chuva e o `resumo` de cada uma.
pub const ESCOLHAS_RESUMOS: [(&str, &str); 2] = [("Total do período (mm)", "total"), ("Média por ano (mm por ano)", "anual")];

/// O resumo da chuva (sem ele, o total).
pub fn resumo(texto: Option<&str>) -> Result<Resumo, String> {
    match texto.map(|t| t.trim().to_lowercase()).as_deref() {
        None | Some("total") => Ok(Resumo::Total),
        Some("anual" | "por-ano" | "ano") => Ok(Resumo::PorAno),
        Some(t) => Err(format!("o resumo é total ou anual; não '{t}'")),
    }
}

impl Variavel {
    /// O comando do dado.
    pub fn comando(self) -> &'static str {
        match self {
            Variavel::Chuva => "era5-chuva",
            Variavel::Temperatura(_) => "era5-temperatura",
            Variavel::Umidade => "era5-umidade",
        }
    }

    /// A variável diária do Open-Meteo.
    pub fn api(self) -> &'static str {
        match self {
            Variavel::Chuva => "precipitation_sum",
            Variavel::Temperatura(t) => t.variavel(),
            Variavel::Umidade => "soil_moisture_0_to_7cm_mean",
        }
    }

    /// A unidade do mapa.
    pub fn unidade(self, r: Resumo) -> &'static str {
        match (self, r) {
            (Variavel::Chuva, Resumo::Total) => "mm",
            (Variavel::Chuva, Resumo::PorAno) => "mm por ano",
            (Variavel::Temperatura(_), _) => "°C",
            (Variavel::Umidade, _) => "%",
        }
    }
}

/// O primeiro dia do ERA5.
pub const INICIO: Data = Data { ano: 1940, mes: 1, dia: 1 };

/// O período mais longo: 30 anos (o de uma normal climatológica).
pub const MAX_ANOS: i64 = 30;

/// O passo da grade do ERA5, em graus.
pub const PASSO: f64 = 0.25;

/// O custo de um clique na conta do Open-Meteo (que aceita 600 por minuto).
pub const ORCAMENTO: f64 = 500.0;

/// O passo mais aberto da grade de pontos, em graus (cerca de 110 km): acima dele, os nós saem da
/// área e a interpolação não diz nada; o comando recusa e diz o período que cabe.
pub const PASSO_MAX: f64 = 1.0;

/// O período mais longo, em dias, que cabe no orçamento com `pontos` pontos.
pub fn dias_maximos(pontos: usize) -> usize {
    (ORCAMENTO * 140.0 / pontos.max(1) as f64).floor() as usize
}

/// O erro de um período longo demais para a área: com `pontos` pontos a [`PASSO_MAX`], cabem só
/// [`dias_maximos`] dias.
pub fn longo_demais(dias: usize, pontos: usize) -> String {
    let max = dias_maximos(pontos);
    let cabe = if max >= 730 { format!("{} anos", max * 100 / 36525) } else { format!("{max} dias") };
    format!(
        "{dias} dias nesta área passam do limite gratuito do Open-Meteo (que pesa os pontos pelos dias): a grade de pontos ficaria mais aberta que 1°. Use até {cabe}, ou uma área menor"
    )
}

/// Quantos dias tem o período, os dois incluídos.
pub fn dias(inicio: Data, fim: Data) -> usize {
    (fim.dias() - inicio.dias() + 1).max(1) as usize
}

/// O máximo de pontos de um período de `dias` dias: o custo (pontos × dias / 14 × 1/10, uma
/// variável) até [`ORCAMENTO`], e até os 400 da previsão.
pub fn max_pontos(dias: usize) -> usize {
    ((ORCAMENTO * 140.0 / dias as f64).floor() as usize).clamp(1, crate::tempo::MAX_PONTOS)
}

/// Os pontos por endereço: até 30.000 pontos × dias, uns 3 s do Open-Meteo e 550 kB de resposta
/// (o RustGIS espera até 20 s por pedido e lê até 2 MB).
fn por_pedido(dias: usize) -> usize {
    (30_000 / dias.max(1)).clamp(1, 100)
}

/// Um número curto para o endereço: até 4 casas, sem zeros à direita.
fn curto(v: f64) -> String {
    let t = format!("{v:.4}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" { "0".into() } else { t.into() }
}

/// Os endereços dos valores diários de `v` nos `pts`, de `inicio` a `fim`, em grupos.
pub fn pedidos(v: Variavel, inicio: Data, fim: Data, pts: &[[f64; 2]]) -> Vec<String> {
    pts.chunks(por_pedido(dias(inicio, fim)))
        .map(|g| {
            let lat: Vec<String> = g.iter().map(|p| curto(p[1])).collect();
            let lon: Vec<String> = g.iter().map(|p| curto(p[0])).collect();
            format!(
                "https://archive-api.open-meteo.com/v1/archive?latitude={}&longitude={}&start_date={}&end_date={}&daily={}&models=era5&timezone=America%2FSao_Paulo",
                lat.join(","),
                lon.join(","),
                inicio.iso(),
                fim.iso(),
                v.api()
            )
        })
        .collect()
}

/// Um pedido mínimo à previsão, só para saber o dia de hoje em Brasília (o plugin não tem
/// relógio).
pub const HOJE: &str =
    "https://api.open-meteo.com/v1/forecast?latitude=-15.75&longitude=-47.75&daily=precipitation_sum&forecast_days=1&timezone=America%2FSao_Paulo";

/// O dia de hoje na resposta de [`HOJE`].
pub fn hoje(resposta: &Value) -> Option<Data> {
    crate::bdc::ler_data(resposta["daily"]["time"].as_array()?.first()?.as_str()?)
}

/// O período padrão: o último ano completo.
pub fn periodo_padrao(hoje: Data) -> (Data, Data) {
    (Data { ano: hoje.ano - 1, mes: 1, dia: 1 }, Data { ano: hoje.ano - 1, mes: 12, dia: 31 })
}

/// Confere o período pedido.
pub fn conferir_periodo(inicio: Data, fim: Data) -> Result<(), String> {
    if fim < inicio {
        return Err(format!("o fim ({}) vem antes do início ({})", fim.texto(), inicio.texto()));
    }
    if inicio < INICIO {
        return Err(format!("o ERA5 começa em {}", INICIO.texto()));
    }
    let limite = Data { ano: inicio.ano + MAX_ANOS, ..inicio };
    if fim >= limite {
        return Err(format!("o período passa de {MAX_ANOS} anos: escolha um mais curto"));
    }
    Ok(())
}

/// As séries diárias de `v` de cada lugar de uma resposta do Open-Meteo, lidas direto do texto
/// (`host.fetch` com `"json": false`): montar o JSON de 30 anos de dias passa do orçamento de
/// processamento de um clique do plugin. Cada lugar tem um `"<variável>":[…]` em `daily` (em
/// `daily_units` a variável vem com um texto, não uma lista), na ordem dos pontos; `null` é um dia
/// sem dado.
pub fn series(texto: &str, v: Variavel) -> Vec<Vec<Option<f64>>> {
    let chave = format!("\"{}\":[", v.api());
    let mut saida = Vec::new();
    let mut resto = texto;
    while let Some(i) = resto.find(&chave) {
        resto = &resto[i + chave.len()..];
        let fim = resto.find(']').unwrap_or(resto.len());
        let lista = &resto[..fim];
        saida.push(if lista.trim().is_empty() { Vec::new() } else { lista.split(',').map(|x| x.trim().parse::<f64>().ok()).collect() });
        resto = &resto[fim..];
    }
    saida
}

/// O erro de uma resposta do Open-Meteo (`{"error":true,"reason":…}`), lida como texto.
pub fn erro_do_texto(texto: &str) -> Option<String> {
    let t = texto.trim_start();
    if !t.starts_with('{') || !t.contains("\"error\"") {
        return None;
    }
    let j: Value = serde_json::from_str(t).ok()?;
    crate::tempo::erro(&j)
}

/// O valor de um lugar no período de `dias` dias: a chuva somada (ou por ano), a média da
/// temperatura, a média da umidade em %; `None` se falta algum dia.
pub fn valor(serie: &[Option<f64>], v: Variavel, r: Resumo, dias: usize) -> Option<f64> {
    if serie.len() < dias {
        return None;
    }
    let mut soma = 0.0;
    for x in &serie[..dias] {
        soma += (*x)?;
    }
    Some(match (v, r) {
        (Variavel::Chuva, Resumo::Total) => soma,
        (Variavel::Chuva, Resumo::PorAno) => soma / (dias as f64 / 365.25),
        (Variavel::Temperatura(_), _) => soma / dias as f64,
        (Variavel::Umidade, _) => soma / dias as f64 * 100.0,
    })
}

/// Quantos dias seguidos, desde o início, têm valor no lugar que tem mais.
pub fn dias_com_dados(series: &[Vec<Option<f64>>], dias: usize) -> usize {
    series.iter().map(|s| s.iter().take(dias).take_while(|x| x.is_some()).count()).max().unwrap_or(0)
}

/// O dia `n` dias depois de `d`.
pub fn mais_dias(d: Data, n: usize) -> Data {
    (0..n).fold(d, |d, _| d.seguinte())
}

/// O começo dos nomes das camadas.
pub const PREFIXO: &str = "BrasilData: ";
/// O fim dos nomes das camadas.
pub const FIM: &str = " (ERA5)";

/// O resumo no nome da camada.
fn nome_do_resumo(v: Variavel, r: Resumo) -> String {
    match (v, r) {
        (Variavel::Chuva, Resumo::Total) => "Chuva total".into(),
        (Variavel::Chuva, Resumo::PorAno) => "Chuva média por ano".into(),
        (Variavel::Temperatura(t), _) => match t {
            Temperatura::Media => "Temperatura média do ar".into(),
            t => format!("Temperatura {} do ar, média", t.nome()),
        },
        (Variavel::Umidade, _) => "Umidade do solo média".into(),
    }
}

/// O nome da camada: "BrasilData: Chuva total de 01/01/2025 a 31/12/2025 (ERA5)".
pub fn camada(v: Variavel, r: Resumo, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}{} de {} a {}{FIM}", nome_do_resumo(v, r), inicio.texto(), fim.texto())
}

/// Se a camada é do ERA5, pelo nome.
pub fn eh_do_era5(nome: &str) -> bool {
    nome.starts_with(PREFIXO) && nome.ends_with(FIM)
}

/// O comando de uma camada do ERA5, pelo nome.
pub fn comando_da_camada(nome: &str) -> Option<&'static str> {
    if !eh_do_era5(nome) {
        return None;
    }
    let n = &nome[PREFIXO.len()..];
    Some(if n.starts_with("Chuva") {
        "era5-chuva"
    } else if n.starts_with("Temperatura") {
        "era5-temperatura"
    } else {
        "era5-umidade"
    })
}

/// A unidade de uma camada do ERA5, pelo nome.
pub fn unidade_da_camada(nome: &str) -> &'static str {
    match comando_da_camada(nome) {
        Some("era5-chuva") if nome.contains("por ano") => "mm por ano",
        Some("era5-chuva") => "mm",
        Some("era5-temperatura") => "°C",
        _ => "%",
    }
}

/// Valores redondos para o máximo da escala da chuva.
const ESCALAS_CHUVA: [f64; 21] = [
    75.0, 150.0, 200.0, 300.0, 400.0, 600.0, 800.0, 1000.0, 1500.0, 2000.0, 2500.0, 3000.0, 4000.0, 5000.0, 7500.0, 10_000.0, 15_000.0,
    20_000.0, 30_000.0, 50_000.0, 75_000.0,
];

/// A escala das cores, a mesma em todas as áreas: a chuva pela duração do período (2.500 mm num
/// ano); a temperatura de 0 a 40 °C; a umidade de 0 a 50 %.
pub fn escala(v: Variavel, r: Resumo, dias: usize) -> (f64, f64) {
    match (v, r) {
        (Variavel::Chuva, Resumo::PorAno) => (0.0, 2500.0),
        (Variavel::Chuva, Resumo::Total) if dias <= 16 => (0.0, crate::tempo::escala_chuva(dias)),
        (Variavel::Chuva, Resumo::Total) => {
            let alvo = 2500.0 * dias as f64 / 365.25;
            (0.0, ESCALAS_CHUVA.iter().copied().find(|e| *e >= alvo).unwrap_or(ESCALAS_CHUVA[ESCALAS_CHUVA.len() - 1]))
        }
        (Variavel::Temperatura(_), _) => crate::tempo::ESCALA_TEMPERATURA,
        (Variavel::Umidade, _) => (0.0, 50.0),
    }
}

/// A rampa da umidade do solo, de 0 a 50 %, do marrom (seco) ao verde-azulado (úmido).
pub const RAMPA_UMIDADE: (&str, &str) =
    ("BrasilData: umidade do solo 0–50 %", "0 140 81 10\n12.5 216 179 101\n25 246 232 195\n37.5 90 180 172\n50 1 102 94");

/// A citação do ERA5 pelo Open-Meteo.
pub const CITACAO: &str = "Hersbach, H. et al. (2023): ERA5 hourly data on single levels from 1940 to present. Copernicus Climate Change Service (C3S) Climate Data Store (CDS), doi:10.24381/cds.adbb2d47, pela API histórica do Open-Meteo (Zippenfenig, P. 2023. Open-Meteo.com Weather API, doi:10.5281/zenodo.7970649): valores diários nos nós de uma grade de 0,25° em volta da área, interpolados para a grade do projeto e recortados pela área. Contém informação modificada do Copernicus Climate Change Service";

/// A licença.
pub const LICENCA: &str = "licença do Copernicus (C3S): uso livre, com a citação; Open-Meteo em CC BY 4.0, e a API gratuita é para uso não comercial";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn d(t: &str) -> Data {
        Data::ler(t).unwrap()
    }

    #[test]
    fn os_pontos_cabem_no_orcamento() {
        assert_eq!(max_pontos(365), 191);
        assert_eq!(max_pontos(30), 400);
        assert_eq!(max_pontos(10_958), 6);
        assert_eq!(max_pontos(1_000_000), 1);
        // O custo do Open-Meteo: pontos × dias / 14 / 10.
        for dias in [31, 365, 3653, 10_958] {
            assert!(max_pontos(dias) as f64 * dias as f64 / 140.0 <= ORCAMENTO, "{dias}");
        }
    }

    #[test]
    fn o_periodo_que_cabe() {
        // Minas Gerais a 1°: uns 140 pontos, 500 dias; Viçosa a 1°: 4 pontos, 30 anos e mais.
        assert_eq!(dias_maximos(140), 500);
        assert!(longo_demais(10_958, 140).contains("Use até 500 dias"));
        assert!(longo_demais(10_958, 20).contains("Use até 9 anos"));
    }

    #[test]
    fn os_enderecos() {
        let pts = [[-42.75, -20.75], [-42.5, -20.75]];
        let u = pedidos(Variavel::Chuva, d("01/01/2025"), d("31/12/2025"), &pts);
        assert_eq!(u.len(), 1);
        assert_eq!(
            u[0],
            "https://archive-api.open-meteo.com/v1/archive?latitude=-20.75,-20.75&longitude=-42.75,-42.5&start_date=2025-01-01&end_date=2025-12-31&daily=precipitation_sum&models=era5&timezone=America%2FSao_Paulo"
        );
        // 30 anos: 2 pontos por endereço (o Open-Meteo leva uns 3 s por ponto).
        let seis = [[0.0, 0.0]; 6];
        assert_eq!(pedidos(Variavel::Umidade, d("01/01/1991"), d("31/12/2020"), &seis).len(), 3);
        assert!(pedidos(Variavel::Temperatura(Temperatura::Minima), d("01/01/2025"), d("02/01/2025"), &pts)[0].contains("daily=temperature_2m_min&"));
    }

    #[test]
    fn o_periodo() {
        assert_eq!(periodo_padrao(d("19/11/2026")), (d("01/01/2025"), d("31/12/2025")));
        assert!(conferir_periodo(d("01/01/1991"), d("31/12/2020")).is_ok());
        assert!(conferir_periodo(d("01/01/1991"), d("01/01/2021")).unwrap_err().contains("30 anos"));
        assert!(conferir_periodo(d("31/12/1939"), d("01/01/1940")).unwrap_err().contains("1940"));
        assert!(conferir_periodo(d("02/01/2025"), d("01/01/2025")).unwrap_err().contains("antes"));
        assert_eq!(dias(d("01/01/2024"), d("31/12/2024")), 366);
        assert_eq!(mais_dias(d("30/12/2025"), 3), d("02/01/2026"));
        assert_eq!(hoje(&json!({"daily": {"time": ["2026-11-19"]}})), Some(d("19/11/2026")));
    }

    #[test]
    fn as_series_do_texto() {
        // Dois lugares, como o Open-Meteo responde (as unidades antes, com a variável num texto).
        let t = r#"[{"latitude":-20.75,"daily_units":{"time":"iso8601","precipitation_sum":"mm"},"daily":{"time":["2025-01-01","2025-01-02","2025-01-03","2025-01-04"],"precipitation_sum":[1.00,2.0,3,null]}},{"daily_units":{"precipitation_sum":"mm"},"daily":{"time":["2025-01-01"],"precipitation_sum":[0.5, 0.25,0.0,0.0]}}]"#;
        let s = series(t, Variavel::Chuva);
        assert_eq!(s, vec![vec![Some(1.0), Some(2.0), Some(3.0), None], vec![Some(0.5), Some(0.25), Some(0.0), Some(0.0)]]);
        assert_eq!(valor(&s[0], Variavel::Chuva, Resumo::Total, 3), Some(6.0));
        assert_eq!(valor(&s[0], Variavel::Chuva, Resumo::Total, 4), None);
        assert_eq!(valor(&s[0], Variavel::Chuva, Resumo::Total, 5), None);
        assert_eq!(dias_com_dados(&s, 4), 4);
        assert_eq!(dias_com_dados(&s[..1], 4), 3);
        // Um lugar só volta como objeto.
        let u = series(r#"{"daily":{"soil_moisture_0_to_7cm_mean":[0.3,0.4]}}"#, Variavel::Umidade);
        assert!((valor(&u[0], Variavel::Umidade, Resumo::Total, 2).unwrap() - 35.0).abs() < 1e-9);
        let ano = vec![Some(2.0); 365];
        assert!((valor(&ano, Variavel::Chuva, Resumo::PorAno, 365).unwrap() - 730.5).abs() < 1e-9);
        assert!(erro_do_texto(r#"{"error":true,"reason":"Parameter 'start_date' is out of allowed range"}"#).unwrap().contains("start_date"));
        assert_eq!(erro_do_texto(t), None);
    }

    #[test]
    fn os_nomes() {
        let n = camada(Variavel::Chuva, Resumo::Total, d("01/01/2025"), d("31/12/2025"));
        assert_eq!(n, "BrasilData: Chuva total de 01/01/2025 a 31/12/2025 (ERA5)");
        assert_eq!(comando_da_camada(&n), Some("era5-chuva"));
        assert_eq!(unidade_da_camada(&n), "mm");
        let t = camada(Variavel::Temperatura(Temperatura::Maxima), Resumo::Total, d("01/01/2025"), d("31/12/2025"));
        assert_eq!(t, "BrasilData: Temperatura máxima do ar, média de 01/01/2025 a 31/12/2025 (ERA5)");
        assert_eq!(comando_da_camada(&t), Some("era5-temperatura"));
        let u = camada(Variavel::Umidade, Resumo::Total, d("01/01/2025"), d("31/12/2025"));
        assert_eq!(comando_da_camada(&u), Some("era5-umidade"));
        assert_eq!(unidade_da_camada(&u), "%");
        assert_eq!(comando_da_camada("BrasilData: Umidade do solo média de 01/08/2026 a 31/10/2026 (AMSR2, JAXA Earth)"), None);
        assert!(!crate::tempo::eh_da_previsao(&t));
        assert!(!crate::jaxa::eh_do_jaxa(&u));
    }

    #[test]
    fn as_escalas() {
        assert_eq!(escala(Variavel::Chuva, Resumo::Total, 365), (0.0, 2500.0));
        assert_eq!(escala(Variavel::Chuva, Resumo::Total, 31), (0.0, 300.0));
        assert_eq!(escala(Variavel::Chuva, Resumo::Total, 7), (0.0, 150.0));
        assert_eq!(escala(Variavel::Chuva, Resumo::Total, 10_958), (0.0, 75_000.0));
        assert_eq!(escala(Variavel::Chuva, Resumo::PorAno, 10_958), (0.0, 2500.0));
        assert_eq!(resumo(Some("anual")), Ok(Resumo::PorAno));
        assert!(resumo(Some("x")).is_err());
    }
}
