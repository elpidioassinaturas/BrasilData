//! As cenas do EMIT (NASA/JPL, na Estação Espacial Internacional): o contorno, a data, a nuvem e os
//! endereços de cada cena da refletância hiperespectral da superfície (EMIT L2A, 285 bandas, 60 m).
//!
//! - **De onde:** a busca do CMR (`cmr.earthdata.nasa.gov`, aberta e com CORS), pelo retângulo da
//!   área. O KML dá o contorno (uns 360 bytes por cena) e o CSV, a nuvem e os endereços (uns 700); o
//!   JSON, que tem tudo, passa de 5 KB por cena, e o Brasil tem milhares.
//! - **Só a lista:** cada cena é um netCDF de quase 2 GB (com a incerteza, 3,5 GB), que pede o login
//!   Earthdata e o plugin não lê. O mapa diz onde há cena limpa, e o campo `arquivo` leva a ela.
//! - **As versões:** a V002 (2026) reprocessa a V001 com outra máscara de nuvem. Uma cena que tem as
//!   duas (o mesmo instante inicial) entra uma vez, com a nuvem da V002; a V001 só completa.
//! - **A nuvem** é a fração da cena inteira, não da área.

use std::collections::HashSet;

use serde_json::{Value, json};

/// O começo do nome de toda camada das cenas.
const INICIO: &str = "BrasilData: Cenas do EMIT";

/// O começo do nome de toda cena da refletância.
const NOME: &str = "EMIT_L2A_RFL_";

/// A camada de passagem: as cenas do retângulo, antes da seleção pela área.
pub const NO_RETANGULO: &str = "BrasilData: cenas do EMIT no retângulo";

/// Quantas cenas vêm em cada página (o máximo do CMR).
pub const POR_PAGINA: usize = 2000;

/// O máximo de páginas de cada lista: o Brasil inteiro tem 7 da V002 sem limite de nuvem (11/2026).
pub const MAX_PAGINAS: usize = 12;

/// Quantos bytes do CSV há, por alto, em mil cenas.
pub const BYTES_POR_MIL: usize = 700_000;

/// Mais bytes de CSV que isto (umas 15 mil cenas, somadas as duas versões) passam da cota de instruções
/// do comando.
pub const MAX_BYTES: usize = 15 * BYTES_POR_MIL;

/// O limite de nuvem, se o comando não diz, em %.
pub const NUVEM: u32 = 30;

/// Até quantas cenas o comando lê de volta, para dizer as mais limpas.
pub const PARA_LER: u64 = 150;

/// Quantas cenas mais limpas o aviso diz.
const MAIS_LIMPAS: usize = 5;

/// O primeiro ano com cenas.
pub const PRIMEIRO_ANO: u32 = 2022;

/// O campo da classe de nuvem.
pub const CAMPO: &str = "classe";

/// As classes de nuvem, da mais limpa para a mais nublada: o limite (a nuvem até ele, em %), o nome
/// (a ordem alfabética é a da legenda) e a cor.
pub const CLASSES: [(u32, &str, &str); 4] = [
    (10, "0 a 10% de nuvem", "#08519c"),
    (30, "11 a 30% de nuvem", "#6baed6"),
    (50, "31 a 50% de nuvem", "#c6dbef"),
    (100, "51 a 100% de nuvem", "#d9d9d9"),
];

/// O estilo do projeto com as classes.
pub const ESTILO: &str = "BrasilData: nuvem das cenas do EMIT";

/// A citação, com as duas versões.
pub const CITACAO: &str = "EMIT L2A Surface Reflectance and Uncertainty and Masks 60 m V002 (Brodrick et al., 2026, doi:10.5067/EMIT/EMITL2ARFL.002) e, nas cenas ainda sem ela, EMIT L2A Estimated Surface Reflectance and Uncertainty and Masks 60 m V001 (Green, 2022, doi:10.5067/EMIT/EMITL2ARFL.001), NASA LP DAAC; contornos e nuvem do catálogo CMR da NASA";

/// O nome da camada das cenas com até `nuvem`% de nuvem, de um ano ou de todos.
pub fn camada(nuvem: u32, ano: Option<u32>) -> String {
    let de = ano.map(|a| format!(" de {a}")).unwrap_or_default();
    let com = if nuvem < 100 { format!(" com até {nuvem}% de nuvem") } else { String::new() };
    format!("{INICIO}{de}{com} (NASA)")
}

/// Se a camada `nome` é das cenas do EMIT.
pub fn eh_do_emit(nome: &str) -> bool {
    nome.starts_with(INICIO) && nome.ends_with(" (NASA)")
}

