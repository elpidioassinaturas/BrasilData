//! O radar de banda L do NISAR (NASA e ISRO): o GCOV, o retroespalhamento corrigido do relevo, em
//! grade de 20 m, distribuído pelo ASF DAAC.
//!
//! - **De onde:** a busca do CMR (`cmr.earthdata.nasa.gov`, aberta e com CORS) lista as cenas
//!   (*frames*, cerca de 240 × 220 km) que cruzam o retângulo da área, as mais recentes primeiro.
//! - **Formato:** cada cena é um HDF5 de 2 a 6 GB, que o plugin não lê. Quem lê é o Harmony da
//!   NASA (`harmony.earthdata.nasa.gov`): ele tira do HDF5 só uma polarização, recorta pelo
//!   retângulo da área, reprojeta para o fuso UTM e devolve um GeoTIFF (cerca de 6 MB para
//!   Viçosa, em 40 s). O pedido é síncrono, uma cena por vez.
//! - **Login:** o Harmony pede o token Earthdata Login, que o RustGIS guarda e manda aos hosts
//!   `*.earthdata.nasa.gov`; o plugin só passa o endereço ao `layer.addUrl`.
//! - **Os valores:** o gama0 (potência linear) da frequência A (20 MHz). Vai para dB e passa pelo filtro
//!   sigma de Lee no dB ([`SIGMA`]), que guarda as bordas; a razão HV/HH, em dB, é HV − HH.

use serde_json::{Value, json};

use crate::datas::Data;
use crate::radar::data_de;

/// A coleção do GCOV no CMR (NISAR_L2_GCOV_V1).
pub const COLECAO: &str = "C3622241360-ASF";

/// O Harmony da NASA.
const HARMONY: &str = "https://harmony.earthdata.nasa.gov";

/// Quantas cenas a busca devolve: umas 25 passagens sobre uma área pequena (cerca de 60 KB).
pub const MAX_BUSCA: usize = 100;

/// Mais cenas que isto numa passagem é área grande demais.
pub const MAX_CENAS: usize = 4;

/// O lado maior do retângulo da área, em graus, que o NISAR aceita (cerca de 110 km): a 20 m são
/// até 5.500 × 5.500 células por polarização, uns 120 MB pelo Harmony.
pub const MAX_GRAUS: f64 = 1.0;

/// A célula do GCOV, em metros.
pub const CELULA: f64 = 20.0;

/// Uma passagem vale se cobre a área com pelo menos esta fração das cenas da melhor.
const QUASE_TANTO: f64 = 0.8;

/// A citação do GCOV.
pub const CITACAO: &str = "NISAR Geocoded Polarimetric Covariance Product, version 1 (NISAR_L2_GCOV_V1), NASA/JPL e ISRO, distribuído pelo ASF DAAC (doi:10.5067/NIL2GCOV-V1); recorte e reprojeção pelo Harmony (NASA Earthdata)";

/// A licença dele.
pub const LICENCA: &str = "dados da NASA de uso livre e aberto, com citação (NASA ESDIS Data Use and Citation Guidance)";

/// Uma polarização (do GCOV de dupla polarização H: HH e HV).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pol {
    Hh,
    Hv,
}

/// A rampa de cores das camadas HH e HV, do roxo escuro (eco fraco: água, solo liso) ao amarelo
/// (eco forte), como nas imagens do NISAR da NASA.
pub const RAMPA: &str = "Magma";

/// O sigma do filtro de Lee, em dB: perto do desvio do granulado que sobra no GCOV em 20 m (cerca
/// de 2,2 dB numa janela 5 × 5 em Viçosa). No dB, o filtro só mistura vizinhos parecidos e guarda
/// as bordas; a média simples da janela (o filtro do Sentinel-1) borrava a imagem do NISAR.
/// O `lee_filter` do GeoLibre na potência linear vira essa média: opengeos/whitebox-wasm#66.
pub const SIGMA: f64 = 2.5;

/// A escala de cinza da razão HV/HH, em dB (HV − HH).
pub const ESCALA_DA_RAZAO: (f64, f64) = (-11.0, -1.0);

