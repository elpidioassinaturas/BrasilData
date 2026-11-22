"""Escreve o saude.csv: indicadores de saúde de cada município, do DATASUS, para o plugin embutir.

    uv run --python 3.12 --with pyreaddbc --with dbfread python scripts/baixar_saude.py [PASTA]

PASTA guarda os arquivos baixados (padrão: ../dados_datasus, fora do repositório); um arquivo que
já está lá não é baixado de novo.

O DATASUS publica os microdados no FTP de disseminação, em .dbc (DBF comprimido), longe do que
um plugin consegue ler: o RustGIS não fala FTP, e um ano do SIM tem 138 MB. Por isso o plugin leva
um retrato, município a município, por local de residência:
- SINASC (nascidos vivos): os nascidos do último ano fechado, e entre eles a parte com baixo peso
  (< 2.500 g), por cesárea, de mãe com menos de 20 anos e com 7 ou mais consultas de pré-natal;
- SIM (óbitos): os óbitos do último ano fechado, a taxa por mil habitantes e a parte por causas
  externas, doenças do aparelho circulatório, neoplasias e doenças do aparelho respiratório
  (capítulos da CID-10); a mortalidade infantil por mil nascidos vivos, somando os 3 últimos anos
  (num município pequeno, um ano só dá um número que salta de um ano para outro);
- CNES (leitos): os leitos existentes e os do SUS no último mês publicado, e os do SUS por mil
  habitantes;
- a população é a estimativa do IBGE para o último ano fechado (tabela 6579 do SIDRA).
Rode de novo quando o DATASUS fechar um ano novo (o SIM e o SINASC saem do "PRELIM" no fim do ano
seguinte).
"""

from __future__ import annotations

import csv
import json
import pathlib
import re
import sys
import urllib.request
from collections import Counter, defaultdict

ROOT = pathlib.Path(__file__).resolve().parent.parent
FTP = "ftp://ftp.datasus.gov.br/dissemin/publicos"
SIM = f"{FTP}/SIM/CID10/DORES"
SINASC = f"{FTP}/SINASC/1996_/Dados/DNRES"
LEITOS = f"{FTP}/CNES/200508_/Dados/LT"
UFS = "AC AL AM AP BA CE DF ES GO MA MG MS MT PA PB PE PI PR RJ RN RO RR RS SC SE SP TO".split()
SIDRA = "https://servicodados.ibge.gov.br/api/v3/agregados/6579/periodos/{ano}/variaveis/9324?localidades=N6[all]"
COLUNAS = [
    "codigo", "ano", "nascidos", "baixo_peso_pct", "cesarea_pct", "mae_adolescente_pct", "prenatal7_pct",
    "obitos", "mortalidade_mil", "mort_infantil", "externas_pct", "circulatorio_pct", "neoplasias_pct",
    "respiratorio_pct", "leitos", "leitos_sus", "leitos_sus_mil", "leitos_mes", "populacao",
]  # fmt: skip


def listar(url: str) -> list[str]:
    with urllib.request.urlopen(url + "/", timeout=120) as r:
        return [ln.split()[-1] for ln in r.read().decode("latin-1").splitlines() if ln.strip()]


def baixar(url: str, pasta: pathlib.Path) -> pathlib.Path:
    destino = pasta / url.rsplit("/", 1)[-1]
    if not destino.exists():
        print("baixando", url, flush=True)
        tmp = destino.with_suffix(".parte")
        urllib.request.urlretrieve(url, tmp)
        tmp.replace(destino)
    return destino


def registros(dbc: pathlib.Path, campos: list[str]):
    """Os registros de um .dbc, só com os campos pedidos."""
    import dbfread  # noqa: PLC0415
    import pyreaddbc  # noqa: PLC0415

    dbf = dbc.with_suffix(".dbf")
    if not dbf.exists():
        pyreaddbc.dbc2dbf(str(dbc), str(dbf))
    for r in dbfread.DBF(str(dbf), encoding="iso-8859-1", char_decode_errors="ignore", load=False):
        yield {c: r.get(c) for c in campos}


def codigos7() -> dict[str, str]:
    """Do código de 6 dígitos do DATASUS ao de 7 do IBGE."""
    with open(ROOT / "municipios.csv", encoding="utf-8") as f:
        return {l["codigo"][:6]: l["codigo"] for l in csv.DictReader(f)}


def ultimo_ano_fechado() -> int:
    anos = [int(m.group(1)) for n in listar(SIM) if (m := re.fullmatch(r"DOBR(\d{4})\.dbc", n, flags=re.I))]
    return max(anos)


def inteiro(v) -> int | None:
    try:
        return int(str(v).strip())
    except (TypeError, ValueError):
        return None


def pct(parte: int, total: int) -> str:
    return f"{100 * parte / total:.1f}" if total else ""


