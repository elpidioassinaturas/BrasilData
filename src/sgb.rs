//! As camadas do Serviço Geológico do Brasil (SGB), do GeoServer do GeoSGB
//! (`geoservicos.sgb.gov.br/geoserver`), e as cores oficiais delas.
//!
//! - **A geologia** (litoestratigrafia) vem em seis escalas, cada uma um tipo do WFS: o mapa
//!   1:1.000.000 do Brasil, os mapas dos estados e as folhas 1:250.000, 1:100.000, 1:50.000 e
//!   1:25.000, que só existem onde houve mapeamento. O comando `geologia-sgb` escolhe uma.
//! - **As cores** são as dos estilos (SLD) do GeoServer: uma regra por sigla, até 2.873 numa escala.
//!   O `scripts/baixar_sgb_cores.py` as lê e escreve o `sgb_cores.csv`, que vem com o plugin:
//!   pedir um SLD de 2 MB a cada mapa seria lento e passaria perto do limite do `host.fetch`.
//! - **As outras camadas** (pontos, geodiversidade, riscos, suscetibilidade, SIAGAS) estão no
//!   `fontes.csv`; daqui vêm só os estilos por classe das que o SGB pinta por classe.
//! - **Conferido em 26/11/2026:** o WFS responde com `Access-Control-Allow-Origin: *`; a escala
//!   1:50.000 traz os campos em maiúsculas (`NOME`, `HIERARQUIA`) e a geometria em `GEOMETRY`.

use crate::Tema;
use serde_json::{Value, json};
use std::collections::{BTreeMap, HashMap};

/// O WFS do SGB.
pub const WFS: &str = "https://geoservicos.sgb.gov.br/geoserver/ows";

/// As cores: `chave,valor,cor,contorno`, a chave a de [`Escala::chave`] ou a de um tema.
const CORES: &str = include_str!("../sgb_cores.csv");

const LIVRE: &str = "dados abertos do SGB; licença não declarada no serviço";

/// Os campos da geologia que ficam, nas escalas com os nomes em minúsculas.
const CAMPOS: &str =
    "sigla,nome,hierarquia,litotipos,legenda,idade_min,idade_max,eon_max,era_max,sistema_max,ambiente_tectonico,escala,mapa";
/// Os mesmos, na escala 1:50.000, que os tem em maiúsculas.
const CAMPOS_50K: &str =
    "sigla,NOME,HIERARQUIA,LITOTIPOS,LEGENDA,idade_min,idade_max,EON_MAX,ERA_MAX,SISTEMA_MAX,AMBIENTE_TECTONICO,ESCALA,MAPA";

/// Uma escala da geologia do SGB.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Escala {
    Milhao,
    Estados,
    E250k,
    E100k,
    E50k,
    E25k,
}

/// Todas, da mais geral para a mais detalhada.
pub const ESCALAS: [Escala; 6] = [Escala::Milhao, Escala::Estados, Escala::E250k, Escala::E100k, Escala::E50k, Escala::E25k];

const fn tema(
    id: &'static str,
    nome: &'static str,
    camada: &'static str,
    fonte: &'static str,
    campos: &'static str,
    geom: &'static str,
) -> Tema {
    Tema {
        id,
        grupo: "Meio físico",
        nome,
        servico: WFS,
        tipo: "wfs",
        camada,
        geometria: "poligono",
        recorta: true,
        fonte,
        licenca: LIVRE,
        filtro: "",
        campos,
        geom,
        pagina: 2000,
        ordem: "",
    }
}

/// Os temas das escalas, na ordem de [`ESCALAS`].
static TEMAS: [Tema; 6] = [
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:1.000.000",
        "geosgb:litoestratigrafia_1m",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia 1:1.000.000 (Mapa Geológico do Brasil, Carta Geológica ao Milionésimo)",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, mapa do estado",
        "geosgb:litoestratigrafia_estados",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia dos mapas geológicos estaduais",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:250.000",
        "geosgb:litoestratigrafia_250k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:250.000",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:100.000",
        "geosgb:litoestratigrafia_100k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:100.000",
        CAMPOS,
        "geom",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:50.000",
        "geosgb:litoestratigrafia_50k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:50.000",
        CAMPOS_50K,
        "GEOMETRY",
    ),
    tema(
        "geologia-sgb",
        "Geologia do SGB, 1:25.000",
        "geosgb:litoestratigrafia_25k",
        "SGB (Serviço Geológico do Brasil), GeoSGB, Litoestratigrafia das cartas geológicas 1:25.000",
        CAMPOS,
        "geometry",
    ),
];

