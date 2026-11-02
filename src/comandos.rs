//! Os comandos do plugin: o que fala com o RustGIS.

use rustgis_plugin::{Request, Value, geojson, host, json};

use crate::censo;
use crate::indicadores::{self, Fonte};
use crate::mapbiomas;
use crate::municipios::{self, Municipio};
use crate::saude;
use crate::versao;
use crate::{AREA, PREFIXO, TEMAS, Tema, falha_passageira, pedido_de_feicoes, sistema, tema_da_camada, tiles};

/// Os nomes das camadas dos censos, depois de [`PREFIXO`].
const INDICADORES: &str = "Indicadores por município";
const CENSO_SETORES: &str = "Setores censitários 2022";

/// O título do painel do plugin (o do `plugin.json`).
const PAINEL: &str = "Área e fontes";

/// Mais tiles do Copernicus que isto (cerca de 59 MB cada) é área grande demais para baixá-los inteiros.
const MAX_TILES: usize = 4;

fn handle(req: Request) -> Result<Value, String> {
    if let Some(t) = TEMAS.iter().find(|t| t.id == req.id) {
        return tema(t);
    }
    match req.id.as_str() {
        "area-municipio" => area_municipio(&req),
        "area-selecao" => area_selecao(),
        "painel" => painel(&req),
        "escolher-municipio" => escolher_municipio(&req),
        "elevacao" => elevacao(req.f64("celula").unwrap_or(30.0)),
        "fontes" => fontes(),
        "atualizacao" => atualizacao(),
        "indicadores" => indicadores(&fontes_de(&req)?),
        "indicadores-todos" => indicadores(&indicadores::TODAS),
        "censo-municipios" => indicadores(&[Fonte::Censo]),
        "censo-agro" => indicadores(&[Fonte::Agro]),
        "censo-agroecologia" => indicadores(&[Fonte::Agroeco]),
        "indicadores-pix" => indicadores(&[Fonte::Pix]),
        "indicadores-credito" => indicadores(&[Fonte::Credito]),
        "indicadores-tse" => indicadores(&[Fonte::Tse]),
        "indicadores-caged" => indicadores(&[Fonte::Caged]),
        "censo-setores" => censo_setores(),
        "saude-estabelecimentos" => estabelecimentos_de_saude(),
        "mapbiomas-cobertura" => cobertura_mapbiomas(req.f64("ano").map_or(mapbiomas::ULTIMO_ANO, |a| a as i64)),
        other => Err(format!("o BrasilData não tem o comando '{other}'")),
    }
}

rustgis_plugin::export!(handle);

/// Os nomes das camadas do mapa.
fn nomes() -> Result<Vec<String>, String> {
    Ok(host::layers()?.as_array().into_iter().flatten().filter_map(|l| l["name"].as_str().map(str::to_string)).collect())
}

/// Tira do mapa as camadas com estes nomes que existirem.
fn remover(alvo: &[&str]) -> Result<(), String> {
    let ha = nomes()?;
    let fora: Vec<&str> = alvo.iter().copied().filter(|n| ha.iter().any(|h| h == n)).collect();
    if !fora.is_empty() {
        host::run("layer.remove", json!({"layer": fora}))?;
    }
    Ok(())
}

/// Põe `fc` como a área, no lugar da anterior.
fn por_area(fc: Value) -> Result<Value, String> {
    let caixa = caixa_de(&fc).ok_or("a área não tem geometria")?;
    let texto = descrever(&fc);
    remover(&[AREA])?;
    host::add_layer(AREA, fc)?;
    // Escolhida pelo nome, a área pode estar longe da vista.
    host::run("layer.zoomTo", json!({"layer": AREA}))?;
    host::notify(&format!("Área do BrasilData: {texto}"));
    Ok(json!({"area": AREA, "bbox": caixa, "panel": {"items": {"area": {"text": texto}}}}))
}

/// O que a área é, pelos nomes nas feições (os do painel, ou os da malha do IBGE em
/// `nm_mun` e `sigla_uf`), e o retângulo.
fn descrever(fc: &Value) -> String {
    let n = fc["features"].as_array().map_or(0, Vec::len);
    texto_da_area(&nomes_nas_feicoes(fc), n, caixa_de(fc))
}

/// Os nomes das feições de `fc`: o do BrasilData (`nome`, `uf`) ou o de uma camada do IBGE
/// (`nm_mun`, `nm_meso`, `nm_uf`…, com `sigla_uf`).
fn nomes_nas_feicoes(fc: &Value) -> Vec<String> {
    fc["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let p = &f["properties"];
            let nome = p["nome"]
                .as_str()
                .or_else(|| p.as_object()?.iter().find(|(k, v)| k.starts_with("nm_") && v.is_string()).and_then(|(_, v)| v.as_str()))?;
            Some(match p["uf"].as_str().or(p["sigla_uf"].as_str()) {
                Some(uf) => format!("{nome} ({uf})"),
                None => nome.to_string(),
            })
        })
        .collect()
}

/// O texto do painel sobre a área: os nomes (ou quantos polígonos são) e o retângulo.
fn texto_da_area(nomes: &[String], poligonos: usize, caixa: Option<[f64; 4]>) -> String {
    let quais = match nomes.len() {
        0 if poligonos == 1 => "1 polígono".to_string(),
        0 => format!("{poligonos} polígonos"),
        1..=4 => nomes.join(", "),
        n => format!("{} e mais {}", nomes[..3].join(", "), n - 3),
    };
    match caixa {
        Some(c) => format!("{quais}\n{:.3}, {:.3} a {:.3}, {:.3}", c[0], c[1], c[2], c[3]),
        None => quais,
    }
}

/// A camada `nome` como o RustGIS a descreve (`info.project`): o retângulo, quantas feições e os
/// campos, sem o plugin ler as feições, que podem ser muitas e detalhadas.
fn camada_no_projeto(nome: &str) -> Result<Option<Value>, String> {
    let info = host::run("info.project", json!({"nameFilter": nome}))?;
    Ok(info["layers"].as_array().into_iter().flatten().find(|l| l["name"].as_str() == Some(nome)).cloned())
}

/// Os nomes da área (o que o painel mostra), lidos de uma cópia dela com o contorno bem
/// simplificado (1/50 da diagonal do retângulo `c`), que é apagada em seguida: só os atributos
/// interessam, e a cópia é leve em qualquer tamanho de área.
fn nomes_da_area(c: [f64; 4]) -> Result<Vec<String>, String> {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let diagonal_km = ((c[2] - c[0]) * 111.32 * lat.cos()).hypot((c[3] - c[1]) * 110.57);
    let copia = format!("{AREA} (nomes)");
    remover(&[&copia])?;
    let r = host::run("gp.run", json!({"tool": "simplify", "params": {"input": AREA, "tolerance": (diagonal_km / 50.0).max(0.1)}}))?;
    host::run(
        "layer.rename",
        json!({"layer": r.get("layer").cloned().ok_or("a simplificação não disse a camada que fez")?, "name": copia}),
    )?;
    let fc = host::features(&copia, false);
    remover(&[&copia])?;
    Ok(nomes_nas_feicoes(&fc?))
}

/// O retângulo `[oeste, sul, leste, norte]` da camada `nome`, pelo RustGIS.
fn extensao(nome: &str) -> Result<Option<[f64; 4]>, String> {
    Ok(camada_no_projeto(nome)?.and_then(|l| {
        let e = l["extent"].as_array()?;
        Some([e.first()?.as_f64()?, e.get(1)?.as_f64()?, e.get(2)?.as_f64()?, e.get(3)?.as_f64()?])
    }))
}

