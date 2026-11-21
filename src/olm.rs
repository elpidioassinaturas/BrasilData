//! As propriedades do solo do OpenLandMap-soildb a 30 m (OpenGeoHub, Tomislav Hengl e outros):
//! carbono orgânico, densidade de carbono, pH em água, densidade do solo e as frações da textura
//! (argila, areia e silte), em três profundidades, por aprendizado de máquina espaço-temporal
//! sobre cerca de 400 mil amostras de solo.
//!
//! - **De onde:** COGs globais no servidor `ceph.opengeohub.org` (o catálogo STAC
//!   `stac.opengeohub.org/v1/cat/openlandmap`, sem login, licença CC BY 4.0), um por propriedade,
//!   profundidade e período: WGS 84, células de 0,00025° (cerca de 30 m), inteiros com o valor
//!   multiplicado (por 10 ou por 100; o RustGIS não aplica a escala do arquivo, o plugin divide), e
//!   dez níveis de pirâmide, cada um com o dobro da célula.
//! - **Períodos:** o carbono e o pH vêm de 5 em 5 anos (2000–2005 a 2020–2022); a densidade do
//!   solo e a textura, só 2020–2022.
//! - **Como:** só a janela da área (`layer.addUrl` com `bbox`), do nível de pirâmide mais grosso
//!   cujas células não passam das da grade; o valor passa à unidade, vai para a grade do projeto e
//!   é recortado pela área.
//! - **App web:** o servidor manda o cabeçalho CORS (`Access-Control-Allow-Origin: *`): funciona
//!   também no navegador.

/// O lado da célula do arquivo inteiro, em graus.
pub const CELULA_GRAUS: f64 = 0.00025;

/// Quantos níveis de pirâmide o arquivo tem além do inteiro.
pub const PIRAMIDES: i32 = 10;

const BASE: &str = "https://ceph.opengeohub.org/arco/soildb";

/// A propriedade do solo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Propriedade {
    Carbono,
    DensidadeCarbono,
    Ph,
    DensidadeSolo,
    Argila,
    Areia,
    Silte,
}

impl Propriedade {
    pub const TODAS: [Propriedade; 7] = [
        Propriedade::Carbono,
        Propriedade::DensidadeCarbono,
        Propriedade::Ph,
        Propriedade::DensidadeSolo,
        Propriedade::Argila,
        Propriedade::Areia,
        Propriedade::Silte,
    ];

