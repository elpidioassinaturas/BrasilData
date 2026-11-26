//! A previsão do tempo do BAM, o modelo global do CPTEC/INPE (Brazilian Global Atmospheric Model,
//! TQ0666L064: cerca de 20 km), lida dos arquivos da rodada no servidor de dados do CPTEC.
//!
//! - **De onde:** `dataserver.cptec.inpe.br/dataserver_modelos/bam/TQ0666L064/brutos/AAAA/MM/DD/00/`, um
//!   GRIB1 do globo por prazo, de 6 em 6 horas até 264 h (cerca de 1,6 GB cada), com o inventário dele
//!   (`.inv`, texto: o byte em que começa cada registro). O RustGIS 0.8.0 baixa só o registro pedido
//!   (`layer.addUrl` com `bytes`, cerca de 4 MB), numa grade regular de 0,18° (2000 × 1000).
//! - **Os registros:** `prec` é a chuva média das 6 horas que terminam no prazo, em mm por dia (a soma
//!   de um dia é a média dos quatro × 1; um prazo vale 6/24 dela); `tp2m` é a temperatura a 2 m no
//!   instante do prazo, em °C. O modelo espectral deixa valores um pouco negativos de chuva, zerados.
//! - **Conferido em 25/11/2026:** dois registros lidos com o GDAL (rasterio): unidades "Kg/m2/day" e
//!   "C", grade de 0,18° a partir de −180,09°, 90,09°.

use crate::datas::Data;

/// A pasta das rodadas.
pub const BASE: &str = "https://dataserver.cptec.inpe.br/dataserver_modelos/bam/TQ0666L064/brutos/";

/// O começo do nome das camadas.
pub const PREFIXO_CAMADA: &str = "BrasilData: Previsão do BAM (CPTEC/INPE): ";

/// O começo do nome das camadas de passagem (um registro baixado, um recorte).
pub const PASSAGEM: &str = "BrasilData: BAM, registro ";

/// A primeira rodada que o servidor guarda (conferido em 25/11/2026).
pub const PRIMEIRA: Data = Data { ano: 2025, mes: 1, dia: 1 };

/// O mais e o padrão de dias.
pub const MAX_DIAS: u32 = 10;
pub const DIAS_PADRAO: u32 = 3;

/// A grade do arquivo: o canto noroeste e a célula, em graus.
pub const ORIGEM: [f64; 2] = [-180.09, 90.09];
pub const CELULA: f64 = 0.18;

/// A margem (graus) do recorte de cada registro em volta da área: duas células.
pub const MARGEM: f64 = 0.4;

/// As células do recorte de um registro acima das quais a área é grande demais (o `cube.reduce` junta
/// até 32 milhões de valores, e são até 40 registros).
pub const MAX_CELULAS: f64 = 600_000.0;

pub const CITACAO: &str = "CPTEC/INPE, Modelo Atmosférico Global Brasileiro (BAM), resolução TQ0666L064 (cerca de 20 km), rodada das 00 UTC; Figueroa, S. N. et al. (2016), The Brazilian Global Atmospheric Model (BAM): performance for tropical rainfall forecasting and sensitivity to convective scheme and horizontal resolution, Weather and Forecasting 31, 1547–1572, doi:10.1175/WAF-D-16-0062.1";

pub const LICENCA: &str = "dados públicos do INPE, com a citação da fonte";

/// O que a previsão traz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variavel {
    Chuva,
    Temperatura,
}

impl Variavel {
    pub fn ler(t: Option<&str>) -> Result<Variavel, String> {
        match t.map(|t| t.trim().to_lowercase()).as_deref() {
            None | Some("") | Some("chuva") => Ok(Variavel::Chuva),
            Some("temperatura") => Ok(Variavel::Temperatura),
            Some(o) => Err(format!("variável '{o}': use chuva ou temperatura")),
        }
    }

    /// O nome do registro no inventário.
    pub fn registro(self) -> &'static str {
        match self {
            Variavel::Chuva => ":prec:",
            Variavel::Temperatura => ":tp2m:",
        }
    }

    pub fn codigo(self) -> &'static str {
        match self {
            Variavel::Chuva => "chuva",
            Variavel::Temperatura => "temperatura",
        }
    }
}

