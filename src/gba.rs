//! As edificações do GlobalBuildingAtlas (GBA, Universidade Técnica de Munique): o polígono de
//! cada edificação com a altura estimada, de imagens PlanetScope de 2019.
//!
//! - **De onde:** a cópia em GeoParquet do Source Cooperative (`tge-labs/globalbuildingatlas-lod1`,
//!   sem login, CORS aberto), um arquivo por bloco de 5° × 5° nomeado pelos cantos
//!   (`w045_s20_w040_s25`: oeste, norte, leste, sul), em WGS 84, com os campos `height` (m),
//!   `var` (a variância da altura), `source` (de onde veio o polígono: Google, Microsoft, OSM…) e
//!   uma coluna `bbox` por linha, ordenada no espaço. No mar não há arquivo: a listagem do bucket
//!   diz quais há.
//! - **Como:** o RustGIS 0.7.0 lê o arquivo no lugar (`layer.addUrl`, sem baixar o arquivo de
//!   1 GB ou mais): abre pelo rodapé e busca, pela coluna `bbox`, só os grupos de linhas da vista.
//!   A camada não é recortada pela área (são milhões de edificações num bloco): mostra o que a
//!   vista mostra.
//! - **Cores:** classes fixas de altura, por estilo (`style.add`): o `layer.symbology` do RustGIS
//!   0.7.0 recusa classes numa camada lida no lugar ("has no numeric values").
//! - **Licença:** os polígonos e as alturas do GBA são CC BY-NC 4.0 (sem uso comercial); os
//!   polígonos que vêm do OpenStreetMap seguem a ODbL.

/// Onde ficam os arquivos.
pub const BASE: &str = "https://data.source.coop/tge-labs/globalbuildingatlas-lod1";

/// A listagem do bucket (um XML de uns 300 KB com os 923 blocos).
pub const LISTAGEM: &str =
    "https://s3.us-west-2.amazonaws.com/us-west-2.opendata.source.coop?list-type=2&prefix=tge-labs/globalbuildingatlas-lod1/&max-keys=1000";

/// O nome da camada de um bloco.
pub const PREFIXO_CAMADA: &str = "BrasilData: Edificações e alturas (GlobalBuildingAtlas";

/// Quantos blocos de 5° um clique abre, no máximo.
pub const MAX_BLOCOS: usize = 4;

/// O nome do estilo das edificações no projeto.
pub const ESTILO: &str = "BrasilData: edificações por altura";

/// As classes de altura: o limite de cima (m), o rótulo e a cor.
pub const CLASSES: [(f64, &str, [u8; 3]); 6] = [
    (4.0, "até 4 m (1 pavimento)", [254, 240, 217]),
    (7.0, "4 a 7 m (2 pavimentos)", [253, 204, 138]),
    (10.0, "7 a 10 m (3 pavimentos)", [252, 141, 89]),
    (15.0, "10 a 15 m (4 a 5 pavimentos)", [227, 74, 51]),
    (25.0, "15 a 25 m (6 a 8 pavimentos)", [179, 0, 0]),
    (1000.0, "mais de 25 m", [110, 0, 40]),
];

/// O nome de um bloco de 5° pelos cantos: `w045_s20_w040_s25`.
fn nome(oeste: i32, norte: i32) -> String {
    let lon = |v: i32| format!("{}{:03}", if v < 0 { 'w' } else { 'e' }, v.abs());
    let lat = |v: i32| format!("{}{:02}", if v < 0 { 's' } else { 'n' }, v.abs());
    format!("{}_{}_{}_{}", lon(oeste), lat(norte), lon(oeste + 5), lat(norte - 5))
}

/// Os blocos de 5° que o retângulo `c` (em graus) toca, pelo nome.
pub fn blocos(c: [f64; 4]) -> Vec<String> {
    let passo = |v: f64| (v / 5.0).floor() as i32 * 5;
    let (o0, o1) = (passo(c[0]), passo(c[2] - 1e-9));
    let (n0, n1) = (passo(c[1]) + 5, passo(c[3] - 1e-9) + 5);
    let mut v = Vec::new();
    for norte in (n0..=n1).rev().step_by(5) {
        for oeste in (o0..=o1).step_by(5) {
            v.push(nome(oeste, norte));
        }
    }
    v
}

/// O endereço do arquivo de um bloco.
pub fn url(bloco: &str) -> String {
    format!("{BASE}/{bloco}.parquet")
}

