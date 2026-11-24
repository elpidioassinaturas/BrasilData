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

/// A célula de quem não escolhe, numa área pequena: 30 m, a do Copernicus GLO-30 e do MapBiomas.
pub const CELULA_PADRAO: f64 = 30.0;

/// Quantas células, no máximo, a grade padrão tem: uma área maior ganha uma célula maior
/// ([`celula_padrao`]). Um município e a Zona da Mata ficam com 30 m; Minas Gerais, com 250 m; o
/// Brasil, com 500 m.
pub const MAX_CELULAS_PADRAO: f64 = 1.0e8;

/// Quantas células um raster pode ter no RustGIS ("more than this computer's 1000000000"): uma
/// grade com mais não serve a nenhum comando.
pub const MAX_CELULAS: f64 = 1.0e9;

/// O aviso de que o dado, de células de `nativa` metros, foi posto numa grade de `celula` metros
/// bem diferente: numa grade mais grossa (mais de 1,5 vez), cada célula resume várias do dado e o
/// detalhe se perde; numa mais fina (menos de 2/3), cada célula do dado se repete, sem detalhe a
/// mais. `None` quando as duas se parecem. `dado` é o nome do dado no aviso.
pub fn aviso_de_reamostragem(dado: &str, nativa: f64, celula: f64, mais_grossa_tira: &str) -> Option<String> {
    // A célula do dado em metros no chão vem de graus (26,0253… m): até 10 m, com dois decimais;
    // acima, inteira.
    let n = texto_da_celula(if nativa < 10.0 { (nativa * 100.0).round() / 100.0 } else { nativa.round() });
    let g = texto_da_celula(celula);
    let fina = texto_da_celula(nativa.ceil().max(1.0));
    if celula > nativa * 1.5 {
        Some(format!(
            "Atenção: {dado} tem células de {n}, e a grade do projeto, de {g}: cada célula da camada {mais_grossa_tira} de umas {} células do dado, e o detalhe se perde. Para ver o dado na resolução dele, use Grade do projeto com {fina} (numa área pequena) e rode de novo",
            ((celula / nativa).powi(2)).round()
        ))
    } else if celula < nativa / 1.5 {
        Some(format!(
            "Atenção: {dado} tem células de {n}, mais grossas que as de {g} da grade do projeto: cada célula do dado se repete na grade, sem detalhe a mais"
        ))
    } else {
        None
    }
}

/// Quantos valores uma operação de raster do RustGIS (`raster.project`, o mosaico, o recorte)
/// aceita ("cells × outputs is … values, more than 32000000").
pub const MAX_VALORES_OPERACAO: f64 = 32e6;

/// A menor célula, em metros inteiros, com que a grade do retângulo `c` cabe numa operação de
/// raster do RustGIS ([`MAX_VALORES_OPERACAO`]), com 5% de folga para a borda da grade.
pub fn menor_celula_da_operacao(c: [f64; 4]) -> f64 {
    (celulas(c, 1.0) * 1.05 / MAX_VALORES_OPERACAO).sqrt().ceil()
}

/// Quantas células, mais ou menos, uma grade de `celula` metros tem no retângulo `c` (em graus).
pub fn celulas(c: [f64; 4], celula: f64) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians().cos();
    let (largura, altura) = ((c[2] - c[0]).abs() * 111_320.0 * lat, (c[3] - c[1]).abs() * 110_574.0);
    largura * altura / (celula * celula)
}

/// A célula padrão para a área de retângulo `c`: a menor do painel, de [`CELULA_PADRAO`] para
/// cima, que deixa a grade com até [`MAX_CELULAS_PADRAO`] células; numa área ainda maior, a maior
/// do painel.
pub fn celula_padrao(c: [f64; 4]) -> f64 {
    CELULAS.into_iter().filter(|m| *m >= CELULA_PADRAO).find(|m| celulas(c, *m) <= MAX_CELULAS_PADRAO).unwrap_or(CELULAS[CELULAS.len() - 1])
}

/// A célula para a área de retângulo `c` com a extensão Imagens do RustGIS desligada: a menor do
/// painel, de `minima` para cima, com que a grade cabe no recorte sem ela
/// ([`MAX_VALORES_OPERACAO`], módulo [`crate::recorte`]); numa área ainda maior, a maior do painel.
pub fn celula_sem_imagens(c: [f64; 4], escolha: Escolha, minima: f64) -> f64 {
    let epsg = escolha.epsg(c);
    CELULAS
        .into_iter()
        .filter(|m| *m >= minima)
        .find(|m| celulas_no_sistema(c, epsg, *m) <= MAX_VALORES_OPERACAO)
        .unwrap_or(CELULAS[CELULAS.len() - 1])
}