/// O que o nome da camada diz das cenas (" de 2024 com até 30% de nuvem"), para os textos.
pub fn quais(nome: &str) -> &str {
    nome.trim_start_matches(INICIO).trim_end_matches(" (NASA)")
}

/// O limite de nuvem pedido, em %: um número de 0 a 100.
pub fn conferir_nuvem(v: f64) -> Result<u32, String> {
    if (0.0..=100.0).contains(&v) && v.fract() == 0.0 {
        Ok(v as u32)
    } else {
        Err("o limite de nuvem é um número inteiro de 0 a 100".into())
    }
}

/// O ano pedido.
pub fn conferir_ano(v: f64) -> Result<u32, String> {
    if v >= f64::from(PRIMEIRO_ANO) && v < 2100.0 && v.fract() == 0.0 {
        Ok(v as u32)
    } else {
        Err(format!("o ano das cenas do EMIT é um número de {PRIMEIRO_ANO} em diante (ou vazio, para todos)"))
    }
}

/// De quantos graus² em diante o retângulo da área vai em faixas ([`faixas`]): um estado médio cabe num
/// retângulo só.
pub const GRANDE: f64 = 100.0;

/// A altura das faixas, em graus.
const ALTURA: f64 = 2.0;

/// O retângulo `c` da área em faixas de [`ALTURA`] graus, cada uma só da largura que a área tem nela: o
/// retângulo do Brasil pega metade da América do Sul, e as cenas dos Andes passariam da cota de
/// instruções do comando. `fc` é a área simplificada, e `folga`, em graus, o erro máximo dela. Se as
/// faixas não chegam às quatro bordas de `c` (uma ilha que a simplificação perdeu), fica o retângulo.
pub fn faixas(fc: &Value, c: [f64; 4], folga: f64) -> Vec<[f64; 4]> {
    let n = ((c[3] - c[1]) / ALTURA).ceil().max(1.0) as usize;
    let mut larguras = vec![(f64::INFINITY, f64::NEG_INFINITY); n];
    let mut toca = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    let mut aneis: Vec<&Value> = Vec::new();
    for g in fc["features"].as_array().into_iter().flatten().map(|f| &f["geometry"]) {
        let partes = g["coordinates"].as_array().into_iter().flatten();
        match g["type"].as_str() {
            Some("Polygon") => aneis.extend(partes),
            Some("MultiPolygon") => aneis.extend(partes.flat_map(|p| p.as_array().into_iter().flatten())),
            _ => return vec![c],
        }
    }
    for anel in aneis {
        let p: Vec<(f64, f64)> = anel.as_array().into_iter().flatten().filter_map(|q| Some((q[0].as_f64()?, q[1].as_f64()?))).collect();
        for (x, y) in &p {
            toca = [toca[0].min(*x), toca[1].min(*y), toca[2].max(*x), toca[3].max(*y)];
        }
        for s in p.windows(2) {
            let ((xa, ya), (xb, yb)) = if s[0].1 <= s[1].1 { (s[0], s[1]) } else { (s[1], s[0]) };
            for (i, l) in larguras.iter_mut().enumerate() {
                // A faixa, com a folga, e o pedaço do lado dentro dela.
                let (y0, y1) = (c[1] + i as f64 * ALTURA - folga, c[1] + (i + 1) as f64 * ALTURA + folga);
                if yb < y0 || ya > y1 {
                    continue;
                }
                let em = |y: f64| if yb > ya { xa + (xb - xa) * ((y - ya) / (yb - ya)).clamp(0.0, 1.0) } else { xa };
                let (p, q) = if yb > ya { (em(y0), em(y1)) } else { (xa, xb) };
                *l = (l.0.min(p.min(q)), l.1.max(p.max(q)));
            }
        }
    }
    if toca[0] > c[0] + folga || toca[1] > c[1] + folga || toca[2] < c[2] - folga || toca[3] < c[3] - folga {
        return vec![c];
    }
    larguras
        .iter()
        .enumerate()
        .filter(|(_, l)| l.0 <= l.1)
        .map(|(i, l)| {
            let y = c[1] + i as f64 * ALTURA;
            [(l.0 - folga).max(c[0]), y, (l.1 + folga).min(c[2]), (y + ALTURA).min(c[3])]
        })
        .collect()
}

