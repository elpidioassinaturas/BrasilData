"""Lê as cores oficiais dos estilos (SLD) do GeoServer do Serviço Geológico do Brasil e escreve o
sgb_cores.csv: uma linha por regra (a chave da camada, o valor, a cor e se ela é só do contorno),
que o src/sgb.rs embute.

Uso:
    python baixar_sgb_cores.py              # pede os SLD ao GeoServer (GetStyles)
    python baixar_sgb_cores.py --de pasta/  # lê os SLD já baixados: pasta/sld_<tipo sem o prefixo>.xml

Um SLD da litoestratigrafia tem uma regra por sigla (até 2.873, 2 MB), grande demais para o plugin
pedir a cada mapa: as cores vêm com ele, e este script as atualiza.
"""

import csv
import re
import sys
import time
import urllib.parse
import urllib.request
from pathlib import Path

WMS = "https://geoservicos.sgb.gov.br/geoserver/ows"
AGENTE = {"User-Agent": "Mozilla/5.0"}
ROOT = Path(__file__).resolve().parent.parent

# As camadas cujas cores o plugin usa, com o campo que as regras comparam.
# A chave é a do src/sgb.rs.
CAMADAS = [
    ("1m", "geosgb:litoestratigrafia_1m", "sigla"),
    ("250k", "geosgb:litoestratigrafia_250k", "sigla"),
    ("100k", "geosgb:litoestratigrafia_100k", "sigla"),
    ("50k", "geosgb:litoestratigrafia_50k", "sigla"),
    ("25k", "geosgb:litoestratigrafia_25k", "sigla"),
    ("estados", "geosgb:litoestratigrafia_estados", "sigla"),
    ("movimento", "gestao-territorial:suscet_movimento_de_massa", "classe"),
    ("inundacao", "gestao-territorial:suscet_inundacao", "classe"),
    ("risco", "gestao-territorial:risco_geologico", "grau_risco"),
]

REGRA = re.compile(r"<(?:sld:)?Rule>(.*?)</(?:sld:)?Rule>", re.S)
IGUAL = re.compile(
    r"<ogc:PropertyIsEqualTo[^>]*>\s*<ogc:PropertyName>([^<]+)</ogc:PropertyName>\s*<ogc:Literal>([^<]*)</ogc:Literal>\s*</ogc:PropertyIsEqualTo>"
)
PREENCHE = re.compile(r'<(?:sld:)?Fill>.*?<(?:sld:)?CssParameter name="fill">\s*(#[0-9A-Fa-f]{6})\s*<', re.S)
BORDA = re.compile(r'<(?:sld:)?Stroke>.*?<(?:sld:)?CssParameter name="stroke">\s*(#[0-9A-Fa-f]{6})\s*<', re.S)


def sld(camada: str, pasta: Path | None) -> str:
    if pasta is not None:
        return (pasta / f"sld_{camada.split(':', 1)[1]}.xml").read_text(encoding="utf-8")
    q = urllib.parse.urlencode({"service": "WMS", "version": "1.1.1", "request": "GetStyles", "layers": camada})
    for tentativa in range(4):
        try:
            with urllib.request.urlopen(urllib.request.Request(f"{WMS}?{q}", headers=AGENTE), timeout=300) as r:
                return r.read().decode("utf-8")
        except Exception as e:
            print(f"  {camada}, tentativa {tentativa + 1}: {e}", flush=True)
            time.sleep(15 * (tentativa + 1))
    raise SystemExit(f"o GeoServer do SGB não deu o estilo de {camada}")


def regras(texto: str, campo: str) -> list[tuple[str, str, str]]:
    """(valor, cor, "1" se a regra só pinta o contorno) de cada regra que compara `campo` com um
    valor só."""
    saida = []
    for r in REGRA.findall(texto):
        iguais = IGUAL.findall(r)
        if len(iguais) != 1 or iguais[0][0].strip() != campo:
            continue
        p = PREENCHE.search(r)
        b = BORDA.search(r)
        if p:
            saida.append((iguais[0][1], p.group(1).lower(), ""))
        elif b:
            saida.append((iguais[0][1], b.group(1).lower(), "1"))
    return saida


def main() -> None:
    pasta = None
    if len(sys.argv) == 3 and sys.argv[1] == "--de":
        pasta = Path(sys.argv[2])
    elif len(sys.argv) != 1:
        raise SystemExit(__doc__)
    linhas = []
    for chave, camada, campo in CAMADAS:
        rs = regras(sld(camada, pasta), campo)
        vistos = set()
        # O src/sgb.rs lê o CSV sem aspas: um valor com vírgula ou quebra de linha fica de fora, dito.
        fora = [v for v, _, _ in rs if "," in v or "\n" in v]
        for valor, cor, contorno in rs:
            if valor in vistos or valor in fora:
                continue
            vistos.add(valor)
            linhas.append((chave, valor, cor, contorno))
        print(f"{camada}: {len(vistos)} valores" + (f", {len(fora)} com vírgula de fora: {fora[:5]}" if fora else ""), flush=True)
    with open(ROOT / "sgb_cores.csv", "w", encoding="utf-8", newline="") as f:
        w = csv.writer(f, lineterminator="\n")
        w.writerow(["chave", "valor", "cor", "contorno"])
        w.writerows(linhas)
    print(f"sgb_cores.csv: {len(linhas)} linhas")


if __name__ == "__main__":
    main()