    /// O valor do parâmetro `propriedade`.
    pub fn valor(self) -> &'static str {
        match self {
            Propriedade::Carbono => "carbono",
            Propriedade::DensidadeCarbono => "densidade-carbono",
            Propriedade::Ph => "ph",
            Propriedade::DensidadeSolo => "densidade",
            Propriedade::Argila => "argila",
            Propriedade::Areia => "areia",
            Propriedade::Silte => "silte",
        }
    }

    /// A propriedade do parâmetro `propriedade` (sem ele, o carbono).
    pub fn ler(t: Option<&str>) -> Result<Propriedade, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Propriedade::Carbono);
        }
        Propriedade::TODAS.into_iter().find(|p| p.valor() == t).ok_or_else(|| {
            let todas: Vec<&str> = Propriedade::TODAS.iter().map(|p| p.valor()).collect();
            format!("propriedade '{t}': use {}", todas.join(", "))
        })
    }

    /// O começo do nome do arquivo e a versão.
    fn arquivo(self) -> (&'static str, &'static str) {
        match self {
            Propriedade::Carbono => ("soildbpc_ccwpml", "v1.2.0"),
            Propriedade::DensidadeCarbono => ("soildbpc_cdmgcm3", "v1.2.0"),
            Propriedade::Ph => ("soildbpc_phh2o", "v1.2.0"),
            Propriedade::DensidadeSolo => ("soildbpc_bdcore", "v1.0.0"),
            Propriedade::Argila => ("soildbtxt_clay", "v1.0.0"),
            Propriedade::Areia => ("soildbtxt_sand", "v1.0.0"),
            Propriedade::Silte => ("soildbtxt_silt", "v1.0.0"),
        }
    }

    /// Por quanto o arquivo multiplica o valor.
    pub fn divisor(self) -> f64 {
        match self {
            Propriedade::Carbono | Propriedade::DensidadeCarbono | Propriedade::Ph => 10.0,
            Propriedade::DensidadeSolo => 100.0,
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => 1.0,
        }
    }

    /// Se vem de 5 em 5 anos (senão, só 2020–2022).
    pub fn no_tempo(self) -> bool {
        matches!(self, Propriedade::Carbono | Propriedade::DensidadeCarbono | Propriedade::Ph)
    }

    /// O nome, como entra no nome da camada.
    pub fn nome(self) -> &'static str {
        match self {
            Propriedade::Carbono => "Carbono orgânico do solo",
            Propriedade::DensidadeCarbono => "Densidade de carbono do solo",
            Propriedade::Ph => "pH do solo em água",
            Propriedade::DensidadeSolo => "Densidade do solo",
            Propriedade::Argila => "Argila",
            Propriedade::Areia => "Areia",
            Propriedade::Silte => "Silte",
        }
    }

    /// A unidade.
    pub fn unidade(self) -> &'static str {
        match self {
            Propriedade::Carbono => "g/kg",
            Propriedade::DensidadeCarbono => "kg/m³",
            Propriedade::Ph => "pH",
            Propriedade::DensidadeSolo => "g/cm³",
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => "%",
        }
    }

    /// Os valores possíveis depois da divisão, para tirar o que não for solo.
    pub fn faixa(self) -> (f64, f64) {
        match self {
            Propriedade::Carbono => (0.0, 1000.0),
            Propriedade::DensidadeCarbono => (0.0, 1000.0),
            Propriedade::Ph => (0.0, 14.0),
            Propriedade::DensidadeSolo => (0.0, 3.0),
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => (0.0, 100.0),
        }
    }

    /// A escala das cores.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Propriedade::Carbono => (0.0, 60.0),
            Propriedade::DensidadeCarbono => (0.0, 40.0),
            Propriedade::Ph => (4.0, 8.5),
            Propriedade::DensidadeSolo => (0.8, 1.7),
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => (0.0, 80.0),
        }
    }

    /// A rampa de cores (valor e cor, no formato do GDAL que o `ramp.add` lê).
    pub fn rampa(self) -> (&'static str, &'static str) {
        match self {
            Propriedade::Carbono | Propriedade::DensidadeCarbono => {
                ("BrasilData: carbono do solo", "0 250 240 215\n10 222 196 140\n20 180 140 90\n35 120 85 50\n60 55 35 20")
            }
            Propriedade::Ph => {
                ("BrasilData: pH do solo", "4 215 48 39\n5 252 141 89\n5.8 254 224 139\n6.5 217 239 139\n7.3 145 207 96\n8.5 26 152 80")
            }
            Propriedade::DensidadeSolo => {
                ("BrasilData: densidade do solo", "0.8 255 247 236\n1.1 253 212 158\n1.4 239 101 72\n1.7 127 0 0")
            }
            Propriedade::Argila => ("BrasilData: argila", "0 255 247 236\n20 253 208 162\n40 241 105 19\n60 166 54 3\n80 90 30 5"),
            Propriedade::Areia => ("BrasilData: areia", "0 255 255 229\n20 254 236 160\n40 254 196 79\n60 236 112 20\n80 140 45 4"),
            Propriedade::Silte => ("BrasilData: silte", "0 247 252 245\n20 199 233 192\n40 116 196 118\n60 35 139 69\n80 0 68 27"),
        }
    }
}

/// As profundidades, em cm.
pub const PROFUNDIDADES: [(u32, u32); 3] = [(0, 30), (30, 60), (60, 100)];

/// A profundidade do parâmetro `profundidade` ("0-30", "30-60" ou "60-100"; sem ele, 0–30 cm).
pub fn ler_profundidade(t: Option<&str>) -> Result<(u32, u32), String> {
    let t = t.map(str::trim).unwrap_or("");
    if t.is_empty() {
        return Ok(PROFUNDIDADES[0]);
    }
    PROFUNDIDADES
        .into_iter()
        .find(|(a, b)| format!("{a}-{b}") == t)
        .ok_or_else(|| format!("profundidade '{t}': use 0-30, 30-60 ou 60-100 (cm)"))
}

/// Os períodos: o primeiro e o último ano.
pub const PERIODOS: [(u32, u32); 5] = [(2000, 2005), (2005, 2010), (2010, 2015), (2015, 2020), (2020, 2022)];

/// O período do parâmetro `periodo` ("2020-2022" etc.; sem ele, o último), conferido com a
/// propriedade.
pub fn ler_periodo(t: Option<&str>, p: Propriedade) -> Result<(u32, u32), String> {
    let t = t.map(str::trim).unwrap_or("");
    let ultimo = PERIODOS[PERIODOS.len() - 1];
    if t.is_empty() {
        return Ok(ultimo);
    }
    let per = PERIODOS
        .into_iter()
        .find(|(a, b)| format!("{a}-{b}") == t)
        .ok_or_else(|| format!("período '{t}': use 2000-2005, 2005-2010, 2010-2015, 2015-2020 ou 2020-2022"))?;
    if per != ultimo && !p.no_tempo() {
        return Err(format!("{} só tem o período 2020-2022: os de 5 em 5 anos são do carbono e do pH", p.nome().to_lowercase()));
    }
    Ok(per)
}

/// O endereço do arquivo.
pub fn url(p: Propriedade, prof: (u32, u32), per: (u32, u32)) -> String {
    let (raiz, versao) = p.arquivo();
    format!("{BASE}/{raiz}_m_30m_bg{}..{}cm_{}0101_{}1231_go01_epsg4326_{versao}_srl04.tif", prof.0, prof.1, per.0, per.1)
}

