//! O CEMADEN (Centro Nacional de Monitoramento e Alertas de Desastres Naturais, MCTI): a chuva
//! medida nas estações da rede observacional e os alertas de risco geo-hidrológico vigentes.
//!
//! - **Chuva:** os acumulados de 1 a 96 horas de cada estação, por UF, do serviço que alimenta o
//!   mapa interativo (`resources.cemaden.gov.br/graficos/interativo/getJson2.php?uf=`), sem login,
//!   atualizado a cada 10 minutos. Ele vem sem coordenadas: elas vêm do `cemaden.csv`, embutido
//!   no plugin (`scripts/baixar_cemaden.py`), pelo `idestacao`. O arquivo do mapa que já tem as
//!   coordenadas (`dados/311_24.json`, 2,2 MB) passa do limite de 2 MB do `host.fetch`, e o
//!   GeoServer do CEMADEN tem o WFS desligado.
//! - **Alertas:** os vigentes, do painel de alertas (`painelalertas.cemaden.gov.br/wsAlertas2`):
//!   um por município, com o evento (risco hidrológico ou movimentos de massa) e o nível. O
//!   contorno é o do município, da malha embutida no plugin ([`crate::malhas`]).
//! - **Só no desktop:** nenhum servidor do CEMADEN manda CORS para outras origens.
//! - **Nomes:** o serviço da chuva perde os acentos de alguns municípios; o nome vem da lista do
//!   IBGE embutida ([`crate::municipios`]) pelo código, e o da estação, do `cemaden.csv`.

use std::collections::HashMap;

use serde_json::{Value, json};

/// A camada da chuva nas estações.
pub const CHUVA: &str = "BrasilData: Chuva nas estações (CEMADEN)";

/// A camada dos alertas vigentes.
pub const ALERTAS: &str = "BrasilData: Alertas vigentes (CEMADEN)";

/// O endereço dos alertas vigentes.
pub const URL_ALERTAS: &str = "https://painelalertas.cemaden.gov.br/wsAlertas2";

/// A citação, como a rede observacional pede, com o serviço.
pub const CITACAO: &str = "CEMADEN/MCTI, rede observacional de estações (data from the CEMADEN/MCTI observational network), pelo mapa interativo (mapainterativo.cemaden.gov.br)";

/// A citação dos alertas.
pub const CITACAO_ALERTAS: &str = "CEMADEN/MCTI, alertas de risco geo-hidrológico vigentes, pelo painel de alertas (painelalertas.cemaden.gov.br); contornos dos municípios do IBGE";

/// A licença: o CEMADEN não declara uma para os dados; o conteúdo do portal é CC BY-ND 3.0.
pub const LICENCA: &str = "licença não declarada para os dados; citar a fonte (o conteúdo do portal do CEMADEN é CC BY-ND 3.0)";

/// O endereço da chuva nas estações de uma UF.
pub fn url_chuva(uf: &str) -> String {
    format!("https://resources.cemaden.gov.br/graficos/interativo/getJson2.php?uf={uf}")
}

/// Uma estação do `cemaden.csv`.
#[derive(Debug)]
pub struct Estacao {
    pub id: u32,
    pub codigo: &'static str,
    pub nome: &'static str,
    pub tipo: &'static str,
    pub uf: &'static str,
    pub cod_ibge: &'static str,
    /// O nome do município, do IBGE.
    pub municipio: &'static str,
    pub lon: f64,
    pub lat: f64,
    /// As coordenadas como estão no CSV, para o GeoJSON sem formatar números.
    pub lon_txt: &'static str,
    pub lat_txt: &'static str,
}

const CSV: &str = include_str!("../cemaden.csv");

/// Um registro de 16 bytes por estação, na ordem do `id`: o id, a longitude e a latitude em
/// milionésimos de grau e o começo da linha dela no [`CSV`] (`scripts/baixar_cemaden.py`).
const BIN: &[u8] = include_bytes!("../cemaden.bin");

/// Quantas estações há nas tabelas.
pub fn quantas() -> usize {
    BIN.len() / 16
}

/// O registro `i` do [`BIN`]: `(id, lon, lat, início da linha)`.
fn registro(i: usize) -> (u32, f64, f64, usize) {
    let r = &BIN[16 * i..16 * i + 16];
    let n = |k: usize| [r[k], r[k + 1], r[k + 2], r[k + 3]];
    (
        u32::from_le_bytes(n(0)),
        f64::from(i32::from_le_bytes(n(4))) / 1e6,
        f64::from(i32::from_le_bytes(n(8))) / 1e6,
        u32::from_le_bytes(n(12)) as usize,
    )
}

