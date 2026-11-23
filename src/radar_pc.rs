//! O radar do Sentinel-1 em 10 m, corrigido do relevo, do Planetary Computer da Microsoft: a
//! coleção `sentinel-1-rtc` (o GRD da ESA processado pela Catalyst, gama0 em potência linear, COG
//! em UTM com células de 10 m, licença CC-BY 4.0), uma passagem a cada 12 dias desde 2014.
//!
//! - **De onde:** a busca do STAC do Planetary Computer (GET, com `fields` para a resposta caber
//!   folgada no `host.fetch`) lista as cenas que cruzam o retângulo da área, as mais recentes
//!   primeiro. Valem as da órbita e do sentido (ascendente ou descendente) da mais recente: a
//!   mesma visada, para a mediana não misturar encostas iluminadas de lados diferentes.
//! - **Login:** nenhum. Os arquivos ficam no Azure Blob Storage e são lidos com um token anônimo
//!   do serviço de tokens do Planetary Computer, que vale por algumas horas e vai no fim do
//!   endereço ([`assinar`]).
//! - **Os valores:** só a janela da área de cada cena, lida por faixas (`layer.addUrl` com
//!   `bbox`), posta na grade do projeto; a mediana das passagens (`cube.reduce`) tira o granulado
//!   sem borrar as bordas, como no Sentinel-1 Global Backscatter Model da TU Wien, e chega em dB.
//! - **A posição:** o dado vem deslocado para oeste. Medido em 22/11/2026 contra o Sentinel-2 L2A
//!   (VH mediano x NDVI, em oito áreas planas, 17 medidas, S1A, S1C e S1D): de 13 a 33 m a oeste,
//!   mediana de 18 m, e de 7 m ao sul a 8 m ao norte; o controle, Sentinel-2 x Sentinel-2, dá até
//!   1,4 m. Não é desta leitura: o rasterio e o RustGIS leem o arquivo no mesmo lugar (1 m). Não
//!   inverte entre as órbitas na única área com as duas (Cascavel: 33 m na descendente, 25 m na
//!   ascendente), e em Viçosa, de relevo forte, muda ao longo da área. Sem um valor único, nada é
//!   corrigido: o comando e a ficha avisam ([`AVISO_POSICAO`]).

use serde_json::Value;

use crate::datas::Data;
use crate::radar::Pol;

/// A busca de cenas do STAC do Planetary Computer.
const BUSCA: &str = "https://planetarycomputer.microsoft.com/api/stac/v1/search";

/// A coleção.
pub const COLECAO: &str = "sentinel-1-rtc";

/// O pedido do token anônimo que lê os arquivos da coleção.
pub const URL_DO_TOKEN: &str = "https://planetarycomputer.microsoft.com/api/sas/v1/token/sentinel-1-rtc";

/// Quantas cenas a busca devolve (cerca de 1,3 kB cada com os `fields` pedidos).
pub const MAX_BUSCA: usize = 100;

/// Quantas passagens, por padrão, e no máximo.
pub const PASSAGENS: usize = 10;
pub const MAX_PASSAGENS: usize = 30;

/// O lado maior do retângulo da área, em graus, que o comando aceita (cerca de 220 km); a janela
/// de uma área grande vem de uma visão reduzida do arquivo ([`celula_de_leitura`]).
pub const MAX_GRAUS: f64 = 2.0;

/// A rampa, como no Sentinel-1 Global Backscatter Model.
pub const RAMPA: &str = "Viridis";

/// A citação.
pub const CITACAO: &str = "Sentinel-1 Radiometrically Terrain Corrected (RTC), Microsoft Planetary Computer (coleção sentinel-1-rtc), processado pela Catalyst a partir do GRD da ESA. Contém dados Copernicus Sentinel-1 modificados (ESA)";

/// A licença.
pub const LICENCA: &str = "CC-BY 4.0, com citação; dados Copernicus Sentinel de uso livre e gratuito";