/// Uma página da lista das cenas da `versao` (2 ou 1) que cruzam um dos retângulos `caixas`, em
/// `formato` (`kml` ou `csv`), com até `nuvem`% (100: todas), de um ano ou de todos.
pub fn url(formato: &str, versao: u32, caixas: &[[f64; 4]], nuvem: u32, ano: Option<u32>, pagina: usize) -> String {
    let mut u = format!(
        "https://cmr.earthdata.nasa.gov/search/granules.{formato}?short_name=EMITL2ARFL&version={versao:03}&sort_key=start_date&page_size={POR_PAGINA}&page_num={pagina}"
    );
    let chave = if caixas.len() > 1 { "bounding_box%5B%5D" } else { "bounding_box" };
    for c in caixas {
        u.push_str(&format!("&{chave}={:.4},{:.4},{:.4},{:.4}", c[0], c[1], c[2], c[3]));
    }
    if caixas.len() > 1 {
        u.push_str("&options%5Bbounding_box%5D%5Bor%5D=true");
    }
    if nuvem < 100 {
        u.push_str(&format!("&cloud_cover=0,{nuvem}"));
    }
    if let Some(a) = ano {
        u.push_str(&format!("&temporal={a}-01-01T00:00:00Z,{a}-12-31T23:59:59Z"));
    }
    u
}

/// Uma cena: os textos são os da resposta crua.
#[derive(Debug, PartialEq)]
pub struct Cena<'a> {
    pub nome: &'a str,
    /// O instante inicial ([`instante`]).
    quando: u64,
    /// O contorno, como no KML: `lon,lat lon,lat …`.
    contorno: &'a str,
    pub nuvem: u32,
    versao: u32,
}

/// O instante inicial de uma cena (`20240318T165039`, aqui 20240318165039), do nome: o mesmo nas duas
/// versões.
fn instante(nome: &str) -> Option<u64> {
    let t = nome.as_bytes().get(17..32)?;
    let mut v = 0;
    for (i, b) in t.iter().enumerate() {
        match b {
            b'T' if i == 8 => {}
            b'0'..=b'9' if i != 8 => v = v * 10 + u64::from(b - b'0'),
            _ => return None,
        }
    }
    Some(v)
}

/// As linhas do CSV na resposta crua (com as quebras escapadas, `\n`). A procura é pela barra, que
/// quase não aparece no texto, e não por `"\\n"`: esta, byte a byte, custaria a cota do comando.
fn linhas(texto: &str) -> impl Iterator<Item = &str> {
    let mut resto = texto;
    std::iter::from_fn(move || {
        if resto.is_empty() {
            return None;
        }
        let mut i = 0;
        while let Some(k) = resto[i..].find('\\') {
            let b = i + k;
            if resto.as_bytes().get(b + 1) == Some(&b'n') {
                let l = &resto[..b];
                resto = &resto[b + 2..];
                return Some(l);
            }
            // Uma aspa escapada (`\"`), ou outra coisa de dois bytes.
            i = b + 2;
            if i >= resto.len() || !resto.is_char_boundary(i) {
                break;
            }
        }
        Some(std::mem::take(&mut resto))
    })
}

/// O que se usa de uma linha do CSV: o nome, que é o primeiro campo, e a nuvem (%), que é o
/// antepenúltimo. Lidos pelas pontas: no meio há uns 500 bytes de endereços, e separar todos os campos
/// passava da cota de instruções do comando no Brasil.
fn da_linha(linha: &str) -> Option<(&str, u32)> {
    let b = linha.as_bytes();
    let antes = |ate: usize| b[..ate].iter().rposition(|c| *c == b',');
    let nome = linha.get(..b.iter().position(|c| *c == b',')?)?;
    let dia = antes(antes(b.len())?)?;
    let pct = linha.get(antes(dia)? + 1..dia)?.parse::<f64>().ok()?;
    Some((nome, pct.clamp(0.0, 100.0).round() as u32))
}

/// Se a página pode ter vindo cheia ([`POR_PAGINA`] cenas), pelo tamanho: uma cena ocupa uns 360 bytes
/// no KML e 680 no CSV. Na dúvida, o comando pede a página seguinte, que vem vazia.
pub fn cheia(formato: &str, texto: &str) -> bool {
    texto.len() >= POR_PAGINA * if formato == "kml" { 250 } else { 450 }
}

/// O que há no KML entre o nome de uma cena e o contorno dela, e entre o contorno e o nome da seguinte:
/// pulados de uma vez quando o comprimento bate, porque a procura de marca em marca, nas 12 mil cenas do
/// Brasil, pesa na cota de instruções.
const ENTRE: [&str; 2] = [
    "</name><styleUrl>#geodetic_style</styleUrl><Polygon><tessellate>1</tessellate><outerBoundaryIs><LinearRing><coordinates>",
    "</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark><Placemark><name>",
];

