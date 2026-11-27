//! As propriedades do solo do SoilGrids 2.0 a 250 m (ISRIC – World Soil Information): textura,
//! carbono, pH, CTC, nitrogênio, densidade, fragmentos grossos e água retida, em seis
//! profundidades até 2 m, com a média, a mediana, os quantis de 5 e 95% e a incerteza, previstas
//! por florestas aleatórias de quantis sobre os perfis do WoSIS.
//!
//! - **De onde:** o WCS do ISRIC (`maps.isric.org`, um MapServer por propriedade; sem login,
//!   licença CC BY 4.0). Os arquivos estão na projeção Homolosine de Goode, interrompida; o WCS
//!   recorta pelo retângulo em graus (`SUBSETTINGCRS` = EPSG:4326) e entrega na projeção original,
//!   que o RustGIS lê e reprojeta. (Pedir a saída já em graus faz o servidor demorar mais de um
//!   minuto e falhar; o VRT de 250 m, de 5,9 MB, o RustGIS não lê por endereço.)
//! - **Numa área grande:** o servidor reduz a imagem (`SCALEFACTOR`) pelo vizinho mais próximo, não
//!   pela média: o plugin pede a menor redução que deixa a imagem em até 4 milhões de células, e
//!   não mais grossa que a grade.
//! - **Valores:** inteiros multiplicados (por 10 ou por 100, como na tabela do ISRIC); o plugin
//!   divide. A imagem do WCS não declara o sem dado, e nas cidades e na água vem 0: os negativos
//!   (o -32768 do mar) e o 0 viram sem dado ([`sem_valor`]), menos nos fragmentos grossos, em que o
//!   0 é um valor de verdade (solo sem cascalho). O `raster.mask` só esconde no desenho: a média e a
//!   reprojeção ainda veriam o 0.
//! - **App web:** o servidor manda o cabeçalho CORS (`Access-Control-Allow-Origin: *`).

/// O WCS do ISRIC.
const BASE: &str = "https://maps.isric.org/mapserv";

/// O lado da célula do SoilGrids, em metros (na Homolosine, que conserva as áreas).
pub const CELULA: f64 = 250.0;

/// Quantas células a imagem lida pode ter: o MapServer entrega uns 4 milhões em 10 a 20 s.
pub const MAX_CELULAS: f64 = 4e6;

/// Um grau de latitude, em metros.
const GRAU_M: f64 = 111_320.0;

/// O menor valor que fica: abaixo da menor unidade do arquivo, acima do 0 das cidades e da água.
const MINIMO: f64 = 0.001;

/// A propriedade do solo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Propriedade {
    Argila,
    Areia,
    Silte,
    Carbono,
    DensidadeCarbono,
    EstoqueCarbono,
    Ph,
    Ctc,
    Nitrogenio,
    DensidadeSolo,
    Cascalho,
    Agua10,
    Agua33,
    Agua1500,
}

impl Propriedade {
    pub const TODAS: [Propriedade; 14] = [
        Propriedade::Argila,
        Propriedade::Areia,
        Propriedade::Silte,
        Propriedade::Carbono,
        Propriedade::DensidadeCarbono,
        Propriedade::EstoqueCarbono,
        Propriedade::Ph,
        Propriedade::Ctc,
        Propriedade::Nitrogenio,
        Propriedade::DensidadeSolo,
        Propriedade::Cascalho,
        Propriedade::Agua10,
        Propriedade::Agua33,
        Propriedade::Agua1500,
    ];

