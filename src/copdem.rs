//! A elevação do Copernicus GLO-30 (o modelo digital de superfície a 30 m do Copernicus): os tiles
//! de 1° que a área toca e as contas da leitura.
//!
//! - **De onde:** o balde `copernicus-dem` do Planetary Computer da Microsoft, sem login, com o
//!   token anônimo do [`crate::radar_pc`]. São os mesmos COGs do balde da AWS, que derruba a conexão
//!   no meio (5 de 6 leituras de Sergipe, em 24/11/2026). O espelho da OpenTopography responde, mas
//!   tem tiles de 3601 células, e as visões reduzidas dele não são médias nem caem na mesma rede de
//!   um tile para o outro.
//! - **Os arquivos:** 3600 × 3600 células de 1″ por tile, `float32`, `PixelIsPoint` (o canto a meia
//!   célula do grau inteiro), e três visões reduzidas, cada uma a média exata de 2 × 2 células da
//!   anterior (conferido no tile S21 W043). No mar não há tile ([`Tile::existe`]).
//! - **Como:** de cada tile, só a janela da área (`layer.addUrl` com `bbox`), da visão mais grossa
//!   cujas células não passam das da grade, e só dos tiles que o contorno toca ([`contornos`]): o
//!   retângulo do Amazonas tem 234, e o contorno toca 161. As janelas se juntam na rede de células do
//!   arquivo, sem reamostrar, e o mosaico vai para a grade do projeto de uma vez: postas na grade uma
//!   a uma, cada emenda deixaria um risco no relevo.
//! - **Em faixas:** o `cube.reduce`, que junta as janelas, vai até 32 milhões de células. Uma área
//!   maior vai em faixas de latitude ([`faixas`]), com sobra dos dois lados; cada uma vai para a
//!   grade e é cortada na divisa antes de se juntar às outras.

use serde_json::{Value, json};

use crate::cog_global::Arquivo;

/// O pedido do token anônimo que lê os tiles.
pub const URL_DO_TOKEN: &str = "https://planetarycomputer.microsoft.com/api/sas/v1/token/cop-dem-glo-30";

/// Onde os tiles ficam.
const SERVIDOR: &str = "https://elevationeuwest.blob.core.windows.net/copernicus-dem/COP30_hh";

/// Quantos tiles inteiros (cerca de 45 MB cada) um clique baixa, no máximo: no app web, que ainda não lê
/// a janela de um arquivo remoto.
pub const MAX_INTEIROS: usize = 4;

/// O começo do nome das camadas de passagem com as janelas.
pub const JANELAS: &str = "BrasilData: Copernicus";

/// O lado da célula do arquivo inteiro, em graus.
pub const LADO: f64 = 1.0 / 3600.0;

/// Quantas visões reduzidas os arquivos têm.
const PIRAMIDES: i32 = 3;

/// Quantos tiles um clique lê, no máximo: qualquer estado (o Amazonas toca 161) cabe.
pub const MAX_TILES: usize = 250;

/// Quantas células uma faixa pode ter: o `cube.reduce` vai até 32 milhões.
const MAX_DA_FAIXA: f64 = 28e6;

/// O retângulo da máscara [`EXISTEM`]: a primeira coluna (oeste) e a primeira linha (sul), em graus.
const OESTE: i32 = -75;
const SUL: i32 = -35;
const COLUNAS: i32 = 48;

/// Os tiles que existem (do `tileList.txt` do balde da AWS, de 24/11/2026), um número por grau de
/// latitude de 35° S a 5° N e um bit por grau de longitude de 75° O a 28° O, a partir do menos
/// significativo. Na janela, o RustGIS não diz ao plugin que um arquivo não existe, e um tile de mar
/// pedido ali não chegaria nunca.
const EXISTEM: [u64; 41] = [
    0x0000003ffffc,
    0x0000007ffff8,
    0x000000fffff8,
    0x000001fffff8,
    0x000001fffff8,
    0x000003fffff8,
    0x000007fffff8,
    0x000007fffff8,
    0x000007fffff0,
    0x00000ffffff0,
    0x00003ffffff0,
    0x0003fffffff0,
    0x0007fffffff0,
    0x0007fffffff0,
    0x6007fffffff0,
    0x000ffffffff0,
    0x000ffffffff0,
    0x001ffffffffc,
    0x001fffffffff,
    0x001fffffffff,
    0x001fffffffff,
    0x001fffffffff,
    0x003fffffffff,
    0x003fffffffff,
    0x007fffffffff,
    0x00ffffffffff,
    0x01ffffffffff,
    0x01ffffffffff,
    0x01ffffffffff,
    0x00ffffffffff,
    0x007fffffffff,
    0x061fffffffff,
    0x000fffffffff,
    0x00007fffffff,
    0x00001fffffff,
    0x000003ffffff,
    0x000003ffffff,
    0x000001ffffff,
    0x000001ffffff,
    0x000000ffffff,
    0x0000007fffff,
];

