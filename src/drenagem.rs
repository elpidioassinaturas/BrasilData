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

/// A largura de cada trecho (pontos) pela ordem de Strahler, de 0,2 pt na ordem 1 a 3 pt na ordem
/// 10, numa curva que cresce devagar e depois depressa (ordem 6: 1,1 pt; ordem 8: 1,9 pt): de
/// longe, onde só aparecem rios de ordem 6 a 10, os maiores se destacam.
pub const LARGURA_PELA_ORDEM: &str = "scale_exp(strahler, 1, 10, 0.2, 3, 2)";

/// O azul das linhas da drenagem, que se distingue do mar e dos lagos do mapa-base.
pub const AZUL: &str = "#1f78b4";

/// O campo da drenagem a montante (km).
pub const MONTANTE: &str = "montante_km";

/// Mais UFs que isto numa área: o país inteiro pela vista, sem recorte (as UFs da Amazônia têm
/// até 87 MB cada).
pub const MAX_UFS: usize = 6;

/// O arquivo de uma UF (sigla em maiúsculas).
pub fn url_uf(uf: &str) -> String {
    format!("{BASE}drenagem_{}.parquet", uf.to_uppercase())
}

/// O fim do nome da camada dos rótulos: os rios em linhas contínuas, de linha invisível, só com
/// o nome (`scripts/rios_para_rotulos.py`). A BC250 corta cada rio em cada confluência, em
/// trechos curtos demais para um rótulo ao longo da linha.
pub const ROTULOS: &str = "nomes dos rios";

/// O nome da camada dos rótulos de um tema chamado `nome` no mapa.
pub fn camada_dos_rotulos(nome: &str) -> String {
    format!("{nome}: {ROTULOS}")
}

/// Se a camada `nome` é a dos rótulos (fica fora da legenda: a linha dela é invisível).
pub fn so_rotulos(nome: &str) -> bool {
    nome.ends_with(&format!(": {ROTULOS}"))
}

/// O arquivo dos rótulos de uma UF (as linhas recortadas por ela).
pub fn url_rotulos_uf(uf: &str) -> String {
    format!("{BASE}rios_nomes_{}.parquet", uf.to_uppercase())
}

/// Os rótulos do país inteiro, como [`NIVEIS`]: de longe, só os rios de ordem 6 ou mais (uma
/// amostra do arquivo inteiro deixaria rios grandes sem nome); de perto, todos.
pub const NIVEIS_DOS_ROTULOS: &[(&str, &str, Option<f64>, Option<f64>)] = &[
    ("rios_nomes_ordem6.parquet", "nomes dos rios de ordem 6 ou mais", None, Some(2_500_000.0)),
    ("rios_nomes.parquet", ROTULOS, Some(2_500_000.0), None),
];

/// Uma classe de rótulo dos nomes dos rios.
pub struct ClasseDeRotulo {
    pub nome: &'static str,
    /// A cláusula das feições que ela rotula.
    pub onde: &'static str,
    /// Em pontos.
    pub tamanho: f64,
    pub cor: &'static str,
    /// "parallel" (reto, ao longo da linha) ou "curved" (seguindo as curvas).
    pub posicao: &'static str,
    /// A escala além da qual ela some de longe.
    pub longe: Option<f64>,
}

/// As classes de rótulo, na ordem em que o RustGIS as coloca (a primeira tem a preferência). A
/// última é a Default, com a cláusula que sobra. A importância é a drenagem a montante, não a
/// ordem de Strahler: a ordem satura em 5 a 7 nos rios regionais, e na Zona da Mata o Rio Pomba
/// (5.415 km a montante) e o Xopotó (1.645 km), ambos de ordem 6, caíam na mesma classe, onde o
/// Xopotó ficava com o rótulo e o Pomba sem. As de cima pedem também um comprimento mínimo da
/// linha do rio: nas zonas de canais trançados (Amazônia, Pantanal) um furo curto leva parte da
/// drenagem do rio e não deve tomar o lugar dele.
pub const CLASSES_DE_ROTULO: &[ClasseDeRotulo] = &[
    ClasseDeRotulo {
        nome: "Rios principais",
        onde: "montante_km >= 5000 AND comp_km >= 30",
        tamanho: 10.0,
        cor: "#08306b",
        posicao: "parallel",
        longe: None,
    },
    ClasseDeRotulo {
        nome: "Rios médios",
        onde: "montante_km >= 1000 AND comp_km >= 10 AND NOT (montante_km >= 5000 AND comp_km >= 30)",
        tamanho: 8.5,
        cor: "#1f4e79",
        posicao: "parallel",
        longe: Some(2_000_000.0),
    },
    ClasseDeRotulo {
        nome: "Córregos e ribeirões",
        onde: "montante_km < 1000 OR comp_km < 10",
        tamanho: 7.0,
        cor: "#2c6a9e",
        posicao: "curved",
        longe: Some(400_000.0),
    },
];

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
    fn rotulos() {
        let n = camada_dos_rotulos("BrasilData: Drenagem (BC250 2025)");
        assert_eq!(n, "BrasilData: Drenagem (BC250 2025): nomes dos rios");
        assert!(so_rotulos(&n) && !so_rotulos("BrasilData: Drenagem (BC250 2025)"));
        assert!(url_rotulos_uf("sp").ends_with("/rios_nomes_SP.parquet"));
        for par in NIVEIS_DOS_ROTULOS.windows(2) {
            assert_eq!(par[0].3, par[1].2);
        }
        // A preferência é dos rios maiores: as classes vão da maior drenagem a montante para a menor.
        assert!(
            CLASSES_DE_ROTULO[0].onde.starts_with("montante_km >= 5000")
                && CLASSES_DE_ROTULO.last().unwrap().onde.starts_with("montante_km < 1000")
        );
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
