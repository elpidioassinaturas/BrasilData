//! Os estabelecimentos de saúde do CNES (Cadastro Nacional de Estabelecimentos de Saúde).
//!
//! - **De onde:** o GeoParquet nacional do geobr (Ipea), feito do CNES do Ministério da Saúde.
//!   O zip do próprio Ministério (OpenDataSUS) vem em Latin-1, que o RustGIS 0.5.0 não lê, e
//!   a API do CNES não responde. O RustGIS lê o GeoParquet pelo endereço, aos pedaços.
//! - **Só no desktop:** o GitHub não manda CORS para os arquivos das releases.
//! - **O que fica:** os estabelecimentos ativos, sem os de pessoa física (o nome deles é o de uma
//!   pessoa), e só os campos do estabelecimento: nada de CPF, CNPJ, diretor ou usuário.
//! - **Quem faz o trabalho:** o RustGIS, com seleção por atributos e cálculo de campos. Um
//!   comando de plugin tem uma cota de instruções que ler mil estabelecimentos já passa.

/// O GeoParquet do CNES no geobr: competência de abril de 2026, 615.401 estabelecimentos, 98 MB.
pub const URL: &str = "https://github.com/ipea/geobr_prep_data/releases/download/v2.0.0/healthfacilities_202604.parquet";

/// A competência do arquivo, para o nome da camada.
pub const COMPETENCIA: &str = "04/2026";

/// A citação, como o geobr pede, com a fonte original.
pub const CITACAO: &str = "Ministério da Saúde, Cadastro Nacional de Estabelecimentos de Saúde (CNES), competência 04/2026, pelo geobr: Pereira, R.H.M.; Barbosa, R.J. et al. (2026), geobr: Download Official Spatial Data Sets of Brazil, v2.0.0 (Ipea), github.com/ipea/geobr. As coordenadas são as do CNES ou, quando estas faltam ou se afastam mais de 800 m do endereço, as do geocodebr (campo origem_coord)";

/// A licença: o CNES é dado aberto do Ministério da Saúde, e o geobr é MIT.
pub const LICENCA: &str = "dados abertos do Ministério da Saúde, com citação da fonte; geobr sob licença MIT";

/// Os desativados: os que têm motivo de desativação.
pub const DESATIVADO: &str = "NOT co_motivo_desab IS NULL";

/// Os de pessoa, cujo nome é o de uma pessoa: pessoa física no CNES (`tp_pfpj` 1), natureza
/// jurídica de pessoa física (4xxx) ou empresário individual (2135, que leva o nome do dono).
/// Os `coalesce` fazem a condição nunca ser nula.
pub const DE_PESSOA: &str = "coalesce(tp_pfpj, 0) = 1 OR (coalesce(co_natureza_jur, 0) >= 4000 AND coalesce(co_natureza_jur, 0) < 5000) OR coalesce(co_natureza_jur, 0) = 2135";

/// Os que ficam na camada: ativos e de pessoa jurídica.
pub fn ficam() -> String {
    format!("co_motivo_desab IS NULL AND NOT ({DE_PESSOA})")
}

/// Os de pessoa entre os ativos.
pub fn de_pessoa_ativos() -> String {
    format!("co_motivo_desab IS NULL AND ({DE_PESSOA})")
}

/// Os tipos de unidade do CNES (`tp_unidade`), pela tabela do CNES.
const TIPOS: [(u32, &str); 39] = [
    (1, "Posto de saúde"),
    (2, "Centro de saúde / Unidade básica"),
    (4, "Policlínica"),
    (5, "Hospital geral"),
    (7, "Hospital especializado"),
    (15, "Unidade mista"),
    (20, "Pronto-socorro geral"),
    (21, "Pronto-socorro especializado"),
    (22, "Consultório isolado"),
    (32, "Unidade móvel fluvial"),
    (36, "Clínica / Centro de especialidade"),
    (39, "Unidade de apoio diagnose e terapia (SADT isolado)"),
    (40, "Unidade móvel terrestre"),
    (42, "Unidade móvel de nível pré-hospitalar na área de urgência"),
    (43, "Farmácia"),
    (50, "Unidade de vigilância em saúde"),
    (60, "Cooperativa ou empresa de cessão de trabalhadores na saúde"),
    (61, "Centro de parto normal (isolado)"),
    (62, "Hospital-dia (isolado)"),
    (64, "Central de regulação de serviços de saúde"),
    (67, "Laboratório central de saúde pública (LACEN)"),
    (68, "Central de gestão em saúde"),
    (69, "Centro de atenção hemoterápica e/ou hematológica"),
    (70, "Centro de atenção psicossocial (CAPS)"),
    (71, "Centro de apoio à saúde da família"),
    (72, "Unidade de atenção à saúde indígena"),
    (73, "Pronto atendimento"),
    (74, "Polo academia da saúde"),
    (75, "Telessaúde"),
    (76, "Central de regulação médica das urgências"),
    (77, "Serviço de atenção domiciliar isolado (home care)"),
    (78, "Unidade de atenção em regime residencial"),
    (79, "Oficina ortopédica"),
    (80, "Laboratório de saúde pública"),
    (81, "Central de regulação do acesso"),
    (82, "Central de notificação, captação e distribuição de órgãos estadual"),
    (83, "Polo de prevenção de doenças e agravos e promoção da saúde"),
    (84, "Central de abastecimento"),
    (85, "Centro de imunização"),
];

