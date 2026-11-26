//! Os pontos de solo do SoilData, o Repositório Brasileiro de Dados do Solo (Laboratório de
//! Pedometria da UTFPR, com o MapBiomas): as camadas de solo descritas e analisadas nos
//! levantamentos e nas pesquisas publicados nele.
//!
//! - **De onde:** o *Brazilian Soil Dataset 2024* (Samuel-Rosa & Horst), um arquivo de texto
//!   separado por tabulações (19,8 MB, CC BY 4.0) que junta os conjuntos do repositório, uma linha
//!   por camada de solo. O Dataverse do SoilData o serve pelo número do arquivo, sem login, com
//!   CORS aberto e leitura por trechos.
//! - **Quem faz o trabalho:** o RustGIS. Ele abre o arquivo inteiro como pontos (`x`, `y` e
//!   `delimiter` do `layer.addUrl`, desde a 0.5.0), recorta pela área, seleciona as camadas da
//!   profundidade e calcula os campos; o plugin não lê as linhas (a cota de instruções de um
//!   comando não daria para milhares de pontos).
//! - **O arquivo:** 57.077 camadas, 9.755 sem coordenada (entram sem geometria e o recorte as
//!   tira); 47.322 com coordenada, de 14.334 perfis de 204 conjuntos (26/11/2026). Os atributos
//!   vêm como texto, porque os vazios são `NA`: o valor sai com `to_real`. Há alguns valores
//!   negativos (erros), que ficam de fora.
//! - **Licenças:** a de cada conjunto de origem, no campo `dataset_licenca`: CC BY 4.0 ou CC BY-NC
//!   4.0 (sem uso comercial, 40% das linhas). As duas ficam na camada, e o aviso conta as NC.
//! - **Classes antigas:** o `taxon_sibcs` mistura o SiBCS atual com os nomes de antes de 1999
//!   (Podzólico, Terra Roxa Estruturada, Areias Quartzosas, Glei...); a ordem vem de uma tabela de
//!   equivalências aproximadas ([`expressao_ordem`]).

use serde_json::{Value, json};

use crate::PREFIXO;

/// O arquivo do Brazilian Soil Dataset 2024 no Dataverse do SoilData (doi:10.60502/SoilData/BCAV2B).
pub const URL: &str = "https://repositorio.soildata.mapbiomas.org/api/access/datafile/832";

/// A camada com o arquivo inteiro, que sai depois do recorte.
pub const BRUTO: &str = "BrasilData: SoilData do Brasil (sem recorte)";

/// As colunas das coordenadas, o separador e o sistema do arquivo, para o `layer.addUrl`.
pub fn pedido() -> Value {
    json!({"url": URL, "name": BRUTO, "x": "coord_x", "y": "coord_y", "delimiter": "\t", "crs": "EPSG:4326", "zoom": false})
}

/// A citação, como o repositório pede, com a do conjunto de origem de cada ponto.
pub const CITACAO: &str = "Samuel-Rosa, A.; Horst, T. Z. Brazilian Soil Dataset 2024. SoilData, Repositório Brasileiro de Dados do Solo (UTFPR e MapBiomas), doi:10.60502/SoilData/BCAV2B; cada ponto vem de um conjunto do SoilData (campos dataset_id e dataset_titulo), que também se cita";

/// A licença: a do arquivo, e a de cada conjunto de origem no campo `dataset_licenca`.
pub const LICENCA: &str = "CC BY 4.0 (o arquivo); cada ponto tem a licença do conjunto de origem no campo dataset_licenca: CC BY 4.0 ou CC BY-NC 4.0 (sem uso comercial)";

/// As linhas de conjuntos sem uso comercial.
pub const NAO_COMERCIAL: &str = "\"dataset_licenca\" LIKE '%NC%'";

/// O campo da classe, pelo qual os pontos são coloridos.
pub const CAMPO_CLASSE: &str = "classe";

/// As profundidades que se pode pedir, em cm: a camada que contém a profundidade. Sem 20 cm,
/// porque muitas camadas terminam nela (0–20 cm) e a seguinte começa nela: a 20 cm sobram metade
/// das camadas de 10 ou 30 cm.
pub const PROFUNDIDADES: [u32; 5] = [5, 10, 30, 50, 100];

