//! A área equipada para irrigação do Global Map of Irrigation Areas, versão 5 (GMIA v5, FAO e
//! Universidade de Bonn): a porcentagem de cada célula de 5′ (cerca de 9 km) equipada para
//! irrigação, por volta de 2005.
//!
//! - **De onde:** o COG mundial da FAO (AQUAMAPS, `storage.googleapis.com`, sem login, 3,9 MB),
//!   float32, sem dado -9, com pirâmides. Só a janela da área é lida (`layer.addUrl` com `bbox`,
//!   depois da 0.5.0), com uma célula de margem.
//! - **Como:** a janela vai para a grade do projeto pelo vizinho mais próximo (uma porcentagem da
//!   célula de 5′ não se interpola: cada célula da grade mostra a da FAO onde cai) ou pela média,
//!   quando a célula da grade é maior que a de 5′, e é recortada pela área.

/// O COG da FAO.
pub const URL: &str = "https://storage.googleapis.com/fao-gismgr-aquamaps-data/DATA/AQUAMAPS/MAP/AQUAMAPS.GMIA.tif";

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Área equipada para irrigação (GMIA v5, FAO)";

/// A camada de passagem: a janela lida do COG, e ela na grade.
pub const JANELA: &str = "BrasilData: irrigação (janela)";
pub const NA_GRADE: &str = "BrasilData: irrigação na grade";

/// A citação, como a FAO pede.
pub const CITACAO: &str = "Siebert, S., Henrich, V., Frenken, K. e Burke, J. (2013). Global Map of Irrigation Areas version 5. Rheinische Friedrich-Wilhelms-University, Bonn, Alemanha / Food and Agriculture Organization of the United Nations, Roma, Itália. Porcentagem de cada célula de 5′ (cerca de 9 km) equipada para irrigação, por volta de 2005; posta na grade do projeto e recortada pela área";

pub const LICENCA: &str = "CC BY 4.0 (FAO)";

/// O lado da célula do GMIA, em graus (5′).
pub const LADO: f64 = 5.0 / 60.0;

/// A célula do GMIA em metros, no equador (o `cellSize` do `layer.addUrl` fala em metros).
pub const LADO_M: f64 = LADO * 111_320.0;

/// A janela: o retângulo `c` com uma célula de margem.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - LADO, c[1] - LADO, c[2] + LADO, c[3] + LADO]
}

/// A escala das cores, em %: fixa, para áreas diferentes se compararem. Acima de 40% (poucas
/// células, nos perímetros irrigados), a cor mais escura.
pub const ESCALA: (f64, f64) = (0.0, 40.0);

/// A reamostragem para uma grade de células de `celula` metros.
pub fn reamostragem(celula: f64) -> &'static str {
    if celula > LADO_M { "average" } else { "nearest" }
}

/// A rampa de cores (formato do GDAL), do bege ao azul-esverdeado escuro, com mais tons nos
/// valores baixos: a irrigação se concentra em poucas células, e uma rampa linear deixaria quase
/// toda a área na cor mais clara.
pub const RAMPA: (&str, &str) =
    ("BrasilData: irrigação", "0 246 239 214\n1 214 236 196\n2.5 168 221 181\n5 116 196 168\n10 61 160 160\n20 24 112 143\n40 8 64 110");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_janela_tem_uma_celula_de_margem() {
        let j = janela([-47.0, -17.5, -46.5, -17.0]);
        assert!((j[0] - (-47.0 - LADO)).abs() < 1e-12 && (j[3] - (-17.0 + LADO)).abs() < 1e-12);
        assert!((LADO_M - 9276.67).abs() < 0.1);
    }

    #[test]
    fn a_rampa_cobre_a_escala() {
        let (_, cores) = RAMPA;
        let valores: Vec<f64> = cores.lines().map(|l| l.split(' ').next().unwrap().parse().unwrap()).collect();
        assert_eq!(valores.first(), Some(&ESCALA.0));
        assert_eq!(valores.last(), Some(&ESCALA.1));
        assert!(valores.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn a_grade_fina_pega_o_mais_proximo_e_a_grossa_a_media() {
        assert_eq!(reamostragem(30.0), "nearest");
        assert_eq!(reamostragem(5000.0), "nearest");
        assert_eq!(reamostragem(10_000.0), "average");
    }
}