/// Um tile: o grau inteiro a sul e a oeste dele, o do nome.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tile {
    pub lat: i32,
    pub lon: i32,
}

impl Tile {
    /// O canto no nome do arquivo ("S11 W037").
    pub fn rotulo(&self) -> String {
        let (ns, ew) = (if self.lat < 0 { 'S' } else { 'N' }, if self.lon < 0 { 'W' } else { 'E' });
        format!("{ns}{:02} {ew}{:03}", self.lat.abs(), self.lon.abs())
    }

    /// O endereço do arquivo, sem o token.
    pub fn url(&self) -> String {
        format!("{SERVIDOR}/Copernicus_DSM_COG_10_{}_00_DEM.tif", self.rotulo().replace(' ', "_00_"))
    }

    /// O nome da camada de passagem com a janela do tile, lida no nível `k`.
    pub fn nome_da_janela(&self, k: i32) -> String {
        format!("{JANELAS} {} nível {k} (janela)", self.rotulo())
    }

    /// O retângulo das células do tile, em graus.
    pub fn caixa(&self) -> [f64; 4] {
        let (o, s, m) = (f64::from(self.lon), f64::from(self.lat), LADO / 2.0);
        [o - m, s + m, o + 1.0 - m, s + 1.0 + m]
    }

    /// Se o tile existe (no mar, não). Fora do retângulo da máscara, sempre.
    pub fn existe(&self) -> bool {
        let coluna = self.lon - OESTE;
        match usize::try_from(self.lat - SUL).ok().and_then(|l| EXISTEM.get(l)) {
            Some(linha) if (0..COLUNAS).contains(&coluna) => linha >> coluna & 1 == 1,
            _ => true,
        }
    }
}

/// A parte comum de dois retângulos, se não for só uma borda.
fn comum(a: [f64; 4], b: [f64; 4]) -> Option<[f64; 4]> {
    let r = [a[0].max(b[0]), a[1].max(b[1]), a[2].min(b[2]), a[3].min(b[3])];
    (r[2] - r[0] > 1e-9 && r[3] - r[1] > 1e-9).then_some(r)
}

/// O retângulo `c` com `m` graus a mais de cada lado.
fn com_margem(c: [f64; 4], m: f64) -> [f64; 4] {
    [c[0] - m, c[1] - m, c[2] + m, c[3] + m]
}

/// Os tiles que existem e têm células no retângulo `c` com `margem`, com a parte dele em cada um.
pub fn tiles(c: [f64; 4], margem: f64) -> Vec<(Tile, [f64; 4])> {
    let j = com_margem(c, margem);
    let graus = |a: f64, b: f64| (a.floor() as i32 - 1)..=(b.floor() as i32 + 1);
    graus(j[1], j[3])
        .flat_map(|lat| graus(j[0], j[2]).map(move |lon| Tile { lat, lon }))
        .filter(Tile::existe)
        .filter_map(|t| Some((t, comum(t.caixa(), j)?)))
        .collect()
}

/// Dos tiles `ts`, os que têm parte do retângulo `c`, sem a margem: os que se baixam inteiros.
pub fn inteiros(ts: &[(Tile, [f64; 4])], c: [f64; 4]) -> Vec<Tile> {
    let grau = |t: &Tile| [f64::from(t.lon), f64::from(t.lat), f64::from(t.lon + 1), f64::from(t.lat + 1)];
    ts.iter().map(|(t, _)| *t).filter(|t| comum(grau(t), c).is_some()).collect()
}