/// O retângulo `[oeste, sul, leste, norte]` das feições de `fc`.
fn caixa_de(fc: &Value) -> Option<[f64; 4]> {
    let pts: Vec<[f64; 2]> = fc["features"].as_array()?.iter().flat_map(|f| geojson::positions(&f["geometry"])).collect();
    geojson::bbox(&pts)
}

/// O município de `codigo`, ou de `nome` (e `uf`), como área.
fn area_municipio(req: &Request) -> Result<Value, String> {
    let m = match (req.params.get("codigo"), req.str("nome")) {
        (Some(c), _) => {
            let codigo = match c {
                Value::Number(n) => n.to_string(),
                Value::String(s) => s.trim().to_string(),
                _ => String::new(),
            };
            municipios::por_codigo(&codigo)
                .ok_or(format!("'{codigo}' não é o código de um município do IBGE (7 dígitos; 2211001 é Teresina)"))?
        }
        (None, Some(nome)) => municipios::por_nome(nome, req.str("uf"))?,
        (None, None) => {
            return Err(format!(
                "diga o município: {{\"nome\": \"Teresina\"}} (com \"uf\" quando o nome se repete) ou {{\"codigo\": 2211001}}; na janela, escolha-o no painel {PAINEL}"
            ));
        }
    };
    area_de(m)
}

/// O limite de `m` pela API de malhas do IBGE, como área.
fn area_de(m: &Municipio) -> Result<Value, String> {
    let url = format!(
        "https://servicodados.ibge.gov.br/api/v4/malhas/municipios/{}?formato=application/vnd.geo%2Bjson&qualidade=intermediaria",
        m.codigo
    );
    let r = host::call("host.fetch", json!({"url": url}))?;
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        // No app web, o RustGIS roda o comando de novo quando a resposta chegar.
        return Ok(r);
    }
    let mut fc = r.get("json").cloned().ok_or("a API de malhas do IBGE não respondeu GeoJSON")?;
    // O nome vai nas feições, para o painel dizer qual é a área.
    for f in fc["features"].as_array_mut().into_iter().flatten() {
        f["properties"] = json!({"nome": m.nome, "uf": m.uf, "codigo": m.codigo});
    }
    por_area(fc)
}

/// Os polígonos selecionados da camada ativa.
fn area_selecao() -> Result<Value, String> {
    let camadas = host::layers()?;
    let ativa = camadas
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["active"].as_bool() == Some(true))
        .ok_or("nenhuma camada ativa: selecione um polígono numa camada")?;
    let nome = ativa["name"].as_str().unwrap_or_default().to_string();
    if ativa["selected"].as_u64().unwrap_or(0) == 0 {
        return Err(format!("nada selecionado em '{nome}': selecione o polígono da área"));
    }
    if !ativa["geometry"].as_str().is_some_and(|g| g.contains("Polygon")) {
        return Err(format!("'{nome}' não é uma camada de polígonos: selecione o polígono da área numa camada de polígonos"));
    }
    // A área é a seleção com o contorno exato: o RustGIS recorta por ela. O plugin não lê o
    // polígono, que pode ter dezenas de milhares de vértices (uma mesorregião do IBGE) e passaria
    // da cota de instruções do plugin; os comandos pedem os dados pelo retângulo da área.
    remover(&[AREA])?;
    let r = host::run("select.layerFromSelection", json!({"layer": nome}))?;
    host::run("layer.rename", json!({"layer": r.get("layer").cloned().ok_or("a seleção não virou camada")?, "name": AREA}))?;
    let caixa = extensao(AREA)?.ok_or("a seleção não tem geometria")?;
    let nomes = nomes_da_area(caixa)?;
    host::run("layer.zoomTo", json!({"layer": AREA}))?;
    let texto = texto_da_area(&nomes, ativa["selected"].as_u64().unwrap_or(0) as usize, Some(caixa));
    host::notify(&format!("Área do BrasilData: {texto}"));
    Ok(json!({"area": AREA, "bbox": caixa, "panel": {"items": {"area": {"text": texto}}}}))
}

/// O retângulo da área, se houver; senão o da vista. E se é a área.
fn retangulo() -> Result<([f64; 4], bool), String> {
    if nomes()?.iter().any(|n| n == AREA) {
        return Ok((extensao(AREA)?.ok_or("a camada da área está vazia")?, true));
    }
    let v = host::view()?;
    let (lon, lat) = (v["center"][0].as_f64().ok_or("vista sem centro")?, v["center"][1].as_f64().ok_or("vista sem centro")?);
    Ok((crate::vista::vista_no_brasil(lon, lat, v["scale"].as_f64().ok_or("vista sem escala")?)?, false))
}

/// Quantas vezes um serviço que falhou de passagem é pedido de novo.
const TENTATIVAS: usize = 8;

/// `service.add` de `t`, só do retângulo `c`, com o nome `nome`, tentado de novo quando o
/// servidor falha de passagem.
///
/// O gateway do TerraBrasilis reparte os pedidos entre servidores, e um deles responde 502: em
/// 30/10, de 40 pedidos seguidos, 16 falharam, nunca mais de dois em fila. O `service.add` faz
/// dois pedidos (as capacidades e o `GetFeature`), e repetido logo em seguida pode cair sempre
/// no servidor ruim num dos dois. Por isso, depois de uma falha, as capacidades vêm à parte
/// (`host.fetch`) e vão no `text`: cada tentativa passa a ser um pedido só.
fn trazer(t: &Tema, nome: &str, c: [f64; 4]) -> Result<Value, String> {
    let mut p = json!({"url": t.servico, "type": t.tipo, "layer": t.camada, "name": nome, "bbox": c});
    // Um servidor `https` que anuncia o próprio endereço em `http` (a ANP): o erro cita esse
    // endereço, e as capacidades corrigidas resolvem.
    let em_http = t.servico.strip_prefix("https://").map(|r| format!("http://{}", r.split('/').next().unwrap_or(r)));
    let mut ultimo = String::new();
    for i in 1..=TENTATIVAS {
        match host::run("service.add", p.clone()) {
            Ok(r) => return Ok(r),
            // App web: o `service.add` de um comando não baixa feições.
            Err(e) if e.contains("in the web app") => return trazer_pela_web(t, nome, c),
            Err(e) if falha_passageira(&e) || em_http.as_ref().is_some_and(|h| e.contains(h.as_str())) => {
                host::log(&format!("{}: tentativa {i} de {TENTATIVAS} falhou: {e}", t.camada));
                ultimo = e;
            }
            Err(e) => return Err(e),
        }
        // Um ArcGIS não tem capacidades: só se tenta de novo.
        if t.tipo == "wfs" && p.get("text").is_none() {
            match capacidades(t, i)? {
                Capacidades::Texto(texto) => p["text"] = Value::String(texto),
                Capacidades::Esperando(r) => return Ok(r),
                Capacidades::Nenhuma => {}
            }
        }
    }
    Err(format!("{}: o servidor recusou {TENTATIVAS} pedidos seguidos; tente de novo daqui a pouco ({ultimo})", t.fonte))
}

