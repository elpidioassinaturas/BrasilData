//! Os comandos do plugin: o que fala com o RustGIS.

use std::collections::HashMap;

use rustgis_plugin::{Request, Value, geojson, host, json};

use crate::amostragem::{self, Candidato, Sorteio};
use crate::anadem;
use crate::aqueduct;
use crate::bdc;
use crate::cemaden;
use crate::censo;
use crate::chm;
use crate::chm_amazonia;
use crate::clms;
use crate::cog_global;
use crate::copdem;
use crate::ctrees;
use crate::datas::{self, Data};
use crate::dialogos;
use crate::dist;
use crate::drenagem;
use crate::emit;
use crate::enderecos;
use crate::era5;
use crate::estilo;
use crate::extensoes;
use crate::forty;
use crate::gba;
use crate::gdo;
use crate::gedi;
use crate::gedtm;
use crate::gem15;
use crate::grade::{self, Escolha, GRADE, Grade};
use crate::indicadores::{self, Fonte};
use crate::irrigacao;
use crate::jaxa;
use crate::layout;
use crate::localidades;
use crate::luzes;
use crate::malhas;
use crate::mapbiomas;
use crate::metano;
use crate::municipios::{self, Territorio};
use crate::ndvi_cdse;
use crate::nisar;
use crate::olm;
use crate::power;
use crate::radar;
use crate::radar_pc;
use crate::recorte;
use crate::risco_fogo;
use crate::rodovias;
use crate::saude;
use crate::sem_nuvens::{self, Caminho, Composicao, DataNaArea, Valor};
use crate::sol;
use crate::tempo;
use crate::vento;
use crate::versao;
use crate::worldcover;
use crate::zoom;
// A Chuva agora (PARDIG) fica em comandos/agora.rs.
mod agora;
use crate::{AREA, PREFIXO, TEMAS, Tema, falha_passageira, pedido_de_feicoes, tema_da_camada, tile_inexistente, tiles_que_nao_vieram};

/// Os nomes das camadas dos censos, depois de [`PREFIXO`].
const INDICADORES: &str = "Indicadores por município";
const CENSO_SETORES: &str = "Setores censitários 2022";

/// O título do painel do plugin (o do `plugin.json`).
const PAINEL: &str = "Área de trabalho";

/// A árvore Limites políticos do painel (`municipios::arvore`).
const ARVORE: &str = "limites";

/// A camada em que o usuário desenha a área, até usar o desenho.
const DESENHO: &str = "BrasilData: área (desenho)";

/// Um comando do plugin, com o estilo que o usuário deu às camadas do BrasilData guardado: um tema
/// buscado de novo (outra área, outra vista) troca a camada por uma nova, e a nova fica com a
/// legenda, as cores, os rótulos e a transparência da antiga. O estilo fica na camada (e no
/// projeto, com ela); o plugin só o leva da antiga para a nova. O estado do painel, que o RustGIS
/// pede a cada abertura, não mexe em camadas.
///
/// Os parâmetros vêm como um script os daria ([`dialogos::normalizar`]: a janela do comando
/// manda as escolhas pelo texto), e a resposta leva os avisos do comando como o `report` que a
/// janela mostra ([`dialogos::com_relato`]).
fn handle(mut req: Request) -> Result<Value, String> {
    if req.id == "painel" {
        return executar(req);
    }
    dialogos::normalizar(&req.id, &mut req.params);
    AVISOS.with(|a| a.borrow_mut().clear());
    guardar_estilos();
    let r = executar(req);
    if let Ok(v) = &r
        && !estilo::esperando(v)
    {
        devolver_estilos();
    }
    let avisos = AVISOS.with(|a| std::mem::take(&mut *a.borrow_mut()));
    r.map(|v| dialogos::com_relato(v, &avisos, PREFIXO))
}

thread_local! {
    /// Os avisos do comando que está rodando, para o `report` da resposta.
    static AVISOS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Um aviso ao usuário (`host::notify`), guardado também para o `report` da janela do comando.
fn avisar(texto: &str) {
    AVISOS.with(|a| a.borrow_mut().push(texto.to_string()));
    host::notify(texto);
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
    // A grade de um projeto feito antes da 0.1.40 pode ter ganhado de volta o preenchimento.
    if nomes().is_ok_and(|n| n.iter().any(|n| n == GRADE)) {
        let _ = aparencia_da_grade();
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
    "sobre-autoria",
    "sobre-fontes",
    "sobre-contribuir",
    // Os mapas do MapBiomas são do Brasil inteiro: a área não muda nada neles.
    "mapbiomas-cobertura",
    "mapbiomas-irrigacao",
];

/// Roda o comando e, se ele traz uma camada de dados, põe a ficha dela no painel.
///
/// Um comando que deixa um download pendente não pode ter criado ou tirado camadas: quando o
/// download chega, o RustGIS 0.7.0 roda o comando de novo e o recusa ("the live source or account
/// changed since this request started"), desfazendo o comando inteiro, e cada clique repetiria o
/// mesmo. Por isso a área já vem com a grade ([`por_area`]), e um comando que precisou criar ou
/// refazer a grade para nela, com a grade feita ([`GRADE_FEITA`]): o clique seguinte lê os dados.
fn executar(req: Request) -> Result<Value, String> {
    let ficha = crate::ficha(&req.id);
    let mut r = match executar_comando(req) {
        Err(e) if e == GRADE_FEITA => {
            let t = "Grade do projeto feita para a área. Clique de novo para trazer os dados";
            avisar(t);
            let mut r = json!({"grid": GRADE, "report": t});
            if let Some(g) = grade_no_mapa()? {
                grade_no_painel(&mut r, &g);
            }
            r
        }
        r => r?,
    };
    if let Some(f) = ficha
        && r.is_object()
    {
        r["panel"]["items"]["ficha"] = json!({"text": f.texto});
    }
    Ok(r)
}

/// Se a resposta `r` de um comando diz que um download ficou pendente (`"pending"`: `true` ou
/// quantos).
fn pendente(r: &Value) -> bool {
    match &r["pending"] {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_u64().is_some_and(|n| n > 0),
        _ => false,
    }
}

fn executar_comando(req: Request) -> Result<Value, String> {
    if !SEM_RECORTE.contains(&req.id.as_str())
        && let Some(pendente) = afinar_area(true)?
    {
        return Ok(pendente);
    }
    if let Some(t) = TEMAS.iter().find(|t| t.id == req.id) {
        let r = tema(t)?;
        if t.id == aqueduct::TEMA && !estilo::esperando(&r) {
            aqueduct_em_portugues(t)?;
        }
        if t.id == localidades::TEMA && !estilo::esperando(&r) {
            localidades_por_escala(t)?;
        }
        return Ok(r);
    }
    match req.id.as_str() {
        "area-municipio" => area_municipio(&req),
        "area-selecao" => area_selecao(),
        "area-desenho" => area_desenho(),
        "painel" => painel(&req),
        "escolher-area" => escolher_area(&req),
        "elevacao" => elevacao(&req),
        "sol-radiacao" => sol_na_area(&req),
        "vento-50m" => vento_na_area(&req),
        "emit-cenas" => cenas_do_emit(&req),
        "power-umidade" => power_na_area(&req, power::Variavel::Umidade),
        "power-evapotranspiracao" => power_na_area(&req, power::Variavel::Evapotranspiracao),
        "chuva-prevista" => chuva_prevista(&req),
        "chuva-ensemble" => chuva_do_ensemble(&req),
        "temperatura-prevista" => temperatura_prevista(&req),
        "chirps-mes" => chuva_do_mes(&req),
        "pardig-agora" => agora::chuva_agora(&req),
        "irrigacao-gmia" => irrigacao_na_area(req.f64("celula")),
        "risco-fogo" => risco_de_fogo(&req),
        "gem15" => gem15_na_area(&req),
        "chm-altura" => chm_na_area(&req),
        "chm-amazonia" => chm_amazonia_na_area(&req),
        "gba-edificacoes" => gba_na_area(),
        "osm-vias" => osm_vias(),
        "enderecos" => enderecos_na_area(),
        "gdo-seca" => gdo_seca(&req),
        "forty-tipologia" => forty_na_area(&req),
        "ctrees-biomassa" => ctrees_na_area(&req),
        "olm-solo" => olm_solo_na_area(&req),
        "gedtm-relevo" => gedtm_na_area(&req),
        "anadem" => anadem_na_area(&req),
        "metano-tropomi" => metano_na_area(&req),
        "ndvi-copernicus" => ndvi_copernicus(&req),
        "luzes-noturnas" => luzes_na_area(&req),
        "luzes-mudanca" => mudanca_das_luzes(&req),
        "bdc-indice" => bdc_indice(&req),
        "bdc-sem-nuvens" => bdc_sem_nuvens(&req),
        "jaxa-chuva" => jaxa_na_area(&req, jaxa::Variavel::Chuva),
        "jaxa-umidade" => jaxa_na_area(&req, jaxa::Variavel::Umidade),
        "jaxa-temperatura" => jaxa_na_area(&req, jaxa::Variavel::Temperatura(turno_do_jaxa(&req)?)),
        "jaxa-evapotranspiracao" => jaxa_na_area(&req, jaxa::Variavel::Evapotranspiracao),
        "jaxa-seca" => jaxa_na_area(&req, jaxa::Variavel::Seca),
        "era5-chuva" => era5_na_area(&req, era5::Variavel::Chuva),
        "era5-temperatura" => era5_na_area(&req, era5::Variavel::Temperatura(tempo::temperatura(req.str("tipo"))?)),
        "era5-umidade" => era5_na_area(&req, era5::Variavel::Umidade),
        "clms-cobertura" => cobertura_clms(&req),
        "worldcover" => worldcover_na_area(&req),
        "radar-vv" => radar(&req, Some(radar::Pol::Vv)),
        "radar-vh" => radar(&req, Some(radar::Pol::Vh)),
        "radar-razao" => radar(&req, None),
        "nisar-hh" => radar_nisar(&req, Some(nisar::Pol::Hh)),
        "nisar-hv" => radar_nisar(&req, Some(nisar::Pol::Hv)),
        "nisar-razao" => radar_nisar(&req, None),
        "nisar-cor" => composicao_nisar(&req),
        "nisar-tempo" => nisar_no_tempo(&req),
        "radar-10m" => radar_10m_no_tempo(&req),
        "radar-10m-cor" => radar_10m_cor(&req),
        "dist-situacao" => perturbacao(&req, dist::Camada::Situacao),
        "dist-data" => perturbacao(&req, dist::Camada::Data),
        "dist-confianca" => perturbacao(&req, dist::Camada::Confianca),
        "dist-intensidade" => perturbacao(&req, dist::Camada::Intensidade),
        "grade" => grade_do_projeto(&req),
        "fontes" => fontes(),
        "atualizacao" => atualizacao(),
        "sobre" => sobre(&req),
        "sobre-autoria" | "sobre-fontes" | "sobre-contribuir" => aba_sobre(&req.id),
        "amostra-aleatoria" => amostra_aleatoria(&req),
        "amostra-estratificada" => amostra_estratificada(&req),
        "amostra-sistematica" => amostra_sistematica(&req),
        "indicadores" => indicadores(&fontes_de(&req)?),
        "indicadores-todos" => indicadores(&indicadores::TODAS),
        "censo-municipios" => indicadores(&[Fonte::Censo]),
        "censo-agro" => indicadores(&[Fonte::Agro]),
        "censo-agroecologia" => indicadores(&[Fonte::Agroeco]),
        "indicadores-pix" => indicadores(&[Fonte::Pix]),
        "indicadores-credito" => indicadores(&[Fonte::Credito]),
        "indicadores-tse" => indicadores(&[Fonte::Tse]),
        "indicadores-caged" => indicadores(&[Fonte::Caged]),
        "indicadores-saude" => indicadores(&[Fonte::Saude]),
        "indicadores-seeg" => indicadores(&[Fonte::Seeg]),
        "censo-setores" => censo_setores(),
        "saude-estabelecimentos" => estabelecimentos_de_saude(),
        "cemaden-chuva" => chuva_cemaden(),
        "cemaden-alertas" => alertas_cemaden(),
        id if gedi::Variavel::do_comando(id).is_some() => {
            gedi_na_area(gedi::Variavel::do_comando(id).ok_or("comando do GEDI")?, req.f64("celula"))
        }
        "layout" => layout_do_mapa(&req),
        "mapbiomas-cobertura" => {
            cobertura_mapbiomas(ano_do_mapbiomas(&req), req.params.get("recorte").and_then(Value::as_bool) != Some(false))
        }
        "inpe-focos-periodo" => focos_no_periodo(&req),
        "mapbiomas-irrigacao" => irrigacao_mapbiomas(ano_do_mapbiomas(&req)),
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

/// Tira do mapa camadas que o comando acabou de pôr, sem ler a lista de camadas antes ([`remover`]):
/// com as janelas de um estado no mapa, cada leitura dela gasta boa parte da cota de instruções.
fn tirar_as_de_passagem(nomes: &[&str]) {
    if !nomes.is_empty() {
        let _ = host::run("layer.remove", json!({"layer": nomes}));
    }
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

/// Uma camada nova, do `geojson` em texto, que vai no pedido como está: montá-lo como `Value` e
/// copiá-lo para o pedido (o `host::add_layer`) custaria a cota de instruções do comando.
fn por_geojson(nome: &str, geojson: &str) -> Result<(), String> {
    let req = format!("{{\"op\":\"host.addLayer\",\"params\":{{\"name\":{},\"geojson\":{geojson}}}}}", json!(nome));
    let r: Value = serde_json::from_slice(&chamar_cru(&req)?).map_err(|e| format!("resposta ilegível do host.addLayer ({e})"))?;
    match r.get("error") {
        Some(e) => Err(e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string())),
        None => Ok(()),
    }
}

/// Põe o `geojson` (em texto), de retângulo `caixa`, como a área, no lugar da anterior. O `texto` é
/// o do painel ([`texto_da_area`]).
fn por_area(geojson: &str, caixa: [f64; 4], texto: &str) -> Result<Value, String> {
    // A grade acompanha a área, com as mesmas escolhas; no automático, o fuso pode mudar. Ela
    // vem antes, para a área ficar como a camada ativa.
    // Sem grade, a padrão (30 m, sistema automático): um comando de dados que a criasse antes de
    // pedir as janelas seria recusado quando elas chegam (ver executar).
    let havia = grade_no_mapa()?;
    let nova = havia.is_none();
    // A célula padrão é a da área; a que alguém escolheu fica, a não ser que não caiba nesta.
    let (grade, outra) = match havia {
        Some(g) => grade_refeita(caixa, g),
        None => grade_nova(caixa, Escolha::Automatico, None),
    };
    let grade = por_grade(grade)?;
    remover(&[AREA])?;
    por_geojson(AREA, geojson)?;
    vista_no_sistema(caixa, Some(grade.epsg));
    // Escolhida pelo nome, a área pode estar longe da vista.
    host::run("layer.zoomTo", json!({"layer": AREA}))?;
    // O porquê de outra célula vem antes: a barra de baixo só mostra o começo.
    let outra = outra.map_or(String::new(), |t| format!("{t}. "));
    avisar(&if nova {
        format!(
            "{outra}Área do BrasilData: {texto}. Grade do projeto: {}; para outra, use Grade do projeto no painel Área de trabalho",
            grade.texto()
        )
    } else {
        format!("{outra}Área do BrasilData: {texto}. Grade refeita para ela: {}", grade.texto())
    });
    let mut r = json!({"area": AREA, "bbox": caixa, "panel": {"items": {"area": {"text": texto}}}});
    grade_no_painel(&mut r, &grade);
    Ok(r)
}

/// Põe a `grade` no painel da resposta `r`: o texto e, se são dos controles, a célula e o sistema.
fn grade_no_painel(r: &mut Value, grade: &Grade) {
    r["panel"]["items"]["grade-texto"] = json!({"text": grade.texto()});
    if let Some(i) = grade.no_controle_da_celula() {
        r["panel"]["values"]["grade-celula"] = json!(i);
    }
    if let Some(i) = grade.no_controle_do_sistema() {
        r["panel"]["values"]["grade-sistema"] = json!(i);
    }
}

/// Desenha o mapa no sistema `epsg` da grade, ou, sem grade, no automático da área de retângulo
/// `c` (o UTM do fuso, ou o Albers do IBGE): a projeção que o projeto tinha pode deformar a área
/// (o Eckert IV de um mapa-múndi a deixa 27% mais estreita na Bahia). Uma falha deixa a vista
/// como está.
fn vista_no_sistema(c: [f64; 4], epsg: Option<u32>) {
    let epsg = epsg.unwrap_or_else(|| Escolha::Automatico.epsg(c));
    let _ = host::run("map.projection", json!({"type": format!("EPSG:{epsg}")}));
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
    aparencia_da_grade()?;
    Ok(g)
}

/// A grade só como contorno roxo. O preenchimento sai pela cor transparente, não pelo
/// `layer.noFill`: o estilo guardado e devolvido a cada comando ([`devolver_estilos`]) perde o
/// "sem preenchimento" no RustGIS 0.7.0, e a grade voltava com uma cor que cobria o mapa fora da
/// área (um dado do JAXA parecia não recortado).
fn aparencia_da_grade() -> Result<(), String> {
    host::run("layer.color", json!({"layer": GRADE, "color": "#00000000"}))?;
    host::run("layer.outline", json!({"layer": GRADE, "color": "#6a3d9a", "width": 1.5}))?;
    Ok(())
}

/// O botão Grade do projeto: a grade dos rasters do BrasilData na área, com a célula e o sistema da
/// janela do comando ou de um script (`celula`, `crs`), ou, sem nenhum dos dois (um RustGIS sem
/// janelas de plugin), os dos controles do painel. Com o sistema e sem a célula, a padrão da área.
fn grade_do_projeto(req: &Request) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro: a grade cobre o retângulo da área".into());
    }
    let posicao = |id: &str| req.panel.get(id).and_then(Value::as_u64).map(|i| i as usize);
    let da_area = req.f64("celula").is_none() && req.params.get("crs").is_some();
    let celula = match req.f64("celula") {
        Some(m) => m,
        None => posicao("grade-celula").and_then(|i| grade::CELULAS.get(i).copied()).unwrap_or(grade::CELULA_PADRAO),
    };
    let escolha = match req.params.get("crs") {
        Some(v) => Escolha::do_parametro(v)?,
        None => posicao("grade-sistema").and_then(|i| Escolha::DO_PAINEL.get(i).copied()).unwrap_or(Escolha::Automatico),
    };
    if !da_area && grade::celulas(c, celula) > grade::MAX_CELULAS {
        return Err(format!(
            "uma grade de {} teria cerca de {} milhões de células nesta área, mais que o RustGIS aceita (1 bilhão): escolha {} ou mais",
            grade::texto_da_celula(celula),
            (grade::celulas(c, celula) / 1e6).round(),
            grade::texto_da_celula(grade::celula_padrao(c))
        ));
    }
    let (g, outra) = if da_area { grade_nova(c, escolha, None) } else { (Grade::nova(c, escolha, celula), None) };
    let g = por_grade(g)?;
    vista_no_sistema(c, Some(g.epsg));
    let outra = outra.map_or(String::new(), |t| format!("{t}. "));
    let t = format!(
        "{outra}Grade do projeto: {}. Os rasters do BrasilData vão para ela, e o mapa passa a ser desenhado no sistema dela",
        g.texto()
    );
    avisar(&t);
    let mut r = json!({"layer": GRADE, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "report": t});
    grade_no_painel(&mut r, &g);
    Ok(r)
}

/// A grade para um raster na área de retângulo `c`: a do mapa, refeita se não cobre mais a área
/// (a camada da área trocada à mão) ou se as células dela não cabem no RustGIS nesta área, ou,
/// sem grade, a padrão (sistema automático) com `celula` ou a célula padrão da área (30 m num
/// município, maior numa área grande). Um `celula` diferente do da grade é recusado: os rasters
/// ficariam desalinhados.
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
            // A célula de uma grade que cobre a área vale: é a que ficou com ela, ou a escolhida depois.
            if g.cobre(c) && grade::celulas(c, g.celula) <= grade::MAX_CELULAS {
                g
            } else {
                // Uma célula pedida neste comando (a mesma da grade) conta como escolhida.
                let (nova, outra) = grade_refeita(c, Grade { automatica: g.automatica && celula.is_none(), ..g });
                por_grade(nova)?;
                if let Some(t) = outra {
                    avisar(&t);
                }
                return Err(GRADE_FEITA.into());
            }
        }
        None => {
            let (nova, outra) = match celula {
                Some(m) => (Grade::nova(c, Escolha::Automatico, m), None),
                None => grade_nova(c, Escolha::Automatico, None),
            };
            let g = por_grade(nova)?;
            avisar(&format!(
                "{}Grade do projeto criada: {}. Para outra, use Grade do projeto no painel Área de trabalho",
                outra.map_or(String::new(), |t| format!("{t}. ")),
                g.texto()
            ));
            return Err(GRADE_FEITA.into());
        }
    };
    Ok(g)
}

/// A célula `celula` da grade, ou, se nesta área de retângulo `c` ela passa do limite do RustGIS
/// ([`grade::MAX_CELULAS`]), a padrão da área, dito num aviso.
fn celula_que_cabe(c: [f64; 4], celula: f64) -> f64 {
    if grade::celulas(c, celula) <= grade::MAX_CELULAS {
        return celula;
    }
    let m = grade::celula_padrao(c);
    avisar(&format!(
        "Grade do projeto: células de {} dariam cerca de {} milhões nesta área, mais que o RustGIS aceita; ficou com {}",
        grade::texto_da_celula(celula),
        (grade::celulas(c, celula) / 1e6).round(),
        grade::texto_da_celula(m)
    ));
    m
}

/// A grade nova da área de retângulo `c`, com a `celula` escolhida ou, sem ela, a padrão da área, e o
/// que dizer ao usuário se a célula teve de ser outra ([`grade::celula_da_grade`]). Uma célula
/// trocada conta como a padrão: numa área menor, ela volta.
fn grade_nova(c: [f64; 4], escolha: Escolha, celula: Option<f64>) -> (Grade, Option<String>) {
    let pedida = celula.unwrap_or_else(|| grade::celula_padrao(c));
    let (m, outra) = grade::celula_da_grade(c, escolha, pedida, sem_imagens);
    (Grade { automatica: celula.is_none() || outra.is_some(), ..Grade::nova(c, escolha, m) }, outra)
}

/// A grade `g` refeita para a área de retângulo `c`: o mesmo sistema, e a célula que alguém escolheu
/// ou, se ela era a padrão da outra área, a padrão desta.
fn grade_refeita(c: [f64; 4], g: Grade) -> (Grade, Option<String>) {
    grade_nova(c, g.escolha, (!g.automatica).then_some(g.celula))
}

/// Recusa, antes de ler qualquer arquivo, uma grade fina demais para a área: com células de
/// `celula` m, a grade do retângulo `c` passaria do que uma operação de raster do RustGIS aceita
/// ([`grade::MAX_VALORES_OPERACAO`]), e o comando pararia no meio com o erro em inglês.
fn conferir_grade_fina(c: [f64; 4], celula: f64, dado: &str) -> Result<(), String> {
    let menor = grade::menor_celula_da_operacao(c);
    if celula >= menor {
        return Ok(());
    }
    Err(format!(
        "{dado}: com células de {} a grade desta área teria cerca de {} milhões de células, e o RustGIS processa até 32 milhões por vez; use uma grade de {} m ou mais (Grade do projeto), ou uma área menor",
        grade::texto_da_celula(celula),
        (grade::celulas(c, celula) / 1e6).round(),
        menor
    ))
}

/// O "erro" com que [`grade_para`] para um comando depois de criar ou refazer a grade: o
/// [`executar`] o troca por uma resposta, e o comando não é desfeito (ver lá por quê).
const GRADE_FEITA: &str = "BrasilData: grade feita (aviso interno)";

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
    Ok(nomes_nas_feicoes(&area_simplificada(c)?))
}

/// A tolerância da área simplificada, em km: 1/50 da diagonal do retângulo `c`.
fn tolerancia_dos_nomes(c: [f64; 4]) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let diagonal_km = ((c[2] - c[0]) * 111.32 * lat.cos()).hypot((c[3] - c[1]) * 110.57);
    (diagonal_km / 50.0).max(0.1)
}

/// A área com o contorno bem simplificado, para ler os atributos ([`nomes_da_area`]).
fn area_simplificada(c: [f64; 4]) -> Result<Value, String> {
    let copia = format!("{AREA} (nomes)");
    remover(&[&copia])?;
    let r = host::run("gp.run", json!({"tool": "simplify", "params": {"input": AREA, "tolerance": tolerancia_dos_nomes(c)}}))?;
    host::run(
        "layer.rename",
        json!({"layer": r.get("layer").cloned().ok_or("a simplificação não disse a camada que fez")?, "name": copia}),
    )?;
    let fc = host::features(&copia, false);
    remover(&[&copia])?;
    fc
}

/// O retângulo `[oeste, sul, leste, norte]` da camada `nome`, pelo RustGIS.
fn extensao(nome: &str) -> Result<Option<[f64; 4]>, String> {
    Ok(camada_no_projeto(nome)?.and_then(|l| {
        let e = l["extent"].as_array()?;
        Some([e.first()?.as_f64()?, e.get(1)?.as_f64()?, e.get(2)?.as_f64()?, e.get(3)?.as_f64()?])
    }))
}

/// O retângulo `[oeste, sul, leste, norte]` de cada camada com `nome` no nome, num pedido só: um por
/// camada ([`extensao`]) gasta a cota de instruções do comando quando são muitas.
fn extensoes_das(nome: &str) -> Result<HashMap<String, [f64; 4]>, String> {
    let info = host::run("info.project", json!({"nameFilter": nome}))?;
    Ok(info["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|l| {
            let e = l["extent"].as_array()?;
            let n = |i: usize| e.get(i)?.as_f64();
            Some((l["name"].as_str()?.to_string(), [n(0)?, n(1)?, n(2)?, n(3)?]))
        })
        .collect())
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
    area_municipal(&m.into())
}

/// A área de `t` com o contorno oficial do IBGE, quando `t` é um município ([`afinar_area`]).
///
/// O limite oficial é pedido antes de mudar o mapa: no app ele chega depois, e o comando, que
/// fica pendente sem ter mexido no projeto, roda de novo quando ele chega e põe a área já com ele.
/// Pôr a área e só então pedir o limite não dá: a nova rodada seria recusada porque a área mudou
/// desde o começo do comando, e ele seria desfeito. Pelo mesmo motivo, se o limite oficial não vem
/// (o WFS do IBGE fora do ar), ele não é pedido outra vez nesta rodada: o RustGIS responde uma
/// falha uma vez só, e o segundo pedido, já com a área no mapa, seria um novo download pendente. A
/// reserva, a malha máxima, também vem antes.
fn area_municipal(t: &Territorio) -> Result<Value, String> {
    // Sem o limite oficial: o código do município e a malha máxima, se ela veio.
    let mut sem_oficial: Option<(&str, Option<Value>)> = None;
    if let [codigo] = t.codigos.as_slice()
        && codigo.len() == 7
    {
        let oficial = format!("{AREA} (limite oficial {codigo})");
        if camada_no_projeto(&oficial)?.is_none() {
            match host::run("layer.addUrl", json!({"url": malhas::url_oficial(codigo), "name": oficial, "zoom": false})) {
                Ok(a) if a.get("pending").and_then(Value::as_bool) == Some(true) => {
                    avisar(&format!("Área: baixando o limite oficial de {} (IBGE); a área aparece quando ele chegar", t.nome));
                    return Ok(a);
                }
                Ok(_) => {}
                Err(e) => host::log(&format!("área: o limite oficial do IBGE não veio ({e})")),
            }
        }
        if camada_no_projeto(&oficial)?.is_none_or(|l| l["features"].as_u64().unwrap_or(0) == 0) {
            sem_oficial = Some((codigo, None));
            match buscar_varios(&[malhas::url_maxima(codigo)]) {
                Ok(Ok(r)) => sem_oficial = Some((codigo, r.into_iter().next())),
                Ok(Err(p)) => return Ok(p),
                Err(e) => host::log(&format!("área: a malha máxima do IBGE não veio ({e})")),
            }
        }
    }
    let mut r = area_de(t)?;
    let trocada = match sem_oficial {
        None => afinar_area(false)?.is_some(),
        Some((codigo, maxima)) => {
            remover(&[&format!("{AREA} (limite oficial {codigo})")])?;
            let trocada = match maxima {
                Some(m) => trocar_pela_maxima(&host::features(AREA, false)?, codigo, m)?,
                None => false,
            };
            avisar(if trocada {
                "Área: o limite oficial do IBGE não veio (o serviço dele não respondeu); o contorno é o da malha máxima do IBGE"
            } else {
                "Área: o limite oficial do IBGE não veio (o serviço dele não respondeu); o contorno é o que vem no plugin"
            });
            trocada
        }
    };
    if trocada {
        // O limite oficial passa um pouco do contorno embutido: a grade, feita para ele, é
        // refeita para cobrir o oficial (senão o primeiro comando de dados a refaria).
        let (c, _) = retangulo()?;
        if let Some(g) = grade_no_mapa()?
            && !g.cobre(c)
        {
            let g = por_grade(grade_refeita(c, g).0)?;
            r["bbox"] = json!(c);
            grade_no_painel(&mut r, &g);
        }
    }
    Ok(r)
}

/// Troca o contorno da área `fc` (as feições dela) pelo da `resposta` da API do IBGE para o
/// município `codigo` ([`malhas::url_maxima`]). `false` se a resposta não tem contorno.
fn trocar_pela_maxima(fc: &Value, codigo: &str, resposta: Value) -> Result<bool, String> {
    let novo = malhas::com_malhas(fc, &[(codigo.to_string(), resposta)], "maxima");
    if novo == *fc {
        return Ok(false);
    }
    remover(&[AREA])?;
    host::add_layer(AREA, novo)?;
    Ok(true)
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
fn afinar_area(de_dados: bool) -> Result<Option<Value>, String> {
    // Trocado o contorno, o comando para aqui. Seguir com a área trocada e uma janela pendente
    // não dá: quando a janela chega, o RustGIS roda o comando de novo e o recusa porque a área
    // mudou desde o começo ("the live source or account changed since this request started");
    // o comando inteiro é desfeito, a troca também, e cada clique repetia o mesmo.
    let trocada = |de: &str| -> Value {
        let t = if de_dados {
            format!("Área: contorno trocado pelo {de} do IBGE. Clique de novo para trazer os dados")
        } else {
            format!("Área: contorno trocado pelo {de} do IBGE")
        };
        avisar(&t);
        json!({"area": AREA, "report": t})
    };
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
                avisar("Área: baixando o limite oficial do município (IBGE). Quando ele chegar, clique de novo");
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
        return Ok(Some(trocada("limite oficial")));
    }
    remover(&[&oficial])?;
    // A reserva: a malha máxima da API, pequena, lida pelo plugin.
    match buscar_varios(&[malhas::url_maxima(codigo)]) {
        Ok(Ok(r)) => {
            if trocar_pela_maxima(&fc, codigo, r.into_iter().next().unwrap_or_default())? {
                return Ok(Some(trocada("contorno da malha máxima")));
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
    let mut geojson = String::from("{\"type\":\"FeatureCollection\",\"features\":[");
    let mut caixa = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for (i, codigo) in t.codigos.iter().enumerate() {
        let mut p = json!({"nome": t.nome, "codigo": codigo});
        if !t.uf.is_empty() {
            p["uf"] = json!(t.uf);
        }
        // Só o contorno de um município se troca pelo oficial ([`afinar_area`], que assim reconhece
        // as outras áreas sem ler as feições).
        if codigo.len() != 7 {
            p["malha"] = json!("embutida");
        }
        if i > 0 {
            geojson.push(',');
        }
        geojson.push_str(&format!("{{\"type\":\"Feature\",\"properties\":{p},\"geometry\":"));
        let r = malhas::escrever_geometria(codigo, &mut geojson)
            .ok_or_else(|| format!("o plugin não tem o limite de {} ({codigo})", t.nome))?;
        geojson.push('}');
        caixa = [caixa[0].min(r[0]), caixa[1].min(r[1]), caixa[2].max(r[2]), caixa[3].max(r[3])];
    }
    geojson.push_str("]}");
    let nome = if t.uf.is_empty() { t.nome.clone() } else { format!("{} ({})", t.nome, t.uf) };
    por_area(&geojson, caixa, &texto_da_area(&[nome], t.codigos.len(), Some(caixa)))
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
    avisar(&format!("{}. A área só muda quando você usar o desenho", DICA_DO_DESENHO));
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
    let fc = json!({"type": "FeatureCollection", "features": feicoes});
    let mut r = por_area(&fc.to_string(), caixa_de(&fc).ok_or("a área não tem geometria")?, &descrever(&fc))?;
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
    avisar(&format!("Área do BrasilData: {texto}"));
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
    if t.id == "drenagem" {
        return drenagem_bc250(t).map_err(crate::explicar);
    }
    if t.id == "rodovias" {
        return rodovias_bc250(t);
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
        avisar(&format!("{nome}: {aviso}; escolha uma área menor ({})", t.fonte));
    }
    if r.get("downloading").is_some() {
        // Um RustGIS que não roda o comando de novo põe a camada nova sem mais nada: a antiga sai
        // agora. Num que roda, esta volta é desfeita, e a antiga sai depois da chamada.
        remover(&[&nome, &bruto])?;
        avisar(&if na_area {
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
    avisar(&format!("{curto} já está no mapa, recortada por esta área; para trazer de novo, remova a camada"));
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
        avisar(&format!("{nome}: nenhuma feição {onde} ({})", t.fonte));
        return Ok(json!({"layer": null, "features": 0}));
    }
    if !na_area {
        let onde = if area { "no retângulo da área" } else { onde };
        avisar(&format!("{nome}: {} {onde} ({})", feicoes(lidas), t.fonte));
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
            avisar(&format!("{nome}: nada dentro da área ({} no retângulo; {})", feicoes(lidas), t.fonte));
            Ok(json!({"layer": null, "read": lidas, "features": 0, "clipped": true}))
        }
        _ => {
            let n = ficaram.map_or(String::new(), |n| format!("{n} de "));
            avisar(&format!(
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
        avisar(&format!("{nome}: a data em texto (data_hora_utc) não foi calculada ({e}); {campo} está em milissegundos desde 1970, UTC"));
    }
}

/// Mais feições que isto num tema pedido pelo plugin num pedido só é área grande demais (o mesmo
/// teto do RustGIS para as páginas de um serviço).
const MAX_FEICOES: u64 = 100_000;

/// O teto de um tema baixado em páginas (`pagina` > 0): cada página é um download à parte do
/// RustGIS, juntado no fim, e o teto de um serviço não vale. Em Minas Gerais (22/11/2026), 182 mil
/// trechos da drenagem da BC250 pelo WFS, em 10 páginas de 20 mil, vieram em 2 min 15 s com 418 MB
/// de memória.
const MAX_FEICOES_EM_PAGINAS: u64 = 300_000;

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
    avisar(&if minima > 0.0 {
        format!(
            "{curto}: só os rios com área de drenagem de {} ou mais (os menores não cabem nesta área; numa área menor eles vêm). De longe aparecem só os principais; aproxime o mapa para ver os menores",
            zoom::km2(minima)
        )
    } else {
        format!("{curto}: de longe aparecem só os rios principais (pela área de drenagem); aproxime o mapa para ver os menores")
    });
    Ok(())
}

/// A drenagem da BC250 com a ordem de Strahler e a drenagem a montante (veja [`drenagem`]): numa
/// área de até [`drenagem::MAX_UFS`] UFs, os arquivos delas, juntados e recortados; sem área, ou
/// numa área maior, o país inteiro pela vista.
fn drenagem_bc250(t: &Tema) -> Result<Value, String> {
    // Sem área, o país inteiro, em qualquer escala: o `retangulo` da vista recusa uma vista maior
    // que o Brasil, e aqui o RustGIS lê só o que a vista mostra.
    if !nomes()?.iter().any(|n| n == AREA) {
        return drenagem_do_brasil(t, "");
    }
    let (c, _) = retangulo()?;
    let ufs = match ufs_da_area(c)? {
        Ok(u) => u,
        Err(p) => return Ok(p),
    };
    if ufs.is_empty() || ufs.len() > drenagem::MAX_UFS {
        let motivo = format!(
            " (a área cruza {} UFs; o plugin baixa e recorta até {}: para recortar, escolha uma área menor)",
            ufs.len(),
            drenagem::MAX_UFS
        );
        return drenagem_do_brasil(t, &motivo);
    }
    let nome = t.camada_no_mapa();
    let bruto = format!("{nome} (sem recorte)");
    let rotulos = drenagem::camada_dos_rotulos(&nome);
    let bruto_rot = format!("{rotulos} (sem recorte)");
    let partes = partes_por_uf(&ufs, &nome, &bruto);
    let partes_rot = partes_por_uf(&ufs, &rotulos, &bruto_rot);
    // Nos apps, as UFs que já chegaram (de um clique anterior, ou da volta do RustGIS quando um
    // download chega) ficam; só as que faltam são pedidas.
    let ha = nomes()?;
    let mut pedidos: Vec<(String, &String)> = Vec::new();
    for (i, uf) in ufs.iter().enumerate() {
        pedidos.push((drenagem::url_uf(uf), &partes[i]));
        // Os nomes dos rios só se a camada deles ainda não está no mapa (de um clique anterior).
        if !ha.contains(&rotulos) {
            pedidos.push((drenagem::url_rotulos_uf(uf), &partes_rot[i]));
        }
    }
    let (mut baixando, mut cheio) = (0, false);
    for (url, parte) in pedidos {
        if ha.contains(parte) {
            continue;
        }
        // Inteiro ("whole"): lido pela vista, o recorte e o merge baixariam o arquivo de novo.
        let pedido = json!({"url": url, "name": parte, "zoom": false, "whole": true});
        match host::run("layer.addUrl", pedido) {
            Ok(r) if r.get("pending").and_then(Value::as_bool) == Some(true) => baixando += 1,
            Ok(_) => {}
            Err(e) if crate::fila_cheia(&e) => {
                cheio = true;
                break;
            }
            Err(e) => return Err(e),
        }
    }
    if baixando > 0 || cheio {
        avisar(&format!(
            "{nome}: baixando a drenagem de {} (com a ordem e os nomes dos rios); quando tudo aparecer no mapa, clique de novo neste botão para juntar e recortar pela área",
            ufs.join(", ")
        ));
        return Ok(json!({"downloading": nome, "parts": partes.len() * 2}));
    }
    let lidas = juntar(&partes, &bruto)?;
    let r = concluir(t, lidas, true, true)?;
    if let Some(camada) = r["layer"].as_str() {
        estilo_da_drenagem(camada, 0.0)?;
        if !ha.contains(&rotulos) {
            juntar(&partes_rot, &bruto_rot)?;
            let recortada = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto_rot, "clip": AREA}}))?;
            let feita = recortada.get("layer").cloned().ok_or("o clip dos nomes dos rios não disse a camada que fez")?;
            host::run("layer.rename", json!({"layer": feita, "name": rotulos}))?;
            remover(&[&bruto_rot])?;
        }
        rotulos_da_drenagem(&rotulos)?;
        avisar(&format!(
            "{}: de longe aparecem só os rios com mais drenagem a montante (campo {}), e os nomes dos rios maiores têm a preferência; aproxime o mapa para ver os menores. A ordem de Strahler está no campo strahler",
            camada.trim_start_matches(PREFIXO),
            drenagem::MONTANTE
        ));
    }
    Ok(r)
}

/// Os nomes das camadas das UFs de um tema baixado por UF: com uma UF só, a camada `bruto` (sem
/// recorte) direto; com mais, `{nome} ({UF})`, juntadas depois.
fn partes_por_uf(ufs: &[String], nome: &str, bruto: &str) -> Vec<String> {
    if ufs.len() == 1 { vec![bruto.to_string()] } else { ufs.iter().map(|uf| format!("{nome} ({uf})")).collect() }
}

/// Os rótulos da drenagem na camada dos rios em linhas contínuas (`camada`): a linha invisível, o
/// nome do rio, e uma classe por faixa de ordem de Strahler, colocadas na ordem de
/// [`drenagem::CLASSES_DE_ROTULO`], de modo que os rios maiores ficam com os lugares e os menores
/// entram onde sobra.
fn rotulos_da_drenagem(camada: &str) -> Result<(), String> {
    host::run("layer.color", json!({"layer": camada, "color": "#1f78b400"}))?;
    host::run("labels.enable", json!({"layer": camada, "on": true}))?;
    host::run("labels.class.clear", json!({"layer": camada}))?;
    let (ultima, antes) = drenagem::CLASSES_DE_ROTULO.split_last().ok_or("sem classes de rótulo")?;
    for c in antes {
        host::run("labels.class.add", json!({"layer": camada, "name": c.nome, "where": c.onde}))?;
    }
    host::run("labels.class.update", json!({"layer": camada, "where": ultima.onde}))?;
    let n = drenagem::CLASSES_DE_ROTULO.len();
    for (i, c) in drenagem::CLASSES_DE_ROTULO.iter().enumerate() {
        // A última é a Default (sem "class"), com a cláusula que sobra.
        let qual = |mut v: Value| {
            if i + 1 < n {
                v["class"] = json!(c.nome);
            }
            v
        };
        host::run("labels.field", qual(json!({"layer": camada, "field": "nome"})))?;
        host::run(
            "labels.style",
            qual(json!({"layer": camada, "size": c.tamanho, "italic": true, "color": c.cor, "halo": true, "haloColor": "#FFFFFF"})),
        )?;
        host::run("labels.placement", qual(json!({"layer": camada, "position": c.posicao, "otherPositions": true})))?;
        if let Some(l) = c.longe {
            host::run("labels.scaleRange", qual(json!({"layer": camada, "min": l})))?;
        }
    }
    Ok(())
}

/// O estilo de uma camada da drenagem que já garante `minimo` km a montante: azul (a cor do
/// símbolo, que a legenda mostra), a largura pela ordem de Strahler e os filtros por escala.
fn estilo_da_drenagem(camada: &str, minimo: f64) -> Result<(), String> {
    host::run("layer.color", json!({"layer": camada, "color": drenagem::AZUL}))?;
    host::run("layer.symbolBy", json!({"layer": camada, "strokeWidth": drenagem::LARGURA_PELA_ORDEM}))?;
    host::run("layer.displayFilters", json!({"layer": camada, "filters": drenagem::filtros(minimo)}))?;
    Ok(())
}

/// O país inteiro, lido pela web só na parte da vista, em níveis num grupo (veja
/// [`drenagem::NIVEIS`]). `motivo` explica por que não veio recortado.
fn drenagem_do_brasil(t: &Tema, motivo: &str) -> Result<Value, String> {
    let grupo = t.camada_no_mapa();
    let ha = nomes()?;
    let mut camadas = Vec::new();
    let mut baixando = 0;
    // As linhas, de longe para perto, e por cima delas os nomes dos rios.
    let niveis = drenagem::NIVEIS.iter().map(|n| (n, false)).chain(drenagem::NIVEIS_DOS_ROTULOS.iter().map(|n| (n, true)));
    for (&(arquivo, rotulo, longe, perto), so_nomes) in niveis {
        let nome = format!("{grupo}: {rotulo}");
        if !ha.contains(&nome) {
            let r = host::run("layer.addUrl", json!({"url": format!("{}{arquivo}", drenagem::BASE), "name": nome, "zoom": false}))?;
            if r.get("pending").and_then(Value::as_bool) == Some(true) {
                baixando += 1;
                continue;
            }
        }
        let mut faixa = json!({"layer": nome});
        if let Some(l) = longe {
            faixa["min"] = json!(l);
        }
        if let Some(p) = perto {
            faixa["max"] = json!(p);
        }
        host::run("layer.scaleRange", faixa)?;
        if so_nomes {
            rotulos_da_drenagem(&nome)?;
        } else {
            estilo_da_drenagem(&nome, drenagem::minimo_do_nivel(arquivo))?;
        }
        camadas.push(nome);
    }
    if baixando > 0 {
        avisar(&format!("{grupo}: abrindo os arquivos do país inteiro; a drenagem aparece no mapa quando eles chegarem"));
        return Ok(json!({"downloading": grupo, "parts": drenagem::NIVEIS.len() + drenagem::NIVEIS_DOS_ROTULOS.len()}));
    }
    // Todos num grupo com o nome do tema.
    let info = host::run("info.project", json!({}))?;
    let ja_agrupadas = info["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["name"].as_str().is_some_and(|n| camadas.iter().any(|c| c == n)))
        .all(|l| l["group"].as_str() == Some(grupo.as_str()));
    if !ja_agrupadas {
        host::run("group.add", json!({"name": grupo, "layers": camadas}))?;
    }
    avisar(&format!(
        "{grupo}: o país inteiro, lido pela vista{motivo}. De longe aparecem só os rios com mais drenagem a montante (campo {}), e os nomes dos rios maiores têm a preferência; aproxime o mapa para ver os menores. A ordem de Strahler está no campo strahler",
        drenagem::MONTANTE
    ));
    Ok(json!({"group": grupo, "layers": camadas, "streamed": true}))
}

/// As rodovias e estradas da BC250 ([`rodovias`]): pedidas como os outros temas (em páginas, e
/// recortadas pela área), com o estilo por jurisdição e revestimento, os filtros por escala e as
/// siglas das federais e estaduais pavimentadas.
fn rodovias_bc250(t: &Tema) -> Result<Value, String> {
    let r = tema_proprio(t)?;
    if estilo::esperando(&r) {
        return Ok(r);
    }
    if let Some(camada) = r["layer"].as_str() {
        estilo_de_vias(camada, rodovias::ESTILO_BC250, rodovias::CLASSES_BC250, "jurisdicao", rodovias::filtros_bc250())?;
        siglas_das_rodovias(camada)?;
        avisar(&format!(
            "{}: cores pela jurisdição e pelo revestimento; de longe aparecem só as federais, e de 1:1.500.000 para perto todos os trechos. As siglas das federais e estaduais pavimentadas aparecem de 1:2.000.000 para perto",
            camada.trim_start_matches(PREFIXO)
        ));
    }
    Ok(r)
}

/// O fim do nome da camada das siglas das rodovias.
const SIGLAS: &str = ": siglas";

/// Se a camada `nome` só leva rótulos (os nomes dos rios, as siglas das rodovias): a linha dela é
/// invisível, e ela fica fora da legenda.
fn so_rotulos(nome: &str) -> bool {
    drenagem::so_rotulos(nome) || nome.ends_with(SIGLAS)
}

/// As siglas das federais e estaduais pavimentadas numa camada à parte (`{camada}: siglas`), de
/// linha invisível: a BC250 corta cada rodovia em trechos curtos, e o rótulo em cada trecho (e
/// repetido ao longo dele) enchia a rodovia de siglas. Uma cópia da camada fica só com o texto do
/// rótulo, agrupada numa feição por sigla; os rótulos da camada das rodovias saem.
fn siglas_das_rodovias(camada: &str) -> Result<(), String> {
    let siglas = format!("{camada}{SIGLAS}");
    let provisoria = format!("{camada} (siglas, montando)");
    remover(&[&siglas, &provisoria])?;
    let _ = host::run("labels.enable", json!({"layer": camada, "on": false}));
    let copia = host::run("gp.run", json!({"tool": "merge", "params": {"inputs": [camada]}}))?;
    let copia = copia.get("layer").cloned().ok_or("o merge das siglas não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": copia, "name": provisoria}))?;
    host::run("edit.addField", json!({"layer": provisoria, "name": "rotulo", "type": "text"}))?;
    host::run("edit.calculateField", json!({"layer": provisoria, "field": "rotulo", "expression": rodovias::ROTULO_BC250}))?;
    let campos = host::run("layer.fields", json!({"layer": provisoria}))?;
    for f in campos.as_array().into_iter().flatten().filter_map(|f| f["name"].as_str()) {
        if f != "rotulo" {
            let _ = host::run("edit.deleteField", json!({"layer": provisoria, "field": f}));
        }
    }
    // Uma feição por sigla (todos os trechos dela, uma multilinha): o rótulo vai no trecho mais
    // longo dela na vista, e a repetição espaça os outros. O Unsplit Line só juntava trechos por
    // nós de grau 2, e a BC250 corta as rodovias em cada cruzamento com uma estrada de terra.
    let juntas =
        host::run("gp.run", json!({"tool": "geolibre:singlepart_to_multipart", "params": {"input": provisoria, "field": "rotulo"}}))?;
    // As ferramentas da GeoLibre dizem as camadas que fizeram em "layers".
    let juntas =
        juntas.get("layer").or_else(|| juntas["layers"].get(0)).cloned().ok_or("o agrupamento das siglas não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": juntas, "name": siglas}))?;
    remover(&[&provisoria])?;
    host::run("layer.color", json!({"layer": siglas, "color": "#00000000"}))?;
    host::run("labels.field", json!({"layer": siglas, "field": "rotulo"}))?;
    host::run("labels.placement", json!({"layer": siglas, "position": "parallel", "repeat": rodovias::SIGLAS_A_CADA}))?;
    host::run("labels.style", json!({"layer": siglas, "size": 6.5, "color": "#4a1010", "halo": true}))?;
    host::run("labels.scaleRange", json!({"layer": siglas, "min": rodovias::SIGLAS_MAIS_LONGE}))?;
    host::run("labels.enable", json!({"layer": siglas, "on": true}))?;
    Ok(())
}

/// O estilo de uma camada de vias (BC250 ou OpenStreetMap): Valores Únicos pelas `classes` (um
/// estilo do projeto, `nome`, que serve também a uma camada lida no lugar), o traço de cada classe
/// com as principais por cima, e os filtros por escala. Uma camada que chega com um símbolo só (a
/// que o RustGIS pôs quando o download chegou) perde o estilo que o plugin guardou dela antes do
/// comando, que voltaria no fim por cima deste.
fn estilo_de_vias(camada: &str, nome: &str, classes: &[rodovias::Classe], campo: &str, filtros: Value) -> Result<(), String> {
    let simples = camada_no_projeto(camada)?.is_none_or(|l| l["renderer"].as_str() == Some("Single Symbol"));
    let _ = host::run("style.remove", json!({"name": nome}));
    host::run("style.add", json!({"text": rodovias::estilo(nome, classes, campo)}))?;
    host::run("style.apply", json!({"layer": camada, "name": nome}))?;
    host::run("layer.strokeLevels", json!({"layer": camada, "on": true}))?;
    for mut traco in rodovias::tracos(classes) {
        traco["layer"] = json!(camada);
        host::run("layer.lineStrokes", traco)?;
    }
    host::run("layer.displayFilters", json!({"layer": camada, "filters": filtros}))?;
    if simples {
        let _ = host::run("style.remove", json!({"name": estilo::guardado(camada)}));
    }
    Ok(())
}

/// As ruas e estradas do OpenStreetMap ([`rodovias`]): os segmentos do tema de transportes do
/// Overture nos arquivos que cobrem a área (ou a vista), lidos no lugar pelo RustGIS, só as vias
/// (`subtype = 'road'`), com o estilo por classe, visíveis só de perto e mascarados pela área.
fn osm_vias() -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    let (versao, arquivos) = match arquivos_do_overture(
        c,
        rodovias::COLECAO_OSM,
        "vias",
        rodovias::MAX_ARQUIVOS_OSM,
        ", ou use Rodovias e estradas (IBGE BC250)",
    )? {
        Ok(a) => a,
        Err(p) => return Ok(p),
    };
    let camadas = match camadas_do_overture(&arquivos, rodovias::camadas_osm(arquivos.len()), rodovias::eh_do_osm)? {
        Ok(c) => c,
        Err(p) => return Ok(p),
    };
    for nome in &camadas {
        host::run("layer.definitionQuery", json!({"layer": nome, "where": "subtype = 'road'"}))?;
        estilo_de_vias(nome, rodovias::ESTILO_OSM, rodovias::CLASSES_OSM, "class", rodovias::filtros_osm())?;
        host::run("layer.scaleRange", json!({"layer": nome, "min": rodovias::OSM_MAIS_LONGE}))?;
        if na_area {
            let _ = host::run("layer.mask", json!({"layer": nome, "mask": AREA}));
        }
    }
    let texto = format!(
        "{}: vias do OpenStreetMap pelo Overture Maps ({} arquivo(s), versão {versao}), lidas no lugar, só o que a vista mostra; {}. Aparecem de 1:150.000 para perto (de longe, use Rodovias e estradas do IBGE), e de 1:50.000 para perto também as trilhas e as vias de serviço. Os nomes das vias não vêm. © colaboradores do OpenStreetMap (ODbL)",
        camadas[0].trim_start_matches(PREFIXO),
        camadas.len(),
        if na_area { "mascaradas pelo contorno da área, sem recorte" } else { "sem área, as da vista" }
    );
    avisar(&texto);
    Ok(json!({"layers": camadas, "files": camadas.len(), "report": texto}))
}

/// Os endereços do CNEFE ([`enderecos`]): os pontos do tema de endereços do Overture nos arquivos que
/// cobrem a área (ou a vista), lidos no lugar, com e sem número, visíveis só de perto, com o número
/// como rótulo e mascarados pela área.
fn enderecos_na_area() -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    let (versao, arquivos) = match arquivos_do_overture(c, enderecos::COLECAO, "endereços", enderecos::MAX_ARQUIVOS, "")? {
        Ok(a) => a,
        Err(p) => return Ok(p),
    };
    let camadas = match camadas_do_overture(&arquivos, enderecos::camadas(arquivos.len()), enderecos::eh_daqui)? {
        Ok(c) => c,
        Err(p) => return Ok(p),
    };
    for nome in &camadas {
        // Uma camada que o usuário já estilizou fica como ele a deixou (`com_estilo`).
        let simples = camada_no_projeto(nome)?.is_none_or(|l| l["renderer"].as_str() == Some("Single Symbol"));
        let _ = host::run("style.remove", json!({"name": enderecos::ESTILO}));
        host::run("style.add", json!({"text": enderecos::estilo()}))?;
        host::run("style.apply", json!({"layer": nome, "name": enderecos::ESTILO}))?;
        host::run("layer.scaleRange", json!({"layer": nome, "min": enderecos::MAIS_LONGE}))?;
        host::run("labels.enable", json!({"layer": nome, "on": true}))?;
        host::run("labels.class.update", json!({"layer": nome, "where": enderecos::rotulados()}))?;
        host::run("labels.scaleRange", json!({"layer": nome, "min": enderecos::NUMEROS_MAIS_LONGE}))?;
        host::run("labels.placement", json!({"layer": nome, "position": "right", "otherPositions": true}))?;
        if na_area {
            let _ = host::run("layer.mask", json!({"layer": nome, "mask": AREA}));
        }
        if simples {
            let _ = host::run("style.remove", json!({"name": estilo::guardado(nome)}));
        }
    }
    let texto = format!(
        "{}: um ponto por endereço do Censo 2022 (IBGE), pelo Overture Maps ({} arquivo(s), versão {versao}), com a rua, o número e o CEP, lidos no lugar, só o que a vista mostra; {}. Aparecem de 1:50.000 para perto, e os números, de 1:3.000",
        camadas[0].trim_start_matches(PREFIXO),
        camadas.len(),
        if na_area { "mascarados pelo contorno da área, sem recorte" } else { "sem área, os da vista" }
    );
    avisar(&texto);
    Ok(json!({"layers": camadas, "files": camadas.len(), "report": texto}))
}

/// A versão do Overture e os GeoParquet da coleção `tema` que cobrem `c` (até `maximo`): do catálogo,
/// da coleção e dos itens dos arquivos (veja `rodovias::STAC_OVERTURE`). Cada busca pode ficar
/// pendente no app web: o `Err` de dentro. `o_que` é o que a coleção tem; `ou` completa o aviso de
/// área grande demais.
fn arquivos_do_overture(
    c: [f64; 4],
    tema: &str,
    o_que: &str,
    maximo: usize,
    ou: &str,
) -> Result<Result<(String, Vec<String>), Value>, String> {
    let passageira = |e: String| {
        if crate::falha_passageira(&e) || e.contains("timeout") {
            format!("o Overture Maps não respondeu agora: tente de novo daqui a pouco ({e})")
        } else {
            e
        }
    };
    let catalogo = match buscar(&[rodovias::url_catalogo_overture()]).map_err(passageira)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let versao =
        catalogo.first().and_then(|r| rodovias::versao_overture(&r["json"])).ok_or("o catálogo do Overture Maps não diz a versão")?;
    let colecao = match buscar(&[rodovias::url_colecao_overture(&versao, tema)]).map_err(passageira)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let itens = colecao.first().map(|r| rodovias::itens_overture(&r["json"], c)).unwrap_or_default();
    if itens.is_empty() {
        return Err(format!("o Overture Maps não tem {o_que} na área"));
    }
    if itens.len() > maximo {
        return Err(format!(
            "a área toca {} arquivos do Overture Maps (até {maximo} de uma vez): escolha uma área menor, como um município ou uma microrregião{ou}",
            itens.len()
        ));
    }
    let respostas = match buscar(&itens).map_err(passageira)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let arquivos: Vec<String> = respostas.iter().filter_map(|r| rodovias::arquivo_overture(&r["json"])).collect();
    if arquivos.len() != itens.len() {
        return Err("um item do Overture Maps não diz o arquivo GeoParquet dele".into());
    }
    Ok(Ok((versao, arquivos)))
}

/// O fim do nome provisório da camada que vai tomar o lugar de outra de mesmo nome.
const NOVA: &str = " (nova)";

/// Os `arquivos` do Overture abertos no lugar, um por camada das `camadas` (as da família são as que
/// `eh_daqui` reconhece): as que estão no mapa, ou o `Err(pendente)` de dentro, enquanto abrem.
fn camadas_do_overture(
    arquivos: &[String],
    camadas: Vec<String>,
    eh_daqui: fn(&str) -> bool,
) -> Result<Result<Vec<String>, Value>, String> {
    // As que já são de um destes arquivos (de um clique anterior, ou da volta do RustGIS quando um
    // download chega) ficam, e só as que faltam são pedidas. As de outros arquivos (outra área, ou
    // outra versão) só saem depois que todas chegaram: na janela, o comando que fica à espera de um
    // download volta atrás no que fez, e a camada nova chegaria ao lado da velha, com "(2)" no nome.
    let mut daqui = Vec::new();
    for nome in nomes()?.into_iter().filter(|n| eh_daqui(n)) {
        let fonte = camada_no_projeto(&nome)?.and_then(|l| l["source"].as_str().map(|s| s.split('?').next().unwrap_or(s).to_string()));
        daqui.push((nome, fonte));
    }
    let do_arquivo =
        |daqui: &[(String, Option<String>)], url: &str| daqui.iter().find(|(_, f)| f.as_deref() == Some(url)).map(|(n, _)| n.clone());
    let mut baixando = 0;
    for (url, nome) in arquivos.iter().zip(&camadas) {
        if do_arquivo(&daqui, url).is_some() {
            continue;
        }
        let ocupado = daqui.iter().any(|(n, _)| n == nome);
        let provisorio = if ocupado { format!("{nome}{NOVA}") } else { nome.clone() };
        let r = host::run("layer.addUrl", json!({"url": url, "name": provisorio, "zoom": false}))?;
        if estilo::esperando(&r) {
            baixando += 1;
        } else {
            daqui.push((provisorio, Some(url.clone())));
        }
    }
    if baixando > 0 {
        let nome = camadas[0].trim_start_matches(PREFIXO).trim_end_matches(" (parte 1)");
        avisar(&format!("{nome}: abrindo os arquivos do Overture Maps; a camada vem quando eles chegarem"));
        return Ok(Err(json!({"downloading": camadas[0], "parts": camadas.len()})));
    }
    let velhas: Vec<&str> =
        daqui.iter().filter(|(_, f)| f.as_ref().is_none_or(|f| !arquivos.contains(f))).map(|(n, _)| n.as_str()).collect();
    remover(&velhas)?;
    // Os nomes por parte, na ordem dos arquivos: primeiro um provisório, porque uma camada que ficou
    // pode ter o nome que agora é de outra.
    let presentes = nomes()?;
    let mut trocas = Vec::new();
    let mut prontas = Vec::new();
    for (url, nome) in arquivos.iter().zip(camadas) {
        let Some(atual) = do_arquivo(&daqui, url).filter(|n| presentes.contains(n)) else { continue };
        if atual != nome {
            let provisorio = format!("{nome}{NOVA}");
            if atual != provisorio {
                host::run("layer.rename", json!({"layer": atual, "name": provisorio}))?;
            }
            trocas.push((provisorio, nome.clone()));
        }
        prontas.push(nome);
    }
    for (provisorio, nome) in trocas {
        host::run("layer.rename", json!({"layer": provisorio, "name": nome}))?;
    }
    if prontas.is_empty() {
        return Err("o Overture Maps não devolveu camada nenhuma para a área".into());
    }
    Ok(Ok(prontas))
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
        avisar(&format!("{nome}: nenhuma feição {onde} ({})", t.fonte));
        return Ok(json!({"layer": null, "features": 0}));
    }
    let teto = if t.pagina > 0 { MAX_FEICOES_EM_PAGINAS } else { MAX_FEICOES };
    if total > teto {
        return Err(format!("{nome}: {total} feições {onde}; o plugin traz até {teto}: escolha uma área menor"));
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
            avisar(&format!(
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
        avisar(&if depois.is_empty() {
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
            // A contagem que passa do tempo do RustGIS é a de trechos demais (no Brasil inteiro, a
            // de 500 km² leva de 13 a mais de 20 s no serviço da ANA, a de 10 mil, 2,5 s): a busca
            // segue para os rios maiores.
            Err(e) if e.contains("timeout") => {
                host::log(&format!("contagem com {} ou mais passou do tempo: {e}", zoom::km2(n)));
                return Ok(u64::MAX);
            }
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
    // Uma parte só, lida pela vista (um GeoParquet grande, como o da drenagem de MG, de 47 MB, que
    // o RustGIS lê por partes mesmo com "whole"): ela diz 0 feições, e o tema acabava em "nenhuma
    // feição na área". O merge de uma camada só a carrega inteira.
    let lida_pela_vista = partes.len() == 1 && feicoes_da_camada(&partes[0])?.unwrap_or(0) == 0;
    if partes.len() == 1 && !lida_pela_vista {
        if partes[0] != destino {
            host::run("layer.rename", json!({"layer": partes[0], "name": destino}))?;
        }
    } else {
        let r = host::run("gp.run", json!({"tool": "merge", "params": {"inputs": partes}}))?;
        let junta = r.get("layer").cloned().ok_or("o merge não disse a camada que fez")?;
        // Um nome provisório: a parte pode já se chamar `destino` (uma parte só), e tirá-la do mapa
        // pelo nome levaria a junta junto.
        let provisorio = format!("{destino} (juntando)");
        host::run("layer.rename", json!({"layer": junta, "name": provisorio}))?;
        let p: Vec<&str> = partes.iter().map(String::as_str).collect();
        remover(&p)?;
        host::run("layer.rename", json!({"layer": provisorio, "name": destino}))?;
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
            // Uma citação por fonte cujos campos estão na camada. Os nomes dos campos vêm do
            // `info.project`: ler as feições trazia junto os contornos da Malha Municipal (mais de
            // 4 MB numa microrregião), que estouravam a cota da chamada.
            let info = host::run("info.project", json!({}))?;
            let campos: Vec<String> = info["layers"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|l| l["name"].as_str() == Some(n.as_str()))
                .and_then(|l| l["fields"].as_array())
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(|f| f.split(" (").next().unwrap_or(f).to_string())
                .collect();
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
        } else if (n.starts_with(&format!("{PREFIXO}Uso e cobertura ")) && n.ends_with(" (MapBiomas, recortado)"))
            || mapbiomas::eh_das_classes(&n)
        {
            lista.push(json!({"layer": n, "source": mapbiomas::CITACAO_RECORTE, "license": mapbiomas::LICENCA_RECORTE}));
        } else if mapbiomas::eh_da_irrigacao(&n) {
            lista.push(json!({"layer": n, "source": mapbiomas::CITACAO_IRRIGACAO, "license": mapbiomas::LICENCA}));
        } else if n == camada_de_saude() {
            lista.push(json!({"layer": n, "source": saude::CITACAO, "license": saude::LICENCA}));
        } else if n == cemaden::CHUVA {
            lista.push(json!({"layer": n, "source": cemaden::CITACAO, "license": cemaden::LICENCA}));
        } else if n == cemaden::ALERTAS {
            lista.push(json!({"layer": n, "source": cemaden::CITACAO_ALERTAS, "license": cemaden::LICENCA}));
        } else if radar_pc::eh_daqui(&n) {
            lista.push(json!({"layer": n, "source": radar_pc::CITACAO, "license": radar_pc::LICENCA}));
        } else if nisar::eh_do_nisar(&n) {
            lista.push(json!({"layer": n, "source": nisar::CITACAO, "license": nisar::LICENCA}));
        } else if radar::eh_do_radar(&n) {
            lista.push(json!({"layer": n, "source": radar::CITACAO, "license": radar::LICENCA}));
        } else if let Some(l) = luzes::da_camada(&n) {
            lista.push(json!({"layer": n, "source": luzes::citacao(l), "license": luzes::LICENCA}));
        } else if ndvi_cdse::da_camada(&n).is_some() {
            lista.push(json!({"layer": n, "source": ndvi_cdse::CITACAO, "license": ndvi_cdse::LICENCA}));
        } else if metano::da_camada(&n) {
            lista.push(json!({"layer": n, "source": metano::citacao(&n), "license": metano::LICENCA}));
        } else if sol::eh_do_sol(&n) {
            lista.push(json!({"layer": n, "source": sol::CITACAO, "license": sol::LICENCA}));
        } else if emit::eh_do_emit(&n) {
            lista.push(json!({"layer": n, "source": emit::CITACAO, "license": nisar::LICENCA}));
        } else if vento::eh_do_vento(&n) {
            lista.push(json!({"layer": n, "source": vento::CITACAO, "license": sol::LICENCA}));
        } else if let Some((v, _, _)) = power::ler_camada(&n) {
            lista.push(json!({"layer": n, "source": v.citacao(), "license": sol::LICENCA}));
        } else if tempo::eh_da_previsao(&n) {
            lista.push(json!({"layer": n, "source": tempo::citacao_da_camada(&n), "license": tempo::LICENCA}));
        } else if era5::eh_do_era5(&n) {
            lista.push(json!({"layer": n, "source": era5::CITACAO, "license": era5::LICENCA}));
        } else if tempo::eh_do_chirps(&n) {
            lista.push(json!({"layer": n, "source": tempo::CITACAO_CHIRPS, "license": tempo::LICENCA_CHIRPS}));
        } else if crate::pardig::eh_do_pardig(&n) {
            lista.push(json!({"layer": n, "source": crate::pardig::CITACAO, "license": crate::pardig::LICENCA}));
        } else if risco_fogo::mes_da_camada(&n).is_some() {
            lista.push(json!({"layer": n, "source": risco_fogo::CITACAO, "license": risco_fogo::LICENCA}));
        } else if n == irrigacao::CAMADA {
            lista.push(json!({"layer": n, "source": irrigacao::CITACAO, "license": irrigacao::LICENCA}));
        } else if gedi::eh_do_gedi(&n) {
            lista.push(json!({"layer": n, "source": gedi::CITACAO, "license": gedi::LICENCA}));
        } else if dist::eh_do_dist(&n) {
            lista.push(json!({"layer": n, "source": dist::CITACAO, "license": radar::LICENCA}));
        } else if let Some(ano) = clms::ano_da_camada(&n) {
            lista.push(json!({"layer": n, "source": clms::citacao(ano), "license": clms::LICENCA}));
        } else if let Some(ano) = worldcover::ano_da_camada(&n) {
            lista.push(json!({"layer": n, "source": worldcover::citacao(ano), "license": worldcover::LICENCA}));
        } else if n == chm::CAMADA {
            lista.push(json!({"layer": n, "source": chm::CITACAO, "license": chm::LICENCA}));
        } else if n == chm_amazonia::CAMADA {
            lista.push(json!({"layer": n, "source": chm_amazonia::CITACAO, "license": chm_amazonia::LICENCA}));
        } else if gba::eh_do_gba(&n) {
            lista.push(json!({"layer": n, "source": gba::CITACAO, "license": gba::LICENCA}));
        } else if enderecos::eh_daqui(&n) {
            lista.push(json!({"layer": n, "source": enderecos::CITACAO, "license": enderecos::LICENCA}));
        } else if rodovias::eh_do_osm(&n) {
            lista.push(json!({"layer": n, "source": rodovias::CITACAO_OSM, "license": rodovias::LICENCA_OSM}));
        } else if gdo::eh_do_gdo(&n) {
            lista.push(json!({"layer": n, "source": gdo::CITACAO, "license": gdo::LICENCA}));
        } else if n == forty::CAMADA {
            lista.push(json!({"layer": n, "source": forty::CITACAO, "license": forty::LICENCA}));
        } else if let Some((v, ano)) = ctrees::da_camada(&n) {
            lista.push(json!({"layer": n, "source": ctrees::citacao(v, ano), "license": ctrees::LICENCA}));
        } else if let Some((p, prof, per)) = olm::da_camada(&n) {
            lista.push(json!({"layer": n, "source": olm::citacao(p, prof, per), "license": olm::LICENCA}));
        } else if n == anadem::CAMADA {
            lista.push(json!({"layer": n, "source": anadem::CITACAO, "license": anadem::LICENCA}));
        } else if let Some((v, e)) = gedtm::da_camada(&n) {
            lista.push(json!({"layer": n, "source": gedtm::citacao(v, e), "license": gedtm::LICENCA}));
        } else if let Some(sat) = bdc::satelite_da_camada(&n) {
            lista.push(json!({"layer": n, "source": sat.citacao(), "license": sat.licenca()}));
        } else if let Some(v) = jaxa::variavel_da_camada(&n) {
            lista.push(json!({"layer": n, "source": v.citacao(), "license": jaxa::LICENCA}));
        } else if gem15::eh_do_gem15(&n) {
            lista.push(json!({"layer": n, "source": gem15::CITACAO, "license": gem15::LICENCA}));
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
        avisar("Nenhuma camada do BrasilData no mapa");
        return Ok(json!({"layers": lista}));
    }
    // Uma tabela no projeto (sem geometria), salva com ele: uma linha por citação.
    let linhas: Vec<Value> = lista
        .iter()
        .map(|f| {
            // A ficha da camada (do fichas.csv): o que é, o período, a escala, as unidades, a
            // legenda, os cuidados, o app web e a página com o mapa exemplo.
            let fi = crate::fichas::da_camada(f["layer"].as_str().unwrap_or(""), f["source"].as_str().unwrap_or(""));
            let de = |c: fn(&crate::Ficha) -> &'static str| fi.map_or("", c);
            json!({"type": "Feature", "geometry": null, "properties": {
                "camada": f["layer"].as_str().unwrap_or("").trim_start_matches(PREFIXO),
                "o_que_e": de(|f| f.o_que_e),
                "periodo": de(|f| f.periodo),
                "resolucao": de(|f| f.resolucao),
                "unidades": de(|f| f.unidades),
                "legenda": de(|f| f.legenda),
                "fonte": f["source"],
                "licenca": f["license"],
                "cuidados": de(|f| f.cuidados),
                "app_web": de(|f| f.web),
                "ficha": de(|f| f.pagina),
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
    avisar(&match (camadas.len(), lista.len()) {
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

/// A citação da elevação: a do Copernicus DEM, que a licença dele pede, e de onde vêm os tiles.
const CITACAO_ELEVACAO: &str = "Copernicus DEM GLO-30, produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved. Lido do Microsoft Planetary Computer";

/// O erro de [`janelas_da_elevacao`] num RustGIS que não lê só a janela de um arquivo remoto.
const ELEVACAO_SEM_JANELA: &str = "este RustGIS não lê só a janela da área de um arquivo remoto";

/// O Copernicus GLO-30 na área, na grade do projeto ([`copdem`]): a janela da área em cada tile de 1°
/// que ela toca, lida do Planetary Computer; as janelas se juntam na rede de células do arquivo, e o
/// mosaico vai para a grade e é recortado pela área. Onde o RustGIS não lê janelas (o app web), os
/// tiles inteiros ([`elevacao_inteira`]).
fn elevacao(req: &Request) -> Result<Value, String> {
    let (c, g) = area_e_grade(req)?;
    // Antes de ler os tiles: sem a Imagens, o recorte de uma grade maior pararia no fim.
    let celulas = grade::celulas_no_sistema(c, g.epsg, g.celula);
    if celulas > grade::MAX_VALORES_OPERACAO && sem_imagens() {
        return Err(format!(
            "com células de {} a grade desta área tem cerca de {} milhões de células, e com a extensão Imagens do RustGIS desligada o recorte vai até 32 milhões: ligue a Imagens (Plugins › Extensões), ou use uma célula maior (Grade do projeto) ou uma área menor",
            grade::texto_da_celula(g.celula),
            (celulas / 1e6).round()
        ));
    }
    let (arquivo, lat) = (copdem::arquivo(), (c[1] + c[3]) / 2.0);
    let k = arquivo.nivel(g.celula, lat);
    let m = copdem::margem(k, g.celula);
    let ts = tiles_no_contorno(copdem::tiles(c, m), m)?;
    if ts.is_empty() {
        return Err("o Copernicus não tem dados na área".into());
    }
    let janelas = if le_janela_remota()? { janelas_da_elevacao(&ts, k) } else { Err(ELEVACAO_SEM_JANELA.into()) };
    let lidas = match janelas {
        Ok(Ok(l)) => l,
        Ok(Err(pendente)) => return Ok(pendente),
        Err(e) if e == ELEVACAO_SEM_JANELA => return elevacao_inteira(&copdem::inteiros(&ts, c), &g),
        Err(e) => return Err(e),
    };
    if lidas.is_empty() {
        return Err("o Copernicus não tem dados na área".into());
    }
    let lido = arquivo.metros_do_nivel(k, lat);
    let resampling = cog_global::reamostragem(g.celula, lido);
    remover(&[ELEVACAO])?;
    let mut temporarias = Vec::new();
    let feito = juntar_elevacao(&lidas, c, &g, (k, m, resampling), &mut temporarias);
    tirar_as_de_passagem(&lidas.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>());
    let t: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&t)?;
    let r = feito?;
    let texto = format!("{}: {} tile(s), {}", ELEVACAO.trim_start_matches(PREFIXO), ts.len(), texto_da_leitura(k, lido, resampling, &g));
    avisar(&texto);
    Ok(json!({
        "layer": ELEVACAO, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "tiles": ts.len(), "level": k,
        "width": r["width"], "height": r["height"], "corner": r["corner"], "report": texto,
    }))
}

/// O token anônimo que lê os tiles da Elevação. `Err(pendente)` dentro do `Ok` enquanto ele não vem.
fn token_do_copdem() -> Result<Result<String, Value>, String> {
    let r = match buscar_varios(&[copdem::URL_DO_TOKEN.to_string()])? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let token = radar_pc::token(&r[0]).ok_or("o serviço de tokens do Planetary Computer não deu um token: tente de novo daqui a pouco")?;
    Ok(Ok(token))
}

/// A Elevação dos tiles `ts` baixados inteiros (cerca de 45 MB cada), para o app web: juntos,
/// recortados pela área e postos na grade `g`.
fn elevacao_inteira(ts: &[copdem::Tile], g: &Grade) -> Result<Value, String> {
    if ts.len() > copdem::MAX_INTEIROS {
        return Err(format!(
            "{ELEVACAO_SEM_JANELA} (o app web ainda não lê), e a área pede {} tiles inteiros do Copernicus (cerca de 45 MB cada; o limite é {}): escolha uma área menor, ou use o RustGIS do desktop",
            ts.len(),
            copdem::MAX_INTEIROS
        ));
    }
    remover(&[ELEVACAO])?;
    // Os tiles já baixados: rasters no mapa com o nome do tile.
    let rasters: Vec<String> = host::layers()?
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["raster"].as_bool() == Some(true))
        .filter_map(|l| l["name"].as_str().map(str::to_string))
        .collect();
    let token = match token_do_copdem()? {
        Ok(t) => t,
        Err(p) => return Ok(p),
    };
    let mut partes: Vec<String> = Vec::new();
    let mut baixando = 0;
    let mut falhas: Vec<(String, String)> = Vec::new();
    for t in ts {
        let nome = format!("{PREFIXO}tile {}", t.rotulo());
        if rasters.contains(&nome) {
            partes.push(nome);
            continue;
        }
        let url = t.url();
        match host::run("layer.addUrl", json!({"url": radar_pc::assinar(&url, &token), "name": nome, "whole": true, "zoom": false})) {
            Ok(r) if pendente(&r) => baixando += 1,
            Ok(_) => partes.push(nome),
            // Mar: o Copernicus não tem tile ali. Um tile que existe e não veio não é mar.
            Err(e) if tile_inexistente(&e) => host::log(&format!("sem tile em {url}")),
            Err(e) => falhas.push((t.rotulo(), e.replace(&format!("?{token}"), ""))),
        }
    }
    if baixando > 0 {
        avisar(&format!(
            "Elevação: baixando {baixando} tile(s) do Copernicus (cerca de 45 MB cada). Se a camada não vier quando eles aparecerem no mapa, clique em Elevação de novo"
        ));
        return Ok(json!({"pending": baixando, "tiles": ts.len()}));
    }
    if !falhas.is_empty() {
        return Err(tiles_que_nao_vieram(&falhas, ts.len()));
    }
    if partes.is_empty() {
        return Err("o Copernicus não tem dados na área".into());
    }
    let mut temporarias = partes.clone();
    let mosaico = if let [um] = partes.as_slice() {
        um.clone()
    } else {
        let m = format!("{PREFIXO}mosaico");
        mosaicar(&partes, &m)?;
        temporarias.push(m.clone());
        m
    };
    let recorte = format!("{PREFIXO}recorte");
    recortar(&mosaico, AREA, &recorte)?;
    temporarias.push(recorte.clone());
    let resampling = cog_global::reamostragem(g.celula, copdem::arquivo().metros_do_nivel(0, 0.0));
    let r = host::run(
        "raster.project",
        json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": ELEVACAO}),
    )?;
    remover(&temporarias.iter().map(String::as_str).collect::<Vec<_>>())?;
    let texto = format!(
        "{}: {} tile(s) inteiros, na grade do projeto, {} ({resampling})",
        ELEVACAO.trim_start_matches(PREFIXO),
        ts.len(),
        g.texto()
    );
    avisar(&texto);
    Ok(json!({
        "layer": ELEVACAO, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "tiles": ts.len(), "level": 0,
        "width": r["width"], "height": r["height"], "corner": r["corner"], "report": texto,
    }))
}

/// A camada temporária dos retângulos dos tiles da Elevação. O nome não começa como o das janelas.
const CONTORNOS_ELEVACAO: &str = "BrasilData: contornos da elevação";

/// Dos tiles `ts`, os do retângulo da área, só os que o contorno toca, com a margem `m`. Quem compara é
/// o RustGIS: ler o contorno de um estado aqui gastaria a cota de instruções. Se ele não comparar, vão
/// todos, que só custam leituras à toa. Mais que [`copdem::MAX_TILES`] é área grande demais.
fn tiles_no_contorno(ts: Vec<(copdem::Tile, [f64; 4])>, m: f64) -> Result<Vec<(copdem::Tile, [f64; 4])>, String> {
    let conferir = |n: usize| {
        if n <= copdem::MAX_TILES {
            return Ok(());
        }
        Err(format!(
            "a área toca {n} tiles de 1° do Copernicus; o limite de um clique é {}: escolha uma área menor, ou o Terreno da América do Sul (ANADEM), que é um arquivo só",
            copdem::MAX_TILES
        ))
    };
    if ts.len() < 2 {
        return Ok(ts);
    }
    let selecao = host::add_layer(CONTORNOS_ELEVACAO, copdem::contornos(&ts, m)).and_then(|_| {
        let r = host::run("gp.run", json!({"tool": "selectByLocation", "params": {"input": CONTORNOS_ELEVACAO, "select": AREA}}));
        // Os de uma área grande demais (o Brasil) não se leem: gastariam a cota de instruções.
        let quantos = r.as_ref().ok().and_then(|r| r["selected"].as_u64()).map_or(0, |n| n as usize);
        let cabem = conferir(quantos);
        let r = r.and_then(|_| if cabem.is_ok() { host::features(CONTORNOS_ELEVACAO, true) } else { Ok(Value::Null) });
        tirar_as_de_passagem(&[CONTORNOS_ELEVACAO]);
        r.map(|fc| (fc, cabem))
    });
    match selecao {
        Ok((fc, cabem)) => {
            cabem?;
            let mut indices: Vec<usize> =
                fc["features"].as_array().into_iter().flatten().filter_map(|f| Some(f["properties"]["i"].as_u64()? as usize)).collect();
            // Na ordem de `ts`, a das linhas e colunas dos tiles.
            indices.sort_unstable();
            host::log(&format!("elevação: o contorno toca {} dos {} tiles do retângulo", indices.len(), ts.len()));
            Ok(indices.into_iter().filter_map(|i| ts.get(i).copied()).collect())
        }
        Err(e) => {
            host::log(&format!("elevação: todos os tiles do retângulo, sem a escolha pelo contorno ({e})"));
            conferir(ts.len()).map(|()| ts)
        }
    }
}

/// Uma janela de tile que está no mapa: a camada e a parte do tile que ela tem.
type JanelaDeTile = (String, [f64; 4]);

/// As janelas dos tiles `ts` no nível `k`, as que faltam lidas agora. `Err(pendente)` dentro do `Ok`
/// quando o app ficou esperando as leituras.
fn janelas_da_elevacao(ts: &[(copdem::Tile, [f64; 4])], k: i32) -> Result<Result<Vec<JanelaDeTile>, Value>, String> {
    // As janelas de um clique anterior servem se cobrem a parte (no desktop, o comando roda de novo
    // quando cada uma chega).
    let ha = nomes()?;
    let onde = extensoes_das(copdem::JANELAS)?;
    let mosaicos = mosaicos_no_mapa()?;
    let mut lidas: Vec<JanelaDeTile> = Vec::new();
    let mut faltam = Vec::new();
    for (t, parte) in ts {
        let nome = t.nome_da_janela(k);
        if ha.contains(&nome)
            && let Some(e) = onde.get(&nome)
            && retangulo_cabe(*parte, *e)
        {
            lidas.push((nome, *parte));
        } else {
            faltam.push((t, *parte, nome));
        }
    }
    if faltam.is_empty() {
        return Ok(Ok(lidas));
    }
    let token = match token_do_copdem()? {
        Ok(t) => t,
        Err(p) => return Ok(Err(p)),
    };
    let mut lendo = 0;
    let mut falhas: Vec<(String, String)> = Vec::new();
    for (t, parte, nome) in faltam {
        if lendo >= crate::FILA_DO_APP {
            break;
        }
        if ha.contains(&nome) || mosaicos.contains(&nome) {
            tirar_janela(&nome)?;
        }
        let url = t.url();
        let mut p = json!({"url": radar_pc::assinar(&url, &token), "name": nome, "bbox": parte, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(cog_global::celula_pedida(copdem::arquivo().graus_do_nivel(k)));
        }
        let pedir = || host::run("layer.addUrl", p.clone());
        // De novo, até duas vezes, se a conexão caiu (onde o RustGIS espera a leitura).
        let de_novo = |e: String| if tile_inexistente(&e) || crate::fila_cheia(&e) { Err(e) } else { pedir() };
        let resposta = pedir();
        // O app web não lê janelas: pedir de novo daria o mesmo.
        if resposta.as_ref().is_err_and(|e| copdem::sem_janela(e)) {
            return Err(ELEVACAO_SEM_JANELA.into());
        }
        match resposta.or_else(de_novo).or_else(de_novo) {
            Ok(r) if pendente(&r) => lendo += 1,
            Ok(r) if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() => {
                tirar_janela(&nome)?;
                return Err(ELEVACAO_SEM_JANELA.into());
            }
            Ok(_) => lidas.push((nome, parte)),
            Err(e) if crate::fila_cheia(&e) && lendo > 0 => break,
            // Mar: o Copernicus não tem tile ali. Um tile que existe e não veio não é mar.
            Err(e) if tile_inexistente(&e) => host::log(&format!("sem tile em {url}")),
            Err(e) => falhas.push((t.rotulo(), e.replace(&format!("?{token}"), ""))),
        }
    }
    if lendo > 0 {
        avisar(&format!(
            "Elevação: lendo a área em {lendo} tile(s) do Copernicus; a camada vem quando chegarem (se não vier, clique de novo)"
        ));
        return Ok(Err(json!({"pending": lendo, "tiles": ts.len()})));
    }
    if !falhas.is_empty() {
        return Err(tiles_que_nao_vieram(&falhas, ts.len()));
    }
    Ok(Ok(lidas))
}

/// As janelas `lidas` juntas, na grade `g` e recortadas pela área `c`, na camada [`ELEVACAO`]: em uma
/// faixa ou em várias ([`copdem::faixas`]). O nível lido, a margem e a reamostragem vêm em `leitura`;
/// as camadas de passagem vão para `temporarias`, que quem chama tira.
fn juntar_elevacao(
    lidas: &[JanelaDeTile],
    c: [f64; 4],
    g: &Grade,
    leitura: (i32, f64, &str),
    temporarias: &mut Vec<String>,
) -> Result<Value, String> {
    let (k, m, resampling) = leitura;
    let faixas = copdem::faixas(c, k, m);
    let mut na_grade = Vec::new();
    let mut passo = |o_que: &str, i: usize| {
        let n = format!("{PREFIXO}Elevação {o_que} {i}");
        temporarias.push(n.clone());
        n
    };
    for (i, f) in faixas.iter().enumerate() {
        let j = copdem::janela_da_faixa(c, *f, m);
        let dela: Vec<&String> = lidas.iter().filter(|(_, parte)| copdem::na_faixa(*parte, j, k)).map(|(n, _)| n).collect();
        let junta = match dela.as_slice() {
            [] => continue,
            [uma] => (*uma).clone(),
            [primeira, ..] => {
                let (base, junta) = (passo("rede", i), passo("junta", i));
                host::run("raster.project", copdem::estender(primeira, j, k, &base))?;
                let entradas: Vec<String> = std::iter::once(base.clone()).chain(dela.iter().skip(1).map(|n| (*n).clone())).collect();
                juntar_na_grade(&entradas, &junta, "first")?;
                tirar_as_de_passagem(&[&base]);
                junta
            }
        };
        let posta = passo("na grade", i);
        host::run(
            "raster.project",
            json!({"layer": junta, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": posta}),
        )?;
        if dela.len() > 1 {
            tirar_as_de_passagem(&[&junta]);
        }
        if faixas.len() == 1 {
            na_grade.push(posta);
            continue;
        }
        // A sobra da faixa, que existe para a reamostragem da divisa, sai.
        let (corte, cortada) = (passo("divisa", i), passo("cortada", i));
        host::add_layer(&corte, copdem::corte(c, *f))?;
        recortar(&posta, &corte, &cortada)?;
        tirar_as_de_passagem(&[&posta, &corte]);
        na_grade.push(cortada);
    }
    let inteira = match na_grade.as_slice() {
        [] => return Err("o Copernicus não tem dados na área".into()),
        [uma] => uma.clone(),
        varias => {
            let n = passo("inteira", 0);
            mosaicar(varias, &n)?;
            n
        }
    };
    let recorte = passo("recorte", 0);
    recortar(&inteira, AREA, &recorte)?;
    // O recorte da Imagens deixa só o retângulo do contorno: de volta à grade inteira, nas mesmas
    // células, para que todo raster na grade tenha as mesmas linhas e colunas.
    host::run(
        "raster.project",
        json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "nearest", "name": ELEVACAO}),
    )
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
    let pedidos: Vec<(String, String)> =
        recortes.iter().map(|r| (r.url(p).to_string(), format!("{}radar {} {}", PREFIXO, p.sigla(), r.rotulo()))).collect();
    let sar = Sar { sigla: p.sigla(), de_onde: "do ASF (cerca de 7 MB cada)", celula: 30.0, escala: p.escala(), rampa: None, sigma: None };
    sar_na_grade(&pedidos, &sar, g, c, nome)
}

/// O que muda de um radar para outro em [`sar_na_grade`].
struct Sar {
    /// A polarização, para os avisos.
    sigla: &'static str,
    /// De onde vêm os arquivos, para o aviso do download.
    de_onde: &'static str,
    /// A célula dos arquivos, em metros.
    celula: f64,
    /// A escala de cinza, em dB.
    escala: (f64, f64),
    /// Uma rampa de cores no lugar do cinza (a do NISAR é a "Magma", como nas imagens da NASA).
    rampa: Option<&'static str>,
    /// O sigma do filtro de Lee, em dB (o filtro no dB, que guarda as bordas); sem ele, a média da
    /// janela na potência linear.
    sigma: Option<f64>,
}

/// Os arquivos de radar `pedidos` (endereço, nome da camada bruta), em potência linear, na grade
/// `g`, com o filtro de Lee refinado, recortados pela área e em dB, como a camada `nome`; ou o pedido
/// pendente (a janela e o app web, que não esperam o download).
fn sar_na_grade(pedidos: &[(String, String)], sar: &Sar, g: &Grade, c: [f64; 4], nome: &str) -> Result<Option<Value>, String> {
    // A lista de camadas do mapa custa caro ao plugin (cerca de 16 milhões de instruções, com
    // rasters no mapa): é lida uma vez, e não a cada recorte, senão uma mesorregião passa da cota.
    let ha = nomes()?;
    // As camadas intermediárias de uma vez anterior que tenha parado no meio.
    let (mosaico, recorte) = ("BrasilData: mosaico do radar".to_string(), "BrasilData: recorte do radar".to_string());
    let brutos: Vec<String> = pedidos.iter().map(|(_, b)| b.clone()).collect();
    let logs: Vec<String> = brutos.iter().map(|b| format!("{b} log")).collect();
    let em_db: Vec<String> = brutos.iter().map(|b| format!("{b} dB")).collect();
    let filtrados: Vec<String> = brutos.iter().map(|b| format!("{b} filtrado")).collect();
    let na_grade: Vec<String> = brutos.iter().map(|b| format!("{b} na grade")).collect();
    let na_area: Vec<String> = brutos.iter().map(|b| format!("{b} na área")).collect();
    let (linear, log) = (format!("{nome} linear"), format!("{nome} log"));
    let velhas: Vec<&str> = filtrados
        .iter()
        .chain(&logs)
        .chain(&em_db)
        .chain(&na_grade)
        .chain(&na_area)
        .map(String::as_str)
        .chain([mosaico.as_str(), recorte.as_str(), linear.as_str(), log.as_str()])
        .filter(|n| ha.iter().any(|h| h == n))
        .collect();
    if !velhas.is_empty() {
        host::run("layer.remove", json!({"layer": velhas}))?;
    }
    // Os arquivos inteiros.
    let pares: Vec<(&str, &str)> = pedidos.iter().map(|(u, b)| (u.as_str(), b.as_str())).collect();
    let baixando = baixar_inteiros(&pares, &ha)?;
    if baixando > 0 {
        let n = if baixando == 1 { "1 recorte".to_string() } else { format!("{baixando} recortes") };
        avisar(&format!(
            "Radar {}: baixando {n} {}. Quando aparecerem no mapa como \"BrasilData: radar\", clique de novo",
            sar.sigla, sar.de_onde
        ));
        return Ok(Some(json!({"pending": baixando, "bursts": pedidos.len()})));
    }
    let mut temporarias: Vec<String> = Vec::new();
    let resultado = (|| -> Result<(), String> {
        for ((((bruto, d), f), q), a) in brutos.iter().zip(&em_db).zip(&filtrados).zip(&na_grade).zip(&na_area) {
            // A célula do próprio arquivo, que pode não ser a do produto: o NISAR vem em 20 m no
            // modo de 20 MHz e em 10 m no de 40 MHz.
            let real = celula_do_raster(bruto)?.unwrap_or(sar.celula);
            let lado = filtro_do_radar(sar.celula, real);
            // Em dB (10 · log10, como o radar costuma ser visto; o `bandMath` não tem logaritmo, o
            // GeoLibre tem), antes ou depois do filtro de Lee, que tira o granulado (speckle) na
            // geometria do próprio recorte, antes de qualquer reamostragem. O `lee_filter` do
            // GeoLibre é o filtro sigma de Lee com o sigma absoluto: a média dos vizinhos que
            // diferem da célula até sigma.
            let log = format!("{bruto} log");
            match sar.sigma {
                // No dB, com sigma em dB: só entram os vizinhos parecidos, e as bordas (talhões,
                // estradas, casas) não se espalham.
                Some(sigma) => {
                    ferramenta("geolibre:log10", json!({"input": bruto}), &log)?;
                    host::run("raster.function", json!({"layer": log, "function": "bandMath", "expression": "b1 * 10", "name": d}))?;
                    host::run("layer.remove", json!({"layer": [bruto, &log]}))?;
                    ferramenta(
                        "geolibre:lee_filter",
                        json!({"input": d, "filter_size_x": lado, "filter_size_y": lado, "sigma": sigma}),
                        f,
                    )?;
                    host::run("layer.remove", json!({"layer": [d]}))?;
                }
                // Na potência linear (valores < 1), com o sigma padrão (10): todo vizinho entra, e
                // é a média simples da janela, que alisa mais e borra as bordas.
                None => {
                    ferramenta("geolibre:lee_filter", json!({"input": bruto, "filter_size_x": lado, "filter_size_y": lado}), d)?;
                    ferramenta("geolibre:log10", json!({"input": d}), &log)?;
                    host::run("raster.function", json!({"layer": log, "function": "bandMath", "expression": "b1 * 10", "name": f}))?;
                    host::run("layer.remove", json!({"layer": [bruto, d, &log]}))?;
                }
            }
            // Para células bem maiores que as do arquivo, a média, que também tira granulado; a
            // bilinear só pegaria uma amostra de cada 3 × 3 (de 10 m para 30 m).
            let resampling = if g.celula > 1.5 * real { "average" } else { "bilinear" };
            // Cada recorte vai para a rede da grade (o sistema e a célula dela, origem [0, 0]) no
            // próprio retângulo: os recortes, que podem estar em fusos UTM diferentes, passam a ter
            // o mesmo sistema e as mesmas células, e o mosaico os junta.
            host::run("raster.project", json!({"layer": f, "crs": g.epsg, "cellSize": g.celula, "resampling": resampling, "name": q}))?;
            // Só o retângulo da área: os recortes passam bastante dela, e o mosaico de uma
            // mesorregião inteira passaria do limite de 120 milhões de valores de um raster.
            recortar_retangulo(q, c, g.celula, a)?;
            temporarias.push(a.clone());
            // O filtrado e o projetado saem já, para poupar memória.
            host::run("layer.remove", json!({"layer": [f, q]}))?;
        }
        let junto = if na_area.len() == 1 {
            na_area[0].clone()
        } else {
            mosaicar(&na_area, &mosaico)?;
            temporarias.push(mosaico.clone());
            mosaico.clone()
        };
        recortar(&junto, AREA, &recorte)?;
        temporarias.push(recorte.clone());
        // Já na rede da grade: a extensão dela, sem reamostrar.
        host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "nearest", "name": nome}),
        )?;
        if let Some(rampa) = sar.rampa {
            host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": rampa}))?;
        }
        let (min, max) = sar.escala;
        let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": min, "max": max}));
        Ok(())
    })();
    let fora: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    if !fora.is_empty() {
        host::run("layer.remove", json!({"layer": fora})).map(|_| ()).or_else(|e| if resultado.is_err() { Ok(()) } else { Err(e) })?;
    }
    resultado.map(|()| None)
}

/// O lado (ímpar, de 3 a 15) do filtro de Lee para células de `real` metros: 5 na célula `produto`
/// do radar, e a mesma largura no chão numa célula menor.
fn filtro_do_radar(produto: f64, real: f64) -> i64 {
    let k = ((5.0 * produto / real.max(1.0)).round() as i64).clamp(3, 15);
    if k % 2 == 0 { k + 1 } else { k }
}

/// A célula de um raster do mapa, em metros (num sistema em graus, pelo grau no equador).
fn celula_do_raster(camada: &str) -> Result<Option<f64>, String> {
    let i = host::run("raster.info", json!({"layer": camada}))?;
    let Some(x) = i["cellSize"][0].as_f64().map(f64::abs).filter(|x| *x > 0.0) else { return Ok(None) };
    let graus = i["crs"].as_str().is_some_and(|c| c == "EPSG:4326" || c == "EPSG:4674");
    Ok(Some(if graus { x * 111_320.0 } else { x }))
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
        let a = host::run("layer.addUrl", json!({"url": url, "name": nome, "whole": true, "zoom": false}))
            .map_err(token_recem_lido)
            .map_err(|e| {
                if e.starts_with("o RustGIS acabou de ler") {
                    e
                } else if e.contains("token") || e.contains("401") || e.contains("Earthdata") {
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
        avisar(&format!(
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
            recortar_retangulo(q, c, g.celula, a)?;
            temporarias.push(a.clone());
            host::run("layer.remove", json!({"layer": [bruto, q]}))?;
        }
        // Onde dois tiles se sobrepõem, o mosaico fica com o primeiro: o mais recente.
        let junto = if na_area.len() == 1 {
            na_area[0].clone()
        } else {
            mosaicar_com(&na_area, &mosaico, "max")?;
            temporarias.push(mosaico.clone());
            mosaico.clone()
        };
        recortar(&junto, AREA, &recorte)?;
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
                    avisar(&format!(
                        "Perturbação da vegetação: área grande demais para as manchas com borda (até cerca de {:.0} milhões de células da grade); fica o raster",
                        dist::MAX_CELULAS_MANCHAS / 1e6
                    ));
                } else if let Err(e) = manchas_da_situacao(&junto, nome, ate, paleta, g.celula) {
                    let manchas = dist::nome_das_manchas(ate);
                    let _ = remover(&[&manchas, &format!("{manchas} (sem recorte)"), &format!("{manchas} (em graus)")]);
                    host::log(&format!("manchas: {e}"));
                    avisar(&format!("Perturbação da vegetação: as manchas com borda não saíram ({e}); fica o raster"));
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
    let codigos = format!("{manchas} (códigos)");
    remover(&[&manchas, &bruto, &em_graus, &codigos])?;
    // Só os códigos 1 a 8: o 0 (sem perturbação) e o sem dado viram sem dado declarado. Sem a
    // extensão Imagens, o mosaico (cube.reduce) deixa NaN onde não há tile, e a ferramenta do
    // GeoLibre, que não toma NaN por sem dado, não cria camada nenhuma ("GeoJSON: expected value").
    let faixas: Vec<[u8; 3]> = (1..=8u8).map(|v| [v, v + 1, v]).collect();
    host::run("raster.function", json!({"layer": retangulo, "function": "remap", "ranges": faixas, "name": codigos}))?;
    // As regiões dos códigos, com o código em `VALUE`. A ferramenta do GeoLibre não grava em todo
    // sistema (o Albers do IBGE, EPSG:10857, que a grade usa quando a área passa de um fuso, ela
    // recusa): então as manchas saem de uma cópia em graus, com as mesmas células.
    let vetorizado = (|| -> Result<(), String> {
        if let Err(e) = ferramenta("geolibre:raster_to_vector_polygons", json!({"input": codigos}), &bruto) {
            host::log(&format!("manchas: no sistema da grade não deu ({e}); vão em graus"));
            remover(&[&bruto])?;
            host::run(
                "raster.project",
                json!({"layer": codigos, "crs": 4326, "cellSize": celula / 111_320.0, "resampling": "nearest", "name": em_graus}),
            )?;
            let feito = ferramenta("geolibre:raster_to_vector_polygons", json!({"input": em_graus}), &bruto);
            remover(&[&em_graus])?;
            feito?;
        }
        Ok(())
    })();
    remover(&[&codigos])?;
    vetorizado?;
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
    avisar(&format!("{}: {} tiles {datas}, na grade do projeto, {}{codigos}", nome.trim_start_matches(PREFIXO), produtos.len(), g.texto()));
    Ok(json!({
        "layer": nome, "date": mais_novo.texto(), "products": produtos.iter().map(|p| p.nome.clone()).collect::<Vec<_>>(),
        "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// Roda uma ferramenta do RustGIS (`gp.run`) e dá à camada que ela cria o nome `nome`.
fn ferramenta(tool: &str, params: Value, nome: &str) -> Result<(), String> {
    let r = host::run("gp.run", json!({"tool": tool, "params": params})).map_err(extensoes::erro_da_ferramenta)?;
    let id = r["layers"].get(0).cloned().ok_or(format!("{tool} não criou camada"))?;
    host::run("layer.rename", json!({"layer": id, "name": nome}))?;
    Ok(())
}

/// Se a extensão Imagens do RustGIS 0.7.0 está desligada (módulo [`extensoes`]). Um RustGIS sem
/// extensões (0.6.0 e antes) não tem o `extension.list`: lá os comandos de raster estão sempre.
fn sem_imagens() -> bool {
    host::run("extension.list", json!({})).is_ok_and(|l| extensoes::imagens_desligada(&l))
}

/// Roda um comando da extensão Imagens; se ela estiver desligada (perguntada antes, ou recusado
/// agora), roda `sem` no lugar, que faz o mesmo sem ela.
fn com_imagens(comando: &str, params: Value, sem: impl FnOnce() -> Result<(), String>) -> Result<(), String> {
    let sem = || sem().map_err(extensoes::ou_ligue_a_imagens);
    if sem_imagens() {
        return sem();
    }
    match host::run(comando, params) {
        Err(e) if extensoes::recusa_de_extensao(&e) => sem(),
        r => r.map(|_| ()),
    }
}

/// Recorta o raster `camada` pelo polígono da camada `poligono`, numa camada nova `nome`: pelo
/// `raster.process clip`, ou, sem a extensão Imagens, pela máscara do contorno (módulo [`recorte`]).
/// O `geolibre:clip_raster_to_polygon`, lento e opcional, fica para quando ela não dá (um raster
/// maior que o limite do `cube.reduce`).
fn recortar(camada: &str, poligono: &str, nome: &str) -> Result<(), String> {
    com_imagens("raster.process", json!({"layer": camada, "operation": "clip", "polygon": poligono, "name": nome}), || {
        let mut temporarias = Vec::new();
        let feito = recortar_pela_mascara(camada, poligono, nome, &mut temporarias);
        if !temporarias.is_empty() {
            host::run("layer.remove", json!({"layer": temporarias}))?;
        }
        let Err(motivo) = feito else { return Ok(()) };
        host::log(&format!("recorte pela máscara de {camada}: {motivo}; vai pelo GeoLibre"));
        ferramenta("geolibre:clip_raster_to_polygon", json!({"input": camada, "polygons": poligono}), nome)
            .map_err(|e| recorte::nao_saiu(&recorte::motivo(&motivo), &e))?;
        let i = host::run("raster.info", json!({"layer": camada})).unwrap_or_default();
        if let Some(a) = recorte::aviso_de_demora(i["width"].as_f64().unwrap_or(0.0), i["height"].as_f64().unwrap_or(0.0)) {
            avisar(&a);
        }
        Ok(())
    })
}

/// O recorte sem a extensão Imagens e sem o GeoLibre (módulo [`recorte`]): os valores de `camada` só
/// nas células com o centro dentro do contorno. As camadas de passagem vão para `temporarias` (os
/// ids), que quem chama tira. Um raster de mais bandas (a imagem sem nuvens) vai banda a banda e se
/// junta de novo pelo `geolibre:composite_bands`, com que ele foi montado.
fn recortar_pela_mascara(camada: &str, poligono: &str, nome: &str, temporarias: &mut Vec<Value>) -> Result<(), String> {
    fn feita(r: &Value, temporarias: &mut Vec<Value>) -> Result<Value, String> {
        let id = camada_feita(r).ok_or("o RustGIS não disse a camada que fez")?;
        temporarias.push(id.clone());
        Ok(id)
    }
    let bandas = host::run("raster.info", json!({"layer": camada}))?["bands"].as_array().map_or(1, Vec::len).max(1);
    let m = format!("{PREFIXO}máscara da área");
    let r = host::run("raster.rasterize", recorte::mascara(poligono, camada, &m))?;
    let mut mascara = (feita(&r, temporarias)?, false);
    let mut partes = Vec::new();
    for b in 0..bandas {
        let passo = |p: &str| format!("{nome} ({p} {})", b + 1);
        let origem = if bandas > 1 {
            let n = passo("banda");
            feita(&host::run("cube.reduce", recorte::so_a_banda(camada, b, bandas, &n))?, temporarias)?;
            n
        } else {
            camada.to_string()
        };
        let cortada = passo("recorte");
        let mut feito = host::run("cube.reduce", recorte::combinar(&origem, &mascara.0, mascara.1, &cortada));
        // Grande demais inteiro: só o retângulo do contorno, nas mesmas células.
        if let (Err(e), false) = (&feito, mascara.1) {
            let pedido = recorte::mascara_menor(poligono, &r, &m).ok_or_else(|| e.clone())?;
            mascara = (feita(&host::run("raster.rasterize", pedido)?, temporarias)?, true);
            feito = host::run("cube.reduce", recorte::combinar(&origem, &mascara.0, true, &cortada));
        }
        feita(&feito?, temporarias)?;
        partes.push(cortada);
    }
    match partes.as_slice() {
        [uma] => host::run("raster.function", recorte::declarar_sem_dado(uma, nome)).map(|_| ()),
        _ => ferramenta("geolibre:composite_bands", json!({"inputs": partes}), nome),
    }
}

/// Recorta o raster `camada` pelo retângulo `c` (oeste, sul, leste, norte, em graus), numa camada
/// nova `nome`. Sem a extensão Imagens, o `raster.project` faz o mesmo no próprio sistema da
/// camada, com células `celula` e a origem [0, 0] da grade, sem reamostrar.
fn recortar_retangulo(camada: &str, c: [f64; 4], celula: f64, nome: &str) -> Result<(), String> {
    com_imagens("raster.process", json!({"layer": camada, "operation": "clip", "extent": c, "name": nome}), || {
        host::run("raster.project", json!({"layer": camada, "cellSize": celula, "extent": c, "name": nome})).map(|_| ())
    })
}

/// Junta os rasters `entradas` (o primeiro vale onde se sobrepõem) numa camada nova `nome`: pelo
/// `raster.mosaic`, ou, sem a extensão Imagens, como [`mosaico_sem_imagens`] diz.
fn mosaicar(entradas: &[String], nome: &str) -> Result<(), String> {
    mosaicar_com(entradas, nome, "first")
}

/// [`mosaicar`], com a estatística `stat` do `cube.reduce` onde as entradas se sobrepõem sem a
/// extensão Imagens (`"max"` no DIST-S1: a data mais recente, a situação mais adiantada).
fn mosaicar_com(entradas: &[String], nome: &str, stat: &str) -> Result<(), String> {
    com_imagens("raster.mosaic", json!({"inputs": entradas, "name": nome}), || mosaico_sem_imagens(entradas, nome, stat))
}

/// O mosaico sem a extensão Imagens. Rasters na mesma grade (as janelas postas na grade do projeto,
/// sem valor fora de si): o `cube.reduce` com `stat`, que toma o que falta (NaN) como falta. O
/// GeoLibre não serve aí: ele não toma NaN por sem-dado, e o `geolibre:mosaic` de entradas de mesma
/// extensão sai igual à primeira (o GEM-15 de Viçosa vinha só com a faixa de uma célula, média de
/// 639 m em vez de 717). Rasters de extensões diferentes (como tiles baixados inteiros):
/// postos antes na janela da área ([`recorte::tiles_na_janela`]), e só se isso não der (uma janela
/// acima do limite do `cube.reduce`) o `geolibre:mosaic`, com o aviso de que ele os desloca.
fn mosaico_sem_imagens(entradas: &[String], nome: &str, stat: &str) -> Result<(), String> {
    let infos: Vec<(String, Value)> =
        entradas.iter().map(|e| (e.clone(), host::run("raster.info", json!({"layer": e})).unwrap_or_default())).collect();
    let grade = |i: &Value| json!([i["width"], i["height"], i["extent"], i["crs"]]);
    let mesma_grade = infos.first().is_some_and(|(_, p)| !p.is_null() && infos.iter().all(|(_, i)| grade(i) == grade(p)));
    if mesma_grade {
        return juntar_na_grade(entradas, nome, stat).map_err(|e| recorte::mosaico_nao_saiu(&e, true));
    }
    let Some(pedidos) = recorte::tiles_na_janela(&infos, AREA, |k| format!("{nome} (janela {k})")) else {
        return ferramenta("geolibre:mosaic", json!({"inputs": entradas}), nome);
    };
    let mut janelas = Vec::new();
    let feito = pedidos
        .into_iter()
        .try_for_each(|p| {
            let n = p["name"].as_str().unwrap_or_default().to_string();
            host::run("raster.project", p)?;
            janelas.push(n);
            Ok(())
        })
        .and_then(|()| juntar_na_grade(&janelas, nome, stat));
    if !janelas.is_empty() {
        host::run("layer.remove", json!({"layer": janelas}))?;
    }
    let Err(motivo) = feito else { return Ok(()) };
    host::log(&format!("mosaico na janela da área: {motivo}; vai pelo GeoLibre"));
    ferramenta("geolibre:mosaic", json!({"inputs": entradas}), nome)
        .map_err(|e| format!("{}. (Pelo GeoLibre: {e})", recorte::mosaico_nao_saiu(&motivo, false)))?;
    avisar(recorte::AVISO_DO_MOSAICO);
    Ok(())
}

/// Junta rasters da mesma grade pelo `cube.reduce` ([`recorte::juntar`]). O RustGIS 0.7.0 não tem o
/// `first`: lá vai o maior valor, que ao menos é o de uma das entradas.
fn juntar_na_grade(entradas: &[String], nome: &str, stat: &str) -> Result<(), String> {
    let r = host::run("cube.reduce", recorte::juntar(entradas, stat, nome));
    match r {
        Err(_) if stat == "first" => host::run("cube.reduce", recorte::juntar(entradas, "max", nome)),
        r => r,
    }
    .map(|_| ())
}

/// A diferença `depois` − `antes`, numa camada nova `nome`: pelo `raster.change difference`, ou, sem a
/// extensão Imagens, pela soma do `cube.reduce` ([`recorte::somar`]). O `geolibre:subtract`, que o
/// usuário instala à parte, fica para um raster maior que o limite do `cube.reduce`.
fn diferenca(antes: &str, depois: &str, nome: &str) -> Result<(), String> {
    com_imagens("raster.change", json!({"before": antes, "after": depois, "method": "difference", "name": nome}), || {
        let menos = format!("{nome} (menos)");
        let soma = format!("{nome} (soma)");
        let feito = (|| -> Result<(), String> {
            host::run("raster.function", json!({"layer": antes, "function": "bandMath", "expression": "b1 * -1", "name": menos}))?;
            host::run("cube.reduce", recorte::somar(depois, &menos, &soma))?;
            host::run("raster.function", recorte::declarar_sem_dado(&soma, nome)).map(|_| ())
        })();
        remover(&[&menos, &soma])?;
        let Err(motivo) = feito else { return Ok(()) };
        host::log(&format!("diferença de {depois} e {antes}: {motivo}; vai pelo GeoLibre"));
        ferramenta("geolibre:subtract", json!({"input1": depois, "input2": antes}), nome)
            .map_err(|e| format!("{e} ({})", recorte::motivo(&motivo)))
    })
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
        diferenca(&feitas[0], &feitas[1], &razao)?;
        let (min, max) = radar::ESCALA_DA_RAZAO;
        let _ = host::run("raster.stretch", json!({"layer": razao, "type": "custom", "min": min, "max": max}));
        camada = razao;
    }
    avisar(&format!(
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

/// A camada temporária dos contornos das cenas do NISAR.
const CONTORNOS_NISAR: &str = "BrasilData: contornos do NISAR";

/// As cenas do NISAR GCOV que cruzam o polígono da área, da passagem escolhida
/// ([`nisar::passagem`]), ou o pedido pendente (app web).
fn cenas_do_nisar(c: [f64; 4], dia: Option<Data>) -> Result<Result<Vec<nisar::Cena>, Value>, String> {
    Ok(passagens_do_nisar(c, dia, 1)?.map(|mut v| v.remove(0)))
}

/// As cenas do NISAR GCOV que cruzam o polígono da área, em até `n` passagens da mesma trilha
/// ([`nisar::passagens`]), a mais recente primeiro; ou o pedido pendente (app web).
fn passagens_do_nisar(c: [f64; 4], dia: Option<Data>, n: usize) -> Result<Result<Vec<Vec<nisar::Cena>>, Value>, String> {
    let resposta = match buscar_varios(&[nisar::url_busca(c, dia)])? {
        Ok(mut r) => r.remove(0),
        Err(p) => return Ok(Err(p)),
    };
    let todas = nisar::cenas(&resposta);
    let quando = dia.map(|d| format!(" em {}", d.texto())).unwrap_or_default();
    if todas.is_empty() {
        return Err(format!(
            "o ASF não tem cenas do NISAR GCOV com HH e HV na área{quando} (a missão distribui dados desde meados de 2026)"
        ));
    }
    remover(&[CONTORNOS_NISAR])?;
    host::add_layer(CONTORNOS_NISAR, nisar::camada_dos_contornos(&todas))?;
    let selecao = host::run("gp.run", json!({"tool": "selectByLocation", "params": {"input": CONTORNOS_NISAR, "select": AREA}}))
        .and_then(|_| host::features(CONTORNOS_NISAR, true));
    remover(&[CONTORNOS_NISAR])?;
    let indices: Vec<usize> =
        selecao?["features"].as_array().into_iter().flatten().filter_map(|f| f["properties"]["i"].as_u64().map(|i| i as usize)).collect();
    let cruzam: Vec<&nisar::Cena> = indices.iter().filter_map(|i| todas.get(*i)).collect();
    let passagens = nisar::passagens(&cruzam, n);
    if passagens.is_empty() {
        return Err(format!("nenhuma cena do NISAR GCOV cruza a área{quando}"));
    }
    let mut todas_as_escolhidas = Vec::new();
    for (data, trilha) in passagens {
        let escolhidas: Vec<nisar::Cena> = cruzam.iter().filter(|c| c.passagem() == (data, trilha)).map(|c| (*c).clone()).collect();
        if escolhidas.len() > nisar::MAX_CENAS {
            return Err(format!(
                "a passagem de {} tem {} cenas na área; o limite é {}: escolha uma área menor",
                data.texto(),
                escolhidas.len(),
                nisar::MAX_CENAS
            ));
        }
        host::log(&format!(
            "nisar: {} cenas cruzam a área, {} da passagem de {} (trilha {trilha}): {}",
            indices.len(),
            escolhidas.len(),
            data.texto(),
            escolhidas.iter().map(|c| c.id.as_str()).collect::<Vec<_>>().join(", ")
        ));
        todas_as_escolhidas.push(escolhidas);
    }
    Ok(Ok(todas_as_escolhidas))
}

/// O radar de banda L do NISAR (GCOV) na área: uma polarização, ou a razão HV/HH (que deixa as
/// duas no mapa também). O Harmony da NASA tira a polarização do HDF5, recorta pelo retângulo da
/// área e devolve um GeoTIFF; daí em diante, o caminho é o do radar do Sentinel-1. Por padrão, a
/// passagem mais recente que cobre a área; por script, a de um dia.
fn radar_nisar(req: &Request, qual: Option<nisar::Pol>) -> Result<Value, String> {
    let PedidoAsf { c, celula, dia } = area_do_asf(req, "o NISAR")?;
    conferir_tamanho_do_nisar(c)?;
    let g = grade_para(c, celula)?;
    let cenas = match cenas_do_nisar(c, dia)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let data = cenas[0].data;
    // Um pouco além do retângulo da área, para o filtro e a reprojeção não perderem a borda.
    let folga = 0.01;
    let pedido = [c[0] - folga, c[1] - folga, c[2] + folga, c[3] + folga];
    let epsg = nisar::utm_do_harmony(c);
    let pols = match qual {
        Some(p) => vec![p],
        None => vec![nisar::Pol::Hh, nisar::Pol::Hv],
    };
    let ha = nomes()?;
    let mut feitas = Vec::new();
    for p in pols {
        let nome = nisar::nome_da_camada(p, data);
        if qual.is_none() && ha.contains(&nome) {
            feitas.push(nome);
            continue;
        }
        remover(&[&nome])?;
        let pedidos: Vec<(String, String)> =
            cenas.iter().map(|k| (k.url(p, pedido, epsg), format!("{}NISAR {} {}", PREFIXO, p.sigla(), k.rotulo()))).collect();
        let sar = Sar {
            sigla: p.sigla(),
            de_onde: "pelo Harmony da NASA (cerca de 40 s e 6 MB cada, para um município)",
            celula: nisar::CELULA,
            escala: p.escala(),
            rampa: Some(nisar::RAMPA),
            sigma: Some(nisar::SIGMA),
        };
        if let Some(pendente) = sar_na_grade(&pedidos, &sar, &g, c, &nome)? {
            return Ok(pendente);
        }
        feitas.push(nome);
    }
    let mut camada = feitas[0].clone();
    if qual.is_none() {
        let razao = nisar::nome_da_razao(data);
        remover(&[&razao])?;
        // Em dB, a razão HV/HH é a diferença HV − HH.
        diferenca(&feitas[0], &feitas[1], &razao)?;
        let (min, max) = nisar::ESCALA_DA_RAZAO;
        let _ = host::run("raster.stretch", json!({"layer": razao, "type": "custom", "min": min, "max": max}));
        camada = razao;
    }
    avisar(&format!(
        "{}: {} da passagem de {}, na grade do projeto, {}",
        camada.trim_start_matches(PREFIXO),
        if cenas.len() == 1 { "1 cena".to_string() } else { format!("{} cenas", cenas.len()) },
        data.texto(),
        g.texto()
    ));
    Ok(json!({
        "layer": camada, "date": data.texto(), "scenes": cenas.iter().map(|k| k.nome.clone()).collect::<Vec<_>>(),
        "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// Uma polarização de uma passagem da composição temporal: o nome da camada em dB e os pedidos ao
/// Harmony (endereço, nome da camada bruta).
struct CamadaDaPassagem {
    pol: nisar::Pol,
    nome: String,
    pedidos: Vec<(String, String)>,
}

/// Se o retângulo `c` cabe no NISAR.
fn conferir_tamanho_do_nisar(c: [f64; 4]) -> Result<(), String> {
    if nisar::cabe(c) {
        return Ok(());
    }
    Err(format!(
        "a área passa de {:.0}° de lado; o NISAR, a 20 m, lê no máximo cerca de 110 km (uma microrregião). Escolha uma área menor",
        nisar::MAX_GRAUS
    ))
}

/// A composição temporal do NISAR, como a da NASA sobre o Vetrivier (África do Sul): a mediana do
/// HH (vermelho), a mediana do HV (verde) e o desvio-padrão do HV (azul) em várias passagens da
/// mesma trilha, em dB. A mediana tira o granulado que uma passagem só deixa (e, ao contrário da
/// média, não se deixa puxar por uma passagem fora do comum); o desvio do HV sobe onde a
/// vegetação muda entre as passagens (lavoura que cresce ou é colhida). Só a composição fica no
/// mapa.
fn nisar_no_tempo(req: &Request) -> Result<Value, String> {
    let PedidoAsf { c, celula, .. } = area_do_asf(req, "o NISAR")?;
    conferir_tamanho_do_nisar(c)?;
    let n = req.f64("passagens").map_or(nisar::PASSAGENS_DO_TEMPO, |x| x.round().max(0.0) as usize).clamp(2, nisar::MAX_PASSAGENS);
    let g = grade_para(c, celula)?;
    let passagens = match passagens_do_nisar(c, None, n)? {
        Ok(p) => p,
        Err(p) => return Ok(p),
    };
    if passagens.len() < 2 {
        return Err(format!(
            "a trilha da passagem de {} só tem ela na área por enquanto: a composição temporal precisa de pelo menos 2 passagens",
            passagens[0][0].data.texto()
        ));
    }
    let folga = 0.01;
    let pedido = [c[0] - folga, c[1] - folga, c[2] + folga, c[3] + folga];
    let epsg = nisar::utm_do_harmony(c);
    let pols = [nisar::Pol::Hh, nisar::Pol::Hv];
    // Cada polarização de cada passagem: o nome da camada em dB e os pedidos ao Harmony.
    let mut camadas: Vec<CamadaDaPassagem> = Vec::new();
    for cenas in &passagens {
        for pol in pols {
            let pedidos =
                cenas.iter().map(|k| (k.url(pol, pedido, epsg), format!("{}NISAR {} {}", PREFIXO, pol.sigla(), k.rotulo()))).collect();
            camadas.push(CamadaDaPassagem { pol, nome: nisar::nome_da_camada(pol, cenas[0].data), pedidos });
        }
    }
    let ha = nomes()?;
    // Os arquivos que faltam, poucos de cada vez ([`nisar::PEDIDOS_DE_UMA_VEZ`]): na janela, o
    // comando roda de novo quando eles chegam, e pede os próximos.
    let todos_os_que_faltam: Vec<(&str, &str)> = camadas
        .iter()
        .filter(|k| !ha.contains(&k.nome))
        .flat_map(|k| k.pedidos.iter().map(|(u, b)| (u.as_str(), b.as_str())))
        .filter(|(_, b)| !ha.iter().any(|h| h == b))
        .collect();
    let faltam: Vec<(&str, &str)> = todos_os_que_faltam.iter().take(nisar::PEDIDOS_DE_UMA_VEZ).copied().collect();
    let baixando = baixar_inteiros(&faltam, &ha)?;
    if baixando > 0 {
        avisar(&format!(
            "NISAR, composição temporal: baixando {baixando} dos {} recortes que faltam, pelo Harmony da NASA ({} passagens; {} de cada vez, cerca de 1 min cada vez)",
            todos_os_que_faltam.len(),
            passagens.len(),
            nisar::PEDIDOS_DE_UMA_VEZ
        ));
        return Ok(json!({"pending": baixando}));
    }
    for k in &camadas {
        if ha.contains(&k.nome) {
            continue;
        }
        let sar = Sar {
            sigla: k.pol.sigla(),
            de_onde: "pelo Harmony da NASA",
            celula: nisar::CELULA,
            escala: k.pol.escala(),
            rampa: None,
            sigma: Some(nisar::SIGMA),
        };
        if let Some(pendente) = sar_na_grade(&k.pedidos, &sar, &g, c, &k.nome)? {
            return Ok(pendente);
        }
    }
    let datas: Vec<String> = passagens.iter().map(|cs| cs[0].data.iso()).collect();
    let de_cada = |p: nisar::Pol| -> Vec<String> { camadas.iter().filter(|k| k.pol == p).map(|k| k.nome.clone()).collect() };
    let (hh, hv) = (de_cada(nisar::Pol::Hh), de_cada(nisar::Pol::Hv));
    let (ultima, primeira) = (passagens[0][0].data, passagens[passagens.len() - 1][0].data);
    let nome = nisar::nome_da_temporal(primeira, ultima);
    let (hh_media, hv_media, hv_desvio) = (format!("{nome} HH"), format!("{nome} HV"), format!("{nome} desvio"));
    remover(&[&nome, &hh_media, &hv_media, &hv_desvio])?;
    // A mediana; um RustGIS antes da 0.8.0 (sem a mediana no `cube.reduce`, opengeos/RustGIS#2596)
    // fica com a média, e o aviso diz.
    let mut estatistica = "mediana";
    let feito = (|| -> Result<(), String> {
        // O `cube.reduce` deixa de fora, em cada célula, as passagens sem dado nela.
        for (entradas, saida) in [(&hh, &hh_media), (&hv, &hv_media)] {
            let mediana = host::run("cube.reduce", json!({"layers": entradas, "dates": datas, "stat": "median", "name": saida}));
            if let Err(e) = mediana {
                host::log(&format!("nisar: sem a mediana no cube.reduce ({e}); vai a média"));
                estatistica = "média";
                host::run("cube.reduce", json!({"layers": entradas, "dates": datas, "stat": "mean", "name": saida}))?;
            }
        }
        host::run("cube.reduce", json!({"layers": hv, "dates": datas, "stat": "std", "name": hv_desvio}))?;
        ferramenta("geolibre:composite_bands", json!({"inputs": [hh_media, hv_media, hv_desvio]}), &nome)?;
        host::run("raster.symbology", json!({"layer": nome, "type": "rgb"}))?;
        let [r, g_, b] = nisar::ESCALAS_DA_TEMPORAL;
        host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": [r.0, g_.0, b.0], "max": [r.1, g_.1, b.1]}))?;
        Ok(())
    })();
    let mut fora: Vec<&str> = vec![&hh_media, &hv_media, &hv_desvio];
    fora.extend(hh.iter().chain(&hv).map(String::as_str));
    remover(&fora)?;
    feito?;
    sob_as_linhas(&nome)?;
    let mut texto = format!(
        "{}: {} passagens da trilha {} ({}), na grade do projeto, {}. Vermelho: {estatistica} do HH (solo exposto, superfícies ásperas, construções); verde: {estatistica} do HV (vegetação); azul: desvio-padrão do HV entre as passagens (vegetação que mudou)",
        nome.trim_start_matches(PREFIXO),
        passagens.len(),
        passagens[0][0].trilha,
        passagens.iter().map(|cs| cs[0].data.texto()).collect::<Vec<_>>().join(", "),
        g.texto()
    );
    if estatistica == "média" {
        texto.push_str(". A mediana precisa do RustGIS 0.8.0 ou mais novo; neste, foi a média");
    }
    avisar(&texto);
    Ok(json!({"layer": nome, "dates": datas, "report": texto}))
}

/// O radar do Sentinel-1 em 10 m do Planetary Computer ([`radar_pc`]) na polarização pedida
/// ([`radar_10m`]).
fn radar_10m_no_tempo(req: &Request) -> Result<Value, String> {
    let pol = match req.str("polarizacao").map(str::trim) {
        None | Some("") | Some("VV") | Some("vv") => radar::Pol::Vv,
        Some("VH") | Some("vh") => radar::Pol::Vh,
        Some(o) => return Err(format!("polarização '{o}': use VV ou VH")),
    };
    radar_10m(req, pol, false).map(com_aviso_de_posicao)
}

/// O resultado `r` do radar de 10 m com o aviso da posição ([`radar_pc::AVISO_POSICAO`]), no mapa e no
/// texto da janela do comando; um pedido pendente fica como está.
fn com_aviso_de_posicao(mut r: Value) -> Value {
    if pendente(&r) || r.get("layer").is_none() {
        return r;
    }
    avisar(radar_pc::AVISO_POSICAO);
    if let Some(t) = r["report"].as_str() {
        r["report"] = json!(format!("{t}\n\n{}", radar_pc::AVISO_POSICAO));
    }
    r
}

/// A composição colorida do radar de 10 m: a mediana do VV no vermelho, a do VH no verde e VV − VH
/// (a razão VV/VH, em dB) no azul. Faz antes as camadas VV e VH ([`radar_10m`]), que ficam no
/// mapa; as de um clique anterior com as mesmas passagens servem.
fn radar_10m_cor(req: &Request) -> Result<Value, String> {
    let vv = radar_10m(req, radar::Pol::Vv, true)?;
    if pendente(&vv) || vv.get("layer").is_none() {
        return Ok(vv);
    }
    let vh = radar_10m(req, radar::Pol::Vh, true)?;
    if pendente(&vh) || vh.get("layer").is_none() {
        return Ok(vh);
    }
    let (camada_vv, camada_vh) =
        (vv["layer"].as_str().unwrap_or_default().to_string(), vh["layer"].as_str().unwrap_or_default().to_string());
    let Some(nome) = radar_pc::nome_da_composicao(&camada_vv) else {
        return Err(format!("a camada {camada_vv} não tem o nome esperado"));
    };
    let azul = format!("{nome} azul");
    remover(&[&nome, &azul])?;
    let feito = (|| -> Result<(), String> {
        // VV − VH: a razão VV/VH em dB.
        calculadora(&format!("\"{camada_vv}\" - \"{camada_vh}\""), &azul)?;
        ferramenta("geolibre:composite_bands", json!({"inputs": [camada_vv, camada_vh, azul]}), &nome)?;
        host::run("raster.symbology", json!({"layer": nome, "type": "rgb"}))?;
        let [r, g, b] = radar_pc::ESCALAS_DA_COMPOSICAO;
        host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": [r.0, g.0, b.0], "max": [r.1, g.1, b.1]}))?;
        Ok(())
    })();
    remover(&[&azul])?;
    feito?;
    sob_as_linhas(&nome)?;
    let [r, g, b] = radar_pc::ESCALAS_DA_COMPOSICAO;
    let faixa = |e: (f64, f64)| format!("{} a {} dB", decimal(e.0, 0), decimal(e.1, 1).trim_end_matches(",0"));
    let texto = format!(
        "{}: vermelho, a mediana do VV ({}); verde, a do VH ({}); azul, VV − VH, a razão VV/VH ({}). Cidade e construções em rosa e magenta, mata e cerrado em amarelo e verde, lavoura e solo exposto em lilás, roxo e azul, água preta; as camadas VV e VH também ficam no mapa",
        nome.trim_start_matches(PREFIXO),
        faixa(r),
        faixa(g),
        faixa(b)
    );
    avisar(&texto);
    let mut saida = vv;
    saida["layer"] = json!(nome);
    saida["report"] = json!(texto);
    Ok(com_aviso_de_posicao(saida))
}

/// O radar do Sentinel-1 em 10 m do Planetary Computer ([`radar_pc`]): a mediana das passagens
/// mais recentes da mesma órbita, na polarização `pol`, em dB, na grade do projeto, em Viridis. A
/// janela da área de cada cena é lida por faixas; na janela do RustGIS, o comando roda de novo
/// quando elas chegam. Com `reaproveitar`, a camada já no mapa com as mesmas passagens serve.
fn radar_10m(req: &Request, pol: radar::Pol, reaproveitar: bool) -> Result<Value, String> {
    let n = req.f64("passagens").map_or(radar_pc::PASSAGENS, |x| x.round().max(0.0) as usize).clamp(2, radar_pc::MAX_PASSAGENS);
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if (c[2] - c[0]).max(c[3] - c[1]) > radar_pc::MAX_GRAUS {
        return Err(format!(
            "a área passa de {}° de lado: o radar de 10 m lê áreas de até cerca de 220 km; escolha uma área menor",
            decimal(radar_pc::MAX_GRAUS, 1)
        ));
    }
    let sem_janela = "o radar de 10 m precisa de um RustGIS que leia só a janela da área de um arquivo remoto (desktop ou linha de comando, depois da 0.5.0)";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    // Sem grade no mapa, uma de 10 m, a célula do dado (se couber na área).
    let celula = match (celula, grade_no_mapa()?) {
        (None, None) => Some(celula_que_cabe(c, 10.0)),
        (m, _) => m,
    };
    let g = grade_para(c, celula)?;
    conferir_grade_fina(c, g.celula, "o radar de 10 m")?;
    // As passagens e o token.
    let r = match buscar_varios(&[radar_pc::url_busca(c), radar_pc::URL_DO_TOKEN.to_string()])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let passagens = radar_pc::passagens(&r[0], pol, n);
    if passagens.len() < 2 {
        return Err(format!(
            "o Planetary Computer tem {} passagem(ns) do Sentinel-1 nesta área: a mediana precisa de pelo menos 2",
            passagens.len()
        ));
    }
    let token = radar_pc::token(&r[1]).ok_or("o serviço de tokens do Planetary Computer não deu um token: tente de novo daqui a pouco")?;
    let (ultima, primeira) = (passagens[0].data, passagens[passagens.len() - 1].data);
    let nome = radar_pc::nome_da_camada(pol, passagens.len(), primeira, ultima);
    let datas: Vec<String> = passagens.iter().map(|p| p.data.iso()).collect();
    if reaproveitar && nomes()?.contains(&nome) {
        return Ok(json!({"layer": nome, "dates": datas}));
    }
    let leitura = radar_pc::celula_de_leitura(c, g.celula);
    let folga = 0.005;
    let janela = [c[0] - folga, c[1] - folga, c[2] + folga, c[3] + folga];
    // As janelas que faltam (as de um clique anterior ficam).
    let ha = nomes()?;
    let mut lendo = 0;
    let mut sem_janela_lida = false;
    for ps in &passagens {
        for (i, href) in ps.arquivos.iter().enumerate() {
            let nome = radar_pc::nome_da_janela(pol, ps.data, i);
            if ha.contains(&nome) {
                continue;
            }
            if lendo >= crate::FILA_DO_APP {
                break;
            }
            let r = host::run(
                "layer.addUrl",
                // Com folga: a célula de uma visão reduzida passa um pouco do dobro (20,007 m
                // numa janela de largura ímpar), e o RustGIS só a escolhe se ela não passar do
                // pedido.
                json!({"url": radar_pc::assinar(href, &token), "name": nome, "bbox": janela, "cellSize": leitura * 1.05, "zoom": false}),
            )
            .map_err(|e| {
                if falha_passageira(&e) || e.contains("timeout") || e.contains("403") {
                    format!("o Planetary Computer não respondeu agora: tente de novo daqui a pouco ({e})")
                } else {
                    e
                }
            })?;
            if pendente(&r) {
                lendo += 1;
            } else if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
                sem_janela_lida = true;
            }
        }
    }
    if sem_janela_lida {
        let fora: Vec<String> =
            passagens.iter().flat_map(|ps| (0..ps.arquivos.len()).map(move |i| radar_pc::nome_da_janela(pol, ps.data, i))).collect();
        let fora: Vec<&str> = fora.iter().map(String::as_str).collect();
        remover(&fora)?;
        return Err(sem_janela.into());
    }
    if lendo > 0 {
        avisar(&format!(
            "Radar {} 10 m (Sentinel-1, Planetary Computer): lendo a área em {lendo} cena(s) de {} passagens; a mediana vem quando chegarem",
            pol.sigla(),
            passagens.len()
        ));
        return Ok(json!({"pending": lendo}));
    }
    let (mediana, log, recorte) = (format!("{nome} linear"), format!("{nome} log"), format!("{nome} recorte"));
    let mut temporarias: Vec<String> = Vec::new();
    remover(&[&nome, &mediana, &log, &recorte])?;
    let mut estatistica = "mediana";
    let feito = (|| -> Result<(), String> {
        // Cada passagem na grade, no retângulo da área: as cenas do dia juntadas.
        let mut na_grade = Vec::new();
        let mut datas = Vec::new();
        for ps in &passagens {
            let mut partes = Vec::new();
            for i in 0..ps.arquivos.len() {
                let bruta = radar_pc::nome_da_janela(pol, ps.data, i);
                temporarias.push(bruta.clone());
                // O 0 e os negativos (sem dado na borda da cena) ficam de fora da mediana.
                host::run("raster.mask", json!({"layer": bruta, "min": 1e-6, "max": 1000.0}))?;
                let (q, a) = (format!("{bruta} na grade"), format!("{bruta} na área"));
                let resampling = if g.celula > 1.5 * leitura { "average" } else { "bilinear" };
                host::run(
                    "raster.project",
                    json!({"layer": bruta, "crs": g.epsg, "cellSize": g.celula, "resampling": resampling, "name": q}),
                )?;
                temporarias.push(q.clone());
                recortar_retangulo(&q, c, g.celula, &a)?;
                temporarias.push(a.clone());
                partes.push(a);
            }
            let dia = if partes.len() == 1 {
                partes.remove(0)
            } else {
                let m = format!("{}radar 10 m {} {} junto", PREFIXO, pol.sigla(), ps.data.iso());
                mosaicar(&partes, &m)?;
                temporarias.push(m.clone());
                m
            };
            na_grade.push(dia);
            datas.push(ps.data.iso());
        }
        // A mediana; um RustGIS antes da 0.8.0 (sem a mediana no `cube.reduce`) fica com a média.
        if let Err(e) = host::run("cube.reduce", json!({"layers": na_grade, "dates": datas, "stat": "median", "name": mediana})) {
            host::log(&format!("radar 10 m: sem a mediana no cube.reduce ({e}); vai a média"));
            estatistica = "média";
            host::run("cube.reduce", json!({"layers": na_grade, "dates": datas, "stat": "mean", "name": mediana}))?;
        }
        temporarias.push(mediana.clone());
        // Em dB: 10 · log10 (o `bandMath` não tem logaritmo; o GeoLibre tem).
        ferramenta("geolibre:log10", json!({"input": mediana}), &log)?;
        temporarias.push(log.clone());
        let db = format!("{nome} dB");
        host::run("raster.function", json!({"layer": log, "function": "bandMath", "expression": "b1 * 10", "name": db}))?;
        temporarias.push(db.clone());
        recortar(&db, AREA, &recorte)?;
        temporarias.push(recorte.clone());
        host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "nearest", "name": nome}),
        )?;
        if host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": radar_pc::RAMPA})).is_err() {
            host::log("radar 10 m: sem a rampa Viridis; fica a de cinza");
        }
        let (min, max) = radar_pc::escala(pol);
        let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": min, "max": max}));
        Ok(())
    })();
    let fora: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&fora)?;
    feito?;
    sob_as_linhas(&nome)?;
    let (min, max) = radar_pc::escala(pol);
    let mut texto = format!(
        "{}: a {estatistica} do retroespalhamento {} de {} passagens da órbita {} ({}), de {} a {}, em dB, na grade do projeto, {}; lida em células de {} m. A {estatistica} de várias passagens tira o granulado sem borrar as bordas. Viridis de {} a {} dB: água e sombra do relevo escuras, cidade e encostas viradas para o satélite em amarelo",
        nome.trim_start_matches(PREFIXO),
        pol.sigla(),
        passagens.len(),
        passagens[0].orbita,
        if passagens[0].sentido == "ascending" { "ascendente" } else { "descendente" },
        primeira.texto(),
        ultima.texto(),
        g.texto(),
        leitura,
        decimal(min, 0),
        decimal(max, 0)
    );
    if estatistica == "média" {
        texto.push_str(". A mediana precisa do RustGIS 0.8.0 ou mais novo; neste, foi a média");
    }
    avisar(&texto);
    Ok(json!({"layer": nome, "dates": datas, "report": texto}))
}

/// A composição colorida do NISAR: HH no vermelho, HV no verde e HH − HV no azul, em dB, nas
/// escalas do estilo da NASA ([`nisar::ESCALAS_DA_COMPOSICAO`]). A cidade (o eco duplo das
/// construções: HH forte, HV fraco) sai rosa e magenta, a mata (o volume das copas: HV forte)
/// verde forte, o pasto e o solo exposto (HV fraco) azuis e roxos, a água escura. Faz antes as
/// camadas HH, HV e a razão, que ficam no mapa.
fn composicao_nisar(req: &Request) -> Result<Value, String> {
    let r = radar_nisar(req, None)?;
    let Some(data) = r["date"].as_str().and_then(Data::ler) else {
        return Ok(r);
    };
    let (hh, hv, razao) =
        (nisar::nome_da_camada(nisar::Pol::Hh, data), nisar::nome_da_camada(nisar::Pol::Hv, data), nisar::nome_da_razao(data));
    let nome = nisar::nome_da_composicao(data);
    let azul = format!("{nome} azul");
    remover(&[&nome, &azul])?;
    let feito = (|| -> Result<(), String> {
        // HH − HV é o contrário da razão.
        host::run("raster.function", json!({"layer": razao, "function": "bandMath", "expression": "b1 * -1", "name": azul}))?;
        ferramenta("geolibre:composite_bands", json!({"inputs": [hh, hv, azul]}), &nome)?;
        host::run("raster.symbology", json!({"layer": nome, "type": "rgb"}))?;
        let [vermelho, verde, azul_] = nisar::ESCALAS_DA_COMPOSICAO;
        host::run(
            "raster.stretch",
            json!({"layer": nome, "type": "custom", "min": [vermelho.0, verde.0, azul_.0], "max": [vermelho.1, verde.1, azul_.1]}),
        )?;
        Ok(())
    })();
    remover(&[&azul])?;
    feito?;
    avisar(&format!(
        "{}: vermelho HH, verde HV, azul HH − HV. No estilo das imagens do NISAR da NASA: mata em verde forte, cidade em rosa e magenta, pasto e solo exposto em azul e roxo escuros, água escura",
        nome.trim_start_matches(PREFIXO)
    ));
    let mut r = r;
    r["layer"] = json!(nome);
    Ok(r)
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
    // Uma grande região são os contornos dos estados dela, que juntos pesam demais.
    if do_painel && n <= 50 && !area_de_estados(&camada) {
        return Ok(descrever(&host::features(AREA, false)?));
    }
    // O limite oficial de um município ([`afinar_area`]) também pode ser pesado demais para o plugin
    // ler: os nomes vêm de uma cópia simplificada.
    if tem_campo(&camada, "malha")
        && let Some(c) = extensao(AREA)?
    {
        return Ok(texto_da_area(&nomes_da_area(c)?, n, Some(c)));
    }
    Ok(texto_da_area(&[], n, extensao(AREA)?))
}

/// Se a área (como [`camada_no_projeto`] a descreve) é uma grande região do painel: os contornos dos
/// estados dela, que juntos o plugin não lê sem passar da cota de instruções.
fn area_de_estados(camada: &Value) -> bool {
    camada["features"].as_u64().unwrap_or(0) > 1 && tem_campo(camada, "malha") && tem_campo(camada, "codigo")
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
    // A amostragem: os números de pontos e os espaçamentos das posições dos controles.
    let pontos: Vec<String> = amostragem::PONTOS.iter().map(|n| format!("{n} pontos")).collect();
    let espacamentos: Vec<String> = amostragem::ESPACAMENTOS.iter().map(|m| amostragem::texto_do_espacamento(*m)).collect();
    r["panel"]["items"][AMOSTRA_PONTOS] = json!({"labels": pontos});
    r["panel"]["items"][AMOSTRA_ESPACAMENTO] = json!({"labels": espacamentos});
    // O mês do risco de fogo, pelo nome.
    r["panel"]["items"][risco_fogo::CONTROLE] = json!({"labels": risco_fogo::NOMES_DOS_MESES});
    if let Some(g) = g {
        grade_no_painel(&mut r, &g);
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
    let mut r = area_municipal(&t)?;
    if pendente(&r) {
        return Ok(r);
    }
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

/// O máximo de municípios da camada de indicadores. Os contornos vêm num pedido só (o WFS), e o
/// limite é o dos pedidos das fontes: o crédito rural pede [`indicadores::POR_PEDIDO_SICOR`]
/// municípios por vez, e uma chamada faz até 64 pedidos.
const LIMITE_DE_MUNICIPIOS: usize = 200;

/// Os municípios de uma área do painel (estado, meso, micro ou município), pela lista embutida: a
/// Zona da Mata tem 142, e o retângulo dela pegava 288 no WFS, que passavam do
/// [`LIMITE_DE_MUNICIPIOS`]. `None` sem área, ou num desenho ou numa seleção, que não têm código.
fn municipios_das_malhas() -> Result<Option<Vec<String>>, String> {
    let Some(camada) = camada_no_projeto(AREA)? else {
        return Ok(None);
    };
    let area = match extensao(AREA)? {
        Some(c) if area_de_estados(&camada) => area_simplificada(c)?,
        _ => host::features(AREA, false)?,
    };
    let feicoes = area["features"].as_array().cloned().unwrap_or_default();
    if feicoes.is_empty() {
        return Ok(None);
    }
    let malhas: Option<Vec<Vec<&str>>> =
        feicoes.iter().map(|f| f["properties"]["codigo"].as_str().and_then(municipios::da_malha)).collect();
    Ok(malhas.map(|v| v.concat().into_iter().map(str::to_string).collect()))
}

/// Os códigos dos municípios do retângulo da área (do WFS). Num desenho ou numa seleção com mais
/// do que o [`LIMITE_DE_MUNICIPIOS`], só os que têm o ponto de dentro na área.
fn municipios_do_retangulo(municipios: Vec<(String, String, String)>) -> Result<Vec<String>, String> {
    let codigos: Vec<String> = municipios.into_iter().map(|m| m.0).collect();
    if codigos.len() <= LIMITE_DE_MUNICIPIOS || !nomes()?.iter().any(|n| n == AREA) {
        return Ok(codigos);
    }
    let area = host::features(AREA, false)?;
    let geometrias: Vec<&Value> = area["features"].as_array().into_iter().flatten().map(|f| &f["geometry"]).collect();
    Ok(codigos
        .into_iter()
        .filter(|cd| censo::ponto_do_municipio(cd).is_some_and(|p| geometrias.iter().any(|g| censo::dentro(p, g))))
        .collect())
}

/// Os municípios que cruzam a área, com os indicadores das `fontes`, numa camada só: a
/// "Indicadores por município". Os contornos são os da Malha Municipal 2025 do IBGE, que o próprio
/// RustGIS baixa do WFS pelo retângulo da área: a geometria não passa pelo plugin (ler aqui os
/// contornos da Malha estourava a cota de instruções da chamada, e os da API de malhas são
/// generalizados). A camada fica mascarada pelo limite da área (o Masking do RustGIS), e os
/// indicadores entram nos campos pelo Calculate Field, um CASE pelo código do município. Se a camada
/// já está no mapa para esta área, os campos novos se somam aos que ela tem.
fn indicadores(fontes: &[Fonte]) -> Result<Value, String> {
    let c = retangulo_da_area()?;
    // Os municípios: os da malha da área, sem rede; num desenho ou numa seleção, os do retângulo,
    // pelo WFS (e a camada dos contornos, que vêm pelo retângulo, é reconhecida por esse número).
    let (codigos, no_retangulo) = match municipios_das_malhas()? {
        Some(codigos) => (codigos, None),
        None => {
            let r = match buscar_varios(&[censo::url_municipios(c)])? {
                Ok(r) => r,
                Err(p) => return Ok(p),
            };
            let municipios = censo::municipios(&r[0]);
            let n = municipios.len();
            (municipios_do_retangulo(municipios)?, Some(n))
        }
    };
    if codigos.is_empty() {
        return Err("nenhum município do IBGE na área".into());
    }
    // Os pedidos das fontes (o SIDRA, o SICOR) vão por município, numa chamada de até 64 pedidos.
    if codigos.len() > LIMITE_DE_MUNICIPIOS {
        return Err(format!(
            "{} municípios na área; o plugin traz até {LIMITE_DE_MUNICIPIOS} de cada vez: escolha uma área menor",
            codigos.len()
        ));
    }
    let nome = format!("{PREFIXO}{INDICADORES}");
    // Os contornos: os da camada que está no mapa, se ela é desta área; senão, o WFS.
    let info = host::run("info.project", json!({}))?;
    let atual = info["layers"].as_array().into_iter().flatten().find(|l| l["name"].as_str() == Some(nome.as_str()));
    let pronta = atual.is_some_and(|l| censo::contornos_da_area(l, c, no_retangulo));
    // O app desktop também baixa o WFS fora do comando e o roda de novo quando a camada chega: na
    // nova rodada, ela está no mapa com os campos do WFS e ainda sem o `malha`.
    let recem_baixada = !pronta && atual.is_some_and(|l| censo::contornos_recem_baixados(l, c, no_retangulo));
    if !pronta && !recem_baixada {
        remover(&[&nome])?;
        let r = host::run("layer.addUrl", json!({"url": censo::url_contornos(c), "name": nome, "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            return Ok(r);
        }
    }
    if !pronta {
        host::run("edit.addField", json!({"layer": nome, "name": "malha", "type": "text"}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": "malha", "expression": format!("'{}'", censo::MALHA)}))?;
        for campo in censo::CAMPOS_DO_WFS {
            let _ = host::run("edit.deleteField", json!({"layer": nome, "field": campo}));
        }
    }
    // Num pedido de várias fontes, as que já estão na camada desta área não são pedidas de novo: um
    // novo clique continua de onde o anterior parou.
    let ja_na_camada: Vec<String> = if pronta && fontes.len() > 1 {
        atual
            .and_then(|l| l["fields"].as_array())
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(|f| f.split(" (").next().unwrap_or(f).to_string())
            .collect()
    } else {
        Vec::new()
    };
    let mut pesadas_nesta = 0;
    let limite_de_pesadas = indicadores::pesadas_por_chamada(codigos.len());
    let mut campos = indicadores::PorMunicipio::new();
    let (mut feitas, mut faltam, mut erros) = (Vec::new(), Vec::new(), Vec::new());
    for &f in fontes {
        if ja_na_camada.iter().any(|c| f.eh_campo(c)) {
            feitas.push(f.id());
            continue;
        }
        // As fontes pesadas (as tabelas do SIDRA, o SICOR) gastam a cota de instruções da chamada
        // na proporção dos municípios: com muitos, poucas por clique.
        if f.pesada() {
            if pesadas_nesta >= limite_de_pesadas {
                faltam.push(f.id());
                continue;
            }
            pesadas_nesta += 1;
        }
        match campos_de(f, &codigos) {
            Ok(Ok(m)) => {
                indicadores::juntar(&mut campos, m);
                feitas.push(f.id());
            }
            // App web: o RustGIS roda o comando de novo quando as respostas chegarem; os contornos
            // ficam no mapa, e a nova rodada os reaproveita.
            Ok(Err(pendente)) => return Ok(pendente),
            Err(e) if limite_da_chamada(&e) => faltam.push(f.id()),
            Err(e) => {
                host::log(&format!("{}: {e}", f.id()));
                erros.push(format!("{}: {e}", f.id()));
            }
        }
    }
    // Cada campo, de uma vez em todos os municípios: um CASE pelo código, nulo onde a fonte não
    // tem o dado (e nos municípios do retângulo que a lista não tem).
    let existentes: Vec<String> = host::run("info.project", json!({}))?["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| l["name"].as_str() == Some(nome.as_str()))
        .and_then(|l| l["fields"].as_array())
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(|f| f.split(" (").next().unwrap_or(f).to_string())
        .collect();
    for (campo, tipo, expressao) in censo::campos_por_case(&campos) {
        if !existentes.iter().any(|e| e.eq_ignore_ascii_case(&campo)) {
            host::run("edit.addField", json!({"layer": nome, "name": campo, "type": tipo}))?;
        }
        host::run("edit.calculateField", json!({"layer": nome, "field": campo, "expression": expressao}))?;
    }
    // Só a área: a camada mascarada pelo limite (sem área escolhida, a vista inteira). O campo
    // `na_area` marca os municípios cujo centro cai dentro dela: o layout só os rotula, e os nomes
    // dos vizinhos não ficam soltos na parte mascarada.
    if nomes()?.iter().any(|n| n == AREA) {
        let _ = host::run("layer.mask", json!({"layer": nome, "mask": AREA}));
        let area = host::features(AREA, false)?;
        let geometrias: Vec<&Value> = area["features"].as_array().into_iter().flatten().map(|f| &f["geometry"]).collect();
        let na_area: Vec<&String> = codigos
            .iter()
            .filter(|cd| censo::ponto_do_municipio(cd).is_some_and(|p| geometrias.iter().any(|g| censo::dentro(p, g))))
            .collect();
        if !existentes.iter().any(|e| e == "na_area") {
            host::run("edit.addField", json!({"layer": nome, "name": "na_area", "type": "long"}))?;
        }
        let lista: Vec<String> = na_area.iter().map(|cd| format!("'{cd}'")).collect();
        let expressao =
            if lista.is_empty() { "0".to_string() } else { format!("CASE WHEN cd_mun IN ({}) THEN 1 ELSE 0 END", lista.join(", ")) };
        host::run("edit.calculateField", json!({"layer": nome, "field": "na_area", "expression": expressao}))?;
    }
    let n = codigos.len();
    let mut aviso = format!("{nome}: {n} municípios");
    if !feitas.is_empty() {
        aviso.push_str(&format!("; com {}", feitas.join(", ")));
    }
    if !faltam.is_empty() {
        aviso.push_str(&format!(
            "; faltam {} (não couberam nesta chamada): clique de novo, e o que já veio não é pedido outra vez",
            faltam.join(", ")
        ));
    }
    if !erros.is_empty() {
        aviso.push_str(&format!("; falharam {}", erros.join("; ")));
    }
    avisar(&aviso);
    if feitas.is_empty() && faltam.is_empty() {
        return Err(erros.join("; "));
    }
    Ok(json!({"layer": nome, "features": n, "sources": feitas, "missing": faltam, "errors": erros}))
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
        Fonte::Saude => campos_saude(codigos),
        Fonte::Seeg => indicadores::campos_seeg(codigos),
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
            Fonte::de(id).ok_or(format!("'{id}' não é uma fonte de indicadores: censo, agro, agroeco, pix, credito, tse, caged ou saude"))
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
        let juntadas = juntar_partes_dos_setores(&nome)?;
        nomear_campos_dos_setores(&nome)?;
        let n = feicoes_da_camada(&nome)?.unwrap_or(0);
        feitas.push(json!({"layer": nome, "features": n, "joined": juntadas}));
    }
    if !baixando.is_empty() {
        avisar(&format!(
            "Setores censitários de {}: baixando; quando a camada aparecer no mapa, clique de novo para recortá-la pela área",
            baixando.join(", ")
        ));
    }
    if !feitas.is_empty() {
        let juntadas: u64 = feitas.iter().filter_map(|f| f["joined"].as_u64()).sum();
        let partes = if juntadas == 0 {
            String::new()
        } else {
            format!(
                " {juntadas} {} do IBGE com partes do mesmo setor (ilhas, pedaços separados) {} juntada{} num setor só: \
                 a população de cada setor conta uma vez.",
                if juntadas == 1 { "linha" } else { "linhas" },
                if juntadas == 1 { "foi" } else { "foram" },
                if juntadas == 1 { "" } else { "s" },
            )
        };
        avisar(&format!(
            "Setores censitários de 2022 recortados pela área ({}).{partes} {}",
            feitas.iter().map(|f| format!("{}: {}", f["layer"].as_str().unwrap_or(""), f["features"])).collect::<Vec<_>>().join("; "),
            censo::CAMPOS_DOS_SETORES
        ));
    }
    Ok(json!({"layers": feitas, "downloading": baixando}))
}

/// Junta numa feição só as linhas de um mesmo setor. A malha com atributos do IBGE grava cada parte
/// de um setor (uma ilha, um pedaço separado) numa linha própria, com a população e os domicílios
/// do setor inteiro repetidos: somar a coluna `pessoas` contaria o setor uma vez por parte (na
/// Grande Vitória, 3.583 pessoas a mais). O `dissolve` por `CD_SETOR` faz um multipolígono por
/// setor com os valores da primeira linha (as linhas de um setor têm os mesmos valores), e cada
/// campo volta ao nome e ao tipo de antes (o RustGIS não renomeia campos: campo novo, valor, e o
/// `FIRST_` sai). A densidade continua certa: ela usa `AREA_KM2`, que já é a área do setor inteiro.
/// Diz quantas linhas a menos a camada ficou; sem setor repetido, ela fica como está.
fn juntar_partes_dos_setores(camada: &str) -> Result<u64, String> {
    let antes = feicoes_da_camada(camada)?.unwrap_or(0);
    let campos = host::run("layer.fields", json!({"layer": camada}))?;
    let lista: Vec<(String, String)> = campos
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| Some((f["name"].as_str()?.to_string(), f["type"].as_str().unwrap_or("Text").to_lowercase())))
        .collect();
    let Some(chave) = lista.iter().map(|(n, _)| n).find(|n| n.eq_ignore_ascii_case("cd_setor")).cloned() else {
        return Ok(0);
    };
    let outros: Vec<&(String, String)> = lista.iter().filter(|(n, _)| *n != chave).collect();
    let estatisticas: Vec<Value> = outros.iter().map(|(n, _)| json!({"field": n, "statistic": "FIRST"})).collect();
    let r = host::run("gp.run", json!({"tool": "dissolve", "params": {"input": camada, "field": chave, "statistics": estatisticas}}))?;
    let juntada = r.get("layer").cloned().ok_or("o dissolve não disse a camada que fez")?;
    let provisoria = format!("{camada} (juntando partes)");
    host::run("layer.rename", json!({"layer": juntada, "name": provisoria}))?;
    let depois = feicoes_da_camada(&provisoria)?.unwrap_or(antes);
    if depois >= antes {
        remover(&[&provisoria])?;
        return Ok(0);
    }
    remover(&[camada])?;
    host::run("layer.rename", json!({"layer": provisoria, "name": camada}))?;
    for (nome, tipo) in outros {
        let primeiro = format!("FIRST_{nome}");
        host::run("edit.addField", json!({"layer": camada, "name": nome, "type": tipo}))?;
        host::run("edit.calculateField", json!({"layer": camada, "field": nome, "expression": primeiro}))?;
        host::run("edit.deleteField", json!({"layer": camada, "field": primeiro}))?;
    }
    // O dissolve conta as linhas de cada grupo num campo "count"; o setor não precisa dele.
    let _ = host::run("edit.deleteField", json!({"layer": camada, "field": "count"}));
    Ok(antes - depois)
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
    // A densidade, para comparar setores de tamanhos tão diferentes (o mapa do layout é dela), com
    // os campos de agora.
    let agora = host::run("layer.fields", json!({"layer": camada}))?;
    let tem = |n: &str| {
        agora.as_array().into_iter().flatten().filter_map(|f| f["name"].as_str()).find(|x| x.eq_ignore_ascii_case(n)).map(str::to_string)
    };
    if let Some(area) = tem("area_km2")
        && tem("pessoas").is_some()
        && tem(censo::CAMPO_DENSIDADE_SETOR).is_none()
    {
        host::run("edit.addField", json!({"layer": camada, "name": censo::CAMPO_DENSIDADE_SETOR, "type": "double"}))?;
        host::run(
            "edit.calculateField",
            json!({"layer": camada, "field": censo::CAMPO_DENSIDADE_SETOR, "expression": format!("pessoas / {area}")}),
        )?;
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
            avisar(&format!(
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
        avisar(&format!("{nome}: nenhum estabelecimento ativo na área ({lidos} lidos; {fora})"));
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
    avisar(&format!("{nome}: {ficaram} de {lidos} na área ({fora}; Ministério da Saúde, pelo geobr)"));
    Ok(json!({"layer": nome, "read": lidos, "features": ficaram, "inactive": desativados, "personal": de_pessoa}))
}

/// Mais UFs que isto no retângulo é área grande demais para a chuva do CEMADEN: cada UF é um
/// pedido de até uns 250 KB, e o serviço não filtra pelo retângulo.
const MAX_UFS_CEMADEN: usize = 6;

/// Mais estações que isto no retângulo é área grande demais: cada uma custa umas 150 mil
/// instruções (o ponto montado em texto), e a cota de um comando é de 5×10⁸ no RustGIS 0.6.
const MAX_ESTACOES_CEMADEN: usize = 1500;

/// `aaaa-mm-dd hh:mm` como `dd/mm/aaaa hh:mm`.
fn data_br(iso: &str) -> String {
    match (iso.get(..4), iso.get(5..7), iso.get(8..10), iso.get(11..16)) {
        (Some(a), Some(m), Some(d), Some(h)) => format!("{d}/{m}/{a} {h}"),
        _ => iso.to_string(),
    }
}

/// Põe as `feicoes` no mapa como `nome`, no lugar da anterior: recortadas pela área quando
/// `recortar` (a camada `(sem recorte)` entra, o clip a recorta e ela sai). Devolve quantas
/// ficaram; sem nenhuma, a camada sai.
fn por_feicoes(nome: &str, feicoes: &[String], recortar: bool) -> Result<u64, String> {
    let bruto = format!("{nome} (sem recorte)");
    remover(&[nome, &bruto])?;
    let n = feicoes.len() as u64;
    // O GeoJSON vai como texto, montado aqui: como `Value`, centenas de pontos passavam da cota
    // de instruções do comando (ver `cemaden::chuva_dos_bytes`).
    let camada = |nome: &str| por_geojson(nome, &format!(r#"{{"type":"FeatureCollection","features":[{}]}}"#, feicoes.join(",")));
    if !recortar {
        camada(nome)?;
        return Ok(n);
    }
    camada(&bruto)?;
    let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": bruto, "clip": AREA}}))?;
    let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": recortada, "name": nome}))?;
    remover(&[&bruto])?;
    let ficaram = feicoes_da_camada(nome)?.unwrap_or(0);
    if ficaram == 0 {
        remover(&[nome])?;
    }
    Ok(ficaram)
}

/// Aplica a `camada` o estilo `texto` (do `style.add`) de nome `estilo`. Se o usuário tinha dado
/// outro à camada anterior, [`devolver_estilos`] o põe de volta depois.
fn estilo_padrao(camada: &str, estilo: &str, texto: &str) {
    let _ = host::run("style.remove", json!({"name": estilo}));
    if host::run("style.add", json!({"text": texto})).is_ok() {
        let _ = host::run("style.apply", json!({"layer": camada, "name": estilo}));
    }
}

#[link(wasm_import_module = "rustgis")]
unsafe extern "C" {
    #[link_name = "call"]
    fn host_call(ptr: *const u8, len: usize) -> i32;
    #[link_name = "result"]
    fn host_result(ptr: *mut u8);
}

/// Um pedido ao RustGIS (`{"op", "params"}`, já em texto), com a resposta em bytes
/// (`{"ok": ...}` ou `{"error": ...}`), como o `host::call` do SDK faz, mas sem ler a resposta
/// como JSON nem montar o pedido como `Value`.
fn chamar_cru(req: &str) -> Result<Vec<u8>, String> {
    // SAFETY: como no SDK: o RustGIS lê `len` bytes de `req`, que vive durante a chamada, e
    // escreve em `buf` exatamente os `len` bytes da resposta que mediu.
    let n = unsafe { host_call(req.as_ptr(), req.len()) };
    let len = usize::try_from(n).map_err(|_| String::from("o RustGIS não respondeu"))?;
    let mut buf: Vec<u8> = Vec::with_capacity(len);
    unsafe {
        host_result(buf.as_mut_ptr());
        buf.set_len(len);
    }
    Ok(buf)
}

/// O `host.fetch` de cada endereço, com as respostas em bytes, sem lê-las como JSON: o
/// `host::call` do SDK lê a resposta inteira (o texto e o JSON do documento) e a copia, e a
/// chuva de três estados do CEMADEN (uns 250 KB cada) já passa da cota de instruções do
/// comando. Uma resposta curta (um erro, o pedido pendente do app web) é lida. Como em
/// [`buscar`], `Ok(Err(p))` é um pedido pendente (até 8 de cada vez): o RustGIS roda o comando de novo.
fn buscar_bytes(urls: &[String]) -> Result<Result<Vec<Vec<u8>>, Value>, String> {
    let mut respostas = Vec::new();
    let (mut pendente, mut esperando) = (None, 0);
    for url in urls {
        let buf = chamar_cru(&json!({"op": "host.fetch", "params": {"url": url}}).to_string())?;
        if buf.len() < 16_384 {
            let v: Value = serde_json::from_slice(&buf).map_err(|e| format!("{url}: resposta ilegível ({e})"))?;
            if let Some(e) = v.get("error") {
                return Err(e.as_str().map(str::to_string).unwrap_or_else(|| e.to_string()));
            }
            if v["ok"].get("pending").and_then(Value::as_bool) == Some(true) {
                pendente = Some(v["ok"].clone());
                esperando += 1;
                if esperando == 8 {
                    break;
                }
                continue;
            }
        }
        respostas.push(buf);
    }
    Ok(match pendente {
        Some(p) => Err(p),
        None => Ok(respostas),
    })
}

/// A chuva medida nas estações do CEMADEN na área: os acumulados de 1 a 96 horas e os 10
/// minutos mais recentes, de cada estação, recortados pela área (sem área, os da vista).
fn chuva_cemaden() -> Result<Value, String> {
    let nome = cemaden::CHUVA;
    let curto = nome.trim_start_matches(PREFIXO);
    let (c, area) = retangulo()?;
    let onde = if area { "na área" } else { "na vista" };
    let ufs = cemaden::ufs_no_retangulo(c);
    if ufs.is_empty() {
        remover(&[nome])?;
        avisar(&format!("{curto}: nenhuma estação do CEMADEN {onde}"));
        return Ok(json!({"layer": null, "features": 0}));
    }
    let no_retangulo = cemaden::quantas_no_retangulo(c);
    if no_retangulo > MAX_ESTACOES_CEMADEN {
        return Err(format!(
            "o retângulo da área tem {no_retangulo} estações do CEMADEN; o limite é {MAX_ESTACOES_CEMADEN} por vez: escolha uma área menor"
        ));
    }
    if ufs.len() > MAX_UFS_CEMADEN {
        return Err(format!(
            "a área cruza {} estados ({}); a chuva do CEMADEN vem por estado, e o limite é {MAX_UFS_CEMADEN}: escolha uma área menor",
            ufs.len(),
            ufs.join(", ")
        ));
    }
    let urls: Vec<String> = ufs.iter().map(|u| cemaden::url_chuva(u)).collect();
    let respostas = match buscar_bytes(&urls)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let bytes: Vec<&[u8]> = respostas.iter().map(Vec::as_slice).collect();
    let chuva = cemaden::chuva_dos_bytes(&bytes, c);
    let lidas = chuva.feicoes.len() as u64;
    let novas = if chuva.sem_coordenadas > 0 {
        format!(
            "; {} estações de {} ficaram de fora, sem coordenadas no plugin (instaladas depois desta versão)",
            chuva.sem_coordenadas,
            ufs.join(", ")
        )
    } else {
        String::new()
    };
    if lidas == 0 {
        remover(&[nome])?;
        avisar(&format!("{curto}: nenhuma estação {onde}{novas}"));
        return Ok(json!({"layer": null, "features": 0}));
    }
    let ficaram = por_feicoes(nome, &chuva.feicoes, area)?;
    if ficaram == 0 {
        avisar(&format!("{curto}: nenhuma estação dentro da área ({lidas} no retângulo){novas}"));
        return Ok(json!({"layer": null, "read": lidas, "features": 0}));
    }
    estilo_padrao(nome, cemaden::ESTILO_CHUVA, &cemaden::estilo_chuva());
    let ultima = chuva.ultima.as_deref().map(|u| format!("; leitura mais recente em {} UTC", data_br(u))).unwrap_or_default();
    let recorte = if area { format!(" de {lidas} no retângulo, recortadas pela área") } else { " na vista".into() };
    avisar(&format!(
        "{curto}: {ficaram} {}{recorte} ({} no retângulo com leitura nas últimas 24 h{ultima}){novas}. Fonte: CEMADEN; só no app desktop",
        if ficaram == 1 { "estação" } else { "estações" },
        chuva.com_leitura
    ));
    Ok(
        json!({"layer": nome, "read": lidas, "features": ficaram, "withReading": chuva.com_leitura, "latest": chuva.ultima, "missingCoordinates": chuva.sem_coordenadas}),
    )
}

/// Os códigos dos municípios (`feicoes`, com o `cod_ibge`) que têm ao menos
/// [`cemaden::FRACAO_MINIMA`] dentro da área: o RustGIS recorta uma cópia, e o plugin compara as
/// áreas. As camadas do recorte saem do mapa.
fn municipios_dentro_da_area(feicoes: &[Value]) -> Result<Vec<String>, String> {
    let teste = format!("{PREFIXO}municípios com alerta (teste)");
    let pedacos = format!("{teste}, recortados");
    remover(&[&teste, &pedacos])?;
    let mut fc = json!({"type": "FeatureCollection"});
    fc["features"] = Value::Array(feicoes.to_vec());
    host::add_layer(&teste, fc)?;
    let feito = (|| -> Result<Vec<String>, String> {
        let r = host::run("gp.run", json!({"tool": "clip", "params": {"input": teste, "clip": AREA}}))?;
        let recortada = r.get("layer").cloned().ok_or("o clip não disse a camada que fez")?;
        host::run("layer.rename", json!({"layer": recortada, "name": pedacos}))?;
        let lidos = host::features(&pedacos, false)?;
        let lista = lidos["features"].as_array().map_or(&[][..], Vec::as_slice);
        Ok(cemaden::dentro_da_area(feicoes, lista))
    })();
    remover(&[&teste, &pedacos])?;
    feito
}

/// Os alertas de risco geo-hidrológico vigentes do CEMADEN nos municípios da área (sem área, os
/// do retângulo da vista), com o contorno inteiro do município: o alerta vale para ele todo, como
/// os limites, que também vêm inteiros.
fn alertas_cemaden() -> Result<Value, String> {
    let nome = cemaden::ALERTAS;
    let curto = nome.trim_start_matches(PREFIXO);
    let (c, area) = retangulo()?;
    let onde = if area { "nos municípios da área" } else { "nos municípios da vista" };
    let r = match buscar_varios(&[cemaden::URL_ALERTAS.to_string()])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let lista = cemaden::alertas(&r[0]);
    // Pelo retângulo do contorno embutido e, com área, pela parte do município dentro dela: o
    // retângulo de um estado pega municípios do vizinho, e um município vizinho deixa na área uma
    // lasca da divisa (o contorno embutido não é o oficial).
    let mut feicoes = cemaden::alertas_no_retangulo(&lista, c, malhas::geometria);
    if area && !feicoes.is_empty() {
        let dentro = municipios_dentro_da_area(&feicoes)?;
        feicoes.retain(|f| f["properties"]["cod_ibge"].as_str().is_some_and(|cod| dentro.iter().any(|d| d == cod)));
    }
    let no_brasil = match lista.len() {
        0 => "nenhum no Brasil agora".to_string(),
        n => format!("{n} no Brasil agora"),
    };
    if feicoes.is_empty() {
        remover(&[nome])?;
        avisar(&format!("{curto}: nenhum alerta vigente {onde} ({no_brasil}; CEMADEN)"));
        return Ok(json!({"layer": null, "features": 0, "brazil": lista.len()}));
    }
    let resumo: Vec<String> = feicoes
        .iter()
        .map(|f| {
            let p = &f["properties"];
            format!(
                "{}/{}: {}, {}",
                p["municipio"].as_str().unwrap_or(""),
                p["uf"].as_str().unwrap_or(""),
                p["evento"].as_str().unwrap_or("").to_lowercase(),
                p["nivel"].as_str().unwrap_or("").to_lowercase()
            )
        })
        .collect();
    let textos: Vec<String> = feicoes.iter().map(Value::to_string).collect();
    let n = por_feicoes(nome, &textos, false)?;
    estilo_padrao(nome, cemaden::ESTILO_ALERTAS, &cemaden::estilo_alertas());
    avisar(&format!(
        "{curto}: {n} {} {onde} ({}; {no_brasil}). Fonte: CEMADEN; só no app desktop",
        if n == 1 { "alerta" } else { "alertas" },
        resumo.join("; ")
    ));
    Ok(json!({"layer": nome, "features": n, "brazil": lista.len()}))
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
    avisar(&format!(
        "{nome}: mapa para ver, com as cores da legenda oficial (brasil.mapbiomas.org); não é recortado pela área{motivo}. Vale por algumas horas"
    ));
    Ok(json!({"tiles": nome, "year": ano, "notClipped": por_que}))
}

/// Os sistemas de irrigação do MapBiomas em `ano`: um mapa para ver, do Brasil inteiro (o
/// MapBiomas não publica um arquivo deles que se leia pela janela da área).
fn irrigacao_mapbiomas(ano: i64) -> Result<Value, String> {
    if !(mapbiomas::PRIMEIRO_ANO..=mapbiomas::ULTIMO_ANO).contains(&ano) {
        return Err(format!("ano é de {} a {}", mapbiomas::PRIMEIRO_ANO, mapbiomas::ULTIMO_ANO));
    }
    let r = host::call("host.fetch", json!({"url": mapbiomas::url_da_irrigacao(ano)}))?;
    if r.get("pending").and_then(Value::as_bool) == Some(true) {
        return Ok(r);
    }
    let url =
        r.get("json").and_then(mapbiomas::mosaicos).ok_or_else(|| format!("a API do MapBiomas não deu o mapa de irrigação de {ano}"))?;
    let nome = mapbiomas::camada_da_irrigacao(ano);
    // O endereço do Earth Engine vence em algumas horas: um novo clique troca o mapa.
    if mosaicos_no_mapa()?.contains(&nome) {
        host::run("map.removeTiles", json!({"name": nome}))?;
    }
    host::run(
        "map.addTiles",
        json!({"url": url, "kind": "raster", "name": nome, "attribution": "MapBiomas, sistemas de irrigação (CC BY 4.0)", "maxZoom": 14}),
    )?;
    avisar(&format!(
        "{nome}: mapa para ver, com as cores da legenda oficial (roxo, pivô central; azul-claro, outros sistemas; rosa, inundação, o arroz irrigado); não é recortado pela área. Vale por algumas horas"
    ));
    Ok(json!({"tiles": nome, "year": ano}))
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
        avisar(&format!("{}; saiu do mapa o arquivo inteiro que uma tentativa anterior trouxe", sem_janela()));
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
                avisar(&format!("{}; saiu do mapa o arquivo inteiro que uma tentativa anterior trouxe", sem_janela()));
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
        avisar(&format!("{} (GEDI L4D): lendo a área em {lendo} arquivo(s) da NASA; a camada vem quando chegarem", v.titulo()));
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
            mosaicar(&na_grade, &junto)?;
            junto.clone()
        };
        recortar(&mosaico, AREA, saida)?;
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
    avisar(&format!(
        "{saida}: em {}, de 2019 a 2023 (GEDI imputado com o Landsat), na grade do projeto, {}{piramide}{media}",
        v.unidade(),
        g.texto()
    ));
    Ok(json!({"layer": saida, "files": urls.len(), "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// Se o RustGIS tem o comando `id` (o `info.commands`).
fn tem_comando(id: &str) -> Result<bool, String> {
    let lista = host::run("info.commands", json!({}))?;
    let comandos = lista.get("commands").unwrap_or(&lista);
    Ok(comandos.as_array().into_iter().flatten().any(|c| c["id"].as_str() == Some(id)))
}

/// A radiação solar do NASA POWER na área ([`sol`]): as células de 1° em volta dela viram um raster
/// de 1°, interpolado para a grade do projeto e recortado pela área.
fn sol_na_area(req: &Request) -> Result<Value, String> {
    let mes = sol::conferir_mes(req.f64("mes"))?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_comando("raster.rasterize")? {
        return Err("a radiação solar precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0): as células de 1° do NASA POWER viram um raster com ele".into());
    }
    let respostas = match buscar_bytes(&sol::pedidos(c))? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let textos: Vec<&str> = respostas.iter().map(|r| std::str::from_utf8(r).unwrap_or_default()).collect();
    if let Some(e) = textos.iter().find_map(|t| sol::erro(t)) {
        return Err(e);
    }
    let (geojson, n) = sol::celulas(&textos, mes);
    if n == 0 {
        return Err("o NASA POWER não tem a radiação solar na área".into());
    }
    let g = grade_para(c, celula)?;
    let saida = sol::camada(mes);
    let temporarias = [sol::CELULAS, sol::RASTER_1, sol::NA_GRADE];
    remover(&temporarias)?;
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        por_geojson(sol::CELULAS, &geojson)?;
        // Células de 1° com os cantos nos graus inteiros, como as do POWER.
        host::run(
            "raster.rasterize",
            json!({"layer": sol::CELULAS, "field": sol::CAMPO, "nodata": -999, "crs": 4326, "cellSize": 1.0, "origin": [0.0, 0.0], "name": sol::RASTER_1}),
        )?;
        host::run(
            "raster.project",
            json!({"layer": sol::RASTER_1, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "bilinear", "name": sol::NA_GRADE}),
        )?;
        recortar(sol::NA_GRADE, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 15.0}))?;
        let (rampa, cores) = sol::RAMPA;
        // A rampa já registrada (de um clique anterior) fica como está.
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlOrRd"}))?;
        }
        let (a, b) = sol::ESCALA;
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} kWh/m² por dia", formatar_decimal(m)))
        .unwrap_or_default();
    avisar(&format!(
        "{saida}: {} de 2001 a 2025 (NASA POWER, {n} células de 1° em volta da área), interpolada para a grade do projeto, {}{media}",
        sol::periodo(mes),
        g.texto()
    ));
    Ok(json!({"layer": saida, "cells": n, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// O vento a 50 m do NASA POWER na área ([`vento`]): as células do MERRA-2 em volta dela viram um
/// raster, interpolado para a grade do projeto e recortado pela área.
fn vento_na_area(req: &Request) -> Result<Value, String> {
    let mes = sol::conferir_mes(req.f64("mes"))?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_comando("raster.rasterize")? {
        return Err("o vento a 50 m precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0): as células do NASA POWER viram um raster com ele".into());
    }
    let respostas = match buscar_bytes(&vento::pedidos(c))? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let textos: Vec<&str> = respostas.iter().map(|r| std::str::from_utf8(r).unwrap_or_default()).collect();
    if let Some(e) = textos.iter().find_map(|t| sol::erro(t)) {
        return Err(e);
    }
    let merra2 = textos.iter().all(|t| vento::do_merra2(t));
    let (geojson, n) = vento::celulas(&textos, mes);
    if n == 0 {
        return Err("o NASA POWER não tem o vento na área".into());
    }
    let g = grade_para(c, celula)?;
    let saida = vento::camada(mes);
    let temporarias = [vento::CELULAS, vento::RASTER, vento::NA_GRADE];
    remover(&temporarias)?;
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        por_geojson(vento::CELULAS, &geojson)?;
        host::run(
            "raster.rasterize",
            json!({"layer": vento::CELULAS, "field": vento::CAMPO, "nodata": -999, "crs": 4326, "cellSize": [vento::LADO.0, vento::LADO.1],
                "origin": [vento::CANTO.0, vento::CANTO.1], "name": vento::RASTER}),
        )?;
        host::run(
            "raster.project",
            json!({"layer": vento::RASTER, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "bilinear", "name": vento::NA_GRADE}),
        )?;
        recortar(vento::NA_GRADE, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 60.0}))?;
        host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": vento::RAMPA}))?;
        let (a, b) = vento::ESCALA;
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    sob_as_linhas(&saida)?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} m/s", formatar_decimal(m)))
        .unwrap_or_default();
    let fonte = if merra2 { "NASA POWER, da reanálise MERRA-2" } else { "NASA POWER; a resposta não diz que é do MERRA-2" };
    let mut texto = format!(
        "{}: a velocidade {} de 2001 a 2025 ({fonte}), de {n} células de 0,625° × 0,5° em volta da área, interpolada para a grade do projeto, {}{media}",
        saida.trim_start_matches(PREFIXO),
        vento::periodo(mes),
        g.texto()
    );
    avisar(&texto);
    let cabem = vento::celulas_na_area(c);
    if cabem < vento::MIN_CELULAS {
        let aviso = format!(
            "Vento a 50 m: no retângulo da área cabem {} células do dado (cerca de 55 × 65 km cada): o mapa mostra a interpolação entre elas, não o vento de cada lugar. Use o valor como regional",
            decimal(cabem, 1)
        );
        avisar(&aviso);
        texto = format!("{texto}\n\n{aviso}");
    }
    Ok(json!({"layer": saida, "cells": n, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "report": texto}))
}

/// Uma lista do CMR ([`emit::url`]), página a página até a que não vem cheia: as respostas cruas, ou o
/// pedido pendente (app web).
fn lista_do_emit(formato: &str, versao: u32, c: &[[f64; 4]], nuvem: u32, ano: Option<u32>) -> Result<Result<Vec<String>, Value>, String> {
    let mut paginas = Vec::new();
    for p in 1..=emit::MAX_PAGINAS {
        let r = match buscar_bytes(&[emit::url(formato, versao, c, nuvem, ano, p)]).map_err(|e| {
            if crate::falha_passageira(&e) || e.contains("timeout") {
                format!("o catálogo da NASA (CMR) não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })? {
            Ok(mut r) => r.remove(0),
            Err(p) => return Ok(Err(p)),
        };
        let t = String::from_utf8(r).unwrap_or_default();
        if !emit::eh_lista(formato, &t) {
            return Err("o catálogo da NASA (CMR) não devolveu a lista das cenas: tente de novo daqui a pouco".into());
        }
        let cheia = emit::cheia(formato, &t);
        paginas.push(t);
        if !cheia {
            return Ok(Ok(paginas));
        }
    }
    Err(format!(
        "há mais de {} cenas do EMIT em volta da área: escolha uma área menor, um ano ou menos nuvem",
        emit::MAX_PAGINAS * emit::POR_PAGINA
    ))
}

/// As cenas do EMIT ([`emit`]) que cruzam a área (sem área, o retângulo da vista), inteiras, na cor da
/// nuvem de cada uma.
fn cenas_do_emit(req: &Request) -> Result<Value, String> {
    let nuvem = req.f64("nuvem").map(emit::conferir_nuvem).transpose()?.unwrap_or(emit::NUVEM);
    let ano = req.f64("ano").map(emit::conferir_ano).transpose()?;
    let (c, area) = retangulo()?;
    let nome = emit::camada(nuvem, ano);
    let curto = nome.trim_start_matches(PREFIXO).to_string();
    // Uma área grande vai em faixas, pelo contorno simplificado dela.
    let caixas = if area && (c[2] - c[0]) * (c[3] - c[1]) > emit::GRANDE {
        let km = tolerancia_dos_nomes(c);
        let perto_do_polo = c[1].abs().max(c[3].abs()).to_radians().cos().max(0.2);
        emit::faixas(&area_simplificada(c)?, c, km / (110.0 * perto_do_polo))
    } else {
        vec![c]
    };
    // As listas: primeiro os CSV, com o limite de nuvem, que dizem se são cenas demais; depois os
    // contornos, os da V002 sem o limite (o que ela tem, para a V001 só completar).
    let mut listas = Vec::new();
    let mut bytes = 0;
    for (formato, versao, limite) in [("csv", 2, nuvem), ("csv", 1, nuvem), ("kml", 2, 100), ("kml", 1, nuvem)] {
        let l = match lista_do_emit(formato, versao, &caixas, limite, ano)? {
            Ok(l) => l,
            Err(p) => return Ok(p),
        };
        if formato == "csv" {
            bytes += l.iter().map(String::len).sum::<usize>();
            if bytes > emit::MAX_BYTES {
                return Err(format!(
                    "há cenas demais do EMIT em volta da área (o limite é de umas {} mil, somadas as duas versões): escolha uma área menor, um ano ou menos nuvem",
                    emit::MAX_BYTES / emit::BYTES_POR_MIL
                ));
            }
        }
        listas.push(l);
    }
    let textos: Vec<Vec<&str>> = listas.iter().map(|l| l.iter().map(String::as_str).collect()).collect();
    let v2 = emit::Listas { csv: textos[0].clone(), kml: textos[2].clone() };
    let v1 = emit::Listas { csv: textos[1].clone(), kml: textos[3].clone() };
    let lista = emit::cenas(&v2, &v1);
    let onde = if area { "cruza a área" } else { "cruza a vista" };
    let (geojson, lidas) = emit::geojson(&lista);
    remover(&[&nome, emit::NO_RETANGULO])?;
    if lidas == 0 {
        let t = format!("{curto}: nenhuma cena {onde}{}", if nuvem < 100 { "; aceite mais nuvem" } else { "" });
        avisar(&t);
        return Ok(json!({"layer": null, "features": 0, "report": t}));
    }
    let ficaram = if area {
        por_geojson(emit::NO_RETANGULO, &geojson)?;
        let n = (|| -> Result<u64, String> {
            host::run("gp.run", json!({"tool": "selectByLocation", "params": {"input": emit::NO_RETANGULO, "select": AREA}}))?;
            let n = camada_no_projeto(emit::NO_RETANGULO)?.and_then(|l| l["selected"].as_u64()).unwrap_or(0);
            if n > 0 {
                let r = host::run("select.layerFromSelection", json!({"layer": emit::NO_RETANGULO}))?;
                host::run("layer.rename", json!({"layer": r.get("layer").cloned().ok_or("a seleção não virou camada")?, "name": nome}))?;
            }
            Ok(n)
        })();
        remover(&[emit::NO_RETANGULO])?;
        n?
    } else {
        por_geojson(&nome, &geojson)?;
        lidas as u64
    };
    if ficaram == 0 {
        let t = format!("{curto}: nenhuma cena {onde} ({lidas} em volta dela)");
        avisar(&t);
        return Ok(json!({"layer": null, "read": lidas, "features": 0, "report": t}));
    }
    estilo_padrao(&nome, emit::ESTILO, &emit::estilo(nuvem));
    sob_as_linhas(&nome)?;
    // Quantas há em cada classe, e as mais limpas se são poucas.
    let mut contagem = Vec::new();
    for (_, classe, _) in emit::classes(nuvem) {
        let onde = format!("{} = '{classe}'", emit::CAMPO);
        let n = host::run("select.byAttributes", json!({"layer": nome, "where": onde}))?["selected"].as_u64().unwrap_or(0);
        if n > 0 {
            contagem.push(format!("{n} com {classe}"));
        }
    }
    let _ = host::run("select.clear", json!({"layer": nome}));
    let limpas = if ficaram <= emit::PARA_LER { emit::as_mais_limpas(&host::features(&nome, false)?) } else { Vec::new() };
    let melhores = if limpas.is_empty() { String::new() } else { format!(". As mais limpas: {}", limpas.join("; ")) };
    let quantas = match (ficaram, area) {
        (1, true) => "cena cruza a área",
        (1, false) => "cena cruza a vista",
        (_, true) => "cenas cruzam a área",
        (_, false) => "cenas cruzam a vista",
    };
    let texto = format!(
        "{curto}: {ficaram} {quantas} ({}){melhores}. A nuvem é da cena inteira, não da área; o campo arquivo leva ao netCDF (login Earthdata) e o previa, à imagem de prévia",
        contagem.join(", ")
    );
    avisar(&texto);
    Ok(json!({"layer": nome, "read": lidas, "features": ficaram, "report": texto}))
}

/// A umidade do solo nas raízes ou a evapotranspiração do NASA POWER na área ([`power`]): as células do
/// MERRA-2 em volta dela viram um raster, interpolado para a grade do projeto e recortado pela área.
fn power_na_area(req: &Request, v: power::Variavel) -> Result<Value, String> {
    let e = power::conferir(req.str("medida"), req.f64("ano"), req.f64("mes"))?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_comando("raster.rasterize")? {
        return Err(format!(
            "{} precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0): as células do NASA POWER viram um raster com ele",
            v.por_extenso()
        ));
    }
    let (do_ano, da_media) = power::pedidos(c, v, e);
    let quantos = do_ano.len();
    let respostas = match buscar_bytes(&[do_ano, da_media].concat())? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let textos: Vec<&str> = respostas.iter().map(|r| std::str::from_utf8(r).unwrap_or_default()).collect();
    if let Some(erro) = textos.iter().find_map(|t| sol::erro(t)) {
        return Err(erro);
    }
    let (geojson, n) = power::celulas(&textos[..quantos], &textos[quantos..], v, e);
    if n == 0 {
        return Err(power::sem_dado(v, e));
    }
    let g = grade_para(c, celula)?;
    let saida = e.camada(v);
    let temporarias = [power::CELULAS, power::RASTER, power::NA_GRADE];
    remover(&temporarias)?;
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        por_geojson(power::CELULAS, &geojson)?;
        host::run(
            "raster.rasterize",
            json!({"layer": power::CELULAS, "field": power::CAMPO, "nodata": power::SEM_DADO, "crs": 4326, "cellSize": [vento::LADO.0, vento::LADO.1],
                "origin": [vento::CANTO.0, vento::CANTO.1], "name": power::RASTER}),
        )?;
        host::run(
            "raster.project",
            json!({"layer": power::RASTER, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "bilinear", "name": power::NA_GRADE}),
        )?;
        recortar(power::NA_GRADE, AREA, &saida)?;
        let (min, max) = v.aceitos(e.anomalia());
        host::run("raster.mask", json!({"layer": saida, "min": min, "max": max}))?;
        let (rampa, cores) = v.rampa(e.anomalia());
        let reserva = if v == power::Variavel::Evapotranspiracao && !e.anomalia() { "YlGn" } else { "BrBG" };
        pintar(&saida, &Pintura { rampa: (rampa, &cores), reserva, escala: v.escala(e.anomalia()), aceitos: (min, max) });
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    sob_as_linhas(&saida)?;
    let media = host::run("raster.info", json!({"layer": saida})).ok().and_then(|i| i["bands"][0]["mean"].as_f64());
    let na_media = media.map(|m| format!("; média na área: {} {}", decimal(m, v.casas()), v.unidade())).unwrap_or_default();
    let total = match (v, media) {
        (power::Variavel::Evapotranspiracao, Some(m)) => {
            format!(", ou {} mm no {}", decimal(m * e.dias(), 0), if e.mes.is_some() { "mês" } else { "ano" })
        }
        _ => String::new(),
    };
    let o_que = match e.medida {
        power::Medida::Media => format!("{}, {} de 2001 a 2025", v.por_extenso(), e.periodo()),
        power::Medida::Valor => format!("{}, em {}", v.por_extenso(), e.periodo()),
        power::Medida::Anomalia => format!("{}, em {}, menos a média de 2001 a 2025", v.por_extenso(), e.periodo()),
    };
    let mut texto = format!(
        "{}: {o_que} (NASA POWER, da reanálise MERRA-2), de {n} células de 0,625° × 0,5° em volta da área, interpolada para a grade do projeto, {}{na_media}{total}",
        saida.trim_start_matches(PREFIXO),
        g.texto()
    );
    avisar(&texto);
    let cabem = vento::celulas_na_area(c);
    if cabem < vento::MIN_CELULAS {
        let aviso = format!(
            "{}: no retângulo da área cabem {} células do dado (cerca de 55 × 65 km cada): o mapa mostra a interpolação entre elas, não o valor de cada lugar. Use o valor como regional",
            v.nome(e.anomalia()),
            decimal(cabem, 1)
        );
        avisar(&aviso);
        texto = format!("{texto}\n\n{aviso}");
    }
    Ok(json!({"layer": saida, "cells": n, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "mean": media, "report": texto}))
}

/// O retângulo da área e a célula pedida, para a previsão do Open-Meteo, com as conferências
/// comuns aos três comandos.
fn preparar_previsao(req: &Request) -> Result<([f64; 4], Option<f64>), String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_comando("raster.rasterize")? {
        return Err("a previsão do tempo precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0): os pontos do Open-Meteo viram um raster com ele".into());
    }
    Ok((c, celula))
}

/// Os lugares das respostas do Open-Meteo, um por ponto, na ordem dos pontos; ou, para o comando
/// devolver, o pedido pendente (app web) ou a resposta incompleta.
///
/// No desktop, cada pedido espera até 20 s e todos os de um clique até 60 s. Um grupo de pontos
/// que passa disso fica de fora, os outros seguem, e o comando avisa em vez de falhar (um script
/// segue adiante): o RustGIS guarda por 10 minutos as respostas que chegaram e por 30 s os
/// erros, e o clique seguinte, depois desses 30 s, só pede o que faltou.
fn buscar_previsao(urls: &[String], pontos: usize) -> Result<Result<Vec<Value>, Value>, String> {
    let respostas = match buscar_open_meteo(urls, true)? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let respostas: Vec<Value> = respostas
        .into_iter()
        .map(|mut r| r.get_mut("json").map(Value::take).ok_or("o Open-Meteo não respondeu em JSON".to_string()))
        .collect::<Result<_, _>>()?;
    if let Some(e) = respostas.iter().find_map(tempo::erro) {
        return Err(e);
    }
    // Um pedido de vários pontos volta como lista; de um só, como objeto. Os lugares são tirados
    // das respostas sem copiar (copiar 51 membros gasta o orçamento de processamento do clique).
    let mut lugares = Vec::with_capacity(pontos);
    for r in respostas {
        match r {
            Value::Array(a) => lugares.extend(a),
            outro => lugares.push(outro),
        }
    }
    if lugares.len() != pontos {
        return Err(format!("o Open-Meteo devolveu {} lugares para {pontos} pontos", lugares.len()));
    }
    Ok(Ok(lugares))
}

/// As respostas do `host.fetch` aos endereços do Open-Meteo, na ordem deles (com o `json` ou, com
/// `com_json` falso, só o `text`, que o RustGIS 0.7.0 deixa de ler como JSON); ou, para o comando
/// devolver, o pedido pendente (app web) ou a resposta incompleta (ver [`buscar_previsao`]).
fn buscar_open_meteo(urls: &[String], com_json: bool) -> Result<Result<Vec<Value>, Value>, String> {
    let mut respostas = Vec::new();
    let (mut pendente, mut esperando, mut faltam, mut motivo) = (None, 0, 0, String::new());
    for url in urls {
        let pedido = if com_json { json!({"url": url}) } else { json!({"url": url, "json": false}) };
        match host::call("host.fetch", pedido) {
            Ok(r) if r.get("pending").and_then(Value::as_bool) == Some(true) => {
                pendente = Some(r);
                esperando += 1;
                if esperando == 8 {
                    break;
                }
            }
            Ok(r) => respostas.push(r),
            Err(e) if e.contains("429") => {
                return Err("o Open-Meteo recusou por excesso de pedidos: espere um minuto, ou use uma área menor".into());
            }
            Err(e) if e.contains("timeout") || e.contains("timed out") => {
                faltam += 1;
                motivo = e;
            }
            Err(e) => return Err(e),
        }
    }
    if let Some(p) = pendente {
        return Ok(Err(p));
    }
    if faltam > 0 {
        avisar(&format!(
            "Resposta do Open-Meteo incompleta: o servidor demorou mais do que o RustGIS espera (20 s por pedido, 60 s por clique), e chegaram {} de {} grupos de pontos, guardados por 10 minutos. Clique de novo daqui a 30 segundos para buscar os {faltam} que faltam ({motivo})",
            urls.len() - faltam,
            urls.len()
        ));
        return Ok(Err(json!({"incomplete": faltam, "of": urls.len()})));
    }
    Ok(Ok(respostas))
}

/// O primeiro e o último dia da previsão, da primeira resposta que os tiver.
fn periodo_da_previsao(lugares: &[Value], dias: usize) -> Result<(String, String), String> {
    lugares.iter().find_map(|l| tempo::periodo(l, dias)).ok_or_else(|| "o Open-Meteo não devolveu as datas da previsão".to_string())
}

/// Como uma previsão vai para o mapa: a rampa, a escala das cores e os valores aceitos.
struct Pintura<'a> {
    rampa: (&'a str, &'a str),
    reserva: &'a str,
    escala: (f64, f64),
    aceitos: (f64, f64),
}

/// As células da previsão viram um raster de `passo` graus, interpolado (bilinear) para a grade do
/// projeto, recortado pela área e pintado.
fn previsao_para_grade(c: [f64; 4], celula: Option<f64>, fc: Value, passo: f64, saida: &str, p: &Pintura) -> Result<Grade, String> {
    let g = grade_para(c, celula)?;
    let temporarias = [tempo::CELULAS, tempo::RASTER, tempo::NA_GRADE];
    remover(&temporarias)?;
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        host::add_layer(tempo::CELULAS, fc)?;
        // As células têm o centro nos nós da grade de pontos (múltiplos do passo).
        host::run(
            "raster.rasterize",
            json!({"layer": tempo::CELULAS, "field": tempo::CAMPO, "nodata": -9999, "crs": 4326, "cellSize": passo, "origin": [passo / 2.0, passo / 2.0], "name": tempo::RASTER}),
        )?;
        host::run(
            "raster.project",
            json!({"layer": tempo::RASTER, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "bilinear", "name": tempo::NA_GRADE}),
        )?;
        recortar(tempo::NA_GRADE, AREA, saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": p.aceitos.0, "max": p.aceitos.1}))?;
        pintar(saida, p);
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    Ok(g)
}

/// Pinta o raster `saida` com a rampa da [`Pintura`] (ou a reserva do RustGIS) na escala dela.
fn pintar(saida: &str, p: &Pintura) {
    // A rampa já registrada (de um clique anterior) fica como está.
    let _ = host::run("ramp.add", json!({"text": p.rampa.1, "format": "gdal", "name": p.rampa.0}));
    if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": p.rampa.0})).is_err() {
        let _ = host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": p.reserva}));
    }
    let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": p.escala.0, "max": p.escala.1}));
}

/// A média do raster `nome` na área, para o aviso (sem a unidade, se ela é vazia).
fn media_na_area(nome: &str, unidade: &str) -> String {
    let u = if unidade.is_empty() { String::new() } else { format!(" {unidade}") };
    host::run("raster.info", json!({"layer": nome}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}{u}", formatar_decimal(m)))
        .unwrap_or_default()
}

/// O passo da grade de pontos, para o aviso ("0,25°").
fn texto_do_passo(passo: f64) -> String {
    format!("{}°", format!("{passo}").replace('.', ","))
}

/// O aviso de uma previsão com menos dias que os pedidos: o modelo não vai tão longe, ou a
/// rodada tem o último dia incompleto.
fn aviso_de_corte(m: &tempo::Modelo, pedidos: usize, dias: usize) -> String {
    if dias < pedidos { format!("; pedidos {pedidos} dias, o {} tem {dias} completos", m.nome) } else { String::new() }
}

/// Os dias pedidos (já conferidos por [`tempo::dias`]), para o aviso.
fn dias_pedidos(req: &Request, padrao: usize) -> usize {
    req.f64("dias").map_or(padrao, |d| d as usize)
}

/// Os dias completos da previsão, ou o erro de nenhum.
fn dias_da_resposta(lugares: &[Value], variavel: &str, dias: usize) -> Result<usize, String> {
    match tempo::dias_completos(lugares, variavel, dias) {
        0 => Err("o Open-Meteo não tem a previsão na área".into()),
        d => Ok(d),
    }
}

/// O número de células com valor, ou o erro de nenhuma.
fn celulas_com_valor(fc: &Value) -> Result<usize, String> {
    match fc["features"].as_array().map_or(0, Vec::len) {
        0 => Err("o Open-Meteo não tem a previsão na área".into()),
        n => Ok(n),
    }
}

/// O limiar escrito para o aviso, sem ",0".
fn texto_do_limiar(limiar: f64) -> String {
    let t = formatar_decimal(limiar);
    t.strip_suffix(",0").map(str::to_string).unwrap_or(t)
}

/// A chuva prevista de um modelo determinístico ([`tempo`]): a soma dos dias pedidos nos nós em
/// volta da área, interpolada para a grade do projeto e recortada pela área.
fn chuva_prevista(req: &Request) -> Result<Value, String> {
    let m = tempo::modelo(&tempo::DETERMINISTICOS, req.str("modelo"))?;
    let (pedidos, _) = tempo::dias(req.f64("dias"), tempo::DIAS_CHUVA, m)?;
    let (c, celula) = preparar_previsao(req)?;
    let (passo, pts) = tempo::pontos(c, m.passo, tempo::MAX_PONTOS);
    let lugares = match buscar_previsao(&tempo::pedidos(m, "precipitation_sum", pedidos, &pts), pts.len())? {
        Ok(l) => l,
        Err(p) => return Ok(p),
    };
    let dias = dias_da_resposta(&lugares, "precipitation_sum", pedidos)?;
    let valores: Vec<Option<f64>> = lugares.iter().map(|l| tempo::soma(l, "precipitation_sum", dias)).collect();
    let (de, ate) = periodo_da_previsao(&lugares, dias)?;
    let fc = tempo::celulas(&pts, &valores, passo);
    let n = celulas_com_valor(&fc)?;
    let saida = tempo::camada_chuva(m, dias, &de, &ate);
    let max = tempo::escala_chuva(dias);
    let (rn, rt) = tempo::rampa_chuva(max);
    let pintura = Pintura { rampa: (&rn, &rt), reserva: "YlGnBu", escala: (0.0, max), aceitos: (0.0, 5000.0) };
    let g = previsao_para_grade(c, celula, fc, passo, &saida, &pintura)?;
    avisar(&format!(
        "{saida}: a chuva somada de {de} a {ate} ({dias} dias a partir de hoje), do {} (Open-Meteo), em {n} pontos a cada {}, interpolada para a grade do projeto, {}{}{}",
        m.nome,
        texto_do_passo(passo),
        g.texto(),
        media_na_area(&saida, "mm"),
        aviso_de_corte(m, dias_pedidos(req, tempo::DIAS_CHUVA), dias)
    ));
    Ok(json!({"layer": saida, "points": n, "step": passo, "days": dias, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// A chuva de um ensemble ([`tempo`]): a chance de passar de um limiar, a média dos membros ou um
/// percentil, nos nós em volta da área, interpolada para a grade do projeto e recortada pela área.
fn chuva_do_ensemble(req: &Request) -> Result<Value, String> {
    let m = tempo::modelo(&tempo::ENSEMBLES, req.str("modelo"))?;
    let medida = tempo::medida(req.str("medida"))?;
    let limiar = tempo::limiar(req.f64("limiar"))?;
    let (pedidos, _) = tempo::dias(req.f64("dias"), tempo::DIAS_CHUVA, m)?;
    let (c, celula) = preparar_previsao(req)?;
    let (passo, pts) = tempo::pontos(c, m.passo, tempo::max_pontos_ensemble(m));
    let lugares = match buscar_previsao(&tempo::pedidos_ensemble(m, pedidos, &pts), pts.len())? {
        Ok(l) => l,
        Err(p) => return Ok(p),
    };
    let dias = dias_da_resposta(&lugares, "precipitation_sum", pedidos)?;
    let membros: Vec<Vec<f64>> = lugares.iter().map(|l| tempo::membros(l, dias)).collect();
    let n_membros = membros.iter().map(Vec::len).max().unwrap_or(0);
    let valores: Vec<Option<f64>> = membros.iter().map(|v| tempo::resumir(v, medida, limiar)).collect();
    let (de, ate) = periodo_da_previsao(&lugares, dias)?;
    let fc = tempo::celulas(&pts, &valores, passo);
    let n = celulas_com_valor(&fc)?;
    let saida = tempo::camada_ensemble(m, medida, limiar, dias, &de, &ate);
    let max = tempo::escala_chuva(dias);
    let (rn, rt) = tempo::rampa_chuva(max);
    let chuva = Pintura { rampa: (&rn, &rt), reserva: "YlGnBu", escala: (0.0, max), aceitos: (0.0, 5000.0) };
    let (pintura, unidade, o_que) = match medida {
        tempo::Medida::Chance => (
            Pintura { rampa: tempo::RAMPA_CHANCE, reserva: "YlOrRd", escala: (0.0, 100.0), aceitos: (0.0, 100.0) },
            "%",
            format!("a chance, entre os {n_membros} membros, de a chuva de {de} a {ate} passar de {} mm", texto_do_limiar(limiar)),
        ),
        tempo::Medida::Media => (chuva, "mm", format!("a média dos {n_membros} membros da chuva de {de} a {ate}")),
        tempo::Medida::P10 => {
            (chuva, "mm", format!("o cenário seco (9 em cada 10 dos {n_membros} membros dão mais chuva que isso) de {de} a {ate}"))
        }
        tempo::Medida::P90 => {
            (chuva, "mm", format!("o cenário chuvoso (só 1 em cada 10 dos {n_membros} membros dá mais chuva que isso) de {de} a {ate}"))
        }
    };
    let g = previsao_para_grade(c, celula, fc, passo, &saida, &pintura)?;
    avisar(&format!(
        "{saida}: {o_que}, do {} (Open-Meteo), em {n} pontos a cada {}, interpolada para a grade do projeto, {}{}{}",
        m.nome,
        texto_do_passo(passo),
        g.texto(),
        media_na_area(&saida, unidade),
        aviso_de_corte(m, dias_pedidos(req, tempo::DIAS_CHUVA), dias)
    ));
    Ok(
        json!({"layer": saida, "points": n, "members": n_membros, "step": passo, "days": dias, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}),
    )
}

/// A temperatura prevista ([`tempo`]): a média, nos dias pedidos, da máxima, da mínima ou da média
/// do dia, nos nós em volta da área, interpolada para a grade do projeto e recortada pela área.
fn temperatura_prevista(req: &Request) -> Result<Value, String> {
    let m = tempo::modelo(&tempo::DETERMINISTICOS, req.str("modelo"))?;
    let t = tempo::temperatura(req.str("tipo"))?;
    let (pedidos, _) = tempo::dias(req.f64("dias"), tempo::DIAS_TEMPERATURA, m)?;
    let (c, celula) = preparar_previsao(req)?;
    let (passo, pts) = tempo::pontos(c, m.passo, tempo::MAX_PONTOS);
    let lugares = match buscar_previsao(&tempo::pedidos(m, t.variavel(), pedidos, &pts), pts.len())? {
        Ok(l) => l,
        Err(p) => return Ok(p),
    };
    let dias = dias_da_resposta(&lugares, t.variavel(), pedidos)?;
    let valores: Vec<Option<f64>> = lugares.iter().map(|l| tempo::media(l, t.variavel(), dias)).collect();
    let (de, ate) = periodo_da_previsao(&lugares, dias)?;
    let fc = tempo::celulas(&pts, &valores, passo);
    let n = celulas_com_valor(&fc)?;
    let saida = tempo::camada_temperatura(t, m, dias, &de, &ate);
    let pintura = Pintura { rampa: tempo::RAMPA_TEMPERATURA, reserva: "RdYlBu", escala: tempo::ESCALA_TEMPERATURA, aceitos: (-60.0, 60.0) };
    let g = previsao_para_grade(c, celula, fc, passo, &saida, &pintura)?;
    avisar(&format!(
        "{saida}: a temperatura {} do dia a 2 m, em média de {de} a {ate}, do {} (Open-Meteo, ajustada à altitude de cada ponto), em {n} pontos a cada {}, interpolada para a grade do projeto, {}{}{}",
        t.nome(),
        m.nome,
        texto_do_passo(passo),
        g.texto(),
        media_na_area(&saida, "°C"),
        aviso_de_corte(m, dias_pedidos(req, tempo::DIAS_TEMPERATURA), dias)
    ));
    Ok(json!({"layer": saida, "points": n, "step": passo, "days": dias, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// Um dado do ERA5 na área ([`era5`]), resumido num período: os valores diários da API histórica
/// do Open-Meteo nos nós de 0,25° em volta da área (mais abertos num período longo, pelo custo),
/// interpolados para a grade do projeto e recortados pela área. Sem período, o último ano completo.
fn era5_na_area(req: &Request, v: era5::Variavel) -> Result<Value, String> {
    let r = era5::resumo(req.str("resumo"))?;
    let (c, celula) = preparar_previsao(req)?;
    let data = |k: &str| -> Result<Option<Data>, String> {
        req.str(k).map(|t| bdc::ler_data(t).ok_or(format!("a data '{t}' não é dd/mm/aaaa"))).transpose()
    };
    let (inicio, fim) = match (data("inicio")?, data("fim")?) {
        (Some(a), Some(b)) => (a, b),
        (a, b) => {
            // O plugin não tem relógio: o dia de hoje vem de um pedido mínimo à previsão.
            let resposta = match buscar_previsao(&[era5::HOJE.to_string()], 1)? {
                Ok(l) => l,
                Err(p) => return Ok(p),
            };
            let hoje = resposta.first().and_then(era5::hoje).ok_or("o Open-Meteo não disse o dia de hoje: diga o início e o fim")?;
            let (i, f) = era5::periodo_padrao(hoje);
            (a.unwrap_or(i), b.unwrap_or(f))
        }
    };
    era5::conferir_periodo(inicio, fim)?;
    let dias = era5::dias(inicio, fim);
    let (passo, pts) = tempo::pontos(c, era5::PASSO, era5::max_pontos(dias));
    if passo > era5::PASSO_MAX {
        return Err(era5::longo_demais(dias, tempo::pontos(c, era5::PASSO_MAX, usize::MAX).1.len()));
    }
    let respostas = match buscar_open_meteo(&era5::pedidos(v, inicio, fim, &pts), false)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let mut series = Vec::with_capacity(pts.len());
    for r in &respostas {
        let texto = r["text"].as_str().unwrap_or_default();
        if let Some(e) = era5::erro_do_texto(texto) {
            return Err(e);
        }
        series.extend(era5::series(texto, v));
    }
    drop(respostas);
    if series.len() != pts.len() {
        return Err(format!("o Open-Meteo devolveu {} lugares para {} pontos", series.len(), pts.len()));
    }
    let sem_dados = || format!("o Open-Meteo não tem o ERA5 de {} a {} na área", inicio.texto(), fim.texto());
    let com_dados = era5::dias_com_dados(&series, dias);
    if com_dados == 0 {
        return Err(sem_dados());
    }
    if com_dados < dias {
        let ultimo = era5::mais_dias(inicio, com_dados - 1);
        return Err(format!("o ERA5 do Open-Meteo vai até {} (sai com uns 5 dias de atraso): use um fim até esse dia", ultimo.texto()));
    }
    let valores: Vec<Option<f64>> = series.iter().map(|s| era5::valor(s, v, r, dias)).collect();
    let fc = tempo::celulas(&pts, &valores, passo);
    let n = match fc["features"].as_array().map_or(0, Vec::len) {
        0 => return Err(sem_dados()),
        n => n,
    };
    let saida = era5::camada(v, r, inicio, fim);
    let unidade = v.unidade(r);
    let escala = era5::escala(v, r, dias);
    let rampa_chuva = tempo::rampa_chuva(escala.1);
    let pintura = match v {
        era5::Variavel::Chuva => Pintura { rampa: (&rampa_chuva.0, &rampa_chuva.1), reserva: "YlGnBu", escala, aceitos: (0.0, 200_000.0) },
        era5::Variavel::Temperatura(_) => Pintura { rampa: tempo::RAMPA_TEMPERATURA, reserva: "RdYlBu", escala, aceitos: (-60.0, 60.0) },
        era5::Variavel::Umidade => Pintura { rampa: era5::RAMPA_UMIDADE, reserva: "BrBG", escala, aceitos: (0.0, 100.0) },
    };
    let g = previsao_para_grade(c, celula, fc, passo, &saida, &pintura)?;
    let o_que = match (v, r) {
        (era5::Variavel::Chuva, era5::Resumo::Total) => "a chuva somada".to_string(),
        (era5::Variavel::Chuva, era5::Resumo::PorAno) => "a chuva média por ano".to_string(),
        (era5::Variavel::Temperatura(tempo::Temperatura::Media), _) => "a temperatura média do ar a 2 m".to_string(),
        (era5::Variavel::Temperatura(t), _) => format!("a temperatura {} do dia a 2 m, em média", t.nome()),
        (era5::Variavel::Umidade, _) => "a umidade média dos primeiros 7 cm do solo".to_string(),
    };
    let aberta = if passo > era5::PASSO {
        format!(
            " (a grade de pontos abriu de 0,25° para {} para caber no limite do Open-Meteo, que pesa os pontos pelos dias)",
            texto_do_passo(passo)
        )
    } else {
        String::new()
    };
    avisar(&format!(
        "{}: {o_que} de {} a {} ({dias} dias), da reanálise ERA5 (ECMWF/Copernicus, pelo Open-Meteo), em {n} pontos a cada {}{aberta}, interpolada para a grade do projeto, {}{}",
        saida.trim_start_matches(era5::PREFIXO),
        inicio.texto(),
        fim.texto(),
        texto_do_passo(passo),
        g.texto(),
        media_na_area(&saida, unidade)
    ));
    Ok(json!({
        "layer": saida, "points": n, "step": passo, "days": dias, "start": inicio.iso(), "end": fim.iso(), "unit": unidade,
        "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// O mês do CHIRPS: o pedido ("mm/aaaa") ou o mais recente da pasta do Climate Hazards Center.
fn mes_do_chirps(req: &Request) -> Result<Result<(i64, u32), Value>, String> {
    if let Some(t) = req.str("mes") {
        return tempo::ler_mes(t).map(Ok).ok_or_else(|| format!("o mês é mm/aaaa (por exemplo 10/2026), não '{t}'"));
    }
    let r = match buscar(&[tempo::PASTA_CHIRPS.to_string()]).map_err(|e| {
        if e.contains("429") {
            "o servidor do CHIRPS (data.chc.ucsb.edu) recusou por excesso de pedidos: tente daqui a alguns minutos, ou diga o mês (mm/aaaa), que não precisa da lista da pasta".to_string()
        } else {
            e
        }
    })? {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    let texto = r.first().and_then(|r| r["text"].as_str()).unwrap_or_default();
    tempo::ultimo_mes(texto)
        .map(Ok)
        .ok_or_else(|| "não achei o último mês do CHIRPS na pasta do Climate Hazards Center: diga o mês (mm/aaaa)".into())
}

/// A chuva de um mês do CHIRPS v3 ([`tempo`]): a janela da área no COG mundial do mês, posta na
/// grade do projeto e recortada pela área.
fn chuva_do_mes(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "a chuva do CHIRPS precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): cada mês cobre o mundo a 5 km";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let (ano, mes) = match mes_do_chirps(req)? {
        Ok(m) => m,
        Err(p) => return Ok(p),
    };
    let janela = tempo::janela(c);
    let nome_da_janela = tempo::janela_chirps(ano, mes);
    let serve = extensao(&nome_da_janela)?.is_some_and(|e| retangulo_cabe(janela, e));
    if !serve {
        tirar_janela(&nome_da_janela)?;
        let r = host::run(
            "layer.addUrl",
            json!({"url": tempo::url_chirps(ano, mes), "name": nome_da_janela, "bbox": janela, "cog": true, "zoom": false}),
        )
        .map_err(|e| {
            if e.contains("404") {
                format!("o CHIRPS ainda não tem {mes:02}/{ano}: o total do mês sai na terceira semana do mês seguinte")
            } else if falha_passageira(&e) || e.contains("timeout") {
                format!("o servidor do CHIRPS (data.chc.ucsb.edu) não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!("Chuva de {mes:02}/{ano} (CHIRPS v3): lendo a área no arquivo do mês; a camada vem quando chegar"));
            return Ok(json!({"pending": 1}));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome_da_janela)?;
            return Err(sem_janela.into());
        }
    }
    let g = grade_para(c, celula)?;
    let saida = tempo::camada_chirps(ano, mes);
    let temporarias = [nome_da_janela.as_str(), tempo::NA_GRADE_CHIRPS];
    remover(&[&saida])?;
    let (rn, rt) = tempo::rampa_chuva(tempo::ESCALA_CHIRPS);
    let pintura = Pintura { rampa: (&rn, &rt), reserva: "YlGnBu", escala: (0.0, tempo::ESCALA_CHIRPS), aceitos: (0.0, 5000.0) };
    let feito = (|| -> Result<(), String> {
        // O mar e o sem dado (negativos) saem antes da grade, para não entrar na interpolação.
        host::run("raster.mask", json!({"layer": nome_da_janela, "min": 0.0, "max": 5000.0}))?;
        host::run(
            "raster.project",
            json!({"layer": nome_da_janela, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": tempo::reamostragem_chirps(g.celula), "name": tempo::NA_GRADE_CHIRPS}),
        )?;
        recortar(tempo::NA_GRADE_CHIRPS, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 5000.0}))?;
        pintar(&saida, &pintura);
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    avisar(&format!(
        "{saida}: o total do mês, por satélite e estações (CHIRPS v3, 0,05°), na grade do projeto, {}{}",
        g.texto(),
        media_na_area(&saida, "mm")
    ));
    Ok(json!({"layer": saida, "month": format!("{mes:02}/{ano}"), "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
}

/// A área equipada para irrigação do GMIA v5 na área ([`irrigacao`]): a janela da área no COG da
/// FAO, recortada pela área; posta na grade do projeto só quando a grade é tão grossa quanto o dado.
fn irrigacao_na_area(celula: Option<f64>) -> Result<Value, String> {
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "a área equipada para irrigação (GMIA v5) precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0)";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let janela = irrigacao::janela(c);
    // A janela de um clique anterior serve se cobre a área (no desktop, o comando roda de novo
    // quando ela chega).
    let serve = extensao(irrigacao::JANELA)?.is_some_and(|e| retangulo_cabe(janela, e));
    if !serve {
        tirar_janela(irrigacao::JANELA)?;
        let r = host::run("layer.addUrl", json!({"url": irrigacao::URL, "name": irrigacao::JANELA, "bbox": janela, "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar("Área equipada para irrigação (GMIA v5): lendo a área no arquivo da FAO; a camada vem quando chegar");
            return Ok(json!({"pending": 1}));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(irrigacao::JANELA)?;
            return Err(sem_janela.into());
        }
    }
    let g = grade_para(c, celula)?;
    // Numa grade mais fina que a célula de 5′ do GMIA, a camada fica na resolução do dado (fora da
    // grade do projeto), só recortada pela área, como os dados do JAXA: pôr 9 km numa grade de
    // 500 m repetiria cada célula centenas de vezes (uns 23 milhões de células no Brasil inteiro),
    // e o comando, que roda na linha da janela do RustGIS (RustGIS#1562), a travaria esse tempo.
    let nativa = g.celula < irrigacao::LADO_M / 1.5;
    let saida = irrigacao::CAMADA;
    let temporarias = [irrigacao::JANELA, irrigacao::NA_GRADE];
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        if nativa {
            recortar(irrigacao::JANELA, AREA, saida)?;
        } else {
            host::run(
                "raster.project",
                json!({"layer": irrigacao::JANELA, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": irrigacao::reamostragem(g.celula), "name": irrigacao::NA_GRADE}),
            )?;
            recortar(irrigacao::NA_GRADE, AREA, saida)?;
        }
        // O -9 do arquivo e o que não for porcentagem viram sem dado.
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 100.0}))?;
        let (rampa, cores) = irrigacao::RAMPA;
        // A rampa já registrada (de um clique anterior) fica como está.
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "GnBu"}))?;
        }
        let (a, b) = irrigacao::ESCALA;
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}%", formatar_decimal(m)))
        .unwrap_or_default();
    let onde = if nativa {
        format!("na resolução do dado (5′, em graus), fora da grade do projeto, que é mais fina ({})", g.texto())
    } else {
        format!("na grade do projeto, {}", g.texto())
    };
    avisar(&format!(
        "{saida}: % de cada célula de 5′ (cerca de 9 km) equipada para irrigação, por volta de 2005, {onde}; cores de 0 a 40% ou mais, com mais tons até 10%{media}"
    ));
    if nativa {
        Ok(json!({"layer": saida, "crs": "EPSG:4326", "cellSize": irrigacao::LADO, "native": true}))
    } else {
        Ok(json!({"layer": saida, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula}))
    }
}

/// Depois do tema das localidades ([`localidades`]): numa camada que acaba de chegar (de um símbolo
/// só), a cor de cada tipo, cada um a partir de uma escala da vista. O estilo que o plugin guardou
/// dela antes do comando (o símbolo só, quando o comando roda de novo porque a camada chegou) sai,
/// para não voltar no fim; uma camada que o usuário estilizou fica como está.
fn localidades_por_escala(t: &Tema) -> Result<(), String> {
    let nome = t.camada_no_mapa();
    let Some(l) = camada_no_projeto(&nome)? else { return Ok(()) };
    if l["renderer"].as_str() != Some("Single Symbol") {
        return Ok(());
    }
    let _ = host::run("style.remove", json!({"name": localidades::ESTILO}));
    host::run("style.add", json!({"text": localidades::estilo()}))?;
    host::run("style.apply", json!({"layer": nome, "name": localidades::ESTILO}))?;
    let _ = host::run("style.remove", json!({"name": estilo::guardado(&nome)}));
    avisar(&format!(
        "{}: as cidades aparecem em qualquer escala; as vilas e os núcleos urbanos, de 1:2.000.000 para perto; os aglomerados rurais, de 1:500.000 para perto",
        nome.trim_start_matches(PREFIXO)
    ));
    Ok(())
}

/// Depois do tema do Aqueduct ([`aqueduct`]): a categoria em português no campo
/// [`aqueduct::CAMPO`] e, numa camada que acaba de chegar (de um símbolo só), as cores do WRI. Uma
/// camada que o usuário já estilizou recebe o estilo dela de volta no fim do comando.
fn aqueduct_em_portugues(t: &Tema) -> Result<(), String> {
    let nome = t.camada_no_mapa();
    let Some(l) = camada_no_projeto(&nome)? else { return Ok(()) };
    let campos: Vec<&str> =
        l["fields"].as_array().into_iter().flatten().filter_map(Value::as_str).map(|f| f.split(" (").next().unwrap_or(f)).collect();
    if !campos.contains(&"bws_cat") {
        return Ok(());
    }
    if !campos.contains(&aqueduct::CAMPO) {
        host::run("edit.addField", json!({"layer": nome, "name": aqueduct::CAMPO, "type": "text"}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": aqueduct::CAMPO, "expression": aqueduct::expressao()}))?;
    }
    if l["renderer"].as_str() == Some("Single Symbol") {
        let _ = host::run("style.remove", json!({"name": aqueduct::ESTILO}));
        host::run("style.add", json!({"text": aqueduct::estilo()}))?;
        host::run("style.apply", json!({"layer": nome, "name": aqueduct::ESTILO}))?;
    }
    Ok(())
}

/// O mês do risco de fogo: o do script, o do controle do painel ou setembro.
fn mes_do_risco(req: &Request) -> Result<u8, String> {
    match req.f64("mes") {
        Some(m) => risco_fogo::conferir_mes(m),
        None => Ok(req
            .panel
            .get(risco_fogo::CONTROLE)
            .and_then(Value::as_u64)
            .and_then(|m| u8::try_from(m).ok())
            .filter(|m| (1..=12).contains(m))
            .unwrap_or(risco_fogo::MES_PADRAO)),
    }
}

/// O Risco de Fogo do INPE no mês `mes` ([`risco_fogo`]): a janela da área no arquivo do mês,
/// posta na grade do projeto e recortada pela área, nas cores das classes do método.
fn risco_de_fogo(req: &Request) -> Result<Value, String> {
    let mes = mes_do_risco(req)?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "o Risco de Fogo do INPE precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): cada mês cobre a América do Sul a 1 km";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let url = risco_fogo::url(mes);
    let janela = risco_fogo::janela(c);
    let nome_da_janela = risco_fogo::janela_do_mes(mes);
    // A janela de um clique anterior serve se cobre a área (no desktop, o comando roda de novo
    // quando ela chega); o nome dela diz o mês.
    let serve = extensao(&nome_da_janela)?.is_some_and(|e| retangulo_cabe(janela, e));
    if !serve {
        tirar_janela(&nome_da_janela)?;
        let r = host::run("layer.addUrl", json!({"url": url, "name": nome_da_janela, "bbox": janela, "zoom": false})).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o servidor do INPE (dataserver-coids.inpe.br) não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!(
                "Risco de fogo em {} (INPE): lendo a área no arquivo do INPE; a camada vem quando chegar",
                risco_fogo::nome_do_mes(mes)
            ));
            return Ok(json!({"pending": 1}));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome_da_janela)?;
            return Err(sem_janela.into());
        }
    }
    let g = grade_para(c, celula)?;
    let saida = risco_fogo::camada(mes);
    let temporarias = [nome_da_janela.as_str(), risco_fogo::NA_GRADE];
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        // O -999 do arquivo vira sem dado antes da grade, para não entrar na interpolação.
        host::run("raster.mask", json!({"layer": nome_da_janela, "min": 0.0, "max": 1.0}))?;
        host::run(
            "raster.project",
            json!({"layer": nome_da_janela, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": risco_fogo::reamostragem(g.celula), "name": risco_fogo::NA_GRADE}),
        )?;
        recortar(risco_fogo::NA_GRADE, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 1.0}))?;
        // A rampa já registrada (de um clique anterior) fica como está.
        let _ = host::run("ramp.add", json!({"text": risco_fogo::rampa(), "format": "gdal", "name": risco_fogo::RAMPA}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": risco_fogo::RAMPA})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "RdYlGn", "invert": true}))?;
        }
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": 0.0, "max": 1.0}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}", format!("{m:.2}").replace('.', ",")))
        .unwrap_or_default();
    let texto = format!(
        "{saida}: o perigo meteorológico de fogo de 0 a 1, média de {} de 2001 a 2025, na grade do projeto, {}; classes do INPE: {}{media}",
        risco_fogo::nome_do_mes(mes),
        g.texto(),
        risco_fogo::texto_das_classes()
    );
    avisar(&texto);
    // O "report" é o texto que a janela do comando mostra depois de rodar (#1723).
    Ok(
        json!({"layer": saida, "month": mes, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "report": texto.trim_start_matches(PREFIXO)}),
    )
}

/// As células do GEM-15 que existem entre as `cs`, pelos catálogos dos blocos delas (um bloco sem
/// catálogo não tem célula nenhuma), ou o pedido pendente.
fn celulas_do_gem15(cs: &[gem15::Celula]) -> Result<Result<Vec<gem15::Celula>, Value>, String> {
    let mut ha: Vec<String> = Vec::new();
    for b in gem15::blocos(cs) {
        match buscar_varios(&[gem15::catalogo(&b)]) {
            Ok(Ok(r)) => ha.extend(r.iter().flat_map(gem15::celulas_do_catalogo)),
            Ok(Err(p)) => return Ok(Err(p)),
            Err(e) if e.contains("404") || e.contains("403") => host::log(&format!("GEM-15 sem o bloco {b}: {e}")),
            Err(e) => return Err(format!("o catálogo do GEM-15 (storage.googleapis.com) não respondeu: {e}")),
        }
    }
    Ok(Ok(cs.iter().copied().filter(|c| ha.contains(&c.nome())).collect()))
}

/// Os modelos de elevação do Google a 15 m na área ([`gem15`]): a janela da área em cada célula
/// de 1° que ela toca, posta na grade do projeto; as células se juntam e o mosaico é recortado
/// pela área. A altura acima do terreno é a superfície menos o terreno, na grade.
fn gem15_na_area(req: &Request) -> Result<Value, String> {
    let modelo = gem15::Modelo::do_valor(req.params.get("modelo").and_then(Value::as_str))?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "o GEM-15 precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): cada arquivo cobre 1° × 1° a 15 m";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let todas = gem15::celulas(c);
    if todas.len() > gem15::MAX_CELULAS {
        return Err(format!(
            "a área toca {} células de 1° do GEM-15; o limite de um clique é {}: escolha uma área menor",
            todas.len(),
            gem15::MAX_CELULAS
        ));
    }
    let cs = match celulas_do_gem15(&todas)? {
        Ok(cs) => cs,
        Err(p) => return Ok(p),
    };
    if cs.is_empty() {
        return Err("o GEM-15 não tem dados na área (fora da cobertura, ou só água)".into());
    }
    let g = grade_para(c, celula)?;
    // A pirâmide mais grossa que não é mais grossa que a grade, se as janelas cabem no limite do
    // RustGIS; senão, pede uma grade mais grossa.
    let lido = gem15::nivel(g.celula);
    let janelas: Vec<(gem15::Celula, [f64; 4])> = cs.iter().filter_map(|&k| gem15::janela(k, c, lido).map(|j| (k, j))).collect();
    let js: Vec<[f64; 4]> = janelas.iter().map(|(_, j)| *j).collect();
    for a in modelo.arquivos() {
        let cabe = gem15::celula_que_cabe(&js, *a);
        if cabe.is_none_or(|m| m > lido) {
            let dica = cabe
                .map(|m| format!(": use uma grade de células de {m} m ou mais (Grade do projeto), ou uma área menor"))
                .unwrap_or_else(|| ": escolha uma área menor".into());
            return Err(format!("a área é grande demais para ler o GEM-15 a {lido} m numa janela só por célula{dica}"));
        }
    }
    // As janelas de um clique anterior servem se cobrem a área (no desktop, o comando roda de novo
    // quando cada uma chega).
    let ha = nomes()?;
    let mut lidas: Vec<(gem15::Arquivo, String)> = Vec::new();
    let mut lendo = 0;
    for a in modelo.arquivos() {
        for (k, j) in &janelas {
            let nome = gem15::nome_da_janela(*k, *a, lido);
            if ha.contains(&nome)
                && let Some(e) = extensao(&nome)?
                && retangulo_cabe([j[0].max(c[0]), j[1].max(c[1]), j[2].min(c[2]), j[3].min(c[3])], e)
            {
                lidas.push((*a, nome));
                continue;
            }
            tirar_janela(&nome)?;
            let mut p = json!({"url": k.url(*a), "name": nome, "bbox": j, "zoom": false});
            if lido > gem15::NIVEIS[0] {
                p["cellSize"] = json!(lido);
            }
            let r = host::run("layer.addUrl", p).map_err(|e| {
                if falha_passageira(&e) || e.contains("timeout") {
                    format!("o Google Cloud Storage não respondeu agora: tente de novo daqui a pouco ({e})")
                } else {
                    e
                }
            })?;
            if r.get("pending").and_then(Value::as_bool) == Some(true) {
                lendo += 1;
                continue;
            }
            if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
                tirar_janela(&nome)?;
                return Err(sem_janela.into());
            }
            lidas.push((*a, nome));
        }
    }
    if lendo > 0 {
        avisar(&format!("{} (GEM-15): lendo a área em {lendo} arquivo(s) do Google; a camada vem quando chegarem", modelo.titulo()));
        return Ok(json!({"pending": lendo, "cells": cs.len()}));
    }
    let saida = modelo.camada();
    let mut temporarias: Vec<String> = lidas.iter().map(|(_, n)| n.clone()).collect();
    remover(&[saida])?;
    let resampling = gem15::reamostragem(g.celula, lido);
    let feito = (|| -> Result<(), String> {
        // Cada arquivo: as janelas na grade (do DSM, só a primeira banda, a superfície) e juntas.
        let mut por_arquivo: Vec<String> = Vec::new();
        for a in modelo.arquivos() {
            let mut na_grade: Vec<String> = Vec::new();
            for (i, (_, j)) in lidas.iter().filter(|(b, _)| b == a).enumerate() {
                let mut origem = j.clone();
                if *a == gem15::Arquivo::Dsm {
                    let b1 = format!("{PREFIXO}GEM-15 banda 1 {i}");
                    host::run("raster.function", json!({"layer": j, "function": "bandMath", "expression": "b1", "name": b1}))?;
                    temporarias.push(b1.clone());
                    origem = b1;
                }
                let n = format!("{PREFIXO}GEM-15 {a:?} na grade {i}");
                host::run(
                    "raster.project",
                    json!({"layer": origem, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
                )?;
                temporarias.push(n.clone());
                na_grade.push(n);
            }
            let junto = if na_grade.len() == 1 {
                na_grade[0].clone()
            } else {
                let m = format!("{PREFIXO}GEM-15 {a:?} junto");
                mosaicar(&na_grade, &m)?;
                temporarias.push(m.clone());
                m
            };
            por_arquivo.push(junto);
        }
        let inteiro = if modelo == gem15::Modelo::Altura {
            let d = format!("{PREFIXO}GEM-15 DSM menos DTM");
            ferramenta("geolibre:subtract", json!({"input1": por_arquivo[0], "input2": por_arquivo[1]}), &d)?;
            temporarias.push(d.clone());
            d
        } else {
            por_arquivo[0].clone()
        };
        recortar(&inteiro, AREA, saida)?;
        if modelo == gem15::Modelo::Altura {
            let _ = host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlGn"}));
            let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": 0.0, "max": 30.0}));
        } else {
            let _ = host::run("raster.symbology", json!({"layer": saida, "type": "shadedRelief"}));
        }
        Ok(())
    })();
    let t: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&t)?;
    feito?;
    let faixa = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| Some((i["bands"][0]["min"].as_f64()?, i["bands"][0]["max"].as_f64()?, i["bands"][0]["mean"].as_f64()?)))
        .map(|(a, b, m)| format!("; na área, de {} a {} m, média de {} m", formatar_decimal(a), formatar_decimal(b), formatar_decimal(m)))
        .unwrap_or_default();
    let piramide = if lido > gem15::NIVEIS[0] { format!(", lida da pirâmide de {lido} m do arquivo") } else { String::new() };
    // A altura acima do terreno é uma diferença: o geoide não entra nela.
    let geoide = if modelo == gem15::Modelo::Altura { "" } else { " (alturas no geoide EGM96)" };
    let texto = format!(
        "{saida}: {}, em metros{geoide}, de {} célula(s) de 1° do GEM-15 (Google, experimental), na grade do projeto, {} ({resampling}){piramide}{faixa}. Água e superfícies sem textura ficam sem dado",
        modelo.titulo().to_lowercase(),
        cs.len(),
        g.texto()
    );
    avisar(&texto);
    Ok(json!({
        "layer": saida, "model": modelo.valor(), "cells": cs.len(), "read": lido, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// Os tiles do CHMv2 que existem entre os `ts`, pela listagem do bucket (no mar, ou fora da
/// cobertura, não há arquivo), ou o pedido pendente.
fn tiles_do_chm(ts: &[chm::Tile]) -> Result<Result<Vec<chm::Tile>, Value>, String> {
    let urls: Vec<String> = ts.iter().map(chm::Tile::listagem).collect();
    let respostas =
        match buscar(&urls).map_err(|e| format!("o bucket do CHMv2 (dataforgood-fb-data.s3.amazonaws.com) não respondeu: {e}"))? {
            Ok(r) => r,
            Err(p) => return Ok(Err(p)),
        };
    Ok(Ok(ts.iter().zip(&respostas).filter(|(t, r)| t.existe(r["text"].as_str().unwrap_or(""))).map(|(t, _)| t.clone()).collect()))
}

/// A cobertura da terra do Copernicus a 100 m ([`clms`]) de um ano de 2015 a 2019 na área: a janela
/// do retângulo lida do GeoTIFF do Zenodo, recortada pelo contorno e posta na grade do projeto com
/// a classe mais frequente, com as cores e os nomes da legenda oficial.
fn cobertura_clms(req: &Request) -> Result<Value, String> {
    let ano = req.f64("ano").map(|a| a as i64).unwrap_or(clms::ULTIMO_ANO);
    let url = clms::url(ano).ok_or(format!("o ano da cobertura do Copernicus é de {} a {}", clms::PRIMEIRO_ANO, clms::ULTIMO_ANO))?;
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "a cobertura do Copernicus precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): o arquivo é o mundo inteiro, com 362.880 × 141.120 células";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    if clms::celulas(c) > clms::MAX_CELULAS {
        return Err(format!(
            "a área é grande demais para a cobertura do Copernicus: a janela teria {:.0} milhões de células de 100 m, e o limite é {:.0} milhões (uns 7° × 7°, um estado médio); escolha uma área menor",
            clms::celulas(c) / 1e6,
            clms::MAX_CELULAS / 1e6
        ));
    }
    let nome = clms::camada(ano);
    let bruto = format!("{PREFIXO}Copernicus 100 m {ano} (janela)");
    let recorte = format!("{PREFIXO}Copernicus 100 m {ano} (recorte)");
    // A janela de um clique anterior serve se cobre a área (no desktop, o comando roda de novo
    // quando ela chega).
    let e = if nomes()?.contains(&bruto) { extensao(&bruto)? } else { None };
    if !e.is_some_and(|e| retangulo_cabe(c, e)) {
        tirar_janela(&bruto)?;
        let p = json!({"url": url, "name": bruto, "bbox": clms::janela(c), "zoom": false});
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if e.contains("429") {
                "o Zenodo limita os pedidos por minuto e pediu uma pausa: clique de novo daqui a uns 30 segundos".to_string()
            } else if falha_passageira(&e) || e.contains("timeout") {
                format!("o Zenodo não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!(
                "Cobertura da terra {ano} (Copernicus 100 m): lendo a janela da área no arquivo do Zenodo; a camada vem quando ela chegar"
            ));
            return Ok(r);
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&bruto)?;
            return Err(sem_janela.into());
        }
    }
    let g = grade_para(c, None)?;
    remover(&[&recorte, &nome])?;
    let feito = (|| -> Result<Value, String> {
        recortar(&bruto, AREA, &recorte)?;
        let r = host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "mode", "name": nome}),
        )?;
        host::run("style.importQgis", json!({"layer": nome, "text": clms::qml()}))?;
        // O 255 (sem dado) e o 0 (sem dado de entrada) ficam transparentes.
        let _ = host::run("raster.mask", json!({"layer": nome, "values": [0, 255]}));
        Ok(r)
    })();
    remover(&[&bruto, &recorte])?;
    let r = feito?;
    avisar(&format!(
        "{nome}: classes do Copernicus (CLMS Land Cover 100 m, {ano}) na grade do projeto, {}, com a classe mais frequente; cores e nomes da legenda oficial. A grade mais fina que 100 m repete a célula do arquivo",
        g.texto()
    ));
    Ok(json!({
        "layer": nome, "year": ano, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "width": r["width"], "height": r["height"],
    }))
}

/// A cobertura da terra do ESA WorldCover a 10 m ([`worldcover`]) de 2020 ou 2021 na área: a janela
/// da área em cada bloco de 3° que ela toca, lida do Planetary Computer, posta na grade do projeto
/// com a classe mais frequente; as janelas se juntam e o mosaico é recortado pela área.
fn worldcover_na_area(req: &Request) -> Result<Value, String> {
    let ano = req.f64("ano").map(worldcover::conferir_ano).transpose()?.unwrap_or(worldcover::ANOS[0]);
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "o WorldCover precisa de um RustGIS que leia só a janela da área de um arquivo remoto (desktop ou linha de comando, depois da 0.5.0)";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let quantos = worldcover::quantos_blocos(c);
    if quantos > worldcover::MAX_BLOCOS {
        return Err(format!(
            "a área toca {quantos} blocos de 3° do WorldCover; o limite de um clique é {} (um estado médio): escolha uma área menor",
            worldcover::MAX_BLOCOS
        ));
    }
    // Sem grade no mapa, uma de 10 m, a célula do dado (se couber na área).
    let celula = match (celula, grade_no_mapa()?) {
        (None, None) => Some(celula_que_cabe(c, 10.0)),
        (m, _) => m,
    };
    let g = grade_para(c, celula)?;
    conferir_grade_fina(c, g.celula, "a cobertura da terra do WorldCover")?;
    // Os blocos e o token.
    let r = match buscar_varios(&[worldcover::url_busca(c), worldcover::URL_DO_TOKEN.to_string()])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let bs = worldcover::blocos(&r[0], ano, c);
    if bs.is_empty() {
        return Err(format!("o Planetary Computer não tem blocos do WorldCover {ano} nesta área (no mar não há)"));
    }
    let token = radar_pc::token(&r[1]).ok_or("o serviço de tokens do Planetary Computer não deu um token: tente de novo daqui a pouco")?;
    let lat = (c[1] + c[3]) / 2.0;
    let k = worldcover::nivel(&bs, g.celula, lat).ok_or("a área é grande demais para o WorldCover: escolha uma área menor")?;
    let arquivo = worldcover::arquivo();
    // As janelas de um clique anterior servem se cobrem a parte da área (no desktop, o comando
    // roda de novo quando cada uma chega).
    let ha = nomes()?;
    let mut lidas: Vec<String> = Vec::new();
    let mut lendo = 0;
    for b in &bs {
        let nome = b.nome_da_janela(ano, k);
        if ha.contains(&nome)
            && let Some(e) = extensao(&nome)?
            && retangulo_cabe(b.parte, e)
        {
            lidas.push(nome);
            continue;
        }
        if lendo >= crate::FILA_DO_APP {
            break;
        }
        tirar_janela(&nome)?;
        let mut p = json!({"url": radar_pc::assinar(&b.href, &token), "name": nome, "bbox": arquivo.janela(b.parte, k), "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(worldcover::celula_pedida(k));
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") || e.contains("403") {
                format!("o Planetary Computer não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if pendente(&r) {
            lendo += 1;
            continue;
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome)?;
            return Err(sem_janela.into());
        }
        lidas.push(nome);
    }
    if lendo > 0 {
        avisar(&format!("Cobertura da terra {ano} (WorldCover): lendo a área em {lendo} bloco(s); a camada vem quando chegarem"));
        return Ok(json!({"pending": lendo, "tiles": bs.len()}));
    }
    let saida = worldcover::camada(ano);
    let lido = arquivo.metros_do_nivel(k, lat);
    let resampling = forty::reamostragem(g.celula, lido);
    let na_grade: Vec<String> = (0..lidas.len()).map(|i| format!("{PREFIXO}WorldCover na grade {i}")).collect();
    let junto = format!("{PREFIXO}WorldCover junto");
    let mut temporarias: Vec<&str> = lidas.iter().chain(&na_grade).map(String::as_str).collect();
    temporarias.push(junto.as_str());
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        for (j, n) in lidas.iter().zip(&na_grade) {
            host::run(
                "raster.project",
                json!({"layer": j, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
            )?;
            // Só as classes: o 0 (sem dado) e o que fica fora da janela somem, para não cobrir o
            // bloco vizinho no mosaico.
            host::run("raster.mask", json!({"layer": n, "min": 5.0, "max": 105.0}))?;
        }
        let mosaico = if na_grade.len() == 1 {
            na_grade[0].clone()
        } else {
            mosaicar(&na_grade, &junto)?;
            junto.clone()
        };
        recortar(&mosaico, AREA, &saida)?;
        host::run("style.importQgis", json!({"layer": saida, "text": worldcover::qml()}))?;
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let piramide = if k > 0 { format!(", lida da visão de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    let texto = format!(
        "{saida}: {} bloco(s), na grade do projeto, {} ({}){piramide}. Códigos: {}",
        bs.len(),
        g.texto(),
        if resampling == "mode" { "a classe mais frequente" } else { "a célula mais próxima" },
        worldcover::CHAVE
    );
    avisar(&texto);
    // O aviso dos anos só quando os dois estão no mapa, que é quando alguém os compararia.
    let ha = nomes()?;
    if worldcover::ANOS.iter().all(|&a| ha.contains(&worldcover::camada(a))) {
        avisar(worldcover::AVISO_ANOS);
    }
    Ok(json!({
        "layer": saida, "year": ano, "tiles": bs.len(), "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// Se o RustGIS tem o comando `id` com o parâmetro `param` (`info.commands`).
fn tem_parametro(id: &str, param: &str) -> Result<bool, String> {
    Ok(parametros_do_comando(id)?.is_some_and(|p| p.contains(&format!("\"{param}\""))))
}

/// O texto dos parâmetros do comando `id` deste RustGIS (`info.commands`), se ele o tem.
fn parametros_do_comando(id: &str) -> Result<Option<String>, String> {
    let lista = host::run("info.commands", json!({}))?;
    let comandos = lista.get("commands").unwrap_or(&lista);
    Ok(comandos
        .as_array()
        .into_iter()
        .flatten()
        .find(|c| c["id"].as_str() == Some(id))
        .and_then(|c| c["params"].as_str())
        .map(str::to_string))
}

/// O satélite, o índice, o início e o fim de um pedido ao cubo do INPE.
type PedidoBdc = (bdc::Satelite, bdc::Indice, Data, Data);

/// O satélite, o índice e o período de um comando do cubo do INPE ([`bdc`]): os do script ou da
/// janela; sem período, os últimos 3 meses da coleção (o plugin não tem relógio: a última data
/// vem do STAC do BDC). `Err(pendente)` dentro do `Ok` quando o STAC ficou para depois (app web).
fn pedido_do_bdc(req: &Request) -> Result<Result<PedidoBdc, Value>, String> {
    let s = match req.str("satelite") {
        Some(t) => bdc::Satelite::ler(t).ok_or(format!("satélite '{t}': use sentinel-2, landsat, cbers-4 ou modis"))?,
        None => bdc::Satelite::Sentinel2,
    };
    let i = match req.str("indice") {
        Some(t) => bdc::Indice::ler(t).ok_or(format!("índice '{t}': use NDVI ou EVI"))?,
        None => bdc::Indice::Ndvi,
    };
    let (inicio, fim) = match periodo_do_bdc(req, s)? {
        Ok(p) => p,
        Err(p) => return Ok(Err(p)),
    };
    bdc::conferir_periodo(s, inicio, fim)?;
    Ok(Ok((s, i, inicio, fim)))
}

/// O início e o fim de um comando do cubo do INPE, do satélite `s`: os do script ou da janela;
/// sem período, os últimos 3 meses da coleção (o plugin não tem relógio: a última data vem do STAC
/// do BDC). `Err(pendente)` dentro do `Ok` quando o STAC ficou para depois (app web).
fn periodo_do_bdc(req: &Request, s: bdc::Satelite) -> Result<Result<(Data, Data), Value>, String> {
    let data = |k: &str| -> Result<Option<Data>, String> {
        req.str(k).map(|t| bdc::ler_data(t).ok_or(format!("a data '{t}' não é dd/mm/aaaa"))).transpose()
    };
    let (inicio, fim) = (data("inicio")?, data("fim")?);
    Ok(Ok(match (inicio, fim) {
        (Some(a), Some(b)) => (a, b),
        _ => {
            let url = format!("{}collections/{}", bdc::STAC, s.colecao());
            let r = match buscar_varios(&[url])? {
                Ok(r) => r,
                Err(p) => return Ok(Err(p)),
            };
            let ultimo = r.first().and_then(bdc::fim_da_colecao).ok_or("o STAC do INPE não disse a última data da coleção")?;
            match (inicio, fim) {
                (Some(a), None) => (a, ultimo),
                (None, Some(b)) => (bdc::periodo_padrao(b).0, b),
                _ => bdc::periodo_padrao(ultimo),
            }
        }
    }))
}

/// O índice de vegetação do cubo do INPE num período, resumido (a média, a máxima, a mínima ou o
/// desvio das datas de 16 dias): lido só na área pelo `cube.reduce` do RustGIS, posto na grade do
/// projeto e recortado pelo contorno.
fn bdc_indice(req: &Request) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_parametro("cube.reduce", "collection")? {
        return Err("o índice do cubo do INPE precisa de um RustGIS que resuma um cubo STAC na área (cube.reduce, desde a 0.6.0)".into());
    }
    let e = match req.str("estatistica") {
        Some(t) => bdc::Estatistica::ler(t).ok_or(format!("estatística '{t}': use media, maxima, minima ou desvio"))?,
        None => bdc::Estatistica::Media,
    };
    let (s, i, inicio, fim) = match pedido_do_bdc(req)? {
        Ok(p) => p,
        Err(p) => return Ok(p),
    };
    let g = grade_para(c, req.f64("celula"))?;
    let nome = bdc::camada(s, i, e, inicio, fim);
    let lido = format!("{nome} (lido)");
    let na_grade = format!("{nome} (na grade)");
    // O que um clique anterior leu serve: no app, o comando roda de novo quando a leitura chega.
    // Nada se tira antes da leitura (ver executar).
    if !nomes()?.contains(&lido) {
        let r = host::run(
            "cube.reduce",
            json!({
                "url": bdc::STAC, "collection": s.colecao(), "asset": i.banda(), "bbox": c,
                "start": inicio.iso(), "end": fim.iso(), "cellSize": bdc::celula_de_leitura(s, g.celula),
                "resampling": "average", "stat": e.stat(), "name": lido,
            }),
        )
        .map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o cubo do INPE não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if pendente(&r) {
            avisar(&format!(
                "{} do {} (cubo do INPE): lendo a área de {} a {}; a camada vem quando chegar",
                i.banda(),
                s.nome(),
                inicio.texto(),
                fim.texto()
            ));
            return Ok(r);
        }
        if r["dates"].as_u64() == Some(0) {
            remover(&[&lido])?;
            return Err(format!("o cubo do {} não tem datas na área de {} a {}", s.nome(), inicio.texto(), fim.texto()));
        }
    }
    remover(&[&nome, &na_grade])?;
    let resampling = if bdc::celula_de_leitura(s, g.celula) < g.celula { "average" } else { "bilinear" };
    let feito = (|| -> Result<(), String> {
        host::run(
            "raster.project",
            json!({"layer": lido, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": na_grade}),
        )?;
        recortar(&na_grade, AREA, &nome)?;
        let (rampa, cores) = bdc::rampa(i);
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": "RdYlGn"}))?;
        }
        if e != bdc::Estatistica::Desvio {
            let (a, b) = i.escala();
            let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": a, "max": b}));
        }
        Ok(())
    })();
    remover(&[&lido, &na_grade])?;
    feito?;
    let media = host::run("raster.info", json!({"layer": nome}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}", format!("{m:.2}").replace('.', ",")))
        .unwrap_or_default();
    let texto = format!(
        "{}: das datas de 16 dias do cubo do {} (coleção {} do INPE), na grade do projeto, {}{media}",
        nome.trim_start_matches(PREFIXO),
        s.nome(),
        s.colecao(),
        g.texto()
    );
    avisar(&texto);
    Ok(json!({
        "layer": nome, "satellite": s.id(), "collection": s.colecao(), "index": i.banda(), "stat": e.stat(),
        "start": inicio.iso(), "end": fim.iso(), "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
    }))
}

/// A imagem sem nuvens do cubo do INPE ([`sem_nuvens`]): o melhor pixel das datas de 16 dias de
/// um período, em cor verdadeira ou falsa-cor, recortado pelo contorno, com a data de origem de
/// cada célula. O caminho (uma leitura por data, ou o `cube.reduce` com máscara num RustGIS sem
/// a Raster Calculator) segue [`sem_nuvens::caminho`].
fn bdc_sem_nuvens(req: &Request) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let parametros = parametros_do_comando("cube.reduce")?.unwrap_or_default();
    if !parametros.contains("\"collection\"") {
        return Err("a imagem sem nuvens precisa de um RustGIS que leia um cubo STAC na área (cube.reduce, desde a 0.6.0)".into());
    }
    let s = match req.str("satelite") {
        Some(t) => bdc::Satelite::ler(t)
            .filter(|s| sem_nuvens::SATELITES.contains(s))
            .ok_or(format!("satélite '{t}': use sentinel-2, landsat ou cbers-4"))?,
        None => bdc::Satelite::Sentinel2,
    };
    let comp = match req.str("composicao") {
        Some(t) => Composicao::ler(t).ok_or(format!("composição '{t}': use natural ou falsa-cor"))?,
        None => Composicao::Natural,
    };
    let (inicio, fim) = match periodo_do_bdc(req, s)? {
        Ok(p) => p,
        Err(p) => return Ok(p),
    };
    sem_nuvens::conferir_periodo(s, inicio, fim)?;
    let g = grade_para(c, None)?;
    // O caminho por data primeiro; sem a Raster Calculator ("unknown tool"), o cube.reduce com
    // máscara, se este RustGIS o tiver (sem_nuvens::caminho).
    let sem_ferramenta = |e: String| if sem_nuvens::ferramenta_ausente(&e) { sem_nuvens::PRECISA_DO_GEOLIBRE.to_string() } else { e };
    match sem_nuvens_por_data(c, &g, s, comp, inicio, fim) {
        Err(e) if sem_nuvens::ferramenta_ausente(&e) => match sem_nuvens::caminho(&parametros, false) {
            Some(Caminho::PeloCubeReduce) => sem_nuvens_pelo_cube_reduce(c, &g, s, comp, inicio, fim).map_err(sem_ferramenta),
            _ => Err(sem_nuvens::PRECISA_DO_GEOLIBRE.into()),
        },
        r => r,
    }
}

/// A Raster Calculator do GeoLibre com a `expressao`, numa camada nova `nome`. As entradas vão na
/// ordem em que os nomes aparecem na expressão: o RustGIS 0.7.0 as liga pela posição (os mais
/// novos, pelo nome).
fn calculadora(expressao: &str, nome: &str) -> Result<(), String> {
    remover(&[nome])?;
    let entradas = sem_nuvens::entradas_da_expressao(expressao);
    ferramenta("geolibre:raster_calculator", json!({"expression": expressao, "inputs": entradas}), nome)
}

/// Lê do cubo do satélite `s`, com o `cube.reduce` de uma data só (`start` = `end`), cada
/// (banda, data) de `pedidos` que ainda não está no mapa. `Err(pendente)` dentro do `Ok` quando o
/// app ficou esperando as leituras (até [`crate::FILA_DO_APP`] de cada vez; o comando roda de novo
/// quando elas chegam e lê as que faltam). No `Ok(Ok)`, as datas que não têm dado na área.
fn ler_do_cubo(s: bdc::Satelite, c: [f64; 4], celula: f64, pedidos: &[(&str, Data)]) -> Result<Result<Vec<Data>, Value>, String> {
    let ha = nomes()?;
    let mut lendo = 0;
    let mut sem_dado = Vec::new();
    for (banda, d) in pedidos {
        let nome = sem_nuvens::temporaria(s, banda, *d);
        if ha.contains(&nome) {
            continue;
        }
        if lendo >= crate::FILA_DO_APP {
            break;
        }
        let r = host::run(
            "cube.reduce",
            json!({
                "url": bdc::STAC, "collection": s.colecao(), "asset": banda, "bbox": c, "start": d.iso(), "end": d.iso(),
                "cellSize": celula, "stat": "max", "name": nome,
            }),
        )
        .map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o cubo do INPE não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if pendente(&r) {
            lendo += 1;
            continue;
        }
        if r["dates"].as_u64() == Some(0) {
            remover(&[&nome])?;
            sem_dado.push(*d);
        }
    }
    if lendo > 0 {
        avisar(&format!("Imagem sem nuvens do {} (cubo do INPE): lendo {lendo} banda(s) da área; a imagem vem quando chegarem", s.nome()));
        return Ok(Err(json!({"pending": lendo, "reads": pedidos.len()})));
    }
    Ok(Ok(sem_dado))
}

/// Um número em português, com `casas` decimais: "0,8".
fn decimal(x: f64, casas: usize) -> String {
    format!("{x:.casas$}").replace('.', ",")
}

/// A imagem sem nuvens pelo caminho do RustGIS 0.7.0 em diante ([`Caminho::PorData`]): as
/// máscaras de todas as datas, a nuvem dilatada de cada uma e a parte nublada na área, a ordem, a
/// data de origem, as bandas só das datas que a imagem usa, o melhor pixel de cada banda, a junção
/// e o recorte.
fn sem_nuvens_por_data(c: [f64; 4], g: &Grade, s: bdc::Satelite, comp: Composicao, inicio: Data, fim: Data) -> Result<Value, String> {
    let m = sem_nuvens::mascara(s).ok_or("o satélite não tem máscara de nuvem no cubo")?;
    let bandas = sem_nuvens::bandas(s, comp).ok_or("o satélite não tem as bandas de cor no cubo")?;
    let celula = sem_nuvens::celula_de_leitura(s, c);
    // As datas do período, pela busca de itens do STAC.
    let r = match buscar_varios(&[sem_nuvens::url_dos_itens(s, c, inicio, fim)])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let todas = r.first().map(|r| sem_nuvens::datas_dos_itens(r, inicio, fim)).unwrap_or_default();
    if todas.is_empty() {
        return Err(format!("o cubo do {} não tem datas na área de {} a {}", s.nome(), inicio.texto(), fim.texto()));
    }
    // As máscaras de todas as datas: a ordem depende delas.
    let pedidos: Vec<(&str, Data)> = todas.iter().map(|d| (m.banda, *d)).collect();
    let sem_dado = match ler_do_cubo(s, c, celula, &pedidos)? {
        Ok(v) => v,
        Err(p) => return Ok(p),
    };
    let datas: Vec<Data> = todas.iter().copied().filter(|d| !sem_dado.contains(d)).collect();
    let mut temporarias: Vec<String> = todas.iter().map(|d| sem_nuvens::temporaria(s, m.banda, *d)).collect();
    let feito = (|| -> Result<Value, String> {
        // A nuvem dilatada de cada data (1 nuvem, 0 limpo) e a parte nublada na área.
        let ha = nomes()?;
        let mut na_area = Vec::new();
        for d in &datas {
            let mascara = sem_nuvens::temporaria(s, m.banda, *d);
            let nuvem = sem_nuvens::temporaria(s, "nuvem", *d);
            temporarias.push(nuvem.clone());
            if !ha.contains(&nuvem) {
                let bruta = format!("{nuvem} sem dilatar");
                calculadora(&m.nuvem(&mascara), &bruta)?;
                let feita = ferramenta(
                    "geolibre:maximum_filter",
                    json!({"input": bruta, "filter_size_x": m.janela(), "filter_size_y": m.janela()}),
                    &nuvem,
                );
                remover(&[&bruta])?;
                feita?;
            }
            let parte =
                host::run("raster.info", json!({"layer": nuvem})).ok().and_then(|i| i["bands"][0]["mean"].as_f64()).unwrap_or(f64::NAN);
            na_area.push(DataNaArea { data: *d, nuvem: parte });
        }
        let ordem = sem_nuvens::ordenar(&na_area);
        if ordem.is_empty() {
            return Err(format!(
                "as {} data(s) do {} de {} a {} estão cobertas de nuvem na área: escolha outro período",
                datas.len(),
                s.nome(),
                inicio.texto(),
                fim.texto()
            ));
        }
        // A data de origem de cada célula: a posição, na ordem, da primeira data limpa (0: nenhuma).
        let origem = sem_nuvens::passo(s, "origem", inicio, fim);
        temporarias.push(origem.clone());
        let entradas: Vec<(String, Valor)> =
            ordem.iter().enumerate().map(|(k, x)| (sem_nuvens::temporaria(s, "nuvem", x.data), Valor::Numero(k as u32 + 1))).collect();
        calculadora(&sem_nuvens::melhor_pixel(&entradas), &origem)?;
        let info = host::run("raster.info", json!({"layer": origem}))?;
        // As datas que a imagem usa: até a última que dá alguma célula.
        let usadas = info["bands"][0]["max"].as_f64().map_or(ordem.len(), |x| (x.round() as usize).clamp(1, ordem.len()));
        let lista: Vec<String> = ordem.iter().map(|x| format!("{} ({}% de nuvem)", x.data.texto(), decimal(100.0 * x.nuvem, 1))).collect();
        let ordem = &ordem[..usadas];
        // As bandas só dessas datas.
        let pedidos: Vec<(&str, Data)> = ordem.iter().flat_map(|x| bandas.iter().map(move |b| (*b, x.data))).collect();
        temporarias.extend(pedidos.iter().map(|(b, d)| sem_nuvens::temporaria(s, b, *d)));
        match ler_do_cubo(s, c, celula, &pedidos)? {
            Ok(v) if !v.is_empty() => {
                return Err(format!("o cubo do {} não tem as bandas de {} na área", s.nome(), v[0].texto()));
            }
            Ok(_) => {}
            Err(p) => return Ok(p),
        }
        // O melhor pixel de cada banda, e a junção em vermelho, verde e azul.
        let mut canais = Vec::new();
        for b in bandas {
            let melhor = sem_nuvens::passo(s, &format!("melhor {b}"), inicio, fim);
            let entradas: Vec<(String, Valor)> = ordem
                .iter()
                .map(|x| (sem_nuvens::temporaria(s, "nuvem", x.data), Valor::Camada(sem_nuvens::temporaria(s, b, x.data))))
                .collect();
            temporarias.push(melhor.clone());
            calculadora(&sem_nuvens::melhor_pixel(&entradas), &melhor)?;
            canais.push(melhor);
        }
        let junto = sem_nuvens::passo(s, "junto", inicio, fim);
        temporarias.push(junto.clone());
        remover(&[&junto])?;
        ferramenta("geolibre:composite_bands", json!({"inputs": canais}), &junto)?;
        let nome = sem_nuvens::camada(s, comp, inicio, fim);
        let nome_da_data = sem_nuvens::camada_da_data(s, inicio, fim);
        remover(&[&nome, &nome_da_data])?;
        let (junto_na_grade, origem_na_grade) = (format!("{junto} na grade"), format!("{origem} na grade"));
        temporarias.extend([junto_na_grade.clone(), origem_na_grade.clone()]);
        no_sistema_da_grade(&junto, g, celula, &junto_na_grade)?;
        recortar(&junto_na_grade, AREA, &nome)?;
        simbolizar_sem_nuvens(&nome, s, comp)?;
        no_sistema_da_grade(&origem, g, celula, &origem_na_grade)?;
        recortar(&origem_na_grade, AREA, &nome_da_data)?;
        let datas_usadas: Vec<Data> = ordem.iter().map(|x| x.data).collect();
        host::run("style.importQgis", json!({"layer": nome_da_data, "text": sem_nuvens::qml_da_data(&datas_usadas)}))?;
        let _ = host::run("raster.mask", json!({"layer": nome_da_data, "values": [0]}));
        let _ = host::run("layer.visibility", json!({"layer": nome_da_data, "visible": false}));
        // A parte da área sem nenhuma data limpa.
        let buracos = sem_nuvens::passo(s, "buracos", inicio, fim);
        temporarias.push(buracos.clone());
        let vazia = calculadora(&format!("if((\"{origem}\" == 0), 1, 0)"), &buracos)
            .ok()
            .and_then(|_| host::run("raster.info", json!({"layer": buracos})).ok())
            .and_then(|i| i["bands"][0]["mean"].as_f64())
            .unwrap_or(0.0);
        let vazia = if vazia > 0.0005 {
            format!(" {}% do retângulo da área ficou sem data limpa (transparente).", decimal(100.0 * vazia, 1))
        } else {
            String::new()
        };
        avisar(&format!(
            "{}: o melhor pixel das datas de 16 dias da coleção {} do INPE, a {} m, no sistema da grade do projeto, {}. Cada célula vem da primeira data limpa, da menos nublada na área para a mais nublada, com a nuvem crescida {} célula(s) para os lados. Das {} data(s) do período, na ordem: {}{}. A imagem usou {}.{vazia} A camada {} diz de que data veio cada célula.",
            nome.trim_start_matches(PREFIXO),
            s.colecao(),
            decimal(celula, 0),
            grade::nome_do_sistema(g.epsg),
            m.dilatacao,
            todas.len(),
            lista.join(", "),
            if todas.len() > lista.len() {
                format!("; {} sem dado ou coberta(s) de nuvem", todas.len() - lista.len())
            } else {
                String::new()
            },
            if usadas == 1 { "só a primeira".to_string() } else { format!("as {usadas} primeiras") },
            nome_da_data.trim_start_matches(PREFIXO),
        ));
        Ok(json!({
            "layer": nome, "dates": nome_da_data, "satellite": s.id(), "collection": s.colecao(), "composition": comp.id(),
            "start": inicio.iso(), "end": fim.iso(), "crs": format!("EPSG:{}", g.epsg), "cellSize": celula,
            "order": datas_usadas.iter().map(Data::iso).collect::<Vec<_>>(),
        }))
    })();
    // Uma leitura que ficou pendente guarda o que já veio (o comando roda de novo e o usa).
    if feito.as_ref().is_ok_and(pendente) {
        return feito;
    }
    let t: Vec<&str> = temporarias.iter().map(String::as_str).collect();
    remover(&t)?;
    feito
}

/// O raster `camada` no sistema da grade do projeto `g`, com células de `celula` metros (as do
/// satélite) no retângulo dela, numa camada nova `nome`, pelo vizinho mais próximo (as classes da
/// data de origem e as células vazias, 0 nas três bandas, ficam como estão). O cubo do BDC vem num
/// Albers sem código EPSG, e o recorte do GeoLibre (o do app desktop, que traz a extensão Imagens
/// desligada) não reprojeta um raster sem EPSG para o contorno da área.
fn no_sistema_da_grade(camada: &str, g: &Grade, celula: f64, nome: &str) -> Result<(), String> {
    remover(&[nome])?;
    host::run(
        "raster.project",
        json!({"layer": camada, "crs": g.epsg, "cellSize": celula, "extent": GRADE, "resampling": "nearest", "name": nome}),
    )?;
    Ok(())
}

/// O vermelho, o verde e o azul da imagem `nome`, com o contraste fixo do satélite `s` na
/// composição `comp`.
fn simbolizar_sem_nuvens(nome: &str, s: bdc::Satelite, comp: Composicao) -> Result<(), String> {
    host::run("raster.symbology", json!({"layer": nome, "type": "rgb", "red": 1, "green": 2, "blue": 3}))?;
    let (min, max) = sem_nuvens::contraste(s, comp);
    host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": min, "max": max}))?;
    // Onde nenhuma data foi limpa, as três bandas são 0: transparente, não preto.
    let _ = host::run("raster.mask", json!({"layer": nome, "values": [0]}));
    Ok(())
}

/// A imagem sem nuvens pelo `cube.reduce` com máscara e `stat: "first"`
/// ([`Caminho::PeloCubeReduce`], opengeos/RustGIS#2596): uma leitura por banda, a junção e o
/// recorte. Sem a dilatação da máscara nem a camada da data de origem.
fn sem_nuvens_pelo_cube_reduce(
    c: [f64; 4],
    g: &Grade,
    s: bdc::Satelite,
    comp: Composicao,
    inicio: Data,
    fim: Data,
) -> Result<Value, String> {
    let bandas = sem_nuvens::bandas(s, comp).ok_or("o satélite não tem as bandas de cor no cubo")?;
    let ha = nomes()?;
    let canais: Vec<String> = bandas.iter().map(|b| sem_nuvens::passo(s, &format!("primeira {b}"), inicio, fim)).collect();
    let mut lendo = 0;
    for (b, canal) in bandas.iter().zip(&canais) {
        if ha.contains(canal) {
            continue;
        }
        let p = sem_nuvens::cube_reduce_com_mascara(s, b, c, inicio, fim, canal).ok_or("o satélite não tem máscara de nuvem no cubo")?;
        let r = host::run("cube.reduce", p)?;
        if pendente(&r) {
            lendo += 1;
        } else if r["dates"].as_u64() == Some(0) {
            remover(&[canal])?;
            return Err(format!("o cubo do {} não tem datas na área de {} a {}", s.nome(), inicio.texto(), fim.texto()));
        }
    }
    if lendo > 0 {
        avisar(&format!("Imagem sem nuvens do {} (cubo do INPE): lendo a área; a imagem vem quando chegar", s.nome()));
        return Ok(json!({"pending": lendo}));
    }
    let junto = sem_nuvens::passo(s, "junto", inicio, fim);
    let junto_na_grade = format!("{junto} na grade");
    let nome = sem_nuvens::camada(s, comp, inicio, fim);
    let feito = (|| -> Result<(), String> {
        remover(&[&junto, &junto_na_grade, &nome])?;
        ferramenta("geolibre:composite_bands", json!({"inputs": canais}), &junto)?;
        no_sistema_da_grade(&junto, g, sem_nuvens::celula_de_leitura(s, c), &junto_na_grade)?;
        recortar(&junto_na_grade, AREA, &nome)?;
        simbolizar_sem_nuvens(&nome, s, comp)
    })();
    let mut fora: Vec<&str> = canais.iter().map(String::as_str).collect();
    fora.extend([junto.as_str(), junto_na_grade.as_str()]);
    remover(&fora)?;
    feito?;
    avisar(&format!(
        "{}: a primeira data limpa de cada célula, na ordem da parte limpa na área, da coleção {} do INPE (cube.reduce com máscara)",
        nome.trim_start_matches(PREFIXO),
        s.colecao()
    ));
    Ok(
        json!({"layer": nome, "satellite": s.id(), "collection": s.colecao(), "composition": comp.id(), "start": inicio.iso(), "end": fim.iso()}),
    )
}

/// Um dado mensal do JAXA Earth na área ([`jaxa`]), resumido num período (a média, a máxima, a
/// mínima ou o desvio dos meses): lido só na área pelo `cube.reduce` do RustGIS na coleção estática
/// do conjunto, convertido para a unidade do texto e recortado pelo contorno, na resolução original
/// do dado (fora da grade do projeto). Sem período, os 3 meses até o último com dados (lido no
/// `collection.json`).
fn jaxa_na_area(req: &Request, v: jaxa::Variavel) -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    if !tem_parametro("cube.reduce", "collection")? {
        return Err("os dados do JAXA Earth precisam de um RustGIS que resuma um cubo STAC na área (cube.reduce, desde a 0.6.0)".into());
    }
    let e = match req.str("estatistica") {
        Some(t) => bdc::Estatistica::ler(t).ok_or(format!("estatística '{t}': use media, maxima, minima ou desvio"))?,
        None => bdc::Estatistica::Media,
    };
    let data = |k: &str| -> Result<Option<Data>, String> {
        req.str(k).map(|t| bdc::ler_data(t).ok_or(format!("a data '{t}' não é dd/mm/aaaa"))).transpose()
    };
    let (inicio, fim) = match (data("inicio")?, data("fim")?) {
        (Some(a), Some(b)) => (a, b),
        (a, b) => {
            let r = match buscar_varios(&[v.colecao()])? {
                Ok(r) => r,
                Err(p) => return Ok(p),
            };
            let ultimo = r.first().and_then(jaxa::ultimo_mes).ok_or("o JAXA Earth não disse o último mês do conjunto")?;
            let (i, f) = jaxa::periodo_padrao(ultimo);
            (a.unwrap_or(i), b.unwrap_or(f))
        }
    };
    jaxa::conferir_periodo(v, inicio, fim)?;
    let nome = jaxa::camada(v, e, inicio, fim);
    let lido = format!("{nome} (lido)");
    let convertido = format!("{nome} (convertido)");
    // O que um clique anterior leu serve: no app, o comando roda de novo quando a leitura chega.
    // Nada se tira antes da leitura (ver executar).
    if !nomes()?.contains(&lido) {
        let r = host::run(
            "cube.reduce",
            json!({
                "url": v.colecao(), "collection": v.conjunto(), "asset": v.banda(), "bbox": c,
                "start": inicio.iso(), "end": fim.iso(), "stat": e.stat(), "name": lido,
            }),
        )
        .map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o JAXA Earth não respondeu agora: tente de novo daqui a pouco ({e})")
            } else if e.contains("no item") {
                format!("o JAXA Earth não tem este dado na área de {} a {}", inicio.texto(), fim.texto())
            } else {
                e
            }
        })?;
        if pendente(&r) {
            avisar(&format!(
                "{} ({}, JAXA Earth): lendo a área de {} a {}; a camada vem quando chegar",
                v.titulo().0,
                v.fonte(),
                inicio.texto(),
                fim.texto()
            ));
            return Ok(r);
        }
    }
    remover(&[&nome, &convertido])?;
    let feito = (|| -> Result<(), String> {
        // Na resolução original: as células do JAXA, recortadas pela área, sem reamostrar.
        let origem = match v.expressao(e) {
            Some(x) => {
                host::run("raster.function", json!({"layer": lido, "function": "bandMath", "expression": x, "name": convertido}))?;
                convertido.clone()
            }
            None => lido.clone(),
        };
        recortar(&origem, AREA, &nome)?;
        let (rampa, cores) = v.rampa();
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": "Viridis"}))?;
        }
        if e != bdc::Estatistica::Desvio {
            let (a, b) = v.escala();
            let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": a, "max": b}));
        }
        Ok(())
    })();
    remover(&[&lido, &convertido])?;
    feito?;
    let media = host::run("raster.info", json!({"layer": nome}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}{}", format!("{m:.2}").replace('.', ","), com_espaco(v.unidade())))
        .unwrap_or_default();
    avisar(&format!(
        "{}: dos meses do JAXA Earth ({}){}, na resolução original do dado, {}, recortada pela área (fora da grade do projeto){media}",
        nome.trim_start_matches(PREFIXO),
        v.conjunto(),
        if v.unidade().is_empty() { String::new() } else { format!(", em {}", v.unidade()) },
        v.resolucao()
    ));
    Ok(json!({
        "layer": nome, "dataset": v.conjunto(), "band": v.banda(), "unit": v.unidade(), "stat": e.stat(),
        "start": inicio.iso(), "end": fim.iso(), "resolution": v.resolucao(),
    }))
}

/// " mm/dia", ou nada sem unidade.
fn com_espaco(unidade: &str) -> String {
    if unidade.is_empty() { String::new() } else { format!(" {unidade}") }
}

/// O turno da temperatura do JAXA (`turno`: dia, o padrão, ou noite).
fn turno_do_jaxa(req: &Request) -> Result<jaxa::Turno, String> {
    match req.str("turno") {
        Some(t) => jaxa::Turno::ler(t).ok_or(format!("turno '{t}': use dia ou noite")),
        None => Ok(jaxa::Turno::Dia),
    }
}

/// Um mapa de seca do Global Drought Observatory ([`gdo`]): a anomalia da umidade do solo, a da
/// vegetação ou o SPI do CHIRPS, numa data ou na mais recente, como mapa para ver (o WMS do GDO),
/// com a legenda do serviço.
fn gdo_seca(req: &Request) -> Result<Value, String> {
    let i = match req.str("indicador") {
        Some(t) => gdo::Indicador::ler(t).ok_or(format!("indicador '{t}': use umidade, vegetacao, spi1, spi3, spi6 ou spi12"))?,
        None => gdo::Indicador::Umidade,
    };
    let data = req
        .str("data")
        .filter(|t| !t.trim().is_empty())
        .map(|t| bdc::ler_data(t).ok_or(format!("a data '{t}' não é dd/mm/aaaa")))
        .transpose()?
        .map(|d| i.passo(d));
    let nome = i.camada(data);
    if mosaicos_no_mapa()?.contains(&nome) {
        host::run("map.removeTiles", json!({"name": nome}))?;
    }
    host::run(
        "map.addTiles",
        json!({"url": i.url(data), "kind": "raster", "name": nome, "overlay": true, "opacity": 0.85,
            "attribution": "Copernicus GDO (CEMS, JRC), CC BY 4.0", "legend": i.legenda()}),
    )?;
    let quando = match data {
        Some(_) => "na data pedida (o começo do decêndio ou do mês que a contém)",
        None => "na data mais recente do serviço",
    };
    avisar(&format!(
        "{}: mapa para ver do Global Drought Observatory (Copernicus), {quando}, com a legenda do serviço sob a camada; não é recortado pela área (o RustGIS 0.7.0 não recorta um mapa de tiles: RustGIS#2530), e os valores do índice não vêm nele (o GDO só os publica em arquivos do mundo inteiro por ano)",
        nome.trim_start_matches(PREFIXO)
    ));
    Ok(json!({"tiles": nome, "indicator": format!("{i:?}")}))
}

/// As edificações do GlobalBuildingAtlas na área ([`gba`]): o arquivo de cada bloco de 5° que a
/// área toca, lido no lugar pelo RustGIS (só o que a vista mostra), com as classes de altura.
fn gba_na_area() -> Result<Value, String> {
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let todos = gba::blocos(c);
    if todos.len() > gba::MAX_BLOCOS {
        return Err(format!(
            "a área toca {} blocos de 5° do GlobalBuildingAtlas; o limite de um clique é {}: escolha uma área menor",
            todos.len(),
            gba::MAX_BLOCOS
        ));
    }
    let r = match buscar(&[gba::LISTAGEM.to_string()]).map_err(|e| format!("o Source Cooperative (data.source.coop) não respondeu: {e}"))?
    {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let lista = r.first().and_then(|r| r["text"].as_str()).unwrap_or("");
    let blocos: Vec<&String> = todos.iter().filter(|b| gba::existe(b, lista)).collect();
    if blocos.is_empty() {
        return Err("o GlobalBuildingAtlas não tem edificações na área".into());
    }
    let varios = blocos.len() > 1;
    let _ = host::run("style.remove", json!({"name": gba::ESTILO}));
    host::run("style.add", json!({"text": gba::estilo()}))?;
    let ha = nomes()?;
    let (mut abertas, mut lendo, mut edificacoes) = (Vec::new(), 0, 0u64);
    for b in &blocos {
        let nome = gba::camada(b, varios);
        if !ha.contains(&nome) {
            let r = host::run("layer.addUrl", json!({"url": gba::url(b), "name": nome})).map_err(|e| {
                if falha_passageira(&e) || e.contains("timeout") {
                    format!("o Source Cooperative não respondeu agora: tente de novo daqui a pouco ({e})")
                } else {
                    e
                }
            })?;
            if r.get("pending").and_then(Value::as_bool) == Some(true) {
                lendo += 1;
                continue;
            }
            edificacoes += r["features"].as_u64().unwrap_or(0);
        }
        // Uma camada que acaba de chegar (de um símbolo só) ganha as classes de altura, e o estilo
        // que o plugin guardou dela antes do comando (o símbolo só, quando o comando roda de novo
        // porque o arquivo chegou) sai, para não voltar no fim. Uma que o usuário estilizou fica.
        let simples = camada_no_projeto(&nome)?.is_none_or(|l| l["renderer"].as_str() == Some("Single Symbol"));
        if simples {
            host::run("style.apply", json!({"layer": nome, "name": gba::ESTILO}))?;
            let _ = host::run("style.remove", json!({"name": estilo::guardado(&nome)}));
        }
        abertas.push(nome);
    }
    if lendo > 0 {
        avisar(&format!(
            "Edificações (GlobalBuildingAtlas): abrindo {lendo} arquivo(s) do Source Cooperative; a camada vem quando chegarem"
        ));
        return Ok(json!({"pending": lendo, "tiles": blocos.len()}));
    }
    let quantas = if edificacoes > 0 {
        format!(" ({} milhões de edificações no bloco)", formatar_decimal((edificacoes as f64 / 1e5).round() / 10.0))
    } else {
        String::new()
    };
    let texto = format!(
        "{}: polígonos e alturas do GlobalBuildingAtlas{quantas}, lidos no lugar, só o que a vista mostra: a camada não é recortada pela área. Cores por classe de altura (campo height, em m); altura estimada de imagens PlanetScope de 2019, menos precisa na América do Sul. Licença CC BY-NC 4.0: sem uso comercial",
        abertas.first().map(String::as_str).unwrap_or(gba::PREFIXO_CAMADA).trim_start_matches(PREFIXO)
    );
    avisar(&texto);
    Ok(json!({"layers": abertas, "tiles": blocos.len(), "features": edificacoes, "report": texto}))
}

/// Os tiles da altura da Amazônia que existem entre os `ts`, pela listagem do bucket (fora da
/// floresta amazônica não há arquivo), ou o pedido pendente.
fn tiles_da_amazonia(ts: &[chm_amazonia::Tile]) -> Result<Result<Vec<chm_amazonia::Tile>, Value>, String> {
    let urls: Vec<String> = ts.iter().map(chm_amazonia::Tile::listagem).collect();
    let respostas = match buscar(&urls)
        .map_err(|e| format!("o bucket da CTrees (ctrees-amazon-canopy-height.s3.us-west-2.amazonaws.com) não respondeu: {e}"))?
    {
        Ok(r) => r,
        Err(p) => return Ok(Err(p)),
    };
    Ok(Ok(ts.iter().zip(&respostas).filter(|(t, r)| t.existe(r["text"].as_str().unwrap_or(""))).map(|(t, _)| t.clone()).collect()))
}

/// A altura do dossel da Amazônia da CTrees a 4,78 m na área ([`chm_amazonia`]): a janela da área
/// em cada tile que ela toca, posta na grade do projeto; os tiles se juntam, o mosaico é recortado
/// pela área e passa a metros.
fn chm_amazonia_na_area(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela =
        "a altura do dossel da Amazônia precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0)";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let todos = chm_amazonia::tiles(c);
    if todos.len() > chm_amazonia::MAX_TILES {
        return Err(format!(
            "a área toca {} tiles da altura da Amazônia (cerca de 20 × 20 km cada); o limite de um clique é {}: escolha uma área menor, como um município pequeno",
            todos.len(),
            chm_amazonia::MAX_TILES
        ));
    }
    let ts = match tiles_da_amazonia(&todos)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    if ts.is_empty() {
        return Err("a altura do dossel da CTrees cobre só a floresta amazônica: não há dados na área".into());
    }
    let g = grade_para(c, celula)?;
    conferir_grade_fina(c, g.celula, "a altura do dossel da Amazônia")?;
    let lat = (c[1] + c[3]) / 2.0;
    let k = chm_amazonia::nivel(g.celula, lat);
    let lado = chm_amazonia::lado_do_nivel(k);
    let janelas: Vec<(&chm_amazonia::Tile, [f64; 4])> =
        ts.iter().filter_map(|t| chm_amazonia::janela(t, c, lado).map(|j| (t, j))).collect();
    let js: Vec<[f64; 4]> = janelas.iter().map(|(_, j)| *j).collect();
    match chm_amazonia::nivel_que_cabe(&js, k) {
        Some(n) if n == k => {}
        cabe => {
            let dica = cabe
                .map(|n| {
                    format!(
                        ": use uma grade de células de {} m ou mais (Grade do projeto), ou uma área menor",
                        chm_amazonia::lado_no_chao(n, lat).ceil()
                    )
                })
                .unwrap_or_else(|| ": escolha uma área menor".into());
            return Err(format!(
                "a área é grande demais para ler a altura da Amazônia com células de {} m numa janela só por tile{dica}",
                formatar_decimal(chm_amazonia::lado_no_chao(k, lat))
            ));
        }
    }
    // As janelas de um clique anterior servem se cobrem a área (no desktop, o comando roda de novo
    // quando cada uma chega).
    let ha = nomes()?;
    let mut lidas: Vec<String> = Vec::new();
    let mut lendo = 0;
    for (t, j) in &janelas {
        let nome = t.nome_da_janela(k);
        if ha.contains(&nome)
            && let Some(e) = extensao(&nome)?
            && retangulo_cabe(chm_amazonia::a_cobrir(t, c), e)
        {
            lidas.push(nome);
            continue;
        }
        tirar_janela(&nome)?;
        let mut p = json!({"url": t.url(), "name": nome, "bbox": j, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(lado);
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o bucket da CTrees não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            lendo += 1;
            continue;
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome)?;
            return Err(sem_janela.into());
        }
        lidas.push(nome);
    }
    if lendo > 0 {
        avisar(&format!("Altura do dossel da Amazônia (CTrees): lendo a área em {lendo} arquivo(s); a camada vem quando chegarem"));
        return Ok(json!({"pending": lendo, "tiles": ts.len()}));
    }
    let saida = chm_amazonia::CAMADA;
    let lido = chm_amazonia::lado_no_chao(k, lat);
    let resampling = chm_amazonia::reamostragem(g.celula, lido);
    let na_grade: Vec<String> = (0..lidas.len()).map(|i| format!("{PREFIXO}CTrees Amazônia na grade {i}")).collect();
    let junto = format!("{PREFIXO}CTrees Amazônia junto");
    let recortado = format!("{PREFIXO}CTrees Amazônia recortado");
    let mut temporarias: Vec<&str> = lidas.iter().chain(&na_grade).map(String::as_str).collect();
    temporarias.push(junto.as_str());
    temporarias.push(recortado.as_str());
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        for (j, n) in lidas.iter().zip(&na_grade) {
            host::run(
                "raster.project",
                json!({"layer": j, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
            )?;
            // O 255 (sem dado) e o que fica fora da janela somem, para não cobrir o tile vizinho
            // no mosaico (o valor gravado vai de 0 a 254: a altura × 2,5).
            host::run("raster.mask", json!({"layer": n, "min": 0.0, "max": 254.5}))?;
        }
        let mosaico = if na_grade.len() == 1 {
            na_grade[0].clone()
        } else {
            mosaicar(&na_grade, &junto)?;
            junto.clone()
        };
        recortar(&mosaico, AREA, &recortado)?;
        host::run(
            "raster.function",
            json!({"layer": recortado, "function": "bandMath", "expression": format!("b1 / {}", chm_amazonia::FATOR), "name": saida}),
        )?;
        // As cores da altura do GEDI e do CHMv2, para que as três se comparem.
        let (rampa, cores) = gedi::Variavel::Altura.rampa();
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlGn"}))?;
        }
        let (a, b) = gedi::Variavel::Altura.escala();
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} m", formatar_decimal(m)))
        .unwrap_or_default();
    let piramide = if k > 0 { format!(", lida da pirâmide de {} m do arquivo", formatar_decimal(lido)) } else { String::new() };
    let texto = format!(
        "{saida}: em metros, média de 2020 a 2024, de {} tile(s) da CTrees, na grade do projeto, {} ({resampling}){piramide}{media}",
        ts.len(),
        g.texto()
    );
    avisar(&texto);
    if let Some(a) =
        grade::aviso_de_reamostragem("a altura da Amazônia (CTrees)", chm_amazonia::lado_no_chao(0, lat), g.celula, "é a média")
    {
        avisar(&a);
    }
    Ok(json!({
        "layer": saida, "tiles": ts.len(), "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// A altura do dossel da Meta e do WRI a cerca de 1 m na área ([`chm`]): a janela da área em cada
/// tile que ela toca, posta na grade do projeto; os tiles se juntam e o mosaico é recortado pela
/// área.
fn chm_na_area(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "a altura do dossel do CHMv2 precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): cada arquivo tem 32.768 × 32.768 células";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let todos = chm::tiles(c);
    if todos.len() > chm::MAX_TILES {
        return Err(format!(
            "a área toca {} tiles do CHMv2 (cerca de 37 × 37 km cada); o limite de um clique é {}: escolha uma área menor",
            todos.len(),
            chm::MAX_TILES
        ));
    }
    let ts = match tiles_do_chm(&todos)? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    if ts.is_empty() {
        return Err("o CHMv2 não tem dados na área".into());
    }
    let g = grade_para(c, celula)?;
    conferir_grade_fina(c, g.celula, "a altura do dossel do CHMv2")?;
    let lat = (c[1] + c[3]) / 2.0;
    // O nível mais grosso que não é mais grosso que a grade, se as janelas cabem no limite do
    // RustGIS; senão, pede uma grade mais grossa.
    let k = chm::nivel(g.celula, lat);
    let lado = chm::lado_do_nivel(k);
    let janelas: Vec<(&chm::Tile, [f64; 4])> = ts.iter().filter_map(|t| chm::janela(t, c, lado).map(|j| (t, j))).collect();
    let js: Vec<[f64; 4]> = janelas.iter().map(|(_, j)| *j).collect();
    match chm::nivel_que_cabe(&js, k) {
        Some(n) if n == k => {}
        cabe => {
            let dica = cabe
                .map(|n| {
                    format!(
                        ": use uma grade de células de {} m ou mais (Grade do projeto), ou uma área menor",
                        chm::lado_no_chao(n, lat).ceil()
                    )
                })
                .unwrap_or_else(|| ": escolha uma área menor".into());
            return Err(format!(
                "a área é grande demais para ler o CHMv2 com células de {} m numa janela só por tile{dica}",
                formatar_decimal(chm::lado_no_chao(k, lat))
            ));
        }
    }
    // As janelas de um clique anterior servem se cobrem a área (no desktop, o comando roda de novo
    // quando cada uma chega).
    let ha = nomes()?;
    let mut lidas: Vec<String> = Vec::new();
    let mut lendo = 0;
    for (t, j) in &janelas {
        let nome = chm::nome_da_janela(t, k);
        if ha.contains(&nome)
            && let Some(e) = extensao(&nome)?
            && retangulo_cabe(chm::a_cobrir(t, c), e)
        {
            lidas.push(nome);
            continue;
        }
        tirar_janela(&nome)?;
        let mut p = json!({"url": t.url(), "name": nome, "bbox": j, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(lado);
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o bucket do CHMv2 não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            lendo += 1;
            continue;
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome)?;
            return Err(sem_janela.into());
        }
        lidas.push(nome);
    }
    if lendo > 0 {
        avisar(&format!("Altura do dossel (CHMv2): lendo a área em {lendo} arquivo(s) da Meta; a camada vem quando chegarem"));
        return Ok(json!({"pending": lendo, "tiles": ts.len()}));
    }
    let saida = chm::CAMADA;
    let lido = chm::lado_no_chao(k, lat);
    let resampling = chm::reamostragem(g.celula, lido);
    let mais_um: Vec<String> = (0..lidas.len()).map(|i| format!("{PREFIXO}CHMv2 mais 1 {i}")).collect();
    let na_grade: Vec<String> = (0..lidas.len()).map(|i| format!("{PREFIXO}CHMv2 na grade {i}")).collect();
    let junto = format!("{PREFIXO}CHMv2 junto");
    let recortado = format!("{PREFIXO}CHMv2 recortado");
    let mut temporarias: Vec<&str> = lidas.iter().chain(&mais_um).chain(&na_grade).map(String::as_str).collect();
    temporarias.push(junto.as_str());
    temporarias.push(recortado.as_str());
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        // O arquivo não tem valor de sem dado: na grade, o que fica fora de uma janela viraria 0
        // e cobriria o tile vizinho no mosaico. A altura sobe 1, o que fica abaixo de 0,5 (fora
        // da janela) vira sem dado, e no fim desce 1 de novo.
        for ((j, u), n) in lidas.iter().zip(&mais_um).zip(&na_grade) {
            host::run("raster.function", json!({"layer": j, "function": "bandMath", "expression": "b1 + 1", "name": u}))?;
            host::run(
                "raster.project",
                json!({"layer": u, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
            )?;
            host::run("raster.mask", json!({"layer": n, "min": 0.5, "max": 1000.0}))?;
        }
        let mosaico = if na_grade.len() == 1 {
            na_grade[0].clone()
        } else {
            mosaicar(&na_grade, &junto)?;
            junto.clone()
        };
        recortar(&mosaico, AREA, &recortado)?;
        host::run("raster.function", json!({"layer": recortado, "function": "bandMath", "expression": "b1 - 1", "name": saida}))?;
        // As cores da altura do GEDI, para que as duas se comparem.
        let (rampa, cores) = gedi::Variavel::Altura.rampa();
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlGn"}))?;
        }
        let (a, b) = gedi::Variavel::Altura.escala();
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} m", formatar_decimal(m)))
        .unwrap_or_default();
    let piramide = if k > 0 { format!(", lida da pirâmide de {} m do arquivo", formatar_decimal(lido)) } else { String::new() };
    let texto = format!(
        "{saida}: em metros, de {} tile(s) do CHMv2 (Meta e WRI), na grade do projeto, {} ({resampling}){piramide}{media}. O 0 é sem árvore (ou sem dado)",
        ts.len(),
        g.texto()
    );
    avisar(&texto);
    if let Some(a) = grade::aviso_de_reamostragem("a altura do CHMv2", chm::lado_no_chao(0, lat), g.celula, "é a média") {
        avisar(&a);
    }
    Ok(json!({
        "layer": saida, "tiles": ts.len(), "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// A tipologia florestal de 2020 do ForTy a 10 m na área ([`forty`]): a janela da área em cada
/// bloco que ela toca, posta na grade do projeto com a classe mais frequente; os blocos se juntam e
/// o mosaico é recortado pela área.
fn forty_na_area(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela =
        "a tipologia florestal do ForTy precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0)";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let bs = forty::blocos(c);
    if bs.is_empty() {
        return Err("o ForTy não tem dados na área (ele cobre a terra do Brasil; no mar não há blocos)".into());
    }
    if bs.len() > forty::MAX_BLOCOS {
        return Err(format!(
            "a área toca {} blocos do ForTy (cerca de 33 × 45 km cada); o limite de um clique é {}: escolha uma área menor, como um município ou uma microrregião",
            bs.len(),
            forty::MAX_BLOCOS
        ));
    }
    let g = grade_para(c, celula)?;
    conferir_grade_fina(c, g.celula, "a tipologia florestal do ForTy")?;
    // O nível mais grosso que não é mais grosso que a grade, se as janelas cabem no limite do
    // RustGIS; senão, pede uma grade mais grossa.
    let k = forty::nivel(g.celula);
    let lado = forty::lado_do_nivel(k);
    let janelas: Vec<(&forty::Bloco, [f64; 4])> = bs.iter().map(|b| (b, forty::janela(b, lado))).collect();
    let js: Vec<[f64; 4]> = janelas.iter().map(|(_, j)| *j).collect();
    match forty::nivel_que_cabe(&js, k) {
        Some(n) if n == k => {}
        cabe => {
            let dica = cabe
                .map(|n| {
                    format!(": use uma grade de células de {} m ou mais (Grade do projeto), ou uma área menor", forty::lado_do_nivel(n))
                })
                .unwrap_or_else(|| ": escolha uma área menor".into());
            return Err(format!("a área é grande demais para ler o ForTy com células de {lado} m numa janela só por bloco{dica}"));
        }
    }
    // As janelas de um clique anterior servem se cobrem a parte da área (no desktop, o comando
    // roda de novo quando cada uma chega).
    let ha = nomes()?;
    let mut lidas: Vec<String> = Vec::new();
    let mut lendo = 0;
    for (b, j) in &janelas {
        let nome = b.nome_da_janela(k);
        if ha.contains(&nome)
            && let Some(e) = extensao(&nome)?
            && b.a_cobrir().is_none_or(|a| retangulo_cabe(a, e))
        {
            lidas.push(nome);
            continue;
        }
        tirar_janela(&nome)?;
        let mut p = json!({"url": b.url(), "name": nome, "bbox": j, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(lado);
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o bucket do ForTy (storage.googleapis.com) não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            lendo += 1;
            continue;
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&nome)?;
            return Err(sem_janela.into());
        }
        lidas.push(nome);
    }
    if lendo > 0 {
        avisar(&format!("Tipologia florestal (ForTy): lendo a área em {lendo} bloco(s); a camada vem quando chegarem"));
        return Ok(json!({"pending": lendo, "tiles": bs.len()}));
    }
    let saida = forty::CAMADA;
    let resampling = forty::reamostragem(g.celula, lado);
    let na_grade: Vec<String> = (0..lidas.len()).map(|i| format!("{PREFIXO}ForTy na grade {i}")).collect();
    let junto = format!("{PREFIXO}ForTy junto");
    let mut temporarias: Vec<&str> = lidas.iter().chain(&na_grade).map(String::as_str).collect();
    temporarias.push(junto.as_str());
    remover(&[saida])?;
    let feito = (|| -> Result<(), String> {
        for (j, n) in lidas.iter().zip(&na_grade) {
            host::run(
                "raster.project",
                json!({"layer": j, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": n}),
            )?;
            // Só as classes de 1 a 6: o 255 (sem dado) e o que fica fora da janela somem, para não
            // cobrir o bloco vizinho no mosaico.
            host::run("raster.mask", json!({"layer": n, "min": 0.5, "max": 6.5}))?;
        }
        let mosaico = if na_grade.len() == 1 {
            na_grade[0].clone()
        } else {
            mosaicar(&na_grade, &junto)?;
            junto.clone()
        };
        recortar(&mosaico, AREA, saida)?;
        host::run("style.importQgis", json!({"layer": saida, "text": forty::qml()}))?;
        Ok(())
    })();
    remover(&temporarias)?;
    feito?;
    let piramide = if k > 0 { format!(", lida da pirâmide de {lado} m do arquivo") } else { String::new() };
    let texto = format!(
        "{saida}: classes do ForTy 2020 (Nature Trace: Google DeepMind, WRI e IIASA), de {} bloco(s), na grade do projeto, {} ({}){piramide}. Códigos: {}",
        bs.len(),
        g.texto(),
        if resampling == "mode" { "a classe mais frequente" } else { "a célula mais próxima" },
        forty::CHAVE
    );
    avisar(&texto);
    if let Some(a) = grade::aviso_de_reamostragem("o ForTy", forty::LADO, g.celula, "fica com a classe mais frequente") {
        avisar(&a);
    }
    Ok(json!({
        "layer": saida, "tiles": bs.len(), "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// A biomassa acima do solo da CTrees a 100 m num ano de 2000 a 2025, ou a incerteza dela
/// ([`ctrees`]): a janela da área no COG global do ano, em Mg/ha, posta na grade do projeto e
/// recortada pela área.
fn ctrees_na_area(req: &Request) -> Result<Value, String> {
    let ano = req.f64("ano").map(ctrees::conferir_ano).transpose()?.unwrap_or(ctrees::ULTIMO_ANO);
    let v = match req.str("camada") {
        Some(t) => ctrees::Variavel::ler(t).ok_or(format!("camada '{t}': use biomassa ou incerteza"))?,
        None => ctrees::Variavel::Biomassa,
    };
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let sem_janela = "a biomassa da CTrees precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): o arquivo é o mundo inteiro, com 405.000 × 202.500 células";
    if !le_janela_remota()? {
        return Err(sem_janela.into());
    }
    let g = grade_para(c, celula)?;
    let lat = (c[1] + c[3]) / 2.0;
    let k = match ctrees::nivel_da_leitura(c, g.celula, lat) {
        Some(k) => k,
        None => return Err("a área é grande demais para a biomassa da CTrees: escolha uma área menor".into()),
    };
    let janela = ctrees::janela(c, k);
    let bruto = v.janela(ano, k);
    // A janela de um clique anterior serve se cobre a área (no desktop, o comando roda de novo
    // quando ela chega).
    let serve = nomes()?.contains(&bruto) && extensao(&bruto)?.is_some_and(|e| retangulo_cabe(c, e));
    if !serve {
        tirar_janela(&bruto)?;
        let mut p = json!({"url": v.url(ano), "name": bruto, "bbox": janela, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(cog_global::celula_pedida(ctrees::graus_do_nivel(k)));
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("o bucket da CTrees (AWS) não respondeu agora: tente de novo daqui a pouco ({e})")
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!("Biomassa da CTrees ({ano}): lendo a área no arquivo global; a camada vem quando chegar"));
            return Ok(json!({"pending": 1}));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&bruto)?;
            return Err(sem_janela.into());
        }
    }
    let saida = v.camada(ano);
    let sem_mar = format!("{PREFIXO}CTrees sem o mar");
    let em_mg = format!("{PREFIXO}CTrees em Mg/ha");
    let na_grade = format!("{PREFIXO}CTrees na grade");
    let lido = ctrees::metros_do_nivel(k, lat);
    let resampling = ctrees::reamostragem(g.celula, lido);
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        // O que é negativo (o mar, e a mistura dele com a terra na pirâmide) vira o −9999, e o −9999
        // vira sem dado na conta, para não entrar na reamostragem nem na média.
        host::run(
            "raster.function",
            json!({"layer": bruto, "function": "remap", "ranges": [[-1e6, 0, -9999]], "keep": true, "name": sem_mar}),
        )?;
        host::run("raster.function", json!({"layer": sem_mar, "function": "bandMath", "expression": ctrees::EM_MG, "name": em_mg}))?;
        host::run(
            "raster.project",
            json!({"layer": em_mg, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": na_grade}),
        )?;
        recortar(&na_grade, AREA, &saida)?;
        host::run("raster.mask", json!({"layer": saida, "min": 0.0, "max": 3000.0}))?;
        let (rampa, cores) = v.rampa();
        let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
        if host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": rampa})).is_err() {
            host::run("raster.symbology", json!({"layer": saida, "type": "stretch", "ramp": "YlGn"}))?;
        }
        let (a, b) = v.escala();
        let _ = host::run("raster.stretch", json!({"layer": saida, "type": "custom", "min": a, "max": b}));
        Ok(())
    })();
    remover(&[&bruto, &sem_mar, &em_mg, &na_grade])?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {} Mg/ha", formatar_decimal(m)))
        .unwrap_or_default();
    let piramide = if k > 0 { format!(", lida da pirâmide de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    let o_que = if v == ctrees::Variavel::Biomassa { "biomassa acima do solo" } else { "incerteza (erro-padrão) da biomassa" };
    let texto = format!("{saida}: {o_que} em Mg/ha de {ano} (CTrees), na grade do projeto, {} ({resampling}){piramide}{media}", g.texto());
    avisar(&texto);
    if let Some(a) = grade::aviso_de_reamostragem("a biomassa da CTrees", ctrees::metros_do_nivel(0, lat), g.celula, "é a média") {
        avisar(&a);
    }
    Ok(json!({
        "layer": saida, "year": ano, "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// O que se lê de um COG global ([`cog_global`]) e o que se faz com o valor.
struct LeituraGlobal<'a> {
    arquivo: cog_global::Arquivo,
    /// O nome da camada de passagem com a janela lida no nível `k`.
    janela: &'a dyn Fn(i32) -> String,
    saida: String,
    /// Por quanto o arquivo multiplica o valor (1: o valor como está).
    divisor: f64,
    /// Os valores que ficam, depois da divisão.
    faixa: (f64, f64),
    /// Se é uma classe (a classe mais frequente numa grade mais grossa, a mais próxima senão).
    classes: bool,
    /// O dado, nas mensagens ("o pH do solo do OpenLandMap").
    dado: &'a str,
    /// O servidor, nas mensagens.
    servidor: &'a str,
}

/// Pede a janela da área `c` de um COG global, na camada de passagem, se ela ainda não está no mapa.
/// Devolve o nível de pirâmide lido; ou o `pending` de uma leitura que ainda vem (no desktop, o
/// comando roda de novo quando ela chega). Um comando que lê dois arquivos pede as duas janelas
/// antes de mexer no mapa: depois disso, o RustGIS recusa a nova rodada.
fn pedir_janela_global(l: &LeituraGlobal, c: [f64; 4], g: &Grade) -> Result<Result<i32, Value>, String> {
    let sem_janela = format!(
        "{} precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0): o arquivo é o mundo inteiro",
        l.dado
    );
    if !le_janela_remota()? {
        return Err(sem_janela);
    }
    let lat = (c[1] + c[3]) / 2.0;
    let a = &l.arquivo;
    let k = match a.nivel_que_cabe(c, a.nivel(g.celula, lat)) {
        Some(k) => k,
        None if l.classes => {
            return Err(format!(
                "a área é grande demais para {}, que se lê sempre da imagem inteira: escolha uma área menor ou uma escala maior",
                l.dado
            ));
        }
        None => return Err(format!("a área é grande demais para {}: escolha uma área menor", l.dado)),
    };
    let janela = a.janela(c, k);
    let bruto = (l.janela)(k);
    // A janela de um clique anterior serve se cobre a área.
    let serve = nomes()?.contains(&bruto) && extensao(&bruto)?.is_some_and(|e| retangulo_cabe(c, e));
    if !serve {
        tirar_janela(&bruto)?;
        let mut p = json!({"url": a.url, "name": bruto, "bbox": janela, "zoom": false});
        if k > 0 {
            p["cellSize"] = json!(cog_global::celula_pedida(a.graus_do_nivel(k)));
        }
        let r = host::run("layer.addUrl", p).map_err(|e| {
            if falha_passageira(&e) || e.contains("timeout") {
                format!("{} não respondeu agora: tente de novo daqui a pouco ({e})", l.servidor)
            } else {
                e
            }
        })?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!("{}: lendo a área no arquivo global; a camada vem quando chegar", maiuscula(l.dado)));
            return Ok(Err(json!({"pending": 1})));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            tirar_janela(&bruto)?;
            return Err(sem_janela);
        }
    }
    Ok(Ok(k))
}

/// A janela da área `c` de um COG global, posta na grade `g`, dividida, recortada pela área e
/// limpa dos valores fora da faixa, na camada `l.saida`. Devolve o nível de pirâmide lido, a
/// célula dele em metros e a reamostragem; ou o `pending` de uma leitura que ainda vem.
fn ler_cog_global(l: &LeituraGlobal, c: [f64; 4], g: &Grade) -> Result<Result<(i32, f64, &'static str), Value>, String> {
    let k = match pedir_janela_global(l, c, g)? {
        Ok(k) => k,
        Err(p) => return Ok(Err(p)),
    };
    let (a, bruto) = (&l.arquivo, (l.janela)(k));
    let lido = a.metros_do_nivel(k, (c[1] + c[3]) / 2.0);
    let resampling = if l.classes { forty::reamostragem(g.celula, lido) } else { cog_global::reamostragem(g.celula, lido) };
    let na_unidade = format!("{PREFIXO}OpenGeoHub na unidade");
    let na_grade = format!("{PREFIXO}OpenGeoHub na grade");
    remover(&[&l.saida])?;
    let feito = (|| -> Result<(), String> {
        let fonte = if l.divisor == 1.0 {
            bruto.clone()
        } else {
            host::run(
                "raster.function",
                json!({"layer": bruto, "function": "bandMath", "expression": format!("b1 / {}", l.divisor), "name": na_unidade}),
            )?;
            na_unidade.clone()
        };
        host::run(
            "raster.project",
            json!({"layer": fonte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": na_grade}),
        )?;
        recortar(&na_grade, AREA, &l.saida)?;
        host::run("raster.mask", json!({"layer": l.saida, "min": l.faixa.0, "max": l.faixa.1}))?;
        Ok(())
    })();
    remover(&[&bruto, &na_unidade, &na_grade])?;
    feito?;
    Ok(Ok((k, lido, resampling)))
}

/// `t` com a primeira letra maiúscula.
fn maiuscula(t: &str) -> String {
    let mut c = t.chars();
    c.next().map(|p| p.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// As luzes noturnas de um ano ([`luzes`]).
fn leitura_das_luzes<'a>(ano: i64, saida: String, janela: &'a dyn Fn(i32) -> String) -> LeituraGlobal<'a> {
    LeituraGlobal {
        arquivo: luzes::arquivo(ano),
        janela,
        saida,
        divisor: 1.0,
        faixa: luzes::FAIXA,
        classes: false,
        dado: "as luzes noturnas",
        servidor: "o GitHub, onde ficam os arquivos das luzes noturnas,",
    }
}

/// A área e a grade, com a célula pedida se não há uma.
fn area_e_grade(req: &Request) -> Result<([f64; 4], Grade), String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    Ok((c, grade_para(c, celula)?))
}

/// As cores da camada `nome`, com a rampa do plugin (ou a `reserva`, se o RustGIS não a aceita) na
/// `escala`.
fn cores_na_escala(nome: &str, (rampa, cores): (&str, &str), reserva: &str, (a, b): (f64, f64)) -> Result<(), String> {
    let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
    if host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": rampa})).is_err() {
        host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": reserva}))?;
    }
    let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": a, "max": b}));
    Ok(())
}

/// O texto da visão lida e da grade, para o aviso.
fn texto_da_leitura(k: i32, lido: f64, resampling: &str, g: &Grade) -> String {
    let visao = if k > 0 { format!(", lidas da visão de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    format!("na grade do projeto, {} ({resampling}){visao}", g.texto())
}

/// As luzes noturnas de um ano ([`luzes`]): a janela da área no COG do Brasil, posta na grade do
/// projeto e recortada pela área.
fn luzes_na_area(req: &Request) -> Result<Value, String> {
    let ano = req.f64("ano").map(luzes::conferir_ano).transpose()?.unwrap_or(luzes::ULTIMO_ANO);
    let (c, g) = area_e_grade(req)?;
    let janela = |k: i32| luzes::janela_nome(ano, k);
    let l = leitura_das_luzes(ano, luzes::camada(ano), &janela);
    let (k, lido, resampling) = match ler_cog_global(&l, c, &g)? {
        Ok(r) => r,
        Err(pendente) => return Ok(pendente),
    };
    cores_na_escala(&l.saida, luzes::RAMPA, "Inferno", luzes::ESCALA)?;
    let sensor = if ano < luzes::PRIMEIRO_DO_VIIRS { "sensor DMSP" } else { "sensor VIIRS, na escala da série" };
    let texto = format!(
        "{}: {sensor}, número relativo, sem unidade, {}{}",
        l.saida,
        texto_da_leitura(k, lido, resampling, &g),
        media_na_area(&l.saida, "")
    );
    avisar(&texto);
    Ok(json!({
        "layer": l.saida, "year": ano, "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// A mudança das luzes noturnas entre dois anos ([`luzes`]): o mais novo menos o mais antigo, na grade
/// do projeto.
fn mudanca_das_luzes(req: &Request) -> Result<Value, String> {
    let inicio = req.f64("inicio").map(luzes::conferir_ano).transpose()?.unwrap_or(2014);
    let fim = req.f64("fim").map(luzes::conferir_ano).transpose()?.unwrap_or(luzes::ULTIMO_ANO);
    if inicio >= fim {
        return Err(format!("o primeiro ano ({inicio}) vem antes do último ({fim})"));
    }
    let (c, g) = area_e_grade(req)?;
    let antes = format!("{PREFIXO}luzes antes");
    let depois = format!("{PREFIXO}luzes depois");
    let (ja, jb) = (|k: i32| luzes::janela_nome(inicio, k), |k: i32| luzes::janela_nome(fim, k));
    let (la, lb) = (leitura_das_luzes(inicio, antes.clone(), &ja), leitura_das_luzes(fim, depois.clone(), &jb));
    // As duas janelas antes de mexer no mapa.
    let pedidos = [pedir_janela_global(&la, c, &g)?, pedir_janela_global(&lb, c, &g)?];
    if let Some(p) = pedidos.into_iter().find_map(Result::err) {
        return Ok(p);
    }
    let saida = luzes::camada_mudanca(inicio, fim);
    remover(&[&saida])?;
    let feito = (|| -> Result<Result<(i32, f64, &'static str), Value>, String> {
        if let Err(p) = ler_cog_global(&la, c, &g)? {
            return Ok(Err(p));
        }
        let r = ler_cog_global(&lb, c, &g)?;
        if r.is_ok() {
            diferenca(&antes, &depois, &saida)?;
        }
        Ok(r)
    })();
    remover(&[&antes, &depois])?;
    let (k, lido, resampling) = match feito? {
        Ok(r) => r,
        Err(pendente) => return Ok(pendente),
    };
    cores_na_escala(&saida, luzes::RAMPA_MUDANCA, "PuOr", luzes::ESCALA_MUDANCA)?;
    let texto = format!(
        "{saida}: {fim} menos {inicio}, número relativo, sem unidade, {}{}",
        texto_da_leitura(k, lido, resampling, &g),
        media_na_area(&saida, "")
    );
    avisar(&texto);
    if luzes::troca_de_sensor(inicio, fim) {
        avisar(
            "Luzes noturnas: o período passa de 2013 para 2014, quando o sensor muda do DMSP para o VIIRS: a série os corrige um pelo outro, mas parte da diferença pode vir da troca. Para comparar anos do mesmo sensor, fique até 2013 ou a partir de 2014",
        );
    }
    Ok(json!({
        "layer": saida, "from": inicio, "to": fim, "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// Uma propriedade do solo do OpenLandMap-soildb a 30 m ([`olm`]): a janela da área no COG global
/// da propriedade, da profundidade e do período, na unidade, posta na grade do projeto e recortada
/// pela área.
fn olm_solo_na_area(req: &Request) -> Result<Value, String> {
    let p = olm::Propriedade::ler(req.str("propriedade"))?;
    let prof = olm::ler_profundidade(req.str("profundidade"))?;
    let per = olm::ler_periodo(req.str("periodo"), p)?;
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let g = grade_para(c, celula)?;
    let dado = format!("{} do OpenLandMap", minuscula_inicial(p.nome()));
    let janela = |k: i32| olm::janela_nome(p, prof, per, k);
    let l = LeituraGlobal {
        arquivo: olm::arquivo(p, prof, per),
        janela: &janela,
        saida: olm::camada(p, prof, per),
        divisor: p.divisor(),
        faixa: p.faixa(),
        classes: false,
        dado: &dado,
        servidor: "o servidor da OpenGeoHub (ceph.opengeohub.org)",
    };
    let (k, lido, resampling) = match ler_cog_global(&l, c, &g)? {
        Ok(r) => r,
        Err(pendente) => return Ok(pendente),
    };
    let (rampa, cores) = p.rampa();
    let _ = host::run("ramp.add", json!({"text": cores, "format": "gdal", "name": rampa}));
    if host::run("raster.symbology", json!({"layer": l.saida, "type": "stretch", "ramp": rampa})).is_err() {
        host::run("raster.symbology", json!({"layer": l.saida, "type": "stretch", "ramp": "YlOrBr"}))?;
    }
    let (a, b) = p.escala();
    let _ = host::run("raster.stretch", json!({"layer": l.saida, "type": "custom", "min": a, "max": b}));
    let unidade = if p == olm::Propriedade::Ph { "" } else { p.unidade() };
    let media = media_na_area(&l.saida, unidade);
    let piramide = if k > 0 { format!(", lida da pirâmide de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    let em = if unidade.is_empty() { String::new() } else { format!(" em {unidade}") };
    let texto = format!(
        "{}: {}{em} de {}–{} cm, {}–{} (OpenLandMap-soildb), na grade do projeto, {} ({resampling}){piramide}{media}",
        l.saida,
        minuscula_inicial(p.nome()),
        prof.0,
        prof.1,
        per.0,
        per.1,
        g.texto()
    );
    avisar(&texto);
    let lat = (c[1] + c[3]) / 2.0;
    if let Some(a) = grade::aviso_de_reamostragem(&dado, l.arquivo.metros_do_nivel(0, lat), g.celula, "é a média") {
        avisar(&a);
    }
    Ok(json!({
        "layer": l.saida, "property": p.valor(), "depth": format!("{}-{}", prof.0, prof.1), "period": format!("{}-{}", per.0, per.1),
        "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula, "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// `t` com a primeira letra minúscula, menos numa sigla ("pH", "TWI").
fn minuscula_inicial(t: &str) -> String {
    let mut c = t.chars();
    match (c.next(), c.next()) {
        (Some(a), Some(b)) if !b.is_uppercase() => a.to_lowercase().chain(std::iter::once(b)).chain(c).collect(),
        _ => t.to_string(),
    }
}

/// Uma variável do relevo do GEDTM30 ([`gedtm`]): a janela da área no COG global da variável e
/// da escala, na unidade, posta na grade do projeto e recortada pela área.
fn gedtm_na_area(req: &Request) -> Result<Value, String> {
    let v = gedtm::Variavel::ler(req.str("variavel"))?;
    let pedida = gedtm::ler_escala(
        req.f64("escala").or_else(|| req.str("escala").and_then(|t| t.trim().trim_end_matches('m').trim().parse().ok())),
    )?;
    let (escala, troca) = gedtm::escala_lida(v, pedida);
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    let g = grade_para(c, celula)?;
    let dado = format!("{} do GEDTM30", minuscula_inicial(v.nome()));
    let janela = |k: i32| gedtm::janela_nome(v, escala, k);
    let arquivo = v.arquivo(escala);
    let host = arquivo.url.split('/').nth(2).unwrap_or("opengeohub.org");
    let quem = if host.contains("opentopography") { "OpenTopography" } else { "OpenGeoHub" };
    let servidor = format!("o servidor da {quem} ({host})");
    let l = LeituraGlobal {
        arquivo: arquivo.clone(),
        janela: &janela,
        saida: gedtm::camada(v, escala),
        divisor: v.divisor(),
        faixa: v.faixa(),
        classes: v.classes(),
        dado: &dado,
        servidor: &servidor,
    };
    let (k, lido, resampling) = match ler_cog_global(&l, c, &g)? {
        Ok(r) => r,
        Err(pendente) => return Ok(pendente),
    };
    let mut chave = String::new();
    if v.classes() {
        host::run("style.importQgis", json!({"layer": l.saida, "text": gedtm::qml_geomorfons()}))?;
        chave = format!(". Códigos: {}", gedtm::chave_geomorfons());
    } else {
        let (rampa, invertida) = v.rampa();
        host::run("raster.symbology", json!({"layer": l.saida, "type": "stretch", "ramp": rampa, "invert": invertida}))?;
    }
    let media = if v.classes() { String::new() } else { media_na_area(&l.saida, v.unidade()) };
    let piramide = if k > 0 { format!(", lida da pirâmide de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    let unidade = if v.unidade().is_empty() || v.classes() { String::new() } else { format!(" em {}", v.unidade()) };
    let texto = format!(
        "{}: {}{unidade} na escala de {escala} m (GEDTM30), na grade do projeto, {} ({resampling}){piramide}{media}{chave}",
        l.saida,
        minuscula_inicial(v.nome()),
        g.texto()
    );
    avisar(&texto);
    if let Some(a) = &troca {
        avisar(a);
    }
    let lat = (c[1] + c[3]) / 2.0;
    let mais_grossa = if v.classes() { "fica com a classe mais frequente" } else { "é a média" };
    if let Some(a) = grade::aviso_de_reamostragem(&dado, l.arquivo.metros_do_nivel(0, lat), g.celula, mais_grossa) {
        avisar(&a);
    }
    Ok(json!({
        "layer": l.saida, "variable": v.valor(), "scale": escala, "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        // A troca de escala vai também no resultado da janela do comando, não só nos avisos.
        "report": match &troca {
            Some(a) => format!("{}\n\n{a}", texto.trim_start_matches(PREFIXO)),
            None => texto.trim_start_matches(PREFIXO).to_string(),
        },
    }))
}

/// O metano no ar ([`metano`]): a coluna média de metano do Sentinel-5P/TROPOMI do período, da
/// grade de 0,1° do BrasilData. Numa área, só a janela dela, lida do COG e recortada pelo contorno;
/// sem área, o Brasil inteiro, lido pela vista.
fn metano_na_area(req: &Request) -> Result<Value, String> {
    let (ano, mes) = metano::ler_periodo(req.str("periodo"))?;
    let url = metano::url(ano, mes);
    let nome = metano::camada(ano, mes);
    let periodo = metano::periodo_texto(ano, mes);
    let ha = nomes()?;
    let fonte = format!("contém dados modificados do Copernicus Sentinel {ano} (ESA), processados pelo BrasilData");
    if !ha.iter().any(|n| n == AREA) {
        remover(&[&nome])?;
        let r = host::run("layer.addUrl", json!({"url": url, "name": nome, "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!("Metano no ar ({periodo}): abrindo o arquivo do Brasil; a camada vem quando chegar"));
            return Ok(json!({"pending": 1}));
        }
        estilo_do_metano(&nome)?;
        sob_as_linhas(&nome)?;
        let texto = format!(
            "{}: coluna média de metano (XCH4, ppb) do Sentinel-5P/TROPOMI em {periodo}, em células de 0,1° (cerca de 11 km), no Brasil inteiro; {fonte}. {}",
            nome.trim_start_matches(PREFIXO),
            metano::VAZIOS
        );
        avisar(&texto);
        return Ok(json!({"layer": nome, "report": texto}));
    }
    if !le_janela_remota()? {
        return Err("o metano precisa de um RustGIS que leia só a janela da área de um arquivo remoto (depois da 0.5.0)".into());
    }
    let (c, _) = retangulo()?;
    let bruto = format!("{nome} (janela)");
    // A janela de um clique anterior serve se cobre a área.
    let serve = ha.contains(&bruto) && extensao(&bruto)?.is_some_and(|e| retangulo_cabe(c, e));
    if !serve {
        tirar_janela(&bruto)?;
        let r = host::run("layer.addUrl", json!({"url": url, "name": bruto, "bbox": metano::janela(c), "zoom": false}))?;
        if r.get("pending").and_then(Value::as_bool) == Some(true) {
            avisar(&format!("Metano no ar ({periodo}): lendo a área no arquivo do Brasil; a camada vem quando chegar"));
            return Ok(json!({"pending": 1}));
        }
    }
    // Mascarada pelo contorno, sem recorte: o recorte guarda só as células com o centro na área, e
    // as de 11 km deixavam uma faixa vazia na divisa.
    remover(&[&nome])?;
    host::run("layer.rename", json!({"layer": bruto, "name": nome}))?;
    let _ = host::run("layer.mask", json!({"layer": nome, "mask": AREA}));
    estilo_do_metano(&nome)?;
    sob_as_linhas(&nome)?;
    let texto = format!(
        "{}: coluna média de metano (XCH4, ppb) do Sentinel-5P/TROPOMI em {periodo}, em células de 0,1° (cerca de 11 km), mascarada pelo contorno da área; {fonte}. Mede o metano no ar, não a emissão de cada lugar: o valor cai com a altitude. {}",
        nome.trim_start_matches(PREFIXO),
        metano::VAZIOS
    );
    avisar(&texto);
    Ok(json!({"layer": nome, "year": ano, "month": mes, "report": texto}))
}

/// Pede `pedido` (um `layer.addUrl` de uma janela do NDVI do Copernicus) até `vezes` vezes, a cada vez
/// no endereço que `url` dá (ver [`ndvi_cdse`]). Devolve a vez que deu certo; ou o `pending` de uma
/// leitura que ainda vem (no desktop, o comando roda de novo quando ela chega).
fn insistir(pedido: &Value, vezes: i32, url: impl Fn(i32) -> String) -> Result<Result<i32, Value>, String> {
    let mut i = 1;
    loop {
        let mut p = pedido.clone();
        p["url"] = json!(url(i));
        let e = match host::run("layer.addUrl", p) {
            Ok(r) if pendente(&r) => return Ok(Err(r)),
            Ok(r) if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() => {
                tirar_janela(pedido["name"].as_str().unwrap_or_default())?;
                return Err("o NDVI do Copernicus precisa de um RustGIS que leia só a janela da área: o arquivo é o mundo inteiro".into());
            }
            Ok(_) => return Ok(Ok(i)),
            Err(e) => e,
        };
        if e.contains("http status: 401") || e.contains("http status: 403") {
            return Err(format!(
                "o Copernicus Data Space recusou o login: entre de novo em Início › Dados web › Copernicus Data Space ({e})"
            ));
        }
        if !ndvi_cdse::conexao_caiu(&e) {
            return Err(e);
        }
        if i == vezes {
            return Err(format!(
                "o servidor de arquivos do Copernicus recusou ou derrubou {i} pedidos seguidos: espere um minuto e clique de novo ({e})"
            ));
        }
        i += 1;
    }
}

/// A camada de passagem com uma célula só de `bruto`, para o cabeçalho do arquivo.
fn sonda_do_ndvi(bruto: &str) -> String {
    format!("{bruto}, sonda")
}

/// A janela da área `c` no nível `k` do NDVI do Copernicus, na camada de passagem `bruto`, se ela ainda
/// não está no mapa; ou o `pending` de uma leitura que ainda vem. Primeiro uma célula só, mudando de
/// endereço até o cabeçalho do arquivo vir; depois a janela, sempre no endereço que respondeu: cada
/// vez só pede os blocos que faltam.
fn janela_do_ndvi(p: &ndvi_cdse::Produto, c: [f64; 4], k: i32, bruto: &str) -> Result<Result<(), Value>, String> {
    if nomes()?.iter().any(|n| n == bruto) && extensao(bruto)?.is_some_and(|e| retangulo_cabe(c, e)) {
        return Ok(Ok(()));
    }
    tirar_janela(bruto)?;
    let a = ndvi_cdse::arquivo(p.url.clone());
    let pedido = |nome: &str, caixa: [f64; 4]| {
        let mut v = json!({"name": nome, "cog": true, "bbox": caixa, "zoom": false});
        if k > 0 {
            v["cellSize"] = json!(cog_global::celula_pedida(a.graus_do_nivel(k)));
        }
        v
    };
    let sonda = sonda_do_ndvi(bruto);
    let aviso = |v: Value| {
        avisar("NDVI do Copernicus: lendo a área no arquivo global; a camada vem quando chegar");
        Ok(Err(v))
    };
    let j = match insistir(&pedido(&sonda, ndvi_cdse::uma_celula(&a, c, k)), ndvi_cdse::TENTATIVAS, |i| ndvi_cdse::endereco(&p.url, i))? {
        Ok(j) => j,
        Err(v) => return aviso(v),
    };
    let lida = insistir(&pedido(bruto, a.janela(c, k)), ndvi_cdse::TENTATIVAS_DA_JANELA, |_| ndvi_cdse::endereco(&p.url, j));
    if !matches!(lida, Ok(Err(_))) {
        remover(&[&sonda])?;
    }
    match lida? {
        Ok(_) => Ok(Ok(())),
        Err(v) => aviso(v),
    }
}

/// O NDVI de 10 em 10 dias a 300 m do Copernicus ([`ndvi_cdse`]): o produto mais recente, ou o do dia
/// pedido, lido do Copernicus Data Space com o login do RustGIS.
fn ndvi_copernicus(req: &Request) -> Result<Value, String> {
    const SEM_LOGIN: &str = "o NDVI do Copernicus pede o login do Copernicus Data Space, que é grátis: entre em Início › Dados web › Copernicus Data Space (a conta se cria em dataspace.copernicus.eu)";
    let dia = match req.str("data") {
        Some(t) => Some(bdc::ler_data(t).ok_or(format!("data '{t}': use dia/mês/ano, como 15/10/2026"))?),
        None => None,
    };
    if dia.is_some_and(|d| d < ndvi_cdse::PRIMEIRO) {
        return Err(format!("o NDVI do Copernicus a 300 m começa em {}", ndvi_cdse::PRIMEIRO.texto()));
    }
    let (c, g) = area_e_grade(req)?;
    // O login guardado neste computador (remembered) entra sozinho quando o arquivo é pedido.
    let s = host::run("copernicus.status", json!({})).map_err(|e| {
        format!("o NDVI do Copernicus precisa de um RustGIS com o login do Copernicus Data Space (0.7.0 ou mais novo): {e}")
    })?;
    if s["signedIn"].as_bool() != Some(true) && s["remembered"].as_bool() != Some(true) {
        return Err(SEM_LOGIN.into());
    }
    let r = match buscar_varios(&[ndvi_cdse::url_busca(dia)])? {
        Ok(r) => r,
        Err(p) => return Ok(p),
    };
    let p = ndvi_cdse::produto(&r[0])
        .ok_or("o catálogo do Copernicus Data Space não respondeu com um NDVI de 300 m: tente de novo daqui a pouco")?;
    let (a, lat) = (ndvi_cdse::arquivo(p.url.clone()), (c[1] + c[3]) / 2.0);
    let k = ndvi_cdse::nivel(&a, c).ok_or("a área é grande demais para o NDVI do Copernicus: escolha uma área menor")?;
    let bruto = format!("{PREFIXO}NDVI Copernicus {} (janela {k})", p.inicio.iso());
    if let Err(pendente) = janela_do_ndvi(&p, c, k, &bruto)? {
        return Ok(pendente);
    }
    let saida = ndvi_cdse::camada(p.inicio, p.fim);
    let (conta, na_grade) = (format!("{PREFIXO}NDVI na unidade"), format!("{PREFIXO}NDVI na grade"));
    let lido = a.metros_do_nivel(k, lat);
    let resampling = cog_global::reamostragem(g.celula, lido);
    remover(&[&saida])?;
    let feito = (|| -> Result<(), String> {
        host::run(
            "raster.functionChain",
            json!({"layer": bruto, "name": conta, "steps": [
                {"function": "bandMath", "expression": "b1 + 1"},
                {"function": "remap", "ranges": [[251.5, 1e9, 0]], "keep": true},
                {"function": "bandMath", "expression": ndvi_cdse::CONTA},
            ]}),
        )?;
        // (o `raster.georeference` é da extensão Imagens)
        if k > 0
            && let Some(e) = extensao(&conta)?
            && let Err(e) = host::run("raster.georeference", json!({"layer": conta, "extent": ndvi_cdse::posta_no_lugar(&a, e, k)}))
        {
            host::log(&format!("NDVI do Copernicus, o ajuste da posição: {e}"));
            avisar(
                "NDVI do Copernicus: nesta área, lida da visão reduzida do arquivo, as células ficam cerca de 0,5 km a sudeste do lugar; com a extensão Imagens ligada (Plugins › Extensões), o plugin corrige",
            );
        }
        host::run(
            "raster.project",
            json!({"layer": conta, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": resampling, "name": na_grade}),
        )?;
        recortar(&na_grade, AREA, &saida)?;
        // As mesmas cores e escala do NDVI do cubo do INPE, para que se comparem.
        let (rampa, cores) = bdc::rampa(bdc::Indice::Ndvi);
        cores_na_escala(&saida, (rampa, &cores), "RdYlGn", bdc::Indice::Ndvi.escala())
    })();
    // No desktop, a janela chega com a sonda ainda no mapa.
    remover(&[&bruto, &sonda_do_ndvi(&bruto), &conta, &na_grade])?;
    feito?;
    let media = host::run("raster.info", json!({"layer": saida}))
        .ok()
        .and_then(|i| i["bands"][0]["mean"].as_f64())
        .map(|m| format!("; média na área: {}", format!("{m:.2}").replace('.', ",")))
        .unwrap_or_default();
    let texto = format!(
        "{}: {}, {}{}",
        saida.trim_start_matches(PREFIXO),
        ndvi_cdse::satelite(p.inicio),
        texto_da_leitura(k, lido, resampling, &g),
        media
    );
    avisar(&texto);
    Ok(json!({
        "layer": saida, "start": p.inicio.iso(), "end": p.fim.iso(), "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto,
    }))
}

/// Põe a camada `nome` (um raster, que chega no topo) logo abaixo das camadas de linhas e de pontos
/// que estão no topo do mapa (rios, estradas, localidades), que ele cobriria.
fn sob_as_linhas(nome: &str) -> Result<(), String> {
    let info = host::run("info.project", json!({}))?;
    let acima = info["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["name"].as_str() != Some(nome))
        .take_while(|l| l["type"].as_str().is_some_and(|t| t.contains("line") || t.contains("Point")))
        .count();
    if acima > 0 {
        host::run("layer.move", json!({"layer": nome, "to": acima}))?;
    }
    Ok(())
}

/// As cores do metano: do amarelo ao vermelho, na faixa fixa [`metano::FAIXA`] (a mesma em todo
/// lugar e todo período, para comparar).
fn estilo_do_metano(nome: &str) -> Result<(), String> {
    host::run("raster.symbology", json!({"layer": nome, "type": "stretch", "ramp": "Ember"}))?;
    let _ = host::run("raster.stretch", json!({"layer": nome, "type": "custom", "min": metano::FAIXA.0, "max": metano::FAIXA.1}));
    Ok(())
}

/// O terreno da América do Sul a 30 m do ANADEM ([`anadem`]): a janela da área no COG do S3 da
/// OpenTopography, posta na grade do projeto e recortada pela área.
fn anadem_na_area(req: &Request) -> Result<Value, String> {
    let celula = req.f64("celula");
    if let Some(m) = celula {
        grade::conferir_celula(m)?;
    }
    let (c, na_area) = retangulo()?;
    if !na_area {
        return Err("escolha a área primeiro (painel Área de trabalho, ou Área: seleção)".into());
    }
    // O ANADEM cobre a América do Sul: de 82,5° O a 34,7° O e de 56,5° S a 14,1° N.
    if c[2] < -82.52 || c[0] > -34.72 || c[3] < -56.52 || c[1] > 14.08 {
        return Err("o ANADEM cobre só a América do Sul: a área está fora dele".into());
    }
    let g = grade_para(c, celula)?;
    let l = LeituraGlobal {
        arquivo: anadem::arquivo(),
        janela: &anadem::janela_nome,
        saida: anadem::CAMADA.to_string(),
        divisor: 1.0,
        faixa: anadem::FAIXA,
        classes: false,
        dado: "o terreno do ANADEM",
        servidor: "o servidor da OpenTopography (opentopography.s3.sdsc.edu)",
    };
    let (k, lido, resampling) = match ler_cog_global(&l, c, &g)? {
        Ok(r) => r,
        Err(pendente) => return Ok(pendente),
    };
    host::run("raster.symbology", json!({"layer": l.saida, "type": "stretch", "ramp": "Terrain"}))?;
    let media = media_na_area(&l.saida, "m");
    let piramide = if k > 0 { format!(", lida da pirâmide de cerca de {} m do arquivo", lido.round()) } else { String::new() };
    let texto = format!(
        "{}: altitude do terreno nu em m (ANADEM, ANA e IPH/UFRGS, pela OpenTopography), na grade do projeto, {} ({resampling}){piramide}{media}",
        l.saida,
        g.texto()
    );
    avisar(&texto);
    let lat = (c[1] + c[3]) / 2.0;
    if let Some(a) = grade::aviso_de_reamostragem("o terreno do ANADEM", l.arquivo.metros_do_nivel(0, lat), g.celula, "é a média") {
        avisar(&a);
    }
    Ok(json!({
        "layer": l.saida, "level": k, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "report": texto.trim_start_matches(PREFIXO),
    }))
}

/// O botão Layout: uma folha pronta com o dado principal do BrasilData no mapa (a primeira camada
/// visível com receita, [`layout::receita`]), o nome da área no título, as fontes das camadas e a
/// projeção da grade. Na folha do RustGIS (`layout.publication`), a legenda deixa de fora a área e
/// a grade, e a barra de cores ganha o título com a unidade.
fn layout_do_mapa(req: &Request) -> Result<Value, String> {
    // A folha em pé, deitada ou a que der o mapa maior: conferida antes de mexer no mapa.
    let perfil = layout::perfil_com_orientacao(req.str("perfil").unwrap_or("a4"), req.str("orientacao"))?;
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
    // Os mapas do BrasilData para ver (os mosaicos do MapBiomas) contam depois das camadas.
    let mosaicos: Vec<String> = host::run("map.tiles", json!({}))?
        .as_array()
        .into_iter()
        .flatten()
        .filter(|t| t["visible"].as_bool() != Some(false))
        .filter_map(|t| t["name"].as_str())
        .filter(|n| n.starts_with(PREFIXO))
        .map(str::to_string)
        .collect();
    let com_mosaicos: Vec<&String> = visiveis.iter().chain(&mosaicos).collect();
    let (principal, r) = com_mosaicos
        .iter()
        .find_map(|n| layout::receita(n).map(|r| ((*n).clone(), r)))
        .ok_or("nenhum dado do BrasilData visível no mapa para o layout: traga um dado (um tema, o censo, a elevação, o radar, o GEDI...) e deixe-o visível")?;
    let titulo = layout::titulo(&area_que_ha()?);
    // As cenas do EMIT são inteiras, de uns 80 km: num município, a folha enquadra as cenas (a área
    // ficaria toda dentro de uma), e o contorno da área vai por cima delas, em vermelho.
    let emit = emit::eh_do_emit(&principal);
    let c = match extensao(&principal) {
        Ok(Some(e)) if emit && (e[2] - e[0] > 1.5 * (c[2] - c[0]) || e[3] - e[1] > 1.5 * (c[3] - c[1])) => e,
        _ => c,
    };
    // As fontes das camadas visíveis, sem repetir, e a do limite.
    let mut fontes: Vec<String> = Vec::new();
    for f in citacoes()?.0 {
        if f["layer"].as_str().is_some_and(|l| com_mosaicos.iter().any(|v| *v == l))
            && let Some(s) = f["source"].as_str()
        {
            // Na folha, a fonte sem o dicionário de campos (que fica na tabela Fontes).
            let s = s.split(" Campos").next().unwrap_or(s).trim_end_matches(['.', ' ']).to_string();
            if !fontes.contains(&s) {
                fontes.push(s);
            }
        }
    }
    fontes.push("Limite da área: IBGE · dados trazidos e recortados pelo plugin BrasilData".into());
    // A projeção da grade do projeto, ou a automática da área (UTM do fuso, ou o Albers do IBGE).
    let g = grade_no_mapa()?;
    let epsg = g.as_ref().map_or_else(|| Escolha::Automatico.epsg(c), |g| g.epsg);
    host::run("map.projection", json!({"type": format!("EPSG:{epsg}")}))?;
    let passo = layout::passo_da_grade(c);
    // Um dado do JAXA Earth sai na resolução original, fora da grade: a célula dele, não a da grade.
    let celula = match jaxa::variavel_da_camada(&principal) {
        Some(v) => format!("; células do dado: {}", v.resolucao()),
        // O metano fica na grade de 0,1° do arquivo.
        None if metano::da_camada(&principal) => "; células do dado: 0,1° (cerca de 11 km)".into(),
        None => g.as_ref().map(|g| format!("; células de {}", grade::texto_da_celula(g.celula))).unwrap_or_default(),
    };
    let referencia = [
        format!("Projeção: {}", grade::nome_do_sistema(epsg)),
        format!("Grade de {}, coordenadas em metros{celula}", grade::texto_da_celula(passo)),
    ];
    // A área só como contorno, e a grade (um retângulo de trabalho) fora do mapa.
    // (o `layer.noFill` não basta na 0.5.0: a cor transparente, sim)
    host::run("layer.color", json!({"layer": AREA, "color": "#00000000"}))?;
    host::run(
        "layer.outline",
        json!({"layer": AREA, "color": if emit { "#D7301F" } else { "#5E5547" }, "width": if emit { 1.6 } else { 1.2 }}),
    )?;
    if emit {
        host::run("layer.move", json!({"layer": AREA, "to": 0}))?;
    }
    // O fim do comando devolve a cada camada o estilo do começo ([`devolver_estilos`]): o da
    // área passa a ser este.
    let _ = host::run("style.remove", json!({"name": estilo::guardado(AREA)}));
    let _ = host::run("style.new", json!({"layer": AREA, "name": estilo::guardado(AREA)}));
    let dados = info["layers"].as_array().into_iter().flatten().find(|l| l["name"].as_str() == Some(principal.as_str()));
    let mut subtitulo = r.subtitulo.clone();
    let nomes_das_feicoes = |l: &Value, ate: u64| {
        if let Some(campo) = layout::campo_do_nome(l)
            && l["features"].as_u64().is_some_and(|n| n <= ate)
        {
            let _ = host::run("labels.field", json!({"layer": principal, "field": campo}));
            let _ = host::run("labels.enable", json!({"layer": principal, "on": true}));
            // A camada de indicadores mascarada pela área: os nomes só dos municípios de dentro.
            let tem_na_area = l["fields"].as_array().into_iter().flatten().filter_map(Value::as_str).any(|f| f.starts_with("na_area"));
            let filtro = if tem_na_area { json!("na_area = 1") } else { Value::Null };
            let _ = host::run("labels.class.update", json!({"layer": principal, "where": filtro}));
        }
    };
    // Indicadores e setores: um mapa coroplético da variável principal da camada, em 5 classes:
    // quantis com muitas feições, quebras naturais com poucas (os quantis de 9 municípios repetem
    // limites). O que a variável é vai no subtítulo: um título na legenda tiraria uma classe da
    // caixa, que tem a altura das linhas.
    let metodo = if dados.and_then(|l| l["features"].as_u64()).is_some_and(|n| n >= 30) { "quantile" } else { "naturalBreaks" };
    if let Some(l) = dados
        && let Some((campo, o_que_e, rampa)) = layout::coropletico(l)
        && host::run(
            "layer.symbology",
            json!({"layer": principal, "type": "graduated", "field": campo, "classes": 5, "method": metodo, "ramp": rampa}),
        )
        .is_ok()
    {
        subtitulo = format!("{}: {o_que_e}", r.subtitulo);
        host::run("layer.outline", json!({"layer": principal, "color": "#FFFFFF", "width": 0.4}))?;
        // As classes opacas: uma camada do projeto por baixo (países, um mapa temático) não as tinge.
        // (com menos de 5 classes, as que faltam são recusadas, e ficam como estão)
        for (i, cor) in layout::cores_opacas(rampa).into_iter().flatten().enumerate() {
            let _ = host::run("layer.classStyle", json!({"layer": principal, "index": i, "color": cor}));
        }
        // Os nomes dos municípios; os dos setores seriam centenas iguais.
        nomes_das_feicoes(l, 60);
        let _ = host::run("style.remove", json!({"name": estilo::guardado(&principal)}));
        let _ = host::run("style.new", json!({"layer": principal, "name": estilo::guardado(&principal)}));
    }
    // Um polígono de referência maior que a área (municípios, regiões, bacias...) pintaria a folha
    // inteira de uma cor: fica com um véu claro, o contorno e o nome de cada feição.
    else if let Some(l) = dados
        && layout::referencia_maior(l, c)
    {
        host::run("layer.color", json!({"layer": principal, "color": "#B07AA133"}))?;
        host::run("layer.outline", json!({"layer": principal, "color": "#7A3E6C", "width": 0.8}))?;
        nomes_das_feicoes(l, 300);
        let _ = host::run("style.remove", json!({"name": estilo::guardado(&principal)}));
        let _ = host::run("style.new", json!({"layer": principal, "name": estilo::guardado(&principal)}));
    }
    if nomes()?.iter().any(|n| n == GRADE) {
        host::run("layer.visibility", json!({"layer": [GRADE], "visible": false}))?;
    }
    let nome = format!("BrasilData: {}", titulo.to_lowercase());
    let _ = host::run("layout.remove", json!({"name": nome}));
    let texto = |k: &str, padrao: &str| req.params.get(k).and_then(Value::as_str).unwrap_or(padrao).to_string();
    let mut p = json!({
        "name": nome, "profile": perfil, "style": texto("estilo", "classic"), "extent": c,
        "title": titulo, "subtitle": subtitulo, "sources": fontes, "reference": referencia,
        "author": "Elaborado com RustGIS e BrasilData · {date}",
        "headings": ["Fonte dos dados", "Referência cartográfica", "Nota"],
        "locator": "globe", "decimal": ",",
    });
    if let Some(n) = r.nota {
        p["note"] = json!(n);
    }
    // As camadas visíveis que não são do BrasilData (as do projeto do usuário, como as do World
    // Atlas) são desligadas no mapa antes da folha, e o aviso diz quais. Ligadas, elas cobriam o mapa
    // inteiro: a legenda não achava um canto livre e ia, com o globo, a escala e o norte, para o meio
    // da imagem, ou o véu clareava uma faixa em volta da área; e um polígono delas tingia as classes.
    let alheias = layout::camadas_alheias(&info);
    if !alheias.is_empty() {
        host::run("layer.visibility", json!({"layer": alheias, "visible": false}))?;
    }
    let folha = host::run("layout.publication", p)?;
    let elemento = |qual: &str| {
        folha["elements"].as_array().into_iter().flatten().find(|e| e["name"].as_str() == Some(qual)).and_then(|e| e["element"].as_u64())
    };
    // A legenda: sem a área, a grade e o raster que a barra de cores já mostra; vazia, some.
    // (o `exclude` recusa um nome que não está no mapa)
    let no_mapa = nomes()?;
    let mut fora: Vec<String> = [AREA, GRADE, DESENHO].into_iter().filter(|n| no_mapa.iter().any(|m| m == n)).map(String::from).collect();
    fora.extend(alheias.iter().filter(|n| no_mapa.iter().any(|m| m == *n)).cloned());
    // Os nomes dos rios da drenagem: a linha deles é invisível, só os rótulos aparecem.
    fora.extend(no_mapa.iter().filter(|n| so_rotulos(n)).cloned());
    // (uma composição colorida não tem classes, nem os canais dela, que ficam por baixo: a legenda
    // seria uma caixa vazia)
    if layout::composicao(&principal) {
        fora.extend(visiveis.iter().filter(|v| radar_pc::eh_daqui(v) || nisar::eh_do_nisar(v)).map(|v| v.to_string()));
    } else if r.barra.is_some() {
        fora.push(principal.clone());
    }
    // O MapBiomas recortado com a camada das classes logo abaixo: a legenda mostra os nomes das
    // classes (a camada delas), e não os códigos do raster.
    let com_classes = mapbiomas::recortes_com_classes(&no_mapa);
    fora.extend(com_classes.iter().filter(|n| !fora.contains(n)).cloned().collect::<Vec<_>>());
    if let Some(id) = elemento("Legend") {
        let sobra = visiveis.iter().any(|v| !fora.contains(v)) || !com_classes.is_empty();
        // Um título a mais tira uma linha da caixa, que tem a altura das classes: só nas receitas
        // com classes nomeadas (o DIST), em que ela cabe.
        let mut u = json!({"layout": nome, "element": id, "exclude": fora, "visible": sobra});
        if let Some(t) = r.legenda {
            u["title"] = json!(t);
        }
        host::run("layout.update", u)?;
        // A caixa da legenda some com ela (um mapa do MapBiomas para ver não tem legenda).
        if !sobra && let Some(caixa) = elemento("Legend Box") {
            host::run("layout.update", json!({"layout": nome, "element": caixa, "visible": false}))?;
        }
        // Os nomes das camadas na legenda sem o "BrasilData: " (o "labels" da legenda, depois da
        // 0.6.0: uma versão que não o conhece recusa a chamada, e a legenda fica como está).
        let rotulos: serde_json::Map<String, Value> =
            visiveis.iter().filter(|v| !fora.contains(v)).map(|v| (v.clone(), json!(v.trim_start_matches(PREFIXO)))).collect();
        if sobra && !rotulos.is_empty() {
            let _ = host::run("layout.update", json!({"layout": nome, "element": id, "labels": rotulos}));
        }
    }
    // Sem legenda nem barra de cores (uma composição colorida), a caixa delas sai também.
    if r.barra.is_none()
        && elemento("Legend").is_none()
        && let Some(caixa) = elemento("Legend Box")
    {
        host::run("layout.update", json!({"layout": nome, "element": caixa, "visible": false}))?;
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
    avisar(&format!(
        "Layout \"{nome}\": {} na área, em {}. Abra-o em Publicar para ajustar e exportar; um novo clique o refaz",
        subtitulo,
        grade::nome_do_sistema(epsg)
    ));
    if !alheias.is_empty() {
        avisar(&format!(
            "Layout: desliguei no mapa {}, que não {} do BrasilData, para a folha sair só com o dado. Para {} na folha, ligue de novo no painel Camadas",
            alheias.join(", "),
            if alheias.len() == 1 { "é" } else { "são" },
            if alheias.len() == 1 { "tê-la" } else { "tê-las" }
        ));
    }
    Ok(json!({"layout": nome, "layer": principal, "crs": format!("EPSG:{epsg}"), "outside": alheias}))
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
    if let Some(pendente) = afinar_area(true)? {
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
            avisar(&format!("Uso e cobertura {ano} (MapBiomas): lendo a janela da área no COG da coleção; o raster vem quando ela chegar"));
            return Ok(Recorte::Feito(r));
        }
        if r.get("inPlace").and_then(Value::as_bool) == Some(true) || r.get("window").is_none() {
            // A 0.5.0 põe o COG inteiro no mapa.
            tirar_janela(&bruto)?;
            return Ok(Recorte::SemJanela);
        }
    }
    let g = grade_para(c, None)?;
    let classes = mapbiomas::nome_das_classes(ano);
    remover(&[&recorte, &nome, &classes])?;
    let feito = recortar(&bruto, AREA, &recorte).and_then(|_| {
        host::run(
            "raster.project",
            json!({"layer": recorte, "crs": g.epsg, "cellSize": g.celula, "extent": GRADE, "resampling": "mode", "name": nome}),
        )
    });
    remover(&[&bruto, &recorte])?;
    let r = feito?;
    // As cores e os nomes da legenda oficial: o recorte sem a extensão Imagens perde a
    // tabela de cores do arquivo, que o raster.process levava junto.
    host::run("style.importQgis", json!({"layer": nome, "text": mapbiomas::qml(None)}))?;
    // Uma área grande vem de uma pirâmide do arquivo: as células lidas são maiores que as da grade.
    let piramide = mapbiomas::celula_da_janela(c)
        .map(|m| format!(" A área é grande para 30 m: a janela veio da pirâmide de {} m do arquivo.", m - 1.0))
        .unwrap_or_default();
    // Os nomes das classes, para a legenda do layout e o identificar. Sem eles o raster serve igual.
    let nomes_das_classes = match poligonos_das_classes(&nome, &classes, c, g.epsg, g.celula) {
        Ok(()) => {
            // O estilo do raster só com as classes da área: o painel Camadas lista uma linha por
            // classe do estilo, e não as 38 da Coleção 10.
            let presentes = classes_presentes(&classes);
            if !presentes.is_empty() {
                let _ = host::run("style.importQgis", json!({"layer": nome, "text": mapbiomas::qml(Some(&presentes))}));
            }
            format!(" Os nomes das classes estão na camada {classes}, logo abaixo.")
        }
        Err(e) => {
            host::log(&format!("classes do MapBiomas: {e}"));
            let _ = remover(&[&classes]);
            String::new()
        }
    };
    avisar(&format!(
        "{nome}: classes do MapBiomas (Coleção 10) na grade do projeto, {}, com a classe mais frequente; as cores são as da legenda oficial.{piramide}{nomes_das_classes}",
        g.texto()
    ));
    Ok(Recorte::Feito(json!({
        "layer": nome, "year": ano, "crs": format!("EPSG:{}", g.epsg), "cellSize": g.celula,
        "width": r["width"], "height": r["height"],
    })))
}

/// Os códigos das classes do MapBiomas que a camada das classes `camada` tem (campo `VALUE`), pela
/// seleção, que não traz a geometria para o plugin (os polígonos de uma área grande passariam do
/// combustível de um comando). Vazio se não deu para saber; a seleção da camada fica limpa.
fn classes_presentes(camada: &str) -> Vec<u8> {
    let tem = |v: u8| {
        host::run("select.byAttributes", json!({"layer": camada, "where": format!("VALUE = {v}")}))
            .is_ok_and(|r| r["selected"].as_u64().is_some_and(|n| n > 0))
    };
    let presentes = mapbiomas::CLASSES_10.iter().map(|(v, ..)| *v).filter(|v| tem(*v)).collect();
    let _ = host::run("select.clear", json!({"layer": camada}));
    presentes
}

/// As classes do raster `raster` do MapBiomas como polígonos, uma feição por classe, com o nome da
/// classe (campo `classe`) e a cor da legenda oficial, na camada `nome`, logo abaixo do raster, que
/// a cobre. A legenda do layout do RustGIS lista só camadas vetoriais, e não as classes de um
/// raster: é esta camada que põe os nomes nela (e no painel Camadas e no identificar).
///
/// Um retângulo acima de [`mapbiomas::MAX_CELULAS_CLASSES`] vetoriza uma cópia de células maiores,
/// com a classe mais frequente: uma classe que só ocupa manchas pequenas pode ficar de fora. Como as
/// manchas do DIST, os polígonos saem de uma cópia em graus quando a ferramenta do GeoLibre recusa o
/// sistema da grade (o Albers do IBGE, EPSG:10857).
fn poligonos_das_classes(raster: &str, nome: &str, c: [f64; 4], epsg: u32, celula: f64) -> Result<(), String> {
    let poligonos = format!("{nome} (polígonos)");
    let maiores = format!("{nome} (células maiores)");
    let em_graus = format!("{nome} (em graus)");
    remover(&[nome, &poligonos, &maiores, &em_graus])?;
    let feito = (|| {
        let grossa = mapbiomas::celula_das_classes(c, celula);
        let origem = if grossa > celula {
            host::run("raster.project", json!({"layer": raster, "crs": epsg, "cellSize": grossa, "resampling": "mode", "name": maiores}))?;
            maiores.as_str()
        } else {
            raster
        };
        if let Err(e) = ferramenta("geolibre:raster_to_vector_polygons", json!({"input": origem}), &poligonos) {
            host::log(&format!("classes: no sistema da grade não deu ({e}); vão em graus"));
            remover(&[&poligonos])?;
            host::run(
                "raster.project",
                json!({"layer": origem, "crs": 4326, "cellSize": grossa / 111_320.0, "resampling": "nearest", "name": em_graus}),
            )?;
            ferramenta("geolibre:raster_to_vector_polygons", json!({"input": em_graus}), &poligonos)?;
        }
        let r = host::run("gp.run", json!({"tool": "dissolve", "params": {"input": poligonos, "field": "VALUE"}}))?;
        let id = r.get("layer").or_else(|| r["layers"].get(0)).cloned().ok_or("o dissolve não disse a camada que fez")?;
        host::run("layer.rename", json!({"layer": id, "name": nome}))?;
        // Antes de descer a camada: as do meio, acima do raster, a deixariam sobre ele.
        remover(&[&poligonos, &maiores, &em_graus])?;
        host::run("edit.addField", json!({"layer": nome, "name": "classe", "type": "text"}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": "classe", "expression": mapbiomas::expressao_das_classes()}))?;
        let _ = host::run("style.remove", json!({"name": mapbiomas::ESTILO_DAS_CLASSES}));
        host::run("style.add", json!({"text": mapbiomas::estilo_das_classes()}))?;
        host::run("style.apply", json!({"layer": nome, "name": mapbiomas::ESTILO_DAS_CLASSES}))?;
        // A camada nova chega no topo, logo acima do raster: uma posição abaixo, fica sob ele.
        host::run("layer.moveDown", json!({"layer": nome}))?;
        Ok(())
    })();
    remover(&[&poligonos, &maiores, &em_graus])?;
    feito
}

/// Os controles da amostragem no painel.
const AMOSTRA_TEXTO: &str = "amostra-texto";
const AMOSTRA_PONTOS: &str = "amostra-pontos";
const AMOSTRA_ESPACAMENTO: &str = "amostra-espacamento";
const AMOSTRA_SORTEIO: &str = "amostra-sorteio";

/// Os métodos, para o gerador do sorteio: a mesma área e o mesmo número de sorteio dão
/// candidatos diferentes em cada método.
const ALEATORIA: u64 = 1;
const ESTRATIFICADA: u64 = 2;
const SISTEMATICA: u64 = 3;

/// O número do sorteio: o `sorteio` do script, o do controle do painel, ou 1.
fn sorteio_de(req: &Request) -> Result<i64, String> {
    let n = req.f64("sorteio").map(|x| x as i64).or_else(|| req.panel.get(AMOSTRA_SORTEIO).and_then(Value::as_i64)).unwrap_or(1);
    if n < 1 {
        return Err(format!("o sorteio é um número a partir de 1, não {n}"));
    }
    Ok(n)
}

/// Quantos pontos: o `pontos` do script, o do controle do painel, ou o padrão.
fn pontos_de(req: &Request) -> Result<u64, String> {
    let n = match req.f64("pontos") {
        Some(n) => n,
        None => req
            .panel
            .get(AMOSTRA_PONTOS)
            .and_then(Value::as_u64)
            .and_then(|i| amostragem::PONTOS.get(i as usize))
            .copied()
            .unwrap_or(amostragem::PONTOS_PADRAO)
            .into(),
    };
    if n.fract() != 0.0 || !(1.0..=f64::from(amostragem::MAX_PONTOS)).contains(&n) {
        return Err(format!("os pontos vão de 1 a {}, não {n}", amostragem::MAX_PONTOS));
    }
    Ok(n as u64)
}

/// O espaçamento da grade, em metros: o `espacamento` do script, o do controle, ou o padrão.
fn espacamento_de(req: &Request) -> f64 {
    req.f64("espacamento").unwrap_or_else(|| {
        req.panel
            .get(AMOSTRA_ESPACAMENTO)
            .and_then(Value::as_u64)
            .and_then(|i| amostragem::ESPACAMENTOS.get(i as usize))
            .copied()
            .unwrap_or(amostragem::ESPACAMENTO_PADRAO)
    })
}

/// O retângulo da área. A amostra é sempre dentro dela: sem área, não há o que sortear.
fn area_da_amostra() -> Result<[f64; 4], String> {
    if !nomes()?.iter().any(|n| n == AREA) {
        return Err("escolha a área de trabalho primeiro (painel Área de trabalho): os pontos são sorteados dentro dela".into());
    }
    extensao(AREA)?.ok_or_else(|| "a camada da área está vazia".to_string())
}

/// A camada que uma ferramenta do `gp.run` fez: o RustGIS 0.5.0 a diz em `layer`; outras
/// versões, numa lista `layers`.
fn camada_feita(r: &Value) -> Option<Value> {
    r.get("layer").filter(|v| !v.is_null()).or_else(|| r["layers"].get(0)).cloned()
}

/// Roda a ferramenta `tool` e dá o nome `nome` à camada que ela fizer, no lugar de uma anterior.
fn ferramenta_em(tool: &str, params: Value, nome: &str) -> Result<(), String> {
    let r = host::run("gp.run", json!({"tool": tool, "params": params})).map_err(extensoes::erro_da_ferramenta)?;
    let id = camada_feita(&r).ok_or_else(|| format!("{tool} não disse a camada que fez"))?;
    remover(&[nome])?;
    host::run("layer.rename", json!({"layer": id, "name": nome}))?;
    Ok(())
}

/// Recorta os candidatos pela área, na camada `destino`, e diz quantos ficaram.
fn recortar_pela_area(candidatos: &[Candidato], destino: &str) -> Result<u64, String> {
    remover(&[amostragem::CANDIDATOS, destino])?;
    host::add_layer(amostragem::CANDIDATOS, amostragem::camada_de_candidatos(candidatos))?;
    let feito = ferramenta_em("clip", json!({"input": amostragem::CANDIDATOS, "clip": AREA}), destino);
    remover(&[amostragem::CANDIDATOS])?;
    feito.map_err(|e| format!("o recorte dos pontos pela área falhou: {e}"))?;
    Ok(feicoes_da_camada(destino)?.unwrap_or(0))
}

/// A área num polígono só, para o sorteio: a de um estado ou região vem em várias feições.
const AREA_UNICA: &str = "BrasilData: amostragem (área)";

/// `m` pontos ao acaso dentro da área, sorteados pelo RustGIS (`analysis.simulate`, com a
/// semente do método e do sorteio), na camada [`amostragem::NA_AREA`], cada um com a ordem do
/// sorteio (de 1 a `m`). Devolve quantos vieram (uma área muito fina pode dar menos).
fn sortear_na_area(m: u64, metodo: u64, sorteio: i64) -> Result<u64, String> {
    remover(&[amostragem::NA_AREA, AREA_UNICA])?;
    ferramenta_em("dissolve", json!({"input": AREA}), AREA_UNICA)?;
    let r = host::run(
        "analysis.simulate",
        json!({"layer": AREA_UNICA, "count": m, "seed": amostragem::semente(metodo, sorteio as u64), "name": amostragem::NA_AREA}),
    );
    remover(&[AREA_UNICA])?;
    let id = camada_feita(&r?).ok_or("o sorteio dos pontos não disse a camada que fez")?;
    host::run("layer.rename", json!({"layer": id, "name": amostragem::NA_AREA}))?;
    host::run("edit.addField", json!({"layer": amostragem::NA_AREA, "name": amostragem::ORDEM, "type": "long"}))?;
    host::run("edit.calculateField", json!({"layer": amostragem::NA_AREA, "field": amostragem::ORDEM, "expression": "@row_number"}))?;
    Ok(feicoes_da_camada(amostragem::NA_AREA)?.unwrap_or(0))
}

/// Quantas feições de `camada` atendem `onde` (selecionando-as).
fn contar(camada: &str, onde: &str) -> Result<u64, String> {
    host::run("select.byAttributes", json!({"layer": camada, "where": onde}))?["selected"]
        .as_u64()
        .ok_or_else(|| format!("a seleção em '{camada}' não disse quantas feições"))
}

/// A cláusula `condicao` (se houver) com a ordem do sorteio abaixo de `corte`.
fn antes_de(condicao: Option<&str>, corte: u64) -> String {
    let ordem = format!("{} < {corte}", amostragem::sql_campo(amostragem::ORDEM));
    condicao.map_or_else(|| ordem.clone(), |c| format!("({c}) AND {ordem}"))
}

/// O menor corte da ordem do sorteio em que `condicao` tem `k` pontos de `camada` antes dele:
/// uma busca binária com seleções por atributo, que só devolvem a contagem (a ordem vai de 0 a
/// `sorteados`). Como as ordens não se repetem, a contagem cresce de um em um, e o corte dá
/// exatamente `k`.
fn corte_para(camada: &str, condicao: Option<&str>, k: u64, sorteados: usize) -> Result<u64, String> {
    let (mut baixo, mut alto) = (0u64, sorteados as u64);
    while baixo < alto {
        let meio = (baixo + alto) / 2;
        if contar(camada, &antes_de(condicao, meio))? >= k {
            alto = meio;
        } else {
            baixo = meio + 1;
        }
    }
    Ok(baixo)
}

/// Faz da seleção de [`amostragem::NA_AREA`] (ou de `origem`) por `onde` a camada da amostra
/// `nome`: o rótulo de cada ponto pela linha, o método, o sorteio, os `extras` (campo, tipo e
/// expressão) e as coordenadas em graus (SIRGAS 2000, 6 casas). Os outros campos saem. A amostra
/// fica com pontos vermelhos e o rótulo, e a tabela aberta (na janela). Devolve quantos pontos.
fn amostra_da_selecao(
    origem: &str,
    onde: &str,
    nome: &str,
    metodo: &str,
    sorteio: i64,
    extras: &[(&str, &str, String)],
) -> Result<u64, String> {
    contar(origem, onde)?;
    let r = host::run("select.layerFromSelection", json!({"layer": origem}))?;
    let selecao = r.get("layer").cloned().ok_or("a seleção não virou camada")?;
    remover(&[nome])?;
    host::run("layer.rename", json!({"layer": selecao, "name": nome}))?;
    // Os campos point_x e point_y, na própria camada.
    host::run("gp.run", json!({"tool": "calculateGeometry", "params": {"input": nome}}))?;
    let n = feicoes_da_camada(nome)?.unwrap_or(0);
    let antes: Vec<String> = host::run("layer.fields", json!({"layer": nome}))?
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| f["name"].as_str().map(str::to_string))
        .collect();
    let mut campos: Vec<(&str, &str, String)> = vec![
        ("ponto", "text", amostragem::expressao_do_rotulo(n as usize)),
        ("metodo", "text", format!("'{metodo}'")),
        ("sorteio", "long", sorteio.to_string()),
    ];
    campos.extend(extras.iter().cloned());
    campos.push(("lon", "double", "round(\"point_x\", 6)".into()));
    campos.push(("lat", "double", "round(\"point_y\", 6)".into()));
    for (campo, tipo, expressao) in &campos {
        host::run("edit.addField", json!({"layer": nome, "name": campo, "type": tipo}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": campo, "expression": expressao}))?;
    }
    for campo in antes {
        host::run("edit.deleteField", json!({"layer": nome, "field": campo}))?;
    }
    let _ = host::run("select.clear", json!({"layer": nome}));
    let _ = host::run("layer.color", json!({"layer": nome, "color": "#e31a1c"}));
    let _ = host::run("labels.field", json!({"layer": nome, "field": "ponto"}));
    let _ = host::run("table.open", json!({"layer": nome}));
    Ok(n)
}

/// O texto no aviso e no painel, e a resposta de uma amostra. Com `arquivo` no pedido (a janela do
/// comando, ou um script), a amostra também vai para esse arquivo ([`gravar_amostra`]); se não der,
/// a camada fica no mapa e o aviso diz por quê.
fn resposta_da_amostra(req: &Request, nome: &str, pontos: u64, texto: &str, mais: Value) -> Value {
    let curto: String = texto.chars().take(480).collect();
    avisar(&curto);
    let (arquivo, sobre_o_arquivo) = match gravar_amostra(req, nome) {
        Ok(None) => (None, String::new()),
        Ok(Some(c)) => (Some(c.clone()), format!("Os pontos foram gravados em {c}.")),
        Err(e) => (None, format!("Os pontos ficaram só no mapa: {e}.")),
    };
    if !sobre_o_arquivo.is_empty() {
        avisar(&sobre_o_arquivo);
    }
    let texto = if sobre_o_arquivo.is_empty() { texto.to_string() } else { format!("{texto}\n\n{sobre_o_arquivo}") };
    // `report`: o texto que a janela do comando mostra depois de executar (opengeos/RustGIS#1547).
    let mut r = json!({"layer": nome, "points": pontos, "report": texto, "panel": {"items": {AMOSTRA_TEXTO: {"text": texto}}}});
    if let Some(c) = arquivo {
        r["file"] = json!(c);
    }
    if let (Some(r), Some(m)) = (r.as_object_mut(), mais.as_object()) {
        r.extend(m.clone());
    }
    r
}

/// Grava a camada da amostra `nome` no arquivo do pedido (`arquivo`), se houver, e devolve o
/// caminho ([`amostragem::caminho_do_arquivo`]: só o nome vai para a pasta do projeto). Os pontos
/// vão em SIRGAS 2000 (EPSG:4674), como as colunas lon e lat; o GeoJSON, o KML e o GPX ficam no
/// WGS 84 que o formato pede. Um GeoPackage que já existe ganha a tabela de novo; os outros
/// formatos não sobrescrevem um arquivo.
fn gravar_amostra(req: &Request, nome: &str) -> Result<Option<String>, String> {
    let arquivo = req.str("arquivo").unwrap_or("").trim().to_string();
    if arquivo.is_empty() {
        return Ok(None);
    }
    let projeto = host::run("info.project", json!({"nameFilter": nome}))?;
    let Some(caminho) = amostragem::caminho_do_arquivo(&arquivo, projeto["path"].as_str())? else {
        return Ok(None);
    };
    let ext = caminho.rsplit_once('.').map(|(_, e)| e.to_lowercase()).unwrap_or_default();
    let mut p = json!({"layer": nome, "path": caminho});
    if ["gpkg", "shp", "fgb"].contains(&ext.as_str()) {
        p["crs"] = json!("EPSG:4674");
    }
    if ext == "gpkg" {
        p["tableName"] = json!(amostragem::tabela_do_arquivo(&caminho));
        if host::run("share.exportLayer", p.clone()).is_ok() {
            return Ok(Some(caminho));
        }
        p["mode"] = json!("replaceTable");
    }
    // O GPX e o KML dão a cada ponto o nome do campo `name` (senão, o número da feição): o rótulo
    // P01, P02…, num campo que só existe durante a gravação.
    let com_nome = ["gpx", "kml"].contains(&ext.as_str());
    if com_nome {
        host::run("edit.addField", json!({"layer": nome, "name": "name", "type": "text"}))?;
        host::run("edit.calculateField", json!({"layer": nome, "field": "name", "expression": "\"ponto\""}))?;
    }
    let gravado = host::run("share.exportLayer", p).map_err(|e| format!("o RustGIS não gravou {caminho} ({e})"));
    if com_nome {
        host::run("edit.deleteField", json!({"layer": nome, "field": "name"}))?;
    }
    gravado?;
    Ok(Some(caminho))
}

/// Amostragem aleatória simples: `pontos` ao acaso na área, todos com a mesma chance.
fn amostra_aleatoria(req: &Request) -> Result<Value, String> {
    let (n, sorteio) = (pontos_de(req)?, sorteio_de(req)?);
    area_da_amostra()?;
    let veio = sortear_na_area(n, ALEATORIA, sorteio)?;
    let feito = if veio < n {
        Err(format!("só {veio} de {n} pontos couberam na área: ela é fina demais para o sorteio; peça menos pontos"))
    } else {
        amostra_da_selecao(amostragem::NA_AREA, &antes_de(None, n + 1), amostragem::ALEATORIA, "aleatória simples", sorteio, &[])
    };
    remover(&[amostragem::NA_AREA])?;
    let n = feito?;
    let texto = format!("Amostragem aleatória simples: {n} pontos na área, sorteio {sorteio}. Outro sorteio dá outros pontos");
    Ok(resposta_da_amostra(req, amostragem::ALEATORIA, n, &texto, json!({})))
}

/// A camada dos estratos: a do `camada` do script, ou a ativa. Precisa ser de polígonos, e não a
/// área nem uma camada da própria amostragem.
fn camada_dos_estratos(req: &Request) -> Result<String, String> {
    let camadas = host::layers()?;
    let lista = camadas.as_array().cloned().unwrap_or_default();
    let escolhida = match req.str("camada").map(str::trim).filter(|c| !c.is_empty()) {
        Some(nome) => lista.iter().find(|l| l["name"].as_str() == Some(nome)).ok_or_else(|| format!("não há camada '{nome}' no mapa"))?,
        None => lista.iter().find(|l| l["active"].as_bool() == Some(true)).ok_or(
            "escolha a camada dos estratos: clique nela no painel Camadas (por exemplo, BrasilData: Solos) e de novo em Amostragem estratificada",
        )?,
    };
    let nome = escolhida["name"].as_str().unwrap_or_default().to_string();
    if nome == AREA || nome == GRADE || nome.starts_with("BrasilData: amostragem") {
        return Err(format!(
            "'{nome}' não serve de estratos: clique no painel Camadas numa camada de polígonos com classes (por exemplo, BrasilData: Solos) e de novo em Amostragem estratificada"
        ));
    }
    if !escolhida["geometry"].as_str().is_some_and(|g| g.contains("Polygon")) {
        return Err(format!("'{nome}' não é uma camada de polígonos: os estratos são polígonos, como os de Solos ou Geologia"));
    }
    Ok(nome)
}

/// As feições de `camada` que atendem `condicao` (todas, se houver no máximo `k`; senão as `k`
/// primeiras na ordem do sorteio), lidas pelo plugin: só para descobrir campos e classes.
fn primeiras(camada: &str, condicao: Option<&str>, k: u64, sorteados: usize) -> Result<Vec<Value>, String> {
    let onde = match condicao {
        Some(c) if contar(camada, c)? <= k => c.to_string(),
        _ => antes_de(condicao, corte_para(camada, condicao, k, sorteados)?),
    };
    contar(camada, &onde)?;
    Ok(host::features(camada, true)?["features"].as_array().cloned().unwrap_or_default())
}

/// Um estrato: o valor do campo (como o RustGIS o compara), o nome e quantos candidatos.
struct Estrato {
    valor: String,
    nome: String,
    candidatos: u64,
}

/// Os estratos de `campo` nos pontos de `camada`: as classes que aparecem nos primeiros pontos e,
/// em até 6 buscas, as que ainda faltam (os pontos com valor fora das já vistas), cada uma com a
/// sua contagem de candidatos. Devolve também quantos pontos ficam sem estrato.
fn estratos_de(camada: &str, campo: &str, primeiros: &[Value], sorteados: usize) -> Result<(Vec<Estrato>, u64), String> {
    let c = amostragem::sql_campo(campo);
    let mut vistos: Vec<(String, String)> = Vec::new();
    let ver = |fs: &[Value], vistos: &mut Vec<(String, String)>| {
        for f in fs {
            let v = &f["properties"][campo];
            if let (Some(nome), Some(valor)) = (amostragem::valor_do_estrato(v), amostragem::sql_valor(v))
                && !vistos.iter().any(|(x, _)| *x == valor)
            {
                vistos.push((valor, nome));
            }
        }
    };
    ver(primeiros, &mut vistos);
    for _ in 0..6 {
        let fora: Vec<&str> = vistos.iter().map(|(v, _)| v.as_str()).collect();
        let resto =
            if fora.is_empty() { format!("{c} IS NOT NULL") } else { format!("{c} IS NOT NULL AND {c} NOT IN ({})", fora.join(", ")) };
        let antes = vistos.len();
        if contar(camada, &resto)? == 0 {
            break;
        }
        ver(&primeiras(camada, Some(&resto), 40, sorteados)?, &mut vistos);
        if vistos.len() == antes {
            // Só valores vazios ("", espaços): não são estratos.
            break;
        }
    }
    let mut estratos = Vec::new();
    for (valor, nome) in vistos {
        let candidatos = contar(camada, &format!("{c} = {valor}"))?;
        estratos.push(Estrato { valor, nome, candidatos });
    }
    estratos.sort_by(|a, b| a.nome.cmp(&b.nome));
    let com_estrato: u64 = estratos.iter().map(|e| e.candidatos).sum();
    let total = feicoes_da_camada(camada)?.unwrap_or(0);
    Ok((estratos, total.saturating_sub(com_estrato)))
}

/// Amostragem aleatória estratificada: os estratos são as classes de um campo de uma camada de
/// polígonos. Cada estrato recebe pontos na proporção da sua área (estimada pelos candidatos que
/// caem nele), com um mínimo, e os pontos caem ao acaso dentro dele.
fn amostra_estratificada(req: &Request) -> Result<Value, String> {
    let (n, sorteio) = (pontos_de(req)?, sorteio_de(req)?);
    let minimo = req.f64("minimo").map_or(amostragem::MINIMO_PADRAO, |m| m.max(0.0) as usize);
    let estratos = camada_dos_estratos(req)?;
    area_da_amostra()?;
    // Pontos bastantes para estimar a área de cada estrato e ter pontos nos pequenos.
    let veio = sortear_na_area(amostragem::pontos_para_estratos(n), ESTRATIFICADA, sorteio)?;
    if veio == 0 {
        remover(&[amostragem::NA_AREA])?;
        return Err("nenhum ponto coube na área".into());
    }
    // As ordens vão de 1 a `veio`: o corte vai até `veio + 1`.
    let sorteados = veio as usize + 1;
    let feito = estratificar(req, &estratos, n, minimo, sorteio, sorteados);
    remover(&[amostragem::NA_AREA, amostragem::NOS_ESTRATOS])?;
    feito
}

/// O resto da estratificada, com os candidatos da área já em [`amostragem::NA_AREA`].
fn estratificar(req: &Request, estratos: &str, n: u64, minimo: usize, sorteio: i64, sorteados: usize) -> Result<Value, String> {
    // Em que estrato cai cada candidato: o spatialJoin do RustGIS.
    ferramenta_em("spatialJoin", json!({"target": amostragem::NA_AREA, "join": estratos}), amostragem::NOS_ESTRATOS)
        .map_err(|e| format!("a junção dos pontos com '{estratos}' falhou: {e}"))?;
    let camada = amostragem::NOS_ESTRATOS;
    // O campo: o do script, ou o de menos classes (de 2 a 30) nos 120 primeiros pontos.
    let primeiros = primeiras(camada, None, 120, sorteados)?;
    let campos = amostragem::campos_dos_estratos(&primeiros);
    let campo = match req.str("campo").map(str::trim).filter(|c| !c.is_empty()) {
        Some(c) => host::run("layer.fields", json!({"layer": camada}))?
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|f| f["name"].as_str())
            .find(|x| x.eq_ignore_ascii_case(c) || amostragem::nome_legivel(x).eq_ignore_ascii_case(c))
            .map(str::to_string)
            .ok_or_else(|| format!("'{estratos}' não tem o campo '{c}'"))?,
        None => campos.first().map(|(c, _)| c.clone()).ok_or_else(|| {
            format!(
                "'{estratos}' não tem um campo com 2 a 30 classes nos pontos da área: escolha outra camada, ou diga o campo por script (\"campo\")"
            )
        })?,
    };
    let (lista, sem_estrato) = estratos_de(camada, &campo, &primeiros, sorteados)?;
    if lista.is_empty() {
        return Err(format!("nenhum ponto da área caiu num polígono de '{estratos}' com valor em '{campo}'"));
    }
    let contagens: Vec<usize> = lista.iter().map(|e| e.candidatos as usize).collect();
    let alocacao = amostragem::alocar(&contagens, n as usize, minimo);
    // Os primeiros de cada estrato na ordem do sorteio, numa cláusula só.
    let c = amostragem::sql_campo(&campo);
    let mut partes = Vec::new();
    for (e, k) in lista.iter().zip(&alocacao) {
        if *k == 0 {
            continue;
        }
        let deste = format!("{c} = {}", e.valor);
        let corte = corte_para(camada, Some(&deste), *k as u64, sorteados)?;
        partes.push(format!("({})", antes_de(Some(&deste), corte)));
    }
    let estrato_expr = format!("to_string({c})");
    let total = amostra_da_selecao(
        camada,
        &partes.join(" OR "),
        amostragem::ESTRATIFICADA,
        "estratificada",
        sorteio,
        &[("estrato", "text", estrato_expr)],
    )?;
    let resumo: Vec<String> = lista.iter().zip(&alocacao).map(|(e, k)| format!("{} {k}", e.nome)).collect();
    let legivel = amostragem::nome_legivel(&campo);
    let mut texto = format!(
        "Amostragem estratificada: {total} pontos em {} estratos de '{legivel}' ({estratos}), na proporção da área, sorteio {sorteio}: {}",
        lista.len(),
        resumo.join(", ")
    );
    if total > n {
        texto.push_str(&format!(". São mais que {n} porque cada estrato tem pelo menos {minimo}"));
    }
    let com: u64 = lista.iter().map(|e| e.candidatos).sum();
    if sem_estrato * 20 > com + sem_estrato {
        texto.push_str(&format!(
            ". Parte da área ({}%) fica fora dos polígonos com classe e não recebe pontos",
            sem_estrato * 100 / (com + sem_estrato)
        ));
    }
    let outros: Vec<String> =
        campos.iter().filter(|(c, _)| *c != campo).take(6).map(|(c, k)| format!("{} ({k})", amostragem::nome_legivel(c))).collect();
    if !outros.is_empty() {
        texto.push_str(&format!(". Outros campos, por script (\"campo\"): {}", outros.join(", ")));
    }
    let nomes: Vec<&str> = lista.iter().map(|e| e.nome.as_str()).collect();
    Ok(resposta_da_amostra(
        req,
        amostragem::ESTRATIFICADA,
        total,
        &texto,
        json!({"field": legivel, "strata": nomes, "allocation": alocacao}),
    ))
}

/// Amostragem sistemática: uma grade de pontos com o espaçamento em metros e a origem sorteada,
/// os que caem na área, numerados em zigue-zague.
fn amostra_sistematica(req: &Request) -> Result<Value, String> {
    let (espacamento, sorteio) = (espacamento_de(req), sorteio_de(req)?);
    let c = area_da_amostra()?;
    let grade: Vec<Candidato> = amostragem::sistematicos(c, espacamento, &mut Sorteio::novo(SISTEMATICA, sorteio as u64))?
        .into_iter()
        .enumerate()
        .map(|(i, p)| (i as u64, p))
        .collect();
    let esp = amostragem::texto_do_espacamento(espacamento);
    let dentro = recortar_pela_area(&grade, amostragem::NA_AREA)?;
    if dentro == 0 {
        remover(&[amostragem::NA_AREA])?;
        return Err(format!("nenhum ponto da grade de {esp} caiu na área: use um espaçamento menor"));
    }
    let feito = amostra_da_selecao(
        amostragem::NA_AREA,
        &antes_de(None, grade.len() as u64),
        amostragem::SISTEMATICA,
        "sistemática",
        sorteio,
        &[("espacamento_m", "double", espacamento.to_string())],
    );
    remover(&[amostragem::NA_AREA])?;
    let n = feito?;
    let texto = format!(
        "Amostragem sistemática: {n} pontos a cada {esp} na área, origem do sorteio {sorteio}, numerados em zigue-zague de norte a sul"
    );
    Ok(resposta_da_amostra(req, amostragem::SISTEMATICA, n, &texto, json!({})))
}

/// O botão Sobre: a janela do comando mostra a autoria ao abrir (a ajuda dele); Executar põe no
/// `report` as fontes e como contribuir (`completo`), ou a autoria. Por script, `parte` escolhe a
/// parte (tudo, o padrão, autoria, fontes ou contribuir). Sem nenhum dos dois (um RustGIS sem
/// janelas de plugin, antes da 0.7.0, roda o botão direto), vai também o aviso curto, que o
/// RustGIS mostra e tira logo.
fn sobre(req: &Request) -> Result<Value, String> {
    let completo = req.params.get("completo").and_then(Value::as_bool);
    let parte = req.str("parte");
    let texto = match (completo, parte) {
        (Some(true), _) => format!(
            "{}\n\n{}",
            crate::parte_do_sobre("fontes").unwrap_or_default(),
            crate::parte_do_sobre("contribuir").unwrap_or_default()
        ),
        (Some(false), _) => crate::parte_do_sobre("autoria").unwrap_or_default(),
        (None, p) => crate::parte_do_sobre(p.unwrap_or("tudo")).ok_or("parte: use tudo, autoria, fontes ou contribuir")?,
    };
    if completo.is_none() && parte.is_none() {
        avisar(crate::SOBRE_AVISO);
    }
    Ok(json!({"version": versao::ESTA, "text": crate::SOBRE, "report": texto}))
}

/// Uma aba Sobre do fim do painel ([`crate::ABAS_SOBRE`]): o texto dela no lugar do da aba
/// anterior, com o nome da aba no título.
fn aba_sobre(id: &str) -> Result<Value, String> {
    let (_, rotulo, parte) = crate::ABAS_SOBRE.iter().find(|(c, _, _)| *c == id).ok_or("aba Sobre desconhecida")?;
    let texto = crate::parte_do_sobre(parte).ok_or("parte do Sobre desconhecida")?;
    Ok(json!({
        "report": texto,
        "panel": {"items": {"sobre": {"text": texto, "label": format!("Sobre o BrasilData: {rotulo}")}}},
    }))
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
    avisar(&texto);
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
