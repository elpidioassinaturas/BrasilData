//! Os censos do IBGE: os endereços que o plugin pede e a leitura das respostas.
//!
//! - **Municípios da área:** o WFS do IBGE com só os códigos (`propertyName`, sem geometria).
//! - **Valores por município:** a API de agregados (SIDRA), `servicodados.ibge.gov.br/api/v3`:
//!   Censo Demográfico 2022 (tabela 4714) e Censo Agropecuário 2017 (tabela 6753); e, para a
//!   fonte do Censo Agropecuário, as tabelas dele em [`AGRO`].
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

/// Os campos da fonte do Censo Agropecuário de 2017: os da agricultura familiar (como nos
/// censos por município), a utilização das terras, o rebanho bovino, o pessoal ocupado, as
/// máquinas, a irrigação, os agrotóxicos, a energia elétrica, a orientação técnica e o valor da
/// produção. Uma tabela por tema; a classificação escolhe a categoria (as outras ficam no total).
pub const AGRO: &[Variavel] = &[
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 183,
        classificacao: None,
        campo: "estab_2017",
        descricao: "estabelecimentos agropecuários",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 184,
        classificacao: None,
        campo: "area_agro_ha",
        descricao: "área dos estabelecimentos (ha)",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 183,
        classificacao: Some("829[46304]"),
        campo: "estab_fam",
        descricao: "estabelecimentos da agricultura familiar",
    },
    Variavel {
        tabela: 6753,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("829[46304]"),
        campo: "area_fam_ha",
        descricao: "área da agricultura familiar (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[113471]"),
        campo: "lav_temp_ha",
        descricao: "lavouras temporárias (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[113470]"),
        campo: "lav_perm_ha",
        descricao: "lavouras permanentes (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[113472]"),
        campo: "past_nat_ha",
        descricao: "pastagens naturais (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[113469]"),
        campo: "past_boa_ha",
        descricao: "pastagens plantadas em boas condições (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[40678]"),
        campo: "past_ruim_ha",
        descricao: "pastagens plantadas em más condições (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[40679]"),
        campo: "mata_app_rl_ha",
        descricao: "matas naturais de preservação permanente ou reserva legal (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[40680]"),
        campo: "mata_nat_ha",
        descricao: "outras matas naturais (ha)",
    },
    Variavel {
        tabela: 6881,
        periodo: "2017",
        variavel: 184,
        classificacao: Some("222[40681]"),
        campo: "flor_plant_ha",
        descricao: "florestas plantadas (ha)",
    },
    Variavel {
        tabela: 6907,
        periodo: "2017",
        variavel: 2209,
        classificacao: Some("12443[110056]"),
        campo: "bovinos",
        descricao: "bovinos (cabeças)",
    },
    Variavel {
        tabela: 6884,
        periodo: "2017",
        variavel: 185,
        classificacao: None,
        campo: "pessoal_ocup",
        descricao: "pessoal ocupado (pessoas)",
    },
    Variavel { tabela: 6870, periodo: "2017", variavel: 1862, classificacao: None, campo: "tratores", descricao: "tratores" },
    Variavel {
        tabela: 6858,
        periodo: "2017",
        variavel: 2372,
        classificacao: None,
        campo: "estab_irrig",
        descricao: "estabelecimentos com irrigação",
    },
    Variavel {
        tabela: 6858,
        periodo: "2017",
        variavel: 2373,
        classificacao: None,
        campo: "area_irrig_ha",
        descricao: "área irrigada (ha)",
    },
    Variavel {
        tabela: 6852,
        periodo: "2017",
        variavel: 183,
        classificacao: Some("12521[111611]"),
        campo: "estab_agrotox",
        descricao: "estabelecimentos que usaram agrotóxicos",
    },
    Variavel {
        tabela: 6778,
        periodo: "2017",
        variavel: 183,
        classificacao: Some("309[3011]"),
        campo: "estab_energia",
        descricao: "estabelecimentos com energia elétrica",
    },
    Variavel {
        tabela: 6780,
        periodo: "2017",
        variavel: 183,
        classificacao: Some("12567[113111]"),
        campo: "estab_orient",
        descricao: "estabelecimentos que recebem orientação técnica",
    },
    Variavel {
        tabela: 6897,
        periodo: "2017",
        variavel: 1999,
        classificacao: None,
        campo: "vp_mil_rs",
        descricao: "valor da produção (mil R$)",
    },
    Variavel {
        tabela: 6897,
        periodo: "2017",
        variavel: 1999,
        classificacao: Some("12547[111999]"),
        campo: "vp_veg_mil_rs",
        descricao: "valor da produção vegetal (mil R$)",
    },
    Variavel {
        tabela: 6897,
        periodo: "2017",
        variavel: 1999,
        classificacao: Some("12547[111995]"),
        campo: "vp_ani_mil_rs",
        descricao: "valor da produção animal (mil R$)",
    },
];

