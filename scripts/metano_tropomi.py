"""A coluna média de metano (XCH4, ppb) do Sentinel-5P/TROPOMI sobre o Brasil, em grade de 0,1°.

Não há produto oficial do TROPOMI em grade fina (o oficial é por órbita, nível 2; o gradeado do
Copernicus CDS tem 1°). Este script lê as órbitas de metano do ano no Microsoft Planetary Computer
(coleção `sentinel-5p-l2-netcdf`, sem login), só as variáveis necessárias e só as linhas sobre o
Brasil (pedidos parciais HTTP, cerca de 8 s por órbita), e acumula numa grade de 0,1° (~11 km):

- `methane_mixing_ratio_bias_corrected` (ppb), dos pixels com `qa_value` ≥ 0,5 (a recomendação do
  produto), pelo centro do pixel;
- a média de cada mês (células com 3 observações ou mais) e a do ano (10 ou mais), e quantas
  observações entraram em cada célula.

Escolha do ano: o Planetary Computer não tem as órbitas de 16/11 a 31/12/2024, nem a maior parte de
julho e agosto de 2025; 2023 tem todos os meses (de 84 a 95 órbitas).

Grava GeoTIFFs otimizados para a web (COG, EPSG:4326): `metano_tropomi_<ano>.tif` (média anual),
`metano_tropomi_<ano>_<mm>.tif` (cada mês) e `metano_tropomi_<ano>_obs.tif` (observações no ano).
Contém dados modificados do Copernicus Sentinel (ESA), processados pelo BrasilData.

Uso: uv run --python 3.12 --with h5py --with fsspec --with aiohttp --with requests --with numpy \
         --with rasterio python scripts/metano_tropomi.py 2023 saida/
"""

from __future__ import annotations

import json
import sys
import time
import urllib.request
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

import fsspec
import h5py
import numpy as np

STAC = "https://planetarycomputer.microsoft.com/api/stac/v1/search"
TOKEN = "https://planetarycomputer.microsoft.com/api/sas/v1/token/sentinel5euwest/sentinel-5p"
OESTE, SUL, LESTE, NORTE = -74.0, -34.0, -34.0, 6.0
PASSO = 0.1
NX = int(round((LESTE - OESTE) / PASSO))
NY = int(round((NORTE - SUL) / PASSO))
QA_MINIMO = 0.5
# Observações mínimas numa célula: com menos, a média é ruído (em 2024 uma célula de uma ou duas
# observações deu 1.016 ppb, contra a mediana de 1.874).
MIN_OBS_MES = 3
MIN_OBS_ANO = 10


def _json(url: str, corpo: dict | None = None) -> dict:
    dados = json.dumps(corpo).encode() if corpo is not None else None
    pedido = urllib.request.Request(url, dados, {"Content-Type": "application/json"} if dados else {})
    for tentativa in range(5):
        try:
            with urllib.request.urlopen(pedido, timeout=120) as r:
                return json.load(r)
        except Exception:
            if tentativa == 4:
                raise
            time.sleep(5 * (tentativa + 1))
    raise RuntimeError("inalcançável")


def orbitas_do_mes(ano: int, mes: int) -> list[str]:
    ultimo = (np.datetime64(f"{ano}-{mes:02d}") + np.timedelta64(1, "M")).astype("datetime64[D]") - np.timedelta64(1, "D")
    corpo = {
        "collections": ["sentinel-5p-l2-netcdf"],
        "bbox": [OESTE, SUL, LESTE, NORTE],
        "datetime": f"{ano}-{mes:02d}-01T00:00:00Z/{ultimo}T23:59:59Z",
        "query": {"s5p:product_name": {"eq": "ch4"}, "s5p:processing_mode": {"eq": "OFFL"}},
        "limit": 1000,
    }
    urls = []
    while True:
        d = _json(STAC, corpo)
        urls += [f["assets"]["ch4"]["href"] for f in d["features"]]
        prox = next((l for l in d.get("links", []) if l.get("rel") == "next"), None)
        if not prox:
            break
        corpo = prox.get("body") or corpo
    return sorted(set(urls))