/// O texto depois da próxima `marca` (`<name>` ou `<coordinates>`): onde ela está a `salto` bytes do
/// começo, sem procurar.
fn depois<'a>(texto: &'a str, marca: &str, salto: usize) -> Option<&'a str> {
    if let Some(t) = texto.get(salto..)
        && texto.get(..salto).is_some_and(|p| p.ends_with(marca))
    {
        return Some(t);
    }
    let mut resto = texto;
    loop {
        resto = &resto[resto.find('<')?..];
        if let Some(t) = resto.strip_prefix(marca) {
            return Some(t);
        }
        resto = &resto[1..];
    }
}

/// O instante inicial e o contorno de cada cena das páginas do KML, em ordem de instante.
fn placas<'a>(paginas: &[&'a str]) -> Vec<(u64, &'a str)> {
    let mut v = Vec::with_capacity(paginas.len() * POR_PAGINA);
    for pagina in paginas {
        let mut resto = *pagina;
        // Antes do primeiro nome há um cabeçalho, que não é do comprimento do salto: ele é procurado.
        while let Some(t) = depois(resto, "<name>", ENTRE[1].len()) {
            let Some(fim) = t.find('<') else { break };
            let Some(c) = depois(&t[fim..], "<coordinates>", ENTRE[0].len()) else { break };
            let Some(fim_c) = c.find('<') else { break };
            if let Some(quando) = instante(&t[..fim]) {
                v.push((quando, &c[..fim_c]));
            }
            resto = &c[fim_c..];
        }
    }
    // As páginas vêm em ordem do início: nelas, isto só confere.
    v.sort_by_key(|p| p.0);
    v
}

/// O lugar da cena que começa em `quando` entre as `placas`.
fn lugar(placas: &[(u64, &str)], quando: u64) -> Option<usize> {
    placas.binary_search_by_key(&quando, |p| p.0).ok()
}

/// Se a resposta crua é a lista que se pediu: o KML tem `</kml>`; o CSV, a linha dos nomes.
pub fn eh_lista(formato: &str, texto: &str) -> bool {
    match formato {
        "kml" => texto.rfind("</kml>").is_some(),
        _ => texto.contains("Granule UR,Producer Granule ID,"),
    }
}

/// As respostas cruas de uma versão: os KML (na V002, sem o limite de nuvem, para saber o que ela
/// tem) e os CSV (com o limite).
pub struct Listas<'a> {
    pub kml: Vec<&'a str>,
    pub csv: Vec<&'a str>,
}

/// As cenas das duas versões, das mais nubladas para as mais limpas (e, na mesma nuvem, das antigas
/// para as novas): no mapa, a mais limpa fica por cima. Uma cena da V001 que a V002 tem (o mesmo
/// instante) sai, mesmo que na V002 ela passe do limite de nuvem.
pub fn cenas<'a>(v2: &Listas<'a>, v1: &Listas<'a>) -> Vec<Cena<'a>> {
    let (p2, p1) = (placas(&v2.kml), placas(&v1.kml));
    let mut lista: Vec<Cena<'a>> = Vec::new();
    for (versao, l, contornos) in [(2, v2, &p2), (1, v1, &p1)] {
        let mut vistas = vec![false; contornos.len()];
        for t in &l.csv {
            for linha in linhas(t).filter(|l| l.starts_with(NOME)) {
                let Some((nome, nuvem)) = da_linha(linha) else { continue };
                let Some((quando, i)) = instante(nome).and_then(|q| Some((q, lugar(contornos, q)?))) else { continue };
                if std::mem::replace(&mut vistas[i], true) || versao == 1 && lugar(&p2, quando).is_some() {
                    continue;
                }
                lista.push(Cena { nome, quando, contorno: contornos[i].1, nuvem, versao });
            }
        }
    }
    lista.sort_unstable_by_key(|c| (100 - c.nuvem, c.quando));
    lista
}

/// O nome da classe de `nuvem`.
pub fn classe(nuvem: u32) -> &'static str {
    CLASSES.iter().find(|(ate, ..)| nuvem <= *ate).map_or(CLASSES[3].1, |c| c.1)
}

/// Escreve em `t` o contorno do KML (`lon,lat lon,lat …`) como o anel de um GeoJSON (sem os colchetes de
/// fora); `false` se ele não é um polígono de números (`t` volta ao que era).
fn anel(contorno: &str, t: &mut Vec<u8>) -> bool {
    let inicio = t.len();
    // As vírgulas no ponto, e os pontos.
    let (mut virgulas, mut pontos) = (0, 1);
    t.push(b'[');
    for &b in contorno.trim_ascii().as_bytes() {
        match b {
            b'0'..=b'9' | b'.' | b'-' if virgulas < 2 => t.push(b),
            // A altitude, se houver, sai.
            b'0'..=b'9' | b'.' | b'-' => {}
            b',' => {
                virgulas += 1;
                if virgulas == 1 {
                    t.push(b',');
                }
            }
            b' ' if virgulas > 0 => {
                t.extend_from_slice(b"],[");
                (virgulas, pontos) = (0, pontos + 1);
            }
            _ => {
                t.truncate(inicio);
                return false;
            }
        }
    }
    t.push(b']');
    if pontos < 4 || virgulas == 0 {
        t.truncate(inicio);
        return false;
    }
    true
}

