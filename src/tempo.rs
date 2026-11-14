//! Tempo e clima: a previsão do tempo dos modelos globais, pelo Open-Meteo, e a chuva observada do
//! mês, do CHIRPS v3.
//!
//! - **Previsão (Open-Meteo):** a API de previsão (`api.open-meteo.com`) e a de ensembles
//!   (`ensemble-api.open-meteo.com`), sem login, com CORS, que devolvem os valores diários de vários
//!   pontos num pedido. Os pontos são os nós de uma grade de 0,25° (0,5° no GEFS) que cercam a
//!   área; numa área grande, a grade dobra de passo até caber no limite de pontos (a API gratuita
//!   conta cada ponto como um pedido, até 600 por minuto, e o plugin tem um orçamento de
//!   processamento por clique, que um ensemble de 51 membros gasta depressa). Cada nó vira a célula em
//!   volta dele, as células viram um raster (`raster.rasterize`), que vai para a grade do projeto
//!   por interpolação bilinear e é recortado pela área, como a radiação solar do NASA POWER.
//! - **Os dias** contam a partir de hoje, no horário de Brasília (o dia de hoje inteiro, com as
//!   horas que já passaram tiradas da análise do modelo). O plugin não tem relógio: as datas vêm
//!   da resposta.
//! - **Chuva observada (CHIRPS v3):** o total do mês num COG mundial a 0,05° do Climate Hazards
//!   Center (UCSB), lido só na janela da área (`layer.addUrl` com `bbox`), como o Risco de Fogo.
//!   O servidor não manda CORS: só no app desktop.

use serde_json::{Value, json};

/// Um modelo de previsão do Open-Meteo.
#[derive(Debug, PartialEq)]
pub struct Modelo {
    /// O valor do parâmetro `modelo` dos comandos.
    pub chave: &'static str,
    /// O nome do modelo na API (`models=`).
    pub api: &'static str,
    /// O nome curto, para a camada.
    pub nome: &'static str,
    /// Quantos dias o modelo prevê.
    pub dias: usize,
    /// O passo da grade de pontos, em graus (a resolução do modelo).
    pub passo: f64,
    /// Quem faz o modelo, para a citação.
    pub autor: &'static str,
    /// Os membros (1 num modelo determinístico).
    pub membros: usize,
}

/// Os modelos determinísticos, o padrão primeiro.
pub const DETERMINISTICOS: [Modelo; 5] = [
    Modelo { chave: "ecmwf", api: "ecmwf_ifs025", nome: "ECMWF IFS", dias: 15, passo: 0.25, autor: "ECMWF (CC BY 4.0)", membros: 1 },
    Modelo {
        chave: "aifs",
        api: "ecmwf_aifs025_single",
        nome: "ECMWF AIFS",
        dias: 15,
        passo: 0.25,
        autor: "ECMWF (CC BY 4.0)",
        membros: 1,
    },
    Modelo { chave: "gfs", api: "gfs_seamless", nome: "GFS", dias: 16, passo: 0.25, autor: "NOAA/NCEP (domínio público)", membros: 1 },
    Modelo { chave: "icon", api: "icon_seamless", nome: "ICON", dias: 7, passo: 0.25, autor: "DWD (CC BY 4.0)", membros: 1 },
    Modelo { chave: "melhor", api: "best_match", nome: "melhor modelo do Open-Meteo", dias: 16, passo: 0.25, autor: "vários", membros: 1 },
];

/// Os ensembles, o padrão primeiro.
pub const ENSEMBLES: [Modelo; 3] = [
    Modelo {
        chave: "ecmwf",
        api: "ecmwf_ifs025",
        nome: "ECMWF ENS, 51 membros",
        dias: 15,
        passo: 0.25,
        autor: "ECMWF (CC BY 4.0)",
        membros: 51,
    },
    Modelo {
        chave: "aifs",
        api: "ecmwf_aifs025",
        nome: "ECMWF AIFS-ENS, 51 membros",
        dias: 15,
        passo: 0.25,
        autor: "ECMWF (CC BY 4.0)",
        membros: 51,
    },
    Modelo {
        chave: "gfs",
        api: "gfs05",
        nome: "GEFS, 31 membros",
        dias: 16,
        passo: 0.5,
        autor: "NOAA/NCEP (domínio público)",
        membros: 31,
    },
];

/// As escolhas da janela dos modelos determinísticos e o `modelo` de cada uma.
pub const ESCOLHAS_DETERMINISTICOS: [(&str, &str); 5] = [
    ("ECMWF IFS (15 dias)", "ecmwf"),
    ("ECMWF AIFS, de IA (15 dias)", "aifs"),
    ("GFS da NOAA (16 dias)", "gfs"),
    ("ICON do DWD (7 dias)", "icon"),
    ("O melhor do Open-Meteo para o lugar (16 dias)", "melhor"),
];

