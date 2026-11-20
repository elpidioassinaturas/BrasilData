//! A biomassa acima do solo da CTrees a 100 m, um mapa por ano de 2000 a 2025, e a incerteza de
//! cada mapa: um modelo de IA (DenseNet) calibrado com biomassa de lidar aéreo e de parcelas, a
//! partir de séries do Landsat, do radar ALOS PALSAR-2 e das alturas do GEDI e do ICESat-2.
//!
//! - **De onde:** COGs globais no bucket público `ctrees-agb-100m-global` da AWS (Registry of Open
//!   Data, sem login, licença CC BY 4.0), um por ano e um da incerteza (o erro-padrão): WGS 84,
//!   células de 1/1125° (cerca de 100 m), Int16 com o valor em décimos de Mg/ha, −9999 sem dado, e
//!   dez níveis de pirâmide (cúbica), cada um com o dobro da célula.
//! - **Como:** só a janela da área (`layer.addUrl` com `bbox`), do nível de pirâmide mais grosso
//!   cujas células não passam das da grade; o valor passa a Mg/ha, vai para a grade do projeto e é
//!   recortado pela área.
//! - **App web:** o bucket manda o cabeçalho CORS: funciona também no navegador.

/// O primeiro e o último ano.
pub const PRIMEIRO_ANO: i64 = 2000;
pub const ULTIMO_ANO: i64 = 2025;

/// A célula do arquivo inteiro, em graus.
pub const CELULA_GRAUS: f64 = 1.0 / 1125.0;

/// Quantos níveis de pirâmide o arquivo tem além do inteiro.
pub const PIRAMIDES: i32 = 10;

/// Quantos valores a janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

const BASE: &str = "https://ctrees-agb-100m-global.s3.us-west-2.amazonaws.com/cogs/global_agb_100m_landsat0024_all";

/// O que se lê: a biomassa ou a incerteza dela.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variavel {
    Biomassa,
    Incerteza,
}

impl Variavel {
    pub fn ler(t: &str) -> Option<Variavel> {
        match t.trim() {
            "" | "biomassa" => Some(Variavel::Biomassa),
            "incerteza" | "erro" => Some(Variavel::Incerteza),
            _ => None,
        }
    }

    pub fn url(self, ano: i64) -> String {
        let fim = match self {
            Variavel::Biomassa => "",
            Variavel::Incerteza => "_uncertainty_sem",
        };
        format!("{BASE}_{ano}_densenet_l1_agb_mosaic_100m_base_cd_ts{fim}.tif")
    }

    /// O nome da camada.
    pub fn camada(self, ano: i64) -> String {
        match self {
            Variavel::Biomassa => format!("BrasilData: Biomassa acima do solo {ano} (CTrees, 100 m)"),
            Variavel::Incerteza => format!("BrasilData: Incerteza da biomassa {ano} (CTrees, 100 m)"),
        }
    }

    /// A camada de passagem com a janela lida no nível `k`.
    pub fn janela(self, ano: i64, k: i32) -> String {
        let o_que = if self == Variavel::Biomassa { "biomassa" } else { "incerteza" };
        format!("BrasilData: CTrees {o_que} {ano} nível {k} (janela)")
    }

    /// A escala das cores, em Mg/ha: a floresta amazônica passa de 300.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Variavel::Biomassa => (0.0, 400.0),
            Variavel::Incerteza => (0.0, 100.0),
        }
    }

    /// A rampa de cores (valor e cor, no formato do GDAL que o `ramp.add` lê), do bege visível
    /// sobre o papel ao escuro.
    pub fn rampa(self) -> (&'static str, &'static str) {
        match self {
            Variavel::Biomassa => (
                "BrasilData: biomassa da CTrees",
                "0 239 230 200\n25 214 222 154\n75 160 200 130\n150 78 154 134\n250 34 112 110\n400 12 60 72",
            ),
            Variavel::Incerteza => ("BrasilData: incerteza da biomassa", "0 245 240 230\n20 230 200 160\n50 200 120 110\n100 110 40 90"),
        }
    }
}

/// O ano da camada `nome` e o que ela é, se for do CTrees.
pub fn da_camada(nome: &str) -> Option<(Variavel, i64)> {
    (PRIMEIRO_ANO..=ULTIMO_ANO).flat_map(|a| [(Variavel::Biomassa, a), (Variavel::Incerteza, a)]).find(|(v, a)| v.camada(*a) == nome)
}

/// O ano pedido, conferido.
pub fn conferir_ano(a: f64) -> Result<i64, String> {
    let ano = a as i64;
    if a.fract() != 0.0 || !(PRIMEIRO_ANO..=ULTIMO_ANO).contains(&ano) {
        return Err(format!("o ano da biomassa da CTrees é de {PRIMEIRO_ANO} a {ULTIMO_ANO}"));
    }
    Ok(ano)
}

