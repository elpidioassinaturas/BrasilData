"""Os rios da drenagem da BC250 em linhas contínuas, para os rótulos (saída de ordem_drenagem.py).

A BC250 corta cada rio em cada confluência, em trechos de poucos km: um rótulo ao longo da linha
precisa de um trecho contínuo mais comprido que o texto, e quase nenhum cabia. Aqui os trechos de
mesmo nome que se tocam viram uma linha só (`line_merge`), simplificada em 400 m (o rótulo curvo
desiste onde a linha dobra demais de uma letra para a outra), com a maior ordem de Strahler e a
maior drenagem a montante dos trechos que ela cobre, e o comprimento dela (`comp_km`). Só se
juntam trechos que se tocam: rios homônimos em lugares diferentes (os 106 "Rio Preto" do país)
ficam linhas separadas, cada uma com a ordem dos seus trechos.

Grava:
- `rios_nomes.parquet`: o país inteiro;
- `rios_nomes_ordem6.parquet`: só os de ordem 6 ou mais, para as escalas pequenas (o RustGIS lê
  até 60 mil feições por vista, e uma amostra do arquivo inteiro deixaria rios grandes sem nome);
- `uf/rios_nomes_<UF>.parquet`: as linhas recortadas por UF (um rio que cruza UFs fica em cada
  uma, na parte dela).

Uso: uv run --python 3.12 --with geopandas --with pyogrio --with pyarrow \
         python scripts/rios_para_rotulos.py saida/bc250_2025_drenagem_ordem.parquet bruto/bc250_....zip saida/
"""

from __future__ import annotations

import sys
import time
import zipfile
from pathlib import Path

import geopandas as gpd
import numpy as np
import pyogrio
import shapely

TOLERANCIA_M = 400


def gravar(df: gpd.GeoDataFrame, caminho: Path) -> None:
    df = df.iloc[df.geometry.hilbert_distance().argsort()]
    df.to_parquet(caminho, write_covering_bbox=True, row_group_size=10_000, compression="zstd")
    print(f"  {caminho.name}: {len(df)} linhas, {caminho.stat().st_size / 1e6:.1f} MB", flush=True)


def main() -> None:
    entrada, bruto, saida = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    t = time.time()
    df = gpd.read_parquet(entrada, columns=["nome", "strahler", "montante_km", "geometry"])
    df = df[df["nome"].fillna("") != ""].reset_index(drop=True)
    print(f"{len(df)} trechos com nome ({time.time() - t:.0f} s)", flush=True)
    geoms = np.asarray(df.geometry.array)
    ordem = df["strahler"].to_numpy()
    montante = df["montante_km"].to_numpy()
    nomes, partes, ordens, montantes = [], [], [], []
    for nome, idx in df.groupby("nome").indices.items():
        g = geoms[idx]
        unidas = shapely.get_parts(shapely.line_merge(shapely.union_all(g)))
        if len(unidas) == 1:
            pares = [(0, k) for k in range(len(idx))]
        else:
            a, b = shapely.STRtree(g).query(unidas, predicate="intersects")
            pares = list(zip(a, b))
        mo = np.zeros(len(unidas), dtype=np.int16)
        mm = np.zeros(len(unidas))
        for p, k in pares:
            mo[p] = max(mo[p], ordem[idx[k]])
            mm[p] = max(mm[p], montante[idx[k]])
        nomes += [nome] * len(unidas)
        partes += list(unidas)
        ordens += list(mo)
        montantes += list(mm)
    rios = gpd.GeoDataFrame({"nome": nomes, "strahler": ordens, "montante_km": np.round(montantes, 2)}, geometry=partes, crs=df.crs)
    print(f"{len(rios)} linhas de rio ({time.time() - t:.0f} s)", flush=True)
    # O comprimento de cada linha: nas zonas de canais trançados (Amazônia, Pantanal) todo canal tem
    # a ordem da zona, e um furo curto de ordem 10 não deve ter a preferência de um rio grande.
    rios["comp_km"] = np.round(rios.geometry.to_crs(10857).length.to_numpy() / 1000, 1)
    rios["geometry"] = rios.geometry.to_crs(10857).simplify(TOLERANCIA_M).to_crs(df.crs)
    rios = rios[~rios.geometry.is_empty]
    saida.mkdir(parents=True, exist_ok=True)
    gravar(rios, saida / "rios_nomes.parquet")
    gravar(rios[rios["strahler"] >= 6], saida / "rios_nomes_ordem6.parquet")
    with zipfile.ZipFile(bruto) as z:
        gpkg = next(n for n in z.namelist() if n.endswith(".gpkg"))
    ufs = pyogrio.read_dataframe(f"/vsizip/{bruto.as_posix()}/{gpkg}", layer="lml_unidade_federacao_a")
    sigla = next(c for c in ("sigla", "sigla_uf", "sigl_uf") if c in ufs.columns)
    ufs = ufs.to_crs(df.crs)
    (saida / "uf").mkdir(parents=True, exist_ok=True)
    for _, u in ufs.iterrows():
        parte = gpd.clip(rios, u.geometry)
        parte = parte[parte.geometry.geom_type.isin(["LineString", "MultiLineString"])]
        gravar(parte, saida / "uf" / f"rios_nomes_{u[sigla]}.parquet")
    print(f"pronto ({time.time() - t:.0f} s)", flush=True)


if __name__ == "__main__":
    main()