/// As `lista` de cenas como o texto de um GeoJSON, e quantas são. Em texto, e não em `Value`, pela
/// cota de instruções do comando (o Brasil tem milhares de cenas). Os endereços saem do nome da cena:
/// o CSV os traz, mas em 89 das 22 mil cenas da V001 o primeiro é o da incerteza; a regra daqui bate
/// com as 34.920 cenas do retângulo do Brasil (23/11/2026).
pub fn geojson(lista: &[Cena]) -> (String, usize) {
    let mut t: Vec<u8> = Vec::with_capacity(lista.len() * 800 + 64);
    t.extend_from_slice(b"{\"type\":\"FeatureCollection\",\"features\":[");
    let mut n = 0;
    for c in lista {
        let antes = t.len();
        if n > 0 {
            t.push(b',');
        }
        t.extend_from_slice(b"{\"type\":\"Feature\",\"geometry\":{\"type\":\"Polygon\",\"coordinates\":[[");
        if !anel(c.contorno, &mut t) || c.nome.contains(['"', '\\']) {
            t.truncate(antes);
            continue;
        }
        n += 1;
        t.extend_from_slice(b"]]},\"properties\":{\"cena\":\"");
        t.extend_from_slice(c.nome.as_bytes());
        // As partes do instante, que o nome traz como texto (`instante` já o conferiu).
        let i = &c.nome.as_bytes()[17..32];
        for (antes, parte) in [
            (&b"\",\"data\":\""[..], &i[..4]),
            (b"-", &i[4..6]),
            (b"-", &i[6..8]),
            (b"\",\"hora_utc\":\"", &i[9..11]),
            (b":", &i[11..13]),
            (b"\",\"ano\":", &i[..4]),
            (b",\"nuvem_pct\":", c.nuvem.to_string().as_bytes()),
            (b",\"classe\":\"", classe(c.nuvem).as_bytes()),
        ] {
            t.extend_from_slice(antes);
            t.extend_from_slice(parte);
        }
        let v: &[u8] = if c.versao == 2 { b"2" } else { b"1" };
        for (antes, depois) in [
            (&b"\",\"versao\":\"V00"[..], &b""[..]),
            (b"\",\"previa\":\"https://data.lpdaac.earthdatacloud.nasa.gov/lp-prod-public/EMITL2ARFL.00", b".png"),
            (b"\",\"arquivo\":\"https://data.lpdaac.earthdatacloud.nasa.gov/lp-prod-protected/EMITL2ARFL.00", b".nc"),
        ] {
            t.extend_from_slice(antes);
            t.extend_from_slice(v);
            if !depois.is_empty() {
                t.push(b'/');
                t.extend_from_slice(c.nome.as_bytes());
                t.push(b'/');
                t.extend_from_slice(c.nome.as_bytes());
                t.extend_from_slice(depois);
            }
        }
        t.extend_from_slice(b"\"}}");
    }
    t.extend_from_slice(b"]}");
    (String::from_utf8(t).unwrap_or_default(), n)
}

/// As cenas mais limpas das feições `fc` (da camada), como "12/09/2024 (0%)", a mais recente na frente
/// das de mesma nuvem. Duas cenas do mesmo dia são vizinhas na mesma faixa: a data vem uma vez.
pub fn as_mais_limpas(fc: &Value) -> Vec<String> {
    let mut v: Vec<(i64, &str)> = fc["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| Some((f["properties"]["nuvem_pct"].as_f64()? as i64, f["properties"]["data"].as_str()?)))
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(a.1)));
    let mut dias = HashSet::new();
    v.retain(|(_, d)| dias.insert(*d));
    v.iter()
        .take(MAIS_LIMPAS)
        .map(|(n, d)| match (d.get(..4), d.get(5..7), d.get(8..10)) {
            (Some(a), Some(m), Some(dd)) => format!("{dd}/{m}/{a} ({n}%)"),
            _ => format!("{d} ({n}%)"),
        })
        .collect()
}

/// As classes que uma camada com até `nuvem`% de nuvem pode ter.
pub fn classes(nuvem: u32) -> impl Iterator<Item = &'static (u32, &'static str, &'static str)> {
    CLASSES.iter().take_while(move |c| classe(nuvem) >= c.1)
}