/// A escala, em dB, da mediana: os percentis 2 e 98 de Viçosa com 10 passagens (03/08 a
/// 19/11/2026, grade de 10 m), arredondados (VV de −12,5 a −2,8; VH de −17,8 a −9,1).
pub fn escala(p: Pol) -> (f64, f64) {
    match p {
        Pol::Vv => (-13.0, -3.0),
        Pol::Vh => (-18.0, -9.0),
    }
}

/// O aviso da posição, dado depois da camada (o RustGIS corta um aviso em 500 caracteres, e o texto da
/// camada já ocupa quase tudo).
pub const AVISO_POSICAO: &str = "Radar 10 m: a posição deste dado não é exata. Em oito áreas planas do Brasil ele ficou de 13 a 33 m a oeste do Sentinel-2 (mediana de 18 m), e no relevo forte o desvio muda de um lugar para outro. Confira antes de sobrepor a outro dado de 10 m";

/// A chave do arquivo da polarização na cena.
fn chave(p: Pol) -> &'static str {
    match p {
        Pol::Vv => "vv",
        Pol::Vh => "vh",
    }
}

/// A busca das cenas que cruzam o retângulo `c`, as mais recentes primeiro.
pub fn url_busca(c: [f64; 4]) -> String {
    format!(
        "{BUSCA}?collections={COLECAO}&bbox={:.4},{:.4},{:.4},{:.4}&limit={MAX_BUSCA}&sortby=-datetime&fields=id,properties.datetime,properties.sat:relative_orbit,properties.sat:orbit_state,assets.vv.href,assets.vh.href",
        c[0], c[1], c[2], c[3]
    )
}

/// Uma passagem: o dia, a órbita, o sentido e os arquivos da polarização (um por cena; a área
/// pode cair em duas cenas seguidas da mesma passagem).
#[derive(Clone, Debug, PartialEq)]
pub struct Passagem {
    pub data: Data,
    pub orbita: u64,
    pub sentido: String,
    pub arquivos: Vec<String>,
}

/// As `n` passagens mais recentes da resposta da busca `r`, na polarização `p`, todas da órbita e
/// do sentido da cena mais recente; da mais nova para a mais velha.
pub fn passagens(r: &Value, p: Pol, n: usize) -> Vec<Passagem> {
    let mut cenas: Vec<(String, u64, String, String)> = r["features"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|f| {
            let quando = f["properties"]["datetime"].as_str()?.to_string();
            let orbita = f["properties"]["sat:relative_orbit"].as_u64()?;
            let sentido = f["properties"]["sat:orbit_state"].as_str()?.to_string();
            let href = f["assets"][chave(p)]["href"].as_str()?.to_string();
            Some((quando, orbita, sentido, href))
        })
        .collect();
    // As mais recentes primeiro (a busca já pede assim; aqui, por garantia).
    cenas.sort_by(|a, b| b.0.cmp(&a.0));
    let Some((_, orbita, sentido, _)) = cenas.first().cloned() else { return Vec::new() };
    let mut saida: Vec<Passagem> = Vec::new();
    for (quando, o, s, href) in cenas {
        if o != orbita || s != sentido {
            continue;
        }
        let Some(data) = data_iso(&quando) else { continue };
        match saida.last_mut() {
            Some(ultima) if ultima.data == data => {
                if !ultima.arquivos.contains(&href) {
                    ultima.arquivos.push(href);
                }
            }
            _ => {
                if saida.len() == n {
                    break;
                }
                saida.push(Passagem { data, orbita, sentido: sentido.clone(), arquivos: vec![href] });
            }
        }
    }
    saida
}

/// A data de "aaaa-mm-ddT…".
fn data_iso(t: &str) -> Option<Data> {
    let d = t.get(..10)?;
    let mut p = d.split('-');
    let (ano, mes, dia) = (p.next()?, p.next()?, p.next()?);
    Data::ler(&format!("{dia}/{mes}/{ano}"))
}

