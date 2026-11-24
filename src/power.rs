//! A umidade do solo na zona das raízes e a evapotranspiração do NASA POWER, da reanálise MERRA-2
//! (NASA GMAO): a média de 2001 a 2025, um mês ou um ano desde 1981 e a anomalia dele.
//!
//! - **De onde:** a API regional do POWER, sem login e com CORS, como a radiação solar e o vento
//!   ([`crate::sol`], [`crate::vento`]): a climatologia e, para um ano, a mensal, que traz os 12 meses
//!   e o ano de uma vez. Os pontos vêm na grade do MERRA-2, de 0,625° por 0,5°, só em terra.
//! - **Até quando:** o POWER usa o MERRA-2 até setembro de 2026 e, daí em diante, o GEOS-IT, outro
//!   modelo: na emenda a umidade do solo muda de patamar, e a anomalia dela não diria nada. Por isso
//!   os meses seguintes são recusados ([`conferir`]).
//! - **Como:** cada ponto vira a célula em volta dele, as células viram um raster, que vai para a
//!   grade do projeto por interpolação bilinear e é recortado pela área. A anomalia é a diferença, em
//!   cada ponto, entre o mês (ou o ano) e a média dele em 2001 a 2025.

use crate::risco_fogo::NOMES_DOS_MESES;
use crate::sol;
use crate::vento::LADO;

/// As camadas de passagem.
pub const CELULAS: &str = "BrasilData: NASA POWER em células do MERRA-2";
pub const RASTER: &str = "BrasilData: NASA POWER na grade do MERRA-2";
pub const NA_GRADE: &str = "BrasilData: NASA POWER na grade";

/// O campo das células.
pub const CAMPO: &str = "valor";

/// O sem dado do raster (a anomalia tem valores negativos).
pub const SEM_DADO: f64 = -999.0;

/// O primeiro ano dos dados mensais e o último mês do MERRA-2 no POWER.
pub const PRIMEIRO_ANO: i32 = 1981;
pub const ULTIMO: (i32, u8) = (2026, 9);

/// O que cada comando traz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variavel {
    Umidade,
    Evapotranspiracao,
}

/// A média de 2001 a 2025, o valor de um mês ou ano, ou a diferença entre os dois.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Medida {
    Media,
    Valor,
    Anomalia,
}

/// As escolhas da janela e os valores delas ([`crate::dialogos`]).
pub const ESCOLHAS_MEDIDAS: [(&str, &str); 3] =
    [("Média de 2001 a 2025", "media"), ("O valor de um mês ou ano", "valor"), ("Anomalia de um mês ou ano", "anomalia")];

impl Variavel {
    pub const TODAS: [Variavel; 2] = [Variavel::Umidade, Variavel::Evapotranspiracao];

