//! Os dados mensais do JAXA Earth (a agência espacial do Japão) que valem para o Brasil: a chuva
//! do GSMaP, a umidade do solo do AMSR2, a temperatura da superfície e a evapotranspiração do
//! GCOM-C e o índice de seca SPI do GSMaP, resumidos num período.
//!
//! Cada conjunto é uma coleção STAC estática, um catálogo por mês (`collection.json`), que o
//! `cube.reduce` do RustGIS lê só na área. O plugin converte a unidade (a taxa de chuva em mm/h
//! para mm/dia, o kelvin para °C), põe o resultado na grade do projeto e recorta pela área. Este
//! módulo tem as escolhas, as conversões, os nomes e as citações; quem chama o RustGIS é o
//! `comandos.rs`.

use serde_json::Value;

use crate::PREFIXO;
use crate::bdc::Estatistica;
use crate::datas::{Data, dias_no_mes};

/// O fim das camadas do JAXA: "…(GSMaP, JAXA Earth)".
const FIM: &str = ", JAXA Earth)";

/// O período mais longo de um clique, em dias (5 anos: 60 meses).
pub const MAX_DIAS: i64 = 5 * 366;

/// Os termos de uso do JAXA Earth.
pub const TERMOS: &str = "https://earth.jaxa.jp/en/data/policy/";

/// A hora da passagem do GCOM-C, para a temperatura da superfície.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turno {
    Dia,
    Noite,
}

impl Turno {
    /// O turno pelo parâmetro (`dia` ou `noite`).
    pub fn ler(texto: &str) -> Option<Turno> {
        match texto.trim().to_lowercase().as_str() {
            "dia" => Some(Turno::Dia),
            "noite" => Some(Turno::Noite),
            _ => None,
        }
    }
}

/// O dado do JAXA Earth.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variavel {
    Chuva,
    Umidade,
    Temperatura(Turno),
    Evapotranspiracao,
    Seca,
}

