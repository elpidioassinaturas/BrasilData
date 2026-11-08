//! Os alertas de perturbação da vegetação do radar do Sentinel-1: o OPERA DIST-S1 (JPL),
//! distribuído pelo ASF DAAC.
//!
//! - **De onde:** a API de busca do ASF (`processingLevel=DIST-ALERT-S1`) lista os produtos que
//!   cruzam o retângulo da área, os mais recentes primeiro. Cada produto cobre um tile MGRS
//!   (cerca de 110 × 110 km, em UTM, com células de 30 m) e tem uma GeoTIFF por camada
//!   (`_GEN-DIST-STATUS.tif`, `_GEN-DIST-DATE.tif`…).
//! - **Acumulado:** cada produto traz a situação dos alertas desde o começo do monitoramento até a
//!   data dele. Por isso vale o produto mais recente de cada tile ([`mais_recentes`]); não é
//!   preciso juntar passagens, como no radar.
//! - **Login:** os arquivos pedem o token Earthdata Login, que o RustGIS guarda e manda, como no
//!   radar.
//! - **Os códigos:** os da OPERA (`opera-adt/dist-s1`, `constants.py`), na paleta do BrasilData
//!   ou na oficial ([`Paleta`]).

use std::collections::HashMap;

use serde_json::{Value, json};

use crate::datas::Data;
use crate::radar::{data_de, poligono};

/// O endereço da busca do ASF.
const BUSCA: &str = "https://api.daac.asf.alaska.edu/services/search/param";

/// Quantos produtos a busca devolve: os de um tile chegam a cada 6 dias ou menos, e 200 cobrem
/// semanas de uma mesorregião (16 tiles na Zona da Mata), em cerca de 1,6 MB (o `host.fetch`
/// aceita até 2 MB).
pub const MAX_BUSCA: usize = 200;

/// Mais tiles que isto é área grande demais para ler de uma vez.
pub const MAX_TILES: usize = 30;

/// Onde o ASF guarda os arquivos do OPERA DIST-S1.
const ARQUIVOS: &str = "https://cumulus.asf.earthdatacloud.nasa.gov/OPERA/OPERA_L3_DIST-ALERT-S1_V1";

/// O começo do monitoramento do DIST-S1: os primeiros produtos do ASF são desta data (conferido
/// em 08/11/2026: 195.724 produtos no mundo, de 04/01 a 28/02/2026).
pub const INICIO: Data = Data { ano: 2026, mes: 1, dia: 4 };

/// O dia 0 do `GEN-DIST-DATE`: os dias contam a partir de 31/12/2020.
const DIA_ZERO: Data = Data { ano: 2020, mes: 12, dia: 31 };

/// A citação do OPERA DIST-S1.
pub const CITACAO: &str = "OPERA Land Surface Disturbance Alert from Sentinel-1 validated product, version 1 (OPERA_L3_DIST-ALERT-S1_V1), JPL OPERA Project, distribuído pelo ASF DAAC (doi:10.5067/SNWG/OPL3DISTALS1-V1). Contém dados Copernicus Sentinel-1 modificados (ESA)";

/// A camada de um produto.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camada {
    /// A situação do alerta (códigos 0 a 8).
    Situacao,
    /// O dia da primeira detecção.
    Data,
    /// A confiança.
    Confianca,
    /// A intensidade: o maior desvio do sinal desde a primeira detecção.
    Intensidade,
}

