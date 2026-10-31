//! Os censos do IBGE: os endereços que o plugin pede e a leitura das respostas.
//!
//! - **Municípios da área:** o WFS do IBGE com só os códigos (`propertyName`, sem geometria).
//! - **Valores por município:** a API de agregados (SIDRA), `servicodados.ibge.gov.br/api/v3`:
//!   Censo Demográfico 2022 (tabela 4714) e Censo Agropecuário 2017 (tabela 6753).
//! - **Contornos:** a API de malhas, um município por pedido (ela não aceita vários).
//! - **Setores censitários de 2022:** um GeoPackage por estado, com os agregados básicos.

use serde_json::Value;
use std::collections::HashMap;

/// Os municípios que cruzam `c`: só código, nome e UF, sem geometria (poucos KB).
pub fn url_municipios(c: [f64; 4]) -> String {
    format!(
        "https://geoservicos.ibge.gov.br/geoserver/wfs?service=WFS&version=2.0.0&request=GetFeature&typeNames=CGMAT:qg_2022_030_munic&outputFormat=application/json&propertyName=cd_mun,nm_mun,sigla_uf&bbox={:.6},{:.6},{:.6},{:.6},EPSG:4674",
        c[0], c[1], c[2], c[3]
    )
}

/// Os municípios de uma resposta de [`url_municipios`]: `(código, nome, UF)`, sem repetição.
pub fn municipios(resposta: &Value) -> Vec<(String, String, String)> {
    let mut out: Vec<(String, String, String)> = Vec::new();
    for f in resposta["features"].as_array().into_iter().flatten() {
        let p = &f["properties"];
        let (Some(cd), Some(nm), Some(uf)) = (p["cd_mun"].as_str(), p["nm_mun"].as_str(), p["sigla_uf"].as_str()) else {
            continue;
        };
        if !out.iter().any(|m| m.0 == cd) {
            out.push((cd.to_string(), nm.to_string(), uf.to_string()));
        }
    }
    out
}

/// O contorno de um município pela API de malhas (qualidade intermediária: leve).
pub fn url_malha(codigo: &str) -> String {
    format!("https://servicodados.ibge.gov.br/api/v4/malhas/municipios/{codigo}?formato=application/vnd.geo%2Bjson&qualidade=intermediaria")
}

/// Uma variável de uma tabela do SIDRA que vira um campo da camada.
pub struct Variavel {
    pub tabela: u32,
    pub periodo: &'static str,
    pub variavel: u32,
    /// `classificação[categoria]`, quando não é o total.
    pub classificacao: Option<&'static str>,
    pub campo: &'static str,
    /// Para o painel: o que o campo é, com a unidade.
    pub descricao: &'static str,
}

/// Os campos da camada de censos por município.
pub const VARIAVEIS: &[Variavel] = &[
    Variavel {
        tabela: 4714,
        periodo: "2022",
        variavel: 93,
        classificacao: None,
        campo: "pop_2022",
        descricao: "população residente em 2022 (pessoas)",
    },
    Variavel {
        tabela: 4714,
        periodo: "2022",
        variavel: 6318,
        classificacao: None,
        campo: "area_km2",
        descricao: "área do município (km²)",
    },
    Variavel {
        tabela: 4714,
        periodo: "2022",
        variavel: 614,
        classificacao: None,
        campo: "dens_2022",
        descricao: "densidade demográfica em 2022 (hab/km²)",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 183,
        classificacao: None,
        campo: "estab_2017",
        descricao: "estabelecimentos agropecuários em 2017",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 184,
        classificacao: None,
        campo: "area_agro_ha",
        descricao: "área dos estabelecimentos agropecuários em 2017 (ha)",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 183,
        classificacao: Some("829[46304]"),
        campo: "estab_fam",
        descricao: "estabelecimentos da agricultura familiar em 2017",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("829[46304]"),
        campo: "area_fam_ha",
        descricao: "área da agricultura familiar em 2017 (ha)",
    },
];

/// O que junta variáveis num só pedido ao SIDRA: a tabela, o período e a classificação.
type Chave = (u32, &'static str, Option<&'static str>);