/// A profundidade sem o parâmetro.
pub const PROFUNDIDADE_PADRAO: u32 = 10;

/// Os campos do arquivo que ficam na camada, além do valor e da classe.
const FICAM: [&str; 12] = [
    "dataset_id",
    "observacao_id",
    "dataset_titulo",
    "dataset_licenca",
    "estado_id",
    "municipio_id",
    "coord_precisao",
    "coord_fonte",
    "data_ano",
    "taxon_sibcs",
    "profund_sup",
    "profund_inf",
];

/// Uma classe de valores: o limite de cima (fora dela), o nome na legenda e a cor.
pub type Classe = (f64, &'static str, &'static str);

/// O atributo que colore os pontos.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Atributo {
    Argila,
    Areia,
    Silte,
    Carbono,
    Ctc,
    Ph,
    Densidade,
    /// A ordem do SiBCS de cada perfil (um ponto por perfil, sem profundidade).
    Classe,
}

impl Atributo {
    /// Todos, na ordem da janela.
    pub const TODOS: [Atributo; 8] = [
        Atributo::Argila,
        Atributo::Areia,
        Atributo::Silte,
        Atributo::Carbono,
        Atributo::Ctc,
        Atributo::Ph,
        Atributo::Densidade,
        Atributo::Classe,
    ];

