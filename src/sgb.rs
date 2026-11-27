//! As camadas do Serviço Geológico do Brasil (SGB), do GeoServer do GeoSGB
//! (`geoservicos.sgb.gov.br/geoserver`), e as cores oficiais delas.
//!
//! - **A geologia** (litoestratigrafia) vem em seis escalas, cada uma um tipo do WFS: o mapa
//!   1:1.000.000 do Brasil, os mapas dos estados e as folhas 1:250.000, 1:100.000, 1:50.000 e
//!   1:25.000, que só existem onde houve mapeamento. O comando `geologia-sgb` escolhe uma.
//! - **As cores** são as dos estilos (SLD) do GeoServer: uma regra por sigla, até 2.873 numa escala.
//!   O `scripts/baixar_sgb_cores.py` as lê e escreve o `sgb_cores.csv`, que vem com o plugin:
//!   pedir um SLD de 2 MB a cada mapa seria lento e passaria perto do limite do `host.fetch`.
//! - **As outras camadas** (pontos, geodiversidade, riscos, suscetibilidade, SIAGAS) estão no
//!   `fontes.csv`; daqui vêm só os estilos por classe das que o SGB pinta por classe.
//! - **Conferido em 26/11/2026:** o WFS responde com `Access-Control-Allow-Origin: *`; a escala
//!   1:50.000 traz os campos em maiúsculas (`NOME`, `HIERARQUIA`) e a geometria em `GEOMETRY`.

use crate::Tema;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// O WFS do SGB.
pub const WFS: &str = "https://geoservicos.sgb.gov.br/geoserver/ows";

/// As cores: `chave,valor,cor,contorno`, a chave a de [`Escala::chave`] ou a de um tema.
const CORES: &str = include_str!("../sgb_cores.csv");

const LIVRE: &str = "dados abertos do SGB; licença não declarada no serviço";

/// Os campos da geologia que ficam, nas escalas com os nomes em minúsculas.
const CAMPOS: &str =
    "sigla,nome,hierarquia,litotipos,legenda,idade_min,idade_max,eon_max,era_max,sistema_max,ambiente_tectonico,escala,mapa";
/// Os mesmos, na escala 1:50.000, que os tem em maiúsculas.
const CAMPOS_50K: &str =
    "sigla,NOME,HIERARQUIA,LITOTIPOS,LEGENDA,idade_min,idade_max,EON_MAX,ERA_MAX,SISTEMA_MAX,AMBIENTE_TECTONICO,ESCALA,MAPA";

/// Uma escala da geologia do SGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escala {
    Milhao,
    Estados,
    E250k,
    E100k,
    E50k,
    E25k,
}

/// Todas, da mais geral para a mais detalhada.
pub const ESCALAS: [Escala; 6] = [Escala::Milhao, Escala::Estados, Escala::E250k, Escala::E100k, Escala::E50k, Escala::E25k];

const fn tema(
    id: &'static str,
    nome: &'static str,
    camada: &'static str,
    fonte: &'static str,
    campos: &'static str,
    geom: &'static str,
) -> Tema {
    Tema {
        id,
        grupo: "Meio físico",
        nome,
        servico: WFS,
        tipo: "wfs",
        camada,
        geometria: "poligono",
        recorta: true,
        fonte,
        licenca: LIVRE,
        filtro: "",
        campos,
        geom,
        pagina: 2000,
        ordem: "",
    }
}

/// Os temas das escalas, na ordem de [`ESCALAS`].
static TEMAS: [Tema; 6] = [
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:1.000.000",
        "geosgb:litoestratigrafia_1m",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia 1:1.000.000 (Mapa Geológico do Brasil, Carta Geológica ao Milionésimo)",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, mapa do estado",
        "geosgb:litoestratigrafia_estados",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia dos mapas geológicos estaduais",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:250.000",
        "geosgb:litoestratigrafia_250k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:250.000",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:100.000",
        "geosgb:litoestratigrafia_100k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:100.000",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:50.000",
        "geosgb:litoestratigrafia_50k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:50.000",
        CAMPOS_50K,
        "GEOMETRY",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:25.000",
        "geosgb:litoestratigrafia_25k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:25.000",
        CAMPOS,
        "geometry",
    ),
];

