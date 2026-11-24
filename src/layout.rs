//! O layout do mapa: uma folha pronta (`layout.publication` do RustGIS) com o título da área e os
//! textos do dado principal, a "receita" de cada camada do BrasilData.
//!
//! - **Qual dado:** a primeira camada visível do BrasilData, de cima para baixo, que tem receita;
//!   a área, a grade e o desenho não contam.
//! - **A receita:** o subtítulo (o que é o dado e de quando), a barra de cores (o título com a
//!   unidade e quantos valores ela mostra) ou a legenda das classes, e a nota.
//! - **O resto vem do plugin:** o título é o nome da área, as fontes são as citações da tabela
//!   Fontes, e a projeção é a da grade do projeto (ou a automática da área).

use crate::gedi;
use crate::grade::GRADE;
use crate::{AREA, PREFIXO};

/// O que o layout diz de uma camada.
#[derive(Debug, PartialEq)]
pub struct Receita {
    /// Abaixo do título: o que é o dado e de quando.
    pub subtitulo: String,
    /// A barra de cores de um raster contínuo: o título (com a unidade) e quantos valores ela
    /// mostra, as pontas incluídas. Sem barra, a camada vai para a legenda.
    pub barra: Option<(String, usize)>,
    /// O título da legenda, quando a camada vai para ela com classes nomeadas (sem ele, a legenda
    /// fica sem título: um título a mais tira uma linha da caixa).
    pub legenda: Option<&'static str>,
    pub nota: Option<&'static str>,
}

const NOTA_GEDI: &str = "Valores imputados a 30 m (cada pixel recebe o disparo do GEDI mais parecido pelas séries do Landsat): mostram padrões, não medidas locais.";
const NOTA_SOL: &str =
    "Células de 1° (cerca de 110 km) do NASA POWER interpoladas para a grade: mostram a variação regional, não detalhes locais.";
const NOTA_VENTO: &str = "Células de 0,625° × 0,5° (cerca de 55 × 65 km) da reanálise MERRA-2, interpoladas para a grade: mostram a variação regional, não o vento de cada lugar.";
const NOTA_EMIT: &str = "Contornos das cenas da refletância hiperespectral (60 m) do EMIT, na Estação Espacial Internacional; onde elas se sobrepõem, aparece a menos nublada. A nuvem é a fração da cena inteira, não de cada lugar dela.";
const NOTA_POWER: &str = "Células de 0,625° × 0,5° (cerca de 55 × 65 km) da reanálise MERRA-2, interpoladas para a grade: mostram a variação regional, não o valor de cada lugar.";
const NOTA_IRRIGACAO: &str =
    "Porcentagem de cada célula de 5′ (cerca de 9 km) equipada para irrigação por volta de 2005; a área de fato irrigada pode ser menor.";
const NOTA_AQUEDUCT: &str = "Por sub-bacia (HydroBASINS, nível 6) e unidade administrativa: a retirada anual de água para todos os usos dividida pela oferta renovável de superfície e subterrânea.";
const NOTA_IRRIGACAO_MAPBIOMAS: &str = "Cores da legenda oficial do MapBiomas: roxo, pivô central; azul-claro, outros sistemas de irrigação; rosa, inundação (arroz irrigado). Mapa do Brasil inteiro, não recortado pela área.";
const NOTA_COBERTURA_MAPBIOMAS: &str =
    "Cores da legenda oficial do MapBiomas (brasil.mapbiomas.org). Mapa do Brasil inteiro, não recortado pela área.";
const NOTA_RISCO_FOGO: &str = "Perigo meteorológico de fogo do modelo do INPE (versão 11), não a ocorrência de fogo. Classes do INPE: mínimo, abaixo de 0,15; baixo, de 0,15 a 0,40; médio, de 0,40 a 0,70; alto, de 0,70 a 0,95; crítico, acima de 0,95.";
const NOTA_JAXA: &str =
    "Dados mensais do JAXA Earth na resolução original, em células de 2,8 a 22 km: mostram o padrão regional, não o detalhe de cada lugar.";
const NOTA_ERA5: &str = "Reanálise ERA5 do ECMWF (Copernicus), em nós de uma grade de 0,25° (cerca de 28 km) ou mais aberta, interpolados para a grade: mostra o clima regional, não o de cada encosta ou vale.";
const NOTA_SEM_NUVENS: &str = "O melhor pixel das datas de 16 dias do cubo de dados do INPE (Brazil Data Cube): cada célula vem da primeira data limpa, da menos nublada na área para a mais nublada; bordas de nuvem e sombras finas ainda podem aparecer.";
const NOTA_BDC: &str = "Compostos de 16 dias do cubo de dados do INPE (Brazil Data Cube), com o pixel mais limpo de cada período; nuvens e sombras persistentes ainda podem aparecer.";
const NOTA_CTREES: &str = "Biomassa estimada por um modelo de IA calibrado com lidar aéreo e parcelas de campo, a partir do Landsat, do radar ALOS PALSAR-2 e das alturas do GEDI; a série da CTrees ainda está em pré-publicação, sem revisão por pares.";
const NOTA_FORTY: &str = "Códigos da legenda: 1 floresta primária, 2 em regeneração natural, 3 plantada, 4 plantio florestal comercial, 5 culturas arbóreas e agrofloresta, 6 outros usos. Classes de um modelo de IA sobre imagens de 2020, alinhadas à FAO e ao EUDR.";
const NOTA_WORLDCOVER: &str = "Códigos da legenda: 10 árvores, 20 arbustos, 30 campo e pastagem, 40 cultivos anuais, 50 área construída, 60 solo exposto ou vegetação esparsa, 80 água, 90 área úmida herbácea, 95 manguezal. Mapa mundial feito com o Sentinel-1 e o Sentinel-2; os de 2020 e 2021 usam versões diferentes do algoritmo e não se subtraem.";
const NOTA_OLM: &str = "Propriedade do solo prevista a 30 m por aprendizado de máquina sobre amostras de solo do mundo inteiro (OpenLandMap-soildb): mostra padrões, não substitui a análise de laboratório; o Brasil tem menos amostras que a Europa e os Estados Unidos.";
const NOTA_GEDTM: &str = "Derivada do GEDTM30, um modelo do terreno a 30 m feito por aprendizado de máquina com os pontos de laser do ICESat-2 e do GEDI: sob a mata o terreno é estimado, não medido.";
const NOTA_GEOMORFONS: &str = "Códigos da legenda: 1 plano, 2 pico, 3 crista, 4 ombro, 5 esporão, 6 encosta, 7 concavidade, 8 sopé, 9 vale, 10 depressão. Formas do relevo do GEDTM30 (OpenGeoHub), um terreno feito por aprendizado de máquina com o laser do ICESat-2 e do GEDI.";
const NOTA_ANADEM: &str = "Terreno nu: o Copernicus GLO-30 sem o viés da vegetação, removido por aprendizado de máquina calibrado para a América do Sul (ANA e IPH/UFRGS). Altitudes em metros.";
const NOTA_GDO: &str = "Mapa do serviço do Global Drought Observatory (Copernicus), nas cores do GDO: os tons de marrom são mais seco que o normal, os de azul, mais úmido.";
const NOTA_GBA: &str = "Polígonos do Google, da Microsoft e do OpenStreetMap; altura estimada por um modelo de IA, menos precisa na América do Sul. Licença CC BY-NC 4.0: sem uso comercial.";
const NOTA_CHM_AMAZONIA: &str =
    "Altura estimada por um modelo de IA (U-Net) sobre imagens Planet NICFI, calibrado com lidar aéreo da Amazônia; média de 2020 a 2024.";
