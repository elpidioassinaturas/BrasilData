//! A altura do dossel da Meta e do World Resources Institute (Canopy Height Maps v2, CHMv2): a
//! altura das árvores a cerca de 1 m, estimada por um modelo (DINOv3) sobre imagens de satélite de
//! alta resolução.
//!
//! - **De onde:** COGs públicos num bucket S3 da Meta (`dataforgood-fb-data`, sem login, licença
//!   CC BY 4.0), um por tile de Web Mercator de zoom 10, nomeado pelo quadkey (`2110222310`):
//!   EPSG:3857, células de 1,19 m (no equador; menos ao sul e ao norte), byte (a altura em metros
//!   inteiros), com sete níveis de pirâmide que são médias. Um tile sem dado (mar) não existe: a
//!   listagem do bucket (`list-type=2` com o quadkey como prefixo, uns 600 bytes) diz quais há.
//!   O GeoJSON de cada tile com as datas das imagens tem de 9 a 13 MB, mais que o `host.fetch`
//!   aceita (2 MB): as datas ficam de fora.
//! - **O zero:** é tanto "sem árvore" quanto "sem dado" (a máscara do arquivo os separa, mas o
//!   RustGIS não a lê): entra como altura 0, o que está certo dentro da cobertura.
//! - **Como:** de cada tile que a área toca, só a janela da área é lida (`layer.addUrl` com
//!   `bbox`), do nível de pirâmide mais grosso cujas células no chão não passam das da grade (a
//!   grade de 30 m lê o de cerca de 18 m; a de 1 m, o arquivo inteiro). Cada janela vai para a
//!   grade do projeto (média numa grade mais grossa), as janelas se juntam e o mosaico é recortado
//!   pela área.

use std::f64::consts::PI;

/// Onde ficam os arquivos.
pub const BASE: &str = "https://dataforgood-fb-data.s3.amazonaws.com/forests/v2/global/dinov3_global_chm_v2_ml3";

/// O zoom dos tiles.
pub const ZOOM: u32 = 10;

/// O raio da esfera do Web Mercator, em metros.
const RAIO: f64 = 6_378_137.0;

/// O lado da célula do arquivo, em metros de Web Mercator.
pub const LADO: f64 = 2.0 * PI * RAIO / (256.0 * 32_768.0 * 4.0);

/// Quantos níveis de pirâmide o arquivo tem além do inteiro.
pub const PIRAMIDES: i32 = 6;

/// Quantos tiles um clique lê, no máximo.
pub const MAX_TILES: usize = 16;

/// Quantos valores uma janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Altura do dossel a 1 m (Meta CHMv2)";

/// O tile de zoom [`ZOOM`] que contém o ponto (coluna, linha).
pub fn tile_do_ponto(lon: f64, lat: f64) -> (u32, u32) {
    let n = f64::from(1u32 << ZOOM);
    let lat = lat.clamp(-85.05, 85.05).to_radians();
    let x = ((lon + 180.0) / 360.0 * n).floor().clamp(0.0, n - 1.0);
    let y = ((1.0 - lat.tan().asinh() / PI) / 2.0 * n).floor().clamp(0.0, n - 1.0);
    (x as u32, y as u32)
}

/// O quadkey do tile (coluna, linha): um dígito por nível, o mais alto primeiro.
pub fn quadkey(x: u32, y: u32) -> String {
    (1..=ZOOM)
        .rev()
        .map(|i| {
            let m = 1u32 << (i - 1);
            let d = u32::from(x & m != 0) + 2 * u32::from(y & m != 0);
            char::from(b'0' + d as u8)
        })
        .collect()
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
    pub quadkey: String,
    pub retangulo: [f64; 4],
}

impl Tile {
    pub fn url(&self) -> String {
        format!("{BASE}/chm/{}.tif", self.quadkey)
    }

    /// A listagem do bucket com o prefixo do arquivo: diz se ele existe.
    pub fn listagem(&self) -> String {
        let pasta = BASE.trim_start_matches("https://dataforgood-fb-data.s3.amazonaws.com/");
        format!("https://dataforgood-fb-data.s3.amazonaws.com/?list-type=2&max-keys=5&prefix={pasta}/chm/{}", self.quadkey)
    }

    /// Se a `listagem` (o XML do S3) tem o arquivo.
    pub fn existe(&self, listagem: &str) -> bool {
        listagem.contains(&format!("/chm/{}.tif</Key>", self.quadkey))
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
                v.push(Tile { quadkey: quadkey(x, y), retangulo: r });
            }
        }
    }
    v
}

/// O lado da célula do nível `k` da pirâmide (0 é o arquivo inteiro), em metros de Web Mercator.
pub fn lado_do_nivel(k: i32) -> f64 {
    LADO * 2f64.powi(k)
}

/// O fator de escala do Web Mercator na latitude `lat`: um metro no chão vale `1 / cos` metros
/// no arquivo.
fn no_chao(lat: f64) -> f64 {
    lat.to_radians().cos()
}

/// O nível mais grosso cujas células, no chão da latitude `lat`, não passam de `celula` metros.
pub fn nivel(celula: f64, lat: f64) -> i32 {
    (0..=PIRAMIDES).rev().find(|&k| lado_do_nivel(k) * no_chao(lat) <= celula + 1e-6).unwrap_or(0)
}

/// A janela do tile para o retângulo `c`: a parte dele dentro do tile, com três células de
/// `lado` metros (de Web Mercator) de margem; `None` se ele não o toca.
pub fn janela(t: &Tile, c: [f64; 4], lado: f64) -> Option<[f64; 4]> {
    let r = t.retangulo;
    let (o, s, l, n) = (c[0].max(r[0]), c[1].max(r[1]), c[2].min(r[2]), c[3].min(r[3]));
    if o >= l || s >= n {
        return None;
    }
    let m = 3.0 * lado * no_chao((s + n) / 2.0) / 111_320.0;
    Some([o - m, s - m, l + m, n + m])
}