    pub fn comando(self) -> &'static str {
        match self {
            Variavel::Umidade => "power-umidade",
            Variavel::Evapotranspiracao => "power-evapotranspiracao",
        }
    }

    /// O parâmetro do POWER.
    pub fn parametro(self) -> &'static str {
        match self {
            Variavel::Umidade => "GWETROOT",
            Variavel::Evapotranspiracao => "EVLAND",
        }
    }

    /// O nome, como começa o da camada.
    pub fn nome(self, anomalia: bool) -> &'static str {
        match (self, anomalia) {
            (Variavel::Umidade, false) => "Umidade do solo nas raízes",
            (Variavel::Umidade, true) => "Anomalia da umidade do solo nas raízes",
            (Variavel::Evapotranspiracao, false) => "Evapotranspiração",
            (Variavel::Evapotranspiracao, true) => "Anomalia da evapotranspiração",
        }
    }

    /// O nome por extenso, para os avisos.
    pub fn por_extenso(self) -> &'static str {
        match self {
            Variavel::Umidade => "a umidade do solo na zona das raízes (o primeiro metro)",
            Variavel::Evapotranspiracao => "a evapotranspiração da terra",
        }
    }

    pub fn unidade(self) -> &'static str {
        match self {
            Variavel::Umidade => "da saturação",
            Variavel::Evapotranspiracao => "mm por dia",
        }
    }

    /// As casas dos números nos avisos.
    pub fn casas(self) -> usize {
        match self {
            Variavel::Umidade => 2,
            Variavel::Evapotranspiracao => 1,
        }
    }

    /// A escala das cores. Nos 2.264 pontos do Brasil, 96% das médias dos meses da umidade ficam entre
    /// 0,35 e 0,95 e as da evapotranspiração, entre 0,4 e 5,2; nas anomalias dos meses de sete anos
    /// (1998, 2012, 2015, 2016, 2023, 2024 e 2026), os percentis 1 e 99 de cada ano vão de −0,27 a 0,22
    /// e de −2,9 a 1,7.
    pub fn escala(self, anomalia: bool) -> (f64, f64) {
        match (self, anomalia) {
            (Variavel::Umidade, false) => (0.2, 1.0),
            (Variavel::Umidade, true) => (-0.2, 0.2),
            (Variavel::Evapotranspiracao, false) => (0.0, 6.0),
            (Variavel::Evapotranspiracao, true) => (-2.0, 2.0),
        }
    }

    /// Os valores aceitos, com folga.
    pub fn aceitos(self, anomalia: bool) -> (f64, f64) {
        match (self, anomalia) {
            (Variavel::Umidade, false) => (0.0, 1.0),
            (Variavel::Umidade, true) => (-1.0, 1.0),
            (Variavel::Evapotranspiracao, false) => (0.0, 30.0),
            (Variavel::Evapotranspiracao, true) => (-30.0, 30.0),
        }
    }

    /// A rampa (o nome e o texto do `ramp.add`, formato GDAL): do marrom (seco) ao verde-azulado
    /// (úmido), a da evapotranspiração do JAXA, ou, nas anomalias, as duas pontas com o branco no zero.
    pub fn rampa(self, anomalia: bool) -> (&'static str, String) {
        let (nome, cores): (&str, [&str; 5]) = match (self, anomalia) {
            (Variavel::Umidade, false) => {
                ("BrasilData: umidade nas raízes", ["140 81 10", "216 179 101", "246 232 195", "90 180 172", "1 102 94"])
            }
            (Variavel::Umidade, true) => {
                ("BrasilData: anomalia da umidade", ["140 81 10", "216 179 101", "245 245 245", "90 180 172", "1 102 94"])
            }
            (Variavel::Evapotranspiracao, false) => {
                ("BrasilData: evapotranspiração", ["255 255 204", "194 230 153", "120 198 121", "49 163 84", "0 104 55"])
            }
            (Variavel::Evapotranspiracao, true) => {
                ("BrasilData: anomalia da evapotranspiração", ["140 81 10", "216 179 101", "245 245 245", "127 191 123", "27 120 55"])
            }
        };
        let (a, b) = self.escala(anomalia);
        let texto: Vec<String> =
            cores.iter().enumerate().map(|(i, c)| format!("{} {c}", sol::decimos(a + (b - a) * i as f64 / 4.0))).collect();
        (nome, texto.join("\n"))
    }

    /// O título da barra de cores do layout e quantos valores ela mostra.
    pub fn barra(self, anomalia: bool) -> (&'static str, usize) {
        match (self, anomalia) {
            (Variavel::Umidade, false) => ("Umidade do solo (0: seco; 1: saturado)", 5),
            (Variavel::Umidade, true) => ("Diferença da média (fração da saturação)", 5),
            (Variavel::Evapotranspiracao, false) => ("Evapotranspiração (mm por dia)", 7),
            (Variavel::Evapotranspiracao, true) => ("Diferença da média (mm por dia)", 5),
        }
    }

    /// A citação, como o POWER pede.
    pub fn citacao(self) -> &'static str {
        match self {
            Variavel::Umidade => {
                "The data was obtained from the NASA Langley Research Center (LaRC) POWER Project funded through the NASA Earth Science/Applied Science Program (APIs mensal e de climatologia). Umidade do solo na zona das raízes (GWETROOT), da reanálise MERRA-2 (NASA GMAO; Gelaro et al., 2017, doi:10.1175/JCLI-D-16-0758.1), em células de 0,625° por 0,5°; a climatologia e a referência da anomalia são de 2001 a 2025; interpolada para a grade do projeto e recortada pela área"
            }
            Variavel::Evapotranspiracao => {
                "The data was obtained from the NASA Langley Research Center (LaRC) POWER Project funded through the NASA Earth Science/Applied Science Program (APIs mensal e de climatologia). Evapotranspiração da terra (EVLAND), da reanálise MERRA-2 (NASA GMAO; Gelaro et al., 2017, doi:10.1175/JCLI-D-16-0758.1), em células de 0,625° por 0,5°; a climatologia e a referência da anomalia são de 2001 a 2025; interpolada para a grade do projeto e recortada pela área"
            }
        }
    }
}