/// Quantas células uma grade de `celula` metros tem no retângulo `c` (em graus) posto no sistema
/// `epsg`: o retângulo fica torto longe do meridiano central, e a grade que o envolve tem mais
/// células que a conta de [`celulas`] (14% no Rio Grande do Norte, no Albers).
///
/// A projeção é a da esfera: contra o RustGIS 0.8.0, em nove retângulos (Viçosa, sete UFs e o
/// Brasil), a conta dá de 0 a 1% a mais. Um sistema que não é o UTM SIRGAS 2000, o Albers do IBGE ou
/// a Policônica leva a conta de [`celulas`] com 15% de folga.
pub fn celulas_no_sistema(c: [f64; 4], epsg: u32, celula: f64) -> f64 {
    const PARTES: i32 = 32;
    let lados = |t: f64| {
        let (x, y) = (c[0] + t * (c[2] - c[0]), c[1] + t * (c[3] - c[1]));
        [(x, c[1]), (x, c[3]), (c[0], y), (c[2], y)]
    };
    let (mut x, mut y) = ((f64::MAX, f64::MIN), (f64::MAX, f64::MIN));
    for (lon, lat) in (0..=PARTES).flat_map(|i| lados(f64::from(i) / f64::from(PARTES))) {
        let Some((px, py)) = na_esfera(epsg, lon, lat) else { return celulas(c, celula) * 1.15 };
        x = (x.0.min(px), x.1.max(px));
        y = (y.0.min(py), y.1.max(py));
    }
    // A grade vai até a célula inteira de cada lado.
    ((x.1 - x.0) / celula + 1.0) * ((y.1 - y.0) / celula + 1.0)
}

