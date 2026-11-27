//! Os aeroportos e os portos: os aeródromos públicos do cadastro da ANAC e as instalações
//! portuárias da ANTAQ, nas cópias publicadas pela Infra S.A. (geo.infrasa.gov.br, ArcGIS com
//! CORS), cada tipo num símbolo.

use serde_json::{Value, json};

/// Os tipos de aeródromo (o `vhr_tipo` da ANAC) e a cor e o tamanho de cada um.
const AEROPORTOS: &[(&str, [u8; 3], f64)] = &[
    ("Aeroporto Público com Voos Internacionais", [128, 0, 38], 9.0),
    ("Aeródromo Concedido com voos Internacionais", [128, 0, 38], 9.0),
    ("Aeroporto Público com Voos Regulares", [227, 26, 28], 7.5),
    ("Aeródromo Concedido", [252, 78, 42], 7.0),
    ("Aeródromo Público", [253, 141, 60], 5.5),
];

/// Os tipos de instalação portuária (o `tipo` da ANTAQ) e a cor e o tamanho de cada um.
const PORTOS: &[(&str, [u8; 3], f64)] = &[
    ("Porto Organizado", [8, 48, 107], 8.5),
    ("Terminal de uso privado", [33, 113, 181], 6.5),
    ("ETC", [107, 174, 214], 6.0),
    ("IP4", [158, 202, 225], 5.0),
];

/// O campo do tipo, as classes e o marcador do tema `id`.
/// Um tipo: o valor, a cor e o tamanho do símbolo.
type Classe = (&'static str, [u8; 3], f64);

fn classes(id: &str) -> Option<(&'static str, &'static [Classe], &'static str)> {
    match id {
        "aeroportos" => Some(("vhr_tipo", AEROPORTOS, "Triangle")),
        "portos" => Some(("tipo", PORTOS, "Square")),
        _ => None,
    }
}

/// O estilo do tema `id`: um símbolo por tipo, com o nome como rótulo.
pub fn estilo_do_tema(id: &str) -> Option<(String, String)> {
    let (campo, lista, marcador) = classes(id)?;
    let nome = format!("BrasilData: {id} (por tipo)");
    let simbolo = |c: [u8; 3], tamanho: f64| json!({"fill": [c[0], c[1], c[2], 255], "stroke": [255, 255, 255, 255], "stroke_width": 0.6, "size": tamanho, "marker": marcador});
    let classes: Vec<Value> = lista.iter().map(|(v, c, t)| json!([v, simbolo(*c, *t)])).collect();
    let texto = json!([{"name": nome, "geometry": "Point",
        "renderer": {"Unique": {"field": campo, "classes": classes, "other": simbolo([150, 150, 150], 5.0), "ramp": "Harbour"}},
        "labels": {"enabled": true, "field": "nome", "size": 7.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    Some((nome, texto))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn estilos_dos_dois_temas() {
        for id in ["aeroportos", "portos"] {
            let (_, texto) = estilo_do_tema(id).unwrap();
            let v: Value = serde_json::from_str(&texto).unwrap();
            assert_eq!(v[0]["geometry"], "Point");
            assert!(crate::TEMAS.iter().any(|t| t.id == id), "{id} fora do fontes.csv");
        }
        assert!(estilo_do_tema("ucs").is_none());
    }
}
