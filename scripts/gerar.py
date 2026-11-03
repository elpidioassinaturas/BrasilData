"""Gera o plugin.json, o src/catalogo.rs e a tabela de fontes do README a partir do fontes.csv.

    python scripts/gerar.py           # escreve os três
    python scripts/gerar.py --check   # só confere que estão em dia (para o CI)

A aba vai de tópicos a subtópicos e camadas: o grupo é o tópico (Água, Meio físico, Saúde…), o
menu é o subtópico (Bacias, Relevo…) e os itens do menu são as camadas, com a fonte entre
parênteses ("Geologia (IBGE)"). Os tópicos seguem as categorias temáticas da ISO 19115 e do Perfil
MGB da INDE, juntadas para caber nos 8 grupos que o RustGIS aceita.

O fontes.csv tem uma linha por camada vetorial: id, grupo (o tópico), menu (o subtópico: o menu
do grupo em que o botão fica; vazio, um botão solto), rotulo (o botão; \\n quebra a linha), nome (o da camada no
mapa, depois de "BrasilData: "), servico (o endereço), tipo (wfs ou arcgis), camada (o tipo de
feição do WFS, ou o número da camada do ArcGIS), geometria (poligono, linha ou ponto: o ícone),
recorta (sim/nao: se é recortada pelo polígono da área), web (sim/nao: se o servidor permite
pedidos do app web, CORS; so-web: só o app web o alcança), fonte (a citação) e licenca.

As cinco últimas colunas, quando preenchidas, fazem o plugin montar o pedido ele mesmo (em vez do
`service.add`): filtro (o CQL de um WFS ou o `where` de um ArcGIS, junto com o retângulo), campos
(os atributos que vêm, separados por vírgula: o `propertyName` ou o `outFields`; deixa de fora
nomes de pessoas e CPF/CNPJ), geom (o atributo da geometria de um WFS, que o filtro e os campos
pedem), pagina (feições por pedido: o plugin conta antes e pede as páginas) e ordem (o atributo
que ordena as páginas). Um `{uf}` na camada vira a sigla, em minúsculas, de cada UF que a área
cruza (o CAR tem uma camada por UF).
"""

import csv
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSAO = "0.1.13"

