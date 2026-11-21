"""Gera a página do BrasilData (GitHub Pages) a partir dos mesmos arquivos do plugin.

    python scripts/site.py                 # escreve _site/
    python scripts/site.py --saida pasta   # escreve noutra pasta

Nada da página é escrito à mão, para ela não envelhecer:
- os textos de apresentação, de instalação, de como pedir uma fonte nova, da autoria e da
  licença vêm do README.md (o que está antes de "## O que traz" e as seções Instalar, Autoria e
  Licença);
- os tópicos, subtópicos e botões, na ordem da aba, e as janelas dos comandos vêm do plugin.json;
- as fichas vêm do fontes.csv e do fichas.csv, lidos e conferidos pelo scripts/gerar.py;
- os mapas exemplo são os docs/camadas/<id>.png.

O estilo e a busca ficam em site/estilo.css e site/busca.js (HTML e CSS estáticos, sem
dependências; a busca roda no navegador). O workflow Pages (.github/workflows/pages.yml) roda
este script e publica a pasta.
"""

import hashlib
import html
import json
import pathlib
import re
import shutil
import struct
import sys
import unicodedata

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))
import gerar  # noqa: E402

ROOT = gerar.ROOT
REPO = gerar.REPOSITORIO
RELEASES = f"{REPO}/releases"
ULTIMA = f"{RELEASES}/latest"
ISSUE_NOVA = f"{REPO}/issues/new"

# A única frase que não vem do README nem do plugin: para quem é o plugin.
PARA_QUEM = (
    "Para quem precisa de dados públicos do Brasil sobre um lugar e não quer baixar o país inteiro "
    "para recortar depois: estudantes, professores e pesquisadores, técnicos de prefeituras, de "
    "órgãos ambientais e de extensão rural, consultores e quem faz mapas para relatórios, teses e "
    "artigos. Cada camada chega com a fonte, a licença e uma ficha com os cuidados de uso."
)

WEB_CLASSES = {
    "sim": "funciona no app web",
    "nao": "só no app desktop",
    "so-web": "só no app web",
    "depende": "app web: depende",
}


# ---------------------------------------------------------------- Markdown (o pouco do README)


def esc(t: str) -> str:
    return html.escape(t, quote=True)


def endereco(url: str) -> str:
    """Um link relativo do README aponta para o arquivo no GitHub."""
    if re.match(r"^[a-z]+:", url) or url.startswith("#"):
        return url
    return f"{REPO}/blob/main/{url}"


def inline(t: str) -> str:
    """Código, links, negrito, itálico e referências owner/repo#n de uma linha de Markdown."""
    # Código e links viram marcadores antes do resto, e o texto de um link pode ter código.
    guardados: list[str] = []

    def guarda(h: str) -> str:
        guardados.append(h)
        return f"\x01{len(guardados) - 1}\x01"

    def devolve(t: str) -> str:
        return re.sub(r"\x01(\d+)\x01", lambda m: guardados[int(m.group(1))], t)

    t = re.sub(r"`([^`]+)`", lambda m: guarda(f"<code>{esc(m.group(1))}</code>"), t)
    t = re.sub(
        r"\[([^\]]+)\]\(([^)\s]+)\)",
        lambda m: guarda(f'<a href="{esc(endereco(m.group(2)))}">{devolve(esc(m.group(1)))}</a>'),
        t,
    )
    t = esc(t)
    t = re.sub(
        r"\b([\w.-]+/[\w.-]+)#(\d+)\b",
        lambda m: guarda(f'<a href="https://github.com/{m.group(1)}/issues/{m.group(2)}">{m.group(0)}</a>'),
        t,
    )
    t = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", t)
    t = re.sub(r"(?<![\w*])\*(?!\s)(.+?)(?<!\s)\*(?![\w*])", r"<em>\1</em>", t)
    return devolve(t)


