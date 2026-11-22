"""Divide a drenagem do BC250 com ordem (saída de scripts/ordem_drenagem.py) por UF e em níveis nacionais.

- `uf/drenagem_<UF>.parquet`: os trechos cujo ponto do meio cai na UF (cada trecho num arquivo
  só), para o plugin baixar as UFs da área, juntar e recortar.
- `brasil/drenagem_montante_<N>km.parquet`: os trechos com pelo menos N km de drenagem a montante,
  no país inteiro, lidos pela vista nas escalas pequenas (o RustGIS lê até 60 mil feições por
  vista; cada nível fica abaixo disso).

Uso: uv run --python 3.12 --with geopandas --with pyogrio --with pyarrow \
         python scripts/dividir_drenagem_por_uf.py saida/bc250_2025_drenagem_ordem.parquet bruto/bc250_....zip saida/
"""

from __future__ import annotations

import sys
import time
import zipfile
from pathlib import Path

import geopandas as gpd
import pyogrio

NIVEIS_KM = (5_000, 1_000, 100)


def gravar(df: gpd.GeoDataFrame, caminho: Path) -> None:
    df = df.iloc[df.geometry.hilbert_distance().argsort()]
    df.to_parquet(caminho, write_covering_bbox=True, row_group_size=10_000, compression="zstd")
    print(f"  {caminho.name}: {len(df)} trechos, {caminho.stat().st_size / 1e6:.1f} MB", flush=True)


def main() -> None:
    ordem, bruto, saida = Path(sys.argv[1]), Path(sys.argv[2]), Path(sys.argv[3])
    t = time.time()
    df = gpd.read_parquet(ordem)
    print(f"{len(df)} trechos ({time.time() - t:.0f} s)", flush=True)
    with zipfile.ZipFile(bruto) as z:
        gpkg = next(n for n in z.namelist() if n.endswith(".gpkg"))
    ufs = pyogrio.read_dataframe(f"/vsizip/{bruto.as_posix()}/{gpkg}", layer="lml_unidade_federacao_a")
    sigla = next(c for c in ("sigla", "sigla_uf", "sigl_uf") if c in ufs.columns)
    ufs = ufs[[sigla, "geometry"]].rename(columns={sigla: "uf"}).to_crs(df.crs)
    meio = gpd.GeoDataFrame(geometry=df.geometry.interpolate(0.5, normalized=True), crs=df.crs)
    junto = gpd.sjoin(meio, ufs, how="left", predicate="within")
    junto = junto[~junto.index.duplicated()]
    df["uf"] = junto["uf"].reindex(df.index)
    # Pontos do meio fora de toda UF (no mar, na fronteira): a UF mais próxima.
    sem = df["uf"].isna()
    if sem.any():
        perto = gpd.sjoin_nearest(meio[sem], ufs, how="left")
        perto = perto[~perto.index.duplicated()]
        df.loc[sem, "uf"] = perto["uf"]
    print(f"UFs atribuídas ({time.time() - t:.0f} s); sem UF: {int(df['uf'].isna().sum())}", flush=True)
    (saida / "uf").mkdir(parents=True, exist_ok=True)
    (saida / "brasil").mkdir(parents=True, exist_ok=True)
    for uf, parte in df.groupby("uf"):
        gravar(parte.drop(columns="uf"), saida / "uf" / f"drenagem_{uf}.parquet")
    for km in NIVEIS_KM:
        gravar(df[df["montante_km"] >= km].drop(columns="uf"), saida / "brasil" / f"drenagem_montante_{km}km.parquet")
    print(f"pronto ({time.time() - t:.0f} s)", flush=True)


if __name__ == "__main__":
    main()
