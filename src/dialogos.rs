//! As janelas dos comandos (`commands[].form` do `plugin.json`, opengeos/RustGIS#1723): o
//! botão de um comando com janela a abre, e o **Executar** dela roda o comando com os valores
//! dos campos.
//!
//! - **O que a janela manda:** um campo de texto deixado vazio pode chegar como `""`, e uma
//!   escolha chega como o texto dela ("setembro", "Folha A4"). [`normalizar`] tira os vazios e
//!   troca cada escolha pelo valor que o comando já aceita de um script (`9`, `"a4"`), de modo
//!   que um script e a janela dão no mesmo. Um valor que não é uma escolha passa como veio.
//! - **O que a janela mostra:** o `report` da resposta. [`com_relato`] o acrescenta com os avisos
//!   que o comando deu, para que a janela não mostre o JSON da resposta.
//!
//! As escolhas daqui são as do `scripts/gerar.py`; o teste [`tests::conhece_todas_as_escolhas`]
//! confere, no `plugin.json`, que cada uma é conhecida.

use serde_json::{Value, json};

use crate::risco_fogo::NOMES_DOS_MESES;
use crate::tempo::{ESCOLHAS_DETERMINISTICOS, ESCOLHAS_ENSEMBLES, ESCOLHAS_MEDIDAS, ESCOLHAS_TEMPERATURAS};

/// A escolha da radiação solar que pede a média do ano (o comando sem `mes`).
pub const MEDIA_DO_ANO: &str = "média do ano";

/// As escolhas do sistema da grade e o `crs` de cada uma.
pub const SISTEMAS: [(&str, &str); 3] = [
    ("Automático: UTM do fuso, ou Albers se a área passa de um fuso", "automatico"),
    ("Albers do IBGE (EPSG:10857)", "albers"),
    ("Policônica do Brasil (EPSG:5880)", "policonica"),
];

/// As escolhas do formato do layout e o `perfil` de cada uma.
pub const PERFIS: [(&str, &str); 8] = [
    ("Folha A4", "a4"),
    ("Folha A3", "a3"),
    ("Folha A2", "a2"),
    ("Figura de 1 coluna (90 mm)", "figure-1col"),
    ("Figura de 1,5 coluna (140 mm)", "figure-1.5col"),
    ("Figura de 2 colunas (190 mm)", "figure-2col"),
    ("Tese: página inteira (160 × 225 mm)", "thesis-full"),
    ("Tese: meia página (160 × 110 mm)", "thesis-half"),
];

/// As escolhas da orientação da folha e a `orientacao` de cada uma.
pub const ORIENTACOES: [(&str, &str); 3] =
    [("Automática: a que der o mapa maior", "automatica"), ("Retrato (em pé)", "retrato"), ("Paisagem (deitada)", "paisagem")];

/// As escolhas do que trazer da CTrees e a `camada` de cada uma.
pub const CAMADAS_CTREES: [(&str, &str); 2] = [("Biomassa (Mg/ha)", "biomassa"), ("Incerteza: erro-padrão (Mg/ha)", "incerteza")];

/// As escolhas do indicador do GDO e o `indicador` de cada uma.
pub const INDICADORES_GDO: [(&str, &str); 6] = [
    ("Anomalia da umidade do solo", "umidade"),
    ("Anomalia da vegetação (fAPAR)", "vegetacao"),
    ("SPI de 1 mês (CHIRPS)", "spi1"),
    ("SPI de 3 meses (CHIRPS)", "spi3"),
    ("SPI de 6 meses (CHIRPS)", "spi6"),
    ("SPI de 12 meses (CHIRPS)", "spi12"),
];

/// As escolhas do estilo da folha e o `estilo` de cada uma.
pub const ESTILOS: [(&str, &str); 6] = [
    ("Clássico", "classic"),
    ("Faixa", "banner"),
    ("Painel", "panel"),
    ("Editorial", "editorial"),
    ("Modular", "modular"),
    ("Pôster", "poster"),
];

/// As escolhas das cores da situação do DIST-S1 e a `paleta` de cada uma.
pub const PALETAS: [(&str, &str); 2] = [("A do BrasilData: etapa e confiança", "brasildata"), ("A oficial da OPERA", "oficial")];

/// As polarizações do radar de 10 m.
pub const POLARIZACOES: [(&str, &str); 2] = [("VV: solo, água e construções", "VV"), ("VH: volume da vegetação", "VH")];

