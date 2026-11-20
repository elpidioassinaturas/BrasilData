//! A tipologia florestal de 2020 do ForTy (Forest Typology v1, Nature Trace: Google DeepMind e
//! Google Research, com o WRI e o IIASA): seis classes a 10 m, alinhadas à FAO e ao regulamento
//! europeu contra o desmatamento (EUDR), cuja data de corte é 31/12/2020.
//!
//! - **De onde:** a camada de classificação (a classe mais provável de cada pixel, de 1 a 6; 255 é
//!   sem dado) em COGs no bucket público `nature-trace` do Google Cloud Storage, sem login, licença
//!   CC BY 4.0. Cada quadrícula MGRS (6° × 8°) é dividida em 20 × 20 blocos de uns 33 × 45 km, em
//!   UTM (do hemisfério norte na faixa N), com seis níveis de pirâmide (de 20 a 640 m). O Figshare+ tem os mesmos arquivos em zips,
//!   que o RustGIS não lê aos pedaços.
//! - **A grade dos blocos:** o oeste e a largura de cada coluna e o sul e a altura de cada linha
//!   (variam de 160 m entre blocos) de cada quadrícula do Brasil, e quais blocos existem (no mar não há), estão em [`crate::forty_grade`], tirados dos
//!   cabeçalhos dos arquivos por `scripts/grade_forty.py`. Os arquivos vão do sul para o norte.
//! - **Como:** a área é dividida pelas quadrículas que ela cruza; de cada bloco que cada parte toca,
//!   só a janela dessa parte é lida (`layer.addUrl` com `bbox`), do nível de pirâmide mais grosso
//!   cujas células não passam das da grade. Cada janela vai para a grade do projeto com a classe
//!   mais frequente, as janelas se juntam e o mosaico é recortado pela área.
//! - **App web:** o bucket não manda o cabeçalho CORS nos arquivos: só no desktop e no rustgis-cli.

use crate::forty_grade::{QUADRICULAS, Quadricula};

/// Onde ficam os arquivos.
pub const BASE: &str = "https://storage.googleapis.com/nature-trace/export_rasters/forest_typology_2020_v1_0_classification";

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Tipologia florestal 2020 (ForTy, 10 m)";

/// Quantos blocos um clique lê, no máximo (o RustGIS espera até 16 downloads de uma vez).
pub const MAX_BLOCOS: usize = 16;

/// O lado da célula do arquivo inteiro, em metros.
pub const LADO: f64 = 10.0;

/// Quantos níveis de pirâmide o arquivo tem além do inteiro.
pub const PIRAMIDES: i32 = 6;

/// Quantos valores uma janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

/// As classes: o valor, o nome em português e a cor.
pub const CLASSES: [(u8, &str, &str); 6] = [
    (1, "Floresta primária", "#0b4d1e"),
    (2, "Floresta em regeneração natural", "#5aa832"),
    (3, "Floresta plantada", "#a6d96a"),
    (4, "Plantio florestal comercial", "#7b3294"),
    (5, "Culturas arbóreas e agrofloresta", "#e08214"),
    (6, "Outros usos da terra", "#f2ead3"),
];