def blocos(md: str) -> str:
    """Parágrafos, listas (- e 1.), citações (>) e blocos de código, o que o README usa."""
    linhas = md.strip("\n").split("\n")
    out: list[str] = []
    i = 0
    while i < len(linhas):
        l = linhas[i]
        if not l.strip():
            i += 1
            continue
        if l.startswith("```"):
            j = i + 1
            while j < len(linhas) and not linhas[j].startswith("```"):
                j += 1
            out.append("<pre><code>" + esc("\n".join(linhas[i + 1 : j])) + "</code></pre>")
            i = j + 1
            continue
        if l.startswith(">"):
            j = i
            while j < len(linhas) and linhas[j].startswith(">"):
                j += 1
            dentro = "\n".join(x[1:].removeprefix(" ") for x in linhas[i:j])
            out.append(f"<blockquote>{blocos(dentro)}</blockquote>")
            i = j
            continue
        lista = re.match(r"^(- |\d+\. )", l)
        if lista:
            tag = "ul" if l.startswith("- ") else "ol"
            itens: list[str] = []
            j = i
            while j < len(linhas) and linhas[j].strip():
                if re.match(r"^(- |\d+\. )", linhas[j]):
                    itens.append(re.sub(r"^(- |\d+\. )", "", linhas[j]))
                else:
                    itens[-1] += " " + linhas[j].strip()
                j += 1
            out.append(f"<{tag}>" + "".join(f"<li>{inline(x)}</li>" for x in itens) + f"</{tag}>")
            i = j
            continue
        if l.startswith("|") or l.startswith("#"):
            # Tabelas e títulos do README não entram nos trechos da página.
            i += 1
            continue
        j = i
        while j < len(linhas) and linhas[j].strip() and not re.match(r"^(- |\d+\. |>|```|\|)", linhas[j]):
            j += 1
        out.append("<p>" + inline(" ".join(x.strip() for x in linhas[i:j])) + "</p>")
        i = j
    return "\n".join(out)


def secao(readme: str, titulo: str) -> str:
    m = re.search(rf"^## {re.escape(titulo)}\n(.*?)(?=^## |\Z)", readme, re.S | re.M)
    if not m:
        sys.exit(f"README.md: falta a seção '## {titulo}'")
    return m.group(1)


def abertura(readme: str) -> tuple[str, str, str]:
    """O português, o inglês (o parágrafo em itálico) e a citação de pedir fonte, do topo do README."""
    topo = readme.split("\n## ", 1)[0].split("\n", 1)[1]
    partes = [p.strip() for p in re.split(r"\n\s*\n", topo) if p.strip()]
    pt = [p for p in partes if not p.startswith("*") and not p.startswith(">")]
    en = [p for p in partes if p.startswith("*") and p.endswith("*")]
    pedir = [p for p in partes if p.startswith(">")]
    if not (pt and en and pedir):
        sys.exit("README.md: o topo precisa do parágrafo em português, do em inglês (em itálico) e da citação de pedir fonte")
    # Na citação, o parágrafo em inglês fica de fora da página (ela já tem o seu no topo).
    dentro = "\n".join(x[1:].removeprefix(" ") for x in pedir[0].split("\n"))
    pedir_md = "\n\n".join(
        q for q in re.split(r"\n\s*\n", dentro) if not (q.strip().startswith("*") and q.strip().endswith("*"))
    )
    return pt[0], en[0].strip("*").replace("\n", " "), pedir_md


# ---------------------------------------------------------------- dados


def sem_acento(t: str) -> str:
    return "".join(c for c in unicodedata.normalize("NFD", t.lower()) if unicodedata.category(c) != "Mn")