impl Camada {
    /// O fim do nome do arquivo.
    pub fn sufixo(self) -> &'static str {
        match self {
            Camada::Situacao => "GEN-DIST-STATUS",
            Camada::Data => "GEN-DIST-DATE",
            Camada::Confianca => "GEN-DIST-CONF",
            Camada::Intensidade => "GEN-METRIC-MAX",
        }
    }

    /// Se os valores são códigos (classes, ou dias), que não se misturam ao reamostrar.
    pub fn discreta(self) -> bool {
        matches!(self, Camada::Situacao | Camada::Data)
    }

    /// Cerca do tamanho de um arquivo, em MB, para o aviso do download.
    pub fn megas(self) -> u32 {
        match self {
            Camada::Situacao | Camada::Data => 2,
            Camada::Confianca | Camada::Intensidade => 9,
        }
    }

    /// A escala de cores fixa: os percentis 2 e 98 dos alertas confirmados num tile de Viçosa
    /// (MG) em 28/02/2026 foram de 23 a 126 na confiança e de 3,2 a 12 na intensidade; os da
    /// primeira detecção começam em 2,5. Fixa, como no radar: datas e lugares comparáveis.
    pub fn escala(self) -> Option<(f64, f64)> {
        match self {
            Camada::Confianca => Some((2.5, 100.0)),
            Camada::Intensidade => Some((2.5, 12.0)),
            Camada::Situacao | Camada::Data => None,
        }
    }

    /// O nome da camada, com a data do produto mais recente.
    pub fn nome(self, ate: Data) -> String {
        let d = ate.texto();
        let p = crate::PREFIXO;
        match self {
            Camada::Situacao => format!("{p}Perturbação da vegetação (OPERA DIST-S1, até {d})"),
            Camada::Data => format!("{p}Perturbação, dias antes de {d} (OPERA DIST-S1)"),
            Camada::Confianca => format!("{p}Confiança da perturbação (OPERA DIST-S1, até {d})"),
            Camada::Intensidade => format!("{p}Intensidade da perturbação (OPERA DIST-S1, até {d})"),
        }
    }

    /// Um nome curto, para os avisos.
    pub fn curto(self) -> &'static str {
        match self {
            Camada::Situacao => "situação",
            Camada::Data => "data",
            Camada::Confianca => "confiança",
            Camada::Intensidade => "intensidade",
        }
    }
}

/// A paleta da situação.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paleta {
    /// A do BrasilData (padrão): o matiz diz a etapa (amarelo, laranja, vermelho, roxo) e, em
    /// cada etapa, a confiança alta é a cor mais escura.
    BrasilData,
    /// A oficial da OPERA (`opera-adt/dist-s1`, `constants.py`, `DIST_STATUS_CMAP`).
    Oficial,
}

impl Paleta {
    /// A paleta pedida (`"brasildata"`, o padrão, ou `"oficial"`).
    pub fn ler(texto: Option<&str>) -> Result<Paleta, String> {
        match texto.map(str::trim) {
            None | Some("") | Some("brasildata") => Ok(Paleta::BrasilData),
            Some("oficial") => Ok(Paleta::Oficial),
            Some(t) => Err(format!("paleta '{t}': use \"brasildata\" (o padrão) ou \"oficial\" (a da OPERA)")),
        }
    }

