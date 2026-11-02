//! Os indicadores por município: os endereços das fontes e a leitura das respostas, campo a campo.
//!
//! Cada fonte dá, para cada município (código do IBGE, 7 dígitos), alguns campos da camada
//! "Indicadores por município":
//! - **Censos** do IBGE ([`crate::censo`]): população, área e densidade de 2022, estabelecimentos
//!   e agricultura familiar de 2017.
//! - **Censo Agropecuário** de 2017 (IBGE, [`crate::censo::AGRO`]): utilização das terras,
//!   rebanho bovino, pessoal ocupado, tratores, irrigação, agrotóxicos, energia, orientação
//!   técnica e valor da produção.
//! - **Pix** (Banco Central, Olinda): o último mês completo, todos os municípios num pedido só.
//! - **Crédito rural de custeio** (Banco Central, SICOR): o último ano completo, em pedidos de
//!   [`POR_PEDIDO_SICOR`] municípios (o SICOR leva de 9 a 19 s por pedido).
//! - **Eleições** (TSE, a divulgação dos resultados): eleitorado, comparecimento e abstenção do
//!   1º turno de 2026 e de 2024, um arquivo por UF.
//! - **Emprego formal** (Novo CAGED, pelo IPEA Data): um retrato embutido (`caged.csv`, de
//!   `scripts/baixar_caged.py`), porque a API do IPEA só devolve a série inteira (64 MB).

use serde_json::{Map, Value, json};
use std::collections::HashMap;

/// Os campos de cada município, pelo código do IBGE.
pub type PorMunicipio = HashMap<String, Map<String, Value>>;

/// Uma fonte de indicadores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fonte {
    Censo,
    Agro,
    Pix,
    Credito,
    Tse,
    Caged,
}

pub const TODAS: [Fonte; 6] = [Fonte::Censo, Fonte::Agro, Fonte::Pix, Fonte::Credito, Fonte::Tse, Fonte::Caged];

impl Fonte {
    /// O nome nos parâmetros (`{"fontes": ["pix", …]}`).
    pub fn id(self) -> &'static str {
        match self {
            Fonte::Censo => "censo",
            Fonte::Agro => "agro",
            Fonte::Pix => "pix",
            Fonte::Credito => "credito",
            Fonte::Tse => "tse",
            Fonte::Caged => "caged",
        }
    }

    pub fn de(id: &str) -> Option<Fonte> {
        TODAS.into_iter().find(|f| f.id() == id)
    }

    /// O começo dos nomes dos campos da fonte (os censos têm os nomes das variáveis).
    pub fn eh_campo(self, campo: &str) -> bool {
        match self {
            // Os campos que as duas trazem (estabelecimentos e agricultura familiar, da tabela 6753,
            // que as duas citações nomeiam) não dizem qual delas está na camada.
            Fonte::Censo => {
                crate::censo::VARIAVEIS.iter().any(|v| v.campo == campo) && !crate::censo::AGRO.iter().any(|v| v.campo == campo)
            }
            Fonte::Agro => crate::censo::AGRO.iter().any(|v| v.campo == campo) && !crate::censo::VARIAVEIS.iter().any(|c| c.campo == campo),
            Fonte::Pix => campo.starts_with("pix_"),
            Fonte::Credito => campo.starts_with("credito_"),
            Fonte::Tse => ["eleitores_", "comparec_", "abstencao_"].iter().any(|p| campo.starts_with(p)),
            Fonte::Caged => campo.starts_with("caged_"),
        }
    }

    /// A citação.
    pub fn citacao(self) -> String {
        match self {
            Fonte::Censo => "IBGE, Censo Demográfico 2022 (tabela 4714) e Censo Agropecuário 2017 (tabela 6753), API de agregados; contornos da API de malhas".into(),
            Fonte::Agro => {
                let campos: Vec<String> = crate::censo::AGRO.iter().map(|v| format!("{}: {}", v.campo, v.descricao)).collect();
                format!(
                    "IBGE, Censo Agropecuário 2017, resultados definitivos (tabelas 6753, 6778, 6780, 6852, 6858, 6870, 6881, 6884, 6897 e 6907), API de agregados; contornos da API de malhas. Campos: {}",
                    campos.join("; ")
                )
            }
            Fonte::Pix => "Banco Central do Brasil, Estatísticas do Pix por município (Olinda, Pix_DadosAbertos); valores em R$".into(),
            Fonte::Credito => "Banco Central do Brasil, Matriz de Dados do Crédito Rural (SICOR), custeio por município e produto; valores em R$, Pronaf = programa 0001".into(),
            Fonte::Tse => "Tribunal Superior Eleitoral, divulgação oficial dos resultados, 1º turno de 2026 (eleição 6257) e de 2024 (619)".into(),
            Fonte::Caged => format!("Ministério do Trabalho e Emprego, Novo CAGED sem ajuste, pelo IPEA Data (ADMISNC, DESLIGNC); retrato até {}", caged_ate()),
        }
    }

    /// A licença.
    pub fn licenca(self) -> &'static str {
        match self {
            Fonte::Censo | Fonte::Agro => "uso livre com citação da fonte",
            Fonte::Pix | Fonte::Credito => "ODbL (dados abertos do Banco Central)",
            Fonte::Tse => "CC BY (dados abertos do TSE)",
            Fonte::Caged => "dados abertos; licença do IPEA Data não declarada",
        }
    }
}

