//! O layout do mapa: uma folha pronta (`layout.publication` do RustGIS) com o título da área e os
//! textos do dado principal, a "receita" de cada camada do BrasilData.
//!
//! - **Qual dado:** a primeira camada visível do BrasilData, de cima para baixo, que tem receita;
//!   a área, a grade e o desenho não contam.
//! - **A receita:** o subtítulo (o que é o dado e de quando), a barra de cores (o título com a
//!   unidade e quantos valores ela mostra) ou a legenda das classes, e a nota.
//! - **O resto vem do plugin:** o título é o nome da área, as fontes são as citações da tabela
//!   Fontes, e a projeção é a da grade do projeto (ou a automática da área).

use crate::gedi;
use crate::grade::GRADE;
use crate::{AREA, PREFIXO};

/// O que o layout diz de uma camada.
#[derive(Debug, PartialEq)]
pub struct Receita {
    /// Abaixo do título: o que é o dado e de quando.
    pub subtitulo: String,
    /// A barra de cores de um raster contínuo: o título (com a unidade) e quantos valores ela
    /// mostra, as pontas incluídas. Sem barra, a camada vai para a legenda.
    pub barra: Option<(String, usize)>,
    /// O título da legenda, quando a camada vai para ela com classes nomeadas (sem ele, a legenda
    /// fica sem título: um título a mais tira uma linha da caixa).
    pub legenda: Option<&'static str>,
    pub nota: Option<&'static str>,
}

const NOTA_GEDI: &str = "Valores imputados a 30 m (cada pixel recebe o disparo do GEDI mais parecido pelas séries do Landsat): mostram padrões, não medidas locais.";
const NOTA_DIST: &str = "Alertas acumulados desde o início do monitoramento (04/01/2026). A primeira detecção ainda não foi confirmada e inclui mudanças sazonais.";

/// A data no fim de um nome do DIST ("…, até 28/02/2026)").
fn data_no_nome(n: &str) -> Option<&str> {
    let i = n.rfind("até ")? + "até ".len();
    n.get(i..i + 10)
}