impl Variavel {
    /// O comando do plugin.
    pub fn comando(self) -> &'static str {
        match self {
            Variavel::Chuva => "jaxa-chuva",
            Variavel::Umidade => "jaxa-umidade",
            Variavel::Temperatura(_) => "jaxa-temperatura",
            Variavel::Evapotranspiracao => "jaxa-evapotranspiracao",
            Variavel::Seca => "jaxa-seca",
        }
    }

    /// O conjunto no catálogo do JAXA Earth.
    pub fn conjunto(self) -> &'static str {
        match self {
            Variavel::Chuva => "JAXA.EORC_GSMaP_standard.Gauge.00Z-23Z.v6_monthly",
            Variavel::Umidade => "JAXA.G-Portal_GCOM-W.AMSR2_standard.L3-SMC.daytime.v3_global_monthly",
            Variavel::Temperatura(Turno::Dia) => "JAXA.G-Portal_GCOM-C.SGLI_standard.L3-LST.daytime.v3_global_monthly",
            Variavel::Temperatura(Turno::Noite) => "JAXA.G-Portal_GCOM-C.SGLI_standard.L3-LST.nighttime.v3_global_monthly",
            Variavel::Evapotranspiracao => "JAXA.JASMES_GCOM-C.SGLI_ET.v1_global_monthly",
            Variavel::Seca => "JAXA.EORC_GSMaP_SPI.climate.gnrt6_monthly",
        }
    }

    /// O `collection.json` do conjunto (o SPI fica no segundo bucket do JAXA Earth).
    pub fn colecao(self) -> String {
        let bucket = if self == Variavel::Seca { "je-pds2" } else { "je-pds" };
        format!("https://s3.ap-northeast-1.wasabisys.com/{bucket}/cog/v1/{}/collection.json", self.conjunto())
    }

    /// A banda do conjunto.
    pub fn banda(self) -> &'static str {
        match self {
            Variavel::Chuva => "PRECIP",
            Variavel::Umidade => "SMC",
            Variavel::Temperatura(_) => "LST",
            Variavel::Evapotranspiracao => "ET",
            Variavel::Seca => "SPI",
        }
    }

    /// O nome do dado na camada, com o gênero para o resumo.
    pub fn titulo(self) -> (&'static str, bool) {
        match self {
            Variavel::Chuva => ("Chuva", true),
            Variavel::Umidade => ("Umidade do solo", true),
            Variavel::Temperatura(Turno::Dia) => ("Temperatura da superfície de dia", true),
            Variavel::Temperatura(Turno::Noite) => ("Temperatura da superfície à noite", true),
            Variavel::Evapotranspiracao => ("Evapotranspiração", true),
            Variavel::Seca => ("Índice de seca (SPI)", false),
        }
    }

    /// O satélite ou o produto, no nome da camada.
    pub fn fonte(self) -> &'static str {
        match self {
            Variavel::Chuva | Variavel::Seca => "GSMaP",
            Variavel::Umidade => "AMSR2",
            Variavel::Temperatura(_) | Variavel::Evapotranspiracao => "GCOM-C",
        }
    }

    /// A unidade, depois da conversão (o SPI não tem).
    pub fn unidade(self) -> &'static str {
        match self {
            Variavel::Chuva | Variavel::Evapotranspiracao => "mm/dia",
            Variavel::Umidade => "%",
            Variavel::Temperatura(_) => "°C",
            Variavel::Seca => "",
        }
    }

    /// A conversão do valor do arquivo: `a · x + b` (a taxa de chuva em mm/h vira mm/dia; o
    /// kelvin, °C). No desvio-padrão só vale o `a`.
    pub fn conversao(self) -> (f64, f64) {
        match self {
            Variavel::Chuva => (24.0, 0.0),
            Variavel::Temperatura(_) => (1.0, -273.15),
            _ => (1.0, 0.0),
        }
    }

    /// A expressão do `bandMath` que converte o resumo `e`, ou nada quando ele já está na unidade.
    pub fn expressao(self, e: Estatistica) -> Option<String> {
        let (a, b) = self.conversao();
        let b = if e == Estatistica::Desvio { 0.0 } else { b };
        match (a == 1.0, b == 0.0) {
            (true, true) => None,
            (false, true) => Some(format!("b1 * {a}")),
            (true, false) => Some(format!("b1 - {}", -b)),
            (false, false) => Some(format!("b1 * {a} + {b}")),
        }
    }

    /// A escala das cores (na unidade convertida), a mesma em todas as áreas e períodos.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Variavel::Chuva => (0.0, 12.0),
            Variavel::Umidade => (0.0, 30.0),
            Variavel::Temperatura(_) => (10.0, 45.0),
            Variavel::Evapotranspiracao => (0.0, 6.0),
            Variavel::Seca => (-2.0, 2.0),
        }
    }

    /// A rampa (nome e texto do `ramp.add`, formato GDAL), do menor ao maior valor da escala.
    pub fn rampa(self) -> (&'static str, String) {
        let (nome, cores): (&str, [&str; 5]) = match self {
            Variavel::Chuva => ("BrasilData: chuva do JAXA", ["255 255 229", "199 233 180", "65 182 196", "34 94 168", "8 29 88"]),
            Variavel::Umidade => ("BrasilData: umidade do solo", ["140 81 10", "216 179 101", "246 232 195", "90 180 172", "1 102 94"]),
            Variavel::Temperatura(_) => ("BrasilData: temperatura", ["49 54 149", "145 191 219", "255 255 191", "252 141 89", "165 0 38"]),
            Variavel::Evapotranspiracao => {
                ("BrasilData: evapotranspiração", ["255 255 204", "194 230 153", "120 198 121", "49 163 84", "0 104 55"])
            }
            Variavel::Seca => ("BrasilData: SPI", ["166 97 26", "223 194 125", "245 245 245", "128 205 193", "1 133 113"]),
        };
        let (lo, hi) = self.escala();
        let passo = (hi - lo) / 4.0;
        (nome, cores.iter().enumerate().map(|(k, c)| format!("{} {c}\n", lo + passo * k as f64)).collect())
    }

    /// O primeiro mês com dados na árvore do STAC (a da evapotranspiração começa em 2025-04,
    /// embora o catálogo diga 2018).
    pub fn inicio(self) -> Data {
        let (ano, mes) = match self {
            Variavel::Chuva => (2000, 3),
            Variavel::Umidade => (2012, 7),
            Variavel::Temperatura(_) => (2018, 1),
            Variavel::Evapotranspiracao => (2025, 4),
            Variavel::Seca => (2019, 6),
        };
        Data { ano, mes, dia: 1 }
    }

    /// A citação (a de cada conjunto no seu `collection.json`, com o DOI) e o crédito que os
    /// termos do JAXA pedem: "JAXA" e o nome do conjunto.
    pub fn citacao(self) -> String {
        let fonte = match self {
            Variavel::Chuva => "Japan Aerospace Exploration Agency, GSMaP (Monthly), v6, doi:10.57746/EO.01gs73bktym5xwr10j34rwqgj8",
            Variavel::Umidade => {
                "Japan Aerospace Exploration Agency, GCOM-W/AMSR2 L3 Soil Moisture Content (1-Month, 0.1 deg), v3, doi:10.57746/EO.01gs73b17zekqwyj4hh9xpqgy7"
            }
            Variavel::Temperatura(_) => {
                "Japan Aerospace Exploration Agency, GCOM-C/SGLI L3 Map Land surface temperature (LST) (1-Month, 1/24 deg), v3, doi:10.57746/EO.01gs73bfs5sbzhyxg4j9ytajsa"
            }
            Variavel::Evapotranspiracao => {
                "Japan Aerospace Exploration Agency, JASMES Evapotranspiration (ET) (Monthly, 0.05 deg), v1, doi:10.57746/EO.01jjxj3qr7wrdzskzjs9cmxhnf"
            }
            Variavel::Seca => "Japan Aerospace Exploration Agency, GSMaP SPI, Standardized Precipitation Index (Monthly)",
        };
        format!("{fonte}; pelo JAXA Earth API ({})", self.conjunto())
    }
}