/// As escolhas da janela dos ensembles e o `modelo` de cada uma.
pub const ESCOLHAS_ENSEMBLES: [(&str, &str); 3] = [
    ("ECMWF ENS, 51 membros (15 dias)", "ecmwf"),
    ("ECMWF AIFS-ENS, de IA, 51 membros (15 dias)", "aifs"),
    ("GEFS da NOAA, 31 membros (16 dias)", "gfs"),
];

/// O que mostrar do ensemble.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Medida {
    /// A chance (%) de a chuva passar do limiar.
    Chance,
    /// A média dos membros (mm).
    Media,
    /// O percentil 10: 9 em 10 membros dão mais que isso (mm).
    P10,
    /// O percentil 90: só 1 em 10 membros dá mais que isso (mm).
    P90,
}

/// As escolhas da janela da medida do ensemble e a `medida` de cada uma.
pub const ESCOLHAS_MEDIDAS: [(&str, &str); 4] = [
    ("Chance de passar do limiar (%)", "chance"),
    ("Média dos membros (mm)", "media"),
    ("Cenário seco: percentil 10 (mm)", "p10"),
    ("Cenário chuvoso: percentil 90 (mm)", "p90"),
];

/// A temperatura.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Temperatura {
    Maxima,
    Minima,
    Media,
}

/// As escolhas da janela da temperatura e o `tipo` de cada uma.
pub const ESCOLHAS_TEMPERATURAS: [(&str, &str); 3] =
    [("Máxima do dia (média dos dias)", "maxima"), ("Mínima do dia (média dos dias)", "minima"), ("Média do dia", "media")];

impl Temperatura {
    /// A variável diária do Open-Meteo.
    pub fn variavel(self) -> &'static str {
        match self {
            Temperatura::Maxima => "temperature_2m_max",
            Temperatura::Minima => "temperature_2m_min",
            Temperatura::Media => "temperature_2m_mean",
        }
    }

    /// O nome, para a camada.
    pub fn nome(self) -> &'static str {
        match self {
            Temperatura::Maxima => "máxima",
            Temperatura::Minima => "mínima",
            Temperatura::Media => "média",
        }
    }
}

/// O modelo de `chave` em `lista` (sem chave, o primeiro).
pub fn modelo<'a>(lista: &'a [Modelo], chave: Option<&str>) -> Result<&'a Modelo, String> {
    match chave {
        None => Ok(&lista[0]),
        Some(k) => lista
            .iter()
            .find(|m| m.chave == k.trim().to_lowercase())
            .ok_or_else(|| format!("o modelo é um de: {}; não '{k}'", lista.iter().map(|m| m.chave).collect::<Vec<_>>().join(", "))),
    }
}

/// A medida do ensemble (sem ela, a chance).
pub fn medida(texto: Option<&str>) -> Result<Medida, String> {
    match texto.map(|t| t.trim().to_lowercase()).as_deref() {
        None | Some("chance") => Ok(Medida::Chance),
        Some("media" | "média") => Ok(Medida::Media),
        Some("p10") => Ok(Medida::P10),
        Some("p90") => Ok(Medida::P90),
        Some(t) => Err(format!("a medida é chance, media, p10 ou p90; não '{t}'")),
    }
}

/// O tipo da temperatura (sem ele, a máxima).
pub fn temperatura(texto: Option<&str>) -> Result<Temperatura, String> {
    match texto.map(|t| t.trim().to_lowercase()).as_deref() {
        None | Some("maxima" | "máxima") => Ok(Temperatura::Maxima),
        Some("minima" | "mínima") => Ok(Temperatura::Minima),
        Some("media" | "média") => Ok(Temperatura::Media),
        Some(t) => Err(format!("o tipo é maxima, minima ou media; não '{t}'")),
    }
}

/// Os dias pedidos, de 1 até os que o modelo prevê; sem eles, `padrao` (cortado no do modelo).
/// O segundo valor diz se o pedido foi cortado.
pub fn dias(pedido: Option<f64>, padrao: usize, m: &Modelo) -> Result<(usize, bool), String> {
    match pedido {
        None => Ok((padrao.min(m.dias), padrao > m.dias)),
        Some(d) if d.fract() == 0.0 && (1.0..=16.0).contains(&d) => {
            let d = d as usize;
            Ok((d.min(m.dias), d > m.dias))
        }
        Some(d) => Err(format!("os dias vão de 1 a 16, não {d}")),
    }
}

/// O limiar da chance, em mm (sem ele, 100).
pub fn limiar(pedido: Option<f64>) -> Result<f64, String> {
    match pedido {
        None => Ok(LIMIAR_PADRAO),
        Some(l) if l > 0.0 && l <= 2000.0 => Ok(l),
        Some(l) => Err(format!("o limiar vai de 0 a 2.000 mm, não {l}")),
    }
}

pub const LIMIAR_PADRAO: f64 = 100.0;
pub const DIAS_CHUVA: usize = 15;
pub const DIAS_TEMPERATURA: usize = 7;

