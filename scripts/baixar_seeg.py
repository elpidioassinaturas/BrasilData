"""Baixa o gases.csv de cada UF da pasta pública "Dados por município — SEEG 14.0" (Google Drive).

Uso: python baixar_seeg.py seeg/
"""

import re
import sys
import time
import urllib.request
from pathlib import Path

RAIZ = "1Au51aKx2jjiVQ1dSKFl3CaUW4tAhtxxh"
AGENTE = {"User-Agent": "Mozilla/5.0"}


def ler(url: str) -> str:
    with urllib.request.urlopen(urllib.request.Request(url, headers=AGENTE), timeout=120) as r:
        return r.read().decode("utf8", "replace")


def itens(pasta: str) -> list[tuple[str, str, str]]:
    h = ler(f"https://drive.google.com/embeddedfolderview?id={pasta}")
    pastas = [("pasta", i, n) for i, n in re.findall(r'href="https://drive.google.com/drive/folders/([\w-]+)"[^>]*>.*?<div class="flip-entry-title">([^<]+)</div>', h, re.S)]
    arquivos = [("arquivo", i, n) for i, n in re.findall(r'href="https://drive.google.com/file/d/([\w-]+)/[^"]*"[^>]*>.*?<div class="flip-entry-title">([^<]+)</div>', h, re.S)]
    return pastas + arquivos


def baixar(arquivo_id: str, destino: Path) -> None:
    url = f"https://drive.usercontent.google.com/download?id={arquivo_id}&export=download&confirm=t"
    parcial = destino.with_suffix(".parcial")
    for tentativa in range(4):
        try:
            with urllib.request.urlopen(urllib.request.Request(url, headers=AGENTE), timeout=600) as r, open(parcial, "wb") as f:
                while bloco := r.read(1 << 20):
                    f.write(bloco)
            parcial.replace(destino)
            return
        except Exception as e:
            print(f"  tentativa {tentativa + 1}: {e}", flush=True)
            time.sleep(10 * (tentativa + 1))
    raise RuntimeError(f"não baixou {destino.name}")


def main() -> None:
    saida = Path(sys.argv[1])
    saida.mkdir(parents=True, exist_ok=True)
    t = time.time()
    for tipo, pasta, uf in itens(RAIZ):
        if tipo != "pasta" or uf == "NA":
            continue
        destino = saida / f"gases_{uf}.csv"
        if destino.exists():
            continue
        alvo = [i for k, i, n in itens(pasta) if k == "arquivo" and n == "gases.csv"]
        if not alvo:
            print(f"{uf}: sem gases.csv", flush=True)
            continue
        baixar(alvo[0], destino)
        print(f"{uf}: {destino.stat().st_size / 1e6:.0f} MB ({time.time() - t:.0f} s)", flush=True)
    print("pronto", flush=True)


if __name__ == "__main__":
    main()
