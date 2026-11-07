//! O radar do Sentinel-1 corrigido do relevo: o OPERA RTC-S1 (JPL), distribuído pelo ASF DAAC.
//!
//! - **De onde:** a API de busca do ASF (`api.daac.asf.alaska.edu`, aberta e com CORS) lista os
//!   recortes (*bursts*, cerca de 20 × 85 km) que cruzam o retângulo da área, os mais recentes
//!   primeiro. Cada recorte tem uma GeoTIFF por polarização (`_VV.tif`, `_VH.tif`), em COG, em UTM,
//!   com células de 30 m.
//! - **Login:** os arquivos ficam no Earthdata Cloud da NASA (`cumulus.asf.earthdatacloud.nasa.gov`)
//!   e pedem o token Earthdata Login do usuário. Quem guarda e manda o token é o RustGIS
//!   (`earthdata.login`, no painel NASA Earthaccess); o plugin só passa o endereço ao
//!   `layer.addUrl`.
//! - **Quais recortes:** os contornos vão para uma camada e o RustGIS seleciona os que cruzam o
//!   polígono da área (`selectByLocation`), não só o retângulo dela. Entre as passagens que restam,
//!   vale a mais recente que cobre a área quase tão bem quanto a melhor ([`passagem`]).
//! - **Os valores:** o gama0 do OPERA (potência linear) passa pelo filtro de Lee (5 × 5, o
//!   `geolibre:lee_filter`), que tira o granulado do radar, e chega em dB (10 · log10, pelo
//!   `geolibre:log10`: o `bandMath` só tem + - * /). A razão VH/VV, em dB, é a diferença VH − VV.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::datas::Data;

/// O endereço da busca do ASF.
const BUSCA: &str = "https://api.daac.asf.alaska.edu/services/search/param";

/// Quantos recortes a busca devolve: cerca de quatro passagens sobre uma mesorregião, em cerca de
/// 1,3 MB (o `host.fetch` aceita até 2 MB).
pub const MAX_BUSCA: usize = 200;

/// Mais recortes que isto numa passagem é área grande demais para ler de uma vez.
pub const MAX_RECORTES: usize = 60;

/// O lado maior do retângulo da área, em graus, que o radar aceita (cerca de 400 km).
pub const MAX_GRAUS: f64 = 3.6;

/// Uma passagem vale se cobre a área com pelo menos esta fração dos recortes da melhor.
const QUASE_TANTO: f64 = 0.8;

/// A citação do OPERA RTC-S1.
pub const CITACAO: &str = "OPERA Radiometric Terrain Corrected SAR Backscatter from Sentinel-1 validated product, version 1 (OPERA_L2_RTC-S1_V1), JPL OPERA Project, distribuído pelo ASF DAAC (doi:10.5067/SNWG/OPERA_L2_RTC-S1_V1). Contém dados Copernicus Sentinel-1 modificados (ESA)";

/// A licença dele.
pub const LICENCA: &str = "dados da NASA sem restrição de uso, com citação; dados Copernicus Sentinel de uso livre e gratuito";

/// Uma polarização.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pol {
    Vv,
    Vh,
}

/// A escala de cinza da razão VH/VV, em dB (VH − VV).
pub const ESCALA_DA_RAZAO: (f64, f64) = (-8.0, -3.0);

impl Pol {
    /// A escala de cinza, em dB, depois do filtro de Lee: um pouco além dos percentis 2 e 98 da
    /// Zona da Mata mineira em 26/10/2026 (VV de −8,8 a −5,0; VH de −14,7 a −11,3). Limites fixos,
    /// e não os do histograma: um punhado de valores extremos (encostas viradas para o satélite)
    /// põe quase tudo na primeira faixa do histograma do RustGIS, e a imagem sai toda branca ou
    /// toda preta. Fixos, eles também deixam datas e lugares diferentes comparáveis. Água fica
    /// preta, cidade fica branca.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Pol::Vv => (-10.0, -3.0),
            Pol::Vh => (-16.0, -9.0),
        }
    }

    /// "VV" ou "VH".
    pub fn sigla(self) -> &'static str {
        match self {
            Pol::Vv => "VV",
            Pol::Vh => "VH",
        }
    }
}