impl Escala {
    pub fn ler(t: Option<&str>) -> Result<Escala, String> {
        let t = t.map(|t| t.trim().to_lowercase().replace(['.', ' '], ""));
        Ok(match t.as_deref() {
            None | Some("") | Some("1:1000000") | Some("1m") | Some("milionesimo") => Escala::Milhao,
            Some("estado") | Some("estados") | Some("estadual") => Escala::Estados,
            Some("1:250000") | Some("250k") => Escala::E250k,
            Some("1:100000") | Some("100k") => Escala::E100k,
            Some("1:50000") | Some("50k") => Escala::E50k,
            Some("1:25000") | Some("25k") => Escala::E25k,
            Some(o) => return Err(format!("escala '{o}': use 1:1.000.000, estado, 1:250.000, 1:100.000, 1:50.000 ou 1:25.000")),
        })
    }

    fn indice(self) -> usize {
        ESCALAS.iter().position(|e| *e == self).unwrap_or(0)
    }

    /// O tema do WFS da escala.
    pub fn tema(self) -> &'static Tema {
        &TEMAS[self.indice()]
    }

    /// A chave das cores no `sgb_cores.csv`.
    pub fn chave(self) -> &'static str {
        ["1m", "estados", "250k", "100k", "50k", "25k"][self.indice()]
    }

    /// Como o aviso a chama.
    pub fn texto(self) -> &'static str {
        ["1:1.000.000", "mapa do estado", "1:250.000", "1:100.000", "1:50.000", "1:25.000"][self.indice()]
    }

    /// O campo do nome da unidade.
    pub fn campo_nome(self) -> &'static str {
        if self == Escala::E50k { "NOME" } else { "nome" }
    }
}

/// A escala da camada `nome` do mapa, se ela é da geologia do SGB.
pub fn escala_da_camada(nome: &str) -> Option<Escala> {
    ESCALAS.into_iter().find(|e| e.tema().camada_no_mapa() == nome)
}

/// A cor de `valor` na chave `chave`, e se ela é só do contorno.
pub fn cor(chave: &str, valor: &str) -> Option<([u8; 3], bool)> {
    CORES.lines().skip(1).find_map(|l| {
        let mut p = l.split(',');
        if p.next()? != chave || p.next()? != valor {
            return None;
        }
        Some((hex(p.next()?)?, p.next() == Some("1")))
    })
}

fn hex(t: &str) -> Option<[u8; 3]> {
    let t = t.strip_prefix('#')?;
    if t.len() != 6 {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

/// As cores da chave `chave`, lidas de uma vez (a busca de [`cor`] percorre a tabela inteira:
/// para as centenas de siglas de um estado, passaria da cota de instruções do plugin).
pub fn cores(chave: &str) -> HashMap<&'static str, [u8; 3]> {
    CORES
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut p = l.split(',');
            (p.next()? == chave).then_some(())?;
            Some((p.next()?, hex(p.next()?)?))
        })
        .collect()
}

/// As cores da geologia na escala `e`. As cartas (1:1.000.000 a 1:25.000) usam a mesma cor para
/// a mesma sigla (em 26/11/2026, 717 de 721 siglas comuns ao 1:1.000.000 e ao 1:100.000), e o SLD
/// de uma escala não tem todas as siglas dela (93 das de Minas Gerais faltam no do 1:1.000.000):
/// a que falta vem da tabela de outra escala das cartas. Os mapas dos estados têm outra paleta
/// (354 de 1.232 iguais) e ficam só com as próprias cores.
pub fn cores_da_escala(e: Escala) -> HashMap<&'static str, [u8; 3]> {
    let mut t = cores(e.chave());
    if e != Escala::Estados {
        for outra in [Escala::E250k, Escala::E100k, Escala::Milhao, Escala::E50k, Escala::E25k] {
            if outra != e {
                for (s, c) in cores(outra.chave()) {
                    t.entry(s).or_insert(c);
                }
            }
        }
    }
    t
}

/// O nome do estilo da geologia de uma escala.
pub fn nome_do_estilo(e: Escala) -> String {
    format!("BrasilData: geologia do SGB, {}", e.texto())
}

