//! Os comandos do plugin: o que fala com o RustGIS.

use rustgis_plugin::{Request, Value, geojson, host, json};

use crate::censo;
use crate::datas::{self, Data};
use crate::dist;
use crate::estilo;
use crate::gedi;
use crate::grade::{self, Escolha, GRADE, Grade};
use crate::indicadores::{self, Fonte};
use crate::layout;
use crate::malhas;
use crate::mapbiomas;
use crate::municipios::{self, Territorio};
use crate::radar;
use crate::saude;
use crate::versao;
use crate::zoom;
use crate::{AREA, PREFIXO, TEMAS, Tema, falha_passageira, pedido_de_feicoes, tema_da_camada, tiles};

/// Os nomes das camadas dos censos, depois de [`PREFIXO`].
const INDICADORES: &str = "Indicadores por município";
const CENSO_SETORES: &str = "Setores censitários 2022";

/// O título do painel do plugin (o do `plugin.json`).
const PAINEL: &str = "Área de trabalho";

/// A árvore Limites políticos do painel (`municipios::arvore`).
const ARVORE: &str = "limites";

/// A camada em que o usuário desenha a área, até usar o desenho.
const DESENHO: &str = "BrasilData: área (desenho)";

/// Mais tiles do Copernicus que isto (cerca de 59 MB cada) é área grande demais para baixá-los inteiros.
const MAX_TILES: usize = 4;

/// Um comando do plugin, com o estilo que o usuário deu às camadas do BrasilData guardado: um tema
/// buscado de novo (outra área, outra vista) troca a camada por uma nova, e a nova fica com a
/// legenda, as cores, os rótulos e a transparência da antiga. O estilo fica na camada (e no
/// projeto, com ela); o plugin só o leva da antiga para a nova. O estado do painel, que o RustGIS
/// pede a cada abertura, não mexe em camadas.
fn handle(req: Request) -> Result<Value, String> {
    if req.id == "painel" {
        return executar(req);
    }
    guardar_estilos();
    let r = executar(req);
    if let Ok(v) = &r
        && !estilo::esperando(v)
    {
        devolver_estilos();
    }
    r
}

/// As camadas do BrasilData no mapa cujo estilo é do usuário ([`estilo::com_estilo`]).
fn camadas_com_estilo() -> Vec<String> {
    host::layers().map(|c| estilo::com_estilo(&c, &[DESENHO, CONTORNOS_RADAR, CONTORNOS_DIST, FONTES])).unwrap_or_default()
}

/// Guarda, como estilos do projeto, o estilo de cada camada do BrasilData no mapa. Uma falha aqui
/// só deixa a camada nova com o estilo de sempre.
fn guardar_estilos() {
    for n in camadas_com_estilo() {
        let _ = host::run("style.new", json!({"layer": n, "name": estilo::guardado(&n)}));
    }
}

/// Devolve a cada camada do BrasilData no mapa o estilo guardado com o nome dela, nova ou não, e
/// tira esse estilo do projeto. Sem um (a primeira vez de um tema) ou com um que não serve (outra
/// geometria), a camada fica como veio. O de uma camada que saiu do mapa e ainda não voltou (um
/// download no meio) fica guardado até ela voltar.
fn devolver_estilos() {
    for n in camadas_com_estilo() {
        let _ = host::run("style.apply", json!({"layer": n, "name": estilo::guardado(&n)}));
        let _ = host::run("style.remove", json!({"name": estilo::guardado(&n)}));
    }
}

/// Os comandos que não recortam nada pela área: a área, o painel, a grade, os textos e o mapa do
/// MapBiomas.
const SEM_RECORTE: &[&str] = &[
    "painel",
    "escolher-area",
    "area-municipio",
    "area-selecao",
    "area-desenho",
    "grade",
    "fontes",
    "atualizacao",
    "sobre",
    // O mapa do MapBiomas é do Brasil inteiro: a área não muda nada nele.
    "mapbiomas-cobertura",
];

fn executar(req: Request) -> Result<Value, String> {
    if !SEM_RECORTE.contains(&req.id.as_str())
        && let Some(pendente) = afinar_area()?
    {
        return Ok(pendente);
    }
    if let Some(t) = TEMAS.iter().find(|t| t.id == req.id) {
        return tema(t);
    }
    match req.id.as_str() {
        "area-municipio" => area_municipio(&req),
        "area-selecao" => area_selecao(),
        "area-desenho" => area_desenho(),
        "painel" => painel(&req),
        "escolher-area" => escolher_area(&req),
        "elevacao" => elevacao(req.f64("celula")),
        "radar-vv" => radar(&req, Some(radar::Pol::Vv)),
        "radar-vh" => radar(&req, Some(radar::Pol::Vh)),
        "radar-razao" => radar(&req, None),
        "dist-situacao" => perturbacao(&req, dist::Camada::Situacao),
        "dist-data" => perturbacao(&req, dist::Camada::Data),
        "dist-confianca" => perturbacao(&req, dist::Camada::Confianca),
        "dist-intensidade" => perturbacao(&req, dist::Camada::Intensidade),
        "grade" => grade_do_projeto(&req),
        "fontes" => fontes(),
        "atualizacao" => atualizacao(),
        "sobre" => sobre(),
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
        id if gedi::Variavel::do_comando(id).is_some() => {
            gedi_na_area(gedi::Variavel::do_comando(id).ok_or("comando do GEDI")?, req.f64("celula"))
        }
        "layout" => layout_do_mapa(&req),
        "mapbiomas-cobertura" => {
            cobertura_mapbiomas(ano_do_mapbiomas(&req), req.params.get("recorte").and_then(Value::as_bool) != Some(false))
        }
        "inpe-focos-periodo" => focos_no_periodo(&req),
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

/// Tira do mapa as camadas com os nomes `antigas`, menos a que uma chamada acaba de responder
/// (`veio`, o `layer` da resposta), que fica com o nome `destino`: com o nome já em uso, ela entrou
/// como "nome (2)". Uma camada nova só substitui a antiga depois da chamada: quando o RustGIS roda
/// o comando de novo com o download pronto, a camada que chegou já está no mapa, e tirá-la antes da
/// chamada dá o erro de camada desfeita.
fn substituir(antigas: &[&str], destino: &str, veio: &Value) -> Result<(), String> {
    let veio = Some(veio).filter(|v| !v.is_null());
    let camadas = host::layers()?;
    let lista = camadas.as_array().map_or(&[][..], Vec::as_slice);
    let fora: Vec<&Value> = lista
        .iter()
        .filter(|l| antigas.iter().any(|n| l["name"].as_str() == Some(n)) && Some(&l["id"]) != veio)
        .map(|l| &l["id"])
        .collect();
    if !fora.is_empty() {
        host::run("layer.remove", json!({"layer": fora}))?;
    }
    if let Some(l) = lista.iter().find(|l| Some(&l["id"]) == veio)
        && l["name"].as_str() != Some(destino)
    {
        host::run("layer.rename", json!({"layer": l["id"], "name": destino}))?;
    }
    Ok(())
}

/// Põe `fc` como a área, no lugar da anterior.
fn por_area(fc: Value) -> Result<Value, String> {
    let caixa = caixa_de(&fc).ok_or("a área não tem geometria")?;
    let texto = descrever(&fc);
    // A grade acompanha a área, com as mesmas escolhas; no automático, o fuso pode mudar. Ela
    // vem antes, para a área ficar como a camada ativa.
    let grade = match grade_no_mapa()? {
        Some(g) => Some(por_grade(Grade::nova(caixa, g.escolha, g.celula))?),
        None => None,
    };
    remover(&[AREA])?;
    host::add_layer(AREA, fc)?;
    // Escolhida pelo nome, a área pode estar longe da vista.
    host::run("layer.zoomTo", json!({"layer": AREA}))?;
    match &grade {
        Some(g) => host::notify(&format!("Área do BrasilData: {texto}. Grade refeita para ela: {}", g.texto())),
        None => host::notify(&format!("Área do BrasilData: {texto}")),
    }
    let mut r = json!({"area": AREA, "bbox": caixa, "panel": {"items": {"area": {"text": texto}}}});
    if let Some(g) = grade {
        r["panel"]["items"]["grade-texto"] = json!({"text": g.texto()});
    }
    Ok(r)
}

/// A grade que está no mapa, se houver.
fn grade_no_mapa() -> Result<Option<Grade>, String> {
    if feicoes_da_camada(GRADE)?.is_none() {
        return Ok(None);
    }
    Ok(Grade::ler(&host::features(GRADE, false)?))
}

/// Põe `g` no mapa, no lugar da grade anterior: só o contorno, para não cobrir o mapa.
fn por_grade(g: Grade) -> Result<Grade, String> {
    grade::conferir_celula(g.celula)?;
    remover(&[GRADE])?;
    host::add_layer(GRADE, g.camada())?;
    host::run("layer.noFill", json!({"layer": GRADE}))?;
    host::run("layer.outline", json!({"layer": GRADE, "color": "#6a3d9a", "width": 1.5}))?;
    Ok(g)
}

/// O botão Grade do projeto: a grade dos rasters do BrasilData na área, com a célula e o sistema
/// dos controles do painel, ou os `celula` e `crs` de um script.
fn grade_do_projeto(req: &Request) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro: a grade cobre o retângulo da área".into());
    }
    let posicao = |id: &str| req.panel.get(id).and_then(Value::as_u64).map(|i| i as usize);
    let celula = match req.f64("celula") {
        Some(m) => m,
        None => posicao("grade-celula").and_then(|i| grade::CELULAS.get(i).copied()).unwrap_or(grade::CELULA_PADRAO),
    };
    let escolha = match req.params.get("crs") {
        Some(v) => Escolha::do_parametro(v)?,
        None => posicao("grade-sistema").and_then(|i| Escolha::DO_PAINEL.get(i).copied()).unwrap_or(Escolha::Automatico),
    };
    let g = por_grade(Grade::nova(c, escolha, celula))?;
    host::notify(&format!("Grade do projeto: {}. Os rasters do BrasilData (Elevação) vão para ela", g.texto()));
    Ok(
        json!({"layer": GRADE, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "panel": {"items": {"grade-texto": {"text": g.texto()}}}}),
    )
}

/// A grade para um raster na área de retângulo `c`: a do mapa, refeita se não cobre mais a área
/// (a camada da área trocada à mão), ou, sem grade, a padrão (sistema automático) com
/// `celula` ou 30 m. Um `celula` diferente do da grade é recusado: os rasters ficariam
/// desalinhados.
fn grade_para(c: [f64; 4], celula: Option<f64>) -> Result<Grade, String> {
    let g = match grade_no_mapa()? {
        Some(g) => {
            if let Some(m) = celula
                && (m - g.celula).abs() > 1e-9
            {
                return Err(format!(
                    "a grade do projeto tem células de {}, não de {}: mude a grade (Grade do projeto) ou deixe o celula de fora",
                    grade::texto_da_celula(g.celula),
                    grade::texto_da_celula(m)
                ));
            }
            if g.cobre(c) { g } else { por_grade(Grade::nova(c, g.escolha, g.celula))? }
        }
        None => {
            let m = celula.unwrap_or(grade::CELULA_PADRAO);
            grade::conferir_celula(m)?;
            let g = por_grade(Grade::nova(c, Escolha::Automatico, m))?;
            host::notify(&format!("Grade do projeto criada: {}. Para outra, use Grade do projeto no painel Área de trabalho", g.texto()));
            g
        }
    };
    Ok(g)
}

/// O que a área é, pelos nomes nas feições (os do painel, ou os da malha do IBGE em
/// `nm_mun` e `sigla_uf`), e o retângulo.
fn descrever(fc: &Value) -> String {
    let n = fc["features"].as_array().map_or(0, Vec::len);
    texto_da_area(&nomes_nas_feicoes(fc), n, caixa_de(fc))
}

/// Os nomes das feições de `fc`: o do BrasilData (`nome`, `uf`) ou o de uma camada do IBGE
/// (`nm_mun`, `nm_meso`, `nm_uf`…, com `sigla_uf`). Um nome repetido em seguida conta uma vez: uma
/// grande região e o Brasil são os seus estados, todos com o nome da área.
fn nomes_nas_feicoes(fc: &Value) -> Vec<String> {
    let mut nomes: Vec<String> = fc["features"]
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
        .collect();
    nomes.dedup();
    nomes
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
    area_de(&m.into())
}

/// Se a camada `camada` (como [`camada_no_projeto`] a descreve) tem o campo `campo`.
fn tem_campo(camada: &Value, campo: &str) -> bool {
    let prefixo = format!("{campo} ");
    camada["fields"].as_array().is_some_and(|f| f.iter().any(|c| c.as_str().is_some_and(|c| c == campo || c.starts_with(&prefixo))))
}