    /// O valor do parâmetro `atributo`.
    pub fn valor(self) -> &'static str {
        match self {
            Atributo::Argila => "argila",
            Atributo::Areia => "areia",
            Atributo::Silte => "silte",
            Atributo::Carbono => "carbono",
            Atributo::Ctc => "ctc",
            Atributo::Ph => "ph",
            Atributo::Densidade => "densidade",
            Atributo::Classe => "classe",
        }
    }

    /// O atributo do parâmetro `atributo` (sem ele, a argila).
    pub fn ler(t: Option<&str>) -> Result<Atributo, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Atributo::Argila);
        }
        Atributo::TODOS.into_iter().find(|a| a.valor() == t).ok_or_else(|| {
            let todos: Vec<&str> = Atributo::TODOS.iter().map(|a| a.valor()).collect();
            format!("atributo '{t}': use {}", todos.join(", "))
        })
    }

    /// A coluna do arquivo.
    pub fn coluna(self) -> &'static str {
        match self {
            Atributo::Argila => "argila",
            Atributo::Areia => "areia",
            Atributo::Silte => "silte",
            Atributo::Carbono => "carbono",
            Atributo::Ctc => "ctc",
            Atributo::Ph => "ph",
            Atributo::Densidade => "dsi",
            Atributo::Classe => "taxon_sibcs",
        }
    }

    /// O campo numérico da camada, com a unidade no nome (nenhum na classe). Outro nome que o da
    /// coluna, que é texto e sai.
    pub fn campo(self) -> Option<&'static str> {
        match self {
            Atributo::Argila => Some("argila_gkg"),
            Atributo::Areia => Some("areia_gkg"),
            Atributo::Silte => Some("silte_gkg"),
            Atributo::Carbono => Some("carbono_gkg"),
            Atributo::Ctc => Some("ctc_cmolc_kg"),
            Atributo::Ph => Some("ph_medido"),
            Atributo::Densidade => Some("densidade_gcm3"),
            Atributo::Classe => None,
        }
    }

    /// O nome, para a camada e o aviso.
    pub fn nome(self) -> &'static str {
        match self {
            Atributo::Argila => "Argila",
            Atributo::Areia => "Areia",
            Atributo::Silte => "Silte",
            Atributo::Carbono => "Carbono orgânico",
            Atributo::Ctc => "CTC",
            Atributo::Ph => "pH",
            Atributo::Densidade => "Densidade do solo",
            Atributo::Classe => "Classe do solo",
        }
    }

    /// As classes de valores, do menor ao maior (vazio na classe do solo). A última vai até o fim.
    pub fn classes(self) -> &'static [Classe] {
        const INF: f64 = f64::INFINITY;
        match self {
            // Os grupos de textura do SiBCS pela argila (a arenosa e a siltosa dependem também da
            // areia e do silte: abaixo de 150 g/kg fica sem nome).
            Atributo::Argila => &[
                (150.0, "menos de 150 g/kg", "#F3DFA2"),
                (350.0, "150 a 350 g/kg (textura média)", "#E0A458"),
                (600.0, "350 a 600 g/kg (argilosa)", "#B9603A"),
                (INF, "600 g/kg ou mais (muito argilosa)", "#7A2E1E"),
            ],
            Atributo::Areia => &[
                (200.0, "menos de 200 g/kg", "#FFF7D6"),
                (400.0, "200 a 400 g/kg", "#FBE3A1"),
                (600.0, "400 a 600 g/kg", "#F2C55C"),
                (800.0, "600 a 800 g/kg", "#D99A2B"),
                (INF, "800 g/kg ou mais", "#A8691A"),
            ],
            Atributo::Silte => &[
                (100.0, "menos de 100 g/kg", "#F2F0F7"),
                (200.0, "100 a 200 g/kg", "#CBC9E2"),
                (300.0, "200 a 300 g/kg", "#9E9AC8"),
                (450.0, "300 a 450 g/kg", "#756BB1"),
                (INF, "450 g/kg ou mais", "#54278F"),
            ],
            Atributo::Carbono => &[
                (5.0, "menos de 5 g/kg", "#F6EBD9"),
                (10.0, "5 a 10 g/kg", "#DDBF91"),
                (20.0, "10 a 20 g/kg", "#B88A55"),
                (40.0, "20 a 40 g/kg", "#835A30"),
                (INF, "40 g/kg ou mais", "#432C17"),
            ],
            Atributo::Ctc => &[
                (4.0, "menos de 4 cmolc/kg", "#EDF8E9"),
                (8.0, "4 a 8 cmolc/kg", "#BAE4B3"),
                (15.0, "8 a 15 cmolc/kg", "#74C476"),
                (27.0, "15 a 27 cmolc/kg", "#31A354"),
                (INF, "27 cmolc/kg ou mais", "#006D2C"),
            ],
            // As classes de reação do solo da Embrapa (Manual de descrição e coleta de solo).
            Atributo::Ph => &[
                (4.3, "extremamente ácido (menos de 4,3)", "#B2182B"),
                (5.4, "fortemente ácido (4,3 a 5,3)", "#EF8A62"),
                (6.6, "moderadamente ácido (5,4 a 6,5)", "#FDDBC7"),
                (7.4, "praticamente neutro (6,6 a 7,3)", "#D1E5F0"),
                (8.4, "moderadamente alcalino (7,4 a 8,3)", "#67A9CF"),
                (INF, "fortemente alcalino (mais de 8,3)", "#2166AC"),
            ],
            Atributo::Densidade => &[
                (1.0, "menos de 1,0 g/cm³", "#F1EEF6"),
                (1.2, "1,0 a 1,2 g/cm³", "#D7B5D8"),
                (1.4, "1,2 a 1,4 g/cm³", "#DF65B0"),
                (1.6, "1,4 a 1,6 g/cm³", "#DD1C77"),
                (INF, "1,6 g/cm³ ou mais", "#980043"),
            ],
            Atributo::Classe => &[],
        }
    }
}

/// A profundidade do parâmetro `profundidade` (texto ou número, em cm; sem ele, 10 cm).
pub fn ler_profundidade(texto: Option<&str>, numero: Option<f64>) -> Result<u32, String> {
    let t = texto.map(str::trim).unwrap_or("");
    let d = match (t.is_empty(), numero) {
        (false, _) => t.trim_end_matches("cm").trim().parse::<f64>().ok(),
        (true, Some(n)) => Some(n),
        (true, None) => return Ok(PROFUNDIDADE_PADRAO),
    };
    let lista = PROFUNDIDADES.map(|p| p.to_string()).join(", ");
    match d {
        Some(d) if d.fract() == 0.0 && PROFUNDIDADES.contains(&(d as u32)) => Ok(d as u32),
        _ => Err(format!(
            "profundidade '{}': use {lista} (cm)",
            if t.is_empty() { numero.map(|n| n.to_string()).unwrap_or_default() } else { t.to_string() }
        )),
    }
}

