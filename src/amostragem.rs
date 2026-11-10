//! A amostragem de campo na área de trabalho: pontos para coletar solo, vegetação ou o que for,
//! sorteados dentro da área.
//!
//! - **Aleatória simples:** `n` pontos ao acaso, com a mesma chance em qualquer lugar da área.
//! - **Estratificada:** os estratos são os polígonos de uma camada do mapa (Solos, Geologia,
//!   MapBiomas recortado…), agrupados por um campo. Cada estrato recebe pontos na proporção da
//!   sua área, com um mínimo por estrato, e os pontos de um estrato caem ao acaso dentro dele.
//! - **Sistemática:** uma grade regular de pontos, com o espaçamento em metros e a origem
//!   sorteada, numerada em zigue-zague (uma linha de oeste para leste, a seguinte de volta).
//!
//! O plugin não lê o polígono da área, que pode ter dezenas de milhares de vértices, nem os
//! pontos da amostra: ler ou montar alguns milhares de feições em JSON passa da cota de instruções
//! de uma chamada de plugin. O trabalho é do RustGIS:
//! - na aleatória e na estratificada, ele sorteia os pontos dentro da área (`analysis.simulate`,
//!   com semente, na área dissolvida num polígono só), uniformes em longitude e latitude: numa
//!   área do tamanho de um estado, a diferença para uniforme na superfície fica abaixo de 3%;
//! - na sistemática, o plugin monta a grade (até [`MAX_CANDIDATOS`] pontos no retângulo da área) e
//!   o RustGIS a recorta pela área (`clip`);
//! - cada ponto leva a ordem do sorteio ([`ORDEM`]); o estrato de cada um vem do `spatialJoin`; os
//!   primeiros `n` de um estrato na ordem do sorteio saem de seleções por atributo, numa busca
//!   binária pelo corte da ordem, que só devolvem contagens; a numeração é a `@row_number` e as
//!   coordenadas, as do `calculateGeometry`.
//!
//! Como os pontos sorteados são uniformes, a parte deles que cai num estrato estima a área do
//! estrato, e os primeiros na ordem do sorteio que caem num estrato são uma amostra ao acaso dele.
//!
//! O sorteio é reproduzível: a mesma área, o mesmo método e o mesmo número de sorteio dão os
//! mesmos pontos. Outro número dá outra amostra.

use std::collections::BTreeMap;

use serde_json::{Value, json};

/// As camadas da amostra, uma por método.
pub const ALEATORIA: &str = "BrasilData: amostragem aleatória";
pub const ESTRATIFICADA: &str = "BrasilData: amostragem estratificada";
pub const SISTEMATICA: &str = "BrasilData: amostragem sistemática";

/// As camadas de passagem, tiradas do mapa no fim.
pub const CANDIDATOS: &str = "BrasilData: amostragem (candidatos)";
pub const NA_AREA: &str = "BrasilData: amostragem (na área)";
pub const NOS_ESTRATOS: &str = "BrasilData: amostragem (nos estratos)";

/// O campo com a ordem do sorteio de cada candidato.
pub const ORDEM: &str = "bd_ordem";

/// Um candidato: a ordem do sorteio e o ponto (longitude, latitude).
pub type Candidato = (u64, [f64; 2]);

/// Os números de pontos do controle do painel.
pub const PONTOS: [u32; 14] = [5, 10, 15, 20, 25, 30, 40, 50, 75, 100, 150, 200, 300, 500];
pub const PONTOS_PADRAO: u32 = 30;
/// O máximo de pontos de uma amostra aleatória ou estratificada: a amostra é para ir a campo.
pub const MAX_PONTOS: u32 = 500;

/// Os espaçamentos do controle do painel, em metros.
pub const ESPACAMENTOS: [f64; 9] = [50.0, 100.0, 200.0, 250.0, 500.0, 1000.0, 2000.0, 5000.0, 10000.0];
pub const ESPACAMENTO_PADRAO: f64 = 500.0;

/// O controle do sorteio vai de 1 a este número.
pub const SORTEIOS: i64 = 20;

/// O mínimo de pontos por estrato, quando o script não diz outro.
pub const MINIMO_PADRAO: usize = 1;

/// Os pontos da grade sistemática no retângulo da área: mais que isto passa da cota de instruções
/// do plugin ao montar a camada deles.
pub const MAX_CANDIDATOS: usize = 2_000;

