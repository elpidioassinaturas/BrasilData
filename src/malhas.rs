//! Os limites do IBGE (`malhas.bin`, de `scripts/baixar_malhas.py`), embutidos no plugin: a escolha
//! da área no painel não depende do site do IBGE, e o contorno é leve o bastante para o plugin ler.

use serde_json::{Value, json};

const DADOS: &[u8] = include_bytes!("../malhas.bin");

fn u32_em(pos: usize) -> u32 {
    u32::from_le_bytes([DADOS[pos], DADOS[pos + 1], DADOS[pos + 2], DADOS[pos + 3]])
}

/// Lê os inteiros de uma malha, a partir de `pos`.
struct Leitor {
    pos: usize,
}

impl Leitor {
    fn varint(&mut self) -> u64 {
        let (mut n, mut desloca) = (0, 0);
        loop {
            let b = DADOS[self.pos];
            self.pos += 1;
            n |= u64::from(b & 0x7F) << desloca;
            if b < 0x80 {
                return n;
            }
            desloca += 7;
        }
    }

    fn zigzag(&mut self) -> i64 {
        let n = self.varint();
        (n >> 1) as i64 ^ -((n & 1) as i64)
    }
}

/// A geometria GeoJSON (`Polygon` ou `MultiPolygon`) do código do IBGE: o de um estado (2 dígitos),
/// mesorregião (4), microrregião (5) ou município (7), ou o do Brasil (0).
pub fn geometria(codigo: &str) -> Option<Value> {
    let alvo: u32 = codigo.parse().ok()?;
    let (mut baixo, mut alto) = (0, u32_em(0) as usize);
    while baixo < alto {
        let meio = (baixo + alto) / 2;
        if u32_em(4 + 8 * meio) < alvo {
            baixo = meio + 1;
        } else {
            alto = meio;
        }
    }
    if baixo == u32_em(0) as usize || u32_em(4 + 8 * baixo) != alvo {
        return None;
    }
    let mut l = Leitor { pos: u32_em(4 + 8 * baixo + 4) as usize };
    let (mut x, mut y) = (0, 0);
    let mut poligonos: Vec<Value> = Vec::new();
    for _ in 0..l.varint() {
        let mut aneis: Vec<Value> = Vec::new();
        for _ in 0..l.varint() {
            let mut anel: Vec<Value> = Vec::new();
            for _ in 0..l.varint() {
                x += l.zigzag();
                y += l.zigzag();
                anel.push(Value::Array(vec![(x as f64 / 10_000.0).into(), (y as f64 / 10_000.0).into()]));
            }
            aneis.push(Value::Array(anel));
        }
        poligonos.push(Value::Array(aneis));
    }
    // Sem passar as coordenadas pelo `json!`, que as copiaria.
    let (tipo, coordenadas) =
        if poligonos.len() == 1 { ("Polygon", poligonos.remove(0)) } else { ("MultiPolygon", Value::Array(poligonos)) };
    let mut g = json!({"type": tipo});
    g["coordinates"] = coordenadas;
    Some(g)
}

/// Se o contorno de um polígono desenhado (`Polygon` em GeoJSON) cruza a si mesmo. Um polígono em
/// laço não dá erro no recorte do RustGIS: dá um recorte errado, sem aviso.
pub fn se_cruza(geometria: &Value) -> bool {
    let pontos: Vec<(f64, f64)> =
        geometria["coordinates"][0].as_array().into_iter().flatten().filter_map(|c| Some((c[0].as_f64()?, c[1].as_f64()?))).collect();
    let lado = |a: (f64, f64), b: (f64, f64), c: (f64, f64)| ((b.0 - a.0) * (c.1 - a.1) - (b.1 - a.1) * (c.0 - a.0)).signum();
    let cruzam = |a, b, c, d| lado(a, b, c) * lado(a, b, d) < 0.0 && lado(c, d, a) * lado(c, d, b) < 0.0;
    let segmentos = pontos.len().saturating_sub(1);
    (0..segmentos).any(|i| (i + 2..segmentos).any(|j| cruzam(pontos[i], pontos[i + 1], pontos[j], pontos[j + 1])))
}

/// Os municípios afinados de uma vez, no máximo: uma área do painel tem um; uma seleção do mapa já
/// vem com o contorno de quem a desenhou.
pub const MAX_AFINAR: usize = 10;

