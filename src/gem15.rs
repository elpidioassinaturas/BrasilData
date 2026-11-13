//! Os modelos de elevação do Google a 15 m (GEM-15, Google Research, experimental): a superfície
//! (DSM), o terreno nu (DTM) e, da diferença, a altura do que está sobre o terreno (dossel,
//! prédios).
//!
//! - **De onde:** COGs públicos no Google Cloud Storage (`climate-and-sustainability/gem-15`, sem
//!   login, licença CC-BY 4.0), um por célula de 1° × 1° e por modelo, no UTM WGS 84 do fuso da
//!   célula, a 15 m, float32, sem dado NaN, com pirâmides de 30, 60, 120 e 240 m. O DSM tem cinco
//!   bandas (a superfície, os percentis 5, 50 e 95 e o desvio padrão dos pixels de 0,5 m de cada
//!   célula de 15 m); o DTM, uma.
//! - **As células:** a pasta da célula leva o canto sudoeste em graus inteiros (`wm43sm21`: oeste
//!   -43, sul -21, com `m` nos negativos) e fica no bloco de 10° que a contém (`m30_m50`, com `m`
//!   ou `p`). O catálogo STAC de cada bloco lista as células que existem: no mar, fora da
//!   cobertura (Rússia, China, Mongólia, Groenlândia, Antártica) ou sem imagem, não há arquivo.
//! - **Como:** de cada célula que a área toca, só a janela da área é lida (`layer.addUrl` com
//!   `bbox`, depois da 0.5.0), da pirâmide mais fina que não é mais fina que a grade. Cada janela
//!   vai para a grade do projeto, as janelas se juntam e o mosaico é recortado pela área.
//! - **Cuidados:** alturas ortométricas no geoide EGM96 (o Copernicus usa o EGM2008); água (com 9 m
//!   de margem), neve, salinas e superfícies sem textura ficam sem dado; o DTM é pior sob mata
//!   densa, onde o chão não se vê; e há desvios regionais de alguns metros (de -4,8 a +1,9 m em
//!   relação ao lidar, no artigo).

/// Onde ficam os arquivos.
pub const BASE: &str = "https://storage.googleapis.com/climate-and-sustainability/gem-15";

/// O lado do pixel do arquivo, em metros, e os das pirâmides.
pub const NIVEIS: [f64; 5] = [15.0, 30.0, 60.0, 120.0, 240.0];

/// Quantas células de 1° um clique lê, no máximo.
pub const MAX_CELULAS: usize = 12;

/// Quantos valores uma janela pode ter (todas as bandas): o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

/// Um metro em graus de latitude.
const GRAU_M: f64 = 111_320.0;

/// O que o comando traz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Modelo {
    /// O terreno nu (DTM).
    Terreno,
    /// A superfície, com a vegetação e as construções (DSM).
    Superficie,
    /// A altura acima do terreno: DSM menos DTM.
    Altura,
}

impl Modelo {
    pub const TODOS: [Modelo; 3] = [Modelo::Terreno, Modelo::Superficie, Modelo::Altura];

