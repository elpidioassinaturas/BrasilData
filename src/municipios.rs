//! Os municípios do IBGE (`municipios.csv`, de `scripts/baixar_municipios.py`), embutidos no
//! plugin, com a micro e a mesorregião de cada um: a escolha da área no painel não depende de rede.

use std::sync::OnceLock;

use serde_json::{Value, json};

/// Um município: código do IBGE (7 dígitos), nome, unidade da federação e a micro e a mesorregião
/// (vazias num município criado depois dessa divisão do IBGE, de 1989).
#[derive(Debug)]
pub struct Municipio {
    pub codigo: &'static str,
    pub nome: &'static str,
    pub uf: &'static str,
    pub uf_nome: &'static str,
    pub micro: &'static str,
    pub micro_nome: &'static str,
    pub meso: &'static str,
    pub meso_nome: &'static str,
}

const CSV: &str = include_str!("../municipios.csv");

/// Todos os municípios, na ordem dos nomes: estado, mesorregião, microrregião e município.
pub fn todos() -> &'static [Municipio] {
    static TODOS: OnceLock<Vec<Municipio>> = OnceLock::new();
    TODOS.get_or_init(|| {
        CSV.lines()
            .skip(1)
            .filter_map(|l| {
                let mut c = l.split(',');
                Some(Municipio {
                    codigo: c.next()?,
                    nome: c.next()?,
                    uf: c.next()?,
                    uf_nome: c.next()?,
                    micro: c.next()?,
                    micro_nome: c.next()?,
                    meso: c.next()?,
                    meso_nome: c.next()?,
                })
            })
            .collect()
    })
}

/// As unidades da federação, `(sigla, nome)`, na ordem dos nomes.
pub fn ufs() -> Vec<(&'static str, &'static str)> {
    let mut v: Vec<(&str, &str)> = todos().iter().map(|m| (m.uf, m.uf_nome)).collect();
    v.dedup();
    v
}

/// Os municípios de uma unidade da federação, pela sigla.
pub fn da_uf(uf: &str) -> impl Iterator<Item = &'static Municipio> + '_ {
    todos().iter().filter(move |m| m.uf.eq_ignore_ascii_case(uf))
}

/// O município de um código.
pub fn por_codigo(codigo: &str) -> Option<&'static Municipio> {
    todos().iter().find(|m| m.codigo == codigo)
}

/// Um território que pode virar a área: o código da malha, o nome e a sigla da UF.
#[derive(Debug, PartialEq)]
pub struct Territorio {
    pub codigo: &'static str,
    pub nome: String,
    pub uf: &'static str,
}

impl From<&'static Municipio> for Territorio {
    fn from(m: &'static Municipio) -> Territorio {
        Territorio { codigo: m.codigo, nome: m.nome.to_string(), uf: m.uf }
    }
}

