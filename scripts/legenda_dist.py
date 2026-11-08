"""Legenda da situação do OPERA DIST-S1: confiança nas linhas, etapa nas colunas (PNG a 300 dpi e SVG).

Uso: py -3.12 scripts/legenda_dist.py <saída sem extensão> [brasildata|oficial]
As cores são as de `Paleta::cores` em src/dist.rs.
"""
import sys
from PIL import Image, ImageDraw, ImageFont

saida = sys.argv[1]
paleta = sys.argv[2] if len(sys.argv) > 2 else "brasildata"
if paleta == "oficial":
    # Cores oficiais da OPERA (opera-adt/dist-s1, constants.py, DIST_STATUS_CMAP) e o alfa que o plugin usa.
    cor = {1: "#005555", 2: "#897f4e", 3: "#dee043", 4: "#008888", 5: "#e48727", 6: "#e01b07", 7: "#777777", 8: "#dddddd"}
    alfa = {1: 110, 4: 110}
else:
    # Proposta: o matiz diz a etapa (amarelo, laranja, vermelho, roxo) e, em cada coluna, a confiança
    # alta é mais escura que a baixa. Sem transparência.
    cor = {1: "#fbe6a2", 4: "#f6d256", 2: "#f0a06e", 5: "#e8711f", 3: "#ee8a8a", 6: "#b3121d", 7: "#d3c3e6", 8: "#7a55a8"}
    alfa = {}
colunas = [("Primeira", "detecção"), ("Provisória", "(2 passagens)"), ("Confirmada", "(3 ou mais)"), ("Encerrada", "(voltou ao normal)")]
linhas = [(("Alta", "confiança"), [4, 5, 6, 8]), (("Baixa", "confiança"), [1, 2, 3, 7])]
titulo = "Perturbação da vegetação (OPERA DIST-S1)"
notas = (["Sem perturbação: transparente. Primeira detecção translúcida: ainda não confirmada.",
          "Cores: paleta oficial da OPERA (JPL/NASA). Números: códigos do arquivo GEN-DIST-STATUS."]
         if paleta == "oficial" else
         ["Sem perturbação: transparente. Cor: a etapa; mais escura: confiança alta.",
          "Números: códigos do arquivo GEN-DIST-STATUS (OPERA, JPL/NASA)."])

# Medidas em pontos (1/72 pol); o PNG sai a 300 dpi.
W, H = 340, 150
gx, gy, cw, ch, gap = 66, 42, 68, 30, 7  # grade: canto, largura e altura da célula, espaço entre linhas
pad = 6  # margem dentro da coluna
def rgb(h):
    h = h.lstrip("#"); return tuple(int(h[i:i + 2], 16) for i in (0, 2, 4))

# --- SVG
s = [f'<svg xmlns="http://www.w3.org/2000/svg" width="{W}pt" height="{H}pt" viewBox="0 0 {W} {H}" font-family="Arial, Helvetica, sans-serif">',
     f'<rect width="{W}" height="{H}" fill="white"/>',
     f'<text x="7" y="16" font-size="9" font-weight="bold" fill="#111">{titulo}</text>']
for j, (a, b) in enumerate(colunas):
    cx = gx + j * cw + cw / 2
    s.append(f'<text x="{cx}" y="{gy - 13}" font-size="7.5" text-anchor="middle" fill="#111">{a}</text>')
    s.append(f'<text x="{cx}" y="{gy - 4}" font-size="7" text-anchor="middle" fill="#444">{b}</text>')
for i, ((a, b), cods) in enumerate(linhas):
    y = gy + i * (ch + gap)
    s.append(f'<text x="7" y="{y + ch / 2 - 1}" font-size="7.5" fill="#111">{a}</text>')
    s.append(f'<text x="7" y="{y + ch / 2 + 8}" font-size="7.5" fill="#111">{b}</text>')
    for j, k in enumerate(cods):
        x = gx + j * cw + pad
        op = alfa.get(k, 255) / 255
        s.append(f'<rect x="{x}" y="{y}" width="{cw - 2 * pad}" height="{ch}" fill="{cor[k]}" fill-opacity="{op:.3f}" stroke="#555" stroke-width="0.4"/>')
        claro = (k in (3, 8) or k in alfa) if paleta == "oficial" else k in (1, 4, 2, 3, 7)
        s.append(f'<text x="{x + (cw - 2 * pad) / 2}" y="{y + ch / 2 + 2.7}" font-size="7.5" text-anchor="middle" fill="{"#222" if claro else "white"}">{k}</text>')
for n, t in enumerate(notas):
    s.append(f'<text x="7" y="{gy + 2 * ch + gap + 18 + n * 10}" font-size="6.5" fill="#333">{t}</text>')
s.append("</svg>")
open(saida + ".svg", "w", encoding="utf-8").write("\n".join(s))

# --- PNG (mesmo desenho)
k_ = 300 / 72
im = Image.new("RGB", (round(W * k_), round(H * k_)), "white")
dr = ImageDraw.Draw(im, "RGBA")
def f(pt, negrito=False):
    return ImageFont.truetype("C:/Windows/Fonts/arialbd.ttf" if negrito else "C:/Windows/Fonts/arial.ttf", round(pt * k_))
def txt(x, y, t, pt, c="#111", meio=False, negrito=False):
    fo = f(pt, negrito)
    # y é a linha de base, como no SVG
    dr.text((x * k_, y * k_), t, font=fo, fill=c, anchor="ms" if meio else "ls")
txt(7, 16, titulo, 9, negrito=True)
for j, (a, b) in enumerate(colunas):
    cx = gx + j * cw + cw / 2
    txt(cx, gy - 13, a, 7.5, meio=True)
    txt(cx, gy - 4, b, 7, "#444", meio=True)
for i, ((a, b), cods) in enumerate(linhas):
    y = gy + i * (ch + gap)
    txt(7, y + ch / 2 - 1, a, 7.5)
    txt(7, y + ch / 2 + 8, b, 7.5)
    for j, k in enumerate(cods):
        x = gx + j * cw + pad
        caixa = [x * k_, y * k_, (x + cw - 2 * pad) * k_, (y + ch) * k_]
        dr.rectangle(caixa, fill=rgb(cor[k]) + (alfa.get(k, 255),), outline=(85, 85, 85, 255), width=2)
        claro = (k in (3, 8) or k in alfa) if paleta == "oficial" else k in (1, 4, 2, 3, 7)
        txt(x + (cw - 2 * pad) / 2, y + ch / 2 + 2.7, str(k), 7.5, "#222" if claro else "white", meio=True)
for n, t in enumerate(notas):
    txt(7, gy + 2 * ch + gap + 18 + n * 10, t, 6.5, "#333")
im.save(saida + ".png", dpi=(300, 300))
print("ok")
