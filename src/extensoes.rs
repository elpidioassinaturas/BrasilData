//! A extensão Imagens (Imagery) do RustGIS 0.7.0 (opengeos/RustGIS#1752).
//!
//! Do 0.7.0 em diante, `raster.process`, `raster.mosaic` e `raster.change`, entre outros, são da
//! extensão Imagens, que o app desktop e o app web trazem desligada (o `rustgis-cli` a liga). Com
//! ela desligada, o RustGIS recusa esses comandos, e um plugin não pode ligá-la
//! (`extension.enable` é do usuário). Por isso o plugin pergunta (`extension.list`) e, com a
//! Imagens desligada, recorta por comandos que ficam sempre ligados (módulo [`crate::recorte`]) e
//! junta e subtrai por eles ou pelas ferramentas do GeoLibre (`gp.run`), que o usuário instala à
//! parte. Este módulo lê as respostas; quem chama o RustGIS é o `comandos.rs`.

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

/// O nome da ferramenta do GeoLibre que falta, se o erro do `gp.run` é o de ele não estar instalado.
///
/// O RustGIS 0.8.0 diz "no tool 'geolibre:…' is published; locate or reload external tools from the
/// native app first", e, se o GeoLibre sumiu depois de aberto o app, "the GeoLibre runner isn't
/// installed" (aí sem o nome: vem vazio).
///
/// Args:
///     erro: a mensagem de erro.
pub fn falta_do_geolibre(erro: &str) -> Option<&str> {
    if let Some((_, resto)) = erro.split_once("no tool 'geolibre:") {
        return resto.split_once("' is published").map(|(nome, _)| nome);
    }
    erro.contains("the GeoLibre runner isn't installed").then_some("")
}

/// O que a mensagem de [`erro_da_ferramenta`] diz quando o GeoLibre não está instalado.
const NAO_ESTA: &str = "do GeoLibre não está no RustGIS";

/// Se `erro` é o aviso de [`erro_da_ferramenta`] de que o GeoLibre não está instalado.
pub fn falta_o_geolibre(erro: &str) -> bool {
    erro.contains(NAO_ESTA)
}

/// O erro de um comando que a Imagens faria, com a saída de ligá-la quando o que falta é o GeoLibre.
///
/// Args:
///     erro: a mensagem, já passada por [`erro_da_ferramenta`].
///
/// Returns:
///     A mensagem a mostrar.
pub fn ou_ligue_a_imagens(erro: String) -> String {
    if !erro.contains(NAO_ESTA) || erro.contains("Plugins › Extensões") {
        return erro;
    }
    format!("{erro}; ou ligue a extensão Imagens em Plugins › Extensões, que dispensa essa ferramenta")
}

/// O erro de uma ferramenta do `gp.run`, em português quando é o de o GeoLibre não estar instalado;
/// os outros ficam como vieram.
///
/// Args:
///     erro: a mensagem do RustGIS.
///
/// Returns:
///     A mensagem a mostrar.
pub fn erro_da_ferramenta(erro: String) -> String {
    let Some(nome) = falta_do_geolibre(&erro) else { return erro };
    let qual = if nome.is_empty() { "a ferramenta".to_string() } else { format!("a ferramenta {nome}") };
    format!(
        "{qual} {NAO_ESTA}: no painel Geoprocessamento, clique em Instalar as ferramentas Whitebox e GeoLibre (cerca de 20 MB) e rode de novo"
    )
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
    fn o_geolibre_que_falta_vira_aviso_em_portugues() {
        let e = "gp.run: no tool 'geolibre:clip_raster_to_polygon' is published; locate or reload external tools from the native app first";
        assert_eq!(falta_do_geolibre(e), Some("clip_raster_to_polygon"));
        let texto = erro_da_ferramenta(e.to_string());
        assert!(texto.starts_with("a ferramenta clip_raster_to_polygon do GeoLibre não está no RustGIS"), "{texto}");
        assert!(texto.contains("Geoprocessamento") && !texto.contains("published"));
        let com_saida = ou_ligue_a_imagens(texto.clone());
        assert!(com_saida.starts_with(&texto) && com_saida.contains("Plugins › Extensões"));
        // A saída é dita uma vez só, e não num erro que nada tem com o GeoLibre.
        assert_eq!(ou_ligue_a_imagens(com_saida.clone()), com_saida);
        assert_eq!(ou_ligue_a_imagens("outro erro".into()), "outro erro");
        assert_eq!(falta_do_geolibre("the GeoLibre runner isn't installed"), Some(""));
        // Outro erro da ferramenta, ou uma ferramenta que não é do GeoLibre, fica como veio.
        for e in ["geolibre:raster_calculator: unknown tool", "no tool 'x' is published; locate", "no raster layer 'x'"] {
            assert_eq!(falta_do_geolibre(e), None);
            assert_eq!(erro_da_ferramenta(e.to_string()), e);
        }
    }

    #[test]
    fn reconhece_a_recusa_do_0_7_0() {
        let e = "raster.process is part of the Imagery extension, which is off; turn it on in Plugins › Extensions (extension.enable)";
        assert!(recusa_de_extensao(e));
        assert!(!recusa_de_extensao("no raster layer 'x'"));
    }
}
