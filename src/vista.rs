//! A vista, quando não há área: o retângulo que ela mostra, cortado pelo Brasil.

/// O retângulo do Brasil, com as ilhas oceânicas (Trindade e Martim Vaz, São Pedro e São Paulo)
/// e uma pequena folga.
pub const BRASIL: [f64; 4] = [-74.1, -33.9, -28.5, 5.4];

/// O retângulo que a vista mostra (uma tela de 1600 × 1000 px a 96 dpi, nessa escala), cortado
/// pelo Brasil. Uma vista que não mostra o Brasil, ou que mostra o país inteiro e mais (o mundo),
/// é recusada: pediria a cada serviço o Brasil todo, ou nada.
pub fn vista_no_brasil(lon: f64, lat: f64, escala: f64) -> Result<[f64; 4], String> {
    let m_px = escala * 0.0254 / 96.0;
    let (dx, dy) = (800.0 * m_px / (111_320.0 * lat.to_radians().cos().max(0.01)), 500.0 * m_px / 110_540.0);
    let [w, s, e, n] = [lon - dx, lat - dy, lon + dx, lat + dy];
    let [bw, bs, be, bn] = BRASIL;
    if w <= bw && s <= bs && e >= be && n >= bn {
        return Err("a vista mostra mais que o Brasil inteiro: aproxime o mapa ou escolha a área (painel Área de trabalho)".into());
    }
    let caixa = [w.max(bw), s.max(bs), e.min(be), n.min(bn)];
    if caixa[0] >= caixa[2] || caixa[1] >= caixa[3] {
        return Err("a vista não mostra o Brasil: leve o mapa até a área ou escolha a área (painel Área de trabalho)".into());
    }
    Ok(caixa)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vista_e_cortada_pelo_brasil() {
        // Teresina a cerca de 1:500.000: dentro do Brasil, fica como está.
        let c = vista_no_brasil(-42.8, -5.1, 500_000.0).unwrap();
        assert!(c[0] > -44.0 && c[2] < -41.6 && c[1] > -6.0 && c[3] < -4.2, "{c:?}");
        // Uma vista que passa da fronteira é cortada no retângulo do Brasil.
        let c = vista_no_brasil(-74.0, -10.0, 5_000_000.0).unwrap();
        assert_eq!(c[0], BRASIL[0]);
        // O mundo, ou uma vista sem o Brasil, é recusada.
        assert!(vista_no_brasil(0.0, 0.0, 150_000_000.0).unwrap_err().contains("mais que o Brasil inteiro"));
        assert!(vista_no_brasil(10.0, 45.0, 1_000_000.0).unwrap_err().contains("não mostra o Brasil"));
    }
}