/// Põe na camada `camada` o campo de texto `campo` com o valor `valor` em todas as feições.
fn campo_fixo(camada: &str, campo: &str, valor: &str) -> Result<(), String> {
    host::run("edit.addField", json!({"layer": camada, "name": campo, "type": "text"}))?;
    host::run("edit.calculateField", json!({"layer": camada, "field": campo, "expression": format!("'{}'", valor.replace('\'', "''"))}))?;
    Ok(())
}

/// Troca o contorno de um município escolhido como área pelo limite oficial do IBGE (a Malha
/// Municipal 2025, o WFS da camada Municípios), antes de um comando que recorta. O plugin leva os
/// municípios na malha intermediária, que se afasta da oficial até centenas de metros (815 m em
/// Viçosa): o recorte por ela deixava células faltando ou sobrando na divisa, e os temas não
/// batiam com a camada de Limites.
///
/// O limite oficial chega como camada pelo próprio RustGIS (`layer.addUrl`): lido pelo plugin, o
/// de um município grande passa da cota de instruções (Manaus, 457 KB; o Rio de Janeiro, 754 KB).
/// Pelo mesmo motivo, a área tratada é reconhecida pelo campo `malha` (`info.project`), sem ler as
/// feições. Se o WFS falhar, vale a malha máxima da API do IBGE (até 144 m da oficial em Viçosa,
/// e pequena); sem rede, segue com a embutida. As outras áreas (estado, meso e microrregião, que já
/// vêm na malha máxima; seleção e desenho) ficam como estão. No app, o pedido pendente: o limite
/// chega depois do clique.
fn afinar_area() -> Result<Option<Value>, String> {
    let Some(camada) = camada_no_projeto(AREA)? else {
        return Ok(None);
    };
    if tem_campo(&camada, "malha") || !tem_campo(&camada, "codigo") {
        return Ok(None);
    }
    // Uma área do painel, ainda na malha embutida: leve, o plugin a lê.
    let fc = host::features(AREA, false)?;
    let codigos = malhas::a_afinar(&fc, &[]);
    let [codigo] = codigos.as_slice() else {
        // Estado, meso ou microrregião (ou mais de um município): fica, e não se olha de novo.
        campo_fixo(AREA, "malha", "embutida")?;
        return Ok(None);
    };
    let oficial = format!("{AREA} (limite oficial {codigo})");
    if camada_no_projeto(&oficial)?.is_none() {
        match host::run("layer.addUrl", json!({"url": malhas::url_oficial(codigo), "name": oficial, "zoom": false})) {
            Ok(a) if a.get("pending").and_then(Value::as_bool) == Some(true) => {
                host::notify("Área: baixando o limite oficial do município (IBGE). Quando ele chegar, clique de novo");
                return Ok(Some(a));
            }
            Ok(_) => {}
            Err(e) => host::log(&format!("área: o limite oficial do IBGE não veio ({e})")),
        }
    }
    if camada_no_projeto(&oficial)?.is_some_and(|l| l["features"].as_u64().unwrap_or(0) > 0) {
        campo_fixo(&oficial, "codigo", codigo)?;
        campo_fixo(&oficial, "malha", "oficial")?;
        remover(&[AREA])?;
        host::run("layer.rename", json!({"layer": oficial, "name": AREA}))?;
        host::log("área: contorno trocado pelo limite oficial do IBGE");
        return Ok(None);
    }
    remover(&[&oficial])?;
    // A reserva: a malha máxima da API, pequena, lida pelo plugin.
    match buscar_varios(&[malhas::url_maxima(codigo)]) {
        Ok(Ok(r)) => {
            let novo = malhas::com_malhas(&fc, &[(codigo.clone(), r.into_iter().next().unwrap_or_default())], "maxima");
            if novo != fc {
                remover(&[AREA])?;
                host::add_layer(AREA, novo)?;
                host::log("área: contorno trocado pela malha máxima do IBGE");
                return Ok(None);
            }
        }
        Ok(Err(p)) => return Ok(Some(p)),
        Err(e) => host::log(&format!("área: a malha máxima do IBGE não veio ({e})")),
    }
    host::log("área: o recorte segue com a malha embutida");
    Ok(None)
}

/// O limite de `t`, que vem embutido no plugin, como área: uma feição por malha (uma grande região
/// e o Brasil são os seus estados, com o contorno detalhado). O nome vai em cada feição, para o
/// painel dizer qual é a área.
fn area_de(t: &Territorio) -> Result<Value, String> {
    let mut feicoes = Vec::with_capacity(t.codigos.len());
    for codigo in &t.codigos {
        let geometria = malhas::geometria(codigo).ok_or_else(|| format!("o plugin não tem o limite de {} ({codigo})", t.nome))?;
        let mut feicao = json!({"type": "Feature", "properties": {"nome": t.nome, "codigo": codigo}});
        if !t.uf.is_empty() {
            feicao["properties"]["uf"] = json!(t.uf);
        }
        feicao["geometry"] = geometria;
        feicoes.push(feicao);
    }
    let mut fc = json!({"type": "FeatureCollection"});
    fc["features"] = Value::Array(feicoes);
    por_area(fc)
}

/// `r` com a árvore sem marca, depois de uma área que veio de outro lugar (a seleção, um desenho):
/// um clique num nó escolhe de novo.
fn desmarcar_na_arvore(r: &mut Value) {
    r["panel"]["values"][ARVORE] = Value::Null;
}

/// Se há um desenho em andamento, desiste dele: a área veio de outro lugar.
fn desistir_do_desenho(r: &mut Value) -> Result<(), String> {
    if feicoes_da_camada(DESENHO)?.is_some() {
        remover(&[DESENHO])?;
        host::run("tool.set", json!({"tool": "explore"}))?;
        r["panel"]["items"]["area-desenho"] = json!({"label": null});
    }
    Ok(())
}

/// O botão Área: desenho. Na primeira vez, põe o usuário a desenhar: uma camada vazia, ativa, e a
/// ferramenta de criar polígono do RustGIS. O RustGIS não avisa o plugin quando um polígono fica
/// pronto; por isso, o botão passa a dizer Usar o desenho, e o segundo clique faz dele a área.
fn area_desenho() -> Result<Value, String> {
    let ja = feicoes_da_camada(DESENHO)?;
    if ja.is_some_and(|n| n > 0) {
        return usar_desenho();
    }
    if ja.is_none() {
        host::run("layer.template", json!({"template": "polygons", "name": DESENHO}))?;
    }
    host::run("layer.select", json!({"layer": DESENHO}))?;
    // A ferramenta só grava com a edição ligada, e sem ela o polígono se perderia sem aviso.
    host::run("edit.enable", json!({"on": true}))?;
    host::run("tool.set", json!({"tool": "createPolygon"}))?;
    host::notify(&format!("{}. A área só muda quando você usar o desenho", DICA_DO_DESENHO));
    Ok(json!({"layer": DESENHO, "drawing": true, "panel": {"items": {
        "area": {"text": area_no_mapa()?},
        "area-desenho": {"label": "Usar o desenho"},
    }}}))
}

/// O que fazer para desenhar a área.
const DICA_DO_DESENHO: &str =
    "Desenhando a área: clique nos cantos do polígono no mapa e dê um duplo clique para fechá-lo; depois, Usar o desenho";

/// Os polígonos desenhados, como área. São poucos e leves, e o plugin pode lê-los.
fn usar_desenho() -> Result<Value, String> {
    let fc = host::features(DESENHO, false)?;
    let desenhos: Vec<&Value> = fc["features"].as_array().into_iter().flatten().map(|f| &f["geometry"]).collect();
    if let Some(i) = desenhos.iter().position(|g| malhas::se_cruza(g)) {
        return Err(format!(
            "o desenho {} cruza a si mesmo e daria um recorte errado: desfaça-o (Ctrl+Z) e desenhe de novo, sem cruzar as linhas",
            i + 1
        ));
    }
    let n = desenhos.len();
    let feicoes: Vec<Value> = desenhos
        .iter()
        .enumerate()
        .map(|(i, g)| {
            let nome = if n == 1 { "Desenho livre".to_string() } else { format!("Desenho {}", i + 1) };
            json!({"type": "Feature", "properties": {"nome": nome}, "geometry": g})
        })
        .collect();
    let mut r = por_area(json!({"type": "FeatureCollection", "features": feicoes}))?;
    desistir_do_desenho(&mut r)?;
    desmarcar_na_arvore(&mut r);
    Ok(r)
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
    let mut r = json!({"area": AREA, "bbox": caixa, "panel": {"items": {"area": {"text": texto}}}});
    desistir_do_desenho(&mut r)?;
    desmarcar_na_arvore(&mut r);
    Ok(r)
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
    Ok(json!({"layer": r["layer"], "features": r["features"].as_u64().unwrap_or(0)}))
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
        Some(n) => {
            remover(&[&nome])?;
            json!({"features": n})
        }
        None => trazer(t, if na_area { &bruto } else { &nome }, c)?,
    };
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        // O RustGIS roda o comando de novo quando as respostas chegarem.
        return Ok(r);
    }
    if baixada.is_none() && r.get("downloading").is_none() {
        substituir(&[&nome, &bruto], if na_area { &bruto } else { &nome }, &r["layer"])?;
    }
    if let Some(aviso) = corte_do_servico(&r) {
        host::notify(&format!("{nome}: {aviso}; escolha uma área menor ({})", t.fonte));
    }
    if r.get("downloading").is_some() {
        // Um RustGIS que não roda o comando de novo põe a camada nova sem mais nada: a antiga sai
        // agora. Num que roda, esta volta é desfeita, e a antiga sai depois da chamada.
        remover(&[&nome, &bruto])?;
        host::notify(&if na_area {
            format!("{nome}: baixando; quando a camada aparecer no mapa, clique de novo neste botão para recortá-la pela área")
        } else {
            format!("{nome}: baixando; a camada aparece no mapa quando chegar")
        });
        return Ok(r);
    }
    concluir(t, r["features"].as_u64().unwrap_or(0), area, na_area)
}

/// O aviso, em português, de um `service.add` que trouxe só parte das feições (o RustGIS lê no
/// máximo uma página de um WFS: 50 mil feições na 0.5.0), que senão passaria calado.
fn corte_do_servico(r: &Value) -> Option<String> {
    let avisos = r.get("notices")?.as_array()?;
    let corte = avisos.iter().filter_map(Value::as_str).find(|a| a.contains("aren't read"))?;
    // "<camada>: the server sent 50000 of its 1552635 features (one page); ...": o nome da
    // camada pode ter números (BC250 2025).
    let depois = corte.split_once(" sent ").map_or("", |(_, d)| d);
    let numeros: Vec<&str> = depois.split(|c: char| !c.is_ascii_digit()).filter(|s| !s.is_empty()).collect();
    Some(match numeros.as_slice() {
        [veio, tem, ..] => format!("o servidor mandou só {veio} das {tem} feições do retângulo"),
        _ => "o servidor mandou só parte das feições do retângulo".into(),
    })
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
        data_legivel(t, &nome);
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
            data_legivel(t, &nome);
            Ok(json!({"layer": nome, "read": lidas, "features": ficaram, "clipped": true}))
        }
    }
}

/// O endereço que pede ao serviço dos focos o instante do último foco (o maior `datetime_gmt`).
fn url_do_ultimo_foco(t: &Tema) -> String {
    let estatistica = r#"[{"statisticType":"max","onStatisticField":"datetime_gmt","outStatisticFieldName":"ultimo"}]"#;
    format!("{}/{}/query?where=1%3D1&outStatistics={}&f=json", t.servico, t.camada, crate::escapar(estatistica))
}

/// O mês corrente dos focos: o do último foco que o serviço guarda (o plugin não tem relógio).
/// `Ok(None)` quando o serviço não tem foco nenhum (o começo de um mês); `Err` com o pedido
/// pendente no app web, onde a resposta chega depois.
fn mes_dos_focos(t: &Tema) -> Result<Result<Option<Data>, Value>, String> {
    let r = match buscar(&[url_do_ultimo_foco(t)])? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let json = r.first().and_then(|r| r.get("json"));
    if let Some(e) = json.and_then(|j| j.get("error")) {
        return Err(format!("{}: {}", t.fonte, e["message"].as_str().unwrap_or("o serviço respondeu com um erro")));
    }
    Ok(Ok(json.and_then(|j| j["features"][0]["attributes"]["ultimo"].as_i64()).map(Data::de_milissegundos)))
}