/// Se o erro `e` de um `layer.addUrl` é o do app web, que não lê a janela de um arquivo remoto.
pub fn sem_janela(e: &str) -> bool {
    e.contains("is read on the desktop and the command line")
}

/// Os retângulos dos tiles `ts` (o campo `i` é a posição), para o RustGIS selecionar os que o contorno
/// da área toca. A `margem` é a da leitura: um tile que o contorno não toca, mas que tem células que a
/// reamostragem da borda usa, fica.
pub fn contornos(ts: &[(Tile, [f64; 4])], margem: f64) -> Value {
    let feicao = |(i, (t, _)): (usize, &(Tile, [f64; 4]))| {
        let [o, s, l, n] = com_margem(t.caixa(), margem);
        json!({"type": "Feature", "properties": {"i": i},
            "geometry": {"type": "Polygon", "coordinates": [[[o, s], [l, s], [l, n], [o, n], [o, s]]]}})
    };
    json!({"type": "FeatureCollection", "features": ts.iter().enumerate().map(feicao).collect::<Vec<_>>()})
}

/// O arquivo de um tile, para as contas do nível (o endereço não entra nelas).
pub fn arquivo() -> Arquivo {
    Arquivo { url: String::new(), graus: LADO, piramides: PIRAMIDES }
}

/// A sobra em volta da área, em graus: três células do nível `k` mais uma da grade (`celula`
/// metros), para a reamostragem ter o que precisa na borda.
pub fn margem(k: i32, celula: f64) -> f64 {
    3.0 * arquivo().graus_do_nivel(k) + celula / 100_000.0
}

// --- Faixas -------------------------------------------------------------------------------------

/// As faixas de latitude (sul, norte) em que o retângulo `c` se junta no nível `k`: com a `margem`
/// dos dois lados, cada uma cabe no `cube.reduce`.
pub fn faixas(c: [f64; 4], k: i32, margem: f64) -> Vec<[f64; 2]> {
    let g = arquivo().graus_do_nivel(k);
    let largura = (c[2] - c[0] + 2.0 * margem) / g + 2.0;
    let altura = (MAX_DA_FAIXA / largura - 2.0) * g - 2.0 * margem;
    let n = ((c[3] - c[1]) / altura.max(g)).ceil().max(1.0) as usize;
    let passo = (c[3] - c[1]) / n as f64;
    let divisa = |i: usize| if i == n { c[3] } else { c[1] + i as f64 * passo };
    (0..n).map(|i| [divisa(i), divisa(i + 1)]).collect()
}

/// O retângulo das células que a faixa `f` da área `c` junta.
pub fn janela_da_faixa(c: [f64; 4], f: [f64; 2], margem: f64) -> [f64; 4] {
    com_margem([c[0], f[0], c[2], f[1]], margem)
}

/// Se uma janela (a `parte` de um tile) tem células na faixa de retângulo `j`: pelo menos duas do
/// nível `k`, porque o `cube.reduce` recusa uma camada que fica fora da primeira.
pub fn na_faixa(parte: [f64; 4], j: [f64; 4], k: i32) -> bool {
    let g = 2.0 * arquivo().graus_do_nivel(k);
    comum(parte, j).is_some_and(|r| r[2] - r[0] >= g && r[3] - r[1] >= g)
}

/// O `raster.project` que estende a `janela` à faixa de retângulo `j`, na rede de células do nível
/// `k`, sem reamostrar (a célula mais próxima cai no centro da do arquivo). A rede é a mesma em todos
/// os tiles: 1° tem um número inteiro de células até o último nível.
pub fn estender(janela: &str, j: [f64; 4], k: i32, nome: &str) -> Value {
    let g = arquivo().graus_do_nivel(k);
    json!({"layer": janela, "cellSize": [g, g], "origin": [-LADO / 2.0, LADO / 2.0], "extent": j, "resampling": "nearest", "name": nome})
}