# Os comandos que não vêm da tabela, com o grupo da aba em que ficam. Na aba, o grupo Área
# tem o painel (a escolha do município pelo nome) e a área pela seleção; o comando
# area-municipio fica para scripts e para a linha de comando.
AREA = [
    {
        "id": "area-selecao",
        "label": "Área:\nseleção",
        "help": "Torna a área do BrasilData os polígonos selecionados da camada ativa (por exemplo, um município trazido por Municípios).",
        "icon": "polygon_layer",
        "params": "{}",
    },
    {
        "id": "area-municipio",
        "label": "Área:\nmunicípio",
        "help": "Traz o limite de um município pela API de malhas do IBGE e o torna a área do BrasilData. Na janela, use o painel Área e fontes.",
        "icon": "polygon_layer",
        "params": "{\"nome\": nome do município (\"Teresina\", \"Bom Jesus/PI\"), \"uf\"?: sigla, quando o nome se repete} | {\"codigo\": código do IBGE, 7 dígitos}",
    },
]
NA_ABA_AREA = ["panel", "area-selecao"]
# A verificação de atualização fica no fim do grupo Área, depois das fontes.
# Os comandos do painel, que não vão para a aba.
DO_PAINEL = [
    {
        "id": "painel",
        "label": "Atualizar painel",
        "help": "O estado do painel Área e fontes: a árvore de estados e municípios, a área e as fontes.",
        "icon": "polygon_layer",
        "params": "{}",
    },
    {
        "id": "escolher-municipio",
        "label": "Escolher município",
        "help": "O nó escolhido na árvore do painel: um estado mostra os seus municípios; um município vira a área.",
        "icon": "polygon_layer",
        "params": "{\"panel\": {\"municipio\": \"uf:PI\" | \"mun:2211001\"}}",
    },
]
PAINEL = {
    "title": "Área e fontes",
    "icon": "polygon_layer",
    "help": "Escolha o município da área pelo nome, e veja as fontes das camadas do BrasilData no mapa.",
    "state": "painel",
    "items": [
        {"kind": "text", "id": "area", "label": "Área do BrasilData"},
        {
            "kind": "tree",
            "id": "municipio",
            "label": "Município",
            "help": "Abra o estado e escolha o município (a caixa acima da árvore busca pelo nome); ele vira a área.",
            "search": True,
            "command": "escolher-municipio",
        },
        {"kind": "button", "command": "area-selecao"},
        {"kind": "button", "command": "fontes"},
        {
            "kind": "slider",
            "id": "focos-de",
            "label": "Focos de calor: do dia",
            "help": "O primeiro dia do período, dia/mês/ano, no mês corrente (UTC): o serviço do IBAMA só guarda o mês corrente. As datas aparecem quando o mês é conhecido (no app web, depois do primeiro período).",
            "min": 1,
            "max": 31,
            "default": 1,
            "steps": True,
        },
        {
            "kind": "slider",
            "id": "focos-ate",
            "label": "Focos de calor: até o dia",
            "help": "O último dia do período, dia/mês/ano, no mês corrente (UTC), incluído.",
            "min": 1,
            "max": 31,
            "default": 31,
            "steps": True,
        },
        {"kind": "button", "command": "inpe-focos-periodo"},
        {"kind": "text", "id": "citacao", "label": "Fontes das camadas no mapa"},
        {"kind": "button", "command": "atualizacao"},
        {"kind": "text", "id": "versao", "label": "Versão"},
    ],
}
# Os comandos que vão no fim de um grupo da aba: as fontes, com a área.
FIM = [
    (
        "Área",
        {
            "id": "fontes",
            "label": "Fontes",
            "help": "Lista a fonte e a licença de cada camada do BrasilData que está no mapa, para a citação.",
            "icon": "table",
            "params": "{}",
        },
    ),
    (
        "Área",
        {
            "id": "atualizacao",
            "label": "Verificar\natualização",
            "help": "Compara esta versão do BrasilData com a última publicada no GitHub e diz como atualizar: pelo Gerenciador de plugins, se ele veio do registro, ou pelo arquivo .rgplugin da release.",
            "icon": "update",
            "params": "{}",
        },
    ),
]


