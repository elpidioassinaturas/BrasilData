//! O recorte de um raster pela área com a extensão Imagens do RustGIS desligada ([`crate::extensoes`]).
//!
//! O `raster.process clip` é da Imagens. Sem ela, o `geolibre:clip_raster_to_polygon` não serve de
//! primeira escolha: pede o GeoLibre instalado, leva 95 s em Minas Gerais a 250 m (4800 × 4019
//! células; a Imagens, menos de 1 s), troca umas 900 células da borda de Viçosa a 30 m e põe 0, um
//! valor, fora do contorno de um raster de bytes sem sem-dado declarado.
//!
//! O caminho daqui usa três comandos do próprio RustGIS que ficam sempre ligados:
//!
//! 1. `raster.rasterize` do contorno, na grade do raster: 1 nas células com o centro dentro, sem
//!    dado nas outras (a regra do clip da Imagens);
//! 2. `cube.reduce` do raster e da máscara como duas datas, com a primeira que tem valor (`first`,
//!    a do raster, que vem antes) e `minValid` 1: a célula sem valor numa das duas fica vazia;
//! 3. `raster.function bandMath` `b1`, que declara o sem-dado do clip da Imagens (o `cube.reduce`
//!    deixa NaN sem declarar, e há ferramentas do GeoLibre que não o tomam por sem-dado).
//!
//! Nada é interpolado: os valores saem iguais, célula a célula, aos do clip da Imagens. Só a
//! extensão (a do raster, e não a justa do contorno), o tipo (Float32) e a tabela de cores mudam; o
//! plugin põe o estilo depois. Este módulo monta os pedidos; quem chama o RustGIS é o `comandos.rs`.

use serde_json::{Value, json};

/// A data do raster no `cube.reduce`, que quer uma por camada: a primeira.
const DATA_DO_RASTER: &str = "2000-01-01";
/// A da máscara, depois da do raster.
const DATA_DA_MASCARA: &str = "2001-01-01";
/// As células a partir das quais o recorte do GeoLibre demora e o plugin avisa (uns 5 s por milhão
/// de células com o contorno de um estado).
pub const CELULAS_DEMORADAS: f64 = 2.0e6;

/// O `raster.rasterize` da máscara do contorno `poligono` na grade do raster `camada`.
pub fn mascara(poligono: &str, camada: &str, nome: &str) -> Value {
    json!({"layer": poligono, "grid": camada, "name": nome})
}

/// O `raster.rasterize` da máscara só no retângulo do contorno, com as células do raster.
///
/// Serve a um raster maior que o limite do `cube.reduce` (32 milhões de valores no desktop), como o
/// mosaico de quatro tiles do Copernicus em volta de um município.
///
/// Args:
///     poligono: a camada do contorno.
///     inteira: a resposta do `raster.rasterize` de [`mascara`], que diz o sistema, o canto e a
///         célula do raster.
///     nome: o nome da máscara nova.
///
/// Returns:
///     O pedido, ou `None` se o sistema do raster não tem código EPSG.
pub fn mascara_menor(poligono: &str, inteira: &Value, nome: &str) -> Option<Value> {
    let epsg: u32 = inteira["crs"].as_str()?.strip_prefix("EPSG:")?.parse().ok()?;
    let (canto, celula) = (&inteira["corner"], &inteira["cellSize"]);
    let par = |v: &Value| v.as_array().is_some_and(|a| a.len() == 2 && a.iter().all(Value::is_number));
    (par(canto) && par(celula)).then(|| json!({"layer": poligono, "crs": epsg, "cellSize": celula, "origin": canto, "name": nome}))
}

/// O `cube.reduce` que deixa os valores do raster só onde a máscara tem valor. A grade que sai é a
/// da primeira camada da lista.
///
/// Args:
///     camada: o raster, de uma banda.
///     mascara: a camada da máscara (o id).
///     menor: se a máscara é a de [`mascara_menor`], cuja grade é a que sai.
///     nome: o nome da camada nova.
pub fn combinar(camada: &str, mascara: &Value, menor: bool, nome: &str) -> Value {
    let (camadas, datas) = if menor {
        (json!([mascara, camada]), [DATA_DA_MASCARA, DATA_DO_RASTER])
    } else {
        (json!([camada, mascara]), [DATA_DO_RASTER, DATA_DA_MASCARA])
    };
    json!({"layers": camadas, "dates": datas, "stat": "first", "order": "date", "minValid": 1, "name": nome})
}

/// O `cube.reduce` que tira a banda `banda` (a primeira é 0) de um raster de `bandas` bandas, como
/// estava: as bandas fazem de datas, e só a dela entra. O `bandMath` deixaria vazia a célula que
/// outra banda não tem.
pub fn so_a_banda(camada: &str, banda: usize, bandas: usize, nome: &str) -> Value {
    let data = |i: usize| format!("{}-01-01", 2000 + i);
    let datas: Vec<String> = (0..bandas).map(data).collect();
    json!({"layer": camada, "dates": datas, "start": data(banda), "end": data(banda), "stat": "first", "order": "date", "name": nome})
}

/// O `raster.function` que copia a banda única da camada `camada` com o sem-dado declarado.
pub fn declarar_sem_dado(camada: &str, nome: &str) -> Value {
    json!({"layer": camada, "function": "bandMath", "expression": "b1", "name": nome})
}