/// As feições de `t` que cruzam `c`, como a camada `nome`, pelo `layer.addUrl`: o caminho do
/// app web, onde o `service.add` de um comando não baixa feições. O RustGIS baixa o GeoJSON fora
/// do plugin (sem o limite de 2 MB do `host.fetch`, e sem gastar o orçamento da chamada, que
/// não dava para os 4 MB da geologia de Teresina) e põe a camada no mapa quando ele chega; o
/// comando não espera. Responde `{"downloading": nome}` nesse caso, e como o `service.add`
/// (`{"features": n}`) quando a camada já veio.
fn trazer_pela_web(t: &Tema, nome: &str, c: [f64; 4]) -> Result<Value, String> {
    let r = host::run("layer.addUrl", json!({"url": pedido_de_feicoes(t, c), "name": nome, "zoom": false}))?;
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        return Ok(json!({"downloading": nome}));
    }
    Ok(json!({"features": r["features"].as_u64().unwrap_or(0)}))
}

enum Capacidades {
    Texto(String),
    /// App web: o RustGIS roda o comando de novo quando a resposta chegar.
    Esperando(Value),
    /// Não vieram (falha, ou mais de 2 MB): o `service.add` as pede de novo.
    Nenhuma,
}

/// As capacidades do WFS de `t`. O endereço leva o número da tentativa, porque o RustGIS guarda
/// por 30 segundos o erro de um endereço, e uma falha de passagem ficaria guardada.
fn capacidades(t: &Tema, tentativa: usize) -> Result<Capacidades, String> {
    let url = format!("{}?service=WFS&version=2.0.0&request=GetCapabilities&tentativa={tentativa}", t.servico);
    match host::call("host.fetch", json!({"url": url})) {
        Ok(r) if r.get("pending").and_then(Value::as_bool) == Some(true) => Ok(Capacidades::Esperando(r)),
        Ok(r) => Ok(r["text"].as_str().map_or(Capacidades::Nenhuma, |s| Capacidades::Texto(crate::com_https(s, t.servico)))),
        Err(e) => {
            host::log(&format!("{}: as capacidades não vieram: {e}", t.camada));
            Ok(Capacidades::Nenhuma)
        }
    }
}

/// Quantas feições tem a camada `nome` do mapa, se ela está lá.
fn feicoes_da_camada(nome: &str) -> Result<Option<u64>, String> {
    Ok(host::layers()?
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["name"].as_str() == Some(nome))
        .map(|l| l["features"].as_u64().unwrap_or(0)))
}

/// "1 feição", "N feições".
fn feicoes(n: u64) -> String {
    if n == 1 { "1 feição".into() } else { format!("{n} feições") }
}

/// Um tema vetorial: só o que cruza a área, recortado por ela; sem área, o que está na vista.
fn tema(t: &Tema) -> Result<Value, String> {
    if let Some(r) = ja_no_mapa(&t.camada_no_mapa(), t.recorta)? {
        return Ok(r);
    }
    if t.pedido_proprio() {
        return tema_proprio(t);
    }
    let (c, area) = retangulo()?;
    let na_area = area && t.recorta;
    let nome = t.camada_no_mapa();
    let bruto = format!("{nome} (sem recorte)");
    // App web: a camada baixada num clique anterior, que este clique recorta.
    let baixada = if na_area { feicoes_da_camada(&bruto)? } else { None };
    let r = match baixada {
        Some(n) => json!({"features": n}),
        None => {
            remover(&[&nome, &bruto])?;
            trazer(t, if na_area { &bruto } else { &nome }, c)?
        }
    };
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        // App web: o RustGIS roda o comando de novo quando as respostas chegarem.
        return Ok(r);
    }
    if r.get("downloading").is_some() {
        host::notify(&if na_area {
            format!("{nome}: baixando; quando a camada aparecer no mapa, clique de novo neste botão para recortá-la pela área")
        } else {
            format!("{nome}: baixando; a camada aparece no mapa quando chegar")
        });
        return Ok(r);
    }
    concluir(t, r["features"].as_u64().unwrap_or(0), area, na_area)
}

/// Se a camada `nome`, recortada pela área (`recorta`), já está no mapa para a área de agora: o
/// retângulo dela cabe no da área. Na janela, um comando de plugin congela a tela enquanto baixa,
/// sem sinal de que começou, e quem clica de novo baixaria tudo outra vez. Com a área trocada, o
/// retângulo não cabe, e o tema vem de novo.
fn ja_no_mapa(nome: &str, recorta: bool) -> Result<Option<Value>, String> {
    if !recorta || !nomes()?.iter().any(|n| n == AREA) {
        return Ok(None);
    }
    let (Some(camada), Some(area)) = (extensao(nome)?, extensao(AREA)?) else { return Ok(None) };
    if !retangulo_cabe(camada, area) {
        return Ok(None);
    }
    let curto = nome.trim_start_matches(PREFIXO);
    host::notify(&format!("{curto} já está no mapa, recortada por esta área; para trazer de novo, remova a camada"));
    Ok(Some(json!({"layer": nome, "already": true})))
}

/// Se o retângulo `dentro` cabe em `fora` (com folga de arredondamento).
fn retangulo_cabe(dentro: [f64; 4], fora: [f64; 4]) -> bool {
    let f = 1e-6;
    dentro[0] >= fora[0] - f && dentro[1] >= fora[1] - f && dentro[2] <= fora[2] + f && dentro[3] <= fora[3] + f
}

/// O fim de um tema, com as `lidas` feições no mapa: a camada `nome` do tema, ou a `(sem recorte)`
/// quando `na_area` (que é recortada pela área e renomeada).
fn concluir(t: &Tema, lidas: u64, area: bool, na_area: bool) -> Result<Value, String> {
    let nome = t.camada_no_mapa();
    let bruto = format!("{nome} (sem recorte)");
    let onde = if area { "na área" } else { "na vista" };
    if lidas == 0 {
        // Uma resposta sem feições entra como uma tabela sem geometria, não como camada.
        for n in [&nome, &bruto] {
            let _ = host::run("table.remove", json!({"table": n}));
        }
        remover(&[&nome, &bruto])?;
        host::notify(&format!("{nome}: nenhuma feição {onde} ({})", t.fonte));
        return Ok(json!({"layer": null, "features": 0}));
    }
    if !na_area {
        let onde = if area { "no retângulo da área" } else { onde };
        host::notify(&format!("{nome}: {} {onde} ({})", feicoes(lidas), t.fonte));
        return Ok(json!({"layer": nome, "features": lidas, "clipped": false}));
    }
    // Sem "output": para um plugin, o RustGIS conta essa chave como um arquivo e pede a
    // permissão `files` (`PLACE_KEYS` em plugins.rs), mesmo sendo o nome de uma camada.
    // A camada recortada é renomeada depois.
    let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto, "clip": AREA}}))?;
    let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": recortada, "name": nome}))?;
    remover(&[&bruto])?;
    let ficaram = host::layers()?
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["name"].as_str() == Some(nome.as_str()))
        .and_then(|l| l["features"].as_u64());
    match ficaram {
        Some(0) => {
            remover(&[&nome])?;
            host::notify(&format!("{nome}: nada dentro da área ({} no retângulo; {})", feicoes(lidas), t.fonte));
            Ok(json!({"layer": null, "read": lidas, "features": 0, "clipped": true}))
        }
        _ => {
            let n = ficaram.map_or(String::new(), |n| format!("{n} de "));
            host::notify(&format!(
                "{nome}: {n}{}, {} pela área ({})",
                feicoes(lidas),
                if ficaram == Some(1) { "recortada" } else { "recortadas" },
                t.fonte
            ));
            Ok(json!({"layer": nome, "read": lidas, "features": ficaram, "clipped": true}))
        }
    }
}

