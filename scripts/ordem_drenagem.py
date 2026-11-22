"""Ordem de Strahler e comprimento de drenagem a montante de cada trecho do BC250 (IBGE).

O sentido em que os trechos foram digitalizados não serve: no oeste da Amazônia até 80% das
confluências estão digitalizadas de jusante para montante. Por isso a rede é orientada pela
topologia:

1. A rede é um grafo sem sentido, com um nó por ponta de trecho (coordenadas a 1e-7 grau). Um
   afluente que termina na margem de um rio de margem dupla, de uma represa ou de outra massa
   d'água com trecho de drenagem dentro (`possuitrechodrenagem` = Sim), a até 30 m dela, é ligado
   ao nó mais próximo da linha central dentro dela (desligado: na rede de 2025 não liga nenhum).
2. Em cada rede conectada, a foz é a ponta solta (nó de grau 1) a até 2 km do mar (foz marítima,
   oceano, baía, enseada ou laguna do BC250); sem nenhuma, a ponta solta a até 2 km da borda do
   país (onde o Paraguai, o Paraná e o Uruguai saem); entre as candidatas, e sem nenhuma, a ponta
   com que o sentido digitalizado mais concorda (contado para todas de uma vez, re-enraizando uma
   árvore geradora).
3. Os trechos correm para a foz: os da árvore, do filho para o pai; os outros (braços, ilhas),
   do nó mais longe da foz para o mais perto.
4. De montante para jusante:
   - strahler: 1 nas nascentes; numa confluência, a maior ordem que chega, mais 1 quando dois ou
     mais trazem essa ordem. Uma zona de canais trançados (trechos em ciclo: ilhas, furos,
     paranás) conta como um trecho só: todos os canais dela têm a ordem que sai dela.
   - montante_km: o comprimento de toda a drenagem a montante do fim do trecho, ele incluído. Num
     nó de onde saem vários trechos (braços), o que chega é dividido entre eles.
   - comp_km: o comprimento do trecho (Albers do IBGE, EPSG:10857).

Grava um GeoParquet com coluna de retângulo (bbox), em grupos de linhas ordenados no espaço.

Uso: uv run --python 3.12 --with geopandas --with pyogrio --with pyarrow \
         python scripts/ordem_drenagem.py bruto/bc250_....zip saida/
"""

from __future__ import annotations

import sys
import time
import zipfile
from collections import deque
from pathlib import Path

import numpy as np
import pandas as pd
import pyogrio
import shapely

CAMADA = "hid_trecho_drenagem_l"
MAR = ("Oceano", "Baía", "Enseada", "Laguna")
MARGEM_GRAUS = 30 / 111_000
COSTA_GRAUS = 2_000 / 111_000


def achar_gpkg(entrada: Path) -> str:
    if entrada.suffix.lower() == ".gpkg":
        return str(entrada)
    with zipfile.ZipFile(entrada) as z:
        nomes = [n for n in z.namelist() if n.lower().endswith(".gpkg")]
    if not nomes:
        sys.exit(f"nenhum .gpkg em {entrada}")
    return f"/vsizip/{entrada.as_posix()}/{nomes[0]}"