/// O estilo do raster (paleta com os nomes das classes), no formato do QGIS que o
/// `style.importQgis` lê. O RustGIS 0.7.0 não guarda os rótulos (RustGIS#889): a legenda mostra
/// os códigos, que o aviso e a nota do layout ([`CHAVE`]) explicam.
pub fn qml() -> String {
    let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
    for (v, nome, cor) in CLASSES {
        q.push_str(&format!(r#"<paletteEntry value="{v}" color="{cor}" alpha="255" label="{nome}"/>"#));
    }
    q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
    q
}

/// O que cada código quer dizer, para o aviso.
pub const CHAVE: &str = "1 floresta primária (verde-escuro), 2 em regeneração natural (verde), 3 plantada (verde-claro), 4 plantio florestal comercial (roxo), 5 culturas arbóreas e agrofloresta (laranja), 6 outros usos (bege)";

/// A citação, como a licença pede.
pub const CITACAO: &str = "Neumann, M. et al. Global forest typology at 10-meter resolution for forest and land-use monitoring. Scientific Data (2026), doi:10.1038/s41597-026-08299-5; Forest Typology 2020 (v1) Classification Layer, Figshare+, doi:10.25452/figshare.plus.32301177. Lido do bucket público nature-trace (Google Cloud), posto na grade do projeto com a classe mais frequente e recortado pela área";

pub const LICENCA: &str = "CC BY 4.0";

// --- UTM (WGS 84), pela série de Krüger -------------------------------------------------------

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;
const K0: f64 = 0.9996;
const FALSO_LESTE: f64 = 500_000.0;
const FALSO_NORTE_SUL: f64 = 10_000_000.0;

fn constantes() -> (f64, [f64; 3], [f64; 3]) {
    let n = F / (2.0 - F);
    let (n2, n3) = (n * n, n * n * n);
    let a = A / (1.0 + n) * (1.0 + n2 / 4.0 + n2 * n2 / 64.0);
    let alfa = [n / 2.0 - 2.0 * n2 / 3.0 + 5.0 * n3 / 16.0, 13.0 * n2 / 48.0 - 3.0 * n3 / 5.0, 61.0 * n3 / 240.0];
    let beta = [n / 2.0 - 2.0 * n2 / 3.0 + 37.0 * n3 / 96.0, n2 / 48.0 + n3 / 15.0, 17.0 * n3 / 480.0];
    (a, alfa, beta)
}

fn meridiano(fuso: u8) -> f64 {
    f64::from(fuso) * 6.0 - 183.0
}

fn falso_norte(norte: bool) -> f64 {
    if norte { 0.0 } else { FALSO_NORTE_SUL }
}

/// (leste, norte) no fuso `fuso` (EPSG:326xx com `norte`, 327xx sem) do ponto (lon, lat).
pub fn para_utm(fuso: u8, norte: bool, lon: f64, lat: f64) -> (f64, f64) {
    let (a, alfa, _) = constantes();
    let n = F / (2.0 - F);
    let phi = lat.to_radians();
    let lam = (lon - meridiano(fuso)).to_radians();
    let e = 2.0 * n.sqrt() / (1.0 + n);
    let t = (phi.sin().atanh() - e * (e * phi.sin()).atanh()).sinh();
    let xi = t.atan2(lam.cos());
    let eta = (lam.sin() / (1.0 + t * t).sqrt()).atanh();
    let (mut x, mut y) = (eta, xi);
    for (j, al) in alfa.iter().enumerate() {
        let k = 2.0 * (j as f64 + 1.0);
        x += al * (k * xi).cos() * (k * eta).sinh();
        y += al * (k * xi).sin() * (k * eta).cosh();
    }
    (FALSO_LESTE + K0 * a * x, falso_norte(norte) + K0 * a * y)
}

/// (lon, lat) do ponto (`leste`, `y`) no fuso `fuso` (EPSG:326xx com `norte`, 327xx sem).
pub fn de_utm(fuso: u8, norte: bool, leste: f64, y: f64) -> (f64, f64) {
    let (a, _, beta) = constantes();
    let n = F / (2.0 - F);
    let xi = (y - falso_norte(norte)) / (K0 * a);
    let eta = (leste - FALSO_LESTE) / (K0 * a);
    let (mut xl, mut el) = (xi, eta);
    for (j, b) in beta.iter().enumerate() {
        let k = 2.0 * (j as f64 + 1.0);
        xl -= b * (k * xi).sin() * (k * eta).cosh();
        el -= b * (k * xi).cos() * (k * eta).sinh();
    }
    let chi = (xl.sin() / el.cosh()).asin();
    let e = 2.0 * n.sqrt() / (1.0 + n);
    // A latitude geodésica pela conforme, por iteração.
    let quarto = std::f64::consts::FRAC_PI_4;
    let mut phi = chi;
    for _ in 0..10 {
        let es = e * phi.sin();
        phi = 2.0 * ((quarto + chi / 2.0).tan() * ((1.0 + es) / (1.0 - es)).powf(e / 2.0)).atan() - 2.0 * quarto;
    }
    let lam = el.sinh().atan2(xl.cos());
    (meridiano(fuso) + lam.to_degrees(), phi.to_degrees())
}

// --- Quadrículas e blocos --------------------------------------------------------------------

/// As letras das faixas de latitude do MGRS, de 80° S para o norte (sem I e O).
const FAIXAS: &str = "CDEFGHJKLMNPQRSTUVWX";

fn faixa(lat: f64) -> Option<(usize, char)> {
    let i = ((lat + 80.0) / 8.0).floor();
    (0.0..20.0).contains(&i).then(|| (i as usize, FAIXAS.as_bytes()[i as usize] as char))
}

fn fuso(lon: f64) -> u8 {
    (((lon + 180.0) / 6.0).floor() as i64 + 1).clamp(1, 60) as u8
}

/// O retângulo em graus de uma quadrícula.
fn retangulo_da_quadricula(q: &Quadricula) -> [f64; 4] {
    let i = FAIXAS.find(q.faixa).unwrap_or(0) as f64;
    let oeste = meridiano(q.fuso) - 3.0;
    [oeste, -80.0 + 8.0 * i, oeste + 6.0, -72.0 + 8.0 * i]
}

/// O retângulo (oeste, sul, leste, norte) em UTM do fuso que cobre o retângulo `c` em graus.
fn em_utm(fuso: u8, norte: bool, c: [f64; 4]) -> [f64; 4] {
    let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let passos = 8;
    for i in 0..=passos {
        let t = f64::from(i) / f64::from(passos);
        let lon = c[0] + (c[2] - c[0]) * t;
        let lat = c[1] + (c[3] - c[1]) * t;
        for (x, y) in [(lon, c[1]), (lon, c[3]), (c[0], lat), (c[2], lat)] {
            let (e, n) = para_utm(fuso, norte, x, y);
            r = [r[0].min(e), r[1].min(n), r[2].max(e), r[3].max(n)];
        }
    }
    r
}

/// O retângulo em graus que cobre o retângulo `u` em UTM do fuso.
fn em_graus(fuso: u8, norte: bool, u: [f64; 4]) -> [f64; 4] {
    let mut r = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    let passos = 8;
    for i in 0..=passos {
        let t = f64::from(i) / f64::from(passos);
        let x = u[0] + (u[2] - u[0]) * t;
        let y = u[1] + (u[3] - u[1]) * t;
        for (e, n) in [(x, u[1]), (x, u[3]), (u[0], y), (u[2], y)] {
            let (lon, lat) = de_utm(fuso, norte, e, n);
            r = [r[0].min(lon), r[1].min(lat), r[2].max(lon), r[3].max(lat)];
        }
    }
    r
}

/// O maior retângulo em graus dentro do retângulo `u` em UTM do fuso: o bloco é um retângulo
/// em UTM, inclinado em relação aos meridianos, e o retângulo de fora ([`em_graus`]) passa dele
/// nos cantos.
fn dentro_em_graus(fuso: u8, norte: bool, u: [f64; 4]) -> [f64; 4] {
    let mut r = [f64::MIN, f64::MIN, f64::MAX, f64::MAX];
    let passos = 8;
    for i in 0..=passos {
        let t = f64::from(i) / f64::from(passos);
        let x = u[0] + (u[2] - u[0]) * t;
        let y = u[1] + (u[3] - u[1]) * t;
        r[0] = r[0].max(de_utm(fuso, norte, u[0], y).0);
        r[2] = r[2].min(de_utm(fuso, norte, u[2], y).0);
        r[1] = r[1].max(de_utm(fuso, norte, x, u[1]).1);
        r[3] = r[3].min(de_utm(fuso, norte, x, u[3]).1);
    }
    r
}

fn intersecao(a: [f64; 4], b: [f64; 4]) -> Option<[f64; 4]> {
    let r = [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])];
    (r[0] < r[2] && r[1] < r[3]).then_some(r)
}