/// Mais feições que isto num tema pedido pelo plugin é área grande demais (o mesmo teto do
/// RustGIS para as páginas de um serviço).
const MAX_FEICOES: u64 = 100_000;

/// Um tema que o plugin pede ele mesmo (com filtro, campos, páginas ou uma camada por UF):
/// conta as feições de cada camada no retângulo, baixa as páginas com `layer.addUrl` (o RustGIS
/// baixa fora do plugin), junta-as (`gp.run merge`) e termina como os outros temas. No app web
/// as páginas chegam depois; um segundo clique junta e recorta.
fn tema_proprio(t: &Tema) -> Result<Value, String> {
    tema_proprio_de(t).map_err(crate::explicar)
}

/// [`tema_proprio`], sem a explicação dos erros.
fn tema_proprio_de(t: &Tema) -> Result<Value, String> {
    let (c, area) = retangulo()?;
    let na_area = area && t.recorta;
    let nome = t.camada_no_mapa();
    let bruto = format!("{nome} (sem recorte)");
    let destino = if na_area { &bruto } else { &nome };
    let parte = format!("{nome} (parte ");
    // App web: o que um clique anterior baixou.
    let ha = nomes()?;
    let partes: Vec<String> = ha.iter().filter(|n| n.starts_with(&parte)).cloned().collect();
    if let Some(primeira) = partes.first() {
        let de = primeira.rsplit(" de ").next().and_then(|s| s.trim_end_matches(')').parse::<usize>().ok()).unwrap_or(partes.len());
        if partes.len() < de {
            host::notify(&format!("{nome}: {} de {de} partes no mapa; clique de novo quando todas tiverem chegado", partes.len()));
            return Ok(json!({"downloading": nome, "parts": partes.len(), "of": de}));
        }
        let lidas = juntar(&partes, destino)?;
        return concluir(t, lidas, area, na_area);
    }
    if na_area && let Some(n) = feicoes_da_camada(&bruto)? {
        return concluir(t, n, area, na_area);
    }
    remover(&[&nome, &bruto])?;
    let ufs = if t.camada.contains("{uf}") {
        match ufs_da_area(c)? {
            Ok(u) => u,
            Err(p) => return Ok(p),
        }
    } else {
        Vec::new()
    };
    let camadas = t.camadas(&ufs);
    if camadas.is_empty() {
        return Err("nenhuma UF do IBGE no retângulo".into());
    }
    let contagens: Vec<String> = camadas.iter().map(|k| crate::pedido_de_contagem(t, k, c)).collect();
    let respostas = match buscar(&contagens)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let mut pedidos = Vec::new();
    let mut total = 0;
    for (k, r) in camadas.iter().zip(&respostas) {
        let n = crate::contagem(r.get("json"), r["text"].as_str()).map_err(|e| format!("{} ({k}): {e}", t.fonte))?;
        total += n;
        pedidos.extend(crate::paginas(n, t.pagina).into_iter().map(|p| crate::pedido(t, k, c, p)));
    }
    let onde = if area { "na área" } else { "na vista" };
    if total == 0 {
        host::notify(&format!("{nome}: nenhuma feição {onde} ({})", t.fonte));
        return Ok(json!({"layer": null, "features": 0}));
    }
    if total > MAX_FEICOES {
        return Err(format!("{nome}: {total} feições {onde}; o plugin traz até {MAX_FEICOES}: escolha uma área menor"));
    }
    let n = pedidos.len();
    let nomes_das_partes: Vec<String> =
        if n == 1 { vec![destino.clone()] } else { (1..=n).map(|i| format!("{nome} (parte {i} de {n})")).collect() };
    let mut baixando = 0;
    for (url, p) in pedidos.iter().zip(&nomes_das_partes) {
        let r = host::run("layer.addUrl", json!({"url": url, "name": p, "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            baixando += 1;
        }
    }
    if baixando > 0 {
        let partes = if n == 1 { String::new() } else { format!(" em {n} partes") };
        let depois = match (n > 1, na_area) {
            (true, true) => "juntá-las e recortá-las pela área",
            (true, false) => "juntá-las",
            (false, true) => "recortá-la pela área",
            (false, false) => "",
        };
        host::notify(&if depois.is_empty() {
            format!("{nome}: {total} feições baixando; a camada aparece no mapa quando chegar")
        } else {
            format!("{nome}: {total} feições baixando{partes}; quando tudo aparecer no mapa, clique de novo neste botão para {depois}")
        });
        return Ok(json!({"downloading": nome, "features": total, "parts": n}));
    }
    let lidas = if n == 1 { feicoes_da_camada(destino)?.unwrap_or(0) } else { juntar(&nomes_das_partes, destino)? };
    concluir(t, lidas, area, na_area)
}

/// Junta as camadas `partes` numa só, `destino`, e diz quantas feições ela tem.
fn juntar(partes: &[String], destino: &str) -> Result<u64, String> {
    if partes.len() == 1 {
        if partes[0] != destino {
            host::run("layer.rename", json!({"layer": partes[0], "name": destino}))?;
        }
    } else {
        let r = host::run("gp.run", json!({"tool": "merge", "params": {"inputs": partes}}))?;
        let junta = r.get("layer").cloned().ok_or("o merge não disse a camada que fez")?;
        host::run("layer.rename", json!({"layer": junta, "name": destino}))?;
        let p: Vec<&str> = partes.iter().map(String::as_str).collect();
        remover(&p)?;
    }
    Ok(feicoes_da_camada(destino)?.unwrap_or(0))
}

/// As siglas das UFs que o retângulo `c` cruza, pelo WFS do IBGE (ou o pedido pendente, no app
/// web).
fn ufs_da_area(c: [f64; 4]) -> Result<Result<Vec<String>, Value>, String> {
    let r = match buscar_varios(&[censo::url_municipios(c)])? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let mut ufs: Vec<String> = censo::municipios(&r[0]).into_iter().map(|m| m.2).collect();
    ufs.sort();
    ufs.dedup();
    Ok(Ok(ufs))
}

/// A fonte e a licença de cada camada do BrasilData no mapa, e o texto delas para o painel.
fn citacoes() -> Result<(Vec<Value>, String), String> {
    let mut lista: Vec<Value> = Vec::new();
    // As camadas e os mapas de mosaicos (o do MapBiomas).
    for n in nomes()?.into_iter().chain(mosaicos_no_mapa().unwrap_or_default()) {
        if let Some(t) = tema_da_camada(&n) {
            lista.push(json!({"layer": n, "source": t.fonte, "license": t.licenca, "service": t.servico, "featureType": t.camada}));
        } else if n == format!("{PREFIXO}{INDICADORES}") {
            // Uma citação por fonte cujos campos estão na camada.
            let fc = host::features(&n, false)?;
            let campos: Vec<String> = fc["features"][0]["properties"].as_object().map(|p| p.keys().cloned().collect()).unwrap_or_default();
            for f in indicadores::TODAS {
                if campos.iter().any(|c| f.eh_campo(c)) {
                    lista.push(json!({"layer": n, "source": f.citacao(), "license": f.licenca()}));
                }
            }
        } else if n.starts_with(&format!("{PREFIXO}Uso e cobertura ")) && n.ends_with(" (MapBiomas)") {
            lista.push(json!({"layer": n, "source": mapbiomas::CITACAO, "license": mapbiomas::LICENCA}));
        } else if n == camada_de_saude() {
            lista.push(json!({"layer": n, "source": saude::CITACAO, "license": saude::LICENCA}));
        } else if n == ELEVACAO {
            lista.push(json!({"layer": n, "source": CITACAO_ELEVACAO, "license": "licença do Copernicus DEM: uso livre e gratuito, com esta citação"}));
        } else if n.starts_with(&format!("{PREFIXO}{CENSO_SETORES}")) {
            lista.push(json!({"layer": n, "source": format!("IBGE, Censo Demográfico 2022, agregados por setores censitários (malha com atributos). {}", censo::CAMPOS_DOS_SETORES), "license": "uso livre com citação da fonte"}));
        }
    }
    let texto = if lista.is_empty() {
        "Nenhuma camada do BrasilData no mapa.".to_string()
    } else {
        lista
            .iter()
            .map(|f| {
                let camada = f["layer"].as_str().unwrap_or("").trim_start_matches(crate::PREFIXO);
                format!("{camada}: {} ({}).", f["source"].as_str().unwrap_or(""), f["license"].as_str().unwrap_or(""))
            })
            .collect::<Vec<_>>()
            .join(
                "
",
            )
    };
    Ok((lista, texto))
}

/// As citações das camadas do BrasilData no mapa, no painel.
fn fontes() -> Result<Value, String> {
    let (lista, texto) = citacoes()?;
    // A camada de indicadores tem uma citação por fonte.
    let mut camadas: Vec<&str> = lista.iter().filter_map(|f| f["layer"].as_str()).collect();
    camadas.dedup();
    host::notify(&match (camadas.len(), lista.len()) {
        (0, _) => "Nenhuma camada do BrasilData no mapa".to_string(),
        (1, 1) => format!("A fonte da camada está no painel {PAINEL}"),
        (1, n) => format!("As {n} fontes da camada estão no painel {PAINEL}"),
        (c, _) => format!("As fontes das {c} camadas estão no painel {PAINEL}"),
    });
    Ok(json!({"layers": lista, "panel": {"items": {"citacao": {"text": texto}}}}))
}

/// O nome da camada de elevação.
const ELEVACAO: &str = "BrasilData: Elevação (Copernicus GLO-30)";

/// A citação da elevação: a do Copernicus DEM, que a licença dele pede, e a da OpenTopography,
/// de onde vêm os tiles (os termos dela pedem que seja citada junto com a fonte).
const CITACAO_ELEVACAO: &str = "Copernicus DEM GLO-30, produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved. Tiles do espelho da OpenTopography (opentopography.org), com apoio da National Science Foundation";

/// O Copernicus GLO-30 na área, na grade do fuso, com células de `celula` metros.
fn elevacao(celula: f64) -> Result<Value, String> {
    if !(1.0..=10_000.0).contains(&celula) {
        return Err("celula é um tamanho em metros, de 1 a 10.000".into());
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (Área: seleção ou Área: município)".into());
    }
    let urls = tiles(c);
    if urls.len() > MAX_TILES {
        return Err(format!("a área pede {} tiles do Copernicus (cerca de 59 MB cada); o limite desta prova é {MAX_TILES}", urls.len()));
    }
    let nome = ELEVACAO;
    remover(&[nome])?;
    // Os tiles já baixados, de um clique anterior: rasters no mapa com o nome do tile.
    let rasters: Vec<String> = host::layers()?
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["raster"].as_bool() == Some(true))
        .filter_map(|l| l["name"].as_str().map(str::to_string))
        .collect();
    let mut partes: Vec<String> = Vec::new();
    let mut baixando = 0;
    for (i, (url, aws)) in urls.iter().enumerate() {
        let p = format!("BrasilData: tile {i}");
        if rasters.contains(&p) {
            partes.push(p);
            continue;
        }
        let pedir = |u: &str| host::run("layer.addUrl", json!({"url": u, "name": p, "whole": true, "zoom": false}));
        // O espelho da OpenTopography; se ele falhar onde o RustGIS espera o download (o
        // `rustgis-cli`), o bucket da AWS. Onde não espera, a falha só se vê depois.
        match pedir(url).or_else(|e| pedir(aws).map_err(|e2| format!("{e}; {e2}"))) {
            // A janela do RustGIS (e o app web) não espera o download: o tile chega depois, como
            // camada com este nome, e o próximo clique o encontra.
            Ok(r) if r.get("pending").and_then(Value::as_bool) == Some(true) => baixando += 1,
            Ok(_) => partes.push(p),
            // Mar: o Copernicus não tem tile ali.
            Err(e) => host::log(&format!("sem tile em {url}: {e}")),
        }
    }
    if baixando > 0 {
        let tiles = if baixando == 1 { "1 tile".to_string() } else { format!("{baixando} tiles") };
        host::notify(&format!(
            "Elevação: baixando {tiles} do Copernicus (cerca de 59 MB cada). Quando aparecerem no mapa como \"BrasilData: tile\", clique em Elevação de novo"
        ));
        return Ok(json!({"pending": baixando, "tiles": urls.len()}));
    }
    if partes.is_empty() {
        return Err("o Copernicus não tem dados na área".into());
    }
    let mut temporarias: Vec<String> = partes.clone();
    let mosaico = if partes.len() == 1 {
        partes[0].clone()
    } else {
        let m = "BrasilData: mosaico".to_string();
        host::run("raster.mosaic", json!({"inputs": partes, "name": m}))?;
        temporarias.push(m.clone());
        m
    };
    let recorte = "BrasilData: recorte".to_string();
    host::run("raster.process", json!({"layer": mosaico, "operation": "clip", "polygon": AREA, "name": recorte}))?;
    temporarias.push(recorte.clone());
    let crs = sistema(c);
    // O Copernicus tem ~30 m: para células maiores, a média dos pixels; senão, bilinear.
    let resampling = if celula > 45.0 { "average" } else { "bilinear" };
    let r = host::run("raster.project", json!({"layer": recorte, "crs": crs, "cellSize": celula, "resampling": resampling, "name": nome}))?;
    let t: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&t)?;
    host::notify(&format!("{nome}: EPSG:{crs}, células de {celula} m ({resampling})"));
    Ok(
        json!({"layer": nome, "crs": format!("EPSG:{crs}"), "cellSize": celula, "tiles": urls.len(), "width": r["width"], "height": r["height"]}),
    )
}

