//! Os indicadores de seca do GRACE (NASA): o percentil semanal da água no solo e no subsolo, do modelo
//! de superfície da NASA (Catchment) que assimila as medidas de gravidade dos satélites GRACE e
//! GRACE-FO, em células de 0,25° (cerca de 28 km) no mundo inteiro.
//!
//! - **De onde:** o centro de seca da Universidade de Nebraska, que distribui os mapas da NASA:
//!   `globaldata/AAAAMMDD/` por semana (toda segunda-feira, desde 03/02/2003; faltam duas), um GeoTIFF
//!   de cerca de 0,9 MB por indicador, sem login. Não libera a leitura pelo navegador (CORS).
//! - **Os valores:** o percentil de 0 a 100 da semana na distribuição de 1948 a 2012 do mesmo lugar e
//!   da mesma época: 5 é mais seco que em 95% do tempo. −999 é sem dado (mar, gelo).
//! - **A semana:** o plugin não tem relógio: a lista do servidor diz as que há ([`semana`]).

use serde_json::Value;

use crate::datas::Data;

/// A pasta das semanas no servidor (a lista delas é a página da própria pasta).
pub const BASE: &str = "https://nasagrace.unl.edu/globaldata/";

/// A primeira semana da série.
pub const PRIMEIRA: Data = Data { ano: 2003, mes: 2, dia: 3 };

/// O começo do nome das camadas.
pub const PREFIXO_CAMADA: &str = "BrasilData: Seca (GRACE, NASA): ";

/// A margem (graus) da janela lida em volta da área: um pouco mais que uma célula.
pub const MARGEM: f64 = 0.3;

/// Uma área menor que isto (graus, no maior lado) tem poucas células.
pub const AREA_PEQUENA: f64 = 1.5;

pub const CITACAO: &str = "NASA Goddard Space Flight Center, GRACE Data Assimilation for Drought Monitoring (GRACE-DA-DM Global 3.0), distribuído pelo National Drought Mitigation Center, Universidade de Nebraska-Lincoln (nasagrace.unl.edu); Li, B. et al. (2019), Global GRACE data assimilation for groundwater and drought monitoring, Water Resources Research 55, 7564–7586, doi:10.1029/2018WR024618";

pub const LICENCA: &str = "CC BY 4.0 no registro de dados abertos da AWS; o GES DISC da NASA o diz aberto e sem restrição, com a citação";

/// O limite de cima de cada classe do percentil, a cor (as do mapa da NASA) e o que ela quer dizer.
pub const CLASSES: [(f64, &str); 11] = [
    (2.0, "#730000"),
    (5.0, "#E60000"),
    (10.0, "#FFAA00"),
    (20.0, "#FCD37F"),
    (30.0, "#FFFF00"),
    (70.0, "#FFFFFF"),
    (80.0, "#BFFFFF"),
    (90.0, "#7FD4FF"),
    (95.0, "#3C9DFF"),
    (98.0, "#1A1AFF"),
    (100.0, "#0D0090"),
];

/// Qual água.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indicador {
    Raizes,
    Subterranea,
    Superficie,
}

pub const INDICADORES: [Indicador; 3] = [Indicador::Raizes, Indicador::Subterranea, Indicador::Superficie];

impl Indicador {
    /// O indicador pelo nome do parâmetro.
    pub fn ler(t: &str) -> Option<Indicador> {
        match t.trim().to_lowercase().as_str() {
            "raizes" | "raízes" => Some(Indicador::Raizes),
            "subterranea" | "subterrânea" => Some(Indicador::Subterranea),
            "superficie" | "superfície" => Some(Indicador::Superficie),
            _ => None,
        }
    }

    /// O começo do nome do arquivo.
    fn arquivo(self) -> &'static str {
        match self {
            Indicador::Raizes => "rtzsm",
            Indicador::Subterranea => "gws",
            Indicador::Superficie => "sfsm",
        }
    }

    /// O nome na camada.
    pub fn nome(self) -> &'static str {
        match self {
            Indicador::Raizes => "umidade nas raízes",
            Indicador::Subterranea => "água subterrânea",
            Indicador::Superficie => "umidade na superfície",
        }
    }

    /// O que é, por extenso.
    pub fn titulo(self) -> &'static str {
        match self {
            Indicador::Raizes => "umidade do primeiro metro do solo (zona das raízes)",
            Indicador::Subterranea => "água subterrânea",
            Indicador::Superficie => "umidade da superfície do solo",
        }
    }
}

/// A semana "AAAAMMDD" do endereço.
fn compacta(d: Data) -> String {
    format!("{}{:02}{:02}", d.ano, d.mes, d.dia)
}

/// O arquivo do indicador na semana `d`.
pub fn url(i: Indicador, d: Data) -> String {
    let s = compacta(d);
    format!("{BASE}{s}/{}_perc_025deg_GL_{s}.tif", i.arquivo())
}

/// O nome da camada.
pub fn camada(i: Indicador, d: Data) -> String {
    format!("{PREFIXO_CAMADA}{}, {}", i.nome(), d.texto())
}