/// O token da resposta do serviço de tokens.
pub fn token(r: &Value) -> Option<String> {
    r["token"].as_str().map(str::trim).filter(|t| !t.is_empty() && !t.contains(char::is_whitespace)).map(String::from)
}

/// O endereço de um arquivo com o token no fim (um endereço que já traga um `?` fica como está).
pub fn assinar(href: &str, token: &str) -> String {
    if href.contains('?') { href.to_string() } else { format!("{href}?{token}") }
}

/// A célula da leitura das janelas, em metros: as visões do arquivo vão de 10 m em 10 m dobrando;
/// a mais fina que deixa a janela do retângulo `c` abaixo de 50 milhões de células (o RustGIS lê
/// até 64 milhões de valores) e não é bem mais fina que a grade de `celula` m.
pub fn celula_de_leitura(c: [f64; 4], celula: f64) -> f64 {
    let lat = (c[1] + c[3]) / 2.0;
    let largura = (c[2] - c[0]).abs() * 111_320.0 * lat.to_radians().cos();
    let altura = (c[3] - c[1]).abs() * 110_574.0;
    let minima = (largura * altura / 50.0e6).sqrt();
    let mut l = 10.0;
    while l < minima || l * 2.0 <= celula {
        l *= 2.0;
    }
    l
}

/// O nome da janela de uma cena: a polarização, o dia e qual das cenas da passagem.
pub fn nome_da_janela(p: Pol, d: Data, i: usize) -> String {
    format!("{}radar 10 m {} {} {}", crate::PREFIXO, p.sigla(), d.iso(), i + 1)
}

/// O nome da camada: a polarização, quantas passagens e o período.
pub fn nome_da_camada(p: Pol, n: usize, de: Data, ate: Data) -> String {
    format!("{}Radar {} 10 m, mediana de {n} passagens (Sentinel-1, {} a {})", crate::PREFIXO, p.sigla(), de.texto(), ate.texto())
}

/// O nome da composição colorida feita das camadas VV e VH de nome como o `vv` (o período e as
/// passagens vêm dele).
pub fn nome_da_composicao(vv: &str) -> Option<String> {
    let resto = vv.strip_prefix(crate::PREFIXO)?.strip_prefix("Radar VV 10 m, ")?;
    Some(format!("{}Composição VV, VH, VV/VH 10 m, {resto}", crate::PREFIXO))
}

/// As escalas, em dB, do vermelho (VV), do verde (VH) e do azul (VV − VH) da composição: entre os
/// percentis 2 e 98 de Viçosa (VV de −12,5 a −2,8; VH de −17,8 a −9,1; VV − VH de 3,2 a 8,5) e os
/// de uma área de pivôs de João Pinheiro (−16,1 a −5,9; −23,1 a −11,4; 3,3 a 9,6), com 10
/// passagens de agosto a novembro de 2026.
pub const ESCALAS_DA_COMPOSICAO: [(f64, f64); 3] = [(-15.0, -3.0), (-21.0, -10.0), (3.0, 9.0)];