/// A receita da camada `nome`, se o layout a conhece.
pub fn receita(nome: &str) -> Option<Receita> {
    let n = nome.strip_prefix(PREFIXO)?;
    if [AREA, GRADE].contains(&nome) {
        return None;
    }
    if let Some(v) = gedi::Variavel::TODAS.into_iter().find(|v| v.camada() == nome) {
        let (lo, hi) = v.escala();
        let passo = match v {
            gedi::Variavel::Altura => 5.0,
            gedi::Variavel::Biomassa => 50.0,
            gedi::Variavel::Cobertura => 25.0,
        };
        return Some(Receita {
            subtitulo: format!("{} pelo laser GEDI (NASA), 2019–2023", v.titulo()),
            barra: Some((format!("{} ({})", v.titulo(), v.unidade()), ((hi - lo) / passo) as usize + 1)),
            legenda: None,
            nota: Some(NOTA_GEDI),
        });
    }
    if n.starts_with("Perturbação da vegetação") && n.contains("OPERA DIST-S1") {
        let quando = data_no_nome(n).map(|d| format!(", situação em {d}")).unwrap_or_default();
        return Some(Receita {
            subtitulo: format!("Alertas de perturbação da vegetação pelo radar Sentinel-1 (OPERA DIST-S1){quando}"),
            barra: None,
            legenda: Some("Situação do alerta"),
            nota: Some(NOTA_DIST),
        });
    }
    for (inicio, titulo, barra) in [
        ("Perturbação, dias antes de", "Dias desde a primeira detecção da perturbação", "Dias antes do produto"),
        ("Confiança da perturbação", "Confiança dos alertas de perturbação", "Confiança"),
        ("Intensidade da perturbação", "Intensidade da perturbação da vegetação", "Intensidade"),
    ] {
        if n.starts_with(inicio) && n.contains("OPERA DIST-S1") {
            return Some(Receita {
                subtitulo: format!("{titulo} pelo radar Sentinel-1 (OPERA DIST-S1)"),
                barra: Some((barra.to_string(), 5)),
                legenda: None,
                nota: Some(NOTA_DIST),
            });
        }
    }
    if n == "Elevação (Copernicus GLO-30)" {
        return Some(Receita {
            subtitulo: "Elevação do terreno (Copernicus DEM GLO-30)".into(),
            barra: Some(("Altitude (m)".into(), 5)),
            legenda: None,
            nota: None,
        });
    }
    if crate::radar::eh_do_radar(nome) {
        // "Radar VV em dB (Sentinel-1, 26/10/2026)": a polarização e a data da passagem.
        let qual = n.split(" (").next().unwrap_or(n).trim_end_matches(" em dB");
        let quando = n.rsplit(", ").next().map(|d| d.trim_end_matches(')')).filter(|d| d.len() == 10);
        let quando = quando.map(|d| format!(" em {d}")).unwrap_or_default();
        // As escalas fixas do plugin: 7 dB nas polarizações (marcas de 1 dB) e 5 dB na razão.
        let marcas = if qual.starts_with("Razão") { 6 } else { 8 };
        return Some(Receita {
            subtitulo: format!("{qual} do Sentinel-1{quando}, corrigido do relevo (OPERA RTC-S1)"),
            barra: Some((format!("{qual} (dB)"), marcas)),
            legenda: None,
            nota: None,
        });
    }
    if let Some(resto) = n.strip_prefix("Uso e cobertura ")
        && resto.ends_with(" (MapBiomas, recortado)")
    {
        let ano = resto.split(' ').next().unwrap_or("");
        return Some(Receita {
            subtitulo: format!("Uso e cobertura da terra em {ano} (MapBiomas, Coleção 10)"),
            barra: None,
            legenda: Some("Classes do MapBiomas"),
            nota: None,
        });
    }
    // Um tema vetorial: o subtítulo é o nome dele, e as classes vão para a legenda.
    if let Some(t) = crate::tema_da_camada(nome) {
        return Some(Receita { subtitulo: t.nome.to_string(), barra: None, legenda: None, nota: None });
    }
    // Qualquer outro dado do BrasilData (setores e indicadores do censo, saúde, focos...): o
    // subtítulo é o nome da camada, e ela vai para a legenda como está.
    Some(Receita { subtitulo: n.to_string(), barra: None, legenda: None, nota: None })
}

/// Os mapas coropléticos das camadas de indicadores e de setores: o campo, o que ele é (vai no
/// subtítulo, depois do nome da camada) e a rampa de cores. Vale o primeiro que a camada tem (os
/// campos de cada fonte de indicadores).
pub const COROPLETICOS: &[(&str, &str, &str)] = &[
    ("dens_hab_km2", "densidade demográfica (hab/km²)", "YlOrRd"),
    ("dens_2022", "densidade demográfica em 2022 (hab/km²)", "YlOrRd"),
    ("estab_2017", "estabelecimentos agropecuários em 2017", "YlGn"),
    ("estab_organico", "estabelecimentos com produção orgânica em 2017", "Greens"),
    ("pix_recebido", "Pix recebido no mês (R$)", "Blues"),
    ("credito_custeio", "crédito rural de custeio no ano (R$)", "Greens"),
    ("abstencao_2026", "abstenção no 1º turno de 2026 (%)", "Purples"),
    ("caged_saldo_12m", "saldo de empregos formais em 12 meses (CAGED)", "Oranges"),
];

/// O mapa coroplético da camada `l` (uma do `info.project`): um polígono com um dos campos de
/// [`COROPLETICOS`].
pub fn coropletico(l: &serde_json::Value) -> Option<(&'static str, &'static str, &'static str)> {
    if !l["type"].as_str().is_some_and(|t| t.contains("Polygon")) {
        return None;
    }
    let campos: Vec<&str> = l["fields"].as_array()?.iter().filter_map(|f| f.as_str()).map(|f| f.split(" (").next().unwrap_or(f)).collect();
    COROPLETICOS.iter().find(|(c, ..)| campos.contains(c)).copied()
}

