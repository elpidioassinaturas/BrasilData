"""Faz o mapa exemplo de cada camada, para as fichas (docs/camadas/<id>.png).

    py -3.12 -X utf8 scripts/mapas_exemplo.py [id ...] [--cli rustgis-cli] [--workers 4] [--refazer]

Para cada ficha do fichas.csv com área de exemplo, um rustgis-cli novo, com o plugin de
dist/brasildata: escolhe a área (Viçosa, a Zona da Mata ou o município da ficha), roda o comando,
pinta as classes pelo campo da coluna classes (valores únicos, rampa tab20), faz o layout com o
botão Layout (folha A3 deitada, estilo painel: o mapa à esquerda; o título, o globo, a legenda e as
fontes à direita), tira o "BrasilData: " do nome da camada na legenda e exporta a folha a 80 dpi.
O PNG é reduzido a 256 cores (FASTOCTREE). Sem ids, faz os que ainda não têm PNG (com --refazer, todos). O
registro de cada um vai para saida/mapas_exemplo/<id>.log. Depois, rode scripts/gerar.py: as
fichas passam a mostrar os mapas.
"""

import argparse
import csv
import json
import os
import pathlib
import shutil
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor

ROOT = pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import gerar  # noqa: E402

PASTA = ROOT / gerar.PAGINAS
LOGS = ROOT / "saida" / "mapas_exemplo"
# Os parâmetros do comando no mapa exemplo, quando o padrão não serve: o MapBiomas recortado só
# vai de 2019 a 2024; a radiação solar e a temperatura, em Minas Gerais, numa grade de 1 km.
PARAMS = {"mapbiomas-cobertura": {"ano": 2024}, "bdc-sem-nuvens": {"inicio": "01/06/2025", "fim": "31/08/2025"}, "sol-radiacao": {"celula": 1000}, "temperatura-prevista": {"celula": 1000}}
# Os comandos que pedem o login Earthdata (pelo começo do id).
EARTHDATA = ("gedi-", "radar-", "nisar-", "dist-")
# A rampa das classes, quando a tab20 (categorias sem ordem) não combina com o tema: os solos em
# cores vivas (com poucas classes, as rampas em sequência deixam a primeira quase branca), os anos do PRODES do mais claro ao mais escuro.
RAMPAS = {"solos": "Set2", "vegetacao": "Set3", "geomorfologia": "tab20", "ana-bho": "Blues"}
RAMPAS.update({i: "YlOrRd" for i in ("prodes-cerrado", "prodes-caatinga", "prodes-amazonia", "prodes-mata-atlantica", "prodes-pantanal", "prodes-pampa")})
# Os nomes das camadas que não estão no fontes.csv (para as classes e a legenda), quando são fixos.
NOMES = {
    "saude-estabelecimentos": "BrasilData: Estabelecimentos de saúde (CNES 04/2026)",
    "censo-setores": "BrasilData: Setores censitários 2022 (MG)",
    "osm-vias": "BrasilData: Ruas e estradas (OpenStreetMap, Overture)",
    **{i: "BrasilData: Indicadores por município" for i in ("censo-municipios", "censo-agro", "censo-agroecologia", "indicadores-pix", "indicadores-credito", "indicadores-tse", "indicadores-caged", "indicadores-todos")},
}


def cmd(cid: str, params: dict) -> list[str]:
    return ["--cmd", f"{cid}={json.dumps(params, ensure_ascii=False)}"]


