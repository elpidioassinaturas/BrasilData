"""Escreve o tse.csv: eleitorado, comparecimento e abstenção de cada município, para o plugin embutir.

    python scripts/baixar_tse.py

Da divulgação oficial do TSE (`resultados.tse.jus.br`), 1º turno de 2026 (eleição 6257) e de 2024
(619): a tabela dos municípios (`mun-e<eleição>-cm.json`, que liga o código do TSE ao do IBGE) e
um arquivo de comparecimento por UF (`<uf>-e<eleição>-ab.json`, ~190 KB cada). Lidos no plugin a
cada clique, esses arquivos gastavam o orçamento de uma chamada; os resultados de um turno
encerrado não mudam. Rode de novo depois de cada eleição, acrescentando-a em ELEICOES.
"""

import csv
import json
import pathlib
import urllib.error
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
BASE = "https://resultados.tse.jus.br/oficial"
# (ciclo, código da eleição, ano dos campos)
ELEICOES = [("ele2026", "6257", 2026), ("ele2024", "619", 2024)]


def baixar(url: str) -> dict:
    with urllib.request.urlopen(url, timeout=120) as r:
        return json.loads(r.read().decode("utf-8"))


def numero(t: str | None) -> float | None:
    return None if t in (None, "") else float(t.replace(",", "."))


def main() -> None:
    ciclo, cod, _ = ELEICOES[0]
    cm = baixar(f"{BASE}/{ciclo}/{cod}/config/mun-e{cod:0>6}-cm.json")
    ibge = {m["cd"]: m["cdi"] for uf in cm["abr"] for m in uf["mu"] if m.get("cdi")}
    ufs = sorted({uf["cd"] for uf in cm["abr"] if uf["cd"] not in ("br", "zz")})
    linhas: dict[str, dict] = {c: {} for c in ibge.values()}
    for ciclo, cod, ano in ELEICOES:
        for uf in ufs:
            try:
                ab = baixar(f"{BASE}/{ciclo}/{cod}/dados/{uf}/{uf}-e{cod:0>6}-ab.json")
            except urllib.error.HTTPError as e:
                # O DF não tem eleição municipal.
                if e.code != 404:
                    raise
                print(ano, uf, "sem arquivo")
                continue
            for a in ab["abr"]:
                c = ibge.get(a.get("cdabr"))
                if a.get("tpabr") != "mun" or c is None:
                    continue
                e = a["e"]
                linhas[c][f"eleitores_{ano}"] = int(e["te"])
                linhas[c][f"comparec_{ano}"] = round(numero(e.get("pcn") or e.get("pc")), 2)
                linhas[c][f"abstencao_{ano}"] = round(numero(e.get("pan") or e.get("pa")), 2)
        print(ano, "lido", flush=True)
    campos = [f"{k}_{ano}" for _, _, ano in ELEICOES for k in ("eleitores", "comparec", "abstencao")]
    with open(ROOT / "tse.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["ibge"] + campos)
        for c in sorted(linhas):
            w.writerow([c] + [linhas[c].get(k, "") for k in campos])
    print(f"tse.csv: {len(linhas)} municípios, {len(ufs)} UFs")


if __name__ == "__main__":
    main()
