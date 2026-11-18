//! O cubo de dados do INPE (Brazil Data Cube, BDC): os compostos de 16 dias de índices de
//! vegetação do Sentinel-2, do Landsat, do CBERS-4 e do MODIS, pelo STAC do INPE.
//!
//! Dois comandos: o índice de um período resumido (a média, a máxima, a mínima ou o desvio das
//! datas), lido só na área pelo `cube.reduce` do RustGIS, posto na grade do projeto e recortado
//! pelo contorno; e a série no tempo, pelo `stac.addCube`, que segue a barra de tempo do RustGIS
//! (só no app desktop). Este módulo tem as escolhas, os nomes das camadas e as citações; quem
//! chama o RustGIS é o `comandos.rs`.

use serde_json::Value;

use crate::PREFIXO;
use crate::datas::Data;

/// O STAC do Brazil Data Cube.
pub const STAC: &str = "https://data.inpe.br/bdc/stac/v1/";

/// A página do Brazil Data Cube.
pub const PAGINA: &str = "https://data.inpe.br/bdc/";

/// O fim das camadas do cubo: "…(Sentinel-2, cubo do INPE)".
const FIM: &str = ", cubo do INPE)";

/// O período mais longo de um clique, em dias (cerca de 3 anos, umas 70 datas de 16 dias).
pub const MAX_DIAS: i64 = 3 * 366;

/// Os dias do período padrão, até a última data da coleção (cerca de 3 meses, 6 datas).
pub const DIAS_PADRAO: i64 = 90;

/// O satélite de um cubo de 16 dias do BDC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Satelite {
    Sentinel2,
    Landsat,
    Cbers4,
    Modis,
}

impl Satelite {
    /// Todos, na ordem da janela.
    pub const TODOS: [Satelite; 4] = [Satelite::Sentinel2, Satelite::Landsat, Satelite::Cbers4, Satelite::Modis];

    /// O satélite pelo valor do parâmetro (`sentinel-2`, `landsat`, `cbers-4`, `modis`).
    pub fn ler(texto: &str) -> Option<Satelite> {
        let t = texto.trim().to_lowercase();
        Satelite::TODOS.into_iter().find(|s| s.id() == t)
    }

    /// O valor do parâmetro.
    pub fn id(self) -> &'static str {
        match self {
            Satelite::Sentinel2 => "sentinel-2",
            Satelite::Landsat => "landsat",
            Satelite::Cbers4 => "cbers-4",
            Satelite::Modis => "modis",
        }
    }

    /// O nome no texto e na camada.
    pub fn nome(self) -> &'static str {
        match self {
            Satelite::Sentinel2 => "Sentinel-2",
            Satelite::Landsat => "Landsat",
            Satelite::Cbers4 => "CBERS-4 WFI",
            Satelite::Modis => "MODIS",
        }
    }

    /// A coleção do STAC do BDC.
    pub fn colecao(self) -> &'static str {
        match self {
            Satelite::Sentinel2 => "S2-16D-2",
            Satelite::Landsat => "LANDSAT-16D-1",
            Satelite::Cbers4 => "CBERS4-WFI-16D-2",
            Satelite::Modis => "mod13q1-6.1",
        }
    }

    /// O lado das células dos arquivos, em metros.
    pub fn celula(self) -> f64 {
        match self {
            Satelite::Sentinel2 => 10.0,
            Satelite::Landsat => 30.0,
            Satelite::Cbers4 => 64.0,
            Satelite::Modis => 232.0,
        }
    }

    /// A primeira data da coleção.
    pub fn inicio(self) -> Data {
        match self {
            Satelite::Sentinel2 => Data { ano: 2017, mes: 1, dia: 1 },
            Satelite::Landsat => Data { ano: 1990, mes: 1, dia: 1 },
            Satelite::Cbers4 => Data { ano: 2016, mes: 1, dia: 1 },
            Satelite::Modis => Data { ano: 2000, mes: 2, dia: 18 },
        }
    }

    /// A licença das imagens.
    pub fn licenca(self) -> &'static str {
        match self {
            Satelite::Sentinel2 => "dados Copernicus Sentinel: livres e abertos, com atribuição",
            Satelite::Landsat => "dados Landsat do USGS: domínio público",
            Satelite::Cbers4 => "CC BY 4.0 (INPE)",
            Satelite::Modis => "dados MODIS do LP DAAC da NASA: sem restrição de uso",
        }
    }

    /// A citação da coleção.
    pub fn citacao(self) -> String {
        let origem = match self {
            Satelite::Sentinel2 => "imagens Sentinel-2/MSI nível 2A (Copernicus, ESA)",
            Satelite::Landsat => "imagens Landsat Coleção 2 nível 2 (USGS)",
            Satelite::Cbers4 => "imagens CBERS-4/WFI nível 4 (INPE)",
            Satelite::Modis => "o produto MOD13Q1 v6.1 da NASA (LP DAAC)",
        };
        format!(
            "Brazil Data Cube (INPE), coleção {} ({PAGINA}), de {origem}; Ferreira, K. R. et al., Earth Observation Data Cubes for Brazil: Requirements, Methodology and Products, Remote Sensing 12(24), 4033, 2020, doi:10.3390/rs12244033",
            self.colecao()
        )
    }
}

