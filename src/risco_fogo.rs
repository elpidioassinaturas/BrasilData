//! O Risco de Fogo do INPE (Programa Queimadas): a climatologia mensal de 2001 a 2025 do perigo
//! meteorológico de fogo, de 0 a 1, do modelo RF versão 11 (dias de secura pela chuva do IMERG,
//! umidade e temperatura do GFS, a vegetação, o relevo e a latitude).
//!
//! - **De onde:** um GeoTIFF por mês no servidor do INPE (`dataserver-coids.inpe.br`, sem login),
//!   a América do Sul a 0,01° (cerca de 1 km), float32 em blocos de 256, sem dado -999, sem
//!   pirâmides. Cada mês é a média dos valores diários desse mês de 2001 a 2025.
//! - **Como:** só a janela da área é lida (`layer.addUrl` com `bbox`, depois da 0.5.0), posta na
//!   grade do projeto (bilinear numa grade mais fina, média numa mais grossa) e recortada pela área.
//! - **Cores:** as cinco classes do método (Tabela 3.5): mínimo abaixo de 0,15, baixo até 0,40,
//!   médio até 0,70, alto até 0,95 e crítico acima: uma rampa em degraus que muda de cor nos
//!   limites, porque o RustGIS não classifica um raster em limites dados.

/// O endereço do mês `mes` (1 a 12).
pub fn url(mes: u8) -> String {
    format!(
        "https://dataserver-coids.inpe.br/queimadas/queimadas/riscofogo_meteorologia/climatologia_risco_de_fogo/INPE_FireRiskModel_Climatologia_AMS_2001_2025{mes:02}01.tif"
    )
}

/// O mês padrão: setembro, o auge da estação de fogo no Brasil central.
pub const MES_PADRAO: u8 = 9;

/// O controle do mês no painel.
pub const CONTROLE: &str = "risco-fogo-mes";

pub const NOMES_DOS_MESES: [&str; 12] =
    ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"];

/// O nome do mês `mes` (1 a 12).
pub fn nome_do_mes(mes: u8) -> &'static str {
    NOMES_DOS_MESES.get(usize::from(mes).wrapping_sub(1)).copied().unwrap_or("setembro")
}

/// O mês do parâmetro `mes` (1 a 12).
pub fn conferir_mes(mes: f64) -> Result<u8, String> {
    if mes.fract() == 0.0 && (1.0..=12.0).contains(&mes) {
        Ok(mes as u8)
    } else {
        Err(format!("o mes vai de 1 (janeiro) a 12 (dezembro), não {mes}"))
    }
}

/// O nome da camada do mês `mes`.
pub fn camada(mes: u8) -> String {
    format!("BrasilData: Risco de fogo em {} (INPE, 2001–2025)", nome_do_mes(mes))
}

/// O mês da camada `nome`, se ela é do risco de fogo.
pub fn mes_da_camada(nome: &str) -> Option<u8> {
    (1..=12).find(|&m| camada(m) == nome)
}

/// A camada de passagem com a janela lida do mês `mes`.
pub fn janela_do_mes(mes: u8) -> String {
    format!("BrasilData: risco de fogo {mes:02} (janela)")
}

/// A camada de passagem com a janela na grade.
pub const NA_GRADE: &str = "BrasilData: risco de fogo na grade";

/// O lado da célula do arquivo, em graus, e em metros no equador.
pub const LADO: f64 = 0.01;
pub const LADO_M: f64 = LADO * 111_320.0;

/// A janela: o retângulo `c` com duas células de margem (os vizinhos da interpolação).
pub fn janela(c: [f64; 4]) -> [f64; 4] {
    [c[0] - 2.0 * LADO, c[1] - 2.0 * LADO, c[2] + 2.0 * LADO, c[3] + 2.0 * LADO]
}

/// A reamostragem para uma grade de células de `celula` metros.
pub fn reamostragem(celula: f64) -> &'static str {
    if celula > LADO_M { "average" } else { "bilinear" }
}

/// A citação.
pub const CITACAO: &str = "INPE, Programa Queimadas: Risco de Fogo, climatologia mensal de 2001 a 2025 (a média dos valores diários do mês), modelo RF versão 11 (Setzer, A., Sismanoglu, R. e Santos, J. G. 2019. Método do cálculo do risco de fogo do Programa do INPE – Versão 11, sid.inpe.br/mtc-m21c/2019/11.21.11.03-PRP), 0,01°; posto na grade do projeto e recortado pela área";

pub const LICENCA: &str = "licença não declarada nos arquivos; uso com a citação do INPE e do método";

/// As classes do método (Tabela 3.5): o limite de cima, o nome e a cor.
pub const CLASSES: [(f64, &str, &str); 5] = [
    (0.15, "mínimo", "26 152 80"),
    (0.40, "baixo", "166 217 106"),
    (0.70, "médio", "254 224 139"),
    (0.95, "alto", "244 109 67"),
    (1.0, "crítico", "165 0 38"),
];

/// O nome da rampa.
pub const RAMPA: &str = "BrasilData: risco de fogo (classes do INPE)";

/// A rampa em degraus (formato do GDAL): cada classe com uma cor só, de um limite ao outro.
pub fn rampa() -> String {
    let mut linhas = Vec::new();
    let mut de = 0.0_f64;
    for (ate, _, cor) in CLASSES {
        linhas.push(format!("{de} {cor}"));
        let fim = if ate < 1.0 { format!("{:.4}", ate - 0.0001) } else { format!("{ate}") };
        linhas.push(format!("{fim} {cor}"));
        de = ate;
    }
    linhas.join("\n")
}

/// As classes por extenso, para os avisos e a nota do layout.
pub fn texto_das_classes() -> String {
    "mínimo, abaixo de 0,15; baixo, de 0,15 a 0,40; médio, de 0,40 a 0,70; alto, de 0,70 a 0,95; crítico, acima de 0,95".into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn o_endereco_e_o_nome_seguem_o_mes() {
        assert!(url(9).ends_with("/INPE_FireRiskModel_Climatologia_AMS_2001_20250901.tif"));
        assert!(url(12).ends_with("_2001_20251201.tif"));
        assert_eq!(camada(9), "BrasilData: Risco de fogo em setembro (INPE, 2001–2025)");
        assert_eq!(mes_da_camada(&camada(3)), Some(3));
        assert_eq!(mes_da_camada("BrasilData: Radiação solar (NASA POWER)"), None);
        assert_eq!(conferir_mes(7.0), Ok(7));
        assert!(conferir_mes(0.0).is_err() && conferir_mes(13.0).is_err() && conferir_mes(1.5).is_err());
    }

    #[test]
    fn a_rampa_muda_de_cor_nos_limites_das_classes() {
        let r = rampa();
        let linhas: Vec<&str> = r.lines().collect();
        assert_eq!(linhas.len(), 10);
        assert_eq!(linhas[0], "0 26 152 80");
        assert_eq!(linhas[1], "0.1499 26 152 80");
        assert_eq!(linhas[2], "0.15 166 217 106");
        assert_eq!(linhas[9], "1 165 0 38");
        let valores: Vec<f64> = linhas.iter().map(|l| l.split(' ').next().unwrap().parse().unwrap()).collect();
        assert!(valores.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn a_janela_tem_margem_e_a_grade_grossa_tira_a_media() {
        let j = janela([-47.0, -11.0, -46.0, -10.0]);
        assert!((j[0] + 47.02).abs() < 1e-12 && (j[3] + 9.98).abs() < 1e-12);
        assert_eq!(reamostragem(30.0), "bilinear");
        assert_eq!(reamostragem(2000.0), "average");
    }
}
