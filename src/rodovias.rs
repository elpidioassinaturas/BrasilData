//! As rodovias e estradas, de duas fontes que se completam:
//!
//! - **IBGE, BC250 2025** (`rodovias`, no `fontes.csv`): os trechos rodoviários da Base
//!   Cartográfica Contínua 1:250.000, pelo WFS do IBGE, com a jurisdição (federal, estadual,
//!   municipal), o revestimento (pavimentado, revestimento primário, leito natural) e a sigla
//!   (BR-120, MG-280). São 124.687 trechos no país (22/11/2026): numa área, recortados por ela; com
//!   a área do Brasil, o país inteiro em páginas. A mesma base da drenagem do plugin, e os mapas
//!   regionais e do país saem dela.
//! - **OpenStreetMap, pelo Overture Maps** (`osm-vias`): as ruas e estradas do OSM, do tema de
//!   transportes do Overture (os segmentos), que o RustGIS 0.7.0 lê no lugar (`overture.add`), só
//!   o que a vista mostra, dos arquivos GeoParquet que cobrem a área. É o detalhe local: todas as
//!   ruas da cidade e as estradas de terra que o OSM tem. Não é recortada (são milhões de trechos
//!   num arquivo): a camada é mascarada pelo contorno da área. O RustGIS lê no máximo 60 mil
//!   feições por vista, e de longe a amostra deixa as vias em pedaços: a camada só aparece de
//!   [`OSM_MAIS_LONGE`] para perto. De longe, a BC250. Os tiles PMTiles do Overture não servem de
//!   longe: até o zoom 9 eles vêm sem a classe da via (conferido em 22/11/2026).
//!
//! As duas camadas têm o estilo por classe num estilo do projeto (`style.add`, Valores Únicos por
//! uma expressão): o `layer.symbology` não acha as classes numa camada lida no lugar, e o estilo
//! serve às duas. Os filtros de exibição (`layer.displayFilters`) deixam de longe só as vias
//! principais.

use serde_json::{Value, json};

/// Uma classe de via: o nome (a legenda), a condição (no SQL do RustGIS), a cor, a largura (pt) e
/// o tracejado (pt), se houver.
pub struct Classe {
    pub nome: &'static str,
    pub condicao: &'static str,
    pub cor: &'static str,
    pub largura: f64,
    pub tracejado: Option<[f64; 2]>,
}

/// As classes da BC250, da mais importante para a menos: a primeira condição que vale dá a classe.
pub const CLASSES_BC250: &[Classe] = &[
    Classe {
        nome: "Rodovia federal pavimentada",
        condicao: "jurisdicao = 'Federal' AND revestimento = 'Pavimentado'",
        cor: "#c8102e",
        largura: 1.6,
        tracejado: None,
    },
    Classe {
        nome: "Rodovia estadual pavimentada",
        condicao: "jurisdicao = 'Estadual/Distrital' AND revestimento = 'Pavimentado'",
        cor: "#e8590c",
        largura: 1.2,
        tracejado: None,
    },
    Classe { nome: "Outra rodovia pavimentada", condicao: "revestimento = 'Pavimentado'", cor: "#e0a100", largura: 0.9, tracejado: None },
    Classe {
        nome: "Rodovia federal ou estadual sem pavimento",
        condicao: "jurisdicao IN ('Federal', 'Estadual/Distrital')",
        cor: "#9c4a1c",
        largura: 1.0,
        tracejado: Some([4.0, 1.5]),
    },
    Classe { nome: "Estrada sem pavimento", condicao: "TRUE", cor: "#6e4f2e", largura: 0.8, tracejado: Some([3.0, 1.5]) },
];