/// Os pontos sorteados na área para a estratificada (os que estimam a área de cada estrato):
/// 20 por ponto da amostra, de 2.000 a 20.000. O RustGIS os sorteia; o plugin não os lê.
pub fn pontos_para_estratos(n: u64) -> u64 {
    (20 * n).clamp(2000, 20_000)
}

/// A semente do `analysis.simulate` para o sorteio `n` de um método.
pub fn semente(metodo: u64, n: u64) -> u64 {
    Sorteio::novo(metodo, n).proximo_u64() >> 12
}

/// Os campos que não servem de estrato: os do sorteio e os que o `spatialJoin` acrescenta.
const NAO_ESTRATO: [&str; 6] = [ORDEM, "row", "Join_Count", "TARGET_FID", "JOIN_FID", "fid"];

/// O gerador do sorteio (SplitMix64): rápido, sem dependências e igual em toda máquina.
pub struct Sorteio(u64);

impl Sorteio {
    /// O gerador do sorteio de número `n` de um método (`metodo` separa os métodos, para a
    /// aleatória e a estratificada de mesmo número não darem os mesmos candidatos).
    pub fn novo(metodo: u64, n: u64) -> Self {
        Sorteio(metodo.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ n.wrapping_mul(0xD1B5_4A32_D192_ED03) ^ 0x5EED)
    }

