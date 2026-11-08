//! A estrutura da vegetação do GEDI L4D (ORNL DAAC da NASA): altura do dossel, biomassa acima do
//! solo e cobertura do dossel a 30 m, imputadas com o Landsat a partir dos disparos do laser do
//! GEDI de 2019 a 2023.
//!
//! - **De onde:** um COG por fuso UTM e hemisfério e por variável
//!   (`GEDI_L4D_UTM23_South_20190418_20230316_rh_098.tif`), achado pela busca do CMR
//!   (`cmr.earthdata.nasa.gov`, sem login, com CORS). Os arquivos pedem o login Earthdata, que o
//!   RustGIS manda aos hosts de dados da NASA; o plugin nunca vê o token.
//! - **Como:** cada arquivo é grande (um fuso inteiro a 30 m), então só serve lido pela janela da
//!   área (`layer.addUrl` com `bbox`, opengeos/RustGIS#1210, depois da 0.5.0). Cada janela vai
//!   para a grade do projeto, as dos fusos se juntam e o mosaico é recortado pelo contorno da área.
//! - **Valores:** a altura vem em centímetros (int16, sem dado -32768) e vira metros; a biomassa em
//!   Mg/ha e a cobertura em % (0 a 100), com -9999 onde o disparo não tem o valor. As pirâmides
//!   do arquivo tiram a média com esses -9999, que ficam negativos: os negativos viram sem dado
//!   em cada janela, antes da grade, e não entram nas médias.

use serde_json::Value;

/// A coleção no CMR.
pub const COLECAO: &str = "GEDI_L4D_Imputed_Waveforms_2455";

/// A citação, como o ORNL DAAC pede.
pub const CITACAO: &str = "Seo, E., S.P. Healey, Z. Yang, R.O. Dubayah, T. De Conto e J. Armston. 2025. GEDI L4D Imputed Waveforms, Version 2. ORNL DAAC, Oak Ridge, Tennessee, EUA. https://doi.org/10.3334/ORNLDAAC/2455. Disparos do GEDI de 18/04/2019 a 16/03/2023 imputados a cada pixel de 30 m com séries do Landsat; recortado pela área e posto na grade do projeto";

/// A licença: a política de dados da NASA (uso livre, com a citação).
pub const LICENCA: &str = "política de dados da NASA: uso livre e gratuito, com a citação";

/// As variáveis que o plugin traz.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Variavel {
    /// A altura relativa em que volta 98% da energia da onda (rh98): a altura do dossel.
    Altura,
    /// A biomassa acima do solo (AGBD), do L4A.
    Biomassa,
    /// A cobertura do dossel ao nível do solo, do L2B.
    Cobertura,
}

impl Variavel {
    pub const TODAS: [Variavel; 3] = [Variavel::Altura, Variavel::Biomassa, Variavel::Cobertura];

    /// O comando do plugin.
    pub fn comando(self) -> &'static str {
        match self {
            Variavel::Altura => "gedi-altura",
            Variavel::Biomassa => "gedi-biomassa",
            Variavel::Cobertura => "gedi-cobertura",
        }
    }

    pub fn do_comando(id: &str) -> Option<Variavel> {
        Self::TODAS.into_iter().find(|v| v.comando() == id)
    }

    /// O fim do nome do arquivo (`…_rh_098.tif`).
    fn sufixo(self) -> &'static str {
        match self {
            Variavel::Altura => "_rh_098.tif",
            Variavel::Biomassa => "_agbd.tif",
            Variavel::Cobertura => "_cover_z_000.tif",
        }
    }

    /// O nome da camada.
    pub fn camada(self) -> &'static str {
        match self {
            Variavel::Altura => "BrasilData: Altura do dossel (GEDI L4D)",
            Variavel::Biomassa => "BrasilData: Biomassa acima do solo (GEDI L4D)",
            Variavel::Cobertura => "BrasilData: Cobertura do dossel (GEDI L4D)",
        }
    }

    /// O que é, para os avisos.
    pub fn titulo(self) -> &'static str {
        match self {
            Variavel::Altura => "Altura do dossel",
            Variavel::Biomassa => "Biomassa acima do solo",
            Variavel::Cobertura => "Cobertura do dossel",
        }
    }

    pub fn unidade(self) -> &'static str {
        match self {
            Variavel::Altura => "m",
            Variavel::Biomassa => "Mg/ha",
            Variavel::Cobertura => "%",
        }
    }

    /// O último passo da limpeza (o valor subiu 1, e o que era negativo virou 0): volta o valor,
    /// dá sem dado no 0 (0 / 0) e passa para a unidade (a altura vem em centímetros).
    pub fn conta(self) -> &'static str {
        match self {
            Variavel::Altura => "(b1 / b1) * (b1 - 1) / 100",
            _ => "(b1 / b1) * (b1 - 1)",
        }
    }

    /// Os valores válidos, na unidade: fora deles, sem dado (os negativos das pirâmides, o
    /// -9999 e o -32768).
    pub fn validos(self) -> (f64, f64) {
        match self {
            Variavel::Altura => (0.0, 150.0),
            Variavel::Biomassa => (0.0, 2000.0),
            Variavel::Cobertura => (0.0, 100.0),
        }
    }

    /// A escala das cores, na unidade.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Variavel::Altura => (0.0, 35.0),
            Variavel::Biomassa => (0.0, 300.0),
            Variavel::Cobertura => (0.0, 100.0),
        }
    }
}

/// Se a camada `nome` é do GEDI L4D.
pub fn eh_do_gedi(nome: &str) -> bool {
    Variavel::TODAS.iter().any(|v| v.camada() == nome)
}