/// Os focos de calor do INPE de um período do mês corrente (UTC, os dois dias incluídos): `de` e
/// `ate` como "dd/mm/aaaa" ou como o dia do mês (1 a 31); sem eles, o mês inteiro. Por enquanto só
/// por script: o RustGIS não abre uma janela de parâmetros para um comando de plugin, e as datas
/// no painel Área de trabalho confundiam (ficavam fora do lugar). O serviço do IBAMA só
/// guarda o mês corrente, o do último foco. É o tema do mês corrente com outro filtro e outro
/// nome, um por período, para que dois períodos fiquem lado a lado no mapa.
fn focos_no_periodo(req: &Request) -> Result<Value, String> {
    let mes = TEMAS.iter().find(|t| t.id == "inpe-focos-mes").ok_or("o catálogo não tem os focos do mês corrente")?;
    let ultimo = match mes_dos_focos(mes)? {
        Ok(u) => u,
        Err(p) => return Ok(p),
    };
    // Um dia do período: uma data dd/mm/aaaa (do mês corrente) ou o dia do mês.
    let dia = |param: &str, padrao: u32| -> Result<u32, String> {
        if let Some(texto) = req.params.get(param).and_then(Value::as_str) {
            let d =
                Data::ler(texto).ok_or_else(|| format!("\"{param}\": \"{texto}\" não é uma data dia/mês/ano válida (como 01/11/2026)"))?;
            return match ultimo {
                Some(u) if (d.ano, d.mes) == (u.ano, u.mes) => Ok(d.dia),
                Some(u) => Err(format!(
                    "{}: o serviço só guarda os focos do mês corrente ({:02}/{}); escolha uma data de {} a {}",
                    d.texto(),
                    u.mes,
                    u.ano,
                    Data { dia: 1, ..u }.texto(),
                    u.texto()
                )),
                None => Err("o serviço ainda não tem focos neste mês".into()),
            };
        }
        let d = req.f64(param).unwrap_or(f64::from(padrao));
        if d.fract() != 0.0 || !(1.0..=31.0).contains(&d) {
            return Err(format!("o dia do período é dia/mês/ano ou um número inteiro de 1 a 31, não {d}"));
        }
        Ok(d as u32)
    };
    let fim_do_mes = ultimo.map_or(31, |u| datas::dias_no_mes(u.ano, u.mes));
    let (de, ate) = (dia("de", 1)?, dia("ate", fim_do_mes)?.min(fim_do_mes));
    if de > ate {
        let (a, b) = match ultimo {
            Some(u) => (Data { dia: de, ..u }.texto(), Data { dia: ate, ..u }.texto()),
            None => (format!("dia {de}"), format!("dia {ate}")),
        };
        return Err(format!("o período começa em {a} e termina em {b}; escolha um início antes do fim"));
    }
    // Com o mês conhecido, o filtro e o nome levam as datas; sem ele, o dia do mês.
    let (filtro, nome) = match ultimo {
        Some(u) => {
            let (a, b) = (Data { dia: de, ..u }, Data { dia: ate, ..u });
            let filtro = format!("datetime_gmt >= DATE '{}' AND datetime_gmt < DATE '{}'", a.iso(), b.seguinte().iso());
            let nome = if de == ate {
                format!("Focos de calor de {} (INPE)", a.texto())
            } else {
                format!("Focos de calor de {} a {} (INPE)", a.texto(), b.texto())
            };
            (filtro, nome)
        }
        None => (
            format!("EXTRACT(DAY FROM datetime_gmt) BETWEEN {de} AND {ate}"),
            format!("Focos de calor dos dias {de} a {ate} do mês corrente (INPE)"),
        ),
    };
    // Os textos de um tema são estáticos; os deste período ficam na memória do plugin, que é
    // pequena e é refeita com ele.
    let (filtro, nome): (&'static str, &'static str) = (Box::leak(filtro.into_boxed_str()), Box::leak(nome.into_boxed_str()));
    tema(&Tema { id: "inpe-focos-periodo", nome, filtro, ..*mes })
}

/// Os temas cuja data o ArcGIS manda em milissegundos desde 1970 (UTC), pelo começo do id, e o
/// campo dela: a camada ganha `data_hora_utc`, a data e a hora em texto ("2026-11-01 20:00").
const DATAS: &[(&str, &str)] = &[("inpe-focos-", "datetime_gmt")];

/// A expressão do RustGIS que escreve os milissegundos de `campo` como "AAAA-MM-DD HH:MM" (UTC).
/// As expressões não têm datas: o dia vem do número de dias desde 1º de março de 2000, pelo
/// algoritmo `civil_from_days` de Howard Hinnant, que sem séculos fica simples (certo de março
/// de 2000 ao fim de 2099, conferido dia a dia).
pub fn expressao_data(campo: &str) -> String {
    let dias = format!("(floor({campo} / 86400000) - 11017)");
    let ano = format!("floor(({dias} - floor({dias} / 1460)) / 365)");
    let no_ano = format!("({dias} - 365 * {ano} - floor({ano} / 4))");
    let mp = format!("floor((5 * {no_ano} + 2) / 153)");
    let dia = format!("({no_ano} - floor((153 * {mp} + 2) / 5) + 1)");
    let mes = format!("({mp} + 3 - 12 * floor({mp} / 10))");
    let ano = format!("(2000 + {ano} + floor({mp} / 10))");
    let resto = format!("({campo} % 86400000)");
    let hora = format!("floor({resto} / 3600000)");
    let minuto = format!("floor(({resto} % 3600000) / 60000)");
    let dois = |x: &str| format!("right('0' || to_string(to_int({x})), 2)");
    format!(
        "if({campo} IS NULL, NULL, to_string(to_int({ano})) || '-' || {} || '-' || {} || ' ' || {} || ':' || {})",
        dois(&mes),
        dois(&dia),
        dois(&hora),
        dois(&minuto)
    )
}

/// Acrescenta `data_hora_utc` à camada `nome` de um tema de [`DATAS`]. Uma falha não desfaz o
/// tema: é avisada, e a data fica só em milissegundos.
fn data_legivel(t: &Tema, nome: &str) {
    let Some((_, campo)) = DATAS.iter().find(|(id, _)| t.id.starts_with(id)) else { return };
    let feito = host::run("edit.addField", json!({"layer": nome, "name": "data_hora_utc", "type": "text"})).and_then(|_| {
        host::run("edit.calculateField", json!({"layer": nome, "field": "data_hora_utc", "expression": expressao_data(campo)}))
    });
    if let Err(e) = feito {
        host::notify(&format!(
            "{nome}: a data em texto (data_hora_utc) não foi calculada ({e}); {campo} está em milissegundos desde 1970, UTC"
        ));
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
    let r = tema_proprio_de(t).map_err(crate::explicar)?;
    if let (Some(campo), Some(nome)) = (zoom::campo(t), r["layer"].as_str()) {
        por_zoom(campo, nome, &r)?;
    }
    Ok(r)
}

/// Os filtros por escala de um tema por zoom na camada `nome`, que acabou de chegar (`r`): de
/// longe, só os rios principais; de perto, também os menores.
fn por_zoom(campo: &str, nome: &str, r: &Value) -> Result<(), String> {
    // As faixas com área até a que veio não filtram nada: com 0, valem todas, e a camada que
    // chegou num clique anterior (app web) também as recebe.
    let minima = r["areaMinima"].as_f64().unwrap_or(0.0);
    host::run("layer.displayFilters", json!({"layer": nome, "filters": zoom::filtros_de_exibicao(campo, minima)}))?;
    let curto = nome.trim_start_matches(PREFIXO);
    host::notify(&if minima > 0.0 {
        format!(
            "{curto}: só os rios com área de drenagem de {} ou mais (os menores não cabem nesta área; numa área menor eles vêm). De longe aparecem só os principais; aproxime o mapa para ver os menores",
            zoom::km2(minima)
        )
    } else {
        format!("{curto}: de longe aparecem só os rios principais (pela área de drenagem); aproxime o mapa para ver os menores")
    });
    Ok(())
}

/// [`tema_proprio`], sem a explicação dos erros.
fn tema_proprio_de(t: &Tema) -> Result<Value, String> {
    let (c, area) = retangulo()?;
    let na_area = area && t.recorta;
    let nome = t.camada_no_mapa();
    let bruto = format!("{nome} (sem recorte)");
    let destino = if na_area { &bruto } else { &nome };
    let parte = format!("{nome} (parte ");
    let ja_ha_partes = nomes()?.iter().any(|n| n.starts_with(&parte));
    // App web: a camada de uma página só, baixada num clique anterior, que este recorta.
    if na_area
        && !ja_ha_partes
        && let Some(n) = feicoes_da_camada(&bruto)?
    {
        remover(&[&nome])?;
        return concluir(t, n, area, na_area);
    }
    // Rios conforme o zoom: só os rios cuja área de drenagem cabe no orçamento.
    let filtrado;
    let mut minima = None;
    let t = match zoom::campo(t) {
        Some(campo) => {
            let m = match area_que_cabe(t, campo, c)? {
                Ok(m) => m,
                Err(p) => return Ok(p),
            };
            minima = Some(m);
            // O catálogo é estático: o filtro desta área vive até o fim do plugin (um por clique).
            filtrado = Tema { filtro: Box::leak(zoom::filtro(t, campo, m).into_boxed_str()), ..*t };
            &filtrado
        }
        None => t,
    };
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
        remover(&[&nome, &bruto])?;
        host::notify(&format!("{nome}: nenhuma feição {onde} ({})", t.fonte));
        return Ok(json!({"layer": null, "features": 0}));
    }
    if total > MAX_FEICOES {
        return Err(format!("{nome}: {total} feições {onde}; o plugin traz até {MAX_FEICOES}: escolha uma área menor"));
    }
    let n = pedidos.len();
    let nomes_das_partes: Vec<String> =
        if n == 1 { vec![destino.clone()] } else { (1..=n).map(|i| format!("{nome} (parte {i} de {n})")).collect() };
    // Nos apps, as partes que já chegaram (de um clique anterior, ou da volta do RustGIS quando
    // um download chega) ficam; só as que faltam são pedidas.
    let ha = if n > 1 { nomes()? } else { Vec::new() };
    let faltam: Vec<usize> = (0..n).filter(|&i| !ha.contains(&nomes_das_partes[i])).collect();
    let mut baixando = 0;
    let mut cheio = false;
    let mut veio = Value::Null;
    for &i in &faltam {
        let r = match host::run("layer.addUrl", json!({"url": pedidos[i], "name": nomes_das_partes[i], "zoom": false})) {
            Ok(r) => r,
            // O app espera no máximo 16 downloads de uma vez: as outras partes ficam para a volta
            // seguinte, quando estas tiverem chegado.
            Err(e) if crate::fila_cheia(&e) => {
                cheio = true;
                break;
            }
            Err(e) => return Err(e),
        };
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            baixando += 1;
        } else if n == 1 {
            veio = r["layer"].clone();
        }
    }
    // Um download pendente ainda não está no mapa (veja `tema`).
    substituir(&[&nome, &bruto], destino, &veio)?;
    if cheio && baixando == 0 && n == 1 {
        return Err(format!("{nome}: o RustGIS já espera {} downloads; clique de novo quando eles chegarem", crate::FILA_DO_APP));
    }
    if baixando > 0 || cheio {
        let chegaram = n - faltam.len();
        if n > 1 && (chegaram > 0 || cheio) {
            host::notify(&format!(
                "{nome}: {chegaram} de {n} partes no mapa, {baixando} baixando; as outras vêm quando estas chegarem (se nada mudar, clique de novo neste botão)"
            ));
            return Ok(json!({"downloading": nome, "features": total, "parts": n, "landed": chegaram}));
        }
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
    let mut r = concluir(t, lidas, area, na_area)?;
    if let Some(m) = minima {
        r["areaMinima"] = json!(m);
    }
    Ok(r)
}