/// A estação do registro `i`, com os textos da linha dela no [`CSV`].
fn estacao(i: usize) -> Option<Estacao> {
    let (id, lon, lat, inicio) = registro(i);
    let linha = CSV.get(inicio..)?.lines().next()?;
    let mut c = linha.split(',');
    if c.next()?.parse::<u32>().ok()? != id {
        return None;
    }
    Some(Estacao {
        id,
        codigo: c.next()?,
        nome: c.next()?,
        tipo: c.next()?,
        uf: c.next()?,
        cod_ibge: c.next()?,
        municipio: c.next()?,
        lon_txt: c.next()?,
        lat_txt: c.next()?,
        lon,
        lat,
    })
}

/// Todas as estações, na ordem do `id` (para os testes: lê a tabela inteira).
pub fn estacoes() -> impl Iterator<Item = Estacao> {
    (0..quantas()).filter_map(estacao)
}

/// O registro de um `idestacao`, por busca binária.
fn indice(id: u32) -> Option<usize> {
    let (mut baixo, mut alto) = (0, quantas());
    while baixo < alto {
        let meio = (baixo + alto) / 2;
        let (m, ..) = registro(meio);
        match m.cmp(&id) {
            std::cmp::Ordering::Less => baixo = meio + 1,
            std::cmp::Ordering::Greater => alto = meio,
            std::cmp::Ordering::Equal => return Some(meio),
        }
    }
    None
}

/// A estação de um `idestacao`.
#[cfg(test)]
fn por_id(id: u32) -> Option<Estacao> {
    indice(id).and_then(estacao)
}

/// Quantas estações caem no retângulo `c` (sem ler o CSV).
pub fn quantas_no_retangulo(c: [f64; 4]) -> usize {
    (0..quantas()).filter(|&i| no_retangulo(registro(i).1, registro(i).2, c)).count()
}

/// Se o ponto cai no retângulo `c` (oeste, sul, leste, norte).
fn no_retangulo(lon: f64, lat: f64, c: [f64; 4]) -> bool {
    lon >= c[0] && lon <= c[2] && lat >= c[1] && lat <= c[3]
}

/// As UFs das estações no retângulo `c`, em ordem: as que pedir ao serviço da chuva.
pub fn ufs_no_retangulo(c: [f64; 4]) -> Vec<&'static str> {
    let mut ufs: Vec<&str> =
        (0..quantas()).filter(|&i| no_retangulo(registro(i).1, registro(i).2, c)).filter_map(|i| Some(estacao(i)?.uf)).collect();
    ufs.sort_unstable();
    ufs.dedup();
    ufs
}

/// O campo do estilo: a faixa da chuva em 24 horas.
pub const CAMPO_FAIXA: &str = "faixa_24h";

/// As faixas da chuva em 24 horas, com a cor, na ordem da legenda: o limite de cima de cada uma
/// (mm; a primeira, 0 mm, vai até a menor medida do pluviômetro, 0,2 mm; as outras incluem o
/// limite; a última não tem). Os nomes ficam em ordem alfabética na ordem da chuva, porque o
/// RustGIS põe as classes da legenda em ordem alfabética.
pub const FAIXAS: [(&str, f64, &str); 6] = [
    ("0 mm", 0.2, "#ffffff"),
    ("0,2 a 10 mm", 10.0, "#b3d9f2"),
    ("10 a 25 mm", 25.0, "#5aa9e0"),
    ("25 a 50 mm", 50.0, "#1f6fbf"),
    ("50 a 100 mm", 100.0, "#7b3fbf"),
    ("> 100 mm", f64::INFINITY, "#d4148c"),
];

/// A faixa de quem não teve leitura nas últimas 24 horas: depois das outras na ordem alfabética.
pub const SEM_LEITURA: &str = "sem leitura em 24 h";

/// A faixa de uma chuva em 24 horas.
pub fn faixa(mm: Option<f64>) -> &'static str {
    match mm {
        None => SEM_LEITURA,
        Some(v) if v < FAIXAS[0].1 => FAIXAS[0].0,
        Some(v) => FAIXAS[1..].iter().find(|f| v <= f.1).map_or(FAIXAS[5].0, |f| f.0),
    }
}