/// O máximo de pontos de uma previsão determinística.
pub const MAX_PONTOS: usize = 400;

/// O máximo de pontos de um ensemble: até 100 (o ensemble pesa cerca de 5,5 pedidos por ponto na
/// conta do Open-Meteo) e até uns 3.000 pontos vezes membros, o que o orçamento de processamento
/// de um clique do plugin aguenta ler (o GEFS, 31 membros, passou com 90 pontos; o ECMWF ENS, 51,
/// não). Dá 58 pontos no ECMWF e 96 no GEFS.
pub fn max_pontos_ensemble(m: &Modelo) -> usize {
    (3000 / m.membros.max(1)).min(100)
}
/// Os pontos por endereço (o endereço fica com uns 3 kB). O ensemble vai em grupos menores: o
/// Open-Meteo leva uns 2 s para 10 pontos, mas às vezes mais de 30 s para 25, e o RustGIS espera
/// até 20 s por pedido e 60 s por clique.
const POR_PEDIDO: usize = 100;
const POR_PEDIDO_ENSEMBLE: usize = 10;

/// Os nós da grade de passo `passo` (múltiplos dele) que cercam o retângulo `c` (`[oeste, sul,
/// leste, norte]`, em graus): o primeiro abaixo e o primeiro acima de cada lado, de modo que a
/// interpolação bilinear entre eles cubra a área toda. O passo dobra até caberem `max` nós.
/// Devolve o passo usado e os nós, de oeste para leste e de sul para norte.
pub fn pontos(c: [f64; 4], passo: f64, max: usize) -> (f64, Vec<[f64; 2]>) {
    let mut p = passo;
    loop {
        let (x0, x1) = ((c[0] / p).floor() as i64, (c[2] / p).ceil() as i64);
        let (y0, y1) = ((c[1] / p).floor() as i64, (c[3] / p).ceil() as i64);
        let n = ((x1 - x0 + 1) * (y1 - y0 + 1)) as usize;
        if n <= max {
            let mut v = Vec::with_capacity(n);
            for j in y0..=y1 {
                for i in x0..=x1 {
                    v.push([i as f64 * p, j as f64 * p]);
                }
            }
            return (p, v);
        }
        p *= 2.0;
    }
}