/// A menor área a montante (`campo`, km²) cujos trechos de `t` em `c` cabem em [`zoom::ORCAMENTO`],
/// ou o pedido pendente (app web). Cada contagem é a mesma que [`tema_proprio_de`] faz depois com
/// o filtro, e o `host.fetch` a guarda: a escolhida não é pedida de novo.
fn area_que_cabe(t: &Tema, campo: &str, c: [f64; 4]) -> Result<Result<f64, Value>, String> {
    enum Parada {
        Erro(String),
        Pendente(Value),
    }
    let contar = |n: f64| -> Result<u64, Parada> {
        let com = Tema { filtro: Box::leak(zoom::filtro(t, campo, n).into_boxed_str()), ..*t };
        let r = match buscar(&[crate::pedido_de_contagem(&com, com.camada, c)]) {
            Ok(Ok(r)) => r,
            Ok(Err(p)) => return Err(Parada::Pendente(p)),
            Err(e) => return Err(Parada::Erro(e)),
        };
        let r = r.first().ok_or_else(|| Parada::Erro("a contagem não veio".into()))?;
        crate::contagem(r.get("json"), r["text"].as_str())
            .map_err(|e| Parada::Erro(format!("{} (área de drenagem de {} ou mais): {e}", t.fonte, zoom::km2(n))))
    };
    match zoom::area_minima(zoom::ORCAMENTO, contar) {
        Ok(m) => Ok(Ok(m)),
        Err(Parada::Pendente(p)) => Ok(Err(p)),
        Err(Parada::Erro(e)) => Err(e),
    }
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
        } else if n.starts_with(&format!("{PREFIXO}Focos de calor d"))
            && n.ends_with(" (INPE)")
            && let Some(t) = TEMAS.iter().find(|t| t.id == "inpe-focos-mes")
        {
            // Um período do mês corrente (`focos_no_periodo`): a fonte dos focos do mês.
            lista.push(json!({"layer": n, "source": t.fonte, "license": t.licenca, "service": t.servico, "featureType": t.camada}));
        } else if n.starts_with(&format!("{PREFIXO}Uso e cobertura ")) && n.ends_with(" (MapBiomas)") {
            lista.push(json!({"layer": n, "source": mapbiomas::CITACAO, "license": mapbiomas::LICENCA}));
        } else if n.starts_with(&format!("{PREFIXO}Uso e cobertura ")) && n.ends_with(" (MapBiomas, recortado)") {
            lista.push(json!({"layer": n, "source": mapbiomas::CITACAO_RECORTE, "license": mapbiomas::LICENCA_RECORTE}));
        } else if n == camada_de_saude() {
            lista.push(json!({"layer": n, "source": saude::CITACAO, "license": saude::LICENCA}));
        } else if radar::eh_do_radar(&n) {
            lista.push(json!({"layer": n, "source": radar::CITACAO, "license": radar::LICENCA}));
        } else if gedi::eh_do_gedi(&n) {
            lista.push(json!({"layer": n, "source": gedi::CITACAO, "license": gedi::LICENCA}));
        } else if dist::eh_do_dist(&n) {
            lista.push(json!({"layer": n, "source": dist::CITACAO, "license": radar::LICENCA}));
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
    let (lista, _) = citacoes()?;
    // A camada de indicadores tem uma citação por fonte.
    let mut camadas: Vec<&str> = lista.iter().filter_map(|f| f["layer"].as_str()).collect();
    camadas.dedup();
    // Ela entra como uma camada sem geometria (uma tabela na lista de camadas).
    remover(&[FONTES])?;
    if lista.is_empty() {
        host::notify("Nenhuma camada do BrasilData no mapa");
        return Ok(json!({"layers": lista}));
    }
    // Uma tabela no projeto (sem geometria), salva com ele: uma linha por citação.
    let linhas: Vec<Value> = lista
        .iter()
        .map(|f| {
            json!({"type": "Feature", "geometry": null, "properties": {
                "camada": f["layer"].as_str().unwrap_or("").trim_start_matches(PREFIXO),
                "fonte": f["source"],
                "licenca": f["license"],
                "servico": f["service"].as_str().unwrap_or(""),
                "camada_do_servico": f["featureType"].as_str().unwrap_or(""),
            }})
        })
        .collect();
    let mut fc = json!({"type": "FeatureCollection"});
    fc["features"] = Value::Array(linhas);
    host::add_layer(FONTES, fc)?;
    // A tabela de atributos dela, na janela (no rustgis-cli não há o que abrir).
    let _ = host::run("table.open", json!({"layer": FONTES}));
    host::notify(&match (camadas.len(), lista.len()) {
        (1, 1) => format!("A fonte da camada está na tabela {FONTES}"),
        (1, n) => format!("As {n} fontes da camada estão na tabela {FONTES}"),
        (c, _) => format!("As fontes das {c} camadas estão na tabela {FONTES}"),
    });
    Ok(json!({"layers": lista, "table": FONTES}))
}

/// A tabela das fontes (comando Fontes).
const FONTES: &str = "BrasilData: fontes";

/// O nome da camada de elevação.
const ELEVACAO: &str = "BrasilData: Elevação (Copernicus GLO-30)";

/// A citação da elevação: a do Copernicus DEM, que a licença dele pede, e a da OpenTopography,
/// de onde vêm os tiles (os termos dela pedem que seja citada junto com a fonte).
const CITACAO_ELEVACAO: &str = "Copernicus DEM GLO-30, produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved. Tiles do espelho da OpenTopography (opentopography.org), com apoio da National Science Foundation";

/// O Copernicus GLO-30 na área, na grade do projeto (sem grade, a padrão, com `celula` ou 30 m).
fn elevacao(celula: Option<f64>) -> Result<Value, String> {
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (Área: seleção ou Área: município)".into());
    }
    let g = grade_para(c, celula)?;
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
    // O Copernicus tem ~30 m: para células maiores, a média dos pixels; senão, bilinear.
    let resampling = if g.celula > 45.0 { "average" } else { "bilinear" };
    // O retângulo da grade como extensão e a origem padrão, [0, 0]: todo raster na grade tem as
    // mesmas linhas e colunas.
    let r = host::run(
        "raster.project",
        json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": nome}),
    )?;
    let t: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&t)?;
    host::notify(&format!("{nome}: na grade do projeto, {} ({resampling})", g.texto()));
    Ok(json!({
        "layer": nome, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "tiles": urls.len(),
        "width": r["width"], "height": r["height"], "corner": r["corner"],
    }))
}

/// A camada temporária dos contornos dos recortes do radar.
const CONTORNOS_RADAR: &str = "BrasilData: contornos do radar";

/// Os recortes do radar que cruzam o polígono da área, da passagem escolhida ([`radar::passagem`]),
/// ou o pedido pendente (app web).
fn recortes_do_radar(c: [f64; 4], dia: Option<Data>) -> Result<Result<Vec<radar::Recorte>, Value>, String> {
    let resposta = match buscar_varios(&[radar::url_busca(c, dia)])? {
        Ok(mut r) => r.remove(0),
        Err(p) => return Ok(Err(p)),
    };
    let todos = radar::recortes(&resposta);
    let quando = dia.map(|d| format!(" em {}", d.texto())).unwrap_or_default();
    if todos.is_empty() {
        return Err(format!("o ASF não tem recortes do OPERA RTC-S1 na área{quando}"));
    }
    // Os contornos numa camada, e o RustGIS seleciona os que cruzam o polígono da área.
    remover(&[CONTORNOS_RADAR])?;
    host::add_layer(CONTORNOS_RADAR, radar::camada_dos_contornos(&todos))?;
    let selecao = host::run("gp.run", json!({"tool": "selectByLocation", "params": {"input": CONTORNOS_RADAR, "select": AREA}}))
        .and_then(|_| host::features(CONTORNOS_RADAR, true));
    remover(&[CONTORNOS_RADAR])?;
    let indices: Vec<usize> =
        selecao?["features"].as_array().into_iter().flatten().filter_map(|f| f["properties"]["i"].as_u64().map(|i| i as usize)).collect();
    let cruzam: Vec<&radar::Recorte> = indices.iter().filter_map(|i| todos.get(*i)).collect();
    let (data, orbita) = radar::passagem(&cruzam).ok_or(format!("nenhum recorte do OPERA RTC-S1 cruza a área{quando}"))?;
    let escolhidos: Vec<radar::Recorte> = cruzam.into_iter().filter(|r| r.passagem() == (data, orbita)).cloned().collect();
    if escolhidos.len() > radar::MAX_RECORTES {
        return Err(format!(
            "a passagem de {} tem {} recortes na área; o limite é {}: escolha uma área menor",
            data.texto(),
            escolhidos.len(),
            radar::MAX_RECORTES
        ));
    }
    host::log(&format!(
        "radar: {} recortes cruzam a área, {} da passagem de {} (órbita {orbita})",
        indices.len(),
        escolhidos.len(),
        data.texto()
    ));
    Ok(Ok(escolhidos))
}

/// Uma polarização dos `recortes` na grade `g`, recortada pela área, como a camada `nome`; ou o
/// pedido pendente (app web).
fn radar_na_grade(recortes: &[radar::Recorte], p: radar::Pol, g: &Grade, c: [f64; 4], nome: &str) -> Result<Option<Value>, String> {
    // A lista de camadas do mapa custa caro ao plugin (cerca de 16 milhões de instruções, com
    // rasters no mapa): é lida uma vez, e não a cada recorte, senão uma mesorregião passa da cota.
    let ha = nomes()?;
    // As camadas intermediárias de uma vez anterior que tenha parado no meio.
    let (mosaico, recorte) = ("BrasilData: mosaico do radar".to_string(), "BrasilData: recorte do radar".to_string());
    let brutos: Vec<String> = recortes.iter().map(|r| format!("{}radar {} {}", PREFIXO, p.sigla(), r.rotulo())).collect();
    let filtrados: Vec<String> = brutos.iter().map(|b| format!("{b} filtrado")).collect();
    let na_grade: Vec<String> = brutos.iter().map(|b| format!("{b} na grade")).collect();
    let na_area: Vec<String> = brutos.iter().map(|b| format!("{b} na área")).collect();
    let (linear, log) = (format!("{nome} linear"), format!("{nome} log"));
    let velhas: Vec<&str> = filtrados
        .iter()
        .chain(&na_grade)
        .chain(&na_area)
        .map(String::as_str)
        .chain([mosaico.as_str(), recorte.as_str(), linear.as_str(), log.as_str()])
        .filter(|n| ha.iter().any(|h| h == n))
        .collect();
    if !velhas.is_empty() {
        host::run("layer.remove", json!({"layer": velhas}))?;
    }
    // Os recortes inteiros (cerca de 7 MB cada).
    let pedidos: Vec<(&str, &str)> = recortes.iter().zip(&brutos).map(|(r, b)| (r.url(p), b.as_str())).collect();
    let baixando = baixar_inteiros(&pedidos, &ha)?;
    if baixando > 0 {
        let n = if baixando == 1 { "1 recorte".to_string() } else { format!("{baixando} recortes") };
        host::notify(&format!(
            "Radar {}: baixando {n} do ASF (cerca de 7 MB cada). Quando aparecerem no mapa como \"BrasilData: radar\", clique de novo",
            p.sigla()
        ));
        return Ok(Some(json!({"pending": baixando, "bursts": recortes.len()})));
    }
    let mut temporarias: Vec<String> = Vec::new();
    // Para células maiores que as do OPERA (30 m), a média; senão, bilinear.
    let resampling = if g.celula > 45.0 { "average" } else { "bilinear" };
    let resultado = (|| -> Result<(), String> {
        for (((bruto, f), q), a) in brutos.iter().zip(&filtrados).zip(&na_grade).zip(&na_area) {
            // O filtro de Lee (5 × 5) tira o granulado (speckle) do radar, na geometria do próprio
            // recorte, antes de qualquer reamostragem, como se faz com radar.
            ferramenta("geolibre:lee_filter", json!({"input": bruto, "filter_size_x": 5, "filter_size_y": 5}), f)?;
            // Cada recorte vai para a rede da grade (o sistema e a célula dela, origem [0, 0]) no
            // próprio retângulo: os recortes, que podem estar em fusos UTM diferentes, passam a ter
            // o mesmo sistema e as mesmas células, e o mosaico os junta.
            host::run("raster.project", json!({"layer": f, "crs": g.epsg, "cellSize": g.celula, "resampling": resampling, "name": q}))?;
            // Só o retângulo da área: os recortes passam bastante dela, e o mosaico de uma
            // mesorregião inteira passaria do limite de 120 milhões de valores de um raster.
            host::run("raster.process", json!({"layer": q, "operation": "clip", "extent": c, "name": a}))?;
            temporarias.push(a.clone());
            // O bruto, o filtrado e o projetado saem já, para poupar memória.
            host::run("layer.remove", json!({"layer": [bruto, f, q]}))?;
        }
        let junto = if na_area.len() == 1 {
            na_area[0].clone()
        } else {
            host::run("raster.mosaic", json!({"inputs": na_area, "name": mosaico}))?;
            temporarias.push(mosaico.clone());
            mosaico.clone()
        };
        host::run("raster.process", json!({"layer": junto, "operation": "clip", "polygon": AREA, "name": recorte}))?;
        temporarias.push(recorte.clone());
        // Já na rede da grade: a extensão dela, sem reamostrar.
        host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "nearest", "name": linear}),
        )?;
        temporarias.push(linear.clone());
        // Em dB, como o radar costuma ser visto: 10 · log10 (o `bandMath` não tem logaritmo; o
        // GeoLibre tem).
        ferramenta("geolibre:log10", json!({"input": linear}), &log)?;
        temporarias.push(log.clone());
        host::run("raster.function", json!({"layer": log, "function": "bandMath", "expression": "b1 * 10", "name": nome}))?;
        let (min, max) = p.escala();
        let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": min, "max": max}));
        Ok(())
    })();
    let fora: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    if !fora.is_empty() {
        host::run("layer.remove", json!({"layer": fora})).map(|_| ()).or_else(|e| if resultado.is_err() { Ok(()) } else { Err(e) })?;
    }
    resultado.map(|()| None)
}

