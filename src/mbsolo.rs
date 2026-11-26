//! Os pontos de treino do MapBiomas Solo (Coleção 3, beta): as amostras de solo com que o
//! MapBiomas treinou os modelos dos mapas de carbono e de textura do solo, publicadas por ele no
//! SoilData (UTFPR e MapBiomas).
//!
//! - **Carbono:** o estoque de carbono orgânico de 0 a 30 cm de cada ponto, com o ano da coleta, do
//!   arquivo de modelagem (`c3_soildata_soc_modeling.tab`, 2,2 MB). Ele traz, por ponto, o estoque
//!   somado da superfície até o fim de cada camada (g/m²); fica a linha que vai até 30 cm, com a
//!   correção por quantis que o MapBiomas aplicou às amostras do Inventário Florestal Nacional
//!   (`soc_stock_gm2_cum_qmap`, igual à outra nas demais). Os pontos só até 20 cm ou menos ficam de
//!   fora, e também as 3.000 pseudoamostras (afloramentos de rocha e areias, com estoque 0) que o
//!   modelo usou e que não são coletas. Em 26/11/2026: 11.598 pontos de 1985 a 2024.
//! - **Textura:** a argila, a areia, o silte e a fração grossa (g/kg) de cada camada, do arquivo da
//!   plataforma (`c3_soildata_psd_platform.tab`, 10,4 MB), que traz a licença e o conjunto de origem
//!   de cada camada (o de modelagem tem cópias e pseudoamostras); a camada que contém a
//!   profundidade escolhida, de 0 a 100 cm.
//! - **Coordenadas com erro:** no arquivo do carbono, 657 linhas perderam o ponto decimal de uma
//!   coordenada (-29782 em vez de -29,782; 328 dos pontos de 0 a 30 cm). O RustGIS 0.5.0 as põe sem
//!   geometria; o 0.8.0 as põe no mapa, e o recorte recusa uma latitude além de 90°: elas saem
//!   antes ([`coordenadas_validas`]). O arquivo da plataforma tem as mesmas amostras certas.
//! - **Quem faz o trabalho:** o RustGIS, como nos [pontos do SoilData](crate::soildata): abre o
//!   arquivo pelo endereço (`x`, `y` e `delimiter`), recorta, seleciona e calcula.

use serde_json::{Value, json};

use crate::PREFIXO;
use crate::soildata::{self, Classe};

/// O endereço de um arquivo no Dataverse do SoilData, pelo número.
fn url(numero: u32) -> String {
    format!("https://repositorio.soildata.mapbiomas.org/api/access/datafile/{numero}")
}

/// O arquivo de modelagem do carbono (doi:10.60502/SoilData/IUZOAK).
const ARQUIVO_CARBONO: u32 = 862;

/// O arquivo da plataforma da textura (doi:10.60502/SoilData/OXSR2N).
const ARQUIVO_TEXTURA: u32 = 858;

/// As profundidades da textura, em cm: a camada que contém a profundidade (as camadas vão até 100
/// cm, e nenhuma começa nele).
pub const PROFUNDIDADES: [u32; 5] = [5, 10, 30, 50, 90];

/// A profundidade sem o parâmetro.
pub const PROFUNDIDADE_PADRAO: u32 = 10;

/// O campo da classe, pelo qual a tabela diz a faixa de cada ponto.
pub const CAMPO_CLASSE: &str = soildata::CAMPO_CLASSE;

/// O campo do estoque de carbono, em t/ha.
pub const CAMPO_CARBONO: &str = "carbono_0_30_tha";

/// O que o comando traz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dado {
    /// O estoque de carbono orgânico de 0 a 30 cm (t/ha), um ponto por coleta.
    Carbono,
    Argila,
    Areia,
    Silte,
    /// Cascalho e calhaus (mais de 2 mm).
    FracaoGrossa,
}

impl Dado {
    /// Todos, na ordem da janela.
    pub const TODOS: [Dado; 5] = [Dado::Carbono, Dado::Argila, Dado::Areia, Dado::Silte, Dado::FracaoGrossa];

