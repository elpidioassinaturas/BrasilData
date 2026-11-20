//! A altura do dossel da Amazônia da CTrees a 4,78 m: a média de 2020 a 2024, de um modelo de IA
//! (U-Net) sobre as imagens Planet do programa NICFI, calibrado com lidar aéreo da Amazônia.
//!
//! - **De onde:** 22.063 COGs no bucket público `ctrees-amazon-canopy-height` da AWS (Registry of
//!   Open Data, sem login), um por quadrante do mosaico Planet NICFI: o tile de Web Mercator de
//!   zoom 11 (cerca de 20 × 20 km), nomeado pela coluna e pela linha contada do sul
//!   (`PAGE_0571-0993_…`), com
//!   4096 × 4096 células de 4,78 m (no equador; menos ao sul), byte com a altura × 2,5, 255 sem
//!   dado, e três níveis de pirâmide. No que não é floresta amazônica não há arquivo: a listagem
//!   do bucket com o nome como prefixo diz quais há.
//! - **Licença:** a página do Registry of Open Data diz que segue o contrato do Planet NICFI; o
//!   `collection_metadata.yaml` do bucket diz CC BY 4.0. A ficha diz as duas.
//! - **Como:** de cada tile que a área toca, só a janela da área (`layer.addUrl` com `bbox`), do
//!   nível de pirâmide mais grosso cujas células no chão não passam das da grade; cada janela vai
//!   para a grade do projeto, as janelas se juntam e o mosaico, recortado pela área, passa a metros.
//! - **App web:** o bucket manda o cabeçalho CORS.

use std::f64::consts::PI;

/// Onde ficam os arquivos.
pub const BASE: &str = "https://ctrees-amazon-canopy-height.s3.us-west-2.amazonaws.com";

/// O fim do nome de cada arquivo, depois de `PAGE_xxxx-yyyy`.
const FIM: &str = "_START_2015-12-01_END_2024-06-01_height_mean.tif";

/// O zoom dos tiles.
pub const ZOOM: u32 = 11;

/// O raio da esfera do Web Mercator, em metros.
const RAIO: f64 = 6_378_137.0;

/// O lado da célula do arquivo, em metros de Web Mercator (4096 células por tile de zoom 11).
pub const LADO: f64 = 2.0 * PI * RAIO / (2048.0 * 4096.0);

/// Quantos níveis de pirâmide o arquivo tem além do inteiro.
pub const PIRAMIDES: i32 = 3;

/// O valor gravado é a altura vezes este fator.
pub const FATOR: f64 = 2.5;

/// Quantos tiles um clique lê, no máximo (o RustGIS espera até 16 downloads de uma vez).
pub const MAX_TILES: usize = 16;

/// Quantos valores uma janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Altura do dossel da Amazônia a 4,78 m (CTrees)";

/// O tile de zoom [`ZOOM`] que contém o ponto (coluna, linha).
pub fn tile_do_ponto(lon: f64, lat: f64) -> (u32, u32) {
    let n = f64::from(1u32 << ZOOM);
    let lat = lat.clamp(-85.05, 85.05).to_radians();
    let x = ((lon + 180.0) / 360.0 * n).floor().clamp(0.0, n - 1.0);
    let y = ((1.0 - lat.tan().asinh() / PI) / 2.0 * n).floor().clamp(0.0, n - 1.0);
    (x as u32, y as u32)
}

/// O retângulo do tile (coluna, linha), em graus.
pub fn retangulo_do_tile(x: u32, y: u32) -> [f64; 4] {
    let n = f64::from(1u32 << ZOOM);
    let lon = |x: f64| x / n * 360.0 - 180.0;
    let lat = |y: f64| (PI * (1.0 - 2.0 * y / n)).sinh().atan().to_degrees();
    [lon(f64::from(x)), lat(f64::from(y) + 1.0), lon(f64::from(x) + 1.0), lat(f64::from(y))]
}

/// Um tile que a área toca.
#[derive(Clone, Debug, PartialEq)]
pub struct Tile {
    pub x: u32,
    pub y: u32,
    pub retangulo: [f64; 4],
}