/// O ponto `lon`, `lat` (em graus) no sistema `epsg`, em metros, na esfera, sem os deslocamentos da
/// origem; `None` num sistema que não é um dos da grade.
fn na_esfera(epsg: u32, lon: f64, lat: f64) -> Option<(f64, f64)> {
    const RAIO: f64 = 6_378_137.0;
    let f = lat.to_radians();
    let delta = |lon0: f64| (lon - lon0).to_radians();
    let fuso = match epsg {
        31977..=31985 => Some(epsg - 31960),
        31965..=31976 => Some(epsg - 31954),
        _ => None,
    };
    if let Some(z) = fuso {
        let l = delta(-183.0 + 6.0 * f64::from(z));
        let k = 0.9996 * RAIO;
        return Some((k * (f.cos() * l.sin()).atanh(), k * f.sin().atan2(f.cos() * l.cos())));
    }
    match epsg {
        // O Albers do IBGE: paralelos padrão 2° S e 22° S, meridiano 54° W.
        10857 => {
            let (p1, p2) = ((-2f64).to_radians(), (-22f64).to_radians());
            let n = (p1.sin() + p2.sin()) / 2.0;
            let rho = (p1.cos().powi(2) + 2.0 * n * p1.sin() - 2.0 * n * f.sin()).sqrt() * RAIO / n;
            let t = n * delta(-54.0);
            Some((rho * t.sin(), -rho * t.cos()))
        }
        // A Policônica: o equador é uma reta.
        5880 => {
            let l = delta(-54.0);
            if f.abs() < 1e-9 {
                return Some((RAIO * l, 0.0));
            }
            let (t, cot) = (l * f.sin(), f.cos() / f.sin());
            Some((RAIO * cot * t.sin(), RAIO * (f + cot * (1.0 - t.cos()))))
        }
        _ => None,
    }
}

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

    #[test]
    fn a_celula_padrao_acompanha_a_area() {
        let vicosa = [-43.003, -20.838, -42.754, -20.655];
        let zona_da_mata = [-43.37, -22.92, -40.56, -20.52];
        let minas = [-51.05, -22.93, -39.85, -14.23];
        let brasil = [-73.99, -33.75, -28.84, 5.27];
        assert_eq!(celula_padrao(vicosa), 30.0);
        assert_eq!(celula_padrao(zona_da_mata), 30.0);
        assert_eq!(celula_padrao(minas), 250.0);
        assert_eq!(celula_padrao(brasil), 500.0);
        // O erro do RustGIS: o Brasil a 30 m tem uns 23 bilhões de células.
        assert!(celulas(brasil, 30.0) > MAX_CELULAS);
        assert!(celulas(brasil, 500.0) < MAX_CELULAS_PADRAO);
    }

    const SERGIPE: [f64; 4] = [-38.245, -11.5681, -36.3959, -9.515];
    const MINAS: [f64; 4] = [-51.0461, -22.9228, -39.8568, -14.2333];
    const BRASIL: [f64; 4] = [-73.9831, -33.7439, -32.4026, 5.2673];
    const VICOSA: [f64; 4] = [-43.004866, -20.838979, -42.753575, -20.654733];
    const RN: [f64; 4] = [-38.5821, -6.9827, -34.9685, -4.8314];

    #[test]
    fn as_celulas_no_sistema_sao_as_do_rustgis() {
        // O que o RustGIS 0.8.0 faz da grade (raster.rasterize dela, com a célula).
        for (c, epsg, m, n) in [
            (MINAS, 10857, 250.0, 4800.0 * 4019.0),
            (BRASIL, 10857, 1000.0, 4740.0 * 4379.0),
            (VICOSA, 31983, 30.0, 882.0 * 692.0),
            (RN, 10857, 100.0, 4105.0 * 2642.0),
        ] {
            let razao = celulas_no_sistema(c, epsg, m) / n;
            assert!((0.99..1.02).contains(&razao), "{c:?}: {razao}");
        }
        // Na Policônica, perto do meridiano dela, e num sistema qualquer, a conta é parecida.
        let c = [-55.0, -16.0, -53.0, -14.0];
        for epsg in [5880, 31982, 32722] {
            let razao = celulas_no_sistema(c, epsg, 100.0) / celulas(c, 100.0);
            assert!((0.95..1.2).contains(&razao), "{epsg}: {razao}");
        }
    }

    #[test]
    fn sem_a_imagens_a_grade_padrao_cabe_no_recorte() {
        for (c, m) in [(SERGIPE, 50.0), (MINAS, 250.0), (BRASIL, 1000.0), (VICOSA, 30.0)] {
            assert_eq!(celula_sem_imagens(c, Escolha::Automatico, celula_padrao(c)), m, "{c:?}");
            assert!(celulas_no_sistema(c, sistema(c), m) <= MAX_VALORES_OPERACAO);
        }
        // Uma célula que já cabe fica; uma fora da lista do painel sobe para a seguinte.
        assert_eq!(celula_sem_imagens(SERGIPE, Escolha::Automatico, 250.0), 250.0);
        assert_eq!(celula_sem_imagens(SERGIPE, Escolha::Automatico, 35.0), 50.0);
        // Maior que tudo: a maior do painel.
        assert_eq!(celula_sem_imagens([-180.0, -60.0, 180.0, 60.0], Escolha::Albers, 30.0), 1000.0);
    }

    const TERESINA: [f64; 4] = [-42.9707, -5.5866, -42.599, -4.7863];
    /// O Piauí passa dos fusos 23 e 24.
    const PIAUI: [f64; 4] = [-45.994, -10.928, -40.37, -2.739];

    #[test]
    fn o_aviso_de_reamostragem() {
        let a = aviso_de_reamostragem("a altura da CTrees", 4.78, 500.0, "é a média").unwrap();
        assert!(a.contains("células de 4,78 m") && a.contains("de 500 m") && a.contains("10942 células"), "{a}");
        assert!(a.contains("Grade do projeto com 5 m"), "{a}");
        assert!(aviso_de_reamostragem("x", 10.0, 1.0, "é a média").unwrap().contains("se repete"));
        assert_eq!(aviso_de_reamostragem("x", 10.0, 10.0, "é a média"), None);
        assert_eq!(aviso_de_reamostragem("x", 4.78, 6.0, "é a média"), None);
        let olm = aviso_de_reamostragem("o pH", 26.025_353_977_274_932, 500.0, "é a média").unwrap();
        assert!(olm.contains("células de 26 m, e a grade do projeto, de 500 m"), "{olm}");
    }

    #[test]
    fn a_menor_celula_de_uma_operacao() {
        // Teixeirópolis (RO): 5 m dá uns 35 milhões de células, 6 m cabe.
        let c = [-62.495, -11.030, -62.123, -10.835];
        assert!(celulas(c, 5.0) > MAX_VALORES_OPERACAO);
        assert_eq!(menor_celula_da_operacao(c), 6.0);
        assert!(celulas(c, 6.0) < MAX_VALORES_OPERACAO);
    }

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