/// Um número curto para o endereço: até 4 casas, sem zeros à direita.
fn curto(v: f64) -> String {
    let t = format!("{v:.4}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" { "0".into() } else { t.into() }
}

/// Os endereços da previsão de `variaveis` diárias dos `pts`, em grupos.
fn enderecos(base: &str, m: &Modelo, variaveis: &str, dias: usize, pts: &[[f64; 2]], por_pedido: usize) -> Vec<String> {
    pts.chunks(por_pedido)
        .map(|g| {
            let lat: Vec<String> = g.iter().map(|p| curto(p[1])).collect();
            let lon: Vec<String> = g.iter().map(|p| curto(p[0])).collect();
            format!(
                "{base}?latitude={}&longitude={}&daily={variaveis}&forecast_days={dias}&models={}&timezone=America%2FSao_Paulo",
                lat.join(","),
                lon.join(","),
                m.api
            )
        })
        .collect()
}

/// Os endereços da previsão determinística.
pub fn pedidos(m: &Modelo, variavel: &str, dias: usize, pts: &[[f64; 2]]) -> Vec<String> {
    enderecos("https://api.open-meteo.com/v1/forecast", m, variavel, dias, pts, POR_PEDIDO)
}

/// Os endereços da chuva do ensemble.
pub fn pedidos_ensemble(m: &Modelo, dias: usize, pts: &[[f64; 2]]) -> Vec<String> {
    enderecos("https://ensemble-api.open-meteo.com/v1/ensemble", m, "precipitation_sum", dias, pts, POR_PEDIDO_ENSEMBLE)
}

/// O erro de uma resposta do Open-Meteo (`{"error": true, "reason": ...}`).
pub fn erro(resposta: &Value) -> Option<String> {
    if resposta["error"].as_bool() != Some(true) {
        return None;
    }
    let motivo = resposta["reason"].as_str().unwrap_or("sem motivo");
    Some(if motivo.to_lowercase().contains("limit") {
        format!("o Open-Meteo recusou por excesso de pedidos ({motivo}): espere um minuto, ou use uma área menor")
    } else {
        format!("o Open-Meteo recusou o pedido: {motivo}")
    })
}

/// Os lugares das respostas, na ordem dos pontos (um pedido de um ponto só volta como objeto).
pub fn lugares(respostas: &[Value]) -> Vec<&Value> {
    respostas
        .iter()
        .flat_map(|r| match r.as_array() {
            Some(a) => a.iter().collect::<Vec<_>>(),
            None => vec![r],
        })
        .collect()
}

/// Os `dias` primeiros valores diários de `variavel` de um lugar, ou `None` se falta algum.
fn diarios(lugar: &Value, variavel: &str, dias: usize) -> Option<Vec<f64>> {
    let v = lugar["daily"][variavel].as_array()?;
    if v.len() < dias {
        return None;
    }
    v[..dias].iter().map(Value::as_f64).collect()
}

/// Quantos dos `dias` primeiros dias de `variavel` vêm completos, no lugar que tem mais. O último
/// dia de uma rodada pode vir em `null`: a rodada das 00 UTC do ECMWF vai até 360 h, e o 15º dia no
/// horário de Brasília fica sem as 3 últimas horas.
pub fn dias_completos(lugares: &[Value], variavel: &str, dias: usize) -> usize {
    lugares
        .iter()
        .filter_map(|l| l["daily"][variavel].as_array())
        .map(|v| v.iter().take(dias).take_while(|x| x.is_number()).count())
        .max()
        .unwrap_or(0)
}

/// A chuva somada nos `dias` primeiros dias, em mm.
pub fn soma(lugar: &Value, variavel: &str, dias: usize) -> Option<f64> {
    diarios(lugar, variavel, dias).map(|v| v.iter().sum())
}

/// A média dos `dias` primeiros valores diários.
pub fn media(lugar: &Value, variavel: &str, dias: usize) -> Option<f64> {
    diarios(lugar, variavel, dias).map(|v| v.iter().sum::<f64>() / v.len() as f64)
}

/// A chuva somada de cada membro do ensemble (o controle, `precipitation_sum`, e os
/// `precipitation_sum_memberNN`), sem os membros com dia faltando.
pub fn membros(lugar: &Value, dias: usize) -> Vec<f64> {
    let Some(d) = lugar["daily"].as_object() else { return Vec::new() };
    d.keys()
        .filter(|k| *k == "precipitation_sum" || k.starts_with("precipitation_sum_member"))
        .filter_map(|k| soma(lugar, k, dias))
        .collect()
}

/// O percentil `p` (0 a 100) de `v`, por interpolação linear entre os valores ordenados.
fn percentil(v: &[f64], p: f64) -> f64 {
    let mut s = v.to_vec();
    s.sort_by(|a, b| a.total_cmp(b));
    let pos = p / 100.0 * (s.len() - 1) as f64;
    let (i, f) = (pos.floor() as usize, pos.fract());
    if i + 1 < s.len() { s[i] + f * (s[i + 1] - s[i]) } else { s[i] }
}

/// A medida dos membros: a chance (%) de passar de `limiar`, a média ou um percentil (mm).
pub fn resumir(v: &[f64], medida: Medida, limiar: f64) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    Some(match medida {
        Medida::Chance => v.iter().filter(|x| **x > limiar).count() as f64 / v.len() as f64 * 100.0,
        Medida::Media => v.iter().sum::<f64>() / v.len() as f64,
        Medida::P10 => percentil(v, 10.0),
        Medida::P90 => percentil(v, 90.0),
    })
}

/// O primeiro e o último dia da previsão de `dias` dias, "dd/mm", da resposta.
pub fn periodo(lugar: &Value, dias: usize) -> Option<(String, String)> {
    let t = lugar["daily"]["time"].as_array()?;
    let dm = |v: &Value| {
        let s = v.as_str()?;
        Some(format!("{}/{}", s.get(8..10)?, s.get(5..7)?))
    };
    Some((dm(t.first()?)?, dm(t.get(dias.checked_sub(1)?)?)?))
}

/// O campo das células.
pub const CAMPO: &str = "valor";

