//! Rios conforme o zoom: um tema de drenagem com a área de drenagem a montante de cada trecho vem
//! só a partir da menor área que cabe num orçamento de feições, e a camada mostra os rios
//! principais de longe e os menores à medida que o mapa aproxima (os filtros de exibição por escala
//! do RustGIS).
//!
//! A hierarquia é a área a montante, não a ordem de Strahler: na BHO, a ordem cresce com a
//! densidade do mapeamento, e o Rio Amazonas (ordem 9) fica abaixo de rios do Sudeste de ordem 10
//! e 11; com a ordem, a vista do Brasil mostrava os rios do Sudeste e quase nada da Amazônia.

use serde_json::{Value, json};

use crate::Tema;

/// Os temas de drenagem com a área a montante (km²), e o atributo dela.
const TEMAS_POR_ZOOM: &[(&str, &str)] = &[("ana-bho", "nuareamont")];

/// Mais trechos que isto num tema por zoom: o serviço da ANA manda mil por pedido, e cada página é
/// um pedido (30 mil: cerca de 30 pedidos, meio minuto).
pub const ORCAMENTO: u64 = 30_000;

/// As áreas a montante (km²) que o plugin tenta, dos rios maiores aos menores; 0 é tudo. No
/// retângulo do Brasil (ANA, 07/11/2026): 4.235 trechos com 100 mil km² ou mais, 27.431 com 10
/// mil, 98.463 com mil.
const LIMIARES: &[f64] =
    &[100_000.0, 50_000.0, 20_000.0, 10_000.0, 5_000.0, 2_000.0, 1_000.0, 500.0, 200.0, 100.0, 50.0, 20.0, 10.0, 5.0, 0.0];

/// A área a montante mínima mostrada em cada faixa de escala, de longe para perto:
/// `(denominador, área)` diz que, de 1:`denominador` para longe (até a faixa anterior), só
/// aparecem os trechos com área ≥ `área` km². Mais perto que a última faixa, aparece tudo o que
/// veio.
const FAIXAS: &[(f64, f64)] = &[
    (20_000_000.0, 50_000.0),
    (10_000_000.0, 20_000.0),
    (5_000_000.0, 5_000.0),
    (2_500_000.0, 2_000.0),
    (1_200_000.0, 500.0),
    (600_000.0, 100.0),
    (300_000.0, 20.0),
];

/// O atributo da área a montante de `t`, se ele é um tema por zoom.
pub fn campo(t: &Tema) -> Option<&'static str> {
    TEMAS_POR_ZOOM.iter().find(|(id, _)| *id == t.id).map(|(_, c)| *c)
}

/// A menor área a montante (km²) cujos trechos (`contar(área)`: quantos têm área ≥ essa) cabem
/// no `orcamento`, por busca binária em [`LIMIARES`]: no máximo 4 contagens, cada uma de poucos
/// segundos. Contar todos os trechos do Brasil leva 24 s, e o RustGIS dá 20 s a cada pedido: a
/// busca só chega a 0 (tudo) numa área em que os limiares acima dele já cabem. O primeiro limiar
/// vem mesmo quando passa do orçamento.
pub fn area_minima<E>(orcamento: u64, mut contar: impl FnMut(f64) -> Result<u64, E>) -> Result<f64, E> {
    let (mut cabe, mut alto) = (0, LIMIARES.len() - 1);
    while cabe < alto {
        let meio = (cabe + alto).div_ceil(2);
        if contar(LIMIARES[meio])? <= orcamento {
            cabe = meio;
        } else {
            alto = meio - 1;
        }
    }
    Ok(LIMIARES[cabe])
}

/// O `where` que deixa só os trechos com área ≥ `minima` (junto com o filtro do catálogo); com 0,
/// o filtro do catálogo só.
pub fn filtro(t: &Tema, campo: &str, minima: f64) -> String {
    match (t.filtro.is_empty(), minima > 0.0) {
        (_, false) => t.filtro.to_string(),
        (true, true) => format!("{campo} >= {minima}"),
        (false, true) => format!("({}) AND {campo} >= {minima}", t.filtro),
    }
}

/// `km` com o ponto dos milhares: "10.000 km²".
pub fn km2(km: f64) -> String {
    let n = format!("{km:.0}");
    let mut s = String::new();
    for (i, c) in n.chars().enumerate() {
        if i > 0 && (n.len() - i).is_multiple_of(3) {
            s.push('.');
        }
        s.push(c);
    }
    format!("{s} km²")
}