/// O texto do painel sobre a área do mapa.
fn area_no_mapa() -> Result<String, String> {
    if !nomes()?.iter().any(|n| n == AREA) {
        return Ok("Nenhuma ainda: escolha o município abaixo, ou selecione polígonos no mapa e use Área: seleção.".into());
    }
    let camada = camada_no_projeto(AREA)?.ok_or("a camada da área sumiu")?;
    let n = camada["features"].as_u64().unwrap_or(0) as usize;
    // Um município escolhido no painel vem leve, com o nome; uma área de seleção pode ser
    // detalhada demais para o plugin ler, e fica com o retângulo e quantos polígonos são.
    let do_painel = camada["fields"].as_array().is_some_and(|f| f.iter().any(|c| c.as_str().is_some_and(|c| c.starts_with("nome "))));
    if do_painel && n <= 50 {
        return Ok(descrever(&host::features(AREA, false)?));
    }
    Ok(texto_da_area(&[], n, extensao(AREA)?))
}

/// A árvore de estados do painel, com os municípios de todos (fechados até o usuário abrir um):
/// a busca do painel só procura nos nós que recebeu.
fn arvore() -> Value {
    let nos: Vec<Value> = municipios::ufs()
        .into_iter()
        .map(|(uf, nome)| {
            let filhos: Vec<Value> = municipios::da_uf(uf).map(|m| json!({"id": format!("mun:{}", m.codigo), "label": m.nome})).collect();
            json!({"id": format!("uf:{uf}"), "label": nome, "count": filhos.len(), "children": filhos})
        })
        .collect();
    Value::Array(nos)
}