/// Pede ao RustGIS os arquivos do ASF `pedidos` (endereço, nome da camada) inteiros, como
/// camadas raster: um COG lido no lugar fica fora do alcance dos comandos `raster.*`. Os que já
/// estão no mapa (`ha`), de um clique anterior, ficam; os outros são pedidos todos de uma vez. A
/// janela do RustGIS (e o app web) não espera o download: eles chegam depois, com estes nomes, e
/// o próximo clique os encontra. Devolve quantos ficaram baixando.
fn baixar_inteiros(pedidos: &[(&str, &str)], ha: &[String]) -> Result<usize, String> {
    let mut baixando = 0;
    for (url, nome) in pedidos {
        if ha.iter().any(|h| h == nome) {
            continue;
        }
        let a = host::run("layer.addUrl", json!({"url": url, "name": nome, "whole": true, "zoom": false})).map_err(|e| {
            if e.contains("token") || e.contains("401") || e.contains("Earthdata") {
                format!("{e}. Os dados do ASF pedem o login Earthdata: Início › Dados web › NASA Earthaccess › Earthdata Login")
            } else {
                e
            }
        })?;
        if a.get("pending").and_then(Value::as_bool) == Some(true) {
            baixando += 1;
        }
    }
    Ok(baixando)
}

/// O que o radar e o DIST-S1 conferem antes de buscar: a célula pedida, a data (até o #1547, só
/// por script), a área (que tem de existir e caber, `o_que` diz de quem é o limite) e o login
/// Earthdata.
fn area_do_asf(req: &Request, o_que: &str) -> Result<PedidoAsf, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let dia = match req.str("data") {
        Some(t) => Some(Data::ler(t).ok_or(format!("data '{t}': use dia/mês/ano, como 26/10/2026"))?),
        None => None,
    };
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err(format!("escolha a área primeiro (painel {PAINEL})"));
    }
    if !radar::cabe(c) {
        return Err(format!(
            "a área passa de {:.1}° de lado; {o_que} lê no máximo cerca de 400 km (uma mesorregião). Escolha uma área menor",
            radar::MAX_GRAUS
        ));
    }
    // Sem o token, os arquivos são recusados: avisa antes de buscar.
    if let Ok(s) = host::run("earthdata.status", json!({}))
        && s["token"].as_bool() == Some(false)
    {
        return Err(format!(
            "{o_que} do ASF pede o login Earthdata, que é grátis: crie um token em urs.earthdata.nasa.gov (Generate Token) e cole-o no RustGIS, em Início › Dados web › NASA Earthaccess › Earthdata Login"
        ));
    }
    Ok(PedidoAsf { c, celula, dia })
}

/// O que [`area_do_asf`] conferiu: o retângulo da área, a célula pedida e a data.
struct PedidoAsf {
    c: [f64; 4],
    celula: Option<f64>,
    dia: Option<Data>,
}

/// A camada temporária dos contornos dos produtos do DIST-S1.
const CONTORNOS_DIST: &str = "BrasilData: contornos do DIST-S1";

/// O produto mais recente de cada tile do DIST-S1 que cruza o polígono da área (até o dia `ate`,
/// se dado), ou o pedido pendente (app web).
fn produtos_do_dist(c: [f64; 4], ate: Option<Data>) -> Result<Result<Vec<dist::Produto>, Value>, String> {
    let resposta = match buscar_varios(&[dist::url_busca(c, ate)])? {
        Ok(mut r) => r.remove(0),
        Err(p) => return Ok(Err(p)),
    };
    let todos = dist::mais_recentes(&resposta);
    let quando = ate.map(|d| format!(" até {}", d.texto())).unwrap_or_default();
    if todos.is_empty() {
        return Err(format!("o ASF não tem alertas do OPERA DIST-S1 na área{quando} (o monitoramento começou em 04/01/2026)"));
    }
    // Os contornos numa camada, e o RustGIS seleciona os tiles que cruzam o polígono da área.
    remover(&[CONTORNOS_DIST])?;
    host::add_layer(CONTORNOS_DIST, dist::camada_dos_contornos(&todos))?;
    let selecao = host::run("gp.run", json!({"tool": "selectByLocation", "params": {"input": CONTORNOS_DIST, "select": AREA}}))
        .and_then(|_| host::features(CONTORNOS_DIST, true));
    remover(&[CONTORNOS_DIST])?;
    let mut indices: Vec<usize> =
        selecao?["features"].as_array().into_iter().flatten().filter_map(|f| f["properties"]["i"].as_u64().map(|i| i as usize)).collect();
    // Na ordem de `todos` (o mais recente primeiro), que o mosaico respeita.
    indices.sort_unstable();
    let escolhidos: Vec<dist::Produto> = indices.iter().filter_map(|i| todos.get(*i)).cloned().collect();
    if escolhidos.is_empty() {
        return Err(format!("nenhum tile do OPERA DIST-S1 cruza a área{quando}"));
    }
    if escolhidos.len() > dist::MAX_TILES {
        return Err(format!("a área cruza {} tiles do DIST-S1; o limite é {}: escolha uma área menor", escolhidos.len(), dist::MAX_TILES));
    }
    host::log(&format!(
        "dist: {} tiles cruzam a área: {}",
        escolhidos.len(),
        escolhidos.iter().map(|p| format!("{} de {}", p.tile, p.data.texto())).collect::<Vec<_>>().join(", ")
    ));
    Ok(Ok(escolhidos))
}

/// Uma camada dos `produtos` na grade `g`, recortada pela área, como a camada `nome`; ou o
/// pedido pendente. `ate` é a data do produto mais recente (para a camada da data).
fn dist_na_grade(
    produtos: &[dist::Produto],
    camada: dist::Camada,
    g: &Grade,
    c: [f64; 4],
    nome: &str,
    ate: Data,
    paleta: dist::Paleta,
) -> Result<Option<Value>, String> {
    // Lida uma vez: a lista de camadas custa caro ao plugin com rasters no mapa (ver o radar).
    let ha = nomes()?;
    let base = format!("{PREFIXO}DIST-S1 {}", camada.curto());
    let (mosaico, recorte, na_rede) = (format!("{base}: mosaico"), format!("{base}: recorte"), format!("{base}: na grade"));
    let brutos: Vec<String> = produtos.iter().map(|p| format!("{base} {}", p.rotulo())).collect();
    let na_grade: Vec<String> = brutos.iter().map(|b| format!("{b} na grade")).collect();
    let na_area: Vec<String> = brutos.iter().map(|b| format!("{b} na área")).collect();
    // As intermediárias de uma vez anterior que tenha parado no meio.
    let velhas: Vec<&str> = na_grade
        .iter()
        .chain(&na_area)
        .map(String::as_str)
        .chain([mosaico.as_str(), recorte.as_str(), na_rede.as_str()])
        .filter(|n| ha.iter().any(|h| h == n))
        .collect();
    if !velhas.is_empty() {
        host::run("layer.remove", json!({"layer": velhas}))?;
    }
    let urls: Vec<String> = produtos.iter().map(|p| p.url(camada)).collect();
    let pedidos: Vec<(&str, &str)> = urls.iter().map(String::as_str).zip(brutos.iter().map(String::as_str)).collect();
    let baixando = baixar_inteiros(&pedidos, &ha)?;
    if baixando > 0 {
        let n = if baixando == 1 { "1 tile".to_string() } else { format!("{baixando} tiles") };
        host::notify(&format!(
            "Perturbação da vegetação ({}): baixando {n} do ASF (cerca de {} MB cada). Quando aparecerem no mapa como \"{base}\", clique de novo",
            camada.curto(),
            camada.megas()
        ));
        return Ok(Some(json!({"pending": baixando, "tiles": produtos.len()})));
    }
    // Códigos e dias não se misturam: o vizinho mais próximo, ou o mais frequente em células
    // maiores; a confiança e a intensidade, a média em células maiores.
    let resampling = match (g.celula > 45.0, camada.discreta()) {
        (false, _) => "nearest",
        (true, true) => "mode",
        (true, false) => "average",
    };
    let mut temporarias: Vec<String> = Vec::new();
    let resultado = (|| -> Result<(), String> {
        for ((bruto, q), a) in brutos.iter().zip(&na_grade).zip(&na_area) {
            // Cada tile na rede da grade (os tiles podem estar em fusos UTM diferentes), só no
            // retângulo da área, como no radar.
            host::run("raster.project", json!({"layer": bruto, "crs": g.epsg, "cellSize": g.celula, "resampling": resampling, "name": q}))?;
            host::run("raster.process", json!({"layer": q, "operation": "clip", "extent": c, "name": a}))?;
            temporarias.push(a.clone());
            host::run("layer.remove", json!({"layer": [bruto, q]}))?;
        }
        // Onde dois tiles se sobrepõem, o mosaico fica com o primeiro: o mais recente.
        let junto = if na_area.len() == 1 {
            na_area[0].clone()
        } else {
            host::run("raster.mosaic", json!({"inputs": na_area, "name": mosaico}))?;
            temporarias.push(mosaico.clone());
            mosaico.clone()
        };
        host::run("raster.process", json!({"layer": junto, "operation": "clip", "polygon": AREA, "name": recorte}))?;
        temporarias.push(recorte.clone());
        let saida = if camada == dist::Camada::Data { &na_rede } else { &nome.to_string() };
        host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "nearest", "name": saida}),
        )?;
        match camada {
            dist::Camada::Situacao => {
                host::run("style.importQgis", json!({"layer": nome, "text": paleta.qml()}))?;
                // Sem perturbação: transparente.
                host::run("raster.mask", json!({"layer": nome, "values": [0]}))?;
                // Sem as manchas, a situação fica no raster, visível: o comando não cai por elas.
                if !dist::manchas_cabem(c, g.celula) {
                    host::notify(&format!(
                        "Perturbação da vegetação: área grande demais para as manchas com borda (até cerca de {:.0} milhões de células da grade); fica o raster",
                        dist::MAX_CELULAS_MANCHAS / 1e6
                    ));
                } else if let Err(e) = manchas_da_situacao(&junto, nome, ate, paleta, g.celula) {
                    let manchas = dist::nome_das_manchas(ate);
                    let _ = remover(&[&manchas, &format!("{manchas} (sem recorte)"), &format!("{manchas} (em graus)")]);
                    host::log(&format!("manchas: {e}"));
                    host::notify(&format!("Perturbação da vegetação: as manchas com borda não saíram ({e}); fica o raster"));
                }
            }
            dist::Camada::Data => {
                temporarias.push(na_rede.clone());
                // Quantos dias antes do produto mais recente a perturbação foi vista: o arquivo
                // conta os dias desde 31/12/2020, e 0 é "sem perturbação".
                let n = dist::dia_do_dist(ate);
                host::run(
                    "raster.function",
                    json!({"layer": na_rede, "function": "bandMath", "expression": format!("{n} - b1"), "name": nome}),
                )?;
                let ate_o_inicio = (n - dist::dia_do_dist(dist::INICIO)).max(1);
                // O "sem perturbação" (0) vira `n` dias: fora da escala, escondido.
                host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": "Ember", "invert": true}))?;
                let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": 0, "max": ate_o_inicio}));
                host::run("raster.mask", json!({"layer": nome, "min": 0, "max": ate_o_inicio}))?;
            }
            dist::Camada::Confianca | dist::Camada::Intensidade => {
                host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": "Ember"}))?;
                if let Some((min, max)) = camada.escala() {
                    let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": min, "max": max}));
                }
                host::run("raster.mask", json!({"layer": nome, "values": [0]}))?;
            }
        }
        Ok(())
    })();
    let fora: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    if !fora.is_empty() {
        host::run("layer.remove", json!({"layer": fora})).map(|_| ()).or_else(|e| if resultado.is_err() { Ok(()) } else { Err(e) })?;
    }
    resultado.map(|()| None)
}