/// O polígono que corta a faixa `f` (de várias) na divisa com as vizinhas. Ele passa da área para
/// os outros lados, e tem um vértice a cada 0,05° para a divisa acompanhar o paralelo em qualquer
/// sistema.
pub fn corte(c: [f64; 4], f: [f64; 2]) -> Value {
    const PASSO: f64 = 0.05;
    const SOBRA: f64 = 1.0;
    let s = if f[0] == c[1] { f[0] - SOBRA } else { f[0] };
    let n = if f[1] == c[3] { f[1] + SOBRA } else { f[1] };
    let (o, partes) = (c[0] - SOBRA, ((c[2] - c[0] + 2.0 * SOBRA) / PASSO).ceil() as usize);
    let lon = |i: usize| o + i as f64 * PASSO;
    let mut anel: Vec<[f64; 2]> = (0..=partes).map(|i| [lon(i), s]).collect();
    anel.extend((0..=partes).rev().map(|i| [lon(i), n]));
    anel.push([lon(0), s]);
    json!({"type": "FeatureCollection", "features": [
        {"type": "Feature", "properties": {}, "geometry": {"type": "Polygon", "coordinates": [anel]}}
    ]})
}

#[cfg(test)]
mod testes {
    use super::*;

    const SERGIPE: [f64; 4] = [-38.2451, -11.568, -36.3944, -9.5151];

    #[test]
    fn os_tiles_tem_o_endereco_e_so_os_que_existem() {
        let t = Tile { lat: -11, lon: -37 };
        assert_eq!(
            t.url(),
            "https://elevationeuwest.blob.core.windows.net/copernicus-dem/COP30_hh/Copernicus_DSM_COG_10_S11_00_W037_00_DEM.tif"
        );
        assert_eq!(Tile { lat: 2, lon: -61 }.rotulo(), "N02 W061");
        assert!(t.existe());
        // O mar de Sergipe, Fernando de Noronha, Trindade e o meio do Atlântico.
        assert!(!Tile { lat: -12, lon: -37 }.existe());
        assert!(Tile { lat: -4, lon: -33 }.existe() && Tile { lat: -21, lon: -30 }.existe());
        assert!(!Tile { lat: -10, lon: -30 }.existe());
        // Fora da máscara, o pedido é que diz.
        assert!(Tile { lat: 40, lon: -8 }.existe() && Tile { lat: -10, lon: -90 }.existe());
        let m = margem(0, 50.0);
        let ts = tiles(SERGIPE, m);
        assert_eq!(ts.len(), 8, "{ts:?}");
        // A parte fica dentro do tile e da área com a margem.
        for (t, p) in &ts {
            let b = t.caixa();
            assert!(p[0] >= b[0] && p[3] <= b[3] && p[0] >= SERGIPE[0] - m - 1e-12 && p[3] <= SERGIPE[3] + m + 1e-12);
        }
    }

    #[test]
    fn os_inteiros_sao_os_do_retangulo_sem_a_margem() {
        let m = margem(0, 30.0);
        let teresina = [-42.9707, -5.5866, -42.599, -4.7863];
        let ts = inteiros(&tiles(teresina, m), teresina);
        assert_eq!(ts, [Tile { lat: -6, lon: -43 }, Tile { lat: -5, lon: -43 }]);
        // Um grau inteiro como borda não puxa o vizinho, que a margem puxaria.
        let grau = [-43.0, -6.0, -42.0, -5.0];
        assert!(tiles(grau, m).len() > 1);
        assert_eq!(inteiros(&tiles(grau, m), grau).len(), 1);
        // A recusa do RustGIS (0.6.0 a 0.8.0) no app web.
        let recusa = "a window of a cloud-optimised GeoTIFF (\"bbox\") is read on the desktop and the command line for now";
        assert!(sem_janela(&format!("https://x.tif: {recusa}; here, leave out \"bbox\" to draw the file in place")));
        assert!(!sem_janela("https://x.tif: http status: 503"));
    }