/// O indicador e a semana da camada `nome`, se ela é daqui.
pub fn da_camada(nome: &str) -> Option<(Indicador, Data)> {
    let (qual, quando) = nome.strip_prefix(PREFIXO_CAMADA)?.rsplit_once(", ")?;
    let i = INDICADORES.into_iter().find(|i| i.nome() == qual)?;
    Some((i, Data::ler(quando)?))
}

/// A semana pedida, na lista do servidor (a página da pasta): a última que começa até `pedida`, ou a
/// mais recente sem ela.
pub fn semana(lista: &str, pedida: Option<Data>) -> Result<Data, String> {
    let (mut ultima, mut escolhida): (Option<Data>, Option<Data>) = (None, None);
    for (i, marca) in lista.match_indices("/globaldata/") {
        let Some(t) = lista.get(i + marca.len()..i + marca.len() + 8).filter(|t| t.bytes().all(|b| b.is_ascii_digit())) else { continue };
        let (Ok(ano), Ok(mes), Ok(dia)) = (t[..4].parse(), t[4..6].parse(), t[6..].parse()) else { continue };
        let d = Data { ano, mes, dia };
        ultima = ultima.max(Some(d));
        if pedida.is_none_or(|p| d <= p) {
            escolhida = escolhida.max(Some(d));
        }
    }
    let ultima = ultima.ok_or("o servidor do GRACE (nasagrace.unl.edu) não listou as semanas: tente de novo daqui a pouco")?;
    if let Some(p) = pedida
        && p.dias() > ultima.dias() + 6
    {
        return Err(format!("a semana mais recente do GRACE é a de {}: peça uma data até ela, ou deixe a data vazia", ultima.texto()));
    }
    escolhida.ok_or(format!("a série do GRACE começa em {}", PRIMEIRA.texto()))
}

/// A janela lida em volta da área `c`, dentro do arquivo (60° S a 90° N).
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [(c[0] - MARGEM).max(-180.0), (c[1] - MARGEM).max(-60.0), (c[2] + MARGEM).min(180.0), (c[3] + MARGEM).min(90.0)]
}

/// O estilo, no formato do QGIS que o `style.importQgis` lê: as classes do percentil, nas cores da NASA.
pub fn qml() -> String {
    let itens: String =
        CLASSES.iter().map(|(v, c)| format!("<item value=\"{v}\" color=\"{c}\" alpha=\"255\" label=\"até {v}\"/>")).collect();
    format!(
        "<qgis version=\"3.34\"><pipe><rasterrenderer type=\"singlebandpseudocolor\" band=\"1\" opacity=\"1\" alphaBand=\"-1\" classificationMin=\"0\" classificationMax=\"100\"><rastershader><colorrampshader colorRampType=\"DISCRETE\" classificationMode=\"1\" clip=\"0\" minimumValue=\"0\" maximumValue=\"100\">{itens}</colorrampshader></rastershader></rasterrenderer></pipe></qgis>"
    )
}

/// A lista das semanas na resposta do servidor.
pub fn lista(resposta: Option<&Value>) -> &str {
    resposta.and_then(|r| r["text"].as_str()).unwrap_or("")
}

#[cfg(test)]
mod testes {
    use super::*;

    const D: fn(i64, u32, u32) -> Data = |ano, mes, dia| Data { ano, mes, dia };

    #[test]
    fn as_semanas() {
        let l = "<A HREF=\"/globaldata/20030203/\">20030203</A> <A HREF=\"/globaldata/20261116/\">x</A> <A HREF=\"/globaldata/20261123/\">x</A> <A HREF=\"/globaldata/current/\">x</A> <A HREF=\"/globaldata/web.config\">x</A>";
        assert_eq!(semana(l, None).unwrap(), D(2026, 11, 23));
        assert_eq!(semana(l, Some(D(2026, 11, 22))).unwrap(), D(2026, 11, 16));
        assert_eq!(semana(l, Some(D(2026, 11, 29))).unwrap(), D(2026, 11, 23));
        assert!(semana(l, Some(D(2026, 11, 30))).unwrap_err().contains("23/11/2026"));
        assert!(semana(l, Some(D(2003, 2, 2))).unwrap_err().contains("03/02/2003"));
        assert!(semana("", None).is_err());
    }

    #[test]
    fn os_nomes_e_os_enderecos() {
        let d = D(2026, 11, 23);
        assert!(url(Indicador::Subterranea, d).ends_with("/globaldata/20261123/gws_perc_025deg_GL_20261123.tif"));
        for i in INDICADORES {
            assert_eq!(da_camada(&camada(i, d)), Some((i, d)));
        }
        assert_eq!(da_camada("BrasilData: Seca: Anomalia da umidade do solo (GDO, 01/10/2026)"), None);
        assert_eq!(Indicador::ler("Raízes"), Some(Indicador::Raizes));
        assert_eq!(Indicador::ler("lua"), None);
        assert_eq!(janela([-74.0, -59.9, -34.0, 6.0]), [-74.3, -60.0, -33.7, 6.3]);
    }

    #[test]
    fn o_estilo_tem_as_classes() {
        let q = qml();
        assert_eq!(q.matches("<item ").count(), 11);
        assert!(q.contains("value=\"100\"") && q.contains("DISCRETE"));
        assert!(CLASSES.windows(2).all(|w| w[0].0 < w[1].0));
    }
}