    /// O valor do parâmetro `propriedade`.
    pub fn valor(self) -> &'static str {
        match self {
            Propriedade::Argila => "argila",
            Propriedade::Areia => "areia",
            Propriedade::Silte => "silte",
            Propriedade::Carbono => "carbono",
            Propriedade::DensidadeCarbono => "densidade-carbono",
            Propriedade::EstoqueCarbono => "estoque-carbono",
            Propriedade::Ph => "ph",
            Propriedade::Ctc => "ctc",
            Propriedade::Nitrogenio => "nitrogenio",
            Propriedade::DensidadeSolo => "densidade",
            Propriedade::Cascalho => "cascalho",
            Propriedade::Agua10 => "agua-10kpa",
            Propriedade::Agua33 => "agua-33kpa",
            Propriedade::Agua1500 => "agua-1500kpa",
        }
    }

    /// A propriedade do parâmetro `propriedade` (sem ele, a argila).
    pub fn ler(t: Option<&str>) -> Result<Propriedade, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Propriedade::Argila);
        }
        Propriedade::TODAS.into_iter().find(|p| p.valor() == t).ok_or_else(|| {
            let todas: Vec<&str> = Propriedade::TODAS.iter().map(|p| p.valor()).collect();
            format!("propriedade '{t}': use {}", todas.join(", "))
        })
    }

    /// O código do ISRIC (o nome do mapa e o começo da cobertura).
    pub fn codigo(self) -> &'static str {
        match self {
            Propriedade::Argila => "clay",
            Propriedade::Areia => "sand",
            Propriedade::Silte => "silt",
            Propriedade::Carbono => "soc",
            Propriedade::DensidadeCarbono => "ocd",
            Propriedade::EstoqueCarbono => "ocs",
            Propriedade::Ph => "phh2o",
            Propriedade::Ctc => "cec",
            Propriedade::Nitrogenio => "nitrogen",
            Propriedade::DensidadeSolo => "bdod",
            Propriedade::Cascalho => "cfvo",
            Propriedade::Agua10 => "wv0010",
            Propriedade::Agua33 => "wv0033",
            Propriedade::Agua1500 => "wv1500",
        }
    }

    /// Por quanto o arquivo multiplica o valor na unidade (a tabela de conversão do ISRIC).
    pub fn divisor(self) -> f64 {
        match self {
            Propriedade::EstoqueCarbono => 1.0,
            Propriedade::Nitrogenio | Propriedade::DensidadeSolo => 100.0,
            _ => 10.0,
        }
    }

    /// O nome, como entra no nome da camada.
    pub fn nome(self) -> &'static str {
        match self {
            Propriedade::Argila => "Argila",
            Propriedade::Areia => "Areia",
            Propriedade::Silte => "Silte",
            Propriedade::Carbono => "Carbono orgânico do solo",
            Propriedade::DensidadeCarbono => "Densidade de carbono do solo",
            Propriedade::EstoqueCarbono => "Estoque de carbono do solo",
            Propriedade::Ph => "pH do solo em água",
            Propriedade::Ctc => "CTC do solo a pH 7",
            Propriedade::Nitrogenio => "Nitrogênio total do solo",
            Propriedade::DensidadeSolo => "Densidade do solo",
            Propriedade::Cascalho => "Fragmentos grossos do solo",
            Propriedade::Agua10 => "Água retida a 10 kPa",
            Propriedade::Agua33 => "Água retida a 33 kPa",
            Propriedade::Agua1500 => "Água retida a 1500 kPa",
        }
    }

    /// A unidade.
    pub fn unidade(self) -> &'static str {
        match self {
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => "%",
            Propriedade::Carbono | Propriedade::Nitrogenio => "g/kg",
            Propriedade::DensidadeCarbono => "kg/m³",
            Propriedade::EstoqueCarbono => "t/ha",
            Propriedade::Ph => "pH",
            Propriedade::Ctc => "cmolc/kg",
            Propriedade::DensidadeSolo => "g/cm³",
            Propriedade::Cascalho | Propriedade::Agua10 | Propriedade::Agua33 | Propriedade::Agua1500 => "% do volume",
        }
    }

    /// Se vem da coleção da água retida (Turek e outros, 2023), e não do SoilGrids de 2021.
    pub fn da_agua(self) -> bool {
        matches!(self, Propriedade::Agua10 | Propriedade::Agua33 | Propriedade::Agua1500)
    }

    /// Os valores possíveis depois da divisão, para tirar o que não for solo: o 0 das cidades e da
    /// água fica de fora, menos nos fragmentos grossos.
    pub fn faixa(self) -> (f64, f64) {
        match self {
            Propriedade::Ph => (MINIMO, 14.0),
            Propriedade::DensidadeSolo => (MINIMO, 3.0),
            Propriedade::Cascalho => (0.0, 100.0),
            Propriedade::Argila
            | Propriedade::Areia
            | Propriedade::Silte
            | Propriedade::Agua10
            | Propriedade::Agua33
            | Propriedade::Agua1500 => (MINIMO, 100.0),
            _ => (MINIMO, 1000.0),
        }
    }

    /// A escala das cores.
    pub fn escala(self) -> (f64, f64) {
        match self {
            Propriedade::Argila | Propriedade::Areia | Propriedade::Silte => (0.0, 80.0),
            Propriedade::Carbono => (0.0, 60.0),
            Propriedade::DensidadeCarbono => (0.0, 60.0),
            Propriedade::EstoqueCarbono => (0.0, 120.0),
            Propriedade::Ph => (4.0, 8.5),
            Propriedade::Ctc => (0.0, 30.0),
            Propriedade::Nitrogenio => (0.0, 4.0),
            Propriedade::DensidadeSolo => (0.8, 1.7),
            Propriedade::Cascalho => (0.0, 40.0),
            Propriedade::Agua10 => (20.0, 50.0),
            Propriedade::Agua33 => (15.0, 45.0),
            Propriedade::Agua1500 => (5.0, 35.0),
        }
    }

    /// A rampa de cores (valor e cor, no formato do GDAL que o `ramp.add` lê), nos valores da
    /// [`escala`](Self::escala).
    pub fn rampa(self) -> (&'static str, &'static str) {
        match self {
            Propriedade::Argila => ("BrasilData: argila", "0 255 247 236\n20 253 208 162\n40 241 105 19\n60 166 54 3\n80 90 30 5"),
            Propriedade::Areia => ("BrasilData: areia", "0 255 255 229\n20 254 236 160\n40 254 196 79\n60 236 112 20\n80 140 45 4"),
            Propriedade::Silte => ("BrasilData: silte", "0 247 252 245\n20 199 233 192\n40 116 196 118\n60 35 139 69\n80 0 68 27"),
            Propriedade::Carbono | Propriedade::DensidadeCarbono => {
                ("BrasilData: carbono do solo", "0 250 240 215\n10 222 196 140\n20 180 140 90\n35 120 85 50\n60 55 35 20")
            }
            Propriedade::EstoqueCarbono => {
                ("BrasilData: estoque de carbono (SoilGrids)", "0 250 240 215\n20 222 196 140\n40 180 140 90\n70 120 85 50\n120 55 35 20")
            }
            Propriedade::Ph => {
                ("BrasilData: pH do solo", "4 215 48 39\n5 252 141 89\n5.8 254 224 139\n6.5 217 239 139\n7.3 145 207 96\n8.5 26 152 80")
            }
            Propriedade::Ctc => {
                ("BrasilData: CTC do solo (SoilGrids)", "0 252 251 253\n5 218 218 235\n10 158 154 200\n20 106 81 163\n30 63 0 125")
            }
            Propriedade::Nitrogenio => {
                ("BrasilData: nitrogênio do solo (SoilGrids)", "0 255 255 229\n1 217 240 163\n2 120 198 121\n3 35 132 67\n4 0 69 41")
            }
            Propriedade::DensidadeSolo => {
                ("BrasilData: densidade do solo", "0.8 255 247 236\n1.1 253 212 158\n1.4 239 101 72\n1.7 127 0 0")
            }
            Propriedade::Cascalho => {
                ("BrasilData: fragmentos grossos (SoilGrids)", "0 255 255 255\n10 217 217 217\n20 150 150 150\n40 60 60 60")
            }
            Propriedade::Agua10 => (
                "BrasilData: água a 10 kPa (SoilGrids)",
                "20 255 255 217\n26 199 233 180\n32 127 205 187\n38 29 145 192\n44 34 94 168\n50 8 29 88",
            ),
            Propriedade::Agua33 => (
                "BrasilData: água a 33 kPa (SoilGrids)",
                "15 255 255 217\n21 199 233 180\n27 127 205 187\n33 29 145 192\n39 34 94 168\n45 8 29 88",
            ),
            Propriedade::Agua1500 => (
                "BrasilData: água a 1500 kPa (SoilGrids)",
                "5 255 255 217\n11 199 233 180\n17 127 205 187\n23 29 145 192\n29 34 94 168\n35 8 29 88",
            ),
        }
    }
}

