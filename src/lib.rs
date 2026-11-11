//! BrasilData, etapa 2 do `../PLANO.md`: o MVP vetorial.
//!
//! A área do usuário fica no mapa, numa camada de nome fixo ([`AREA`]), porque um plugin não
//! guarda nada entre chamadas. Cada tema vetorial do catálogo ([`TEMAS`], gerado do
//! `fontes.csv` por `scripts/gerar.py`) chega por `service.add` com o retângulo da área
//! (`bbox`): o RustGIS lê o WFS, pagina, e põe a longitude primeiro (opengeos/RustGIS#1053,
//! corrigido no #1071). Depois, `gp.run clip` recorta pelo polígono da área.
//!
//! A elevação (Copernicus GLO-30) é a da prova de conceito: os tiles de 1° que cobrem a área,
//! baixados inteiros, juntados (`raster.mosaic`), recortados (`raster.process clip`) e postos
//! na grade do projeto (`raster.project`, módulo [`grade`]): por padrão, o UTM SIRGAS 2000 do
//! fuso da área, com células de 30 m.

pub mod amostragem;
pub mod aqueduct;
mod catalogo;
pub mod censo;
pub mod datas;
pub mod dialogos;
pub mod dist;
pub mod estilo;
pub mod grade;
pub mod indicadores;
pub mod irrigacao;
// O que fala com o RustGIS só compila no wasm: fora dele (nos testes), as importações do
// RustGIS ficariam sem resolver no linker do Windows.
#[cfg(target_arch = "wasm32")]
mod comandos;
pub mod gedi;
pub mod layout;
pub mod malhas;
pub mod mapbiomas;
pub mod municipios;
pub mod radar;
pub mod risco_fogo;
pub mod saude;
pub mod sol;
pub mod versao;
pub mod vista;
pub mod zoom;

pub use catalogo::{SOBRE, SOBRE_AVISO, TEMAS};

/// O nome da camada da área.
pub const AREA: &str = "BrasilData: área";

/// O começo do nome de toda camada do BrasilData.
pub const PREFIXO: &str = "BrasilData: ";

/// Um tema vetorial: um tipo de feição de um WFS.
pub struct Tema {
    pub id: &'static str,
    /// O grupo da aba.
    pub grupo: &'static str,
    /// O nome da camada no mapa, depois de [`PREFIXO`].
    pub nome: &'static str,
    /// O endereço do serviço.
    pub servico: &'static str,
    /// `wfs` ou `arcgis`, o `type` do `service.add`.
    pub tipo: &'static str,
    /// O tipo de feição do WFS, ou o número da camada do ArcGIS.
    pub camada: &'static str,
    /// `poligono`, `linha` ou `ponto`.
    pub geometria: &'static str,
    /// Se é recortado pela área. Os limites (municípios, estados) vêm inteiros: recortados, os
    /// vizinhos virariam retalhos.
    pub recorta: bool,
    /// A citação.
    pub fonte: &'static str,
    pub licenca: &'static str,
    /// O filtro, junto com o retângulo: o CQL de um WFS, o `where` de um ArcGIS.
    pub filtro: &'static str,
    /// Os atributos que vêm, separados por vírgula (vazio: todos).
    pub campos: &'static str,
    /// O atributo da geometria de um WFS, para o filtro e os campos.
    pub geom: &'static str,
    /// Feições por pedido; 0 é um pedido só.
    pub pagina: u32,
    /// O atributo que ordena as páginas.
    pub ordem: &'static str,
}

impl Tema {
    /// O nome da camada no mapa.
    pub fn camada_no_mapa(&self) -> String {
        format!("{PREFIXO}{}", self.nome)
    }

    /// Se o plugin monta o pedido ele mesmo, em vez do `service.add`, que não leva filtro nem
    /// campos e não sabe a camada de cada UF.
    pub fn pedido_proprio(&self) -> bool {
        !self.filtro.is_empty() || !self.campos.is_empty() || self.pagina > 0 || self.camada.contains("{uf}")
    }

    /// As camadas a pedir: a do catálogo, ou uma por UF de `ufs` quando ela leva `{uf}`.
    pub fn camadas(&self, ufs: &[String]) -> Vec<String> {
        if !self.camada.contains("{uf}") {
            return vec![self.camada.to_string()];
        }
        ufs.iter().map(|uf| self.camada.replace("{uf}", &uf.to_lowercase())).collect()
    }
}