/// Os pedidos ao SIDRA para os `codigos`: um por tabela e classificação, com as variáveis
/// juntas. Devolve o endereço e os índices de [`VARIAVEIS`] que ele traz.
pub fn pedidos_sidra(codigos: &[String]) -> Vec<(String, Vec<usize>)> {
    let mut grupos: Vec<(Chave, Vec<usize>)> = Vec::new();
    for (i, v) in VARIAVEIS.iter().enumerate() {
        let chave = (v.tabela, v.periodo, v.classificacao);
        match grupos.iter_mut().find(|g| g.0 == chave) {
            Some(g) => g.1.push(i),
            None => grupos.push((chave, vec![i])),
        }
    }
    let locais = codigos.join(",");
    grupos
        .into_iter()
        .map(|((tabela, periodo, classificacao), idx)| {
            let vars: Vec<String> = idx.iter().map(|&i| VARIAVEIS[i].variavel.to_string()).collect();
            let mut url = format!(
                "https://servicodados.ibge.gov.br/api/v3/agregados/{tabela}/periodos/{periodo}/variaveis/{}?localidades=N6[{locais}]",
                vars.join("|")
            );
            if let Some(c) = classificacao {
                url.push_str(&format!("&classificacao={c}"));
            }
            (url, idx)
        })
        .collect()
}

/// Os valores de uma resposta do SIDRA: por código de município, por id de variável. Os
/// sinais do IBGE para "sem dado" (`-`, `...`, `X` de sigilo) ficam de fora.
pub fn valores(resposta: &Value) -> HashMap<String, HashMap<u32, f64>> {
    let mut out: HashMap<String, HashMap<u32, f64>> = HashMap::new();
    for v in resposta.as_array().into_iter().flatten() {
        let Some(id) = v["id"].as_str().and_then(|s| s.parse::<u32>().ok()).or_else(|| v["id"].as_u64().map(|n| n as u32)) else {
            continue;
        };
        for r in v["resultados"].as_array().into_iter().flatten() {
            for s in r["series"].as_array().into_iter().flatten() {
                let Some(cd) = s["localidade"]["id"].as_str() else {
                    continue;
                };
                let Some(valor) = s["serie"].as_object().and_then(|o| o.values().next()).and_then(Value::as_str) else {
                    continue;
                };
                if let Ok(x) = valor.trim().parse::<f64>() {
                    out.entry(cd.to_string()).or_default().insert(id, x);
                }
            }
        }
    }
    out
}

/// O GeoPackage dos setores censitários de 2022 de uma UF, com os agregados básicos.
pub fn url_setores(uf: &str) -> String {
    format!(
        "https://ftp.ibge.gov.br/Censos/Censo_Demografico_2022/Agregados_por_Setores_Censitarios/malha_com_atributos/setores/gpkg/UF/{uf}/{uf}_setores_CD2022.gpkg"
    )
}

/// O que são os campos `v0001` a `v0007` dos setores (dicionário de dados do IBGE).
pub const CAMPOS_DOS_SETORES: &str = "v0001: pessoas; v0002: domicílios; v0003: domicílios particulares; v0004: domicílios coletivos; v0005: média de moradores por domicílio particular ocupado; v0006: % de domicílios particulares ocupados imputados; v0007: domicílios particulares ocupados.";

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn os_pedidos_ao_sidra_juntam_as_variaveis_de_cada_tabela() {
        let p = pedidos_sidra(&["2211001".into(), "2112209".into()]);
        assert_eq!(p.len(), 3);
        assert!(p[0].0.contains("/agregados/4714/periodos/2022/variaveis/93|6318|614?localidades=N6[2211001,2112209]"), "{}", p[0].0);
        assert!(p[1].0.contains("/agregados/6753/periodos/2017/variaveis/183|184?") && !p[1].0.contains("classificacao"));
        assert!(p[2].0.ends_with("&classificacao=829[46304]"));
        assert_eq!(p[2].1, vec![5, 6]);
    }

    #[test]
    fn os_valores_vem_por_municipio_e_variavel_sem_os_sinais_de_falta() {
        let r = json!([
            {"id": "93", "resultados": [{"series": [
                {"localidade": {"id": "2211001"}, "serie": {"2022": "866300"}},
                {"localidade": {"id": "2112209"}, "serie": {"2022": "-"}}
            ]}]},
            {"id": "614", "resultados": [{"series": [{"localidade": {"id": "2211001"}, "serie": {"2022": "622.66"}}]}]}
        ]);
        let v = valores(&r);
        assert_eq!(v["2211001"][&93], 866300.0);
        assert_eq!(v["2211001"][&614], 622.66);
        assert!(!v.contains_key("2112209"));
    }

    #[test]
    fn os_municipios_vem_sem_repeticao() {
        let r = json!({"features": [
            {"properties": {"cd_mun": "2211001", "nm_mun": "Teresina", "sigla_uf": "PI"}},
            {"properties": {"cd_mun": "2211001", "nm_mun": "Teresina", "sigla_uf": "PI"}},
            {"properties": {"cd_mun": "2112209", "nm_mun": "Timon", "sigla_uf": "MA"}}
        ]});
        assert_eq!(municipios(&r).len(), 2);
        assert!(url_setores("PI").ends_with("/gpkg/UF/PI/PI_setores_CD2022.gpkg"));
    }
}