def main():
    pasta = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT.parent / "dados_datasus"
    pasta.mkdir(parents=True, exist_ok=True)
    cod = codigos7()
    ano = ultimo_ano_fechado()
    anos = [ano - 2, ano - 1, ano]
    print("último ano fechado:", ano, flush=True)

    # Nascidos vivos.
    nasc = defaultdict(Counter)
    for a in anos:
        for r in registros(baixar(f"{SINASC}/DNBR{a}.dbc", pasta), ["CODMUNRES", "PESO", "PARTO", "IDADEMAE", "CONSULTAS"]):
            m = cod.get(str(r["CODMUNRES"] or "")[:6])
            if not m:
                continue
            c = nasc[m]
            c[f"nv_{a}"] += 1
            if a != ano:
                continue
            peso, parto, idade, consultas = (inteiro(r[k]) for k in ("PESO", "PARTO", "IDADEMAE", "CONSULTAS"))
            if peso:
                c["peso_n"] += 1
                c["peso_baixo"] += peso < 2500
            if parto in (1, 2):
                c["parto_n"] += 1
                c["cesarea"] += parto == 2
            if idade:
                c["idade_n"] += 1
                c["adolescente"] += idade < 20
            if consultas in (1, 2, 3, 4):
                c["consultas_n"] += 1
                c["prenatal7"] += consultas == 4
        print("SINASC", a, "lido", flush=True)

    # Óbitos.
    obit = defaultdict(Counter)
    for a in anos:
        for r in registros(baixar(f"{SIM}/DOBR{a}.dbc", pasta), ["CODMUNRES", "IDADE", "CAUSABAS"]):
            m = cod.get(str(r["CODMUNRES"] or "")[:6])
            if not m:
                continue
            c = obit[m]
            idade = str(r["IDADE"] or "")
            if idade[:1] in ("0", "1", "2", "3"):  # minutos, horas, dias ou meses: menos de 1 ano
                c[f"infantis_{a}"] += 1
            if a != ano:
                continue
            c["obitos"] += 1
            causa = str(r["CAUSABAS"] or "").upper()
            letra = causa[:1]
            if letra in "VWXY":
                c["externas"] += 1
            elif letra == "I":
                c["circulatorio"] += 1
            elif letra == "C" or (letra == "D" and causa[1:2] in "01234"):
                c["neoplasias"] += 1
            elif letra == "J":
                c["respiratorio"] += 1
        print("SIM", a, "lido", flush=True)

    # Leitos: o último mês com os 27 estados.
    nomes = listar(LEITOS)
    meses = sorted({m.group(1) for n in nomes if (m := re.fullmatch(r"LT[A-Z]{2}(\d{4})\.dbc", n, flags=re.I))})
    mes = next(mm for mm in reversed(meses) if all(f"LT{uf}{mm}.dbc".lower() in {n.lower() for n in nomes} for uf in UFS))
    leitos = defaultdict(Counter)
    for uf in UFS:
        nome = next(n for n in nomes if n.lower() == f"lt{uf}{mes}.dbc".lower())
        for r in registros(baixar(f"{LEITOS}/{nome}", pasta), ["CODUFMUN", "QT_EXIST", "QT_SUS"]):
            m = cod.get(str(r["CODUFMUN"] or "")[:6])
            if m:
                leitos[m]["existentes"] += inteiro(r["QT_EXIST"]) or 0
                leitos[m]["sus"] += inteiro(r["QT_SUS"]) or 0
    leitos_mes = f"20{mes[:2]}-{mes[2:]}"
    print("CNES leitos", leitos_mes, "lido", flush=True)

    # População.
    with urllib.request.urlopen(SIDRA.format(ano=ano), timeout=300) as r:
        serie = json.load(r)[0]["resultados"][0]["series"]
    pop = {s["localidade"]["id"]: inteiro(s["serie"].get(str(ano))) for s in serie}

    linhas = []
    for m in sorted(set(cod.values())):
        n, o, lt, p = nasc.get(m, Counter()), obit.get(m, Counter()), leitos.get(m, Counter()), pop.get(m)
        nv3 = sum(n[f"nv_{a}"] for a in anos)
        inf3 = sum(o[f"infantis_{a}"] for a in anos)
        linhas.append(
            {
                "codigo": m,
                "ano": ano,
                "nascidos": n[f"nv_{ano}"],
                "baixo_peso_pct": pct(n["peso_baixo"], n["peso_n"]),
                "cesarea_pct": pct(n["cesarea"], n["parto_n"]),
                "mae_adolescente_pct": pct(n["adolescente"], n["idade_n"]),
                "prenatal7_pct": pct(n["prenatal7"], n["consultas_n"]),
                "obitos": o["obitos"],
                "mortalidade_mil": f"{1000 * o['obitos'] / p:.2f}" if p else "",
                "mort_infantil": f"{1000 * inf3 / nv3:.1f}" if nv3 else "",
                "externas_pct": pct(o["externas"], o["obitos"]),
                "circulatorio_pct": pct(o["circulatorio"], o["obitos"]),
                "neoplasias_pct": pct(o["neoplasias"], o["obitos"]),
                "respiratorio_pct": pct(o["respiratorio"], o["obitos"]),
                "leitos": lt["existentes"],
                "leitos_sus": lt["sus"],
                "leitos_sus_mil": f"{1000 * lt['sus'] / p:.2f}" if p else "",
                "leitos_mes": leitos_mes,
                "populacao": p or "",
            }
        )
    with open(ROOT / "saude.csv", "w", encoding="utf-8", newline="\n") as f:
        w = csv.DictWriter(f, COLUNAS, lineterminator="\n")
        w.writeheader()
        w.writerows(linhas)
    tot = lambda k: sum(int(l[k] or 0) for l in linhas)  # noqa: E731
    print(f"saude.csv: {len(linhas)} municípios; {tot('nascidos')} nascidos e {tot('obitos')} óbitos em {ano}; {tot('leitos_sus')} leitos SUS em {leitos_mes}")


if __name__ == "__main__":
    main()