/// O aviso de que o recorte do GeoLibre demorou, se o raster tem muitas células.
///
/// Args:
///     largura, altura: as do raster, em células.
///
/// Returns:
///     O aviso, ou `None` se o raster é pequeno.
pub fn aviso_de_demora(largura: f64, altura: f64) -> Option<String> {
    (largura * altura > CELULAS_DEMORADAS).then(|| {
        format!(
            "Este raster ({largura} × {altura} células) foi recortado pelo GeoLibre, que leva minutos numa área grande (o Windows pode marcar a janela como sem resposta). Com a extensão Imagens ligada (Plugins › Extensões), o recorte leva poucos segundos"
        )
    })
}

/// O motivo de o `cube.reduce` não ter recortado, em português quando é o limite de valores dele; os
/// outros ficam como vieram.
///
/// Args:
///     erro: a mensagem do RustGIS ("34229440 cells × 1 outputs is 34229440 values, more than
///         32000000: take a smaller area…").
///
/// Returns:
///     A mensagem a mostrar.
pub fn motivo(erro: &str) -> String {
    let numero = |t: &str| t.trim().chars().take_while(char::is_ascii_digit).collect::<String>().parse::<f64>().ok();
    let (Some((antes, _)), Some((_, limite))) = (erro.split_once(" cells × "), erro.split_once("more than ")) else {
        return erro.to_string();
    };
    let celulas = antes.rsplit(' ').next().and_then(numero);
    let (Some(n), Some(l)) = (celulas, numero(limite)) else { return erro.to_string() };
    format!("o raster tem {:.1} milhões de células e o limite é de {:.0} milhões", n / 1e6, l / 1e6).replace('.', ",")
}

/// A mensagem de um recorte que não saiu por nenhum dos caminhos.
///
/// Args:
///     motivo: o erro do caminho sem a Imagens e sem o GeoLibre.
///     geolibre: o erro do GeoLibre, já em português se é o de ele não estar instalado.
pub fn nao_saiu(motivo: &str, geolibre: &str) -> String {
    format!(
        "o recorte pela área não saiu com a extensão Imagens do RustGIS desligada. Ligue a Imagens em Plugins › Extensões e clique de novo, ou use uma área menor ou uma célula maior. (Sem ela: {motivo}. Pelo GeoLibre: {geolibre})"
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn o_raster_vem_antes_da_mascara_nas_datas_e_a_grade_e_a_da_primeira_camada() {
        let p = combinar("raster", &json!(7), false, "saida");
        assert_eq!(p["layers"], json!(["raster", 7]));
        assert_eq!(p["dates"], json!(["2000-01-01", "2001-01-01"]));
        assert_eq!((p["stat"].as_str(), p["order"].as_str(), p["minValid"].as_i64()), (Some("first"), Some("date"), Some(1)));
        let p = combinar("raster", &json!(7), true, "saida");
        assert_eq!(p["layers"], json!([7, "raster"]));
        assert_eq!(p["dates"], json!(["2001-01-01", "2000-01-01"]));
    }

    #[test]
    fn a_mascara_menor_fica_nas_celulas_do_raster() {
        let r = json!({"crs": "EPSG:10857", "corner": [5303750.0, 9747750.0], "cellSize": [250.0, 250.0], "layer": 3});
        let p = mascara_menor("área", &r, "m").unwrap();
        assert_eq!(p["crs"], 10857);
        assert_eq!(p["origin"], json!([5303750.0, 9747750.0]));
        assert_eq!(p["cellSize"], json!([250.0, 250.0]));
        assert!(p.get("grid").is_none() && p.get("extent").is_none());
        // Um sistema sem EPSG, ou uma resposta sem a grade: não há máscara menor.
        assert!(mascara_menor("área", &json!({"crs": "Albers", "corner": [0.0, 0.0], "cellSize": [1.0, 1.0]}), "m").is_none());
        assert!(mascara_menor("área", &json!({"crs": "EPSG:4326"}), "m").is_none());
    }

    #[test]
    fn a_banda_e_a_unica_data_do_periodo() {
        let p = so_a_banda("rgb", 1, 3, "b");
        assert_eq!(p["dates"], json!(["2000-01-01", "2001-01-01", "2002-01-01"]));
        assert_eq!((p["start"].as_str(), p["end"].as_str()), (Some("2001-01-01"), Some("2001-01-01")));
    }

    #[test]
    fn so_um_raster_grande_avisa_da_demora() {
        // Viçosa a 30 m e Minas Gerais a 250 m.
        assert!(aviso_de_demora(882.0, 692.0).is_none());
        assert!(aviso_de_demora(4800.0, 4019.0).unwrap().contains("4800 × 4019"));
    }

    #[test]
    fn o_limite_de_valores_e_dito_em_portugues() {
        let e = "cube.reduce: 34229440 cells × 1 outputs is 34229440 values, more than 32000000: take a smaller area, a coarser cell size or fewer outputs";
        assert_eq!(motivo(e), "o raster tem 34,2 milhões de células e o limite é de 32 milhões");
        assert_eq!(motivo("outro erro"), "outro erro");
    }

    #[test]
    fn a_mensagem_diz_o_que_ligar() {
        let t = nao_saiu("motivo", "outro");
        assert!(t.contains("Plugins › Extensões") && t.contains("motivo") && t.contains("outro"));
    }
}