    /// O valor do parâmetro `dado`.
    pub fn valor(self) -> &'static str {
        match self {
            Dado::Carbono => "carbono",
            Dado::Argila => "argila",
            Dado::Areia => "areia",
            Dado::Silte => "silte",
            Dado::FracaoGrossa => "fracao-grossa",
        }
    }

    /// O dado do parâmetro `dado` (sem ele, o carbono).
    pub fn ler(t: Option<&str>) -> Result<Dado, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Dado::Carbono);
        }
        Dado::TODOS.into_iter().find(|d| d.valor() == t).ok_or_else(|| {
            let todos: Vec<&str> = Dado::TODOS.iter().map(|d| d.valor()).collect();
            format!("dado '{t}': use {}", todos.join(", "))
        })
    }

    /// O nome, para a camada.
    pub fn nome(self) -> &'static str {
        match self {
            Dado::Carbono => "Carbono orgânico de 0 a 30 cm",
            Dado::Argila => "Argila",
            Dado::Areia => "Areia",
            Dado::Silte => "Silte",
            Dado::FracaoGrossa => "Fração grossa",
        }
    }

    /// O campo numérico da camada: o calculado no carbono, o do arquivo na textura.
    pub fn campo(self) -> &'static str {
        match self {
            Dado::Carbono => CAMPO_CARBONO,
            Dado::Argila => "fracao_argila_gkg",
            Dado::Areia => "fracao_areia_gkg",
            Dado::Silte => "fracao_silte_gkg",
            Dado::FracaoGrossa => "fracao_grossa_gkg",
        }
    }

    /// Se é da textura (o arquivo da plataforma, com profundidade).
    pub fn textura(self) -> bool {
        self != Dado::Carbono
    }

    /// As faixas de valores, do menor ao maior.
    fn classes(self) -> &'static [Classe] {
        const INF: f64 = f64::INFINITY;
        match self {
            Dado::Carbono => &[
                (20.0, "menos de 20 t/ha", "#F6EBD9"),
                (40.0, "20 a 40 t/ha", "#E2C79A"),
                (60.0, "40 a 60 t/ha", "#C49A62"),
                (90.0, "60 a 90 t/ha", "#9A6B3B"),
                (150.0, "90 a 150 t/ha", "#6B4423"),
                (INF, "150 t/ha ou mais", "#3B2412"),
            ],
            Dado::Argila => soildata::Atributo::Argila.classes(),
            Dado::Areia => soildata::Atributo::Areia.classes(),
            Dado::Silte => soildata::Atributo::Silte.classes(),
            // Os valores são inteiros: abaixo de 1 é zero.
            Dado::FracaoGrossa => &[
                (1.0, "sem cascalho (0 g/kg)", "#F7F7F7"),
                (50.0, "1 a 50 g/kg", "#CCCCCC"),
                (150.0, "50 a 150 g/kg", "#969696"),
                (500.0, "150 a 500 g/kg", "#636363"),
                (INF, "500 g/kg ou mais", "#252525"),
            ],
        }
    }
}

/// A profundidade do parâmetro `profundidade` (texto ou número, em cm; sem ele, 10 cm).
pub fn ler_profundidade(texto: Option<&str>, numero: Option<f64>) -> Result<u32, String> {
    let t = texto.map(str::trim).unwrap_or("");
    let d = match (t.is_empty(), numero) {
        (false, _) => t.trim_end_matches("cm").trim().parse::<f64>().ok(),
        (true, Some(n)) => Some(n),
        (true, None) => return Ok(PROFUNDIDADE_PADRAO),
    };
    match d {
        Some(d) if d.fract() == 0.0 && PROFUNDIDADES.contains(&(d as u32)) => Ok(d as u32),
        _ => {
            let lista = PROFUNDIDADES.map(|p| p.to_string()).join(", ");
            let dado = if t.is_empty() { numero.map(|n| n.to_string()).unwrap_or_default() } else { t.to_string() };
            Err(format!("profundidade '{dado}': use {lista} (cm)"))
        }
    }
}

/// A camada com o arquivo inteiro, que sai depois do recorte.
pub fn bruto(d: Dado) -> &'static str {
    if d.textura() { "BrasilData: MapBiomas Solo, textura (sem recorte)" } else { "BrasilData: MapBiomas Solo, carbono (sem recorte)" }
}

/// O pedido do arquivo ao `layer.addUrl`, com as colunas das coordenadas e o separador.
pub fn pedido(d: Dado) -> Value {
    let (numero, x, y) =
        if d.textura() { (ARQUIVO_TEXTURA, "longitude_grau", "latitude_grau") } else { (ARQUIVO_CARBONO, "coord_x", "coord_y") };
    json!({"url": url(numero), "name": bruto(d), "x": x, "y": y, "delimiter": "\t", "crs": "EPSG:4326", "zoom": false})
}

