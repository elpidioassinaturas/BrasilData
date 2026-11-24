"""As luzes noturnas do Brasil de 1992 a 2024, um GeoTIFF otimizado para a web (COG) por ano.

De onde: PCNL, "A Pixel-scale Corrected Nighttime Light Dataset (PCNL, 1992-2024) Combining DMSP-OLS
and NPP-VIIRS" (Li, Cao et al.; doi:10.5281/zenodo.17013414, CC BY 4.0), um GeoTIFF do mundo por ano,
em blocos: só os blocos do Brasil são lidos (pedidos parciais HTTP), e o mundo não é baixado. O
recorte existe porque o Zenodo limita os pedidos por minuto de cada endereço de rede, e os arquivos
não têm visões: numa área grande, o plugin passava do limite.

O que faz: de cada ano, a janela do Brasil, célula a célula, sem reamostrar nem mudar valores
(números de 32 bits), com visões por média; confere a grade de cada arquivo e os valores.

Uso: python scripts/luzes_noturnas.py <pasta de saída> [ano ...]   (precisa de rasterio)
"""

import os
import sys
import time
from pathlib import Path

os.environ.update(GDAL_DISABLE_READDIR_ON_OPEN="EMPTY_DIR", CPL_VSIL_CURL_ALLOWED_EXTENSIONS=".tif", GDAL_HTTP_TIMEOUT="120")

import numpy as np  # noqa: E402
import rasterio  # noqa: E402
from rasterio.shutil import copy as copiar  # noqa: E402
from rasterio.transform import from_origin  # noqa: E402
from rasterio.windows import Window  # noqa: E402

ORIGEM = "/vsicurl/https://zenodo.org/records/17013414/files/PCNL{}.tif"
ANOS = range(1992, 2025)
# A caixa do Brasil, com as ilhas oceânicas, em graus.
OESTE, SUL, LESTE, NORTE = -74.05, -34.0, -28.75, 5.35
CELULA = 1 / 120
# Pausas do limite do Zenodo.
TENTATIVAS, PAUSA = 8, 40


def ler(ano: int) -> tuple[np.ndarray, float, float]:
    for k in range(TENTATIVAS):
        try:
            with rasterio.open(ORIGEM.format(ano)) as d:
                g = d.transform.a
                assert (d.width, d.height, d.count) == (43201, 16802, 1) and abs(g - CELULA) < 1e-9 and d.dtypes[0] == "float32", (ano, d.profile)
                c0, l0 = round((OESTE - d.transform.c) / g), round((d.transform.f - NORTE) / g)
                nc, nl = round((LESTE - OESTE) / g), round((NORTE - SUL) / g)
                a = d.read(1, window=Window(c0, l0, nc, nl))
                return a, d.transform.c + c0 * g, d.transform.f - l0 * g
        except rasterio.errors.RasterioError as e:
            print(ano, "tentativa", k + 1, str(e)[:120], flush=True)
            time.sleep(PAUSA)
    sys.exit(f"{ano}: o Zenodo não respondeu")


def main() -> None:
    saida = Path(sys.argv[1])
    saida.mkdir(parents=True, exist_ok=True)
    for ano in [int(x) for x in sys.argv[2:]] or ANOS:
        f = saida / f"luzes_noturnas_{ano}.tif"
        if f.exists():
            continue
        t = time.time()
        a, x0, y0 = ler(ano)
        assert np.isfinite(a).all() and a.min() >= 0 and a.max() < 1e4, (ano, a.min(), a.max())
        perfil = {"driver": "GTiff", "width": a.shape[1], "height": a.shape[0], "count": 1, "dtype": "float32", "crs": "EPSG:4326", "transform": from_origin(x0, y0, CELULA, CELULA)}
        with rasterio.MemoryFile() as m:
            with m.open(**perfil) as d:
                d.write(a, 1)
                d.update_tags(
                    TITULO=f"Luzes noturnas {ano} (PCNL, recorte do Brasil)",
                    FONTE="Li, S., Cao, X. et al., Pixel-scale Corrected Nighttime Light Dataset (PCNL, 1992-2024), doi:10.5281/zenodo.17013414, CC BY 4.0",
                )
            with m.open() as d:
                copiar(d, f, driver="COG", compress="DEFLATE", predictor=3, level=9, blocksize=512, overview_resampling="average")
        print(ano, f.stat().st_size // 1024, "KB; máximo %.2f" % a.max(), "; acesas %.2f%%" % (100 * (a > 0).mean()), "; soma %.0f" % a.astype("f8").sum(), "; %.0f s" % (time.time() - t), flush=True)
        time.sleep(15)


if __name__ == "__main__":
    main()