/// As escolhas do modelo do GEM-15 e o `modelo` de cada uma.
pub const MODELOS_GEM15: [(&str, &str); 3] = [
    ("Terreno nu (DTM)", "dtm"),
    ("Superfície, com vegetação e construções (DSM)", "dsm"),
    ("Altura acima do terreno (DSM menos DTM)", "altura"),
];

/// As escolhas do satélite do cubo do INPE e o `satelite` de cada uma.
pub const SATELITES_BDC: [(&str, &str); 4] = [
    ("Sentinel-2 (10 m, desde 2017)", "sentinel-2"),
    ("Landsat (30 m, desde 1990)", "landsat"),
    ("CBERS-4 WFI (64 m, desde 2016)", "cbers-4"),
    ("MODIS (250 m, desde 2000)", "modis"),
];

/// As escolhas do satélite da imagem sem nuvens e o `satelite` de cada uma (sem o MODIS, que não
/// tem as bandas de cor no cubo).
pub const SATELITES_SEM_NUVENS: [(&str, &str); 3] = [
    ("Sentinel-2 (10 m, desde 2017)", "sentinel-2"),
    ("Landsat (30 m, desde 1990)", "landsat"),
    ("CBERS-4 WFI (64 m, desde 2016)", "cbers-4"),
];

/// As escolhas da composição da imagem sem nuvens e a `composicao` de cada uma.
pub const COMPOSICOES: [(&str, &str); 2] =
    [("Cor verdadeira (vermelho, verde e azul)", "natural"), ("Falsa-cor (infravermelho, vermelho e verde)", "falsa-cor")];

/// As escolhas do índice do cubo do INPE e o `indice` de cada uma.
pub const INDICES_BDC: [(&str, &str); 2] = [("NDVI", "ndvi"), ("EVI", "evi")];

/// As escolhas do resumo das datas do cubo do INPE e a `estatistica` de cada uma.
pub const ESTATISTICAS_BDC: [(&str, &str); 4] =
    [("Média das datas", "media"), ("Máxima", "maxima"), ("Mínima", "minima"), ("Desvio-padrão", "desvio")];

/// As escolhas do turno da temperatura do JAXA e o `turno` de cada uma.
pub const TURNOS_JAXA: [(&str, &str); 2] = [("De dia (por volta das 10h30)", "dia"), ("À noite (por volta das 22h30)", "noite")];

/// As escolhas da propriedade do solo do OpenLandMap e a `propriedade` de cada uma.
pub const PROPRIEDADES_OLM: [(&str, &str); 7] = [
    ("Carbono orgânico (g/kg)", "carbono"),
    ("Densidade de carbono (kg/m³)", "densidade-carbono"),
    ("pH em água", "ph"),
    ("Densidade do solo (g/cm³)", "densidade"),
    ("Argila (%)", "argila"),
    ("Areia (%)", "areia"),
    ("Silte (%)", "silte"),
];

/// As escolhas da profundidade e a `profundidade` de cada uma.
pub const PROFUNDIDADES_OLM: [(&str, &str); 3] = [("0 a 30 cm", "0-30"), ("30 a 60 cm", "30-60"), ("60 a 100 cm", "60-100")];

/// As escolhas do período e o `periodo` de cada um.
pub const PERIODOS_OLM: [(&str, &str); 5] = [
    ("2020 a 2022", "2020-2022"),
    ("2015 a 2020 (só carbono e pH)", "2015-2020"),
    ("2010 a 2015 (só carbono e pH)", "2010-2015"),
    ("2005 a 2010 (só carbono e pH)", "2005-2010"),
    ("2000 a 2005 (só carbono e pH)", "2000-2005"),
];

/// As escolhas da variável do relevo do GEDTM30 e a `variavel` de cada uma.
pub const VARIAVEIS_GEDTM: [(&str, &str); 16] = [
    ("Elevação do terreno (m)", "elevacao"),
    ("Declividade (graus)", "declividade"),
    ("Índice topográfico de umidade (TWI)", "twi"),
    ("Diferença da elevação média em volta (m)", "dfme"),
    ("Rugosidade do terreno (graus)", "rugosidade"),
    ("Curvatura mínima", "curvatura-minima"),
    ("Curvatura máxima", "curvatura-maxima"),
    ("Curvatura de perfil", "curvatura-perfil"),
    ("Curvatura tangencial", "curvatura-tangencial"),
    ("Curvatura anelar (240 m ou mais)", "curvatura-anelar"),
    ("Índice de forma (480 m ou mais)", "indice-forma"),
    ("Abertura positiva (graus, 480 m ou mais)", "abertura-positiva"),
    ("Abertura negativa (graus, 480 m ou mais)", "abertura-negativa"),
    ("Sombreamento", "sombreamento"),
    ("Geomorfons (formas do relevo)", "geomorfons"),
    ("Fator LS (erosão)", "fator-ls"),
];