def slug(t: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", sem_acento(t)).strip("-")


def orgao_curto(orgao: str) -> str:
    """O órgão para o filtro: o nome antes do primeiro parêntese, vírgula, ponto e vírgula ou " e "."""
    curto = re.split(r" \(|,|;| e | pelo | pela ", orgao, maxsplit=1)[0].strip()
    return curto or orgao


def classe_web(web: str) -> str:
    w = web.strip().lower()
    if w.startswith("sim"):
        return "sim"
    if w.startswith("só no app web"):
        return "so-web"
    if w.startswith("não"):
        return "nao"
    return "depende"


def com_versao(nome: str) -> str:
    """O endereço de site/<nome> com o resumo do conteúdo (?v=): o GitHub Pages guarda os arquivos
    no cache por 10 minutos, e sem isso a página nova podia rodar com o script ou o estilo antigos."""
    resumo = hashlib.sha256((ROOT / "site" / nome).read_bytes()).hexdigest()[:10]
    return f"{nome}?v={resumo}"


def tamanho_png(p: pathlib.Path) -> tuple[int, int]:
    with open(p, "rb") as f:
        cab = f.read(24)
    if cab[:8] != b"\x89PNG\r\n\x1a\n":
        sys.exit(f"{p}: não é um PNG")
    return struct.unpack(">II", cab[16:24])


def ler_tudo():
    linhas = gerar.ler()
    nomes = gerar.nomes_dos_comandos(linhas)
    fichas = gerar.ler_fichas(linhas, list(nomes))
    manifesto = json.loads((ROOT / "plugin.json").read_text(encoding="utf-8"))
    if manifesto["version"] != gerar.VERSAO:
        sys.exit("plugin.json está velho: rode python scripts/gerar.py")
    readme = (ROOT / "README.md").read_text(encoding="utf-8")
    return linhas, nomes, fichas, manifesto, readme


# ---------------------------------------------------------------- HTML


def janela(cmd: dict) -> str:
    """Os campos da janela do comando, do form do plugin.json."""
    campos = cmd.get("form") or []
    if not campos:
        return ""
    itens = []
    for f in campos:
        extra = ""
        if f.get("choices"):
            padrao = f.get("default")
            ops = "".join(
                f'<li{" class=padrao title=padrão" if o == padrao else ""}>{esc(o)}</li>' for o in f["choices"]
            )
            extra = f'<ul class="escolhas">{ops}</ul>'
        elif "default" in f and f["default"] not in ("", None):
            d = f["default"]
            d = "sim" if d is True else "não" if d is False else str(d)
            extra = f'<p class="padrao-txt">Padrão: {esc(d)}</p>'
        ajuda = f'<p>{esc(f["help"])}</p>' if f.get("help") else ""
        itens.append(f"<li><strong>{esc(f['label'])}</strong>{ajuda}{extra}</li>")
    return (
        '<section class="janela"><h5>Janela do comando</h5>'
        "<p class=nota>Os parâmetros que a janela pede antes de Executar (RustGIS 0.7.0 ou mais novo).</p>"
        f"<ul class=campos>{''.join(itens)}</ul></section>"
    )


def cartao(cid: str, fi: dict, nome: str, lugares: list[str], cmd: dict, vetorial: dict | None, imagem: tuple | None, topico: str) -> str:
    """A ficha de uma camada; lugares são os menus da aba em que o botão está, o primeiro o desta ficha."""
    onde = lugares[0]
    tambem = f" (também em {esc(' e '.join(lugares[1:]))})" if len(lugares) > 1 else ""
    web = classe_web(fi["web"])
    orgao = orgao_curto(fi["orgao"])
    selos = [f'<span class="selo web-{web}">{WEB_CLASSES[web]}</span>']
    if imagem:
        selos.append('<span class="selo mapa">mapa exemplo</span>')
    if cmd.get("form"):
        selos.append('<span class="selo janela-selo">janela</span>')
    campos = [
        ("Período", fi["periodo"]),
        ("Resolução ou escala", fi["resolucao"]),
        ("Unidades", fi["unidades"]),
        ("Legenda", fi["legenda"]),
        ("Cuidados de uso", fi["cuidados"]),
        ("Licença", fi["licenca"]),
        ("Citação", fi["citacao"]),
        ("App web", fi["web"]),
    ]
    dl = "".join(f"<dt>{esc(k)}</dt><dd>{esc(gerar.frase(v))}</dd>" for k, v in campos if v.strip())
    if vetorial:
        tipo = {"wfs": "WFS", "arcgis": "ArcGIS REST"}[vetorial["tipo"]]
        dl += (
            f'<dt>Serviço</dt><dd><a href="{esc(vetorial["servico"])}">{tipo}</a> '
            f'<code>{esc(vetorial["camada"])}</code></dd>'
            f'<dt>Recorte</dt><dd>{"recortada pelo polígono da área" if vetorial["recorta"] == "sim" else "feições inteiras que cruzam o retângulo da área"}.</dd>'
        )
    figura = ""
    if imagem:
        larg, alt, area = imagem
        figura = (
            f'<figure><a href="camadas/{cid}.png"><img src="camadas/{cid}.png" width="{larg}" height="{alt}" '
            f'loading="lazy" decoding="async" alt="Mapa exemplo: {esc(nome)} em {esc(area)}"></a>'
            f"<figcaption>Mapa exemplo: {esc(area)}, feito com o plugin (botão Layout).</figcaption></figure>"
        )
    botao = cmd["label"].replace("\n", " ")
    return (
        f'<details class="camada" id="c-{cid}" data-topico="{slug(topico)}" data-orgao="{esc(slug(orgao))}" '
        f'data-web="{web}" data-mapa="{"1" if imagem else "0"}">'
        f'<summary><span class="nome">{esc(nome)}</span>'
        f'<span class="meta">{esc(orgao)} · {esc(onde)}</span>'
        f'<span class="selos">{"".join(selos)}</span></summary>'
        f'<div class="ficha"><p class="oque">{esc(gerar.frase(fi["o_que_e"]))}</p>{figura}'
        f"<dl>{dl}</dl>{janela(cmd)}"
        f'<p class="rodape-ficha">Na aba: <strong>{esc(onde)} › {esc(botao)}</strong>{tambem} · '
        f'<a href="{esc(gerar.endereco_da_ficha(cid))}">ficha no GitHub</a> · '
        f'<a href="#c-{cid}" class="link-camada">link desta camada</a></p></div></details>'
    )


def eco(cid: str, nome: str, onde_principal: str, topico: str, fi: dict, imagem) -> str:
    """Uma camada que também aparece num segundo subtópico: um link para a ficha."""
    web = classe_web(fi["web"])
    orgao = orgao_curto(fi["orgao"])
    return (
        f'<div class="camada eco" data-topico="{slug(topico)}" data-orgao="{esc(slug(orgao))}" data-web="{web}" '
        f'data-mapa="{"1" if imagem else "0"}"><a href="#c-{cid}">{esc(nome)}</a>'
        f'<span class="meta">{esc(orgao)} · a ficha está em {esc(onde_principal)}</span></div>'
    )


def ferramenta(item_id: str, manifesto: dict, cmds: dict) -> str:
    if item_id == "panel":
        p = manifesto["panel"]
        nome, ajuda, form = p["title"], p["help"], ""
    else:
        c = cmds[item_id]
        nome, ajuda, form = c["label"].replace("\n", " "), c["help"], janela(c)
    return (
        f'<details class="camada ferramenta" id="f-{item_id}"><summary><span class="nome">{esc(nome)}</span></summary>'
        f'<div class="ficha"><p>{esc(ajuda)}</p>{form}</div></details>'
    )


def pagina(linhas, nomes, fichas, manifesto, readme, imagens) -> str:
    cmds = {c["id"]: c for c in manifesto["commands"]}
    vetoriais = {l["id"]: l for l in linhas}
    # Os menus da aba em que cada botão está (uns poucos estão em dois).
    lugares: dict[str, list[str]] = {}
    for g in manifesto["tab"]["groups"]:
        for it in g["items"]:
            for i in it["items"] if isinstance(it, dict) else [it]:
                lugares.setdefault(i, []).append(g["label"] + (f" › {it['menu']}" if isinstance(it, dict) else ""))
    pt, en, pedir = abertura(readme)

    # O catálogo, na ordem da aba do plugin.json: tópico › subtópico › botões.
    vistos: set[str] = set()
    secoes, ferramentas, opcoes_topico = [], [], []
    orgaos: dict[str, str] = {}
    for grupo in manifesto["tab"]["groups"]:
        topico = grupo["label"]
        if topico == "Área":
            for it in grupo["items"]:
                for i in it["items"] if isinstance(it, dict) else [it]:
                    ferramentas.append(ferramenta(i, manifesto, cmds))
            continue
        blocos_sub = []
        soltos = [i for i in grupo["items"] if isinstance(i, str)]
        menus = [(m["menu"], m["items"]) for m in grupo["items"] if isinstance(m, dict)]
        if soltos:
            menus.append(("Botões fora dos menus", soltos))
        conta = 0
        for menu, itens in menus:
            cartoes = []
            for cid in itens:
                fi = fichas[cid]
                orgaos[slug(orgao_curto(fi["orgao"]))] = orgao_curto(fi["orgao"])
                if cid in vistos:
                    cartoes.append(eco(cid, nomes[cid], lugares[cid][0], topico, fi, imagens.get(cid)))
                    continue
                vistos.add(cid)
                conta += 1
                cartoes.append(cartao(cid, fi, nomes[cid], lugares[cid], cmds[cid], vetoriais.get(cid), imagens.get(cid), topico))
            blocos_sub.append(
                f'<section class="subtopico"><h4>{esc(menu)} <span class="conta">{len(itens)}</span></h4>{"".join(cartoes)}</section>'
            )
        opcoes_topico.append(f'<option value="{slug(topico)}">{esc(topico)}</option>')
        secoes.append(
            f'<section class="topico" id="t-{slug(topico)}" data-topico="{slug(topico)}">'
            f'<h3>{esc(topico)} <span class="conta">{conta}</span></h3>{"".join(blocos_sub)}</section>'
        )
    faltam = set(fichas) - vistos
    if faltam:
        sys.exit(f"plugin.json: as camadas {', '.join(sorted(faltam))} não estão na aba")

    opcoes_orgao = "".join(
        f'<option value="{k}">{esc(v)}</option>' for k, v in sorted(orgaos.items(), key=lambda kv: sem_acento(kv[1]))
    )
    nav_topicos = "".join(
        f'<a href="#t-{slug(g["label"])}">{esc(g["label"])}</a>' for g in manifesto["tab"]["groups"] if g["label"] != "Área"
    )
    n_orgaos = len({orgao_curto(f["orgao"]) for f in fichas.values()})
    n_mapas = len(imagens)
    versao = gerar.VERSAO
    min_rustgis = manifesto.get("minRustgisVersion", "")

    autoria = blocos(secao(readme, "Autoria"))
    licenca = blocos(secao(readme, "Licença"))
    instalar = blocos(secao(readme, "Instalar"))
    o_que_traz = blocos(re.split(r"\n\s*\n", secao(readme, "O que traz").strip())[0])

    return f"""<!doctype html>
<html lang="pt-BR">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>BrasilData: dados públicos do Brasil no RustGIS</title>
<meta name="description" content="Plugin do RustGIS que traz dados públicos do Brasil e conjuntos globais recortados pela área do usuário, com a fonte, a licença e a ficha de cada camada.">
<meta name="color-scheme" content="light dark">
<link rel="stylesheet" href="{com_versao("estilo.css")}">
</head>
<body>
<!-- Gerado por scripts/site.py a partir do README.md, do plugin.json, do fontes.csv, do fichas.csv e de docs/camadas; não edite à mão. -->
<a class="pular" href="#catalogo">Ir para o catálogo</a>
<header class="topo">
  <div class="largura">
    <p class="marca">BrasilData <span class="versao">versão {versao}</span></p>
    <nav class="links">
      <a href="#instalar">Instalar</a>
      <a href="#catalogo">Catálogo</a>
      <a href="#pedir">Pedir uma fonte</a>
      <a href="{RELEASES}">Versões</a>
      <a href="{REPO}">Repositório</a>
    </nav>
  </div>
</header>
<main>
<section class="abertura">
  <div class="largura">
    <h1>Dados públicos do Brasil, recortados pela sua área</h1>
    {blocos(pt)}
    <p class="en" lang="en">{inline(en)}</p>
    <div class="acoes">
      <a class="botao principal" href="#instalar">Como instalar</a>
      <a class="botao" href="{ULTIMA}">Baixar a última versão</a>
      <a class="botao" href="#catalogo">Ver as camadas</a>
    </div>
    <ul class="numeros">
      <li><strong>{len(fichas)}</strong> camadas com ficha</li>
      <li><strong>{n_mapas}</strong> mapas exemplo</li>
      <li><strong>{n_orgaos}</strong> órgãos e produtores</li>
      <li><strong>{len(manifesto["commands"])}</strong> comandos</li>
    </ul>
  </div>
</section>

<section class="largura bloco" id="para-quem">
  <h2>Para quem é</h2>
  <p>{esc(PARA_QUEM)}</p>
  {o_que_traz}
</section>

<section class="largura bloco" id="instalar">
  <h2>Como instalar</h2>
  <p>A versão atual é a <strong>{versao}</strong>, para o RustGIS {esc(min_rustgis)} ou mais novo.
  As notas de cada versão e o arquivo <code>.rgplugin</code> estão nas <a href="{RELEASES}">versões do repositório</a>.</p>
  {instalar}
</section>

<section class="largura bloco destaque" id="pedir">
  <h2>Pedir uma fonte nova</h2>
  {blocos(pedir)}
  <p><a class="botao principal" href="{ISSUE_NOVA}">Abrir uma issue</a></p>
</section>

<section class="largura bloco" id="ferramentas">
  <h2>O grupo Área</h2>
  <p>O primeiro grupo da aba escolhe a área de trabalho e a grade, sorteia pontos de campo e monta a folha do mapa.</p>
  <div class="lista">{"".join(ferramentas)}</div>
</section>

<section class="largura bloco" id="catalogo">
  <h2>Catálogo das camadas</h2>
  <p>As camadas na ordem da aba <strong>BrasilData</strong>: cada tópico é um grupo, cada subtópico um menu. Abra uma camada para ver a ficha: o que é, o período, a resolução, as unidades, a legenda, os cuidados, a citação, a licença e o mapa exemplo.</p>
  <form class="filtros" role="search" onsubmit="return false">
    <label class="busca"><span>Buscar</span><input type="search" id="busca" placeholder="chuva, solos, IBGE, Viçosa…" autocomplete="off"></label>
    <label><span>Tópico</span><select id="f-topico"><option value="">Todos</option>{"".join(opcoes_topico)}</select></label>
    <label><span>Fonte (órgão)</span><select id="f-orgao"><option value="">Todas</option>{opcoes_orgao}</select></label>
    <label><span>App web</span><select id="f-web"><option value="">Tanto faz</option>
      <option value="sim">funciona no app web</option><option value="nao">só no app desktop</option>
      <option value="so-web">só no app web</option><option value="depende">depende (login, a confirmar)</option></select></label>
    <label class="caixa"><input type="checkbox" id="f-mapa"> <span>Só com mapa exemplo</span></label>
  </form>
  <p class="resultado" id="resultado" aria-live="polite"></p>
  <nav class="topicos" aria-label="Tópicos">{nav_topicos}</nav>
  {"".join(secoes)}
  <p class="vazio" id="vazio" hidden>Nenhuma camada com esses filtros. Sentiu falta de algum dado? <a href="#pedir">Peça uma fonte nova</a>.</p>
</section>

<section class="largura bloco" id="autoria">
  <h2>Autoria e licença</h2>
  {autoria}
  {licenca}
</section>
</main>
<footer class="rodape">
  <div class="largura">
    <p>BrasilData {versao} · <a href="{REPO}">repositório</a> · <a href="{RELEASES}">versões</a> · <a href="{ISSUE_NOVA}">pedir uma fonte</a></p>
    <p class="nota">Página gerada a partir do repositório; os mapas exemplo foram feitos com o próprio plugin.</p>
  </div>
</footer>
<script src="{com_versao("busca.js")}"></script>
</body>
</html>
"""


def main() -> None:
    saida = ROOT / "_site"
    if "--saida" in sys.argv:
        saida = pathlib.Path(sys.argv[sys.argv.index("--saida") + 1]).resolve()
    linhas, nomes, fichas, manifesto, readme = ler_tudo()
    pasta = ROOT / gerar.PAGINAS
    imagens = {}
    for cid, fi in fichas.items():
        area = gerar.area_do_exemplo(fi["exemplo"])
        png = pasta / f"{cid}.png"
        if area and png.exists():
            imagens[cid] = (*tamanho_png(png), area[1])
    texto = pagina(linhas, nomes, fichas, manifesto, readme, imagens)
    # Esvazia a pasta sem apagá-la (um servidor local pode estar rodando nela).
    if saida.exists():
        for p in saida.iterdir():
            shutil.rmtree(p) if p.is_dir() else p.unlink()
    (saida / "camadas").mkdir(parents=True)
    (saida / "index.html").write_text(texto, encoding="utf-8", newline="\n")
    for nome in ("estilo.css", "busca.js"):
        shutil.copyfile(ROOT / "site" / nome, saida / nome)
    for cid in imagens:
        shutil.copyfile(pasta / f"{cid}.png", saida / "camadas" / f"{cid}.png")
    (saida / ".nojekyll").write_text("", encoding="utf-8")
    print(f"página em {saida}: {len(fichas)} camadas, {len(imagens)} mapas exemplo")


if __name__ == "__main__":
    main()