/// A classificação e a categoria de `"222[113471]"`.
fn categoria(c: &str) -> Option<(u32, u32)> {
    let (cls, resto) = c.split_once('[')?;
    Some((cls.parse().ok()?, resto.strip_suffix(']')?.parse().ok()?))
}

/// O que junta variáveis num só pedido ao SIDRA: a tabela, o período e o id da classificação
/// (as categorias dela vão juntas no pedido).
type Chave = (u32, &'static str, Option<u32>);

/// Os pedidos ao SIDRA de `lista` para os `codigos`: um por tabela e classificação, com as
/// variáveis e as categorias juntas. Devolve o endereço, os índices de `lista` que ele traz e a
/// classificação, para ler a resposta ([`valores`]).
pub fn pedidos_sidra(lista: &[Variavel], codigos: &[String]) -> Vec<(String, Vec<usize>, Option<u32>)> {
    let mut grupos: Vec<(Chave, Vec<usize>)> = Vec::new();
    for (i, v) in lista.iter().enumerate() {
        let chave = (v.tabela, v.periodo, v.classificacao.and_then(categoria).map(|c| c.0));
        match grupos.iter_mut().find(|g| g.0 == chave) {
            Some(g) => g.1.push(i),
            None => grupos.push((chave, vec![i])),
        }
    }
    let locais = codigos.join(",");
    grupos
        .into_iter()
        .map(|((tabela, periodo, cls), idx)| {
            let mut vars: Vec<u32> = idx.iter().map(|&i| lista[i].variavel).collect();
            vars.sort_unstable();
            vars.dedup();
            let vars: Vec<String> = vars.iter().map(u32::to_string).collect();
            let mut url = format!(
                "https://servicodados.ibge.gov.br/api/v3/agregados/{tabela}/periodos/{periodo}/variaveis/{}?localidades=N6[{locais}]",
                vars.join("|")
            );
            if let Some(cls) = cls {
                let mut cats: Vec<String> = Vec::new();
                for c in idx.iter().filter_map(|&i| lista[i].classificacao.and_then(categoria)).map(|c| c.1.to_string()) {
                    if !cats.contains(&c) {
                        cats.push(c);
                    }
                }
                url.push_str(&format!("&classificacao={cls}[{}]", cats.join(",")));
            }
            (url, idx, cls)
        })
        .collect()
}

/// A categoria de `v` na sua classificação, para achar o valor em [`valores`].
pub fn categoria_de(v: &Variavel) -> Option<u32> {
    v.classificacao.and_then(categoria).map(|c| c.1)
}

/// Os valores de uma resposta do SIDRA: por código de município, por id de variável e categoria
/// da classificação `cls` pedida (`None` sem classificação). Os sinais do IBGE para "sem dado"
/// (`-`, `...`, `X` de sigilo) ficam de fora.
pub fn valores(resposta: &Value, cls: Option<u32>) -> HashMap<String, HashMap<(u32, Option<u32>), f64>> {
    let mut out: HashMap<String, HashMap<(u32, Option<u32>), f64>> = HashMap::new();
    for v in resposta.as_array().into_iter().flatten() {
        let Some(id) = v["id"].as_str().and_then(|s| s.parse::<u32>().ok()).or_else(|| v["id"].as_u64().map(|n| n as u32)) else {
            continue;
        };
        for r in v["resultados"].as_array().into_iter().flatten() {
            // A categoria deste resultado na classificação pedida.
            let cat = cls.and_then(|cls| {
                r["classificacoes"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|c| c["id"].as_u64() == Some(u64::from(cls)) || c["id"].as_str() == Some(cls.to_string().as_str()))
                    .and_then(|c| c["categoria"].as_object())
                    .and_then(|o| o.keys().next())
                    .and_then(|k| k.parse::<u32>().ok())
            });
            if cls.is_some() && cat.is_none() {
                continue;
            }
            for s in r["series"].as_array().into_iter().flatten() {
                let Some(cd) = s["localidade"]["id"].as_str() else {
                    continue;
                };
                let Some(valor) = s["serie"].as_object().and_then(|o| o.values().next()).and_then(Value::as_str) else {
                    continue;
                };
                if let Ok(x) = valor.trim().parse::<f64>() {
                    out.entry(cd.to_string()).or_default().insert((id, cat), x);
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
        let p = pedidos_sidra(VARIAVEIS, &["2211001".into(), "2112209".into()]);
        assert_eq!(p.len(), 3);
        assert!(p[0].0.contains("/agregados/4714/periodos/2022/variaveis/93|614|6318?localidades=N6[2211001,2112209]"), "{}", p[0].0);
        assert!(p[1].0.contains("/agregados/6753/periodos/2017/variaveis/183|184?") && !p[1].0.contains("classificacao"));
        assert!(p[2].0.ends_with("&classificacao=829[46304]"));
        assert_eq!((p[2].1.clone(), p[2].2), (vec![5, 6], Some(829)));
    }

    #[test]
    fn as_categorias_de_uma_classificacao_vao_num_pedido_so() {
        let p = pedidos_sidra(AGRO, &["2211001".into()]);
        let terras = p.iter().find(|p| p.0.contains("/agregados/6881/")).unwrap();
        assert!(terras.0.ends_with("&classificacao=222[113471,113470,113472,113469,40678,40679,40680,40681]"), "{}", terras.0);
        assert_eq!(terras.1.len(), 8);
        let producao = p.iter().filter(|p| p.0.contains("/agregados/6897/")).count();
        assert_eq!(producao, 2, "o total e a classificação por tipo de produção");
        // Os campos não se repetem.
        let mut campos: Vec<&str> = AGRO.iter().map(|v| v.campo).collect();
        campos.sort_unstable();
        campos.dedup();
        assert_eq!(campos.len(), AGRO.len());
    }

    #[test]
    fn os_valores_de_varias_categorias_vem_cada_um_na_sua() {
        let r = json!([{"id": "184", "resultados": [
            {"classificacoes": [{"id": "829", "categoria": {"46302": "Total"}}, {"id": "222", "categoria": {"113471": "Lavouras - temporárias"}}],
             "series": [{"localidade": {"id": "2211001"}, "serie": {"2017": "1138"}}]},
            {"classificacoes": [{"id": "829", "categoria": {"46302": "Total"}}, {"id": "222", "categoria": {"113472": "Pastagens - naturais"}}],
             "series": [{"localidade": {"id": "2211001"}, "serie": {"2017": "4589"}}]}
        ]}]);
        let v = valores(&r, Some(222));
        assert_eq!((v["2211001"][&(184, Some(113471))], v["2211001"][&(184, Some(113472))]), (1138.0, 4589.0));
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
        let v = valores(&r, None);
        assert_eq!(v["2211001"][&(93, None)], 866300.0);
        assert_eq!(v["2211001"][&(614, None)], 622.66);
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