/// O nome da camada.
pub fn camada(a: Atributo, d: u32) -> String {
    match a {
        Atributo::Classe => format!("{PREFIXO}Classe do solo nos perfis (SoilData)"),
        _ => format!("{PREFIXO}{} a {d} cm (SoilData)", a.nome()),
    }
}

/// Se `nome` é uma camada deste comando (para a tabela de fontes).
pub fn eh_daqui(nome: &str) -> bool {
    nome.starts_with(PREFIXO) && nome.ends_with(" (SoilData)")
}

/// As camadas que ficam: as que contêm a profundidade `d` e têm o atributo, com valor de zero
/// para cima; na classe do solo, a primeira camada de cada perfil. Uns poucos perfis (de 16 a 34
/// por profundidade, em 14 mil) têm duas camadas que se sobrepõem na profundidade: os dois pontos
/// ficam.
pub fn filtro(a: Atributo, d: u32) -> String {
    match a {
        Atributo::Classe => "\"camada_id\" = 1".into(),
        _ => {
            let c = a.coluna();
            format!("\"profund_sup\" <= {d} AND \"profund_inf\" > {d} AND \"{c}\" <> 'NA' AND to_real(\"{c}\") >= 0")
        }
    }
}

/// Um número como o RustGIS o lê numa expressão.
fn num(v: f64) -> String {
    let t = format!("{v}");
    if t.contains('.') { t } else { format!("{t}.0") }
}

/// O texto entre aspas simples de uma expressão.
fn texto(t: &str) -> String {
    format!("'{}'", t.replace('\'', "''"))
}

/// A expressão da classe de um atributo de valores: o nome da faixa do campo numérico.
pub fn expressao_faixas(campo: &str, classes: &[Classe]) -> String {
    let mut e = String::from("CASE");
    let (ultima, antes) = classes.split_last().expect("um atributo de valores tem classes");
    for (lim, nome, _) in antes {
        e += &format!(" WHEN \"{campo}\" < {} THEN {}", num(*lim), texto(nome));
    }
    e += &format!(" ELSE {} END", texto(ultima.1));
    e
}

/// As ordens do SiBCS (Embrapa, 5ª edição, 2018), com os começos do `taxon_sibcs` (em maiúsculas)
/// que dão cada uma: o nome atual e os nomes de antes de 1999 que correspondem a ela, pela tabela
/// de correspondência do SiBCS. A correspondência é aproximada: um Podzólico Vermelho-Escuro pode
/// ser hoje um Nitossolo, e uma Terra Roxa Estruturada, um Latossolo. A ordem conta: o Podzol
/// (Espodossolo) vem antes do Podzólico (Argissolo). No arquivo de 2024, só 116 dos 14 mil perfis
/// ficam em outra classe (abreviações como Ca Tb, aterros, nomes locais).
pub const ORDENS: [(&str, &str, &[&str]); 13] = [
    ("Espodossolos", "#A3A3C2", &["PODZOL", "ESPODOSSOLO", "PODZOL "]),
    ("Latossolos", "#B5442E", &["LATOSSOLO", "LATOSOL", "TERRA ROXA LEG"]),
    ("Argissolos", "#E8A33D", &["ARGISSOLO", "PODZ%LICO", "SOLOS PODZOLIZADOS", "SOLO PODZOLIZADO", "ALISSOLO"]),
    ("Nitossolos", "#7D2E2E", &["NITOSSOLO", "TERRA ROXA ESTRUTURADA", "TERRA BRUNA ESTRUTURADA"]),
    ("Cambissolos", "#C9B26B", &["CAMBISSOLO", "CAMBISOL"]),
    (
        "Neossolos",
        "#E9D8A6",
        &[
            "NEOSSOLO",
            "AREIA",
            "SOLO LIT",
            "SOLOS LIT",
            "SOLO L%TICO",
            "LITOSSOLO",
            "LITOSOL",
            "SOLO ALUVIA",
            "SOLOS ALUVIA",
            "REGOSSOLO",
            "REGOSOL",
        ],
    ),
    ("Gleissolos", "#5B8DB8", &["GLEI", "GLEY", "SOLO GLEI", "SOLOS GLEI"]),
    ("Plintossolos", "#D9C04E", &["PLINTOSSOLO", "LATERITA", "SOLO CONCRECION", "SOLOS CONCRECION"]),
    ("Planossolos", "#8FB3A3", &["PLANOSSOLO", "PLANOSOL", "SOLONETZ"]),
    ("Chernossolos", "#6B4A2B", &["CHERNOSSOLO", "BRUNIZ", "RENDZINA"]),
    ("Luvissolos", "#C07A9B", &["LUVISSOLO", "BRUNO N%C%LCICO", "MEDITERR"]),
    ("Organossolos", "#3A3A3A", &["ORGANOSSOLO", "SOLO ORG%NICO", "SOLOS ORG%NICOS"]),
    ("Vertissolos", "#6E7F3A", &["VERTISSOLO", "VERTISOL", "GRUMUSSOLO"]),
];