    /// O valor do parâmetro `modelo`.
    pub fn valor(self) -> &'static str {
        match self {
            Modelo::Terreno => "dtm",
            Modelo::Superficie => "dsm",
            Modelo::Altura => "altura",
        }
    }

    /// O modelo do parâmetro `modelo` (sem ele, o terreno).
    pub fn do_valor(v: Option<&str>) -> Result<Modelo, String> {
        match v.map(str::trim) {
            None | Some("") => Ok(Modelo::Terreno),
            Some(t) => Self::TODOS
                .into_iter()
                .find(|m| m.valor().eq_ignore_ascii_case(t))
                .ok_or(format!("o modelo do GEM-15 é \"dtm\" (terreno), \"dsm\" (superfície) ou \"altura\" (DSM menos DTM), não \"{t}\"")),
        }
    }

    /// Os arquivos que ele lê.
    pub fn arquivos(self) -> &'static [Arquivo] {
        match self {
            Modelo::Terreno => &[Arquivo::Dtm],
            Modelo::Superficie => &[Arquivo::Dsm],
            Modelo::Altura => &[Arquivo::Dsm, Arquivo::Dtm],
        }
    }

    /// O nome da camada.
    pub fn camada(self) -> &'static str {
        match self {
            Modelo::Terreno => "BrasilData: Terreno (GEM-15 DTM)",
            Modelo::Superficie => "BrasilData: Superfície (GEM-15 DSM)",
            Modelo::Altura => "BrasilData: Altura acima do terreno (GEM-15)",
        }
    }

    /// O que é, para os avisos e o layout.
    pub fn titulo(self) -> &'static str {
        match self {
            Modelo::Terreno => "Elevação do terreno nu",
            Modelo::Superficie => "Elevação da superfície (com vegetação e construções)",
            Modelo::Altura => "Altura acima do terreno (vegetação e construções)",
        }
    }
}

/// Um dos dois arquivos de cada célula.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Arquivo {
    Dsm,
    Dtm,
}

impl Arquivo {
    fn pasta(self) -> &'static str {
        match self {
            Arquivo::Dsm => "dsm",
            Arquivo::Dtm => "dtm",
        }
    }

    /// Quantas bandas ele tem.
    pub fn bandas(self) -> f64 {
        match self {
            Arquivo::Dsm => 5.0,
            Arquivo::Dtm => 1.0,
        }
    }
}

/// Um grau inteiro no nome de uma célula: `m21` para -21, `0`, `42`.
fn no_nome(g: i32) -> String {
    if g < 0 { format!("m{}", -g) } else { g.to_string() }
}

/// Um grau inteiro no nome de um bloco: `m30`, `p0`, `p10`.
fn no_bloco(g: i32) -> String {
    if g < 0 { format!("m{}", -g) } else { format!("p{g}") }
}

/// Uma célula de 1°, pelo canto sudoeste.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Celula {
    pub oeste: i32,
    pub sul: i32,
}

impl Celula {
    /// O nome da pasta (`wm43sm21`).
    pub fn nome(self) -> String {
        format!("w{}s{}", no_nome(self.oeste), no_nome(self.sul))
    }

    /// O bloco de 10° que a contém (`m30_m50`: sul, depois oeste).
    pub fn bloco(self) -> String {
        format!("{}_{}", no_bloco(self.sul.div_euclid(10) * 10), no_bloco(self.oeste.div_euclid(10) * 10))
    }

    /// O endereço do arquivo.
    pub fn url(self, a: Arquivo) -> String {
        format!("{BASE}/{}/{}/{}/zall/mosaic.tif", self.bloco(), self.nome(), a.pasta())
    }

    /// O retângulo dela, em graus.
    pub fn retangulo(self) -> [f64; 4] {
        [f64::from(self.oeste), f64::from(self.sul), f64::from(self.oeste) + 1.0, f64::from(self.sul) + 1.0]
    }
}

/// O catálogo STAC do bloco `bloco`, que lista as células que existem.
pub fn catalogo(bloco: &str) -> String {
    format!("{BASE}/stac/gem-15/{bloco}/catalog.json")
}

/// Os nomes das células que o catálogo `c` de um bloco lista (o título de cada item).
pub fn celulas_do_catalogo(c: &serde_json::Value) -> Vec<String> {
    c["links"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| matches!(l["rel"].as_str(), Some("item" | "child")))
        .filter_map(|l| l["title"].as_str().map(str::to_string))
        .collect()
}

/// As células de 1° que o retângulo `c` toca.
pub fn celulas(c: [f64; 4]) -> Vec<Celula> {
    let (o, s) = (c[0].floor() as i32, c[1].floor() as i32);
    // Um lado em grau inteiro não toca a célula seguinte.
    let (l, n) = ((c[2].ceil() as i32 - 1).max(o), (c[3].ceil() as i32 - 1).max(s));
    let mut v = Vec::new();
    for sul in s..=n {
        for oeste in o..=l {
            v.push(Celula { oeste, sul });
        }
    }
    v
}

