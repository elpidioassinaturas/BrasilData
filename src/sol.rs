//! A radiação solar na superfície do NASA POWER: a climatologia de 2001 a 2025 da irradiação
//! global horizontal (todos os céus, `ALLSKY_SFC_SW_DWN`), em kWh/m² por dia, do CERES SYN1deg.
//!
//! - **De onde:** a API de climatologia regional do POWER (`power.larc.nasa.gov`, sem login, com
//!   CORS), que devolve os pontos de uma grade de 1° (centros em x,5°) de um retângulo de 2° a 10°
//!   de cada lado. O retângulo da área vira o dos graus inteiros com uma célula de margem, em
//!   pedaços de até 10°.
//! - **Como:** cada ponto vira a célula de 1° em volta dele (um polígono com o valor), as células
//!   viram um raster de 1° (`raster.rasterize`), que vai para a grade do projeto por interpolação
//!   bilinear (a radiação varia devagar: a margem dá os vizinhos das bordas) e é recortado pela área.

use serde_json::Value;

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Radiação solar (NASA POWER)";

/// As camadas de passagem: as células e o raster de 1°.
pub const CELULAS: &str = "BrasilData: radiação solar em células de 1°";
pub const RASTER_1: &str = "BrasilData: radiação solar a 1°";
pub const NA_GRADE: &str = "BrasilData: radiação solar na grade";

/// O parâmetro do POWER.
const PARAMETRO: &str = "ALLSKY_SFC_SW_DWN";

/// O campo das células.
pub const CAMPO: &str = "kwh_m2_dia";

/// A citação, como o POWER pede, com a versão da API.
pub const CITACAO: &str = "The data was obtained from the NASA Langley Research Center (LaRC) POWER Project funded through the NASA Earth Science/Applied Science Program (API de climatologia v2.10.1). Irradiação global horizontal na superfície, todos os céus (ALLSKY_SFC_SW_DWN), média diária de 2001 a 2025, do CERES SYN1deg, em células de 1°; interpolada para a grade do projeto e recortada pela área";

/// A licença: a política de dados da NASA.
pub const LICENCA: &str = "política de dados da NASA: uso livre e gratuito, com a citação";

/// Os meses, como o POWER os chama, e o ano.
pub const MESES: [&str; 12] = ["JAN", "FEB", "MAR", "APR", "MAY", "JUN", "JUL", "AUG", "SEP", "OCT", "NOV", "DEC"];
const NOMES_DOS_MESES: [&str; 12] =
    ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

/// O período: a média do ano (`None`) ou a de um mês (1 a 12).
pub fn chave(mes: Option<u8>) -> &'static str {
    mes.and_then(|m| MESES.get(usize::from(m).wrapping_sub(1)).copied()).unwrap_or("ANN")
}

/// O período, para o nome da camada e os avisos.
pub fn periodo(mes: Option<u8>) -> String {
    match mes.and_then(|m| NOMES_DOS_MESES.get(usize::from(m).wrapping_sub(1))) {
        Some(n) => format!("média diária de {n}"),
        None => "média diária do ano".into(),
    }
}

/// O mês do parâmetro `mes` (1 a 12, ou ausente para o ano).
pub fn conferir_mes(mes: Option<f64>) -> Result<Option<u8>, String> {
    match mes {
        None => Ok(None),
        Some(m) if m.fract() == 0.0 && (1.0..=12.0).contains(&m) => Ok(Some(m as u8)),
        Some(m) => Err(format!("o mes vai de 1 a 12 (sem ele, a média do ano), não {m}")),
    }
}

/// O nome da camada de um período: a do ano é [`CAMADA`].
pub fn camada(mes: Option<u8>) -> String {
    match mes {
        None => CAMADA.to_string(),
        Some(_) => format!("BrasilData: Radiação solar em {} (NASA POWER)", periodo(mes).trim_start_matches("média diária de ")),
    }
}

/// O mês da camada `nome` ([`camada`]): `None` na do ano.
pub fn mes_da_camada(nome: &str) -> Option<u8> {
    (1..=12).find(|&m| camada(Some(m)) == nome)
}

/// Se a camada `nome` é a da radiação solar (do ano ou de um mês).
pub fn eh_do_sol(nome: &str) -> bool {
    nome == CAMADA || (nome.starts_with("BrasilData: Radiação solar em ") && nome.ends_with(" (NASA POWER)"))
}

/// O maior lado de um pedido, em graus (o POWER recusa mais de 10°).
const MAX_LADO: i32 = 10;

