//! A ficha de cada camada do mapa (do `fichas.csv`, pelo `scripts/gerar.py`): de que comando ela
//! veio, para o botão Fontes pôr na tabela o que ela é, o período, a resolução, as unidades, a
//! legenda, os cuidados e se funciona no app web.

use crate::{
    Ficha, PREFIXO, cemaden, chm, clms, dist, gedi, gem15, indicadores, irrigacao, mapbiomas, nisar, radar, risco_fogo, sol, tema_da_camada,
};

/// O comando que traz a camada `nome` do mapa. `fonte` é a citação dela na tabela Fontes: a
/// camada Indicadores por município tem uma por fonte, e a ficha é a da fonte.
pub fn comando_da_camada(nome: &str, fonte: &str) -> Option<&'static str> {
    if let Some(t) = tema_da_camada(nome) {
        return Some(t.id);
    }
    let n = nome.strip_prefix(PREFIXO)?;
    let por_fonte = [
        (indicadores::Fonte::Censo, "censo-municipios"),
        (indicadores::Fonte::Agro, "censo-agro"),
        (indicadores::Fonte::Agroeco, "censo-agroecologia"),
        (indicadores::Fonte::Pix, "indicadores-pix"),
        (indicadores::Fonte::Credito, "indicadores-credito"),
        (indicadores::Fonte::Tse, "indicadores-tse"),
        (indicadores::Fonte::Caged, "indicadores-caged"),
    ];
    if n == "Indicadores por município" {
        return por_fonte.iter().find(|(f, _)| f.citacao() == fonte).map(|(_, id)| *id);
    }
    let id = if n.starts_with("Focos de calor d") && n.ends_with(" (INPE)") {
        "inpe-focos-periodo"
    } else if (n.starts_with("Uso e cobertura ") && n.contains("(MapBiomas")) || mapbiomas::eh_das_classes(nome) {
        "mapbiomas-cobertura"
    } else if mapbiomas::eh_da_irrigacao(nome) {
        "mapbiomas-irrigacao"
    } else if n.starts_with("Estabelecimentos de saúde") {
        "saude-estabelecimentos"
    } else if nome == cemaden::CHUVA {
        "cemaden-chuva"
    } else if nome == cemaden::ALERTAS {
        "cemaden-alertas"
    } else if nisar::eh_do_nisar(nome) {
        if n.starts_with("Radar HH") {
            "nisar-hh"
        } else if n.starts_with("Radar HV") {
            "nisar-hv"
        } else if n.starts_with("Razão") {
            "nisar-razao"
        } else {
            "nisar-cor"
        }
    } else if radar::eh_do_radar(nome) {
        if n.starts_with("Radar VV") {
            "radar-vv"
        } else if n.starts_with("Radar VH") {
            "radar-vh"
        } else {
            "radar-razao"
        }
    } else if sol::eh_do_sol(nome) {
        "sol-radiacao"
    } else if risco_fogo::mes_da_camada(nome).is_some() {
        "risco-fogo"
    } else if nome == irrigacao::CAMADA {
        "irrigacao-gmia"
    } else if gedi::eh_do_gedi(nome) {
        gedi::Variavel::TODAS.into_iter().find(|v| v.camada() == nome)?.comando()
    } else if dist::eh_do_dist(nome) {
        if n.starts_with("Perturbação, dias") {
            "dist-data"
        } else if n.starts_with("Confiança") {
            "dist-confianca"
        } else if n.starts_with("Intensidade") {
            "dist-intensidade"
        } else {
            "dist-situacao"
        }
    } else if nome == chm::CAMADA {
        "chm-altura"
    } else if nome == crate::chm_amazonia::CAMADA {
        "chm-amazonia"
    } else if crate::gba::eh_do_gba(nome) {
        "gba-edificacoes"
    } else if crate::gdo::eh_do_gdo(nome) {
        "gdo-seca"
    } else if nome == crate::forty::CAMADA {
        "forty-tipologia"
    } else if crate::ctrees::da_camada(nome).is_some() {
        "ctrees-biomassa"
    } else if crate::bdc::satelite_da_camada(nome).is_some() {
        "bdc-indice"
    } else if let Some(v) = crate::jaxa::variavel_da_camada(nome) {
        v.comando()
    } else if let Some(c) = crate::era5::comando_da_camada(nome) {
        c
    } else if gem15::eh_do_gem15(nome) {
        "gem15"
    } else if clms::ano_da_camada(nome).is_some() {
        "clms-cobertura"
    } else if n.starts_with("Elevação (Copernicus") {
        "elevacao"
    } else if n.starts_with("Setores censitários") {
        "censo-setores"
    } else {
        return None;
    };
    Some(id)
}