/// A estatística da previsão.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Estatistica {
    Media,
    Mediana,
    Q05,
    Q95,
    Incerteza,
}

impl Estatistica {
    pub const TODAS: [Estatistica; 5] =
        [Estatistica::Media, Estatistica::Mediana, Estatistica::Q05, Estatistica::Q95, Estatistica::Incerteza];

    /// O valor do parâmetro `estatistica`.
    pub fn valor(self) -> &'static str {
        match self {
            Estatistica::Media => "media",
            Estatistica::Mediana => "mediana",
            Estatistica::Q05 => "q05",
            Estatistica::Q95 => "q95",
            Estatistica::Incerteza => "incerteza",
        }
    }

    /// A estatística do parâmetro `estatistica` (sem ele, a média).
    pub fn ler(t: Option<&str>) -> Result<Estatistica, String> {
        let t = t.map(str::trim).unwrap_or("");
        if t.is_empty() {
            return Ok(Estatistica::Media);
        }
        Estatistica::TODAS
            .into_iter()
            .find(|e| e.valor() == t)
            .ok_or_else(|| format!("estatística '{t}': use media, mediana, q05, q95 ou incerteza"))
    }

    /// O fim do nome da cobertura do ISRIC.
    fn codigo(self) -> &'static str {
        match self {
            Estatistica::Media => "mean",
            Estatistica::Mediana => "Q0.5",
            Estatistica::Q05 => "Q0.05",
            Estatistica::Q95 => "Q0.95",
            Estatistica::Incerteza => "uncertainty",
        }
    }

    /// O nome, no nome da camada (a média não entra).
    pub fn nome(self) -> &'static str {
        match self {
            Estatistica::Media => "média",
            Estatistica::Mediana => "mediana",
            Estatistica::Q05 => "quantil de 5%",
            Estatistica::Q95 => "quantil de 95%",
            Estatistica::Incerteza => "incerteza",
        }
    }
}