/// A expressão do nome do tipo de unidade; um código fora da tabela fica "tipo N".
fn expressao_tipo() -> String {
    let casos: String = TIPOS.iter().map(|(c, n)| format!(" WHEN {c} THEN '{n}'")).collect();
    format!("CASE tp_unidade{casos} ELSE 'tipo ' || tp_unidade END")
}

/// Os campos da camada, na ordem: nome, tipo e expressão que os calcula dos campos do geobr.
/// O bairro fica com o nome que já tem.
pub fn campos() -> Vec<(&'static str, &'static str, String)> {
    vec![
        ("cnes", "text", "trim(co_cnes)".into()),
        ("nome", "text", "trim(no_fantasia)".into()),
        ("tipo", "text", expressao_tipo()),
        ("cod_tipo", "long", "tp_unidade".into()),
        (
            "natureza",
            "text",
            "CASE WHEN co_natureza_jur >= 1000 AND co_natureza_jur < 2000 THEN 'administração pública' WHEN co_natureza_jur >= 2000 AND co_natureza_jur < 3000 THEN 'entidade empresarial' WHEN co_natureza_jur >= 3000 AND co_natureza_jur < 4000 THEN 'entidade sem fins lucrativos' ELSE 'não informada' END".into(),
        ),
        (
            "gestao",
            "text",
            "CASE trim(tp_gestao) WHEN 'M' THEN 'municipal' WHEN 'E' THEN 'estadual' WHEN 'D' THEN 'dupla' WHEN 'S' THEN 'sem gestão' WHEN 'Z' THEN 'sem gestão' ELSE 'não informada' END".into(),
        ),
        ("municipio", "text", "trim(name_muni)".into()),
        ("uf", "text", "abbrev_state".into()),
        ("origem_coord", "text", "coords_source".into()),
    ]
}

/// Os campos que ficam na camada: os de [`campos`] e o bairro. Os outros (CPF, CNPJ, diretor,
/// usuário, datas, códigos já traduzidos…) são apagados.
pub fn fica_o_campo(nome: &str) -> bool {
    nome == "bairro" || campos().iter().any(|(c, _, _)| *c == nome)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_tipos_viram_nomes_e_os_documentos_saem() {
        let t = expressao_tipo();
        assert!(t.starts_with("CASE tp_unidade WHEN 1 THEN 'Posto de saúde'"));
        assert!(t.contains(" WHEN 5 THEN 'Hospital geral'"));
        assert!(t.ends_with(" ELSE 'tipo ' || tp_unidade END"));
        // Nenhum nome tem aspas simples, que fechariam o texto da expressão.
        assert!(TIPOS.iter().all(|(_, n)| !n.contains('\'')));
        for c in ["cnes", "nome", "tipo", "natureza", "gestao", "bairro", "municipio", "uf", "origem_coord"] {
            assert!(fica_o_campo(c), "{c}");
        }
        for c in ["nu_cpf", "nu_cnpj", "nu_cnpj_mantenedora", "no_razao_social", "co_cpfdiretorcln", "no_usuario_geo", "co_usuario"] {
            assert!(!fica_o_campo(c), "{c}");
        }
    }

    #[test]
    fn os_filtros_separam_ativos_e_pessoas() {
        assert_eq!(ficam(), format!("co_motivo_desab IS NULL AND NOT ({DE_PESSOA})"));
        assert!(de_pessoa_ativos().starts_with("co_motivo_desab IS NULL AND ("));
    }
}
