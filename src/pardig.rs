//! Chuva agora: a taxa de chuva do PARDIG (Google DeepMind e Google Research), a cada 15 minutos.
//!
//! - **O que é:** a chuva instantânea estimada por IA, treinada nas medidas do radar do satélite
//!   GPM (NASA e JAXA) e alimentada pelo mosaico de satélites geoestacionários SatWeave e pela
//!   análise do ECMWF. Global, a 0,1° (cerca de 10 km), de 15 em 15 minutos; o teto é de 75 mm/h.
//! - **De onde vem:** dos buckets públicos do Google Cloud Storage (`pardig_1`), em Zarr 3 com
//!   zstd, sem login. Desde 2026, cada quadro é um store (`NRT/zarr/2026_to_present/<hora>/`), de
//!   um pedaço só com o globo (cerca de 1,2 MB); de 2016 a 2025, o arquivo NRT (`NRT/zarr/2016_to_2025`);
//!   de 1999 a 2015, a reanálise LR (`LR/zarr/1999_to_2023`), quatro quadros por pedaço.
//! - **Como:** o RustGIS lê a janela da área no store (`layer.addZarr` com `bbox`, na longitude de
//!   0 a 360° do store), e o plugin passa de m/h para mm/h, põe na grade do projeto e recorta
//!   pela área, como o CHIRPS. O bucket não manda CORS: só no app desktop.
//! - **O quadro mais recente:** o plugin não tem relógio. A listagem do bucket (a API JSON do
//!   GCS, com `startOffset`) diz se há quadros depois de um nome; o ano, o mês e o dia mais
//!   recentes saem por bisseção (cerca de dez pedidos de poucos bytes) e a lista do último dia dá
//!   a hora.

use crate::datas::Data;

/// O endereço público do bucket.
pub const BUCKET: &str = "https://storage.googleapis.com/pardig_1/";
/// A pasta dos quadros de 2026 em diante, um store por quadro.
pub const PASTA_AO_VIVO: &str = "NRT/zarr/2026_to_present/";
/// O arquivo NRT, de 2016-01-01 07:45 a 2025-12-31 23:45 UTC.
pub const ARQUIVO_NRT: &str = "NRT/zarr/2016_to_2025/predictions.zarr";
/// A reanálise LR, de 1999-01-01 00:00 a 2023-12-31 23:45 UTC.
pub const ARQUIVO_LR: &str = "LR/zarr/1999_to_2023/predictions.zarr";
/// A variável, em m/h.
pub const VARIAVEL: &str = "precipitation";

/// Um instante em UTC, de 15 em 15 minutos.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Quadro {
    pub data: Data,
    pub hora: u32,
    pub minuto: u32,
}