/// Se a camada `l` (uma do `info.project`) é um polígono de referência maior que o retângulo `c`
/// da área, de um símbolo só: municípios, regiões, bacias, distritos. Pintado como veio, cobriria a
/// folha inteira de uma cor. Um tema com classes de cor fica como está.
pub fn referencia_maior(l: &serde_json::Value, c: [f64; 4]) -> bool {
    let poligono = l["type"].as_str().is_some_and(|t| t.contains("Polygon"));
    let simples = l["renderer"].as_str() == Some("Single Symbol");
    let e: Vec<f64> = l["extent"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_f64).collect();
    let [x0, y0, x1, y1] = e[..] else { return false };
    let (dx, dy) = ((c[2] - c[0]) * 0.02, (c[3] - c[1]) * 0.02);
    poligono && simples && (x0 < c[0] - dx || y0 < c[1] - dy || x1 > c[2] + dx || y1 > c[3] + dy)
}

/// O campo com o nome de cada feição (`nm_mun`, `NM_UF`, `nome`...), para os rótulos: da lista
/// `fields` do `info.project` ("nm_mun (Text)").
pub fn campo_do_nome(l: &serde_json::Value) -> Option<String> {
    l["fields"]
        .as_array()?
        .iter()
        .filter_map(|f| f.as_str())
        .map(|f| f.split(" (").next().unwrap_or(f))
        .find(|f| {
            let f = f.to_ascii_lowercase();
            f.starts_with("nm_") || f == "nome" || f.starts_with("nome_")
        })
        .map(str::to_string)
}

/// O passo da grade em metros: o redondo que dá até 6 linhas no lado maior do retângulo `c`.
pub fn passo_da_grade(c: [f64; 4]) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let lado = ((c[2] - c[0]) * 111_320.0 * lat.cos()).max((c[3] - c[1]) * 110_574.0);
    [1e3, 2e3, 5e3, 1e4, 2e4, 5e4, 1e5, 2e5, 5e5].into_iter().find(|p| lado / p <= 6.0).unwrap_or(1e6)
}

