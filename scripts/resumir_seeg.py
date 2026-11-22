"""Resume os dados por município do SEEG (Observatório do Clima) em tabelas por gás e ano.

Entrada: os `gases.csv` de cada UF da pasta "Dados por município" do SEEG (baixados como
`gases_<UF>.csv`): uma linha por setor, categoria, subcategoria, produto, detalhamento, recorte,
atividade, bioma, tipo (Emissão, Remoção ou Bunker), gás e município, com uma coluna por ano.

Saída: `seeg_<gás>_<ano>.csv`, uma linha por município (código do IBGE), em toneladas do gás:
- `total`: as emissões (sem as remoções nem o bunker, o combustível de navios e aviões
  internacionais, que o inventário conta à parte);
- uma coluna por setor (`agropecuaria`, `energia`, `mudanca_uso_terra`, `residuos`,
  `processos_industriais`);
- `remocoes`: as remoções (negativas; só de CO2, pela floresta em pé e pela regeneração);
- `liquido`: `total` + `remocoes`.

Uso: python scripts/resumir_seeg.py seeg/ saida_seeg/
     python scripts/resumir_seeg.py - saida_seeg/ 2025 seeg.csv   (o retrato que vai no plugin)
"""

from __future__ import annotations

import csv
import sys
import time
from collections import defaultdict
from pathlib import Path

SETORES = {
    "Agropecuária": "agropecuaria",
    "Energia": "energia",
    "Mudança de Uso da Terra e Floresta": "mudanca_uso_terra",
    "Resíduos": "residuos",
    "Processos Industriais": "processos_industriais",
}
COLUNAS = ["total", *SETORES.values(), "remocoes", "liquido"]
PRIMEIRO_ANO = 1990


def gas_curto(g: str) -> str:
    """'CH4 (t)' -> 'ch4'; 'CO2e (t) GWP-AR5' -> 'co2e_gwp_ar5'."""
    base = g.split("(")[0].strip().lower()
    return "".join(c if c.isalnum() else "_" for c in base).strip("_")


def main() -> None:
    entrada, saida = Path(sys.argv[1]), Path(sys.argv[2])
    saida.mkdir(parents=True, exist_ok=True)
    t = time.time()
    # (gás, ano) -> código -> coluna -> t
    somas: dict[tuple[str, int], dict[str, dict[str, float]]] = defaultdict(lambda: defaultdict(lambda: defaultdict(float)))
    nomes: dict[str, str] = {}
    gases_vistos: dict[str, int] = defaultdict(int)
    for arq in sorted(entrada.glob("gases_*.csv")):
        with open(arq, encoding="utf-8-sig", newline="") as f:
            r = csv.reader(f)
            cab = next(r)
            i_setor, i_tipo, i_gas = cab.index("Setor de emissão"), cab.index("Emissão/Remoção/Bunker"), cab.index("Gás")
            i_cidade, i_ibge = cab.index("Cidade"), cab.index("IBGE")
            anos = [(i, int(c)) for i, c in enumerate(cab) if c.isdigit() and int(c) >= PRIMEIRO_ANO]
            for linha in r:
                tipo = linha[i_tipo]
                if tipo == "Bunker":
                    continue
                gas = gas_curto(linha[i_gas])
                gases_vistos[linha[i_gas]] += 1
                codigo = linha[i_ibge].strip()
                if not codigo.isdigit():
                    continue
                nomes[codigo] = linha[i_cidade]
                setor = SETORES.get(linha[i_setor])
                for i, ano in anos:
                    v = linha[i]
                    if not v or v == "0.0":
                        continue
                    x = float(v)
                    s = somas[(gas, ano)][codigo]
                    if tipo == "Remoção" or x < 0:
                        s["remocoes"] += x
                    else:
                        s["total"] += x
                        if setor:
                            s[setor] += x
        print(f"{arq.name} ({time.time() - t:.0f} s)", flush=True)
    print("gases:", dict(gases_vistos), flush=True)
    for (gas, ano), por_mun in sorted(somas.items()):
        with open(saida / f"seeg_{gas}_{ano}.csv", "w", encoding="utf-8", newline="") as f:
            w = csv.writer(f)
            w.writerow(["cd_mun", "municipio", *COLUNAS])
            for codigo in sorted(nomes):
                s = por_mun.get(codigo, {})
                s_liq = s.get("total", 0.0) + s.get("remocoes", 0.0)
                w.writerow([codigo, nomes[codigo], *[round(s.get(c, 0.0), 3) for c in COLUNAS[:-1]], round(s_liq, 3)])
    print(f"pronto: {len(somas)} tabelas, {len(nomes)} municípios ({time.time() - t:.0f} s)", flush=True)




def retrato(saida: Path, ano: int, destino: Path) -> None:
    """O retrato que vai dentro do plugin (`seeg.csv`): por município, as toneladas de `ano` de CH4
    (total e por setor), CO2 (bruto e líquido), N2O e CO2e líquido (AR5, 100 anos: CH4 × 28 e N2O ×
    265, os pesos do inventário brasileiro)."""
    def ler(gas: str) -> dict[str, dict[str, float]]:
        with open(saida / f"seeg_{gas}_{ano}.csv", encoding="utf-8") as f:
            return {r["cd_mun"]: {k: float(v) for k, v in r.items() if k not in ("cd_mun", "municipio")} for r in csv.DictReader(f)}

    ch4, co2, n2o = ler("ch4"), ler("co2"), ler("n2o")
    cols = ["codigo", "ano", "ch4", "ch4_agropecuaria", "ch4_residuos", "ch4_mudanca_uso_terra", "ch4_energia", "co2", "co2_liquido", "n2o", "co2e_liquido"]
    with open(destino, "w", encoding="utf-8", newline="\n") as f:
        f.write(",".join(cols) + "\n")
        for cd in sorted(set(ch4) | set(co2) | set(n2o)):
            # 7 dígitos; "1100000" e afins são as emissões do estado sem município.
            if len(cd) != 7 or cd.endswith("00000"):
                continue
            m, c, n = ch4.get(cd, {}), co2.get(cd, {}), n2o.get(cd, {})
            co2e = c.get("liquido", 0) + 28 * m.get("liquido", 0) + 265 * n.get("liquido", 0)
            v = [m.get("total", 0), m.get("agropecuaria", 0), m.get("residuos", 0), m.get("mudanca_uso_terra", 0), m.get("energia", 0),
                 c.get("total", 0), c.get("liquido", 0), n.get("total", 0), co2e]
            f.write(",".join([cd, str(ano)] + [f"{x:.1f}" for x in v]) + "\n")
    print(f"{destino}: {destino.stat().st_size / 1e3:.0f} kB", flush=True)


if __name__ == "__main__":
    if len(sys.argv) > 3:
        retrato(Path(sys.argv[2]), int(sys.argv[3]), Path(sys.argv[4]))
    else:
        main()
