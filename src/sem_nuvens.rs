//! A imagem sem nuvens do cubo do INPE (Brazil Data Cube): o "melhor pixel" das datas de 16 dias
//! de um período, em cor verdadeira ou em falsa-cor.
//!
//! Os cubos de 16 dias já guardam, em cada data, a observação menos nublada da quinzena; onde
//! todas estavam nubladas, fica a nuvem. Por isso o comando lê a máscara de nuvem de cada data na
//! área, ordena as datas pela parte limpa dentro dela e dá a cada célula a primeira data limpa
//! nessa ordem. A máscara é dilatada (as bordas de nuvem escapam ao SCL do Sentinel-2).
//!
//! Este módulo tem as escolhas (bandas, máscaras, contraste, ordem), as expressões da Raster
//! Calculator do GeoLibre e os nomes das camadas; quem chama o RustGIS é o `comandos.rs`. A
//! escolha do caminho fica em [`caminho`]: hoje uma leitura por data e banda com o `cube.reduce`
//! e a Raster Calculator; quando o `cube.reduce` tiver `mask` e `stat: "first"`
//! (opengeos/RustGIS#2585), uma leitura por banda.

use serde_json::{Value, json};

use crate::PREFIXO;
use crate::bdc::{FIM, STAC, Satelite};
use crate::datas::Data;

/// Os satélites do comando, na ordem da janela (o MODIS não tem as bandas de cor no cubo).
pub const SATELITES: [Satelite; 3] = [Satelite::Sentinel2, Satelite::Landsat, Satelite::Cbers4];

/// O período mais longo de um clique, em dias: um ano, cerca de 23 datas (cada uma é uma leitura
/// da máscara).
pub const MAX_DIAS: i64 = 366;

/// A parte nublada a partir da qual uma data fica de fora (não acrescenta nenhuma célula limpa).
pub const TODA_NUBLADA: f64 = 0.999;

/// As células de uma leitura, no máximo (cerca de 35 × 35 km a 10 m): cada data e banda é uma
/// camada de Float32 na memória, e os arquivos do cubo não têm pirâmides.
pub const MAX_CELULAS: f64 = 12.0e6;

/// O lado das células a ler na área `c` (oeste, sul, leste, norte, em graus): o do satélite, ou
/// maior, para não passar de [`MAX_CELULAS`]. Arredondado ao metro.
pub fn celula_de_leitura(s: Satelite, c: [f64; 4]) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let largura = (c[2] - c[0]).abs() * 111_320.0 * lat.cos();
    let altura = (c[3] - c[1]).abs() * 110_574.0;
    let minima = (largura * altura / MAX_CELULAS).sqrt();
    s.celula().max(minima.ceil())
}

/// O nome da camada temporária de uma leitura, de uma máscara ou de uma banda num dia:
/// "BrasilData: sem nuvens SCL 2025-07-12 (Sentinel-2)". O que um clique anterior leu serve.
pub fn temporaria(s: Satelite, banda: &str, d: Data) -> String {
    format!("{PREFIXO}sem nuvens {banda} {} ({})", d.iso(), s.nome())
}

/// O nome de uma camada temporária de um passo que não é leitura ("nuvem", "melhor B04"…).
pub fn passo(s: Satelite, o_que: &str, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}sem nuvens {o_que} {} a {} ({})", inicio.iso(), fim.iso(), s.nome())
}

/// A composição das cores.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Composicao {
    /// Vermelho, verde e azul: a cor que o olho veria.
    Natural,
    /// Infravermelho próximo, vermelho e verde: a vegetação em vermelho.
    FalsaCor,
}

impl Composicao {
    /// A composição pelo parâmetro (`natural`, `falsa-cor`).
    pub fn ler(texto: &str) -> Option<Composicao> {
        match texto.trim().to_lowercase().as_str() {
            "natural" | "cor-verdadeira" | "verdadeira" | "rgb" => Some(Composicao::Natural),
            "falsa-cor" | "falsa" | "infravermelho" | "nir" => Some(Composicao::FalsaCor),
            _ => None,
        }
    }

