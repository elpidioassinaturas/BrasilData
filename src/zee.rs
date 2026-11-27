//! O Zoneamento Ecológico-Econômico (ZEE) dos estados: as fontes digitais abertas das zonas, as
//! cores de cada uma e, para os estados sem fonte, a situação levantada em 27/11/2026
//! (`PESQUISA_ZEE.md`, a partir da planilha do MMA "Projetos de ZEE concluídos no Brasil, dez/2025").
//!
//! - **Com fonte aberta:** DF (SISDIA), PR (IAT, o estadual e o do litoral), SE (o costeiro, pelo
//!   SIGERCO), MG (IDE-Sisema) e RO (SEDAM, sem CORS: só no desktop). Cada fonte é uma linha do
//!   `fontes.csv`; o comando [`ZEE_DO_ESTADO`] escolhe as da UF da área.
//! - **Sem fonte que funcione:** o aviso diz o porquê (servidor fora do ar, fechado com senha, só
//!   PDFs ou zoneamento não instituído) e a lei, quando há.

use serde_json::{Value, json};

/// O comando que escolhe as fontes pela UF da área.
pub const ZEE_DO_ESTADO: &str = "zee-estado";

/// Uma fonte: o tema do catálogo, o campo das zonas e a cor de cada valor.
pub struct Fonte {
    pub tema: &'static str,
    pub campo: &'static str,
    pub classes: &'static [(&'static str, [u8; 3])],
}

const DF: &[(&str, [u8; 3])] = &[
    ("SZSE-1", [27, 120, 55]),
    ("SZSE-2", [90, 174, 97]),
    ("SZSE-3", [166, 219, 160]),
    ("SZSE-4", [0, 109, 119]),
    ("SZSE-5", [102, 194, 164]),
    ("SZSE-6", [65, 171, 93]),
    ("SZSE-7", [199, 233, 192]),
    ("SZDPE-1", [230, 85, 13]),
    ("SZDPE-2", [253, 141, 60]),
    ("SZDPE-3", [253, 190, 133]),
    ("SZDPE-4", [203, 24, 29]),
    ("SZDPE-5", [251, 106, 74]),
    ("SZDPE-6", [166, 54, 3]),
    ("SZDPE-7", [252, 174, 145]),
];

const PR: &[(&str, [u8; 3])] = &[
    ("1 - Planície Costeira", [141, 211, 199]),
    ("2 - Serra do Mar", [27, 120, 55]),
    ("3 - RMC Sul", [251, 128, 114]),
    ("4 - Campo Largo - Adrianópolis", [190, 186, 218]),
    ("5 - Prudentópolis - Jaguariaíva", [128, 177, 211]),
    ("6 - Ortiguera - Ibaiti", [253, 180, 98]),
    ("7 - União da Vitória - São Mateus do Sul", [179, 222, 105]),
    ("8 - Guarapuava - Pitanga", [252, 205, 229]),
    ("9 - Palmital - General Carneiro - Céu Azul", [188, 128, 189]),
    ("10 - Realeza - Palmas", [204, 235, 197]),
    ("11 - Foz do Iguaçu - Londrina", [255, 237, 111]),
    ("12 - Paranavaí - Umuarama", [217, 217, 217]),
];

const PR_LITORAL: &[(&str, [u8; 3])] = &[
    ("ZU", [227, 26, 28]),
    ("ZDTO", [254, 196, 79]),
    ("ZDD", [217, 160, 102]),
    ("ZEPI", [0, 90, 50]),
    ("ZPL", [116, 196, 118]),
    ("ZRMC", [200, 200, 200]),
    ("ZVR", [170, 170, 170]),
    ("Massa de Água", [158, 202, 225]),
];

const SE_MAR: &[(&str, [u8; 3])] =
    &[("ZPRC", [251, 154, 153]), ("ZPP", [166, 206, 227]), ("ZUS", [102, 194, 164]), ("ZPRM", [31, 120, 180]), ("ZUEM", [158, 154, 200])];