    /// As cores e o alfa dos códigos 1 a 8.
    ///
    /// - **BrasilData:** medida contra o daltonismo (Machado et al., 2009): a menor distância de
    ///   cor entre duas classes é ΔE 25,6 na visão normal, 17,8 na deuteranopia, 21,5 na
    ///   protanopia e 9,5 na tritanopia (a oficial: 21,0; 11,9; 5,9, com o 4 e o 7 quase iguais;
    ///   11,3). A primeira detecção já é clara: não precisa de transparência.
    /// - **Oficial:** feita para fundo preto. Sobre um mapa claro, a primeira detecção (1 e 4), a
    ///   mais comum e ainda não confirmada (12% de um tile de Viçosa em 28/02/2026, contra 0,35%
    ///   de confirmadas), cobria tudo: ela fica translúcida.
    pub fn cores(self) -> [(u8, &'static str, u8); 8] {
        match self {
            Paleta::BrasilData => [
                (1, "#fbe6a2", 255),
                (2, "#f0a06e", 255),
                (3, "#ee8a8a", 255),
                (4, "#f6d256", 255),
                (5, "#e8711f", 255),
                (6, "#b3121d", 255),
                (7, "#d3c3e6", 255),
                (8, "#7a55a8", 255),
            ],
            Paleta::Oficial => [
                (1, "#005555", 110),
                (2, "#897f4e", 255),
                (3, "#dee043", 255),
                (4, "#008888", 110),
                (5, "#e48727", 255),
                (6, "#e01b07", 255),
                (7, "#777777", 255),
                (8, "#dddddd", 255),
            ],
        }
    }

    /// O estilo da situação, como estilo do QGIS (`style.importQgis`, renderizador "paletted").
    /// O 0 (sem perturbação) sai pela máscara (`raster.mask`), porque o RustGIS 0.5.0 não esconde
    /// um valor de alfa 0; ele também não lê os rótulos (`label`): a legenda mostra os códigos,
    /// que o aviso explica.
    pub fn qml(self) -> String {
        let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
        q.push_str(r##"<paletteEntry value="0" color="#121212" alpha="0" label="Sem perturbação"/>"##);
        for (v, cor, alfa) in self.cores() {
            q.push_str(&format!(r#"<paletteEntry value="{v}" color="{cor}" alpha="{alfa}" label="{}"/>"#, ROTULOS[usize::from(v) - 1]));
        }
        q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
        q
    }

    /// O que os códigos querem dizer, com as cores, para o aviso.
    pub fn codigos(self) -> &'static str {
        match self {
            Paleta::BrasilData => {
                "a cor diz a etapa e, mais escura, a confiança alta: amarelo, primeira detecção (1 baixa, 4 alta); laranja, provisória (2, 5); vermelho, confirmada (3, 6); roxo, encerrada, a vegetação voltou ao normal (7, 8)"
            }
            Paleta::Oficial => {
                "paleta oficial da OPERA. 1 a 3: baixa confiança (primeira detecção, translúcida; provisória; confirmada); 4 a 6: alta confiança (idem); 7 e 8: encerrada (a vegetação voltou ao normal)"
            }
        }
    }
}

/// O nome de cada código, de 1 a 8.
const ROTULOS: [&str; 8] = [
    "Primeira detecção, baixa confiança",
    "Provisória, baixa confiança",
    "Confirmada, baixa confiança",
    "Primeira detecção, alta confiança",
    "Provisória, alta confiança",
    "Confirmada, alta confiança",
    "Encerrada, baixa confiança",
    "Encerrada, alta confiança",
];

/// Um produto: um tile MGRS numa data.
#[derive(Clone, Debug, PartialEq)]
pub struct Produto {
    /// O nome (`OPERA_L3_DIST-ALERT-S1_T23KQS_<aquisição>_<processamento>_S1C_30_v1.0`).
    pub nome: String,
    /// O tile MGRS (`T23KQS`).
    pub tile: String,
    /// O dia da aquisição (UTC).
    pub data: Data,
    /// Os endereços das camadas que a resposta traz.
    urls: Vec<String>,
    /// O contorno (GeoJSON).
    pub contorno: Value,
}

impl Produto {
    /// O endereço da GeoTIFF de uma camada.
    pub fn url(&self, c: Camada) -> String {
        let fim = format!("_{}.tif", c.sufixo());
        self.urls.iter().find(|u| u.ends_with(&fim)).cloned().unwrap_or_else(|| format!("{ARQUIVOS}/{}/{}{fim}", self.nome, self.nome))
    }

    /// O tile e o dia, para o nome da camada dele: `T23KQS 20260228`.
    pub fn rotulo(&self) -> String {
        format!("{} {}", self.tile, self.data.iso().replace('-', ""))
    }
}

/// A busca dos produtos que cruzam o retângulo `c` ([oeste, sul, leste, norte]): até o fim de
/// um dia, ou os mais recentes.
pub fn url_busca(c: [f64; 4], ate: Option<Data>) -> String {
    let mut u = format!(
        "{BUSCA}?dataset=OPERA-S1&processingLevel=DIST-ALERT-S1&bbox={:.4},{:.4},{:.4},{:.4}&maxResults={MAX_BUSCA}&output=jsonlite2",
        c[0], c[1], c[2], c[3]
    );
    if let Some(d) = ate {
        u.push_str(&format!("&end={}T00:00:00Z", d.seguinte().iso()));
    }
    u
}

/// O mais recente de cada tile, da resposta da busca (`output=jsonlite2`: `gn` o nome, `st` o
/// início, `w` o contorno em WKT), do mais recente para o mais antigo. Uma aquisição
/// reprocessada vem duas vezes, com o mesmo tile e a mesma hora: fica a de processamento mais
/// novo, que o nome traz logo depois da hora da aquisição.
pub fn mais_recentes(resposta: &Value) -> Vec<Produto> {
    // Por tile: (aquisição, processamento, produto).
    let mut por_tile: HashMap<String, (String, String, Produto)> = HashMap::new();
    for r in resposta["results"].as_array().into_iter().flatten() {
        let Some(nome) = r["gn"].as_str() else {
            continue;
        };
        let partes: Vec<&str> = nome.split('_').collect();
        let (Some(tile), Some(aquisicao), Some(processamento)) = (partes.get(3), partes.get(4), partes.get(5)) else {
            continue;
        };
        let (Some(data), Some(contorno)) = (r["st"].as_str().and_then(data_de), r["w"].as_str().and_then(poligono)) else {
            continue;
        };
        let novo = match por_tile.get(*tile) {
            Some((a, p, _)) => (*aquisicao, *processamento) > (a.as_str(), p.as_str()),
            None => true,
        };
        if novo {
            let urls = r["s1o"]["additionalUrls"].as_array().into_iter().flatten().filter_map(Value::as_str).map(str::to_string).collect();
            let p = Produto { nome: nome.to_string(), tile: tile.to_string(), data, urls, contorno };
            por_tile.insert(tile.to_string(), (aquisicao.to_string(), processamento.to_string(), p));
        }
    }
    let mut lista: Vec<Produto> = por_tile.into_values().map(|(_, _, p)| p).collect();
    // O mais recente primeiro: onde dois tiles se sobrepõem, o mosaico fica com o primeiro.
    lista.sort_by(|a, b| b.data.cmp(&a.data).then_with(|| a.tile.cmp(&b.tile)));
    lista
}

/// A camada dos contornos, para a seleção pela área: o índice de cada produto em `i`.
pub fn camada_dos_contornos(lista: &[Produto]) -> Value {
    let feicoes: Vec<Value> = lista
        .iter()
        .enumerate()
        .map(|(i, p)| {
            json!({"type": "Feature", "geometry": p.contorno, "properties": {
                "i": i, "tile": p.tile, "data": p.data.texto(), "produto": p.nome,
            }})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// O número do dia `d` no `GEN-DIST-DATE` (dias desde 31/12/2020).
pub fn dia_do_dist(d: Data) -> i64 {
    d.dias() - DIA_ZERO.dias()
}

/// Se uma camada do mapa é do DIST-S1 (para a citação).
pub fn eh_do_dist(nome: &str) -> bool {
    nome.starts_with(crate::PREFIXO) && nome.contains("(OPERA DIST-S1")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn produto(nome: &str, t: &str) -> Value {
        json!({"gn": nome, "st": t,
            "w": "POLYGON ((-43.12617 -20.73067, -43.12617 -21.73694, -42.05281 -21.73694, -42.05281 -20.73067, -43.12617 -20.73067))",
            "s1o": {"additionalUrls": [format!("https://x/{nome}/{nome}_GEN-DIST-STATUS.tif"), format!("https://x/{nome}/{nome}.iso.xml")]}})
    }

    #[test]
    fn a_busca_pede_o_retangulo_e_o_fim() {
        let u = url_busca([-43.003, -20.858, -42.754, -20.655], Some(Data { ano: 2026, mes: 2, dia: 28 }));
        assert!(u.contains("dataset=OPERA-S1&processingLevel=DIST-ALERT-S1&bbox=-43.0030,-20.8580,-42.7540,-20.6550"));
        assert!(u.ends_with("&end=2026-03-01T00:00:00Z"), "{u}");
        assert!(!url_busca([0.0; 4], None).contains("end="));
    }

    #[test]
    fn fica_o_mais_recente_de_cada_tile() {
        let l = mais_recentes(&json!({"results": [
            produto("OPERA_L3_DIST-ALERT-S1_T23KQS_20260221T082108Z_20260715T095459Z_S1C_30_v1.0", "2026-02-21T08:21:08Z"),
            produto("OPERA_L3_DIST-ALERT-S1_T23KQS_20260228T081304Z_20260716T035231Z_S1C_30_v1.0", "2026-02-28T08:13:04Z"),
            produto("OPERA_L3_DIST-ALERT-S1_T23KQS_20260228T081304Z_20260720T000000Z_S1C_30_v1.0", "2026-02-28T08:13:04Z"),
            produto("OPERA_L3_DIST-ALERT-S1_T23KQT_20260227T082207Z_20260715T200650Z_S1A_30_v1.0", "2026-02-27T08:22:07Z"),
            json!({"gn": "sem partes", "st": "2026-02-28T00:00:00Z", "w": "POLYGON ((0 0, 1 0, 1 1, 0 0))"}),
        ]}));
        assert_eq!(l.len(), 2, "um por tile");
        assert_eq!(l[0].tile, "T23KQS");
        assert!(l[0].nome.contains("20260720T000000Z"), "a aquisição reprocessada mais nova");
        assert_eq!(l[0].rotulo(), "T23KQS 20260228");
        assert_eq!(l[1].data, Data { ano: 2026, mes: 2, dia: 27 }, "o mais recente primeiro");
        assert!(l[0].url(Camada::Situacao).starts_with("https://x/"), "o endereço da resposta");
        let n = &l[0].nome;
        assert_eq!(l[0].url(Camada::Confianca), format!("{ARQUIVOS}/{n}/{n}_GEN-DIST-CONF.tif"), "senão, o padrão do ASF");
        let c = camada_dos_contornos(&l);
        assert_eq!(c["features"][1]["properties"]["i"], 1);
        assert_eq!(c["features"][0]["properties"]["tile"], "T23KQS");
    }

    #[test]
    fn os_nomes_os_dias_e_a_citacao() {
        let d = Data { ano: 2026, mes: 2, dia: 28 };
        assert_eq!(dia_do_dist(d), 1885);
        assert_eq!(dia_do_dist(Data { ano: 2026, mes: 1, dia: 4 }), 1830);
        assert_eq!(Camada::Situacao.nome(d), "BrasilData: Perturbação da vegetação (OPERA DIST-S1, até 28/02/2026)");
        for c in [Camada::Situacao, Camada::Data, Camada::Confianca, Camada::Intensidade] {
            assert!(eh_do_dist(&c.nome(d)), "{c:?}");
        }
        assert!(!eh_do_dist("BrasilData: Radar VV em dB (Sentinel-1, 26/10/2026)"));
        for p in [Paleta::BrasilData, Paleta::Oficial] {
            let q = p.qml();
            assert_eq!(q.matches("<paletteEntry").count(), 9, "{p:?}");
            assert!(q.contains(r##"value="0" color="#121212" alpha="0""##));
        }
        assert!(Paleta::BrasilData.qml().contains(r##"value="6" color="#b3121d" alpha="255" label="Confirmada, alta confiança""##));
        assert!(Paleta::Oficial.qml().contains(r##"value="1" color="#005555" alpha="110""##));
        assert_eq!(Paleta::ler(None), Ok(Paleta::BrasilData));
        assert_eq!(Paleta::ler(Some("oficial")), Ok(Paleta::Oficial));
        assert!(Paleta::ler(Some("arco-íris")).is_err());
    }
}