impl Pol {
    /// A escala de cinza, em dB, depois do filtro sigma de Lee: os percentis 2 e 98 de Viçosa em
    /// 16/11/2026, em 20 m (HH de −15,1 a −0,7; HV de −18,7 a −7,1; HV − HH de −10,6 a 1,5, onde
    /// o topo é granulado e encosta virada para o radar). Fixos, como no radar do Sentinel-1, para
    /// datas e lugares comparáveis.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Pol::Hh => (-15.0, -1.0),
            Pol::Hv => (-19.0, -7.0),
        }
    }

    /// "HH" ou "HV".
    pub fn sigla(self) -> &'static str {
        match self {
            Pol::Hh => "HH",
            Pol::Hv => "HV",
        }
    }

    /// A variável do HDF5, já codificada para o endereço do Harmony.
    fn variavel(self) -> &'static str {
        match self {
            Pol::Hh => "%2Fscience%2FLSAR%2FGCOV%2Fgrids%2FfrequencyA%2FHHHH",
            Pol::Hv => "%2Fscience%2FLSAR%2FGCOV%2Fgrids%2FfrequencyA%2FHVHV",
        }
    }
}

/// Uma cena do GCOV.
#[derive(Clone, Debug, PartialEq)]
pub struct Cena {
    /// O identificador do CMR (`G4409242572-ASF`), que o Harmony pede.
    pub id: String,
    /// O nome do produto (`NISAR_L2_PR_GCOV_034_117_A_167_…`).
    pub nome: String,
    /// O dia da passagem (UTC).
    pub data: Data,
    /// A órbita relativa (o "track").
    pub trilha: u32,
    /// O frame.
    pub frame: u32,
    /// O contorno (GeoJSON).
    pub contorno: Value,
}

impl Cena {
    /// A trilha e o frame, o dia e a hora do início, para o nome da camada dela:
    /// `T117-F167 20261101 084302`. Com a hora, dois pedaços do mesmo frame no mesmo dia (uma troca
    /// de modo do radar no meio dele) não dão camadas de mesmo nome.
    pub fn rotulo(&self) -> String {
        let hora = self.nome.split('_').nth(11).and_then(|t| t.split_once('T')).map(|(_, h)| format!(" {h}")).unwrap_or_default();
        format!("T{:03}-F{:03} {}{:02}{:02}{hora}", self.trilha, self.frame, self.data.ano, self.data.mes, self.data.dia)
    }

    /// A passagem: o dia e a trilha.
    pub fn passagem(&self) -> (Data, u32) {
        (self.data, self.trilha)
    }

    /// O pedido ao Harmony: a polarização `p` desta cena, recortada pelo retângulo `c` ([oeste,
    /// sul, leste, norte]) e no sistema `epsg` (um fuso UTM do WGS 84, que o Harmony aceita).
    pub fn url(&self, p: Pol, c: [f64; 4], epsg: u32) -> String {
        format!(
            "{HARMONY}/{COLECAO}/ogc-api-coverages/1.0.0/collections/parameter_vars/coverage/rangeset?granuleId={}&variable={}&subset=lat({:.4}:{:.4})&subset=lon({:.4}:{:.4})&outputCrs=EPSG:{epsg}&format=image%2Ftiff",
            self.id,
            p.variavel(),
            c[1],
            c[3],
            c[0],
            c[2]
        )
    }
}

/// A busca das cenas do GCOV que cruzam o retângulo `c`: de um dia, ou as mais recentes.
pub fn url_busca(c: [f64; 4], dia: Option<Data>) -> String {
    let mut u = format!(
        "https://cmr.earthdata.nasa.gov/search/granules.json?collection_concept_id={COLECAO}&bounding_box={:.4},{:.4},{:.4},{:.4}&sort_key=-start_date&page_size={MAX_BUSCA}",
        c[0], c[1], c[2], c[3]
    );
    if let Some(d) = dia {
        u.push_str(&format!("&temporal={}T00:00:00Z,{}T00:00:00Z", d.iso(), d.seguinte().iso()));
    }
    u
}

/// O polígono do CMR ("lat lon lat lon …"), em GeoJSON.
fn poligono(texto: &str) -> Option<Value> {
    let n: Vec<f64> = texto.split_whitespace().map(str::parse).collect::<Result<_, _>>().ok()?;
    if n.len() < 8 || !n.len().is_multiple_of(2) {
        return None;
    }
    let anel: Vec<Value> = n.chunks(2).map(|p| json!([p[1], p[0]])).collect();
    Some(json!({"type": "Polygon", "coordinates": [anel]}))
}