const NOTA_CHM: &str = "Altura estimada por um modelo de IA (DINOv3) sobre imagens de satélite de alta resolução de datas diferentes; 0 é sem árvore (ou sem dado).";
const NOTA_GEM15: &str = "De imagens estéreo de satélite adquiridas até 2024; alturas no geoide EGM96. Água e superfícies sem textura ficam sem dado; sob mata densa, o terreno é estimado.";
const NOTA_PREVISAO: &str = "Previsão de uma rodada de um modelo global, em nós de uma grade de 0,25° (cerca de 28 km) interpolados para a grade: mostra a tendência regional, não a chuva de cada lugar, e muda a cada rodada.";
const NOTA_ENSEMBLE: &str = "Resumo dos membros de um ensemble (o mesmo modelo rodado com pequenas diferenças) em nós de uma grade de 0,25° a 1° interpolados: a chance é a fração dos membros que passa do limiar, não uma probabilidade calibrada.";
const NOTA_CHIRPS: &str = "Estimativa por satélite (infravermelho) corrigida com estações (CHIRPS v3, 0,05°, cerca de 5,5 km); onde há poucas estações, erra mais nas chuvas fortes e no relevo.";
const NOTA_PARDIG: &str = "Taxa de chuva instantânea estimada por IA a partir de satélites (PARDIG, Google DeepMind, 0,1°, cerca de 10 km, a cada 15 minutos); dado experimental, satura em 75 mm/h e erra mais no relevo e na garoa.";
const NOTA_DIST: &str = "Alertas acumulados desde o início do monitoramento (04/01/2026). A primeira detecção ainda não foi confirmada e inclui mudanças sazonais.";

/// A data no fim de um nome do DIST ("…, até 28/02/2026)").
fn data_no_nome(n: &str) -> Option<&str> {
    let i = n.rfind("até ")? + "até ".len();
    n.get(i..i + 10)
}

/// A nota da composição colorida do NISAR.
const NOTA_NISAR_TEMPO: &str = "Vermelho: mediana do HH (−13 a −1 dB); verde: mediana do HV (−19 a −8 dB); azul: desvio-padrão do HV entre as passagens (0 a 3 dB). Verde: mata e vegetação estável; azul e lilás: vegetação que mudou (lavoura, pasto); vermelho e laranja: solo exposto, superfícies ásperas e construções; escuro: água e sombras do relevo.";

/// A nota da composição colorida do radar de 10 m ([`crate::radar_pc`]).
const NOTA_RADAR_10M_COR: &str = "Vermelho: VV (−15 a −3 dB); verde: VH (−21 a −10 dB); azul: VV − VH (3 a 9 dB). Cidade e construções em rosa e magenta, mata e cerrado em amarelo e verde, lavoura e solo exposto em lilás, roxo e azul, água em preto. A posição não é exata: em áreas planas, o dado ficou de 13 a 33 m a oeste do Sentinel-2.";
const NOTA_RADAR_10M: &str = "A posição não é exata: em oito áreas planas do Brasil, o dado ficou de 13 a 33 m a oeste do Sentinel-2 (mediana de 18 m), e no relevo forte o desvio muda de um lugar para outro.";
const NOTA_LUZES: &str = "Número relativo, sem unidade. A série usa o satélite DMSP até 2013 e o VIIRS depois, corrigidos um pelo outro: de 2013 para 2014 as células acesas do Brasil sobem 23%, e boa parte disso deve vir da troca.";
const NOTA_METANO: &str = "Metano na coluna de ar, não a emissão de cada lugar: o valor cai com a altitude (há menos ar acima) e varia com a estação. Grade de 0,1° (cerca de 11 km) calculada pelo BrasilData dos pixels de boa qualidade do satélite.";

const NOTA_NISAR_COR: &str = "Vermelho: HH (−12,7 a 4,3 dB); verde: HV (−18 a −5,3 dB); azul: HH − HV (0,8 a 13,4 dB). Mata em verde forte (volume das copas), cidade em rosa e magenta (eco duplo das construções), pasto e solo exposto em azul e roxo escuros, água escura.";

/// A receita da camada `nome`, se o layout a conhece.
/// O nome antes do período e o período ("01/06/2025 a 31/08/2025"), cortados no " de " seguido de
/// uma data: "Temperatura da superfície de dia média de 01/08/2026 a …" tem outro " de " antes.
fn resumo_e_periodo(n: &str) -> (&str, Option<&str>) {
    let corte = n.match_indices(" de ").map(|(i, _)| i).find(|&i| n[i + 4..].starts_with(|c: char| c.is_ascii_digit()));
    match corte {
        Some(i) => (&n[..i], n[i + 4..].split_once(" (").map(|(p, _)| p)),
        None => (n, None),
    }
}

/// ", de 01/06/2025 a 31/08/2025" de um nome do cubo do INPE ou do JAXA, ou nada.
fn periodo_no_nome(n: &str) -> String {
    resumo_e_periodo(n).1.map(|p| format!(", de {p}")).unwrap_or_default()
}