/// Os dias de 1970-01-01 até a data (o `days_from_civil` de Howard Hinnant).
fn dias_desde_1970(d: Data) -> i64 {
    let a = d.ano - i64::from(d.mes <= 2);
    let era = a.div_euclid(400);
    let yoe = a - era * 400;
    let m = i64::from(d.mes);
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(d.dia) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

impl Quadro {
    /// Os minutos desde 1970-01-01 00:00 UTC.
    pub fn minutos(self) -> i64 {
        dias_desde_1970(self.data) * 1440 + i64::from(self.hora) * 60 + i64::from(self.minuto)
    }

    /// O quadro de `m` minutos desde 1970 (arredondado para baixo aos 15 minutos).
    pub fn de_minutos(m: i64) -> Quadro {
        let m = m - m.rem_euclid(15);
        let data = Data::de_milissegundos(m.div_euclid(1440) * 86_400_000);
        let r = m.rem_euclid(1440);
        Quadro { data, hora: (r / 60) as u32, minuto: (r % 60) as u32 }
    }

    /// O nome do store do quadro ("20261116T103000").
    pub fn nome(self) -> String {
        format!("{:04}{:02}{:02}T{:02}{:02}00", self.data.ano, self.data.mes, self.data.dia, self.hora, self.minuto)
    }

    /// O quadro do nome de um store ("20261116T103000").
    pub fn ler_nome(nome: &str) -> Option<Quadro> {
        let n = nome.as_bytes();
        if n.len() < 15 || n[8] != b'T' || !nome[..8].bytes().chain(nome[9..15].bytes()).all(|b| b.is_ascii_digit()) {
            return None;
        }
        let p = |a: usize, b: usize| nome[a..b].parse::<u32>().ok();
        let data = Data { ano: i64::from(p(0, 4)?), mes: p(4, 6)?, dia: p(6, 8)? };
        let q = Quadro { data, hora: p(9, 11)?, minuto: p(11, 13)? };
        ((1..=12).contains(&data.mes) && (1..=31).contains(&data.dia) && q.hora < 24 && q.minuto.is_multiple_of(15)).then_some(q)
    }

    /// O quadro de uma data e hora de Brasília ("16/11/2026 07:30"; sem a hora, meia-noite),
    /// em UTC, arredondado para baixo aos 15 minutos.
    pub fn de_brasilia(texto: &str) -> Option<Quadro> {
        let mut partes = texto.split_whitespace();
        let data = Data::ler(partes.next()?)?;
        let (h, m) = match partes.next() {
            Some(t) => {
                let (h, m) = t.split_once(':').unwrap_or((t, "0"));
                (h.trim_end_matches('h').parse::<i64>().ok()?, if m.is_empty() { 0 } else { m.parse::<i64>().ok()? })
            }
            None => (0, 0),
        };
        if partes.next().is_some() || !(0..24).contains(&h) || !(0..60).contains(&m) {
            return None;
        }
        let base = Quadro { data, hora: 0, minuto: 0 }.minutos();
        Some(Quadro::de_minutos(base + (h + 3) * 60 + m))
    }

    /// A data e a hora de Brasília (UTC−3), "16/11/2026 07:30".
    pub fn brasilia(self) -> String {
        let q = Quadro::de_minutos(self.minutos() - 180);
        format!("{:02}/{:02}/{} {:02}:{:02}", q.data.dia, q.data.mes, q.data.ano, q.hora, q.minuto)
    }
}

/// Onde está um quadro: o endereço do store e, num arquivo de muitos quadros, o passo no tempo.
#[derive(Clone, Debug, PartialEq)]
pub struct Origem {
    pub url: String,
    pub passo: Option<i64>,
    /// "NRT" ou "LR", para a citação e o aviso.
    pub versao: &'static str,
}

/// O primeiro quadro de cada arquivo.
fn inicio_nrt() -> Quadro {
    Quadro { data: Data { ano: 2016, mes: 1, dia: 1 }, hora: 7, minuto: 45 }
}
fn inicio_lr() -> Quadro {
    Quadro { data: Data { ano: 1999, mes: 1, dia: 1 }, hora: 0, minuto: 0 }
}
fn inicio_ao_vivo() -> Quadro {
    Quadro { data: Data { ano: 2026, mes: 1, dia: 1 }, hora: 0, minuto: 0 }
}

/// Onde ler o quadro `q`: um store por quadro desde 2026, o arquivo NRT de 2016 a 2025 e a
/// reanálise LR antes. `None` antes de 1999.
pub fn origem(q: Quadro) -> Option<Origem> {
    if q >= inicio_ao_vivo() {
        return Some(Origem { url: format!("{BUCKET}{PASTA_AO_VIVO}{}/predictions.zarr", q.nome()), passo: None, versao: "NRT" });
    }
    let (inicio, arquivo, versao) = if q >= inicio_nrt() {
        (inicio_nrt(), ARQUIVO_NRT, "NRT")
    } else if q >= inicio_lr() {
        (inicio_lr(), ARQUIVO_LR, "LR")
    } else {
        return None;
    };
    Some(Origem { url: format!("{BUCKET}{arquivo}"), passo: Some((q.minutos() - inicio.minutos()) / 15), versao })
}

/// O pedido da listagem dos stores de 2026 em diante a partir de `inicio` (um prefixo do nome,
/// como "202611"), até `max` nomes. Vazio vira "0": a pasta tem um objeto vazio com o nome dela,
/// que viria primeiro e gastaria o `max`.
pub fn url_lista(inicio: &str, max: u32) -> String {
    let inicio = if inicio.is_empty() { "0" } else { inicio };
    format!(
        "https://storage.googleapis.com/storage/v1/b/pardig_1/o?delimiter=%2F&prefix={p}&startOffset={p}{inicio}&maxResults={max}&fields=prefixes",
        p = PASTA_AO_VIVO.replace('/', "%2F")
    )
}

/// Os quadros de uma resposta da listagem, em ordem.
pub fn quadros_da_lista(json: &serde_json::Value) -> Vec<Quadro> {
    let mut v: Vec<Quadro> = json["prefixes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|p| p.as_str()?.strip_prefix(PASTA_AO_VIVO))
        .filter_map(|n| Quadro::ler_nome(n.trim_end_matches('/')))
        .collect();
    v.sort();
    v
}

/// O maior `x` em `lo..=hi` para o qual `tem(x)` vale, sabendo que vale em `lo` e que, valendo
/// em `x`, vale em todos os menores.
fn maior<E>(mut lo: u32, mut hi: u32, mut tem: impl FnMut(u32) -> Result<bool, E>) -> Result<u32, E> {
    while lo < hi {
        let meio = (lo + hi).div_ceil(2);
        if tem(meio)? {
            lo = meio;
        } else {
            hi = meio - 1;
        }
    }
    Ok(lo)
}

/// Os quadros mais recentes (do último dia com quadros), pela listagem do bucket: `lista(inicio,
/// max)` devolve os quadros a partir do prefixo `inicio`. Vazio quando não há nenhum.
pub fn ultimos<E>(mut lista: impl FnMut(&str, u32) -> Result<Vec<Quadro>, E>) -> Result<Vec<Quadro>, E> {
    let Some(primeiro) = lista("", 1)?.into_iter().next() else {
        return Ok(Vec::new());
    };
    let mut existe = |prefixo: String| lista(&prefixo, 1).map(|v| !v.is_empty());
    let mut ano = primeiro.data.ano;
    while ano < 2200 && existe(format!("{}", ano + 1))? {
        ano += 1;
    }
    let mes = maior(1, 12, |m| existe(format!("{ano}{m:02}")))?;
    let dia = maior(1, 31, |d| existe(format!("{ano}{mes:02}{d:02}")))?;
    lista(&format!("{ano}{mes:02}{dia:02}"), 200)
}

/// A janela da área `c` (longitude e latitude, de -180 a 180°) na grade do store, de 0 a 360° de
/// longitude, com duas células de margem. `None` se a área cruza o meridiano de Greenwich.
pub fn janela(c: [f64; 4]) -> Option<[f64; 4]> {
    let m = 2.0 * LADO;
    let (o, l) = (c[0] - m, c[2] + m);
    let lon = |x: f64| if x < 0.0 { x + 360.0 } else { x };
    if o < 0.0 && l >= 0.0 {
        return None;
    }
    Some([lon(o), (c[1] - m).max(-90.0), lon(l), (c[3] + m).min(90.0)])
}

/// O lado da célula, em graus e em metros no equador.
pub const LADO: f64 = 0.1;
pub const LADO_M: f64 = LADO * 111_320.0;

/// A reamostragem para células de `celula` metros.
pub fn reamostragem(celula: f64) -> &'static str {
    if celula > LADO_M { "average" } else { "bilinear" }
}

/// A escala das cores, em mm/h.
pub const ESCALA: f64 = 50.0;
/// O teto do PARDIG, em mm/h.
pub const TETO: f64 = 75.0;

const PREFIXO: &str = "BrasilData: Chuva agora, ";
const SUFIXO: &str = " (PARDIG)";

/// O nome da camada do quadro, na hora de Brasília.
pub fn camada(q: Quadro) -> String {
    format!("{PREFIXO}{}{SUFIXO}", q.brasilia())
}

/// Se a camada `nome` é do PARDIG.
pub fn eh_do_pardig(nome: &str) -> bool {
    nome.starts_with(PREFIXO) && nome.ends_with(SUFIXO)
}

/// As camadas de passagem.
pub fn janela_lida(q: Quadro) -> String {
    format!("BrasilData: PARDIG {} (janela)", q.nome())
}
pub const EM_MM: &str = "BrasilData: PARDIG em mm/h";
pub const NA_GRADE: &str = "BrasilData: PARDIG na grade";

pub const CITACAO: &str = "Google DeepMind e Google Research, PARDIG: Precipitation AI Reanalysis, Densely Inferred GPM (Shalev et al. 2026, PARDIG: Global Precipitation Estimates Learned from Sparse Satellite Observations, arXiv:2611.13941), taxa de chuva a 0,1° a cada 15 minutos, bucket público gs://pardig_1; posta na grade do projeto e recortada pela área";
pub const LICENCA: &str = "CC BY 4.0 (dado experimental, fornecido como está; não é observação, previsão nem alerta oficial)";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn q(ano: i64, mes: u32, dia: u32, hora: u32, minuto: u32) -> Quadro {
        Quadro { data: Data { ano, mes, dia }, hora, minuto }
    }

    #[test]
    fn o_nome_do_store_vai_e_volta_e_a_hora_de_brasilia_e_tres_horas_antes() {
        let a = q(2026, 11, 16, 10, 30);
        assert_eq!(a.nome(), "20261116T103000");
        assert_eq!(Quadro::ler_nome("20261116T103000"), Some(a));
        assert_eq!(Quadro::ler_nome("20261116T103700"), None);
        assert_eq!(Quadro::ler_nome("success"), None);
        assert_eq!(a.brasilia(), "16/11/2026 07:30");
        assert_eq!(q(2026, 1, 1, 1, 0).brasilia(), "31/12/2025 22:00");
        assert_eq!(Quadro::de_brasilia("16/11/2026 07:40"), Some(a));
        assert_eq!(Quadro::de_brasilia("16/11/2026 7h"), Some(q(2026, 11, 16, 10, 0)));
        assert_eq!(Quadro::de_brasilia("31/12/2025 22:00"), Some(q(2026, 1, 1, 1, 0)));
        assert_eq!(Quadro::de_brasilia("16/11/2026"), Some(q(2026, 11, 16, 3, 0)));
        assert_eq!(Quadro::de_brasilia("16/11/2026 25:00"), None);
        assert_eq!(Quadro::de_minutos(a.minutos() + 14), a);
    }

    #[test]
    fn cada_periodo_tem_o_seu_store_e_o_passo_conta_de_15_em_15_minutos() {
        let o = origem(q(2026, 11, 16, 10, 30)).unwrap();
        assert_eq!(o.url, "https://storage.googleapis.com/pardig_1/NRT/zarr/2026_to_present/20261116T103000/predictions.zarr");
        assert_eq!(o.passo, None);
        // O último quadro do arquivo NRT é o passo 350656 (o arquivo tem 350657).
        let o = origem(q(2025, 12, 31, 23, 45)).unwrap();
        assert_eq!((o.url.ends_with(ARQUIVO_NRT), o.passo, o.versao), (true, Some(350_656), "NRT"));
        assert_eq!(origem(q(2016, 1, 1, 7, 45)).unwrap().passo, Some(0));
        // Antes das 07:45 de 2016-01-01, a LR, que tem 876576 passos até o fim de 2023.
        let o = origem(q(2016, 1, 1, 7, 30)).unwrap();
        assert_eq!(o.versao, "LR");
        // De 2016 a 2023 as duas cobrem; vale o NRT, de entradas melhores.
        assert_eq!(origem(q(2023, 12, 31, 23, 45)).unwrap().passo, Some(280_480));
        assert_eq!(origem(q(1999, 1, 1, 0, 0)).unwrap().passo, Some(0));
        let lr_fim = (q(2023, 12, 31, 23, 45).minutos() - q(1999, 1, 1, 0, 0).minutos()) / 15;
        assert_eq!(lr_fim, 876_575);
        assert_eq!(origem(q(1998, 12, 31, 23, 45)), None);
    }

    #[test]
    fn a_listagem_vira_quadros_e_a_bissecao_acha_o_ultimo_dia() {
        let r = json!({"prefixes": ["NRT/zarr/2026_to_present/20261116T001500/", "NRT/zarr/2026_to_present/20261116T000000/"]});
        assert_eq!(quadros_da_lista(&r), vec![q(2026, 11, 16, 0, 0), q(2026, 11, 16, 0, 15)]);
        assert!(quadros_da_lista(&json!({})).is_empty());
        assert!(url_lista("202611", 1).contains("&startOffset=NRT%2Fzarr%2F2026_to_present%2F202611&maxResults=1"));
        assert!(url_lista("", 1).contains("&startOffset=NRT%2Fzarr%2F2026_to_present%2F0&"));

        // Um bucket falso, de 01/01/2026 00:00 a 16/11/2026 10:30, de 15 em 15 minutos.
        let todos: Vec<Quadro> =
            (q(2026, 1, 1, 0, 0).minutos()..=q(2026, 11, 16, 10, 30).minutos()).step_by(15).map(Quadro::de_minutos).collect();
        let mut pedidos = 0;
        let v = ultimos(|inicio, max| {
            pedidos += 1;
            Ok::<_, ()>(todos.iter().filter(|x| x.nome().as_str() >= inicio).take(max as usize).copied().collect())
        })
        .unwrap();
        assert_eq!(v.last(), Some(&q(2026, 11, 16, 10, 30)));
        assert_eq!(v.len(), 43);
        assert!(pedidos <= 12, "{pedidos}");
        assert!(ultimos(|_, _| Ok::<_, ()>(Vec::new())).unwrap().is_empty());
    }

    #[test]
    fn a_janela_vai_para_0_a_360_graus() {
        let j = janela([-51.05, -22.93, -39.85, -14.23]).unwrap();
        assert!((j[0] - 308.75).abs() < 1e-9 && (j[2] - 320.35).abs() < 1e-9, "{j:?}");
        assert!((j[1] + 23.13).abs() < 1e-9);
        assert_eq!(janela([-1.0, 40.0, 2.0, 45.0]), None);
        assert!(janela([2.0, 40.0, 3.0, 45.0]).unwrap()[0] > 1.7);
        assert!(eh_do_pardig(&camada(q(2026, 11, 16, 10, 30))));
        assert_eq!(camada(q(2026, 11, 16, 10, 30)), "BrasilData: Chuva agora, 16/11/2026 07:30 (PARDIG)");
    }
}
