//! O estilo que o usuário dá às camadas do BrasilData fica na camada: quando um tema vem de novo
//! (outra área, outra vista), o plugin troca a camada por uma nova, e a nova recebe a legenda, as
//! cores, os rótulos e a transparência da antiga. Durante o comando, o estilo de cada camada fica
//! guardado num estilo do projeto ([`guardado`]), que sai quando a camada volta.

use serde_json::Value;

use crate::PREFIXO;

/// O começo do nome do estilo do projeto em que o plugin guarda, durante um comando, o estilo de
/// uma camada do BrasilData.
const GUARDADO: &str = "BrasilData (estilo guardado): ";

/// O nome do estilo que guarda o da camada `nome`.
pub fn guardado(nome: &str) -> String {
    format!("{GUARDADO}{}", nome.trim_start_matches(PREFIXO))
}

/// As camadas vetoriais do BrasilData cujo estilo é do usuário, de `camadas` (o `host.layers`): não
/// as `passageiras`, as sem recorte, as tabelas, os rasters nem as cópias "nome (2)".
pub fn com_estilo(camadas: &Value, passageiras: &[&str]) -> Vec<String> {
    camadas
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["raster"] != true && !l["geometry"].is_null())
        .filter_map(|l| l["name"].as_str())
        .filter(|n| n.starts_with(PREFIXO) && !n.ends_with("(sem recorte)") && !passageiras.contains(n) && !repetida(n))
        .map(str::to_string)
        .collect()
}

/// Se `nome` é o "nome (2)" que o RustGIS dá a uma camada que chega com o nome já em uso: o plugin
/// a renomeia no fim, e o estilo dela não é do usuário.
fn repetida(nome: &str) -> bool {
    nome.strip_suffix(')').and_then(|n| n.rsplit_once(" (")).is_some_and(|(_, d)| !d.is_empty() && d.chars().all(|c| c.is_ascii_digit()))
}

/// Se a resposta `r` espera um download: o RustGIS roda o comando de novo quando ele chega, e a
/// camada nova só existe então.
pub fn esperando(r: &Value) -> bool {
    r.get("pending").and_then(Value::as_bool) == Some(true) || r.get("downloading").is_some()
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn so_as_camadas_do_usuario_guardam_estilo() {
        let camadas = json!([
            {"name": "BrasilData: Drenagem (BC250 2025)", "raster": false, "geometry": "line"},
            {"name": "BrasilData: Drenagem (BC250 2025) (sem recorte)", "raster": false, "geometry": "line"},
            {"name": "BrasilData: Drenagem (BC250 2025) (2)", "raster": false, "geometry": "line"},
            {"name": "BrasilData: área", "raster": false, "geometry": "polygon"},
            {"name": "BrasilData: área (desenho)", "raster": false, "geometry": "polygon"},
            {"name": "BrasilData: fontes", "raster": false, "geometry": null},
            {"name": "BrasilData: Elevação (Copernicus GLO-30)", "raster": true, "geometry": null},
            {"name": "Municípios do usuário", "raster": false, "geometry": "polygon"},
        ]);
        assert_eq!(com_estilo(&camadas, &["BrasilData: área (desenho)"]), ["BrasilData: Drenagem (BC250 2025)", "BrasilData: área"]);
        assert_eq!(guardado("BrasilData: área"), "BrasilData (estilo guardado): área");
        // Os números do nome do tema não são a cópia do RustGIS.
        assert!(!repetida("BrasilData: Drenagem (BC250 2025)") && !repetida("BrasilData: x ()"));
        assert!(repetida("BrasilData: x (12)"));
    }

    #[test]
    fn um_download_no_meio_espera() {
        assert!(esperando(&json!({"pending": true, "url": "u"})));
        assert!(esperando(&json!({"downloading": "u"})));
        assert!(!esperando(&json!({"layer": "BrasilData: área", "pending": false})));
    }
}