/// As cenas da resposta da busca do CMR de dupla polarização H ou quádrupla (as que têm HH e HV
/// na frequência A). O nome segue `NISAR_L2_<tipo>_GCOV_<ciclo>_<trilha>_<A|D>_<frame>_<modo>_
/// <polarizações>_…`; as polarizações começam pelas da frequência A (`DH`, `QP`...). A mesma
/// cena reprocessada fica uma vez: a primeira, que é a mais nova pela ordem da busca.
pub fn cenas(resposta: &Value) -> Vec<Cena> {
    let mut lista: Vec<Cena> = Vec::new();
    for e in resposta["feed"]["entry"].as_array().into_iter().flatten() {
        let (Some(id), Some(nome)) = (e["id"].as_str(), e["title"].as_str()) else {
            continue;
        };
        let partes: Vec<&str> = nome.split('_').collect();
        if partes.len() < 12 || partes[3] != "GCOV" {
            continue;
        }
        let pols = partes[9];
        if !(pols.starts_with("DH") || pols.starts_with("QP")) {
            continue;
        }
        let (Ok(trilha), Ok(frame)) = (partes[5].parse::<u32>(), partes[7].parse::<u32>()) else {
            continue;
        };
        let (Some(data), Some(contorno)) = (e["time_start"].as_str().and_then(data_de), e["polygons"][0][0].as_str().and_then(poligono))
        else {
            continue;
        };
        // A mesma aquisição (o início, no nome) de novo: outra versão dela.
        if lista.iter().any(|c| c.nome.split('_').nth(11) == Some(partes[11])) {
            continue;
        }
        lista.push(Cena { id: id.to_string(), nome: nome.to_string(), data, trilha, frame, contorno });
    }
    lista
}