/// A licença dos dados do JAXA Earth.
pub const LICENCA: &str = "termos do JAXA Earth (earth.jaxa.jp/en/data/policy): uso livre, com o crédito \"JAXA\" e o nome do conjunto; uso comercial, avisando a JAXA antes";

/// "Chuva média", "Desvio da chuva", "Índice de seca (SPI) máximo".
fn resumo(v: Variavel, e: Estatistica) -> String {
    let (t, feminino) = v.titulo();
    let (o, a) = if feminino { ("a", "a") } else { ("o", "o") };
    match e {
        Estatistica::Media => format!("{t} médi{o}"),
        Estatistica::Maxima => format!("{t} máxim{a}"),
        Estatistica::Minima => format!("{t} mínim{a}"),
        Estatistica::Desvio => {
            let mut c = t.chars();
            let primeira = c.next().map(|p| p.to_lowercase().collect::<String>()).unwrap_or_default();
            format!("Desvio d{a} {primeira}{}", c.as_str())
        }
    }
}

/// A camada: "BrasilData: Chuva média de 01/08/2026 a 31/10/2026 (GSMaP, JAXA Earth)".
pub fn camada(v: Variavel, e: Estatistica, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}{} de {} a {} ({}{FIM}", resumo(v, e), inicio.texto(), fim.texto(), v.fonte())
}

/// Se a camada é do JAXA Earth, pelo nome.
pub fn eh_do_jaxa(nome: &str) -> bool {
    nome.starts_with(PREFIXO) && nome.ends_with(FIM)
}

/// O dado de uma camada do JAXA Earth, pelo nome (o turno da temperatura também).
pub fn variavel_da_camada(nome: &str) -> Option<Variavel> {
    if !eh_do_jaxa(nome) {
        return None;
    }
    let n = nome.strip_prefix(PREFIXO)?.to_lowercase();
    let tem = |t: &str| {
        n.starts_with(&t.to_lowercase())
            || n.starts_with(&format!("desvio da {}", t.to_lowercase()))
            || n.starts_with(&format!("desvio do {}", t.to_lowercase()))
    };
    [
        Variavel::Temperatura(Turno::Dia),
        Variavel::Temperatura(Turno::Noite),
        Variavel::Umidade,
        Variavel::Evapotranspiracao,
        Variavel::Seca,
        Variavel::Chuva,
    ]
    .into_iter()
    .find(|v| tem(v.titulo().0))
}

/// O último mês com dados, pelos catálogos filhos do `collection.json` ("./2026-10/catalog.json").
pub fn ultimo_mes(colecao: &Value) -> Option<(i64, u32)> {
    colecao["links"]
        .as_array()?
        .iter()
        .filter(|l| l["rel"] == "child")
        .filter_map(|l| {
            let h = l["href"].as_str()?.trim_start_matches("./");
            let m = h.split('/').next()?;
            let (a, mm) = m.split_once('-')?;
            Some((a.parse::<i64>().ok()?, mm.parse::<u32>().ok().filter(|m| (1..=12).contains(m))?))
        })
        .max()
}