pub fn receita(nome: &str) -> Option<Receita> {
    let n = nome.strip_prefix(PREFIXO)?;
    if [AREA, GRADE].contains(&nome) {
        return None;
    }
    if let Some(v) = gedi::Variavel::TODAS.into_iter().find(|v| v.camada() == nome) {
        let (lo, hi) = v.escala();
        let passo = match v {
            gedi::Variavel::Altura => 5.0,
            gedi::Variavel::Biomassa => 50.0,
            gedi::Variavel::Cobertura => 25.0,
        };
        return Some(Receita {
            subtitulo: format!("{} pelo laser GEDI (NASA), 2019–2023", v.titulo()),
            barra: Some((format!("{} ({})", v.titulo(), v.unidade()), ((hi - lo) / passo) as usize + 1)),
            legenda: None,
            nota: Some(NOTA_GEDI),
        });
    }
    if n.starts_with("Perturbação da vegetação") && n.contains("OPERA DIST-S1") {
        let quando = data_no_nome(n).map(|d| format!(", situação em {d}")).unwrap_or_default();
        return Some(Receita {
            subtitulo: format!("Alertas de perturbação da vegetação pelo radar Sentinel-1 (OPERA DIST-S1){quando}"),
            barra: None,
            legenda: Some("Situação do alerta"),
            nota: Some(NOTA_DIST),
        });
    }
    for (inicio, titulo, barra) in [
        ("Perturbação, dias antes de", "Dias desde a primeira detecção da perturbação", "Dias antes do produto"),
        ("Confiança da perturbação", "Confiança dos alertas de perturbação", "Confiança"),
        ("Intensidade da perturbação", "Intensidade da perturbação da vegetação", "Intensidade"),
    ] {
        if n.starts_with(inicio) && n.contains("OPERA DIST-S1") {
            return Some(Receita {
                subtitulo: format!("{titulo} pelo radar Sentinel-1 (OPERA DIST-S1)"),
                barra: Some((barra.to_string(), 5)),
                legenda: None,
                nota: Some(NOTA_DIST),
            });
        }
    }
    if let Some(mes) = crate::risco_fogo::mes_da_camada(nome) {
        return Some(Receita {
            subtitulo: format!("Risco de fogo em {}: média de 2001 a 2025 (INPE, Programa Queimadas)", crate::risco_fogo::nome_do_mes(mes)),
            barra: Some(("Risco de fogo (0 a 1)".into(), 11)),
            legenda: None,
            nota: Some(NOTA_RISCO_FOGO),
        });
    }
    if crate::sol::eh_do_sol(nome) {
        let periodo = crate::sol::periodo(crate::sol::mes_da_camada(nome));
        return Some(Receita {
            subtitulo: format!("Radiação solar na superfície, {periodo} de 2001 a 2025 (NASA POWER)"),
            barra: Some(("Radiação solar (kWh/m² por dia)".into(), 6)),
            legenda: None,
            nota: Some(NOTA_SOL),
        });
    }
    if crate::emit::eh_do_emit(nome) {
        return Some(Receita {
            subtitulo: format!("Cenas do EMIT{} (NASA)", crate::emit::quais(nome)),
            barra: None,
            // Sem título: ele tiraria da caixa as linhas das classes.
            legenda: None,
            nota: Some(NOTA_EMIT),
        });
    }
    if crate::vento::eh_do_vento(nome) {
        let periodo = crate::vento::periodo(crate::vento::mes_da_camada(nome));
        return Some(Receita {
            subtitulo: format!("Vento a 50 m de altura, {periodo} de 2001 a 2025 (NASA POWER, MERRA-2)"),
            barra: Some(("Velocidade do vento (m/s)".into(), 6)),
            legenda: None,
            nota: Some(NOTA_VENTO),
        });
    }
    if let Some((v, anomalia, periodo)) = crate::power::ler_camada(nome) {
        let quando = match (anomalia, periodo.starts_with("média")) {
            (true, _) => format!("{periodo}, diferença da média de 2001 a 2025"),
            (false, true) => format!("{periodo} de 2001 a 2025"),
            (false, false) => periodo.to_string(),
        };
        let (titulo, marcas) = v.barra(anomalia);
        return Some(Receita {
            subtitulo: format!("{}, {quando} (NASA POWER, MERRA-2)", v.nome(false)),
            barra: Some((titulo.into(), marcas)),
            legenda: None,
            nota: Some(NOTA_POWER),
        });
    }
    if crate::tempo::eh_da_previsao(nome) {
        let ensemble = n.starts_with("Chance de chuva") || n.starts_with("Chuva do ensemble");
        let barra = if n.starts_with("Chance de chuva") {
            ("Chance (%)".to_string(), 6)
        } else if n.starts_with("Temperatura") {
            ("Temperatura (°C)".to_string(), 5)
        } else {
            (
                "Chuva (mm)".to_string(),
                crate::tempo::marcas_da_chuva(crate::tempo::escala_chuva(crate::tempo::dias_da_camada(nome).unwrap_or(15))),
            )
        };
        return Some(Receita {
            subtitulo: n.to_string(),
            barra: Some(barra),
            legenda: None,
            nota: Some(if ensemble { NOTA_ENSEMBLE } else { NOTA_PREVISAO }),
        });
    }
    if crate::tempo::eh_do_chirps(nome) {
        return Some(Receita {
            subtitulo: n.to_string(),
            barra: Some(("Chuva no mês (mm)".into(), crate::tempo::marcas_da_chuva(crate::tempo::ESCALA_CHIRPS))),
            legenda: None,
            nota: Some(NOTA_CHIRPS),
        });
    }
    if crate::pardig::eh_do_pardig(nome) {
        return Some(Receita {
            subtitulo: n.to_string(),
            barra: Some(("Taxa de chuva (mm/h)".into(), 6)),
            legenda: None,
            nota: Some(NOTA_PARDIG),
        });
    }
    if nome == crate::irrigacao::CAMADA {
        return Some(Receita {
            subtitulo: "Área equipada para irrigação, por volta de 2005 (GMIA v5, FAO)".into(),
            barra: Some(("Área equipada para irrigação (% da célula)".into(), 5)),
            legenda: None,
            nota: Some(NOTA_IRRIGACAO),
        });
    }
    if let Some(c) = crate::era5::comando_da_camada(nome) {
        let resumo = resumo_e_periodo(n).0;
        let marcas = match c {
            "era5-temperatura" => 5,
            _ => 6,
        };
        return Some(Receita {
            subtitulo: format!("{resumo} pela reanálise ERA5 (ECMWF/Copernicus, valores diários){}", periodo_no_nome(n)),
            barra: Some((format!("{resumo} ({})", crate::era5::unidade_da_camada(nome)), marcas)),
            legenda: None,
            nota: Some(NOTA_ERA5),
        });
    }
    if let Some(v) = crate::jaxa::variavel_da_camada(nome) {
        let resumo = resumo_e_periodo(n).0;
        let unidade = if v.unidade().is_empty() { String::new() } else { format!(" ({})", v.unidade()) };
        return Some(Receita {
            subtitulo: format!("{resumo} pelo JAXA Earth ({}, meses){}", v.fonte(), periodo_no_nome(n)),
            barra: Some((format!("{}{unidade}", resumo), 8)),
            legenda: None,
            nota: Some(NOTA_JAXA),
        });
    }
    if let Some(s) = crate::bdc::satelite_da_camada(nome)
        && crate::sem_nuvens::eh_sem_nuvens(nome)
    {
        // "Imagem sem nuvens em cor verdadeira de … a …", ou a data de origem dela.
        let resumo = resumo_e_periodo(n).0;
        let origem = resumo.starts_with("Data de origem");
        return Some(Receita {
            subtitulo: format!("{} pelo cubo de dados do INPE ({}, 16 dias){}", resumo, s.nome(), periodo_no_nome(n)),
            barra: None,
            legenda: origem.then_some("Data de origem"),
            nota: Some(NOTA_SEM_NUVENS),
        });
    }
    if let Some(s) = crate::bdc::satelite_da_camada(nome)
        && !crate::bdc::eh_serie(nome)
    {
        // "NDVI médio de 01/06/2025 a 31/08/2025": o índice é a primeira ou a última palavra
        // antes do período ("Desvio do NDVI").
        let resumo = resumo_e_periodo(n).0;
        let indice = if resumo.contains("EVI") { "EVI" } else { "NDVI" };
        return Some(Receita {
            subtitulo: format!("{} pelo cubo de dados do INPE ({}, 16 dias){}", resumo, s.nome(), periodo_no_nome(n)),
            barra: Some((indice.to_string(), 10)),
            legenda: None,
            nota: Some(NOTA_BDC),
        });
    }
    if crate::gdo::eh_do_gdo(nome) {
        let o_que = n.trim_start_matches("Seca: ");
        return Some(Receita {
            subtitulo: o_que.replace("(GDO, ", "(Global Drought Observatory, Copernicus, "),
            barra: None,
            legenda: None,
            nota: Some(NOTA_GDO),
        });
    }
    if crate::gba::eh_do_gba(nome) {
        return Some(Receita {
            subtitulo: "Edificações e alturas (GlobalBuildingAtlas, imagens PlanetScope de 2019)".into(),
            barra: None,
            legenda: Some("Altura da edificação"),
            nota: Some(NOTA_GBA),
        });
    }
    if nome == crate::chm_amazonia::CAMADA {
        return Some(Receita {
            subtitulo: "Altura do dossel a 4,78 m, média de 2020 a 2024 (CTrees, Planet NICFI)".into(),
            barra: Some(("Altura do dossel (m)".into(), 8)),
            legenda: None,
            nota: Some(NOTA_CHM_AMAZONIA),
        });
    }
    if nome == crate::chm::CAMADA {
        return Some(Receita {
            subtitulo: "Altura do dossel a cerca de 1 m (Meta e WRI, CHMv2)".into(),
            barra: Some(("Altura do dossel (m)".into(), 8)),
            legenda: None,
            nota: Some(NOTA_CHM),
        });
    }
    if let Some((v, ano)) = crate::ctrees::da_camada(nome) {
        let (subtitulo, barra) = match v {
            crate::ctrees::Variavel::Biomassa => (format!("Biomassa acima do solo em {ano} a 100 m (CTrees)"), "Biomassa (Mg/ha)"),
            crate::ctrees::Variavel::Incerteza => (format!("Incerteza da biomassa acima do solo em {ano} (CTrees)"), "Erro-padrão (Mg/ha)"),
        };
        return Some(Receita { subtitulo, barra: Some((barra.into(), 5)), legenda: None, nota: Some(NOTA_CTREES) });
    }
    if let Some((p, prof, per)) = crate::olm::da_camada(nome) {
        let un = if p == crate::olm::Propriedade::Ph { String::new() } else { format!(" ({})", p.unidade()) };
        return Some(Receita {
            subtitulo: format!("{} de {} a {} cm, {}–{}, a 30 m (OpenLandMap-soildb)", p.nome(), prof.0, prof.1, per.0, per.1),
            barra: Some((format!("{}{un}", p.nome()), 5)),
            legenda: None,
            nota: Some(NOTA_OLM),
        });
    }
    if nome == crate::anadem::CAMADA {
        return Some(Receita {
            subtitulo: "Terreno nu a 30 m (ANADEM, ANA e IPH/UFRGS)".into(),
            barra: Some(("Altitude (m)".into(), 5)),
            legenda: None,
            nota: Some(NOTA_ANADEM),
        });
    }
    if let Some((v, e)) = crate::gedtm::da_camada(nome) {
        if v.classes() {
            return Some(Receita {
                subtitulo: format!("Geomorfons na escala de {e} m (GEDTM30, OpenGeoHub)"),
                barra: None,
                legenda: Some("Forma do relevo (código)"),
                nota: Some(NOTA_GEOMORFONS),
            });
        }
        let un = if v.unidade().is_empty() { String::new() } else { format!(" ({})", v.unidade()) };
        return Some(Receita {
            subtitulo: format!("{} na escala de {e} m (GEDTM30, OpenGeoHub)", v.nome()),
            barra: Some((format!("{}{un}", v.nome()), 5)),
            legenda: None,
            nota: Some(NOTA_GEDTM),
        });
    }
    if nome == crate::forty::CAMADA {
        return Some(Receita {
            subtitulo: "Tipologia florestal em 2020 a 10 m (ForTy: Google DeepMind, WRI e IIASA)".into(),
            barra: None,
            legenda: Some("Tipo de floresta (código)"),
            nota: Some(NOTA_FORTY),
        });
    }
    if let Some(ano) = crate::worldcover::ano_da_camada(nome) {
        return Some(Receita {
            subtitulo: format!("Cobertura da terra em {ano} a 10 m (ESA WorldCover)"),
            barra: None,
            // Sem título: ele tiraria linhas da caixa, e as últimas classes sumiriam.
            legenda: None,
            nota: Some(NOTA_WORLDCOVER),
        });
    }
    if let Some(m) = crate::gem15::Modelo::TODOS.into_iter().find(|m| m.camada() == nome) {
        let barra = if m == crate::gem15::Modelo::Altura { "Altura acima do terreno (m)" } else { "Altitude (m)" };
        return Some(Receita {
            subtitulo: format!("{} a 15 m (Google GEM-15, experimental)", m.titulo()),
            barra: Some((barra.into(), 5)),
            legenda: None,
            nota: Some(NOTA_GEM15),
        });
    }
    if n == "Elevação (Copernicus GLO-30)" {
        return Some(Receita {
            subtitulo: "Elevação do terreno (Copernicus DEM GLO-30)".into(),
            barra: Some(("Altitude (m)".into(), 5)),
            legenda: None,
            nota: None,
        });
    }
    if crate::nisar::eh_do_nisar(nome) {
        // "Radar HV em dB (NISAR, 01/11/2026)": a polarização e a data da passagem.
        let qual = n.split(" (").next().unwrap_or(n).trim_end_matches(" em dB");
        let quando = n.rsplit(", ").next().map(|d| d.trim_end_matches(')')).filter(|d| d.len() == 10);
        let quando = quando.map(|d| format!(" em {d}")).unwrap_or_default();
        if qual.starts_with("Composição temporal") {
            let periodo = n.rsplit("(NISAR, ").next().map(|p| p.trim_end_matches(')')).unwrap_or_default();
            return Some(Receita {
                subtitulo: format!("Composição temporal do radar NISAR (banda L), {periodo}: medianas do HH e do HV e variação do HV"),
                barra: None,
                legenda: None,
                nota: Some(NOTA_NISAR_TEMPO),
            });
        }
        if qual.starts_with("Composição") {
            return Some(Receita {
                subtitulo: format!("Composição colorida do radar NISAR (banda L){quando}: HH, HV e HH − HV"),
                barra: None,
                legenda: None,
                nota: Some(NOTA_NISAR_COR),
            });
        }
        // As escalas fixas do plugin: 10 dB nas polarizações (marcas de 2 dB) e 7 dB na razão.
        let marcas = if qual.starts_with("Razão") { 8 } else { 6 };
        return Some(Receita {
            subtitulo: format!("{qual} do NISAR (banda L){quando}, corrigido do relevo (GCOV)"),
            barra: Some((format!("{qual} (dB)"), marcas)),
            legenda: None,
            nota: None,
        });
    }
    if crate::radar_pc::eh_daqui(nome) {
        // "Radar VV 10 m, mediana de 10 passagens (Sentinel-1, 03/08/2026 a 19/11/2026)": antes do
        // OPERA, que tem o mesmo começo.
        let (qual, resto) = n.split_once(" 10 m, ").unwrap_or((n, ""));
        let (quantas, periodo) = resto.split_once(" (Sentinel-1, ").unwrap_or((resto, ""));
        let periodo = periodo.trim_end_matches(')');
        let cor = composicao(nome);
        return Some(Receita {
            subtitulo: if cor {
                format!("Composição colorida do radar Sentinel-1 (VV, VH, VV/VH), {quantas}, {periodo} (Planetary Computer)")
            } else {
                format!("{qual} do Sentinel-1, {quantas}, {periodo}, corrigido do relevo (Planetary Computer)")
            },
            barra: (!cor).then(|| (format!("{qual} (dB)"), if qual.ends_with("VV") { 6 } else { 10 })),
            legenda: None,
            nota: Some(if cor { NOTA_RADAR_10M_COR } else { NOTA_RADAR_10M }),
        });
    }
    if let Some(l) = crate::luzes::da_camada(nome) {
        let (subtitulo, barra, marcas) = match l {
            crate::luzes::Camada::Ano(a) => (format!("Luzes noturnas em {a} (PCNL, DMSP e VIIRS)"), "Luz (número relativo)", 5),
            crate::luzes::Camada::Mudanca(a, b) => {
                (format!("Mudança das luzes noturnas de {a} a {b} (PCNL, DMSP e VIIRS)"), "Diferença (número relativo)", 7)
            }
        };
        return Some(Receita { subtitulo, barra: Some((barra.into(), marcas)), legenda: None, nota: Some(NOTA_LUZES) });
    }
    if let Some(periodo) = crate::metano::periodo_da_camada(nome) {
        return Some(Receita {
            subtitulo: format!("Coluna média de metano no ar (XCH4), {periodo} (Sentinel-5P/TROPOMI)"),
            barra: Some(("Metano no ar, XCH4 (ppb)".into(), 6)),
            legenda: None,
            nota: Some(NOTA_METANO),
        });
    }
    if crate::radar::eh_do_radar(nome) {
        // "Radar VV em dB (Sentinel-1, 26/10/2026)": a polarização e a data da passagem.
        let qual = n.split(" (").next().unwrap_or(n).trim_end_matches(" em dB");
        let quando = n.rsplit(", ").next().map(|d| d.trim_end_matches(')')).filter(|d| d.len() == 10);
        let quando = quando.map(|d| format!(" em {d}")).unwrap_or_default();
        // As escalas fixas do plugin: 7 dB nas polarizações (marcas de 1 dB) e 5 dB na razão.
        let marcas = if qual.starts_with("Razão") { 6 } else { 8 };
        return Some(Receita {
            subtitulo: format!("{qual} do Sentinel-1{quando}, corrigido do relevo (OPERA RTC-S1)"),
            barra: Some((format!("{qual} (dB)"), marcas)),
            legenda: None,
            nota: None,
        });
    }
    if let Some(resto) = n.strip_prefix("Uso e cobertura ")
        && resto.ends_with(" (MapBiomas, recortado)")
    {
        let ano = resto.split(' ').next().unwrap_or("");
        return Some(Receita {
            subtitulo: format!("Uso e cobertura da terra em {ano} (MapBiomas, Coleção 10)"),
            barra: None,
            // As classes vêm da camada dos nomes (polígonos sob o raster), que a legenda lista com
            // o nome dela; um título a mais tiraria uma classe da caixa.
            legenda: None,
            nota: None,
        });
    }
    if crate::TEMAS.iter().any(|t| t.id == crate::aqueduct::TEMA && t.camada_no_mapa() == nome) {
        return Some(Receita {
            subtitulo: "Estresse hídrico de referência: retirada de água sobre a oferta renovável (Aqueduct 4.0, WRI)".into(),
            barra: None,
            legenda: Some("Estresse hídrico"),
            nota: Some(NOTA_AQUEDUCT),
        });
    }
    // Os mapas do MapBiomas para ver (mosaicos do Brasil inteiro, sem legenda no RustGIS): as cores
    // vão na nota.
    if crate::mapbiomas::eh_da_irrigacao(nome) {
        let ano = n.trim_start_matches("Sistemas de irrigação ").split(' ').next().unwrap_or("");
        return Some(Receita {
            subtitulo: format!("Sistemas de irrigação em {ano} (MapBiomas, 30 m)"),
            barra: None,
            legenda: None,
            nota: Some(NOTA_IRRIGACAO_MAPBIOMAS),
        });
    }
    if let Some(resto) = n.strip_prefix("Uso e cobertura ")
        && resto.ends_with(" (MapBiomas)")
    {
        let ano = resto.split(' ').next().unwrap_or("");
        return Some(Receita {
            subtitulo: format!("Uso e cobertura da terra em {ano} (MapBiomas, Coleção 11)"),
            barra: None,
            legenda: None,
            nota: Some(NOTA_COBERTURA_MAPBIOMAS),
        });
    }
    // Um tema vetorial: o subtítulo é o nome dele, e as classes vão para a legenda.
    if let Some(t) = crate::tema_da_camada(nome) {
        return Some(Receita { subtitulo: t.nome.to_string(), barra: None, legenda: None, nota: None });
    }
    // Qualquer outro dado do BrasilData (setores e indicadores do censo, saúde, focos...): o
    // subtítulo é o nome da camada, e ela vai para a legenda como está.
    Some(Receita { subtitulo: n.to_string(), barra: None, legenda: None, nota: None })
}

