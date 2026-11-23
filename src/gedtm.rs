//! As variáveis do relevo do GEDTM30 (OpenGeoHub, Ho e Hengl, 2025), o modelo digital do terreno
//! global a 30 m feito por aprendizado de máquina sobre os pontos de laser do ICESat-2 e do GEDI:
//! declividade, curvaturas, posição, rugosidade, aberturas, geomorfons, umidade e fator LS, as
//! covariáveis de relevo do mapeamento de solos.
//!
//! - **De onde:** COGs globais nos servidores `s3.opengeohub.org` e `ceph.opengeohub.org` (sem
//!   login, licença CC BY 4.0), um por variável e por escala: WGS 84, inteiros com o valor
//!   multiplicado (por 100, 1.000 ou 10.000; o RustGIS não aplica a escala do arquivo, o plugin
//!   divide), com pirâmides de dois em dois.
//! - **Que arquivos:** só os que o RustGIS lê por partes. Em boa parte dos arquivos da OpenGeoHub
//!   o cabeçalho da imagem inteira fica no fim do arquivo (a centenas de GB), e o RustGIS só o
//!   procura nos primeiros 2 GB (opengeos/RustGIS#2569); esses ficam de fora até ele ler o
//!   cabeçalho onde estiver. A elevação vem da cópia da OpenTopography (`opentopography.s3.sdsc.edu`,
//!   com o cabeçalho no começo e um servidor bem mais rápido); a incerteza dela não tem cópia
//!   legível. As variáveis de 30 m são as da v1.0 (sobre o EDTM, de dezembro de 2024) e, das
//!   escalas maiores, ficam as de 480 e 960 m (todas nos geomorfons; 240 m na curvatura anelar).
//! - **A escala:** a variável calculada sobre o terreno de 480 ou 960 m não é a média da de 30 m:
//!   a declividade ou a umidade de uma encosta inteira aparecem na escala maior, os detalhes de
//!   30 m somem. Uma escala que a variável não tem dá a mais próxima que ela tem, com aviso.
//! - **Como:** só a janela da área (`layer.addUrl` com `bbox`), do nível de pirâmide mais grosso
//!   cujas células não passam das da grade (menos os geomorfons: a pirâmide deles é a média dos
//!   códigos); o valor passa à unidade, vai para a grade do projeto e é recortado pela área.
//! - **App web:** os dois servidores mandam o cabeçalho CORS: funciona também no navegador.

use crate::cog_global::Arquivo;

const S3: &str = "https://s3.opengeohub.org/global/dtm";
const CEPH: &str = "https://ceph.opengeohub.org/arco/dtm";
const OPENTOPOGRAPHY: &str = "https://opentopography.s3.sdsc.edu/raster";

/// Um segundo de arco, a célula dos arquivos da v1.2.
const SEGUNDO: f64 = 1.0 / 3600.0;

/// A célula dos arquivos da v1.0, sobre o EDTM.
const EDTM: f64 = 0.00025;

/// As escalas em que as variáveis foram calculadas, em metros.
pub const ESCALAS: [u32; 6] = [30, 60, 120, 240, 480, 960];

/// A variável do relevo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variavel {
    Elevacao,
    Declividade,
    Twi,
    Dfme,
    Rugosidade,
    CurvMinima,
    CurvMaxima,
    CurvPerfil,
    CurvTangencial,
    CurvAnelar,
    IndiceForma,
    AberturaPositiva,
    AberturaNegativa,
    Sombreamento,
    Geomorfons,
    FatorLs,
}

/// As escalas maiores de uma variável no `ceph`: o código, o período, a versão, a célula do
/// terreno de 30 m e as escalas que o RustGIS lê.
struct Maiores {
    codigo: &'static str,
    periodo: &'static str,
    versao: &'static str,
    graus: f64,
    escalas: &'static [u32],
}

const P12: &str = "20060101_20151231";
const P10: &str = "20000101_20221231";

