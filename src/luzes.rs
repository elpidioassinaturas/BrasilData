//! As luzes noturnas de 1992 a 2024 (PCNL, "Pixel-scale Corrected Nighttime Light Dataset", Li, Cao e
//! outros): uma série anual de imagens de satélite da luz à noite, com os dois sensores (o DMSP, de
//! 1992 a 2013, e o VIIRS, depois) corrigidos um pelo outro, na mesma escala.
//!
//! - **De onde:** o recorte do Brasil (`scripts/luzes_noturnas.py`), um COG por ano, numa release de
//!   dados do repositório. O original (doi:10.5281/zenodo.17013414, CC BY 4.0) é um GeoTIFF do mundo
//!   por ano, em blocos e sem visões: lê-se pela janela, mas o Zenodo limita os pedidos por minuto de
//!   cada endereço de rede, e uma área grande passava do limite.
//! - **O valor:** um número relativo, sem unidade; serve para comparar lugares e anos.
//! - **Como:** a janela da área, posta na grade do projeto e recortada ([`crate::cog_global`]); a
//!   mudança é a diferença entre dois anos.

use crate::PREFIXO;
use crate::cog_global::Arquivo;

/// A release de dados com os arquivos.
pub const BASE: &str = "https://github.com/elpidioassinaturas/BrasilData/releases/download/dados-luzes-noturnas/";

pub const PRIMEIRO_ANO: i64 = 1992;
pub const ULTIMO_ANO: i64 = 2024;

/// O primeiro ano depois do DMSP.
pub const PRIMEIRO_DO_VIIRS: i64 = 2014;

/// O lado da célula do arquivo, em graus (30 segundos, cerca de 900 m), e quantas visões ele tem
/// além da inteira.
pub const CELULA_GRAUS: f64 = 1.0 / 120.0;
pub const VISOES: i32 = 4;

/// O fim do nome das camadas.
const FIM: &str = " (PCNL, DMSP e VIIRS)";

/// Os valores possíveis.
pub const FAIXA: (f64, f64) = (0.0, 10000.0);

/// As rampas: a luz, do azul quase preto ao branco, com mais tons no pouco iluminado (das células
/// acesas do Brasil, metade fica abaixo de 10, e 99% abaixo de 75); a mudança, do roxo (menos luz) ao
/// laranja (mais luz).
pub const RAMPA: (&str, &str) =
    ("BrasilData: luzes noturnas", "0 10 10 30\n2 46 24 74\n5 92 36 96\n10 170 62 84\n20 232 116 52\n40 252 196 70\n80 255 246 210");
pub const RAMPA_MUDANCA: (&str, &str) = (
    "BrasilData: mudança das luzes",
    "-30 84 39 136\n-12 153 142 195\n-3 216 218 235\n0 247 247 247\n3 254 224 182\n12 241 163 64\n30 179 88 6",
);
pub const ESCALA: (f64, f64) = (0.0, 80.0);
pub const ESCALA_MUDANCA: (f64, f64) = (-30.0, 30.0);

/// O ano pedido, conferido.
pub fn conferir_ano(a: f64) -> Result<i64, String> {
    let ano = a as i64;
    if a.fract() != 0.0 || !(PRIMEIRO_ANO..=ULTIMO_ANO).contains(&ano) {
        return Err(format!("o ano das luzes noturnas é de {PRIMEIRO_ANO} a {ULTIMO_ANO}"));
    }
    Ok(ano)
}

/// O arquivo do ano.
pub fn arquivo(ano: i64) -> Arquivo {
    Arquivo { url: format!("{BASE}luzes_noturnas_{ano}.tif"), graus: CELULA_GRAUS, piramides: VISOES }
}

/// A camada do ano.
pub fn camada(ano: i64) -> String {
    format!("{PREFIXO}Luzes noturnas {ano}{FIM}")
}

/// A camada da mudança de `a` para `b`.
pub fn camada_mudanca(a: i64, b: i64) -> String {
    format!("{PREFIXO}Mudança das luzes noturnas, {a} a {b}{FIM}")
}