/// Um bloco que a área toca, com a parte da área que ele serve.
#[derive(Clone, Debug, PartialEq)]
pub struct Bloco {
    /// A quadrícula MGRS ("23L").
    pub quadricula: String,
    pub linha: u8,
    pub coluna: u8,
    pub fuso: u8,
    /// O retângulo em graus que cobre o bloco.
    pub retangulo: [f64; 4],
    /// O maior retângulo em graus dentro do bloco.
    pub dentro: [f64; 4],
    /// A parte da área que este bloco lê: a área dentro da quadrícula e do bloco, em graus.
    pub parte: [f64; 4],
}

impl Bloco {
    pub fn url(&self) -> String {
        format!("{BASE}/raster_{}_20_{}_{}.tif", self.quadricula, self.linha, self.coluna)
    }

    /// O que a janela lida precisa cobrir para servir à área: a parte dela dentro do bloco. A
    /// janela pedida passa da borda do arquivo, mas ele acaba nela; e o retângulo de fora do bloco
    /// em graus passa do arquivo nos cantos: medir por ele dava a janela que chegou por incompleta,
    /// e no app desktop o comando a tirava e pedia outra vez, o que o RustGIS recusa ("removed or
    /// undone before the command ran again"). Sem parte dentro (uma lasca na borda), nada.
    pub fn a_cobrir(&self) -> Option<[f64; 4]> {
        intersecao(self.parte, self.dentro)
    }