impl Variavel {
    pub const TODAS: [Variavel; 16] = [
        Variavel::Elevacao,
        Variavel::Declividade,
        Variavel::Twi,
        Variavel::Dfme,
        Variavel::Rugosidade,
        Variavel::CurvMinima,
        Variavel::CurvMaxima,
        Variavel::CurvPerfil,
        Variavel::CurvTangencial,
        Variavel::CurvAnelar,
        Variavel::IndiceForma,
        Variavel::AberturaPositiva,
        Variavel::AberturaNegativa,
        Variavel::Sombreamento,
        Variavel::Geomorfons,
        Variavel::FatorLs,
    ];

    /// O valor do parâmetro `variavel`.
    pub fn valor(self) -> &'static str {
        match self {
            Variavel::Elevacao => "elevacao",
            Variavel::Declividade => "declividade",
            Variavel::Twi => "twi",
            Variavel::Dfme => "dfme",
            Variavel::Rugosidade => "rugosidade",
            Variavel::CurvMinima => "curvatura-minima",
            Variavel::CurvMaxima => "curvatura-maxima",
            Variavel::CurvPerfil => "curvatura-perfil",
            Variavel::CurvTangencial => "curvatura-tangencial",
            Variavel::CurvAnelar => "curvatura-anelar",
            Variavel::IndiceForma => "indice-forma",
            Variavel::AberturaPositiva => "abertura-positiva",
            Variavel::AberturaNegativa => "abertura-negativa",
            Variavel::Sombreamento => "sombreamento",
            Variavel::Geomorfons => "geomorfons",
            Variavel::FatorLs => "fator-ls",
        }
    }

    /// A variável do parâmetro `variavel` (sem ele, a elevação).
    pub fn ler(t: Option<&str>) -> Result<Variavel, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Variavel::Elevacao);
        }
        Variavel::TODAS.into_iter().find(|v| v.valor() == t).ok_or_else(|| {
            let todas: Vec<&str> = Variavel::TODAS.iter().map(|v| v.valor()).collect();
            format!("variável '{t}': use {}", todas.join(", "))
        })
    }

    /// O arquivo de 30 m que o RustGIS lê: o servidor, o caminho e a célula.
    fn de_30m(self) -> Option<(&'static str, &'static str, f64)> {
        let v3 = |c: &'static str| Some((S3, c, EDTM));
        match self {
            Variavel::Elevacao => {
                Some((OPENTOPOGRAPHY, "GEDTM30/GEDTM30_be/gedtm_rf_m_30m_s_20060101_20151231_go_epsg.4326.3855_v1.2.tif", SEGUNDO))
            }
            Variavel::Declividade => v3("v3/slope.in.degree_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::Twi => v3("v3/twi_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::Dfme => v3("v3/dfme_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::Rugosidade => v3("v3/ssdon_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::CurvMinima => v3("v3/minic_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::CurvMaxima => v3("v3/maxic_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::CurvPerfil => v3("v3/pro.curv_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::CurvTangencial => v3("v3/tan.curv_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::FatorLs => v3("v3/ls.factor_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"),
            Variavel::Sombreamento => Some((CEPH, "gedtmlsp_hillshd_m_30m_sg_20060101_20151231_go01_epsg4326_v1.2.0_srl03.tif", SEGUNDO)),
            Variavel::CurvAnelar
            | Variavel::IndiceForma
            | Variavel::AberturaPositiva
            | Variavel::AberturaNegativa
            | Variavel::Geomorfons => None,
        }
    }

    /// As escalas maiores no `ceph`, se a variável as tem.
    fn maiores(self) -> Option<Maiores> {
        let v12 = |codigo| Maiores { codigo, periodo: P12, versao: "v1.2.0", graus: SEGUNDO, escalas: &[480, 960] };
        let v10 = |codigo, escalas| Maiores { codigo, periodo: P10, versao: "v1.0.0", graus: EDTM, escalas };
        match self {
            Variavel::Twi => Some(v12("twi")),
            Variavel::Dfme => Some(v12("dfme")),
            Variavel::Rugosidade => Some(v12("ssdon")),
            Variavel::CurvMinima => Some(v12("minic")),
            Variavel::CurvMaxima => Some(v12("maxic")),
            Variavel::CurvAnelar => Some(v10("ringcurv", &[240, 480, 960])),
            Variavel::IndiceForma => Some(v10("shpindx", &[480, 960])),
            Variavel::AberturaPositiva => Some(v10("posopen", &[480, 960])),
            Variavel::AberturaNegativa => Some(v10("negopen", &[480, 960])),
            Variavel::Geomorfons => Some(v10("geomrphn", &ESCALAS)),
            Variavel::Elevacao
            | Variavel::Declividade
            | Variavel::CurvPerfil
            | Variavel::CurvTangencial
            | Variavel::Sombreamento
            | Variavel::FatorLs => None,
        }
    }

    /// As escalas que a variável tem, da menor para a maior.
    pub fn escalas(self) -> Vec<u32> {
        let mut e: Vec<u32> = self.de_30m().map(|_| 30).into_iter().collect();
        if let Some(m) = self.maiores() {
            for x in m.escalas {
                if !e.contains(x) {
                    e.push(*x);
                }
            }
        }
        e
    }

    /// Se é uma classe (os geomorfons), e não um valor contínuo.
    pub fn classes(self) -> bool {
        self == Variavel::Geomorfons
    }

    /// Por quanto o arquivo multiplica o valor.
    pub fn divisor(self) -> f64 {
        match self {
            Variavel::Elevacao | Variavel::Sombreamento | Variavel::Geomorfons => 1.0,
            Variavel::Declividade
            | Variavel::Dfme
            | Variavel::Rugosidade
            | Variavel::Twi
            | Variavel::AberturaPositiva
            | Variavel::AberturaNegativa => 100.0,
            Variavel::CurvMinima
            | Variavel::CurvMaxima
            | Variavel::CurvPerfil
            | Variavel::CurvTangencial
            | Variavel::IndiceForma
            | Variavel::FatorLs => 1000.0,
            Variavel::CurvAnelar => 10000.0,
        }
    }

    /// O nome, como entra no nome da camada.
    pub fn nome(self) -> &'static str {
        match self {
            Variavel::Elevacao => "Elevação do terreno",
            Variavel::Declividade => "Declividade",
            Variavel::Twi => "Índice topográfico de umidade",
            Variavel::Dfme => "Diferença da elevação média",
            Variavel::Rugosidade => "Rugosidade do terreno",
            Variavel::CurvMinima => "Curvatura mínima",
            Variavel::CurvMaxima => "Curvatura máxima",
            Variavel::CurvPerfil => "Curvatura de perfil",
            Variavel::CurvTangencial => "Curvatura tangencial",
            Variavel::CurvAnelar => "Curvatura anelar",
            Variavel::IndiceForma => "Índice de forma",
            Variavel::AberturaPositiva => "Abertura positiva",
            Variavel::AberturaNegativa => "Abertura negativa",
            Variavel::Sombreamento => "Sombreamento",
            Variavel::Geomorfons => "Geomorfons",
            Variavel::FatorLs => "Fator LS",
        }
    }

    /// A unidade (vazia num índice sem unidade).
    pub fn unidade(self) -> &'static str {
        match self {
            Variavel::Elevacao | Variavel::Dfme => "m",
            Variavel::Declividade | Variavel::Rugosidade | Variavel::AberturaPositiva | Variavel::AberturaNegativa => "graus",
            Variavel::CurvMinima | Variavel::CurvMaxima | Variavel::CurvPerfil | Variavel::CurvTangencial => "1/m",
            Variavel::CurvAnelar => "1/m²",
            Variavel::Geomorfons => "classe",
            Variavel::IndiceForma | Variavel::Sombreamento | Variavel::Twi | Variavel::FatorLs => "",
        }
    }

    /// Os valores possíveis depois da divisão, para tirar o que não for terreno.
    pub fn faixa(self) -> (f64, f64) {
        match self {
            Variavel::Elevacao => (-500.0, 9000.0),
            Variavel::Declividade => (0.0, 90.0),
            Variavel::Dfme => (-1000.0, 1000.0),
            Variavel::Rugosidade => (0.0, 90.0),
            Variavel::CurvMinima | Variavel::CurvMaxima | Variavel::CurvPerfil | Variavel::CurvTangencial => (-32.0, 32.0),
            Variavel::CurvAnelar => (-3.3, 3.3),
            Variavel::IndiceForma => (-1.0, 1.0),
            Variavel::AberturaPositiva | Variavel::AberturaNegativa => (0.0, 180.0),
            Variavel::Sombreamento => (0.0, 65534.0),
            Variavel::Geomorfons => (0.5, 10.5),
            Variavel::Twi => (-50.0, 50.0),
            Variavel::FatorLs => (0.0, 65.0),
        }
    }

    /// A rampa (o nome de uma do RustGIS e se vai invertida); o esticamento usa o mínimo e o
    /// máximo da área.
    pub fn rampa(self) -> (&'static str, bool) {
        match self {
            Variavel::Elevacao => ("Terrain", false),
            Variavel::Declividade | Variavel::FatorLs => ("YlOrBr", false),
            Variavel::Rugosidade => ("OrRd", false),
            Variavel::Dfme
            | Variavel::CurvMinima
            | Variavel::CurvMaxima
            | Variavel::CurvPerfil
            | Variavel::CurvTangencial
            | Variavel::CurvAnelar
            | Variavel::IndiceForma => ("RdBu", true),
            Variavel::AberturaPositiva | Variavel::AberturaNegativa | Variavel::Sombreamento => ("Greys", true),
            Variavel::Twi => ("YlGnBu", false),
            Variavel::Geomorfons => ("", false),
        }
    }

    /// O arquivo da variável na escala `escala` (m, uma das [`Variavel::escalas`]), com a célula e
    /// as pirâmides que servem: as dos geomorfons são a média dos códigos, que não é classe nenhuma,
    /// e eles se leem sempre da imagem inteira.
    pub fn arquivo(self, escala: u32) -> Arquivo {
        if escala == 30
            && let Some((servidor, caminho, graus)) = self.de_30m()
        {
            return Arquivo { url: format!("{servidor}/{caminho}"), graus, piramides: 10 };
        }
        let m = self.maiores().expect("uma escala que a variável tem");
        let vezes = escala / 30;
        Arquivo {
            url: format!("{CEPH}/gedtmlsp_{}_m_{escala}m_sg_{}_go01_epsg4326_{}_srl03.tif", m.codigo, m.periodo, m.versao),
            graus: m.graus * f64::from(vezes),
            piramides: if self.classes() { 0 } else { 10 - vezes.trailing_zeros() as i32 },
        }
    }
}

/// A escala do parâmetro `escala` (em metros; sem ela, 30), conferida.
pub fn ler_escala(e: Option<f64>) -> Result<u32, String> {
    let Some(e) = e else { return Ok(30) };
    let m = e as u32;
    if e.fract() != 0.0 || !ESCALAS.contains(&m) {
        return Err("a escala do GEDTM30 é de 30, 60, 120, 240, 480 ou 960 m".into());
    }
    Ok(m)
}

/// A escala que se lê da variável `v` quando se pede `pedida`: ela mesma, se a variável a tem;
/// senão, a mais próxima que ela tem (a menor das maiores, ou a maior), com o aviso.
pub fn escala_lida(v: Variavel, pedida: u32) -> (u32, Option<String>) {
    let tem = v.escalas();
    if tem.contains(&pedida) {
        return (pedida, None);
    }
    let lida = tem.iter().copied().find(|&e| e > pedida).or_else(|| tem.last().copied()).unwrap_or(30);
    let lista: Vec<String> = tem.iter().map(|e| format!("{e} m")).collect();
    let aviso = format!(
        "{}: o RustGIS só lê esta variável do GEDTM30 nas escalas de {}; veio a de {lida} m, no lugar da de {pedida} m",
        v.nome(),
        juntar(&lista)
    );
    (lida, Some(aviso))
}

/// "a, b e c".
fn juntar(itens: &[String]) -> String {
    match itens {
        [] => String::new(),
        [um] => um.clone(),
        [inicio @ .., ultimo] => format!("{} e {ultimo}", inicio.join(", ")),
    }
}

/// O nome da camada.
pub fn camada(v: Variavel, escala: u32) -> String {
    let u = if v.unidade().is_empty() || v.classes() { String::new() } else { format!(", {}", v.unidade()) };
    format!("BrasilData: {} {escala} m{u} (GEDTM30)", v.nome())
}

/// A camada de passagem com a janela lida no nível `k`.
pub fn janela_nome(v: Variavel, escala: u32, k: i32) -> String {
    format!("BrasilData: GEDTM30 {} {escala} m nível {k} (janela)", v.valor())
}

/// O que é a camada `nome`, se for do GEDTM30.
pub fn da_camada(nome: &str) -> Option<(Variavel, u32)> {
    Variavel::TODAS.into_iter().flat_map(|v| ESCALAS.into_iter().map(move |e| (v, e))).find(|(v, e)| camada(*v, *e) == nome)
}

/// Os geomorfons: o valor, o nome em português e a cor (as da r.geomorphon do GRASS).
pub const GEOMORFONS: [(u8, &str, &str); 10] = [
    (1, "Plano", "#dcdcdc"),
    (2, "Pico", "#380000"),
    (3, "Crista", "#c80000"),
    (4, "Ombro", "#ff5014"),
    (5, "Esporão", "#fad23c"),
    (6, "Encosta", "#ffff3c"),
    (7, "Concavidade", "#b4e614"),
    (8, "Sopé", "#3cfa96"),
    (9, "Vale", "#0000ff"),
    (10, "Depressão", "#000038"),
];

/// O estilo dos geomorfons (paleta com os nomes), no formato do QGIS que o `style.importQgis` lê.
pub fn qml_geomorfons() -> String {
    let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
    for (v, nome, cor) in GEOMORFONS {
        q.push_str(&format!(r#"<paletteEntry value="{v}" color="{cor}" alpha="255" label="{nome}"/>"#));
    }
    q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
    q
}

/// O que cada código dos geomorfons quer dizer, para o aviso.
pub fn chave_geomorfons() -> String {
    GEOMORFONS.iter().map(|(v, n, _)| format!("{v} {}", n.to_lowercase())).collect::<Vec<_>>().join(", ")
}

/// A citação, como a licença pede.
pub fn citacao(v: Variavel, escala: u32) -> String {
    let de_onde = if v == Variavel::Elevacao {
        "lido da cópia distribuída pela OpenTopography (doi:10.5069/G9BV7DT1)"
    } else {
        "lido dos servidores da OpenGeoHub"
    };
    format!(
        "Ho, Y.-F., Grohmann, C. H., Lindsay, J., Reuter, H. I., Parente, L., Witjes, M. e Hengl, T., Global Ensemble Digital Terrain modeling and parametrization at 30 m resolution (GEDTM30), PeerJ 13, e19673 (2025), doi:10.7717/peerj.19673; dados: Ho, Y.-F. e Hengl, T., Global Ensemble Digital Terrain Model 30m (GEDTM30), Zenodo, doi:10.5281/zenodo.14900181 (OpenGeoHub). {} na escala de {escala} m; {de_onde}, posto na grade do projeto e recortado pela área",
        v.nome()
    )
}

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enderecos() {
        let a = Variavel::Declividade.arquivo(30);
        assert_eq!(
            a.url,
            "https://s3.opengeohub.org/global/dtm/v3/slope.in.degree_edtm_m_30m_s_20000101_20221231_go_epsg.4326_v20241230.tif"
        );
        assert_eq!((a.graus, a.piramides), (EDTM, 10));
        let t = Variavel::Twi.arquivo(480);
        assert_eq!(t.url, "https://ceph.opengeohub.org/arco/dtm/gedtmlsp_twi_m_480m_sg_20060101_20151231_go01_epsg4326_v1.2.0_srl03.tif");
        assert!((t.graus - 16.0 / 3600.0).abs() < 1e-12);
        assert_eq!(t.piramides, 6);
        let g = Variavel::Geomorfons.arquivo(30);
        assert!(g.url.ends_with("gedtmlsp_geomrphn_m_30m_sg_20000101_20221231_go01_epsg4326_v1.0.0_srl03.tif"));
        // As pirâmides dos geomorfons são médias de códigos: não se leem.
        assert_eq!((g.graus, g.piramides), (EDTM, 0));
        assert_eq!(Variavel::Geomorfons.arquivo(480).piramides, 0);
        assert_eq!(Variavel::AberturaPositiva.arquivo(960).piramides, 5);
        assert!(Variavel::Sombreamento.arquivo(30).url.starts_with(CEPH));
        let e = Variavel::Elevacao.arquivo(30);
        assert_eq!(
            e.url,
            "https://opentopography.s3.sdsc.edu/raster/GEDTM30/GEDTM30_be/gedtm_rf_m_30m_s_20060101_20151231_go_epsg.4326.3855_v1.2.tif"
        );
        assert_eq!((e.graus, e.piramides), (SEGUNDO, 10));
        assert!(citacao(Variavel::Elevacao, 30).contains("OpenTopography"));
        assert!(!citacao(Variavel::Twi, 30).contains("OpenTopography"));
    }

    #[test]
    fn as_escalas_de_cada_variavel() {
        assert_eq!(Variavel::Declividade.escalas(), [30]);
        assert_eq!(Variavel::Twi.escalas(), [30, 480, 960]);
        assert_eq!(Variavel::Geomorfons.escalas(), ESCALAS);
        assert_eq!(Variavel::CurvAnelar.escalas(), [240, 480, 960]);
        assert_eq!(Variavel::AberturaNegativa.escalas(), [480, 960]);
        for v in Variavel::TODAS {
            assert!(!v.escalas().is_empty(), "{v:?}");
            for e in v.escalas() {
                let _ = v.arquivo(e);
            }
        }
        assert_eq!(escala_lida(Variavel::Twi, 480), (480, None));
        let (e, aviso) = escala_lida(Variavel::Twi, 120);
        assert_eq!(e, 480);
        assert_eq!(
            aviso.unwrap(),
            "Índice topográfico de umidade: o RustGIS só lê esta variável do GEDTM30 nas escalas de 30 m, 480 m e 960 m; veio a de 480 m, no lugar da de 120 m"
        );
        assert_eq!(escala_lida(Variavel::Declividade, 960).0, 30);
        assert_eq!(escala_lida(Variavel::AberturaPositiva, 30).0, 480);
    }

    #[test]
    fn os_parametros_e_os_nomes() {
        assert_eq!(Variavel::ler(None), Ok(Variavel::Elevacao));
        assert_eq!(Variavel::ler(Some("twi")), Ok(Variavel::Twi));
        assert_eq!(Variavel::ler(Some("elevacao")), Ok(Variavel::Elevacao));
        assert!(Variavel::ler(Some("incerteza")).is_err());
        assert_eq!(camada(Variavel::Elevacao, 30), "BrasilData: Elevação do terreno 30 m, m (GEDTM30)");
        assert_eq!(ler_escala(None), Ok(30));
        assert_eq!(ler_escala(Some(240.0)), Ok(240));
        assert!(ler_escala(Some(90.0)).is_err());
        for v in Variavel::TODAS {
            for e in ESCALAS {
                assert_eq!(da_camada(&camada(v, e)), Some((v, e)), "{v:?} {e}");
            }
        }
        assert_eq!(camada(Variavel::Declividade, 30), "BrasilData: Declividade 30 m, graus (GEDTM30)");
        assert_eq!(camada(Variavel::Twi, 480), "BrasilData: Índice topográfico de umidade 480 m (GEDTM30)");
        assert_eq!(camada(Variavel::Geomorfons, 30), "BrasilData: Geomorfons 30 m (GEDTM30)");
        assert!(qml_geomorfons().contains(r##"value="9" color="#0000ff" alpha="255" label="Vale""##));
        assert!(chave_geomorfons().starts_with("1 plano, 2 pico, 3 crista"));
    }
}