/// A incerteza: a largura do intervalo de 90% dividida pela mediana, multiplicada por 10 no arquivo.
const DIVISOR_INCERTEZA: f64 = 10.0;

/// A escala e a rampa da incerteza.
pub const ESCALA_INCERTEZA: (f64, f64) = (0.0, 5.0);
pub const RAMPA_INCERTEZA: (&str, &str) =
    ("BrasilData: incerteza (SoilGrids)", "0 255 255 204\n1 161 218 180\n2 65 182 196\n3.5 34 94 168\n5 37 52 148");

/// A maior incerteza que fica: acima disso a mediana é quase 0 e a razão não diz nada.
const MAXIMO_INCERTEZA: f64 = 100.0;

/// Como ler uma camada: o divisor do arquivo, a unidade, os valores que ficam, a escala e a rampa
/// das cores.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Leitura {
    pub divisor: f64,
    pub unidade: &'static str,
    pub faixa: (f64, f64),
    pub escala: (f64, f64),
    pub rampa: (&'static str, &'static str),
}

/// A leitura da camada: a da propriedade, ou a da incerteza.
pub fn leitura(p: Propriedade, e: Estatistica) -> Leitura {
    if e == Estatistica::Incerteza {
        Leitura {
            divisor: DIVISOR_INCERTEZA,
            unidade: "",
            faixa: (MINIMO, MAXIMO_INCERTEZA),
            escala: ESCALA_INCERTEZA,
            rampa: RAMPA_INCERTEZA,
        }
    } else {
        Leitura { divisor: p.divisor(), unidade: p.unidade(), faixa: p.faixa(), escala: p.escala(), rampa: p.rampa() }
    }
}

