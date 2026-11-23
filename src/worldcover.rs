//! A cobertura da terra do ESA WorldCover a 10 m (2020 e 2021): onze classes, feitas com o
//! Sentinel-1 e o Sentinel-2 por um consórcio liderado pela VITO, licença CC BY 4.0.
//!
//! - **De onde:** a coleção `esa-worldcover` do Planetary Computer da Microsoft. A busca do STAC
//!   lista os blocos que a área toca; os arquivos ficam no Azure Blob Storage e são lidos com um
//!   token anônimo, como no [`crate::radar_pc`]. Sem login; com CORS (`*`) e leitura por partes.
//!   O mesmo dado está no balde `esa-worldcover` da AWS, mas lá sem CORS, e no Terrascope, que
//!   pede conta.
//! - **Os arquivos:** um COG por bloco de 3° × 3° em WGS 84, 36.000 × 36.000 células de 1/12.000°
//!   (9,3 m de norte a sul), `uint8`, 0 é sem dado. A pirâmide é a da classe mais frequente nos
//!   dois anos (conferido em 23/11/2026 no bloco S21W045: onde três ou quatro das quatro células
//!   têm a mesma classe, a da primeira visão é sempre ela).
//! - **Como:** de cada bloco, só a janela da parte da área (`layer.addUrl` com `bbox`), da visão
//!   mais grossa cujas células não passam das da grade; vai para a grade do projeto com a classe
//!   mais frequente, as janelas se juntam e o mosaico é recortado pela área.
//! - **Os anos não se subtraem:** 2020 é a versão 100 do algoritmo e 2021 a 200. A diferença entre
//!   os mapas mistura a mudança do terreno com a do método (aviso da própria ESA).

use serde_json::Value;

use crate::cog_global::{Arquivo, MAX_VALORES};

/// A busca de blocos do STAC do Planetary Computer.
const BUSCA: &str = "https://planetarycomputer.microsoft.com/api/stac/v1/search";

/// A coleção.
const COLECAO: &str = "esa-worldcover";

/// O pedido do token anônimo que lê os arquivos da coleção.
pub const URL_DO_TOKEN: &str = "https://planetarycomputer.microsoft.com/api/sas/v1/token/esa-worldcover";

/// Os anos, do mais recente.
pub const ANOS: [i64; 2] = [2021, 2020];

/// O lado do bloco, em graus.
const BLOCO: f64 = 3.0;

/// O lado da célula do arquivo inteiro, em graus.
pub const LADO: f64 = 1.0 / 12_000.0;

/// Quantos níveis de pirâmide se usam além do inteiro (o sexto, de 562 células, não é exato).
const PIRAMIDES: i32 = 5;

/// Quantos blocos um clique lê, no máximo: um estado médio (Minas Gerais toca 20).
pub const MAX_BLOCOS: usize = 24;

/// As classes: o valor, o nome em português e a cor da legenda oficial.
pub const CLASSES: [(u8, &str, &str); 11] = [
    (10, "Árvores", "#006400"),
    (20, "Arbustos", "#ffbb22"),
    (30, "Campo e pastagem", "#ffff4c"),
    (40, "Cultivos anuais", "#f096ff"),
    (50, "Área construída", "#fa0000"),
    (60, "Solo exposto ou vegetação esparsa", "#b4b4b4"),
    (70, "Neve e gelo", "#f0f0f0"),
    (80, "Água permanente", "#0064c8"),
    (90, "Área úmida herbácea", "#0096a0"),
    (95, "Manguezal", "#00cf75"),
    (100, "Musgos e liquens", "#fae6a0"),
];

/// O que cada código quer dizer, para o aviso (as classes que não existem no Brasil ficam de fora).
pub const CHAVE: &str = "10 árvores (verde-escuro), 20 arbustos (laranja), 30 campo e pastagem (amarelo), 40 cultivos anuais (rosa), 50 área construída (vermelho), 60 solo exposto (cinza), 80 água (azul), 90 área úmida (azul-esverdeado), 95 manguezal (verde-água)";