/// O que é a camada do plugin.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Camada {
    Ano(i64),
    Mudanca(i64, i64),
}

/// A camada `nome`, se é das luzes noturnas.
pub fn da_camada(nome: &str) -> Option<Camada> {
    let n = nome.strip_prefix(PREFIXO)?.strip_suffix(FIM)?;
    if let Some(r) = n.strip_prefix("Mudança das luzes noturnas, ") {
        let (a, b) = r.split_once(" a ")?;
        return Some(Camada::Mudanca(a.parse().ok()?, b.parse().ok()?));
    }
    n.strip_prefix("Luzes noturnas ")?.parse().ok().map(Camada::Ano)
}

/// O nome da janela do ano, lida na visão `k`.
pub fn janela_nome(ano: i64, k: i32) -> String {
    format!("{PREFIXO}luzes {ano} nível {k} (janela)")
}

/// Se os dois anos são de sensores diferentes (o DMSP até 2013; o VIIRS depois).
pub fn troca_de_sensor(a: i64, b: i64) -> bool {
    (a < PRIMEIRO_DO_VIIRS) != (b < PRIMEIRO_DO_VIIRS)
}

pub fn citacao(c: Camada) -> String {
    let o_que = match c {
        Camada::Ano(a) => format!("o ano de {a}"),
        Camada::Mudanca(a, b) => format!("{b} menos {a}"),
    };
    format!(
        "Li, S., Cao, X., Zhao, C., Jie, N., Liu, L., Chen, X. e Cui, X. (2023), Developing a Pixel-Scale Corrected Nighttime Light Dataset (PCNL, 1992–2021) Combining DMSP-OLS and NPP-VIIRS, Remote Sensing 15, 3925, doi:10.3390/rs15163925; dados até 2024: doi:10.5281/zenodo.17013414, {o_que}; recorte do Brasil pelo BrasilData, posto na grade do projeto e recortado pela área"
    )
}

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anos_nomes_e_enderecos() {
        assert_eq!(conferir_ano(2024.0), Ok(2024));
        for a in [1991.0, 2025.0, 2000.5] {
            assert!(conferir_ano(a).is_err(), "{a}");
        }
        assert!(arquivo(1992).url.ends_with("/dados-luzes-noturnas/luzes_noturnas_1992.tif"));
        assert_eq!(camada(2024), "BrasilData: Luzes noturnas 2024 (PCNL, DMSP e VIIRS)");
        assert_eq!(da_camada(&camada(2024)), Some(Camada::Ano(2024)));
        assert_eq!(da_camada(&camada_mudanca(2000, 2025)), Some(Camada::Mudanca(2000, 2025)));
        assert_eq!(da_camada("BrasilData: Luzes noturnas 2025"), None);
        assert_eq!(da_camada(&janela_nome(2025, 0)), None);
        assert_ne!(janela_nome(2025, 0), janela_nome(2025, 1));
        assert!(troca_de_sensor(2000, 2024) && troca_de_sensor(2024, 2013));
        assert!(!troca_de_sensor(2014, 2024) && !troca_de_sensor(1992, 2013));
        assert!(citacao(Camada::Mudanca(2000, 2025)).contains("2025 menos 2000"));
    }

    #[test]
    fn a_visao_da_area() {
        let a = arquivo(2025);
        // Viçosa e um estado, da imagem inteira; o Brasil a 500 m também cabe nela.
        assert_eq!(a.nivel_que_cabe([-43.003, -20.838, -42.754, -20.655], a.nivel(30.0, -20.7)), Some(0));
        assert_eq!(a.nivel_que_cabe([-73.983, -33.751, -32.4, 5.27], a.nivel(500.0, -14.0)), Some(0));
        // Numa grade de 5 km, a visão de cerca de 3,6 km.
        assert_eq!(a.nivel(5000.0, -14.0), 2);
    }
}