/// O nome da camada.
pub fn camada(d: Dado, prof: u32) -> String {
    match d {
        Dado::Carbono => format!("{PREFIXO}{}, pontos de treino (MapBiomas Solo)", d.nome()),
        _ => format!("{PREFIXO}{} a {prof} cm, pontos de treino (MapBiomas Solo)", d.nome()),
    }
}

/// Se `nome` é uma camada deste comando (para a tabela de fontes).
pub fn eh_daqui(nome: &str) -> bool {
    nome.starts_with(PREFIXO) && nome.ends_with(", pontos de treino (MapBiomas Solo)")
}

/// Se a camada `nome` é a do carbono (a outra citação).
pub fn eh_do_carbono(nome: &str) -> bool {
    nome == camada(Dado::Carbono, 0)
}

/// As linhas que ficam: no carbono, o estoque até 30 cm das coletas (sem as pseudoamostras); na
/// textura, a camada que contém a profundidade `prof`.
pub fn filtro(d: Dado, prof: u32) -> String {
    match d {
        Dado::Carbono => "\"profund_inf\" = 30 AND NOT (\"id\" LIKE '%pseudo%')".into(),
        _ => format!("\"profundidade_inicial_cm\" <= {prof} AND \"profundidade_final_cm\" > {prof}"),
    }
}

/// As linhas com coordenadas em graus, antes do recorte: no arquivo do carbono, as que perderam o
/// ponto decimal saem (o RustGIS 0.8.0 as põe no mapa, e o recorte recusa a camada).
pub fn coordenadas_validas(d: Dado) -> Option<String> {
    (!d.textura()).then(|| "\"coord_x\" >= -180 AND \"coord_x\" <= 180 AND \"coord_y\" >= -90 AND \"coord_y\" <= 90".to_string())
}

/// Os campos novos: (nome, tipo, expressão), na ordem em que se calculam. O estoque vem em g/m²
/// (1 g/m² = 0,01 t/ha).
pub fn campos(d: Dado) -> Vec<(&'static str, &'static str, String)> {
    let mut v = Vec::new();
    if d == Dado::Carbono {
        v.push((CAMPO_CARBONO, "double", "\"soc_stock_gm2_cum_qmap\" / 100.0".to_string()));
    }
    v.push((CAMPO_CLASSE, "text", soildata::expressao_faixas(d.campo(), d.classes())));
    v
}

/// Os campos do arquivo que ficam, além do valor e da classe.
const FICAM_CARBONO: [&str; 3] = ["id", "data_ano", "IFN_index"];

const FICAM_TEXTURA: [&str; 10] = [
    "dataset_id",
    "observacao_id",
    "profundidade_inicial_cm",
    "profundidade_final_cm",
    "bioma_nome",
    "estado_nome",
    "municipio_nome",
    "dataset_titulo",
    "dataset_licenca",
    "ano_da_coleta",
];

/// Se o campo `c` fica na camada de `d`.
pub fn fica_o_campo(d: Dado, c: &str) -> bool {
    let ficam: &[&str] = if d.textura() { &FICAM_TEXTURA } else { &FICAM_CARBONO };
    ficam.contains(&c) || c == CAMPO_CLASSE || c == d.campo()
}

/// As linhas de conjuntos sem uso comercial (só a textura diz a licença de cada uma).
pub fn nao_comercial(d: Dado) -> Option<&'static str> {
    d.textura().then_some(soildata::NAO_COMERCIAL)
}

/// O nome do estilo.
pub fn nome_do_estilo(d: Dado) -> String {
    format!("{PREFIXO}MapBiomas Solo ({})", d.valor())
}

/// O estilo graduado do dado.
pub fn estilo(d: Dado) -> String {
    soildata::estilo_graduado(&nome_do_estilo(d), d.campo(), d.classes())
}

/// A citação do carbono, como o SoilData a dá.
pub const CITACAO_CARBONO: &str = "MapBiomas (2026), Training Field Soil Data for Annual Mapping of Soil Organic Carbon Stock (0–30 cm) in Brazil, 1985–2024 (MapBiomas Soil Collection 3, Beta Version), SoilData, doi:10.60502/SoilData/IUZOAK";