/// Se a camada `nome` é uma composição colorida de três canais (o radar de 10 m, o NISAR): ela não
/// tem o que listar na legenda, e o que cada cor é vai na nota.
pub fn composicao(nome: &str) -> bool {
    nome.strip_prefix(crate::PREFIXO).is_some_and(|n| n.starts_with("Composição"))
        && (crate::radar_pc::eh_daqui(nome) || crate::nisar::eh_do_nisar(nome))
}

/// Os mapas coropléticos das camadas de indicadores e de setores: o campo, o que ele é (vai no
/// subtítulo, depois do nome da camada) e a rampa de cores. Vale o primeiro que a camada tem (os
/// campos de cada fonte de indicadores).
pub const COROPLETICOS: &[(&str, &str, &str)] = &[
    ("dens_hab_km2", "densidade demográfica (hab/km²)", "YlOrRd"),
    ("dens_2022", "densidade demográfica em 2022 (hab/km²)", "YlOrRd"),
    ("estab_2017", "estabelecimentos agropecuários em 2017", "YlGn"),
    ("estab_organico", "estabelecimentos com produção orgânica em 2017", "Greens"),
    ("pix_recebido", "Pix recebido no mês (R$)", "Blues"),
    ("credito_custeio", "crédito rural de custeio no ano (R$)", "Greens"),
    ("abstencao_2026", "abstenção no 1º turno de 2026 (%)", "Purples"),
    ("caged_saldo_12m", "saldo de empregos formais em 12 meses (CAGED)", "Oranges"),
    ("saude_mort_infantil", "mortalidade infantil por mil nascidos vivos, em 3 anos (DATASUS)", "Reds"),
    ("seeg_ch4", "metano emitido no ano, em toneladas (SEEG)", "Oranges"),
];