    /// O nome da camada de passagem com a janela do bloco, lida no nível `k`.
    pub fn nome_da_janela(&self, k: i32) -> String {
        format!("BrasilData: ForTy {} {} {} nível {k} (janela)", self.quadricula, self.linha, self.coluna)
    }
}

/// Os blocos que existem e que o retângulo `c` (em graus) toca. Cada ponto da área vem da
/// quadrícula MGRS que o contém, para que blocos vizinhos de quadrículas diferentes (que se
/// sobrepõem um pouco) não sejam lidos duas vezes.
pub fn blocos(c: [f64; 4]) -> Vec<Bloco> {
    let mut v = Vec::new();
    let (Some((s, _)), Some((n, _))) = (faixa(c[1]), faixa(c[3] - 1e-9)) else {
        return v;
    };
    for z in fuso(c[0])..=fuso(c[2] - 1e-9) {
        for i in s..=n {
            let letra = FAIXAS.as_bytes()[i] as char;
            let Some(q) = QUADRICULAS.iter().find(|q| q.fuso == z && q.faixa == letra) else {
                continue;
            };
            let Some(parte) = intersecao(c, retangulo_da_quadricula(q)) else {
                continue;
            };
            let u = em_utm(z, q.norte, parte);
            for &(r, sul, altura) in q.linhas {
                if sul >= u[3] || sul + altura <= u[1] {
                    continue;
                }
                for &(col, oeste, largura) in q.colunas {
                    if oeste >= u[2] || oeste + largura <= u[0] || q.blocos[usize::from(r)] & (1 << col) == 0 {
                        continue;
                    }
                    let u_bloco = [oeste, sul, oeste + largura, sul + altura];
                    let retangulo = em_graus(z, q.norte, u_bloco);
                    let dentro = dentro_em_graus(z, q.norte, u_bloco);
                    let Some(parte) = intersecao(parte, retangulo) else {
                        continue;
                    };
                    v.push(Bloco { quadricula: format!("{z}{letra}"), linha: r, coluna: col, fuso: z, retangulo, dentro, parte });
                }
            }
        }
    }
    v
}