/// O tema de uma camada do mapa, pelo nome.
pub fn tema_da_camada(nome: &str) -> Option<&'static Tema> {
    TEMAS.iter().find(|t| t.camada_no_mapa() == nome)
}

/// Se um erro do RustGIS é o de um servidor que falhou de passagem (um 502 ou 503 do gateway,
/// como o do TerraBrasilis), que vale tentar de novo.
pub fn falha_passageira(erro: &str) -> bool {
    [
        "http status: 502",
        "http status: 503",
        "http status: 504",
        // `host.fetch` (o caminho do app web) diz o status assim.
        "HTTP 502",
        "HTTP 503",
        "HTTP 504",
        "timed out",
    ]
    .iter()
    .any(|s| erro.contains(s))
}

/// Um erro do RustGIS com o porquê, quando é o de um servidor que o app desktop não alcança: o
/// do CAR (`geoserver.car.gov.br`) só aceita TLS 1.2 com troca de chaves RSA (AES-GCM sem ECDHE),
/// que o rustls do RustGIS não tem; o navegador ainda aceita.
pub fn explicar(erro: String) -> String {
    if erro.contains("HandshakeFailure") || erro.contains("handshake failure") {
        format!(
            "{erro}. O servidor só aceita cifras TLS antigas (troca de chaves RSA), que o RustGIS desktop não aceita; no app web, que usa as do navegador, a camada vem"
        )
    } else {
        erro
    }
}

/// As capacidades de um WFS com os endereços do próprio servidor em `https`, quando ele é pedido
/// em `https`. O GeoServer da ANP anuncia `http://geomaps.anp.gov.br/geoserver/wfs`, que
/// responde 522, e o RustGIS pede as feições ao endereço anunciado.
pub fn com_https(capacidades: &str, servico: &str) -> String {
    let Some(resto) = servico.strip_prefix("https://") else {
        return capacidades.to_string();
    };
    let host = resto.split('/').next().unwrap_or(resto);
    capacidades.replace(&format!("http://{host}"), &format!("https://{host}"))
}