/// Os blocos dessas células, sem repetir.
pub fn blocos(cs: &[Celula]) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for c in cs {
        let b = c.bloco();
        if !v.contains(&b) {
            v.push(b);
        }
    }
    v
}

/// O lado do pixel lido para uma grade de células de `celula` metros: o da pirâmide mais grossa
/// que não passa da célula (a grade mais fina que 15 m lê os 15 m).
pub fn nivel(celula: f64) -> f64 {
    NIVEIS.into_iter().rev().find(|n| *n <= celula + 1e-6).unwrap_or(NIVEIS[0])
}

/// A janela da célula `cel` para o retângulo `c`: a parte dele dentro dela, com três pixels de
/// `lado` metros de margem (os vizinhos da interpolação); `None` se ele não a toca.
pub fn janela(cel: Celula, c: [f64; 4], lado: f64) -> Option<[f64; 4]> {
    let r = cel.retangulo();
    let (o, s, l, n) = (c[0].max(r[0]), c[1].max(r[1]), c[2].min(r[2]), c[3].min(r[3]));
    if o >= l || s >= n {
        return None;
    }
    let m = 3.0 * lado / GRAU_M;
    Some([o - m, s - m, l + m, n + m])
}

/// Quantos valores (todas as bandas) a janela `j` tem lida a `lado` metros do arquivo `a`.
pub fn valores(j: [f64; 4], lado: f64, a: Arquivo) -> f64 {
    let lat = ((j[1] + j[3]) / 2.0).to_radians().cos();
    let largura = (j[2] - j[0]) * GRAU_M * lat / lado;
    let altura = (j[3] - j[1]) * GRAU_M / lado;
    largura * altura * a.bandas()
}

/// A menor célula de grade (das pirâmides) com que todas as `janelas` cabem no limite.
pub fn celula_que_cabe(janelas: &[[f64; 4]], a: Arquivo) -> Option<f64> {
    NIVEIS.into_iter().find(|&n| janelas.iter().all(|j| valores(*j, n, a) <= MAX_VALORES))
}

/// A reamostragem para a grade: bilinear até o pixel lido, média numa grade mais grossa.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "average" } else { "bilinear" }
}

/// O nome da camada de passagem com a janela do arquivo `a` da célula, lida a `lado` metros.
pub fn nome_da_janela(cel: Celula, a: Arquivo, lado: f64) -> String {
    format!("BrasilData: GEM-15 {} {} {lado} m (janela)", a.pasta(), cel.nome())
}

/// Se a camada `nome` é do GEM-15.
pub fn eh_do_gem15(nome: &str) -> bool {
    Modelo::TODOS.iter().any(|m| m.camada() == nome)
}

/// A citação, como o Google pede (CC-BY 4.0).
pub const CITACAO: &str = "Google Research, GEM-15: Near-Global Surface and Terrain Elevation Models at 15 m, v1 (experimental), a partir de imagens estéreo ópticas submétricas adquiridas até 2024 (preprint no EarthArXiv, doi:10.31223/X55N75); alturas ortométricas no geoide EGM96; lido do Google Cloud Storage (gs://climate-and-sustainability/gem-15), posto na grade do projeto e recortado pela área";