def ler_orbita(url: str, token: str) -> tuple[np.ndarray, np.ndarray, np.ndarray] | None:
    fs = fsspec.filesystem("https", block_size=2**20)
    for tentativa in range(4):
        try:
            with fs.open(f"{url}?{token}", "rb", cache_type="readahead") as fh, h5py.File(fh, "r") as h:
                p = h["PRODUCT"]
                lat = p["latitude"][0]
                lon = p["longitude"][0]
                dentro = (lat > SUL) & (lat < NORTE) & (lon > OESTE) & (lon < LESTE)
                linhas = np.flatnonzero(dentro.any(axis=1))
                if len(linhas) == 0:
                    return None
                a, b = linhas.min(), linhas.max() + 1
                ch4 = p["methane_mixing_ratio_bias_corrected"][0, a:b]
                qa = p["qa_value"][0, a:b]
                lat, lon = lat[a:b], lon[a:b]
                ok = dentro[a:b] & (qa >= QA_MINIMO) & (ch4 < 1e30) & np.isfinite(ch4)
                return lat[ok], lon[ok], ch4[ok].astype(np.float64)
        except Exception as e:
            if tentativa == 3:
                print(f"  falhou {url.rsplit('/', 1)[-1]}: {e}", flush=True)
                return None
            time.sleep(5 * (tentativa + 1))
    return None


def acumular(soma: np.ndarray, n: np.ndarray, lat: np.ndarray, lon: np.ndarray, v: np.ndarray) -> None:
    col = ((lon - OESTE) / PASSO).astype(int)
    lin = ((NORTE - lat) / PASSO).astype(int)
    ok = (col >= 0) & (col < NX) & (lin >= 0) & (lin < NY)
    np.add.at(soma, (lin[ok], col[ok]), v[ok])
    np.add.at(n, (lin[ok], col[ok]), 1)


def gravar(caminho: Path, dados: np.ndarray, nodata: float, tipo: str) -> None:
    import rasterio
    from rasterio.transform import from_origin

    perfil = {
        "driver": "COG",
        "width": NX,
        "height": NY,
        "count": 1,
        "dtype": tipo,
        "crs": "EPSG:4326",
        "transform": from_origin(OESTE, NORTE, PASSO, PASSO),
        "nodata": nodata,
        "compress": "deflate",
        "blocksize": 256,
    }
    with rasterio.open(caminho, "w", **perfil) as dst:
        dst.write(dados.astype(tipo), 1)
    print(f"  {caminho.name} ({caminho.stat().st_size / 1e3:.0f} kB)", flush=True)


def main() -> None:
    ano, saida = int(sys.argv[1]), Path(sys.argv[2])
    saida.mkdir(parents=True, exist_ok=True)
    t = time.time()
    soma_ano = np.zeros((NY, NX))
    n_ano = np.zeros((NY, NX), dtype=np.int64)
    for mes in range(1, 13):
        parcial = saida / f"parcial_{ano}_{mes:02d}.npz"
        if parcial.exists():
            z = np.load(parcial)
            soma, n = z["soma"], z["n"]
        else:
            urls = orbitas_do_mes(ano, mes)
            token = _json(TOKEN)["token"]
            soma = np.zeros((NY, NX))
            n = np.zeros((NY, NX), dtype=np.int64)
            # Processos, não threads: as threads dividem o laço de eventos do fsspec e leem em fila.
            with ProcessPoolExecutor(8) as ex:
                for fut in as_completed([ex.submit(ler_orbita, u, token) for u in urls]):
                    r = fut.result()
                    if r is not None:
                        acumular(soma, n, *r)
            np.savez_compressed(parcial, soma=soma, n=n)
            print(f"{ano}-{mes:02d}: {len(urls)} órbitas, {int(n.sum())} observações ({time.time() - t:.0f} s)", flush=True)
        media = np.where(n >= MIN_OBS_MES, soma / np.maximum(n, 1), np.nan)
        gravar(saida / f"metano_tropomi_{ano}_{mes:02d}.tif", np.where(np.isnan(media), -9999, media), -9999, "float32")
        soma_ano += soma
        n_ano += n
    media = np.where(n_ano >= MIN_OBS_ANO, soma_ano / np.maximum(n_ano, 1), np.nan)
    gravar(saida / f"metano_tropomi_{ano}.tif", np.where(np.isnan(media), -9999, media), -9999, "float32")
    gravar(saida / f"metano_tropomi_{ano}_obs.tif", n_ano, 0, "int32")
    v = media[np.isfinite(media)]
    print(f"pronto ({time.time() - t:.0f} s): {len(v)} células, XCH4 de {v.min():.0f} a {v.max():.0f} ppb, mediana {np.median(v):.0f}", flush=True)


if __name__ == "__main__":
    main()