impl Tile {
    /// O nome do quadrante do mosaico NICFI: a coluna e a linha contada do sul (como no TMS),
    /// não do norte como a `y` do tile.
    pub fn pagina(&self) -> String {
        format!("PAGE_{:04}-{:04}", self.x, (1u32 << ZOOM) - 1 - self.y)
    }

    pub fn url(&self) -> String {
        format!("{BASE}/v1/{}{FIM}", self.pagina())
    }

    /// A listagem do bucket com o prefixo do arquivo: diz se ele existe.
    pub fn listagem(&self) -> String {
        format!("{BASE}/?list-type=2&max-keys=5&prefix=v1/{}_", self.pagina())
    }

    /// Se a `listagem` (o XML do S3) tem o arquivo.
    pub fn existe(&self, listagem: &str) -> bool {
        listagem.contains(&format!("<Key>v1/{}{FIM}</Key>", self.pagina()))
    }

    /// O nome da camada de passagem com a janela do tile, lida no nível `k`.
    pub fn nome_da_janela(&self, k: i32) -> String {
        format!("BrasilData: CTrees Amazônia {} nível {k} (janela)", self.pagina())
    }
}

/// Os tiles que o retângulo `c` toca.
pub fn tiles(c: [f64; 4]) -> Vec<Tile> {
    let (x0, y0) = tile_do_ponto(c[0], c[3]);
    let (x1, y1) = tile_do_ponto(c[2], c[1]);
    let mut v = Vec::new();
    for y in y0..=y1 {
        for x in x0..=x1 {
            let r = retangulo_do_tile(x, y);
            // Um lado sobre a divisa não pega o tile do outro lado.
            if r[0] < c[2] && r[2] > c[0] && r[1] < c[3] && r[3] > c[1] {
                v.push(Tile { x, y, retangulo: r });
            }
        }
    }
    v
}

/// O lado da célula do nível `k` da pirâmide (0 é o arquivo inteiro), em metros de Web Mercator.
pub fn lado_do_nivel(k: i32) -> f64 {
    LADO * 2f64.powi(k)
}

/// O lado, em metros no chão, das células do nível `k` na latitude `lat`.
pub fn lado_no_chao(k: i32, lat: f64) -> f64 {
    lado_do_nivel(k) * lat.to_radians().cos()
}

/// O nível mais grosso cujas células, no chão da latitude `lat`, não passam de `celula` metros.
pub fn nivel(celula: f64, lat: f64) -> i32 {
    (0..=PIRAMIDES).rev().find(|&k| lado_no_chao(k, lat) <= celula + 1e-6).unwrap_or(0)
}

/// A janela do tile para o retângulo `c`: a parte dele dentro do tile, com três células de `lado`
/// metros (de Web Mercator) de margem; `None` se ele não o toca.
pub fn janela(t: &Tile, c: [f64; 4], lado: f64) -> Option<[f64; 4]> {
    let r = t.retangulo;
    let (o, s, l, n) = (c[0].max(r[0]), c[1].max(r[1]), c[2].min(r[2]), c[3].min(r[3]));
    if o >= l || s >= n {
        return None;
    }
    let m = 3.0 * lado * ((s + n) / 2.0).to_radians().cos() / 111_320.0;
    Some([o - m, s - m, l + m, n + m])
}

/// O que a janela lida do tile `t` precisa cobrir para servir à área `c`: a parte de `c` dentro
/// do tile (a janela pedida passa da borda pela margem, mas o arquivo acaba nela).
pub fn a_cobrir(t: &Tile, c: [f64; 4]) -> [f64; 4] {
    let r = t.retangulo;
    [c[0].max(r[0]), c[1].max(r[1]), c[2].min(r[2]), c[3].min(r[3])]
}