/// Um pedido conferido: a medida, o ano (na média, nenhum) e o mês (nenhum: o ano).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Escolha {
    pub medida: Medida,
    pub ano: Option<i32>,
    pub mes: Option<u8>,
}

/// O pedido do comando: sem `medida`, um ano pede o valor dele e, sem ano, a média. Na média, o ano
/// (que a janela sempre manda) não conta.
pub fn conferir(medida: Option<&str>, ano: Option<f64>, mes: Option<f64>) -> Result<Escolha, String> {
    let mes = sol::conferir_mes(mes)?;
    let medida = match medida {
        None if ano.is_some() => Medida::Valor,
        None | Some("media") => Medida::Media,
        Some("valor") => Medida::Valor,
        Some("anomalia") => Medida::Anomalia,
        Some(m) => return Err(format!("a medida é media, valor ou anomalia, não '{m}'")),
    };
    if medida == Medida::Media {
        return Ok(Escolha { medida, ano: None, mes });
    }
    let ano = ano.ok_or("diga o ano (e o mês, se for de um mês)")?;
    if ano.fract() != 0.0 || ano < f64::from(PRIMEIRO_ANO) || ano > f64::from(ULTIMO.0) {
        return Err(format!("o ano vai de {PRIMEIRO_ANO} a {}, não {ano}", ULTIMO.0));
    }
    let ano = ano as i32;
    if ano == ULTIMO.0 && mes.is_none_or(|m| m > ULTIMO.1) {
        let fim = format!("{} de {}", nome_do_mes(ULTIMO.1), ULTIMO.0);
        return Err(format!(
            "o MERRA-2 do NASA POWER vai até {fim}; daí em diante a fonte é o GEOS-IT, outro modelo, que não se compara com ele: escolha um mês até {fim} ou um ano até {}",
            ULTIMO.0 - 1
        ));
    }
    Ok(Escolha { medida, ano: Some(ano), mes })
}

/// "março".
fn nome_do_mes(mes: u8) -> &'static str {
    NOMES_DOS_MESES.get(usize::from(mes).wrapping_sub(1)).copied().unwrap_or_default()
}

impl Escolha {
    pub fn anomalia(self) -> bool {
        self.medida == Medida::Anomalia
    }

    /// O período: "média do ano", "média de março", "2024" ou "março de 2024".
    pub fn periodo(self) -> String {
        match (self.ano, self.mes) {
            (None, None) => "média do ano".into(),
            (None, Some(m)) => format!("média de {}", nome_do_mes(m)),
            (Some(a), None) => a.to_string(),
            (Some(a), Some(m)) => format!("{} de {a}", nome_do_mes(m)),
        }
    }

    /// O nome da camada.
    pub fn camada(self, v: Variavel) -> String {
        format!("BrasilData: {}, {}{FIM}", v.nome(self.anomalia()), self.periodo())
    }

    /// Os dias do período, para o total da evapotranspiração (na média, com os 29 de fevereiro).
    pub fn dias(self) -> f64 {
        let bissexto = self.ano.is_some_and(|a| a % 4 == 0 && (a % 100 != 0 || a % 400 == 0));
        let extra = match self.ano {
            None => 0.25,
            Some(_) if bissexto => 1.0,
            Some(_) => 0.0,
        };
        match self.mes {
            None => 365.0 + extra,
            Some(2) => 28.0 + extra,
            Some(4 | 6 | 9 | 11) => 30.0,
            Some(_) => 31.0,
        }
    }
}

/// O fim do nome das camadas.
const FIM: &str = " (NASA POWER)";

/// A variável, se é a anomalia e o período da camada `nome`.
pub fn ler_camada(nome: &str) -> Option<(Variavel, bool, &str)> {
    let n = nome.strip_prefix("BrasilData: ")?.strip_suffix(FIM)?;
    Variavel::TODAS
        .into_iter()
        .flat_map(|v| [(v, false), (v, true)])
        .find_map(|(v, a)| n.strip_prefix(v.nome(a))?.strip_prefix(", ").map(|p| (v, a, p)))
}

/// Os pedidos que cobrem o retângulo `c`, em CSV: os do ano (no valor e na anomalia) e os da média (na
/// média e na anomalia).
pub fn pedidos(c: [f64; 4], v: Variavel, e: Escolha) -> (Vec<String>, Vec<String>) {
    let do_ano = e.ano.map(|a| sol::pedidos_do_ano(c, v.parametro(), a)).unwrap_or_default();
    let da_media = if e.medida == Medida::Valor { Vec::new() } else { sol::pedidos_de(c, v.parametro()) };
    (do_ano, da_media)
}