/// O título: o nome da área (a primeira linha do que o painel diz dela), em caixa alta.
pub fn titulo(texto_da_area: &str) -> String {
    let linha = texto_da_area.lines().next().unwrap_or("").trim();
    if linha.is_empty() || linha.ends_with("polígono") || linha.ends_with("polígonos") {
        "ÁREA DE TRABALHO".into()
    } else {
        linha.to_uppercase()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_receitas_das_camadas_do_gedi_e_do_dist() {
        let r = receita(gedi::Variavel::Altura.camada()).unwrap();
        assert_eq!(r.subtitulo, "Altura do dossel pelo laser GEDI (NASA), 2019–2023");
        assert_eq!(r.barra, Some(("Altura do dossel (m)".into(), 8)));
        let r = receita(gedi::Variavel::Biomassa.camada()).unwrap();
        assert_eq!(r.barra, Some(("Biomassa acima do solo (Mg/ha)".into(), 4)));
        let r = receita("BrasilData: Perturbação da vegetação, manchas (OPERA DIST-S1, até 28/02/2026)").unwrap();
        assert!(r.subtitulo.ends_with("situação em 28/02/2026"), "{}", r.subtitulo);
        assert_eq!((r.barra, r.legenda), (None, Some("Situação do alerta")));
        let r = receita("BrasilData: Confiança da perturbação (OPERA DIST-S1, até 28/02/2026)").unwrap();
        assert_eq!(r.barra, Some(("Confiança".into(), 5)));
    }

    #[test]
    fn as_outras_receitas() {
        assert_eq!(receita("BrasilData: Elevação (Copernicus GLO-30)").unwrap().barra, Some(("Altitude (m)".into(), 5)));
        let r = receita("BrasilData: Radar VV em dB (Sentinel-1, 26/10/2026)").unwrap();
        assert_eq!(r.subtitulo, "Radar VV do Sentinel-1 em 26/10/2026, corrigido do relevo (OPERA RTC-S1)");
        assert_eq!(r.barra, Some(("Radar VV (dB)".into(), 8)));
        let r = receita("BrasilData: Razão VH/VV em dB (Sentinel-1, 26/10/2026)").unwrap();
        assert_eq!(r.barra, Some(("Razão VH/VV (dB)".into(), 6)));
        let r = receita("BrasilData: Uso e cobertura 2024 (MapBiomas, recortado)").unwrap();
        assert_eq!(r.subtitulo, "Uso e cobertura da terra em 2024 (MapBiomas, Coleção 10)");
        assert_eq!(receita(AREA), None);
        assert_eq!(receita(GRADE), None);
        assert_eq!(receita("Countries"), None);
        assert_eq!(receita("BrasilData: Setores censitários 2022 (MG)").unwrap().subtitulo, "Setores censitários 2022 (MG)");
    }

    #[test]
    fn o_mapa_coropletico_dos_indicadores() {
        let todos = serde_json::json!({"type": "Polygon", "fields": ["cd_mun (Text)", "pix_recebido (Double)", "dens_2022 (Double)"]});
        assert_eq!(coropletico(&todos).map(|c| c.0), Some("dens_2022"), "a densidade vem antes");
        let pix = serde_json::json!({"type": "Polygon", "fields": ["cd_mun (Text)", "pix_mes (Text)", "pix_recebido (Double)"]});
        assert_eq!(coropletico(&pix).map(|c| c.0), Some("pix_recebido"));
        assert_eq!(coropletico(&serde_json::json!({"type": "Polygon", "fields": ["nm_mun (Text)"]})), None);
        assert_eq!(coropletico(&serde_json::json!({"type": "Point", "fields": ["dens_2022 (Double)"]})), None);
    }

    #[test]
    fn os_poligonos_de_referencia_maiores_que_a_area() {
        let c = [-43.005, -20.839, -42.754, -20.655];
        let municipios = serde_json::json!({"type": "Polygon", "renderer": "Single Symbol", "extent": [-43.19, -20.99, -42.63, -20.45],
            "fields": ["cd_mun (Text)", "nm_mun (Text)", "sigla_uf (Text)"]});
        assert!(referencia_maior(&municipios, c));
        assert_eq!(campo_do_nome(&municipios).as_deref(), Some("nm_mun"));
        // Recortada pela área, ou com classes de cor, fica como está.
        let solos = serde_json::json!({"type": "Polygon", "renderer": "Single Symbol", "extent": c, "fields": ["legenda (Text)"]});
        assert!(!referencia_maior(&solos, c));
        assert_eq!(campo_do_nome(&solos), None);
        let dist = serde_json::json!({"type": "Polygon", "renderer": "Unique Values", "extent": [-44.0, -21.0, -42.0, -20.0]});
        assert!(!referencia_maior(&dist, c));
        let focos = serde_json::json!({"type": "Point", "renderer": "Single Symbol", "extent": [-44.0, -21.0, -42.0, -20.0]});
        assert!(!referencia_maior(&focos, c));
    }

    #[test]
    fn o_passo_da_grade_e_o_titulo() {
        // Viçosa: cerca de 26 km de lado maior.
        assert_eq!(passo_da_grade([-43.005, -20.839, -42.7535, -20.6547]), 5000.0);
        // Minas Gerais: cerca de 1.200 km.
        assert_eq!(passo_da_grade([-51.05, -22.93, -39.85, -14.23]), 200_000.0);
        assert_eq!(titulo("Viçosa (MG)\n-43.005, -20.839 a -42.754, -20.655"), "VIÇOSA (MG)");
        assert_eq!(titulo("3 polígonos\n-43, -21 a -42, -20"), "ÁREA DE TRABALHO");
    }
}
