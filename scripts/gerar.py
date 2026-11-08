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
que ordena as páginas; num WFS pode ficar vazia, e o GeoServer usa a chave primária). Um `{uf}` na camada vira a sigla, em minúsculas, de cada UF que a área
cruza (o CAR tem uma camada por UF).
"""

import csv
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSAO = "0.1.24"

# Os comandos que não vêm da tabela, com o grupo da aba em que ficam. Na aba, o grupo Área
# tem o painel (a escolha do território pelo nome), a área pela seleção e o desenho; o comando
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
        "id": "area-desenho",
        "label": "Área:\ndesenho",
        "help": "Desenhe a área no mapa: o primeiro clique neste botão liga a ferramenta (clique nos cantos do polígono e dê um duplo clique para fechá-lo); o segundo faz do desenho a área do BrasilData.",
        "icon": "polygon_tool",
        "params": "{}",
    },
    {
        "id": "area-municipio",
        "label": "Área:\nmunicípio",
        "help": "Torna um município a área do BrasilData, com o limite do IBGE que vem no plugin. Na janela, use o painel Área de trabalho.",
        "icon": "polygon_layer",
        "params": "{\"nome\": nome do município (\"Teresina\", \"Bom Jesus/PI\"), \"uf\"?: sigla, quando o nome se repete} | {\"codigo\": código do IBGE, 7 dígitos}",
    },
    {
        "id": "grade",
        "label": "Grade do\nprojeto",
        "help": "A grade em que os rasters do BrasilData chegam (a Elevação, e depois os temas rasterizados): o sistema de coordenadas e o tamanho das células, no retângulo da área, como a camada BrasilData: grade. Escolha a célula e o sistema no painel Área de trabalho. Rasters na mesma grade têm as mesmas linhas e colunas. Uma área nova refaz a grade com as mesmas escolhas.",
        "icon": "graticule",
        "params": "{\"celula\"?: tamanho da célula em metros, de 1 a 10.000 (padrão: o do painel, ou 30), \"crs\"?: código EPSG em metros (31983, \"EPSG:5880\") ou \"automatico\" (UTM do fuso, ou o Albers se a área passa de um fuso), \"policonica\" (que o RustGIS 0.5.0 ainda não projeta) ou \"albers\" (EPSG:10857, o do IBGE)}",
    },
]
NA_ABA_AREA = ["panel", "area-selecao", "area-desenho", "grade"]
# A verificação de atualização fica no fim do grupo Área, depois das fontes.
# Os comandos do painel, que não vão para a aba.
DO_PAINEL = [
    {
        "id": "painel",
        "label": "Atualizar painel",
        "help": "O estado do painel Área de trabalho: a árvore Limites políticos, a área e a grade.",
        "icon": "polygon_layer",
        "params": "{}",
    },
    {
        "id": "escolher-area",
        "label": "Escolher área",
        "help": "O nó escolhido na árvore Limites políticos vira a área: o Brasil, uma grande região, um estado, uma mesorregião, uma microrregião ou um município, com o limite do IBGE que vem no plugin.",
        "icon": "polygon_layer",
        "params": "{\"no\": \"br\" | \"reg:2\" | \"uf:PI\" | \"meso:2204\" | \"micro:22010\" | \"mun:2211001\"} (por script), ou o nó escolhido na árvore limites do painel",
    },
]
ARVORE = "Clique no Brasil, numa grande região, estado, mesorregião, microrregião ou município (divisão do IBGE de 1989): ele vira a área de trabalho. O triângulo abre o ramo, e a caixa busca pelo nome. Os municípios aparecem quando você escolhe a grande região deles, ou qualquer lugar dentro dela: o RustGIS guarda até 5.000 nós por árvore, e o Brasil tem 5.570 municípios. Os limites vêm no plugin (IBGE), sem depender da rede; para o contorno exato, traga a camada de Limites e use Área: seleção."
PAINEL = {
    "title": "Área de trabalho",
    "icon": "polygon_layer",
    "help": "Escolha a área de trabalho nos limites políticos (Brasil, grandes regiões, estados, meso e microrregiões, municípios), a seleção ou um desenho, a grade dos rasters e o ano do MapBiomas.",
    "state": "painel",
    "items": [
        {"kind": "text", "id": "area", "label": "Área de trabalho"},
        {
            "kind": "tree",
            "id": "limites",
            "label": "Limites políticos",
            "help": ARVORE,
            "search": True,
            "command": "escolher-area",
        },
        {"kind": "button", "command": "area-selecao"},
        {"kind": "button", "command": "area-desenho"},
        {"kind": "text", "id": "grade-texto", "label": "Grade do projeto"},
        {
            "kind": "slider",
            "id": "grade-celula",
            "label": "Grade: célula",
            "help": "O tamanho das células da grade, de 10 m a 1 km. Vale quando você clicar em Grade do projeto.",
            "min": 0,
            "max": 8,
            "default": 2,
            "steps": True,
        },
        {
            "kind": "slider",
            "id": "grade-sistema",
            "label": "Grade: sistema",
            "help": "O sistema de coordenadas da grade: o automático (UTM SIRGAS 2000 do fuso da área, ou o Albers do IBGE quando a área passa de um fuso), a Policônica do Brasil (EPSG:5880, que o RustGIS 0.5.0 ainda não projeta) ou o Albers do IBGE (EPSG:10857), que preserva as áreas. Vale quando você clicar em Grade do projeto.",
            "min": 0,
            "max": 2,
            "default": 0,
            "steps": True,
        },
        {"kind": "button", "command": "grade"},
        {
            "kind": "slider",
            "id": "mapbiomas-ano",
            "label": "MapBiomas: ano",
            "help": "O ano do mapa de uso e cobertura do MapBiomas (Coleção 11), de 1985 a 2025. Vale quando você clicar em Uso e cobertura (MapBiomas), aqui ou na aba.",
            "min": 1985,
            "max": 2025,
            "default": 2025,
            "steps": True,
        },
        {"kind": "button", "command": "mapbiomas-cobertura"},
    ],
}
# Os comandos que vão no fim de um grupo da aba: as fontes, com a área.
FIM = [
    (
        "Área",
        {
            "id": "fontes",
            "label": "Fontes",
            "help": "A fonte e a licença de cada camada do BrasilData que está no mapa, para a citação, na tabela BrasilData: fontes (salva com o projeto).",
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
    (
        "Área",
        {
            "id": "sobre",
            "label": "Sobre",
            "help": "A autoria do BrasilData, a versão, a licença, o repositório, as fontes dos dados e como contribuir. O texto completo fica em Sobre o BrasilData, no fim do painel Área de trabalho.",
            "icon": "about",
            "params": "{}",
        },
    ),
]

# A seção Sobre: a autoria e o resto, no fim do painel (o RustGIS não tem janela de texto para um
# plugin, e um comando não abre o painel) e, resumida, no aviso do botão Sobre. O autor, a
# licença e o repositório são os do manifesto.
AUTOR = "Elpídio Filho"
LABORATORIO = "Laboratório de Geoprocessamento e Pedometria (LABGEO)"
INSTITUICAO = "Universidade Federal de Viçosa (UFV)"
LICENCA = "MIT"
REPOSITORIO = "https://github.com/elpidioassinaturas/BrasilData"
# As fontes fora do fontes.csv, depois dos órgãos das camadas vetoriais.
OUTRAS_FONTES = [
    "Copernicus DEM GLO-30 (DLR e Airbus, pelo espelho da OpenTopography)",
    "Banco Central (Pix e crédito rural)",
    "TSE",
    "Novo CAGED (IPEA Data)",
    "Ministério da Saúde (CNES, pelo geobr do Ipea)",
]
# O RustGIS aceita até 2.000 caracteres num texto declarado do painel e corta um aviso em 500
# (com o "BrasilData: " na frente).
MAX_SOBRE = 2000
MAX_AVISO = 500 - len("BrasilData: ")


def sobre(linhas: list[dict]) -> tuple[str, str]:
    """O texto da seção Sobre e o aviso curto do botão."""
    # Os órgãos das camadas vetoriais, do que tem mais camadas ao que tem menos.
    conta: dict[str, int] = {}
    for l in linhas:
        o = l["fonte"].split(",")[0].strip()
        conta[o] = conta.get(o, 0) + 1
    orgaos = sorted(conta, key=lambda o: -conta[o])
    texto = "\n\n".join(
        [
            f"BrasilData {VERSAO}, plugin do RustGIS que traz dados públicos do Brasil para a área de trabalho.",
            f"Autor: {AUTOR}, do {LABORATORIO} da {INSTITUICAO}.\n"
            f"Licença: {LICENCA}, a do código. Os dados são dos órgãos que os publicam, cada um com a sua licença: "
            "o botão Fontes põe a citação e a licença de cada camada do mapa na tabela BrasilData: fontes.\n"
            f"Repositório: {REPOSITORIO}",
            f"Fontes dos dados: {', '.join(orgaos[:-1])} e {orgaos[-1]}, pelos serviços de cada órgão; "
            f"{'; '.join(OUTRAS_FONTES[:-1])} e {OUTRAS_FONTES[-1]}.",
            "Como contribuir: sentiu falta de algum dado? Abra uma issue em "
            f"{REPOSITORIO}/issues com o nome do dado, o órgão que o publica, o endereço do serviço "
            "(WFS, ArcGIS REST, API ou arquivo) e para que você o usaria. Para trazer a fonte você "
            "mesmo, acrescente uma linha ao fontes.csv, rode python scripts/gerar.py e abra um pull "
            "request. Correções, avisos de serviço fora do ar e relatos de uso também são bem-vindos.",
        ]
    )
    aviso = (
        f"BrasilData {VERSAO}, de {AUTOR} (LABGEO/UFV), sob a licença {LICENCA}: {REPOSITORIO}. A autoria, as fontes "
        "dos dados e como contribuir estão em Sobre o BrasilData, no fim do painel Área de trabalho."
    )
    if len(texto) > MAX_SOBRE:
        sys.exit(f"o texto Sobre tem {len(texto)} caracteres; o RustGIS aceita até {MAX_SOBRE}")
    if len(aviso) > MAX_AVISO:
        sys.exit(f"o aviso Sobre tem {len(aviso)} caracteres; o RustGIS corta em {MAX_AVISO}")
    return texto, aviso


# Comandos que não vêm da tabela e ficam num grupo da aba: (tópico, subtópico ou None, comando).
EM_MENUS = [
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
            "label": "Uso e cobertura (MapBiomas)",
            "help": "O uso e a cobertura da terra do MapBiomas, com as cores da legenda oficial. O ano, de 1985 a 2025, é o do controle MapBiomas: ano, no painel Área de trabalho (2025 até você mudar). Com uma área, de 2019 a 2024 e num RustGIS depois da 0.5.0, vem o raster das classes (Coleção 10, 30 m): só a área, recortada pelo contorno e posta na grade do projeto. Nos outros casos, um mapa para ver, do Brasil inteiro (Coleção 11), que vale por algumas horas. Cada ano vira uma camada.",
            "icon": "raster_layer",
            "params": "{\"ano\"?: de 1985 a 2025 (padrão: o do painel, ou 2025), \"recorte\"?: false para o mapa do Brasil inteiro mesmo com área}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-vv",
            "label": "Radar VV (Sentinel-1, OPERA)",
            "help": "A polarização VV: responde mais à rugosidade e à umidade do solo. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-vh",
            "label": "Radar VH (Sentinel-1, OPERA)",
            "help": "A polarização VH: responde mais ao volume da vegetação. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-razao",
            "label": "Razão VH/VV (Sentinel-1, OPERA)",
            "help": "A razão VH/VV, que separa vegetação de solo exposto; as camadas VV e VH da mesma passagem também ficam no mapa. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-situacao",
            "label": "Perturbação da vegetação: situação (OPERA DIST-S1)",
            "help": "A situação do alerta: a cor diz a etapa (amarelo, primeira detecção; laranja, provisória; vermelho, confirmada; roxo, encerrada, quando a vegetação voltou ao normal) e, mais escura, a confiança alta; sem perturbação fica transparente. As manchas vêm também como polígonos, com borda fina e o nome da classe na legenda; o raster, na grade, fica escondido. Por script, a paleta oficial da OPERA: {\"paleta\": \"oficial\"}. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30), \"paleta\"?: \"brasildata\" (padrão: a cor diz a etapa e, mais escura, a confiança alta) | \"oficial\" (a da OPERA)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-data",
            "label": "Perturbação da vegetação: data (OPERA DIST-S1)",
            "help": "Quantos dias antes do produto mais recente a perturbação foi vista pela primeira vez: as mais novas em vermelho-escuro. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-confianca",
            "label": "Perturbação da vegetação: confiança (OPERA DIST-S1)",
            "help": "A confiança do alerta, que cresce a cada passagem que confirma a perturbação. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-intensidade",
            "label": "Perturbação da vegetação: intensidade (OPERA DIST-S1)",
            "help": "A intensidade: o maior desvio do sinal do radar em relação ao histórico do lugar, desde a primeira detecção. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "raster_layer",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "elevacao",
            "label": "Elevação (Copernicus GLO-30)",
            "help": "Copernicus GLO-30 recortado pela área e posto na grade do projeto (Grade do projeto, no grupo Área). Sem grade, o primeiro clique cria a padrão: células de 30 m no UTM SIRGAS 2000 do fuso da área, ou no Albers do IBGE quando a área passa de um fuso.",
            "icon": "raster_layer",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30); com grade, só o da grade é aceito}",
        },
    ),
]
# Comandos só para scripts e a linha de comando, fora da aba.
SO_SCRIPTS = [
    {
        "id": "inpe-focos-periodo",
        "label": "Focos de calor: período (INPE)",
        "help": "Os focos de calor do INPE de uma data a outra (dia/mês/ano) do mês corrente (UTC): INPE, Programa Queimadas (Sisfogo, IBAMA/PAMGIA). Só o que cruza a área, recortado pela área; sem área, o que está na vista. Por script, até o RustGIS abrir uma janela de parâmetros para comandos de plugin; na aba, Queimadas tem 48 horas, 7 dias e o mês corrente.",
        "icon": "point_layer",
        "params": "{\"de\"?: \"dd/mm/aaaa\" ou o dia do mês (1 a 31), \"ate\"?: \"dd/mm/aaaa\" ou o dia do mês (padrão: o mês corrente inteiro)}",
    },
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
        # Um WFS (GeoServer) pagina na ordem da chave primária quando não há sortBy.
        if l["pagina"] and not l["ordem"] and l["tipo"] != "wfs":
            sys.exit(f"{onde}: as páginas de um ArcGIS pedem a ordem (o atributo que as ordena)")
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
    painel = dict(PAINEL, items=PAINEL["items"] + [{"kind": "text", "id": "sobre", "label": "Sobre o BrasilData", "text": sobre(linhas)[0]}])
    m = {
        "id": "brasildata",
        "name": "BrasilData",
        "version": VERSAO,
        "description": "Dados públicos do Brasil para a área do usuário (IBGE, MMA, ICMBio, INPE, ANA, ANEEL, ANP, SFB/CAR, INCRA, FUNAI, IBAMA, ANM, DNIT, Ministério da Saúde, Copernicus, NASA/ASF): só o que cruza a área, recortado por ela, com a fonte e a licença de cada camada.",
        "author": AUTOR,
        "homepage": REPOSITORIO,
        "license": LICENCA,
        "entry": "plugin.wasm",
        "abi": 1,
        "minRustgisVersion": "0.5.0",
        "permissions": ["edit", "files", "network"],
        "commands": comandos,
        "panel": painel,
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
    texto, aviso = sobre(linhas)
    out += [
        "",
        "/// O texto da seção Sobre (o do painel Área de trabalho): autoria, versão, licença, repositório,",
        "/// fontes e como contribuir.",
        f"pub const SOBRE: &str = {s(texto)};",
        "",
        "/// O aviso do botão Sobre: o resumo do [`SOBRE`].",
        f"pub const SOBRE_AVISO: &str = {s(aviso)};",
    ]
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