# Comandos que não vêm da tabela e ficam num grupo da aba: (tópico, subtópico ou None, comando).
EM_MENUS = [
    (
        "Vegetação e meio ambiente",
        "Queimadas",
        {
            "id": "inpe-focos-periodo",
            "label": "Focos de calor: período (INPE)",
            "help": "Os focos de calor do INPE de uma data a outra (dia/mês/ano) do mês corrente (UTC), escolhidas nos controles do painel Área e fontes (sem eles, o mês inteiro): INPE, Programa Queimadas (Sisfogo, IBAMA/PAMGIA). Só o que cruza a área, recortado pela área; sem área, o que está na vista.",
            "icon": "point_layer",
            "params": "{\"de\"?: \"dd/mm/aaaa\" ou o dia do mês (1 a 31), \"ate\"?: \"dd/mm/aaaa\" ou o dia do mês (padrão: os do painel; o mês é o corrente)}",
        },
    ),
    (
        "Sociedade e economia",
        "População",
        {
            "id": "censo-setores",
            "label": "Setores censitários (IBGE)",
            "help": "Os setores censitários de 2022 da área, com pessoas e domicílios (IBGE, agregados por setores): o GeoPackage de cada UF, recortado pela área. No app web, um segundo clique recorta.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "População",
        {
            "id": "censo-municipios",
            "label": "Censos por município (IBGE)",
            "help": "População, área e densidade de 2022 (Censo Demográfico) e estabelecimentos agropecuários e agricultura familiar de 2017 (Censo Agropecuário), do IBGE. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "indicadores-pix",
            "label": "Pix (Banco Central)",
            "help": "O Pix do último mês completo: valor pago e recebido (R$), transações pagas e pessoas físicas pagadoras. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "indicadores-credito",
            "label": "Crédito rural de custeio (Banco Central)",
            "help": "O custeio do último ano completo, do SICOR: total, Pronaf, agrícola e pecuário (R$) e área (ha). O SICOR é lento: cerca de 10 s para cada 6 municípios. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "censo-agro",
            "label": "Censo Agropecuário 2017 (IBGE)",
            "help": "Do Censo Agropecuário de 2017 (IBGE): estabelecimentos e agricultura familiar, utilização das terras (lavouras, pastagens e matas, ha), bovinos, pessoal ocupado, tratores, irrigação, uso de agrotóxicos, energia elétrica, orientação técnica e valor da produção (mil R$). Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem; o painel Fontes diz o que é cada campo.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "censo-agroecologia",
            "label": "Agroecologia (Censo Agropecuário 2017)",
            "help": "Práticas agroecológicas do Censo Agropecuário de 2017 (IBGE), em número de estabelecimentos: agricultura ou pecuária orgânica, associação a cooperativa ou entidade, práticas de conservação (plantio em nível, rotação de culturas, pousio, proteção de encostas, recuperação de mata ciliar, reflorestamento de nascentes, manejo florestal), plantio direto (e sua área), cultivo mínimo, adubação orgânica ou química, quem não usou agrotóxicos e a área de sistemas agroflorestais (ha). Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem; o painel Fontes diz o que é cada campo.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Eleições",
        {
            "id": "indicadores-tse",
            "label": "Eleitorado e comparecimento (TSE)",
            "help": "Eleitorado, comparecimento e abstenção (%) no 1º turno de 2026 e de 2024, da divulgação oficial do TSE. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "indicadores-caged",
            "label": "Emprego formal (Novo CAGED)",
            "help": "Admissões, desligamentos e saldo nos últimos 12 meses e o saldo do último mês, do Novo CAGED (IPEA Data), num retrato que vem com o plugin. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        None,
        {
            "id": "indicadores-todos",
            "label": "Todos os indicadores",
            "help": "Censos, Censo Agropecuário, agroecologia, Pix, crédito rural, eleições e emprego formal, de uma vez. Na camada Indicadores por município (os municípios que cruzam a área, até 50): os campos se somam aos que ela já tem. No desktop, se o tempo de uma chamada acabar, o aviso pede um novo clique, que continua de onde parou.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Saúde",
        "Estabelecimentos",
        {
            "id": "saude-estabelecimentos",
            "label": "Estabelecimentos de saúde (CNES, DATASUS)",
            "help": "Os estabelecimentos de saúde ativos na área, do CNES do Ministério da Saúde (competência 04/2026, pelo geobr do Ipea), com nome, tipo, natureza jurídica, gestão e bairro. Os de pessoa física e de empresário individual, cujo nome é o de uma pessoa, ficam de fora. Só no desktop: o primeiro clique abre o arquivo do Brasil (98 MB, lido aos pedaços), o segundo recorta pela área.",
            "icon": "point_layer",
            "params": "{}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "mapbiomas-cobertura",
            "label": "Uso e cobertura 2025 (MapBiomas)",
            "help": "O uso e a cobertura da terra do MapBiomas (Coleção 11, 30 m), com as cores da legenda oficial: um mapa para ver, do Brasil inteiro, não recortado pela área. Por script, outro ano de 1985 a 2025. O mapa vale por algumas horas; um novo clique pede outro.",
            "icon": "raster_layer",
            "params": "{\"ano\"?: de 1985 a 2025 (padrão 2025)}",
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "elevacao",
            "label": "Elevação (Copernicus GLO-30)",
            "help": "Copernicus GLO-30 recortado pela área e posto na grade UTM SIRGAS 2000 do fuso da área (ou na Policônica, quando a área passa de um fuso).",
            "icon": "raster_layer",
            "params": "{\"celula\"?: tamanho da célula em metros (padrão 30)}",
        },
    ),
]
# Comandos só para scripts e a linha de comando, fora da aba.
SO_SCRIPTS = [
    {
        "id": "indicadores",
        "label": "Indicadores por município",
        "help": "Os indicadores das fontes escolhidas na camada Indicadores por município.",
        "icon": "polygon_layer",
        "params": "{\"fontes\"?: lista entre censo, pix, credito, tse e caged (padrão: todas)}",
    },
]


def ler() -> list[dict]:
    with open(ROOT / "fontes.csv", encoding="utf-8", newline="") as f:
        linhas = list(csv.DictReader(f))
    ids = set()
    for n, l in enumerate(linhas, start=2):
        onde = f"fontes.csv, linha {n}"
        if l["id"] in ids or any(l["id"] == c["id"] for c in AREA + DO_PAINEL + SO_SCRIPTS + [c for _, c in FIM] + [c for _, _, c in EM_MENUS]):
            sys.exit(f"{onde}: o id '{l['id']}' se repete")
        ids.add(l["id"])
        for campo, valores in (
            ("recorta", ("sim", "nao")),
            ("web", ("sim", "nao", "so-web")),
            ("tipo", ("wfs", "arcgis")),
            ("geometria", tuple(ICONES)),
        ):
            if l[campo] not in valores:
                sys.exit(f"{onde}: {campo} é {' ou '.join(valores)}, não '{l[campo]}'")
        if not l["servico"].startswith("https://"):
            sys.exit(f"{onde}: o serviço precisa de https://")
        if l["tipo"] == "arcgis" and not l["camada"].isdigit():
            sys.exit(f"{onde}: a camada de um ArcGIS é o número dela no serviço")
        for campo in ("grupo", "rotulo", "nome", "camada", "fonte", "licenca"):
            if not l[campo].strip():
                sys.exit(f"{onde}: {campo} está vazio")
        if l["pagina"] and not (l["pagina"].isdigit() and int(l["pagina"]) > 0):
            sys.exit(f"{onde}: pagina é o número de feições por pedido, ou vazio")
        if l["pagina"] and not l["ordem"]:
            sys.exit(f"{onde}: as páginas pedem a ordem (o atributo que as ordena)")
        if l["tipo"] == "wfs" and (l["filtro"] or l["campos"]) and not l["geom"]:
            sys.exit(f"{onde}: o filtro e os campos de um WFS pedem geom, o atributo da geometria")
        if "{uf}" in l["camada"] and l["tipo"] != "wfs":
            sys.exit(f"{onde}: só a camada de um WFS leva {{uf}}")
        if any(c.strip() != c or not c for c in l["campos"].split(",")) and l["campos"]:
            sys.exit(f"{onde}: campos são nomes separados por vírgula, sem espaços")
    return linhas


# Os tópicos da aba e os subtópicos de cada um, na ordem em que aparecem (o RustGIS aceita até 8
# grupos e 16 itens por grupo). A aba é conferida contra esta lista: um subtópico que não está
# aqui, ou que ficou vazio, para o gerar.py.
TOPICOS = {
    "Área": [],
    "Território": ["Limites", "Recortes especiais", "Cadastro fundiário", "Terras indígenas"],
    "Sociedade e economia": ["População", "Economia", "Agricultura", "Eleições"],
    "Saúde": ["Estabelecimentos"],
    "Meio físico": ["Geologia e recursos minerais", "Relevo", "Solos"],
    "Água": ["Hidrografia", "Bacias", "Recursos hídricos"],
    "Vegetação e meio ambiente": ["Vegetação", "Uso e cobertura", "Desmatamento", "Queimadas", "Áreas protegidas", "Fiscalização"],
    "Infraestrutura": ["Energia", "Petróleo e gás", "Transportes"],
}
GRUPOS = list(TOPICOS)
# Comandos que também aparecem num segundo subtópico: (tópico, subtópico, id). A Agricultura
# tem o Censo Agropecuário próprio (censo-agro); os censos por município ficam na População.
TAMBEM = []

# O ícone de um subtópico, quando um dos ícones do RustGIS diz mais que a geometria da primeira
# camada (o plugin só pode usar os ícones do RustGIS, pelo nome).
MENU_ICONES = {
    "Áreas protegidas": "lock",
    "Cadastro fundiário": "table",
    "População": "stats",
    "Economia": "chart",
    "Eleições": "count",
    "Relevo": "relief",
    "Desmatamento": "time_slider",
    "Transportes": "directions",
}

# O ícone do RustGIS de cada geometria.
ICONES = {"poligono": "polygon_layer", "linha": "line_layer", "ponto": "point_layer"}


def manifesto(linhas: list[dict]) -> str:
    comandos = AREA + DO_PAINEL + SO_SCRIPTS
    grupos: dict[str, list] = {"Área": list(NA_ABA_AREA)}
    for l in linhas:
        onde = "recortado pela área" if l["recorta"] == "sim" else "inteiro, no retângulo da área"
        so_desktop = {
            "sim": "",
            "nao": " Só no app desktop: o servidor não aceita pedidos do app web.",
            "so-web": " Só no app web: o servidor só aceita cifras TLS antigas, que o app desktop não aceita.",
        }[l["web"]]
        comandos.append(
            {
                "id": l["id"],
                "label": l["rotulo"].replace("\\n", "\n"),
                "help": f"{l['nome']}: {l['fonte']}. Só o que cruza a área, {onde}; sem área, o que está na vista.{so_desktop}",
                "icon": ICONES[l["geometria"]],
                "params": "{}",
            }
        )
        itens = grupos.setdefault(l["grupo"], [])
        # A camada vai para o menu da fonte, onde quer que ele esteja no grupo.
        m = next((i for i in itens if isinstance(i, dict) and i["menu"] == l["menu"]), None)
        if not l["menu"]:
            itens.append(l["id"])
        elif m is None:
            itens.append({"menu": l["menu"], "icon": MENU_ICONES.get(l["menu"], ICONES[l["geometria"]]), "items": [l["id"]]})
        else:
            m["items"].append(l["id"])
    for grupo, menu, c in EM_MENUS:
        comandos.append(c)
        itens = grupos.setdefault(grupo, [])
        if menu is None:
            itens.append(c["id"])
            continue
        m = next((i for i in itens if isinstance(i, dict) and i["menu"] == menu), None)
        if m is None:
            itens.append({"menu": menu, "icon": MENU_ICONES.get(menu, c["icon"]), "items": [c["id"]]})
        else:
            m["items"].append(c["id"])
    for grupo, c in FIM:
        comandos.append(c)
        grupos.setdefault(grupo, []).append(c["id"])
    for grupo, menu, cid in TAMBEM:
        m = next((i for i in grupos.get(grupo, []) if isinstance(i, dict) and i["menu"] == menu), None)
        if m is None:
            sys.exit(f"TAMBEM: o subtópico {grupo} › {menu} não existe")
        m["items"].append(cid)
    fora = [g for g in grupos if g not in GRUPOS]
    if fora:
        sys.exit(f"grupos fora de TOPICOS: {', '.join(fora)}")
    grupos = {g: grupos[g] for g in GRUPOS if g in grupos}
    # Os subtópicos na ordem de TOPICOS, e os botões soltos depois deles.
    for g, itens in grupos.items():
        menus = [i["menu"] for i in itens if isinstance(i, dict)]
        if sorted(menus) != sorted(TOPICOS[g]):
            sys.exit(f"{g}: a aba tem os subtópicos {menus}, e TOPICOS diz {TOPICOS[g]}")
        itens.sort(key=lambda i: TOPICOS[g].index(i["menu"]) if isinstance(i, dict) else len(TOPICOS[g]))
    if len(grupos) > 8:
        sys.exit(f"a aba tem {len(grupos)} grupos; o RustGIS aceita até 8")
    m = {
        "id": "brasildata",
        "name": "BrasilData",
        "version": VERSAO,
        "description": "Dados públicos do Brasil para a área do usuário (IBGE, MMA, ICMBio, INPE, ANA, ANEEL, ANP, SFB/CAR, INCRA, FUNAI, IBAMA, ANM, DNIT, Ministério da Saúde, Copernicus): só o que cruza a área, recortado por ela, com a fonte e a licença de cada camada.",
        "author": "Elpídio Filho",
        "homepage": "https://github.com/elpidioassinaturas/BrasilData",
        "license": "MIT",
        "entry": "plugin.wasm",
        "abi": 1,
        "minRustgisVersion": "0.5.0",
        "permissions": ["edit", "files", "network"],
        "commands": comandos,
        "panel": PAINEL,
        "tab": {"label": "BrasilData", "groups": [{"label": g, "items": i} for g, i in grupos.items()]},
    }
    return json.dumps(m, ensure_ascii=False, indent=2) + "\n"


def catalogo(linhas: list[dict]) -> str:
    s = lambda t: json.dumps(t, ensure_ascii=False)  # um literal de texto do Rust
    out = [
        "//! Gerado por `scripts/gerar.py` a partir do `fontes.csv`: não edite à mão.",
        "",
        "use crate::Tema;",
        "",
        "pub const TEMAS: &[Tema] = &[",
    ]
    for l in linhas:
        out += [
            "    Tema {",
            f"        id: {s(l['id'])},",
            f"        grupo: {s(l['grupo'])},",
            f"        nome: {s(l['nome'])},",
            f"        servico: {s(l['servico'])},",
            f"        tipo: {s(l['tipo'])},",
            f"        camada: {s(l['camada'])},",
            f"        geometria: {s(l['geometria'])},",
            f"        recorta: {'true' if l['recorta'] == 'sim' else 'false'},",
            f"        fonte: {s(l['fonte'])},",
            f"        licenca: {s(l['licenca'])},",
            f"        filtro: {s(l['filtro'])},",
            f"        campos: {s(l['campos'])},",
            f"        geom: {s(l['geom'])},",
            f"        pagina: {int(l['pagina'] or 0)},",
            f"        ordem: {s(l['ordem'])},",
            "    },",
        ]
    out.append("];")
    return "\n".join(out) + "\n"


# Os marcadores da tabela de fontes no README: o que fica entre eles é refeito a cada vez.
README_INICIO = "<!-- fontes: início (gerado por scripts/gerar.py a partir do fontes.csv; não edite à mão) -->"
README_FIM = "<!-- fontes: fim -->"


def readme(linhas: list[dict]) -> str:
    """O README com a tabela do fontes.csv entre os marcadores."""
    atual = (ROOT / "README.md").read_text(encoding="utf-8")
    if atual.count(README_INICIO) != 1 or atual.count(README_FIM) != 1:
        sys.exit(f"README.md: falta o par de marcadores {README_INICIO} ... {README_FIM}")
    cel = lambda s: s.replace("|", "\\|").replace("\\n", " ").strip()
    web = {"sim": "sim", "nao": "não (sem CORS)", "so-web": "só no app web"}
    tipo = {"wfs": "WFS", "arcgis": "ArcGIS REST"}
    orgaos = {l["fonte"].split(",")[0].strip() for l in linhas}
    tabela = [
        README_INICIO,
        f"{len(linhas)} camadas de {len(orgaos)} fontes, na ordem do `fontes.csv`. *App web*: se o servidor "
        "permite pedidos do navegador (CORS); *Recorte*: se a camada é recortada pela área.",
        "",
        "| Grupo | Camada | Fonte | Serviço | Licença | App web | Recorte |",
        "|---|---|---|---|---|---|---|",
    ]
    for l in linhas:
        grupo = cel(l["grupo"]) + (f" › {cel(l['menu'])}" if l["menu"] else "")
        servico = f"[{tipo.get(l['tipo'], l['tipo'])}]({l['servico']}) `{cel(l['camada'])}`"
        tabela.append(
            f"| {grupo} | {cel(l['nome'])} | {cel(l['fonte'])} | {servico} | {cel(l['licenca'])} "
            f"| {web.get(l['web'], l['web'])} | {'sim' if l['recorta'] == 'sim' else 'não'} |"
        )
    tabela.append(README_FIM)
    antes, resto = atual.split(README_INICIO)
    depois = resto.split(README_FIM, 1)[1]
    return antes + "\n".join(tabela) + depois


def main() -> None:
    linhas = ler()
    arquivos = {
        ROOT / "plugin.json": manifesto(linhas),
        ROOT / "src" / "catalogo.rs": catalogo(linhas),
        ROOT / "README.md": readme(linhas),
    }
    if "--check" in sys.argv:
        velhos = [p.name for p, t in arquivos.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        if velhos:
            sys.exit(f"desatualizados: {', '.join(velhos)}; rode python scripts/gerar.py")
        print("plugin.json, catalogo.rs e README.md em dia")
        return
    for p, t in arquivos.items():
        p.write_text(t, encoding="utf-8", newline="\n")
        print("escrito", p.relative_to(ROOT))


if __name__ == "__main__":
    main()