/// O aviso de que os anos não se comparam.
pub const AVISO_ANOS: &str = "WorldCover: 2020 e 2021 foram feitos com versões diferentes do algoritmo; a diferença entre os dois mapas não é só mudança do terreno. Para a mudança no tempo, o MapBiomas";

pub const LICENCA: &str = "CC BY 4.0";

/// O ano pedido, se há mapa dele.
pub fn conferir_ano(ano: f64) -> Result<i64, String> {
    ANOS.into_iter().find(|&a| a as f64 == ano).ok_or(format!("o WorldCover tem os anos de 2020 e 2021 (veio {ano})"))
}

/// A versão do algoritmo do mapa de `ano`.
fn versao(ano: i64) -> &'static str {
    if ano == 2020 { "v100" } else { "v200" }
}

/// O nome da camada de `ano`.
pub fn camada(ano: i64) -> String {
    format!("BrasilData: Cobertura da terra {ano} (ESA WorldCover, 10 m)")
}

/// O ano de uma camada desta fonte, pelo nome.
pub fn ano_da_camada(nome: &str) -> Option<i64> {
    ANOS.into_iter().find(|&a| nome == camada(a))
}

/// A citação de `ano`: a atribuição que a ESA pede no mapa e a referência do dado.
pub fn citacao(ano: i64) -> String {
    let referencia = if ano == 2020 {
        "Zanaga, D. et al. (2021). ESA WorldCover 10 m 2020 v100, doi:10.5281/zenodo.5571936"
    } else {
        "Zanaga, D. et al. (2022). ESA WorldCover 10 m 2021 v200, doi:10.5281/zenodo.7254221"
    };
    format!(
        "© ESA WorldCover project {ano} / Contains modified Copernicus Sentinel data ({ano}) processed by ESA WorldCover consortium. {referencia}. Lido do Microsoft Planetary Computer, posto na grade do projeto com a classe mais frequente e recortado pela área"
    )
}

