"""Baixa os limites do IBGE para o malhas.bin que vai embutido no plugin.

    python scripts/baixar_malhas.py             # baixa tudo de novo
    python scripts/baixar_malhas.py --so-brasil  # só troca o contorno do Brasil no malhas.bin

A escolha da área no painel não depende de rede: estados, mesorregiões, microrregiões e municípios
vêm da API de malhas do IBGE e ficam no plugin. A qualidade é a máxima da API nos três níveis de
cima e a intermediária nos municípios (a que o plugin sempre usou para eles). São contornos leves,
que o plugin pode ler; o contorno exato do IBGE continua nas camadas de Limites, com Área: seleção.

O arquivo (inteiros em little-endian; coordenadas em 1/10.000 de grau, as 4 casas da API):

    u32 quantas malhas
    para cada malha, em ordem de código: u32 código, u32 posição no arquivo
    para cada malha: polígonos, e em cada um os anéis, e em cada um os pontos, cada contagem em
    varint, e cada ponto como lon e lat em zigzag varint, a diferença para o ponto anterior

O código diz o nível pelo número de dígitos (2, 4, 5 ou 7); o Brasil é o código 0. O contorno do
Brasil é a malha do país da API, na qualidade máxima dela (cerca de 4.000 vértices, um só contorno):
os 27 estados detalhados juntos passam da cota de instruções de uma chamada de plugin no RustGIS,
e simplificados um a um deixariam frestas nas divisas. As grandes regiões não têm malha própria no
arquivo: o plugin as monta com os seus estados. Os limites mudam pouco; rode de novo e confira o
que o script diz.
"""

import csv
import gzip
import json
import pathlib
import struct
import sys
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


def brasil() -> dict:
    """O contorno do Brasil: a malha do país da API, na qualidade máxima dela."""
    return baixar(f"v4/malhas/paises/BR?{GEOJSON}&qualidade=maxima")["features"][0]["geometry"]


def escrever(corpos: dict[int, bytes]) -> None:
    """Grava o malhas.bin com as malhas já compactadas, por código."""
    codigos = sorted(corpos)
    base = 4 + 8 * len(codigos)
    posicoes, pos = [], base
    for c in codigos:
        posicoes.append(pos)
        pos += len(corpos[c])
    with open(ROOT / "malhas.bin", "wb") as f:
        f.write(struct.pack("<I", len(codigos)))
        for c, p in zip(codigos, posicoes):
            f.write(struct.pack("<II", c, p))
        for c in codigos:
            f.write(corpos[c])
    print(f"malhas.bin: {pos:,} bytes, {len(codigos)} malhas")


def so_brasil() -> None:
    """Troca só o contorno do Brasil (código 0) no malhas.bin que já existe."""
    d = (ROOT / "malhas.bin").read_bytes()
    n = struct.unpack_from("<I", d, 0)[0]
    indice = [struct.unpack_from("<II", d, 4 + 8 * i) for i in range(n)]
    corpos = {}
    for i, (c, p) in enumerate(indice):
        fim = indice[i + 1][1] if i + 1 < n else len(d)
        corpos[c] = d[p:fim]
    g = brasil()
    print(f"Brasil: {vertices(g)} vértices")
    corpos[0] = compactar(g)
    escrever(corpos)


def main() -> None:
    if "--so-brasil" in sys.argv[1:]:
        so_brasil()
        return
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
    malhas["0"] = brasil()

    for digitos, nome in ((1, "Brasil"), (2, "estados"), (4, "mesorregiões"), (5, "microrregiões"), (7, "municípios")):
        do_nivel = {c: g for c, g in malhas.items() if len(c) == digitos}
        maior = max(do_nivel, key=lambda c: vertices(do_nivel[c]))
        total = sum(vertices(g) for g in do_nivel.values())
        print(f"{nome}: {len(do_nivel)}, {total} vértices, o maior é {maior} com {vertices(do_nivel[maior])}")

    escrever({int(c): compactar(g) for c, g in malhas.items()})


if __name__ == "__main__":
    main()