/// Os códigos dos municípios da área `fc` (as feições da camada da área) que ainda estão na malha
/// intermediária embutida, para buscar a máxima do IBGE. O plugin leva os municípios na
/// intermediária (Viçosa: 70 vértices, contra 389 na máxima) para caber no pacote; o recorte de um
/// raster por ela deixa células faltando ou sobrando na divisa. Estados, meso e microrregiões já
/// vêm na máxima; uma seleção ou um desenho não têm código.
pub fn a_afinar(fc: &Value) -> Vec<String> {
    let mut codigos: Vec<String> = fc["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|f| f["properties"]["malha"].as_str() != Some("maxima"))
        .filter_map(|f| f["properties"]["codigo"].as_str())
        .filter(|c| c.len() == 7 && c.bytes().all(|b| b.is_ascii_digit()))
        .map(str::to_string)
        .collect();
    codigos.dedup();
    codigos
}

/// O endereço da malha máxima do município `codigo` na API do IBGE (GeoJSON, com CORS; 10 KB em
/// Viçosa, 117 KB em Altamira, o maior município).
pub fn url_maxima(codigo: &str) -> String {
    format!("https://servicodados.ibge.gov.br/api/v4/malhas/municipios/{codigo}?formato=application/vnd.geo+json&qualidade=maxima")
}

/// `fc` com a geometria de cada município de `maximas` (código, resposta da API) trocada pela
/// malha máxima, marcada com `"malha": "maxima"`. Uma resposta sem geometria deixa a feição como
/// estava.
pub fn com_maximas(fc: &Value, maximas: &[(String, Value)]) -> Value {
    let mut novo = fc.clone();
    for f in novo["features"].as_array_mut().into_iter().flatten() {
        let Some(codigo) = f["properties"]["codigo"].as_str().map(str::to_string) else {
            continue;
        };
        let geometria = maximas.iter().find(|(c, _)| *c == codigo).map(|(_, r)| r["features"][0]["geometry"].clone());
        if let Some(g) = geometria.filter(|g| matches!(g["type"].as_str(), Some("Polygon" | "MultiPolygon"))) {
            f["geometry"] = g;
            f["properties"]["malha"] = json!("maxima");
        }
    }
    novo
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::municipios;

    #[test]
    fn so_o_municipio_na_intermediaria_e_afinado() {
        let f = |p: Value| json!({"type": "Feature", "properties": p, "geometry": {"type": "Polygon", "coordinates": [[[0, 0], [1, 0], [1, 1], [0, 0]]]}});
        let fc = json!({"features": [
            f(json!({"codigo": "3171303", "nome": "Viçosa"})),
            f(json!({"codigo": "3112", "nome": "Zona da Mata"})),
            f(json!({"codigo": "2211001", "malha": "maxima"})),
            f(json!({"nome": "Desenho livre"})),
        ]});
        assert_eq!(a_afinar(&fc), vec!["3171303".to_string()]);
        assert!(url_maxima("3171303").contains("/v4/malhas/municipios/3171303?") && url_maxima("1").ends_with("qualidade=maxima"));
        let resposta = json!({"features": [{"geometry": {"type": "Polygon", "coordinates": [[[0, 0], [2, 0], [2, 2], [0, 0]]]}}]});
        let novo = com_maximas(&fc, &[("3171303".into(), resposta), ("3112".into(), json!({}))]);
        assert_eq!(novo["features"][0]["geometry"]["coordinates"][0][1], json!([2, 0]));
        assert_eq!(novo["features"][0]["properties"]["malha"], "maxima");
        assert_eq!(novo["features"][0]["properties"]["nome"], "Viçosa", "as outras propriedades ficam");
        assert!(novo["features"][1]["properties"]["malha"].is_null(), "resposta sem geometria: fica como estava");
        assert!(a_afinar(&novo).is_empty());
    }

    #[test]
    fn um_desenho_em_laco_se_cruza() {
        let g = |anel: Value| json!({"type": "Polygon", "coordinates": [anel]});
        assert!(!se_cruza(&g(json!([[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]]))));
        assert!(se_cruza(&g(json!([[0, 0], [1, 1], [1, 0], [0, 1], [0, 0]]))));
        // Um polígono côncavo não se cruza.
        assert!(!se_cruza(&g(json!([[0, 0], [2, 0], [2, 2], [1, 0.5], [0, 2], [0, 0]]))));
    }

    /// Os pontos dos anéis de uma geometria; e confere que os anéis fecham.
    fn pontos(g: &Value) -> Vec<(f64, f64)> {
        let poligonos: Vec<&Value> = match g["type"].as_str() {
            Some("Polygon") => vec![&g["coordinates"]],
            _ => g["coordinates"].as_array().unwrap().iter().collect(),
        };
        let mut todos = Vec::new();
        for p in poligonos {
            for anel in p.as_array().unwrap() {
                let a = anel.as_array().unwrap();
                assert!(a.len() >= 4 && a.first() == a.last(), "anel aberto: {}", a.len());
                todos.extend(a.iter().map(|c| (c[0].as_f64().unwrap(), c[1].as_f64().unwrap())));
            }
        }
        todos
    }

    fn caixa(codigo: &str) -> [f64; 4] {
        let p = pontos(&geometria(codigo).unwrap());
        let (xs, ys) = (p.iter().map(|c| c.0), p.iter().map(|c| c.1));
        [xs.clone().fold(f64::MAX, f64::min), ys.clone().fold(f64::MAX, f64::min), xs.fold(f64::MIN, f64::max), ys.fold(f64::MIN, f64::max)]
    }

    #[test]
    fn cada_nivel_acha_a_sua_malha_no_lugar_certo() {
        // Piauí, o Sudeste Piauiense (2204) e Teresina.
        let dentro = |c: [f64; 4], de: [f64; 4]| c[0] >= de[0] && c[1] >= de[1] && c[2] <= de[2] && c[3] <= de[3];
        let pi = caixa("22");
        assert!(dentro(pi, [-46.1, -11.0, -40.3, -2.7]), "{pi:?}");
        assert!(dentro(caixa("2204"), pi) && dentro(caixa("2211001"), pi));
        // Os códigos que não existem.
        assert!(geometria("99").is_none() && geometria("9999999").is_none() && geometria("").is_none() && geometria("abc").is_none());
        // O primeiro e o último do índice.
        assert!(geometria("11").is_some() && geometria(municipios::todos().iter().map(|m| m.codigo).max().unwrap()).is_some());
    }

    #[test]
    fn todo_territorio_da_tabela_tem_malha_leve() {
        for m in municipios::todos() {
            for c in [m.codigo, &m.codigo[..2], m.micro, m.meso] {
                if !c.is_empty() {
                    let g = geometria(c).unwrap_or_else(|| panic!("sem malha: {c}"));
                    // A cota de instruções do plugin passava longe de 10 mil vértices e estourava em
                    // 37 mil (a Zona da Mata exata): uma malha nova mais pesada pede outro teste.
                    let n = pontos(&g).len();
                    assert!(n <= 11_000, "{c}: {n} vértices");
                }
            }
        }
    }
}
