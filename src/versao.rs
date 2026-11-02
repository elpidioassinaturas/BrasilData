//! A verificação de atualização: a última release do BrasilData no GitHub contra esta versão.
//!
//! Um plugin não pode instalar outro (o RustGIS recusa os comandos `plugin.*` vindos de um
//! plugin): o plugin só diz se há versão nova e como instalá-la.

use serde_json::Value;

/// Esta versão (a do `Cargo.toml`, que o `gerar.py` mantém igual à do `plugin.json`).
pub const ESTA: &str = env!("CARGO_PKG_VERSION");

/// A última release, pela API do GitHub (JSON pequeno, com CORS; 60 pedidos por hora sem chave).
pub const URL_ULTIMA: &str = "https://api.github.com/repos/elpidioassinaturas/BrasilData/releases/latest";

/// A página das releases, para baixar o pacote.
pub const PAGINA: &str = "https://github.com/elpidioassinaturas/BrasilData/releases/latest";

/// Uma versão `x.y.z` (com ou sem o `v` da tag), para comparar.
pub fn numeros(v: &str) -> Option<(u64, u64, u64)> {
    let mut p = v.trim().trim_start_matches('v').split('.').map(|n| n.parse::<u64>().ok());
    let t = (p.next()??, p.next()??, p.next()??);
    p.next().is_none().then_some(t)
}

/// A versão da última release na resposta da API (`tag_name`), se ela for mais nova que esta.
pub fn mais_nova(resposta: &Value) -> Result<Option<String>, String> {
    let tag = resposta.get("tag_name").and_then(Value::as_str).ok_or("o GitHub não disse a última versão")?;
    let ultima = numeros(tag).ok_or_else(|| format!("a última versão no GitHub tem um nome inesperado: {tag}"))?;
    let esta = numeros(ESTA).ok_or("a versão deste plugin tem um nome inesperado")?;
    Ok((ultima > esta).then(|| tag.trim_start_matches('v').to_string()))
}

/// O texto para o usuário: em dia, ou a versão nova e como instalá-la.
pub fn mensagem(nova: Option<&str>) -> String {
    match nova {
        None => format!("BrasilData {ESTA}: é a versão mais recente."),
        Some(v) => format!(
            "Há uma versão nova do BrasilData: {v} (esta é a {ESTA}). Se ele veio do registro, use Atualizar em Plugins › Gerenciador de plugins › Instalado. Se veio de um arquivo, baixe o .rgplugin em {PAGINA} e instale-o em Gerenciador de plugins › Configurações › Instalar de arquivo…"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn as_versoes_se_comparam_pelos_numeros() {
        assert_eq!(numeros("v0.1.10"), Some((0, 1, 10)));
        assert_eq!(numeros("0.2.0"), Some((0, 2, 0)));
        assert_eq!(numeros("v0.1"), None);
        assert_eq!(numeros("v0.1.2-beta"), None);
        // 0.1.10 é mais nova que 0.1.9, o que a comparação de texto erraria.
        assert!(numeros("0.1.10") > numeros("0.1.9"));
    }

    #[test]
    fn a_resposta_diz_se_ha_versao_nova() {
        assert_eq!(mais_nova(&json!({"tag_name": "v99.0.0"})).unwrap().as_deref(), Some("99.0.0"));
        assert_eq!(mais_nova(&json!({"tag_name": format!("v{ESTA}")})).unwrap(), None);
        assert_eq!(mais_nova(&json!({"tag_name": "v0.0.1"})).unwrap(), None);
        assert!(mais_nova(&json!({"message": "API rate limit exceeded"})).is_err());
        assert!(mensagem(Some("99.0.0")).contains("Instalado"));
        assert!(mensagem(None).contains(ESTA));
    }
}
