//! O comando "Chuva agora (PARDIG)": a ligação de [`crate::pardig`] com o RustGIS, num arquivo
//! à parte do `comandos.rs` (usa as funções dele pelo `super`).

use rustgis_plugin::{Request, Value, host, json};

use super::{
    Pintura, avisar, buscar, extensao, formatar_decimal, grade_para, media_na_area, pintar, remover, retangulo, retangulo_cabe,
    tirar_janela,
};
use crate::grade::{self, GRADE};
use crate::pardig::{self, Quadro};
use crate::tempo;
use crate::{AREA, falha_passageira};

/// Se o RustGIS lê só uma janela de um store Zarr (o `bbox` do `layer.addZarr`).
fn le_janela_zarr() -> Result<bool, String> {
    let lista = host::run("info.commands", json!({}))?;
    let comandos = lista.get("commands").unwrap_or(&lista);
    Ok(comandos
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"].as_str() == Some("layer.addZarr"))
        .and_then(|c| c["params"].as_str())
        .is_some_and(|p| p.contains("\"bbox\"")))
}

/// Por que a busca do último quadro parou: um erro ou um pedido pendente (app web).
enum Parada {
    Erro(String),
    Pendente(Value),
}

/// Os quadros do último dia com quadros, pela listagem do bucket.
fn ultimos_quadros() -> Result<Result<Vec<Quadro>, Value>, String> {
    let r = pardig::ultimos(|inicio, max| match buscar(&[pardig::url_lista(inicio, max)]) {
        Ok(Ok(r)) => {
            let r = r.into_iter().next().unwrap_or_default();
            let j = match r.get("json") {
                Some(j) => j.clone(),
                None => serde_json::from_str(r["text"].as_str().unwrap_or("{}"))
                    .map_err(|e| Parada::Erro(format!("a listagem do PARDIG não veio em JSON: {e}")))?,
            };
            Ok(pardig::quadros_da_lista(&j))
        }
        Ok(Err(p)) => Err(Parada::Pendente(p)),
        Err(e) => Err(Parada::Erro(e)),
    });
    match r {
        Ok(v) => Ok(Ok(v)),
        Err(Parada::Pendente(p)) => Ok(Err(p)),
        Err(Parada::Erro(e)) => Err(format!("não consegui listar os quadros do PARDIG no Google Cloud Storage: {e}")),
    }
}

/// A chuva agora do PARDIG ([`pardig`]): a janela da área no quadro mais recente (ou no da hora
/// pedida), em mm/h, posta na grade do projeto e recortada pela área.
pub(super) fn chuva_agora(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !le_janela_zarr()? {
        return Err("a chuva do PARDIG precisa de um RustGIS que leia só a janela da área de um store Zarr (layer.addZarr com bbox, da 0.7.0 em diante): cada quadro cobre o mundo a 10 km".into());
    }
    let janela = pardig::janela(c)
        .ok_or("a área cruza o meridiano de Greenwich, e o PARDIG guarda a longitude de 0 a 360°: divida a área em duas")?;
    // Os quadros a tentar, do mais novo para trás: o pedido, ou os dois últimos do bucket (um
    // lote recém-publicado pode estar incompleto).
    let candidatos: Vec<Quadro> = match req.str("quando").map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) => vec![
            Quadro::de_brasilia(t)
                .ok_or_else(|| format!("a hora é dd/mm/aaaa hh:mm, no horário de Brasília (por exemplo 16/11/2026 07:30), não '{t}'"))?,
        ],
        None => match ultimos_quadros()? {
            Ok(v) if v.is_empty() => return Err("não achei nenhum quadro do PARDIG de 2026 em diante no bucket".into()),
            Ok(v) => v.into_iter().rev().take(2).collect(),
            Err(p) => return Ok(p),
        },
    };
    let mut erro = String::new();
    let mut lido = None;
    for q in candidatos {
        let o = pardig::origem(q).ok_or("o PARDIG começa em 01/01/1999")?;
        let nome_da_janela = pardig::janela_lida(q);
        if extensao(&nome_da_janela)?.is_some_and(|e| retangulo_cabe(c, e)) {
            lido = Some((q, o, nome_da_janela));
            break;
        }
        tirar_janela(&nome_da_janela)?;
        let mut pedido = json!({"url": o.url, "variable": pardig::VARIAVEL, "bbox": janela, "name": nome_da_janela});
        if let Some(p) = o.passo {
            pedido["index"] = json!({"time": p});
        }
        match host::run("layer.addZarr", pedido) {
            Ok(r) if r.get("pending").and_then(Value::as_bool) == Some(true) => {
                avisar(&format!("Chuva agora (PARDIG, {}): lendo a área no quadro; a camada vem quando chegar", q.brasilia()));
                return Ok(json!({"pending": 1}));
            }
            Ok(_) => {
                lido = Some((q, o, nome_da_janela));
                break;
            }
            Err(e) => {
                tirar_janela(&nome_da_janela)?;
                erro = if e.contains("404") || e.contains("not found") || e.contains("No such") {
                    format!("o PARDIG não tem o quadro de {} (horário de Brasília)", q.brasilia())
                } else if falha_passageira(&e) || e.contains("timeout") {
                    format!("o Google Cloud Storage não respondeu agora: tente de novo daqui a pouco ({e})")
                } else {
                    e
                };
            }
        }
    }
    let Some((q, o, nome_da_janela)) = lido else {
        return Err(erro);
    };
    let g = grade_para(c, celula)?;
    let saida = pardig::camada(q);
    let temporarias = [nome_da_janela.as_str(), pardig::EM_MM, pardig::NA_GRADE];
    remover(&[&saida])?;
    let (rn, rt) = tempo::rampa_chuva(pardig::ESCALA);
    let pintura = Pintura { rampa: (&rn, &rt), reserva: "YlGnBu", escala: (0.0, pardig::ESCALA), aceitos: (0.0, 1000.0) };
    let feito = (|| -> Result<(), String> {
        // De m/h para mm/h; o sem dado (NaN) fica de fora.
        host::run(
            "raster.function",
            json!({"layer": nome_da_janela, "function": "bandMath", "expression": "b1*1000", "name": pardig::EM_MM}),
        )?;
        host::run("raster.mask", json!({"layer": pardig::EM_MM, "min": 0.0, "max": 1000.0}))?;
        host::run(
            "raster.project",
            json!({"layer": pardig::EM_MM, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": pardig::reamostragem(g.celula), "name": pardig::NA_GRADE}),
        )?;
        super::recortar(pardig::NA_GRADE, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 1000.0}))?;
        pintar(&saida, &pintura);
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let maxima = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["max"].as_f64())
        .map(|m| format!("; máxima: {} mm/h", formatar_decimal(m)))
        .unwrap_or_default();
    avisar(&format!(
        "{saida}: a taxa de chuva estimada às {} (horário de Brasília), por IA a partir de satélites (PARDIG-{}, 0,1°), na grade do projeto, {}{}{maxima}. Dado experimental, não é alerta oficial",
        q.brasilia(),
        o.versao,
        g.texto(),
        media_na_area(&saida, "mm/h")
    ));
    Ok(
        json!({"layer": saida, "time": q.brasilia(), "utc": q.nome(), "version": o.versao, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}),
    )
}