/// As células de `passo` graus em volta dos pontos, com os valores que existem.
pub fn celulas(pts: &[[f64; 2]], valores: &[Option<f64>], passo: f64) -> Value {
    let h = passo / 2.0;
    let feicoes: Vec<Value> = pts
        .iter()
        .zip(valores)
        .filter_map(|(p, v)| {
            let v = (*v)?;
            let (x0, y0, x1, y1) = (p[0] - h, p[1] - h, p[0] + h, p[1] + h);
            Some(json!({"type": "Feature",
                "geometry": {"type": "Polygon", "coordinates": [[[x0, y0], [x1, y0], [x1, y1], [x0, y1], [x0, y0]]]},
                "properties": {CAMPO: v}}))
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// As camadas de passagem.
pub const CELULAS: &str = "BrasilData: previsão em células";
pub const RASTER: &str = "BrasilData: previsão em raster";
pub const NA_GRADE: &str = "BrasilData: previsão na grade";

/// O começo dos nomes das camadas de cada comando.
pub const CHUVA: &str = "BrasilData: Chuva prevista";
pub const CHANCE: &str = "BrasilData: Chance de chuva";
pub const ENSEMBLE: &str = "BrasilData: Chuva do ensemble";
pub const TEMPERATURA: &str = "BrasilData: Temperatura";

/// O nome da chuva prevista.
pub fn camada_chuva(m: &Modelo, dias: usize, de: &str, ate: &str) -> String {
    format!("{CHUVA} em {dias} dia{} ({}, {de} a {ate})", if dias == 1 { "" } else { "s" }, m.nome)
}

/// O nome da camada do ensemble.
pub fn camada_ensemble(m: &Modelo, medida: Medida, limiar: f64, dias: usize, de: &str, ate: &str) -> String {
    let d = format!("{dias} dia{}", if dias == 1 { "" } else { "s" });
    match medida {
        Medida::Chance => format!("{CHANCE} acima de {} mm em {d} ({}, {de} a {ate})", curto(limiar), m.nome),
        Medida::Media => format!("{ENSEMBLE} em {d}: média ({}, {de} a {ate})", m.nome),
        Medida::P10 => format!("{ENSEMBLE} em {d}: cenário seco, P10 ({}, {de} a {ate})", m.nome),
        Medida::P90 => format!("{ENSEMBLE} em {d}: cenário chuvoso, P90 ({}, {de} a {ate})", m.nome),
    }
}

/// O nome da temperatura prevista.
pub fn camada_temperatura(t: Temperatura, m: &Modelo, dias: usize, de: &str, ate: &str) -> String {
    let d = if dias == 1 { "1 dia".to_string() } else { format!("{dias} dias") };
    format!("{TEMPERATURA} {} prevista em {d} ({}, {de} a {ate})", t.nome(), m.nome)
}

/// Se a camada `nome` é uma previsão do Open-Meteo.
pub fn eh_da_previsao(nome: &str) -> bool {
    [CHUVA, CHANCE, ENSEMBLE].iter().any(|p| nome.starts_with(p)) || (nome.starts_with(TEMPERATURA) && nome.contains(" prevista em "))
}

/// A escala da chuva de `dias` dias, em mm: a cor satura no máximo.
pub fn escala_chuva(dias: usize) -> f64 {
    match dias {
        0..=3 => 75.0,
        4..=7 => 150.0,
        8..=10 => 200.0,
        _ => 300.0,
    }
}

/// Os dias de uma camada de previsão ("… em 15 dias (…").
pub fn dias_da_camada(nome: &str) -> Option<usize> {
    let resto = &nome[nome.find(" em ")? + 4..];
    resto.split(' ').next()?.parse().ok()
}

/// Quantos valores a barra de cores da chuva mostra, as pontas incluídas, para marcas redondas.
pub fn marcas_da_chuva(max: f64) -> usize {
    match max as u32 {
        75 => 4,
        200 | 400 => 5,
        _ => 7,
    }
}

/// As cores da chuva, do branco ao roxo, em frações da escala.
const CORES_CHUVA: [(f64, &str); 12] = [
    (0.0, "245 245 250"),
    (0.04, "200 225 245"),
    (0.1, "120 180 235"),
    (0.2, "45 120 210"),
    (0.3, "30 160 70"),
    (0.4, "190 225 60"),
    (0.5, "245 230 60"),
    (0.6, "245 160 45"),
    (0.7, "230 80 40"),
    (0.8, "190 30 60"),
    (0.9, "125 25 120"),
    (1.0, "200 140 230"),
];

/// Uma rampa do GDAL (`valor r g b` por linha) com as `cores` em frações de `max`.
fn rampa_de(cores: &[(f64, &str)], max: f64) -> String {
    cores.iter().map(|(f, c)| format!("{} {c}", curto(f * max))).collect::<Vec<_>>().join("\n")
}

/// O nome e o texto da rampa da chuva de 0 a `max` mm.
pub fn rampa_chuva(max: f64) -> (String, String) {
    (format!("BrasilData: chuva 0–{} mm", curto(max)), rampa_de(&CORES_CHUVA, max))
}

/// A rampa da chance, de 0 a 100%.
pub const RAMPA_CHANCE: (&str, &str) = (
    "BrasilData: chance de 0 a 100%",
    "0 247 247 247\n10 253 240 196\n30 244 184 96\n50 226 96 58\n70 163 36 90\n90 59 47 143\n100 30 42 110",
);

/// A rampa da temperatura, de 0 a 40 °C.
pub const RAMPA_TEMPERATURA: (&str, &str) = (
    "BrasilData: temperatura 0–40 °C",
    "0 49 54 149\n8 69 117 180\n14 116 173 209\n18 171 217 233\n22 254 224 144\n26 253 174 97\n30 244 109 67\n34 215 48 39\n40 165 0 38",
);
pub const ESCALA_TEMPERATURA: (f64, f64) = (0.0, 40.0);

/// A citação da previsão do modelo `m`.
pub fn citacao(m: &Modelo) -> String {
    format!(
        "Open-Meteo (Zippenfenig, P. 2023. Open-Meteo.com Weather API, doi:10.5281/zenodo.7970649), modelo {} de {}: valores diários nos nós de uma grade em volta da área, interpolados para a grade do projeto e recortados pela área",
        m.nome, m.autor
    )
}

/// A citação de uma camada de previsão pelo nome (o modelo vem entre parênteses).
pub fn citacao_da_camada(nome: &str) -> String {
    let m = DETERMINISTICOS.iter().chain(ENSEMBLES.iter()).find(|m| nome.contains(&format!("({},", m.nome)));
    match m {
        Some(m) => citacao(m),
        None => "Open-Meteo (Zippenfenig, P. 2023. Open-Meteo.com Weather API, doi:10.5281/zenodo.7970649), com os modelos do ECMWF, da NOAA e do DWD".into(),
    }
}

pub const LICENCA: &str = "CC BY 4.0 (Open-Meteo; dados do ECMWF e do DWD em CC BY 4.0, da NOAA em domínio público); a API gratuita do Open-Meteo é para uso não comercial";

// ---------------------------------------------------------------- CHIRPS

/// A pasta dos totais mensais do CHIRPS v3 em COG.
pub const PASTA_CHIRPS: &str = "https://data.chc.ucsb.edu/products/CHIRPS/v3.0/monthly/global/cogs/";

/// O endereço do mês.
pub fn url_chirps(ano: i64, mes: u32) -> String {
    format!("{PASTA_CHIRPS}chirps-v3.0.{ano}.{mes:02}.cog")
}

/// O mês mais recente na listagem da pasta (`chirps-v3.0.AAAA.MM.cog`).
pub fn ultimo_mes(listagem: &str) -> Option<(i64, u32)> {
    listagem
        .match_indices("chirps-v3.0.")
        .filter_map(|(i, _)| {
            let r = listagem.get(i + 12..i + 23)?;
            let (a, resto) = r.split_once('.')?;
            let (m, ext) = resto.split_once('.')?;
            (a.len() == 4 && m.len() == 2 && ext.starts_with("cog")).then_some(())?;
            Some((a.parse().ok()?, m.parse().ok()?))
        })
        .filter(|&(_, m): &(i64, u32)| (1..=12).contains(&m))
        .max()
}

/// O mês escrito "mm/aaaa" (ou "m/aaaa").
pub fn ler_mes(texto: &str) -> Option<(i64, u32)> {
    let (m, a) = texto.trim().split_once('/')?;
    let (m, a) = (m.trim().parse::<u32>().ok()?, a.trim());
    (a.len() == 4 && (1..=12).contains(&m)).then_some((a.parse().ok()?, m))
}

const NOMES_DOS_MESES: [&str; 12] =
    ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

/// O nome da camada do mês.
pub fn camada_chirps(ano: i64, mes: u32) -> String {
    format!("BrasilData: Chuva de {} de {ano} (CHIRPS v3)", NOMES_DOS_MESES[(mes - 1) as usize])
}

/// Se a camada `nome` é do CHIRPS.
pub fn eh_do_chirps(nome: &str) -> bool {
    nome.starts_with("BrasilData: Chuva de ") && nome.ends_with(" (CHIRPS v3)")
}

/// A camada de passagem com a janela lida do mês.
pub fn janela_chirps(ano: i64, mes: u32) -> String {
    format!("BrasilData: CHIRPS {ano}-{mes:02} (janela)")
}
pub const NA_GRADE_CHIRPS: &str = "BrasilData: CHIRPS na grade";

/// O lado da célula do CHIRPS, em graus e em metros no equador.
pub const LADO_CHIRPS: f64 = 0.05;
pub const LADO_CHIRPS_M: f64 = LADO_CHIRPS * 111_320.0;

/// A janela: o retângulo `c` com duas células de margem.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    let m = 2.0 * LADO_CHIRPS;
    [c[0] - m, c[1] - m, c[2] + m, c[3] + m]
}

/// A reamostragem para células de `celula` metros.
pub fn reamostragem_chirps(celula: f64) -> &'static str {
    if celula > LADO_CHIRPS_M { "average" } else { "bilinear" }
}

/// A escala da chuva do mês, em mm.
pub const ESCALA_CHIRPS: f64 = 400.0;

pub const CITACAO_CHIRPS: &str = "Climate Hazards Center (UC Santa Barbara), CHIRPS v3.0: precipitação mensal por satélite e estações, 0,05° (Funk, C. et al. 2015. The climate hazards infrared precipitation with stations, a new environmental record for monitoring extremes. Scientific Data 2, 150066, doi:10.1038/sdata.2015.66); posto na grade do projeto e recortado pela área";
pub const LICENCA_CHIRPS: &str =
    "licença não declarada no README da v3; o Climate Hazards Center distribui o CHIRPS como domínio público (CC0), com a citação";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_pontos_cobrem_a_area_com_margem_e_o_passo_dobra_numa_area_grande() {
        // Viçosa: um retângulo pequeno vira 2 × 3 nós de 0,25°, que o cercam.
        let (p, v) = pontos([-43.0, -20.85, -42.75, -20.62], 0.25, MAX_PONTOS);
        assert_eq!(p, 0.25);
        assert_eq!(v.len(), 6);
        assert_eq!(v[0], [-43.0, -21.0]);
        assert_eq!(v[5], [-42.75, -20.5]);
        // Minas Gerais não cabe em 400 nós de 0,25°: o passo dobra.
        let (p, v) = pontos([-51.05, -22.93, -39.85, -14.23], 0.25, MAX_PONTOS);
        assert_eq!(p, 1.0);
        assert!(v.len() <= MAX_PONTOS, "{}", v.len());
        let ens = modelo(&ENSEMBLES, Some("ecmwf")).unwrap();
        assert_eq!(max_pontos_ensemble(ens), 58);
        assert_eq!(max_pontos_ensemble(modelo(&ENSEMBLES, Some("gfs")).unwrap()), 96);
        let (p2, v2) = pontos([-51.05, -22.93, -39.85, -14.23], 0.25, max_pontos_ensemble(ens));
        assert_eq!(p2, 2.0);
        assert!(v2.len() <= 58);
        // A Zona da Mata cabe no ECMWF ENS a 0,5°.
        let (p3, v3) = pontos([-44.223, -22.233, -41.411, -19.828], 0.25, max_pontos_ensemble(ens));
        assert_eq!(p3, 0.5);
        assert_eq!(v3.len(), 8 * 7);
    }

    #[test]
    fn os_enderecos_vao_em_grupos_com_o_modelo_e_o_fuso() {
        let m = modelo(&DETERMINISTICOS, Some("gfs")).unwrap();
        let pts: Vec<[f64; 2]> = (0..250).map(|i| [-43.0 + i as f64 * 0.25, -20.5]).collect();
        let u = pedidos(m, "precipitation_sum", 16, &pts);
        assert_eq!(u.len(), 3);
        assert!(u[0].starts_with("https://api.open-meteo.com/v1/forecast?latitude=-20.5,-20.5,"), "{}", u[0]);
        assert!(u[0].contains("&longitude=-43,-42.75,-42.5,"));
        assert!(u[0].ends_with("&daily=precipitation_sum&forecast_days=16&models=gfs_seamless&timezone=America%2FSao_Paulo"));
        let e = modelo(&ENSEMBLES, None).unwrap();
        let u = pedidos_ensemble(e, 15, &pts);
        assert_eq!(u.len(), 25);
        assert!(u[0].starts_with("https://ensemble-api.open-meteo.com/v1/ensemble?") && u[0].contains("models=ecmwf_ifs025"));
        assert_eq!(curto(-0.0), "0");
        assert_eq!(curto(150.0), "150");
    }

    #[test]
    fn os_parametros_tem_padrao_e_limites() {
        let ecmwf = modelo(&DETERMINISTICOS, None).unwrap();
        assert_eq!(ecmwf.chave, "ecmwf");
        assert!(modelo(&DETERMINISTICOS, Some("xyz")).is_err());
        assert_eq!(dias(None, 15, ecmwf), Ok((15, false)));
        assert_eq!(dias(Some(16.0), 15, ecmwf), Ok((15, true)));
        assert_eq!(dias(Some(3.0), 15, ecmwf), Ok((3, false)));
        assert!(dias(Some(0.0), 15, ecmwf).is_err() && dias(Some(2.5), 15, ecmwf).is_err());
        let icon = modelo(&DETERMINISTICOS, Some("ICON")).unwrap();
        assert_eq!(dias(None, 15, icon), Ok((7, true)));
        assert_eq!(medida(None), Ok(Medida::Chance));
        assert_eq!(medida(Some("P90")), Ok(Medida::P90));
        assert!(medida(Some("x")).is_err());
        assert_eq!(temperatura(Some("mínima")), Ok(Temperatura::Minima));
        assert_eq!(limiar(None), Ok(100.0));
        assert!(limiar(Some(0.0)).is_err());
    }

    fn lugar(chuva: &[f64]) -> Value {
        let tempo: Vec<String> = (0..chuva.len()).map(|i| format!("2026-11-{:02}", 14 + i)).collect();
        json!({"daily": {"time": tempo, "precipitation_sum": chuva, "precipitation_sum_member01": chuva.iter().map(|x| x * 2.0).collect::<Vec<_>>(),
            "precipitation_sum_member02": [null, 1.0, 1.0], "temperature_2m_max": [30.0, 28.0, 26.0]}})
    }

    #[test]
    fn os_valores_somam_os_dias_e_os_membros_resumem() {
        let l = lugar(&[4.0, 6.0, 10.0]);
        assert_eq!(soma(&l, "precipitation_sum", 2), Some(10.0));
        assert_eq!(soma(&l, "precipitation_sum", 4), None);
        assert_eq!(media(&l, "temperature_2m_max", 3), Some(28.0));
        let mut m = membros(&l, 3);
        m.sort_by(f64::total_cmp);
        assert_eq!(m, vec![20.0, 40.0]); // o membro 02 tem um dia faltando
        assert_eq!(resumir(&[10.0, 50.0, 120.0, 200.0], Medida::Chance, 100.0), Some(50.0));
        assert_eq!(resumir(&[10.0, 20.0], Medida::Media, 0.0), Some(15.0));
        assert_eq!(resumir(&[0.0, 10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0], Medida::P10, 0.0), Some(10.0));
        assert_eq!(resumir(&[], Medida::P90, 0.0), None);
        assert_eq!(periodo(&l, 3), Some(("14/11".into(), "16/11".into())));
        let incompleto = json!({"daily": {"precipitation_sum": [1.0, 2.0, null]}});
        assert_eq!(dias_completos(&[incompleto.clone(), l.clone()], "precipitation_sum", 3), 3);
        assert_eq!(dias_completos(&[incompleto], "precipitation_sum", 3), 2);
        let r = [json!([lugar(&[1.0]), lugar(&[2.0])]), lugar(&[3.0])];
        assert_eq!(lugares(&r).len(), 3);
        assert!(erro(&json!({"error": true, "reason": "Minutely API request limit exceeded"})).unwrap().contains("espere um minuto"));
        assert!(erro(&l).is_none());
    }

    #[test]
    fn as_celulas_ficam_em_volta_dos_pontos_sem_os_vazios() {
        let fc = celulas(&[[-43.0, -20.5], [-42.75, -20.5]], &[Some(12.5), None], 0.25);
        let f = fc["features"].as_array().unwrap();
        assert_eq!(f.len(), 1);
        assert_eq!(f[0]["geometry"]["coordinates"][0][0], json!([-43.125, -20.625]));
        assert_eq!(f[0]["properties"][CAMPO], 12.5);
    }

    #[test]
    fn os_nomes_dizem_o_que_e_e_a_citacao_vem_do_nome() {
        let m = modelo(&DETERMINISTICOS, Some("gfs")).unwrap();
        let n = camada_chuva(m, 15, "14/11", "28/11");
        assert_eq!(n, "BrasilData: Chuva prevista em 15 dias (GFS, 14/11 a 28/11)");
        assert!(eh_da_previsao(&n) && citacao_da_camada(&n).contains("modelo GFS de NOAA"));
        let e = modelo(&ENSEMBLES, Some("ecmwf")).unwrap();
        let c = camada_ensemble(e, Medida::Chance, 100.0, 15, "14/11", "28/11");
        assert_eq!(c, "BrasilData: Chance de chuva acima de 100 mm em 15 dias (ECMWF ENS, 51 membros, 14/11 a 28/11)");
        assert!(eh_da_previsao(&c) && citacao_da_camada(&c).contains("ECMWF ENS"));
        let t = camada_temperatura(Temperatura::Maxima, m, 7, "14/11", "20/11");
        assert!(eh_da_previsao(&t) && t.starts_with("BrasilData: Temperatura máxima prevista em 7 dias"));
        assert!(!eh_da_previsao("BrasilData: Radiação solar (NASA POWER)"));
        let (nome, texto) = rampa_chuva(300.0);
        assert_eq!(nome, "BrasilData: chuva 0–300 mm");
        assert!(texto.starts_with("0 245 245 250\n12 200 225 245") && texto.ends_with("300 200 140 230"));
        assert_eq!(escala_chuva(1), 75.0);
        assert_eq!(escala_chuva(15), 300.0);
        assert_eq!(dias_da_camada(&n), Some(15));
        assert_eq!(dias_da_camada(&c), Some(15));
        assert_eq!(marcas_da_chuva(75.0), 4);
        assert_eq!(marcas_da_chuva(150.0), 7);
    }

    #[test]
    fn o_chirps_acha_o_ultimo_mes_e_le_o_mes_pedido() {
        let l = r#"<a href="chirps-v3.0.2026.09.cog">x</a> <a href="chirps-v3.0.2026.10.cog">y</a> <a href="chirps-v3.0.2025.12.cog">"#;
        assert_eq!(ultimo_mes(l), Some((2026, 10)));
        assert_eq!(ultimo_mes("nada"), None);
        assert_eq!(ler_mes("10/2026"), Some((2026, 10)));
        assert_eq!(ler_mes("3/2025"), Some((2025, 3)));
        assert_eq!(ler_mes("13/2025"), None);
        assert_eq!(url_chirps(2026, 3), "https://data.chc.ucsb.edu/products/CHIRPS/v3.0/monthly/global/cogs/chirps-v3.0.2026.03.cog");
        let n = camada_chirps(2026, 10);
        assert_eq!(n, "BrasilData: Chuva de outubro de 2026 (CHIRPS v3)");
        assert!(eh_do_chirps(&n));
    }
}
