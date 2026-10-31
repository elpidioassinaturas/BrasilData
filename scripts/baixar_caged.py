"""Escreve o caged.csv: o emprego formal (Novo CAGED) de cada município, para o plugin embutir.

    python scripts/baixar_caged.py              # baixa as duas séries do IPEA Data (~64 MB cada)
    python scripts/baixar_caged.py --de PASTA   # usa PASTA/ipea_ADMISNC.json e ipea_DESLIGNC.json

O IPEA Data publica o Novo CAGED sem ajuste por município, mês a mês desde 2020 (séries ADMISNC
e DESLIGNC), mas a API ignora o filtro e devolve a série inteira, longe do limite de 2 MB do
`host.fetch` de um plugin. Por isso o plugin leva um retrato: para cada município, o último mês
publicado e a soma dos 12 meses até ele. Rode de novo a cada versão do plugin.
"""

import csv
import json
import pathlib
import sys
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
SERIES = ("ADMISNC", "DESLIGNC")
URL = "https://www.ipeadata.gov.br/api/odata4/ValoresSerie(SERCODIGO='{}')"


def ler(serie: str, pasta: pathlib.Path | None) -> dict[tuple[str, str], float]:
    """Os valores municipais da série: (código IBGE, "AAAA-MM") -> valor."""
    if pasta:
        texto = (pasta / f"ipea_{serie}.json").read_text(encoding="utf-8")
    else:
        print("baixando", serie, "do IPEA Data...", flush=True)
        with urllib.request.urlopen(URL.format(serie), timeout=600) as r:
            texto = r.read().decode("utf-8")
    out = {}
    for v in json.loads(texto)["value"]:
        if v["NIVNOME"] != "Municípios" or v["VALVALOR"] is None:
            continue
        out[(v["TERCODIGO"], v["VALDATA"][:7])] = v["VALVALOR"]
    return out


def meses_ate(ultimo: str, n: int) -> list[str]:
    ano, mes = map(int, ultimo.split("-"))
    out = []
    for _ in range(n):
        out.append(f"{ano:04d}-{mes:02d}")
        ano, mes = (ano - 1, 12) if mes == 1 else (ano, mes - 1)
    return out


def main() -> None:
    pasta = pathlib.Path(sys.argv[sys.argv.index("--de") + 1]) if "--de" in sys.argv else None
    adm, desl = (ler(s, pasta) for s in SERIES)
    ultimo = min(max(m for _, m in adm), max(m for _, m in desl))
    janela = meses_ate(ultimo, 12)
    codigos = sorted({c for c, _ in adm} | {c for c, _ in desl})
    with open(ROOT / "caged.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["codigo", "mes", "adm_mes", "desl_mes", "adm_12m", "desl_12m"])
        for c in codigos:
            soma = lambda s: int(sum(s.get((c, m), 0) for m in janela))
            w.writerow([c, ultimo, int(adm.get((c, ultimo), 0)), int(desl.get((c, ultimo), 0)), soma(adm), soma(desl)])
    print(f"caged.csv: {len(codigos)} municípios, último mês {ultimo}, 12 meses desde {janela[-1]}")


if __name__ == "__main__":
    main()