    pub fn proximo_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Um número de 0 (incluído) a 1 (excluído).
    pub fn proximo(&mut self) -> f64 {
        (self.proximo_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Metros por grau de longitude e de latitude na latitude `lat` (graus), no elipsoide do
/// SIRGAS 2000 (GRS 80), com a série usual.
pub fn metros_por_grau(lat: f64) -> (f64, f64) {
    let f = lat.to_radians();
    let lon = 111_412.84 * f.cos() - 93.5 * (3.0 * f).cos() + 0.118 * (5.0 * f).cos();
    let lat = 111_132.92 - 559.82 * (2.0 * f).cos() + 1.175 * (4.0 * f).cos() - 0.0023 * (6.0 * f).cos();
    (lon, lat)
}

/// A grade de pontos com `espacamento` metros no retângulo `c`, com a origem sorteada dentro de
/// uma célula. As linhas vão de norte a sul, alternando o sentido (zigue-zague), para a
/// numeração seguir um caminho de campo. O espaçamento entre linhas usa a latitude do centro; o
/// espaçamento numa linha usa a latitude dela, e fica certo no chão em qualquer latitude.
pub fn sistematicos(c: [f64; 4], espacamento: f64, s: &mut Sorteio) -> Result<Vec<[f64; 2]>, String> {
    if !(1.0..=100_000.0).contains(&espacamento) {
        return Err(format!("o espaçamento vai de 1 m a 100 km, não {espacamento} m"));
    }
    let centro = (c[1] + c[3]) / 2.0;
    let dlat = espacamento / metros_por_grau(centro).1;
    let (ox, oy) = (s.proximo(), s.proximo());
    let mut linhas = Vec::new();
    let mut lat = c[1] + oy * dlat;
    while lat <= c[3] {
        linhas.push(lat);
        lat += dlat;
    }
    // A conta antes de fazer os pontos: a linha mais longa (a de menor |latitude|) dá o máximo.
    let mais_larga = linhas.iter().map(|l| l.abs()).fold(f64::INFINITY, f64::min);
    let por_linha = if linhas.is_empty() { 0.0 } else { (c[2] - c[0]) / (espacamento / metros_por_grau(mais_larga).0) + 1.0 };
    let estimativa = linhas.len() as f64 * por_linha;
    if estimativa > MAX_CANDIDATOS as f64 {
        let minimo = espacamento * (estimativa / MAX_CANDIDATOS as f64).sqrt();
        return Err(format!(
            "com {} entre pontos, o retângulo da área teria cerca de {} pontos, e o plugin faz até {}: use um espaçamento de pelo menos {}",
            texto_do_espacamento(espacamento),
            estimativa.round(),
            MAX_CANDIDATOS,
            texto_do_espacamento((minimo / 10.0).ceil() * 10.0)
        ));
    }
    let mut pontos = Vec::new();
    for (i, lat) in linhas.iter().rev().enumerate() {
        let dlon = espacamento / metros_por_grau(*lat).0;
        let mut linha = Vec::new();
        let mut lon = c[0] + ox * dlon;
        while lon <= c[2] {
            linha.push([lon, *lat]);
            lon += dlon;
        }
        if i % 2 == 1 {
            linha.reverse();
        }
        pontos.extend(linha);
    }
    Ok(pontos)
}

/// Os candidatos como camada, cada um com a ordem do sorteio.
pub fn camada_de_candidatos(pontos: &[Candidato]) -> Value {
    let feicoes: Vec<Value> = pontos
        .iter()
        .map(|(i, p)| {
            let c = [(p[0] * 1e6).round() / 1e6, (p[1] * 1e6).round() / 1e6];
            json!({"type": "Feature", "properties": {ORDEM: i}, "geometry": {"type": "Point", "coordinates": c}})
        })
        .collect();
    json!({"type": "FeatureCollection", "features": feicoes})
}

/// A ordem e o ponto de uma feição devolvida pelo RustGIS.
pub fn ordem_e_ponto(f: &Value) -> Option<Candidato> {
    let ordem = f["properties"][ORDEM].as_u64().or_else(|| f["properties"][ORDEM].as_f64().map(|x| x as u64))?;
    let c = &f["geometry"]["coordinates"];
    Some((ordem, [c[0].as_f64()?, c[1].as_f64()?]))
}

/// Os pontos de cada estrato: primeiro `minimo` em cada um (ou todos os candidatos dele, se
/// forem menos); depois, um ponto de cada vez ao estrato mais longe da sua parte proporcional
/// ao número de candidatos (que estima a área). Com mais estratos que pontos, o mínimo pode
/// passar de `n`.
pub fn alocar(contagens: &[usize], n: usize, minimo: usize) -> Vec<usize> {
    let total: usize = contagens.iter().sum();
    if total == 0 {
        return vec![0; contagens.len()];
    }
    let ideal: Vec<f64> = contagens.iter().map(|c| n as f64 * *c as f64 / total as f64).collect();
    let mut out: Vec<usize> = contagens.iter().map(|c| minimo.min(*c)).collect();
    while out.iter().sum::<usize>() < n.min(total) {
        let Some(i) = (0..out.len()).filter(|i| out[*i] < contagens[*i]).max_by(|a, b| {
            (ideal[*a] - out[*a] as f64).partial_cmp(&(ideal[*b] - out[*b] as f64)).unwrap_or(std::cmp::Ordering::Equal).then(b.cmp(a))
        }) else {
            break;
        };
        out[i] += 1;
    }
    out
}

/// O valor de um campo como texto de estrato (vazio é sem estrato).
pub fn valor_do_estrato(v: &Value) -> Option<String> {
    match v {
        Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// O prefixo que o `spatialJoin` do RustGIS põe nos campos da camada juntada.
pub const PREFIXO_DA_JUNCAO: &str = "join_";

/// O nome de um campo sem o prefixo da junção, como o usuário o conhece na camada dos estratos.
pub fn nome_legivel(campo: &str) -> &str {
    campo.strip_prefix(PREFIXO_DA_JUNCAO).unwrap_or(campo)
}

/// Os nomes de campo que costumam ter as classes de um mapa temático (Solos, Geologia,
/// Vegetação, MapBiomas…), na ordem de preferência: a ordem do solo antes da legenda, que tem
/// mais classes.
const CAMPOS_PREFERIDOS: [&str; 14] = [
    "ordem",
    "subordem",
    "classe",
    "class",
    "legenda",
    "nome",
    "unidade",
    "tipo",
    "grupo",
    "categoria",
    "bioma",
    "fisionomia",
    "litologia",
    "uso",
];

/// Se o nome é de um código, identificador ou medida, e não de uma classe.
fn campo_de_codigo(nome: &str) -> bool {
    let n = nome_legivel(nome).to_lowercase();
    ["cd_", "cod", "id", "val_", "num", "nr_", "qt", "fid", "objectid", "shape", "area", "perim", "gid", "sigla_uf"]
        .iter()
        .any(|p| n.starts_with(p))
        || n.ends_with("_id")
}

/// A preferência de um campo: um nome de [`CAMPOS_PREFERIDOS`] (exato, depois contido no nome), ou
/// nenhuma.
fn preferencia(nome: &str) -> usize {
    let n = nome_legivel(nome).to_lowercase();
    if let Some(i) = CAMPOS_PREFERIDOS.iter().position(|p| n == *p) {
        return i;
    }
    if let Some(i) = CAMPOS_PREFERIDOS.iter().position(|p| n.contains(p)) {
        return 50 + i;
    }
    100
}

/// Os campos que servem de estrato nas feições, com quantas classes cada um tem: os que têm de 2
/// a 30 classes, valor em pelo menos metade dos pontos e algum valor que não é número (os só
/// numéricos são códigos ou contagens), sem os de código ou identificador. Primeiro os de nome
/// conhecido ([`CAMPOS_PREFERIDOS`]), depois os outros, do que tem menos classes ao que tem mais
/// (classes largas pedem menos pontos).
pub fn campos_dos_estratos(feicoes: &[Value]) -> Vec<(String, usize)> {
    let mut classes: BTreeMap<String, (std::collections::BTreeSet<String>, usize, bool)> = BTreeMap::new();
    for f in feicoes {
        let Some(p) = f["properties"].as_object() else { continue };
        for (k, v) in p {
            if NAO_ESTRATO.iter().any(|n| n.eq_ignore_ascii_case(k)) {
                continue;
            }
            let e = classes.entry(k.clone()).or_default();
            if let Some(t) = valor_do_estrato(v) {
                e.2 |= t.trim().parse::<f64>().is_err();
                e.0.insert(t);
                e.1 += 1;
            }
        }
    }
    let mut out: Vec<(String, usize)> = classes
        .into_iter()
        .filter(|(k, (c, com_valor, texto))| {
            (2..=30).contains(&c.len()) && *com_valor * 2 >= feicoes.len() && *texto && !campo_de_codigo(k)
        })
        .map(|(k, (c, _, _))| (k, c.len()))
        .collect();
    out.sort_by_key(|(k, n)| (preferencia(k), *n));
    out
}

/// O rótulo do ponto `i` (de 0) numa amostra de `total`: P01, P001…
pub fn rotulo(i: usize, total: usize) -> String {
    let digitos = total.max(1).to_string().len().max(2);
    format!("P{:0digitos$}", i + 1)
}

/// A expressão do RustGIS que dá o rótulo de cada ponto pela linha (P01, P001…), numa amostra de
/// `total` pontos.
pub fn expressao_do_rotulo(total: usize) -> String {
    let digitos = total.max(1).to_string().len().max(2);
    format!("concat('P', right(concat('{}', to_string(@row_number)), {digitos}))", "0".repeat(digitos))
}

/// Um nome de campo numa cláusula do RustGIS: entre aspas duplas.
pub fn sql_campo(nome: &str) -> String {
    format!("\"{}\"", nome.replace('"', ""))
}

/// Um valor numa cláusula: número como está, texto entre aspas simples (as de dentro dobradas).
pub fn sql_valor(v: &Value) -> Option<String> {
    match v {
        Value::Number(n) => Some(n.to_string()),
        Value::String(s) => Some(format!("'{}'", s.replace('\'', "''"))),
        Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// "500 m", "1 km", "2,5 km".
pub fn texto_do_espacamento(m: f64) -> String {
    if m >= 1000.0 {
        return format!("{} km", m / 1000.0).replace('.', ",");
    }
    format!("{m} m").replace('.', ",")
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERESINA: [f64; 4] = [-42.9707, -5.5866, -42.599, -4.7863];

    #[test]
    fn o_sorteio_se_repete_e_cada_numero_da_outra_amostra() {
        assert_eq!(semente(1, 1), semente(1, 1));
        assert_ne!(semente(1, 1), semente(1, 2));
        assert_ne!(semente(1, 1), semente(2, 1));
        let mut s = Sorteio::novo(3, 1);
        assert!((0..1000).map(|_| s.proximo()).all(|x| (0.0..1.0).contains(&x)));
        assert_eq!(pontos_para_estratos(30), 2000);
        assert_eq!(pontos_para_estratos(500), 10_000);
        assert_eq!(pontos_para_estratos(5000), 20_000);
    }

    #[test]
    fn a_grade_tem_o_espacamento_no_chao_e_vai_em_zigue_zague() {
        let g = sistematicos(TERESINA, 5000.0, &mut Sorteio::novo(4, 1)).unwrap();
        // 41 km × 89 km: cerca de 8 × 18 pontos.
        assert!((120..=180).contains(&g.len()), "{}", g.len());
        let (mx, my) = metros_por_grau(-5.2);
        // Dois vizinhos numa linha estão a 5 km.
        let d = ((g[1][0] - g[0][0]) * mx).hypot((g[1][1] - g[0][1]) * my);
        assert!((d - 5000.0).abs() < 5.0, "{d}");
        // A primeira linha é a mais ao norte, e a segunda volta de leste para oeste.
        assert!(g[0][1] > g.last().unwrap()[1]);
        let segunda: Vec<&[f64; 2]> = g.iter().filter(|p| (p[1] - g[0][1]).abs() > 1e-9).take(2).collect();
        assert!(segunda[0][0] > segunda[1][0]);
        // Espaçamento pequeno demais para a área: recusa, dizendo o mínimo.
        let e = sistematicos(TERESINA, 50.0, &mut Sorteio::novo(4, 1)).unwrap_err();
        assert!(e.contains("pelo menos") && e.contains("2000"), "{e}");
        assert!(sistematicos(TERESINA, 0.5, &mut Sorteio::novo(4, 1)).is_err());
    }

    #[test]
    fn a_alocacao_e_proporcional_com_minimo_e_soma_n() {
        assert_eq!(alocar(&[600, 300, 100], 10, 1), vec![6, 3, 1]);
        // O estrato pequeno ganha o mínimo, e a soma continua n.
        let a = alocar(&[900, 95, 5], 20, 2);
        assert_eq!(a.iter().sum::<usize>(), 20);
        assert!(a[2] == 2 && a[0] > a[1], "{a:?}");
        // Um estrato com menos candidatos que o mínimo dá todos os que tem.
        assert_eq!(alocar(&[1000, 1], 10, 2)[1], 1);
        // Mais estratos que pontos: o mínimo passa de n.
        assert_eq!(alocar(&[10, 10, 10, 10], 2, 1), vec![1, 1, 1, 1]);
        assert_eq!(alocar(&[], 5, 1), Vec::<usize>::new());
        assert_eq!(alocar(&[0, 0], 5, 1), vec![0, 0]);
    }

    #[test]
    fn o_campo_dos_estratos_tem_poucas_classes_e_valor_na_maioria() {
        let f = |i: usize| {
            let simbolo = ["LAd", "FFc", "RYve"][i % 3];
            let ordem = ["LATOSSOLOS", "PLINTOSSOLOS", "NEOSSOLOS"][i % 3];
            json!({"properties": {
                ORDEM: i,
                "join_cod_simbol": simbolo,
                "join_ordem": ordem,
                "join_legenda": format!("L{}", i % 12),
                "join_val_ncompo": (i % 3) + 2,
                "join_cd_ord_id": format!("{}", i % 4),
                "join_id_poligono": i,
                "join_fonte": "IBGE",
            }})
        };
        let fs: Vec<Value> = (0..40).map(f).collect();
        let c = campos_dos_estratos(&fs);
        // A ordem do solo primeiro, depois a legenda. O símbolo e o cd_ord_id (códigos), o número
        // de componentes (só números), o id (40 classes) e a fonte (uma) ficam de fora.
        let nomes: Vec<&str> = c.iter().map(|(k, _)| nome_legivel(k)).collect();
        assert_eq!(nomes, ["ordem", "legenda"]);
        assert_eq!(c[0], ("join_ordem".to_string(), 3));
        assert_eq!(valor_do_estrato(&json!(" ")), None);
        assert_eq!(valor_do_estrato(&json!(12)).as_deref(), Some("12"));
        assert_eq!(nome_legivel("join_ordem"), "ordem");
        assert_eq!(nome_legivel("classe"), "classe");
    }

    #[test]
    fn os_rotulos_e_as_clausulas() {
        assert_eq!(rotulo(99, 100), "P100");
        assert_eq!(rotulo(4, 1500), "P0005");
        assert_eq!(expressao_do_rotulo(30), "concat('P', right(concat('00', to_string(@row_number)), 2))");
        assert_eq!(expressao_do_rotulo(100), "concat('P', right(concat('000', to_string(@row_number)), 3))");
        assert_eq!(sql_campo("ordem_solo"), "\"ordem_solo\"");
        assert_eq!(sql_valor(&json!("Latossolo d'água")).as_deref(), Some("'Latossolo d''água'"));
        assert_eq!(sql_valor(&json!(12)).as_deref(), Some("12"));
        assert_eq!(sql_valor(&json!(null)), None);
        assert_eq!(texto_do_espacamento(2500.0), "2,5 km");
        assert_eq!(texto_do_espacamento(250.0), "250 m");
        let cand = camada_de_candidatos(&[(7, [-42.80000004, -5.1])]);
        assert_eq!(ordem_e_ponto(&cand["features"][0]), Some((7, [-42.8, -5.1])));
    }
}