def pontas(geoms: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Os pontos inicial e final de cada linha (de uma multilinha, o início da primeira parte e o
    fim da última)."""
    simples = shapely.get_type_id(geoms) == 1
    ini = np.empty((len(geoms), 2))
    fim = np.empty((len(geoms), 2))
    multi = ~simples
    if multi.any():
        primeira = shapely.get_geometry(geoms[multi], 0)
        ultima = shapely.get_geometry(geoms[multi], -1)
        ini[multi] = shapely.get_coordinates(shapely.get_point(primeira, 0))
        fim[multi] = shapely.get_coordinates(shapely.get_point(ultima, -1))
    ini[simples] = shapely.get_coordinates(shapely.get_point(geoms[simples], 0))
    fim[simples] = shapely.get_coordinates(shapely.get_point(geoms[simples], -1))
    return ini, fim


def nos(ini: np.ndarray, fim: np.ndarray, casas: int = 7) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Um número por ponto (coordenadas arredondadas a `casas` decimais de grau, ~1 cm), e as
    coordenadas de cada nó."""
    todos = np.round(np.vstack([ini, fim]), casas)
    unicos, ids = np.unique(todos, axis=0, return_inverse=True)
    ids = ids.ravel()
    n = len(ini)
    return ids[:n], ids[n:], unicos


def ligacoes_nas_margens(u, v, xy, agua) -> list[tuple[int, int]]:
    """Pares (ponta solta, nó da linha central) para os afluentes que param na margem de uma massa
    d'água com trecho de drenagem dentro."""
    n = len(xy)
    grau = np.bincount(u, minlength=n) + np.bincount(v, minlength=n)
    com_trecho = agua[agua["possuitrechodrenagem"] == "Sim"].reset_index(drop=True)
    polis = np.asarray(com_trecho.geometry.array)
    arvore = shapely.STRtree(polis)
    pontos = shapely.points(xy)
    # nós dentro de cada polígono
    dentro_p, dentro_n = arvore.query(pontos, predicate="within")
    poli_do_no = np.full(n, -1)
    poli_do_no[dentro_n] = dentro_p
    soltos = np.flatnonzero(grau == 1)
    sp, sn = arvore.query(pontos[soltos], predicate="dwithin", distance=MARGEM_GRAUS)
    alvo_poli = np.full(len(soltos), -1)
    alvo_poli[sp] = sn
    ligacoes = []
    internos = np.flatnonzero(poli_do_no >= 0)
    if len(internos) == 0:
        return ligacoes
    arvore_nos = shapely.STRtree(pontos[internos])
    candidatos = np.flatnonzero(alvo_poli >= 0)
    pares, _ = arvore_nos.query_nearest(pontos[soltos[candidatos]], return_distance=True, all_matches=False)
    for a, b in pares.T:
        solto = soltos[candidatos[a]]
        no = internos[b]
        if no != solto and poli_do_no[no] == alvo_poli[candidatos[a]] and poli_do_no[solto] != poli_do_no[no]:
            ligacoes.append((int(solto), int(no)))
    return ligacoes


def orientar(u, v, n, costeiro, fronteira) -> tuple[np.ndarray, np.ndarray, int]:
    """Para cada aresta (u[i], v[i]), se ela corre de u para v (True) ou de v para u, se ela é da
    árvore geradora (as outras fecham ciclos: braços, ilhas, canais de delta), e quantas redes
    conectadas há."""
    m = len(u)
    # lista de adjacência em CSR: para cada nó, (vizinho, aresta)
    ends = np.concatenate([u, v])
    outros = np.concatenate([v, u])
    arestas = np.concatenate([np.arange(m), np.arange(m)])
    ordem = np.argsort(ends, kind="stable")
    viz = outros[ordem]
    are = arestas[ordem]
    inicio = np.zeros(n + 1, dtype=np.int64)
    np.add.at(inicio, ends + 1, 1)
    inicio = np.cumsum(inicio)
    grau = np.diff(inicio)

    def bfs(raiz, visto, pai, pai_aresta, prof, seq):
        visto[raiz] = True
        prof[raiz] = 0
        fila = deque([raiz])
        while fila:
            x = fila.popleft()
            seq.append(x)
            for k in range(inicio[x], inicio[x + 1]):
                y = viz[k]
                if not visto[y]:
                    visto[y] = True
                    pai[y] = x
                    pai_aresta[y] = are[k]
                    prof[y] = prof[x] + 1
                    fila.append(y)

    visto = np.zeros(n, bool)
    pai = np.full(n, -1)
    pai_aresta = np.full(n, -1)
    prof = np.zeros(n, dtype=np.int64)
    componentes = []
    for r in range(n):
        if not visto[r]:
            seq = []
            bfs(r, visto, pai, pai_aresta, prof, seq)
            componentes.append(seq)
    # Re-enraizamento: placar(x) = quantas arestas da árvore concordam com o sentido digitalizado
    # quando a raiz é x, a menos de uma constante por rede.
    placar = np.zeros(n)
    raizes = np.zeros(n, bool)
    for seq in componentes:
        r = seq[0]
        for x in seq[1:]:
            p, e = pai[x], pai_aresta[x]
            # com a raiz em p, a aresta corre x -> p; com a raiz em x, corre p -> x
            concorda_para_p = 1.0 if (u[e] == x and v[e] == p) else 0.0
            concorda_para_x = 1.0 if (u[e] == p and v[e] == x) else 0.0
            placar[x] = placar[p] - concorda_para_p + concorda_para_x
        nos_c = np.asarray(seq)
        folhas = nos_c[grau[nos_c] == 1]
        candidatos = folhas if len(folhas) else nos_c
        nota = placar[candidatos] + np.where(costeiro[candidatos], 1e9, 0.0) + np.where(fronteira[candidatos], 1e7, 0.0)
        raizes[candidatos[np.argmax(nota)]] = True
    # Nova busca a partir das fozes escolhidas.
    visto[:] = False
    pai[:] = -1
    pai_aresta[:] = -1
    for r in np.flatnonzero(raizes):
        bfs(int(r), visto, pai, pai_aresta, prof, [])
    de_u_para_v = np.ones(m, bool)
    na_arvore = np.zeros(m, bool)
    filhos = np.flatnonzero(pai >= 0)
    e = pai_aresta[filhos]
    na_arvore[e] = True
    de_u_para_v[e] = u[e] == filhos  # corre do filho para o pai
    fora = ~na_arvore
    pu, pv = prof[u[fora]], prof[v[fora]]
    de_u_para_v[fora] = np.where(pu == pv, True, pu > pv)
    return de_u_para_v, na_arvore, len(componentes)


def calcular(u: np.ndarray, v: np.ndarray, comp: np.ndarray, nome: np.ndarray, arvore: np.ndarray):
    """Strahler e montante numa rede já orientada (cada trecho corre de u para v). A ordem sobe só
    com os trechos da árvore (`arvore`): um braço que fecha um ciclo traz de volta a mesma água, e
    contá-lo faria a ordem crescer a cada ilha (no delta do Amazonas, até 27)."""
    n = len(u)
    nn = int(max(u.max(), v.max())) + 1
    ordem_v = np.argsort(v, kind="stable")
    ini_v = np.searchsorted(v[ordem_v], np.arange(nn + 1))
    n_saem = np.bincount(u, minlength=nn)
    ordem_u = np.argsort(u, kind="stable")
    ini_u = np.searchsorted(u[ordem_u], np.arange(nn + 1))
    falta = (ini_v[u + 1] - ini_v[u]).astype(np.int64)
    fila = deque(np.flatnonzero(falta == 0).tolist())
    ordem = np.zeros(n, dtype=np.int16)
    montante = np.zeros(n)
    feito = np.zeros(n, dtype=bool)
    vistos = 0
    while True:
        while fila:
            i = fila.popleft()
            x = u[i]
            de_cima = [j for j in ordem_v[ini_v[x] : ini_v[x + 1]] if feito[j]]
            if not de_cima:
                ordem[i] = 1
                acima = 0.0
            else:
                contam = [j for j in de_cima if arvore[j]] or de_cima
                mx = max(ordem[j] for j in de_cima)
                maiores = [j for j in contam if ordem[j] == mx]
                nomes = {nome[j] for j in maiores}
                mesmo_rio = len(nomes) == 1 and next(iter(nomes)) != ""
                ordem[i] = mx + 1 if len(maiores) >= 2 and not mesmo_rio else mx
                acima = sum(montante[j] for j in de_cima) / max(1, n_saem[x])
            montante[i] = acima + comp[i]
            feito[i] = True
            vistos += 1
            for k in ordem_u[ini_u[v[i]] : ini_u[v[i] + 1]]:
                falta[k] -= 1
                if falta[k] == 0:
                    fila.append(k)
        if vistos == n:
            break
        pendentes = np.flatnonzero(~feito & (falta > 0))
        if len(pendentes) == 0:
            break
        k = int(pendentes[np.argmin(falta[pendentes])])
        falta[k] = 0
        fila.append(k)
    return ordem, montante


def pontes(u: np.ndarray, v: np.ndarray, n: int) -> np.ndarray:
    """Se cada aresta é uma ponte do grafo sem sentido (tirá-la separa a rede). As que não são
    pontes estão em ciclos: canais trançados, ilhas, furos e paranás. Tarjan iterativo."""
    m = len(u)
    ends = np.concatenate([u, v])
    outros = np.concatenate([v, u])
    arestas = np.concatenate([np.arange(m), np.arange(m)])
    ordem = np.argsort(ends, kind="stable")
    viz, are = outros[ordem], arestas[ordem]
    inicio = np.searchsorted(ends[ordem], np.arange(n + 1))
    disc = np.full(n, -1, dtype=np.int64)
    low = np.zeros(n, dtype=np.int64)
    ponte = np.zeros(m, dtype=bool)
    t = 0
    for r in range(n):
        if disc[r] >= 0:
            continue
        disc[r] = low[r] = t
        t += 1
        pilha = [(r, -1, inicio[r])]
        while pilha:
            x, e_pai, k = pilha[-1]
            if k < inicio[x + 1]:
                pilha[-1] = (x, e_pai, k + 1)
                y, e = viz[k], are[k]
                if e == e_pai:
                    continue
                if disc[y] < 0:
                    disc[y] = low[y] = t
                    t += 1
                    pilha.append((y, e, inicio[y]))
                elif disc[y] < low[x]:
                    low[x] = disc[y]
            else:
                pilha.pop()
                if pilha:
                    p = pilha[-1][0]
                    if low[x] < low[p]:
                        low[p] = low[x]
                    if low[x] > disc[p]:
                        ponte[e_pai] = True
    return ponte


def strahler_por_zonas(de: np.ndarray, para: np.ndarray, n: int) -> np.ndarray:
    """A ordem de Strahler com cada zona de canais trançados (as arestas que não são pontes, e os
    nós que elas ligam) tratada como um trecho só: a ordem que sai dela combina a dos trechos que
    entram, e todos os canais dentro dela a recebem. Sem isso, cada ilha soma 1 e a ordem do
    Amazonas chega a 25."""
    m = len(de)
    ponte = pontes(de, para, n)
    # zona de cada nó: componentes ligados pelas arestas que não são pontes (union-find)
    pai = np.arange(n)

    def raiz(x):
        while pai[x] != x:
            pai[x] = pai[pai[x]]
            x = pai[x]
        return x

    for e in np.flatnonzero(~ponte):
        a, b = raiz(de[e]), raiz(para[e])
        if a != b:
            pai[a] = b
    zona = np.array([raiz(x) for x in range(n)])
    zde, zpara = zona[de], zona[para]
    pontes_idx = np.flatnonzero(ponte)
    entram = {}
    saem = {}
    for e in pontes_idx:
        entram.setdefault(zpara[e], []).append(e)
        saem.setdefault(zde[e], []).append(e)
    zonas = np.unique(zona)
    falta = {z: len(entram.get(z, ())) for z in zonas}
    fila = deque(z for z in zonas if falta[z] == 0)
    saida_da_zona = {}
    ordem = np.zeros(m, dtype=np.int16)
    while fila:
        z = fila.popleft()
        chegam = [ordem[e] for e in entram.get(z, ())]
        if not chegam:
            o = 1
        else:
            mx = max(chegam)
            o = mx + 1 if chegam.count(mx) >= 2 else mx
        saida_da_zona[z] = o
        for e in saem.get(z, ()):
            ordem[e] = o
            w = zpara[e]
            falta[w] -= 1
            if falta[w] == 0:
                fila.append(w)
    dentro = np.flatnonzero(~ponte)
    ordem[dentro] = [saida_da_zona.get(z, 1) for z in zde[dentro]]
    # pontes nunca alcançadas (ciclo na orientação): a ordem da zona de onde saem
    sem = (ordem == 0)
    ordem[sem] = [saida_da_zona.get(z, 1) for z in zde[sem]]
    return ordem


def main() -> None:
    entrada, saida = Path(sys.argv[1]), Path(sys.argv[2])
    saida.mkdir(parents=True, exist_ok=True)
    t = time.time()

    def dizer(s: str) -> None:
        print(f"{s} ({time.time() - t:.0f} s)", flush=True)

    gpkg = achar_gpkg(entrada)
    df = pyogrio.read_dataframe(gpkg, layer=CAMADA)
    dizer(f"{len(df)} trechos, {df.crs}")
    if df.crs is None or df.crs.to_epsg() != 4674:
        df = df.to_crs(4674)
    agua = pyogrio.read_dataframe(gpkg, layer="hid_massa_dagua_a", columns=["tipomassadagua", "possuitrechodrenagem"]).to_crs(4674)
    foz = [pyogrio.read_dataframe(gpkg, layer=f"hid_foz_maritima_{s}", columns=[]).to_crs(4674) for s in "apl"]
    geoms = np.asarray(df.geometry.array)
    ini, fim = pontas(geoms)
    u, v, xy = nos(ini, fim)
    n_nos = len(xy)
    comp = df.geometry.to_crs(10857).length.to_numpy() / 1000.0
    nome = df["nome"].fillna("").to_numpy() if "nome" in df.columns else np.array([""] * len(df))
    dizer(f"{n_nos} nós")
    # A ligação dos afluentes à linha central das massas d'água (ligacoes_nas_margens) não ligou
    # nenhum na rede de 2025, que já forma só 1.627 redes conectadas, e levava 9 minutos.
    ligacoes: list[tuple[int, int]] = []
    mar = list(np.asarray(agua[agua["tipomassadagua"].isin(MAR)].geometry.array))
    for f in foz:
        mar += list(np.asarray(f.geometry.array))
    arvore_mar = shapely.STRtree(np.asarray(mar))
    pontos = shapely.points(xy)
    perto, _ = arvore_mar.query(pontos, predicate="dwithin", distance=COSTA_GRAUS)
    costeiro = np.zeros(n_nos, bool)
    costeiro[perto] = True
    dizer(f"{costeiro.sum()} nós a até 2 km do mar")
    ufs = pyogrio.read_dataframe(gpkg, layer="lml_unidade_federacao_a", columns=[]).to_crs(4674)
    borda = shapely.boundary(shapely.union_all(np.asarray(ufs.geometry.array)))
    # A borda em segmentos de 2 vértices, num índice: medir contra a geometria inteira levava
    # mais de 10 minutos.
    coords = [shapely.get_coordinates(parte) for parte in shapely.get_parts(shapely.line_merge(borda))]
    segmentos = np.concatenate([shapely.linestrings(np.stack([c[:-1], c[1:]], axis=1)) for c in coords if len(c) > 1])
    grau = np.bincount(u, minlength=n_nos) + np.bincount(v, minlength=n_nos)
    soltos = np.flatnonzero(grau == 1)
    achados, _ = shapely.STRtree(segmentos).query(pontos[soltos], predicate="dwithin", distance=COSTA_GRAUS)
    fronteira = np.zeros(n_nos, bool)
    fronteira[soltos[np.unique(achados)]] = True
    fronteira &= ~costeiro
    dizer(f"{fronteira.sum()} nós a até 2 km da fronteira terrestre (ou de outra borda do país)")
    lu = np.array([a for a, _ in ligacoes], dtype=np.int64)
    lv = np.array([b for _, b in ligacoes], dtype=np.int64)
    uu = np.concatenate([u, lu])
    vv = np.concatenate([v, lv])
    sentido, arvore, redes = orientar(uu, vv, n_nos, costeiro, fronteira)
    invertidos = int((~sentido[: len(u)]).sum())
    dizer(f"{redes} redes conectadas; {invertidos} trechos digitalizados contra a corrente")
    de = np.where(sentido, uu, vv)
    para = np.where(sentido, vv, uu)
    cc = np.concatenate([comp, np.zeros(len(lu))])
    nn_ = np.concatenate([nome, np.array([""] * len(lu), dtype=object)])
    _, montante = calcular(de, para, cc, nn_, arvore)
    ordem = strahler_por_zonas(de, para, n_nos)
    ordem, montante = ordem[: len(u)], montante[: len(u)]
    dizer("ordens calculadas")
    df["strahler"] = ordem
    df["montante_km"] = np.round(montante, 2)
    df["comp_km"] = np.round(comp, 3)
    df["contra_corrente"] = ~sentido[: len(u)]
    for o_, c in enumerate(np.bincount(ordem)):
        if c:
            print(f"  ordem {o_}: {c} trechos", flush=True)
    for limiar in (100, 1000, 5000, 20000, 100000):
        print(f"  montante >= {limiar} km: {(montante >= limiar).sum()} trechos", flush=True)
    g = pd.DataFrame({"nome": nome, "o": ordem, "m": montante}).groupby("nome").agg(o=("o", "max"), m=("m", "max"))
    print(g.sort_values("m", ascending=False).head(15).round(0).to_string(), flush=True)
    manter = [c for c in ("nome", "tipotrechodrenagem", "regime", "navegavel") if c in df.columns]
    df = df[manter + ["strahler", "montante_km", "comp_km", "contra_corrente", "geometry"]]
    df = df.iloc[df.geometry.hilbert_distance().argsort()]
    tudo = saida / "bc250_2025_drenagem_ordem.parquet"
    df.to_parquet(tudo, write_covering_bbox=True, row_group_size=20_000, compression="zstd")
    dizer(f"gravado {tudo} ({tudo.stat().st_size / 1e6:.0f} MB)")


if __name__ == "__main__":
    main()