/// O nome da camada.
pub fn camada(p: Propriedade, prof: (u32, u32), per: (u32, u32)) -> String {
    format!("BrasilData: {} {}–{} cm, {}–{} (OpenLandMap)", p.nome(), prof.0, prof.1, per.0, per.1)
}

/// A camada de passagem com a janela lida no nível `k`.
pub fn janela_nome(p: Propriedade, prof: (u32, u32), per: (u32, u32), k: i32) -> String {
    format!("BrasilData: OpenLandMap {} {}-{} cm {}-{} nível {k} (janela)", p.valor(), prof.0, prof.1, per.0, per.1)
}

/// O que uma camada é: a propriedade, a profundidade e o período.
pub type Camada = (Propriedade, (u32, u32), (u32, u32));

/// O que é a camada `nome`, se for do OpenLandMap-soildb.
pub fn da_camada(nome: &str) -> Option<Camada> {
    Propriedade::TODAS
        .into_iter()
        .flat_map(|p| PROFUNDIDADES.into_iter().flat_map(move |d| PERIODOS.into_iter().map(move |t| (p, d, t))))
        .find(|(p, d, t)| camada(*p, *d, *t) == nome)
}

/// O arquivo da propriedade, da profundidade e do período, com a célula e as pirâmides.
pub fn arquivo(p: Propriedade, prof: (u32, u32), per: (u32, u32)) -> crate::cog_global::Arquivo {
    crate::cog_global::Arquivo { url: url(p, prof, per), graus: CELULA_GRAUS, piramides: PIRAMIDES }
}

/// A citação, como a licença pede.
pub fn citacao(p: Propriedade, prof: (u32, u32), per: (u32, u32)) -> String {
    format!(
        "Hengl, T. et al., OpenLandMap-soildb: global soil information at 30 m spatial resolution for 2000–2022+ based on spatiotemporal Machine Learning and harmonized legacy soil samples and observations, Earth System Science Data 18, 989–1036 (2026); dados: Consoli, D. et al., doi:10.5281/zenodo.15470431 (OpenGeoHub). {} ({}) de {}–{} cm, {}–{}, a 30 m; lido do servidor ceph.opengeohub.org (catálogo STAC do OpenLandMap), posto na grade do projeto e recortado pela área",
        p.nome(),
        p.unidade(),
        prof.0,
        prof.1,
        per.0,
        per.1
    )
}

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enderecos_e_nomes() {
        assert_eq!(
            url(Propriedade::Carbono, (0, 30), (2020, 2022)),
            "https://ceph.opengeohub.org/arco/soildb/soildbpc_ccwpml_m_30m_bg0..30cm_20200101_20221231_go01_epsg4326_v1.2.0_srl04.tif"
        );
        assert_eq!(
            url(Propriedade::Argila, (60, 100), (2020, 2022)),
            "https://ceph.opengeohub.org/arco/soildb/soildbtxt_clay_m_30m_bg60..100cm_20200101_20221231_go01_epsg4326_v1.0.0_srl04.tif"
        );
        assert!(
            url(Propriedade::Ph, (30, 60), (2000, 2005))
                .ends_with("soildbpc_phh2o_m_30m_bg30..60cm_20000101_20051231_go01_epsg4326_v1.2.0_srl04.tif")
        );
        assert!(url(Propriedade::DensidadeSolo, (0, 30), (2020, 2022)).contains("soildbpc_bdcore_m_30m_bg0..30cm_"));
        let n = camada(Propriedade::Ph, (0, 30), (2010, 2015));
        assert_eq!(n, "BrasilData: pH do solo em água 0–30 cm, 2010–2015 (OpenLandMap)");
        assert_eq!(da_camada(&n), Some((Propriedade::Ph, (0, 30), (2010, 2015))));
        assert_eq!(da_camada("BrasilData: Solos (BDiA 1:250.000)"), None);
    }

    #[test]
    fn os_parametros() {
        assert_eq!(Propriedade::ler(None), Ok(Propriedade::Carbono));
        assert_eq!(Propriedade::ler(Some("argila")), Ok(Propriedade::Argila));
        assert!(Propriedade::ler(Some("ferro")).is_err());
        assert_eq!(ler_profundidade(Some("30-60")), Ok((30, 60)));
        assert!(ler_profundidade(Some("0-20")).is_err());
        assert_eq!(ler_periodo(None, Propriedade::Argila), Ok((2020, 2022)));
        assert_eq!(ler_periodo(Some("2000-2005"), Propriedade::Carbono), Ok((2000, 2005)));
        assert!(ler_periodo(Some("2000-2005"), Propriedade::Argila).is_err());
        assert!(ler_periodo(Some("1990-1995"), Propriedade::Ph).is_err());
    }

    #[test]
    fn o_arquivo() {
        let a = arquivo(Propriedade::Silte, (0, 30), (2020, 2022));
        assert_eq!((a.graus, a.piramides), (0.00025, 10));
        assert!(a.url.contains("soildbtxt_silt_m_30m_bg0..30cm_"));
    }
}