/// Junta `novos` em `todos`, município a município.
pub fn juntar(todos: &mut PorMunicipio, novos: PorMunicipio) {
    for (cd, campos) in novos {
        todos.entry(cd).or_default().extend(campos);
    }
}

/// `x` com duas casas.
fn duas(x: f64) -> Value {
    json!((x * 100.0).round() / 100.0)
}

// ---------------------------------------------------------------------------------------------
// Pix

const PIX: &str = "https://olinda.bcb.gov.br/olinda/servico/Pix_DadosAbertos/versao/v1/odata/TransacoesPixPorMunicipio(DataBase=@DataBase)";

/// O pedido do mês mais recente do Pix (o mês corrente, ainda incompleto). É também o relógio
/// do plugin: o wasm não sabe a data, e o ano corrente diz qual é o último ano completo do
/// crédito rural.
pub fn url_ultimo_mes() -> String {
    format!("{PIX}?@DataBase='202001'&$format=json&$top=1&$orderby=AnoMes%20desc&$select=AnoMes")
}

/// O mês (`AAAAMM`) de uma resposta a [`url_ultimo_mes`].
pub fn ultimo_mes(r: &Value) -> Option<u32> {
    r["value"][0]["AnoMes"].as_u64().map(|m| m as u32)
}

/// O mês antes de `m` (`AAAAMM`).
pub fn mes_anterior(m: u32) -> u32 {
    if m % 100 == 1 { m - 100 + 11 } else { m - 1 }
}

/// "2026-09" de 202609.
fn mes_texto(m: u32) -> String {
    format!("{}-{:02}", m / 100, m % 100)
}

/// O Pix do mês `mes` nos municípios `codigos`, num pedido só.
pub fn url_pix(mes: u32, codigos: &[String]) -> String {
    let ou = codigos.iter().map(|c| format!("Municipio_Ibge%20eq%20{c}")).collect::<Vec<_>>().join("%20or%20");
    format!(
        "{PIX}?@DataBase='{mes}'&$format=json&$top=10000&$filter=AnoMes%20eq%20{mes}%20and%20({ou})&$select=Municipio_Ibge,VL_PagadorPF,VL_PagadorPJ,QT_PagadorPF,QT_PagadorPJ,VL_RecebedorPF,VL_RecebedorPJ,QT_PES_PagadorPF"
    )
}