/// `s` pronto para ir num endereço: tudo menos as letras, os dígitos e `-_.~` em `%XX` (UTF-8).
pub fn escapar(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// O começo do pedido das feições de `camada` (de `t`) que cruzam `c`, com o filtro: sem o
/// formato, os campos e as páginas. Um WFS recebe o retângulo na forma curta `EPSG:4674`
/// (longitude primeiro no GeoServer); com filtro, ele vai dentro do CQL, porque o GeoServer não
/// aceita `bbox` e `CQL_FILTER` juntos. Um ArcGIS recebe o retângulo como envelope em 4326.
fn consulta(t: &Tema, camada: &str, c: [f64; 4]) -> String {
    let caixa = format!("{:.6},{:.6},{:.6},{:.6}", c[0], c[1], c[2], c[3]);
    if t.tipo == "arcgis" {
        let onde = if t.filtro.is_empty() { "1=1" } else { t.filtro };
        format!(
            "{}/{camada}/query?where={}&geometry={caixa}&geometryType=esriGeometryEnvelope&inSR=4326&spatialRel=esriSpatialRelIntersects",
            t.servico,
            escapar(onde)
        )
    } else {
        let mut u = format!("{}?service=WFS&version=2.0.0&request=GetFeature&typeNames={camada}", t.servico);
        if t.filtro.is_empty() {
            u.push_str(&format!("&bbox={caixa},EPSG:4674"));
        } else {
            let cql = format!("BBOX({},{caixa},'EPSG:4674') AND ({})", t.geom, t.filtro);
            u.push_str(&format!("&CQL_FILTER={}", escapar(&cql)));
        }
        u
    }
}

/// O pedido das feições de `camada` (de `t`) que cruzam `c`, em GeoJSON, longitude primeiro, só
/// com os campos do catálogo: a página `(início, tamanho)`, ou tudo de uma vez.
pub fn pedido(t: &Tema, camada: &str, c: [f64; 4], pagina: Option<(u64, u64)>) -> String {
    let mut u = consulta(t, camada, c);
    if t.tipo == "arcgis" {
        let campos = if t.campos.is_empty() { "*" } else { t.campos };
        u.push_str(&format!("&outFields={}&outSR=4326", escapar(campos)));
        if let Some((inicio, n)) = pagina {
            u.push_str(&format!("&resultOffset={inicio}&resultRecordCount={n}&orderByFields={}", escapar(t.ordem)));
        }
        u.push_str("&f=geojson");
    } else {
        u.push_str("&outputFormat=application/json&srsName=EPSG:4674");
        if !t.campos.is_empty() {
            u.push_str(&format!("&propertyName={}", escapar(&format!("{},{}", t.campos, t.geom))));
        }
        match pagina {
            Some((inicio, n)) => {
                u.push_str(&format!("&count={n}&startIndex={inicio}"));
                // Sem ordem, o GeoServer pagina pela chave primária.
                if !t.ordem.is_empty() {
                    u.push_str(&format!("&sortBy={}", escapar(t.ordem)));
                }
            }
            None => u.push_str("&count=50000"),
        }
    }
    u
}

/// O pedido das feições de `t` que cruzam `c`, tudo de uma vez: o caminho do app web, onde o
/// `service.add` de um comando não baixa feições (o `layer.addUrl` baixa).
pub fn pedido_de_feicoes(t: &Tema, c: [f64; 4]) -> String {
    pedido(t, t.camada, c, None)
}

/// O pedido que conta as feições de `camada` (de `t`) que cruzam `c`, com o filtro.
pub fn pedido_de_contagem(t: &Tema, camada: &str, c: [f64; 4]) -> String {
    let u = consulta(t, camada, c);
    if t.tipo == "arcgis" { format!("{u}&returnCountOnly=true&f=json") } else { format!("{u}&resultType=hits") }
}

/// A contagem de uma resposta a [`pedido_de_contagem`]: o `count` de um ArcGIS (que diz o erro
/// no corpo, com HTTP 200), ou o `numberMatched` do XML de um WFS.
pub fn contagem(json: Option<&serde_json::Value>, texto: Option<&str>) -> Result<u64, String> {
    if let Some(j) = json {
        if let Some(n) = j["count"].as_u64() {
            return Ok(n);
        }
        if let Some(e) = j.get("error") {
            return Err(format!("o serviço recusou a contagem: {}", e["message"].as_str().unwrap_or("erro sem mensagem")));
        }
    }
    let texto = texto.unwrap_or_default();
    if let Some(resto) = texto.split("numberMatched=\"").nth(1)
        && let Ok(n) = resto.split('"').next().unwrap_or("").parse::<u64>()
    {
        return Ok(n);
    }
    let erro = texto.split("ExceptionText>").nth(1).and_then(|e| e.split('<').next()).unwrap_or("a resposta não traz a contagem");
    Err(format!("o serviço recusou a contagem: {}", erro.trim()))
}

/// As páginas de `n` feições em pedidos de `tamanho`: `(início, tamanho)` de cada uma, ou um
/// pedido só (`None`) quando o tema não pagina.
pub fn paginas(n: u64, tamanho: u32) -> Vec<Option<(u64, u64)>> {
    if n == 0 {
        return Vec::new();
    }
    if tamanho == 0 {
        return vec![None];
    }
    let t = u64::from(tamanho);
    (0..n.div_ceil(t)).map(|i| Some((i * t, t))).collect()
}

/// Quantos downloads os apps do RustGIS esperam de uma vez (`MAX_URL_REQUESTS`, 0.5.0): passando
/// disso, o `layer.addUrl` recusa o pedido.
pub const FILA_DO_APP: usize = 16;

/// Se o erro `e` de um `layer.addUrl` é a fila de downloads do app cheia, e não uma falha: o
/// pedido pode ser feito de novo quando os que esperam chegarem.
pub fn fila_cheia(e: &str) -> bool {
    e.contains("downloads are already waiting")
}

/// O código EPSG do UTM SIRGAS 2000 sul do fuso de `lon` (fusos 18 a 25, os do Brasil), ou o
/// Albers do IBGE (SIRGAS 2000, EPSG:10857) quando a área passa de um fuso. A Policônica
/// (EPSG:5880) seria a outra escolha do IBGE, mas o `raster.project` do RustGIS 0.5.0 não a tem
/// (ela chegou ao `main` depois).
pub fn sistema(c: [f64; 4]) -> u32 {
    let fuso = |lon: f64| (((lon + 180.0) / 6.0).floor() as i32 + 1).clamp(18, 25);
    if fuso(c[0]) == fuso(c[2]) { 31960 + fuso((c[0] + c[2]) / 2.0) as u32 } else { 10857 }
}

/// Os tiles de 1° do Copernicus GLO-30 que cobrem `c`: o endereço no espelho da OpenTopography,
/// que manda o cabeçalho CORS e por isso abre também no app web, e o do bucket da AWS, que não
/// manda, para quando o primeiro falha e o RustGIS espera o download.
pub fn tiles(c: [f64; 4]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    // Uma borda exatamente num grau inteiro não puxa o tile vizinho.
    let (s, n) = (c[1].floor() as i32, (c[3] - 1e-9).floor() as i32);
    let (w, e) = (c[0].floor() as i32, (c[2] - 1e-9).floor() as i32);
    for lat in s..=n {
        for lon in w..=e {
            let ns = if lat < 0 { format!("S{:02}", -lat) } else { format!("N{lat:02}") };
            let ew = if lon < 0 { format!("W{:03}", -lon) } else { format!("E{lon:03}") };
            let ot = format!("https://opentopography.s3.sdsc.edu/raster/COP30/COP30_hh/Copernicus_DSM_10_{ns}_00_{ew}_00_DEM.tif");
            let id = format!("Copernicus_DSM_COG_10_{ns}_00_{ew}_00_DEM");
            out.push((ot, format!("https://copernicus-dem-30m.s3.amazonaws.com/{id}/{id}.tif")));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fila_cheia_do_app_nao_e_falha() {
        assert!(fila_cheia("https://x: 16 downloads are already waiting; try again when they have landed"));
        assert!(!fila_cheia("https://x: 404 Not Found"));
    }

    #[test]
    fn teresina_cai_no_fuso_23_e_em_dois_tiles() {
        let teresina = [-42.9707, -5.5866, -42.599, -4.7863];
        assert_eq!(sistema(teresina), 31983);
        let t = tiles(teresina);
        assert_eq!(t.len(), 2);
        assert!(t[0].0.contains("S06_00_W043") && t[1].0.contains("S05_00_W043"), "{t:?}");
        assert!(t[0].0.starts_with("https://opentopography.s3.sdsc.edu/") && t[0].1.contains("copernicus-dem-30m"), "{t:?}");
        // Dois fusos: o Albers do IBGE.
        assert_eq!(sistema([-46.0, -10.0, -40.0, -3.0]), 10857);
        // Um grau inteiro como borda não puxa o vizinho.
        assert_eq!(tiles([-43.0, -6.0, -42.0, -5.0]).len(), 1);
    }

    #[test]
    fn o_catalogo_tem_ids_unicos_e_servicos_https() {
        let mut ids: Vec<&str> = TEMAS.iter().map(|t| t.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), TEMAS.len());
        assert!(TEMAS.iter().all(|t| t.servico.starts_with("https://")));
        assert!(TEMAS.iter().all(|t| match t.tipo {
            "wfs" => t.camada.contains(':'),
            "arcgis" => t.camada.parse::<u32>().is_ok(),
            _ => false,
        }));
        // Os limites vêm inteiros; os temas, recortados.
        let t = |id: &str| TEMAS.iter().find(|t| t.id == id).unwrap();
        assert!(!t("municipios").recorta && t("geologia").recorta);
        assert_eq!(tema_da_camada("BrasilData: Solos (BDiA 1:250.000)").map(|t| t.id), Some("solos"));
        assert!(tema_da_camada(AREA).is_none());
    }

    #[test]
    fn os_controles_da_amostragem_no_painel_batem_com_as_listas() {
        let m: serde_json::Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
        let item = |id: &str| m["panel"]["items"].as_array().unwrap().iter().find(|i| i["id"] == id).cloned().unwrap();
        let (p, e, s) = (item("amostra-pontos"), item("amostra-espacamento"), item("amostra-sorteio"));
        assert_eq!(p["max"], json_len(amostragem::PONTOS.len()));
        assert_eq!(amostragem::PONTOS[p["default"].as_u64().unwrap() as usize], amostragem::PONTOS_PADRAO);
        assert_eq!(e["max"], json_len(amostragem::ESPACAMENTOS.len()));
        assert_eq!(amostragem::ESPACAMENTOS[e["default"].as_u64().unwrap() as usize], amostragem::ESPACAMENTO_PADRAO);
        assert_eq!(s["max"].as_i64(), Some(amostragem::SORTEIOS));
        // Os botões do painel são os comandos do menu Amostragem.
        for c in ["amostra-aleatoria", "amostra-estratificada", "amostra-sistematica"] {
            assert!(m["commands"].as_array().unwrap().iter().any(|x| x["id"] == c), "{c}");
        }
    }

    /// A última posição de um controle com `n` posições.
    fn json_len(n: usize) -> serde_json::Value {
        serde_json::json!(n - 1)
    }

    #[test]
    fn o_sobre_tem_a_autoria_a_versao_e_cabe_no_rustgis() {
        for t in [SOBRE, SOBRE_AVISO] {
            assert!(
                t.contains("Elpídio Filho")
                    && t.contains("LABGEO")
                    && t.contains("UFV")
                    && t.contains("MIT")
                    && t.contains("github.com/elpidioassinaturas/BrasilData"),
                "{t}"
            );
            // A versão do texto é a do Cargo.toml (o gerar.py mantém as duas iguais).
            assert!(t.contains(&format!("BrasilData {}", versao::ESTA)), "{t}");
        }
        assert!(SOBRE.contains("Laboratório de Geoprocessamento e Pedometria") && SOBRE.contains("Universidade Federal de Viçosa"));
        assert!(SOBRE.contains("Como contribuir") && SOBRE.contains("IBGE") && SOBRE.contains("Copernicus"));
        // O texto declarado de um painel vai até 2.000 caracteres; um aviso é cortado em 500.
        assert!(SOBRE.chars().count() <= 2000 && SOBRE_AVISO.chars().count() + "BrasilData: ".len() <= 500);
    }

    #[test]
    fn as_capacidades_da_anp_vao_para_https() {
        let caps = r#"<ows:Get xlink:href="http://geomaps.anp.gov.br/geoserver/wfs"/><a href="http://outro.br/x"/>"#;
        let novo = com_https(caps, "https://geomaps.anp.gov.br/geoserver/ows");
        assert!(novo.contains("https://geomaps.anp.gov.br/geoserver/wfs") && novo.contains("http://outro.br/x"));
        assert_eq!(com_https(caps, "http://geomaps.anp.gov.br/geoserver/ows"), caps);
    }

    #[test]
    fn o_pedido_do_app_web_vai_com_a_longitude_primeiro() {
        let t = |id: &str| TEMAS.iter().find(|t| t.id == id).unwrap();
        let c = [-42.97, -5.59, -42.6, -4.79];
        let wfs = pedido_de_feicoes(t("geologia"), c);
        assert!(
            wfs.contains("typeNames=BDIA:geol_area")
                && wfs.contains("&bbox=-42.970000,-5.590000,-42.600000,-4.790000,EPSG:4674&")
                && wfs.ends_with("&count=50000")
                && !wfs.contains("propertyName"),
            "{wfs}"
        );
        let arc = pedido_de_feicoes(t("aneel-ute"), c);
        assert!(
            arc.contains("/FeatureServer/18/query?") && arc.contains("geometry=-42.970000,-5.590000,") && arc.ends_with("f=geojson"),
            "{arc}"
        );
    }

    #[test]
    fn o_filtro_de_um_wfs_vai_no_cql_com_o_retangulo() {
        let t = |id: &str| TEMAS.iter().find(|t| t.id == id).unwrap();
        let c = [-42.97, -5.59, -42.6, -4.79];
        let car = t("car-imoveis");
        assert!(car.pedido_proprio() && !t("geologia").pedido_proprio());
        assert_eq!(car.camadas(&["PI".into(), "MA".into()]), ["sicar:sicar_imoveis_pi", "sicar:sicar_imoveis_ma"]);
        let u = pedido(car, "sicar:sicar_imoveis_pi", c, Some((10_000, 10_000)));
        // BBOX(geo_area_imovel,-42.970000,...,'EPSG:4674') AND (status_imovel<>'CA')
        assert!(
            u.contains("&CQL_FILTER=BBOX%28geo_area_imovel%2C-42.970000%2C-5.590000%2C-42.600000%2C-4.790000%2C%27EPSG%3A4674%27%29%20AND%20%28status_imovel%3C%3E%27CA%27%29")
                && !u.contains("&bbox=")
                && u.ends_with("&count=10000&startIndex=10000&sortBy=cod_imovel"),
            "{u}"
        );
        let snv = pedido(t("dnit-snv"), "vgeo:vw_snv_rod", c, None);
        assert!(
            snv.contains("dt_fim%20%3E%20now%28%29") && snv.contains("&propertyName=Codigo_BR%2C") && snv.contains("%2CVersao_SNV%2Cgeom&"),
            "{snv}"
        );
        // A drenagem da BC250 passa das 50 mil feições por pedido do RustGIS num estado: vem em
        // páginas, sem sortBy (o GeoServer usa a chave primária).
        let dren = t("drenagem");
        assert!(dren.pedido_proprio() && dren.pagina > 0);
        let d = pedido(dren, dren.camada, c, Some((20_000, 20_000)));
        assert!(
            d.contains("&bbox=-42.970000,-5.590000,-42.600000,-4.790000,EPSG:4674") && d.ends_with("&count=20000&startIndex=20000"),
            "{d}"
        );
        let n = pedido_de_contagem(car, "sicar:sicar_imoveis_pi", c);
        assert!(n.ends_with("&resultType=hits") && !n.contains("outputFormat"), "{n}");
        // Acentos vão em UTF-8.
        assert!(pedido(t("sigef-glebas"), "geonode:sigef_geo", c, None).contains("Gleba%20P%C3%BAblica"));
    }

    #[test]
    fn um_arcgis_leva_os_campos_e_as_paginas_ordenadas() {
        let t = |id: &str| TEMAS.iter().find(|t| t.id == id).unwrap();
        let c = [-42.97, -5.59, -42.6, -4.79];
        let anm = t("anm-processos");
        let u = pedido(anm, anm.camada, c, Some((2000, 2000)));
        assert!(
            u.contains("/MapServer/0/query?where=1%3D1&geometry=-42.970000,")
                && u.contains("&outFields=PROCESSO%2CNUMERO%2C")
                && !u.contains("NOME")
                && u.ends_with("&resultOffset=2000&resultRecordCount=2000&orderByFields=OBJECTID&f=geojson"),
            "{u}"
        );
        assert!(pedido_de_contagem(anm, anm.camada, c).ends_with("&returnCountOnly=true&f=json"));
        // Nenhuma camada do catálogo traz CPF, CNPJ ou o nome do titular.
        for t in TEMAS {
            let campos = t.campos.to_lowercase();
            assert!(!campos.contains("cpf") && !campos.contains("cnpj") && !campos.split(',').any(|c| c == "nome"), "{}", t.id);
        }
    }

    #[test]
    fn a_contagem_vem_do_arcgis_ou_do_wfs_e_os_erros_dizem_por_que() {
        use serde_json::json;
        assert_eq!(contagem(Some(&json!({"count": 481})), None), Ok(481));
        assert!(contagem(Some(&json!({"error": {"message": "Invalid query"}})), None).unwrap_err().contains("Invalid query"));
        let xml = r#"<wfs:FeatureCollection numberMatched="19119" numberReturned="0" next="x">"#;
        assert_eq!(contagem(None, Some(xml)), Ok(19119));
        let erro = "<ows:ExceptionText>Could not parse CQL filter list.</ows:ExceptionText>";
        assert!(contagem(None, Some(erro)).unwrap_err().contains("Could not parse"));
        assert_eq!(paginas(0, 10), vec![]);
        assert_eq!(paginas(25, 0), vec![None]);
        assert_eq!(paginas(25, 10), vec![Some((0, 10)), Some((10, 10)), Some((20, 10))]);
        assert_eq!(escapar("a b'Ç"), "a%20b%27%C3%87");
    }

    #[test]
    fn um_502_vale_outra_tentativa_e_um_404_nao() {
        assert!(falha_passageira("service.add: https://terrabrasilis.dpi.inpe.br/geoserver/ows: http status: 502"));
        assert!(!falha_passageira("service.add: https://x/wfs: http status: 404"));
        // O CAR no app desktop: o erro diz por quê e aponta o app web.
        let e = explicar("https://geoserver.car.gov.br/...: io: received fatal alert: HandshakeFailure".into());
        assert!(e.contains("cifras TLS antigas") && e.contains("app web"));
        assert_eq!(explicar("HTTP 404".into()), "HTTP 404");
    }
}