/// As 5 cores de uma rampa de [`COROPLETICOS`] (ColorBrewer), opacas. O RustGIS pinta as classes
/// de um polígono translúcidas, e uma camada do projeto por baixo (os países do World Atlas) tingia
/// o mapa; com as cores opacas, a cor do mapa é a da legenda.
pub fn cores_opacas(rampa: &str) -> Option<[&'static str; 5]> {
    Some(match rampa {
        "YlOrRd" => ["#FFFFB2FF", "#FECC5CFF", "#FD8D3CFF", "#F03B20FF", "#BD0026FF"],
        "YlGn" => ["#FFFFCCFF", "#C2E699FF", "#78C679FF", "#31A354FF", "#006837FF"],
        "Greens" => ["#EDF8E9FF", "#BAE4B3FF", "#74C476FF", "#31A354FF", "#006D2CFF"],
        "Blues" => ["#EFF3FFFF", "#BDD7E7FF", "#6BAED6FF", "#3182BDFF", "#08519CFF"],
        "Purples" => ["#F2F0F7FF", "#CBC9E2FF", "#9E9AC8FF", "#756BB1FF", "#54278FFF"],
        "Oranges" => ["#FEEDDEFF", "#FDBE85FF", "#FD8D3CFF", "#E6550DFF", "#A63603FF"],
        "Reds" => ["#FEE5D9FF", "#FCAE91FF", "#FB6A4AFF", "#DE2D26FF", "#A50F15FF"],
        _ => return None,
    })
}