/// A busca no CMR dos arquivos que cruzam o retângulo `c` (`[oeste, sul, leste, norte]`): cada
/// fuso tem 16 arquivos, e o Brasil cruza no máximo 8 fusos em 2 hemisférios.
pub fn url_busca(c: [f64; 4]) -> String {
    format!(
        "https://cmr.earthdata.nasa.gov/search/granules.json?short_name={COLECAO}&bounding_box={:.4},{:.4},{:.4},{:.4}&page_size=300",
        c[0], c[1], c[2], c[3]
    )
}

/// Os endereços dos arquivos de `v` na resposta da busca, um por fuso, em ordem.
pub fn arquivos(resposta: &Value, v: Variavel) -> Vec<String> {
    let mut urls: Vec<String> = resposta["feed"]["entry"]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|g| g["links"].as_array().into_iter().flatten())
        .filter(|l| l["rel"].as_str().is_some_and(|r| r.ends_with("/data#")))
        .filter_map(|l| l["href"].as_str())
        .filter(|h| h.starts_with("https://") && h.ends_with(v.sufixo()))
        .map(str::to_string)
        .collect();
    urls.sort();
    urls.dedup();
    urls
}

/// O fuso e o hemisfério de um arquivo (`UTM23_South`), para nomear a janela dele.
pub fn fuso(url: &str) -> String {
    let n = url.rsplit('/').next().unwrap_or(url);
    let n = n.strip_prefix("GEDI_L4D_").unwrap_or(n);
    n.split('_').take(2).collect::<Vec<_>>().join("_")
}

/// A margem em volta do retângulo da área, em graus (cerca de 7 células de 30 m).
const MARGEM: f64 = 0.002;
/// Quantos valores uma janela pode ter: o RustGIS lê até 64 milhões; abaixo disso, com folga.
const MAX_VALORES: f64 = 60_000_000.0;

/// O retângulo da janela: o da área com a margem.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - MARGEM, c[1] - MARGEM, c[2] + MARGEM, c[3] + MARGEM]
}

/// A célula a pedir para a janela do retângulo `c`, em metros: `None` quando a de 30 m cabe;
/// senão, a da primeira pirâmide do COG em que ela cabe (60 m, 120 m...), arredondada para
/// cima, porque o RustGIS lê o nível mais fino com célula até a pedida.
pub fn celula_da_janela(c: [f64; 4]) -> Option<f64> {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let largura = (c[2] - c[0] + 2.0 * MARGEM) * 111_320.0 * lat.cos();
    let altura = (c[3] - c[1] + 2.0 * MARGEM) * 110_574.0;
    let mut valores = (largura / 30.0) * (altura / 30.0);
    let mut celula = 30.0_f64;
    while valores > MAX_VALORES {
        valores /= 4.0;
        celula *= 2.0;
    }
    (celula > 30.0).then_some(celula + 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn os_comandos_e_as_camadas_se_encontram() {
        for v in Variavel::TODAS {
            assert_eq!(Variavel::do_comando(v.comando()), Some(v));
            assert!(eh_do_gedi(v.camada()));
        }
        assert_eq!(Variavel::do_comando("radar-vv"), None);
        assert!(!eh_do_gedi("BrasilData: Elevação (Copernicus GLO-30)"));
        assert_eq!(Variavel::Altura.conta(), "(b1 / b1) * (b1 - 1) / 100");
        assert_eq!(Variavel::Biomassa.conta(), "(b1 / b1) * (b1 - 1)");
    }

    #[test]
    fn a_busca_acha_um_arquivo_por_fuso() {
        let link = |f: &str| json!({"rel": "http://esipfed.org/ns/fedsearch/1.1/data#", "href": format!("https://data.ornldaac.earthdata.nasa.gov/protected/gedi/GEDI_L4D_Imputed_Waveforms/data/{f}")});
        let r = json!({"feed": {"entry": [
            {"links": [link("GEDI_L4D_UTM24_South_20190418_20230316_rh_098.tif"), {"rel": "http://esipfed.org/ns/fedsearch/1.1/metadata#", "href": "https://x/rh_098.tif"}]},
            {"links": [link("GEDI_L4D_UTM23_South_20190418_20230316_rh_098.tif")]},
            {"links": [link("GEDI_L4D_UTM23_South_20190418_20230316_rh_095.tif")]},
            {"links": [link("GEDI_L4D_UTM23_South_20190418_20230316_agbd.tif")]},
            {"links": [link("GEDI_L4D_UTM23_South_20190418_20230316_rh_098.tif")]},
        ]}});
        let a = arquivos(&r, Variavel::Altura);
        assert_eq!(a.len(), 2);
        assert!(a[0].ends_with("UTM23_South_20190418_20230316_rh_098.tif"));
        assert_eq!(fuso(&a[1]), "UTM24_South");
        assert_eq!(arquivos(&r, Variavel::Biomassa).len(), 1);
        assert!(arquivos(&r, Variavel::Cobertura).is_empty());
        assert!(arquivos(&json!({"errors": ["x"]}), Variavel::Altura).is_empty());
        assert!(url_busca([-43.0, -20.85, -42.75, -20.62]).contains("bounding_box=-43.0000,-20.8500,-42.7500,-20.6200"));
    }

    #[test]
    fn a_janela_de_um_municipio_e_de_30_m_e_a_de_uma_mesorregiao_vem_da_piramide() {
        assert_eq!(celula_da_janela([-43.0, -20.85, -42.75, -20.62]), None);
        // A Zona da Mata (MG): cerca de 2,8° por 2,4°, uns 87 milhões de células de 30 m.
        assert_eq!(celula_da_janela([-43.37, -22.92, -40.56, -20.52]), Some(61.0));
    }
}