/// O índice de vegetação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indice {
    Ndvi,
    Evi,
}

impl Indice {
    /// O índice pelo parâmetro (`NDVI` ou `EVI`, em qualquer caixa).
    pub fn ler(texto: &str) -> Option<Indice> {
        match texto.trim().to_uppercase().as_str() {
            "NDVI" => Some(Indice::Ndvi),
            "EVI" => Some(Indice::Evi),
            _ => None,
        }
    }

    /// A banda no STAC, que é também o nome no texto.
    pub fn banda(self) -> &'static str {
        match self {
            Indice::Ndvi => "NDVI",
            Indice::Evi => "EVI",
        }
    }

    /// A escala das cores: a mesma em todas as datas e áreas, para que se comparem.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Indice::Ndvi => (0.0, 0.9),
            Indice::Evi => (0.0, 0.8),
        }
    }
}

/// O resumo das datas do período.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estatistica {
    Media,
    Maxima,
    Minima,
    Desvio,
}

impl Estatistica {
    /// A estatística pelo parâmetro (`media`, `maxima`, `minima`, `desvio`).
    pub fn ler(texto: &str) -> Option<Estatistica> {
        match texto.trim().to_lowercase().as_str() {
            "media" | "média" => Some(Estatistica::Media),
            "maxima" | "máxima" => Some(Estatistica::Maxima),
            "minima" | "mínima" => Some(Estatistica::Minima),
            "desvio" => Some(Estatistica::Desvio),
            _ => None,
        }
    }

    /// O `stat` do `cube.reduce`.
    pub fn stat(self) -> &'static str {
        match self {
            Estatistica::Media => "mean",
            Estatistica::Maxima => "max",
            Estatistica::Minima => "min",
            Estatistica::Desvio => "std",
        }
    }

    /// Como o nome da camada diz o resumo de `indice`: "NDVI médio", "desvio do NDVI".
    pub fn do_indice(self, indice: Indice) -> String {
        let i = indice.banda();
        match self {
            Estatistica::Media => format!("{i} médio"),
            Estatistica::Maxima => format!("{i} máximo"),
            Estatistica::Minima => format!("{i} mínimo"),
            Estatistica::Desvio => format!("Desvio do {i}"),
        }
    }
}

/// A camada do índice resumido: "BrasilData: NDVI médio de 01/06/2025 a 31/08/2025 (Sentinel-2,
/// cubo do INPE)".
pub fn camada(s: Satelite, i: Indice, e: Estatistica, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}{} de {} a {} ({}{FIM}", e.do_indice(i), inicio.texto(), fim.texto(), s.nome())
}

/// A camada da série no tempo: "BrasilData: NDVI no tempo de … a … (Sentinel-2, cubo do INPE)".
pub fn camada_da_serie(s: Satelite, i: Indice, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}{} no tempo de {} a {} ({}{FIM}", i.banda(), inicio.texto(), fim.texto(), s.nome())
}

/// O satélite de uma camada do cubo, pelo nome dela.
pub fn satelite_da_camada(nome: &str) -> Option<Satelite> {
    let resto = nome.strip_prefix(PREFIXO)?.strip_suffix(FIM)?;
    let (_, sat) = resto.rsplit_once(" (")?;
    Satelite::TODOS.into_iter().find(|s| s.nome() == sat)
}

/// Se a camada é uma série no tempo do cubo.
pub fn eh_serie(nome: &str) -> bool {
    satelite_da_camada(nome).is_some() && nome.contains(" no tempo de ")
}

/// O lado das células a ler do cubo para uma grade de `grade` metros: a pirâmide do arquivo mais
/// próxima de 3/4 da grade (o RustGIS lê a mais próxima), nunca mais fina que o arquivo. Numa grade
/// de 30 m, o Sentinel-2 vem da pirâmide de 20 m.
pub fn celula_de_leitura(s: Satelite, grade: f64) -> f64 {
    (0.75 * grade).max(s.celula())
}

/// A última data de uma coleção, pela resposta do STAC (`extent.temporal.interval`).
pub fn fim_da_colecao(colecao: &Value) -> Option<Data> {
    let fim = colecao["extent"]["temporal"]["interval"][0][1].as_str()?;
    let (a, m, d) = (fim.get(0..4)?.parse().ok()?, fim.get(5..7)?.parse().ok()?, fim.get(8..10)?.parse().ok()?);
    Some(Data { ano: a, mes: m, dia: d })
}

/// Uma data escrita "dd/mm/aaaa" (a janela) ou "aaaa-mm-dd" (um script).
pub fn ler_data(texto: &str) -> Option<Data> {
    let t = texto.trim();
    Data::ler(t).or_else(|| {
        let mut p = t.split('-');
        let (a, m, d) = (p.next()?, p.next()?, p.next()?);
        if p.next().is_some() || a.len() != 4 {
            return None;
        }
        Data::ler(&format!("{d}/{m}/{a}"))
    })
}

