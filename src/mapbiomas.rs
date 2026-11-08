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
//!
//! **Recortado pela área.** Com uma área e um RustGIS que lê a janela de um COG remoto como
//! raster (`layer.addUrl` com `bbox`, opengeos/RustGIS#1210, depois da 0.5.0), os anos do espelho
//! do Source Cooperative (Coleção 10, 2019 a 2024, um COG por ano em EPSG:4326, com a tabela de
//! cores e os nomes das classes) vêm como raster de classes: só a janela da área, recortada pelo
//! contorno e posta na grade do projeto com a classe mais frequente (`mode`). O bucket oficial
//! do MapBiomas recusa acesso anônimo desde outubro de 2026. Fora disso, o mapa de mosaicos.

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

/// O controle do painel que escolhe o ano (de `PRIMEIRO_ANO` a `ULTIMO_ANO`).
pub const CONTROLE: &str = "mapbiomas-ano";

/// Os anos que o espelho do Source Cooperative tem (Coleção 10).
pub const ESPELHO_PRIMEIRO: i64 = 2019;
pub const ESPELHO_ULTIMO: i64 = 2024;

/// O COG de `ano` no espelho do Source Cooperative (cerca de 1,2 GB, lido por partes).
pub fn url_do_cog(ano: i64) -> String {
    format!("https://data.source.coop/tristangruppwri/mapbiomas/mapbiomas-brazil/mapbiomas_brazil_{ano}/mapbiomas_brazil_{ano}.tif")
}

/// Se o espelho tem `ano`.
pub fn no_espelho(ano: i64) -> bool {
    (ESPELHO_PRIMEIRO..=ESPELHO_ULTIMO).contains(&ano)
}

/// A célula do COG, em graus (30 m no equador).
const CELULA_GRAUS: f64 = 0.000_269_494_585_235_856_4;
/// A célula do COG, em metros, como o RustGIS a diz (no equador).
const CELULA_M: f64 = 30.000_137_228_455_536;
/// Quantos valores uma janela pode ter: o RustGIS lê até 64 milhões; abaixo disso, com folga.
const MAX_VALORES: f64 = 60_000_000.0;
/// A margem em volta do retângulo da área, em graus (cerca de 7 células): o recorte pelo
/// contorno e a projeção na grade não ficam sem dados na borda.
pub const MARGEM: f64 = 0.002;

/// A célula a pedir para a janela do retângulo `c` (com a margem), em metros: `None` quando a
/// janela de 30 m cabe; senão, a da primeira pirâmide do COG em que ela cabe (60 m, 120 m...),
/// arredondada para cima. O RustGIS lê o nível mais fino com célula até a pedida, e as pirâmides
/// do espelho tomam a classe mais frequente.
pub fn celula_da_janela(c: [f64; 4]) -> Option<f64> {
    let mut valores = ((c[2] - c[0] + 2.0 * MARGEM) / CELULA_GRAUS) * ((c[3] - c[1] + 2.0 * MARGEM) / CELULA_GRAUS);
    let mut celula = CELULA_M;
    while valores > MAX_VALORES {
        valores /= 4.0;
        celula *= 2.0;
    }
    (celula > CELULA_M).then(|| celula.ceil())
}

/// O retângulo da janela: o da área com a margem.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - MARGEM, c[1] - MARGEM, c[2] + MARGEM, c[3] + MARGEM]
}

/// A citação do recorte, como o MapBiomas pede.
pub const CITACAO_RECORTE: &str = "Projeto MapBiomas, Coleção 10 da Série Anual de Mapas de Cobertura e Uso da Terra do Brasil (30 m), pelo espelho do Source Cooperative (data.source.coop/tristangruppwri/mapbiomas). Recortado pela área e posto na grade do projeto com a classe mais frequente; os valores são os códigos da legenda do MapBiomas";

/// A licença do recorte.
pub const LICENCA_RECORTE: &str = "CC BY 4.0, pelos termos do MapBiomas (o espelho declara CC BY-SA 4.0)";

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
    fn a_janela_de_um_municipio_e_de_30_m_e_a_de_um_estado_vem_de_uma_piramide() {
        // Viçosa (MG): cerca de 0,25° por 0,23°, menos de 1 milhão de células.
        assert_eq!(celula_da_janela([-43.0, -20.85, -42.75, -20.62]), None);
        // Minas Gerais: cerca de 12,5° por 9,5°, muito acima de 60 milhões.
        // A Zona da Mata (MG): 10.450 × 8.939 células de 30 m, acima de 64 milhões; o RustGIS
        // pediu "cellSize": 61 ou mais, a pirâmide de 60 m.
        assert_eq!(celula_da_janela([-43.37, -22.92, -40.56, -20.52]), Some(61.0));
        // Minas Gerais: cerca de 11° por 9°: a pirâmide de 240 m (cerca de 21 milhões de células).
        assert_eq!(celula_da_janela([-51.05, -22.93, -39.85, -14.23]), Some(241.0));
        let j = janela([-43.0, -20.85, -42.75, -20.62]);
        for (v, e) in j.iter().zip([-43.002, -20.852, -42.748, -20.618]) {
            assert!((v - e).abs() < 1e-9, "{j:?}");
        }
    }

    #[test]
    fn o_espelho_tem_de_2019_a_2024() {
        assert!(!no_espelho(2018) && no_espelho(2019) && no_espelho(2024) && !no_espelho(2025));
        assert_eq!(
            url_do_cog(2024),
            "https://data.source.coop/tristangruppwri/mapbiomas/mapbiomas-brazil/mapbiomas_brazil_2024/mapbiomas_brazil_2024.tif"
        );
    }

    #[test]
    fn a_resposta_da_os_mosaicos_ou_nada() {
        let ok = json!({"url": "https://earthengine.googleapis.com/v1/projects/mapbiomas/maps/abc/tiles/{z}/{x}/{y}"});
        assert_eq!(mosaicos(&ok).as_deref(), Some("https://earthengine.googleapis.com/v1/projects/mapbiomas/maps/abc/tiles/{z}/{x}/{y}"));
        assert_eq!(mosaicos(&json!({"url": "http://x/{z}/{x}/{y}"})), None);
        assert_eq!(mosaicos(&json!({"erro": "x"})), None);
    }
}
