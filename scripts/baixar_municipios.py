"""Baixa a lista de municípios do IBGE para o municipios.csv que vai embutido no plugin.

    python scripts/baixar_municipios.py

Colunas: codigo (7 dígitos), nome, uf (sigla), uf_nome. A lista muda pouco (um município novo
de tempos em tempos); rode de novo e confira o diff.
"""

import csv
import gzip
import json
import pathlib
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
URL = "https://servicodados.ibge.gov.br/api/v1/localidades/municipios?view=nivelado"


def main() -> None:
    with urllib.request.urlopen(URL, timeout=120) as r:
        corpo = r.read()
    # A API responde comprimida mesmo sem pedir.
    if corpo[:2] == bytes([0x1F, 0x8B]):
        corpo = gzip.decompress(corpo)
    dados = json.loads(corpo)
    linhas = sorted(
        (
            (str(m["municipio-id"]), m["municipio-nome"], m["UF-sigla"], m["UF-nome"])
            for m in dados
        ),
        key=lambda l: (l[3], l[1]),
    )
    with open(ROOT / "municipios.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["codigo", "nome", "uf", "uf_nome"])
        w.writerows(linhas)
    print(len(linhas), "municípios em", len({l[2] for l in linhas}), "unidades da federação")


if __name__ == "__main__":
    main()