/// Um recorte de uma passagem.
#[derive(Clone, Debug, PartialEq)]
pub struct Recorte {
    /// O nome do produto (`OPERA_L2_RTC-S1_T155-331984-IW2_…`).
    pub nome: String,
    /// O dia da passagem (UTC).
    pub data: Data,
    /// A órbita relativa (o "track").
    pub orbita: u32,
    pub vv: String,
    pub vh: String,
    /// O contorno (GeoJSON).
    pub contorno: Value,
}

impl Recorte {
    /// O endereço da GeoTIFF de uma polarização.
    pub fn url(&self, p: Pol) -> &str {
        match p {
            Pol::Vv => &self.vv,
            Pol::Vh => &self.vh,
        }
    }

    /// O recorte e o dia, para o nome da camada dele: `T155-331984-IW2 20261026`.
    pub fn rotulo(&self) -> String {
        let partes: Vec<&str> = self.nome.split('_').collect();
        match (partes.get(3), partes.get(4).and_then(|t| t.get(..8))) {
            (Some(id), Some(dia)) => format!("{id} {dia}"),
            _ => self.nome.clone(),
        }
    }

    /// A passagem: o dia e a órbita.
    pub fn passagem(&self) -> (Data, u32) {
        (self.data, self.orbita)
    }
}

/// A busca dos recortes que cruzam o retângulo `c` ([oeste, sul, leste, norte]): de um dia, ou
/// os mais recentes.
pub fn url_busca(c: [f64; 4], dia: Option<Data>) -> String {
    let mut u = format!(
        "{BUSCA}?dataset=OPERA-S1&processingLevel=RTC&bbox={:.4},{:.4},{:.4},{:.4}&maxResults={MAX_BUSCA}&output=jsonlite2",
        c[0], c[1], c[2], c[3]
    );
    if let Some(d) = dia {
        u.push_str(&format!("&start={}T00:00:00Z&end={}T00:00:00Z", d.iso(), d.seguinte().iso()));
    }
    u
}

/// A data de um `startTime` ("2026-10-26T08:21:21Z").
fn data_de(t: &str) -> Option<Data> {
    let mut p = t.get(..10)?.split('-');
    let (ano, mes, dia) = (p.next()?.parse().ok()?, p.next()?.parse().ok()?, p.next()?.parse().ok()?);
    Some(Data { ano, mes, dia })
}

/// O polígono de um WKT `POLYGON ((x y, …))`, em GeoJSON (só o anel externo, que é o que um
/// recorte tem).
fn poligono(wkt: &str) -> Option<Value> {
    let dentro = wkt.trim().strip_prefix("POLYGON")?.trim().strip_prefix("((")?.split(')').next()?;
    let anel: Vec<Value> = dentro
        .split(',')
        .map(|par| {
            let mut n = par.split_whitespace().map(str::parse::<f64>);
            match (n.next(), n.next()) {
                (Some(Ok(x)), Some(Ok(y))) => Some(json!([x, y])),
                _ => None,
            }
        })
        .collect::<Option<_>>()?;
    (anel.len() >= 4).then(|| json!({"type": "Polygon", "coordinates": [anel]}))
}

