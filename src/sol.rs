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

use serde_json::{Value, json};

/// O nome da camada.
pub const CAMADA: &str = "BrasilData: Radiação solar (NASA POWER)";

/// As camadas de passagem: as células e o raster de 1°.
pub const CELULAS: &str = "BrasilData: radiação solar em células de 1°";
pub const RASTER_1: &str = "BrasilData: radiação solar a 1°";
pub const NA_GRADE: &str = "BrasilData: radiação solar na grade";

/// O parâmetro do POWER.
const PARAMETRO: &str = "ALLSKY_SFC_SW_DWN";

/// O valor que o POWER dá onde não há dado.
const SEM_DADO: f64 = -999.0;

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
    let (o, s) = (c[0].floor() as i32 - 1, c[1].floor() as i32 - 1);
    let (l, n) = (c[2].ceil() as i32 + 1, c[3].ceil() as i32 + 1);
    let mut urls = Vec::new();
    for (y0, y1) in pedacos(s, n) {
        for (x0, x1) in pedacos(o, l) {
            urls.push(format!(
                "https://power.larc.nasa.gov/api/temporal/climatology/regional?latitude-min={y0}&latitude-max={y1}&longitude-min={x0}&longitude-max={x1}&parameters={PARAMETRO}&community=RE&format=JSON"
            ));
        }
    }
    urls
}

/// O erro de uma resposta do POWER sem pontos (as `messages` dele).
pub fn erro(resposta: &Value) -> Option<String> {
    if resposta["features"].is_array() {
        return None;
    }
    let msgs: Vec<&str> = resposta["messages"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
    Some(if msgs.is_empty() {
        "o NASA POWER não devolveu os pontos".into()
    } else {
        format!("o NASA POWER recusou o pedido: {}", msgs.join(" "))
    })
}

/// As células de 1° das `respostas`, com o valor do período em [`CAMPO`]: um polígono por ponto
/// com dado, sem repetir pontos.
pub fn celulas(respostas: &[Value], mes: Option<u8>) -> Value {
    let k = chave(mes);
    let mut vistas: Vec<(i64, i64)> = Vec::new();
    let mut feicoes = Vec::new();
    for f in respostas.iter().flat_map(|r| r["features"].as_array().into_iter().flatten()) {
        let p = &f["geometry"]["coordinates"];
        let (Some(x), Some(y)) = (p[0].as_f64(), p[1].as_f64()) else { continue };
        let Some(v) = f["properties"]["parameter"][PARAMETRO][k].as_f64() else { continue };
        let chave = ((x * 2.0).round() as i64, (y * 2.0).round() as i64);
        if v <= SEM_DADO + 0.5 || v < 0.0 || vistas.contains(&chave) {
            continue;
        }
        vistas.push(chave);
        let (x0, y0, x1, y1) = (x - 0.5, y - 0.5, x + 0.5, y + 0.5);
        feicoes.push(json!({"type": "Feature",
            "geometry": {"type": "Polygon", "coordinates": [[[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]},
            "properties": {CAMPO: v}}));
    }
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// A escala das cores, em kWh/m² por dia: o Brasil vai de cerca de 4 a 6,5 na média do ano.
pub const ESCALA: (f64, f64) = (4.0, 6.5);

/// A rampa de cores (formato do GDAL), do amarelo-claro ao vermelho-escuro.
pub const RAMPA: (&str, &str) =
    ("BrasilData: radiação solar", "4 255 245 200\n4.5 254 224 139\n5 253 174 97\n5.5 244 109 67\n6 215 48 39\n6.5 140 20 30");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn um_municipio_pede_um_retangulo_de_graus_inteiros_com_margem() {
        let u = pedidos([-43.0, -20.85, -42.75, -20.62]);
        assert_eq!(u.len(), 1);
        assert!(u[0].contains("latitude-min=-22&latitude-max=-19&longitude-min=-44&longitude-max=-41"), "{}", u[0]);
        assert!(u[0].contains("parameters=ALLSKY_SFC_SW_DWN&community=RE&format=JSON"));
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
        let ponto = |x: f64, y: f64, ann: f64, jan: f64| {
            json!({"type": "Feature", "geometry": {"type": "Point", "coordinates": [x, y, 700.0]},
                "properties": {"parameter": {"ALLSKY_SFC_SW_DWN": {"JAN": jan, "ANN": ann}}}})
        };
        let r = [
            json!({"type": "FeatureCollection", "features": [ponto(-42.5, -20.5, 4.8554, 5.6), ponto(-41.5, -20.5, -999.0, -999.0)]}),
            json!({"type": "FeatureCollection", "features": [ponto(-42.5, -20.5, 4.8554, 5.6), ponto(-42.5, -19.5, 4.84, 5.5)]}),
        ];
        let fc = celulas(&r, None);
        let f = fc["features"].as_array().unwrap();
        assert_eq!(f.len(), 2);
        assert_eq!(f[0]["properties"][CAMPO], 4.8554);
        assert_eq!(f[0]["geometry"]["coordinates"][0][0], json!([-43.0, -21.0]));
        assert_eq!(f[0]["geometry"]["coordinates"][0][2], json!([-42.0, -20.0]));
        assert_eq!(celulas(&r, Some(1))["features"][1]["properties"][CAMPO], 5.5);
        assert!(erro(&r[0]).is_none());
        let recusa = json!({"header": "x", "messages": ["Please provide at least a 2 degree range in latitude."]});
        assert!(erro(&recusa).unwrap().contains("2 degree range"));
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
