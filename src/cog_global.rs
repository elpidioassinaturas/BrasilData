//! Um COG global em WGS 84 com pirâmides de dois em dois: as contas da janela da área e do nível
//! de pirâmide que se lê (os arquivos do OpenLandMap, [`crate::olm`] e [`crate::gedtm`]).
//!
//! - **O nível:** o mais grosso cujas células, no meio da área, não passam das da grade; se a
//!   janela não cabe no limite do `layer.addUrl` (64 milhões de valores), o mais fino que cabe.
//! - **A janela:** o retângulo da área com três células do nível de margem, para a reamostragem
//!   não perder a borda.

/// Quantos valores a janela pode ter: o limite do `layer.addUrl` é 64 milhões.
pub const MAX_VALORES: f64 = 60e6;

/// Um metro em graus de latitude (o inverso).
const GRAU_M: f64 = 111_320.0;

/// O `cellSize` que faz o `layer.addUrl` ler a visão cuja célula tem `graus` de lado: em metros, com
/// o grau valendo 111,32 km (é como ele mede um arquivo em longitude e latitude), e com folga, porque
/// a célula de uma visão reduzida pode passar um pouco do dobro e ele só a escolhe se ela não passar
/// do pedido.
pub fn celula_pedida(graus: f64) -> f64 {
    graus * GRAU_M * 1.05
}

/// O arquivo: o endereço, o lado da célula do arquivo inteiro, em graus, e quantos níveis de
/// pirâmide ele tem além do inteiro.
#[derive(Clone, Debug, PartialEq)]
pub struct Arquivo {
    pub url: String,
    pub graus: f64,
    pub piramides: i32,
}

impl Arquivo {
    /// O lado da célula do nível `k`, em graus.
    pub fn graus_do_nivel(&self, k: i32) -> f64 {
        self.graus * 2f64.powi(k)
    }

    /// O lado, em metros no chão (o menor: o leste-oeste, na latitude `lat`), da célula do nível
    /// `k`.
    pub fn metros_do_nivel(&self, k: i32, lat: f64) -> f64 {
        self.graus_do_nivel(k) * GRAU_M * lat.to_radians().cos()
    }

    /// O nível mais grosso cujas células, na latitude `lat`, não passam de `celula` metros.
    pub fn nivel(&self, celula: f64, lat: f64) -> i32 {
        (0..=self.piramides).rev().find(|&k| self.metros_do_nivel(k, lat) <= celula + 1e-6).unwrap_or(0)
    }

    /// A janela da área `c`, com três células do nível `k` de margem.
    pub fn janela(&self, c: [f64; 4], k: i32) -> [f64; 4] {
        let m = 3.0 * self.graus_do_nivel(k);
        [c[0] - m, c[1] - m, c[2] + m, c[3] + m]
    }

    /// O nível mais fino, a partir de `k`, com que a janela da área cabe no limite.
    pub fn nivel_que_cabe(&self, c: [f64; 4], k: i32) -> Option<i32> {
        (k..=self.piramides).find(|&n| {
            let j = self.janela(c, n);
            let g = self.graus_do_nivel(n);
            ((j[2] - j[0]) / g) * ((j[3] - j[1]) / g) <= MAX_VALORES
        })
    }
}

/// A reamostragem de um valor contínuo para a grade: bilinear até a célula lida, média numa grade
/// mais grossa.
pub fn reamostragem(celula: f64, lido: f64) -> &'static str {
    if celula > lido * 1.5 { "average" } else { "bilinear" }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_nivel_e_os_limites() {
        let a = Arquivo { url: String::new(), graus: 0.00025, piramides: 10 };
        // A 20° S, a célula do arquivo tem uns 26 m de leste a oeste.
        assert!((a.metros_do_nivel(0, -20.0) - 26.15).abs() < 0.01);
        assert_eq!(a.nivel(30.0, -20.0), 0);
        assert_eq!(a.nivel(60.0, -20.0), 1);
        assert_eq!(a.nivel(1000.0, -20.0), 5);
        // Viçosa cabe a 30 m; Minas Gerais inteira, só da pirâmide de cerca de 200 m.
        assert_eq!(a.nivel_que_cabe([-43.003, -20.838, -42.754, -20.655], 0), Some(0));
        assert_eq!(a.nivel_que_cabe([-51.05, -22.93, -39.85, -14.23], 0), Some(3));
        // O pedido, em metros, escolhe a visão do nível e não a seguinte. A dos arquivos passa até
        // 0,3% do dobro.
        for k in 0..=10 {
            let lado = 27.83 * 2f64.powi(k);
            let pedido = celula_pedida(a.graus_do_nivel(k));
            assert!(pedido > 1.003 * lado && pedido < 1.5 * lado, "{k}");
        }
        // O Brasil inteiro, da pirâmide de uns 900 m; o mundo não cabe com só duas pirâmides.
        assert_eq!(a.nivel_que_cabe([-74.0, -34.0, -34.8, 5.3], 0), Some(5));
        assert_eq!(Arquivo { piramides: 2, ..a }.nivel_que_cabe([-180.0, -56.0, 180.0, 76.0], 0), None);
        assert_eq!(reamostragem(30.0, 26.0), "bilinear");
        assert_eq!(reamostragem(1000.0, 418.0), "average");
    }
}