/// A ficha da camada `nome` (com a citação `fonte`, como em [`comando_da_camada`]).
pub fn da_camada(nome: &str, fonte: &str) -> Option<&'static Ficha> {
    comando_da_camada(nome, fonte).and_then(crate::ficha)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FICHAS;

    #[test]
    fn cada_ficha_tem_um_texto_que_cabe_no_painel() {
        for f in FICHAS {
            assert!(f.texto.chars().count() <= 4000, "{}", f.id);
            assert!(f.texto.contains(f.pagina), "{}", f.id);
        }
    }

    #[test]
    fn as_camadas_do_mapa_achadas_pelo_nome() {
        let casos = [
            ("BrasilData: Solos (BDiA 1:250.000)", "solos"),
            ("BrasilData: Chuva nas estações (CEMADEN)", "cemaden-chuva"),
            ("BrasilData: Alertas vigentes (CEMADEN)", "cemaden-alertas"),
            ("BrasilData: Radar HV em dB (NISAR, 01/11/2026)", "nisar-hv"),
            ("BrasilData: Composição HH, HV, HH − HV (NISAR, 01/11/2026)", "nisar-cor"),
            ("BrasilData: Razão VH/VV em dB (Sentinel-1, 26/10/2026)", "radar-razao"),
            ("BrasilData: Perturbação, dias antes de 28/02/2026 (OPERA DIST-S1)", "dist-data"),
            ("BrasilData: Perturbação da vegetação (OPERA DIST-S1, até 28/02/2026)", "dist-situacao"),
            ("BrasilData: Biomassa acima do solo (GEDI L4D)", "gedi-biomassa"),
            ("BrasilData: Uso e cobertura 2024 (MapBiomas, recortado)", "mapbiomas-cobertura"),
            ("BrasilData: Elevação (Copernicus GLO-30)", "elevacao"),
            (chm::CAMADA, "chm-altura"),
            (crate::chm_amazonia::CAMADA, "chm-amazonia"),
            ("BrasilData: Edificações e alturas (GlobalBuildingAtlas)", "gba-edificacoes"),
            ("BrasilData: Seca: Anomalia da umidade do solo (GDO, 01/10/2026)", "gdo-seca"),
            (crate::forty::CAMADA, "forty-tipologia"),
            ("BrasilData: Biomassa acima do solo 2025 (CTrees, 100 m)", "ctrees-biomassa"),
            ("BrasilData: Incerteza da biomassa 2010 (CTrees, 100 m)", "ctrees-biomassa"),
            (irrigacao::CAMADA, "irrigacao-gmia"),
        ];
        for (nome, id) in casos {
            assert_eq!(comando_da_camada(nome, ""), Some(id), "{nome}");
            assert!(da_camada(nome, "").is_some(), "{nome}");
        }
        let pix = indicadores::Fonte::Pix.citacao();
        assert_eq!(comando_da_camada("BrasilData: Indicadores por município", &pix), Some("indicadores-pix"));
        assert_eq!(comando_da_camada("Outra camada", ""), None);
    }
}
