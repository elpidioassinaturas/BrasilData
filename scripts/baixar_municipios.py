"""Baixa a lista de municípios do IBGE para o municipios.csv que vai embutido no plugin.

    python scripts/baixar_municipios.py

Colunas: codigo (7 dígitos), nome, uf (sigla), uf_nome, e a micro e a mesorregião do município
(micro, micro_nome, meso, meso_nome; vazias num município novo, que o IBGE não põe nessa divisão
de 1989). A lista muda pouco (um município novo de tempos em tempos); rode de novo e confira o
diff, e depois scripts/baixar_malhas.py.
"""

import csv
import gzip
import json
import pathlib
import unicodedata
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
URL = "https://servicodados.ibge.gov.br/api/v1/localidades/municipios?view=nivelado"


def dobrar(t: str) -> str:
    """`t` em minúsculas e sem acentos: a árvore do painel segue a ordem da tabela."""
    return "".join(c for c in unicodedata.normalize("NFD", t.casefold()) if not unicodedata.combining(c))


def main() -> None:
    with urllib.request.urlopen(URL, timeout=120) as r:
        corpo = r.read()
    # A API responde comprimida mesmo sem pedir.
    if corpo[:2] == bytes([0x1F, 0x8B]):
        corpo = gzip.decompress(corpo)
    dados = json.loads(corpo)
    linhas = sorted(
        (
            (
                str(m["municipio-id"]),
                m["municipio-nome"],
                m["UF-sigla"],
                m["UF-nome"],
                str(m["microrregiao-id"] or ""),
                m["microrregiao-nome"] or "",
                str(m["mesorregiao-id"] or ""),
                m["mesorregiao-nome"] or "",
            )
            for m in dados
        ),
        key=lambda l: (dobrar(l[3]), dobrar(l[7]), dobrar(l[5]), dobrar(l[1])),
    )
    # O plugin separa as colunas pela vírgula, sem aspas.
    if any("," in c for l in linhas for c in l):
        raise SystemExit("um nome tem vírgula")
    with open(ROOT / "municipios.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["codigo", "nome", "uf", "uf_nome", "micro", "micro_nome", "meso", "meso_nome"])
        w.writerows(linhas)
    print(len(linhas), "municípios em", len({l[2] for l in linhas}), "unidades da federação")


if __name__ == "__main__":
    main()