/// Os filtros de exibição do `layer.displayFilters`: em cada faixa de escala, os trechos com área
/// ≥ a da faixa. Faixas que não deixariam nada de fora (área ≤ `minima`, a que veio) ficam de fora.
pub fn filtros_de_exibicao(campo: &str, minima: f64) -> Value {
    let mut filtros = Vec::new();
    let mut longe: Option<f64> = None;
    for &(perto, area) in FAIXAS {
        if area > minima {
            let mut f = json!({"name": format!("Área de drenagem de {} ou mais", km2(area)), "where": format!("{campo} >= {area}"), "maxScale": perto});
            if let Some(l) = longe {
                f["minScale"] = json!(l);
            }
            filtros.push(f);
        }
        longe = Some(perto);
    }
    Value::Array(filtros)
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::TEMAS;

    fn bho() -> &'static Tema {
        TEMAS.iter().find(|t| t.id == "ana-bho").unwrap()
    }

    #[test]
    fn a_bho_e_um_tema_por_zoom_e_a_bc250_nao() {
        assert_eq!(campo(bho()), Some("nuareamont"));
        assert_eq!(campo(TEMAS.iter().find(|t| t.id == "drenagem").unwrap()), None);
        assert_eq!(filtro(bho(), "nuareamont", 10_000.0), "nuareamont >= 10000");
        assert_eq!(filtro(bho(), "nuareamont", 0.0), "");
        assert_eq!(km2(10_000.0), "10.000 km²");
        assert_eq!(km2(500.0), "500 km²");
        assert_eq!(km2(100_000.0), "100.000 km²");
    }

    #[test]
    fn a_area_minima_cabe_no_orcamento_com_poucas_contagens() {
        // Trechos com área ≥ A no retângulo do Brasil (ANA, 07/11/2026; de 500 para baixo,
        // estimados).
        let brasil = |a: f64| -> Result<u64, ()> {
            Ok(match a as u64 {
                100_000 => 4_235,
                50_000 => 8_440,
                20_000 => 17_682,
                10_000 => 27_431,
                5_000 => 42_386,
                2_000 => 69_162,
                1_000 => 98_463,
                0 => 3_244_592,
                _ => 200_000,
            })
        };
        let mut pedidas = Vec::new();
        let m = area_minima(ORCAMENTO, |a| {
            pedidas.push(a);
            brasil(a)
        });
        assert_eq!(m, Ok(10_000.0));
        // Nunca conta todos os trechos do Brasil (área ≥ 0), a contagem lenta.
        assert!(pedidas.len() <= 4 && !pedidas.contains(&0.0), "{pedidas:?}");
        // Uma área pequena: cabe tudo.
        assert_eq!(area_minima(ORCAMENTO, |_| Ok::<u64, ()>(500)), Ok(0.0));
        // Nem os maiores cabem: vêm eles.
        assert_eq!(area_minima(ORCAMENTO, |_| Ok::<u64, ()>(50_000)), Ok(100_000.0));
        // Um erro (ou um pedido pendente, no app web) para a busca.
        assert_eq!(area_minima(ORCAMENTO, |_| Err::<u64, &str>("pendente")), Err("pendente"));
    }

    #[test]
    fn os_filtros_mostram_rios_menores_de_perto() {
        let f = filtros_de_exibicao("nuareamont", 0.0);
        let f = f.as_array().unwrap();
        assert_eq!(f.len(), FAIXAS.len());
        assert_eq!(
            f[0],
            json!({"name": "Área de drenagem de 50.000 km² ou mais", "where": "nuareamont >= 50000", "maxScale": 20_000_000.0})
        );
        assert_eq!(f[1]["minScale"], 20_000_000.0);
        assert_eq!(f[1]["maxScale"], 10_000_000.0);
        assert_eq!(f.last().unwrap()["where"], "nuareamont >= 20");
        // Com 10 mil km² como mínima, as faixas de 10 mil para baixo não filtram nada.
        let g = filtros_de_exibicao("nuareamont", 10_000.0);
        assert_eq!(g.as_array().unwrap().len(), 2);
        assert_eq!(g[1]["where"], "nuareamont >= 20000");
    }
}