/// A data do serviço da chuva, `dd/mm/aa hh:mm` em UTC, como `aaaa-mm-dd hh:mm`, que se ordena.
pub fn data_iso(s: &str) -> Option<String> {
    let (d, h) = s.trim().split_once(' ')?;
    let mut p = d.split('/');
    let (dia, mes, ano) = (p.next()?, p.next()?, p.next()?);
    if dia.len() != 2 || mes.len() != 2 || ano.len() != 2 || h.len() != 5 {
        return None;
    }
    Some(format!("20{ano}-{mes}-{dia} {h}"))
}

/// A chuva nas estações do retângulo, das respostas do serviço (uma por UF).
#[derive(Debug, Default)]
pub struct Chuva {
    /// Os pontos, cada um uma `Feature` do GeoJSON, já em texto.
    pub feicoes: Vec<String>,
    /// As estações do serviço que não estão no `cemaden.csv` (instaladas depois dele).
    pub sem_coordenadas: usize,
    /// As estações no retângulo com leitura nas últimas 24 horas.
    pub com_leitura: usize,
    /// A leitura mais recente, `aaaa-mm-dd hh:mm` em UTC.
    pub ultima: Option<String>,
}

/// O que procurar, nos bytes de uma resposta, para achar cada estação: a chave do `idestacao`
/// com as aspas de um JSON. No `text` da resposta do `host.fetch`, o mesmo documento vem como
/// texto, com as aspas escapadas (`\"idestacao\":`), e não casa.
const CHAVE_DO_ID: &[u8] = b"\"idestacao\":";

/// As estações no retângulo `c`, lidas dos bytes de respostas do `host.fetch` a [`url_chuva`]
/// (`{"ok": {"json": [...], "text": "...", "url": ...}}`), sem ler nada como JSON.
///
/// A cota de instruções de um comando de plugin não dá para ler como JSON três estados (o SDK
/// leria a resposta inteira, com o texto e o JSON, e a copiaria), nem para montar centenas de
/// pontos como `Value`: na Zona da Mata (688 estações no retângulo), isso passava de 5×10⁸
/// instruções. Cada estação é um objeto sem objetos dentro: o plugin acha o `idestacao`,
/// procura a estação nas tabelas embutidas, e só nas que caem no retângulo copia os valores do
/// texto do objeto para o texto do ponto.
pub fn chuva_dos_bytes(respostas: &[&[u8]], c: [f64; 4]) -> Chuva {
    let mut out = Chuva::default();
    for b in respostas {
        let mut i = 0;
        while let Some(k) = achar(&b[i..], CHAVE_DO_ID) {
            let inicio_num = i + k + CHAVE_DO_ID.len();
            i = inicio_num;
            let fim_num = b[inicio_num..].iter().position(|c| !c.is_ascii_digit()).map_or(b.len(), |n| inicio_num + n);
            let Some(k) = std::str::from_utf8(&b[inicio_num..fim_num]).ok().and_then(|s| s.parse().ok()).and_then(indice) else {
                out.sem_coordenadas += 1;
                continue;
            };
            let (_, lon, lat, _) = registro(k);
            if !no_retangulo(lon, lat, c) {
                continue;
            }
            let Some(e) = estacao(k) else { continue };
            // O objeto em volta: da `{` antes da chave à `}` depois dela.
            let (Some(abre), Some(fecha)) =
                (b[..inicio_num].iter().rposition(|&c| c == b'{'), b[inicio_num..].iter().position(|&c| c == b'}').map(|n| inicio_num + n))
            else {
                continue;
            };
            ponto(&b[abre..=fecha], &e, &mut out);
        }
    }
    out
}

/// Onde `agulha` aparece em `palheiro`.
fn achar(palheiro: &[u8], agulha: &[u8]) -> Option<usize> {
    let primeiro = *agulha.first()?;
    let mut i = 0;
    while let Some(k) = palheiro[i..].iter().position(|&c| c == primeiro) {
        let p = i + k;
        if palheiro[p..].starts_with(agulha) {
            return Some(p);
        }
        i = p + 1;
    }
    None
}

/// O valor de `chave` (com as aspas e os dois pontos: `"acc24hr":`) num objeto JSON plano: o
/// texto de um número, ou o de uma string sem as aspas (o serviço não põe aspas nem vírgulas
/// dentro dos valores que o plugin lê).
fn campo<'a>(obj: &'a [u8], chave: &[u8]) -> Option<&'a str> {
    let resto = &obj[achar(obj, chave)? + chave.len()..];
    let valor = if let Some(s) = resto.strip_prefix(b"\"") {
        &s[..s.iter().position(|&c| c == b'"')?]
    } else {
        &resto[..resto.iter().position(|&c| c == b',' || c == b'}')?]
    };
    std::str::from_utf8(valor).ok().map(str::trim)
}

