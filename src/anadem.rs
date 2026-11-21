//! O ANADEM (ANA e IPH/UFRGS, 2024): o modelo digital do terreno da América do Sul a 30 m, o
//! Copernicus GLO-30 sem o viés da vegetação, removido por aprendizado de máquina calibrado para o
//! continente (índices do Landsat 8 e do Sentinel-2, alturas do GEDI e do ICESat-2).
//!
//! - **De onde:** um COG da América do Sul inteira no S3 público da OpenTopography
//!   (`opentopography.s3.sdsc.edu`, sem login), SIRGAS 2000 geográfico (EPSG:4674), células de
//!   0,000269494585236° (cerca de 30 m), float32 em metros, −9999 sem dado, blocos de 512 × 512 e
//!   dez níveis de pirâmide, cada um com o dobro da célula.
//! - **Como:** só a janela da área (`layer.addUrl` com `bbox`), do nível de pirâmide mais grosso
//!   cujas células não passam das da grade; vai para a grade do projeto e é recortada pela área.
//! - **App web:** o servidor manda o cabeçalho CORS: funciona também no navegador.
//! - **Licença:** a ANA e o IPH/UFRGS o publicam como livre e aberto, sem uma licença escrita dos
//!   dados (o código é MIT); a OpenTopography pede a citação.

use crate::cog_global::Arquivo;

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Terreno a 30 m (ANADEM)";

/// O arquivo.
pub fn arquivo() -> Arquivo {
    Arquivo {
        url: "https://opentopography.s3.sdsc.edu/raster/ANADEM/ANADEM_be/anadem_v1_compressed_COG.tif".into(),
        graus: 0.000_269_494_585_236,
        piramides: 10,
    }
}

/// A camada de passagem com a janela lida no nível `k`.
pub fn janela_nome(k: i32) -> String {
    format!("BrasilData: ANADEM nível {k} (janela)")
}

/// Os valores que ficam: o −9999 do arquivo e o que não for terreno saem.
pub const FAIXA: (f64, f64) = (-500.0, 9000.0);

/// A citação, como a OpenTopography pede.
pub const CITACAO: &str = "Agência Nacional de Águas e Saneamento Básico (2025). ANADEM: A Digital Terrain Model for South America. Distribuído pela OpenTopography, doi:10.5069/G9736P4G; Laipelt, L., Andrade, B. C., Collischonn, W., Teixeira, A. A., Paiva, R. C. D. e Ruhoff, A., ANADEM: A Digital Terrain Model for South America, Remote Sensing 16(13), 2321 (2024), doi:10.3390/rs16132321. Lido do S3 público da OpenTopography, posto na grade do projeto e recortado pela área";

/// A licença.
pub const LICENCA: &str = "livre e aberto (ANA e IPH/UFRGS), com a citação";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_arquivo_e_o_nivel() {
        let a = arquivo();
        assert!(a.url.ends_with("anadem_v1_compressed_COG.tif"));
        // A 20° S, a célula tem uns 28 m de leste a oeste; Viçosa cabe sem pirâmide.
        assert!((a.metros_do_nivel(0, -20.0) - 28.19).abs() < 0.01);
        assert_eq!(a.nivel(30.0, -20.0), 0);
        assert_eq!(a.nivel_que_cabe([-43.003, -20.838, -42.754, -20.655], 0), Some(0));
        assert!(CITACAO.contains("10.5069/G9736P4G") && CITACAO.contains("10.3390/rs16132321"));
        assert_eq!(janela_nome(2), "BrasilData: ANADEM nível 2 (janela)");
    }
}