/// O ponto sem classe no arquivo (vazio ou `NA`).
pub const SEM_CLASSE: &str = "sem classe no levantamento";

/// O ponto com uma classe que não é de uma ordem (abreviações, aterros, nomes locais).
pub const OUTRA: &str = "outra classe";

/// A expressão da ordem do SiBCS pelo `taxon_sibcs` (ver [`ORDENS`]).
pub fn expressao_ordem() -> String {
    let t = "upper(trim(coalesce(\"taxon_sibcs\", '')))";
    let mut e = format!("CASE WHEN {t} = '' OR {t} = 'NA' THEN {}", texto(SEM_CLASSE));
    for (nome, _, comecos) in ORDENS {
        let casos: Vec<String> = comecos
            .iter()
            .map(|c| if *c == "PODZOL" { format!("{t} = 'PODZOL'") } else { format!("{t} LIKE {}", texto(&format!("{c}%"))) })
            .collect();
        e += &format!(" WHEN {} THEN {}", casos.join(" OR "), texto(nome));
    }
    e += &format!(" ELSE {} END", texto(OUTRA));
    e
}

/// Os campos novos: (nome, tipo, expressão), na ordem em que se calculam.
pub fn campos(a: Atributo) -> Vec<(&'static str, &'static str, String)> {
    match a.campo() {
        Some(campo) => {
            vec![(campo, "double", format!("to_real(\"{}\")", a.coluna())), (CAMPO_CLASSE, "text", expressao_faixas(campo, a.classes()))]
        }
        None => vec![(CAMPO_CLASSE, "text", expressao_ordem())],
    }
}

/// Se o campo `c` fica na camada de `a`.
pub fn fica_o_campo(a: Atributo, c: &str) -> bool {
    FICAM.contains(&c) || c == CAMPO_CLASSE || Some(c) == a.campo()
}

/// O nome do estilo do atributo.
pub fn nome_do_estilo(a: Atributo) -> String {
    format!("{PREFIXO}SoilData ({})", a.valor())
}

fn rgba(h: &str) -> Value {
    let n = u32::from_str_radix(h.trim_start_matches('#'), 16).unwrap_or(0);
    json!([(n >> 16) & 255, (n >> 8) & 255, n & 255, 255])
}

/// O limite de uma faixa como o RustGIS o lê: uma classe graduada vai até o limite, com ele
/// (`v <= limite`), e as faixas daqui param antes dele (`v < limite`, como os nomes dizem: 4,3 já é
/// fortemente ácido). Os valores do arquivo têm no máximo duas casas.
fn limite_graduado(lim: f64) -> f64 {
    if lim.is_finite() { lim - 1e-6 } else { 1e9 }
}

/// O símbolo de um ponto: um círculo da cor `cor` com borda escura.
fn simbolo(cor: &str) -> Value {
    json!({"fill": rgba(cor), "stroke": [50, 50, 50, 255], "stroke_width": 0.5, "size": 6.0, "marker": "Circle"})
}