/// As células, como o texto de um GeoJSON com o valor em [`CAMPO`], das respostas do ano e da média, e
/// quantos são os pontos com dado. Na anomalia, a diferença em cada ponto que está nas duas.
pub fn celulas(do_ano: &[&str], da_media: &[&str], v: Variavel, e: Escolha) -> (String, usize) {
    let ler = |r: &[&str]| sol::pontos_de(r, v.parametro(), LADO, e.mes);
    let pontos = match e.medida {
        Medida::Media => ler(da_media),
        Medida::Valor => ler(do_ano),
        Medida::Anomalia => {
            let media: std::collections::HashMap<_, _> =
                ler(da_media).into_iter().map(|(x, y, m)| (sol::chave_do_ponto(x, y, LADO), m)).collect();
            ler(do_ano).into_iter().filter_map(|(x, y, a)| Some((x, y, a - media.get(&sol::chave_do_ponto(x, y, LADO))?))).collect()
        }
    };
    (sol::celulas_dos_pontos(&pontos, LADO, CAMPO), pontos.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const MENSAL: &str = "-BEGIN HEADER-\r\nGWETROOT     MERRA-2 Root Zone Soil Wetness (1) \r\n-END HEADER-\r\nPARAMETER,YEAR,LAT,LON,JAN,FEB,MAR,APR,MAY,JUN,JUL,AUG,SEP,OCT,NOV,DEC,ANN\r\n";
    const MEDIA: &str = "-BEGIN HEADER-\r\nGWETROOT     MERRA-2 Root Zone Soil Wetness (1) \r\n-END HEADER-\r\nPARAMETER,LAT,LON,JAN,FEB,MAR,APR,MAY,JUN,JUL,AUG,SEP,OCT,NOV,DEC,ANN\r\n";

    fn resposta(t: String) -> String {
        json!({"ok": {"status": 200, "text": t}}).to_string()
    }

    fn e(medida: Medida, ano: Option<i32>, mes: Option<u8>) -> Escolha {
        Escolha { medida, ano, mes }
    }

    fn valores(texto: &str) -> Vec<(f64, f64, f64)> {
        let fc: Value = serde_json::from_str(texto).unwrap();
        fc["features"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                let c = &f["geometry"]["coordinates"][0];
                let meio = |i: usize| (c[0][i].as_f64().unwrap() + c[2][i].as_f64().unwrap()) / 2.0;
                (meio(0), meio(1), f["properties"][CAMPO].as_f64().unwrap())
            })
            .collect()
    }

    #[test]
    fn o_pedido_confere_o_ano_o_mes_e_o_fim_do_merra2() {
        assert_eq!(conferir(None, None, None), Ok(e(Medida::Media, None, None)));
        assert_eq!(conferir(None, Some(2024.0), Some(9.0)), Ok(e(Medida::Valor, Some(2024), Some(9))));
        assert_eq!(conferir(Some("media"), Some(2024.0), Some(9.0)), Ok(e(Medida::Media, None, Some(9))));
        assert_eq!(conferir(Some("anomalia"), Some(1981.0), None), Ok(e(Medida::Anomalia, Some(1981), None)));
        assert!(conferir(Some("anomalia"), None, Some(3.0)).is_err());
        assert!(conferir(Some("valor"), Some(1980.0), None).is_err());
        assert!(conferir(Some("valor"), Some(2027.0), None).is_err());
        assert!(conferir(Some("valor"), Some(2001.5), None).is_err());
        assert!(conferir(Some("outra"), None, None).is_err());
        assert!(conferir(Some("valor"), Some(2026.0), Some(9.0)).is_ok());
        assert!(conferir(Some("valor"), Some(2026.0), Some(10.0)).unwrap_err().contains("GEOS-IT"));
        let sem_mes = conferir(Some("anomalia"), Some(2026.0), None).unwrap_err();
        assert!(sem_mes.contains("setembro de 2026") && sem_mes.contains("um ano até 2025"), "{sem_mes}");
    }

    #[test]
    fn os_nomes_vao_e_voltam() {
        for v in Variavel::TODAS {
            for (esc, periodo) in [
                (e(Medida::Media, None, None), "média do ano"),
                (e(Medida::Media, None, Some(3)), "média de março"),
                (e(Medida::Valor, Some(2024), Some(9)), "setembro de 2024"),
                (e(Medida::Anomalia, Some(2023), None), "2023"),
            ] {
                assert_eq!(ler_camada(&esc.camada(v)), Some((v, esc.anomalia(), periodo)));
            }
        }
        assert_eq!(
            e(Medida::Anomalia, Some(2024), Some(9)).camada(Variavel::Umidade),
            "BrasilData: Anomalia da umidade do solo nas raízes, setembro de 2024 (NASA POWER)"
        );
        for outra in [sol::CAMADA, crate::vento::CAMADA, "BrasilData: Evapotranspiração (GCOM-C, JAXA)"] {
            assert_eq!(ler_camada(outra), None);
        }
        assert!(!sol::eh_do_sol(&e(Medida::Media, None, None).camada(Variavel::Umidade)));
        assert!(!crate::vento::eh_do_vento(&e(Medida::Media, None, None).camada(Variavel::Umidade)));
    }

    #[test]
    fn os_pedidos_de_cada_medida() {
        let c = [-43.0, -20.85, -42.75, -20.62];
        let (a, m) = pedidos(c, Variavel::Umidade, e(Medida::Anomalia, Some(2024), Some(9)));
        assert!(
            a[0].contains("monthly/regional?start=2024&end=2024&") && a[0].ends_with("&parameters=GWETROOT&community=RE&format=CSV"),
            "{}",
            a[0]
        );
        assert!(m[0].contains("climatology/regional?latitude-min=-22&") && a.len() == 1 && m.len() == 1);
        let (a, m) = pedidos(c, Variavel::Evapotranspiracao, e(Medida::Valor, Some(2024), None));
        assert_eq!((a.len(), m.len()), (1, 0));
        assert_eq!(pedidos(c, Variavel::Evapotranspiracao, e(Medida::Media, None, None)).0.len(), 0);
    }

    #[test]
    fn a_anomalia_e_a_diferenca_em_cada_ponto_e_os_vazios_ficam_de_fora() {
        let mes = |x: f64, y: f64, set: &str, ann: &str| {
            format!("GWETROOT,2024,{y},{x},0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,{set},0.1,0.2,0.3,{ann}\r\n")
        };
        let media =
            |x: f64, y: f64, set: f64, ann: f64| format!("GWETROOT,{y},{x},0.1,0.2,0.3,0.4,0.5,0.6,0.7,0.8,{set},0.1,0.2,0.3,{ann}\r\n");
        let ano = resposta(format!(
            "{MENSAL}{}{}{}",
            mes(-42.5, -20.5, "0.40", "0.50"),
            mes(-41.875, -20.5, "", ""),
            mes(-42.5, -20.0, "0.60", "0.55")
        ));
        let clim = resposta(format!("{MEDIA}{}{}", media(-42.5, -20.5, 0.55, 0.52), media(-42.5, -20.0, 0.5, 0.5)));
        let (t, n) = celulas(&[&ano], &[&clim], Variavel::Umidade, e(Medida::Anomalia, Some(2024), Some(9)));
        assert_eq!(n, 2);
        let vs = valores(&t);
        let no = |x: f64, y: f64| vs.iter().find(|v| (v.0 - x).abs() < 1e-6 && (v.1 - y).abs() < 1e-6).unwrap().2;
        assert!((no(-42.5, -20.5) + 0.15).abs() < 1e-9 && (no(-42.5, -20.0) - 0.1).abs() < 1e-9);
        assert_eq!(vs.len(), 2);
        let (t, n) = celulas(&[&ano], &[], Variavel::Umidade, e(Medida::Valor, Some(2024), None));
        assert_eq!((n, valores(&t)[0].2), (2, 0.5));
        let (_, n) = celulas(&[], &[&clim], Variavel::Umidade, e(Medida::Media, None, Some(9)));
        assert_eq!(n, 2);
        // Nada em comum, nenhuma célula.
        assert_eq!(celulas(&[&ano], &[], Variavel::Umidade, e(Medida::Anomalia, Some(2024), Some(9))).1, 0);
    }

    #[test]
    fn as_rampas_e_os_dias() {
        let (_, r) = Variavel::Umidade.rampa(true);
        assert!(
            r.starts_with("-0.2000 140 81 10\n-0.1000 ") && r.contains("\n0.0000 245 245 245\n") && r.ends_with("0.2000 1 102 94"),
            "{r}"
        );
        assert_eq!(e(Medida::Valor, Some(2024), Some(2)).dias(), 29.0);
        assert_eq!(e(Medida::Valor, Some(2023), None).dias(), 365.0);
        assert_eq!(e(Medida::Media, None, Some(9)).dias(), 30.0);
        assert_eq!(e(Medida::Media, None, None).dias(), 365.25);
    }
}