/// As escolhas da escala do GEDTM30 e a `escala` de cada uma (em metros).
pub const ESCALAS_GEDTM: [(&str, &str); 6] =
    [("30 m", "30"), ("60 m", "60"), ("120 m", "120"), ("240 m", "240"), ("480 m", "480"), ("960 m", "960")];

/// As escolhas do período do metano (a janela) e o valor de cada uma ("2023" ou "2023-03").
pub const PERIODOS_METANO: [(&str, &str); 13] = [
    ("média de 2023", "2023"),
    ("janeiro de 2023", "2023-01"),
    ("fevereiro de 2023", "2023-02"),
    ("março de 2023", "2023-03"),
    ("abril de 2023", "2023-04"),
    ("maio de 2023", "2023-05"),
    ("junho de 2023", "2023-06"),
    ("julho de 2023", "2023-07"),
    ("agosto de 2023", "2023-08"),
    ("setembro de 2023", "2023-09"),
    ("outubro de 2023", "2023-10"),
    ("novembro de 2023", "2023-11"),
    ("dezembro de 2023", "2023-12"),
];

/// As escolhas de um parâmetro e o valor de cada uma.
type Escolhas = &'static [(&'static str, &'static str)];

/// As escolhas de texto de cada comando: (comando, parâmetro, escolhas e valores).
const TROCAS: &[(&str, &str, Escolhas)] = &[
    ("grade", "crs", &SISTEMAS),
    ("layout", "perfil", &PERFIS),
    ("layout", "orientacao", &ORIENTACOES),
    ("layout", "estilo", &ESTILOS),
    ("ctrees-biomassa", "camada", &CAMADAS_CTREES),
    ("gdo-seca", "indicador", &INDICADORES_GDO),
    ("dist-situacao", "paleta", &PALETAS),
    ("radar-10m", "polarizacao", &POLARIZACOES),
    ("gem15", "modelo", &MODELOS_GEM15),
    ("olm-solo", "propriedade", &PROPRIEDADES_OLM),
    ("olm-solo", "profundidade", &PROFUNDIDADES_OLM),
    ("olm-solo", "periodo", &PERIODOS_OLM),
    ("gedtm-relevo", "variavel", &VARIAVEIS_GEDTM),
    ("gedtm-relevo", "escala", &ESCALAS_GEDTM),
    ("bdc-indice", "satelite", &SATELITES_BDC),
    ("bdc-indice", "indice", &INDICES_BDC),
    ("bdc-indice", "estatistica", &ESTATISTICAS_BDC),
    ("bdc-sem-nuvens", "satelite", &SATELITES_SEM_NUVENS),
    ("bdc-sem-nuvens", "composicao", &COMPOSICOES),
    ("jaxa-chuva", "estatistica", &ESTATISTICAS_BDC),
    ("jaxa-umidade", "estatistica", &ESTATISTICAS_BDC),
    ("jaxa-temperatura", "estatistica", &ESTATISTICAS_BDC),
    ("jaxa-temperatura", "turno", &TURNOS_JAXA),
    ("jaxa-evapotranspiracao", "estatistica", &ESTATISTICAS_BDC),
    ("jaxa-seca", "estatistica", &ESTATISTICAS_BDC),
    ("era5-chuva", "resumo", &crate::era5::ESCOLHAS_RESUMOS),
    ("era5-temperatura", "tipo", &ESCOLHAS_TEMPERATURAS),
    ("chuva-prevista", "modelo", &ESCOLHAS_DETERMINISTICOS),
    ("temperatura-prevista", "modelo", &ESCOLHAS_DETERMINISTICOS),
    ("temperatura-prevista", "tipo", &ESCOLHAS_TEMPERATURAS),
    ("chuva-ensemble", "modelo", &ESCOLHAS_ENSEMBLES),
    ("chuva-ensemble", "medida", &ESCOLHAS_MEDIDAS),
    ("metano-tropomi", "periodo", &PERIODOS_METANO),
];

