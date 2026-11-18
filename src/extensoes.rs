//! A extensão Imagens (Imagery) do RustGIS 0.7.0 (opengeos/RustGIS#1752).
//!
//! Do 0.7.0 em diante, `raster.process`, `raster.mosaic` e `raster.change`, entre outros, são da
//! extensão Imagens, que o app desktop e o app web trazem desligada (o `rustgis-cli` a liga). Com
//! ela desligada, o RustGIS recusa esses comandos, e um plugin não pode ligá-la
//! (`extension.enable` é do usuário). Por isso o plugin pergunta (`extension.list`) e, com a
//! Imagens desligada, recorta, junta e subtrai pelas ferramentas do GeoLibre (`gp.run`), que são
//! do RustGIS e ficam sempre ligadas. Este módulo lê a resposta; quem chama o RustGIS é o
//! `comandos.rs`.

use serde_json::Value;

/// O id da extensão Imagens.
pub const IMAGENS: &str = "imagery";

/// Se a resposta do `extension.list` diz que a Imagens está desligada.
///
/// Só um `"enabled": false` na entrada da Imagens conta: uma resposta sem ela, ou que não é uma
/// lista (um RustGIS mais novo que mude o formato), deixa os comandos de sempre, que dizem eles
/// mesmos se forem recusados.
///
/// Args:
///     lista: o que o `extension.list` devolveu.
///
/// Returns:
///     `true` quando a Imagens está desligada.
pub fn imagens_desligada(lista: &Value) -> bool {
    lista.as_array().into_iter().flatten().find(|e| e["id"] == IMAGENS).is_some_and(|e| e["enabled"] == Value::Bool(false))
}

/// Se um erro do RustGIS é a recusa de um comando de uma extensão desligada.
///
/// O texto é o do RustGIS 0.7.0 ("raster.process is part of the Imagery extension, which is
/// off; …"). Serve de segunda guarda: a Imagens pode ser desligada durante um comando.
///
/// Args:
///     erro: a mensagem de erro.
///
/// Returns:
///     `true` quando o comando foi recusado por estar numa extensão desligada.
pub fn recusa_de_extensao(erro: &str) -> bool {
    erro.contains("extension, which is off")
}

#[cfg(test)]
mod testes {
    use super::*;
    use serde_json::json;

    #[test]
    fn desligada_so_com_enabled_false() {
        let desligada = json!([{"id": "imagery", "name": "Imagery", "enabled": false, "default": false}]);
        let ligada = json!([{"id": "imagery", "enabled": true}]);
        assert!(imagens_desligada(&desligada));
        assert!(!imagens_desligada(&ligada));
        // Outra extensão desligada não conta; uma resposta sem a Imagens ou fora do formato também não.
        assert!(!imagens_desligada(&json!([{"id": "outra", "enabled": false}])));
        assert!(!imagens_desligada(&json!([])));
        assert!(!imagens_desligada(&json!({"id": "imagery", "enabled": false})));
        assert!(!imagens_desligada(&Value::Null));
    }

    #[test]
    fn reconhece_a_recusa_do_0_7_0() {
        let e = "raster.process is part of the Imagery extension, which is off; turn it on in Plugins › Extensions (extension.enable)";
        assert!(recusa_de_extensao(e));
        assert!(!recusa_de_extensao("no raster layer 'x'"));
    }
}