/// O estilo do raster (paleta com os nomes das classes), no formato do QGIS que o
/// `style.importQgis` lê. O RustGIS ainda não guarda os rótulos (RustGIS#889): a legenda mostra os
/// códigos, que o aviso e a nota do layout ([`CHAVE`]) explicam.
pub fn qml() -> String {
    let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
    for (v, nome, cor) in CLASSES {
        q.push_str(&format!(r#"<paletteEntry value="{v}" color="{cor}" alpha="255" label="{nome}"/>"#));
    }
    q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
    q
}

// --- Blocos -----------------------------------------------------------------------------------

/// Um bloco que a área toca, com a parte da área que ele serve.
#[derive(Clone, Debug, PartialEq)]
pub struct Bloco {
    /// O canto sudoeste no nome do arquivo ("S21W045").
    pub canto: String,
    /// O endereço do arquivo, sem o token.
    pub href: String,
    /// A parte da área dentro do bloco, em graus.
    pub parte: [f64; 4],
}

impl Bloco {
    /// O nome da camada de passagem com a janela do bloco, lida no nível `k`.
    pub fn nome_da_janela(&self, ano: i64, k: i32) -> String {
        format!("BrasilData: WorldCover {ano} {} nível {k} (janela)", self.canto)
    }
}

/// Quantos blocos de 3° o retângulo `c` toca (no mar alguns não existem).
pub fn quantos_blocos(c: [f64; 4]) -> usize {
    let n = |a: f64, b: f64| ((b / BLOCO).ceil() - (a / BLOCO).floor()).max(0.0) as usize;
    n(c[0], c[2]) * n(c[1], c[3])
}

/// A busca dos blocos que o retângulo `c` toca, dos dois anos.
pub fn url_busca(c: [f64; 4]) -> String {
    format!("{BUSCA}?collections={COLECAO}&bbox={:.4},{:.4},{:.4},{:.4}&limit=100&fields=id,assets.map.href", c[0], c[1], c[2], c[3])
}

/// O canto sudoeste, em graus (lon, lat), de um nome como "S21W045".
fn canto_em_graus(canto: &str) -> Option<(f64, f64)> {
    if canto.len() != 7 || !canto.is_ascii() {
        return None;
    }
    let sinal = |letra: &str, mais: &str, menos: &str| match letra {
        l if l == mais => Some(1.0),
        l if l == menos => Some(-1.0),
        _ => None,
    };
    let lat = sinal(&canto[..1], "N", "S")? * canto[1..3].parse::<f64>().ok()?;
    let lon = sinal(&canto[3..4], "E", "W")? * canto[4..].parse::<f64>().ok()?;
    Some((lon, lat))
}

/// Os blocos de `ano` na resposta da busca `r` que a área `c` toca com alguma largura, em ordem de
/// nome.
pub fn blocos(r: &Value, ano: i64, c: [f64; 4]) -> Vec<Bloco> {
    let prefixo = format!("ESA_WorldCover_10m_{ano}_{}_", versao(ano));
    let mut v: Vec<Bloco> = r["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let canto = f["id"].as_str()?.strip_prefix(&prefixo)?;
            let href = f["assets"]["map"]["href"].as_str()?;
            let (x, y) = canto_em_graus(canto)?;
            let parte = [c[0].max(x), c[1].max(y), c[2].min(x + BLOCO), c[3].min(y + BLOCO)];
            (parte[0] < parte[2] && parte[1] < parte[3]).then(|| Bloco { canto: canto.into(), href: href.into(), parte })
        })
        .collect();
    v.sort_by(|a, b| a.canto.cmp(&b.canto));
    v.dedup_by(|a, b| a.canto == b.canto);
    v
}

/// As contas da pirâmide de um bloco.
pub fn arquivo() -> Arquivo {
    Arquivo { url: String::new(), graus: LADO, piramides: PIRAMIDES }
}

/// O `cellSize` do `layer.addUrl` que lê o nível `k`.
pub fn celula_pedida(k: i32) -> f64 {
    crate::cog_global::celula_pedida(arquivo().graus_do_nivel(k))
}

/// O nível de pirâmide que se lê: o mais grosso cujas células não passam de `celula` metros na
/// latitude `lat`, ou, se uma janela não cabe no limite do `layer.addUrl`, o mais fino em que todas
/// cabem.
pub fn nivel(bs: &[Bloco], celula: f64, lat: f64) -> Option<i32> {
    let a = arquivo();
    let k = a.nivel(celula, lat);
    bs.iter().try_fold(k, |m, b| a.nivel_que_cabe(b.parte, m))
}

// A janela maior possível de um bloco, na visão mais grossa, cabe no limite.
const _: () = assert!((36_000.0 / 32.0) * (36_000.0 / 32.0) < MAX_VALORES);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const VICOSA: [f64; 4] = [-43.0049, -20.839, -42.7536, -20.6547];

    fn item(ano: i64, v: &str, canto: &str) -> Value {
        json!({"id": format!("ESA_WorldCover_10m_{ano}_{v}_{canto}"),
            "assets": {"map": {"href": format!("https://ai4edataeuwest.blob.core.windows.net/esa-worldcover/{v}/{ano}/map/ESA_WorldCover_10m_{ano}_{v}_{canto}_Map.tif")}}})
    }

    #[test]
    fn os_blocos_do_ano_que_a_area_toca() {
        let r = json!({"features": [item(2021, "v200", "S21W045"), item(2020, "v100", "S21W045")]});
        let b = blocos(&r, 2021, VICOSA);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].canto, "S21W045");
        assert_eq!(b[0].parte, VICOSA);
        assert!(b[0].href.ends_with("/v200/2021/map/ESA_WorldCover_10m_2021_v200_S21W045_Map.tif"));
        assert!(blocos(&r, 2020, VICOSA)[0].href.contains("/v100/2020/"));
        assert!(blocos(&json!({"features": []}), 2021, VICOSA).is_empty());
        assert!(blocos(&json!({}), 2021, VICOSA).is_empty());
    }

    #[test]
    fn a_area_que_cruza_blocos_e_repartida_sem_sobrar_nem_faltar() {
        // Um retângulo em volta do encontro de quatro blocos (−42°, −21°); o quinto está fora.
        let c = [-42.2, -21.1, -41.9, -20.8];
        let r = json!({"features": [
            item(2021, "v200", "S24W045"), item(2021, "v200", "S24W042"), item(2021, "v200", "S21W045"),
            item(2021, "v200", "S21W042"), item(2021, "v200", "S21W042"), item(2021, "v200", "S18W042"),
        ]});
        let b = blocos(&r, 2021, c);
        assert_eq!(b.iter().map(|b| b.canto.as_str()).collect::<Vec<_>>(), ["S21W042", "S21W045", "S24W042", "S24W045"]);
        let area: f64 = b.iter().map(|b| (b.parte[2] - b.parte[0]) * (b.parte[3] - b.parte[1])).sum();
        assert!((area - 0.3 * 0.3).abs() < 1e-9, "{area}");
        assert_eq!(quantos_blocos(c), 4);
        assert_eq!(quantos_blocos(VICOSA), 1);
        // Minas Gerais cabe num clique; o Brasil não.
        assert_eq!(quantos_blocos([-51.05, -22.92, -39.85, -14.23]), 20);
        assert!(quantos_blocos([-73.99, -33.75, -34.79, 5.27]) > MAX_BLOCOS);
    }

    #[test]
    fn os_cantos_dos_nomes() {
        assert_eq!(canto_em_graus("S21W045"), Some((-45.0, -21.0)));
        assert_eq!(canto_em_graus("N03W063"), Some((-63.0, 3.0)));
        assert_eq!(canto_em_graus("N00E006"), Some((6.0, 0.0)));
        assert_eq!(canto_em_graus("X21W045"), None);
        assert_eq!(canto_em_graus("S21W45"), None);
        assert_eq!(canto_em_graus("S2aW045"), None);
    }

    #[test]
    fn o_nivel_segue_a_grade_e_o_limite() {
        let b = |parte| Bloco { canto: String::new(), href: String::new(), parte };
        let v = [b(VICOSA)];
        assert_eq!(nivel(&v, 10.0, -20.7), Some(0));
        assert_eq!(nivel(&v, 30.0, -20.7), Some(1), "17 m; a visão de 35 m passaria da grade");
        assert_eq!(nivel(&v, 500.0, -20.7), Some(5));
        // Um bloco inteiro não cabe a 10 m: vem da visão mais fina que cabe, e as outras janelas
        // da mesma leitura também.
        let inteiro = b([-45.0, -21.0, -42.0, -18.0]);
        assert_eq!(nivel(std::slice::from_ref(&inteiro), 10.0, -19.5), Some(3));
        assert_eq!(nivel(&[inteiro, b(VICOSA)], 10.0, -19.5), Some(3));
        assert_eq!(nivel(&[], 30.0, -20.7), Some(1));
        // O pedido escolhe a visão do nível e não a seguinte.
        for k in 0..=PIRAMIDES {
            let lado = 9.2767 * 2f64.powi(k);
            assert!(celula_pedida(k) > lado && celula_pedida(k) < 1.5 * lado, "{k}");
        }
    }

    #[test]
    fn anos_camadas_e_citacoes() {
        for a in ANOS {
            assert_eq!(ano_da_camada(&camada(a)), Some(a));
            assert!(citacao(a).starts_with(&format!("© ESA WorldCover project {a} / ")));
            assert_eq!(conferir_ano(a as f64), Ok(a));
        }
        assert!(citacao(2020).contains("5571936") && citacao(2021).contains("7254221"));
        assert_eq!(ano_da_camada("BrasilData: Cobertura da terra 2019 (Copernicus 100 m)"), None);
        assert!(conferir_ano(2019.0).is_err() && conferir_ano(2020.5).is_err());
    }

    #[test]
    fn a_paleta_e_os_textos() {
        let q = qml();
        assert_eq!(q.matches("<paletteEntry").count(), CLASSES.len());
        assert!(q.contains(r##"value="80" color="#0064c8" alpha="255" label="Água permanente""##));
        // Cabe num aviso do RustGIS, que corta em 500 caracteres.
        assert!(AVISO_ANOS.chars().count() <= 480);
        assert!(url_busca(VICOSA).contains("bbox=-43.0049,-20.8390,-42.7536,-20.6547"));
    }
}