/// Os intervalos do arquivo (`[de, até, 0]`, o fim de fora) que viram 0 antes da divisão, para o
/// `b1 / (b1 / b1)` os tornar sem dado (0 / 0): os negativos e o 0, e na incerteza os valores em que
/// a mediana é quase 0. Nos fragmentos grossos, nenhum (o 0 é um valor).
pub fn sem_valor(p: Propriedade, e: Estatistica) -> Option<Vec<[f64; 3]>> {
    match e {
        Estatistica::Incerteza => Some(vec![[-32768.0, 1.0, 0.0], [MAXIMO_INCERTEZA * DIVISOR_INCERTEZA, 32768.0, 0.0]]),
        _ if p == Propriedade::Cascalho => None,
        _ => Some(vec![[-32768.0, 1.0, 0.0]]),
    }
}

/// As profundidades, em cm.
pub const PROFUNDIDADES: [(u32, u32); 6] = [(0, 5), (5, 15), (15, 30), (30, 60), (60, 100), (100, 200)];

/// A única profundidade do estoque de carbono.
pub const PROFUNDIDADE_DO_ESTOQUE: (u32, u32) = (0, 30);

/// A profundidade do parâmetro `profundidade` ("0-5", "5-15"...; sem ele, 0–5 cm). O estoque de
/// carbono só tem 0–30 cm: vem essa, e o `true` diz que a pedida não valeu.
pub fn ler_profundidade(t: Option<&str>, p: Propriedade) -> Result<((u32, u32), bool), String> {
    let t = t.map(str::trim).unwrap_or("");
    let pedida = if t.is_empty() {
        None
    } else {
        Some(
            PROFUNDIDADES
                .into_iter()
                .chain([PROFUNDIDADE_DO_ESTOQUE])
                .find(|(a, b)| format!("{a}-{b}") == t)
                .ok_or_else(|| format!("profundidade '{t}': use 0-5, 5-15, 15-30, 30-60, 60-100 ou 100-200 (cm)"))?,
        )
    };
    if p == Propriedade::EstoqueCarbono {
        return Ok((PROFUNDIDADE_DO_ESTOQUE, pedida.is_some_and(|d| d != PROFUNDIDADE_DO_ESTOQUE)));
    }
    match pedida {
        None => Ok((PROFUNDIDADES[0], false)),
        Some(d) if d == PROFUNDIDADE_DO_ESTOQUE => Err("a camada de 0 a 30 cm é só do estoque de carbono: use 0-5, 5-15 ou 15-30".into()),
        Some(d) => Ok((d, false)),
    }
}

/// Quantas vezes a imagem é reduzida na área `c`: a menor redução que a deixa em até
/// [`MAX_CELULAS`], e não mais grossa que a grade (de `celula` metros).
pub fn reducao(c: [f64; 4], celula: f64) -> u32 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let colunas = (c[2] - c[0]) * GRAU_M * lat.cos() / CELULA;
    let linhas = (c[3] - c[1]) * GRAU_M / CELULA;
    let mut k = ((celula / CELULA + 1e-9).floor() as u32).max(1);
    while colunas * linhas / f64::from(k * k) > MAX_CELULAS {
        k += 1;
    }
    k
}

/// A janela pedida: o retângulo da área com duas células lidas de margem, para a reprojeção não
/// perder a borda.
pub fn janela(c: [f64; 4], k: u32) -> [f64; 4] {
    let m = 2.0 * CELULA * f64::from(k) / GRAU_M;
    let lat = ((c[1] + c[3]) / 2.0).to_radians().cos().max(0.1);
    [c[0] - m / lat, c[1] - m, c[2] + m / lat, c[3] + m]
}

/// O endereço do pedido ao WCS: a cobertura, o retângulo `j` em graus e a redução `k`.
pub fn url(p: Propriedade, prof: (u32, u32), e: Estatistica, j: [f64; 4], k: u32) -> String {
    let escala = if k > 1 { format!("&SCALEFACTOR={:.6}", 1.0 / f64::from(k)) } else { String::new() };
    format!(
        "{BASE}?map=/map/{cod}.map&SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID={cod}_{a}-{b}cm_{est}&FORMAT=image/tiff&SUBSET=long({:.4},{:.4})&SUBSET=lat({:.4},{:.4})&SUBSETTINGCRS=http://www.opengis.net/def/crs/EPSG/0/4326{escala}",
        j[0],
        j[2],
        j[1],
        j[3],
        cod = p.codigo(),
        a = prof.0,
        b = prof.1,
        est = e.codigo(),
    )
}

