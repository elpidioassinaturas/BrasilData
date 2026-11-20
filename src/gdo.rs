//! Os mapas de seca do Global Drought Observatory (GDO) do Copernicus (CEMS, Centro Comum de
//! Investigação da Comissão Europeia): a anomalia da umidade do solo, a da vegetação (fAPAR do
//! VIIRS) e o SPI do CHIRPS, de 10 em 10 dias (o SPI de 6 e 12 meses, por mês).
//!
//! - **De onde:** o WMS do GDO (`drought.emergency.copernicus.eu/api/wms`), sem login, CC BY 4.0,
//!   com a data em `TIME` e a escala do SPI em `SELECTED_TIMESCALE`. Sem data, o serviço dá a
//!   mais recente.
//! - **Por que um mapa para ver:** o WCS do GDO entrega classes de cor (0 a 9), não o valor do
//!   índice, e ignora o recorte de latitude; o valor só vem nos NetCDF do mundo por ano (centenas
//!   de MB). O WMS desenha o mapa como o GDO o mostra.
//! - **Como:** `map.addTiles` com o GetMap em Web Mercator montado aqui. Pelo `service.add`, o
//!   RustGIS lê nas capacidades do serviço a caixa da camada-mãe, que é a da Europa, e só pediria
//!   imagens dentro dela.
//! - **Recorte:** a camada não é recortada pela área: o RustGIS 0.7.0 não mascara um mapa de
//!   tiles (`layer.mask` só vale para camadas), e o WMS não entrega GeoTIFF. Pedido em RustGIS#2530.
//! - **O Low-Flow Index:** o do GDO só cobre a Europa (WMS, WCS e downloads).

use crate::datas::Data;

/// O GetMap do WMS do GDO.
const WMS: &str = "https://drought.emergency.copernicus.eu/api/wms?SERVICE=WMS&VERSION=1.1.1";

/// O que se traz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Indicador {
    Umidade,
    Vegetacao,
    /// O SPI do CHIRPS de 1, 3, 6 ou 12 meses.
    Spi(u8),
}

impl Indicador {
    pub fn ler(t: &str) -> Option<Indicador> {
        match t.trim() {
            "" | "umidade" => Some(Indicador::Umidade),
            "vegetacao" | "vegetação" => Some(Indicador::Vegetacao),
            "spi1" => Some(Indicador::Spi(1)),
            "spi3" => Some(Indicador::Spi(3)),
            "spi6" => Some(Indicador::Spi(6)),
            "spi12" => Some(Indicador::Spi(12)),
            _ => None,
        }
    }