/// O estilo, no formato do `style.add`: a cena na cor da classe de nuvem, azul-escuro a mais limpa, só
/// com as classes que a camada pode ter. Cores opacas: com transparência, uma cena sobre outra mudava de
/// tom e parecia de outra classe.
pub fn estilo(nuvem: u32) -> String {
    let rgba = |h: &str| {
        let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
        json!([c(1), c(3), c(5), 255])
    };
    let simbolo =
        |cor: &str| json!({"fill": rgba(cor), "stroke": [255, 255, 255, 220], "stroke_width": 0.4, "size": 0.0, "marker": "Circle"});
    let lista: Vec<Value> = classes(nuvem).map(|(_, nome, cor)| json!([nome, simbolo(cor)])).collect();
    json!([{"name": ESTILO, "geometry": "Polygon",
        "renderer": {"Unique": {"field": CAMPO, "classes": lista, "other": simbolo("#d9d9d9"), "ramp": "Harbour"}},
        "labels": {"enabled": false, "field": "data", "size": 8.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAB: &str = "Granule UR,Producer Granule ID,Start Time,End Time,Online Access URLs,Browse URLs,Cloud Cover,Day/Night,Size\n";

    /// Como o `host.fetch` responde: o texto dentro de um JSON.
    fn resposta(t: &str) -> String {
        json!({"ok": {"status": 200, "text": t, "url": "x"}}).to_string()
    }

    fn csv(cenas: &[(&str, &str)]) -> String {
        let l: Vec<String> = cenas
            .iter()
            .map(|(n, nuvem)| {
                format!("{n},,2024-03-18T16:50:39.000Z,2024-03-18T16:50:51.000Z,\"https://d/{n}.nc,https://d/{n}_unc.nc\",\"https://d/{n}.png,s3://d/{n}.png\",{nuvem},DAY,3557.2\n")
            })
            .collect();
        resposta(&format!("{CAB}{}", l.concat()))
    }

    fn kml(nomes: &[&str]) -> String {
        let p: Vec<String> = nomes
            .iter()
            .map(|n| format!("<Placemark><name>{n}</name><styleUrl>#geodetic_style</styleUrl><Polygon><tessellate>1</tessellate><outerBoundaryIs><LinearRing><coordinates>-42.6537309,-20.9838736 -43.2755813,-20.5325538 -43.7686232,-21.1053925 -43.1464732,-21.5589052 -42.6537309,-20.9838736</coordinates></LinearRing></outerBoundaryIs></Polygon></Placemark>"))
            .collect();
        resposta(&format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><kml xmlns=\"http://www.opengis.net/kml/2.2\"><Document><Style id=\"geodetic_style\"></Style>{}</Document></kml>",
            p.concat()
        ))
    }

    const A2: &str = "EMIT_L2A_RFL_002_20240318T165039";
    const B2: &str = "EMIT_L2A_RFL_002_20240912T181913";
    const A1: &str = "EMIT_L2A_RFL_001_20240318T165039_2407811_015";
    const C1: &str = "EMIT_L2A_RFL_001_20251120T131415_2532409_003";

    #[test]
    fn os_enderecos_da_busca() {
        let c = [-43.0049, -20.839, -42.7536, -20.6547];
        let u = url("kml", 2, &[c], 100, None, 1);
        assert_eq!(
            u,
            "https://cmr.earthdata.nasa.gov/search/granules.kml?short_name=EMITL2ARFL&version=002&sort_key=start_date&page_size=2000&page_num=1&bounding_box=-43.0049,-20.8390,-42.7536,-20.6547"
        );
        let u = url("csv", 1, &[c, [-60.0, -10.0, -50.0, 0.0]], 30, Some(2024), 3);
        assert!(u.contains("granules.csv?") && u.contains("version=001") && u.contains("page_num=3"), "{u}");
        assert!(u.contains("&bounding_box%5B%5D=-43.0049,-20.8390,-42.7536,-20.6547&bounding_box%5B%5D=-60.0000,-10.0000,-50.0000,0.0000&options%5Bbounding_box%5D%5Bor%5D=true"), "{u}");
        assert!(u.ends_with("&cloud_cover=0,30&temporal=2024-01-01T00:00:00Z,2024-12-31T23:59:59Z"), "{u}");
    }

    #[test]
    fn as_cenas_juntam_as_versoes() {
        // A cena A tem as duas versões: na V001 ela é limpa (5%), mas a V002 a dá com 53%, acima do
        // limite (não está no CSV dela): sai. A C só tem a V001.
        let (k2, c2) = (kml(&[A2, B2]), csv(&[(B2, "0")]));
        let (k1, c1) = (kml(&[A1, C1]), csv(&[(A1, "5"), (C1, "12")]));
        assert!(eh_lista("kml", &k2) && eh_lista("csv", &c1) && eh_lista("csv", &csv(&[])));
        assert!(!eh_lista("kml", &resposta("Service Unavailable")) && !eh_lista("csv", &resposta("{\"errors\":[]}")));
        let (v2, v1) = (Listas { kml: vec![&k2], csv: vec![&c2] }, Listas { kml: vec![&k1], csv: vec![&c1] });
        let l = cenas(&v2, &v1);
        let nomes: Vec<&str> = l.iter().map(|c| c.nome).collect();
        assert_eq!(nomes, [C1, B2], "a mais limpa por último, por cima no mapa");
        let (t, n) = geojson(&l);
        let fc: Value = serde_json::from_str(&t).expect("o GeoJSON é JSON");
        assert_eq!((n, fc["features"].as_array().map(Vec::len)), (2, Some(2)));
        let f = &fc["features"][0];
        assert_eq!(f["geometry"]["coordinates"][0][0], json!([-42.6537309, -20.9838736]));
        assert_eq!(f["geometry"]["coordinates"][0].as_array().map(Vec::len), Some(5));
        let p = &f["properties"];
        assert_eq!((p["data"].as_str(), p["hora_utc"].as_str(), p["ano"].as_i64()), (Some("2025-11-20"), Some("13:14"), Some(2025)));
        assert_eq!(
            (p["nuvem_pct"].as_i64(), p["classe"].as_str(), p["versao"].as_str()),
            (Some(12), Some("11 a 30% de nuvem"), Some("V001"))
        );
        let base = "https://data.lpdaac.earthdatacloud.nasa.gov/lp-prod";
        assert_eq!(p["arquivo"], format!("{base}-protected/EMITL2ARFL.001/{C1}/{C1}.nc"));
        assert_eq!(p["previa"], format!("{base}-public/EMITL2ARFL.001/{C1}/{C1}.png"));
        assert_eq!(fc["features"][1]["properties"]["versao"], "V002");
        assert_eq!(fc["features"][1]["properties"]["arquivo"], format!("{base}-protected/EMITL2ARFL.002/{B2}/{B2}.nc"));
        assert_eq!(as_mais_limpas(&fc), ["12/09/2024 (0%)", "20/11/2025 (12%)"]);
        // Sem limite, a A entra uma vez, com a nuvem da V002.
        let c2 = csv(&[(A2, "53"), (B2, "0")]);
        let todas = cenas(&Listas { kml: vec![&k2], csv: vec![&c2] }, &v1);
        assert_eq!(todas.iter().map(|c| (c.nome, c.nuvem)).collect::<Vec<_>>(), [(A2, 53), (C1, 12), (B2, 0)]);
        // Sem a V001 (quando a NASA a tirar do ar) e sem nada.
        let vazio = || Listas { kml: vec![], csv: vec![] };
        assert_eq!(cenas(&v2, &vazio()).len(), 1);
        assert!(cenas(&vazio(), &vazio()).is_empty());
        assert_eq!(geojson(&[]), ("{\"type\":\"FeatureCollection\",\"features\":[]}".into(), 0));
        // Uma cena repetida entre páginas entra uma vez.
        assert_eq!(cenas(&Listas { kml: vec![&k2, &k2], csv: vec![&c2, &c2] }, &vazio()).len(), 2);
    }

    #[test]
    fn as_faixas_seguem_a_area() {
        // Um triângulo: largo embaixo, fino em cima.
        let fc = json!({"features": [{"geometry": {"type": "Polygon", "coordinates": [[[-60.0, -10.0], [-40.0, -10.0], [-50.0, 0.0], [-60.0, -10.0]]]}}]});
        let c = [-60.0, -10.0, -40.0, 0.0];
        let f = faixas(&fc, c, 0.5);
        assert_eq!(f.len(), 5);
        assert_eq!(f[0], [-60.0, -10.0, -40.0, -8.0]);
        // A de cima: de −52,5 a −47,5 na base com a folga, mais a folga dos lados.
        assert_eq!(f[4], [-53.0, -2.0, -47.0, 0.0]);
        assert!(f.windows(2).all(|p| p[0][3] == p[1][1]), "sem vão entre as faixas");
        // Uma ilha perdida (a área vai até −30, o contorno não): o retângulo todo.
        let longe = [-60.0, -10.0, -30.0, 0.0];
        assert_eq!(faixas(&fc, longe, 0.5), vec![longe]);
        assert_eq!(faixas(&json!({"features": []}), c, 0.5), vec![c]);
        // Duas partes, uma faixa só da largura das duas.
        let fc = json!({"features": [{"geometry": {"type": "MultiPolygon", "coordinates": [
            [[[-60.0, -1.0], [-59.0, -1.0], [-59.0, 0.0], [-60.0, -1.0]]], [[[-41.0, -1.0], [-40.0, -1.0], [-40.0, 0.0], [-41.0, -1.0]]]]}}]});
        assert_eq!(faixas(&fc, [-60.0, -1.0, -40.0, 0.0], 0.1), vec![[-60.0, -1.0, -40.0, 0.0]]);
    }

    #[test]
    fn as_linhas_e_os_campos_do_csv() {
        assert_eq!(linhas("a\\nb \\\"x,y\\\"\\nc").collect::<Vec<_>>(), ["a", "b \\\"x,y\\\"", "c"]);
        let l = |urls: &str, nuvem: &str| format!("{A1},,s,e,{urls},\\\"https://d/p1.png,s3://d/p1.png\\\",{nuvem},DAY,3575.3");
        let doc = l("\\\"https://d/u1.nc,https://d/u2.nc\\\"", "29");
        assert_eq!(da_linha(&doc), Some((A1, 29)));
        assert_eq!(da_linha(&l("https://d/u1.nc", "0")), Some((A1, 0)), "um endereço só vem sem aspas");
        assert_eq!(da_linha("a,b"), None);
        assert_eq!(da_linha(&l("", "")), None, "sem a nuvem");
        assert!(!cheia("csv", &doc) && !cheia("kml", ""));
        assert!(cheia("kml", &"x".repeat(700_000)) && cheia("csv", &"x".repeat(1_380_000)));
    }

    #[test]
    fn um_contorno_estranho_fica_de_fora() {
        let a = |c: &str| {
            let mut t = b"x".to_vec();
            let ok = anel(c, &mut t);
            (ok, String::from_utf8(t).unwrap())
        };
        assert_eq!(a("-42,-20 -43,-20.5 -43,-21 -42,-20"), (true, "x[-42,-20],[-43,-20.5],[-43,-21],[-42,-20]".into()));
        assert_eq!(a("1,2,0 3,4,0 5,6,0 1,2,0"), (true, "x[1,2],[3,4],[5,6],[1,2]".into()), "a altitude sai");
        assert_eq!(a("-42,-20 -43,-20"), (false, "x".into()));
        assert_eq!(a("-42,-20 x,-20 -43,-21 -42,-20"), (false, "x".into()));
        assert_eq!(a("-42 -43 -43 -42"), (false, "x".into()));
        assert!(instante("EMIT_L2A_RFL_002_2024").is_none());
        assert_eq!(instante(A1), instante(A2));
    }

    #[test]
    fn os_nomes_as_classes_e_o_estilo() {
        assert_eq!(camada(30, None), "BrasilData: Cenas do EMIT com até 30% de nuvem (NASA)");
        assert_eq!(camada(30, Some(2024)), "BrasilData: Cenas do EMIT de 2024 com até 30% de nuvem (NASA)");
        assert_eq!(camada(100, None), "BrasilData: Cenas do EMIT (NASA)");
        assert_eq!(quais(&camada(30, Some(2024))), " de 2024 com até 30% de nuvem");
        assert!([camada(10, None), camada(100, Some(2026))].iter().all(|n| eh_do_emit(n)));
        assert!(!eh_do_emit(NO_RETANGULO));
        assert_eq!(
            (classe(0), classe(10), classe(11), classe(30), classe(31), classe(100)),
            (CLASSES[0].1, CLASSES[0].1, CLASSES[1].1, CLASSES[1].1, CLASSES[2].1, CLASSES[3].1)
        );
        let mut nomes: Vec<&str> = CLASSES.iter().map(|c| c.1).collect();
        let ordem = nomes.clone();
        nomes.sort_unstable();
        assert_eq!(nomes, ordem, "a ordem alfabética é a da legenda");
        for (nuvem, quantas) in [(100, 4), (60, 4), (50, 3), (30, 2), (11, 2), (10, 1), (0, 1)] {
            let e: Value = serde_json::from_str(&estilo(nuvem)).unwrap();
            assert_eq!(e[0]["renderer"]["Unique"]["classes"].as_array().map(Vec::len), Some(quantas), "{nuvem}");
        }
        assert_eq!(conferir_nuvem(30.0), Ok(30));
        assert!(conferir_nuvem(101.0).is_err() && conferir_nuvem(-1.0).is_err() && conferir_nuvem(1.5).is_err());
        assert_eq!(conferir_ano(2024.0), Ok(2024));
        assert!(conferir_ano(2021.0).is_err() && conferir_ano(2024.5).is_err());
    }
}
