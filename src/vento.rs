//! O vento a 50 m do NASA POWER: a climatologia de 2001 a 2025 da velocidade média (`WS50M`), em
//! m/s, da reanálise MERRA-2 (NASA GMAO).
//!
//! - **De onde:** a mesma API da radiação solar ([`crate::sol`]), sem login e com CORS, um parâmetro
//!   por pedido, em retângulos de 2° a 10°. Os pontos vêm na grade do MERRA-2, de 0,625° de
//!   longitude por 0,5° de latitude, com os das bordas repetidos nos pedidos vizinhos.
//! - **Como:** cada ponto vira a célula em volta dele, as células viram um raster (`raster.rasterize`
//!   com `cellSize` de dois valores), que vai para a grade do projeto por interpolação bilinear e é
//!   recortado pela área, como a radiação.
//! - **A escala:** o modelo é de cerca de 50 km: serve para o país, uma região ou um estado, não para
//!   um município nem para localizar uma turbina. A área com menos de [`MIN_CELULAS`] células recebe
//!   um aviso ([`celulas_na_area`]).

use crate::sol;

/// O nome da camada do ano.
pub const CAMADA: &str = "BrasilData: Vento a 50 m (NASA POWER)";

/// As camadas de passagem.
pub const CELULAS: &str = "BrasilData: vento em células do MERRA-2";
pub const RASTER: &str = "BrasilData: vento na grade do MERRA-2";
pub const NA_GRADE: &str = "BrasilData: vento na grade";

/// O parâmetro do POWER.
pub const PARAMETRO: &str = "WS50M";

/// O campo das células.
pub const CAMPO: &str = "m_s";

/// O lado das células, em graus (longitude e latitude), e um canto delas: os pontos são os múltiplos
/// do lado, e os cantos ficam a meia célula.
pub const LADO: (f64, f64) = (0.625, 0.5);
pub const CANTO: (f64, f64) = (0.3125, 0.25);

/// Abaixo disto (em células do MERRA-2, no retângulo da área), o mapa quase só mostra a interpolação.
pub const MIN_CELULAS: f64 = 10.0;

/// A escala das cores, em m/s: dos pontos em terra no Brasil, 98% ficam abaixo de 6,9 na média do ano
/// e de 7,4 nas dos meses (o maior, 9,4).
pub const ESCALA: (f64, f64) = (1.5, 8.0);

/// A rampa.
pub const RAMPA: &str = "Viridis";

/// A citação, como o POWER pede, com a versão da API.
pub const CITACAO: &str = "The data was obtained from the NASA Langley Research Center (LaRC) POWER Project funded through the NASA Earth Science/Applied Science Program (API de climatologia v2.10.2). Velocidade do vento a 50 m (WS50M), média de 2001 a 2025, da reanálise MERRA-2 (NASA GMAO; Gelaro et al., 2017, doi:10.1175/JCLI-D-16-0758.1), em células de 0,625° por 0,5°; interpolada para a grade do projeto e recortada pela área";

/// O nome da camada de um período: a do ano é [`CAMADA`].
pub fn camada(mes: Option<u8>) -> String {
    match mes {
        None => CAMADA.to_string(),
        Some(_) => format!("BrasilData: Vento a 50 m em {} (NASA POWER)", nome_do_mes(mes)),
    }
}

/// "março", ou "ano".
fn nome_do_mes(mes: Option<u8>) -> String {
    sol::periodo(mes).trim_start_matches("média diária d").trim_start_matches("e ").trim_start_matches("o ").to_string()
}

/// O período, para os textos: "média do ano" ou "média de março".
pub fn periodo(mes: Option<u8>) -> String {
    match mes {
        None => "média do ano".into(),
        Some(_) => format!("média de {}", nome_do_mes(mes)),
    }
}

/// O mês da camada `nome`: `None` na do ano.
pub fn mes_da_camada(nome: &str) -> Option<u8> {
    (1..=12).find(|&m| camada(Some(m)) == nome)
}

/// Se a camada `nome` é a do vento (do ano ou de um mês).
pub fn eh_do_vento(nome: &str) -> bool {
    nome == CAMADA || mes_da_camada(nome).is_some()
}

/// Os pedidos que cobrem o retângulo `c`, em CSV ([`sol::pedidos`]).
pub fn pedidos(c: [f64; 4]) -> Vec<String> {
    sol::pedidos_de(c, PARAMETRO)
}

/// Quantas células do MERRA-2 cabem no retângulo `c`.
pub fn celulas_na_area(c: [f64; 4]) -> f64 {
    ((c[2] - c[0]) / LADO.0) * ((c[3] - c[1]) / LADO.1)
}