/// O ponto sem classe: um círculo vazio, cinza.
fn vazio() -> Value {
    json!({"fill": [0, 0, 0, 0], "stroke": [120, 120, 120, 255], "stroke_width": 0.8, "size": 5.0, "marker": "Circle"})
}

/// Um estilo de pontos no formato do `style.add`, com o `renderer` dado e os rótulos (desligados)
/// pelo campo `rotulo`.
fn estilo_de_pontos(nome: &str, renderer: Value, rotulo: &str) -> String {
    json!([{"name": nome, "geometry": "Point",
        "renderer": renderer,
        "labels": {"enabled": false, "field": rotulo, "size": 7.5, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string()
}

/// O estilo graduado `nome` dos pontos pelo campo numérico `campo`, nas faixas `classes`: a legenda
/// as mostra na ordem, do menor ao maior (as únicas sairiam em ordem alfabética).
pub fn estilo_graduado(nome: &str, campo: &str, classes: &[Classe]) -> String {
    let rotulos: serde_json::Map<String, Value> = classes.iter().enumerate().map(|(i, (_, n, _))| (i.to_string(), json!(n))).collect();
    let renderer = json!({"Graduated": {
        "field": campo,
        "breaks": classes.iter().map(|(lim, _, _)| limite_graduado(*lim)).collect::<Vec<_>>(),
        "symbols": classes.iter().map(|(_, _, cor)| simbolo(cor)).collect::<Vec<_>>(),
        "ramp": "Ember", "method": "NaturalBreaks", "min": 0.0, "classes": classes.len(),
        "class_labels": rotulos,
    }});
    estilo_de_pontos(nome, renderer, campo)
}

/// O estilo dos pontos: nos valores, as faixas do atributo pelo campo numérico
/// ([`estilo_graduado`]); na classe do solo, a ordem do SiBCS.
pub fn estilo(a: Atributo) -> String {
    match a.campo() {
        Some(campo) => estilo_graduado(&nome_do_estilo(a), campo, a.classes()),
        None => {
            let mut classes: Vec<Value> = ORDENS.iter().map(|(nome, cor, _)| json!([nome, simbolo(cor)])).collect();
            classes.push(
                json!([OUTRA, {"fill": [190, 190, 190, 255], "stroke": [50, 50, 50, 255], "stroke_width": 0.5, "size": 5.0, "marker": "Circle"}]),
            );
            classes.push(json!([SEM_CLASSE, vazio()]));
            let renderer = json!({"Unique": {"field": CAMPO_CLASSE, "classes": classes, "other": vazio(), "ramp": "Harbour"}});
            estilo_de_pontos(&nome_do_estilo(a), renderer, "taxon_sibcs")
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn le_o_atributo_e_a_profundidade() {
        assert_eq!(Atributo::ler(None).unwrap(), Atributo::Argila);
        assert_eq!(Atributo::ler(Some(" ph ")).unwrap(), Atributo::Ph);
        assert!(Atributo::ler(Some("fosforo")).unwrap_err().contains("argila, areia"));
        assert_eq!(ler_profundidade(None, None).unwrap(), 10);
        assert_eq!(ler_profundidade(Some("30"), None).unwrap(), 30);
        assert_eq!(ler_profundidade(Some("50 cm"), None).unwrap(), 50);
        assert_eq!(ler_profundidade(Some(" "), Some(100.0)).unwrap(), 100);
        assert!(ler_profundidade(Some("20"), None).unwrap_err().contains("5, 10, 30, 50, 100"));
        assert!(ler_profundidade(None, Some(7.5)).is_err());
    }

    #[test]
    fn o_filtro_pega_a_camada_que_contem_a_profundidade() {
        assert_eq!(
            filtro(Atributo::Densidade, 30),
            "\"profund_sup\" <= 30 AND \"profund_inf\" > 30 AND \"dsi\" <> 'NA' AND to_real(\"dsi\") >= 0"
        );
        assert_eq!(filtro(Atributo::Classe, 30), "\"camada_id\" = 1");
    }

    #[test]
    fn as_faixas_vao_do_menor_ao_maior() {
        for a in Atributo::TODOS.into_iter().filter(|a| *a != Atributo::Classe) {
            let c = a.classes();
            assert!(c.len() >= 4, "{a:?}");
            assert!(c.windows(2).all(|w| w[0].0 < w[1].0), "{a:?}");
            assert!(c.last().unwrap().0.is_infinite(), "{a:?}");
        }
        let e = expressao_faixas("ph_medido", Atributo::Ph.classes());
        assert!(e.starts_with("CASE WHEN \"ph_medido\" < 4.3 THEN 'extremamente ácido (menos de 4,3)' WHEN \"ph_medido\" < 5.4 "));
        assert!(e.ends_with(" ELSE 'fortemente alcalino (mais de 8,3)' END"));
        assert!(expressao_faixas("argila_gkg", Atributo::Argila.classes()).contains("< 150.0 THEN"));
    }

    #[test]
    fn a_ordem_le_os_nomes_antigos() {
        let e = expressao_ordem();
        assert!(e.starts_with("CASE WHEN upper(trim(coalesce(\"taxon_sibcs\", ''))) = '' OR "));
        assert!(e.contains("= 'PODZOL' OR "));
        assert!(e.contains("LIKE 'PODZ%LICO%'"));
        // O Podzol (Espodossolo) vem antes do Podzólico (Argissolo).
        assert!(e.find("'Espodossolos'").unwrap() < e.find("'Argissolos'").unwrap());
        assert!(e.ends_with(" ELSE 'outra classe' END"));
        assert_eq!(e.matches(" THEN ").count(), ORDENS.len() + 1);
    }

    #[test]
    fn os_campos_e_o_que_fica() {
        let c = campos(Atributo::Carbono);
        assert_eq!(c[0], ("carbono_gkg", "double", "to_real(\"carbono\")".to_string()));
        assert_eq!(c[1].0, CAMPO_CLASSE);
        assert_eq!(campos(Atributo::Classe).len(), 1);
        assert!(fica_o_campo(Atributo::Carbono, "carbono_gkg"));
        assert!(!fica_o_campo(Atributo::Carbono, "carbono"));
        assert!(!fica_o_campo(Atributo::Carbono, "argila"));
        assert!(fica_o_campo(Atributo::Classe, "taxon_sibcs"));
        assert!(fica_o_campo(Atributo::Classe, CAMPO_CLASSE));
    }

    #[test]
    fn os_nomes_das_camadas() {
        assert_eq!(camada(Atributo::Ph, 10), "BrasilData: pH a 10 cm (SoilData)");
        assert_eq!(camada(Atributo::Classe, 10), "BrasilData: Classe do solo nos perfis (SoilData)");
        assert!(eh_daqui(&camada(Atributo::Carbono, 50)));
        assert!(!eh_daqui(BRUTO));
    }

    #[test]
    fn o_estilo_tem_as_classes() {
        let s: Value = serde_json::from_str(&estilo(Atributo::Argila)).unwrap();
        let g = &s[0]["renderer"]["Graduated"];
        assert_eq!(g["field"], "argila_gkg");
        assert_eq!(g["symbols"].as_array().unwrap().len(), 4);
        assert_eq!(g["symbols"][0]["fill"], json!([243, 223, 162, 255]));
        assert_eq!(g["class_labels"]["3"], "600 g/kg ou mais (muito argilosa)");
        // 150 g/kg já é da segunda faixa, e o último limite pega tudo.
        let b: Vec<f64> = g["breaks"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();
        assert!(b[0] < 150.0 && b[0] > 149.99);
        assert_eq!(b[3], 1e9);
        let s: Value = serde_json::from_str(&estilo(Atributo::Ph)).unwrap();
        assert!(s[0]["renderer"]["Graduated"]["breaks"][0].as_f64().unwrap() < 4.3);
        let s: Value = serde_json::from_str(&estilo(Atributo::Classe)).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), ORDENS.len() + 2);
        assert_eq!(pedido()["delimiter"], "\t");
    }
}