pub const LICENCA: &str = "CC-BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomes_das_celulas_e_dos_blocos() {
        let v = Celula { oeste: -43, sul: -21 };
        assert_eq!((v.nome(), v.bloco()), ("wm43sm21".into(), "m30_m50".into()));
        assert_eq!(v.url(Arquivo::Dtm), format!("{BASE}/m30_m50/wm43sm21/dtm/zall/mosaic.tif"));
        // Os nomes vistos nos catálogos: wm41sm10 em m10_m50, wm50s0 em p0_m50, w10s42 em p40_p10.
        assert_eq!(Celula { oeste: -41, sul: -10 }.bloco(), "m10_m50");
        let e = Celula { oeste: -50, sul: 0 };
        assert_eq!((e.nome(), e.bloco()), ("wm50s0".into(), "p0_m50".into()));
        let n = Celula { oeste: 10, sul: 42 };
        assert_eq!((n.nome(), n.bloco()), ("w10s42".into(), "p40_p10".into()));
        assert_eq!(Celula { oeste: -1, sul: -1 }.bloco(), "m10_m10");
        assert_eq!(catalogo("m30_m50"), format!("{BASE}/stac/gem-15/m30_m50/catalog.json"));
    }

    #[test]
    fn as_celulas_de_um_retangulo() {
        // Viçosa, numa célula só.
        assert_eq!(celulas([-42.95, -20.8, -42.82, -20.7]), vec![Celula { oeste: -43, sul: -21 }]);
        // Dois graus de longitude: duas células.
        assert_eq!(celulas([-43.2, -20.8, -42.5, -20.7]).len(), 2);
        // Um lado no grau inteiro não pega a célula seguinte.
        assert_eq!(celulas([-43.0, -21.0, -42.0, -20.0]).len(), 1);
        assert_eq!(blocos(&celulas([-40.5, -20.5, -39.5, -19.5])), vec!["m30_m50", "m30_m40", "m20_m50", "m20_m40"]);
    }

    #[test]
    fn a_janela_fica_na_celula() {
        let v = Celula { oeste: -43, sul: -21 };
        let j = janela(v, [-43.2, -20.8, -42.5, -20.7], 15.0).unwrap();
        assert!((j[0] - (-43.0 - 45.0 / GRAU_M)).abs() < 1e-9 && (j[2] - (-42.5 + 45.0 / GRAU_M)).abs() < 1e-9);
        assert_eq!(janela(v, [-44.0, -20.8, -43.5, -20.7], 15.0), None);
    }

    #[test]
    fn o_nivel_e_os_limites() {
        assert_eq!(nivel(10.0), 15.0);
        assert_eq!(nivel(30.0), 30.0);
        assert_eq!(nivel(100.0), 60.0);
        assert_eq!(nivel(1000.0), 240.0);
        assert_eq!(reamostragem(30.0, 30.0), "bilinear");
        assert_eq!(reamostragem(100.0, 60.0), "average");
        // Uma célula inteira do DTM a 15 m cabe (cerca de 52 milhões de valores); a do DSM, com 5
        // bandas, só a 60 m.
        let inteira = [-43.0, -21.0, -42.0, -20.0];
        assert_eq!(celula_que_cabe(&[inteira], Arquivo::Dtm), Some(15.0));
        assert!(valores(inteira, 30.0, Arquivo::Dsm) > MAX_VALORES);
        assert_eq!(celula_que_cabe(&[inteira], Arquivo::Dsm), Some(60.0));
    }

    #[test]
    fn o_modelo_do_parametro() {
        assert_eq!(Modelo::do_valor(None), Ok(Modelo::Terreno));
        assert_eq!(Modelo::do_valor(Some("DSM")), Ok(Modelo::Superficie));
        assert_eq!(Modelo::do_valor(Some("altura")), Ok(Modelo::Altura));
        assert!(Modelo::do_valor(Some("lidar")).is_err());
        assert!(eh_do_gem15(Modelo::Altura.camada()));
    }

    #[test]
    fn as_celulas_do_catalogo() {
        let c = serde_json::json!({"links": [
            {"rel": "root", "href": "../catalog.json"},
            {"rel": "child", "href": "./wm43sm21/wm43sm21.json", "title": "wm43sm21"},
            {"rel": "item", "href": "./wm42sm21/wm42sm21.json", "title": "wm42sm21"},
        ]});
        assert_eq!(celulas_do_catalogo(&c), vec!["wm43sm21", "wm42sm21"]);
    }
}
