//! As janelas dos comandos (`commands[].form` do `plugin.json`, opengeos/RustGIS#1723): o
//! botão de um comando com janela a abre, e o **Executar** dela roda o comando com os valores
//! dos campos.
//!
//! - **O que a janela manda:** um campo de texto deixado vazio pode chegar como `""`, e uma
//!   escolha chega como o texto dela ("setembro", "Folha A4"). [`normalizar`] tira os vazios e
//!   troca cada escolha pelo valor que o comando já aceita de um script (`9`, `"a4"`), de modo
//!   que um script e a janela dão no mesmo. Um valor que não é uma escolha passa como veio.
//! - **O que a janela mostra:** o `report` da resposta. [`com_relato`] o acrescenta com os avisos
//!   que o comando deu, para que a janela não mostre o JSON da resposta.
//!
//! As escolhas daqui são as do `scripts/gerar.py`; o teste [`tests::conhece_todas_as_escolhas`]
//! confere, no `plugin.json`, que cada uma é conhecida.

use serde_json::{Value, json};

use crate::risco_fogo::NOMES_DOS_MESES;

/// A escolha da radiação solar que pede a média do ano (o comando sem `mes`).
pub const MEDIA_DO_ANO: &str = "média do ano";

/// As escolhas do sistema da grade e o `crs` de cada uma.
pub const SISTEMAS: [(&str, &str); 3] = [
    ("Automático: UTM do fuso, ou Albers se a área passa de um fuso", "automatico"),
    ("Albers do IBGE (EPSG:10857)", "albers"),
    ("Policônica do Brasil (EPSG:5880)", "policonica"),
];

/// As escolhas do formato do layout e o `perfil` de cada uma.
pub const PERFIS: [(&str, &str); 8] = [
    ("Folha A4", "a4"),
    ("Folha A3", "a3"),
    ("Folha A2", "a2"),
    ("Figura de 1 coluna (90 mm)", "figure-1col"),
    ("Figura de 1,5 coluna (140 mm)", "figure-1.5col"),
    ("Figura de 2 colunas (190 mm)", "figure-2col"),
    ("Tese: página inteira (160 × 225 mm)", "thesis-full"),
    ("Tese: meia página (160 × 110 mm)", "thesis-half"),
];

/// As escolhas do estilo da folha e o `estilo` de cada uma.
pub const ESTILOS: [(&str, &str); 6] = [
    ("Clássico", "classic"),
    ("Faixa", "banner"),
    ("Painel", "panel"),
    ("Editorial", "editorial"),
    ("Modular", "modular"),
    ("Pôster", "poster"),
];

/// As escolhas das cores da situação do DIST-S1 e a `paleta` de cada uma.
pub const PALETAS: [(&str, &str); 2] = [("A do BrasilData: etapa e confiança", "brasildata"), ("A oficial da OPERA", "oficial")];

/// As escolhas de um parâmetro e o valor de cada uma.
type Escolhas = &'static [(&'static str, &'static str)];

/// As escolhas de texto de cada comando: (comando, parâmetro, escolhas e valores).
const TROCAS: &[(&str, &str, Escolhas)] =
    &[("grade", "crs", &SISTEMAS), ("layout", "perfil", &PERFIS), ("layout", "estilo", &ESTILOS), ("dist-situacao", "paleta", &PALETAS)];

/// O número do mês de nome `nome` (janeiro é 1).
fn mes_pelo_nome(nome: &str) -> Option<u8> {
    let nome = nome.trim().to_lowercase();
    NOMES_DOS_MESES.iter().position(|m| *m == nome).and_then(|i| u8::try_from(i + 1).ok())
}

/// Os parâmetros do comando `id` como um script os daria: sem os textos vazios e com cada
/// escolha da janela trocada pelo seu valor.
pub fn normalizar(id: &str, params: &mut Value) {
    let Some(p) = params.as_object_mut() else { return };
    p.retain(|_, v| v.as_str().is_none_or(|t| !t.trim().is_empty()));
    for (comando, nome, escolhas) in TROCAS {
        if *comando != id {
            continue;
        }
        if let Some(v) = p.get(*nome).and_then(Value::as_str).and_then(|t| escolhas.iter().find(|(e, _)| *e == t)).map(|(_, v)| *v) {
            p.insert((*nome).to_string(), json!(v));
        }
    }
    match id {
        // O mês pelo nome; a média do ano é a radiação sem mês.
        "risco-fogo" | "sol-radiacao" => {
            let mes = p.get("mes").and_then(Value::as_str).map(str::to_string);
            if let Some(m) = mes {
                if m == MEDIA_DO_ANO {
                    p.remove("mes");
                } else if let Some(n) = mes_pelo_nome(&m) {
                    p.insert("mes".into(), json!(n));
                }
            }
        }
        // Outro sistema, pelo código EPSG, vale no lugar da escolha.
        "grade" => {
            if let Some(e) = p.remove("epsg") {
                p.insert("crs".into(), e);
            }
        }
        // O código do IBGE no lugar do nome.
        "area-municipio" => {
            let codigo = p
                .get("nome")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|t| t.len() == 7 && t.bytes().all(|b| b.is_ascii_digit()))
                .map(str::to_string);
            if let Some(c) = codigo {
                p.remove("nome");
                p.insert("codigo".into(), json!(c));
            }
        }
        // O dia do mês escrito só com o número.
        "inpe-focos-periodo" => {
            for k in ["de", "ate"] {
                let dia = p.get(k).and_then(Value::as_str).and_then(|t| t.trim().parse::<u32>().ok());
                if let Some(d) = dia {
                    p.insert(k.into(), json!(d));
                }
            }
        }
        _ => {}
    }
}