/// O mapa coroplético da camada `l` (uma do `info.project`): um polígono com um dos campos de
/// [`COROPLETICOS`].
pub fn coropletico(l: &serde_json::Value) -> Option<(&'static str, &'static str, &'static str)> {
    if !l["type"].as_str().is_some_and(|t| t.contains("Polygon")) {
        return None;
    }
    let campos: Vec<&str> = l["fields"].as_array()?.iter().filter_map(|f| f.as_str()).map(|f| f.split(" (").next().unwrap_or(f)).collect();
    COROPLETICOS.iter().find(|(c, ..)| campos.contains(c)).copied()
}

/// As camadas visíveis do `info.project` que não são do BrasilData (as do projeto do usuário): o
/// layout as deixa fora da legenda.
pub fn camadas_alheias(info: &serde_json::Value) -> Vec<String> {
    info["layers"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|l| l["visible"].as_bool() != Some(false))
        .filter_map(|l| l["name"].as_str())
        .filter(|n| !n.starts_with(PREFIXO))
        .map(str::to_string)
        .collect()
}

/// Se a camada `l` (uma do `info.project`) é um polígono de referência maior que o retângulo `c`
/// da área, de um símbolo só: municípios, regiões, bacias, distritos. Pintado como veio, cobriria a
/// folha inteira de uma cor. Um tema com classes de cor fica como está.
pub fn referencia_maior(l: &serde_json::Value, c: [f64; 4]) -> bool {
    let poligono = l["type"].as_str().is_some_and(|t| t.contains("Polygon"));
    let simples = l["renderer"].as_str() == Some("Single Symbol");
    let e: Vec<f64> = l["extent"].as_array().into_iter().flatten().filter_map(serde_json::Value::as_f64).collect();
    let [x0, y0, x1, y1] = e[..] else { return false };
    let (dx, dy) = ((c[2] - c[0]) * 0.02, (c[3] - c[1]) * 0.02);
    poligono && simples && (x0 < c[0] - dx || y0 < c[1] - dy || x1 > c[2] + dx || y1 > c[3] + dy)
}

/// O campo com o nome de cada feição (`nm_mun`, `NM_UF`, `nome`...), para os rótulos: da lista
/// `fields` do `info.project` ("nm_mun (Text)").
pub fn campo_do_nome(l: &serde_json::Value) -> Option<String> {
    l["fields"]
        .as_array()?
        .iter()
        .filter_map(|f| f.as_str())
        .map(|f| f.split(" (").next().unwrap_or(f))
        .find(|f| {
            let f = f.to_ascii_lowercase();
            f.starts_with("nm_") || f == "nome" || f.starts_with("nome_")
        })
        .map(str::to_string)
}

/// O passo da grade em metros: o redondo que dá até 6 linhas no lado maior do retângulo `c`.
pub fn passo_da_grade(c: [f64; 4]) -> f64 {
    let lat = ((c[1] + c[3]) / 2.0).to_radians();
    let lado = ((c[2] - c[0]) * 111_320.0 * lat.cos()).max((c[3] - c[1]) * 110_574.0);
    [1e3, 2e3, 5e3, 1e4, 2e4, 5e4, 1e5, 2e5, 5e5].into_iter().find(|p| lado / p <= 6.0).unwrap_or(1e6)
}

/// O título: o nome da área (a primeira linha do que o painel diz dela), em caixa alta.
pub fn titulo(texto_da_area: &str) -> String {
    let linha = texto_da_area.lines().next().unwrap_or("").trim();
    if linha.is_empty() || linha.ends_with("polígono") || linha.ends_with("polígonos") {
        "ÁREA DE TRABALHO".into()
    } else {
        linha.to_uppercase()
    }
}