/// O nível mais grosso cujas células não passam de `celula` metros.
pub fn nivel(celula: f64) -> i32 {
    (0..=PIRAMIDES).rev().find(|&k| lado_do_nivel(k) <= celula + 1e-6).unwrap_or(0)
}

/// O lado da célula do nível `k` da pirâmide (0 é o arquivo inteiro), em metros.
pub fn lado_do_nivel(k: i32) -> f64 {
    LADO * 2f64.powi(k)
}

/// A janela do bloco: a parte da área que ele serve, com três células de `lado` metros de margem.
pub fn janela(b: &Bloco, lado: f64) -> [f64; 4] {
    let p = b.parte;
    let lat = (p[1] + p[3]) / 2.0;
    let my = 3.0 * lado / 110_574.0;
    let mx = 3.0 * lado / (111_320.0 * lat.to_radians().cos());
    [p[0] - mx, p[1] - my, p[2] + mx, p[3] + my]
}

/// Quantos valores a janela `j` (em graus) tem lida com células de `lado` metros.
pub fn valores(j: [f64; 4], lado: f64) -> f64 {
    let lat = (j[1] + j[3]) / 2.0;
    let largura = (j[2] - j[0]) * 111_320.0 * lat.to_radians().cos() / lado;
    let altura = (j[3] - j[1]) * 110_574.0 / lado;
    largura * altura
}

/// O nível mais fino, a partir de `k`, com que todas as `janelas` cabem no limite.
pub fn nivel_que_cabe(janelas: &[[f64; 4]], k: i32) -> Option<i32> {
    (k..=PIRAMIDES).find(|&n| janelas.iter().all(|j| valores(*j, lado_do_nivel(n)) <= MAX_VALORES))
}