/// As manchas da situação como polígonos, com o nome da classe (campo `situacao`, que a legenda
/// mostra no lugar do código) e uma borda fina da própria cor ([`dist::Paleta::estilo_das_manchas`]):
/// sobre um mapa claro, a mancha amarelo-clara sem borda quase some.
///
/// Elas vêm do mosaico `retangulo` (cortado só pelo retângulo da área, nas células da grade) e são
/// recortadas depois pelo polígono da área, como os temas: na divisa, seguem o limite, e não os
/// degraus de 30 m das células que o raster `raster` tem. O raster, que está na grade e serve à
/// análise, fica no mapa, escondido.
fn manchas_da_situacao(retangulo: &str, raster: &str, ate: Data, paleta: dist::Paleta, celula: f64) -> Result<(), String> {
    let manchas = dist::nome_das_manchas(ate);
    let bruto = format!("{manchas} (sem recorte)");
    let em_graus = format!("{manchas} (em graus)");
    remover(&[&manchas, &bruto, &em_graus])?;
    // As regiões diferentes de 0 (sem perturbação) e do sem dado, com o código em `VALUE`. A
    // ferramenta do GeoLibre não grava em todo sistema (o Albers do IBGE, EPSG:10857, que a grade
    // usa quando a área passa de um fuso, ela recusa): então as manchas saem de uma cópia em graus,
    // com as mesmas células.
    if let Err(e) = ferramenta("geolibre:raster_to_vector_polygons", json!({"input": retangulo}), &bruto) {
        host::log(&format!("manchas: no sistema da grade não deu ({e}); vão em graus"));
        remover(&[&bruto])?;
        host::run(
            "raster.project",
            json!({"layer": retangulo, "crs": 4326, "cellSize": celula / 111_320.0, "resampling": "nearest", "name": em_graus}),
        )?;
        let feito = ferramenta("geolibre:raster_to_vector_polygons", json!({"input": em_graus}), &bruto);
        remover(&[&em_graus])?;
        feito?;
    }
    let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto, "clip": AREA}}))?;
    let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": recortada, "name": manchas}))?;
    remover(&[&bruto])?;
    host::run("edit.addField", json!({"layer": manchas, "name": "situacao", "type": "text"}))?;
    host::run("edit.calculateField", json!({"layer": manchas, "field": "situacao", "expression": dist::expressao_dos_nomes()}))?;
    let _ = host::run("style.remove", json!({"name": paleta.nome_do_estilo()}));
    host::run("style.add", json!({"text": paleta.estilo_das_manchas()}))?;
    host::run("style.apply", json!({"layer": manchas, "name": paleta.nome_do_estilo()}))?;
    host::run("layer.visibility", json!({"layer": raster, "visible": false}))?;
    Ok(())
}

