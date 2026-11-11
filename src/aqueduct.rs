//! O estresse hídrico de referência do WRI Aqueduct 4.0 (a retirada de água sobre a oferta
//! renovável, `bws`), pelo serviço Aqueduct Water Risk do ArcGIS Living Atlas: um tema vetorial do
//! catálogo (`fontes.csv`, id [`TEMA`]). Os polígonos são as sub-bacias HydroBASINS de nível 6
//! cortadas pelas unidades administrativas.
//!
//! O serviço dá as categorias em inglês (`bws_label`): o plugin acrescenta o campo [`CAMPO`], com
//! elas em português, e um estilo com as cores do Aqueduct, na primeira vez que a camada chega
//! (depois, vale o estilo do usuário).

use serde_json::{Value, json};

/// O id do tema no catálogo.
pub const TEMA: &str = "aqueduct-estresse";

/// O campo com a categoria em português.
pub const CAMPO: &str = "estresse";

/// O nome do estilo no projeto.
pub const ESTILO: &str = "BrasilData: estresse hídrico (Aqueduct)";

/// As categorias do Aqueduct (`bws_cat`), o nome em português e a cor do WRI, na ordem da legenda.
/// Os nomes começam pela faixa, para a legenda ficar na ordem mesmo que o RustGIS os ordene.
pub const CATEGORIAS: [(i64, &str, &str); 7] = [
    (0, "0–10%: baixo", "#ffff99"),
    (1, "10–20%: baixo a médio", "#ffe600"),
    (2, "20–40%: médio a alto", "#ff9900"),
    (3, "40–80%: alto", "#ff1900"),
    (4, "80% ou mais: extremamente alto", "#990000"),
    (-1, "Árido e de pouco uso", "#808080"),
    (-9999, "Sem dado", "#4e4e4e"),
];

/// A expressão do `edit.calculateField` que dá o nome da categoria pelo `bws_cat`.
pub fn expressao() -> String {
    let casos: String = CATEGORIAS.iter().map(|(c, nome, _)| format!(" WHEN bws_cat = {c} THEN '{nome}'")).collect();
    format!("CASE{casos} ELSE 'Sem dado' END")
}

/// O estilo, no formato do `style.add`: as cores do Aqueduct, com uma borda fina cinza.
pub fn estilo() -> String {
    let rgba = |h: &str| {
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
        json!([c(1), c(3), c(5), 255])
    };
    let classes: Vec<Value> = CATEGORIAS
        .iter()
        .map(|(_, nome, cor)| {
            json!([nome, {"fill": rgba(cor), "stroke": [110, 110, 110, 255], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"}])
        })
        .collect();
    let nada = json!({"fill": [0, 0, 0, 0], "stroke": [110, 110, 110, 255], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"});
    json!([{"name": ESTILO, "geometry": "Polygon",
        "renderer": {"Unique": {"field": CAMPO, "classes": classes, "other": nada, "ramp": "Harbour"}},
        "labels": {"enabled": false, "field": CAMPO, "size": 7.5, "color": [59, 50, 38, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_categorias_viram_portugues_e_cores() {
        let e = expressao();
        assert!(e.starts_with("CASE WHEN bws_cat = 0 THEN '0–10%: baixo'"));
        assert!(e.contains("WHEN bws_cat = -9999 THEN 'Sem dado'") && e.ends_with(" ELSE 'Sem dado' END"));
        let s: Value = serde_json::from_str(&estilo()).unwrap();
        let classes = s[0]["renderer"]["Unique"]["classes"].as_array().unwrap();
        assert_eq!(classes.len(), 7);
        assert_eq!(classes[4][0], "80% ou mais: extremamente alto");
        assert_eq!(classes[4][1]["fill"], json!([153, 0, 0, 255]));
        assert_eq!(s[0]["renderer"]["Unique"]["field"], CAMPO);
        // As faixas em ordem de texto são a ordem da legenda.
        let faixas: Vec<&str> = CATEGORIAS[..5].iter().map(|c| c.1).collect();
        let mut ordenadas = faixas.clone();
        ordenadas.sort();
        assert_eq!(faixas, ordenadas);
    }

    #[test]
    fn o_tema_esta_no_catalogo() {
        let t = crate::TEMAS.iter().find(|t| t.id == TEMA).expect("o tema do Aqueduct no fontes.csv");
        assert_eq!(t.tipo, "arcgis");
        assert!(t.campos.split(',').any(|c| c == "bws_cat"));
    }
}