/// A reamostragem para a grade: a classe mais frequente numa grade mais grossa que a célula lida,
/// a mais próxima nas outras.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "mode" } else { "nearest" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utm_ida_e_volta() {
        // O canto do bloco 23L 0 0, pelo gdalinfo: (168960, 8222720) é 48°05'37,00" O, 16°03'11,09" S.
        let (lon, lat) = de_utm(23, false, 168_960.0, 8_222_720.0);
        assert!((lon - -(48.0 + 5.0 / 60.0 + 37.00 / 3600.0)).abs() < 1e-5, "{lon}");
        assert!((lat - -(16.0 + 3.0 / 60.0 + 11.09 / 3600.0)).abs() < 1e-5, "{lat}");
        let (e, n) = para_utm(23, false, lon, lat);
        assert!((e - 168_960.0).abs() < 0.01 && (n - 8_222_720.0).abs() < 0.01, "{e} {n}");
        // Viçosa (MG), no fuso 23.
        let (e, n) = para_utm(23, false, -42.88, -20.75);
        let (lon, lat) = de_utm(23, false, e, n);
        assert!((lon + 42.88).abs() < 1e-8 && (lat + 20.75).abs() < 1e-8);
        // Boa Vista (RR), no hemisfério norte: sem o falso norte.
        let (e, n) = para_utm(20, true, -60.67, 2.82);
        assert!(n > 0.0 && n < 400_000.0, "{n}");
        let (lon, lat) = de_utm(20, true, e, n);
        assert!((lon + 60.67).abs() < 1e-8 && (lat - 2.82).abs() < 1e-8);
    }

    #[test]
    fn faixas_e_fusos() {
        assert_eq!(faixa(-20.75), Some((7, 'K')));
        assert_eq!(faixa(-16.0), Some((8, 'L')));
        assert_eq!(faixa(2.0), Some((10, 'N')));
        assert_eq!(fuso(-42.88), 23);
        assert_eq!(fuso(-48.0), 23);
        assert_eq!(fuso(-48.01), 22);
    }

    #[test]
    fn os_blocos_de_vicosa() {
        let c = [-43.003, -20.838, -42.754, -20.655];
        let bs = blocos(c);
        assert!(!bs.is_empty() && bs.len() <= 4, "{bs:?}");
        for b in &bs {
            assert_eq!(b.quadricula, "23K");
            // A parte de cada bloco fica dentro da área e do bloco.
            assert!(b.parte[0] >= c[0] - 1e-12 && b.parte[2] <= c[2] + 1e-12);
            assert!(b.parte[0] >= b.retangulo[0] && b.parte[3] <= b.retangulo[3]);
            assert!(b.url().starts_with(&format!("{BASE}/raster_23K_20_")));
        }
        // As partes cobrem a área sem buraco: a soma das áreas passa da área (há sobreposição só
        // na borda dos blocos).
        let area = |r: [f64; 4]| (r[2] - r[0]) * (r[3] - r[1]);
        let soma: f64 = bs.iter().map(|b| area(b.parte)).sum();
        assert!(soma >= area(c) * 0.999, "{soma} < {}", area(c));
    }

    #[test]
    fn uma_area_na_divisa_de_fusos_le_das_duas_quadriculas() {
        // Em volta de 48° O, a 16° S: as quadrículas 22L e 23L.
        let bs = blocos([-48.05, -15.9, -47.95, -15.8]);
        let qs: std::collections::BTreeSet<&str> = bs.iter().map(|b| b.quadricula.as_str()).collect();
        assert_eq!(qs.into_iter().collect::<Vec<_>>(), ["22L", "23L"]);
        // Nenhuma parte passa de 48° O para o lado da outra quadrícula.
        for b in &bs {
            if b.quadricula == "23L" {
                assert!(b.parte[0] >= -48.0);
            } else {
                assert!(b.parte[2] <= -48.0);
            }
        }
    }

    #[test]
    fn o_que_a_janela_cobre_fica_dentro_do_bloco() {
        // A microrregião de Viçosa: o que cada janela precisa cobrir fica dentro do bloco, mesmo
        // com o retângulo de fora em graus passando dele nos cantos.
        for b in blocos([-43.616, -21.167, -42.377, -20.452]) {
            let (d, r) = (b.dentro, b.retangulo);
            assert!(d[0] > r[0] && d[1] > r[1] && d[2] < r[2] && d[3] < r[3], "{b:?}");
            if let Some(a) = b.a_cobrir() {
                assert!(a[0] >= d[0] && a[1] >= d[1] && a[2] <= d[2] && a[3] <= d[3]);
            }
        }
    }

    #[test]
    fn o_mar_nao_tem_blocos() {
        assert!(blocos([-34.0, -20.0, -33.9, -19.9]).is_empty());
        // Fora do Brasil (Europa): nenhuma quadrícula na tabela.
        assert!(blocos([10.0, 45.0, 10.1, 45.1]).is_empty());
    }

    #[test]
    fn o_nivel_e_os_limites() {
        assert_eq!(nivel(10.0), 0);
        assert_eq!(nivel(30.0), 1);
        assert_eq!(nivel(100.0), 3);
        assert_eq!(nivel(5000.0), PIRAMIDES);
        assert_eq!(reamostragem(30.0, 20.0), "nearest");
        assert_eq!(reamostragem(100.0, 40.0), "mode");
        let b = &blocos([-42.88, -20.75, -42.87, -20.74])[0];
        let j = janela(b, 10.0);
        assert!(j[0] < b.parte[0] && j[3] > b.parte[3]);
        assert!(valores(j, 10.0) < MAX_VALORES);
        // Um bloco inteiro (33 × 45 km) a 10 m tem uns 15 milhões de valores: cabe.
        assert_eq!(nivel_que_cabe(&[b.retangulo], 0), Some(0));
    }

    #[test]
    fn a_paleta_tem_as_seis_classes() {
        let q = qml();
        assert_eq!(q.matches("<paletteEntry").count(), 6);
        assert!(q.contains(r##"value="1" color="#0b4d1e" alpha="255" label="Floresta primária""##));
    }
}