/// Quantos valores a janela `j` tem lida com células de `lado` metros de Web Mercator.
pub fn valores(j: [f64; 4], lado: f64) -> f64 {
    let y = |lat: f64| RAIO * lat.to_radians().tan().asinh();
    let largura = (j[2] - j[0]).to_radians() * RAIO / lado;
    let altura = (y(j[3]) - y(j[1])) / lado;
    largura * altura
}

/// O nível mais fino, a partir de `k`, com que todas as `janelas` cabem no limite.
pub fn nivel_que_cabe(janelas: &[[f64; 4]], k: i32) -> Option<i32> {
    (k..=PIRAMIDES).find(|&n| janelas.iter().all(|j| valores(*j, lado_do_nivel(n)) <= MAX_VALORES))
}

/// A reamostragem para a grade: bilinear até a célula lida, média numa grade mais grossa.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "average" } else { "bilinear" }
}

/// A citação, como a licença pede.
pub const CITACAO: &str = "Wagner, F. H. et al. (2025), High Resolution Tree Height Mapping of the Amazon Forest using Planet NICFI Images and LiDAR-Informed U-Net Model, Remote Sensing of Environment (arXiv:2501.10600); CTrees.org (2025), Canopy Tree Height of the Amazon Forest, média de 2020 a 2024 a 4,78 m, lida do bucket público ctrees-amazon-canopy-height (AWS Open Data), posta na grade do projeto e recortada pela área";

pub const LICENCA: &str = "CC BY 4.0 segundo os metadados do bucket; a página do AWS Open Data diz que segue o contrato do Planet NICFI";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_tile_e_o_nome() {
        // O arquivo PAGE_0571-0993 começa em 79°37'44" O, 5°15'57,6" S (gdalinfo).
        // A linha 993 contada do sul é a 1054 contada do norte.
        let r = retangulo_do_tile(571, 1054);
        assert!((r[0] - -(79.0 + 37.0 / 60.0 + 44.06 / 3600.0)).abs() < 1e-4, "{r:?}");
        assert!((r[3] - -(5.0 + 15.0 / 60.0 + 57.63 / 3600.0)).abs() < 1e-4, "{r:?}");
        assert_eq!(tile_do_ponto(-79.55, -5.3), (571, 1054));
        let t = Tile { x: 571, y: 1054, retangulo: r };
        assert_eq!(t.url(), format!("{BASE}/v1/PAGE_0571-0993_START_2015-12-01_END_2024-06-01_height_mean.tif"));
        assert!(t.listagem().ends_with("&prefix=v1/PAGE_0571-0993_"));
        let xml = "<ListBucketResult><Contents><Key>v1/PAGE_0571-0993_START_2015-12-01_END_2024-06-01_height_mean.tif</Key></Contents></ListBucketResult>";
        assert!(t.existe(xml));
        assert!(!t.existe("<ListBucketResult><KeyCount>0</KeyCount></ListBucketResult>"));
    }

    #[test]
    fn os_tiles_e_os_niveis() {
        assert!((LADO - 4.777_314_267_16).abs() < 1e-6);
        // Uma área de uns 30 × 30 km em Rondônia toca de 4 a 9 tiles.
        let ts = tiles([-62.4, -10.8, -62.1, -10.5]);
        assert!((4..=9).contains(&ts.len()), "{}", ts.len());
        // A 10° S, as células do arquivo têm 4,7 m no chão.
        assert_eq!(nivel(5.0, -10.0), 0);
        assert_eq!(nivel(30.0, -10.0), 2);
        assert_eq!(nivel(500.0, -10.0), PIRAMIDES);
        let t = &tiles([-62.3, -10.7, -62.29, -10.69])[0];
        let j = janela(t, t.retangulo, LADO).unwrap();
        // Um tile inteiro no arquivo inteiro: 4096² valores, cabe.
        assert!(valores(j, LADO) < MAX_VALORES);
        let a = a_cobrir(t, [-63.0, -11.0, -62.0, -10.0]);
        assert_eq!(a, t.retangulo);
        assert_eq!(reamostragem(25.0, 18.8), "bilinear");
        assert_eq!(reamostragem(100.0, 37.6), "average");
    }
}