/// A citação da textura.
pub const CITACAO_TEXTURA: &str = "MapBiomas (2026), Training Field Soil Data for Mapping of Soil Particle Size Distribution (0-100 cm) in Brazil (MapBiomas Soil Collection 3, Beta Version), SoilData, doi:10.60502/SoilData/OXSR2N; cada camada vem de um conjunto do SoilData (campos dataset_id e dataset_titulo)";

/// A licença do carbono: a do conjunto no SoilData.
pub const LICENCA_CARBONO: &str = "CC BY 4.0";

/// A licença da textura: a do conjunto, e a de cada camada.
pub const LICENCA_TEXTURA: &str = "CC BY 4.0 (o conjunto); cada camada tem a licença do conjunto de origem no campo dataset_licenca: CC BY ou CC BY-NC (sem uso comercial)";

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_dado_e_a_profundidade() {
        assert_eq!(Dado::ler(None).unwrap(), Dado::Carbono);
        assert_eq!(Dado::ler(Some("fracao-grossa")).unwrap(), Dado::FracaoGrossa);
        assert!(Dado::ler(Some("ph")).unwrap_err().contains("carbono, argila"));
        assert_eq!(ler_profundidade(None, None).unwrap(), 10);
        assert_eq!(ler_profundidade(Some("90 cm"), None).unwrap(), 90);
        assert!(ler_profundidade(Some("100"), None).unwrap_err().contains("5, 10, 30, 50, 90"));
    }

    #[test]
    fn o_carbono_e_o_estoque_ate_30_cm_das_coletas() {
        assert_eq!(filtro(Dado::Carbono, 10), "\"profund_inf\" = 30 AND NOT (\"id\" LIKE '%pseudo%')");
        assert_eq!(filtro(Dado::Argila, 30), "\"profundidade_inicial_cm\" <= 30 AND \"profundidade_final_cm\" > 30");
        let c = campos(Dado::Carbono);
        assert_eq!(c[0], (CAMPO_CARBONO, "double", "\"soc_stock_gm2_cum_qmap\" / 100.0".to_string()));
        assert!(c[1].2.starts_with("CASE WHEN \"carbono_0_30_tha\" < 20.0 THEN 'menos de 20 t/ha'"));
        assert_eq!(campos(Dado::Silte).len(), 1);
        assert_eq!(pedido(Dado::Carbono)["x"], "coord_x");
        assert_eq!(pedido(Dado::Areia)["url"], url(858));
        assert_eq!(nao_comercial(Dado::Carbono), None);
        assert!(coordenadas_validas(Dado::Carbono).unwrap().contains("\"coord_y\" <= 90"));
        assert_eq!(coordenadas_validas(Dado::Argila), None);
    }

    #[test]
    fn os_campos_que_ficam() {
        assert!(fica_o_campo(Dado::Carbono, "data_ano"));
        assert!(fica_o_campo(Dado::Carbono, CAMPO_CARBONO));
        assert!(!fica_o_campo(Dado::Carbono, "soc_stock_gm2_cum"));
        assert!(fica_o_campo(Dado::Argila, "fracao_argila_gkg"));
        assert!(!fica_o_campo(Dado::Argila, "fracao_silte_gkg"));
        assert!(fica_o_campo(Dado::Argila, "dataset_licenca"));
    }

    #[test]
    fn os_nomes_e_o_estilo() {
        assert_eq!(camada(Dado::Carbono, 10), "BrasilData: Carbono orgânico de 0 a 30 cm, pontos de treino (MapBiomas Solo)");
        assert_eq!(camada(Dado::Argila, 30), "BrasilData: Argila a 30 cm, pontos de treino (MapBiomas Solo)");
        assert!(eh_daqui(&camada(Dado::Silte, 5)));
        assert!(eh_do_carbono(&camada(Dado::Carbono, 5)));
        assert!(!eh_do_carbono(&camada(Dado::Argila, 5)));
        assert!(!eh_daqui(bruto(Dado::Carbono)));
        let s: Value = serde_json::from_str(&estilo(Dado::Carbono)).unwrap();
        let g = &s[0]["renderer"]["Graduated"];
        assert_eq!(g["field"], CAMPO_CARBONO);
        assert_eq!(g["class_labels"]["5"], "150 t/ha ou mais");
        assert!(g["breaks"][0].as_f64().unwrap() < 20.0);
    }
}