/// Os intervalos inteiros de `a` a `b` em pedaços iguais de até [`MAX_LADO`] graus. Com a margem,
/// cada um tem 3° ou mais, acima dos 2° que o POWER pede.
fn pedacos(a: i32, b: i32) -> Vec<(i32, i32)> {
    let n = (b - a + MAX_LADO - 1) / MAX_LADO;
    let passo = (b - a + n - 1) / n;
    (0..n).map(|i| (a + i * passo, (a + (i + 1) * passo).min(b))).filter(|(x, y)| y > x).collect()
}

/// Os pedidos que cobrem o retângulo `c` (`[oeste, sul, leste, norte]`) com uma célula de margem:
/// os pontos que voltam são os de centro dentro de cada retângulo, que não se sobrepõem. Cada
/// resposta traz os 12 meses e o ano.
pub fn pedidos(c: [f64; 4]) -> Vec<String> {
    pedidos_de(c, PARAMETRO)
}

/// [`pedidos`] de outro parâmetro do POWER (um por pedido), em CSV, que é um terço do JSON.
pub fn pedidos_de(c: [f64; 4], parametro: &str) -> Vec<String> {
    pedidos_com(c, parametro, "climatology/regional?")
}

/// Os pedidos dos 12 meses e do ano `ano` (a API mensal), nos mesmos retângulos.
pub fn pedidos_do_ano(c: [f64; 4], parametro: &str, ano: i32) -> Vec<String> {
    pedidos_com(c, parametro, &format!("monthly/regional?start={ano}&end={ano}&"))
}

fn pedidos_com(c: [f64; 4], parametro: &str, inicio: &str) -> Vec<String> {
    let (o, s) = (c[0].floor() as i32 - 1, c[1].floor() as i32 - 1);
    let (l, n) = (c[2].ceil() as i32 + 1, c[3].ceil() as i32 + 1);
    let mut urls = Vec::new();
    for (y0, y1) in pedacos(s, n) {
        for (x0, x1) in pedacos(o, l) {
            urls.push(format!(
                "https://power.larc.nasa.gov/api/temporal/{inicio}latitude-min={y0}&latitude-max={y1}&longitude-min={x0}&longitude-max={x1}&parameters={parametro}&community=RE&format=CSV"
            ));
        }
    }
    urls
}