/// Uma camada dos alertas de perturbação da vegetação (OPERA DIST-S1) na área: a do produto mais
/// recente de cada tile, que acumula os alertas desde o começo do monitoramento; por script, até
/// um dia.
fn perturbacao(req: &Request, camada: dist::Camada) -> Result<Value, String> {
    let paleta = dist::Paleta::ler(req.str("paleta"))?;
    if req.str("paleta").is_some() && camada != dist::Camada::Situacao {
        return Err(format!("a paleta vale só para a situação; a {} tem a escala dela", camada.curto()));
    }
    let PedidoAsf { c, celula, dia: ate } = area_do_asf(req, "o DIST-S1")?;
    let g = grade_para(c, celula)?;
    let produtos = match produtos_do_dist(c, ate)? {
        Ok(p) => p,
        Err(p) => return Ok(p),
    };
    // O nome leva a data do mais recente; os outros tiles podem ser de alguns dias antes.
    let (mais_novo, mais_velho) = (produtos.iter().map(|p| p.data).max(), produtos.iter().map(|p| p.data).min());
    let (Some(mais_novo), Some(mais_velho)) = (mais_novo, mais_velho) else {
        return Err("nenhum tile do DIST-S1".into());
    };
    let nome = camada.nome(mais_novo);
    remover(&[&nome, &dist::nome_das_manchas(mais_novo)])?;
    if let Some(pendente) = dist_na_grade(&produtos, camada, &g, c, &nome, mais_novo, paleta)? {
        return Ok(pendente);
    }
    let datas = if mais_velho == mais_novo {
        format!("de {}", mais_novo.texto())
    } else {
        format!("de {} a {}", mais_velho.texto(), mais_novo.texto())
    };
    let codigos = if camada == dist::Camada::Situacao { format!(". Códigos: {}", paleta.codigos()) } else { String::new() };
    host::notify(&format!(
        "{}: {} tiles {datas}, na grade do projeto, {}{codigos}",
        nome.trim_start_matches(PREFIXO),
        produtos.len(),
        g.texto()
    ));
    Ok(json!({
        "layer": nome, "date": mais_novo.texto(), "products": produtos.iter().map(|p| p.nome.clone()).collect::<Vec<_>>(),
        "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// Roda uma ferramenta do RustGIS (`gp.run`) e dá à camada que ela cria o nome `nome`.
fn ferramenta(tool: &str, params: Value, nome: &str) -> Result<(), String> {
    let r = host::run("gp.run", json!({"tool": tool, "params": params}))?;
    let id = r["layers"].get(0).cloned().ok_or(format!("{tool} não criou camada"))?;
    host::run("layer.rename", json!({"layer": id, "name": nome}))?;
    Ok(())
}

/// O radar do Sentinel-1 (OPERA RTC-S1) na área: uma polarização, ou a razão VH/VV (que deixa
/// as duas no mapa também). Por padrão, a passagem mais recente que cobre a área; por script,
/// a de um dia.
fn radar(req: &Request, qual: Option<radar::Pol>) -> Result<Value, String> {
    let PedidoAsf { c, celula, dia } = area_do_asf(req, "o radar")?;
    let g = grade_para(c, celula)?;
    let recortes = match recortes_do_radar(c, dia)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let data = recortes[0].data;
    let pols = match qual {
        Some(p) => vec![p],
        None => vec![radar::Pol::Vv, radar::Pol::Vh],
    };
    let ha = nomes()?;
    let mut feitas = Vec::new();
    for p in pols {
        let nome = radar::nome_da_camada(p, data);
        // A razão reaproveita uma polarização que já está no mapa, da mesma data.
        if qual.is_none() && ha.contains(&nome) {
            feitas.push(nome);
            continue;
        }
        remover(&[&nome])?;
        if let Some(pendente) = radar_na_grade(&recortes, p, &g, c, &nome)? {
            return Ok(pendente);
        }
        feitas.push(nome);
    }
    let mut camada = feitas[0].clone();
    if qual.is_none() {
        let razao = radar::nome_da_razao(data);
        remover(&[&razao])?;
        // Em dB, a razão VH/VV é a diferença VH − VV.
        host::run("raster.change", json!({"before": feitas[0], "after": feitas[1], "method": "difference", "name": razao}))?;
        let (min, max) = radar::ESCALA_DA_RAZAO;
        let _ = host::run("raster.stretch", json!({"layer": razao, "type": "custom", "min": min, "max": max}));
        camada = razao;
    }
    host::notify(&format!(
        "{}: {} recortes da passagem de {}, na grade do projeto, {}",
        camada.trim_start_matches(PREFIXO),
        recortes.len(),
        data.texto(),
        g.texto()
    ));
    Ok(json!({
        "layer": camada, "date": data.texto(), "bursts": recortes.iter().map(|r| r.nome.clone()).collect::<Vec<_>>(),
        "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// O texto do painel sobre a área do mapa.
fn area_no_mapa() -> Result<String, String> {
    let ha = nomes()?;
    let texto = if ha.iter().any(|n| n == AREA) {
        area_que_ha()?
    } else {
        "Nenhuma ainda: clique num limite da árvore abaixo (o Brasil, uma grande região, um estado, uma meso ou microrregião, um município), selecione polígonos no mapa e use Área: seleção, ou desenhe com Área: desenho.".to_string()
    };
    Ok(if ha.iter().any(|n| n == DESENHO) { format!("{texto}\n\n{DICA_DO_DESENHO}.") } else { texto })
}

/// O texto do painel sobre a camada da área.
fn area_que_ha() -> Result<String, String> {
    let camada = camada_no_projeto(AREA)?.ok_or("a camada da área sumiu")?;
    let n = camada["features"].as_u64().unwrap_or(0) as usize;
    // Um território escolhido no painel vem leve, com o nome; uma área de seleção pode ser
    // detalhada demais para o plugin ler, e fica com o retângulo e quantos polígonos são.
    let do_painel = camada["fields"].as_array().is_some_and(|f| f.iter().any(|c| c.as_str().is_some_and(|c| c.starts_with("nome "))));
    if do_painel && n <= 50 {
        return Ok(descrever(&host::features(AREA, false)?));
    }
    // O limite oficial de um município ([`afinar_area`]) pode ser pesado demais para o plugin ler:
    // os nomes vêm de uma cópia simplificada.
    if tem_campo(&camada, "malha")
        && let Some(c) = extensao(AREA)?
    {
        return Ok(texto_da_area(&nomes_da_area(c)?, n, Some(c)));
    }
    Ok(texto_da_area(&[], n, extensao(AREA)?))
}

/// O estado do painel, quando ele abre.
fn painel(req: &Request) -> Result<Value, String> {
    let mut r = json!({"panel": {"items": {
        "area": {"text": area_no_mapa()?},
    }}});
    // A grade: o texto, os nomes das posições dos controles e, havendo grade, os controles nela.
    let g = grade_no_mapa()?;
    r["panel"]["items"]["grade-texto"] = json!({"text": g.map_or_else(sem_grade, |g| g.texto())});
    let celulas: Vec<String> = grade::CELULAS.iter().map(|m| grade::texto_da_celula(*m)).collect();
    let sistemas: Vec<String> = Escolha::DO_PAINEL.iter().map(|e| e.rotulo()).collect();
    r["panel"]["items"]["grade-celula"] = json!({"labels": celulas});
    r["panel"]["items"]["grade-sistema"] = json!({"labels": sistemas});
    if let Some(g) = g {
        if let Some(i) = g.no_controle_da_celula() {
            r["panel"]["values"]["grade-celula"] = json!(i);
        }
        if let Some(i) = g.no_controle_do_sistema() {
            r["panel"]["values"]["grade-sistema"] = json!(i);
        }
    }
    // A árvore, com os municípios da grande região do nó escolhido (a busca só procura nos nós
    // que recebeu).
    let regiao = req.panel.get(ARVORE).and_then(Value::as_str).and_then(municipios::regiao_do_no);
    r["panel"]["items"][ARVORE]["nodes"] = municipios::arvore(regiao);
    // O botão do desenho volta a dizer o que faz quando o painel é reaberto no meio de um desenho.
    let desenhando = feicoes_da_camada(DESENHO)?.is_some();
    r["panel"]["items"]["area-desenho"] = json!({"label": if desenhando { json!("Usar o desenho") } else { Value::Null }});
    Ok(r)
}

/// O texto do painel sem grade no mapa.
fn sem_grade() -> String {
    format!(
        "Nenhuma ainda: escolha a célula e o sistema abaixo e clique em Grade do projeto. Sem isso, a primeira Elevação cria a padrão ({}, sistema automático)",
        grade::texto_da_celula(grade::CELULA_PADRAO)
    )
}

/// Um nó escolhido na árvore Limites políticos vira a área: o Brasil, uma grande região, um
/// estado, uma mesorregião, uma microrregião ou um município. A árvore volta com os municípios da
/// grande região dele, que passam a aparecer na busca. Por script, o nó vem em `no`.
fn escolher_area(req: &Request) -> Result<Value, String> {
    let no = match req.str("no") {
        Some(no) => no,
        None => match req.panel.get(ARVORE).and_then(Value::as_str).filter(|_| req.changed(ARVORE)) {
            Some(no) => no,
            None => return Ok(json!({})),
        },
    };
    let t = municipios::territorio(no).ok_or_else(|| format!("'{no}' não é um nó da árvore Limites políticos do painel {PAINEL}"))?;
    let mut r = area_de(&t)?;
    desistir_do_desenho(&mut r)?;
    r["panel"]["items"][ARVORE]["nodes"] = municipios::arvore(municipios::regiao_do_no(no));
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
    // Os contornos que vieram agora ainda não estão no mapa: vão com os campos, no fim (pô-los
    // antes também gastava a cota de instruções do plugin à toa com todas as fontes juntas).
    let mut contornos_novos = false;
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
            contornos_novos = true;
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
            // App web: o RustGIS roda o comando de novo quando as respostas chegarem. Os contornos
            // ficam no mapa, e a nova rodada os reaproveita.
            Ok(Err(pendente)) => {
                if contornos_novos {
                    remover(&[&nome])?;
                    host::add_layer(&nome, geojson::collection(feicoes))?;
                }
                return Ok(pendente);
            }
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
    // Sem fonte nenhuma, os contornos novos ficam para um novo clique depois de um limite.
    if !feitas.is_empty() || contornos_novos {
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
            let r = host::run("layer.addUrl", json!({"url": censo::url_setores(uf), "name": bruto, "zoom": false}))?;
            if r.get("pending").and_then(Value::as_bool) == Some(true) {
                remover(&[&nome])?;
                baixando.push(uf.clone());
                continue;
            }
        }
        remover(&[&nome])?;
        let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto, "clip": AREA}}))?;
        let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
        host::run("layer.rename", json!({"layer": recortada, "name": nome}))?;
        remover(&[&bruto])?;
        nomear_campos_dos_setores(&nome)?;
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

/// Troca os códigos `v0001` a `v0007` dos setores pelos nomes de [`censo::CAMPOS_SETORES`]: o
/// RustGIS não renomeia campos, então cada um ganha o campo novo, o valor e perde o código. Um
/// campo que não está na camada (o IBGE mudou o arquivo) fica como está.
fn nomear_campos_dos_setores(camada: &str) -> Result<(), String> {
    let campos = host::run("layer.fields", json!({"layer": camada}))?;
    // O nome como está na camada (v0001 ou V0001).
    let nome_de = |n: &str| {
        campos.as_array().into_iter().flatten().filter_map(|f| f["name"].as_str()).find(|x| x.eq_ignore_ascii_case(n)).map(str::to_string)
    };
    for (codigo, novo, tipo) in censo::CAMPOS_SETORES {
        let Some(codigo) = nome_de(codigo) else { continue };
        if nome_de(novo).is_some() {
            continue;
        }
        host::run("edit.addField", json!({"layer": camada, "name": novo, "type": tipo}))?;
        host::run("edit.calculateField", json!({"layer": camada, "field": novo, "expression": codigo}))?;
        host::run("edit.deleteField", json!({"layer": camada, "field": codigo}))?;
    }
    Ok(())
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

/// Se o RustGIS lê só uma janela de um arquivo remoto (o `bbox` do `layer.addUrl`, depois da
/// 0.5.0). A 0.5.0 ignora o `bbox`: no desktop, a leitura fica pendente e chega o arquivo inteiro
/// (um fuso UTM do GEDI, o Brasil do MapBiomas), que nenhum passo seguinte termina de processar.
fn le_janela_remota() -> Result<bool, String> {
    let lista = host::run("info.commands", json!({}))?;
    let comandos = lista.get("commands").unwrap_or(&lista);
    Ok(comandos
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"].as_str() == Some("layer.addUrl"))
        .and_then(|c| c["params"].as_str())
        .is_some_and(|p| p.contains("\"bbox\"")))
}

/// O erro de um pedido que o RustGIS cancelou porque o token Earthdata guardado neste computador
/// foi lido durante o comando: o RustGIS toma a leitura por uma troca de conta e cancela os
/// pedidos que o comando ainda faria. No clique seguinte o token já está lido.
fn token_recem_lido(e: String) -> String {
    if e.contains("cancelled before it was sent") || e.contains("active account changed") {
        "o RustGIS acabou de ler o token Earthdata guardado neste computador e, por segurança, cancelou o pedido deste clique: clique de novo".into()
    } else {
        e
    }
}

/// Uma "janela" que é o arquivo inteiro: muito maior que a `pedida` (uma camada de um clique
/// numa versão que ignorou o `bbox`).
fn janela_inteira(e: [f64; 4], pedida: [f64; 4]) -> bool {
    e[2] - e[0] > 3.0 * (pedida[2] - pedida[0]) + 0.5 || e[3] - e[1] > 3.0 * (pedida[3] - pedida[1]) + 0.5
}

/// Tira do mapa a camada `nome`, também quando ela é de mosaicos (que o `layer.remove` não vê).
fn tirar_janela(nome: &str) -> Result<(), String> {
    if mosaicos_no_mapa()?.iter().any(|n| n == nome) {
        host::run("map.removeTiles", json!({"name": nome}))?;
    }
    remover(&[nome])
}

/// Os nomes das camadas de mosaicos do mapa (o `host.layers` não as lista).
fn mosaicos_no_mapa() -> Result<Vec<String>, String> {
    Ok(host::run("map.tiles", json!({}))?.as_array().into_iter().flatten().filter_map(|t| t["name"].as_str().map(str::to_string)).collect())
}

/// O ano do MapBiomas: o do script, o do controle do painel ou o último da coleção.
fn ano_do_mapbiomas(req: &Request) -> i64 {
    req.f64("ano").map(|a| a as i64).or_else(|| req.panel.get(mapbiomas::CONTROLE).and_then(Value::as_i64)).unwrap_or(mapbiomas::ULTIMO_ANO)
}

/// O nome do raster de uso e cobertura de `ano` recortado pela área.
fn camada_recortada(ano: i64) -> String {
    format!("{PREFIXO}Uso e cobertura {ano} (MapBiomas, recortado)")
}

/// O uso e a cobertura do MapBiomas em `ano`: com uma área, um ano do espelho, `recortar` e um
/// RustGIS que lê a janela de um COG, o raster de classes recortado pela área
/// ([`cobertura_recortada`]); senão, o mapa de mosaicos do Brasil inteiro, com as cores da
/// legenda, dizendo por que não veio recortado.
fn cobertura_mapbiomas(ano: i64, recortar: bool) -> Result<Value, String> {
    if !(mapbiomas::PRIMEIRO_ANO..=mapbiomas::ULTIMO_ANO).contains(&ano) {
        return Err(format!("ano é de {} a {}", mapbiomas::PRIMEIRO_ANO, mapbiomas::ULTIMO_ANO));
    }
    let mut por_que = None;
    if recortar && nomes()?.iter().any(|n| n == AREA) {
        if !mapbiomas::no_espelho(ano) {
            por_que = Some(format!(
                "o recorte pela área existe de {} a {} (o espelho aberto da coleção)",
                mapbiomas::ESPELHO_PRIMEIRO,
                mapbiomas::ESPELHO_ULTIMO
            ));
        } else {
            match cobertura_recortada(ano) {
                Ok(Recorte::Feito(v)) => return Ok(v),
                Ok(Recorte::SemJanela) => {
                    por_que = Some("esta versão do RustGIS não lê só a janela da área de um COG; o recorte vem com a próxima".into())
                }
                Err(e) => por_que = Some(format!("o recorte pela área falhou ({e})")),
            }
        }
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
    let motivo = por_que.as_deref().map(|m| format!(": {m}")).unwrap_or_default();
    host::notify(&format!(
        "{nome}: mapa para ver, com as cores da legenda oficial (brasil.mapbiomas.org); não é recortado pela área{motivo}. Vale por algumas horas"
    ));
    Ok(json!({"tiles": nome, "year": ano, "notClipped": por_que}))
}

/// A variável `v` do GEDI L4D na área, na grade do projeto: as janelas da área nos arquivos dos
/// fusos que ela cruza, cada uma posta na grade, juntadas e recortadas pelo contorno; ou o
/// pedido pendente (a busca no app web, ou as janelas no desktop).
fn gedi_na_area(v: gedi::Variavel, celula: Option<f64>) -> Result<Value, String> {
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if let Ok(s) = host::run("earthdata.status", json!({}))
        && s["token"].as_bool() == Some(false)
    {
        return Err(format!(
            "{} (GEDI, ORNL DAAC da NASA) pede o login Earthdata, que é grátis: crie um token em urs.earthdata.nasa.gov (Generate Token) e cole-o no RustGIS, em Início › Dados web › NASA Earthaccess › Earthdata Login",
            v.titulo()
        ));
    }
    let sem_janela = || {
        format!(
            "{} (GEDI L4D) precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): cada arquivo cobre um fuso UTM inteiro",
            v.titulo()
        )
    };
    if !le_janela_remota()? {
        // A camada que uma tentativa anterior deixou: o arquivo inteiro de um fuso. Um erro
        // desfaria a remoção (o RustGIS desfaz a chamada que falha): sai com um aviso.
        let sufixo = format!(" {} (janela)", v.titulo());
        let mut tiradas = Vec::new();
        for nome in nomes()?.iter().chain(&mosaicos_no_mapa()?) {
            if nome.starts_with(&format!("{PREFIXO}GEDI ")) && nome.ends_with(&sufixo) && !tiradas.contains(nome) {
                tirar_janela(nome)?;
                tiradas.push(nome.clone());
            }
        }
        if tiradas.is_empty() {
            return Err(sem_janela());
        }
        host::notify(&format!("{}; saiu do mapa o arquivo inteiro que uma tentativa anterior trouxe", sem_janela()));
        return Ok(json!({"removed": tiradas}));
    }
    let resposta = match buscar_varios(&[gedi::url_busca(c)]).map_err(token_recem_lido)? {
        Ok(mut r) => r.remove(0),
        Err(p) => return Ok(p),
    };
    let urls = gedi::arquivos(&resposta, v);
    if urls.is_empty() {
        return Err(format!("o GEDI L4D não tem {} na área (a coleção cobre de 51,6° N a 51,6° S)", v.titulo().to_lowercase()));
    }
    // As janelas de um clique anterior servem se cobrem a área (no desktop, o comando roda de novo
    // quando cada uma chega).
    let ha = nomes()?;
    let mut janelas: Vec<String> = Vec::new();
    let mut lendo = 0;
    for url in &urls {
        let nome = format!("{PREFIXO}GEDI {} {} (janela)", gedi::fuso(url), v.titulo());
        if ha.contains(&nome)
            && let Some(e) = extensao(&nome)?
        {
            if janela_inteira(e, gedi::janela(c)) {
                // Sem erro, que desfaria a remoção.
                tirar_janela(&nome)?;
                host::notify(&format!("{}; saiu do mapa o arquivo inteiro que uma tentativa anterior trouxe", sem_janela()));
                return Ok(json!({"removed": [nome]}));
            }
            if retangulo_cabe(c, e) {
                janelas.push(nome);
                continue;
            }
        }
        remover(&[&nome])?;
        let mut p = json!({"url": url, "name": nome, "bbox": gedi::janela(c), "zoom": false});
        if let Some(m) = gedi::celula_da_janela(c) {
            p["cellSize"] = json!(m);
        }
        let r = host::run("layer.addUrl", p).map_err(token_recem_lido)?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            lendo += 1;
            continue;
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            // A 0.5.0 ignora o `bbox` e põe o arquivo inteiro (um fuso a 30 m) no mapa.
            tirar_janela(&nome)?;
            return Err(sem_janela());
        }
        janelas.push(nome);
    }
    if lendo > 0 {
        host::notify(&format!("{} (GEDI L4D): lendo a área em {lendo} arquivo(s) da NASA; a camada vem quando chegarem", v.titulo()));
        return Ok(json!({"pending": lendo, "files": urls.len()}));
    }
    let g = grade_para(c, celula)?;
    let saida = v.camada();
    let limpas: Vec<String> = (0..janelas.len()).map(|i| format!("{PREFIXO}GEDI limpo {i}")).collect();
    let na_grade: Vec<String> = (0..janelas.len()).map(|i| format!("{PREFIXO}GEDI na grade {i}")).collect();
    let junto = format!("{PREFIXO}GEDI junto");
    let mut temporarias: Vec<&str> = janelas.iter().chain(&limpas).chain(&na_grade).map(String::as_str).collect();
    temporarias.push(junto.as_str());
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        // Até 45 m, o valor do pixel imputado (o mais próximo); acima, a média.
        let resampling = if g.celula > 45.0 { "average" } else { "nearest" };
        for ((j, l), n) in janelas.iter().zip(&limpas).zip(&na_grade) {
            // Os negativos (o -9999 e as médias com ele nas pirâmides) viram sem dado antes da
            // grade, para não entrarem nas médias: a conta de bandas só tem + - * /, então o
            // valor sobe 1, o que ficou até 0,5 vira 0 (`remap`), e (b1 / b1) dá sem dado no 0.
            host::run(
                "raster.functionChain",
                json!({"layer": j, "name": l, "steps": [
                    {"function": "bandMath", "expression": "b1 + 1"},
                    {"function": "remap", "ranges": [[-1e9, 0.5, 0]], "keep": true},
                    {"function": "bandMath", "expression": v.conta()},
                ]}),
            )?;
            host::run(
                "raster.project",
                json!({"layer": l, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
            )?;
        }
        let mosaico = if na_grade.len() == 1 {
            na_grade[0].clone()
        } else {
            host::run("raster.mosaic", json!({"inputs": na_grade, "name": junto}))?;
            junto.clone()
        };
        host::run("raster.process", json!({"layer": mosaico, "operation": "clip", "polygon": AREA, "name": saida}))?;
        let (min, max) = v.validos();
        host::run("raster.mask", json!({"layer": saida, "min": min, "max": max}))?;
        let (a, b) = v.escala();
        let (rampa, cores) = v.rampa();
        // A rampa já registrada (de um clique anterior) fica como está.
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlGn"}))?;
        }
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    // A média na área, para o aviso.
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} {}", formatar_decimal(m), v.unidade()))
        .unwrap_or_default();
    let piramide = gedi::celula_da_janela(c).map(|m| format!(", lida da pirâmide de {} m do arquivo", m - 1.0)).unwrap_or_default();
    host::notify(&format!(
        "{saida}: em {}, de 2019 a 2023 (GEDI imputado com o Landsat), na grade do projeto, {}{piramide}{media}",
        v.unidade(),
        g.texto()
    ));
    Ok(json!({"layer": saida, "files": urls.len(), "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// O botão Layout: uma folha pronta com o dado principal do BrasilData no mapa (a primeira camada
/// visível com receita, [`layout::receita`]), o nome da área no título, as fontes das camadas e a
/// projeção da grade. Na folha do RustGIS (`layout.publication`), a legenda deixa de fora a área e
/// a grade, e a barra de cores ganha o título com a unidade.
fn layout_do_mapa(req: &Request) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção): o layout enquadra a área".into());
    }
    let info = host::run("info.project", json!({}))?;
    let visiveis: Vec<String> = info["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["visible"].as_bool() != Some(false))
        .filter_map(|l| l["name"].as_str())
        .filter(|n| n.starts_with(PREFIXO) && ![AREA, GRADE, DESENHO].contains(n))
        .map(str::to_string)
        .collect();
    let (principal, r) = visiveis
        .iter()
        .find_map(|n| layout::receita(n).map(|r| (n.clone(), r)))
        .ok_or("nenhum dado do BrasilData visível no mapa para o layout: traga um dado (um tema, o censo, a elevação, o radar, o GEDI...) e deixe-o visível")?;
    let titulo = layout::titulo(&area_que_ha()?);
    // As fontes das camadas visíveis, sem repetir, e a do limite.
    let mut fontes: Vec<String> = Vec::new();
    for f in citacoes()?.0 {
        if f["layer"].as_str().is_some_and(|l| visiveis.iter().any(|v| v == l))
            && let Some(s) = f["source"].as_str()
            && !fontes.iter().any(|x| x == s)
        {
            fontes.push(s.to_string());
        }
    }
    fontes.push("Limite da área: IBGE · dados trazidos e recortados pelo plugin BrasilData".into());
    // A projeção da grade do projeto, ou a automática da área (UTM do fuso, ou o Albers do IBGE).
    let g = grade_no_mapa()?;
    let epsg = g.as_ref().map_or_else(|| Escolha::Automatico.epsg(c), |g| g.epsg);
    host::run("map.projection", json!({"type": format!("EPSG:{epsg}")}))?;
    let passo = layout::passo_da_grade(c);
    let celula = g.as_ref().map(|g| format!("; células de {}", grade::texto_da_celula(g.celula))).unwrap_or_default();
    let referencia = [
        format!("Projeção: {}", grade::nome_do_sistema(epsg)),
        format!("Grade de {}, coordenadas em metros{celula}", grade::texto_da_celula(passo)),
    ];
    // A área só como contorno, e a grade (um retângulo de trabalho) fora do mapa.
    // (o `layer.noFill` não basta na 0.5.0: a cor transparente, sim)
    host::run("layer.color", json!({"layer": AREA, "color": "#00000000"}))?;
    host::run("layer.outline", json!({"layer": AREA, "color": "#5E5547", "width": 1.2}))?;
    // O fim do comando devolve a cada camada o estilo do começo ([`devolver_estilos`]): o da
    // área passa a ser este.
    let _ = host::run("style.remove", json!({"name": estilo::guardado(AREA)}));
    let _ = host::run("style.new", json!({"layer": AREA, "name": estilo::guardado(AREA)}));
    if nomes()?.iter().any(|n| n == GRADE) {
        host::run("layer.visibility", json!({"layer": [GRADE], "visible": false}))?;
    }
    let nome = format!("BrasilData: {}", titulo.to_lowercase());
    let _ = host::run("layout.remove", json!({"name": nome}));
    let texto = |k: &str, padrao: &str| req.params.get(k).and_then(Value::as_str).unwrap_or(padrao).to_string();
    let mut p = json!({
        "name": nome, "profile": texto("perfil", "a4"), "style": texto("estilo", "classic"), "extent": c,
        "title": titulo, "subtitle": r.subtitulo, "sources": fontes, "reference": referencia,
        "author": "Elaborado com RustGIS e BrasilData · {date}",
        "headings": ["Fonte dos dados", "Referência cartográfica", "Nota"],
        "locator": "globe", "decimal": ",",
    });
    if let Some(n) = r.nota {
        p["note"] = json!(n);
    }
    let folha = host::run("layout.publication", p)?;
    let elemento = |qual: &str| {
        folha["elements"].as_array().into_iter().flatten().find(|e| e["name"].as_str() == Some(qual)).and_then(|e| e["element"].as_u64())
    };
    // A legenda: sem a área, a grade e o raster que a barra de cores já mostra; vazia, some.
    // (o `exclude` recusa um nome que não está no mapa)
    let no_mapa = nomes()?;
    let mut fora: Vec<String> = [AREA, GRADE, DESENHO].into_iter().filter(|n| no_mapa.iter().any(|m| m == n)).map(String::from).collect();
    if r.barra.is_some() {
        fora.push(principal.clone());
    }
    if let Some(id) = elemento("Legend") {
        let sobra = visiveis.iter().any(|v| !fora.contains(v));
        // Um título a mais tira uma linha da caixa, que tem a altura das classes: só nas receitas
        // com classes nomeadas (o DIST), em que ela cabe.
        let mut u = json!({"layout": nome, "element": id, "exclude": fora, "visible": sobra});
        if let Some(t) = r.legenda {
            u["title"] = json!(t);
        }
        host::run("layout.update", u)?;
    }
    if let Some(id) = elemento("Colorbar") {
        match &r.barra {
            Some((t, marcas)) => {
                host::run("layout.update", json!({"layout": nome, "element": id, "layer": principal, "title": t, "ticks": marcas}))?
            }
            None => host::run("layout.update", json!({"layout": nome, "element": id, "visible": false}))?,
        };
    }
    // A barra de escala em 5 partes: números redondos (1 a 5 km), e não 1,25 e 3,75.
    if let Some(id) = elemento("Scale Bar") {
        let _ = host::run("layout.update", json!({"layout": nome, "element": id, "divisions": 5}));
    }
    if let Some(id) = elemento("Map Frame") {
        host::run("layout.update", json!({"layout": nome, "element": id, "grid": {"type": "measured", "step": passo, "labels": true}}))?;
    }
    host::notify(&format!(
        "Layout \"{nome}\": {} na área, em {}. Abra-o em Publicar para ajustar e exportar; um novo clique o refaz",
        r.subtitulo,
        grade::nome_do_sistema(epsg)
    ));
    Ok(json!({"layout": nome, "layer": principal, "crs": format!("EPSG:{epsg}")}))
}

/// Um número com uma casa decimal e vírgula.
fn formatar_decimal(x: f64) -> String {
    format!("{x:.1}").replace('.', ",")
}

/// O que [`cobertura_recortada`] fez.
enum Recorte {
    /// O raster na grade, ou o pedido pendente (a janela ou o limite oficial a caminho).
    Feito(Value),
    /// O RustGIS não leu a janela: pôs o COG inteiro como camada de mosaicos (até a 0.5.0, que
    /// ignora o `bbox`), já tirada do mapa.
    SemJanela,
}

/// O uso e a cobertura de `ano` (um ano do espelho) recortado pela área, na grade do projeto: a
/// janela do retângulo da área lida do COG, recortada pelo contorno e projetada na grade com a
/// classe mais frequente. A tabela de cores do arquivo vem junto.
fn cobertura_recortada(ano: i64) -> Result<Recorte, String> {
    // O contorno oficial do município, antes de recortar (o comando não passa por ele na entrada,
    // para o mapa de mosaicos não gastar um clique).
    if let Some(pendente) = afinar_area()? {
        return Ok(Recorte::Feito(pendente));
    }
    let (c, _) = retangulo()?;
    let nome = camada_recortada(ano);
    let bruto = format!("{PREFIXO}MapBiomas {ano} (janela)");
    let recorte = format!("{PREFIXO}MapBiomas {ano} (recorte)");
    // A janela de um clique anterior serve se cobre a área (no desktop, o comando roda de novo
    // quando ela chega).
    let e = if nomes()?.contains(&bruto) { extensao(&bruto)? } else { None };
    if e.is_some_and(|e| janela_inteira(e, mapbiomas::janela(c))) || !le_janela_remota()? {
        tirar_janela(&bruto)?;
        return Ok(Recorte::SemJanela);
    }
    let serve = e.is_some_and(|e| retangulo_cabe(c, e));
    if !serve {
        remover(&[&bruto])?;
        let mut p = json!({"url": mapbiomas::url_do_cog(ano), "name": bruto, "bbox": mapbiomas::janela(c), "zoom": false});
        if let Some(m) = mapbiomas::celula_da_janela(c) {
            p["cellSize"] = json!(m);
        }
        let r = host::run("layer.addUrl", p)?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            host::notify(&format!(
                "Uso e cobertura {ano} (MapBiomas): lendo a janela da área no COG da coleção; o raster vem quando ela chegar"
            ));
            return Ok(Recorte::Feito(r));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            // A 0.5.0 põe o COG inteiro no mapa.
            tirar_janela(&bruto)?;
            return Ok(Recorte::SemJanela);
        }
    }
    let g = grade_para(c, None)?;
    remover(&[&recorte, &nome])?;
    let feito =
        host::run("raster.process", json!({"layer": bruto, "operation": "clip", "polygon": AREA, "name": recorte})).and_then(|_| {
            host::run(
                "raster.project",
                json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "mode", "name": nome}),
            )
        });
    remover(&[&bruto, &recorte])?;
    let r = feito?;
    // Uma área grande vem de uma pirâmide do arquivo: as células lidas são maiores que as da grade.
    let piramide = mapbiomas::celula_da_janela(c)
        .map(|m| format!(" A área é grande para 30 m: a janela veio da pirâmide de {} m do arquivo.", m - 1.0))
        .unwrap_or_default();
    host::notify(&format!(
        "{nome}: classes do MapBiomas (Coleção 10) na grade do projeto, {}, com a classe mais frequente; as cores são as da legenda oficial.{piramide}",
        g.texto()
    ));
    Ok(Recorte::Feito(json!({
        "layer": nome, "year": ano, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "width": r["width"], "height": r["height"],
    })))
}

/// A autoria e o resto da seção Sobre: o resumo no aviso e o texto inteiro na resposta (a da
/// linha de comando e dos scripts) e no painel, onde ele já está desde a instalação. O RustGIS
/// não tem janela de texto para um plugin, e um comando não abre o painel.
fn sobre() -> Result<Value, String> {
    host::notify(crate::SOBRE_AVISO);
    Ok(json!({"version": versao::ESTA, "text": crate::SOBRE, "panel": {"items": {"sobre": {"text": crate::SOBRE}}}}))
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
    Ok(json!({"version": versao::ESTA, "latest": nova}))
}

#[cfg(test)]
mod testes_do_corte {
    use super::corte_do_servico;
    use serde_json::json;

    #[test]
    fn corte_de_uma_pagina_vira_aviso_com_os_numeros() {
        let r = json!({"notices": ["BrasilData: Drenagem (BC250 2025) (sem recorte): the server sent 50000 of its 184332 features (one page); the rest aren't read"]});
        assert_eq!(corte_do_servico(&r).as_deref(), Some("o servidor mandou só 50000 das 184332 feições do retângulo"));
    }

    #[test]
    fn corte_sem_numeros_e_resposta_inteira() {
        let r = json!({"notices": ["x: the server sent its first features only (its record limit); the rest aren't read"]});
        assert_eq!(corte_do_servico(&r).as_deref(), Some("o servidor mandou só parte das feições do retângulo"));
        assert_eq!(corte_do_servico(&json!({"features": 10, "notices": ["other note"]})), None);
        assert_eq!(corte_do_servico(&json!({"features": 10})), None);
    }
}

#[cfg(test)]
mod testes_janela {
    use super::janela_inteira;

    #[test]
    fn uma_janela_do_tamanho_de_um_fuso_e_o_arquivo_inteiro() {
        let pedida = [-42.60, -20.94, -42.27, -20.48];
        // A janela lida, alinhada aos blocos do arquivo: um pouco maior que a pedida.
        assert!(!janela_inteira([-42.62, -20.96, -42.25, -20.46], pedida));
        // O fuso 23S inteiro do GEDI L4D (de 48° a 42° O, do equador a 34° S).
        assert!(janela_inteira([-48.0, -34.0, -42.0, 0.0], pedida));
    }
}