/// A resposta `r` com o `report` que a janela do comando mostra: os avisos do comando, sem o
/// nome do plugin na frente, quando ele não deu um. Uma resposta que espera a rede, ou que não
/// é um objeto, fica como está.
pub fn com_relato(mut r: Value, avisos: &[String], prefixo: &str) -> Value {
    if avisos.is_empty() || r.get("pending").is_some() {
        return r;
    }
    if let Some(o) = r.as_object_mut()
        && !o.contains_key("report")
    {
        let texto: Vec<&str> = avisos.iter().map(|a| a.strip_prefix(prefixo).unwrap_or(a)).collect();
        o.insert("report".into(), json!(texto.join("\n\n")));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(id: &str, p: Value) -> Value {
        let mut p = p;
        normalizar(id, &mut p);
        p
    }

    #[test]
    fn troca_as_escolhas_pelos_valores() {
        assert_eq!(norm("risco-fogo", json!({"mes": "setembro"})), json!({"mes": 9}));
        assert_eq!(norm("risco-fogo", json!({"mes": 3})), json!({"mes": 3}));
        assert_eq!(norm("sol-radiacao", json!({"mes": MEDIA_DO_ANO})), json!({}));
        assert_eq!(norm("sol-radiacao", json!({"mes": "março"})), json!({"mes": 3}));
        assert_eq!(norm("layout", json!({"perfil": "Folha A3", "estilo": "Pôster"})), json!({"perfil": "a3", "estilo": "poster"}));
        assert_eq!(norm("layout", json!({"perfil": "a2"})), json!({"perfil": "a2"}));
        assert_eq!(norm("dist-situacao", json!({"paleta": "A oficial da OPERA", "data": ""})), json!({"paleta": "oficial"}));
        assert_eq!(norm("grade", json!({"celula": 30, "crs": SISTEMAS[1].0})), json!({"celula": 30, "crs": "albers"}));
        assert_eq!(norm("grade", json!({"crs": SISTEMAS[0].0, "epsg": "31983"})), json!({"crs": "31983"}));
        assert_eq!(norm("grade", json!({"crs": SISTEMAS[0].0, "epsg": " "})), json!({"crs": "automatico"}));
        assert_eq!(norm("area-municipio", json!({"nome": "3171303", "uf": ""})), json!({"codigo": "3171303"}));
        assert_eq!(norm("area-municipio", json!({"nome": "Viçosa", "uf": "MG"})), json!({"nome": "Viçosa", "uf": "MG"}));
        assert_eq!(norm("inpe-focos-periodo", json!({"de": "5", "ate": "10/11/2026"})), json!({"de": 5, "ate": "10/11/2026"}));
        // A escolha de outro comando fica como veio.
        assert_eq!(norm("radar-vv", json!({"paleta": "A oficial da OPERA"})), json!({"paleta": "A oficial da OPERA"}));
    }

    #[test]
    fn o_relato_junta_os_avisos_sem_o_prefixo() {
        let avisos = vec!["BrasilData: Grade do projeto: X".to_string(), "outro".to_string()];
        assert_eq!(com_relato(json!({"layer": "g"}), &avisos, "BrasilData: ")["report"], "Grade do projeto: X\n\noutro");
        assert_eq!(com_relato(json!({"report": "meu"}), &avisos, "BrasilData: ")["report"], "meu");
        assert!(com_relato(json!({"pending": 1}), &avisos, "BrasilData: ").get("report").is_none());
        assert!(com_relato(json!({"layer": "g"}), &[], "BrasilData: ").get("report").is_none());
    }

    /// Cada escolha de cada janela do `plugin.json` vira um valor que o comando aceita: um número
    /// (o mês), nada (a média do ano) ou o valor de [`TROCAS`].
    #[test]
    fn conhece_todas_as_escolhas() {
        let m: Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
        let mut vistas = 0;
        for c in m["commands"].as_array().unwrap() {
            let id = c["id"].as_str().unwrap();
            for f in c["form"].as_array().into_iter().flatten().filter(|f| f["kind"] == "choice") {
                let nome = f["name"].as_str().unwrap();
                for e in f["choices"].as_array().unwrap() {
                    let p = norm(id, json!({nome: e}));
                    assert_ne!(p.get(nome), Some(e), "{id}: a escolha {e} de {nome} não é conhecida");
                    vistas += 1;
                }
            }
        }
        assert!(vistas >= 12 + 13 + 3 + 8 + 6 + 2, "{vistas} escolhas");
    }
}