    /// O valor do parâmetro.
    pub fn id(self) -> &'static str {
        match self {
            Composicao::Natural => "natural",
            Composicao::FalsaCor => "falsa-cor",
        }
    }

    /// Como o nome da camada diz a composição.
    pub fn nome(self) -> &'static str {
        match self {
            Composicao::Natural => "cor verdadeira",
            Composicao::FalsaCor => "falsa-cor",
        }
    }
}

/// As bandas do STAC que vão no vermelho, no verde e no azul.
pub fn bandas(s: Satelite, c: Composicao) -> Option<[&'static str; 3]> {
    Some(match (s, c) {
        (Satelite::Sentinel2, Composicao::Natural) => ["B04", "B03", "B02"],
        (Satelite::Sentinel2, Composicao::FalsaCor) => ["B08", "B04", "B03"],
        (Satelite::Landsat, Composicao::Natural) => ["red", "green", "blue"],
        (Satelite::Landsat, Composicao::FalsaCor) => ["nir08", "red", "green"],
        (Satelite::Cbers4, Composicao::Natural) => ["BAND15", "BAND14", "BAND13"],
        (Satelite::Cbers4, Composicao::FalsaCor) => ["BAND16", "BAND15", "BAND14"],
        (Satelite::Modis, _) => return None,
    })
}

/// O contraste fixo de cada canal (vermelho, verde, azul), em reflectância (0 a 1): o mesmo em
/// todas as áreas e datas, para que as imagens se comparem. O do Sentinel-2 vem do teste em
/// Viçosa-MG (inverno de 2025), e serve ao Landsat; o do CBERS-4 WFI, que tem o azul mais alto e
/// menos contraste, dá a mesma cor média nas mesmas datas. Na falsa-cor,
/// o infravermelho da vegetação vai de 0,05 a 0,45, e o vermelho e o verde ficam como na cor
/// verdadeira.
pub fn contraste(s: Satelite, c: Composicao) -> ([f64; 3], [f64; 3]) {
    let (min, max) = match s {
        Satelite::Cbers4 => ([0.012, 0.02, 0.018], [0.13, 0.11, 0.093]),
        _ => ([0.01, 0.02, 0.01], [0.16, 0.13, 0.10]),
    };
    match c {
        Composicao::Natural => (min, max),
        Composicao::FalsaCor => ([0.05, min[0], min[1]], [0.45, max[0], max[1]]),
    }
}

/// A máscara de nuvem de um satélite no cubo: a banda e o teste de "limpo".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mascara {
    /// A banda de qualidade no STAC.
    pub banda: &'static str,
    /// As classes limpas, quando a banda é de classes (o SCL e o CMASK); vazio no Landsat, que
    /// vale pela contagem de observações limpas (`CLEAROB` > 0).
    pub limpas: &'static [u8],
    /// Quantas células a nuvem cresce para os lados (a dilatação).
    pub dilatacao: u32,
}

/// As classes limpas do SCL do Sentinel-2: 2 (sombra escura do terreno), 4 (vegetação), 5 (solo
/// exposto), 6 (água) e 7 (não classificado, quase sempre limpo). Ficam de fora 0 (sem dado),
/// 1 (saturado), 3 (sombra de nuvem), 8 a 10 (nuvens e cirros) e 11 (neve).
pub const SCL_LIMPAS: [u8; 5] = [2, 4, 5, 6, 7];

/// A máscara do satélite `s`: Sentinel-2 `SCL` ∈ {2, 4, 5, 6, 7}, dilatada 2 células (20 m);
/// Landsat `CLEAROB` > 0, dilatada 1 (30 m); CBERS-4 WFI `CMASK` = 127, dilatada 1 (64 m).
pub fn mascara(s: Satelite) -> Option<Mascara> {
    match s {
        Satelite::Sentinel2 => Some(Mascara { banda: "SCL", limpas: &SCL_LIMPAS, dilatacao: 2 }),
        Satelite::Landsat => Some(Mascara { banda: "CLEAROB", limpas: &[], dilatacao: 1 }),
        Satelite::Cbers4 => Some(Mascara { banda: "CMASK", limpas: &[127], dilatacao: 1 }),
        Satelite::Modis => None,
    }
}