/// As classes do OpenStreetMap, pelo `class` dos segmentos do Overture (o `highway` do OSM).
pub const CLASSES_OSM: &[Classe] = &[
    Classe {
        nome: "Autoestradas e vias expressas",
        condicao: "class IN ('motorway', 'trunk')",
        cor: "#c8102e",
        largura: 1.6,
        tracejado: None,
    },
    Classe { nome: "Vias principais", condicao: "class = 'primary'", cor: "#e8590c", largura: 1.3, tracejado: None },
    Classe { nome: "Vias secundárias", condicao: "class = 'secondary'", cor: "#e0a100", largura: 1.1, tracejado: None },
    Classe {
        nome: "Vias terciárias e estradas vicinais",
        condicao: "class IN ('tertiary', 'unclassified')",
        cor: "#8a6d3b",
        largura: 0.8,
        tracejado: None,
    },
    Classe {
        nome: "Ruas",
        condicao: "class IN ('residential', 'living_street', 'service', 'pedestrian')",
        cor: "#8c8c8c",
        largura: 0.5,
        tracejado: None,
    },
    Classe {
        nome: "Estradas de terra (trilhas de veículos)",
        condicao: "class = 'track'",
        cor: "#9c7a4c",
        largura: 0.6,
        tracejado: Some([3.0, 1.5]),
    },
    Classe {
        nome: "Trilhas e caminhos",
        condicao: "class IN ('path', 'footway', 'cycleway', 'steps', 'bridleway')",
        cor: "#a08c6e",
        largura: 0.4,
        tracejado: Some([1.5, 1.5]),
    },
    Classe { nome: "Outras vias", condicao: "TRUE", cor: "#b4b4b4", largura: 0.4, tracejado: None },
];

/// O nome do estilo das rodovias da BC250 no projeto.
pub const ESTILO_BC250: &str = "BrasilData: rodovias por jurisdição e revestimento (BC250)";

/// O nome do estilo das vias do OpenStreetMap no projeto.
pub const ESTILO_OSM: &str = "BrasilData: ruas e estradas por classe (OpenStreetMap)";

/// O nome da camada do OpenStreetMap (com vários arquivos, uma por arquivo, " (parte N)").
pub const CAMADA_OSM: &str = "BrasilData: Ruas e estradas (OpenStreetMap, Overture)";

/// Mais longe que esta escala (1:N) a camada do OpenStreetMap some: a amostra de 60 mil feições por
/// vista deixaria as vias em pedaços (numa microrregião a 1:430.000, conferido em 22/11/2026).
pub const OSM_MAIS_LONGE: f64 = 150_000.0;

/// As faixas de escala da BC250, de longe para perto: `(denominador, nome, condição)` diz que, de
/// 1:`denominador` para longe (até a faixa anterior), só aparecem os trechos da condição. Mais perto
/// que a última faixa, aparece tudo.
const FAIXAS_BC250: &[(f64, &str, &str)] = &[
    (10_000_000.0, "Só as rodovias federais", "jurisdicao = 'Federal'"),
    (
        4_000_000.0,
        "Federais e estaduais pavimentadas",
        "jurisdicao = 'Federal' OR (jurisdicao = 'Estadual/Distrital' AND revestimento = 'Pavimentado')",
    ),
    (
        1_500_000.0,
        "Pavimentadas, e as federais e estaduais sem pavimento",
        "revestimento = 'Pavimentado' OR jurisdicao IN ('Federal', 'Estadual/Distrital')",
    ),
];

/// As faixas do OpenStreetMap, dentro da escala em que a camada aparece ([`OSM_MAIS_LONGE`]).
const FAIXAS_OSM: &[(f64, &str, &str)] = &[(
    50_000.0,
    "Sem as trilhas, os caminhos e as vias de serviço",
    "class NOT IN ('path', 'footway', 'cycleway', 'steps', 'bridleway', 'service', 'pedestrian') OR class IS NULL",
)];

/// A escala (1:N) de onde para perto aparecem as siglas das rodovias da BC250.
pub const SIGLAS_MAIS_LONGE: f64 = 2_000_000.0;

/// O texto dos rótulos da BC250: a sigla das rodovias federais e estaduais pavimentadas (as outras
/// têm sigla em poucos trechos, e encheriam o mapa de BR repetidas).
pub const ROTULO_BC250: &str = "CASE WHEN revestimento = 'Pavimentado' AND jurisdicao IN ('Federal', 'Estadual/Distrital') THEN sigla END";