/// O estado do painel, quando ele abre.
fn painel(_req: &Request) -> Result<Value, String> {
    Ok(json!({"panel": {"items": {
        "municipio": {"nodes": arvore()},
        "area": {"text": area_no_mapa()?},
        "citacao": {"text": citacoes()?.1},
        "versao": {"text": format!("BrasilData {}. O botão compara com a última versão publicada.", versao::ESTA)},
    }}}))
}

/// Um nó escolhido na árvore: um estado mostra os seus municípios; um município vira a área.
fn escolher_municipio(req: &Request) -> Result<Value, String> {
    let Some(no) = req.panel.get("municipio").and_then(Value::as_str) else {
        return Ok(json!({}));
    };
    let nos = arvore();
    let Some(m) = no.strip_prefix("mun:").and_then(municipios::por_codigo) else {
        // Um estado: os municípios aparecem debaixo dele; a busca acha o nome.
        return Ok(json!({"panel": {"items": {"municipio": {"nodes": nos}}, "values": {"municipio": no}}}));
    };
    let mut r = area_de(m)?;
    if r.get("pending").and_then(Value::as_bool) != Some(true) {
        r["panel"]["items"]["municipio"] = json!({"nodes": nos});
        r["panel"]["values"] = json!({"municipio": no});
    }
    Ok(r)
}