impl Mascara {
    /// O teste de "limpo" da camada `nome` na Raster Calculator. As células sem dado chegam como
    /// NaN, e toda comparação com NaN é falsa: elas contam como nuvem (o `CLEAROB` 0 e o `CMASK`
    /// 255 são o "sem dado" dos arquivos).
    pub fn limpo(&self, nome: &str) -> String {
        let n = format!("\"{nome}\"");
        if self.limpas.is_empty() {
            return format!("({n} > 0)");
        }
        // Classes seguidas viram um intervalo: 4, 5, 6, 7 é "≥ 4 e ≤ 7".
        let mut partes = Vec::new();
        let mut i = 0;
        while i < self.limpas.len() {
            let mut j = i;
            while j + 1 < self.limpas.len() && self.limpas[j + 1] == self.limpas[j] + 1 {
                j += 1;
            }
            if j > i {
                partes.push(format!("(({n} >= {}) && ({n} <= {}))", self.limpas[i], self.limpas[j]));
            } else {
                partes.push(format!("({n} == {})", self.limpas[i]));
            }
            i = j + 1;
        }
        if partes.len() == 1 { partes.remove(0) } else { format!("({})", partes.join(" || ")) }
    }

    /// A expressão da nuvem de uma data: 1 onde não é limpo, 0 onde é.
    pub fn nuvem(&self, nome: &str) -> String {
        format!("if({}, 0, 1)", self.limpo(nome))
    }

    /// O lado da janela do filtro de máximo que dilata a nuvem (1 célula para cada lado: 3).
    pub fn janela(&self) -> u32 {
        2 * self.dilatacao + 1
    }
}

/// Uma data do cubo na área, com a parte dela coberta de nuvem (de 0 a 1), já dilatada.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataNaArea {
    pub data: Data,
    pub nuvem: f64,
}

/// As datas na ordem em que preenchem a imagem: a menos nublada na área primeiro; num empate, a
/// mais recente. As de toda nublada ficam de fora.
pub fn ordenar(datas: &[DataNaArea]) -> Vec<DataNaArea> {
    let mut v: Vec<DataNaArea> = datas.iter().copied().filter(|d| d.nuvem.is_finite() && d.nuvem < TODA_NUBLADA).collect();
    v.sort_by(|a, b| a.nuvem.total_cmp(&b.nuvem).then(b.data.cmp(&a.data)));
    v
}

/// As datas dos compostos do período, pela resposta da busca de itens do STAC
/// (`collections/{c}/items`): o início de cada composto de 16 dias, de `inicio` a `fim`, sem
/// repetir (cada data tem um item por tile), da mais antiga à mais recente.
pub fn datas_dos_itens(resposta: &Value, inicio: Data, fim: Data) -> Vec<Data> {
    let mut v: Vec<Data> = resposta["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let p = &f["properties"];
            let t = p["start_datetime"].as_str().or(p["datetime"].as_str())?;
            crate::bdc::ler_data(t.get(0..10)?)
        })
        .filter(|d| *d >= inicio && *d <= fim)
        .collect();
    v.sort();
    v.dedup();
    v
}

/// O endereço da busca dos itens da coleção do satélite `s` no retângulo `c`, de `inicio` a `fim`
/// (até 200 itens: um ano de datas em até 8 tiles).
pub fn url_dos_itens(s: Satelite, c: [f64; 4], inicio: Data, fim: Data) -> String {
    format!(
        "{STAC}collections/{}/items?bbox={},{},{},{}&datetime={}T00:00:00Z/{}T23:59:59Z&limit=200",
        s.colecao(),
        c[0],
        c[1],
        c[2],
        c[3],
        inicio.iso(),
        fim.iso()
    )
}

/// O "melhor pixel": a expressão que dá, em cada célula, o valor da primeira entrada cuja nuvem é
/// 0, nesta ordem, ou 0 onde nenhuma é limpa. Cada entrada é (camada da nuvem, valor): o valor é
/// um nome de camada (a banda) ou um número (a data de origem).
pub fn melhor_pixel(entradas: &[(String, Valor)]) -> String {
    entradas.iter().rev().fold("0".to_string(), |resto, (nuvem, v)| {
        let v = match v {
            Valor::Camada(c) => format!("\"{c}\""),
            Valor::Numero(n) => n.to_string(),
        };
        format!("if((\"{nuvem}\" == 0), {v}, {resto})")
    })
}