/// O período padrão: os [`DIAS_PADRAO`] até `fim`.
pub fn periodo_padrao(fim: Data) -> (Data, Data) {
    (Data::de_milissegundos((fim.dias() - DIAS_PADRAO) * 86_400_000), fim)
}

/// Confere o período pedido ao satélite `s`.
pub fn conferir_periodo(s: Satelite, inicio: Data, fim: Data) -> Result<(), String> {
    if fim < inicio {
        return Err(format!("o fim ({}) vem antes do início ({})", fim.texto(), inicio.texto()));
    }
    if inicio < s.inicio() {
        return Err(format!("o cubo do {} começa em {}", s.nome(), s.inicio().texto()));
    }
    if fim.dias() - inicio.dias() > MAX_DIAS {
        return Err("o período passa de 3 anos: escolha um mais curto (o clique lê cada data de 16 dias)".into());
    }
    if fim.dias() - inicio.dias() < 15 {
        return Err("o período precisa ter pelo menos 16 dias (uma data do cubo)".into());
    }
    Ok(())
}

/// A rampa do índice, no formato do `ramp.add` (GDAL): do solo exposto ao verde da vegetação densa.
pub fn rampa(i: Indice) -> (&'static str, String) {
    let (lo, hi) = i.escala();
    let passo = (hi - lo) / 4.0;
    let cores = ["166 97 26", "223 194 125", "245 245 245", "128 205 193", "1 133 113"];
    let texto: String = cores.iter().enumerate().map(|(k, c)| format!("{} {c}\n", lo + passo * k as f64)).collect();
    ("BrasilData: índice de vegetação", texto)
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn escolhas_e_colecoes() {
        assert_eq!(Satelite::ler("Sentinel-2"), Some(Satelite::Sentinel2));
        assert_eq!(Satelite::ler("modis").map(Satelite::colecao), Some("mod13q1-6.1"));
        assert_eq!(Satelite::ler("spot"), None);
        assert_eq!(Indice::ler("evi"), Some(Indice::Evi));
        assert_eq!(Estatistica::ler("média").map(Estatistica::stat), Some("mean"));
        assert_eq!(Estatistica::ler("desvio").map(Estatistica::stat), Some("std"));
        assert!(Satelite::Landsat.citacao().contains("LANDSAT-16D-1"));
    }

    #[test]
    fn os_nomes_das_camadas_dizem_o_satelite() {
        let (a, b) = (Data { ano: 2025, mes: 6, dia: 1 }, Data { ano: 2025, mes: 8, dia: 31 });
        let n = camada(Satelite::Sentinel2, Indice::Ndvi, Estatistica::Media, a, b);
        assert_eq!(n, "BrasilData: NDVI médio de 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)");
        assert_eq!(satelite_da_camada(&n), Some(Satelite::Sentinel2));
        assert!(!eh_serie(&n));
        let s = camada_da_serie(Satelite::Cbers4, Indice::Evi, a, b);
        assert_eq!(satelite_da_camada(&s), Some(Satelite::Cbers4));
        assert!(eh_serie(&s));
        assert_eq!(satelite_da_camada("BrasilData: Altura do dossel a 1 m (Meta CHMv2)"), None);
    }

    #[test]
    fn periodo_e_leitura() {
        let col = json!({"extent": {"temporal": {"interval": [["2017-01-01T00:00:00Z", "2026-11-16T00:00:00Z"]]}}});
        let fim = fim_da_colecao(&col).unwrap();
        assert_eq!(fim, Data { ano: 2026, mes: 11, dia: 16 });
        let (i, f) = periodo_padrao(fim);
        assert_eq!((i, f), (Data { ano: 2026, mes: 8, dia: 18 }, fim));
        assert!(conferir_periodo(Satelite::Sentinel2, i, f).is_ok());
        assert!(conferir_periodo(Satelite::Sentinel2, Data { ano: 2016, mes: 1, dia: 1 }, f).is_err());
        assert!(conferir_periodo(Satelite::Landsat, f, i).is_err());
        assert!(conferir_periodo(Satelite::Landsat, Data { ano: 2020, mes: 1, dia: 1 }, f).is_err());
        assert_eq!(ler_data("01/06/2025"), ler_data("2025-06-01"));
        assert_eq!(ler_data("2025-6-1"), Some(Data { ano: 2025, mes: 6, dia: 1 }));
        assert_eq!(ler_data("2025-13-01"), None);
        assert_eq!(celula_de_leitura(Satelite::Sentinel2, 30.0), 22.5);
        assert_eq!(celula_de_leitura(Satelite::Modis, 30.0), 232.0);
        let (_, r) = rampa(Indice::Ndvi);
        assert!(r.starts_with("0 166 97 26\n") && r.contains("0.9 1 133 113"));
    }
}