/// A camada dos contornos, para a seleção pela área: o índice de cada cena em `i`.
pub fn camada_dos_contornos(lista: &[Cena]) -> Value {
    let feicoes: Vec<Value> = lista
        .iter()
        .enumerate()
        .map(|(i, c)| {
            json!({"type": "Feature", "geometry": c.contorno, "properties": {
                "i": i, "cena": c.nome, "data": c.data.texto(), "trilha": c.trilha,
            }})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// A passagem a usar, entre as cenas que cruzam a área: a mais recente com pelo menos 80% das
/// cenas da que tem mais (como no radar do Sentinel-1).
pub fn passagem(cruzam: &[&Cena]) -> Option<(Data, u32)> {
    let mut contagem: Vec<((Data, u32), usize)> = Vec::new();
    for c in cruzam {
        match contagem.iter_mut().find(|(p, _)| *p == c.passagem()) {
            Some((_, n)) => *n += 1,
            None => contagem.push((c.passagem(), 1)),
        }
    }
    let melhor = contagem.iter().map(|(_, n)| *n).max()?;
    contagem.into_iter().filter(|(_, n)| *n as f64 >= QUASE_TANTO * melhor as f64).map(|(p, _)| p).max_by_key(|(d, _)| *d)
}

/// As passagens da composição temporal: a de [`passagem`] e as anteriores da mesma trilha (a mesma
/// geometria de visada) que cobrem a área tão bem quanto ela, até `max`, da mais recente para a
/// mais antiga. A trilha se repete a cada 12 dias.
pub fn passagens(cruzam: &[&Cena], max: usize) -> Vec<(Data, u32)> {
    let Some(primeira) = passagem(cruzam) else { return Vec::new() };
    let mut contagem: Vec<((Data, u32), usize)> = Vec::new();
    for c in cruzam.iter().filter(|c| c.trilha == primeira.1) {
        match contagem.iter_mut().find(|(p, _)| *p == c.passagem()) {
            Some((_, n)) => *n += 1,
            None => contagem.push((c.passagem(), 1)),
        }
    }
    let n_primeira = contagem.iter().find(|(p, _)| *p == primeira).map_or(1, |(_, n)| *n);
    let mut v: Vec<(Data, u32)> =
        contagem.into_iter().filter(|(p, n)| p.0 <= primeira.0 && *n as f64 >= QUASE_TANTO * n_primeira as f64).map(|(p, _)| p).collect();
    v.sort_by_key(|(d, _)| std::cmp::Reverse(*d));
    v.truncate(max);
    v
}

/// Quantas passagens a composição temporal usa, se o comando não diz: umas 7 semanas.
pub const PASSAGENS_DO_TEMPO: usize = 4;

/// Quantos recortes a composição temporal pede ao Harmony de uma vez. Cada pedido é síncrono: com
/// 12 de uma vez (2 passagens de 3 cenas, HH e HV), o Harmony devolveu 504 a todos depois de 9 min
/// (22/11/2026); com 3, cada um leva cerca de 1 min.
pub const PEDIDOS_DE_UMA_VEZ: usize = 3;

/// O máximo de passagens dela (cerca de 3 meses; cada uma são 2 pedidos ao Harmony por cena).
pub const MAX_PASSAGENS: usize = 8;

/// O fuso UTM do WGS 84 do centro do retângulo `c`, que o Harmony aceita (326xx ao norte, 327xx
/// ao sul).
pub fn utm_do_harmony(c: [f64; 4]) -> u32 {
    let (x, y) = ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0);
    let fuso = (((x + 180.0) / 6.0).floor() as i64).clamp(0, 59) as u32 + 1;
    if y < 0.0 { 32700 + fuso } else { 32600 + fuso }
}

/// O nome da camada de uma polarização numa data.
pub fn nome_da_camada(p: Pol, d: Data) -> String {
    format!("{}Radar {} em dB (NISAR, {})", crate::PREFIXO, p.sigla(), d.texto())
}

/// O nome da camada da razão HV/HH numa data.
pub fn nome_da_razao(d: Data) -> String {
    format!("{}Razão HV/HH em dB (NISAR, {})", crate::PREFIXO, d.texto())
}

/// O nome da camada da composição colorida numa data.
pub fn nome_da_composicao(d: Data) -> String {
    format!("{}Composição HH, HV, HH − HV (NISAR, {})", crate::PREFIXO, d.texto())
}

/// As escalas, em dB, do vermelho (HH), do verde (HV) e do azul (HH − HV, o contrário da razão), no
/// estilo das imagens do NISAR da NASA sobre o delta do Mississippi: mais escuras e contrastadas,
/// com a mata em verde forte. São os percentis 5 e 99,5 de Viçosa (16/11/2026, 20 m, com o filtro
/// sigma de Lee) com gama 1,2 no vermelho e no azul e 0,9 no verde, passados para faixas lineares
/// (o realce do RustGIS não tem gama), as que mais se aproximam da curva.
pub const ESCALAS_DA_COMPOSICAO: [(f64, f64); 3] = [(-12.7, 4.3), (-18.0, -5.3), (0.8, 13.4)];

/// O nome da composição temporal das passagens de `de` a `ate`.
pub fn nome_da_temporal(de: Data, ate: Data) -> String {
    format!("{}Composição temporal HH, HV, variação do HV (NISAR, {} a {})", crate::PREFIXO, de.texto(), ate.texto())
}

/// As escalas, em dB, do vermelho (a mediana do HH), do verde (a mediana do HV) e do azul (o
/// desvio-padrão do HV entre as passagens, que sobe onde a vegetação muda: lavoura que cresce ou é
/// colhida, pasto que seca). Os percentis 2 e 98 de Viçosa com 3 passagens (23/10 a 16/11/2026,
/// 20 m, com o filtro sigma de Lee): HH de −13,2 a −0,8, HV de −18,9 a −7,9, desvio de 0,1 a 2,7.
pub const ESCALAS_DA_TEMPORAL: [(f64, f64); 3] = [(-13.0, -1.0), (-19.0, -8.0), (0.0, 3.0)];

/// Se uma camada do mapa é do NISAR (para a citação).
pub fn eh_do_nisar(nome: &str) -> bool {
    let n = nome.strip_prefix(crate::PREFIXO).unwrap_or("");
    (n.starts_with("Radar H") || n.starts_with("Razão HV/HH") || n.starts_with("Composição HH") || n.starts_with("Composição temporal HH"))
        && n.contains("(NISAR, ")
}

/// Se o retângulo `c` cabe no NISAR.
pub fn cabe(c: [f64; 4]) -> bool {
    c[2] - c[0] <= MAX_GRAUS && c[3] - c[1] <= MAX_GRAUS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(id: &str, nome: &str, t: &str) -> Value {
        json!({"id": id, "title": nome, "time_start": t,
            "polygons": [["-21.9414 -41.24235 -19.91882 -41.68292 -20.35208 -44.08572 -22.37026 -43.68325 -21.9414 -41.24235"]]})
    }

    const A: &str = "NISAR_L2_PR_GCOV_034_117_A_167_2005_DHDH_M_20261101T084302_20261101T084335_P05036_N_F_J_001";

    #[test]
    fn a_busca_pede_a_colecao_o_retangulo_e_o_dia() {
        let u = url_busca([-43.0049, -20.839, -42.7536, -20.6547], Some(Data { ano: 2026, mes: 11, dia: 1 }));
        assert!(u.contains("collection_concept_id=C3622241360-ASF&bounding_box=-43.0049,-20.8390,-42.7536,-20.6547"));
        assert!(u.contains("sort_key=-start_date"));
        assert!(u.ends_with("&temporal=2026-11-01T00:00:00Z,2026-11-02T00:00:00Z"));
        assert!(!url_busca([0.0; 4], None).contains("temporal="));
    }

    #[test]
    fn as_cenas_de_dupla_polarizacao_h() {
        let c = cenas(&json!({"feed": {"entry": [
            entrada("G1-ASF", A, "2026-11-01T08:43:02.000Z"),
            entrada("G2-ASF", &A.replace("DHDH", "SHSH"), "2026-11-01T08:43:02.000Z"),
            entrada("G3-ASF", &A.replace("_001", "_002"), "2026-11-01T08:43:02.000Z"),
            entrada("G4-ASF", &A.replace("GCOV", "GSLC"), "2026-11-01T08:43:02.000Z"),
        ]}}));
        assert_eq!(c.len(), 1, "a SH só tem HH, a _002 é outra versão da mesma, e a GSLC não é GCOV");
        assert_eq!(c[0].id, "G1-ASF");
        assert_eq!((c[0].trilha, c[0].frame), (117, 167));
        assert_eq!(c[0].data, Data { ano: 2026, mes: 11, dia: 1 });
        assert_eq!(c[0].rotulo(), "T117-F167 20261101 084302");
        assert_eq!(c[0].contorno["coordinates"][0][0], json!([-41.24235, -21.9414]), "lon, lat");
        assert_eq!(c[0].contorno["coordinates"][0].as_array().map(Vec::len), Some(5));
        let quad = cenas(&json!({"feed": {"entry": [entrada("G5-ASF", &A.replace("DHDH", "QPDH"), "2026-11-01T08:43:02Z")]}}));
        assert_eq!(quad.len(), 1, "a quádrupla também tem HH e HV");
        // 13/11/2026, trilha 117: o frame 167 no modo 2005 e um pedaço dele no 4005 (a troca de modo
        // no meio do frame; o 2005 fica sem dado numa faixa que só o pedaço cobre), e o 168 em dois
        // pedaços: ficam todos, cada um com a hora no nome (antes, os dois do 167 tinham o mesmo nome,
        // o RustGIS chamava o segundo de "(2)" e o comando perdia o primeiro).
        const B: &str = "NISAR_L2_PR_GCOV_035_117_A_167_2005_DHDH_M_20261113T084302_20261113T084336_P05036_N_F_J_001";
        const B4: &str = "NISAR_L2_PR_GCOV_035_117_A_167_4005_DHDH_A_20261113T084333_20261113T084336_P05036_N_P_J_001";
        const C1: &str = "NISAR_L2_PR_GCOV_035_117_A_168_4005_DHDH_A_20261113T084339_20261113T084350_P05036_N_P_J_001";
        const C2: &str = "NISAR_L2_PR_GCOV_035_117_A_168_4005_DHDH_A_20261113T084351_20261113T084410_P05036_N_P_J_001";
        let t = "2026-11-13T08:43:02Z";
        let m = cenas(&json!({"feed": {"entry": [entrada("G1", B4, t), entrada("G2", B, t), entrada("G3", C1, t), entrada("G4", C2, t)]}}));
        let ids: Vec<&str> = m.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["G1", "G2", "G3", "G4"]);
        let rotulos: Vec<String> = m.iter().map(Cena::rotulo).collect();
        assert_eq!(rotulos[0], "T117-F167 20261113 084333");
        assert!(rotulos.iter().enumerate().all(|(i, r)| !rotulos[..i].contains(r)), "nomes diferentes: {rotulos:?}");
        let k = camada_dos_contornos(&c);
        assert_eq!(k["features"][0]["properties"]["i"], 0);
        assert_eq!(k["features"][0]["properties"]["data"], "01/11/2026");
    }

    #[test]
    fn o_pedido_ao_harmony() {
        let c = &cenas(&json!({"feed": {"entry": [entrada("G4409242572-ASF", A, "2026-11-01T08:43:02.000Z")]}}))[0];
        let u = c.url(Pol::Hv, [-43.03, -20.86, -42.73, -20.63], 32723);
        assert_eq!(
            u,
            "https://harmony.earthdata.nasa.gov/C3622241360-ASF/ogc-api-coverages/1.0.0/collections/parameter_vars/coverage/rangeset?granuleId=G4409242572-ASF&variable=%2Fscience%2FLSAR%2FGCOV%2Fgrids%2FfrequencyA%2FHVHV&subset=lat(-20.8600:-20.6300)&subset=lon(-43.0300:-42.7300)&outputCrs=EPSG:32723&format=image%2Ftiff"
        );
        assert!(c.url(Pol::Hh, [0.0; 4], 32723).contains("frequencyA%2FHHHH"));
    }

    #[test]
    fn o_fuso_a_passagem_e_os_nomes() {
        assert_eq!(utm_do_harmony([-43.0, -20.8, -42.75, -20.65]), 32723, "Viçosa");
        assert_eq!(utm_do_harmony([-60.1, 2.7, -60.0, 2.9]), 32620, "Boa Vista, ao norte");
        let d = |dia| Data { ano: 2026, mes: 10, dia };
        let cena = |dia, trilha| Cena { id: String::new(), nome: String::new(), data: d(dia), trilha, frame: 1, contorno: Value::Null };
        let todas = [cena(30, 96), cena(23, 168), cena(23, 168), cena(20, 117), cena(20, 117)];
        let refs: Vec<&Cena> = todas.iter().collect();
        assert_eq!(passagem(&refs), Some((d(23), 168)), "a de 30/10 só tem uma das duas cenas");
        assert_eq!(passagem(&[]), None);
        let n = nome_da_camada(Pol::Hh, d(23));
        assert_eq!(n, "BrasilData: Radar HH em dB (NISAR, 23/10/2026)");
        assert!(eh_do_nisar(&n));
        assert!(eh_do_nisar(&nome_da_razao(d(23))));
        assert_eq!(nome_da_composicao(d(23)), "BrasilData: Composição HH, HV, HH − HV (NISAR, 23/10/2026)");
        assert!(eh_do_nisar(&nome_da_composicao(d(23))));
        let t = nome_da_temporal(d(1), d(23));
        assert_eq!(t, "BrasilData: Composição temporal HH, HV, variação do HV (NISAR, 01/10/2026 a 23/10/2026)");
        assert!(eh_do_nisar(&t));
        // A temporal: a passagem mais recente que cobre bem e as anteriores da mesma trilha, até o
        // máximo; a da outra trilha e a que cobre pouco ficam fora.
        let k = |dia, trilha| Cena { id: String::new(), nome: String::new(), data: d(dia), trilha, frame: 1, contorno: Value::Null };
        let ts = [k(28, 117), k(28, 117), k(25, 96), k(25, 96), k(16, 117), k(16, 117), k(4, 117), k(4, 117), k(2, 117)];
        let rs: Vec<&Cena> = ts.iter().collect();
        assert_eq!(passagens(&rs, 8), vec![(d(28), 117), (d(16), 117), (d(4), 117)]);
        assert_eq!(passagens(&rs, 2), vec![(d(28), 117), (d(16), 117)]);
        assert!(passagens(&[], 4).is_empty());
        assert!(!eh_do_nisar("BrasilData: Radar VV em dB (Sentinel-1, 26/10/2026)"));
        assert!(cabe([-43.0049, -20.839, -42.7536, -20.6547]), "Viçosa cabe");
        assert!(!cabe([-44.235, -22.259, -41.411, -19.828]), "a Zona da Mata não");
    }
}