/// O estilo por valores únicos da sigla, com as unidades `unidades` (sigla → nome) nas cores do
/// SGB e o rótulo "sigla – nome" na legenda; e quantas não têm cor na tabela (ficam cinza).
pub fn estilo_da_geologia(e: Escala, unidades: &BTreeMap<String, String>) -> (String, usize) {
    let tabela = cores_da_escala(e);
    let mut sem_cor = 0;
    let mut classes = Vec::new();
    let mut rotulos = serde_json::Map::new();
    for (sigla, nome) in unidades {
        let rgb = match tabela.get(sigla.as_str()) {
            Some(c) => *c,
            None => {
                sem_cor += 1;
                [190, 190, 190]
            }
        };
        classes.push(json!([sigla, poligono(rgb, 230)]));
        let rotulo = if nome.trim().is_empty() { sigla.clone() } else { format!("{sigla} – {}", nome.trim()) };
        rotulos.insert(sigla.clone(), Value::String(rotulo));
    }
    let texto = json!([{"name": nome_do_estilo(e), "geometry": "Polygon",
        "renderer": {"Unique": {"field": "sigla", "classes": classes, "other": poligono([190, 190, 190], 200), "ramp": "Harbour", "class_labels": rotulos}},
        "labels": {"enabled": true, "field": "sigla", "size": 7.0, "color": [30, 30, 30, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    (texto, sem_cor)
}

fn poligono(rgb: [u8; 3], alfa: u8) -> Value {
    json!({"fill": [rgb[0], rgb[1], rgb[2], alfa], "stroke": [90, 90, 90, 200], "stroke_width": 0.4, "size": 0.0, "marker": "Circle"})
}

/// O pedido da sigla e do nome das unidades de `e` que cruzam `c`, sem a geometria, em CSV (FID,
/// sigla, nome; 690 kB para 10 mil polígonos, mais leve de ler que o GeoJSON): a página `(início,
/// tamanho)`. O plugin lê só isso para montar a legenda.
pub fn pedido_das_unidades(e: Escala, c: [f64; 4], pagina: (u64, u64)) -> String {
    let t = e.tema();
    format!(
        "{}?service=WFS&version=2.0.0&request=GetFeature&typeNames={}&bbox={:.6},{:.6},{:.6},{:.6},EPSG:4674&outputFormat=csv&propertyName=sigla,{}&count={}&startIndex={}",
        t.servico,
        t.camada,
        c[0],
        c[1],
        c[2],
        c[3],
        e.campo_nome(),
        pagina.1,
        pagina.0
    )
}

/// Junta a `unidades` (sigla → nome, em ordem de sigla) as de uma resposta em CSV a
/// [`pedido_das_unidades`]: `FID,sigla,nome`, o nome entre aspas quando tem vírgula. Uma sigla sem
/// texto fica de fora.
pub fn unidades_csv(texto: &str, unidades: &mut BTreeMap<String, String>) {
    for l in texto.lines().skip(1) {
        let mut p = l.splitn(3, ',');
        let (Some(_), Some(sigla)) = (p.next(), p.next()) else { continue };
        let sigla = sigla.trim().trim_matches('"');
        if sigla.is_empty() || unidades.contains_key(sigla) {
            continue;
        }
        let nome = p.next().unwrap_or("").trim();
        let nome = match nome.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
            Some(n) => n.replace("\"\"", "\""),
            None => nome.to_string(),
        };
        unidades.insert(sigla.to_string(), nome);
    }
}

/// O filtro de uma sigla (para contar as feições dela na camada recortada).
pub fn onde_sigla(sigla: &str) -> String {
    format!("\"sigla\" = '{}'", sigla.replace('\'', "''"))
}

/// O estilo por classe de um tema do `fontes.csv` que o SGB pinta por classe: o nome, o texto.
pub fn estilo_do_tema(id: &str) -> Option<(String, String)> {
    let (chave, campo, valores): (&str, &str, &[&str]) = match id {
        "sgb-suscet-inundacao" => ("inundacao", "classe", &["Alta", "Média", "Baixa"]),
        "sgb-suscet-movimento" => ("movimento", "classe", &["Alta", "Média", "Baixa"]),
        "sgb-risco" => ("risco", "grau_risco", &["Muito alto", "Alto", "Médio"]),
        _ => return None,
    };
    let nome = format!("BrasilData: {id} (cores do SGB)");
    let classes: Vec<Value> = valores
        .iter()
        .filter_map(|v| {
            let (c, contorno) = cor(chave, v)?;
            let simbolo = if contorno {
                json!({"fill": [c[0], c[1], c[2], 60], "stroke": [c[0], c[1], c[2], 255], "stroke_width": 1.6, "size": 0.0, "marker": "Circle"})
            } else {
                json!({"fill": [c[0], c[1], c[2], 200], "stroke": [110, 110, 110, 160], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"})
            };
            Some(json!([v, simbolo]))
        })
        .collect();
    let texto = json!([{"name": nome, "geometry": "Polygon",
        "renderer": {"Unique": {"field": campo, "classes": classes, "other": poligono([190, 190, 190], 150), "ramp": "Harbour"}},
        // O RustGIS recusa um estilo sem `labels` ("missing field `labels`").
        "labels": {"enabled": false, "field": campo, "size": 8.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    Some((nome, texto))
}

/// O teto de feições de um tema do SGB por área, quando é menor que o do catálogo: os polígonos
/// da suscetibilidade são muitos e pequenos (mais de 4.800 só no retângulo de Viçosa).
pub fn teto(id: &str) -> Option<u64> {
    match id {
        "sgb-suscet-movimento" | "sgb-suscet-inundacao" => Some(20_000),
        // Os polígonos da geologia de um estado inteiro: dezenas de megabytes.
        "geologia-sgb" => Some(30_000),
        _ => None,
    }
}

// --- Geofísica (etapa 3): o ArcGIS Server do SGB, que devolve a origem do pedido no CORS. ---

/// Os serviços de mapa do SGB.
pub const ARCGIS: &str = "https://geoportal.sgb.gov.br/server/rest/services";

pub const CITACAO_AERO: &str = "SGB (Serviço Geológico do Brasil), GeoSGB, Levantamentos aerogeofísicos (geofisica/aerogeofisica)";
pub const CITACAO_SISMOS: &str = "SGB (Serviço Geológico do Brasil), catálogo de sismos do Brasil (geofisica/sismos), 1720 a 2013";
pub const CITACAO_MAPAS: &str = "SGB (Serviço Geológico do Brasil), Mapas do Brasil 1:1.000.000 de gamaespectrometria (ternário K, eTh, eU) e de amplitude do sinal analítico magnético (Mapas_Tern_Mag_MIL1)";

/// A camada do arquivo inteiro dos levantamentos, que sai no fim.
pub const BRUTO_AERO: &str = "BrasilData: SGB, levantamentos aerogeofísicos (sem recorte)";
/// O nome dos levantamentos na área.
pub const CAMADA_AERO: &str = "BrasilData: Levantamentos aerogeofísicos (SGB)";

/// Os campos dos levantamentos que vêm do serviço.
const CAMPOS_AERO: [&str; 11] = [
    "TITULO",
    "METODOS",
    "ANO_CONCLUSAO",
    "ALTURA_VOO_M",
    "ESPACAMENTO_LV_M",
    "ESPACAMENTO_LC_KM",
    "DIRECAO_LV",
    "AREA_LEVANTADA_KM2",
    "CONTRATANTE",
    "SITUACAO",
    "IMAGENS",
];

/// O pedido dos 395 levantamentos de uma vez (3 MB): a camada recusa o retângulo ("Unable to
/// complete operation", conferido em 27/11/2026), e o plugin os recorta pela área.
pub fn pedido_aero() -> Value {
    let url =
        format!("{ARCGIS}/geofisica/aerogeofisica/MapServer/0/query?where=1%3D1&outFields={}&outSR=4326&f=geojson", CAMPOS_AERO.join(","));
    json!({"url": url, "name": BRUTO_AERO, "zoom": false})
}

/// O campo do método, para o estilo, e as classes dele, na ordem da legenda.
pub const CAMPO_METODO: &str = "metodo";
pub const METODOS: [(&str, &str); 5] = [
    ("Magnetometria e gamaespectrometria", "#d7301f"),
    ("Gamaespectrometria", "#fc8d59"),
    ("Magnetometria", "#2b8cbe"),
    ("Com gravimetria", "#31a354"),
    ("Com eletromagnetometria", "#756bb1"),
];

/// Os campos novos dos levantamentos: o método resumido e onde obter os dados (o link, quando o
/// SGB dá um; senão o que ele diz, como "SOLICITACAO DIRETAMENTE A CBPM").
pub fn campos_aero() -> Vec<(&'static str, &'static str, String)> {
    let gama = "(\"METODOS\" LIKE '%Gama%' OR \"METODOS\" LIKE '%Cintil%')";
    let metodo = format!(
        "CASE WHEN {gama} AND \"METODOS\" LIKE '%Magn%' THEN '{}' WHEN {gama} THEN '{}' WHEN \"METODOS\" LIKE '%Grav%' THEN '{}' WHEN \"METODOS\" LIKE '%Eletromag%' OR \"METODOS\" LIKE '%VLF%' THEN '{}' ELSE '{}' END",
        METODOS[0].0, METODOS[1].0, METODOS[3].0, METODOS[4].0, METODOS[2].0
    );
    let onde = "CASE WHEN \"IMAGENS\" IS NULL OR trim(\"IMAGENS\") = '' THEN 'sem informação no SGB' ELSE \"IMAGENS\" END".to_string();
    vec![(CAMPO_METODO, "text", metodo), ("onde_obter", "text", onde)]
}

/// Se um campo dos levantamentos fica (o link vai para `onde_obter`).
pub fn fica_aero(c: &str) -> bool {
    c == CAMPO_METODO || c == "onde_obter" || (CAMPOS_AERO.contains(&c) && c != "IMAGENS")
}

/// O estilo dos levantamentos: o contorno, na cor do método.
pub fn estilo_aero() -> (String, String) {
    let nome = "BrasilData: levantamentos aerogeofísicos (SGB)".to_string();
    let classes: Vec<Value> = METODOS
        .iter()
        .map(|(m, cor)| {
            let c = hex(cor).unwrap_or([120, 120, 120]);
            json!([m, {"fill": [c[0], c[1], c[2], 35], "stroke": [c[0], c[1], c[2], 255], "stroke_width": 1.4, "size": 0.0, "marker": "Circle"}])
        })
        .collect();
    let texto = json!([{"name": nome, "geometry": "Polygon",
        "renderer": {"Unique": {"field": CAMPO_METODO, "classes": classes, "other": poligono([150, 150, 150], 30), "ramp": "Harbour"}},
        "labels": {"enabled": false, "field": "TITULO", "size": 7.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    (nome, texto)
}

/// A camada dos sismos do arquivo inteiro, e o nome na área.
pub const BRUTO_SISMOS: &str = "BrasilData: SGB, sismos (sem recorte)";
pub const CAMADA_SISMOS: &str = "BrasilData: Sismos (SGB, 1720 a 2013)";

/// O pedido do menor e do maior `ID` dos sismos (o serviço corta em 1.000 feições e não pagina).
/// Os IDs não começam em 1 (em 27/11/2026, de 3.964 a 6.077), e uma faixa vazia vem como um
/// GeoJSON sem feições, que o RustGIS recusa ("unsupported file type").
pub fn url_ids_sismos() -> String {
    format!(
        "{ARCGIS}/geofisica/sismos/MapServer/0/query?where=1%3D1&outStatistics=%5B%7B%22statisticType%22%3A%22min%22%2C%22onStatisticField%22%3A%22ID%22%2C%22outStatisticFieldName%22%3A%22menor%22%7D%2C%7B%22statisticType%22%3A%22max%22%2C%22onStatisticField%22%3A%22ID%22%2C%22outStatisticFieldName%22%3A%22maior%22%7D%5D&f=json"
    )
}

/// O menor e o maior `ID` na resposta a [`url_ids_sismos`] (o ArcGIS devolve os nomes em
/// maiúsculas).
pub fn ids_sismos(r: &Value) -> Option<(u64, u64)> {
    let a = &r["features"][0]["attributes"];
    let v = |n: &str| a.get(n).or_else(|| a.get(n.to_uppercase().as_str())).and_then(Value::as_f64).map(|v| v as u64);
    let (menor, maior) = (v("menor")?, v("maior")?);
    (menor >= 1 && maior >= menor).then_some((menor, maior))
}

/// Os pedidos dos sismos em faixas de 1.000 IDs, de `menor` a `maior`: cada um cabe no corte do
/// serviço, e o primeiro começa no menor ID.
pub fn pedidos_sismos(menor: u64, maior: u64) -> Vec<Value> {
    let inicio = menor - 1;
    let n = (maior - inicio).div_ceil(1000).max(1);
    (0..n)
        .map(|i| {
            let (de, ate) = (inicio + i * 1000, inicio + (i + 1) * 1000);
            let nome = if n == 1 { BRUTO_SISMOS.to_string() } else { format!("{BRUTO_SISMOS} (parte {} de {n})", i + 1) };
            let url = format!(
                "{ARCGIS}/geofisica/sismos/MapServer/0/query?where=ID+%3E+{de}+AND+ID+%3C%3D+{ate}&outFields=ID,ANO,DIA_MES,MAGNITUDE,PROFUNDIDADE,CATEGORIA,IO,LOCALIDADE,UF,FONTE&outSR=4326&f=geojson"
            );
            json!({"url": url, "name": nome, "zoom": false})
        })
        .collect()
}

/// Os campos novos dos sismos: o ano e a magnitude como números (vêm como texto, a magnitude com
/// vírgula: "2,3").
pub fn campos_sismos() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("ano_evento", "double", "to_real(\"ANO\")".to_string()),
        ("magnitude_num", "double", "to_real(replace(\"MAGNITUDE\", ',', '.'))".to_string()),
    ]
}

/// Se um campo dos sismos fica.
pub fn fica_sismos(c: &str) -> bool {
    matches!(c, "ano_evento" | "magnitude_num" | "DIA_MES" | "PROFUNDIDADE" | "CATEGORIA" | "IO" | "LOCALIDADE" | "UF" | "FONTE")
}

/// Os sismos que ficam: sem a categoria E, os de fora do Brasil sentidos aqui (67 em 27/11/2026,
/// como o do Chile de 1906, Ms 8,4), que o catálogo põe na cidade brasileira onde foram sentidos,
/// com a magnitude da origem.
pub const FILTRO_SISMOS: &str = "\"ID\" > 0 AND (\"CATEGORIA\" IS NULL OR \"CATEGORIA\" <> 'E')";

/// As faixas de magnitude dos sismos.
pub const CLASSES_SISMOS: [(f64, &str, &str); 5] = [
    (2.0, "menos de 2", "#fee5d9"),
    (3.0, "2 a 3", "#fcae91"),
    (4.0, "3 a 4", "#fb6a4a"),
    (5.0, "4 a 5", "#de2d26"),
    (f64::INFINITY, "5 ou mais", "#a50f15"),
];

/// Um mapa do Brasil do SGB, só para ver: o ternário de gamaespectrometria ou a amplitude do sinal
/// analítico magnético, que o SGB só serve como imagem (o `export` do MapServer; não há WMS).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MapaGeofisico {
    Ternario,
    Magnetico,
}

impl MapaGeofisico {
    pub fn ler(t: Option<&str>) -> Result<MapaGeofisico, String> {
        match t.map(|t| t.trim().to_lowercase()).as_deref() {
            None | Some("") | Some("ternario") | Some("gama") => Ok(MapaGeofisico::Ternario),
            Some("magnetico") | Some("magnetometria") | Some("amplitude") => Ok(MapaGeofisico::Magnetico),
            Some(o) => Err(format!("mapa '{o}': use ternario ou magnetico")),
        }
    }

    /// A camada do MapServer (`TERN_Brasil.tif` é a 0; `AM_Brasil.tif`, a 1).
    fn camada(self) -> u8 {
        match self {
            MapaGeofisico::Ternario => 0,
            MapaGeofisico::Magnetico => 1,
        }
    }

    /// O nome do mapa no projeto.
    pub fn nome(self) -> &'static str {
        match self {
            MapaGeofisico::Ternario => "BrasilData: Ternário de gamaespectrometria do Brasil (SGB)",
            MapaGeofisico::Magnetico => "BrasilData: Amplitude do sinal analítico magnético do Brasil (SGB)",
        }
    }

    /// O endereço das imagens, no modelo que o RustGIS preenche com o retângulo de cada tile.
    pub fn url(self) -> String {
        format!(
            "{ARCGIS}/Mapas_Tern_Mag_MIL1/MapServer/export?bbox={{bbox-epsg-3857}}&bboxSR=3857&imageSR=3857&size=256,256&format=png32&transparent=true&f=image&layers=show:{}",
            self.camada()
        )
    }

    /// Como o RustGIS junta o mapa ao que está embaixo: o magnético vem com o branco opaco onde
    /// não houve voo (alfa 255 em todo pixel, mesmo com `transparent=true`; conferido em 27/11/2026),
    /// e "multiply" o deixa transparente; o ternário já vem transparente.
    pub fn mistura(self) -> &'static str {
        match self {
            MapaGeofisico::Ternario => "normal",
            MapaGeofisico::Magnetico => "multiply",
        }
    }

    /// Como ler as cores.
    pub fn leitura(self) -> &'static str {
        match self {
            MapaGeofisico::Ternario => {
                "as cores somam os três radioelementos: vermelho é potássio (K), verde é tório (eTh) e azul é urânio (eU); branco, altos nos três; preto, baixos nos três"
            }
            MapaGeofisico::Magnetico => {
                "as cores quentes marcam a amplitude alta do sinal analítico magnético, sobre as bordas dos corpos magnéticos; as frias, a baixa"
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn as_cores_do_sld() {
        // Conferidas no SLD de 26/11/2026.
        assert_eq!(cor("1m", "A23br"), Some(([0x48, 0x63, 0x21], false)));
        assert_eq!(cor("movimento", "Alta"), Some(([0xc4, 0x6d, 0x1b], false)));
        assert_eq!(cor("risco", "Muito alto"), Some(([0xe9, 0x07, 0x00], true)));
        assert_eq!(cor("1m", "nao-existe"), None);
        assert_eq!(cor("250k", "A23br").is_some(), CORES.lines().any(|l| l.starts_with("250k,A23br,")));
        for e in ESCALAS {
            let n = CORES.lines().filter(|l| l.starts_with(&format!("{},", e.chave()))).count();
            assert!(n > 100, "{} só tem {n} cores", e.chave());
        }
        assert!(CORES.lines().skip(1).all(|l| l.split(',').count() == 4 && hex(l.split(',').nth(2).unwrap()).is_some()));
    }

    #[test]
    fn as_escalas() {
        assert_eq!(Escala::ler(None), Ok(Escala::Milhao));
        assert_eq!(Escala::ler(Some("1:100.000")), Ok(Escala::E100k));
        assert_eq!(Escala::ler(Some("Estado")), Ok(Escala::Estados));
        assert!(Escala::ler(Some("1:150.000")).is_err());
        assert_eq!(Escala::E50k.tema().geom, "GEOMETRY");
        assert!(Escala::E50k.tema().campos.contains("NOME"));
        assert_eq!(escala_da_camada("BrasilData: Geologia do SGB, 1:250.000"), Some(Escala::E250k));
        assert_eq!(escala_da_camada("BrasilData: Geologia (IBGE)"), None);
        assert!(ESCALAS.iter().all(|e| e.tema().pedido_proprio()));
    }

    #[test]
    fn o_estilo_so_com_as_unidades_da_area() {
        let csv = "FID,sigla,nome\nl.1,NP3_gamma,Suíte X\nl.2,A23br,Complexo Barro\nl.3,A23br,Complexo Barro\nl.4,,sem sigla\nl.5,Q2a,\"Depósitos, aluvionares \"\"novos\"\"\"\n";
        let mut u = BTreeMap::new();
        unidades_csv(csv, &mut u);
        assert_eq!(u.keys().collect::<Vec<_>>(), ["A23br", "NP3_gamma", "Q2a"]);
        assert_eq!(u["Q2a"], "Depósitos, aluvionares \"novos\"");
        assert_eq!(u["A23br"], "Complexo Barro");
        u.remove("Q2a");
        let (texto, sem_cor) = estilo_da_geologia(Escala::Milhao, &u);
        let s: Value = serde_json::from_str(&texto).unwrap();
        let un = &s[0]["renderer"]["Unique"];
        assert_eq!(un["classes"].as_array().unwrap().len(), 2);
        assert_eq!(un["classes"][0][1]["fill"], json!([0x48, 0x63, 0x21, 230]));
        assert_eq!(un["class_labels"]["A23br"], "A23br – Complexo Barro");
        assert_eq!(sem_cor, usize::from(cor("1m", "NP3_gamma").is_none()));
        assert_eq!(cores("1m").get("A23br"), Some(&[0x48, 0x63, 0x21]));
        assert_eq!(cores("1m").len(), CORES.lines().filter(|l| l.starts_with("1m,")).count());
        // Uma sigla que falta no SLD do 1:1.000.000 vem da tabela de outra carta, nunca da dos estados.
        let falta = cores("100k").into_keys().find(|s| !cores("1m").contains_key(s)).unwrap();
        assert_eq!(cores_da_escala(Escala::Milhao).get(falta), cores("100k").get(falta));
        assert_eq!(cores_da_escala(Escala::Estados).len(), cores("estados").len());
    }

    #[test]
    fn os_pedidos() {
        let u = pedido_das_unidades(Escala::E50k, [-43.0, -20.9, -42.75, -20.6], (2000, 1000));
        assert!(
            u.contains("typeNames=geosgb:litoestratigrafia_50k") && u.contains("propertyName=sigla,NOME") && u.contains("outputFormat=csv")
        );
        assert!(u.contains("&count=1000&startIndex=2000") && u.contains("bbox=-43.000000,-20.900000,-42.750000,-20.600000,EPSG:4674"));
        assert_eq!(onde_sigla("PP4_gamma'b"), "\"sigla\" = 'PP4_gamma''b'");
    }

    #[test]
    fn a_geofisica() {
        let p = pedido_aero();
        assert!(
            p["url"].as_str().unwrap().contains("aerogeofisica/MapServer/0/query?where=1%3D1&outFields=TITULO,") && p["name"] == BRUTO_AERO
        );
        assert!(fica_aero("onde_obter") && fica_aero("TITULO") && !fica_aero("IMAGENS") && !fica_aero("OBJECTID"));
        let c = campos_aero();
        assert!(c[0].2.starts_with("CASE WHEN (\"METODOS\" LIKE '%Gama%'") && c[0].2.contains("'Com gravimetria'"));
        let (_, e) = estilo_aero();
        let s: Value = serde_json::from_str(&e).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), 5);
        assert_eq!(s[0]["labels"]["enabled"], false);
        assert_eq!(ids_sismos(&json!({"features": [{"attributes": {"MENOR": 3964.0, "MAIOR": 6077.0}}]})), Some((3964, 6077)));
        assert_eq!(ids_sismos(&json!({"features": [{"attributes": {"MAIOR": 6077.0}}]})), None);
        let ps = pedidos_sismos(3964, 6077);
        assert_eq!(ps.len(), 3);
        assert!(ps[0]["url"].as_str().unwrap().contains("where=ID+%3E+3963+AND+ID+%3C%3D+4963"));
        assert!(ps[2]["url"].as_str().unwrap().contains("ID+%3E+5963+AND+ID+%3C%3D+6963"));
        assert_eq!(ps[2]["name"], format!("{BRUTO_SISMOS} (parte 3 de 3)"));
        assert_eq!(pedidos_sismos(1, 900)[0]["name"], BRUTO_SISMOS);
        assert!(fica_sismos("magnitude_num") && !fica_sismos("MAGNITUDE") && !fica_sismos("ID"));
        assert_eq!(MapaGeofisico::ler(Some("magnetico")), Ok(MapaGeofisico::Magnetico));
        assert!(MapaGeofisico::ler(Some("gravimetria")).is_err());
        assert!(MapaGeofisico::Ternario.url().ends_with("&layers=show:0") && MapaGeofisico::Magnetico.url().ends_with("&layers=show:1"));
        assert!(MapaGeofisico::Ternario.url().contains("export?bbox={bbox-epsg-3857}&bboxSR=3857"));
        assert_eq!(MapaGeofisico::Magnetico.mistura(), "multiply");
        assert!(FILTRO_SISMOS.contains("<> 'E'"));
    }

    #[test]
    fn os_estilos_dos_temas() {
        let (_, t) = estilo_do_tema("sgb-suscet-inundacao").unwrap();
        let s: Value = serde_json::from_str(&t).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), 3);
        let (_, r) = estilo_do_tema("sgb-risco").unwrap();
        let s: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["field"], "grau_risco");
        assert_eq!(s[0]["renderer"]["Unique"]["classes"][0][1]["stroke_width"], 1.6);
        assert_eq!(s[0]["labels"]["enabled"], false, "o RustGIS recusa um estilo sem labels");
        assert!(estilo_do_tema("sgb-afloramentos").is_none());
        assert_eq!(teto("sgb-suscet-movimento"), Some(20_000));
        assert_eq!(teto("sgb-risco"), None);
    }
}
