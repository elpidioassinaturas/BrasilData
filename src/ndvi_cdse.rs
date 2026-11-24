//! O NDVI de 10 em 10 dias a 300 m do serviço de terra do Copernicus (CLMS, NDVI 300 m v3, do
//! Sentinel-3 OLCI desde janeiro de 2019 e do PROBA-V antes), lido do Copernicus Data Space (CDSE).
//!
//! - **De onde:** o STAC do CDSE (aberto) lista os arquivos; a janela da área é lida do COG global em
//!   `download.dataspace.copernicus.eu`, com o login que o RustGIS guarda (`copernicus.login`): o
//!   plugin nunca vê a senha nem o token.
//! - **Os valores:** `uint8` de 0 a 250, NDVI = valor × 0,004 − 0,08 (−0,08 a 0,92); de 251 a 255 são
//!   marcas (sem dado, água), tiradas antes da reamostragem.
//! - **As visões reduzidas:** de 4, 8, 16 e 32 células (não há a de 2), só para as áreas em que o arquivo
//!   inteiro não cabe numa leitura (mais de uns 23°): cada célula delas é a do canto noroeste do bloco,
//!   sem média (conferido célula a célula), e [`posta_no_lugar`] desfaz o deslocamento que isso dá.
//! - **O servidor:** corta o começo da conexão em parte dos pedidos (medido em 24/11/2026: perto da
//!   metade) e recusa (429) o que passa de quatro pedidos ao mesmo tempo, que é a cota de um usuário
//!   (o RustGIS faz até 16). Um novo pedido passa logo depois. O RustGIS guarda por 60 s a falha do
//!   cabeçalho de um endereço, e a nova tentativa vai com outro ([`endereco`]); os blocos que já vieram
//!   de um endereço ficam, e os que falharam se pedem de novo no mesmo.
//! - **Conferido em 24/11/2026:** em Viçosa, as 332.041 células da grade são as do recorte do mesmo
//!   arquivo feito em 13/11, sem diferença.

use serde_json::Value;

use crate::PREFIXO;
use crate::cog_global::Arquivo;
use crate::datas::Data;

const BUSCA: &str = "https://stac.dataspace.copernicus.eu/v1/search?collections=clms_ndvi_global_300m_10daily_v3_cog";

/// Onde ficam os arquivos: o único servidor (com o `zipper`) a que o RustGIS manda o login.
const DADOS: &str = "https://download.dataspace.copernicus.eu/odata/v1/";

/// A conta que leva o valor do arquivo, somado de 1 (o 0 é um valor) e com as marcas em 0, ao NDVI:
/// `b1 / b1` dá sem dado no 0.
pub const CONTA: &str = "(b1 / b1) * ((b1 - 1) * 0.004 - 0.08)";

/// O primeiro dia da série.
pub const PRIMEIRO: Data = Data { ano: 2014, mes: 1, dia: 1 };

/// O primeiro dia do Sentinel-3 (o nome dos arquivos diz); antes é o PROBA-V.
const PRIMEIRO_DO_SENTINEL3: Data = Data { ano: 2019, mes: 1, dia: 1 };

/// Quantos endereços se tentam até o cabeçalho vir.
pub const TENTATIVAS: i32 = 10;

/// Quantas vezes a janela é pedida no endereço que respondeu.
pub const TENTATIVAS_DA_JANELA: i32 = 16;

/// Os níveis de pirâmide que o arquivo tem (o 0 é ele inteiro).
const NIVEIS: [i32; 5] = [0, 2, 3, 4, 5];

pub const LICENCA: &str = "política de dados do Copernicus: uso livre e gratuito, com a citação";

pub const CITACAO: &str = "Copernicus Land Monitoring Service, Normalised Difference Vegetation Index 300 m, 10-daily, version 3 (União Europeia), pelo Copernicus Data Space Ecosystem; marcas de água e sem dado tiradas, recortado pela área e posto na grade do projeto";

/// O produto de 10 dias: o primeiro e o último dia, e o endereço do COG do NDVI.
#[derive(Clone, Debug, PartialEq)]
pub struct Produto {
    pub inicio: Data,
    pub fim: Data,
    pub url: String,
}

/// A busca do produto mais recente que começa até `dia` (sem dia, o mais recente).
pub fn url_busca(dia: Option<Data>) -> String {
    let quando = dia.map(|d| format!("&datetime={}T00:00:00Z/{}T00:00:00Z", PRIMEIRO.iso(), d.iso())).unwrap_or_default();
    format!(
        "{BUSCA}&limit=1&sortby=-datetime{quando}&fields=id,properties.start_datetime,properties.end_datetime,assets.ndvi.alternate.https.href,-links"
    )
}

/// O produto da busca, o primeiro. Só vale um endereço do servidor de dados do CDSE.
pub fn produto(r: &Value) -> Option<Produto> {
    let f = r["features"].as_array()?.first()?;
    let dia = |k: &str| crate::bdc::ler_data(f["properties"][k].as_str()?.get(..10)?);
    let url = f["assets"]["ndvi"]["alternate"]["https"]["href"].as_str().filter(|h| h.starts_with(DADOS) && !h.contains('?'))?;
    Some(Produto { inicio: dia("start_datetime")?, fim: dia("end_datetime")?, url: url.to_string() })
}

/// O endereço da tentativa `i`, de 1 em diante: o servidor ignora o que vem depois do `?`.
pub fn endereco(url: &str, i: i32) -> String {
    if i <= 1 { url.to_string() } else { format!("{url}?tentativa={i}") }
}

/// Se um erro é o de uma conexão que caiu ou de um servidor que falhou de passagem, que vale tentar
/// de novo.
pub fn conexao_caiu(erro: &str) -> bool {
    crate::falha_passageira(erro) || [": io: ", "10054", "reset", "disconnected", "timeout", "status: 429"].iter().any(|s| erro.contains(s))
}

