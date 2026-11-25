//! Os endereços do CNEFE (o Cadastro Nacional de Endereços para Fins Estatísticos, do Censo 2022 do
//! IBGE), pelo tema de endereços do Overture Maps: um ponto por endereço, com a rua, o número e o CEP.
//!
//! - **De onde:** os GeoParquet do Overture, que os toma do OpenAddresses (`br/<uf>/statewide`), que
//!   os toma do AddressForAll, que processa os arquivos do IBGE (conferido em 25/11/2026). São
//!   89,9 milhões no país. O campo de licença das linhas diz "Proprietary" porque a ficha do
//!   OpenAddresses só pede a citação do IBGE; a página de atribuição do Overture diz CC0.
//! - **Como:** o RustGIS 0.7.0 lê no lugar os arquivos que cobrem a área e busca só o que a vista
//!   mostra, como as ruas do OpenStreetMap ([`crate::rodovias`]). A camada não é recortada (cada
//!   arquivo tem 7 milhões de pontos, e o `clip` lê até 2 milhões): é mascarada pelo contorno.
//! - **O que falta:** o município e o bairro vêm em colunas de lista, que o RustGIS deixa de fora.
//! - **Escala:** o RustGIS lê no máximo 60 mil feições por vista; de longe seria uma amostra.

use serde_json::{Value, json};

/// A coleção do tema no catálogo do Overture.
pub const COLECAO: &str = "addresses/address";

/// O nome da camada (com vários arquivos, uma por arquivo, " (parte N)").
pub const CAMADA: &str = "BrasilData: Endereços (CNEFE 2022, Overture)";

/// O nome do estilo dos endereços no projeto.
pub const ESTILO: &str = "BrasilData: endereços com e sem número";

/// Mais arquivos que isto numa área: grande demais (o Brasil toca 14, e Minas Gerais, 8).
pub const MAX_ARQUIVOS: usize = 8;

/// Mais longe que esta escala (1:N) a camada some.
pub const MAIS_LONGE: f64 = 50_000.0;

/// A escala (1:N) de onde para perto aparecem os números.
pub const NUMEROS_MAIS_LONGE: f64 = 3_000.0;

/// Só os do Brasil: um arquivo pode pegar o outro lado da fronteira. Vai na expressão das classes e
/// na condição dos rótulos, e não numa consulta de definição: numa camada lida por vista, o RustGIS
/// 0.8.0 avisa, a cada consulta, que pode faltar feição, e sugere carregar todas (7 milhões).
const DO_BRASIL: &str = "country = 'BR'";

/// A condição (no SQL do RustGIS) dos endereços sem número: os em branco, os "SN (CASA 1)" e o zero.
const SEM_NUMERO: &str = "number IS NULL OR number LIKE 'SN%' OR number = '0'";

/// Os endereços com número.
const COM_NUMERO: &str = "number IS NOT NULL AND number NOT LIKE 'SN%' AND number <> '0'";

/// A condição dos rótulos: os do Brasil com número.
pub fn rotulados() -> String {
    format!("{DO_BRASIL} AND {COM_NUMERO}")
}

/// As classes: o valor da expressão, o rótulo da legenda e a cor.
const CLASSES: [(&str, &str, [u8; 3]); 2] = [("1", "Endereço com número", [179, 60, 30]), ("2", "Endereço sem número", [120, 120, 120])];

pub const CITACAO: &str = "IBGE, Censo Demográfico 2022, Cadastro Nacional de Endereços para Fins Estatísticos (CNEFE), processado pelo AddressForAll e pelo OpenAddresses; tema de endereços do Overture Maps Foundation, lido no lugar (GeoParquet), sem recorte";

pub const LICENCA: &str = "dados abertos do IBGE, com citação da fonte (a página de atribuição do Overture diz CC0)";

/// Se a camada `nome` é dos endereços.
pub fn eh_daqui(nome: &str) -> bool {
    nome.starts_with(CAMADA)
}

/// O nome da camada de cada um de `n` arquivos.
pub fn camadas(n: usize) -> Vec<String> {
    crate::rodovias::uma_por_arquivo(CAMADA, n)
}

fn simbolo([r, g, b]: [u8; 3]) -> Value {
    json!({"fill": [r, g, b, 255], "stroke": [255, 255, 255, 255], "stroke_width": 0.3, "size": 3.5, "marker": "Circle"})
}

/// O estilo, no formato do `style.add`: Valores Únicos por uma expressão, com número e sem número, e
/// os números como rótulo (só dos que têm, de perto).
pub fn estilo() -> String {
    let lista: Vec<Value> = CLASSES.iter().map(|(v, _, cor)| json!([v, simbolo(*cor)])).collect();
    let rotulos: serde_json::Map<String, Value> = CLASSES.iter().map(|(v, r, _)| (v.to_string(), json!(r))).collect();
    let expressao = format!("CASE WHEN NOT ({DO_BRASIL}) THEN '0' WHEN {SEM_NUMERO} THEN '2' ELSE '1' END");
    let nada = json!({"fill": [0, 0, 0, 0], "stroke": [0, 0, 0, 0], "stroke_width": 0.0, "size": 0.0, "marker": "Circle"});
    json!([{"name": ESTILO, "geometry": "Point",
        "renderer": {"Unique": {"field": "number", "expression": expressao, "classes": lista, "other": nada, "ramp": "Harbour",
            "class_labels": rotulos}},
        "labels": {"enabled": false, "field": "number", "size": 7.0, "color": [60, 30, 20, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn as_camadas_e_o_estilo() {
        assert_eq!(camadas(1), [CAMADA]);
        let v = camadas(2);
        assert_eq!(v[1], format!("{CAMADA} (parte 2)"));
        assert!(v.iter().all(|n| eh_daqui(n)) && !eh_daqui("BrasilData: Ruas e estradas (OpenStreetMap, Overture)"));
        let s: Value = serde_json::from_str(&estilo()).unwrap();
        let u = &s[0]["renderer"]["Unique"];
        assert_eq!(s[0]["geometry"], "Point");
        assert_eq!(u["classes"].as_array().unwrap().len(), 2);
        assert_eq!(u["class_labels"]["2"], "Endereço sem número");
        assert!(u["expression"].as_str().unwrap().contains("LIKE 'SN%'") && u["expression"].as_str().unwrap().contains(DO_BRASIL));
        assert_eq!(u["other"]["size"], 0.0);
        assert!(rotulados().starts_with(DO_BRASIL));
    }
}