/// O nome da camada.
pub fn camada(p: Propriedade, prof: (u32, u32), e: Estatistica) -> String {
    let est = if e == Estatistica::Media { String::new() } else { format!(", {}", e.nome()) };
    format!("BrasilData: {} {}–{} cm{est} (SoilGrids)", p.nome(), prof.0, prof.1)
}

/// A camada de passagem com a imagem lida na redução `k`.
pub fn janela_nome(p: Propriedade, prof: (u32, u32), e: Estatistica, k: u32) -> String {
    format!("BrasilData: SoilGrids {} {}-{} cm {} redução {k} (janela)", p.codigo(), prof.0, prof.1, e.codigo())
}

/// O que uma camada é: a propriedade, a profundidade e a estatística.
pub type Camada = (Propriedade, (u32, u32), Estatistica);

/// O que é a camada `nome`, se for do SoilGrids.
pub fn da_camada(nome: &str) -> Option<Camada> {
    if !nome.ends_with("(SoilGrids)") {
        return None;
    }
    Propriedade::TODAS
        .into_iter()
        .flat_map(|p| {
            PROFUNDIDADES
                .into_iter()
                .chain([PROFUNDIDADE_DO_ESTOQUE])
                .flat_map(move |d| Estatistica::TODAS.into_iter().map(move |e| (p, d, e)))
        })
        .find(|(p, d, e)| camada(*p, *d, *e) == nome)
}