/// O valor de uma entrada do [`melhor_pixel`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Valor {
    Camada(String),
    Numero(u32),
}

/// As camadas de uma expressão da Raster Calculator, na ordem em que os nomes aparecem nela, sem
/// repetir. O RustGIS 0.7.0 liga as entradas à expressão pela posição, não pelo nome: elas têm
/// de ir nesta ordem.
pub fn entradas_da_expressao(expressao: &str) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for (k, pedaco) in expressao.split('"').enumerate() {
        if k % 2 == 1 && !v.iter().any(|n| n == pedaco) {
            v.push(pedaco.to_string());
        }
    }
    v
}

/// A camada da imagem: "BrasilData: Imagem sem nuvens em cor verdadeira de 01/06/2025 a
/// 31/08/2025 (Sentinel-2, cubo do INPE)".
pub fn camada(s: Satelite, c: Composicao, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}Imagem sem nuvens em {} de {} a {} ({}{FIM}", c.nome(), inicio.texto(), fim.texto(), s.nome())
}

/// A camada da data de origem de cada célula: "BrasilData: Data de origem da imagem sem nuvens de
/// 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)".
pub fn camada_da_data(s: Satelite, inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}Data de origem da imagem sem nuvens de {} a {} ({}{FIM}", inicio.texto(), fim.texto(), s.nome())
}

/// Se a camada `nome` é uma imagem sem nuvens ou a data de origem dela.
pub fn eh_sem_nuvens(nome: &str) -> bool {
    crate::bdc::satelite_da_camada(nome).is_some()
        && (nome.starts_with(&format!("{PREFIXO}Imagem sem nuvens "))
            || nome.starts_with(&format!("{PREFIXO}Data de origem da imagem sem nuvens ")))
}

/// As cores das datas de origem, na ordem de preferência (uma paleta qualitativa de 12; depois
/// ela se repete).
const CORES_DAS_DATAS: [&str; 12] =
    ["#1f77b4", "#ff7f0e", "#2ca02c", "#d62728", "#9467bd", "#8c564b", "#e377c2", "#7f7f7f", "#bcbd22", "#17becf", "#aec7e8", "#ffbb78"];