/// O território de um nó da árvore do painel (`uf:PI`, `meso:2204`, `micro:22010`, `mun:2211001`).
pub fn territorio(no: &str) -> Option<Territorio> {
    let (nivel, valor) = no.split_once(':')?;
    let achar = |campo: fn(&'static Municipio) -> &'static str| todos().iter().find(|m| campo(m) == valor);
    match nivel {
        "uf" => achar(|m| m.uf).map(|m| Territorio { codigo: &m.codigo[..2], nome: m.uf_nome.to_string(), uf: m.uf }),
        "meso" if !valor.is_empty() => {
            achar(|m| m.meso).map(|m| Territorio { codigo: m.meso, nome: format!("Mesorregião {}", m.meso_nome), uf: m.uf })
        }
        "micro" if !valor.is_empty() => {
            achar(|m| m.micro).map(|m| Territorio { codigo: m.micro, nome: format!("Microrregião {}", m.micro_nome), uf: m.uf })
        }
        "mun" => por_codigo(valor).map(Territorio::from),
        _ => None,
    }
}

/// Um ramo da árvore: `count` é quantos municípios ele tem. Os filhos entram sem passar pelo
/// `json!`, que copia o valor: a cada nível da árvore, a cópia de tudo o que há abaixo dele
/// estourava a cota de instruções do plugin.
fn ramo(id: String, nome: &str, municipios: usize, filhos: Vec<Value>) -> Value {
    let mut r = json!({"id": id, "label": nome, "count": municipios});
    r["children"] = Value::Array(filhos);
    r
}

/// As microrregiões de uma mesorregião (ou, sem elas, os municípios soltos).
fn nos_da_meso(lista: &'static [Municipio]) -> Vec<Value> {
    lista
        .chunk_by(|a, b| a.micro == b.micro)
        .flat_map(|micro| {
            let nos: Vec<Value> = micro.iter().map(|m| json!({"id": format!("mun:{}", m.codigo), "label": m.nome})).collect();
            if micro[0].micro.is_empty() {
                nos
            } else {
                vec![ramo(format!("micro:{}", micro[0].micro), micro[0].micro_nome, micro.len(), nos)]
            }
        })
        .collect()
}

/// A árvore do painel para as grandes regiões `regioes` (o primeiro dígito do código do IBGE):
/// estado, mesorregião, microrregião e município, cada um uma área que se pode escolher.
pub fn arvore(regioes: &str) -> Value {
    let nos = todos()
        .chunk_by(|a, b| a.uf == b.uf)
        .filter(|uf| regioes.contains(&uf[0].codigo[..1]))
        .map(|uf| {
            let mesos = uf
                .chunk_by(|a, b| a.meso == b.meso)
                .flat_map(|meso| {
                    if meso[0].meso.is_empty() {
                        nos_da_meso(meso)
                    } else {
                        vec![ramo(format!("meso:{}", meso[0].meso), meso[0].meso_nome, meso.len(), nos_da_meso(meso))]
                    }
                })
                .collect();
            ramo(format!("uf:{}", uf[0].uf), uf[0].uf_nome, uf.len(), mesos)
        })
        .collect();
    Value::Array(nos)
}

/// O município de um nome, sem diferença de maiúsculas nem acentos, e da `uf` quando dada.
/// O nome pode trazer a sigla no fim ("Bom Jesus/PI", "Bom Jesus - PI", "Bom Jesus (PI)").
pub fn por_nome(nome: &str, uf: Option<&str>) -> Result<&'static Municipio, String> {
    let (nome, uf) = match (separar_uf(nome), uf) {
        ((n, Some(u)), None) => (n, Some(u)),
        ((n, _), u) => (n, u),
    };
    let alvo = dobrar(nome);
    let achados: Vec<&Municipio> =
        todos().iter().filter(|m| dobrar(m.nome) == alvo && uf.is_none_or(|u| m.uf.eq_ignore_ascii_case(u))).collect();
    match achados.as_slice() {
        [m] => Ok(m),
        [] => Err(match uf {
            Some(u) => format!("não há município chamado '{nome}' em {}", u.to_uppercase()),
            None => format!("não há município chamado '{nome}'"),
        }),
        varios => {
            let ufs: Vec<&str> = varios.iter().map(|m| m.uf).collect();
            Err(format!("há '{nome}' em {}: diga a UF (\"{nome}/{}\")", ufs.join(", "), ufs[0]))
        }
    }
}

/// `s` se é uma sigla de UF (duas letras).
fn sigla(s: &str) -> Option<&str> {
    (s.len() == 2 && s.bytes().all(|b| b.is_ascii_alphabetic())).then_some(s)
}

/// `"Nome/UF"`, `"Nome - UF"` ou `"Nome (UF)"` em nome e sigla; senão, o nome todo.
fn separar_uf(t: &str) -> (&str, Option<&str>) {
    let t = t.trim();
    if let Some(r) = t.strip_suffix(')')
        && let Some((n, u)) = r.rsplit_once('(')
        && let Some(u) = sigla(u.trim())
    {
        return (n.trim(), Some(u));
    }
    for sep in ['/', '-'] {
        if let Some((n, u)) = t.rsplit_once(sep)
            && let Some(u) = sigla(u.trim())
        {
            return (n.trim(), Some(u));
        }
    }
    (t, None)
}