/// O erro de uma resposta crua do `host.fetch` que não é o CSV do POWER (as `messages` dele).
pub fn erro(resposta: &str) -> Option<String> {
    if resposta.contains("-END HEADER-") {
        return None;
    }
    // Uma recusa é curta; um texto longo que não é o CSV não se lê, pela cota de instruções.
    let v: Value = Some(resposta).filter(|r| r.len() < 16_384).and_then(|r| serde_json::from_str(r).ok()).unwrap_or_default();
    let corpo = v["ok"]["text"].as_str().and_then(|t| serde_json::from_str::<Value>(t).ok()).unwrap_or_default();
    let msgs: Vec<&str> = corpo["messages"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    Some(if msgs.is_empty() {
        "o NASA POWER não devolveu os pontos".into()
    } else {
        format!("o NASA POWER recusou o pedido: {}", msgs.join(" "))
    })
}

/// As células de 1° das `respostas`, com o valor do período em [`CAMPO`] ([`celulas_de`]).
pub fn celulas(respostas: &[&str], mes: Option<u8>) -> (String, usize) {
    celulas_de(respostas, PARAMETRO, (1.0, 1.0), CAMPO, mes)
}

/// As células, do tamanho `lado` (longitude e latitude, em graus), das `respostas` cruas do
/// `host.fetch`, como o texto de um GeoJSON com o valor do período em `campo`, e quantas são
/// ([`pontos_de`] e [`celulas_dos_pontos`]).
pub fn celulas_de(respostas: &[&str], parametro: &str, lado: (f64, f64), campo: &str, mes: Option<u8>) -> (String, usize) {
    let pontos = pontos_de(respostas, parametro, lado, mes);
    (celulas_dos_pontos(&pontos, lado, campo), pontos.len())
}

/// O ponto `x`, `y` em meias células de `lado`: os pontos são múltiplos dele, ou os centros entre eles.
pub fn chave_do_ponto(x: f64, y: f64, lado: (f64, f64)) -> (i32, i32) {
    ((x * 2.0 / lado.0).round() as i32, (y * 2.0 / lado.1).round() as i32)
}

/// Os pontos com dado (longitude, latitude e o valor do período) das `respostas` cruas do `host.fetch`,
/// sem repetir os das bordas. Cada resposta é o CSV do POWER dentro do JSON da resposta, com as quebras
/// de linha escapadas: um cabeçalho livre, a linha dos nomes `PARAMETER,LAT,LON,JAN,…,ANN` (na API
/// mensal, com o `YEAR` depois do `PARAMETER`) e uma linha por ponto.
pub fn pontos_de(respostas: &[&str], parametro: &str, lado: (f64, f64), mes: Option<u8>) -> Vec<(f64, f64, f64)> {
    let k = chave(mes);
    let mut vistas = std::collections::HashSet::new();
    let mut pontos = Vec::new();
    for t in respostas {
        let (mut coluna, mut com_ano) = (None, false);
        for l in t.split("\\n") {
            let l = l.trim_end_matches("\\r");
            if let Some(nomes) = l.strip_prefix("PARAMETER,") {
                let sem_ano = nomes.strip_prefix("YEAR,");
                com_ano = sem_ano.is_some();
                coluna = sem_ano.unwrap_or(nomes).strip_prefix("LAT,LON,").and_then(|n| n.split(',').position(|n| n == k));
                continue;
            }
            let (Some(i), Some(resto)) = (coluna, l.strip_prefix(parametro).and_then(|r| r.strip_prefix(','))) else { continue };
            let mut p = resto.split(',');
            if com_ano {
                p.next();
            }
            let (Some(Ok(y)), Some(Ok(x)), Some(v)) = (p.next().map(str::parse::<f64>), p.next().map(str::parse::<f64>), p.nth(i)) else {
                continue;
            };
            // O sem dado (-999) e o que não for um número ficam de fora.
            let Some(v) = v.parse::<f64>().ok().filter(|v| *v >= 0.0 && v.is_finite()) else { continue };
            if vistas.insert(chave_do_ponto(x, y, lado)) {
                pontos.push((x, y, v));
            }
        }
    }
    pontos
}

/// As células, do tamanho `lado`, em volta dos `pontos`, como o texto de um GeoJSON com o valor em
/// `campo`. Em texto, e não em `Value`: o Brasil inteiro (25 pedidos, mais de 2 mil pontos da radiação e
/// 6 mil do vento) passaria da cota de instruções do plugin, com o JSON do POWER ou com o GeoJSON em
/// `Value`.
pub fn celulas_dos_pontos(pontos: &[(f64, f64, f64)], lado: (f64, f64), campo: &str) -> String {
    let mut geojson = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    for (i, &(x, y, v)) in pontos.iter().enumerate() {
        if i > 0 {
            geojson.push(',');
        }
        let (x0, y0) = (decimos(x - lado.0 / 2.0), decimos(y - lado.1 / 2.0));
        let (x1, y1) = (decimos(x + lado.0 / 2.0), decimos(y + lado.1 / 2.0));
        let v = decimos(v);
        geojson.push_str(&format!(
            "{{\"type\":\"Feature\",\"geometry\":{{\"type\":\"Polygon\",\"coordinates\":[[[{x0},{y0}],[{x1},{y0}],[{x1},{y1}],[{x0},{y1}],[{x0},{y0}]]]}},\"properties\":{{\"{campo}\":{v}}}}}"
        ));
    }
    geojson.push_str("]}");
    geojson
}

/// `v` com quatro casas (os cantos das células são exatos nelas, e o POWER não dá mais), por
/// inteiros, que custam menos que o texto de um `f64`.
pub fn decimos(v: f64) -> String {
    crate::malhas::decimos((v * 10_000.0).round() as i64)
}

/// A escala das cores, em kWh/m² por dia: o Brasil vai de cerca de 4 a 6,5 na média do ano.
pub const ESCALA: (f64, f64) = (4.0, 6.5);

/// A rampa de cores (formato do GDAL), do amarelo-claro ao vermelho-escuro.
pub const RAMPA: (&str, &str) =
    ("BrasilData: radiação solar", "4 255 245 200\n4.5 254 224 139\n5 253 174 97\n5.5 244 109 67\n6 215 48 39\n6.5 140 20 30");

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn um_municipio_pede_um_retangulo_de_graus_inteiros_com_margem() {
        let u = pedidos([-43.0, -20.85, -42.75, -20.62]);
        assert_eq!(u.len(), 1);
        assert!(u[0].contains("latitude-min=-22&latitude-max=-19&longitude-min=-44&longitude-max=-41"), "{}", u[0]);
        assert!(u[0].contains("parameters=ALLSKY_SFC_SW_DWN&community=RE&format=CSV"));
    }

    #[test]
    fn o_brasil_vai_em_pedacos_de_ate_10_graus_e_de_pelo_menos_2() {
        // O Brasil: de 74° a 28,8° O e de 33,8° S a 5,3° N.
        let u = pedidos([-74.0, -33.8, -28.8, 5.3]);
        assert_eq!(u.len(), 25);
        for (a, b) in [(-35, 7), (-75, -27), (0, 3), (-1, 10)] {
            let p = pedacos(a, b);
            assert_eq!(p.first().map(|x| x.0), Some(a));
            assert_eq!(p.last().map(|x| x.1), Some(b));
            assert!(p.iter().all(|(x, y)| y - x <= 10 && y - x >= 2), "{a} {b}: {p:?}");
            assert!(p.windows(2).all(|w| w[0].1 == w[1].0));
        }
    }

    #[test]
    fn os_pontos_viram_celulas_de_um_grau_sem_os_sem_dado_e_sem_repetir() {
        let cab = "-BEGIN HEADER-\r\nLocation: Regional \r\n-END HEADER-\r\nPARAMETER,LAT,LON,JAN,FEB,MAR,APR,MAY,JUN,JUL,AUG,SEP,OCT,NOV,DEC,ANN\r\n";
        let linha = |x: f64, y: f64, ann: f64, jan: f64| format!("ALLSKY_SFC_SW_DWN,{y},{x},{jan},2,3,4,5,6,7,8,9,10,11,12,{ann}\r\n");
        // Como o `host.fetch` responde: o texto dentro de um JSON.
        let resposta = |t: String| json!({"ok": {"status": 200, "text": t}}).to_string();
        let a = resposta(format!("{cab}{}{}", linha(-42.5, -20.5, 4.8554, 5.6), linha(-41.5, -20.5, -999.0, -999.0)));
        let b = resposta(format!("{cab}{}{}", linha(-42.5, -20.5, 4.8554, 5.6), linha(-42.5, -19.5, 4.84, 5.5)));
        let r = [a.as_str(), b.as_str()];
        let (texto, n) = celulas(&r, None);
        let fc: Value = serde_json::from_str(&texto).unwrap();
        let f = fc["features"].as_array().unwrap();
        assert_eq!((f.len(), n), (2, 2));
        assert_eq!(f[0]["properties"][CAMPO], 4.8554);
        assert_eq!(f[0]["geometry"]["coordinates"][0][0], json!([-43.0, -21.0]));
        assert_eq!(f[0]["geometry"]["coordinates"][0][2], json!([-42.0, -20.0]));
        let jan: Value = serde_json::from_str(&celulas(&r, Some(1)).0).unwrap();
        assert_eq!(jan["features"][1]["properties"][CAMPO], 5.5);
        assert!(erro(r[0]).is_none());
        // Os centros em volta do zero não se confundem.
        let c = resposta(format!("{cab}{}{}", linha(-60.5, -0.5, 4.5, 4.5), linha(-60.5, 0.5, 4.6, 4.6)));
        assert_eq!(celulas(&[&c], None).1, 2);
        // Uma recusa diz o motivo; outra coisa, que os pontos não vieram.
        let recusa = resposta(json!({"header": "x", "messages": ["Please provide at least a 2 degree range in latitude."]}).to_string());
        assert!(erro(&recusa).unwrap().contains("2 degree range"));
        assert_eq!(celulas(&[&recusa], None).1, 0);
        assert_eq!(erro(&resposta("Service Unavailable".into())).unwrap(), "o NASA POWER não devolveu os pontos");
        assert_eq!((decimos(-0.3125), decimos(0.25), decimos(-43.4375)), ("-0.3125".into(), "0.2500".into(), "-43.4375".into()));
    }

    #[test]
    fn o_mes_muda_a_chave_o_nome_e_o_texto() {
        assert_eq!(chave(None), "ANN");
        assert_eq!(chave(Some(1)), "JAN");
        assert_eq!(chave(Some(12)), "DEC");
        assert_eq!(conferir_mes(Some(7.0)), Ok(Some(7)));
        assert_eq!(conferir_mes(None), Ok(None));
        assert!(conferir_mes(Some(13.0)).is_err() && conferir_mes(Some(2.5)).is_err() && conferir_mes(Some(0.0)).is_err());
        assert_eq!(camada(None), CAMADA);
        assert_eq!(camada(Some(3)), "BrasilData: Radiação solar em março (NASA POWER)");
        assert!(eh_do_sol(CAMADA) && eh_do_sol(&camada(Some(3))) && !eh_do_sol("BrasilData: Elevação (Copernicus GLO-30)"));
        assert_eq!(periodo(Some(6)), "média diária de junho");
        assert_eq!(periodo(None), "média diária do ano");
        assert_eq!(mes_da_camada(&camada(Some(9))), Some(9));
        assert_eq!(mes_da_camada(CAMADA), None);
    }
}
