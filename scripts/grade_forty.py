"""Lê a grade dos blocos do ForTy 2020 (classificação) nas quadrículas MGRS do Brasil.

Para cada quadrícula (fuso 18 a 25, faixas H a N): a listagem do bucket diz quais blocos existem;
o cabeçalho de um bloco por linha e por coluna dá a origem, a largura e a altura de cada um.
Escreve src/forty_grade.rs. Precisa de rede.
"""
import json, struct, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor
PASTA = "export_rasters/forest_typology_2020_v1_0_classification/"
BASE = "https://storage.googleapis.com/nature-trace/" + PASTA
FAIXAS = "HJKLMN"


def listar(celula):
    nomes, token = [], None
    while True:
        url = f"https://storage.googleapis.com/storage/v1/b/nature-trace/o?prefix={PASTA}raster_{celula}_&fields=items(name),nextPageToken&maxResults=1000"
        if token:
            url += "&pageToken=" + token
        d = json.load(urllib.request.urlopen(url))
        nomes += [i["name"] for i in d.get("items", [])]
        token = d.get("nextPageToken")
        if not token:
            return nomes


def cabecalho(nome):
    """(x do oeste, y da origem, largura, altura, tamanho x, tamanho y, EPSG) do bloco, do cabeçalho
    TIFF lido nos primeiros 64 KB (o GDAL trava ao abrir arquivos remotos em várias threads)."""
    pedido = urllib.request.Request(BASE + nome, headers={"Range": "bytes=0-65535"})
    b = urllib.request.urlopen(pedido, timeout=60).read()
    grande = b[2] == 43  # BigTIFF ("II+"), como o Earth Engine os grava
    assert b[:2] == b"II" and b[2] in (42, 43), nome
    if grande:
        (ifd,) = struct.unpack_from("<Q", b, 8)
        (n,) = struct.unpack_from("<Q", b, ifd)
        entrada, passo, cabe = "<HHQQ", 20, 8
    else:
        (ifd,) = struct.unpack_from("<I", b, 4)
        (n,) = struct.unpack_from("<H", b, ifd)
        entrada, passo, cabe = "<HHII", 12, 4
    inicio = ifd + (8 if grande else 2)
    tam = {3: 2, 4: 4, 12: 8, 16: 8}
    fmt = {3: "H", 4: "I", 12: "d", 16: "Q"}
    tags = {}
    for k in range(n):
        tag, tipo, cont, val = struct.unpack_from(entrada, b, inicio + passo * k)
        if tipo not in tam:
            continue
        pos = inicio + passo * k + (12 if grande else 8) if tam[tipo] * cont <= cabe else val
        tags[tag] = struct.unpack_from("<%d%s" % (cont, fmt[tipo]), b, pos)
    largura, altura = tags[256][0], tags[257][0]
    # A matriz de transformação (ModelTransformationTag): o y cresce para o norte.
    m = tags[34264]
    sx, x, sy, y = m[0], m[3], m[5], m[7]
    assert m[1] == 0 and m[4] == 0, nome
    chaves = tags[34735]
    epsg = next(chaves[4 + 4 * k + 3] for k in range(chaves[3]) if chaves[4 + 4 * k] == 3072)
    return x, y, largura, altura, sx, sy, epsg


def celula(z, f):
    nome = f"{z}{f}"
    blocos = set()
    for n in listar(nome):
        partes = n.rsplit("/", 1)[1][:-4].split("_")  # raster_23L_20_r_c
        assert partes[2] == "20", n
        blocos.add((int(partes[3]), int(partes[4])))
    if not blocos:
        return None
    linhas = {}
    colunas = {}
    for r, c in sorted(blocos):
        linhas.setdefault(r, c)
        colunas.setdefault(c, r)
    pedidos = {(r, c) for r, c in linhas.items()} | {(r, c) for c, r in colunas.items()}
    with ThreadPoolExecutor(16) as ex:
        hs = dict(zip(pedidos, ex.map(lambda rc: cabecalho(f"raster_{nome}_20_{rc[0]}_{rc[1]}.tif"), pedidos)))
    epsg = {h[6] for h in hs.values()}
    assert epsg in ({32600 + z}, {32700 + z}), (nome, epsg)
    assert all(h[4] == 10 and h[5] == 10 for h in hs.values()), nome
    # O oeste e a largura de cada coluna (as larguras variam de 16 células entre colunas).
    xs = {}
    for (r, c), h in hs.items():
        xs.setdefault(c, (h[0], h[2] * 10))
        assert xs[c] == (h[0], h[2] * 10), (nome, c)
    # A origem e a altura de cada linha (o arquivo vai do sul para o norte: y é o sul).
    ys = {}
    for (r, c), h in hs.items():
        ys.setdefault(r, (h[1], h[3] * 10))
        assert ys[r] == (h[1], h[3] * 10), (nome, r)
    print(nome, len(blocos), "blocos", flush=True)
    return {"celula": nome, "fuso": z, "faixa": f, "norte": epsg == {32600 + z}, "colunas": xs, "linhas": ys, "blocos": blocos}


def main():
    cels = [(z, f) for z in range(18, 26) for f in FAIXAS]
    with ThreadPoolExecutor(4) as ex:
        res = [r for r in ex.map(lambda a: celula(*a), cels) if r]
    out = ["//! A grade dos blocos do ForTy 2020 nas quadrículas MGRS do Brasil, gerada por",
           "//! `scripts/grade_forty.py` dos cabeçalhos dos arquivos (não edite à mão).",
           "",
           "/// Uma quadrícula MGRS: fuso, faixa, o oeste e a largura de cada coluna que tem bloco (m),",
           "/// o sul e a altura de cada linha que tem bloco (m), e os blocos que existem, uma linha de",
           "/// 20 bits por linha (o bit `c` é a coluna `c`).",
           "pub struct Quadricula {",
           "    pub fuso: u8,",
           "    pub faixa: char,",
           "    /// No hemisfério norte (EPSG:326xx, sem o falso norte de 10.000 km).",
           "    pub norte: bool,",
           "    pub colunas: &'static [(u8, f64, f64)],",
           "    pub linhas: &'static [(u8, f64, f64)],",
           "    pub blocos: [u32; 20],",
           "}",
           "",
           "pub const QUADRICULAS: &[Quadricula] = &["]
    for q in sorted(res, key=lambda q: (q["fuso"], q["faixa"])):
        linhas = ", ".join(f"({r}, {y:.1f}, {h:.1f})" for r, (y, h) in sorted(q["linhas"].items()))
        colunas = ", ".join(f"({c}, {x:.1f}, {w:.1f})" for c, (x, w) in sorted(q["colunas"].items()))
        bits = [0] * 20
        for r, c in q["blocos"]:
            bits[r] |= 1 << c
        out.append(f"    Quadricula {{ fuso: {q['fuso']}, faixa: '{q['faixa']}', norte: {'true' if q['norte'] else 'false'}, colunas: &[{colunas}], linhas: &[{linhas}], blocos: {bits} }},")
    out.append("];")
    open(sys.argv[1], "w", encoding="utf-8", newline="\n").write("\n".join(out) + "\n")
    print(len(res), "quadrículas,", sum(len(q["blocos"]) for q in res), "blocos")


main()
