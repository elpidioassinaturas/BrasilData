"""Baixa os limites do IBGE para o malhas.bin que vai embutido no plugin.

    python scripts/baixar_malhas.py

A escolha da área no painel não depende de rede: estados, mesorregiões, microrregiões e municípios
vêm da API de malhas do IBGE e ficam no plugin. A qualidade é a máxima da API nos três níveis de
cima e a intermediária nos municípios (a que o plugin sempre usou para eles). São contornos leves,
que o plugin pode ler; o contorno exato do IBGE continua nas camadas de Limites, com Área: seleção.

O arquivo (inteiros em little-endian; coordenadas em 1/10.000 de grau, as 4 casas da API):

    u32 quantas malhas
    para cada malha, em ordem de código: u32 código, u32 posição no arquivo
    para cada malha: polígonos, e em cada um os anéis, e em cada um os pontos, cada contagem em
    varint, e cada ponto como lon e lat em zigzag varint, a diferença para o ponto anterior

O código diz o nível pelo número de dígitos (2, 4, 5 ou 7). Os limites mudam pouco; rode de novo
e confira o que o script diz.
"""

import csv
import gzip
import json
import pathlib
import struct
import time
import urllib.request

ROOT = pathlib.Path(__file__).resolve().parent.parent
API = "https://servicodados.ibge.gov.br/api"
GEOJSON = "formato=application/vnd.geo%2Bjson"


def baixar(caminho: str) -> dict:
    """Um documento da API, com novas tentativas: ela falha de passagem em pedidos grandes."""
    for tentativa in range(1, 6):
        try:
            with urllib.request.urlopen(f"{API}/{caminho}", timeout=280) as r:
                corpo = r.read()
            # A API responde comprimida mesmo sem pedir.
            if corpo[:2] == bytes([0x1F, 0x8B]):
                corpo = gzip.decompress(corpo)
            return json.loads(corpo)
        except OSError as e:
            print(f"{caminho}: tentativa {tentativa}: {e}")
            time.sleep(5 * tentativa)
    raise SystemExit(f"a API não respondeu: {caminho}")


def varint(n: int) -> bytes:
    fora = bytearray()
    while n >= 0x80:
        fora.append(n & 0x7F | 0x80)
        n >>= 7
    fora.append(n)
    return bytes(fora)


def zigzag(n: int) -> bytes:
    return varint(n * 2 if n >= 0 else -n * 2 - 1)


def poligonos(geometria: dict) -> list:
    if geometria["type"] == "Polygon":
        return [geometria["coordinates"]]
    if geometria["type"] == "MultiPolygon":
        return geometria["coordinates"]
    raise SystemExit(f"geometria inesperada: {geometria['type']}")


def compactar(geometria: dict) -> bytes:
    fora = bytearray()
    x0, y0 = 0, 0
    lista = poligonos(geometria)
    fora += varint(len(lista))
    for poligono in lista:
        fora += varint(len(poligono))
        for anel in poligono:
            fora += varint(len(anel))
            for lon, lat in anel:
                x, y = round(lon * 10_000), round(lat * 10_000)
                fora += zigzag(x - x0) + zigzag(y - y0)
                x0, y0 = x, y
    return bytes(fora)


def vertices(geometria: dict) -> int:
    return sum(len(anel) for poligono in poligonos(geometria) for anel in poligono)


def main() -> None:
    with open(ROOT / "municipios.csv", encoding="utf-8", newline="") as f:
        linhas = list(csv.DictReader(f))
    esperados = {l["codigo"] for l in linhas}
    esperados |= {l["codigo"][:2] for l in linhas}
    esperados |= {l[c] for l in linhas for c in ("meso", "micro") if l[c]}

    malhas: dict[str, dict] = {}
    # Pedidos um a um, os estados vêm com mais do dobro dos vértices do pedido do Brasil inteiro.
    for uf in sorted({l["codigo"][:2] for l in linhas}):
        for f in baixar(f"v4/malhas/estados/{uf}?{GEOJSON}&qualidade=maxima")["features"]:
            malhas[f["properties"]["codarea"]] = f["geometry"]
    for nivel, qualidade in (("mesorregiao", "maxima"), ("microrregiao", "maxima"), ("municipio", "intermediaria")):
        for f in baixar(f"v3/malhas/paises/BR?intrarregiao={nivel}&{GEOJSON}&qualidade={qualidade}")["features"]:
            malhas[f["properties"]["codarea"]] = f["geometry"]
    # Um município novo pode faltar no pedido do Brasil inteiro.
    for codigo in sorted(esperados - malhas.keys()):
        try:
            for f in baixar(f"v4/malhas/municipios/{codigo}?{GEOJSON}&qualidade=intermediaria")["features"]:
                malhas[codigo] = f["geometry"]
        except SystemExit:
            pass
    faltam = sorted(esperados - malhas.keys())
    print("sem malha na API:", faltam or "nenhum")
    malhas = {c: g for c, g in malhas.items() if c in esperados}

    for digitos, nome in ((2, "estados"), (4, "mesorregiões"), (5, "microrregiões"), (7, "municípios")):
        do_nivel = {c: g for c, g in malhas.items() if len(c) == digitos}
        maior = max(do_nivel, key=lambda c: vertices(do_nivel[c]))
        total = sum(vertices(g) for g in do_nivel.values())
        print(f"{nome}: {len(do_nivel)}, {total} vértices, o maior é {maior} com {vertices(do_nivel[maior])}")

    codigos = sorted(malhas, key=int)
    corpo = bytearray()
    posicoes = []
    base = 4 + 8 * len(codigos)
    for c in codigos:
        posicoes.append(base + len(corpo))
        corpo += compactar(malhas[c])
    with open(ROOT / "malhas.bin", "wb") as f:
        f.write(struct.pack("<I", len(codigos)))
        for c, p in zip(codigos, posicoes):
            f.write(struct.pack("<II", int(c), p))
        f.write(corpo)
    print(f"malhas.bin: {base + len(corpo):,} bytes, {len(codigos)} malhas")


if __name__ == "__main__":
    main()
