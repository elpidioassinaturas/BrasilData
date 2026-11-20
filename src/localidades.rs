//! O estilo das localidades do IBGE (o tema `localidades`, BC250): uma cor por tipo, e cada tipo
//! só a partir de uma escala da vista, para o mapa não se encher de pontos de longe.
//!
//! - **Como:** valores únicos por expressão (o `expression` do renderizador do RustGIS), que lê a
//!   escala do desenho (`@map_scale`): a cidade sempre; a vila e o núcleo urbano de 1:2.000.000
//!   para perto; os aglomerados rurais de 1:500.000 para perto. Fora da escala, a expressão dá
//!   nulo e o ponto cai no "outro", que não se desenha.
//! - **A escala** é a da imagem desenhada: na janela, a da barra de status; numa exportação de
//!   outro tamanho, a da imagem exportada.

use serde_json::{Value, json};

/// O id do tema.
pub const TEMA: &str = "localidades";

/// O nome do estilo no projeto.
pub const ESTILO: &str = "BrasilData: localidades por escala";

/// A escala, de perto para longe, até onde a vila e o núcleo urbano aparecem.
pub const ESCALA_VILAS: f64 = 2_000_000.0;

/// A escala até onde os aglomerados rurais aparecem.
pub const ESCALA_AGLOMERADOS: f64 = 500_000.0;

/// A classe de cada ponto, pela escala do desenho.
pub fn expressao() -> String {
    format!(
        "CASE WHEN \"tipolocalidade\" = 'Cidade' THEN 'Cidade' \
         WHEN \"tipolocalidade\" IN ('Vila', 'Núcleo Urbano') AND @map_scale <= {ESCALA_VILAS} THEN \"tipolocalidade\" \
         WHEN @map_scale <= {ESCALA_AGLOMERADOS} THEN \"tipolocalidade\" END"
    )
}

/// As classes: o valor, o rótulo na legenda, a cor e o tamanho (pt).
pub const CLASSES: [(&str, &str, [u8; 3], f64); 5] = [
    ("Cidade", "Cidade (sede municipal)", [165, 15, 21], 7.0),
    ("Vila", "Vila (de 1:2.000.000 para perto)", [239, 101, 72], 5.0),
    ("Núcleo Urbano", "Núcleo urbano (de 1:2.000.000 para perto)", [252, 146, 114], 4.5),
    ("Aglomerado rural", "Aglomerado rural (de 1:500.000 para perto)", [33, 102, 172], 4.0),
    ("Aglomerado rural isolado", "Aglomerado rural isolado (de 1:500.000 para perto)", [67, 147, 195], 3.5),
];

/// O estilo, no formato do `style.add`.
pub fn estilo() -> String {
    let simbolo = |c: [u8; 3], tamanho: f64| json!({"fill": [c[0], c[1], c[2], 255], "stroke": [255, 255, 255, 255], "stroke_width": 0.6, "size": tamanho, "marker": "Circle"});
    let nada = json!({"fill": [0, 0, 0, 0], "stroke": [0, 0, 0, 0], "stroke_width": 0.0, "size": 0.0, "marker": "Circle"});
    let classes: Vec<Value> = CLASSES.iter().map(|(v, _, c, t)| json!([v, simbolo(*c, *t)])).collect();
    let rotulos: serde_json::Map<String, Value> = CLASSES.iter().map(|(v, r, _, _)| ((*v).to_string(), json!(r))).collect();
    json!([{
        "name": ESTILO, "geometry": "Point",
        "renderer": {"Unique": {
            "field": "", "classes": classes, "other": nada, "ramp": "Harbour", "expression": expressao(),
            "class_labels": rotulos, "other_label": "fora da escala de ver",
        }},
        "labels": {"enabled": false, "field": "identificador", "size": 7.5, "color": [59, 50, 38, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0,
    }])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_estilo_por_escala() {
        let e: Value = serde_json::from_str(&estilo()).unwrap();
        let u = &e[0]["renderer"]["Unique"];
        assert_eq!(u["classes"].as_array().unwrap().len(), 5);
        let x = u["expression"].as_str().unwrap();
        assert!(x.contains("@map_scale <= 2000000") && x.contains("@map_scale <= 500000"), "{x}");
        assert_eq!(u["class_labels"]["Cidade"], "Cidade (sede municipal)");
        assert_eq!(e[0]["name"], ESTILO);
    }
}