/// O `profile` do `layout.publication`: o `perfil` com a `orientacao` (automatica, retrato ou
/// paisagem). Só uma folha (a4, a3, a2...) tem orientação; uma figura tem o tamanho fixo, e um
/// perfil que já diz a sua (a4-portrait) fica como veio.
pub fn perfil_com_orientacao(perfil: &str, orientacao: Option<&str>) -> Result<String, String> {
    let lado = match orientacao.map(str::trim).unwrap_or("automatica") {
        "" | "automatica" | "automática" => None,
        "retrato" | "portrait" => Some("portrait"),
        "paisagem" | "landscape" => Some("landscape"),
        o => return Err(format!("orientação '{o}': use automatica, retrato ou paisagem")),
    };
    let folha = !perfil.contains('-') && !perfil.starts_with("figure") && !perfil.starts_with("thesis");
    Ok(match lado {
        Some(l) if folha => format!("{perfil}-{l}"),
        _ => perfil.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_nota_dos_geomorfons_traz_a_chave() {
        assert!(NOTA_GEOMORFONS.contains(&crate::gedtm::chave_geomorfons()));
        let r = receita("BrasilData: Geomorfons 480 m (GEDTM30)").unwrap();
        assert!(r.barra.is_none() && r.nota == Some(NOTA_GEOMORFONS));
        let d = receita("BrasilData: Declividade 30 m, graus (GEDTM30)").unwrap();
        assert_eq!(d.barra, Some(("Declividade (graus)".to_string(), 5)));
        let s = receita("BrasilData: pH do solo em água 0–30 cm, 2020–2022 (OpenLandMap)").unwrap();
        assert_eq!(s.barra, Some(("pH do solo em água".to_string(), 5)));
    }

    #[test]
    fn a_orientacao_da_folha() {
        assert_eq!(perfil_com_orientacao("a4", None).unwrap(), "a4");
        assert_eq!(perfil_com_orientacao("a4", Some("automatica")).unwrap(), "a4");
        assert_eq!(perfil_com_orientacao("a3", Some("retrato")).unwrap(), "a3-portrait");
        assert_eq!(perfil_com_orientacao("a2", Some("paisagem")).unwrap(), "a2-landscape");
        // A figura tem o tamanho fixo, e um perfil com a orientação fica com a sua.
        assert_eq!(perfil_com_orientacao("figure-2col", Some("paisagem")).unwrap(), "figure-2col");
        assert_eq!(perfil_com_orientacao("thesis-full", Some("retrato")).unwrap(), "thesis-full");
        assert_eq!(perfil_com_orientacao("a4-portrait", Some("paisagem")).unwrap(), "a4-portrait");
        assert!(perfil_com_orientacao("a4", Some("deitada")).is_err());
    }

    #[test]
    fn as_receitas_das_camadas_do_gedi_e_do_dist() {
        let r = receita(gedi::Variavel::Altura.camada()).unwrap();
        assert_eq!(r.subtitulo, "Altura do dossel pelo laser GEDI (NASA), 2019–2023");
        assert_eq!(r.barra, Some(("Altura do dossel (m)".into(), 8)));
        let r = receita(gedi::Variavel::Biomassa.camada()).unwrap();
        assert_eq!(r.barra, Some(("Biomassa acima do solo (Mg/ha)".into(), 4)));
        let r = receita("BrasilData: Perturbação da vegetação, manchas (OPERA DIST-S1, até 28/02/2026)").unwrap();
        assert!(r.subtitulo.ends_with("situação em 28/02/2026"), "{}", r.subtitulo);
        assert_eq!((r.barra, r.legenda), (None, Some("Situação do alerta")));
        let r = receita("BrasilData: Confiança da perturbação (OPERA DIST-S1, até 28/02/2026)").unwrap();
        assert_eq!(r.barra, Some(("Confiança".into(), 5)));
    }

    #[test]
    fn as_outras_receitas() {
        assert_eq!(receita("BrasilData: Elevação (Copernicus GLO-30)").unwrap().barra, Some(("Altitude (m)".into(), 5)));
        assert_eq!(receita(crate::chm::CAMADA).unwrap().barra, Some(("Altura do dossel (m)".into(), 8)));
        let t = receita("BrasilData: Temperatura da superfície de dia média de 01/08/2026 a 31/10/2026 (GCOM-C, JAXA Earth)").unwrap();
        assert_eq!(t.subtitulo, "Temperatura da superfície de dia média pelo JAXA Earth (GCOM-C, meses), de 01/08/2026 a 31/10/2026");
        assert_eq!(t.barra, Some(("Temperatura da superfície de dia média (°C)".into(), 8)));
        let e = receita("BrasilData: Chuva média por ano de 01/01/1991 a 31/12/2020 (ERA5)").unwrap();
        assert_eq!(e.subtitulo, "Chuva média por ano pela reanálise ERA5 (ECMWF/Copernicus, valores diários), de 01/01/1991 a 31/12/2020");
        assert_eq!(e.barra, Some(("Chuva média por ano (mm por ano)".into(), 6)));
        let e = receita("BrasilData: Temperatura máxima do ar, média de 01/01/2025 a 31/12/2025 (ERA5)").unwrap();
        assert_eq!(e.barra, Some(("Temperatura máxima do ar, média (°C)".into(), 5)));
        let b = receita("BrasilData: NDVI médio de 01/06/2025 a 31/08/2025 (Sentinel-2, cubo do INPE)").unwrap();
        assert_eq!(b.subtitulo, "NDVI médio pelo cubo de dados do INPE (Sentinel-2, 16 dias), de 01/06/2025 a 31/08/2025");
        assert_eq!(receita(crate::gem15::Modelo::Altura.camada()).unwrap().barra, Some(("Altura acima do terreno (m)".into(), 5)));
        let r = receita("BrasilData: Radar VV em dB (Sentinel-1, 26/10/2026)").unwrap();
        assert_eq!(r.subtitulo, "Radar VV do Sentinel-1 em 26/10/2026, corrigido do relevo (OPERA RTC-S1)");
        assert_eq!(r.barra, Some(("Radar VV (dB)".into(), 8)));
        let vv = "BrasilData: Radar VV 10 m, mediana de 10 passagens (Sentinel-1, 03/08/2026 a 19/11/2026)";
        let r = receita(vv).unwrap();
        assert_eq!(
            r.subtitulo,
            "Radar VV do Sentinel-1, mediana de 10 passagens, 03/08/2026 a 19/11/2026, corrigido do relevo (Planetary Computer)"
        );
        assert_eq!((r.barra, r.nota.is_some()), (Some(("Radar VV (dB)".into(), 6)), true));
        assert_eq!(receita(&vv.replace("VV", "VH")).unwrap().barra, Some(("Radar VH (dB)".into(), 10)));
        let cor = crate::radar_pc::nome_da_composicao(vv).unwrap();
        let r = receita(&cor).unwrap();
        assert_eq!((r.barra, r.nota), (None, Some(NOTA_RADAR_10M_COR)));
        assert!(composicao(&cor) && !composicao(vv) && composicao("BrasilData: Composição HH, HV, HH − HV (NISAR, 01/11/2026)"));
        let r = receita(&crate::luzes::camada_mudanca(2000, 2024)).unwrap();
        assert_eq!(r.subtitulo, "Mudança das luzes noturnas de 2000 a 2024 (PCNL, DMSP e VIIRS)");
        assert_eq!(receita(&crate::luzes::camada(2024)).unwrap().barra, Some(("Luz (número relativo)".into(), 5)));
        let r = receita(&crate::metano::camada(2023, Some(3))).unwrap();
        assert_eq!(r.subtitulo, "Coluna média de metano no ar (XCH4), março de 2023 (Sentinel-5P/TROPOMI)");
        assert_eq!(r.barra, Some(("Metano no ar, XCH4 (ppb)".into(), 6)));
        let r = receita("BrasilData: Radar HV em dB (NISAR, 01/11/2026)").unwrap();
        assert_eq!(r.subtitulo, "Radar HV do NISAR (banda L) em 01/11/2026, corrigido do relevo (GCOV)");
        assert_eq!(r.barra, Some(("Radar HV (dB)".into(), 6)));
        assert_eq!(receita("BrasilData: Razão HV/HH em dB (NISAR, 01/11/2026)").unwrap().barra, Some(("Razão HV/HH (dB)".into(), 8)));
        let r = receita("BrasilData: Composição HH, HV, HH − HV (NISAR, 01/11/2026)").unwrap();
        assert_eq!((r.barra, r.nota.is_some()), (None, true));
        let r = receita("BrasilData: Razão VH/VV em dB (Sentinel-1, 26/10/2026)").unwrap();
        assert_eq!(r.barra, Some(("Razão VH/VV (dB)".into(), 6)));
        let r = receita("BrasilData: Uso e cobertura 2024 (MapBiomas, recortado)").unwrap();
        assert_eq!(r.subtitulo, "Uso e cobertura da terra em 2024 (MapBiomas, Coleção 10)");
        let r = receita(crate::sol::CAMADA).unwrap();
        assert_eq!(r.barra, Some(("Radiação solar (kWh/m² por dia)".into(), 6)));
        assert!(receita(&crate::sol::camada(Some(8))).unwrap().subtitulo.contains("média diária de agosto"));
        let r = receita(&crate::vento::camada(Some(9))).unwrap();
        assert_eq!(r.barra, Some(("Velocidade do vento (m/s)".into(), 6)));
        assert!(r.subtitulo.contains("média de setembro"));
        let r = receita("BrasilData: Anomalia da umidade do solo nas raízes, setembro de 2024 (NASA POWER)").unwrap();
        assert_eq!(r.subtitulo, "Umidade do solo nas raízes, setembro de 2024, diferença da média de 2001 a 2025 (NASA POWER, MERRA-2)");
        assert_eq!(r.barra, Some(("Diferença da média (fração da saturação)".into(), 5)));
        let r = receita("BrasilData: Evapotranspiração, média de março (NASA POWER)").unwrap();
        assert_eq!(r.subtitulo, "Evapotranspiração, média de março de 2001 a 2025 (NASA POWER, MERRA-2)");
        assert_eq!(r.barra, Some(("Evapotranspiração (mm por dia)".into(), 7)));
        let m = &crate::tempo::DETERMINISTICOS[0];
        let r = receita(&crate::tempo::camada_chuva(m, 5, "14/11", "18/11")).unwrap();
        assert_eq!(r.barra, Some(("Chuva (mm)".into(), 7)));
        assert!(r.subtitulo.starts_with("Chuva prevista em 5 dias"));
        let e = &crate::tempo::ENSEMBLES[0];
        let r = receita(&crate::tempo::camada_ensemble(e, crate::tempo::Medida::Chance, 100.0, 15, "14/11", "28/11")).unwrap();
        assert_eq!(r.barra, Some(("Chance (%)".into(), 6)));
        assert_eq!(receita(&crate::tempo::camada_chirps(2026, 10)).unwrap().barra, Some(("Chuva no mês (mm)".into(), 5)));
        let r = receita(crate::irrigacao::CAMADA).unwrap();
        assert_eq!(r.barra, Some(("Área equipada para irrigação (% da célula)".into(), 5)));
        let aqueduct = crate::TEMAS.iter().find(|t| t.id == crate::aqueduct::TEMA).unwrap().camada_no_mapa();
        let r = receita(&aqueduct).unwrap();
        assert_eq!((r.barra, r.legenda), (None, Some("Estresse hídrico")));
        let r = receita(&crate::mapbiomas::camada_da_irrigacao(2024)).unwrap();
        assert_eq!(r.subtitulo, "Sistemas de irrigação em 2024 (MapBiomas, 30 m)");
        assert!(r.nota.unwrap().contains("pivô central"));
        assert!(receita("BrasilData: Uso e cobertura 2025 (MapBiomas)").unwrap().subtitulo.ends_with("(MapBiomas, Coleção 11)"));
        let r = receita(&crate::risco_fogo::camada(9)).unwrap();
        assert_eq!(r.subtitulo, "Risco de fogo em setembro: média de 2001 a 2025 (INPE, Programa Queimadas)");
        assert_eq!(r.barra, Some(("Risco de fogo (0 a 1)".into(), 11)));
        assert!(r.nota.unwrap().contains("crítico, acima de 0,95"));
        assert_eq!(receita(AREA), None);
        assert_eq!(receita(GRADE), None);
        assert_eq!(receita("Countries"), None);
        assert_eq!(receita("BrasilData: Setores censitários 2022 (MG)").unwrap().subtitulo, "Setores censitários 2022 (MG)");
    }

    #[test]
    fn o_mapa_coropletico_dos_indicadores() {
        let todos = serde_json::json!({"type": "Polygon", "fields": ["cd_mun (Text)", "pix_recebido (Double)", "dens_2022 (Double)"]});
        assert_eq!(coropletico(&todos).map(|c| c.0), Some("dens_2022"), "a densidade vem antes");
        let pix = serde_json::json!({"type": "Polygon", "fields": ["cd_mun (Text)", "pix_mes (Text)", "pix_recebido (Double)"]});
        assert_eq!(coropletico(&pix).map(|c| c.0), Some("pix_recebido"));
        assert_eq!(coropletico(&serde_json::json!({"type": "Polygon", "fields": ["nm_mun (Text)"]})), None);
        assert_eq!(coropletico(&serde_json::json!({"type": "Point", "fields": ["dens_2022 (Double)"]})), None);
    }

    #[test]
    fn os_poligonos_de_referencia_maiores_que_a_area() {
        let c = [-43.005, -20.839, -42.754, -20.655];
        let municipios = serde_json::json!({"type": "Polygon", "renderer": "Single Symbol", "extent": [-43.19, -20.99, -42.63, -20.45],
            "fields": ["cd_mun (Text)", "nm_mun (Text)", "sigla_uf (Text)"]});
        assert!(referencia_maior(&municipios, c));
        assert_eq!(campo_do_nome(&municipios).as_deref(), Some("nm_mun"));
        // Recortada pela área, ou com classes de cor, fica como está.
        let solos = serde_json::json!({"type": "Polygon", "renderer": "Single Symbol", "extent": c, "fields": ["legenda (Text)"]});
        assert!(!referencia_maior(&solos, c));
        assert_eq!(campo_do_nome(&solos), None);
        let dist = serde_json::json!({"type": "Polygon", "renderer": "Unique Values", "extent": [-44.0, -21.0, -42.0, -20.0]});
        assert!(!referencia_maior(&dist, c));
        let focos = serde_json::json!({"type": "Point", "renderer": "Single Symbol", "extent": [-44.0, -21.0, -42.0, -20.0]});
        assert!(!referencia_maior(&focos, c));
    }

    #[test]
    fn o_passo_da_grade_e_o_titulo() {
        // Viçosa: cerca de 26 km de lado maior.
        assert_eq!(passo_da_grade([-43.005, -20.839, -42.7535, -20.6547]), 5000.0);
        // Minas Gerais: cerca de 1.200 km.
        assert_eq!(passo_da_grade([-51.05, -22.93, -39.85, -14.23]), 200_000.0);
        assert_eq!(titulo("Viçosa (MG)\n-43.005, -20.839 a -42.754, -20.655"), "VIÇOSA (MG)");
        assert_eq!(titulo("3 polígonos\n-43, -21 a -42, -20"), "ÁREA DE TRABALHO");
    }

    #[test]
    fn as_camadas_do_projeto_ficam_fora_da_legenda() {
        // O projeto de exemplo do RustGIS (World Atlas) com uma camada do BrasilData por cima.
        let info = serde_json::json!({"layers": [
            {"name": "BrasilData: Indicadores por município", "visible": true},
            {"name": "BrasilData: área", "visible": true},
            {"name": "Cities", "visible": true},
            {"name": "Rivers"},
            {"name": "Countries", "visible": false},
        ]});
        assert_eq!(camadas_alheias(&info), ["Cities", "Rivers"]);
    }
}
