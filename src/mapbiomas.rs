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

use serde_json::{Value, json};

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

/// As classes da Coleção 10 no COG do espelho, na ordem da legenda oficial: o código, o nome em
/// português (Legenda-Colecao-10-Legend-Code.pdf, que o arquivo cita) e a cor da tabela de cores do
/// próprio arquivo (lida com o gdalinfo em 09/11/2026), a mesma que o raster usa no mapa.
pub const CLASSES_10: [(u8, &str, &str); 38] = [
    (1, "Floresta", "#1f8d49"),
    (3, "Formação Florestal", "#1f8d49"),
    (4, "Formação Savânica", "#7dc975"),
    (5, "Mangue", "#04381d"),
    (6, "Floresta Alagável", "#007785"),
    (49, "Restinga Arbórea", "#02d659"),
    (10, "Vegetação Herbácea e Arbustiva", "#d6bc74"),
    (11, "Campo Alagado e Área Pantanosa", "#519799"),
    (12, "Formação Campestre", "#d6bc74"),
    (32, "Apicum", "#fc8114"),
    (29, "Afloramento Rochoso", "#ffaa5f"),
    (50, "Restinga Herbácea", "#ad5100"),
    (14, "Agropecuária", "#ffefc3"),
    (15, "Pastagem", "#edde8e"),
    (18, "Agricultura", "#e974ed"),
    (19, "Lavoura Temporária", "#c27ba0"),
    (39, "Soja", "#f5b3c8"),
    (20, "Cana", "#db7093"),
    (40, "Arroz", "#c71585"),
    (62, "Algodão (beta)", "#ff69b4"),
    (41, "Outras Lavouras Temporárias", "#f54ca9"),
    (36, "Lavoura Perene", "#d082de"),
    (46, "Café", "#d68fe2"),
    (47, "Citrus", "#9932cc"),
    (35, "Dendê", "#9065d0"),
    (48, "Outras Lavouras Perenes", "#e6ccff"),
    (9, "Silvicultura", "#7a5900"),
    (21, "Mosaico de Usos", "#ffefc3"),
    (22, "Área não Vegetada", "#d4271e"),
    (23, "Praia, Duna e Areal", "#ffa07a"),
    (24, "Área Urbanizada", "#d4271e"),
    (30, "Mineração", "#9c0027"),
    (75, "Usina Fotovoltaica (beta)", "#c12100"),
    (25, "Outras Áreas não Vegetadas", "#db4d4f"),
    (26, "Corpo d’Água", "#2532e4"),
    (33, "Rio, Lago e Oceano", "#2532e4"),
    (31, "Aquicultura", "#091077"),
    (27, "Não Observado", "#ffffff"),
];

/// O nome do estilo das classes no projeto.
pub const ESTILO_DAS_CLASSES: &str = "BrasilData: classes do MapBiomas";

/// Quantas células, no máximo, a camada das classes vetoriza: acima disso, uma cópia de células
/// maiores, com a classe mais frequente (o mesmo teto das manchas do DIST).
pub const MAX_CELULAS_CLASSES: f64 = 5.0e6;

/// A camada das classes de `ano`, polígonos logo abaixo do raster recortado. O nome dela é o
/// título das classes na legenda do layout, que mostra o nome da camada (opengeos/RustGIS#1654):
/// por isso vai sem o prefixo do plugin e diz o que o mapa mostra.
pub fn nome_das_classes(ano: i64) -> String {
    format!("Uso e cobertura da terra em {ano} (MapBiomas)")
}

/// Se `nome` é o de uma camada de classes ([`nome_das_classes`]).
pub fn eh_das_classes(nome: &str) -> bool {
    nome.strip_prefix("Uso e cobertura da terra em ")
        .and_then(|r| r.strip_suffix(" (MapBiomas)"))
        .is_some_and(|ano| ano.len() == 4 && ano.bytes().all(|b| b.is_ascii_digit()))
}

/// A célula, em metros, que a camada das classes vetoriza num retângulo `c` ([oeste, sul, leste,
/// norte], em graus) com células de `celula` metros: a própria, ou um múltiplo dela que deixe o
/// retângulo abaixo de [`MAX_CELULAS_CLASSES`].
pub fn celula_das_classes(c: [f64; 4], celula: f64) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let (largura, altura) = ((c[2] - c[0]) * 111_320.0 * lat.cos(), (c[3] - c[1]) * 110_570.0);
    let n = largura * altura / (celula * celula);
    if n <= MAX_CELULAS_CLASSES { celula } else { celula * (n / MAX_CELULAS_CLASSES).sqrt().ceil() }
}