/// O valor da classe de índice `i` na expressão: "1", "2"... O RustGIS põe as classes dos
/// Valores Únicos na legenda pela ordem do valor; com o número, a ordem é a da importância, e o
/// nome vai no rótulo da legenda (`class_labels`). Até 9 classes, a ordem do texto é a do número.
pub fn valor(i: usize) -> String {
    (i + 1).to_string()
}

/// A expressão que dá o [`valor`] da classe de cada trecho.
pub fn expressao(classes: &[Classe]) -> String {
    let mut e = String::from("CASE");
    for (i, c) in classes.iter().enumerate() {
        if c.condicao == "TRUE" {
            e.push_str(&format!(" ELSE '{}' END", valor(i)));
            return e;
        }
        e.push_str(&format!(" WHEN {} THEN '{}'", c.condicao, valor(i)));
    }
    e.push_str(" END");
    e
}

fn rgba(h: &str) -> Value {
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    json!([c(1), c(3), c(5), 255])
}

fn simbolo(c: &Classe) -> Value {
    json!({"fill": rgba(c.cor), "stroke": rgba(c.cor), "stroke_width": c.largura, "size": 0.0, "marker": "Circle"})
}

/// O estilo, no formato do `style.add`: Valores Únicos pela [`expressao`] das `classes`, na ordem
/// delas (a da legenda), com o nome de cada uma no rótulo.
pub fn estilo(nome: &str, classes: &[Classe], campo: &str) -> String {
    let lista: Vec<Value> = classes.iter().enumerate().map(|(i, c)| json!([valor(i), simbolo(c)])).collect();
    let rotulos: serde_json::Map<String, Value> = classes.iter().enumerate().map(|(i, c)| (valor(i), json!(c.nome))).collect();
    let outro = simbolo(classes.last().expect("ao menos uma classe"));
    json!([{"name": nome, "geometry": "Polyline",
        "renderer": {"Unique": {"field": campo, "expression": expressao(classes), "classes": lista, "other": outro, "ramp": "Harbour",
            "class_labels": rotulos}},
        "labels": {"enabled": false, "field": campo, "size": 6.5, "color": [60, 60, 60, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

/// O traço de cada classe (`layer.lineStrokes` com o `value`): a largura, o tracejado e o nível,
/// para as vias principais passarem por cima das outras (`layer.strokeLevels`). Pontas e cantos
/// redondos: os trechos se emendam sem falhas.
pub fn tracos(classes: &[Classe]) -> Vec<Value> {
    let n = classes.len();
    classes
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut t = json!({"width": c.largura, "cap": "round", "join": "round", "level": n - i});
            if let Some(d) = c.tracejado {
                t["dash"] = json!(d);
                t["cap"] = json!("flat");
            }
            json!({"value": valor(i), "strokes": [t]})
        })
        .collect()
}

/// Os filtros de exibição das `faixas`; `mais_longe` é a escala de onde a camada aparece, se
/// houver: a primeira faixa vale dela para perto.
fn filtros(faixas: &[(f64, &str, &str)], mais_longe: Option<f64>) -> Value {
    let mut longe = mais_longe;
    let mut v = Vec::new();
    for &(perto, nome, condicao) in faixas {
        let mut f = json!({"name": nome, "where": condicao, "maxScale": perto});
        if let Some(l) = longe {
            f["minScale"] = json!(l);
        }
        v.push(f);
        longe = Some(perto);
    }
    Value::Array(v)
}

/// Os filtros por escala da BC250.
pub fn filtros_bc250() -> Value {
    filtros(FAIXAS_BC250, None)
}

/// Os filtros por escala do OpenStreetMap.
pub fn filtros_osm() -> Value {
    filtros(FAIXAS_OSM, Some(OSM_MAIS_LONGE))
}

/// O nome da camada do OpenStreetMap de cada um de `n` arquivos.
pub fn camadas_osm(n: usize) -> Vec<String> {
    if n <= 1 { vec![CAMADA_OSM.to_string()] } else { (1..=n).map(|i| format!("{CAMADA_OSM} (parte {i})")).collect() }
}

/// Se a camada `nome` é das vias do OpenStreetMap.
pub fn eh_do_osm(nome: &str) -> bool {
    nome.starts_with(CAMADA_OSM)
}

/// A citação das vias do OpenStreetMap, como a ODbL pede.
pub const CITACAO_OSM: &str = "© colaboradores do OpenStreetMap; segmentos do tema de transportes do Overture Maps Foundation, lidos no lugar (GeoParquet), sem recorte";

pub const LICENCA_OSM: &str = "ODbL 1.0 (Open Database License)";

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn a_expressao_da_bc250() {
        let e = expressao(CLASSES_BC250);
        assert!(e.starts_with("CASE WHEN jurisdicao = 'Federal' AND revestimento = 'Pavimentado' THEN '1'"));
        assert!(e.ends_with(" ELSE '5' END"));
        // Uma condição por classe, menos a última, que é o ELSE.
        assert_eq!(e.matches(" WHEN ").count(), CLASSES_BC250.len() - 1);
        let o = expressao(CLASSES_OSM);
        assert!(o.contains("WHEN class = 'track' THEN '6'") && o.ends_with("ELSE '8' END"));
        // A ordem do texto dos valores é a dos números (a da legenda).
        assert!(CLASSES_BC250.len() <= 9 && CLASSES_OSM.len() <= 9);
    }

    #[test]
    fn o_estilo_tem_as_classes_na_ordem() {
        let s: Value = serde_json::from_str(&estilo(ESTILO_BC250, CLASSES_BC250, "jurisdicao")).unwrap();
        let u = &s[0]["renderer"]["Unique"];
        assert_eq!(u["classes"].as_array().unwrap().len(), CLASSES_BC250.len());
        assert_eq!(u["classes"][0][0], "1");
        assert_eq!(u["class_labels"]["1"], "Rodovia federal pavimentada");
        assert_eq!(u["class_labels"]["5"], "Estrada sem pavimento");
        assert_eq!(u["classes"][0][1]["stroke"], json!([200, 16, 46, 255]));
        assert_eq!(u["expression"], expressao(CLASSES_BC250));
        assert_eq!(s[0]["geometry"], "Polyline");
        // Só a última classe é o ELSE, e todo nome cabe numa string SQL sem aspas a escapar.
        for c in &CLASSES_BC250[..CLASSES_BC250.len() - 1] {
            assert_ne!(c.condicao, "TRUE");
        }
        for c in CLASSES_BC250.iter().chain(CLASSES_OSM) {
            assert!(!c.nome.contains('\''));
        }
    }

    #[test]
    fn os_tracos_poem_as_principais_por_cima() {
        let t = tracos(CLASSES_OSM);
        assert_eq!(t.len(), CLASSES_OSM.len());
        assert_eq!(t[0]["strokes"][0]["level"], CLASSES_OSM.len());
        assert_eq!(t.last().unwrap()["strokes"][0]["level"], 1);
        assert_eq!(t[5]["strokes"][0]["dash"], json!([3.0, 1.5]));
        assert!(t[0]["strokes"][0].get("dash").is_none());
    }

    #[test]
    fn os_filtros_por_escala() {
        let b = filtros_bc250();
        let b = b.as_array().unwrap();
        assert_eq!(b.len(), FAIXAS_BC250.len());
        assert!(b[0].get("minScale").is_none());
        assert_eq!(b[0]["where"], "jurisdicao = 'Federal'");
        for par in b.windows(2) {
            assert_eq!(par[1]["minScale"], par[0]["maxScale"]);
        }
        let o = filtros_osm();
        assert_eq!(o[0]["minScale"], OSM_MAIS_LONGE);
        assert_eq!(o[0]["maxScale"], 50_000.0);
    }

    #[test]
    fn as_camadas_do_osm() {
        assert_eq!(camadas_osm(1), [CAMADA_OSM]);
        let v = camadas_osm(2);
        assert_eq!(v[1], format!("{CAMADA_OSM} (parte 2)"));
        assert!(v.iter().all(|n| eh_do_osm(n)));
        assert!(!eh_do_osm("BrasilData: Rodovias (BC250 2025)"));
    }
}