/// O número do mês de nome `nome` (janeiro é 1).
fn mes_pelo_nome(nome: &str) -> Option<u8> {
    let nome = nome.trim().to_lowercase();
    NOMES_DOS_MESES.iter().position(|m| *m == nome).and_then(|i| u8::try_from(i + 1).ok())
}

/// Os parâmetros do comando `id` como um script os daria: sem os textos vazios e com cada
/// escolha da janela trocada pelo seu valor.
pub fn normalizar(id: &str, params: &mut Value) {
    let Some(p) = params.as_object_mut() else { return };
    p.retain(|_, v| v.as_str().is_none_or(|t| !t.trim().is_empty()));
    for (comando, nome, escolhas) in TROCAS {
        if *comando != id {
            continue;
        }
        if let Some(v) = p.get(*nome).and_then(Value::as_str).and_then(|t| escolhas.iter().find(|(e, _)| *e == t)).map(|(_, v)| *v) {
            p.insert((*nome).to_string(), json!(v));
        }
    }
    match id {
        // O mês pelo nome; a média do ano é a radiação sem mês.
        "risco-fogo" | "sol-radiacao" => {
            let mes = p.get("mes").and_then(Value::as_str).map(str::to_string);
            if let Some(m) = mes {
                if m == MEDIA_DO_ANO {
                    p.remove("mes");
                } else if let Some(n) = mes_pelo_nome(&m) {
                    p.insert("mes".into(), json!(n));
                }
            }
        }
        // A escala do GEDTM30, em metros, e as passagens da composição temporal do NISAR e do
        // radar de 10 m, como números.
        "gedtm-relevo" | "nisar-tempo" | "radar-10m" => {
            let campo = if id == "gedtm-relevo" { "escala" } else { "passagens" };
            let e = p.get(campo).and_then(Value::as_str).and_then(|t| t.trim().parse::<f64>().ok());
            if let Some(e) = e {
                p.insert(campo.into(), json!(e));
            }
        }
        // Outro sistema, pelo código EPSG, vale no lugar da escolha.
        "grade" => {
            if let Some(e) = p.remove("epsg") {
                p.insert("crs".into(), e);
            }
        }
        // O código do IBGE no lugar do nome.
        "area-municipio" => {
            let codigo = p
                .get("nome")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|t| t.len() == 7 && t.bytes().all(|b| b.is_ascii_digit()))
                .map(str::to_string);
            if let Some(c) = codigo {
                p.remove("nome");
                p.insert("codigo".into(), json!(c));
            }
        }
        // O dia do mês escrito só com o número.
        "inpe-focos-periodo" => {
            for k in ["de", "ate"] {
                let dia = p.get(k).and_then(Value::as_str).and_then(|t| t.trim().parse::<u32>().ok());
                if let Some(d) = dia {
                    p.insert(k.into(), json!(d));
                }
            }
        }
        _ => {}
    }
}