/// O período padrão: os 3 meses até o último com dados, do dia 1 do primeiro ao fim do último.
pub fn periodo_padrao((ano, mes): (i64, u32)) -> (Data, Data) {
    let (a0, m0) = if mes > 2 { (ano, mes - 2) } else { (ano - 1, mes + 10) };
    (Data { ano: a0, mes: m0, dia: 1 }, Data { ano, mes, dia: dias_no_mes(ano, mes) })
}

/// Confere o período pedido ao dado `v`.
pub fn conferir_periodo(v: Variavel, inicio: Data, fim: Data) -> Result<(), String> {
    if fim < inicio {
        return Err(format!("o fim ({}) vem antes do início ({})", fim.texto(), inicio.texto()));
    }
    let comeco = v.inicio();
    if fim < comeco {
        return Err(format!("o JAXA Earth tem este dado só desde {}", comeco.texto()));
    }
    if fim.dias() - inicio.dias() > MAX_DIAS {
        return Err("o período passa de 5 anos: escolha um mais curto".into());
    }
    Ok(())
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn conversoes_e_nomes() {
        assert_eq!(Variavel::Chuva.expressao(Estatistica::Media).as_deref(), Some("b1 * 24"));
        assert_eq!(Variavel::Temperatura(Turno::Dia).expressao(Estatistica::Media).as_deref(), Some("b1 - 273.15"));
        assert_eq!(Variavel::Temperatura(Turno::Dia).expressao(Estatistica::Desvio), None);
        assert_eq!(Variavel::Umidade.expressao(Estatistica::Maxima), None);
        let (a, b) = (Data { ano: 2026, mes: 8, dia: 1 }, Data { ano: 2026, mes: 10, dia: 31 });
        let n = camada(Variavel::Chuva, Estatistica::Media, a, b);
        assert_eq!(n, "BrasilData: Chuva média de 01/08/2026 a 31/10/2026 (GSMaP, JAXA Earth)");
        assert_eq!(variavel_da_camada(&n), Some(Variavel::Chuva));
        let d = camada(Variavel::Seca, Estatistica::Desvio, a, b);
        assert_eq!(d, "BrasilData: Desvio do índice de seca (SPI) de 01/08/2026 a 31/10/2026 (GSMaP, JAXA Earth)");
        assert_eq!(variavel_da_camada(&d), Some(Variavel::Seca));
        let t = camada(Variavel::Temperatura(Turno::Noite), Estatistica::Maxima, a, b);
        assert_eq!(variavel_da_camada(&t), Some(Variavel::Temperatura(Turno::Noite)));
        assert_eq!(variavel_da_camada("BrasilData: NDVI médio de 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)"), None);
        assert!(Variavel::Seca.colecao().contains("/je-pds2/"));
        assert!(Variavel::Chuva.colecao().ends_with("v6_monthly/collection.json"));
    }

    #[test]
    fn ultimo_mes_e_periodo() {
        let c = json!({"links": [
            {"rel": "root", "href": "./collection.json"},
            {"rel": "child", "href": "./2026-09/catalog.json"},
            {"rel": "child", "href": "./2026-10/catalog.json"},
            {"rel": "child", "href": "./2025-12/catalog.json"},
            {"rel": "license", "href": "https://earth.jaxa.jp/en/data/policy/"}
        ]});
        assert_eq!(ultimo_mes(&c), Some((2026, 10)));
        assert_eq!(periodo_padrao((2026, 10)), (Data { ano: 2026, mes: 8, dia: 1 }, Data { ano: 2026, mes: 10, dia: 31 }));
        assert_eq!(periodo_padrao((2026, 1)), (Data { ano: 2025, mes: 11, dia: 1 }, Data { ano: 2026, mes: 1, dia: 31 }));
        let (i, f) = periodo_padrao((2026, 10));
        assert!(conferir_periodo(Variavel::Chuva, i, f).is_ok());
        assert!(
            conferir_periodo(Variavel::Evapotranspiracao, Data { ano: 2024, mes: 1, dia: 1 }, Data { ano: 2024, mes: 6, dia: 1 }).is_err()
        );
        assert!(conferir_periodo(Variavel::Chuva, f, i).is_err());
        let (_, r) = Variavel::Temperatura(Turno::Dia).rampa();
        assert!(r.starts_with("10 49 54 149\n"));
    }
}