/// Os campos do Pix de uma resposta a [`url_pix`].
pub fn campos_pix(r: &Value, mes: u32) -> PorMunicipio {
    let mut out = PorMunicipio::new();
    for v in r["value"].as_array().into_iter().flatten() {
        let Some(cd) = v["Municipio_Ibge"].as_u64() else {
            continue;
        };
        let n = |k: &str| v[k].as_f64().unwrap_or(0.0);
        let mut c = Map::new();
        c.insert("pix_mes".into(), json!(mes_texto(mes)));
        c.insert("pix_pago".into(), duas(n("VL_PagadorPF") + n("VL_PagadorPJ")));
        c.insert("pix_recebido".into(), duas(n("VL_RecebedorPF") + n("VL_RecebedorPJ")));
        c.insert("pix_transacoes".into(), json!((n("QT_PagadorPF") + n("QT_PagadorPJ")) as u64));
        c.insert("pix_pf_pagadoras".into(), json!(n("QT_PES_PagadorPF") as u64));
        out.insert(cd.to_string(), c);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Crédito rural

const SICOR: &str = "https://olinda.bcb.gov.br/olinda/servico/SICOR/versao/v2/odata/CusteioMunicipioProduto";

/// Municípios por pedido ao SICOR: 10 levam 19 s, e o `host.fetch` espera 20.
pub const POR_PEDIDO_SICOR: usize = 6;

/// Os pedidos do custeio de `ano` nos municípios `codigos`.
pub fn urls_credito(ano: u32, codigos: &[String]) -> Vec<String> {
    codigos
        .chunks(POR_PEDIDO_SICOR)
        .map(|parte| {
            let ou = parte.iter().map(|c| format!("codIbge%20eq%20'{c}'")).collect::<Vec<_>>().join("%20or%20");
            format!("{SICOR}?$filter=({ou})%20and%20AnoEmissao%20eq%20'{ano}'&$format=json&$top=100000")
        })
        .collect()
}

/// Os campos do crédito de custeio de `ano`, somados das respostas a [`urls_credito`]. Os
/// municípios de `codigos` sem contrato no ano ficam com zero.
pub fn campos_credito(respostas: &[Value], ano: u32, codigos: &[String]) -> PorMunicipio {
    let mut soma: HashMap<String, [f64; 5]> = codigos.iter().map(|c| (c.clone(), [0.0; 5])).collect();
    for r in respostas {
        for v in r["value"].as_array().into_iter().flatten() {
            let Some(s) = v["codIbge"].as_str().and_then(|c| soma.get_mut(c)) else {
                continue;
            };
            let valor = v["VlCusteio"].as_f64().unwrap_or(0.0);
            s[0] += valor;
            if v["cdPrograma"].as_str() == Some("0001") {
                s[1] += valor;
            }
            match v["Atividade"].as_str() {
                Some("1") => s[2] += valor,
                Some("2") => s[3] += valor,
                _ => {}
            }
            s[4] += v["AreaCusteio"].as_f64().unwrap_or(0.0);
        }
    }
    soma.into_iter()
        .map(|(cd, s)| {
            let mut c = Map::new();
            c.insert("credito_ano".into(), json!(ano));
            c.insert("credito_custeio".into(), duas(s[0]));
            c.insert("credito_pronaf".into(), duas(s[1]));
            c.insert("credito_agricola".into(), duas(s[2]));
            c.insert("credito_pecuario".into(), duas(s[3]));
            c.insert("credito_area_ha".into(), duas(s[4]));
            (cd, c)
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Eleições

/// Eleitorado, comparecimento e abstenção (%) de cada município no 1º turno de 2026 e de 2024,
/// de `scripts/baixar_tse.py` (a divulgação oficial do TSE). Lidos no plugin, os arquivos do TSE
/// (534 KB de códigos e ~190 KB por UF e eleição) gastavam o orçamento de uma chamada, e o
/// resultado de um turno encerrado não muda.
const TSE: &str = include_str!("../tse.csv");

/// Os campos das eleições dos municípios `codigos`, da tabela embutida. Um município sem eleição
/// num ano (o DF e Fernando de Noronha em 2024) fica sem os campos desse ano.
pub fn campos_tse(codigos: &[String]) -> PorMunicipio {
    let mut linhas = TSE.lines();
    let cabecalho: Vec<&str> = linhas.next().unwrap_or_default().split(',').collect();
    let mut out = PorMunicipio::new();
    for l in linhas {
        let f: Vec<&str> = l.split(',').collect();
        let Some(cd) = f.first().filter(|cd| codigos.iter().any(|c| c == *cd)) else {
            continue;
        };
        let mut c = Map::new();
        for (k, v) in cabecalho.iter().zip(&f).skip(1) {
            if v.is_empty() {
                continue;
            }
            let valor =
                if k.starts_with("eleitores_") { v.parse::<u64>().map(|n| json!(n)).ok() } else { v.parse::<f64>().map(|x| json!(x)).ok() };
            if let Some(valor) = valor {
                c.insert((*k).to_string(), valor);
            }
        }
        out.insert((*cd).to_string(), c);
    }
    out
}

// ---------------------------------------------------------------------------------------------
// Emprego formal

const CAGED: &str = include_str!("../caged.csv");

/// O último mês do retrato do CAGED ("AAAA-MM").
pub fn caged_ate() -> &'static str {
    CAGED.lines().nth(1).and_then(|l| l.split(',').nth(1)).unwrap_or("?")
}

/// Os campos do CAGED dos municípios `codigos`, do retrato embutido.
pub fn campos_caged(codigos: &[String]) -> PorMunicipio {
    let mut out = PorMunicipio::new();
    for l in CAGED.lines().skip(1) {
        let f: Vec<&str> = l.split(',').collect();
        let [cd, mes, adm, desl, adm12, desl12] = f[..] else {
            continue;
        };
        if !codigos.iter().any(|c| c == cd) {
            continue;
        }
        let n = |s: &str| s.parse::<i64>().unwrap_or(0);
        let mut c = Map::new();
        c.insert("caged_mes".into(), json!(mes));
        c.insert("caged_saldo_mes".into(), json!(n(adm) - n(desl)));
        c.insert("caged_admissoes_12m".into(), json!(n(adm12)));
        c.insert("caged_desligamentos_12m".into(), json!(n(desl12)));
        c.insert("caged_saldo_12m".into(), json!(n(adm12) - n(desl12)));
        out.insert(cd.to_string(), c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cods(c: &[&str]) -> Vec<String> {
        c.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn o_pix_pede_o_mes_completo_de_todos_os_municipios_de_uma_vez() {
        assert_eq!(ultimo_mes(&json!({"value": [{"AnoMes": 202610}]})), Some(202610));
        assert_eq!(mes_anterior(202610), 202609);
        assert_eq!(mes_anterior(202601), 202512);
        let u = url_pix(202609, &cods(&["2211001", "2112209"]));
        assert!(
            u.contains("@DataBase='202609'")
                && u.contains("AnoMes%20eq%20202609%20and%20(Municipio_Ibge%20eq%202211001%20or%20Municipio_Ibge%20eq%202112209)"),
            "{u}"
        );
        let r = json!({"value": [{"Municipio_Ibge": 2211001, "VL_PagadorPF": 5297191999.52, "VL_PagadorPJ": 4868895572.74,
            "QT_PagadorPF": 30, "QT_PagadorPJ": 2, "VL_RecebedorPF": 1.0, "VL_RecebedorPJ": 2.0, "QT_PES_PagadorPF": 675290}]});
        let c = &campos_pix(&r, 202609)["2211001"];
        assert_eq!(c["pix_mes"], "2026-09");
        assert_eq!(c["pix_pago"], json!(10166087572.26));
        assert_eq!(c["pix_transacoes"], 32);
        assert_eq!(c["pix_pf_pagadoras"], 675290);
    }

    #[test]
    fn o_credito_vem_em_pedidos_pequenos_e_soma_pronaf_e_atividade() {
        let c: Vec<String> = (0..13).map(|i| format!("22110{i:02}")).collect();
        let u = urls_credito(2025, &c);
        assert_eq!(u.len(), 3);
        assert!(u[0].contains("codIbge%20eq%20'2211000'%20or%20") && u[0].contains("AnoEmissao%20eq%20'2025'"));
        let r = json!({"value": [
            {"codIbge": "2211209", "cdPrograma": "0001", "Atividade": "1", "VlCusteio": 100.0, "AreaCusteio": 10.0},
            {"codIbge": "2211209", "cdPrograma": "0050", "Atividade": "2", "VlCusteio": 50.5, "AreaCusteio": 0.0},
            {"codIbge": "9999999", "cdPrograma": "0001", "Atividade": "1", "VlCusteio": 7.0, "AreaCusteio": 1.0}
        ]});
        let m = campos_credito(&[r], 2025, &cods(&["2211209", "2211001"]));
        assert_eq!(m["2211209"]["credito_custeio"], json!(150.5));
        assert_eq!(m["2211209"]["credito_pronaf"], json!(100.0));
        assert_eq!(m["2211209"]["credito_pecuario"], json!(50.5));
        assert_eq!(m["2211209"]["credito_area_ha"], json!(10.0));
        // Sem contrato no ano: zero, não vazio.
        assert_eq!(m["2211001"]["credito_custeio"], json!(0.0));
        assert!(!m.contains_key("9999999"));
    }

    #[test]
    fn as_eleicoes_vem_da_tabela_embutida() {
        // Teresina (PI) e Timon (MA); Fernando de Noronha não tem eleição municipal.
        let m = campos_tse(&cods(&["2211001", "2112209", "2605459"]));
        assert_eq!(m["2211001"]["eleitores_2024"], 587098);
        assert_eq!(m["2211001"]["comparec_2024"], json!(83.46));
        assert!(m["2112209"]["eleitores_2026"].as_u64().unwrap() > 100_000);
        assert!(m["2605459"].contains_key("eleitores_2026") && !m["2605459"].contains_key("eleitores_2024"));
    }

    #[test]
    fn o_caged_vem_do_retrato_embutido() {
        assert_eq!(caged_ate().len(), 7);
        let c = campos_caged(&cods(&["2211001"]));
        let t = &c["2211001"];
        assert_eq!(t["caged_mes"], caged_ate());
        assert_eq!(
            t["caged_saldo_12m"].as_i64().unwrap(),
            t["caged_admissoes_12m"].as_i64().unwrap() - t["caged_desligamentos_12m"].as_i64().unwrap()
        );
        assert!(campos_caged(&cods(&["0000000"])).is_empty());
    }

    #[test]
    fn cada_campo_tem_uma_fonte() {
        assert_eq!(Fonte::de("pix"), Some(Fonte::Pix));
        assert!(Fonte::Censo.eh_campo("pop_2022") && !Fonte::Censo.eh_campo("pix_pago"));
        // Os campos que o Censo e o Censo Agropecuário trazem não contam para nenhum dos dois.
        assert!(Fonte::Agro.eh_campo("bovinos") && !Fonte::Agro.eh_campo("estab_2017") && !Fonte::Censo.eh_campo("estab_2017"));
        assert!(Fonte::Tse.eh_campo("comparec_2024") && Fonte::Caged.eh_campo("caged_mes"));
        assert!(Fonte::Caged.citacao().contains(caged_ate()));
    }
}
