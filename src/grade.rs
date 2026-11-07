//! A grade do projeto (etapa 3 do `../PLANO.md`): o sistema de coordenadas e o tamanho das
//! células em que os rasters do BrasilData chegam.
//!
//! - **Onde fica:** no mapa, como a camada [`GRADE`], porque o plugin não guarda nada entre
//!   chamadas. É o retângulo da área, em graus, com o sistema e a célula nos atributos. Fica
//!   salva com o projeto e acompanha a área: uma área nova refaz a grade com as mesmas escolhas.
//! - **Como os rasters vão para ela:** `raster.project` com `crs`, `cellSize` e `extent` (a
//!   camada da grade). A origem fica no padrão do RustGIS, `[0, 0]`: os cantos das células caem
//!   nos múltiplos da célula, e dois rasters na mesma grade têm as mesmas linhas e colunas.
//! - **Depois:** o `raster.rasterize` (opengeos/RustGIS#1409, depois da v0.5.0) aceita os mesmos
//!   parâmetros, e os vetores rasterizados caem nas mesmas células.

use serde_json::{Value, json};

use crate::sistema;

/// O nome da camada da grade.
pub const GRADE: &str = "BrasilData: grade";

/// Os tamanhos de célula do painel, em metros.
pub const CELULAS: [f64; 9] = [10.0, 20.0, 30.0, 50.0, 90.0, 100.0, 250.0, 500.0, 1000.0];

/// A célula de quem não escolhe: 30 m, a do Copernicus GLO-30 e do MapBiomas.
pub const CELULA_PADRAO: f64 = 30.0;

/// Os sistemas geográficos (em graus), que não servem para células em metros.
const EM_GRAUS: [u32; 3] = [4326, 4674, 4618];

/// O sistema de coordenadas escolhido para a grade.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escolha {
    /// O UTM SIRGAS 2000 do fuso da área, ou o Albers do IBGE quando ela passa de um fuso.
    Automatico,
    /// A Policônica SIRGAS 2000 (EPSG:5880), a do IBGE para o país. O `raster.project` do RustGIS
    /// 0.5.0 não a tem; as versões depois dela, sim.
    Policonica,
    /// A Cônica Equivalente de Albers do IBGE, SIRGAS 2000 (EPSG:10857): áreas iguais.
    Albers,
    /// Um código EPSG dado por script.
    Epsg(u32),
}

impl Escolha {
    /// As escolhas do painel, na ordem do controle "Grade: sistema".
    pub const DO_PAINEL: [Escolha; 3] = [Escolha::Automatico, Escolha::Policonica, Escolha::Albers];

    /// O código EPSG da grade numa área de retângulo `c`.
    pub fn epsg(self, c: [f64; 4]) -> u32 {
        match self {
            Escolha::Automatico => sistema(c),
            Escolha::Policonica => 5880,
            Escolha::Albers => 10857,
            Escolha::Epsg(n) => n,
        }
    }

    /// O rótulo curto do controle do painel, que é estreito; a ajuda do controle explica cada um.
    pub fn rotulo(self) -> String {
        match self {
            Escolha::Automatico => "Automático".into(),
            Escolha::Policonica => "Policônica".into(),
            Escolha::Albers => "Albers (IBGE)".into(),
            Escolha::Epsg(n) => format!("EPSG:{n}"),
        }
    }

    /// Como fica no atributo `escolha` da camada.
    pub fn codigo(self) -> String {
        match self {
            Escolha::Automatico => "automatico".into(),
            Escolha::Policonica => "policonica".into(),
            Escolha::Albers => "albers".into(),
            Escolha::Epsg(n) => format!("EPSG:{n}"),
        }
    }

    /// O `crs` de um script: um código EPSG (número, `"EPSG:n"` ou `"n"`) ou o nome de uma
    /// escolha (`automatico`, `policonica`, `albers`).
    pub fn do_parametro(v: &Value) -> Result<Escolha, String> {
        let erro = || format!("crs é um código EPSG em metros (31983, \"EPSG:5880\") ou automatico, policonica ou albers, não {v}");
        let n = match v {
            Value::Number(n) => n.as_u64().and_then(|n| u32::try_from(n).ok()).ok_or_else(erro)?,
            Value::String(s) => {
                let s = s.trim().to_lowercase();
                match s.as_str() {
                    "automatico" | "automático" => return Ok(Escolha::Automatico),
                    "policonica" | "policônica" => return Ok(Escolha::Policonica),
                    "albers" => return Ok(Escolha::Albers),
                    _ => s.strip_prefix("epsg:").unwrap_or(&s).trim().parse().map_err(|_| erro())?,
                }
            }
            _ => return Err(erro()),
        };
        if EM_GRAUS.contains(&n) {
            return Err(format!("EPSG:{n} é em graus; a grade pede um sistema em metros, como o UTM do fuso"));
        }
        Ok(match n {
            5880 => Escolha::Policonica,
            10857 => Escolha::Albers,
            0 => return Err(erro()),
            n => Escolha::Epsg(n),
        })
    }