/// O lado da célula do nível `k`, em graus.
pub fn graus_do_nivel(k: i32) -> f64 {
    CELULA_GRAUS * 2f64.powi(k)
}

/// O lado, em metros no chão (o menor: o leste-oeste, na latitude `lat`), da célula do nível `k`.
pub fn metros_do_nivel(k: i32, lat: f64) -> f64 {
    graus_do_nivel(k) * 111_320.0 * lat.to_radians().cos()
}

/// O nível mais grosso cujas células, na latitude `lat`, não passam de `celula` metros.
pub fn nivel(celula: f64, lat: f64) -> i32 {
    (0..=PIRAMIDES).rev().find(|&k| metros_do_nivel(k, lat) <= celula + 1e-6).unwrap_or(0)
}

/// A janela da área `c`, com três células do nível `k` de margem.
pub fn janela(c: [f64; 4], k: i32) -> [f64; 4] {
    let m = 3.0 * graus_do_nivel(k);
    [c[0] - m, c[1] - m, c[2] + m, c[3] + m]
}

/// Quantos valores a janela `j` tem no nível `k`.
pub fn valores(j: [f64; 4], k: i32) -> f64 {
    let g = graus_do_nivel(k);
    ((j[2] - j[0]) / g) * ((j[3] - j[1]) / g)
}

/// O nível mais fino, a partir de `k`, com que a janela da área cabe no limite.
pub fn nivel_que_cabe(c: [f64; 4], k: i32) -> Option<i32> {
    (k..=PIRAMIDES).find(|&n| valores(janela(c, n), n) <= MAX_VALORES)
}

/// A reamostragem para a grade: bilinear até a célula lida, média numa grade mais grossa.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "average" } else { "bilinear" }
}

/// A citação, como a licença pede.
pub fn citacao(v: Variavel, ano: i64) -> String {
    let o_que = match v {
        Variavel::Biomassa => "biomassa acima do solo",
        Variavel::Incerteza => "incerteza (erro-padrão) da biomassa acima do solo",
    };
    format!(
        "Yang, Y., Saatchi, S. et al., CTrees Global Aboveground Biomass 100 m (2000–2025), doi:10.82924/7VMB-ZV66 (pré-publicação: EarthArXiv, doi:10.31223/X5KJ4Q), {o_que} de {ano} a 100 m; lido do bucket público ctrees-agb-100m-global (AWS Open Data), em Mg/ha, posto na grade do projeto e recortado pela área"
    )
}

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enderecos_e_nomes() {
        assert_eq!(
            Variavel::Biomassa.url(2025),
            "https://ctrees-agb-100m-global.s3.us-west-2.amazonaws.com/cogs/global_agb_100m_landsat0024_all_2025_densenet_l1_agb_mosaic_100m_base_cd_ts.tif"
        );
        assert!(Variavel::Incerteza.url(2000).ends_with("_2000_densenet_l1_agb_mosaic_100m_base_cd_ts_uncertainty_sem.tif"));
        for v in [Variavel::Biomassa, Variavel::Incerteza] {
            assert_eq!(da_camada(&v.camada(2013)), Some((v, 2013)));
        }
        assert_eq!(da_camada("BrasilData: Biomassa acima do solo (GEDI L4D)"), None);
        assert_eq!(Variavel::ler("incerteza"), Some(Variavel::Incerteza));
        assert_eq!(Variavel::ler("outra"), None);
        assert_eq!(conferir_ano(2025.0), Ok(2025));
        assert!(conferir_ano(1999.0).is_err() && conferir_ano(2020.5).is_err());
    }

    #[test]
    fn o_nivel_e_os_limites() {
        // A 20° S, a célula do arquivo tem uns 93 m de leste a oeste.
        assert!((metros_do_nivel(0, -20.0) - 92.98).abs() < 0.01);
        assert_eq!(nivel(30.0, -20.0), 0);
        assert_eq!(nivel(100.0, -20.0), 0);
        assert_eq!(nivel(200.0, -20.0), 1);
        assert_eq!(nivel(1000.0, -20.0), 3);
        // Minas Gerais inteira (uns 12° × 12°) não cabe a 100 m, cabe a 200 m.
        let mg = [-51.05, -22.93, -39.85, -14.23];
        assert_eq!(nivel_que_cabe(mg, 0), Some(1));
        // Viçosa cabe a 100 m.
        assert_eq!(nivel_que_cabe([-43.003, -20.838, -42.754, -20.655], 0), Some(0));
        assert_eq!(reamostragem(30.0, 93.0), "bilinear");
        assert_eq!(reamostragem(1000.0, 372.0), "average");
    }
}