/// A expressão do `edit.calculateField` que dá ao campo `classe` o nome da classe de `VALUE`.
pub fn expressao_das_classes() -> String {
    let casos: String = CLASSES_10.iter().map(|(v, nome, _)| format!(" WHEN VALUE = {v} THEN '{nome}'")).collect();
    format!("CASE{casos} ELSE '' END")
}

/// O estilo das classes (polígonos, campo `classe`), no formato do `style.add`: a cor da legenda
/// oficial, sem borda, na ordem da legenda. A legenda do layout lista só as classes que o mapa
/// desenha.
pub fn estilo_das_classes() -> String {
    let rgba = |h: &str| {
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
        json!([c(1), c(3), c(5), 255])
    };
    let classes: Vec<Value> = CLASSES_10
        .iter()
        .map(|(_, nome, cor)| {
            json!([nome, {"fill": rgba(cor), "stroke": [0, 0, 0, 0], "stroke_width": 0.0, "size": 0.0, "marker": "Circle"}])
        })
        .collect();
    let nada = json!({"fill": [0, 0, 0, 0], "stroke": [0, 0, 0, 0], "stroke_width": 0.0, "size": 0.0, "marker": "Circle"});
    json!([{"name": ESTILO_DAS_CLASSES, "geometry": "Polygon",
        "renderer": {"Unique": {"field": "classe", "classes": classes, "other": nada, "ramp": "Harbour"}},
        "labels": {"enabled": false, "field": "classe", "size": 7.5, "color": [59, 50, 38, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
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

    #[test]
    fn as_classes_tem_codigo_unico_e_a_cor_do_arquivo() {
        let mut codigos: Vec<u8> = CLASSES_10.iter().map(|c| c.0).collect();
        codigos.sort_unstable();
        codigos.dedup();
        assert_eq!(codigos.len(), CLASSES_10.len());
        // Algumas cores da tabela do COG de 2024.
        let cor = |v: u8| CLASSES_10.iter().find(|c| c.0 == v).map(|c| c.2);
        assert_eq!(cor(3), Some("#1f8d49"));
        assert_eq!(cor(15), Some("#edde8e"));
        assert_eq!(cor(24), Some("#d4271e"));
        assert_eq!(cor(33), Some("#2532e4"));
        assert!(CLASSES_10.iter().all(|c| c.2.len() == 7 && c.2.starts_with('#') && !c.1.contains('\'')));
    }

    #[test]
    fn a_expressao_e_o_estilo_dao_nome_e_cor_a_cada_classe() {
        let e = expressao_das_classes();
        assert!(e.starts_with("CASE WHEN VALUE = 1 THEN 'Floresta' WHEN VALUE = 3 THEN 'Formação Florestal'"));
        assert!(e.ends_with(" ELSE '' END"));
        let s: Value = serde_json::from_str(&estilo_das_classes()).unwrap();
        let u = &s[0]["renderer"]["Unique"];
        assert_eq!(u["field"], "classe");
        assert_eq!(u["classes"].as_array().unwrap().len(), CLASSES_10.len());
        assert_eq!(
            u["classes"][1],
            json!(["Formação Florestal", {"fill": [31, 141, 73, 255], "stroke": [0, 0, 0, 0],
            "stroke_width": 0.0, "size": 0.0, "marker": "Circle"}])
        );
        assert_eq!(s[0]["name"], ESTILO_DAS_CLASSES);
    }

    #[test]
    fn as_classes_de_uma_area_grande_vem_de_celulas_maiores() {
        // Viçosa (MG), 30 m: cerca de 700 mil células, a própria célula.
        assert_eq!(celula_das_classes([-43.0, -20.85, -42.75, -20.62], 30.0), 30.0);
        // A Zona da Mata (MG), 30 m: cerca de 90 milhões de células; 5 vezes a célula (150 m) fica
        // abaixo de 5 milhões.
        let z = celula_das_classes([-43.37, -22.92, -40.56, -20.52], 30.0);
        assert_eq!(z, 150.0);
        assert_eq!(nome_das_classes(2024), "Uso e cobertura da terra em 2024 (MapBiomas)");
        assert!(eh_das_classes(&nome_das_classes(2019)));
        assert!(!eh_das_classes("Uso e cobertura da terra em 2024 (outra fonte)"));
        assert!(!eh_das_classes("BrasilData: Uso e cobertura 2024 (MapBiomas)"));
    }
}