    /// Lê o atributo `escolha` da camada.
    fn do_codigo(s: &str) -> Option<Escolha> {
        match s {
            "automatico" => Some(Escolha::Automatico),
            "policonica" => Some(Escolha::Policonica),
            "albers" => Some(Escolha::Albers),
            _ => s.strip_prefix("EPSG:")?.parse().ok().map(Escolha::Epsg),
        }
    }
}

/// O nome de um sistema, para o painel e os avisos.
pub fn nome_do_sistema(epsg: u32) -> String {
    match epsg {
        // SIRGAS 2000 / UTM: 31965 a 31976 são os fusos 11N a 22N, 31977 a 31985 os 17S a 25S.
        31977..=31985 => format!("UTM fuso {}S, SIRGAS 2000 (EPSG:{epsg})", epsg - 31960),
        31965..=31976 => format!("UTM fuso {}N, SIRGAS 2000 (EPSG:{epsg})", epsg - 31954),
        5880 => "Policônica do Brasil, SIRGAS 2000 (EPSG:5880)".into(),
        10857 => "Albers do IBGE, SIRGAS 2000 (EPSG:10857)".into(),
        n => format!("EPSG:{n}"),
    }
}

/// Uma célula em metros, escrita como no painel: "30 m", "12,5 m", "1 km".
pub fn texto_da_celula(m: f64) -> String {
    if m >= 1000.0 && (m / 1000.0).fract() == 0.0 {
        return format!("{} km", m / 1000.0);
    }
    format!("{m} m").replace('.', ",")
}

/// Recusa uma célula fora de 1 m a 10 km.
pub fn conferir_celula(m: f64) -> Result<(), String> {
    if m.is_finite() && (1.0..=10_000.0).contains(&m) {
        Ok(())
    } else {
        Err(format!("a célula é um tamanho em metros, de 1 a 10.000, não {m}"))
    }
}

/// A grade que está no mapa.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grade {
    pub epsg: u32,
    pub celula: f64,
    pub escolha: Escolha,
    /// O retângulo dela, em graus.
    pub caixa: [f64; 4],
}

impl Grade {
    /// A grade de `escolha` e `celula` na área de retângulo `c`.
    pub fn nova(c: [f64; 4], escolha: Escolha, celula: f64) -> Grade {
        Grade { epsg: escolha.epsg(c), celula, escolha, caixa: c }
    }

    /// O texto do painel e dos avisos.
    pub fn texto(&self) -> String {
        format!("{}, células de {}", nome_do_sistema(self.epsg), texto_da_celula(self.celula))
    }

    /// A camada: o retângulo, com o sistema e a célula nos atributos.
    pub fn camada(&self) -> Value {
        let [w, s, e, n] = self.caixa;
        json!({"type": "FeatureCollection", "features": [{
            "type": "Feature",
            "geometry": {"type": "Polygon", "coordinates": [[[w, s], [e, s], [e, n], [w, n], [w, s]]]},
            "properties": {
                "sistema": format!("EPSG:{}", self.epsg),
                "nome_do_sistema": nome_do_sistema(self.epsg),
                "celula_m": self.celula,
                "escolha": self.escolha.codigo(),
                "origem": "0, 0",
            },
        }]})
    }

    /// Lê a grade da camada (`fc`, as feições dela).
    pub fn ler(fc: &Value) -> Option<Grade> {
        let f = fc["features"].as_array()?.first()?;
        let p = &f["properties"];
        let epsg = p["sistema"].as_str()?.strip_prefix("EPSG:")?.parse().ok()?;
        let celula = p["celula_m"].as_f64().filter(|c| conferir_celula(*c).is_ok())?;
        let escolha = p["escolha"].as_str().and_then(Escolha::do_codigo).unwrap_or(Escolha::Epsg(epsg));
        let pts = f["geometry"]["coordinates"][0].as_array()?;
        let xy: Vec<(f64, f64)> = pts.iter().filter_map(|p| Some((p[0].as_f64()?, p[1].as_f64()?))).collect();
        let caixa =
            xy.iter().fold([f64::MAX, f64::MAX, f64::MIN, f64::MIN], |b, (x, y)| [b[0].min(*x), b[1].min(*y), b[2].max(*x), b[3].max(*y)]);
        (!xy.is_empty()).then_some(Grade { epsg, celula, escolha, caixa })
    }