/// Uma rodada (sempre a das 00 UTC, a que o servidor guarda).
pub fn pasta(d: Data) -> String {
    format!("{BASE}{}/{:02}/{:02}/00/", d.ano, d.mes, d.dia)
}

/// "AAAAMMDDHH" de `d` mais `horas`.
fn instante(d: Data, horas: u32) -> String {
    let dia = Data::de_milissegundos((d.dias() + i64::from(horas / 24)) * 86_400_000);
    format!("{}{:02}{:02}{:02}", dia.ano, dia.mes, dia.dia, horas % 24)
}

/// O nome do arquivo (sem a extensão) do prazo `horas` da rodada `d`.
pub fn arquivo(d: Data, horas: u32) -> String {
    format!("GPOSNMC{}{}P.fct.TQ0666L064", instante(d, 0), instante(d, horas))
}

/// Os prazos de `dias` dias: 6, 12, … 24 × dias horas.
pub fn prazos(dias: u32) -> Vec<u32> {
    (1..=dias * 4).map(|k| k * 6).collect()
}

/// O primeiro e o último byte do registro da variável no inventário (`n:byte:d=…:nome:…`), o último
/// pelo começo do registro seguinte.
pub fn trecho(inv: &str, v: Variavel) -> Option<[u64; 2]> {
    let mut linhas = inv.lines();
    while let Some(l) = linhas.next() {
        if l.contains(v.registro()) {
            let inicio: u64 = l.split(':').nth(1)?.parse().ok()?;
            let fim: u64 = linhas.next()?.split(':').nth(1)?.parse().ok()?;
            return (fim > inicio).then_some([inicio, fim - 1]);
        }
    }
    None
}