/// `host.fetch` de vários endereços: as respostas inteiras (`json`, `text`), na ordem, ou o
/// pedido pendente (app web), deixando até 8 pendentes de cada vez, o limite do RustGIS por
/// chamada.
fn buscar(urls: &[String]) -> Result<Result<Vec<Value>, Value>, String> {
    let mut respostas = Vec::new();
    let (mut pendente, mut esperando) = (None, 0);
    for url in urls {
        let r = host::call("host.fetch", json!({"url": url}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            pendente = Some(r);
            esperando += 1;
            if esperando == 8 {
                break;
            }
        } else {
            respostas.push(r);
        }
    }
    Ok(match pendente {
        Some(r) => Err(r),
        None => Ok(respostas),
    })
}

/// [`buscar_varios`], pedindo de novo, até 3 vezes, o que falhar com um erro do servidor: o
/// SICOR do Banco Central responde "Erro desconhecido" (500) às vezes, e a mesma consulta passa
/// logo depois. A nova tentativa vai com outro endereço (`&tentativa=N`), porque o RustGIS guarda
/// o erro de um endereço por 30 s.
fn buscar_com_tentativas(urls: &[String]) -> Result<Result<Vec<Value>, Value>, String> {
    let mut respostas = Vec::new();
    for url in urls {
        let mut i = 1;
        loop {
            let u = if i == 1 { url.clone() } else { format!("{url}&tentativa={i}") };
            match buscar_varios(&[u]) {
                Ok(Ok(mut r)) => {
                    respostas.push(r.remove(0));
                    break;
                }
                Ok(Err(p)) => return Ok(Err(p)),
                Err(e) if i < 4 && (falha_passageira(&e) || e.contains("http status: 500") || e.contains("HTTP 500")) => {
                    host::log(&format!("tentativa {i} falhou: {e}"));
                    i += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }
    Ok(Ok(respostas))
}

/// [`buscar`], com o `json` de cada resposta.
fn buscar_varios(urls: &[String]) -> Result<Result<Vec<Value>, Value>, String> {
    let respostas = match buscar(urls)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    urls.iter()
        .zip(respostas)
        .map(|(url, r)| r.get("json").cloned().ok_or(format!("{url}: a resposta não é JSON")))
        .collect::<Result<Vec<_>, _>>()
        .map(Ok)
}

/// O retângulo da área, que os censos pedem (sem área, a vista seria grande demais).
fn retangulo_da_area() -> Result<[f64; 4], String> {
    let (c, area) = retangulo()?;
    if !area {
        return Err(format!("escolha a área primeiro (painel {PAINEL}, ou Área: seleção)"));
    }
    Ok(c)
}

/// Se um erro do `host.fetch` é o de um limite da chamada (o tempo, ou o número de pedidos): o
/// que já veio fica guardado 10 minutos, e um novo clique continua de onde parou.
fn limite_da_chamada(e: &str) -> bool {
    e.contains("took longer than") || e.contains("may fetch at most")
}

/// O resultado de uma fonte: os campos, ou o pedido pendente (app web).
type Campos = Result<indicadores::PorMunicipio, Value>;

/// Os municípios que cruzam a área, com os indicadores das `fontes`, numa camada só: a
/// "Indicadores por município". Se ela já está no mapa com os mesmos municípios, os campos novos
/// se somam aos que ela tem; senão, os contornos vêm da API de malhas do IBGE.
fn indicadores(fontes: &[Fonte]) -> Result<Value, String> {
    let c = retangulo_da_area()?;
    let r = match buscar_varios(&[censo::url_municipios(c)])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let municipios = censo::municipios(&r[0]);
    if municipios.is_empty() {
        return Err("nenhum município do IBGE na área".into());
    }
    // Um pedido de contorno por município, e os das fontes, numa chamada de até 64 pedidos.
    if municipios.len() > 50 {
        return Err(format!("{} municípios na área; o plugin traz até 50 de cada vez: escolha uma área menor", municipios.len()));
    }
    let codigos: Vec<String> = municipios.iter().map(|m| m.0.clone()).collect();
    let nome = format!("{PREFIXO}{INDICADORES}");
    let mut feicoes = match feicoes_da_camada_de(&nome, &codigos)? {
        Some(f) => f,
        None => {
            let urls: Vec<String> = codigos.iter().map(|cd| censo::url_malha(cd)).collect();
            let malhas = match buscar_varios(&urls)? {
                Ok(r) => r,
                Err(p) => return Ok(p),
            };
            let mut f = Vec::new();
            for ((cd, nm, uf), malha) in municipios.iter().zip(&malhas) {
                let Some(geometria) = malha["features"][0].get("geometry").cloned() else {
                    host::log(&format!("{nm}: a API de malhas não deu o contorno"));
                    continue;
                };
                f.push(geojson::feature(geometria, json!({"cd_mun": cd, "nm_mun": nm, "sigla_uf": uf})));
            }
            // Os contornos ficam no mapa já: um novo clique, depois de um limite, os reaproveita.
            remover(&[&nome])?;
            host::add_layer(&nome, geojson::collection(f.clone()))?;
            f
        }
    };
    let mut campos = indicadores::PorMunicipio::new();
    let (mut feitas, mut faltam, mut erros) = (Vec::new(), Vec::new(), Vec::new());
    for &f in fontes {
        match campos_de(f, &codigos) {
            Ok(Ok(m)) => {
                indicadores::juntar(&mut campos, m);
                feitas.push(f.id());
            }
            // App web: o RustGIS roda o comando de novo quando as respostas chegarem.
            Ok(Err(pendente)) => return Ok(pendente),
            Err(e) if limite_da_chamada(&e) => faltam.push(f.id()),
            Err(e) => {
                host::log(&format!("{}: {e}", f.id()));
                erros.push(format!("{}: {e}", f.id()));
            }
        }
    }
    // Os campos de cada fonte em todos os municípios, nulos onde ela não tem o dado.
    let mut nomes_dos_campos: Vec<String> = Vec::new();
    for m in campos.values() {
        for k in m.keys() {
            if !nomes_dos_campos.contains(k) {
                nomes_dos_campos.push(k.clone());
            }
        }
    }
    for f in &mut feicoes {
        let cd = f["properties"]["cd_mun"].as_str().unwrap_or_default().to_string();
        let doc = campos.get(&cd);
        for k in &nomes_dos_campos {
            f["properties"][k] = doc.and_then(|d| d.get(k)).cloned().unwrap_or(Value::Null);
        }
    }
    let n = feicoes.len();
    if !feitas.is_empty() {
        remover(&[&nome])?;
        host::add_layer(&nome, geojson::collection(feicoes))?;
    }
    let mut aviso = format!("{nome}: {n} municípios");
    if !feitas.is_empty() {
        aviso.push_str(&format!("; com {}", feitas.join(", ")));
    }
    if !faltam.is_empty() {
        aviso.push_str(&format!(
            "; faltam {} (o tempo de uma chamada acabou): clique de novo, e o que já veio não é pedido outra vez",
            faltam.join(", ")
        ));
    }
    if !erros.is_empty() {
        aviso.push_str(&format!("; falharam {}", erros.join("; ")));
    }
    host::notify(&aviso);
    if feitas.is_empty() && faltam.is_empty() {
        return Err(erros.join("; "));
    }
    Ok(json!({"layer": nome, "features": n, "sources": feitas, "missing": faltam, "errors": erros}))
}

/// As feições da camada `nome`, se ela está no mapa com exatamente os municípios `codigos`.
fn feicoes_da_camada_de(nome: &str, codigos: &[String]) -> Result<Option<Vec<Value>>, String> {
    if feicoes_da_camada(nome)?.is_none() {
        return Ok(None);
    }
    let fc = host::features(nome, false)?;
    let feicoes = fc["features"].as_array().cloned().unwrap_or_default();
    let mut tem: Vec<&str> = feicoes.iter().filter_map(|f| f["properties"]["cd_mun"].as_str()).collect();
    tem.sort_unstable();
    let mut quer: Vec<&str> = codigos.iter().map(String::as_str).collect();
    quer.sort_unstable();
    Ok((tem == quer).then_some(feicoes))
}

/// O mês corrente do Pix (`AAAAMM`), que serve de relógio: o wasm não sabe a data.
fn mes_corrente() -> Result<Result<u32, Value>, String> {
    let r = match buscar_varios(&[indicadores::url_ultimo_mes()])? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    indicadores::ultimo_mes(&r[0]).map(Ok).ok_or_else(|| "o Banco Central não disse o mês mais recente do Pix".to_string())
}

/// Os campos das variáveis `lista` do SIDRA nos municípios `codigos`: um pedido por tabela e
/// classificação.
fn campos_sidra(lista: &[censo::Variavel], codigos: &[String]) -> Result<Result<indicadores::PorMunicipio, Value>, String> {
    let pedidos = censo::pedidos_sidra(lista, codigos);
    let urls: Vec<String> = pedidos.iter().map(|p| p.0.clone()).collect();
    let respostas = match buscar_varios(&urls)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let mut out = indicadores::PorMunicipio::new();
    for ((_, idx, cls), resposta) in pedidos.iter().zip(&respostas) {
        for (cd, vars) in &censo::valores(resposta, *cls) {
            let m = out.entry(cd.clone()).or_default();
            for &i in idx {
                let v = &lista[i];
                if let Some(x) = vars.get(&(v.variavel, censo::categoria_de(v))) {
                    m.insert(v.campo.into(), json!(x));
                }
            }
        }
    }
    Ok(Ok(out))
}

/// Os campos de uma fonte nos municípios `codigos`.
fn campos_de(f: Fonte, codigos: &[String]) -> Result<Campos, String> {
    use indicadores::*;
    macro_rules! esperar {
        ($e:expr) => {
            match $e? {
                Ok(v) => v,
                Err(p) => return Ok(Err(p)),
            }
        };
    }
    Ok(Ok(match f {
        Fonte::Censo => esperar!(campos_sidra(censo::VARIAVEIS, codigos)),
        Fonte::Agro => esperar!(campos_sidra(censo::AGRO, codigos)),
        Fonte::Agroeco => esperar!(campos_sidra(censo::AGROECO, codigos)),
        Fonte::Pix => {
            let mes = mes_anterior(esperar!(mes_corrente()));
            let r = esperar!(buscar_varios(&[url_pix(mes, codigos)]));
            campos_pix(&r[0], mes)
        }
        Fonte::Credito => {
            // O último ano completo: o anterior ao do mês corrente do Pix.
            let ano = esperar!(mes_corrente()) / 100 - 1;
            let respostas = esperar!(buscar_com_tentativas(&urls_credito(ano, codigos)));
            campos_credito(&respostas, ano, codigos)
        }
        Fonte::Tse => campos_tse(codigos),
        Fonte::Caged => campos_caged(codigos),
    }))
}

/// As fontes de `{"fontes": ["pix", …]}`, ou todas.
fn fontes_de(req: &Request) -> Result<Vec<Fonte>, String> {
    let Some(lista) = req.params.get("fontes").and_then(Value::as_array) else {
        return Ok(indicadores::TODAS.to_vec());
    };
    lista
        .iter()
        .map(|v| {
            let id = v.as_str().unwrap_or_default();
            Fonte::de(id).ok_or(format!("'{id}' não é uma fonte de indicadores: censo, pix, credito, tse ou caged"))
        })
        .collect()
}

/// Os setores censitários de 2022 da área, com a população e os domicílios: o GeoPackage de cada
/// UF que a área cruza, recortado por ela. No app web a baixa é do RustGIS e chega depois; um
/// segundo clique recorta.
fn censo_setores() -> Result<Value, String> {
    let c = retangulo_da_area()?;
    let ufs = match ufs_da_area(c)? {
        Ok(u) => u,
        Err(p) => return Ok(p),
    };
    if ufs.is_empty() {
        return Err("nenhum município do IBGE na área".into());
    }
    let mut feitas = Vec::new();
    let mut baixando = Vec::new();
    for uf in &ufs {
        let nome = format!("{PREFIXO}{CENSO_SETORES} ({uf})");
        let bruto = format!("{nome} (sem recorte)");
        if feicoes_da_camada(&bruto)?.is_none() {
            remover(&[&nome])?;
            let r = host::run("layer.addUrl", json!({"url": censo::url_setores(uf), "name": bruto, "zoom": false}))?;
            if r.get("pending").and_then(Value::as_bool) == Some(true) {
                baixando.push(uf.clone());
                continue;
            }
        }
        let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto, "clip": AREA}}))?;
        let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
        host::run("layer.rename", json!({"layer": recortada, "name": nome}))?;
        remover(&[&bruto])?;
        let n = feicoes_da_camada(&nome)?.unwrap_or(0);
        feitas.push(json!({"layer": nome, "features": n}));
    }
    if !baixando.is_empty() {
        host::notify(&format!(
            "Setores censitários de {}: baixando; quando a camada aparecer no mapa, clique de novo para recortá-la pela área",
            baixando.join(", ")
        ));
    }
    if !feitas.is_empty() {
        host::notify(&format!(
            "Setores censitários de 2022 recortados pela área ({}). {}",
            feitas.iter().map(|f| format!("{}: {}", f["layer"].as_str().unwrap_or(""), f["features"])).collect::<Vec<_>>().join("; "),
            censo::CAMPOS_DOS_SETORES
        ));
    }
    Ok(json!({"layers": feitas, "downloading": baixando}))
}