/// O que a janela lida do tile `t` precisa cobrir para servir à área `c`: a parte de `c` dentro
/// do tile. A [`janela`] pedida passa da borda do tile pela margem, mas o arquivo acaba nela: medir
/// a janela que chegou contra a pedida a dava por incompleta, e no app desktop o comando, ao rodar
/// de novo quando ela chegava, a tirava e pedia outra vez; o RustGIS respondia com a camada já
/// tirada ("removed or undone before the command ran again").
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

/// O lado, em metros no chão, das células do nível `k` na latitude `lat`.
pub fn lado_no_chao(k: i32, lat: f64) -> f64 {
    lado_do_nivel(k) * no_chao(lat)
}

/// A reamostragem para a grade: bilinear até a célula lida, média numa grade mais grossa.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "average" } else { "bilinear" }
}

/// O nome da camada de passagem com a janela do tile, lida no nível `k`.
pub fn nome_da_janela(t: &Tile, k: i32) -> String {
    format!("BrasilData: CHMv2 {} nível {k} (janela)", t.quadkey)
}

/// A citação, como a licença pede.
pub const CITACAO: &str = "Meta e World Resources Institute, Canopy Height Maps v2 (CHMv2), 2026 (arXiv:2603.06382): altura do dossel a cerca de 1 m estimada com o DINOv3 sobre imagens de satélite de alta resolução; lido do bucket público dataforgood-fb-data (AWS), posto na grade do projeto e recortado pela área";

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_quadkey_de_vicosa() {
        let (x, y) = tile_do_ponto(-42.88, -20.75);
        assert_eq!(quadkey(x, y), "2110222310");
        let r = retangulo_do_tile(x, y);
        let perto = |a: f64, b: f64| (a - b).abs() < 1e-3;
        assert!(perto(r[0], -42.8906) && perto(r[2], -42.5391) && perto(r[1], -20.9614) && perto(r[3], -20.6328), "{r:?}");
        assert_eq!(quadkey(0, 0), "0000000000");
        assert_eq!(quadkey(1023, 1023), "3333333333");
    }

    #[test]
    fn os_tiles_de_um_retangulo() {
        // Viçosa passa da divisa de -42,89°: dois tiles de largura.
        let t = tiles([-43.003, -20.838, -42.754, -20.655]);
        assert_eq!(t.len(), 2, "{t:?}");
        assert!(t.iter().any(|t| t.quadkey == "2110222310"));
        assert_eq!(t[0].url(), format!("{BASE}/chm/{}.tif", t[0].quadkey));
        // Um ponto só: um tile.
        assert_eq!(tiles([-42.88, -20.75, -42.87, -20.74]).len(), 1);
    }

    #[test]
    fn o_nivel_e_os_limites() {
        assert!((LADO - 1.194_328_566_955_874_7).abs() < 1e-9);
        // A 20° S, as células do arquivo têm 1,12 m no chão.
        assert_eq!(nivel(1.0, -20.0), 0);
        assert_eq!(nivel(30.0, -20.0), 4);
        assert_eq!(nivel(10.0, -20.0), 3);
        assert_eq!(nivel(1000.0, -20.0), PIRAMIDES);
        assert!((lado_no_chao(4, -20.0) - 17.96).abs() < 0.01);
        // Um tile inteiro no arquivo inteiro passa do limite (32768² valores).
        let t = &tiles([-42.88, -20.75, -42.87, -20.74])[0];
        let j = janela(t, t.retangulo, LADO).unwrap();
        assert!(valores(j, LADO) > MAX_VALORES);
        assert_eq!(nivel_que_cabe(&[j], 0), Some(3));
        assert_eq!(nivel_que_cabe(&[j], 4), Some(4));
        assert_eq!(reamostragem(30.0, 17.96), "average");
        assert_eq!(reamostragem(10.0, 8.98), "bilinear");
    }

    #[test]
    fn a_janela_basta_ate_a_borda_do_tile() {
        // Viçosa toca dois tiles: o que cada janela precisa cobrir acaba na borda do tile, mesmo
        // com a margem da janela pedida passando dela.
        let c = [-43.003, -20.838, -42.754, -20.655];
        let ts = tiles(c);
        assert_eq!(ts.len(), 2);
        for t in &ts {
            let j = janela(t, c, lado_do_nivel(4)).unwrap();
            let a = a_cobrir(t, c);
            let r = t.retangulo;
            assert!(a[0] >= r[0] && a[1] >= r[1] && a[2] <= r[2] && a[3] <= r[3], "{a:?} dentro de {r:?}");
            assert!(a[0] >= j[0] && a[1] >= j[1] && a[2] <= j[2] && a[3] <= j[3]);
            // A janela pedida passa da borda do tile em algum lado.
            assert!(j[0] < r[0] || j[1] < r[1] || j[2] > r[2] || j[3] > r[3]);
        }
    }

    #[test]
    fn a_listagem_diz_se_o_tile_existe() {
        let t = &tiles([-42.88, -20.75, -42.87, -20.74])[0];
        assert!(t.listagem().ends_with("&prefix=forests/v2/global/dinov3_global_chm_v2_ml3/chm/2110222310"), "{}", t.listagem());
        let xml = "<ListBucketResult><KeyCount>1</KeyCount><Contents><Key>forests/v2/global/dinov3_global_chm_v2_ml3/chm/2110222310.tif</Key></Contents></ListBucketResult>";
        assert!(t.existe(xml));
        assert!(!t.existe("<ListBucketResult><KeyCount>0</KeyCount></ListBucketResult>"));
    }
}