    /// A camada do WMS.
    fn camada_wms(self) -> &'static str {
        match self {
            Indicador::Umidade => "smang",
            Indicador::Vegetacao => "fpanv",
            Indicador::Spi(1 | 3) => "spcST",
            Indicador::Spi(_) => "spcLT",
        }
    }

    /// A escala do SPI no serviço (`01`, `03`…), que ele pede para desenhar.
    fn escala(self) -> Option<String> {
        match self {
            Indicador::Spi(m) => Some(format!("{m:02}")),
            _ => None,
        }
    }

    /// De 10 em 10 dias (dias 1, 11 e 21), ou por mês (o SPI de 6 e 12 meses).
    fn por_mes(self) -> bool {
        matches!(self, Indicador::Spi(6 | 12))
    }

    pub fn titulo(self) -> String {
        match self {
            Indicador::Umidade => "Anomalia da umidade do solo".into(),
            Indicador::Vegetacao => "Anomalia da vegetação (fAPAR)".into(),
            Indicador::Spi(m) => format!("SPI de {m} {} (CHIRPS)", if m == 1 { "mês" } else { "meses" }),
        }
    }

    /// A data do serviço para `d`: o começo do decêndio (dia 1, 11 ou 21) ou do mês.
    pub fn passo(self, d: Data) -> Data {
        let dia = if self.por_mes() {
            1
        } else {
            match d.dia {
                1..=10 => 1,
                11..=20 => 11,
                _ => 21,
            }
        };
        Data { ano: d.ano, mes: d.mes, dia }
    }

    /// O endereço das imagens (Web Mercator, `{bbox-epsg-3857}`), na data `d` ou na mais recente.
    pub fn url(self, d: Option<Data>) -> String {
        let mut u = format!(
            "{WMS}&REQUEST=GetMap&LAYERS={}&STYLES=&SRS=EPSG:3857&BBOX={{bbox-epsg-3857}}&WIDTH=256&HEIGHT=256&FORMAT=image/png&TRANSPARENT=TRUE",
            self.camada_wms()
        );
        if let Some(d) = d {
            u.push_str(&format!("&TIME={:04}-{:02}-{:02}", d.ano, d.mes, d.dia));
        }
        if let Some(e) = self.escala() {
            u.push_str(&format!("&SELECTED_TIMESCALE={e}"));
        }
        u
    }

    /// A legenda do serviço (PNG).
    pub fn legenda(self) -> String {
        let mut u = format!("{WMS}&REQUEST=GetLegendGraphic&LAYER={}&FORMAT=image/png", self.camada_wms());
        if let Some(e) = self.escala() {
            u.push_str(&format!("&SELECTED_TIMESCALE={e}"));
        }
        u
    }

    /// O nome do mapa.
    pub fn camada(self, d: Option<Data>) -> String {
        let quando = d.map(|d| format!("{:02}/{:02}/{}", d.dia, d.mes, d.ano)).unwrap_or_else(|| "mais recente".into());
        format!("{PREFIXO_CAMADA}{} (GDO, {quando})", self.titulo())
    }
}

/// O começo do nome dos mapas do GDO.
pub const PREFIXO_CAMADA: &str = "BrasilData: Seca: ";

/// Se o mapa `nome` é do GDO.
pub fn eh_do_gdo(nome: &str) -> bool {
    nome.starts_with(PREFIXO_CAMADA) && nome.contains("(GDO, ")
}

pub const CITACAO: &str = "Copernicus Emergency Management Service, Global Drought Observatory (GDO), Centro Comum de Investigação da Comissão Europeia; mapa do serviço WMS (drought.emergency.copernicus.eu), não recortado pela área";

pub const LICENCA: &str = "CC BY 4.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_indicadores_e_os_enderecos() {
        let d = Data { ano: 2026, mes: 10, dia: 7 };
        let u = Indicador::Umidade;
        assert_eq!(u.passo(d), Data { ano: 2026, mes: 10, dia: 1 });
        assert!(u.url(Some(u.passo(d))).contains("LAYERS=smang&") && u.url(Some(u.passo(d))).ends_with("&TIME=2026-10-01"));
        assert!(!u.url(None).contains("TIME="));
        let s = Indicador::ler("spi3").unwrap();
        assert_eq!(s.passo(Data { ano: 2026, mes: 11, dia: 25 }).dia, 21);
        assert!(s.url(None).contains("LAYERS=spcST&") && s.url(None).ends_with("&SELECTED_TIMESCALE=03"));
        let l = Indicador::ler("spi12").unwrap();
        assert_eq!(l.passo(Data { ano: 2026, mes: 9, dia: 15 }).dia, 1);
        assert!(l.url(None).contains("LAYERS=spcLT&") && l.legenda().ends_with("&SELECTED_TIMESCALE=12"));
        assert!(Indicador::Vegetacao.url(None).contains("{bbox-epsg-3857}"));
        assert_eq!(Indicador::ler("lfi"), None);
    }

    #[test]
    fn os_nomes() {
        let d = Data { ano: 2026, mes: 10, dia: 1 };
        assert_eq!(Indicador::Umidade.camada(Some(d)), "BrasilData: Seca: Anomalia da umidade do solo (GDO, 01/10/2026)");
        assert_eq!(Indicador::Spi(1).camada(None), "BrasilData: Seca: SPI de 1 mês (CHIRPS) (GDO, mais recente)");
        assert!(eh_do_gdo(&Indicador::Spi(6).camada(None)));
        assert!(!eh_do_gdo("BrasilData: Seca (JAXA)"));
    }
}