/// As pastas na página da lista (`href="25/"`), em ordem.
pub fn pastas(lista: &str) -> Vec<u32> {
    let mut v: Vec<u32> = lista
        .match_indices("href=\"")
        .filter_map(|(i, m)| {
            let resto = &lista[i + m.len()..];
            let nome = resto.split('"').next()?.strip_suffix('/')?;
            (nome.bytes().all(|b| b.is_ascii_digit()) && !nome.is_empty()).then(|| nome.parse().ok()).flatten()
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// Se a lista da pasta da rodada tem o arquivo do último prazo pedido (a rodada pode estar chegando).
pub fn completa(lista: &str, d: Data, dias: u32) -> bool {
    lista.contains(&format!("{}.grb\"", arquivo(d, dias * 24)))
}

/// O período em horário de Brasília (3 horas antes do UTC): do instante da rodada a `dias` dias depois.
pub fn periodo(d: Data, dias: u32) -> (String, String) {
    let antes = Data::de_milissegundos((d.dias() - 1) * 86_400_000);
    let depois = Data::de_milissegundos((d.dias() + i64::from(dias) - 1) * 86_400_000);
    (format!("{:02}/{:02} 21h", antes.dia, antes.mes), format!("{:02}/{:02}/{} 21h", depois.dia, depois.mes, depois.ano))
}

/// O nome da camada.
pub fn camada(v: Variavel, dias: u32, d: Data) -> String {
    let (de, ate) = periodo(d, dias);
    let o_que = match v {
        Variavel::Chuva => "chuva",
        Variavel::Temperatura => "temperatura média",
    };
    format!("{PREFIXO_CAMADA}{o_que} em {dias} dia{}, {de} a {ate}", if dias == 1 { "" } else { "s" })
}

/// Se a camada `nome` é daqui (e, se é, se é de chuva).
pub fn da_camada(nome: &str) -> Option<Variavel> {
    let n = nome.strip_prefix(PREFIXO_CAMADA)?;
    if n.starts_with("chuva em ") {
        Some(Variavel::Chuva)
    } else if n.starts_with("temperatura média em ") {
        Some(Variavel::Temperatura)
    } else {
        None
    }
}

/// Os dias da camada ("… em 3 dias, …").
pub fn dias_da_camada(nome: &str) -> Option<usize> {
    let resto = &nome[nome.find(" em ")? + 4..];
    resto.split(' ').next()?.parse().ok()
}

/// O nome do registro baixado e o do recorte dele.
pub fn registro(v: Variavel, d: Data, horas: u32) -> (String, String) {
    let base = format!("{PASSAGEM}{} {}", v.codigo(), instante(d, horas));
    (base.clone(), format!("{base} (recorte)"))
}

/// O recorte de cada registro: o retângulo da área com a margem, nas células do arquivo.
pub fn recorte(c: [f64; 4]) -> [f64; 4] {
    [(c[0] - MARGEM).max(-180.0), (c[1] - MARGEM).max(-90.0), (c[2] + MARGEM).min(180.0), (c[3] + MARGEM).min(90.0)]
}

/// As células do recorte da área `c`.
pub fn celulas(c: [f64; 4]) -> f64 {
    let r = recorte(c);
    ((r[2] - r[0]) / CELULA).ceil() * ((r[3] - r[1]) / CELULA).ceil()
}

#[cfg(test)]
mod testes {
    use super::*;

    const R: Data = Data { ano: 2026, mes: 11, dia: 25 };

    #[test]
    fn os_arquivos_e_os_prazos() {
        assert_eq!(arquivo(R, 6), "GPOSNMC20261125002026112506P.fct.TQ0666L064");
        assert_eq!(arquivo(R, 24), "GPOSNMC20261125002026112600P.fct.TQ0666L064");
        assert_eq!(arquivo(R, 264), "GPOSNMC20261125002026120600P.fct.TQ0666L064");
        assert_eq!(arquivo(Data { ano: 2026, mes: 12, dia: 31 }, 30), "GPOSNMC20261231002027010106P.fct.TQ0666L064");
        assert_eq!(pasta(R), format!("{BASE}2026/11/25/00/"));
        assert_eq!(prazos(1), [6, 12, 18, 24]);
        assert_eq!(prazos(10).len(), 40);
    }

    #[test]
    fn o_inventario() {
        let inv = "449:1556037632:d=26112500:tp2m:kpds5=128\n450:1560037716:d=26112500:prec:kpds5=61:TR=4\n451:1564037800:d=26112500:neve:kpds5=65\n";
        assert_eq!(trecho(inv, Variavel::Chuva), Some([1_560_037_716, 1_564_037_799]));
        assert_eq!(trecho(inv, Variavel::Temperatura), Some([1_556_037_632, 1_560_037_715]));
        assert_eq!(trecho("1:0:d=1:topo:\n", Variavel::Chuva), None);
    }

    #[test]
    fn as_listas() {
        let l = r#"<a href="?C=N;O=D">x</a> <a href="/dataserver_modelos/bam/">p</a> <a href="23/">23/</a> <a href="25/">25/</a> <a href="24/">24/</a>"#;
        assert_eq!(pastas(l), [23, 24, 25]);
        let r = r#"<a href="GPOSNMC20261125002026112800P.fct.TQ0666L064.grb">x</a>"#;
        assert!(completa(r, R, 3) && !completa(r, R, 4));
    }

    #[test]
    fn os_nomes() {
        assert_eq!(periodo(R, 3), ("24/11 21h".to_string(), "27/11/2026 21h".to_string()));
        let n = camada(Variavel::Chuva, 3, R);
        assert_eq!(n, "BrasilData: Previsão do BAM (CPTEC/INPE): chuva em 3 dias, 24/11 21h a 27/11/2026 21h");
        assert_eq!(da_camada(&n), Some(Variavel::Chuva));
        assert_eq!(dias_da_camada(&n), Some(3));
        let t = camada(Variavel::Temperatura, 1, R);
        assert!(t.contains("temperatura média em 1 dia,") && da_camada(&t) == Some(Variavel::Temperatura));
        assert!(!crate::tempo::eh_da_previsao(&n) && !crate::tempo::eh_da_previsao(&t));
        assert_eq!(registro(Variavel::Chuva, R, 30).1, "BrasilData: BAM, registro chuva 2026112606 (recorte)");
        assert!(Variavel::ler(Some("vento")).is_err() && Variavel::ler(None) == Ok(Variavel::Chuva));
        assert!(celulas([-51.1, -22.9, -39.9, -14.2]) < MAX_CELULAS && celulas([-74.0, -34.0, -34.0, 6.0]) < MAX_CELULAS);
    }
}
