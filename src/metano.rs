//! O metano no ar: a coluna média de metano (XCH4, em ppb) do Sentinel-5P/TROPOMI sobre o Brasil,
//! em grade de 0,1° (~11 km), calculada pelo BrasilData a partir das órbitas (nível 2) do ano
//! (`scripts/metano_tropomi.py`) e publicada numa release de dados do repositório, em COG.
//!
//! O TROPOMI mede quanto metano há na coluna de ar, não quanto cada lugar emite: o valor cai com a
//! altitude (menos ar acima) e varia com a estação; as manchas altas mostram onde há fontes
//! (áreas alagadas, pecuária, aterros) quando aparecem nos vários meses.

/// A release de dados com os arquivos.
pub const BASE: &str = "https://github.com/elpidioassinaturas/BrasilData/releases/download/dados-metano-tropomi/";

/// Os anos com a grade pronta.
pub const ANOS: &[i64] = &[2023];

/// Os meses, para o formulário e as mensagens.
pub const MESES: [&str; 12] =
    ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

/// O começo do nome das camadas.
pub const PREFIXO_CAMADA: &str = "BrasilData: Metano no ar, XCH4 (Sentinel-5P";

/// A faixa das cores (ppb): a coluna de metano sobre o Brasil em 2023 fica quase toda nela (do
/// percentil 5, 1.712, ao 99, 1.881, com o meio entre 1.800 e 1.880).
pub const FAIXA: (f64, f64) = (1790.0, 1890.0);

/// A margem (graus) da janela lida em volta da área: um pouco mais que uma célula.
pub const MARGEM: f64 = 0.15;

/// O período: o ano e, se pedido, o mês (1 a 12). Aceita "2023", "2023-03", "média de 2023",
/// "março de 2023" e, sem nada, o ano mais recente.
pub fn ler_periodo(texto: Option<&str>) -> Result<(i64, Option<u32>), String> {
    let ultimo = *ANOS.last().ok_or("sem anos")?;
    let Some(t) = texto.map(str::trim).filter(|t| !t.is_empty()) else { return Ok((ultimo, None)) };
    let t = t.to_lowercase();
    let ano =
        t.split(|c: char| !c.is_ascii_digit()).filter_map(|p| p.parse::<i64>().ok()).find(|a| (2018..=2100).contains(a)).unwrap_or(ultimo);
    if !ANOS.contains(&ano) {
        return Err(format!("o metano do TROPOMI está pronto para {}; {ano} ainda não", anos_texto()));
    }
    let mes = MESES
        .iter()
        .position(|m| t.contains(m))
        .map(|i| i as u32 + 1)
        .or_else(|| t.split_once('-').and_then(|(_, m)| m.trim().parse::<u32>().ok()).filter(|m| (1..=12).contains(m)));
    Ok((ano, mes))
}

/// Os anos prontos, em texto.
pub fn anos_texto() -> String {
    ANOS.iter().map(i64::to_string).collect::<Vec<_>>().join(", ")
}

/// As escolhas do formulário: a média de cada ano e cada mês dele.
pub fn escolhas() -> Vec<String> {
    let mut v = Vec::new();
    for &a in ANOS.iter().rev() {
        v.push(format!("média de {a}"));
        v.extend(MESES.iter().map(|m| format!("{m} de {a}")));
    }
    v
}

/// O arquivo do período.
pub fn url(ano: i64, mes: Option<u32>) -> String {
    match mes {
        Some(m) => format!("{BASE}metano_tropomi_{ano}_{m:02}.tif"),
        None => format!("{BASE}metano_tropomi_{ano}.tif"),
    }
}

/// O período em texto: "2023" ou "março de 2023".
pub fn periodo_texto(ano: i64, mes: Option<u32>) -> String {
    match mes {
        Some(m) => format!("{} de {ano}", MESES[m as usize - 1]),
        None => ano.to_string(),
    }
}

/// O nome da camada do período.
pub fn camada(ano: i64, mes: Option<u32>) -> String {
    format!("{PREFIXO_CAMADA}, {})", periodo_texto(ano, mes))
}

/// O período no nome da camada ("2023", "março de 2023").
pub fn periodo_da_camada(nome: &str) -> Option<&str> {
    nome.strip_prefix(PREFIXO_CAMADA)?.strip_prefix(", ")?.strip_suffix(')')
}

/// A citação da camada `nome` (o ano dos dados está no período dela).
pub fn citacao(nome: &str) -> String {
    let ano = periodo_da_camada(nome).and_then(|p| p.rsplit(' ').next()).unwrap_or_default();
    format!(
        "Contém dados modificados do Copernicus Sentinel {ano}, processados pelo BrasilData; Sentinel-5P/TROPOMI L2 CH4 (ESA/SRON), pelo Microsoft Planetary Computer"
    )
}

pub const LICENCA: &str = "livre e aberto (licença dos dados Copernicus Sentinel), com a citação";

/// Se a camada `nome` é do metano.
pub fn da_camada(nome: &str) -> bool {
    nome.starts_with(PREFIXO_CAMADA)
}

/// A janela lida em volta da área `c`.
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - MARGEM, c[1] - MARGEM, c[2] + MARGEM, c[3] + MARGEM]
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn periodos() {
        assert_eq!(ler_periodo(None).unwrap(), (2023, None));
        assert_eq!(ler_periodo(Some("média de 2023")).unwrap(), (2023, None));
        assert_eq!(ler_periodo(Some("março de 2023")).unwrap(), (2023, Some(3)));
        assert_eq!(ler_periodo(Some("2023-11")).unwrap(), (2023, Some(11)));
        assert!(ler_periodo(Some("2019")).is_err());
        assert!(url(2023, Some(3)).ends_with("/metano_tropomi_2023_03.tif"));
        assert!(url(2023, None).ends_with("/metano_tropomi_2023.tif"));
        assert_eq!(camada(2023, Some(3)), "BrasilData: Metano no ar, XCH4 (Sentinel-5P, março de 2023)");
        assert_eq!(periodo_da_camada(&camada(2023, Some(3))), Some("março de 2023"));
        assert!(citacao(&camada(2023, Some(3))).contains("Sentinel 2023,") && citacao(&camada(2023, None)).contains("Sentinel 2023,"));
        assert!(da_camada(&camada(2023, None)));
        assert_eq!(escolhas()[0], "média de 2023");
        assert_eq!(escolhas().len(), 13);
    }
}
