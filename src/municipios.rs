//! Os municípios do IBGE (`municipios.csv`, de `scripts/baixar_municipios.py`), embutidos no
//! plugin: a escolha da área por nome não depende de rede.

use std::sync::OnceLock;

/// Um município: código do IBGE (7 dígitos), nome e unidade da federação.
#[derive(Debug)]
pub struct Municipio {
    pub codigo: &'static str,
    pub nome: &'static str,
    pub uf: &'static str,
    pub uf_nome: &'static str,
}

const CSV: &str = include_str!("../municipios.csv");

/// Todos os municípios, por estado (pelo nome do estado) e nome.
pub fn todos() -> &'static [Municipio] {
    static TODOS: OnceLock<Vec<Municipio>> = OnceLock::new();
    TODOS.get_or_init(|| {
        CSV.lines()
            .skip(1)
            .filter_map(|l| {
                let mut c = l.split(',');
                Some(Municipio { codigo: c.next()?, nome: c.next()?, uf: c.next()?, uf_nome: c.next()? })
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