/// Se a `listagem` (o XML do S3) tem o bloco.
pub fn existe(bloco: &str, listagem: &str) -> bool {
    listagem.contains(&format!("<Key>tge-labs/globalbuildingatlas-lod1/{bloco}.parquet</Key>"))
}

/// O nome da camada de um bloco; com um bloco só, sem o nome dele.
pub fn camada(bloco: &str, varios: bool) -> String {
    if varios { format!("{PREFIXO_CAMADA}, {bloco})") } else { format!("{PREFIXO_CAMADA})") }
}

/// Se a camada `nome` é das edificações do GBA.
pub fn eh_do_gba(nome: &str) -> bool {
    nome.starts_with(PREFIXO_CAMADA)
}

/// O estilo das edificações, no formato do `style.add`: as classes de altura, com uma borda da
/// própria cor mais escura (× 0,6).
pub fn estilo() -> String {
    let simbolo = |c: [u8; 3]| {
        let e = |v: u8| (f64::from(v) * 0.6).round() as u8;
        serde_json::json!({"fill": [c[0], c[1], c[2], 255], "stroke": [e(c[0]), e(c[1]), e(c[2]), 255], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"})
    };
    let rotulos: serde_json::Map<String, serde_json::Value> =
        CLASSES.iter().enumerate().map(|(i, (_, r, _))| (i.to_string(), serde_json::json!(r))).collect();
    serde_json::json!([{
        "name": ESTILO, "geometry": "Polygon",
        "renderer": {"Graduated": {
            "field": "height",
            "breaks": CLASSES.iter().map(|(b, _, _)| *b).collect::<Vec<_>>(),
            "symbols": CLASSES.iter().map(|(_, _, c)| simbolo(*c)).collect::<Vec<_>>(),
            "ramp": "Ember", "method": "NaturalBreaks", "min": 0.0, "classes": CLASSES.len(),
            "class_labels": rotulos,
        }},
        "labels": {"enabled": false, "field": "height", "size": 7.5, "color": [59, 50, 38, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0,
    }])
    .to_string()
}

/// A citação, como a licença pede.
pub const CITACAO: &str = "Zhu, X. X. et al. (2025), GlobalBuildingAtlas, Earth System Science Data 17, 6647 (Universidade Técnica de Munique), polígonos e alturas de edificações de imagens PlanetScope de 2019; lido no lugar da cópia em GeoParquet do Source Cooperative (tge-labs/globalbuildingatlas-lod1), sem recorte";

pub const LICENCA: &str = "CC BY-NC 4.0 (sem uso comercial); os polígonos do OpenStreetMap, ODbL";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_nomes_dos_blocos() {
        // Viçosa (MG).
        assert_eq!(blocos([-43.003, -20.838, -42.754, -20.655]), ["w045_s20_w040_s25"]);
        // Belo Horizonte, ao norte de 20° S.
        assert_eq!(blocos([-44.06, -20.06, -43.86, -19.78]), ["w045_s15_w040_s20", "w045_s20_w040_s25"]);
        // No equador e a oeste de 50°: Macapá.
        assert_eq!(blocos([-51.2, -0.1, -50.9, 0.2]), ["w055_n05_w050_n00", "w055_n00_w050_s05"]);
        // Quatro blocos num canto.
        assert_eq!(blocos([-45.5, -20.5, -44.5, -19.5]).len(), 4);
        assert_eq!(url("w045_s20_w040_s25"), format!("{BASE}/w045_s20_w040_s25.parquet"));
    }

    #[test]
    fn a_listagem_e_as_camadas() {
        let xml = "<ListBucketResult><Contents><Key>tge-labs/globalbuildingatlas-lod1/w045_s20_w040_s25.parquet</Key></Contents></ListBucketResult>";
        assert!(existe("w045_s20_w040_s25", xml));
        assert!(!existe("w035_s20_w030_s25", xml));
        assert!(eh_do_gba(&camada("w045_s20_w040_s25", true)));
        assert!(eh_do_gba(&camada("w045_s20_w040_s25", false)));
        assert!(!eh_do_gba("BrasilData: Elevação (Copernicus GLO-30)"));
    }

    #[test]
    fn o_estilo_tem_as_classes() {
        let e: serde_json::Value = serde_json::from_str(&estilo()).unwrap();
        let g = &e[0]["renderer"]["Graduated"];
        assert_eq!(g["breaks"].as_array().unwrap().len(), 6);
        assert_eq!(g["symbols"].as_array().unwrap().len(), 6);
        assert_eq!(g["class_labels"]["0"], "até 4 m (1 pavimento)");
        assert_eq!(e[0]["name"], ESTILO);
    }
}