/// As células do MERRA-2 das `respostas`, com o valor do período em [`CAMPO`] ([`sol::celulas_de`]).
pub fn celulas(respostas: &[&str], mes: Option<u8>) -> (String, usize) {
    sol::celulas_de(respostas, PARAMETRO, LADO, CAMPO, mes)
}

/// Se o cabeçalho do CSV (na resposta crua) diz que o dado é do MERRA-2 (`WS50M     MERRA-2 Wind Speed at 50 Meters`);
/// nos dados recentes, o POWER passa para o GEOS-IT.
pub fn do_merra2(texto: &str) -> bool {
    texto.split("\\n").take_while(|l| !l.starts_with("-END HEADER-")).any(|l| l.starts_with(PARAMETRO) && l.contains("MERRA-2"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn os_pontos_viram_celulas_do_merra2_sem_os_sem_dado_e_sem_repetir_a_borda() {
        let cab = "-BEGIN HEADER-\r\nParameter(s): \r\nWS50M     MERRA-2 Wind Speed at 50 Meters (m/s) \r\n-END HEADER-\r\nPARAMETER,LAT,LON,JAN,FEB,MAR,APR,MAY,JUN,JUL,AUG,SEP,OCT,NOV,DEC,ANN\r\n";
        let linha = |x: f64, y: f64, set: f64, ann: f64| format!("WS50M,{y},{x},1,2,3,4,5,6,7,8,{set},10,11,12,{ann}\r\n");
        // Como o `host.fetch` responde: o texto dentro de um JSON.
        let resposta = |t: String| json!({"ok": {"status": 200, "text": t}}).to_string();
        let a = resposta(format!("{cab}{}{}", linha(-43.125, -20.5, 5.1, 4.5), linha(-42.5, -20.5, -999.0, -999.0)));
        let b = resposta(format!("{cab}{}{}", linha(-43.125, -20.5, 5.1, 4.5), linha(-43.125, -20.0, 5.0, 4.2)));
        let r = [a.as_str(), b.as_str()];
        assert!(do_merra2(r[0]));
        assert!(!do_merra2(&a.replace("MERRA-2", "GEOS-IT")));
        assert_eq!(celulas(&["{\"ok\":{\"text\":\"Service Unavailable\"}}"], None).1, 0);
        let (texto, n) = celulas(&r, None);
        let fc: Value = serde_json::from_str(&texto).unwrap();
        let f = fc["features"].as_array().unwrap();
        assert_eq!((f.len(), n), (2, 2));
        assert_eq!(f[0]["properties"][CAMPO], 4.5);
        assert_eq!(f[0]["geometry"]["coordinates"][0][0], json!([-43.4375, -20.75]));
        assert_eq!(f[0]["geometry"]["coordinates"][0][2], json!([-42.8125, -20.25]));
        let set: Value = serde_json::from_str(&celulas(&r, Some(9)).0).unwrap();
        assert_eq!(set["features"][1]["properties"][CAMPO], 5.0);
        // Os cantos caem na rede do raster de origem CANTO.
        for (v, o, l) in [(-43.4375, CANTO.0, LADO.0), (-20.75, CANTO.1, LADO.1)] {
            let n: f64 = (v - o) / l;
            assert!((n - n.round()).abs() < 1e-9, "{v}");
        }
    }

    #[test]
    fn os_nomes_e_os_periodos() {
        assert_eq!(camada(None), CAMADA);
        assert_eq!(camada(Some(3)), "BrasilData: Vento a 50 m em março (NASA POWER)");
        assert_eq!(camada(Some(10)), "BrasilData: Vento a 50 m em outubro (NASA POWER)");
        assert_eq!(periodo(None), "média do ano");
        assert_eq!(periodo(Some(9)), "média de setembro");
        assert_eq!(mes_da_camada(&camada(Some(9))), Some(9));
        assert!(eh_do_vento(CAMADA) && eh_do_vento(&camada(Some(12))) && !eh_do_vento(sol::CAMADA));
        assert!(!sol::eh_do_sol(CAMADA));
    }

    #[test]
    fn a_area_pequena_e_o_pedido() {
        assert!(celulas_na_area([-43.0, -20.85, -42.75, -20.65]) < 1.0, "Viçosa");
        assert!(celulas_na_area([-51.0, -22.9, -39.9, -14.2]) > 100.0, "Minas Gerais");
        assert!(pedidos([-43.0, -20.85, -42.75, -20.62])[0].ends_with("&parameters=WS50M&community=RE&format=CSV"));
    }
}
