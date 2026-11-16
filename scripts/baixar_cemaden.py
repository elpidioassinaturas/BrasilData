"""Baixa a lista das estações do CEMADEN para o cemaden.csv e o cemaden.bin, embutidos no plugin.

    python scripts/baixar_cemaden.py

O comando Chuva nas estações (CEMADEN) lê os acumulados de chuva por UF
(`getJson2.php?uf=`), que vêm sem coordenadas; elas vêm destas tabelas, pelo `id` da estação. O
arquivo do mapa interativo que tem as coordenadas (`311_24.json`, cerca de 2,2 MB) passa do
limite de 2 MB do `host.fetch`, e o GeoServer do CEMADEN não tem o WFS ligado.

- cemaden.csv: id (o `idestacao`), codigo (o código da estação, que começa pelo do município),
  nome, tipo (Pluviométrica, Acqua, Hidrológica, Geotécnica, Agrometeorológica), uf, cod_ibge,
  municipio (o nome do IBGE, do municipios.csv: o serviço da chuva perde acentos), lon e lat; em
  ordem de id.
- cemaden.bin: um registro de 16 bytes por estação, na mesma ordem: id, longitude e latitude
  (inteiros em milionésimos de grau) e onde começa a linha dela no cemaden.csv (u32 e i32,
  little-endian). O plugin acha uma estação por busca binária e só lê a linha das que usa: ler a
  tabela inteira a cada clique gastava boa parte da cota de instruções do comando.

As estações mudam pouco (umas dezenas por ano); rode de novo antes de cada versão e confira o diff.
"""

import csv
import io
import json
import pathlib
import struct
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
URL = "https://resources.cemaden.gov.br/dados/311_24.json"


def main() -> None:
    with urllib.request.urlopen(URL, timeout=120) as r:
        texto = r.read().decode("utf-8")
    # JSONP: estacoes([{"atualizado": ..., "estacao": [...]}])
    dados = json.loads(texto[texto.index("(") + 1 : texto.rindex(")")])
    with open(ROOT / "municipios.csv", encoding="utf-8", newline="") as f:
        municipios = {l["codigo"]: l["nome"] for l in csv.DictReader(f)}
    linhas = []
    for e in dados[0]["estacao"]:
        if e.get("latitude") is None or e.get("longitude") is None:
            continue
        cod = str(e["codibge"])
        linhas.append(
            (
                e["idestacao"],
                e["codestacao"],
                " ".join(e["nomeestacao"].split()),
                e["tipoestacao"],
                e["uf"],
                cod,
                municipios.get(cod, e["cidade"].title()),
                round(e["longitude"] * 1e6),
                round(e["latitude"] * 1e6),
            )
        )
    linhas.sort(key=lambda l: l[0])
    if len({l[0] for l in linhas}) != len(linhas):
        raise SystemExit("um idestacao se repete")
    # O plugin separa as colunas pela vírgula, sem aspas: a vírgula de um nome vira ponto e vírgula.
    limpa = lambda c: str(c).replace(",", ";").replace("\n", " ")
    saida = io.StringIO()
    saida.write("id,codigo,nome,tipo,uf,cod_ibge,municipio,lon,lat\n")
    registros = bytearray()
    for l in linhas:
        inicio = len(saida.getvalue().encode("utf-8"))
        registros += struct.pack("<Iiii", l[0], l[7], l[8], inicio)
        coords = [f"{l[7] / 1e6:.6f}".rstrip("0").rstrip("."), f"{l[8] / 1e6:.6f}".rstrip("0").rstrip(".")]
        saida.write(",".join([limpa(c) for c in l[:7]] + coords) + "\n")
    (ROOT / "cemaden.csv").write_bytes(saida.getvalue().encode("utf-8"))
    (ROOT / "cemaden.bin").write_bytes(bytes(registros))
    tipos: dict[str, int] = {}
    for l in linhas:
        tipos[l[3]] = tipos.get(l[3], 0) + 1
    print(len(linhas), "estações:", ", ".join(f"{n} {t}" for t, n in sorted(tipos.items())))


if __name__ == "__main__":
    main()