impl Escala {
    pub fn ler(t: Option<&str>) -> Result<Escala, String> {
        let t = t.map(|t| t.trim().to_lowercase().replace(['.', ' '], ""));
        Ok(match t.as_deref() {
            None | Some("") | Some("1:1000000") | Some("1m") | Some("milionesimo") => Escala::Milhao,
            Some("estado") | Some("estados") | Some("estadual") => Escala::Estados,
            Some("1:250000") | Some("250k") => Escala::E250k,
            Some("1:100000") | Some("100k") => Escala::E100k,
            Some("1:50000") | Some("50k") => Escala::E50k,
            Some("1:25000") | Some("25k") => Escala::E25k,
            Some(o) => return Err(format!("escala '{o}': use 1:1.000.000, estado, 1:250.000, 1:100.000, 1:50.000 ou 1:25.000")),
        })
    }

    fn indice(self) -> usize {
        ESCALAS.iter().position(|e| *e == self).unwrap_or(0)
    }

    /// O tema do WFS da escala.
    pub fn tema(self) -> &'static Tema {
        &TEMAS[self.indice()]
    }

    /// A chave das cores no `sgb_cores.csv`.
    pub fn chave(self) -> &'static str {
        ["1m", "estados", "250k", "100k", "50k", "25k"][self.indice()]
    }

    /// Como o aviso a chama.
    pub fn texto(self) -> &'static str {
        ["1:1.000.000", "mapa do estado", "1:250.000", "1:100.000", "1:50.000", "1:25.000"][self.indice()]
    }

    /// O campo do nome da unidade.
    pub fn campo_nome(self) -> &'static str {
        if self == Escala::E50k { "NOME" } else { "nome" }
    }
}

/// A escala da camada `nome` do mapa, se ela é da geologia do SGB.
pub fn escala_da_camada(nome: &str) -> Option<Escala> {
    ESCALAS.into_iter().find(|e| e.tema().camada_no_mapa() == nome)
}

/// A cor de `valor` na chave `chave`, e se ela é só do contorno.
pub fn cor(chave: &str, valor: &str) -> Option<([u8; 3], bool)> {
    CORES.lines().skip(1).find_map(|l| {
        let mut p = l.split(',');
        if p.next()? != chave || p.next()? != valor {
            return None;
        }
        Some((hex(p.next()?)?, p.next() == Some("1")))
    })
}

fn hex(t: &str) -> Option<[u8; 3]> {
    let t = t.strip_prefix('#')?;
    if t.len() != 6 {
        return None;
    }
    let b = |i: usize| u8::from_str_radix(&t[i..i + 2], 16).ok();
    Some([b(0)?, b(2)?, b(4)?])
}

/// As cores da chave `chave`, lidas de uma vez (a busca de [`cor`] percorre a tabela inteira:
/// para as centenas de siglas de um estado, passaria da cota de instruções do plugin).
pub fn cores(chave: &str) -> HashMap<&'static str, [u8; 3]> {
    CORES
        .lines()
        .skip(1)
        .filter_map(|l| {
            let mut p = l.split(',');
            (p.next()? == chave).then_some(())?;
            Some((p.next()?, hex(p.next()?)?))
        })
        .collect()
}