/// O estilo da data de origem, como estilo do QGIS (`style.importQgis`, renderizador "paletted"):
/// o valor `k` é a k-ésima data de `ordem`, com a data como rótulo; o 0 (nenhuma data limpa),
/// transparente.
pub fn qml_da_data(ordem: &[Data]) -> String {
    let mut q = String::from(r#"<qgis><pipe><rasterrenderer type="paletted" band="1" opacity="1"><colorPalette>"#);
    q.push_str(r##"<paletteEntry value="0" color="#000000" alpha="0" label="Nenhuma data limpa"/>"##);
    for (k, d) in ordem.iter().enumerate() {
        let cor = CORES_DAS_DATAS[k % CORES_DAS_DATAS.len()];
        q.push_str(&format!(r#"<paletteEntry value="{}" color="{cor}" alpha="255" label="{}"/>"#, k + 1, d.texto()));
    }
    q.push_str("</colorPalette></rasterrenderer></pipe></qgis>");
    q
}

/// Confere o período pedido: o do índice ([`crate::bdc::conferir_periodo`]) e até um ano.
pub fn conferir_periodo(s: Satelite, inicio: Data, fim: Data) -> Result<(), String> {
    crate::bdc::conferir_periodo(s, inicio, fim)?;
    if fim.dias() - inicio.dias() > MAX_DIAS {
        return Err("o período passa de 1 ano: escolha um mais curto (o clique lê a máscara de cada data de 16 dias)".into());
    }
    Ok(())
}

/// Como o comando monta a imagem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caminho {
    /// Uma leitura do `cube.reduce` por data e banda (`start` = `end`), a nuvem e o melhor pixel
    /// pela Raster Calculator do GeoLibre, a junção pelo Composite Bands. É o do RustGIS 0.7.0.
    PorData,
    /// Uma leitura do `cube.reduce` por banda, com `mask` e `stat: "first"` na ordem da parte
    /// limpa (opengeos/RustGIS#2585). Sem a dilatação da máscara nem a data de origem.
    PeloCubeReduce,
}

/// O caminho, pelos parâmetros que o `cube.reduce` deste RustGIS diz aceitar (o `params` dele no
/// `info.commands`): o da máscara só quando ele tem `mask`, `stat: "first"` e `order`.
pub fn caminho(parametros_do_cube_reduce: &str) -> Caminho {
    let tem = |p: &str| parametros_do_cube_reduce.contains(p);
    if tem("\"mask\"") && tem("\"first\"") && tem("\"order\"") { Caminho::PeloCubeReduce } else { Caminho::PorData }
}

/// Os parâmetros do `cube.reduce` de uma banda no [`Caminho::PeloCubeReduce`]: a máscara da
/// data de cada célula e a primeira data limpa, na ordem da parte limpa na área.
pub fn cube_reduce_com_mascara(s: Satelite, banda: &str, c: [f64; 4], inicio: Data, fim: Data, nome: &str) -> Option<Value> {
    let celula = celula_de_leitura(s, c);
    let m = mascara(s)?;
    let teste = if m.limpas.is_empty() { json!({"asset": m.banda, "drop": [0]}) } else { json!({"asset": m.banda, "keep": m.limpas}) };
    Some(json!({
        "url": STAC, "collection": s.colecao(), "asset": banda, "bbox": c, "start": inicio.iso(), "end": fim.iso(),
        "cellSize": celula, "mask": teste, "stat": "first", "order": "clear", "name": nome,
    }))
}

#[cfg(test)]
mod testes {
    use super::*;

    fn d(ano: i64, mes: u32, dia: u32) -> Data {
        Data { ano, mes, dia }
    }

    #[test]
    fn mascaras_por_satelite() {
        let s2 = mascara(Satelite::Sentinel2).unwrap();
        assert_eq!(s2.banda, "SCL");
        assert_eq!(s2.nuvem("S"), r#"if((("S" == 2) || (("S" >= 4) && ("S" <= 7))), 0, 1)"#);
        assert_eq!(s2.janela(), 5);
        let l = mascara(Satelite::Landsat).unwrap();
        assert_eq!((l.banda, l.limpo("C")), ("CLEAROB", r#"("C" > 0)"#.to_string()));
        assert_eq!(l.janela(), 3);
        let c = mascara(Satelite::Cbers4).unwrap();
        assert_eq!((c.banda, c.limpo("M")), ("CMASK", r#"("M" == 127)"#.to_string()));
        assert_eq!(c.nuvem("M"), r#"if(("M" == 127), 0, 1)"#);
        assert_eq!(mascara(Satelite::Modis), None);
        assert!(SATELITES.iter().all(|s| mascara(*s).is_some()));
    }

    #[test]
    fn o_scl_limpo_e_o_do_teste_de_vicosa() {
        // O mesmo teste do gerar_expressao_melhor_pixel.py: 2, ou de 4 a 7.
        let e = mascara(Satelite::Sentinel2).unwrap().limpo("SCL_0712");
        assert_eq!(e, r#"(("SCL_0712" == 2) || (("SCL_0712" >= 4) && ("SCL_0712" <= 7)))"#);
    }

    #[test]
    fn bandas_e_contraste() {
        assert_eq!(bandas(Satelite::Sentinel2, Composicao::Natural), Some(["B04", "B03", "B02"]));
        assert_eq!(bandas(Satelite::Sentinel2, Composicao::FalsaCor), Some(["B08", "B04", "B03"]));
        assert_eq!(bandas(Satelite::Landsat, Composicao::FalsaCor), Some(["nir08", "red", "green"]));
        assert_eq!(bandas(Satelite::Cbers4, Composicao::Natural), Some(["BAND15", "BAND14", "BAND13"]));
        assert_eq!(bandas(Satelite::Modis, Composicao::Natural), None);
        assert_eq!(contraste(Satelite::Sentinel2, Composicao::Natural), ([0.01, 0.02, 0.01], [0.16, 0.13, 0.10]));
        assert_eq!(contraste(Satelite::Sentinel2, Composicao::FalsaCor), ([0.05, 0.01, 0.02], [0.45, 0.16, 0.13]));
        assert_eq!(contraste(Satelite::Cbers4, Composicao::FalsaCor).1, [0.45, 0.13, 0.11]);
        assert_eq!(Composicao::ler("Falsa-cor"), Some(Composicao::FalsaCor));
        assert_eq!(Composicao::ler("natural"), Some(Composicao::Natural));
        assert_eq!(Composicao::ler("pancromática"), None);
    }

    #[test]
    fn a_ordem_e_a_parte_limpa() {
        let datas = [
            DataNaArea { data: d(2025, 8, 13), nuvem: 0.008 },
            DataNaArea { data: d(2025, 8, 29), nuvem: 0.305 },
            DataNaArea { data: d(2025, 7, 12), nuvem: 0.0 },
            DataNaArea { data: d(2025, 6, 26), nuvem: 0.0 },
            DataNaArea { data: d(2025, 6, 10), nuvem: 1.0 },
            DataNaArea { data: d(2025, 7, 28), nuvem: f64::NAN },
        ];
        let o: Vec<Data> = ordenar(&datas).into_iter().map(|x| x.data).collect();
        // A menos nublada primeiro; no empate, a mais recente; as de toda nublada e sem medida, fora.
        assert_eq!(o, vec![d(2025, 7, 12), d(2025, 6, 26), d(2025, 8, 13), d(2025, 8, 29)]);
    }

    #[test]
    fn o_melhor_pixel_e_as_entradas() {
        let e = melhor_pixel(&[("N1".to_string(), Valor::Camada("B1".into())), ("N2".to_string(), Valor::Camada("B2".into()))]);
        assert_eq!(e, r#"if(("N1" == 0), "B1", if(("N2" == 0), "B2", 0))"#);
        assert_eq!(entradas_da_expressao(&e), vec!["N1", "B1", "N2", "B2"]);
        let datas = melhor_pixel(&[("N1".to_string(), Valor::Numero(1)), ("N2".to_string(), Valor::Numero(2))]);
        assert_eq!(datas, r#"if(("N1" == 0), 1, if(("N2" == 0), 2, 0))"#);
        assert_eq!(entradas_da_expressao(&datas), vec!["N1", "N2"]);
        assert_eq!(melhor_pixel(&[]), "0");
        // Nomes com espaços e dois-pontos, como os das camadas do plugin.
        let n = temporaria(Satelite::Sentinel2, "SCL", d(2025, 7, 12));
        assert_eq!(n, "BrasilData: sem nuvens SCL 2025-07-12 (Sentinel-2)");
        let e = mascara(Satelite::Sentinel2).unwrap().nuvem(&n);
        assert_eq!(entradas_da_expressao(&e), vec![n]);
    }

    #[test]
    fn datas_dos_itens_do_stac() {
        let r = json!({"features": [
            {"properties": {"datetime": "2025-05-25T00:00:00Z"}},
            {"properties": {"start_datetime": "2025-06-10T00:00:00Z", "datetime": "2025-06-10T00:00:00Z"}},
            {"properties": {"datetime": "2025-06-10T00:00:00Z"}},
            {"properties": {"datetime": "2025-08-29T00:00:00Z"}},
            {"properties": {"datetime": "2025-07-12T00:00:00Z"}},
            {"properties": {}},
        ]});
        let v = datas_dos_itens(&r, d(2025, 6, 1), d(2025, 8, 31));
        assert_eq!(v, vec![d(2025, 6, 10), d(2025, 7, 12), d(2025, 8, 29)]);
        let u = url_dos_itens(Satelite::Sentinel2, [-43.0, -20.8, -42.7, -20.6], d(2025, 6, 1), d(2025, 8, 31));
        assert!(u.starts_with("https://data.inpe.br/bdc/stac/v1/collections/S2-16D-2/items?bbox=-43,-20.8,-42.7,-20.6"));
        assert!(u.contains("datetime=2025-06-01T00:00:00Z/2025-08-31T23:59:59Z"));
    }

    #[test]
    fn os_nomes_das_camadas_dizem_o_satelite() {
        let (a, b) = (d(2025, 6, 1), d(2025, 8, 31));
        let n = camada(Satelite::Sentinel2, Composicao::Natural, a, b);
        assert_eq!(n, "BrasilData: Imagem sem nuvens em cor verdadeira de 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)");
        // A citação da coleção vem pelo nome, como no índice.
        assert_eq!(crate::bdc::satelite_da_camada(&n), Some(Satelite::Sentinel2));
        assert!(!crate::bdc::eh_serie(&n));
        let dt = camada_da_data(Satelite::Cbers4, a, b);
        assert_eq!(dt, "BrasilData: Data de origem da imagem sem nuvens de 01/06/2025 a 31/08/2025 (CBERS-4 WFI, cubo do INPE)");
        assert_eq!(crate::bdc::satelite_da_camada(&dt), Some(Satelite::Cbers4));
        assert!(eh_sem_nuvens(&n) && eh_sem_nuvens(&dt));
        assert!(!eh_sem_nuvens(&crate::bdc::camada(Satelite::Sentinel2, crate::bdc::Indice::Ndvi, crate::bdc::Estatistica::Media, a, b)));
        let f = camada(Satelite::Landsat, Composicao::FalsaCor, a, b);
        assert!(f.contains("em falsa-cor de") && crate::bdc::satelite_da_camada(&f) == Some(Satelite::Landsat));
    }

    #[test]
    fn o_estilo_da_data_de_origem() {
        let q = qml_da_data(&[d(2025, 7, 12), d(2025, 8, 13)]);
        assert_eq!(q.matches("<paletteEntry").count(), 3);
        assert!(q.contains(r##"value="1" color="#1f77b4" alpha="255" label="12/07/2025""##));
        assert!(q.contains(r#"value="2""#) && q.contains(r#"label="13/08/2025""#));
        assert!(q.contains(r##"value="0" color="#000000" alpha="0""##));
    }

    #[test]
    fn a_celula_cresce_com_a_area() {
        // Viçosa-MG: cerca de 26 × 20 km, a 10 m (5,7 milhões de células).
        assert_eq!(celula_de_leitura(Satelite::Sentinel2, [-43.003, -20.838, -42.754, -20.655]), 10.0);
        assert_eq!(celula_de_leitura(Satelite::Cbers4, [-43.003, -20.838, -42.754, -20.655]), 64.0);
        // Um quadrado de 1° (cerca de 104 × 111 km): mais de 10 m.
        let c = celula_de_leitura(Satelite::Sentinel2, [-44.0, -21.0, -43.0, -20.0]);
        assert!((30.0..35.0).contains(&c), "{c}");
    }

    #[test]
    fn periodo() {
        assert!(conferir_periodo(Satelite::Sentinel2, d(2025, 6, 1), d(2025, 8, 31)).is_ok());
        assert!(conferir_periodo(Satelite::Sentinel2, d(2025, 6, 1), d(2025, 6, 10)).is_err());
        assert!(conferir_periodo(Satelite::Sentinel2, d(2024, 6, 1), d(2025, 8, 31)).is_err());
        assert!(conferir_periodo(Satelite::Cbers4, d(2015, 6, 1), d(2015, 8, 31)).is_err());
    }

    #[test]
    fn o_caminho_segue_o_cube_reduce() {
        let hoje = r#"{"cube"?: …, "stat": "mean"|"min"|"max", "start"?, "end"?, "name"?: text}"#;
        assert_eq!(caminho(hoje), Caminho::PorData);
        let depois = r#"{"mask"?: {"asset", "keep"}, "stat": "mean"|"first"|"median", "order"?: "clear"}"#;
        assert_eq!(caminho(depois), Caminho::PeloCubeReduce);
        let p =
            cube_reduce_com_mascara(Satelite::Sentinel2, "B04", [-43.0, -20.8, -42.7, -20.6], d(2025, 6, 1), d(2025, 8, 31), "x").unwrap();
        assert_eq!(p["mask"], json!({"asset": "SCL", "keep": [2, 4, 5, 6, 7]}));
        assert_eq!((p["stat"].as_str(), p["order"].as_str(), p["cellSize"].as_f64()), (Some("first"), Some("clear"), Some(10.0)));
        let l = cube_reduce_com_mascara(Satelite::Landsat, "red", [0.0; 4], d(2025, 6, 1), d(2025, 8, 31), "x").unwrap();
        assert_eq!(l["mask"], json!({"asset": "CLEAROB", "drop": [0]}));
        assert!(cube_reduce_com_mascara(Satelite::Modis, "NDVI", [0.0; 4], d(2025, 6, 1), d(2025, 8, 31), "x").is_none());
    }
}
