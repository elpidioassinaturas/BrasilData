//! A cobertura da terra global a 100 m do serviço de terra do Copernicus (CLMS, Copernicus Global
//! Land Service Land Cover 100 m, coleção 3, os anos de 2015 a 2019), feita com o PROBA-V.
//!
//! - **De onde:** as cópias oficiais no Zenodo (um registro por ano, licença CC BY 4.0), sem
//!   login, com CORS e leitura por partes (Range). O mapa de classes discretas é um GeoTIFF global
//!   em graus (EPSG:4326, 1/1008° ≈ 110 m no equador), 362.880 × 141.120 células, `uint8`, em
//!   blocos de 256, deflate, **sem pirâmides**; 255 é sem dado.
//! - **Como:** só a janela do retângulo da área é lida (`layer.addUrl` com `bbox`, depois da 0.5.0),
//!   recortada pelo contorno e posta na grade do projeto com a classe mais frequente, como o
//!   MapBiomas. Sem pirâmides, a janela tem de caber no limite do `layer.addUrl` (64 milhões de
//!   células): uns 7° × 7°, o que cobre um estado médio, não o Brasil.
//! - **Os anos:** 2015 é a base, 2016 a 2018 são consolidados e 2019 é o "nrt" (quase em tempo
//!   real). O mesmo produto está no Copernicus Data Space como COG, mas lá os arquivos pedem login
//!   (opengeos/RustGIS#1980).
//! - **Conferido em Viçosa (13/11/2026):** 2019, janela de 254 × 187 células; herbácea (30) 39%,
//!   florestas (112, 116, 126) 54%, área urbana (50) 2,5%, cultivo (40) 1%.

/// O primeiro e o último ano.
pub const PRIMEIRO_ANO: i64 = 2015;
pub const ULTIMO_ANO: i64 = 2019;

/// O lado da célula do arquivo, em graus.
pub const LADO: f64 = 1.0 / 1008.0;

/// Quantas células a janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_CELULAS: f64 = 60e6;

/// A margem da janela em volta do retângulo da área, em graus (duas células), para o recorte pelo
/// contorno não perder a borda.
const MARGEM: f64 = 2.0 * LADO;

/// O registro do Zenodo e o nome do arquivo de cada ano.
fn registro(ano: i64) -> Option<(u32, &'static str)> {
    Some(match ano {
        2015 => (3939038, "2015-base"),
        2016 => (3518026, "2016-conso"),
        2017 => (3518036, "2017-conso"),
        2018 => (3518038, "2018-conso"),
        2019 => (3939050, "2019-nrt"),
        _ => return None,
    })
}

/// O endereço do mapa de classes de `ano`.
pub fn url(ano: i64) -> Option<String> {
    registro(ano).map(|(id, versao)| {
        format!(
            "https://zenodo.org/api/records/{id}/files/PROBAV_LC100_global_v3.0.1_{versao}_Discrete-Classification-map_EPSG-4326.tif/content"
        )
    })
}

/// A página do registro de `ano`, para a citação.
pub fn pagina(ano: i64) -> Option<String> {
    registro(ano).map(|(id, _)| format!("https://doi.org/10.5281/zenodo.{id}"))
}

/// A janela a ler para o retângulo `c` da área.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - MARGEM, c[1] - MARGEM, c[2] + MARGEM, c[3] + MARGEM]
}

/// Quantas células do arquivo a janela de `c` tem.
pub fn celulas(c: [f64; 4]) -> f64 {
    let j = janela(c);
    ((j[2] - j[0]) / LADO).ceil() * ((j[3] - j[1]) / LADO).ceil()
}

/// O nome da camada de `ano`.
pub fn camada(ano: i64) -> String {
    format!("BrasilData: Cobertura da terra {ano} (Copernicus 100 m)")
}

/// O ano de uma camada desta fonte, pelo nome.
pub fn ano_da_camada(nome: &str) -> Option<i64> {
    (PRIMEIRO_ANO..=ULTIMO_ANO).find(|&a| nome == camada(a))
}

/// A licença.
pub const LICENCA: &str = "CC BY 4.0 (política de dados do Copernicus: uso livre, com a citação)";

