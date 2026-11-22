//! A drenagem da BC250 2025 com a ordem de Strahler e a drenagem a montante de cada trecho,
//! calculadas pelo BrasilData na rede do país inteiro (`scripts/ordem_drenagem.py`) e publicadas
//! numa release de dados do repositório, em GeoParquet.
//!
//! - Numa área: os arquivos das UFs da área, baixados inteiros, juntados e recortados.
//! - Sem área, ou numa área de muitas UFs: o país inteiro, lido pela web só na parte da vista, em
//!   níveis: de longe, um arquivo só com os rios grandes; de perto, o arquivo completo.
//!
//! Nos dois casos, filtros de exibição pela drenagem a montante (`montante_km`) deixam de longe
//! só os rios principais. A ordem não serve para isso: na escala 1:250.000 a maioria dos rios
//! médios tem ordem 5 a 7, e a ordem não distingue um rio longo de um curto.

use serde_json::{Value, json};

/// A release de dados com os arquivos.
pub const BASE: &str = "https://github.com/elpidioassinaturas/BrasilData/releases/download/dados-drenagem-bc250-2025/";

/// O campo da drenagem a montante (km).
pub const MONTANTE: &str = "montante_km";

/// Mais UFs que isto numa área: o país inteiro pela vista, sem recorte (as UFs da Amazônia têm
/// até 87 MB cada).
pub const MAX_UFS: usize = 6;

/// O arquivo de uma UF (sigla em maiúsculas).
pub fn url_uf(uf: &str) -> String {
    format!("{BASE}drenagem_{}.parquet", uf.to_uppercase())
}

/// Os níveis do país inteiro, de longe para perto: (arquivo, nome da camada, escala além da qual
/// some de longe, escala além da qual some de perto). Cada um fica abaixo das 60 mil feições que o
/// RustGIS lê numa vista.
pub const NIVEIS: &[(&str, &str, Option<f64>, Option<f64>)] = &[
    ("drenagem_montante_5000km.parquet", "rios com 5.000 km ou mais a montante", None, Some(10_000_000.0)),
    ("drenagem_montante_1000km.parquet", "rios com 1.000 km ou mais a montante", Some(10_000_000.0), Some(2_500_000.0)),
    ("drenagem_montante_100km.parquet", "rios com 100 km ou mais a montante", Some(2_500_000.0), Some(800_000.0)),
    ("bc250_2025_drenagem_ordem.parquet", "todos os trechos", Some(800_000.0), None),
];

/// A drenagem a montante mínima (km) mostrada em cada faixa de escala, de longe para perto:
/// `(denominador, km)` diz que, de 1:`denominador` para longe (até a faixa anterior), só aparecem
/// os trechos com pelo menos `km`. Mais perto que a última faixa, aparece tudo.
const FAIXAS: &[(f64, f64)] =
    &[(15_000_000.0, 20_000.0), (10_000_000.0, 5_000.0), (2_500_000.0, 1_000.0), (800_000.0, 100.0), (300_000.0, 20.0)];

/// `km` com o ponto dos milhares.
fn milhar(km: f64) -> String {
    crate::zoom::km2(km).trim_end_matches(" km²").to_string()
}

/// Os filtros de exibição pela drenagem a montante. Faixas cujo mínimo não passa de `ja_filtrado`
/// (o que a camada já garante, como o nível de 1.000 km) ficam de fora.
pub fn filtros(ja_filtrado: f64) -> Value {
    let mut filtros = Vec::new();
    let mut longe: Option<f64> = None;
    for &(perto, km) in FAIXAS {
        if km > ja_filtrado {
            let mut f = json!({
                "name": format!("Rios com {} km ou mais a montante", milhar(km)),
                "where": format!("{MONTANTE} >= {km}"),
                "maxScale": perto,
            });
            if let Some(l) = longe {
                f["minScale"] = json!(l);
            }
            filtros.push(f);
        }
        longe = Some(perto);
    }
    Value::Array(filtros)
}

/// O mínimo de drenagem a montante (km) que o arquivo de um nível já garante.
pub fn minimo_do_nivel(arquivo: &str) -> f64 {
    arquivo.strip_prefix("drenagem_montante_").and_then(|r| r.strip_suffix("km.parquet")).and_then(|n| n.parse().ok()).unwrap_or(0.0)
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn enderecos_e_niveis() {
        assert_eq!(
            url_uf("mg"),
            "https://github.com/elpidioassinaturas/BrasilData/releases/download/dados-drenagem-bc250-2025/drenagem_MG.parquet"
        );
        assert_eq!(minimo_do_nivel("drenagem_montante_1000km.parquet"), 1000.0);
        assert_eq!(minimo_do_nivel("bc250_2025_drenagem_ordem.parquet"), 0.0);
        // Os níveis se encaixam: onde um some de perto, o seguinte aparece.
        for par in NIVEIS.windows(2) {
            assert_eq!(par[0].3, par[1].2);
        }
        assert!(NIVEIS[0].2.is_none() && NIVEIS.last().unwrap().3.is_none());
    }

    #[test]
    fn filtros_por_escala() {
        let todos = filtros(0.0);
        let todos = todos.as_array().unwrap();
        assert_eq!(todos.len(), FAIXAS.len());
        assert_eq!(todos[0]["where"], "montante_km >= 20000");
        assert_eq!(todos[0]["name"], "Rios com 20.000 km ou mais a montante");
        assert!(todos[0].get("minScale").is_none());
        for par in todos.windows(2) {
            assert_eq!(par[1]["minScale"], par[0]["maxScale"]);
        }
        assert_eq!(todos.last().unwrap()["maxScale"], 300_000.0);
        // O nível de 5.000 km só precisa do filtro de 20.000 km.
        let n5000 = filtros(5_000.0);
        assert_eq!(n5000.as_array().unwrap().len(), 1);
        // O nível de 100 km não leva o filtro de 100 nem os anteriores que ele já cumpre... mas
        // leva os de mais de 100 km, que valem de mais longe.
        let n100 = filtros(100.0);
        assert_eq!(n100.as_array().unwrap().len(), 3);
    }
}