const RO: &[(&str, [u8; 3])] = &[
    ("ZONA 1.1", [254, 217, 118]),
    ("ZONA 1.2", [254, 178, 76]),
    ("ZONA 1.3", [253, 141, 60]),
    ("ZONA 1.4", [240, 59, 32]),
    ("ZONA 2.1", [199, 233, 192]),
    ("ZONA 2.2", [161, 217, 155]),
    ("ZONA 3.1", [65, 171, 93]),
    ("ZONA 3.2", [0, 90, 50]),
    ("ZONA 3.3", [191, 129, 45]),
    ("RIOS", [158, 202, 225]),
    ("LAGOAS", [158, 202, 225]),
    ("RESERVAT", [158, 202, 225]),
    ("ILHAS", [235, 225, 200]),
];

const MG: &[(&str, [u8; 3])] = &[
    ("Zona Ecológico-Econômica 1", [26, 150, 65]),
    ("Zona Ecológico-Econômica 2", [166, 217, 106]),
    ("Zona Ecológico-Econômica 3", [255, 255, 191]),
    ("Zona Ecológico-Econômica 4", [253, 174, 97]),
    ("Zona Ecológico-Econômica 5", [244, 109, 67]),
    ("Zona Ecológico-Econômica 6", [215, 25, 28]),
];

/// As fontes com as zonas.
pub const FONTES: &[Fonte] = &[
    Fonte { tema: "zee-df", campo: "codigo_subzona", classes: DF },
    Fonte { tema: "zee-pr", campo: "zona", classes: PR },
    Fonte { tema: "zee-pr-litoral", campo: "sigla", classes: PR_LITORAL },
    Fonte { tema: "zee-se-maritimo", campo: "class_zee", classes: SE_MAR },
    Fonte { tema: "zee-ro", campo: "SUBZONA", classes: RO },
    Fonte { tema: "mg-zee-zonas", campo: "indicador", classes: MG },
];

/// A fonte do tema `id`.
pub fn fonte(id: &str) -> Option<&'static Fonte> {
    FONTES.iter().find(|f| f.tema == id)
}

/// Os temas com as zonas da UF `uf` (a sigla), na ordem em que entram no mapa (o mar por baixo).
pub fn temas_da_uf(uf: &str) -> &'static [&'static str] {
    match uf {
        "DF" => &["zee-df"],
        "PR" => &["zee-pr", "zee-pr-litoral"],
        "SE" => &["zee-se-maritimo"],
        "MG" => &["mg-zee-zonas"],
        "RO" => &["zee-ro"],
        _ => &[],
    }
}

/// O que falta das zonas de uma UF que tem fonte, para o aviso.
pub fn nota(uf: &str) -> Option<&'static str> {
    match uf {
        "SE" => Some(
            "SE: só as zonas marítimas do ZEE Costeiro; as terrestres (6 polígonos dissolvidos com o litoral inteiro, mais de 130 MB no SIGERCO) são pesadas demais para o plugin",
        ),
        "MG" => Some("MG: o ZEE-MG é uma grade convertida em polígonos (122 mil no estado): numa área grande, escolha uma menor"),
        _ => None,
    }
}