/// A citação de `ano`.
pub fn citacao(ano: i64) -> String {
    format!(
        "Buchhorn, M. et al. Copernicus Global Land Service: Land Cover 100m: collection 3: epoch {ano}: Globe (Version V3.0.1). Zenodo, {}. Mapa de classes discretas (PROBA-V, 100 m), recortado pela área e posto na grade do projeto com a classe mais frequente; os valores são os códigos da legenda do CLMS",
        pagina(ano).unwrap_or_default()
    )
}

/// As classes: o código, o nome em português e a cor da legenda oficial do CLMS.
pub const CLASSES: [(u8, &str, &str); 23] = [
    (111, "Floresta fechada, perenifólia de acículas", "#58481f"),
    (112, "Floresta fechada, perenifólia de folhas largas", "#009900"),
    (113, "Floresta fechada, decídua de acículas", "#70663e"),
    (114, "Floresta fechada, decídua de folhas largas", "#00cc00"),
    (115, "Floresta fechada, mista", "#4e751f"),
    (116, "Floresta fechada, tipo não identificado", "#007800"),
    (121, "Floresta aberta, perenifólia de acículas", "#666000"),
    (122, "Floresta aberta, perenifólia de folhas largas", "#8db400"),
    (123, "Floresta aberta, decídua de acículas", "#8d7400"),
    (124, "Floresta aberta, decídua de folhas largas", "#a0dc00"),
    (125, "Floresta aberta, mista", "#929900"),
    (126, "Floresta aberta, tipo não identificado", "#648c00"),
    (20, "Arbustos", "#ffbb22"),
    (30, "Vegetação herbácea", "#ffff4c"),
    (90, "Áreas úmidas herbáceas", "#0096a0"),
    (100, "Musgos e liquens", "#fae6a0"),
    (60, "Solo exposto ou vegetação esparsa", "#b4b4b4"),
    (40, "Cultivos e vegetação manejada", "#f096ff"),
    (50, "Área urbana e construída", "#fa0000"),
    (70, "Neve e gelo", "#f0f0f0"),
    (80, "Corpos d'água permanentes", "#0032c8"),
    (200, "Mar aberto", "#000080"),
    (0, "Sem dado de entrada", "#282828"),
];

/// O estilo do raster (paleta com os nomes das classes), no formato do QGIS que o
/// `style.importQgis` lê.
pub fn qml() -> String {
    let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
    for (v, nome, cor) in CLASSES {
        let alfa = if v == 0 { 0 } else { 255 };
        q.push_str(&format!(r#"<paletteEntry value="{v}" color="{cor}" alpha="{alfa}" label="{nome}"/>"#));
    }
    q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enderecos_dos_anos() {
        assert_eq!(
            url(2019).unwrap(),
            "https://zenodo.org/api/records/3939050/files/PROBAV_LC100_global_v3.0.1_2019-nrt_Discrete-Classification-map_EPSG-4326.tif/content"
        );
        assert!(url(2015).unwrap().contains("3939038/files/PROBAV_LC100_global_v3.0.1_2015-base_"));
        assert!(url(2016).unwrap().contains("3518026/files/PROBAV_LC100_global_v3.0.1_2016-conso_"));
        assert!(url(2014).is_none() && url(2020).is_none());
        assert_eq!(pagina(2018).unwrap(), "https://doi.org/10.5281/zenodo.3518038");
    }

    #[test]
    fn camadas_e_anos() {
        for a in PRIMEIRO_ANO..=ULTIMO_ANO {
            assert_eq!(ano_da_camada(&camada(a)), Some(a));
        }
        assert_eq!(ano_da_camada("BrasilData: Cobertura da terra 2020 (Copernicus 100 m)"), None);
    }

    #[test]
    fn janela_de_vicosa_cabe() {
        let c = [-43.0049, -20.839, -42.7536, -20.6547];
        let n = celulas(c);
        assert!(n > 40_000.0 && n < 60_000.0, "{n}");
        // Minas Gerais inteira (cerca de 12° × 9°) passa do limite.
        assert!(celulas([-51.05, -22.92, -39.85, -14.23]) > MAX_CELULAS);
    }

    #[test]
    fn paleta_tem_todas_as_classes() {
        let q = qml();
        assert_eq!(q.matches("<paletteEntry").count(), CLASSES.len());
        assert!(q.contains(r##"value="50" color="#fa0000" alpha="255" label="Área urbana e construída""##));
        assert!(q.contains(r##"value="0" color="#282828" alpha="0""##));
    }
}