/// A citação, como a licença pede.
pub fn citacao(p: Propriedade, prof: (u32, u32), e: Estatistica) -> String {
    let artigo = if p.da_agua() {
        "Turek, M. E. et al., Global mapping of volumetric water retention at 100, 330 and 15 000 cm suction using the WoSIS database, International Soil and Water Conservation Research 11, 225–239 (2023), doi:10.1016/j.iswcr.2022.08.001"
    } else {
        "Poggio, L. et al., SoilGrids 2.0: producing soil information for the globe with quantified spatial uncertainty, SOIL 7, 217–240 (2021), doi:10.5194/soil-7-217-2021"
    };
    let unidade = if e == Estatistica::Incerteza {
        "razão da largura do intervalo de 90% pela mediana".to_string()
    } else {
        p.unidade().to_string()
    };
    format!(
        "{artigo} (ISRIC – World Soil Information, SoilGrids 2.0). {} ({unidade}) de {}–{} cm, {}, a 250 m; lido do WCS do ISRIC (maps.isric.org), posto na grade do projeto e recortado pela área",
        p.nome(),
        prof.0,
        prof.1,
        e.nome()
    )
}

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endereco_e_nomes() {
        let j = [-43.0, -20.9, -42.7, -20.6];
        assert_eq!(
            url(Propriedade::Ctc, (0, 5), Estatistica::Media, j, 1),
            "https://maps.isric.org/mapserv?map=/map/cec.map&SERVICE=WCS&VERSION=2.0.1&REQUEST=GetCoverage&COVERAGEID=cec_0-5cm_mean&FORMAT=image/tiff&SUBSET=long(-43.0000,-42.7000)&SUBSET=lat(-20.9000,-20.6000)&SUBSETTINGCRS=http://www.opengis.net/def/crs/EPSG/0/4326"
        );
        assert!(url(Propriedade::Agua33, (100, 200), Estatistica::Q95, j, 4).contains("COVERAGEID=wv0033_100-200cm_Q0.95&"));
        assert!(url(Propriedade::Agua33, (100, 200), Estatistica::Q95, j, 4).ends_with("&SCALEFACTOR=0.250000"));
        assert_eq!(camada(Propriedade::Argila, (0, 5), Estatistica::Media), "BrasilData: Argila 0–5 cm (SoilGrids)");
        assert_eq!(
            camada(Propriedade::Ctc, (30, 60), Estatistica::Incerteza),
            "BrasilData: CTC do solo a pH 7 30–60 cm, incerteza (SoilGrids)"
        );
        for p in Propriedade::TODAS {
            for e in Estatistica::TODAS {
                let d = if p == Propriedade::EstoqueCarbono { PROFUNDIDADE_DO_ESTOQUE } else { (60, 100) };
                assert_eq!(da_camada(&camada(p, d, e)), Some((p, d, e)));
            }
            assert_eq!(Propriedade::ler(Some(p.valor())), Ok(p));
        }
        assert_eq!(da_camada("BrasilData: Argila 0–30 cm, 2020–2022 (OpenLandMap)"), None);
    }

    #[test]
    fn profundidades() {
        assert_eq!(ler_profundidade(None, Propriedade::Argila), Ok(((0, 5), false)));
        assert_eq!(ler_profundidade(Some("100-200"), Propriedade::Ph), Ok(((100, 200), false)));
        assert_eq!(ler_profundidade(Some("60-100"), Propriedade::EstoqueCarbono), Ok(((0, 30), true)));
        assert_eq!(ler_profundidade(None, Propriedade::EstoqueCarbono), Ok(((0, 30), false)));
        assert!(ler_profundidade(Some("0-30"), Propriedade::Argila).is_err());
        assert!(ler_profundidade(Some("0-20"), Propriedade::Argila).is_err());
    }

    #[test]
    fn reducao_pela_area_e_pela_grade() {
        // Viçosa, numa grade de 30 m: a imagem inteira.
        assert_eq!(reducao([-43.0, -20.9, -42.7, -20.6], 30.0), 1);
        // Numa grade de 1 km: a cada 4 células.
        assert_eq!(reducao([-43.0, -20.9, -42.7, -20.6], 1000.0), 4);
        // Minas Gerais: uns 18 milhões de células, reduzidas 3 vezes.
        assert_eq!(reducao([-51.05, -22.93, -39.85, -14.23], 30.0), 3);
        // O Brasil inteiro.
        let k = reducao([-74.0, -34.0, -34.7, 5.3], 30.0);
        assert!((8..=10).contains(&k), "{k}");
    }

    #[test]
    fn a_janela_tem_margem() {
        let j = janela([-43.0, -20.9, -42.7, -20.6], 1);
        assert!(j[0] < -43.0 && j[1] < -20.9 && j[2] > -42.7 && j[3] > -20.6);
        assert!((j[3] - -20.6 - 500.0 / GRAU_M).abs() < 1e-9);
    }

    #[test]
    fn a_rampa_segue_a_escala() {
        for p in Propriedade::TODAS {
            let (a, b) = p.escala();
            let v: Vec<f64> = p.rampa().1.lines().map(|l| l.split(' ').next().unwrap().parse().unwrap()).collect();
            assert_eq!((v[0], v[v.len() - 1]), (a, b), "{p:?}");
        }
    }

    #[test]
    fn leitura_da_incerteza() {
        assert_eq!(leitura(Propriedade::Nitrogenio, Estatistica::Media).divisor, 100.0);
        assert_eq!(leitura(Propriedade::Nitrogenio, Estatistica::Incerteza).divisor, 10.0);
        assert_eq!(leitura(Propriedade::EstoqueCarbono, Estatistica::Q05).unidade, "t/ha");
        // O 0 das cidades sai, menos no cascalho.
        assert_eq!(sem_valor(Propriedade::Cascalho, Estatistica::Media), None);
        assert_eq!(sem_valor(Propriedade::Argila, Estatistica::Q95), Some(vec![[-32768.0, 1.0, 0.0]]));
        assert_eq!(sem_valor(Propriedade::Cascalho, Estatistica::Incerteza).map(|v| v.len()), Some(2));
        assert!(Propriedade::Ph.faixa().0 > 0.0 && Propriedade::Argila.faixa().0 > 0.0);
        assert_eq!(Propriedade::Cascalho.faixa().0, 0.0);
    }
}