/// O nome da camada dos estabelecimentos de saúde.
fn camada_de_saude() -> String {
    format!("{PREFIXO}Estabelecimentos de saúde (CNES {})", saude::COMPETENCIA)
}

/// O CNES do Brasil inteiro, lido aos pedaços, antes do recorte.
const SAUDE_BRUTO: &str = "BrasilData: CNES do Brasil (sem recorte)";

/// Mais estabelecimentos que isto na área é área grande demais (o mesmo teto dos temas; a
/// cidade de São Paulo tem cerca de 43 mil, contando os desativados).
const MAX_SAUDE: u64 = MAX_FEICOES;

/// Os estabelecimentos de saúde ativos na área, do CNES pelo GeoParquet do geobr. Na janela, o
/// RustGIS não espera o arquivo: o primeiro clique o abre e o segundo recorta.
fn estabelecimentos_de_saude() -> Result<Value, String> {
    let (_, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (Área: seleção ou Área: município)".into());
    }
    let nome = camada_de_saude();
    if let Some(r) = ja_no_mapa(&nome, true)? {
        return Ok(r);
    }
    remover(&[&nome])?;
    if feicoes_da_camada(SAUDE_BRUTO)?.is_none() {
        let r = host::run("layer.addUrl", json!({"url": saude::URL, "name": SAUDE_BRUTO, "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) || r.get("downloading").is_some() {
            host::notify(&format!(
                "Estabelecimentos de saúde: abrindo o CNES do Brasil (geobr, 98 MB, lido aos pedaços). Quando \"{SAUDE_BRUTO}\" aparecer no mapa, clique de novo"
            ));
            return Ok(json!({"pending": true}));
        }
    }
    let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": SAUDE_BRUTO, "clip": AREA}}))?;
    let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
    let temporaria = format!("{nome} (todos)");
    host::run("layer.rename", json!({"layer": recortada, "name": temporaria}))?;
    let lidos = feicoes_da_camada(&temporaria)?.unwrap_or(0);
    if lidos > MAX_SAUDE {
        remover(&[&temporaria, SAUDE_BRUTO])?;
        return Err(format!("a área tem {lidos} estabelecimentos de saúde; o limite é {MAX_SAUDE}: escolha uma área menor"));
    }
    let contar = |onde: &str| -> Result<u64, String> {
        Ok(host::run("select.byAttributes", json!({"layer": temporaria, "where": onde}))?["selected"].as_u64().unwrap_or(0))
    };
    let desativados = contar(saude::DESATIVADO)?;
    let de_pessoa = contar(&saude::de_pessoa_ativos())?;
    let ficaram = contar(&saude::ficam())?;
    let fora = format!("{desativados} desativados e {de_pessoa} de pessoa física ou empresário individual deixados de fora");
    if ficaram == 0 {
        remover(&[&temporaria, SAUDE_BRUTO])?;
        host::notify(&format!("{nome}: nenhum estabelecimento ativo na área ({lidos} lidos; {fora})"));
        return Ok(json!({"layer": null, "read": lidos, "features": 0}));
    }
    let r = host::run("select.layerFromSelection", json!({"layer": temporaria}))?;
    let camada = r.get("layer").cloned().ok_or("a seleção não virou camada")?;
    host::run("layer.rename", json!({"layer": camada, "name": nome}))?;
    remover(&[&temporaria, SAUDE_BRUTO])?;
    // O RustGIS calcula os campos novos e apaga os outros: o plugin não lê os estabelecimentos.
    for (campo, tipo, expressao) in saude::campos() {
        host::run("edit.addField", json!({"layer": nome, "name": campo, "type": tipo}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": campo, "expression": expressao}))?;
    }
    let campos = host::run("layer.fields", json!({"layer": nome}))?;
    for c in campos.as_array().into_iter().flatten().filter_map(|c| c["name"].as_str()) {
        if !saude::fica_o_campo(c) {
            host::run("edit.deleteField", json!({"layer": nome, "field": c}))?;
        }
    }
    host::notify(&format!("{nome}: {ficaram} de {lidos} na área ({fora}; Ministério da Saúde, pelo geobr)"));
    Ok(json!({"layer": nome, "read": lidos, "features": ficaram, "inactive": desativados, "personal": de_pessoa}))
}

/// O nome do mapa de uso e cobertura de `ano`.
fn camada_de_cobertura(ano: i64) -> String {
    format!("{PREFIXO}Uso e cobertura {ano} (MapBiomas)")
}

/// Os nomes das camadas de mosaicos do mapa (o `host.layers` não as lista).
fn mosaicos_no_mapa() -> Result<Vec<String>, String> {
    Ok(host::run("map.tiles", json!({}))?.as_array().into_iter().flatten().filter_map(|t| t["name"].as_str().map(str::to_string)).collect())
}

/// O uso e a cobertura do MapBiomas em `ano`, como mapa de mosaicos com as cores da legenda.
fn cobertura_mapbiomas(ano: i64) -> Result<Value, String> {
    if !(mapbiomas::PRIMEIRO_ANO..=mapbiomas::ULTIMO_ANO).contains(&ano) {
        return Err(format!("ano é de {} a {}", mapbiomas::PRIMEIRO_ANO, mapbiomas::ULTIMO_ANO));
    }
    let r = host::call("host.fetch", json!({"url": mapbiomas::url_do_mapa(ano)}))?;
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        // No app web, o RustGIS roda o comando de novo quando a resposta chegar.
        return Ok(r);
    }
    let url = r.get("json").and_then(mapbiomas::mosaicos).ok_or_else(|| format!("a API do MapBiomas não deu o mapa de {ano}"))?;
    let nome = camada_de_cobertura(ano);
    // O endereço do Earth Engine vence em algumas horas: um novo clique troca o mapa.
    if mosaicos_no_mapa()?.contains(&nome) {
        host::run("map.removeTiles", json!({"name": nome}))?;
    }
    host::run(
        "map.addTiles",
        json!({"url": url, "kind": "raster", "name": nome, "attribution": "MapBiomas, Coleção 11 (CC BY 4.0)", "maxZoom": 14}),
    )?;
    host::notify(&format!(
        "{nome}: mapa para ver, com as cores da legenda oficial (brasil.mapbiomas.org); não é recortado pela área e vale por algumas horas"
    ));
    Ok(json!({"tiles": nome, "year": ano}))
}

/// Compara esta versão com a última release no GitHub e diz como atualizar.
fn atualizacao() -> Result<Value, String> {
    let r = host::call("host.fetch", json!({"url": versao::URL_ULTIMA}))?;
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        // No app web, o RustGIS roda o comando de novo quando a resposta chegar.
        return Ok(r);
    }
    let nova = versao::mais_nova(r.get("json").unwrap_or(&Value::Null))?;
    let texto = versao::mensagem(nova.as_deref());
    host::notify(&texto);
    Ok(json!({"version": versao::ESTA, "latest": nova, "panel": {"items": {"versao": {"text": texto}}}}))
}