/// Se uma camada do mapa é desta fonte (para a citação).
pub fn eh_daqui(nome: &str) -> bool {
    nome.strip_prefix(crate::PREFIXO)
        .is_some_and(|n| (n.starts_with("Radar V") || n.starts_with("Composição VV, VH")) && n.contains(" 10 m, mediana de "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cena(quando: &str, orbita: u64, sentido: &str, nome: &str) -> Value {
        json!({"properties": {"datetime": quando, "sat:relative_orbit": orbita, "sat:orbit_state": sentido},
            "assets": {"vv": {"href": format!("https://x.blob.core.windows.net/c/{nome}/iw-vv.rtc.tiff")},
                       "vh": {"href": format!("https://x.blob.core.windows.net/c/{nome}/iw-vh.rtc.tiff")}}})
    }

    #[test]
    fn as_passagens_da_mesma_orbita_juntando_as_cenas_do_dia() {
        let r = json!({"features": [
            cena("2026-11-19T08:21:26Z", 155, "descending", "a"),
            cena("2026-11-19T08:21:01Z", 155, "descending", "b"),
            cena("2026-11-14T21:40:00Z", 148, "ascending", "c"),
            cena("2026-11-07T08:21:26Z", 155, "descending", "d"),
            cena("2026-10-26T08:21:26Z", 155, "descending", "e"),
        ]});
        let p = passagens(&r, Pol::Vv, 2);
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].data, Data { ano: 2026, mes: 11, dia: 19 });
        assert_eq!(p[0].arquivos.len(), 2, "as duas cenas do dia");
        assert!(p[0].arquivos[0].ends_with("/a/iw-vv.rtc.tiff"));
        assert_eq!(p[1].data, Data { ano: 2026, mes: 11, dia: 7 }, "a de outra órbita fica de fora");
        assert!(passagens(&r, Pol::Vh, 10)[2].arquivos[0].ends_with("/e/iw-vh.rtc.tiff"));
        assert!(passagens(&json!({"features": []}), Pol::Vv, 3).is_empty());
    }

    #[test]
    fn o_token_e_o_endereco_assinado() {
        assert_eq!(
            token(&json!({"msft:expiry": "2026-11-23T00:38:17Z", "token": "st=1&se=2&sig=a%2B"})).as_deref(),
            Some("st=1&se=2&sig=a%2B")
        );
        assert_eq!(token(&json!({"token": ""})), None);
        assert_eq!(assinar("https://x/a.tiff", "st=1&sig=b"), "https://x/a.tiff?st=1&sig=b");
        assert_eq!(assinar("https://x/a.tiff?st=1", "st=2"), "https://x/a.tiff?st=1");
    }

    #[test]
    fn o_aviso_da_posicao_cabe_num_aviso_do_rustgis() {
        assert!(AVISO_POSICAO.chars().count() <= 480, "{}", AVISO_POSICAO.chars().count());
    }

    #[test]
    fn a_celula_de_leitura() {
        let vicosa = [-43.005, -20.839, -42.754, -20.655];
        assert_eq!(celula_de_leitura(vicosa, 10.0), 10.0);
        assert_eq!(celula_de_leitura(vicosa, 30.0), 20.0, "não bem mais fina que a grade");
        // Um retângulo de 2° × 2° só cabe numa visão reduzida.
        assert_eq!(celula_de_leitura([-44.0, -21.0, -42.0, -19.0], 10.0), 40.0);
    }

    #[test]
    fn a_busca_e_os_nomes() {
        let u = url_busca([-43.0, -20.8, -42.7, -20.6]);
        assert!(
            u.starts_with("https://planetarycomputer.microsoft.com/api/stac/v1/search?collections=sentinel-1-rtc&bbox=-43.0000,-20.8000,")
        );
        assert!(u.contains("sortby=-datetime") && u.contains("assets.vv.href"));
        let d = Data { ano: 2026, mes: 11, dia: 19 };
        let n = nome_da_camada(Pol::Vv, 10, Data { ano: 2026, mes: 8, dia: 3 }, d);
        assert_eq!(n, "BrasilData: Radar VV 10 m, mediana de 10 passagens (Sentinel-1, 03/08/2026 a 19/11/2026)");
        assert!(eh_daqui(&n) && !eh_daqui("BrasilData: Radar VV em dB (Sentinel-1, 19/11/2026)"));
        let cor = nome_da_composicao(&n).unwrap();
        assert_eq!(cor, "BrasilData: Composição VV, VH, VV/VH 10 m, mediana de 10 passagens (Sentinel-1, 03/08/2026 a 19/11/2026)");
        assert!(eh_daqui(&cor));
        assert_eq!(nome_da_composicao("BrasilData: Radar VH 10 m, mediana de 10 passagens"), None);
        assert_eq!(nome_da_janela(Pol::Vh, d, 0), "BrasilData: radar 10 m VH 2026-11-19 1");
        assert_eq!(data_iso("2026-08-03T08:21:25.206356Z"), Some(Data { ano: 2026, mes: 8, dia: 3 }));
    }
}