/// Os recortes da resposta da busca (`output=jsonlite2`: `gn` o nome, `st` o início, `p` a
/// órbita, `po` as polarizações, `w` o contorno em WKT), com as duas polarizações; os que não
/// têm alguma delas ficam de fora (o OPERA tem recortes só VV ou só HH em algumas áreas).
pub fn recortes(resposta: &Value) -> Vec<Recorte> {
    let mut lista = Vec::new();
    // O mesmo recorte pode vir mais de uma vez, reprocessado: o nome muda só depois da hora da
    // aquisição (`OPERA_L2_RTC-S1_T155-331984-IW2_20251012T082120Z_<processamento>_…`). Fica o
    // primeiro, que a busca põe antes.
    let mut vistos: HashSet<&str> = HashSet::new();
    for r in resposta["results"].as_array().into_iter().flatten() {
        let Some(nome) = r["gn"].as_str() else {
            continue;
        };
        let tem = |pol: &str| r["po"].as_array().is_some_and(|l| l.iter().any(|p| p.as_str() == Some(pol)));
        if !tem("VV") || !tem("VH") || !vistos.insert(aquisicao(nome)) {
            continue;
        }
        let (Some(data), Some(contorno)) = (r["st"].as_str().and_then(data_de), r["w"].as_str().and_then(poligono)) else {
            continue;
        };
        // Os endereços que a resposta traz; senão, o padrão do ASF para o nome.
        let urls: Vec<&str> = r["s1o"]["additionalUrls"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let url = |pol: &str| {
            let fim = format!("_{pol}.tif");
            urls.iter().find(|u| u.ends_with(&fim)).map(|u| u.to_string()).unwrap_or_else(|| format!("{ARQUIVOS}/{nome}/{nome}{fim}"))
        };
        let orbita = r["p"].as_u64().unwrap_or(0) as u32;
        lista.push(Recorte { nome: nome.to_string(), data, orbita, vv: url("VV"), vh: url("VH"), contorno });
    }
    lista
}

/// O nome de um recorte até a hora da aquisição: o mesmo para as versões reprocessadas dele.
fn aquisicao(nome: &str) -> &str {
    let fim = nome.match_indices('_').nth(4).map_or(nome.len(), |(i, _)| i);
    &nome[..fim]
}

/// Onde o ASF guarda os arquivos do OPERA RTC-S1.
const ARQUIVOS: &str = "https://cumulus.asf.earthdatacloud.nasa.gov/OPERA/OPERA_L2_RTC-S1";

/// A camada dos contornos, para a seleção pela área: o índice de cada recorte em `i`.
pub fn camada_dos_contornos(lista: &[Recorte]) -> Value {
    let feicoes: Vec<Value> = lista
        .iter()
        .enumerate()
        .map(|(i, r)| {
            json!({"type": "Feature", "geometry": r.contorno, "properties": {
                "i": i, "recorte": r.nome, "data": r.data.texto(), "orbita": r.orbita,
            }})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// A passagem a usar, entre os recortes que cruzam a área: a mais recente com pelo menos 80% dos
/// recortes da que tem mais (uma passagem de borda, que só pega uma ponta da área, perde para
/// uma anterior que a cobre).
pub fn passagem(cruzam: &[&Recorte]) -> Option<(Data, u32)> {
    let mut contagem: Vec<((Data, u32), usize)> = Vec::new();
    for r in cruzam {
        match contagem.iter_mut().find(|(p, _)| *p == r.passagem()) {
            Some((_, n)) => *n += 1,
            None => contagem.push((r.passagem(), 1)),
        }
    }
    let melhor = contagem.iter().map(|(_, n)| *n).max()?;
    contagem.into_iter().filter(|(_, n)| *n as f64 >= QUASE_TANTO * melhor as f64).map(|(p, _)| p).max_by_key(|(d, _)| *d)
}

/// O nome da camada de uma polarização numa data.
pub fn nome_da_camada(p: Pol, d: Data) -> String {
    format!("{}Radar {} em dB (Sentinel-1, {})", crate::PREFIXO, p.sigla(), d.texto())
}

/// O nome da camada da razão VH/VV numa data.
pub fn nome_da_razao(d: Data) -> String {
    format!("{}Razão VH/VV em dB (Sentinel-1, {})", crate::PREFIXO, d.texto())
}

/// Se uma camada do mapa é do radar (para a citação).
pub fn eh_do_radar(nome: &str) -> bool {
    let n = nome.strip_prefix(crate::PREFIXO).unwrap_or("");
    (n.starts_with("Radar V") || n.starts_with("Razão VH/VV")) && n.contains("(Sentinel-1, ")
}

/// Se o retângulo `c` cabe no radar.
pub fn cabe(c: [f64; 4]) -> bool {
    c[2] - c[0] <= MAX_GRAUS && c[3] - c[1] <= MAX_GRAUS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feicao(nome: &str, t: &str, orbita: u32, pols: &[&str]) -> Value {
        json!({"gn": nome, "st": t, "p": orbita, "po": pols,
            "w": "POLYGON ((-42.7726 -20.84945, -43.6429 -20.6444, -43.68656 -20.80978, -42.81283 -21.01572, -42.7726 -20.84945))"})
    }

    #[test]
    fn a_busca_pede_o_retangulo_e_o_dia() {
        let u = url_busca([-43.003, -20.858, -42.754, -20.655], Some(Data { ano: 2026, mes: 10, dia: 31 }));
        assert!(u.contains("dataset=OPERA-S1&processingLevel=RTC&bbox=-43.0030,-20.8580,-42.7540,-20.6550"));
        assert!(u.ends_with("&start=2026-10-31T00:00:00Z&end=2026-11-01T00:00:00Z"));
        assert!(!url_busca([0.0; 4], None).contains("start="));
    }

    #[test]
    fn um_recorte_traz_as_duas_polarizacoes() {
        let r = recortes(&json!({"results": [
            feicao("A", "2026-10-26T08:21:21Z", 155, &["VV", "VH"]),
            feicao("B", "2026-10-26T08:21:20Z", 155, &["VV"]),
            feicao("A", "2026-10-26T08:21:21Z", 155, &["VV", "VH"]),
        ]}));
        assert_eq!(r.len(), 1, "B só tem VV, e A repetido fica uma vez");
        let reprocessado = recortes(&json!({"results": [
            feicao("OPERA_L2_RTC-S1_T155-331984-IW2_20251012T082120Z_20251018T021912Z_S1C_30_v1.0", "2025-10-12T08:21:20Z", 155, &["VV", "VH"]),
            feicao("OPERA_L2_RTC-S1_T155-331984-IW2_20251012T082120Z_20251012T111619Z_S1C_30_v1.0", "2025-10-12T08:21:20Z", 155, &["VV", "VH"]),
            feicao("OPERA_L2_RTC-S1_T155-331984-IW1_20251012T082120Z_20251012T111619Z_S1C_30_v1.0", "2025-10-12T08:21:20Z", 155, &["VV", "VH"]),
        ]}));
        assert_eq!(reprocessado.len(), 2, "a versão reprocessada do mesmo recorte conta uma vez");
        assert!(reprocessado[0].nome.contains("20251018T021912Z"), "fica a primeira da busca");
        assert_eq!(reprocessado[0].rotulo(), "T155-331984-IW2 20251012");
        assert_eq!(r[0].data, Data { ano: 2026, mes: 10, dia: 26 });
        assert_eq!(r[0].orbita, 155);
        assert_eq!(r[0].url(Pol::Vv), "https://cumulus.asf.earthdatacloud.nasa.gov/OPERA/OPERA_L2_RTC-S1/A/A_VV.tif");
        assert!(r[0].url(Pol::Vh).ends_with("/A/A_VH.tif"));
        assert_eq!(r[0].contorno["coordinates"][0][1], json!([-43.6429, -20.6444]));
        assert_eq!(r[0].contorno["coordinates"][0].as_array().map(Vec::len), Some(5));
        let c = camada_dos_contornos(&r);
        assert_eq!(c["features"][0]["properties"]["i"], 0);
        assert_eq!(c["features"][0]["properties"]["data"], "26/10/2026");
    }

    #[test]
    fn a_passagem_e_a_mais_recente_que_cobre_quase_tanto() {
        let d = |dia| Data { ano: 2026, mes: 10, dia };
        let r = |dia, orbita, n: usize| -> Vec<Recorte> {
            (0..n)
                .map(|i| Recorte {
                    nome: format!("{dia}-{orbita}-{i}"),
                    data: d(dia),
                    orbita,
                    vv: String::new(),
                    vh: String::new(),
                    contorno: Value::Null,
                })
                .collect()
        };
        // 31/10 só pega uma ponta (3); 26/10 cobre (10); 14/10 também (10).
        let todos: Vec<Recorte> = [r(31, 53, 3), r(26, 155, 10), r(14, 155, 10)].concat();
        let refs: Vec<&Recorte> = todos.iter().collect();
        assert_eq!(passagem(&refs), Some((d(26), 155)));
        // Com 9 de 10, a mais recente ainda vale.
        let todos: Vec<Recorte> = [r(31, 53, 9), r(26, 155, 10)].concat();
        let refs: Vec<&Recorte> = todos.iter().collect();
        assert_eq!(passagem(&refs), Some((d(31), 53)));
        assert_eq!(passagem(&[]), None);
    }

    #[test]
    fn os_nomes_e_a_citacao() {
        let d = Data { ano: 2026, mes: 10, dia: 26 };
        assert_eq!(nome_da_camada(Pol::Vv, d), "BrasilData: Radar VV em dB (Sentinel-1, 26/10/2026)");
        assert!(eh_do_radar(&nome_da_camada(Pol::Vh, d)));
        assert!(eh_do_radar(&nome_da_razao(d)));
        assert!(!eh_do_radar("BrasilData: Radar de chuva"));
        assert!(cabe([-44.235, -22.259, -41.411, -19.828]), "a Zona da Mata cabe");
        assert!(!cabe([-51.0, -23.0, -39.8, -14.2]), "Minas Gerais não");
    }
}