/// A resposta `r` com o `report` que a janela do comando mostra: os avisos do comando, sem o
/// nome do plugin na frente, quando ele não deu um. Uma resposta que espera a rede, ou que não
/// é um objeto, fica como está.
pub fn com_relato(mut r: Value, avisos: &[String], prefixo: &str) -> Value {
    if avisos.is_empty() || r.get("pending").is_some() {
        return r;
    }
    if let Some(o) = r.as_object_mut()
        && !o.contains_key("report")
    {
        let texto: Vec<&str> = avisos.iter().map(|a| a.strip_prefix(prefixo).unwrap_or(a)).collect();
        o.insert("report".into(), json!(texto.join("\n\n")));
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    fn norm(id: &str, p: Value) -> Value {
        let mut p = p;
        normalizar(id, &mut p);
        p
    }

    #[test]
    fn troca_as_escolhas_pelos_valores() {
        assert_eq!(norm("risco-fogo", json!({"mes": "setembro"})), json!({"mes": 9}));
        assert_eq!(norm("risco-fogo", json!({"mes": 3})), json!({"mes": 3}));
        assert_eq!(
            norm("gedtm-relevo", json!({"variavel": "Índice topográfico de umidade (TWI)", "escala": "240 m"})),
            json!({"variavel": "twi", "escala": 240.0})
        );
        assert_eq!(
            norm(
                "olm-solo",
                json!({"propriedade": "pH em água", "profundidade": "30 a 60 cm", "periodo": "2010 a 2015 (só carbono e pH)"})
            ),
            json!({"propriedade": "ph", "profundidade": "30-60", "periodo": "2010-2015"})
        );
        assert_eq!(norm("sol-radiacao", json!({"mes": MEDIA_DO_ANO})), json!({}));
        assert_eq!(norm("sol-radiacao", json!({"mes": "março"})), json!({"mes": 3}));
        assert_eq!(norm("layout", json!({"perfil": "Folha A3", "estilo": "Pôster"})), json!({"perfil": "a3", "estilo": "poster"}));
        assert_eq!(norm("layout", json!({"perfil": "a2"})), json!({"perfil": "a2"}));
        assert_eq!(norm("layout", json!({"orientacao": "Paisagem (deitada)"})), json!({"orientacao": "paisagem"}));
        assert_eq!(norm("dist-situacao", json!({"paleta": "A oficial da OPERA", "data": ""})), json!({"paleta": "oficial"}));
        assert_eq!(norm("grade", json!({"celula": 30, "crs": SISTEMAS[1].0})), json!({"celula": 30, "crs": "albers"}));
        assert_eq!(norm("grade", json!({"crs": SISTEMAS[0].0, "epsg": "31983"})), json!({"crs": "31983"}));
        assert_eq!(norm("grade", json!({"crs": SISTEMAS[0].0, "epsg": " "})), json!({"crs": "automatico"}));
        assert_eq!(norm("area-municipio", json!({"nome": "3171303", "uf": ""})), json!({"codigo": "3171303"}));
        assert_eq!(norm("area-municipio", json!({"nome": "Viçosa", "uf": "MG"})), json!({"nome": "Viçosa", "uf": "MG"}));
        assert_eq!(
            norm("bdc-sem-nuvens", json!({"satelite": SATELITES_SEM_NUVENS[2].0, "composicao": COMPOSICOES[1].0, "inicio": ""})),
            json!({"satelite": "cbers-4", "composicao": "falsa-cor"})
        );
        assert_eq!(norm("inpe-focos-periodo", json!({"de": "5", "ate": "10/11/2026"})), json!({"de": 5, "ate": "10/11/2026"}));
        // A escolha de outro comando fica como veio.
        assert_eq!(norm("gem15", json!({"modelo": MODELOS_GEM15[2].0})), json!({"modelo": "altura"}));
        assert_eq!(norm("radar-vv", json!({"paleta": "A oficial da OPERA"})), json!({"paleta": "A oficial da OPERA"}));
    }

    #[test]
    fn o_relato_junta_os_avisos_sem_o_prefixo() {
        let avisos = vec!["BrasilData: Grade do projeto: X".to_string(), "outro".to_string()];
        assert_eq!(com_relato(json!({"layer": "g"}), &avisos, "BrasilData: ")["report"], "Grade do projeto: X\n\noutro");
        assert_eq!(com_relato(json!({"report": "meu"}), &avisos, "BrasilData: ")["report"], "meu");
        assert!(com_relato(json!({"pending": 1}), &avisos, "BrasilData: ").get("report").is_none());
        assert!(com_relato(json!({"layer": "g"}), &[], "BrasilData: ").get("report").is_none());
    }

    /// Cada escolha de cada janela do `plugin.json` vira um valor que o comando aceita: um número
    /// (o mês), nada (a média do ano) ou o valor de [`TROCAS`].
    #[test]
    fn conhece_todas_as_escolhas() {
        let m: Value = serde_json::from_str(include_str!("../plugin.json")).unwrap();
        let mut vistas = 0;
        for c in m["commands"].as_array().unwrap() {
            let id = c["id"].as_str().unwrap();
            for f in c["form"].as_array().into_iter().flatten().filter(|f| f["kind"] == "choice") {
                let nome = f["name"].as_str().unwrap();
                for e in f["choices"].as_array().unwrap() {
                    let p = norm(id, json!({nome: e}));
                    assert_ne!(p.get(nome), Some(e), "{id}: a escolha {e} de {nome} não é conhecida");
                    vistas += 1;
                }
            }
        }
        assert!(vistas >= 12 + 13 + 3 + 8 + 3 + 6 + 2 + 3 + 2 + 6 + 5, "{vistas} escolhas");
    }
}