/// Por que a UF `uf` não tem as zonas no plugin: o que existe e onde está (27/11/2026).
pub fn situacao(uf: &str) -> &'static str {
    match uf {
        "AC" => "ZEE Fase II (Lei 1.904/2007, 1:250.000): o repositório da SEMA-AC só tem PDFs",
        "AM" => "MacroZEE (Lei 3.417/2009) e ZEE do Purus (Lei 3.645/2011): só PDFs, na SEDECTI",
        "AP" => "ZEE do Amapá (Lei 3.208/2025, 1:250.000): sem as zonas em formato digital aberto",
        "PA" => "MacroZEE (Lei 6.745/2005) e ZEEs da Zona Oeste e da Zona Leste e Calha Norte: sem fonte digital aberta",
        "RR" => "ZEE-RR (LC 323/2022): só em arquivos do portal zee.rr.gov.br, ainda não lidos pelo plugin",
        "TO" => "só o ZEE do Norte do Tocantins (Lei 2.656/2012), em RAR; o ZEE estadual não está aprovado",
        "MA" => {
            "ZEE do bioma Amazônico (Lei 11.269/2020) e do Cerrado e Costeiro (Lei 11.734/2022): só pela API do geoportal do ZEE-MA, ainda não lida pelo plugin"
        }
        "PI" => "MacroZEE de 2014, sem norma; o ZEE dos Cerrados tramita na Assembleia",
        "CE" => "ZEE Costeiro (Lei 19.294/2025): no GeoServer da PEDEA (SEMA-CE), fora do ar em 27/11/2026",
        "RN" => "ZEE Costeiro do Litoral Oriental (Lei 7.871/2000): sem fonte digital aberta (o IDEMA estava fora do ar)",
        "PB" => "sem ZEE concluído",
        "PE" => {
            "ZEE Costeiro do Litoral Sul e Norte: só em mosaicos vetoriais da Plataforma Ecológico-Econômica da CPRH, ainda não lidos pelo plugin"
        }
        "AL" => "sem ZEE instituído",
        "BA" => "ZEE-BA (1:250.000), à espera de decreto: no ArcGIS da CONDER, fora do ar em 27/11/2026",
        "SP" => {
            "ZEE-SP (Decreto 67.430/2022) no DataGeo, fora do ar em 27/11/2026; os ZEE Costeiros do Litoral Norte e da Baixada Santista só em shapefile"
        }
        "RJ" => "ZEE-RJ não instituído; os mapas do INEA saíram do ar (assinatura do ArcGIS Online cancelada)",
        "ES" => "ZEE-ES não instituído; o ZEE Costeiro está em elaboração",
        "SC" => "sem ZEE instituído",
        "RS" => "ZEE-RS (2018–2019) não instituído; o servidor do ZEE (zee.rs.gov.br) pede senha",
        "MS" => "ZEE-MS (Lei 3.839/2009, 1:1.000.000): no SISLA do IMASUL, fora do ar em 27/11/2026",
        "MT" => "ZSEE-MT (Lei 9.523/2011) suspenso por liminar desde 2012; o servidor da SEMA-MT pede senha",
        "GO" => "sem ZEE aprovado (o MacroZAEE de 2014 é indicativo); o SIEG estava fora do ar",
        _ => "sem fonte digital conhecida",
    }
}

/// O estilo do tema `id`: polígonos na cor de cada zona, com o rótulo da zona.
pub fn estilo_do_tema(id: &str) -> Option<(String, String)> {
    let f = fonte(id)?;
    let nome = format!("BrasilData: {id} (zonas do ZEE)");
    let classes: Vec<Value> = f
        .classes
        .iter()
        .map(|(v, c)| json!([v, {"fill": [c[0], c[1], c[2], 190], "stroke": [90, 90, 90, 200], "stroke_width": 0.4, "size": 0.0, "marker": "Circle"}]))
        .collect();
    // Os 122 mil polígonos do ZEE-MG (uma grade convertida) não levam rótulo.
    let rotulos = id != "mg-zee-zonas";
    let texto = json!([{"name": nome, "geometry": "Polygon",
        "renderer": {"Unique": {"field": f.campo, "classes": classes,
            "other": {"fill": [200, 200, 200, 150], "stroke": [110, 110, 110, 160], "stroke_width": 0.3, "size": 0.0, "marker": "Circle"},
            "ramp": "Harbour"}},
        "labels": {"enabled": rotulos, "field": f.campo, "size": 7.5, "color": [40, 40, 40, 255], "halo": true, "bold": false,
            "min_scale": null, "max_scale": null, "closest": null, "farthest": null, "placement": "Center"},
        "transparency": 0.0}])
    .to_string();
    Some((nome, texto))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cada_uf_tem_fonte_ou_situacao() {
        for uf in crate::municipios::ufs().iter().map(|u| u.0) {
            let temas = temas_da_uf(uf);
            assert!(!temas.is_empty() || situacao(uf) != "sem fonte digital conhecida", "{uf}");
            for t in temas {
                assert!(fonte(t).is_some(), "{t}");
                assert!(crate::TEMAS.iter().any(|x| x.id == *t), "{t} fora do fontes.csv");
            }
        }
    }

    #[test]
    fn o_estilo_tem_as_classes() {
        let (nome, texto) = estilo_do_tema("zee-df").unwrap();
        assert!(nome.contains("zee-df"));
        let v: Value = serde_json::from_str(&texto).unwrap();
        assert_eq!(v[0]["renderer"]["Unique"]["field"], "codigo_subzona");
        assert_eq!(v[0]["renderer"]["Unique"]["classes"].as_array().unwrap().len(), 14);
        assert_eq!(v[0]["labels"]["enabled"], true);
        let (_, mg) = estilo_do_tema("mg-zee-zonas").unwrap();
        assert!(mg.contains("\"enabled\":false"));
        assert!(estilo_do_tema("ucs").is_none());
    }
}