/// As cores da geologia na escala `e`. As cartas (1:1.000.000 a 1:25.000) usam a mesma cor para
/// a mesma sigla (em 26/11/2026, 717 de 721 siglas comuns ao 1:1.000.000 e ao 1:100.000), e o SLD
/// de uma escala não tem todas as siglas dela (93 das de Minas Gerais faltam no do 1:1.000.000):
/// a que falta vem da tabela de outra escala das cartas. Os mapas dos estados têm outra paleta
/// (354 de 1.232 iguais) e ficam só com as próprias cores.
pub fn cores_da_escala(e: Escala) -> HashMap<&'static str, [u8; 3]> {
    let mut t = cores(e.chave());
    if e != Escala::Estados {
        for outra in [Escala::E250k, Escala::E100k, Escala::Milhao, Escala::E50k, Escala::E25k] {
            if outra != e {
                for (s, c) in cores(outra.chave()) {
                    t.entry(s).or_insert(c);
                }
            }
        }
    }
    t
}

/// O nome do estilo da geologia de uma escala.
pub fn nome_do_estilo(e: Escala) -> String {
    format!("BrasilData: geologia do SGB, {}", e.texto())
}

/// O estilo por valores únicos da sigla, com as unidades `unidades` (sigla → nome) nas cores do
/// SGB e o rótulo "sigla – nome" na legenda; e quantas não têm cor na tabela (ficam cinza).
pub fn estilo_da_geologia(e: Escala, unidades: &BTreeMap<String, String>) -> (String, usize) {
    let tabela = cores_da_escala(e);
    let mut sem_cor = 0;
    let mut classes = Vec::new();
    let mut rotulos = serde_json::Map::new();
    for (sigla, nome) in unidades {
        let rgb = match tabela.get(sigla.as_str()) {
            Some(c) => *c,
            None => {
                sem_cor += 1;
                [190, 190, 190]
            }
        };
        classes.push(json!([sigla, poligono(rgb, 230)]));
        let rotulo = if nome.trim().is_empty() { sigla.clone() } else { format!("{sigla} – {}", nome.trim()) };
        rotulos.insert(sigla.clone(), Value::String(rotulo));
    }
    let texto = json!([{"name": nome_do_estilo(e), "geometry": "Polygon",
        "renderer": {"Unique": {"field": "sigla", "classes": classes, "other": poligono([190, 190, 190], 200), "ramp": "Harbour", "class_labels": rotulos}},
        "labels": {"enabled": true, "field": "sigla", "size": 7.0, "color": [30, 30, 30, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    (texto, sem_cor)
}

fn poligono(rgb: [u8; 3], alfa: u8) -> Value {
    json!({"fill": [rgb[0], rgb[1], rgb[2], alfa], "stroke": [90, 90, 90, 200], "stroke_width": 0.4, "size": 0.0, "marker": "Circle"})
}

/// O pedido da sigla e do nome das unidades de `e` que cruzam `c`, sem a geometria, em CSV (FID,
/// sigla, nome; 690 kB para 10 mil polígonos, mais leve de ler que o GeoJSON): a página `(início,
/// tamanho)`. O plugin lê só isso para montar a legenda.
pub fn pedido_das_unidades(e: Escala, c: [f64; 4], pagina: (u64, u64)) -> String {
    let t = e.tema();
    format!(
        "{}?service=WFS&version=2.0.0&request=GetFeature&typeNames={}&bbox={:.6},{:.6},{:.6},{:.6},EPSG:4674&outputFormat=csv&propertyName=sigla,{}&count={}&startIndex={}",
        t.servico,
        t.camada,
        c[0],
        c[1],
        c[2],
        c[3],
        e.campo_nome(),
        pagina.1,
        pagina.0
    )
}

/// Junta a `unidades` (sigla → nome, em ordem de sigla) as de uma resposta em CSV a
/// [`pedido_das_unidades`]: `FID,sigla,nome`, o nome entre aspas quando tem vírgula. Uma sigla sem
/// texto fica de fora.
pub fn unidades_csv(texto: &str, unidades: &mut BTreeMap<String, String>) {
    for l in texto.lines().skip(1) {
        let mut p = l.splitn(3, ',');
        let (Some(_), Some(sigla)) = (p.next(), p.next()) else { continue };
        let sigla = sigla.trim().trim_matches('"');
        if sigla.is_empty() || unidades.contains_key(sigla) {
            continue;
        }
        let nome = p.next().unwrap_or("").trim();
        let nome = match nome.strip_prefix('"').and_then(|n| n.strip_suffix('"')) {
            Some(n) => n.replace("\"\"", "\""),
            None => nome.to_string(),
        };
        unidades.insert(sigla.to_string(), nome);
    }
}

/// O filtro de uma sigla (para contar as feições dela na camada recortada).
pub fn onde_sigla(sigla: &str) -> String {
    format!("\"sigla\" = '{}'", sigla.replace('\'', "''"))
}

/// O estilo por classe de um tema do `fontes.csv` que o SGB pinta por classe: o nome, o texto.
pub fn estilo_do_tema(id: &str) -> Option<(String, String)> {
    let (chave, campo, valores): (&str, &str, &[&str]) = match id {
        "sgb-suscet-inundacao" => ("inundacao", "classe", &["Alta", "Média", "Baixa"]),
        "sgb-suscet-movimento" => ("movimento", "classe", &["Alta", "Média", "Baixa"]),
        "sgb-risco" => ("risco", "grau_risco", &["Muito alto", "Alto", "Médio"]),
        _ => return None,
    };
    let nome = format!("BrasilData: {id} (cores do SGB)");
    let classes: Vec<Value> = valores
        .iter()
        .filter_map(|v| {
            let (c, contorno) = cor(chave, v)?;
            let simbolo = if contorno {
                json!({"fill": [c[0], c[1], c[2], 60], "stroke": [c[0], c[1], c[2], 255], "stroke_width": 1.6, "size": 0.0, "marker": "Circle"})
            } else {
                json!({"fill": [c[0], c[1], c[2], 200], "stroke": [110, 110, 110, 160], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"})
            };
            Some(json!([v, simbolo]))
        })
        .collect();
    let texto = json!([{"name": nome, "geometry": "Polygon",
        "renderer": {"Unique": {"field": campo, "classes": classes, "other": poligono([190, 190, 190], 150), "ramp": "Harbour"}},
        // O RustGIS recusa um estilo sem `labels` ("missing field `labels`").
        "labels": {"enabled": false, "field": campo, "size": 8.0, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    Some((nome, texto))
}

/// O teto de feições de um tema do SGB por área, quando é menor que o do catálogo: os polígonos
/// da suscetibilidade são muitos e pequenos (mais de 4.800 só no retângulo de Viçosa).
pub fn teto(id: &str) -> Option<u64> {
    match id {
        "sgb-suscet-movimento" | "sgb-suscet-inundacao" => Some(20_000),
        // Os polígonos da geologia de um estado inteiro: dezenas de megabytes.
        "geologia-sgb" => Some(30_000),
        _ => None,
    }
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn as_cores_do_sld() {
        // Conferidas no SLD de 26/11/2026.
        assert_eq!(cor("1m", "A23br"), Some(([0x48, 0x63, 0x21], false)));
        assert_eq!(cor("movimento", "Alta"), Some(([0xc4, 0x6d, 0x1b], false)));
        assert_eq!(cor("risco", "Muito alto"), Some(([0xe9, 0x07, 0x00], true)));
        assert_eq!(cor("1m", "nao-existe"), None);
        assert_eq!(cor("250k", "A23br").is_some(), CORES.lines().any(|l| l.starts_with("250k,A23br,")));
        for e in ESCALAS {
            let n = CORES.lines().filter(|l| l.starts_with(&format!("{},", e.chave()))).count();
            assert!(n > 100, "{} só tem {n} cores", e.chave());
        }
        assert!(CORES.lines().skip(1).all(|l| l.split(',').count() == 4 && hex(l.split(',').nth(2).unwrap()).is_some()));
    }

    #[test]
    fn as_escalas() {
        assert_eq!(Escala::ler(None), Ok(Escala::Milhao));
        assert_eq!(Escala::ler(Some("1:100.000")), Ok(Escala::E100k));
        assert_eq!(Escala::ler(Some("Estado")), Ok(Escala::Estados));
        assert!(Escala::ler(Some("1:150.000")).is_err());
        assert_eq!(Escala::E50k.tema().geom, "GEOMETRY");
        assert!(Escala::E50k.tema().campos.contains("NOME"));
        assert_eq!(escala_da_camada("BrasilData: Geologia do SGB, 1:250.000"), Some(Escala::E250k));
        assert_eq!(escala_da_camada("BrasilData: Geologia (IBGE)"), None);
        assert!(ESCALAS.iter().all(|e| e.tema().pedido_proprio()));
    }

    #[test]
    fn o_estilo_so_com_as_unidades_da_area() {
        let csv = "FID,sigla,nome\nl.1,NP3_gamma,Suíte X\nl.2,A23br,Complexo Barro\nl.3,A23br,Complexo Barro\nl.4,,sem sigla\nl.5,Q2a,\"Depósitos, aluvionares \"\"novos\"\"\"\n";
        let mut u = BTreeMap::new();
        unidades_csv(csv, &mut u);
        assert_eq!(u.keys().collect::<Vec<_>>(), ["A23br", "NP3_gamma", "Q2a"]);
        assert_eq!(u["Q2a"], "Depósitos, aluvionares \"novos\"");
        assert_eq!(u["A23br"], "Complexo Barro");
        u.remove("Q2a");
        let (texto, sem_cor) = estilo_da_geologia(Escala::Milhao, &u);
        let s: Value = serde_json::from_str(&texto).unwrap();
        let un = &s[0]["renderer"]["Unique"];
        assert_eq!(un["classes"].as_array().unwrap().len(), 2);
        assert_eq!(un["classes"][0][1]["fill"], json!([0x48, 0x63, 0x21, 230]));
        assert_eq!(un["class_labels"]["A23br"], "A23br – Complexo Barro");
        assert_eq!(sem_cor, usize::from(cor("1m", "NP3_gamma").is_none()));
        assert_eq!(cores("1m").get("A23br"), Some(&[0x48, 0x63, 0x21]));
        assert_eq!(cores("1m").len(), CORES.lines().filter(|l| l.starts_with("1m,")).count());
        // Uma sigla que falta no SLD do 1:1.000.000 vem da tabela de outra carta, nunca da dos estados.
        let falta = cores("100k").into_keys().find(|s| !cores("1m").contains_key(s)).unwrap();
        assert_eq!(cores_da_escala(Escala::Milhao).get(falta), cores("100k").get(falta));
        assert_eq!(cores_da_escala(Escala::Estados).len(), cores("estados").len());
    }

    #[test]
    fn os_pedidos() {
        let u = pedido_das_unidades(Escala::E50k, [-43.0, -20.9, -42.75, -20.6], (2000, 1000));
        assert!(
            u.contains("typeNames=geosgb:litoestratigrafia_50k") && u.contains("propertyName=sigla,NOME") && u.contains("outputFormat=csv")
        );
        assert!(u.contains("&count=1000&startIndex=2000") && u.contains("bbox=-43.000000,-20.900000,-42.750000,-20.600000,EPSG:4674"));
        assert_eq!(onde_sigla("PP4_gamma'b"), "\"sigla\" = 'PP4_gamma''b'");
    }

    #[test]
    fn os_estilos_dos_temas() {
        let (_, t) = estilo_do_tema("sgb-suscet-inundacao").unwrap();
        let s: Value = serde_json::from_str(&t).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), 3);
        let (_, r) = estilo_do_tema("sgb-risco").unwrap();
        let s: Value = serde_json::from_str(&r).unwrap();
        assert_eq!(s[0]["renderer"]["Unique"]["field"], "grau_risco");
        assert_eq!(s[0]["renderer"]["Unique"]["classes"][0][1]["stroke_width"], 1.6);
        assert_eq!(s[0]["labels"]["enabled"], false, "o RustGIS recusa um estilo sem labels");
        assert!(estilo_do_tema("sgb-afloramentos").is_none());
        assert_eq!(teto("sgb-suscet-movimento"), Some(20_000));
        assert_eq!(teto("sgb-risco"), None);
    }
}