/// Se o texto é um número JSON (o serviço manda `"-"` quando não há leitura no intervalo).
fn eh_numero(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit() || matches!(c, b'-' | b'.' | b'e' | b'E' | b'+')) && s != "-"
}

/// Um texto como string JSON.
fn texto_json(s: &str) -> String {
    if s.bytes().any(|c| c == b'"' || c == b'\\' || c < 0x20) {
        serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into())
    } else {
        format!("\"{s}\"")
    }
}

/// Os acumulados do serviço como a chave no texto do objeto e o campo na camada.
const ACUMULADOS_JSON: [(&[u8], &str); 8] = [
    (b"\"acc1hr\":", "chuva_1h"),
    (b"\"acc3hr\":", "chuva_3h"),
    (b"\"acc6hr\":", "chuva_6h"),
    (b"\"acc12hr\":", "chuva_12h"),
    (b"\"acc24hr\":", "chuva_24h"),
    (b"\"acc48hr\":", "chuva_48h"),
    (b"\"acc72hr\":", "chuva_72h"),
    (b"\"acc96hr\":", "chuva_96h"),
];

/// O ponto da estação `e`, do objeto `obj` do serviço, em `out`.
fn ponto(obj: &[u8], e: &Estacao, out: &mut Chuva) {
    use std::fmt::Write;
    let mm24 = campo(obj, b"\"acc24hr\":").filter(|s| eh_numero(s));
    if mm24.is_some() {
        out.com_leitura += 1;
    }
    let leitura = campo(obj, b"\"datahoraUltimovalor\":").and_then(data_iso);
    if let Some(l) = &leitura
        && out.ultima.as_ref().is_none_or(|u| l > u)
    {
        out.ultima = Some(l.clone());
    }
    let mut f = String::with_capacity(600);
    let _ = write!(
        f,
        r#"{{"type":"Feature","geometry":{{"type":"Point","coordinates":[{},{}]}},"properties":{{"codigo":{},"nome":{},"tipo":{},"municipio":{},"uf":{},"cod_ibge":{},"leitura_utc":{}"#,
        e.lon_txt,
        e.lat_txt,
        texto_json(e.codigo),
        texto_json(e.nome),
        texto_json(e.tipo),
        texto_json(e.municipio),
        texto_json(e.uf),
        texto_json(e.cod_ibge),
        leitura.as_deref().map_or("null".into(), texto_json),
    );
    let dez = campo(obj, b"\"ultimovalor\":").filter(|s| mm24.is_some() && eh_numero(s));
    let _ = write!(f, r#","chuva_10min":{}"#, dez.unwrap_or("null"));
    for (chave, nome) in ACUMULADOS_JSON {
        let v = campo(obj, chave).filter(|s| eh_numero(s));
        let _ = write!(f, r#","{nome}":{}"#, v.unwrap_or("null"));
    }
    let _ = write!(f, r#","{CAMPO_FAIXA}":{}}}}}"#, texto_json(faixa(mm24.and_then(|s| s.parse().ok()))));
    out.feicoes.push(f);
}

/// Um alerta vigente.
#[derive(Debug, PartialEq)]
pub struct Alerta {
    pub cod_ibge: String,
    pub uf: String,
    pub municipio: String,
    pub evento: String,
    pub nivel: String,
    /// Quando foi emitido e quando mudou por último, `aaaa-mm-dd hh:mm`, horário de Brasília.
    pub emitido: String,
    pub atualizado: String,
}

/// `aaaa-mm-dd hh:mm:ss.sss` sem os segundos.
fn ate_os_minutos(s: &str) -> String {
    s.get(..16).unwrap_or(s).to_string()
}

/// Os alertas de uma resposta de [`URL_ALERTAS`]. O `evento` vem como "Movimentos de Massa -
/// Moderado": fica o tipo, e o nível vai à parte.
pub fn alertas(resposta: &Value) -> Vec<Alerta> {
    resposta["alertas"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| a["status"].as_i64().is_none_or(|s| s == 1))
        .filter_map(|a| {
            let cod = match &a["codibge"] {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.clone(),
                _ => return None,
            };
            let evento = a["evento"].as_str().unwrap_or("");
            let tipo = evento.split_once(" - ").map_or(evento, |(t, _)| t);
            let mut tipo: String = tipo.trim().to_lowercase();
            if let Some(p) = tipo.get(..1) {
                tipo = p.to_uppercase() + &tipo[1..];
            }
            let municipio =
                crate::municipios::por_codigo(&cod).map_or(a["municipio"].as_str().unwrap_or("").to_string(), |m| m.nome.to_string());
            Some(Alerta {
                uf: a["uf"].as_str().unwrap_or("").to_string(),
                municipio,
                evento: tipo,
                nivel: a["nivel"].as_str().unwrap_or("").trim().to_string(),
                emitido: ate_os_minutos(a["datahoracriacao"].as_str().unwrap_or("")),
                atualizado: ate_os_minutos(a["ult_atualizacao"].as_str().unwrap_or("")),
                cod_ibge: cod,
            })
        })
        .collect()
}

/// O retângulo de uma geometria GeoJSON.
fn caixa(g: &Value) -> Option<[f64; 4]> {
    fn andar(v: &Value, c: &mut [f64; 4]) {
        match v.as_array() {
            Some(a) if a.len() >= 2 && a[0].is_number() => {
                let (x, y) = (a[0].as_f64().unwrap_or(0.0), a[1].as_f64().unwrap_or(0.0));
                *c = [c[0].min(x), c[1].min(y), c[2].max(x), c[3].max(y)];
            }
            Some(a) => a.iter().for_each(|v| andar(v, c)),
            None => {}
        }
    }
    let mut c = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    andar(&g["coordinates"], &mut c);
    (c[0] <= c[2]).then_some(c)
}

/// Os municípios com alerta cujo contorno cruza o retângulo `c`, em GeoJSON, com o contorno de
/// `geometria` (o da malha embutida). Um município com dois alertas (chuva e deslizamento) fica
/// com os dois polígonos, um por alerta.
pub fn alertas_no_retangulo(lista: &[Alerta], c: [f64; 4], geometria: impl Fn(&str) -> Option<Value>) -> Vec<Value> {
    lista
        .iter()
        .filter_map(|a| {
            let g = geometria(&a.cod_ibge)?;
            let b = caixa(&g)?;
            if b[0] > c[2] || b[2] < c[0] || b[1] > c[3] || b[3] < c[1] {
                return None;
            }
            Some(json!({"type": "Feature", "geometry": g, "properties": {
                "municipio": a.municipio, "uf": a.uf, "cod_ibge": a.cod_ibge,
                "evento": a.evento, "nivel": a.nivel,
                "emitido": a.emitido, "atualizado": a.atualizado,
            }}))
        })
        .collect()
}

/// A área de uma geometria GeoJSON (`Polygon` ou `MultiPolygon`) em graus², sem os buracos: só
/// para comparar pedaços do mesmo lugar, onde a deformação das coordenadas geográficas se anula.
pub fn area_graus(g: &Value) -> f64 {
    let anel = |a: &Value| -> f64 {
        let p: Vec<(f64, f64)> = a.as_array().into_iter().flatten().filter_map(|c| Some((c[0].as_f64()?, c[1].as_f64()?))).collect();
        (p.windows(2).map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1).sum::<f64>() / 2.0).abs()
    };
    let poligono = |p: &Value| -> f64 {
        let aneis = p.as_array().map_or(&[][..], Vec::as_slice);
        aneis.first().map_or(0.0, anel) - aneis.iter().skip(1).map(anel).sum::<f64>()
    };
    match g["type"].as_str() {
        Some("Polygon") => poligono(&g["coordinates"]),
        Some("MultiPolygon") => g["coordinates"].as_array().into_iter().flatten().map(poligono).sum(),
        _ => 0.0,
    }
}

/// A menor fração do município dentro da área para o alerta dele entrar: o contorno embutido (a
/// malha intermediária) se afasta do oficial, e um vizinho da área deixa nela uma lasca da divisa,
/// que fica bem abaixo disto.
pub const FRACAO_MINIMA: f64 = 0.03;

/// Os códigos dos municípios com uma fração de pelo menos [`FRACAO_MINIMA`] dentro da área: os
/// `pedacos` são as feições recortadas pela área (com o `cod_ibge`), e as `inteiras`, as de antes
/// do recorte.
pub fn dentro_da_area(inteiras: &[Value], pedacos: &[Value]) -> Vec<String> {
    let mut dentro: HashMap<&str, f64> = HashMap::new();
    for p in pedacos {
        if let Some(cod) = p["properties"]["cod_ibge"].as_str() {
            *dentro.entry(cod).or_default() += area_graus(&p["geometry"]);
        }
    }
    let mut out: Vec<String> = inteiras
        .iter()
        .filter_map(|f| {
            let cod = f["properties"]["cod_ibge"].as_str()?;
            let total = area_graus(&f["geometry"]);
            (total > 0.0 && dentro.get(cod).is_some_and(|d| d / total >= FRACAO_MINIMA)).then(|| cod.to_string())
        })
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Os níveis dos alertas, com a cor do painel do CEMADEN, na ordem da legenda.
pub const NIVEIS: [(&str, &str); 3] = [("Moderado", "#f2c80f"), ("Alto", "#f07c00"), ("Muito alto", "#d7191c")];

/// O nome do estilo da chuva no projeto.
pub const ESTILO_CHUVA: &str = "BrasilData: chuva em 24 h (CEMADEN)";

/// O nome do estilo dos alertas no projeto.
pub const ESTILO_ALERTAS: &str = "BrasilData: nível do alerta (CEMADEN)";

fn rgba(h: &str, a: u8) -> Value {
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    json!([c(1), c(3), c(5), a])
}

/// O estilo da chuva, no formato do `style.add`: um círculo por estação, na cor da faixa das
/// 24 horas; sem leitura, um círculo vazio cinza.
pub fn estilo_chuva() -> String {
    let borda = json!([60, 60, 60, 255]);
    let mut classes: Vec<Value> = FAIXAS
        .iter()
        .map(
            |(nome, _, cor)| json!([nome, {"fill": rgba(cor, 255), "stroke": borda, "stroke_width": 0.6, "size": 7.0, "marker": "Circle"}]),
        )
        .collect();
    classes.push(
        json!([SEM_LEITURA, {"fill": [0, 0, 0, 0], "stroke": [140, 140, 140, 255], "stroke_width": 0.8, "size": 6.0, "marker": "Circle"}]),
    );
    let outro = json!({"fill": [0, 0, 0, 0], "stroke": [140, 140, 140, 255], "stroke_width": 0.8, "size": 6.0, "marker": "Circle"});
    json!([{"name": ESTILO_CHUVA, "geometry": "Point",
        "renderer": {"Unique": {"field": CAMPO_FAIXA, "classes": classes, "other": outro, "ramp": "Harbour"}},
        "labels": {"enabled": false, "field": "chuva_24h", "size": 7.5, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

/// O estilo dos alertas: o município na cor do nível, meio transparente, com a borda do nível.
pub fn estilo_alertas() -> String {
    let classes: Vec<Value> = NIVEIS
        .iter()
        .map(|(nome, cor)| json!([nome, {"fill": rgba(cor, 150), "stroke": rgba(cor, 255), "stroke_width": 1.2, "size": 0.0, "marker": "Circle"}]))
        .collect();
    let outro = json!({"fill": [128, 128, 128, 120], "stroke": [90, 90, 90, 255], "stroke_width": 1.0, "size": 0.0, "marker": "Circle"});
    json!([{"name": ESTILO_ALERTAS, "geometry": "Polygon",
        "renderer": {"Unique": {"field": "nivel", "classes": classes, "other": outro, "ramp": "Harbour"}},
        "labels": {"enabled": true, "field": "evento", "size": 8.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tabela_embutida_tem_as_estacoes() {
        let t: Vec<Estacao> = estacoes().collect();
        assert!(t.len() > 5000, "{}", t.len());
        assert_eq!(t.len(), CSV.lines().count() - 1, "uma linha do cemaden.csv não foi lida");
        assert_eq!(t.len(), quantas(), "o cemaden.bin e o cemaden.csv não batem");
        // As coordenadas do índice são as da linha.
        for e in &t {
            let linha = CSV.lines().find(|l| l.starts_with(&format!("{},", e.id))).unwrap();
            let lon: f64 = linha.split(',').nth(7).unwrap().parse().unwrap();
            assert!((lon - e.lon).abs() < 1e-6, "{linha}");
        }
        assert!(t.iter().all(|e| !e.municipio.is_empty()));
        assert_eq!(por_id(t[100].id).map(|e| e.codigo), Some(t[100].codigo));
        assert!(por_id(0).is_none());
        assert!(t.iter().all(|e| (-75.0..-28.0).contains(&e.lon) && (-35.0..6.0).contains(&e.lat)));
        assert!(t.iter().all(|e| e.cod_ibge.len() == 7 && e.uf.len() == 2));
        // O código da estação começa pelo do município, menos em umas poucas que mudaram de lugar.
        assert!(t.iter().filter(|e| !e.codigo.starts_with(e.cod_ibge)).count() < 10);
        assert!(t.windows(2).all(|w| w[0].id < w[1].id));
    }

    #[test]
    fn as_ufs_de_vicosa() {
        // A Zona da Mata mineira, perto da divisa com o Rio.
        assert_eq!(ufs_no_retangulo([-43.0, -21.0, -42.7, -20.6]), ["MG"]);
        assert!(ufs_no_retangulo([-30.0, 0.0, -29.0, 1.0]).is_empty());
    }

    #[test]
    fn as_datas_e_as_faixas() {
        assert_eq!(data_iso("16/11/26 15:30").as_deref(), Some("2026-11-16 15:30"));
        assert_eq!(data_iso(""), None);
        assert_eq!(faixa(None), SEM_LEITURA);
        assert_eq!(faixa(Some(0.0)), "0 mm");
        assert_eq!(faixa(Some(0.2)), "0,2 a 10 mm");
        assert_eq!(faixa(Some(25.0)), "10 a 25 mm");
        assert_eq!(faixa(Some(25.2)), "25 a 50 mm");
        assert_eq!(faixa(Some(100.0)), "50 a 100 mm");
        assert_eq!(faixa(Some(320.0)), "> 100 mm");
        // A ordem alfabética é a da legenda.
        let mut nomes: Vec<&str> = FAIXAS.iter().map(|f| f.0).chain([SEM_LEITURA]).collect();
        let ordem = nomes.clone();
        nomes.sort_unstable();
        assert_eq!(nomes, ordem);
    }

    #[test]
    fn a_chuva_junta_as_coordenadas_pelo_id() {
        let e = &estacoes().next().unwrap();
        let resposta = json!([
            {"idestacao": e.id, "uf": e.uf, "codibge": 0, "cidade": "X", "nomeestacao": "x", "ultimovalor": 0.2,
             "datahoraUltimovalor": "16/11/26 15:30", "acc1hr": "-", "acc3hr": 1.4, "acc6hr": 1.4, "acc12hr": 19.2,
             "acc24hr": 58.4, "acc48hr": 58.4, "acc72hr": 98, "acc96hr": 98.8, "tipoestacao": 1, "status": 0},
            {"idestacao": 999_999_999u32, "acc24hr": 1.0},
        ]);
        let c = [e.lon - 0.1, e.lat - 0.1, e.lon + 0.1, e.lat + 0.1];
        // Como o host.fetch responde: o JSON lido (chaves em ordem) e o texto, com as aspas
        // escapadas, que não pode contar duas vezes.
        let bruto = json!({"ok": {"json": resposta, "text": resposta.to_string(), "url": "x"}}).to_string();
        let r = chuva_dos_bytes(&[bruto.as_bytes()], c);
        assert_eq!((r.feicoes.len(), r.sem_coordenadas, r.com_leitura), (1, 1, 1));
        assert_eq!(r.ultima.as_deref(), Some("2026-11-16 15:30"));
        let f: Value = serde_json::from_str(&r.feicoes[0]).expect("o ponto é JSON");
        let p = &f["properties"];
        assert_eq!(p["chuva_1h"], Value::Null);
        assert_eq!(p["chuva_10min"], json!(0.2));
        assert_eq!(p["chuva_24h"], json!(58.4));
        assert_eq!(p["chuva_72h"], json!(98));
        assert_eq!(p[CAMPO_FAIXA], "50 a 100 mm");
        assert_eq!(p["codigo"], e.codigo);
        assert_eq!(p["municipio"], e.municipio);
        assert_eq!(p["leitura_utc"], "2026-11-16 15:30");
        assert_eq!(f["geometry"]["coordinates"], json!([e.lon, e.lat]));
        // Fora do retângulo, nada; a estação desconhecida conta em qualquer retângulo.
        let fora = chuva_dos_bytes(&[bruto.as_bytes()], [0.0, 0.0, 1.0, 1.0]);
        assert!(fora.feicoes.is_empty());
        assert_eq!(fora.sem_coordenadas, 1);
        // Sem leitura em 24 h: tudo nulo, a faixa dela.
        let parada = json!([{"idestacao": e.id, "ultimovalor": 0, "datahoraUltimovalor": "19/05/26 05:00", "acc1hr": "-", "acc24hr": "-", "acc96hr": "-"}]);
        let bruto = json!({"ok": {"json": parada}}).to_string();
        let r = chuva_dos_bytes(&[bruto.as_bytes()], c);
        let f: Value = serde_json::from_str(&r.feicoes[0]).unwrap();
        assert_eq!((f["properties"]["chuva_24h"].clone(), f["properties"]["chuva_10min"].clone()), (Value::Null, Value::Null));
        assert_eq!(f["properties"][CAMPO_FAIXA], SEM_LEITURA);
        assert_eq!(r.com_leitura, 0);
    }

    #[test]
    fn nomes_com_aspas_viram_json() {
        assert_eq!(texto_json("Morro do \"Alto\""), r#""Morro do \"Alto\"""#);
        assert_eq!(texto_json("Viçosa"), "\"Viçosa\"");
        assert!(eh_numero("12.4") && eh_numero("0") && !eh_numero("-") && !eh_numero(""));
    }

    #[test]
    fn os_alertas_separam_o_evento_do_nivel() {
        let r = json!({"alertas": [
            {"cod_alerta": 3439, "datahoracriacao": "2026-11-16 00:44:35.255", "ult_atualizacao": "2026-11-16 00:55:36.035",
             "codibge": 3113701, "evento": "Movimentos de Massa - Moderado", "nivel": "Moderado", "status": 1, "uf": "MG",
             "municipio": "CARLOS CHAGAS", "latitude": -17.67, "longitude": -40.84},
            {"cod_alerta": 1, "codibge": 4211751, "evento": "Risco Hidrológico - Alto", "nivel": "Alto", "status": 0, "uf": "SC"},
        ]});
        let a = alertas(&r);
        assert_eq!(a.len(), 1, "o alerta com status 0 não está vigente");
        assert_eq!(a[0].evento, "Movimentos de massa");
        assert_eq!(a[0].nivel, "Moderado");
        assert_eq!(a[0].municipio, "Carlos Chagas");
        assert_eq!(a[0].emitido, "2026-11-16 00:44");
        let quadrado =
            |_: &str| Some(json!({"type": "Polygon", "coordinates": [[[-41.0, -18.0], [-40.5, -18.0], [-40.5, -17.5], [-41.0, -18.0]]]}));
        assert_eq!(alertas_no_retangulo(&a, [-42.0, -19.0, -40.0, -17.0], quadrado).len(), 1);
        assert!(alertas_no_retangulo(&a, [-44.0, -22.0, -42.0, -20.0], quadrado).is_empty());
        // A malha embutida tem o município.
        assert!(crate::malhas::geometria(&a[0].cod_ibge).is_some());
    }

    #[test]
    fn a_lasca_da_divisa_nao_conta() {
        let quadrado = |x0: f64, y0: f64, x1: f64, y1: f64| json!({"type": "Polygon", "coordinates": [[[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]});
        let f = |cod: &str, g: Value| json!({"type": "Feature", "properties": {"cod_ibge": cod}, "geometry": g});
        assert!((area_graus(&quadrado(0.0, 0.0, 2.0, 1.0)) - 2.0).abs() < 1e-12);
        let inteiras = [f("1", quadrado(0.0, 0.0, 1.0, 1.0)), f("2", quadrado(1.0, 0.0, 2.0, 1.0))];
        // O 1 tem a metade na área; do 2, só uma lasca de 1%.
        let pedacos = [f("1", quadrado(0.5, 0.0, 1.0, 1.0)), f("2", quadrado(1.0, 0.0, 1.01, 1.0))];
        assert_eq!(dentro_da_area(&inteiras, &pedacos), ["1"]);
        let multi = json!({"type": "MultiPolygon", "coordinates": [[[[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 0.0]]], [[[2.0, 0.0], [3.0, 0.0], [3.0, 1.0], [2.0, 0.0]]]]});
        assert!((area_graus(&multi) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn os_estilos_sao_json() {
        let s: Value = serde_json::from_str(&estilo_chuva()).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), 7);
        assert_eq!(s[0]["renderer"]["Unique"]["field"], CAMPO_FAIXA);
        let s: Value = serde_json::from_str(&estilo_alertas()).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"][2][0], "Muito alto");
    }
}
