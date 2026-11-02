//! O uso e a cobertura da terra do MapBiomas (Coleção 11, 1985 a 2025, 30 m).
//!
//! - **De onde:** a API da plataforma MapBiomas (`prd.plataforma.mapbiomas.org/api/v1`) devolve,
//!   para um ano, o endereço dos mosaicos do mapa no Earth Engine, já com as cores da legenda
//!   oficial. É o caminho do plugin do MapBiomas para o QGIS.
//! - **O que entra:** um mapa de mosaicos (XYZ), para ver. Ele não é recortado pela área nem tem
//!   os valores das classes: o COG da coleção é nacional, e o RustGIS 0.5.0 não lê só a janela
//!   da área de um COG remoto como raster (opengeos/RustGIS#1204).
//! - **Validade:** o endereço do Earth Engine vale por algumas horas; um novo clique pede outro.
//! - **Desktop e web:** a API e os mosaicos mandam CORS.

use serde_json::Value;

/// A API da plataforma, com a organização pública (`tenantId`), que ela exige.
const API: &str = "https://prd.plataforma.mapbiomas.org/api/v1/brazil";
const ORGANIZACAO: &str = "tenantId=mapbiomas";

/// O subtema de cobertura de 30 m e a legenda de cobertura dele.
const SUBTEMA: &str = "coverage_coverage-30m-droufo";
const LEGENDA: &str = "coverage-bypu14";

/// Os anos da coleção.
pub const PRIMEIRO_ANO: i64 = 1985;
pub const ULTIMO_ANO: i64 = 2025;

/// As classes da legenda de cobertura que vão para o mapa (as folhas da legenda, pelo valor do
/// pixel): pedir uma classe-mãe não pinta as filhas.
const CLASSES: [u32; 33] =
    [3, 6, 4, 7, 5, 49, 12, 77, 11, 84, 50, 32, 29, 15, 39, 20, 40, 62, 41, 46, 47, 35, 48, 9, 21, 23, 24, 30, 75, 91, 25, 33, 31];

/// A citação, como o MapBiomas pede.
pub const CITACAO: &str = "Projeto MapBiomas, Coleção 11 da Série Anual de Mapas de Cobertura e Uso da Terra do Brasil (1985 a 2025, 30 m), pela API da plataforma MapBiomas (brasil.mapbiomas.org). Mapa para visualização, com as cores da legenda oficial; não recortado pela área";

/// A licença dos dados do MapBiomas.
pub const LICENCA: &str = "CC BY 4.0, pelos termos do MapBiomas";

/// O endereço que pede o mapa de `ano` à API.
pub fn url_do_mapa(ano: i64) -> String {
    let classes: String = CLASSES.iter().map(|c| format!("&pixelValue={c}")).collect();
    format!("{API}/maps/map?subthemeKey={SUBTEMA}&legendKey={LEGENDA}&year={ano}{classes}&{ORGANIZACAO}")
}

/// O endereço dos mosaicos na resposta da API (`{"url": "https://…/{z}/{x}/{y}"}`).
pub fn mosaicos(resposta: &Value) -> Option<String> {
    let url = resposta.get("url")?.as_str()?;
    (url.starts_with("https://") && url.contains("{z}") && url.contains("{x}") && url.contains("{y}")).then(|| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn o_mapa_de_um_ano_pede_as_classes_e_a_organizacao() {
        let u = url_do_mapa(2025);
        assert!(u.starts_with("https://prd.plataforma.mapbiomas.org/api/v1/brazil/maps/map?subthemeKey=coverage_coverage-30m-droufo&legendKey=coverage-bypu14&year=2025&pixelValue=3&"));
        assert_eq!(u.matches("&pixelValue=").count(), CLASSES.len());
        assert!(u.ends_with("&tenantId=mapbiomas"));
    }

    #[test]
    fn a_resposta_da_os_mosaicos_ou_nada() {
        let ok = json!({"url": "https://earthengine.googleapis.com/v1/projects/mapbiomas/maps/abc/tiles/{z}/{x}/{y}"});
        assert_eq!(mosaicos(&ok).as_deref(), Some("https://earthengine.googleapis.com/v1/projects/mapbiomas/maps/abc/tiles/{z}/{x}/{y}"));
        assert_eq!(mosaicos(&json!({"url": "http://x/{z}/{x}/{y}"})), None);
        assert_eq!(mosaicos(&json!({"erro": "x"})), None);
    }
}
