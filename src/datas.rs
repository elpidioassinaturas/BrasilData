//! Datas no formato dia/mês/ano, para o período dos focos de calor. O plugin não tem relógio:
//! o mês corrente é o do último foco que o serviço guarda.

/// Uma data do calendário.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Data {
    pub ano: i64,
    pub mes: u32,
    pub dia: u32,
}

impl Data {
    /// A data (UTC) de `ms` milissegundos desde 1970, pelo algoritmo `civil_from_days` de Howard
    /// Hinnant.
    pub fn de_milissegundos(ms: i64) -> Data {
        let z = ms.div_euclid(86_400_000) + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let dia = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let mes = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        Data { ano: yoe + era * 400 + i64::from(mes <= 2), mes, dia }
    }

    /// Uma data escrita "dd/mm/aaaa" (o dia e o mês podem ter um algarismo).
    pub fn ler(texto: &str) -> Option<Data> {
        let mut p = texto.trim().split('/');
        let (dia, mes, ano) = (p.next()?.trim().parse::<u32>().ok()?, p.next()?.trim().parse::<u32>().ok()?, p.next()?.trim());
        if p.next().is_some() || ano.len() != 4 {
            return None;
        }
        let ano = ano.parse::<i64>().ok()?;
        ((1..=12).contains(&mes) && dia >= 1 && dia <= dias_no_mes(ano, mes)).then_some(Data { ano, mes, dia })
    }

    /// "dd/mm/aaaa".
    pub fn texto(&self) -> String {
        format!("{:02}/{:02}/{}", self.dia, self.mes, self.ano)
    }

    /// "aaaa-mm-dd", para o `DATE '…'` do filtro.
    pub fn iso(&self) -> String {
        format!("{}-{:02}-{:02}", self.ano, self.mes, self.dia)
    }

    /// Quantos dias desde 01/01/1970 (o `days_from_civil` de Howard Hinnant, o inverso de
    /// [`Data::de_milissegundos`]).
    pub fn dias(&self) -> i64 {
        let (m, d) = (i64::from(self.mes), i64::from(self.dia));
        let a = if m <= 2 { self.ano - 1 } else { self.ano };
        let era = a.div_euclid(400);
        let yoe = a - era * 400;
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// O dia seguinte.
    pub fn seguinte(&self) -> Data {
        if self.dia < dias_no_mes(self.ano, self.mes) {
            Data { dia: self.dia + 1, ..*self }
        } else if self.mes < 12 {
            Data { mes: self.mes + 1, dia: 1, ..*self }
        } else {
            Data { ano: self.ano + 1, mes: 1, dia: 1 }
        }
    }
}

/// Quantos dias tem o mês `mes` (1 a 12) de `ano`.
pub fn dias_no_mes(ano: i64, mes: u32) -> u32 {
    match mes {
        2 if ano % 4 == 0 && (ano % 100 != 0 || ano % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_datas_vem_dos_milissegundos_e_do_texto() {
        assert_eq!(Data::de_milissegundos(1_793_563_200_000), Data { ano: 2026, mes: 11, dia: 1 });
        assert_eq!(Data::de_milissegundos(0), Data { ano: 1970, mes: 1, dia: 1 });
        assert_eq!(Data::de_milissegundos(951_782_400_000), Data { ano: 2000, mes: 2, dia: 29 });
        assert_eq!(Data::ler("1/11/2026"), Some(Data { ano: 2026, mes: 11, dia: 1 }));
        assert_eq!(Data::ler(" 02/11/2026 ").map(|d| d.texto()), Some("02/11/2026".into()));
        for errada in ["31/11/2026", "29/02/2026", "2026-11-01", "01/11/26", "1/13/2026", "x"] {
            assert_eq!(Data::ler(errada), None, "{errada}");
        }
        assert_eq!(Data::ler("29/02/2024").map(|d| d.iso()), Some("2024-02-29".into()));
        assert_eq!(Data::ler("30/11/2026").unwrap().seguinte().iso(), "2026-12-01");
        assert_eq!(Data::ler("31/12/2026").unwrap().seguinte().iso(), "2027-01-01");
        assert_eq!(dias_no_mes(2026, 11), 30);
        assert_eq!(Data { ano: 1970, mes: 1, dia: 1 }.dias(), 0);
        for ms in [1_793_563_200_000_i64, 951_782_400_000, 0] {
            assert_eq!(Data::de_milissegundos(ms).dias(), ms / 86_400_000);
        }
        let d = |t| Data::ler(t).unwrap().dias();
        assert_eq!(d("28/02/2026") - d("31/12/2020"), 1885, "o dia do DIST-S1 de 28/02/2026");
    }
}