    /// Se ela cobre a área de retângulo `c` (com folga de arredondamento).
    pub fn cobre(&self, c: [f64; 4]) -> bool {
        let f = 1e-6;
        c[0] >= self.caixa[0] - f && c[1] >= self.caixa[1] - f && c[2] <= self.caixa[2] + f && c[3] <= self.caixa[3] + f
    }

    /// A posição do controle "Grade: célula" que mostra esta célula, se houver uma.
    pub fn no_controle_da_celula(&self) -> Option<usize> {
        CELULAS.iter().position(|c| (c - self.celula).abs() < 1e-9)
    }

    /// A posição do controle "Grade: sistema" que mostra esta escolha, se houver uma.
    pub fn no_controle_do_sistema(&self) -> Option<usize> {
        Escolha::DO_PAINEL.iter().position(|e| *e == self.escolha)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TERESINA: [f64; 4] = [-42.9707, -5.5866, -42.599, -4.7863];
    /// O Piauí passa dos fusos 23 e 24.
    const PIAUI: [f64; 4] = [-45.994, -10.928, -40.37, -2.739];

    #[test]
    fn o_automatico_e_o_utm_do_fuso_ou_o_albers() {
        assert_eq!(Escolha::Automatico.epsg(TERESINA), 31983);
        assert_eq!(Escolha::Automatico.epsg(PIAUI), 10857);
        assert_eq!(Escolha::Albers.epsg(PIAUI), 10857);
        assert_eq!(nome_do_sistema(31983), "UTM fuso 23S, SIRGAS 2000 (EPSG:31983)");
        assert_eq!(nome_do_sistema(31976), "UTM fuso 22N, SIRGAS 2000 (EPSG:31976)");
        assert_eq!(nome_do_sistema(31965), "UTM fuso 11N, SIRGAS 2000 (EPSG:31965)");
    }

    #[test]
    fn o_crs_de_um_script() {
        assert_eq!(Escolha::do_parametro(&json!(31983)), Ok(Escolha::Epsg(31983)));
        assert_eq!(Escolha::do_parametro(&json!("EPSG:5880")), Ok(Escolha::Policonica));
        assert_eq!(Escolha::do_parametro(&json!(" epsg:10857 ")), Ok(Escolha::Albers));
        assert_eq!(Escolha::do_parametro(&json!("Automático")), Ok(Escolha::Automatico));
        assert!(Escolha::do_parametro(&json!(4674)).unwrap_err().contains("em graus"));
        assert!(Escolha::do_parametro(&json!("utm")).is_err());
        assert!(Escolha::do_parametro(&json!(-1)).is_err());
        assert!(Escolha::do_parametro(&json!(0)).is_err());
    }

    #[test]
    fn a_camada_guarda_a_grade() {
        for escolha in [Escolha::Automatico, Escolha::Albers, Escolha::Epsg(31984)] {
            let g = Grade::nova(TERESINA, escolha, 90.0);
            assert_eq!(Grade::ler(&g.camada()), Some(g));
        }
        let g = Grade::nova(TERESINA, Escolha::Automatico, 30.0);
        assert_eq!(g.texto(), "UTM fuso 23S, SIRGAS 2000 (EPSG:31983), células de 30 m");
        assert!(g.cobre(TERESINA));
        assert!(!g.cobre(PIAUI));
        assert_eq!(g.no_controle_da_celula(), Some(2));
        assert_eq!(g.no_controle_do_sistema(), Some(0));
        assert_eq!(Grade::nova(TERESINA, Escolha::Epsg(31984), 12.5).no_controle_do_sistema(), None);
        assert_eq!(Grade::ler(&json!({"type": "FeatureCollection", "features": []})), None);
    }

    #[test]
    fn a_celula_escrita_e_conferida() {
        assert_eq!(texto_da_celula(30.0), "30 m");
        assert_eq!(texto_da_celula(12.5), "12,5 m");
        assert_eq!(texto_da_celula(1000.0), "1 km");
        assert_eq!(texto_da_celula(2500.0), "2500 m");
        assert!(conferir_celula(30.0).is_ok());
        assert!(conferir_celula(0.5).is_err());
        assert!(conferir_celula(f64::NAN).is_err());
        assert_eq!(CELULAS[CELULAS.iter().position(|c| *c == CELULA_PADRAO).unwrap()], 30.0);
    }
}