/// `t` em minúsculas e sem acentos, para comparar nomes.
pub fn dobrar(t: &str) -> String {
    t.trim()
        .chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lista_tem_os_municipios_e_as_27_ufs() {
        assert!(todos().len() > 5_500);
        assert_eq!(ufs().len(), 27);
        assert_eq!(ufs()[0], ("AC", "Acre"));
        assert!(da_uf("pi").count() > 200);
        assert_eq!(por_codigo("2211001").map(|m| m.nome), Some("Teresina"));
    }

    /// Quantos nós tem `nos`, e quantos são municípios.
    fn contar(nos: &Value) -> (usize, usize) {
        let mut total = (0, 0);
        for n in nos.as_array().into_iter().flatten() {
            total.0 += 1;
            if n["id"].as_str().unwrap().starts_with("mun:") {
                total.1 += 1;
            }
            let (t, m) = contar(&n["children"]);
            total = (total.0 + t, total.1 + m);
        }
        total
    }

    /// O nó `id` em `nos`, em qualquer nível.
    fn achar<'a>(nos: &'a Value, id: &str) -> Option<&'a Value> {
        nos.as_array()?.iter().find_map(|n| if n["id"] == id { Some(n) } else { achar(&n["children"], id) })
    }

    #[test]
    fn as_duas_arvores_cabem_no_limite_do_rustgis_e_tem_todos_os_municipios() {
        // O RustGIS guarda até 5.000 nós por árvore e descarta o resto sem avisar (MAX_TREE_NODES).
        let (a, b) = (arvore("125"), arvore("34"));
        let (na, ma) = contar(&a);
        let (nb, mb) = contar(&b);
        assert!(na <= 5_000 && nb <= 5_000, "{na}, {nb}");
        assert_eq!(ma + mb, todos().len());
        assert!(achar(&a, "uf:TO").is_some() && achar(&b, "uf:SP").is_some() && achar(&b, "mun:3550308").is_some());
        // As mesorregiões e as microrregiões do IBGE.
        let (mesos, micros): (std::collections::HashSet<_>, std::collections::HashSet<_>) =
            todos().iter().filter(|m| !m.meso.is_empty()).map(|m| (m.meso, m.micro)).unzip();
        assert_eq!((mesos.len(), micros.len()), (137, 558));
    }

    #[test]
    fn a_hierarquia_vai_do_estado_ao_municipio() {
        let a = arvore("125");
        let pi = achar(&a, "uf:PI").unwrap();
        assert_eq!(pi["label"], "Piauí");
        assert_eq!(pi["count"], da_uf("PI").count());
        let meso = achar(&pi["children"], "meso:2202").unwrap();
        assert_eq!(meso["label"], "Centro-Norte Piauiense");
        let micro = achar(&meso["children"], "micro:22003").unwrap();
        assert_eq!(micro["label"], "Teresina");
        assert!(achar(&micro["children"], "mun:2211001").is_some());
        // Um município sem micro nem mesorregião fica direto no estado.
        let mt = achar(&a, "uf:MT").unwrap();
        assert!(mt["children"].as_array().unwrap().iter().any(|n| n["id"] == "mun:5101837"));
    }

    #[test]
    fn um_no_da_arvore_diz_o_territorio() {
        let t = |no| territorio(no).unwrap();
        assert_eq!((t("uf:PI").codigo, t("uf:PI").nome.as_str(), t("uf:PI").uf), ("22", "Piauí", "PI"));
        assert_eq!((t("meso:2202").codigo, t("meso:2202").nome.as_str()), ("2202", "Mesorregião Centro-Norte Piauiense"));
        assert_eq!((t("micro:22003").codigo, t("micro:22003").nome.as_str()), ("22003", "Microrregião Teresina"));
        assert_eq!((t("mun:2211001").codigo, t("mun:2211001").nome.as_str(), t("mun:2211001").uf), ("2211001", "Teresina", "PI"));
        assert!(territorio("uf:XX").is_none() && territorio("meso:").is_none() && territorio("outro").is_none());
    }

    #[test]
    fn o_nome_acha_o_municipio_sem_acento_e_com_a_uf() {
        assert_eq!(por_nome("teresina", None).unwrap().codigo, "2211001");
        assert_eq!(por_nome("São Paulo", Some("SP")).unwrap().codigo, "3550308");
        assert_eq!(por_nome("sao paulo/sp", None).unwrap().codigo, "3550308");
        assert_eq!(por_nome("Bom Jesus (PI)", None).unwrap().uf, "PI");
        assert_eq!(por_nome("Bom Jesus - pi", None).unwrap().uf, "PI");
        // Um nome de mais de um estado pede a UF; um traço no nome não é a UF.
        let e = por_nome("Bom Jesus", None).unwrap_err();
        assert!(e.contains("PI") && e.contains("diga a UF"), "{e}");
        assert_eq!(por_nome("Olho d'Água do Borges", None).unwrap().uf, "RN");
        assert!(por_nome("Teresina", Some("GO")).is_err());
    }
}