/// O COG: 1/336° (cerca de 300 m) por célula.
pub fn arquivo(url: String) -> Arquivo {
    Arquivo { url, graus: 1.0 / 336.0, piramides: 5 }
}

/// A caixa de uma célula do nível `k`, no meio de `c`.
pub fn uma_celula(a: &Arquivo, c: [f64; 4], k: i32) -> [f64; 4] {
    let (x, y, m) = ((c[0] + c[2]) / 2.0, (c[1] + c[3]) / 2.0, a.graus_do_nivel(k) / 4.0);
    [x - m, y - m, x + m, y + m]
}

/// O nível de pirâmide a ler: o mais fino que cabe numa leitura, seja qual for a grade (as visões
/// reduzidas não são médias).
pub fn nivel(a: &Arquivo, c: [f64; 4]) -> Option<i32> {
    NIVEIS.into_iter().find(|&k| a.nivel_que_cabe(c, k) == Some(k))
}

/// A extensão `e` (em graus) de uma janela do nível `k`, com cada célula em volta da do arquivo que ela
/// guarda, a do canto noroeste do bloco: o centro dela, e não o do bloco.
pub fn posta_no_lugar(a: &Arquivo, e: [f64; 4], k: i32) -> [f64; 4] {
    let d = (a.graus_do_nivel(k) - a.graus) / 2.0;
    [e[0] - d, e[1] + d, e[2] - d, e[3] + d]
}

const MEIO: &str = "NDVI de ";
const FIM: &str = " (Copernicus 300 m)";

/// "BrasilData: NDVI de 11/10/2026 a 20/10/2026 (Copernicus 300 m)".
pub fn camada(inicio: Data, fim: Data) -> String {
    format!("{PREFIXO}{MEIO}{} a {}{FIM}", inicio.texto(), fim.texto())
}

/// O primeiro e o último dia de uma camada desta fonte, pelo nome.
pub fn da_camada(nome: &str) -> Option<(Data, Data)> {
    let (a, b) = nome.strip_prefix(PREFIXO)?.strip_prefix(MEIO)?.strip_suffix(FIM)?.split_once(" a ")?;
    Some((Data::ler(a)?, Data::ler(b)?))
}

/// O satélite de um produto que começa em `inicio`.
pub fn satelite(inicio: Data) -> &'static str {
    if inicio < PRIMEIRO_DO_SENTINEL3 { "PROBA-V" } else { "Sentinel-3" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn busca_e_produto() {
        assert!(url_busca(None).ends_with("-links") && !url_busca(None).contains("&datetime="));
        assert!(url_busca(Data::ler("25/03/2024")).contains("&datetime=2014-01-01T00:00:00Z/2024-03-25T00:00:00Z"));
        let r = |h: &str| {
            json!({"features": [{"properties": {"start_datetime": "2024-03-21T00:00:00.000000Z", "end_datetime": "2024-03-31T23:59:59Z"},
                "assets": {"ndvi": {"alternate": {"https": {"href": h}}}}}]})
        };
        let h = format!("{DADOS}Products(x)/Nodes(y.tiff)/$value");
        let (a, b) = (Data::ler("21/03/2024").unwrap(), Data::ler("31/03/2024").unwrap());
        assert_eq!(produto(&r(&h)), Some(Produto { inicio: a, fim: b, url: h.clone() }));
        assert!(produto(&r("https://outro.example/x.tiff")).is_none() && produto(&r(&format!("{h}?x=1"))).is_none());
        assert!(produto(&json!({"features": []})).is_none());
        assert_eq!((endereco(&h, 1), endereco(&h, 3)), (h.clone(), format!("{h}?tentativa=3")));
        assert_eq!(camada(a, b), "BrasilData: NDVI de 21/03/2024 a 31/03/2024 (Copernicus 300 m)");
        assert_eq!(da_camada(&camada(a, b)), Some((a, b)));
        assert_eq!(da_camada("BrasilData: NDVI médio de 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)"), None);
        assert_eq!((satelite(a), satelite(Data::ler("21/12/2018").unwrap())), ("Sentinel-3", "PROBA-V"));
    }

    #[test]
    fn as_quedas() {
        for e in [
            "x/$value: io: Foi forçado o cancelamento (os error 10054)",
            "x/$value[query omitted]: Peer disconnected",
            "x: timeout: global",
            "x: http status: 503",
            "x: http status: 429",
        ] {
            assert!(conexao_caiu(e), "{e}");
        }
        for e in ["x: http status: 401", "the box is outside the file", "x: http status: 404"] {
            assert!(!conexao_caiu(e), "{e}");
        }
    }

    #[test]
    fn o_nivel_lido() {
        let a = arquivo(String::new());
        let (vicosa, mg, brasil) = ([-43.003, -20.838, -42.754, -20.655], [-51.05, -22.93, -39.85, -14.23], [-74.0, -34.0, -34.8, 5.3]);
        assert_eq!((nivel(&a, vicosa), nivel(&a, mg)), (Some(0), Some(0)));
        // O Brasil inteiro não cabe no arquivo inteiro: a visão de 4 (a de 2 não existe), com as células
        // 1,5 célula do arquivo a noroeste.
        assert_eq!(nivel(&a, brasil), Some(2));
        let e = posta_no_lugar(&a, [-74.0, -34.0, -34.0, 6.0], 2);
        assert!((e[0] + 74.0 + 1.5 / 336.0).abs() < 1e-9 && (e[3] - 6.0 - 1.5 / 336.0).abs() < 1e-9 && e[1] > -34.0 && e[2] < -34.0);
        assert_eq!(posta_no_lugar(&a, vicosa, 0), vicosa);
    }
}