def fazer(cli: str, plugin: pathlib.Path, fi: dict, nome: str | None) -> tuple[str, str, float]:
    cid = fi["id"]
    no, _ = gerar.area_do_exemplo(fi["exemplo"])
    png = PASTA / f"{cid}.png"
    bruto = LOGS / f"{cid}.png"
    args = [cli, "run", "--no-plugins", "--plugin", str(plugin), "--yes", "--use-saved-secrets"]
    args += cmd("plugin:brasildata/escolher-area", {"no": no})
    args += cmd(f"plugin:brasildata/{cid}", PARAMS.get(cid, {}))
    # Os do login Earthdata: o primeiro clique depois de o RustGIS ler o token guardado é
    # cancelado por segurança, e o plugin pede outro.
    if cid.startswith(EARTHDATA):
        args += cmd(f"plugin:brasildata/{cid}", PARAMS.get(cid, {}))
    if nome and fi["classes"]:
        args += cmd("layer.symbology", {"layer": nome, "type": "unique", "field": fi["classes"], "ramp": RAMPAS.get(cid, "tab20")})
    args += cmd("plugin:brasildata/layout", {"perfil": "a3-landscape", "estilo": "panel"})
    if nome:
        args += cmd("layer.rename", {"layer": nome, "name": nome.removeprefix(gerar_prefixo())})
    args += cmd("layout.export", {"path": str(bruto).replace("\\", "/"), "dpi": 80})
    if bruto.exists():
        bruto.unlink()
    t0 = time.time()
    try:
        r = subprocess.run(args, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=1200)
        saida, erro = r.stdout, r.stderr
    except subprocess.TimeoutExpired:
        saida, erro = "", "TIMEOUT 1200 s"
    dt = time.time() - t0
    (LOGS / f"{cid}.log").write_text(f"$ {' '.join(args)}\n--- stderr ---\n{erro}\n--- stdout ---\n{saida[-20000:]}", encoding="utf-8")
    if not bruto.exists():
        # O porquê: a última mensagem do plugin.
        msgs = [l for l in erro.splitlines() if "brasildata" in l.lower()]
        return cid, "FALHOU: " + (msgs[-1][:200] if msgs else erro.strip()[-200:]), dt
    from PIL import Image

    # FASTOCTREE guarda as cores de pouca área (a barra de cores azul de um mapa quase todo quente),
    # que o MEDIANCUT trocava por cinzas.
    Image.open(bruto).convert("RGB").quantize(256, method=Image.Quantize.FASTOCTREE).save(png, optimize=True)
    return cid, f"ok, {png.stat().st_size // 1024} KB", dt


def gerar_prefixo() -> str:
    return "BrasilData: "


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("ids", nargs="*")
    ap.add_argument("--cli", default=os.environ.get("RUSTGIS_CLI") or shutil.which("rustgis-cli") or "rustgis-cli")
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--refazer", action="store_true")
    a = ap.parse_args()
    plugin = ROOT / "dist" / "brasildata"
    if not (plugin / "plugin.wasm").exists():
        sys.exit("falta dist/brasildata: rode python scripts/package.py")
    linhas = gerar.ler()
    nomes = {l["id"]: gerar_prefixo() + l["nome"] for l in linhas} | NOMES
    with open(ROOT / "fichas.csv", encoding="utf-8", newline="") as f:
        fichas = [fi for fi in csv.DictReader(f) if fi["exemplo"]]
    if a.ids:
        fora = set(a.ids) - {fi["id"] for fi in fichas}
        if fora:
            sys.exit(f"sem ficha com área de exemplo: {', '.join(sorted(fora))}")
        fichas = [fi for fi in fichas if fi["id"] in a.ids]
    elif not a.refazer:
        fichas = [fi for fi in fichas if not (PASTA / f"{fi['id']}.png").exists()]
    PASTA.mkdir(parents=True, exist_ok=True)
    LOGS.mkdir(parents=True, exist_ok=True)
    print(f"{len(fichas)} mapas, {a.workers} de cada vez")
    with ThreadPoolExecutor(a.workers) as ex:
        for cid, res, dt in ex.map(lambda fi: fazer(a.cli, plugin, fi, nomes.get(fi["id"])), fichas):
            print(f"{cid:28} {dt:5.0f} s  {res}", flush=True)


if __name__ == "__main__":
    main()