    #[test]
    fn os_contornos_sao_os_tiles_com_a_margem() {
        let m = margem(0, 50.0);
        let ts = tiles(SERGIPE, m);
        let fc = contornos(&ts, m);
        let feicoes = fc["features"].as_array().unwrap();
        assert_eq!(feicoes.len(), ts.len());
        for (i, (f, (t, _))) in feicoes.iter().zip(&ts).enumerate() {
            assert_eq!(f["properties"]["i"], i);
            let anel = f["geometry"]["coordinates"][0].as_array().unwrap();
            assert_eq!((anel.len(), anel.first()), (5, anel.last()));
            let b = t.caixa();
            assert_eq!((anel[0][0].as_f64().unwrap(), anel[0][1].as_f64().unwrap()), (b[0] - m, b[1] - m));
            assert_eq!((anel[2][0].as_f64().unwrap(), anel[2][1].as_f64().unwrap()), (b[2] + m, b[3] + m));
        }
    }

    #[test]
    fn a_rede_e_a_mesma_em_todos_os_tiles() {
        // O canto do tile, e o da rede do `estender`, caem em células inteiras de todos os níveis.
        for k in 0..=PIRAMIDES {
            let g = arquivo().graus_do_nivel(k);
            let b = Tile { lat: -11, lon: -37 }.caixa();
            for d in [(b[0] + LADO / 2.0) / g, (b[3] - LADO / 2.0) / g] {
                assert!((d - d.round()).abs() < 1e-6, "{k}: {d}");
            }
        }
        let p = estender("j", [-38.0, -11.0, -37.0, -10.0], 1, "n");
        assert_eq!(p["cellSize"], json!([LADO * 2.0, LADO * 2.0]));
        assert!(p.get("crs").is_none());
    }

    #[test]
    fn as_faixas_cabem_e_se_encostam() {
        let (minas, brasil) = ([-51.0462, -22.9228, -39.8568, -14.2332], [-73.9831, -33.7439, -32.4, 5.2695]);
        for (c, celula, quantas) in [(SERGIPE, 50.0, 2), (minas, 250.0, 1), (brasil, 1000.0, 12), ([-43.6, -21.2, -42.4, -20.5], 30.0, 1)] {
            let k = arquivo().nivel(celula, (c[1] + c[3]) / 2.0);
            let m = margem(k, celula);
            let fs = faixas(c, k, m);
            assert_eq!(fs.len(), quantas, "{c:?} nível {k}");
            assert_eq!((fs[0][0], fs[fs.len() - 1][1]), (c[1], c[3]));
            assert!(fs.windows(2).all(|w| w[0][1] == w[1][0]));
            let g = arquivo().graus_do_nivel(k);
            for f in fs {
                let j = janela_da_faixa(c, f, m);
                assert!(((j[2] - j[0]) / g + 2.0) * ((j[3] - j[1]) / g + 2.0) <= MAX_DA_FAIXA * 1.001);
            }
        }
    }

    #[test]
    fn o_corte_vai_da_divisa_para_fora() {
        let fs = faixas(SERGIPE, 0, margem(0, 50.0));
        let anel = |f| corte(SERGIPE, f)["features"][0]["geometry"]["coordinates"][0].as_array().unwrap().clone();
        let (a, b) = (anel(fs[0]), anel(fs[1]));
        assert_eq!(a.first(), a.last());
        let lats = |r: &[Value]| {
            let v: Vec<f64> = r.iter().map(|p| p[1].as_f64().unwrap()).collect();
            (v.iter().copied().fold(f64::MAX, f64::min), v.iter().copied().fold(f64::MIN, f64::max))
        };
        assert_eq!(lats(&a), (SERGIPE[1] - 1.0, fs[0][1]));
        assert_eq!(lats(&b), (fs[1][0], SERGIPE[3] + 1.0));
        // A mesma divisa nas duas, e passos de 0,05°.
        assert_eq!(lats(&a).1, lats(&b).0);
        assert!((a[1][0].as_f64().unwrap() - a[0][0].as_f64().unwrap() - 0.05).abs() < 1e-9);
    }

    #[test]
    fn so_entra_na_faixa_a_janela_com_celulas_nela() {
        let j = [-38.0, -11.0, -37.0, -10.0];
        assert!(na_faixa([-38.5, -10.5, -37.5, -9.5], j, 0));
        assert!(!na_faixa([-38.5, -10.0 - LADO, -37.5, -9.5], j, 0));
        assert!(!na_faixa([-36.0, -10.5, -35.0, -9.5], j, 0));
    }
}
