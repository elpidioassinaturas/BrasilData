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

O fichas.csv tem a ficha de cada camada que um comando traz (as do fontes.csv e as outras): o que
é, o período, a resolução ou escala, as unidades, a legenda, os cuidados de uso, o campo das classes
do mapa exemplo e a área dele (vicosa, zona-da-mata, minas-gerais, brasil ou mun:<código do IBGE>). As camadas que não
estão no fontes.csv dizem também o órgão, a citação, a licença e se funcionam no app web. Delas saem
a ajuda de cada botão, as fichas do catálogo (o painel e a tabela Fontes) e as páginas
docs/camadas/<id>.md, com o mapa exemplo docs/camadas/<id>.png (scripts/mapas_exemplo.py).
"""

import csv
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
VERSAO = "0.1.78"
AUTOR = "Elpídio Filho"
# Como o texto Sobre chama o autor (o manifesto e o README levam o nome completo).
AUTOR_SOBRE = "Professor Elpídio"
LABORATORIO = "Laboratório de Geoprocessamento e Pedometria (LABGEO)"
INSTITUICAO = "Universidade Federal de Viçosa (UFV)"
LICENCA = "MIT"
REPOSITORIO = "https://github.com/elpidioassinaturas/BrasilData"

# A autoria: os dois primeiros parágrafos do texto Sobre, e o que a janela do botão Sobre mostra
# ao abrir, sem Executar.
AUTORIA = (
    f"BrasilData {VERSAO}, plugin do RustGIS que traz dados públicos do Brasil para a área de trabalho.\n\n"
    f"Autor: {AUTOR_SOBRE}, do {LABORATORIO} da {INSTITUICAO}.\n"
    f"Licença: {LICENCA}, a do código. Os dados são dos órgãos que os publicam, cada um com a sua licença: "
    "o botão Fontes põe a citação e a licença de cada camada do mapa na tabela BrasilData: fontes.\n"
    f"Repositório: {REPOSITORIO}"
)

# As escolhas das janelas dos comandos: o texto que a janela manda é o da escolha, e o
# src/dialogos.rs o traduz no valor do parâmetro (um teste confere que ele conhece todas).
MESES = ["janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro"]
MEDIA_DO_ANO = "média do ano"
SISTEMAS = [
    "Automático: UTM do fuso, ou Albers se a área passa de um fuso",
    "Albers do IBGE (EPSG:10857)",
    "Policônica do Brasil (EPSG:5880)",
]
PERFIS = [
    "Folha A4",
    "Folha A3",
    "Folha A2",
    "Figura de 1 coluna (90 mm)",
    "Figura de 1,5 coluna (140 mm)",
    "Figura de 2 colunas (190 mm)",
    "Tese: página inteira (160 × 225 mm)",
    "Tese: meia página (160 × 110 mm)",
]
CAMADAS_CTREES = ["Biomassa (Mg/ha)", "Incerteza: erro-padrão (Mg/ha)"]
VARIAVEIS_BAM = ["Chuva somada (mm)", "Temperatura média (°C)"]
INDICADORES_GRACE = ["Umidade na zona das raízes (1 m)", "Água subterrânea", "Umidade na superfície do solo"]
INDICADORES_GDO = ["Anomalia da umidade do solo", "Anomalia da vegetação (fAPAR)", "SPI de 1 mês (CHIRPS)", "SPI de 3 meses (CHIRPS)", "SPI de 6 meses (CHIRPS)", "SPI de 12 meses (CHIRPS)"]
ORIENTACOES = ["Automática: a que der o mapa maior", "Retrato (em pé)", "Paisagem (deitada)"]
ESTILOS = ["Clássico", "Faixa", "Painel", "Editorial", "Modular", "Pôster"]
# As escolhas do limite de nuvem das cenas do EMIT (as mesmas de src/dialogos.rs).
NUVENS_EMIT = ["até 10%", "até 30%", "até 50%", "todas as cenas"]
# As escolhas do ano do WorldCover (as mesmas de src/dialogos.rs).
ANOS_WORLDCOVER = ["2021 (o mais recente)", "2020 (algoritmo anterior)"]
PALETAS = ["A do BrasilData: etapa e confiança", "A oficial da OPERA"]
# As escolhas do tema Tempo e clima (iguais às de src/tempo.rs).
MODELOS_PREVISAO = ["ECMWF IFS (15 dias)", "ECMWF AIFS, de IA (15 dias)", "GFS da NOAA (16 dias)", "ICON do DWD (7 dias)", "O melhor do Open-Meteo para o lugar (16 dias)"]
MODELOS_ENSEMBLE = ["ECMWF ENS, 51 membros (15 dias)", "ECMWF AIFS-ENS, de IA, 51 membros (15 dias)", "GEFS da NOAA, 31 membros (16 dias)"]
MEDIDAS_ENSEMBLE = ["Chance de passar do limiar (%)", "Média dos membros (mm)", "Cenário seco: percentil 10 (mm)", "Cenário chuvoso: percentil 90 (mm)"]
TEMPERATURAS = ["Máxima do dia (média dos dias)", "Mínima do dia (média dos dias)", "Média do dia"]
# O resumo da chuva do ERA5 (o mesmo de src/era5.rs, ESCOLHAS_RESUMOS).
RESUMOS_ERA5 = ["Total do período (mm)", "Média por ano (mm por ano)"]
CAMPOS_ERA5 = [
    {"name": "inicio", "label": "Início (dd/mm/aaaa)", "kind": "text", "help": "O primeiro dia do período, desde 01/01/1940. Vazio: 1º de janeiro do ano passado."},
    {"name": "fim", "label": "Fim (dd/mm/aaaa)", "kind": "text", "help": "O último dia do período, até uns 5 dias atrás. Vazio: 31 de dezembro do ano passado. Até 30 anos."},
]
PARAMS_ERA5 = "\"inicio\"?, \"fim\"?: dd/mm/aaaa ou aaaa-mm-dd, desde 01/01/1940, até 30 anos (padrão: o ano passado inteiro), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)"
FIM_ERA5 = " Da reanálise ERA5 do ECMWF (Copernicus Climate Change Service), a 0,25° (cerca de 28 km), desde 1940, pela API histórica do Open-Meteo (sem login): os valores diários vêm nos nós da grade que cercam a área e viram um raster, interpolado (bilinear) para a grade do projeto e recortado pela área. O mapa mostra o clima regional, não o de cada encosta. Num período longo ou numa área grande, a grade de pontos fica mais aberta, até 1°: a API gratuita do Open-Meteo pesa cada ponto pelos dias pedidos (30 anos cabem num município; Minas Gerais inteira, uns 500 dias). Acima disso, o comando diz o período que cabe. O ERA5 sai com uns 5 dias de atraso. Sem período, o ano passado inteiro. A reanálise é um modelo ajustado às observações, não a medida de uma estação. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0)."
# A umidade do solo nas raízes e a evapotranspiração do NASA POWER (as mesmas escolhas de src/power.rs).
MEDIDAS_POWER = ["Média de 2001 a 2025", "O valor de um mês ou ano", "Anomalia de um mês ou ano"]
CAMPOS_POWER = [
    {"name": "medida", "label": "O que mostrar", "kind": "choice", "required": True, "choices": MEDIDAS_POWER, "default": MEDIDAS_POWER[0], "help": "A média do período de referência, o valor de um mês ou ano, ou a diferença entre ele e a média do mesmo mês ou ano."},
    {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2025, "help": "De 1981 em diante, para o valor e a anomalia (na média de 2001 a 2025, não conta). Os meses saem com alguns meses de atraso; o ano, só completo."},
    {"name": "mes", "label": "Período", "kind": "choice", "required": True, "choices": [MEDIA_DO_ANO] + MESES, "default": MEDIA_DO_ANO, "help": "O ano inteiro ou um mês."},
]
PARAMS_POWER = "{\"medida\"?: \"media\" | \"valor\" | \"anomalia\" (padrão: media; com o ano, valor), \"ano\"?: de 1981 em diante (no valor e na anomalia), \"mes\"?: de 1 a 12 (padrão: o ano), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}"
# Curta de propósito: com três campos, uma ajuda (já com o resumo da ficha) de 1000 caracteres tira o
# último campo da caixa do RustGIS 0.8.0; com 850, os três cabem.
FIM_POWER = " Do NASA POWER (reanálise MERRA-2), só em terra: serve para o país, uma região ou um estado, e não para medir tendências. Sem login."
# As escolhas do OpenLandMap-soildb e do GEDTM30 (as mesmas de src/dialogos.rs).
PROPRIEDADES_OLM = ["Carbono orgânico (g/kg)", "Densidade de carbono (kg/m³)", "pH em água", "Densidade do solo (g/cm³)", "Argila (%)", "Areia (%)", "Silte (%)"]
PROFUNDIDADES_OLM = ["0 a 30 cm", "30 a 60 cm", "60 a 100 cm"]
ATRIBUTOS_SOLO = ["Argila (g/kg)", "Areia (g/kg)", "Silte (g/kg)", "Carbono orgânico (g/kg)", "CTC (cmolc/kg)", "pH", "Densidade do solo (g/cm³)", "Classe do solo (ordem do SiBCS, um ponto por perfil)"]
PROFUNDIDADES_SOLO = ["5 cm", "10 cm", "30 cm", "50 cm", "100 cm"]
DADOS_MBSOLO = ["Carbono orgânico: estoque de 0 a 30 cm (t/ha)", "Argila (g/kg)", "Areia (g/kg)", "Silte (g/kg)", "Fração grossa: cascalho e calhaus (g/kg)"]
PROFUNDIDADES_MBSOLO = ["5 cm", "10 cm", "30 cm", "50 cm", "90 cm"]
PERIODOS_OLM = ["2020 a 2022", "2015 a 2020 (só carbono e pH)", "2010 a 2015 (só carbono e pH)", "2005 a 2010 (só carbono e pH)", "2000 a 2005 (só carbono e pH)"]
VARIAVEIS_GEDTM = [
    "Elevação do terreno (m)",
    "Declividade (graus)",
    "Índice topográfico de umidade (TWI)",
    "Diferença da elevação média em volta (m)",
    "Rugosidade do terreno (graus)",
    "Curvatura mínima",
    "Curvatura máxima",
    "Curvatura de perfil",
    "Curvatura tangencial",
    "Curvatura anelar (240 m ou mais)",
    "Índice de forma (480 m ou mais)",
    "Abertura positiva (graus, 480 m ou mais)",
    "Abertura negativa (graus, 480 m ou mais)",
    "Sombreamento",
    "Geomorfons (formas do relevo)",
    "Fator LS (erosão)",
]
ESCALAS_GEDTM = ["30 m", "60 m", "120 m", "240 m", "480 m", "960 m"]
MODELOS_GEM15 = ["Terreno nu (DTM)", "Superfície, com vegetação e construções (DSM)", "Altura acima do terreno (DSM menos DTM)"]
# As escolhas do cubo de dados do INPE (as mesmas de src/dialogos.rs).
SATELITES_BDC = ["Sentinel-2 (10 m, desde 2017)", "Landsat (30 m, desde 1990)", "CBERS-4 WFI (64 m, desde 2016)", "MODIS (250 m, desde 2000)"]
INDICES_BDC = ["NDVI", "EVI"]
ESTATISTICAS_BDC = ["Média das datas", "Máxima", "Mínima", "Desvio-padrão"]
# As escolhas da imagem sem nuvens (as mesmas de src/dialogos.rs): sem o MODIS, que não tem as bandas de cor.
SATELITES_SEM_NUVENS = SATELITES_BDC[:3]
COMPOSICOES = ["Cor verdadeira (vermelho, verde e azul)", "Falsa-cor (infravermelho, vermelho e verde)"]
# As partes do texto Sobre (as mesmas de src/dialogos.rs).
# As abas Sobre do fim do painel: o comando de cada uma e o rótulo (os mesmos de src/lib.rs, ABAS_SOBRE).
ABAS_SOBRE = [("sobre-autoria", "Autoria"), ("sobre-fontes", "Fontes dos dados"), ("sobre-contribuir", "Como contribuir")]
# As escolhas do turno da temperatura do JAXA (as mesmas de src/dialogos.rs).
TURNOS_JAXA = ["De dia (por volta das 10h30)", "À noite (por volta das 22h30)"]
CAMPOS_JAXA = [
    {"name": "inicio", "label": "Início (dd/mm/aaaa)", "kind": "text", "help": "O primeiro dia do período (vale o mês dele). Vazio: os 3 meses até o último com dados."},
    {"name": "fim", "label": "Fim (dd/mm/aaaa)", "kind": "text", "help": "O último dia do período. Vazio: o fim do último mês com dados. Até 5 anos."},
    {"name": "estatistica", "label": "Resumo dos meses", "kind": "choice", "required": True, "choices": ESTATISTICAS_BDC, "default": ESTATISTICAS_BDC[0], "help": "A média dos meses do período, a máxima, a mínima ou o desvio-padrão (o quanto variou)."},
]
PARAMS_JAXA = "\"inicio\"?, \"fim\"?: dd/mm/aaaa ou aaaa-mm-dd (padrão: os 3 meses até o último com dados), \"estatistica\"?: \"media\" (padrão) | \"maxima\" | \"minima\" | \"desvio\""
FIM_JAXA = " Do JAXA Earth (a agência espacial do Japão), por mês, lido só na área pelo RustGIS (sem login) e recortado pelo contorno, na resolução original do dado (células de 2,8 a 22 km, em graus), fora da grade do projeto: o mapa mostra o padrão regional. Sem período, os 3 meses até o último com dados. Uso livre com o crédito \"JAXA\"; uso comercial, avisando a JAXA antes. Precisa de um RustGIS com o resumo de um cubo STAC (cube.reduce, desde a 0.6.0); só no app desktop e no rustgis-cli."
CAMPOS_BDC = [
    {"name": "satelite", "label": "Satélite", "kind": "choice", "required": True, "choices": SATELITES_BDC, "default": SATELITES_BDC[0], "help": "O cubo de 16 dias do INPE: o Sentinel-2 tem o detalhe (10 m) e começa em 2017; o Landsat vai até 1990, a 30 m; o CBERS-4 WFI, satélite sino-brasileiro, a 64 m; o MODIS, a 250 m, desde 2000, para áreas grandes."},
    {"name": "indice", "label": "Índice", "kind": "choice", "required": True, "choices": INDICES_BDC, "default": INDICES_BDC[0], "help": "O NDVI mede o verde da vegetação; o EVI satura menos nas florestas densas."},
    {"name": "inicio", "label": "Início (dd/mm/aaaa)", "kind": "text", "help": "A primeira data. Vazio: 3 meses antes do fim."},
    {"name": "fim", "label": "Fim (dd/mm/aaaa)", "kind": "text", "help": "A última data. Vazio: a mais recente do cubo. O período vai de 16 dias a 3 anos."},
]

# Os comandos que não vêm da tabela, com o grupo da aba em que ficam. Na aba, o grupo Área
# tem o painel (a escolha do território pelo nome), a área pela seleção e o desenho; o comando
# area-municipio fica para scripts e para a linha de comando.
AREA = [
    {
        "id": "area-selecao",
        "label": "Área:\nseleção",
        "help": "Torna a área do BrasilData os polígonos selecionados da camada ativa (por exemplo, um município trazido por Municípios).",
        "icon": "select_loc",
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
        "help": "Torna um município a área do BrasilData, com o limite do IBGE que vem no plugin: o nome ou o código vêm da janela do comando (pela Busca de comandos). A árvore Limites políticos, no painel Área de trabalho, faz o mesmo.",
        "icon": "locate",
        "params": "{\"nome\": nome do município (\"Teresina\", \"Bom Jesus/PI\"), \"uf\"?: sigla, quando o nome se repete} | {\"codigo\": código do IBGE, 7 dígitos}",
        "form": [
            {"name": "nome", "label": "Município", "kind": "text", "required": True, "help": "O nome do município (Viçosa, Bom Jesus/PI) ou o código do IBGE, de 7 dígitos (3171303). Ele vira a área de trabalho, com o limite do IBGE que vem no plugin."},
            {"name": "uf", "label": "UF", "kind": "text", "help": "A sigla do estado (MG), quando o nome se repete em mais de um."},
        ],
    },
    {
        "id": "grade",
        "label": "Grade do\nprojeto",
        "help": "A grade em que os rasters do BrasilData chegam (a Elevação, e depois os temas rasterizados): o sistema de coordenadas e o tamanho das células, no retângulo da área, como a camada BrasilData: grade. A célula e o sistema vêm da janela do comando; num RustGIS sem janelas de plugin, do painel Área de trabalho. Rasters na mesma grade têm as mesmas linhas e colunas. Uma área nova refaz a grade: o sistema e a célula escolhidos ficam, e a célula padrão dá lugar à da área nova.",
        "icon": "resample",
        "params": "{\"celula\"?: tamanho da célula em metros, de 1 a 10.000 (padrão: com crs, o da área; sem ele, o do painel, ou 30), \"crs\"?: código EPSG em metros (31983, \"EPSG:5880\") ou \"automatico\" (UTM do fuso, ou o Albers se a área passa de um fuso), \"policonica\" (que o RustGIS 0.5.0 ainda não projeta) ou \"albers\" (EPSG:10857, o do IBGE)}",
        "form": [
            {"name": "celula", "label": "Célula (m; vazio: a padrão)", "kind": "number", "help": "O lado das células, em metros, de 1 a 10.000: 30 m é o do Copernicus e do Landsat; 1.000 m serve a um estado inteiro. Vazio: a padrão da área, de 30 m num município a 500 m no Brasil (ou maior, com a extensão Imagens do RustGIS desligada)."},
            {"name": "crs", "label": "Sistema", "kind": "choice", "required": True, "choices": SISTEMAS, "default": SISTEMAS[0], "help": "O sistema de coordenadas da grade. O automático escolhe o UTM SIRGAS 2000 do fuso da área, ou o Albers do IBGE quando ela passa de um fuso; o Albers preserva as áreas; a Policônica é a do mapa do Brasil (o RustGIS 0.5.0 ainda não a projeta)."},
            {"name": "epsg", "label": "Outro sistema (EPSG)", "kind": "text", "help": "Um código EPSG em metros (31983 é o UTM 23S SIRGAS 2000). Preenchido, vale no lugar do sistema acima."},
        ],
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
        "icon": "map",
        "params": "{}",
    },
    {
        "id": "escolher-area",
        "label": "Escolher área",
        "help": "O nó escolhido na árvore Limites políticos vira a área: o Brasil, uma grande região, um estado, uma mesorregião, uma microrregião ou um município, com o limite do IBGE que vem no plugin.",
        "icon": "locate",
        "params": "{\"no\": \"br\" | \"reg:2\" | \"uf:PI\" | \"meso:2204\" | \"micro:22010\" | \"mun:2211001\"} (por script), ou o nó escolhido na árvore limites do painel",
    },
]
ARVORE = "Clique no Brasil, numa grande região, estado, mesorregião, microrregião ou município (divisão do IBGE de 1989): ele vira a área de trabalho. O triângulo abre o ramo, e a caixa busca pelo nome. Os municípios aparecem quando você escolhe a grande região deles, ou qualquer lugar dentro dela: o RustGIS guarda até 5.000 nós por árvore, e o Brasil tem 5.570 municípios. Os limites vêm no plugin (IBGE), sem depender da rede; para o contorno exato, traga a camada de Limites e use Área: seleção."
PAINEL = {
    "title": "Área de trabalho",
    "icon": "map",
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
            "help": "O tamanho das células da grade, de 10 m a 1 km. Vale quando você clicar em Grade do projeto num RustGIS sem janelas de plugin; com elas, a janela do comando tem a célula.",
            "min": 0,
            "max": 8,
            "default": 2,
            "steps": True,
        },
        {
            "kind": "slider",
            "id": "grade-sistema",
            "label": "Grade: sistema",
            "help": "O sistema de coordenadas da grade: o automático (UTM SIRGAS 2000 do fuso da área, ou o Albers do IBGE quando a área passa de um fuso), a Policônica do Brasil (EPSG:5880, que o RustGIS 0.5.0 ainda não projeta) ou o Albers do IBGE (EPSG:10857), que preserva as áreas. Vale quando você clicar em Grade do projeto num RustGIS sem janelas de plugin; com elas, a janela do comando tem o sistema.",
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
            "help": "O ano do mapa de uso e cobertura do MapBiomas (Coleção 11), de 1985 a 2025. Vale quando você clicar em Uso e cobertura (MapBiomas), aqui ou na aba, num RustGIS sem janelas de plugin; com elas, a janela do comando tem o ano.",
            "min": 1985,
            "max": 2025,
            "default": 2025,
            "steps": True,
        },
        {"kind": "button", "command": "mapbiomas-cobertura"},
        {
            "kind": "slider",
            "id": "risco-fogo-mes",
            "label": "Risco de fogo: mês",
            "help": "O mês da climatologia do Risco de Fogo do INPE (a média de 2001 a 2025 desse mês). Vale quando você clicar em Risco de fogo (INPE), aqui ou na aba Queimadas, num RustGIS sem janelas de plugin; com elas, a janela do comando tem o mês.",
            "min": 1,
            "max": 12,
            "default": 9,
            "steps": True,
        },
        {"kind": "button", "command": "risco-fogo"},
        {
            "kind": "text",
            "id": "amostra-texto",
            "label": "Amostragem de campo",
            "text": "Pontos para ir a campo, sorteados dentro da área: aleatória simples, estratificada (pelas classes de uma camada de polígonos, como Solos: clique nela no painel Camadas antes) ou sistemática (uma grade). O mesmo sorteio dá os mesmos pontos.",
        },
        {
            "kind": "slider",
            "id": "amostra-pontos",
            "label": "Amostragem: pontos",
            "help": "Quantos pontos sortear, de 5 a 500, na aleatória simples e na estratificada. Por script, até 2.000.",
            "min": 0,
            "max": 13,
            "default": 5,
            "steps": True,
        },
        {
            "kind": "slider",
            "id": "amostra-espacamento",
            "label": "Amostragem: espaçamento",
            "help": "A distância entre os pontos da amostragem sistemática, de 50 m a 10 km.",
            "min": 0,
            "max": 8,
            "default": 4,
            "steps": True,
        },
        {
            "kind": "slider",
            "id": "amostra-sorteio",
            "label": "Amostragem: sorteio",
            "help": "O número do sorteio. O mesmo número, na mesma área e com o mesmo método, dá os mesmos pontos; outro número dá outra amostra.",
            "min": 1,
            "max": 20,
            "default": 1,
            "steps": True,
        },
        {"kind": "button", "command": "amostra-aleatoria"},
        {"kind": "button", "command": "amostra-estratificada"},
        {"kind": "button", "command": "amostra-sistematica"},
    ],
}

# A amostragem de campo, num menu do grupo Área (e com os controles no painel).
AMOSTRAGEM = [
    {
        "id": "amostra-aleatoria",
        "label": "Aleatória simples",
        "help": "Amostragem aleatória simples: pontos para ir a campo, sorteados dentro da área de trabalho, todos com a mesma chance, na camada BrasilData: amostragem aleatória (rótulo, método, sorteio, longitude e latitude SIRGAS 2000). O número de pontos, o sorteio e o arquivo de pontos vêm da janela do comando; num RustGIS sem janelas de plugin, do painel Área de trabalho.",
        "icon": "hull",
        "params": "{\"pontos\"?: de 1 a 500 (padrão: o do painel, ou 30), \"sorteio\"?: número a partir de 1 (padrão: o do painel, ou 1), \"arquivo\"?: arquivo em que os pontos são gravados (.gpkg, .shp, .geojson, .csv, .kml, .gpx; só o nome vai para a pasta do projeto)}",
        "form": [
            {"name": "pontos", "label": "Número de pontos", "kind": "number", "required": True, "default": 30, "help": "De 1 a 500 pontos, sorteados dentro da área de trabalho."},
            {"name": "sorteio", "label": "Sorteio", "kind": "number", "default": 1, "help": "O número do sorteio, a partir de 1. O mesmo número, na mesma área e com o mesmo método, dá os mesmos pontos; outro número dá outra amostra."},
            {"name": "arquivo", "label": "Arquivo de pontos", "kind": "text", "default": "amostragem_aleatoria.gpkg", "help": "O arquivo em que os pontos são gravados: só o nome vai para a pasta do projeto; também aceita um caminho completo. Formatos: .gpkg (o padrão), .shp, .geojson, .csv, .kml ou .gpx (para o GPS). Vazio: os pontos ficam só no mapa."},
        ],
    },
    {
        "id": "amostra-estratificada",
        "label": "Estratificada",
        "help": "Amostragem aleatória estratificada: os estratos são as classes de uma camada de polígonos (clique antes nela no painel Camadas, por exemplo BrasilData: Solos). Cada estrato recebe pontos na proporção da sua área dentro da área de trabalho, com pelo menos um, e os pontos caem ao acaso dentro dele. Na janela do comando ficam a camada e o campo dos estratos, o número de pontos, o mínimo por estrato, o sorteio e o arquivo de pontos; sem janela, o campo é o que tem menos classes (de 2 a 30).",
        "icon": "dissolve",
        "params": "{\"pontos\"?: de 1 a 500, \"sorteio\"?: número a partir de 1, \"camada\"?: a camada dos estratos (padrão: a ativa), \"campo\"?: o campo das classes (padrão: o de menos classes, de 2 a 30), \"minimo\"?: pontos por estrato (padrão 1), \"arquivo\"?: arquivo em que os pontos são gravados (.gpkg, .shp, .geojson, .csv, .kml, .gpx; só o nome vai para a pasta do projeto)}",
        "form": [
            {"name": "camada", "label": "Camada dos estratos", "kind": "polygonLayer", "required": True, "help": "Os polígonos que dividem a área em estratos, como BrasilData: Solos ou a camada das classes do MapBiomas."},
            {"name": "campo", "label": "Campo das classes", "kind": "field", "help": "O campo que dá o estrato de cada polígono. Vazio: o que tem menos classes, de 2 a 30."},
            {"name": "pontos", "label": "Número de pontos", "kind": "number", "required": True, "default": 30, "help": "De 1 a 500 pontos no total, repartidos entre os estratos na proporção da área de cada um."},
            {"name": "minimo", "label": "Mínimo por estrato", "kind": "number", "default": 1, "help": "Quantos pontos cada estrato recebe pelo menos, mesmo os pequenos."},
            {"name": "sorteio", "label": "Sorteio", "kind": "number", "default": 1, "help": "O número do sorteio, a partir de 1. O mesmo número, na mesma área e com o mesmo método, dá os mesmos pontos; outro número dá outra amostra."},
            {"name": "arquivo", "label": "Arquivo de pontos", "kind": "text", "default": "amostragem_estratificada.gpkg", "help": "O arquivo em que os pontos são gravados: só o nome vai para a pasta do projeto; também aceita um caminho completo. Formatos: .gpkg (o padrão), .shp, .geojson, .csv, .kml ou .gpx (para o GPS). Vazio: os pontos ficam só no mapa."},
        ],
    },
    {
        "id": "amostra-sistematica",
        "label": "Sistemática (grade)",
        "help": "Amostragem sistemática: uma grade de pontos dentro da área de trabalho, com o espaçamento escolhido e a origem sorteada, numerada em zigue-zague de norte a sul, na camada BrasilData: amostragem sistemática. O espaçamento, o sorteio e o arquivo de pontos vêm da janela do comando; num RustGIS sem janelas de plugin, do painel.",
        "icon": "graticule",
        "params": "{\"espacamento\"?: metros, de 1 a 100.000 (padrão: o do painel, ou 500), \"sorteio\"?: número a partir de 1 (a origem da grade), \"arquivo\"?: arquivo em que os pontos são gravados (.gpkg, .shp, .geojson, .csv, .kml, .gpx; só o nome vai para a pasta do projeto)}",
        "form": [
            {"name": "espacamento", "label": "Espaçamento (m)", "kind": "number", "required": True, "default": 500, "help": "A distância entre os pontos da grade, em metros, de 1 a 100.000. A grade tem no máximo 2.000 pontos no retângulo da área."},
            {"name": "sorteio", "label": "Sorteio", "kind": "number", "default": 1, "help": "O número do sorteio, a partir de 1: ele sorteia a origem da grade. O mesmo número, na mesma área, dá a mesma grade."},
            {"name": "arquivo", "label": "Arquivo de pontos", "kind": "text", "default": "amostragem_sistematica.gpkg", "help": "O arquivo em que os pontos são gravados: só o nome vai para a pasta do projeto; também aceita um caminho completo. Formatos: .gpkg (o padrão), .shp, .geojson, .csv, .kml ou .gpx (para o GPS). Vazio: os pontos ficam só no mapa."},
        ],
    },
]
# Os comandos que vão no fim de um grupo da aba: as fontes, com a área.
FIM = [
    (
        "Área",
        {
            "id": "layout",
            "label": "Layout",
            "help": "Uma folha pronta para exportar, com o dado principal do BrasilData no mapa (a camada visível mais acima): o nome da área no título, o que é o dado no subtítulo, a barra de cores com a unidade ou a legenda das classes, as fontes, a projeção da grade do projeto, a grade em metros e a localização. O mapa passa para a projeção da grade, a área fica só com o contorno e a grade sai da vista. O formato (folha ou figura), a orientação e o estilo da folha vêm da janela do comando. Um novo clique refaz a folha.",
            "icon": "new_layout",
            "params": "{\"perfil\"?: a4 (padrão), a3, a2, a4-portrait, a4-landscape... ou figure-1col, figure-2col, thesis-full, \"orientacao\"?: automatica (padrão), retrato ou paisagem (só nas folhas), \"estilo\"?: classic (padrão), banner, panel, editorial, modular ou poster}",
            "form": [
                {"name": "perfil", "label": "Formato", "kind": "choice", "required": True, "choices": PERFIS, "default": PERFIS[0], "help": "Uma folha (A4, A3, A2) com título, fontes e localização; ou uma figura para artigo ou tese, só o mapa, com o título e a fonte para a legenda da figura."},
                {"name": "orientacao", "label": "Orientação da folha", "kind": "choice", "required": True, "choices": ORIENTACOES, "default": ORIENTACOES[0], "help": "A folha em pé (retrato) ou deitada (paisagem); na automática, a que der o mapa maior. As figuras de artigo e de tese têm o tamanho fixo e não usam."},
                {"name": "estilo", "label": "Estilo da folha", "kind": "choice", "required": True, "choices": ESTILOS, "default": ESTILOS[0], "help": "A arrumação da folha: o título sobre o mapa (clássico), numa faixa colorida, num painel ao lado, em letras grandes (editorial), em cartões (modular) ou numa página colorida (pôster). As figuras não usam."},
            ],
        },
    ),
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
            "icon": "redo",
            "params": "{}",
        },
    ),
    (
        "Área",
        {
            "id": "sobre",
            "label": "Sobre",
            "help": AUTORIA + "\n\nAs fontes dos dados e como contribuir estão nas abas do fim do painel Área de trabalho: basta clicar nelas. Para lê-las aqui, marque a caixa e clique em Executar. A janela fica aberta até o X do canto de cima.",
            "icon": "about",
            "params": "{\"completo\"?: true para as fontes e como contribuir | \"parte\"?: \"tudo\" (padrão) | \"autoria\" | \"fontes\" | \"contribuir\"}",
            "form": [
                {"name": "completo", "label": "Mostrar também as fontes e como contribuir", "kind": "bool", "default": False, "help": "Marcada, Executar mostra as fontes dos dados e como contribuir; desmarcada, a autoria."},
            ],
        },
    ),
]

# A seção Sobre: a autoria e o resto, no fim do painel (o RustGIS não tem janela de texto para um
# plugin, e um comando não abre o painel) e, resumida, no aviso do botão Sobre. O autor, a
# licença e o repositório são os do manifesto.
# As fontes fora do fontes.csv, depois dos órgãos das camadas vetoriais.
OUTRAS_FONTES = [
    "Copernicus DEM GLO-30 (DLR e Airbus, pelo Planetary Computer)",
    "Banco Central (Pix e crédito rural)",
    "TSE",
    "Novo CAGED (IPEA Data)",
    "Ministério da Saúde (CNES, pelo geobr do Ipea)",
    "Open-Meteo (previsão do ECMWF, da NOAA e do DWD)",
    "CHIRPS v3 (Climate Hazards Center, UCSB)",
    "Copernicus Land Monitoring Service (pelo Copernicus Data Space)",
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
            AUTORIA,
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
        f"BrasilData {VERSAO}, do {AUTOR_SOBRE} (LABGEO/UFV), sob a licença {LICENCA}: {REPOSITORIO}. A autoria, as fontes "
        "dos dados e como contribuir estão nas abas Sobre, no fim do painel Área de trabalho."
    )
    if len(texto) > MAX_SOBRE:
        sys.exit(f"o texto Sobre tem {len(texto)} caracteres; o RustGIS aceita até {MAX_SOBRE}")
    if len(aviso) > MAX_AVISO:
        sys.exit(f"o aviso Sobre tem {len(aviso)} caracteres; o RustGIS corta em {MAX_AVISO}")
    return texto, aviso


# Comandos que não vêm da tabela e ficam num grupo da aba: (tópico, subtópico ou None, comando).
# Os comandos cujos dados pedem o login Earthdata: o RustGIS (desde o #2378) desenha uma chave no
# botão, verde com o token guardado e vermelha sem ele; os anteriores ignoram o campo "login".
LOGIN_EARTHDATA = {
    "gedi-altura", "gedi-biomassa", "gedi-cobertura",
    "radar-vv", "radar-vh", "radar-razao",
    "nisar-hh", "nisar-hv", "nisar-razao", "nisar-cor", "nisar-tempo",
    "dist-situacao", "dist-data", "dist-confianca", "dist-intensidade",
}

# Os que pedem o do Copernicus Data Space.
LOGIN_COPERNICUS = {"ndvi-copernicus"}

EM_MENUS = [
    (
        "Sociedade e economia",
        "População",
        {
            "id": "censo-setores",
            "label": "Setores censitários (IBGE)",
            "help": "Os setores censitários de 2022 da área, com pessoas e domicílios (IBGE, agregados por setores): o GeoPackage de cada UF, recortado pela área, com os campos v0001 a v0007 do IBGE renomeados (pessoas, domicilios, dom_particulares, dom_coletivos, media_moradores, prop_imputados, dom_part_ocupados; o código de cada um fica na tabela Fontes). No app web, um segundo clique recorta.",
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
            "help": "População, área e densidade de 2022 (Censo Demográfico) e estabelecimentos agropecuários e agricultura familiar de 2017 (Censo Agropecuário), do IBGE. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "indicadores-pix",
            "label": "Pix (Banco Central)",
            "help": "O Pix do último mês completo: valor pago e recebido (R$), transações pagas e pessoas físicas pagadoras. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "indicadores-credito",
            "label": "Crédito rural de custeio (Banco Central)",
            "help": "O custeio do último ano completo, do SICOR: total, Pronaf, agrícola e pecuário (R$) e área (ha). O SICOR é lento: cerca de 10 s para cada 6 municípios. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "censo-agro",
            "label": "Censo Agropecuário 2017 (IBGE)",
            "help": "Do Censo Agropecuário de 2017 (IBGE): estabelecimentos e agricultura familiar, utilização das terras (lavouras, pastagens e matas, ha), bovinos, pessoal ocupado, tratores, irrigação, uso de agrotóxicos, energia elétrica, orientação técnica e valor da produção (mil R$). Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem; o painel Fontes diz o que é cada campo.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Agricultura",
        {
            "id": "censo-agroecologia",
            "label": "Agroecologia (Censo Agropecuário 2017)",
            "help": "Práticas agroecológicas do Censo Agropecuário de 2017 (IBGE), em número de estabelecimentos: agricultura ou pecuária orgânica, associação a cooperativa ou entidade, práticas de conservação (plantio em nível, rotação de culturas, pousio, proteção de encostas, recuperação de mata ciliar, reflorestamento de nascentes, manejo florestal), plantio direto (e sua área), cultivo mínimo, adubação orgânica ou química, quem não usou agrotóxicos e a área de sistemas agroflorestais (ha). Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem; o painel Fontes diz o que é cada campo.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Eleições",
        {
            "id": "indicadores-tse",
            "label": "Eleitorado e comparecimento (TSE)",
            "help": "Eleitorado, comparecimento e abstenção (%) no 1º turno de 2026 e de 2024, da divulgação oficial do TSE. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "indicadores-caged",
            "label": "Emprego formal (Novo CAGED)",
            "help": "Admissões, desligamentos e saldo nos últimos 12 meses e o saldo do último mês, do Novo CAGED (IPEA Data), num retrato que vem com o plugin. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        None,
        {
            "id": "indicadores-todos",
            "label": "Todos os indicadores",
            "help": "Censos, Censo Agropecuário, agroecologia, Pix, crédito rural, eleições, emprego formal e saúde, de uma vez. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem. No desktop, se o tempo de uma chamada acabar, o aviso pede um novo clique, que continua de onde parou.",
            "icon": "stats",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Saúde",
        {
            "id": "saude-estabelecimentos",
            "label": "Estabelecimentos de saúde (CNES, DATASUS)",
            "help": "Os estabelecimentos de saúde ativos na área, do CNES do Ministério da Saúde (competência 04/2026, pelo geobr do Ipea), com nome, tipo, natureza jurídica, gestão e bairro. Os de pessoa física e de empresário individual, cujo nome é o de uma pessoa, ficam de fora. Só no desktop: o primeiro clique abre o arquivo do Brasil (98 MB, lido aos pedaços), o segundo recorta pela área.",
            "icon": "point_layer",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Saúde",
        {
            "id": "indicadores-saude",
            "label": "Indicadores de saúde (DATASUS)",
            "help": "Indicadores de saúde por município, do DATASUS, num retrato que vem com o plugin: nascidos vivos e, entre eles, a parte com baixo peso, por cesárea, de mãe adolescente e com 7 ou mais consultas de pré-natal (SINASC); óbitos, a taxa de mortalidade por mil habitantes e a parte por causas externas, doenças do aparelho circulatório, neoplasias e doenças respiratórias (SIM); a mortalidade infantil por mil nascidos vivos, em 3 anos; e os leitos existentes e do SUS do CNES, por mil habitantes. Por local de residência. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem; o layout faz o mapa da mortalidade infantil.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "mapbiomas-cobertura",
            "label": "Uso e cobertura (MapBiomas)",
            "help": "O uso e a cobertura da terra do MapBiomas, com as cores da legenda oficial. O ano, de 1985 a 2025, vem da janela do comando; num RustGIS sem janelas de plugin, do controle MapBiomas: ano, no painel Área de trabalho (2025 até você mudar). Com uma área, de 2019 a 2024 e num RustGIS depois da 0.5.0, vem o raster das classes (Coleção 10, 30 m): só a área, recortada pelo contorno e posta na grade do projeto. Nos outros casos, um mapa para ver, do Brasil inteiro (Coleção 11), que vale por algumas horas. Cada ano vira uma camada.",
            "icon": "unique",
            "params": "{\"ano\"?: de 1985 a 2025 (padrão: o do painel, ou 2025), \"recorte\"?: false para o mapa do Brasil inteiro mesmo com área}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2025, "help": "O ano do mapa, de 1985 a 2025."},
                {"name": "recorte", "label": "Só a área (raster das classes)", "kind": "bool", "default": True, "help": "Marcado, com uma área e um ano de 2019 a 2024 num RustGIS depois da 0.5.0: o raster das classes, só a área, posto na grade do projeto. Desmarcado: o mapa do Brasil inteiro, para ver."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "clms-cobertura",
            "label": "Cobertura da terra 100 m (Copernicus)",
            "help": "A cobertura da terra global do serviço de terra do Copernicus (CLMS Land Cover 100 m, coleção 3, feita com o satélite PROBA-V), de 2015 a 2019, com as cores e os nomes da legenda oficial: florestas fechadas e abertas, arbustos, vegetação herbácea, cultivos, área urbana, água e outras, 23 classes. Só a área: a janela lida da cópia oficial no Zenodo (sem login, licença CC BY 4.0), recortada pelo contorno e posta na grade do projeto com a classe mais frequente. As células do arquivo têm cerca de 100 m: numa grade mais fina, cada uma se repete. Áreas até uns 7° × 7° (um estado médio). Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área). Funciona no desktop e no app web.",
            "icon": "unique",
            "params": "{\"ano\"?: de 2015 a 2019 (padrão 2019)}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2019, "help": "O ano do mapa, de 2015 a 2019 (2015 é a base; 2016 a 2018, consolidados; 2019, quase em tempo real)."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "worldcover",
            "label": "Cobertura da terra a 10 m (ESA WorldCover)",
            "help": "A cobertura da terra do ESA WorldCover a 10 m, de 2020 ou 2021, com as cores da legenda oficial: árvores, arbustos, campo e pastagem, cultivos, área construída, solo exposto, água, área úmida e manguezal. Um mapa mundial, feito com o Sentinel-1 e o Sentinel-2 por um consórcio liderado pela VITO (licença CC BY 4.0): serve também para áreas que passam da fronteira. Só a área, lida de cada bloco de 3° que ela toca no Planetary Computer da Microsoft, sem login, posta na grade do projeto com a classe mais frequente e recortada pelo contorno; sem grade no mapa, cria uma de 10 m; numa grade mais grossa, lida da visão reduzida do arquivo. Os dois anos usam versões diferentes do algoritmo: a diferença entre eles não é só mudança do terreno (para isso, o MapBiomas). Pastagem e campo nativo são a mesma classe; cultivos são só os anuais (as lavouras perenes lenhosas, como o café, entram em árvores ou arbustos). Até 24 blocos por clique: um município, uma região ou um estado médio. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área); só no app desktop e no rustgis-cli.",
            "icon": "unique",
            "params": "{\"ano\"?: 2020 ou 2021 (padrão 2021), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 10)}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "choice", "required": True, "choices": ANOS_WORLDCOVER, "default": ANOS_WORLDCOVER[0], "help": "O ano do mapa. O de 2021 é a versão 200 do algoritmo e o de 2020 a 100: não subtraia um do outro."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Cubo de dados do INPE",
        {
            "id": "bdc-indice",
            "label": "Índice de vegetação (cubo do INPE)",
            "help": "O NDVI ou o EVI do cubo de dados do INPE (Brazil Data Cube), dos compostos de 16 dias do Sentinel-2, do Landsat, do CBERS-4 WFI ou do MODIS, resumido num período: a média das datas, a máxima, a mínima ou o desvio-padrão. Os compostos já tiram as nuvens (o pixel mais limpo de cada 16 dias). Só a área, lida pelo RustGIS da coleção no STAC do INPE (sem login), posta na grade do projeto e recortada pelo contorno. Sem período, os 3 meses até a data mais recente do cubo. Precisa de um RustGIS com o resumo de um cubo STAC (cube.reduce, desde a 0.6.0); só no app desktop e no rustgis-cli.",
            "icon": "dra",
            "params": "{\"satelite\"?: \"sentinel-2\" (padrão) | \"landsat\" | \"cbers-4\" | \"modis\", \"indice\"?: \"NDVI\" (padrão) | \"EVI\", \"inicio\"?, \"fim\"?: dd/mm/aaaa ou aaaa-mm-dd (padrão: os 3 meses até a data mais recente), \"estatistica\"?: \"media\" (padrão) | \"maxima\" | \"minima\" | \"desvio\", \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": CAMPOS_BDC + [
                {"name": "estatistica", "label": "Resumo das datas", "kind": "choice", "required": True, "choices": ESTATISTICAS_BDC, "default": ESTATISTICAS_BDC[0], "help": "A média mostra a vegetação típica do período; a máxima, o auge do verde; a mínima, a seca; o desvio-padrão, o quanto ela variou (a lavoura muda muito; a mata, pouco)."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Cubo de dados do INPE",
        {
            "id": "bdc-sem-nuvens",
            "label": "Imagem sem nuvens (cubo do INPE)",
            "help": "Uma imagem de satélite sem nuvens da área, em cor verdadeira ou em falsa-cor (a vegetação em vermelho), dos compostos de 16 dias do Sentinel-2, do Landsat ou do CBERS-4 WFI do cubo de dados do INPE (Brazil Data Cube). Os compostos ainda têm nuvem onde a quinzena toda foi nublada: o plugin lê a máscara de nuvem de cada data na área (o SCL do Sentinel-2, as observações limpas do Landsat, o CMASK do CBERS), a cresce de 1 ou 2 células, ordena as datas da menos nublada para a mais nublada e dá a cada célula a primeira data limpa. Vêm a imagem, recortada pelo contorno, com um contraste fixo, e a data de origem de cada célula. Lida pelo RustGIS da coleção no STAC do INPE (sem login), na resolução do satélite, no sistema da grade do projeto. Sem período, os 3 meses até a data mais recente do cubo; até 1 ano. Precisa de um RustGIS com o resumo de um cubo STAC (cube.reduce, desde a 0.6.0) e das ferramentas Raster Calculator, Maximum Filter e Composite Bands do GeoLibre; só no app desktop e no rustgis-cli.",
            "icon": "rgb",
            "params": "{\"satelite\"?: \"sentinel-2\" (padrão) | \"landsat\" | \"cbers-4\", \"composicao\"?: \"natural\" (padrão: cor verdadeira) | \"falsa-cor\", \"inicio\"?, \"fim\"?: dd/mm/aaaa ou aaaa-mm-dd (padrão: os 3 meses até a data mais recente; de 16 dias a 1 ano)}",
            "form": [
                {"name": "satelite", "label": "Satélite", "kind": "choice", "required": True, "choices": SATELITES_SEM_NUVENS, "default": SATELITES_SEM_NUVENS[0], "help": "O cubo de 16 dias do INPE: o Sentinel-2 tem o detalhe (10 m) e começa em 2017; o Landsat vai até 1990, a 30 m; o CBERS-4 WFI, satélite sino-brasileiro, a 64 m, passa mais vezes."},
                {"name": "composicao", "label": "Composição", "kind": "choice", "required": True, "choices": COMPOSICOES, "default": COMPOSICOES[0], "help": "A cor verdadeira mostra a área como o olho a veria; a falsa-cor põe o infravermelho no vermelho: a vegetação viva sai vermelha, a água escura, a cidade e o solo exposto em azul e cinza."},
                {"name": "inicio", "label": "Início (dd/mm/aaaa)", "kind": "text", "help": "A primeira data. Vazio: 3 meses antes do fim."},
                {"name": "fim", "label": "Fim (dd/mm/aaaa)", "kind": "text", "help": "A última data. Vazio: a mais recente do cubo. O período vai de 16 dias a 1 ano; mais datas dão mais chance de céu limpo, mas misturam estações."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "ndvi-copernicus",
            "label": "NDVI a 300 m, de 10 em 10 dias (Copernicus)",
            "help": "O índice de vegetação NDVI do serviço de terra do Copernicus (CLMS, versão 3), um mapa a cada 10 dias desde janeiro de 2014, em células de cerca de 300 m: o verde da vegetação de um estado ou de uma região numa data, para acompanhar a seca, a safra e as estações. Só a área, lida do Copernicus Data Space e posta na grade do projeto. Pede o login do Copernicus Data Space, que é grátis (Início › Dados web › Copernicus Data Space): o RustGIS o guarda, e o plugin não vê a senha. O servidor às vezes derruba a conexão: o plugin tenta de novo sozinho.",
            "icon": "dra",
            "params": "{\"data\"?: dd/mm/aaaa ou aaaa-mm-dd (o produto de 10 dias que contém o dia; padrão: o mais recente), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data (dd/mm/aaaa)", "kind": "text", "help": "Um dia de janeiro de 2014 em diante: vem o mapa dos 10 dias que o contêm (1 a 10, 11 a 20 ou 21 ao fim do mês). Vazio: o mais recente."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "chm-altura",
            "label": "Altura do dossel a 1 m (Meta CHMv2)",
            "help": "A altura das árvores em metros, a cerca de 1 m, da Meta e do World Resources Institute (Canopy Height Maps v2, licença CC BY 4.0): estimada por um modelo de IA (DINOv3) sobre imagens de satélite de alta resolução, de datas diferentes conforme o lugar. Só a área, lida de cada tile que ela toca, posta na grade do projeto e recortada pelo contorno; numa grade mais grossa, lida da pirâmide do arquivo, que tira a média. Para ver as copas, use uma grade de 1 a 5 m (Grade do projeto) numa área pequena. O 0 é sem árvore, e também sem dado. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "proportional",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "gdo-seca",
            "label": "Seca: umidade do solo, vegetação e SPI (Copernicus GDO)",
            "help": "Um mapa de seca do Global Drought Observatory do Copernicus (CEMS, Comissão Europeia; CC BY 4.0): a anomalia da umidade do solo (modelo LISFLOOD), a anomalia da vegetação (fAPAR do VIIRS) ou o SPI de 1, 3, 6 ou 12 meses do CHIRPS, de 10 em 10 dias (o SPI de 6 e 12 meses, por mês), numa data ou na mais recente. É o mapa do serviço do GDO, com a legenda dele: para ver, não para medir, e não é recortado pela área. O Low-Flow Index (vazão baixa nos rios) do GDO só cobre a Europa.",
            "icon": "dra",
            "params": "{\"indicador\"?: \"umidade\" (padrão) | \"vegetacao\" | \"spi1\" | \"spi3\" | \"spi6\" | \"spi12\", \"data\"?: dd/mm/aaaa ou aaaa-mm-dd (padrão: a mais recente)}",
            "form": [
                {"name": "indicador", "label": "Indicador", "kind": "choice", "required": True, "choices": INDICADORES_GDO, "default": INDICADORES_GDO[0], "help": "A umidade do solo e a vegetação mostram o efeito da seca; o SPI, a falta de chuva no período (1 mês: a seca que começa; 12 meses: a longa)."},
                {"name": "data", "label": "Data (dd/mm/aaaa)", "kind": "text", "help": "Vazio: a mais recente. Vale o decêndio (dias 1, 11 e 21) ou o mês que contém a data."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Previsão",
        {
            "id": "bam-previsao",
            "label": "Previsão do BAM (CPTEC/INPE)",
            "help": "A chuva somada ou a temperatura média prevista pelo BAM, o modelo global do CPTEC/INPE, de 1 a 10 dias da rodada mais recente das 00 UTC (21h de Brasília) ou de uma data passada. Cada registro de 6 horas (cerca de 4 MB) vem sozinho do servidor do CPTEC e é cortado em volta da área, posto na grade do projeto e recortado. A temperatura é a média das 03h, 09h, 15h e 21h. Precisa do RustGIS 0.8.0, no desktop. No 0.8.0, a janela só volta ao comando uns 10 minutos depois que os registros chegam: quando eles aparecerem no mapa, rode o comando de novo",
            "icon": "time_next",
            "params": "{\"variavel\"?: \"chuva\" (padrão) | \"temperatura\", \"dias\"?: de 1 a 10 (padrão 3), \"rodada\"?: dd/mm/aaaa, a rodada das 00 UTC desse dia (padrão: a mais recente), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "variavel", "label": "O que", "kind": "choice", "required": True, "choices": VARIAVEIS_BAM, "default": VARIAVEIS_BAM[0], "help": "A chuva somada nos dias, em mm, ou a temperatura média do ar a 2 m, em °C."},
                {"name": "dias", "label": "Dias", "kind": "number", "required": True, "default": 3, "help": "De 1 a 10. Cada dia baixa 4 registros de cerca de 4 MB."},
                {"name": "rodada", "label": "Rodada (dd/mm/aaaa)", "kind": "text", "help": "Vazio: a mais recente completa. Numa data, a rodada das 00 UTC dela, para comparar a previsão com o que aconteceu."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "grace-seca",
            "label": "Seca: umidade do solo e água subterrânea (GRACE, NASA)",
            "help": "Os indicadores semanais de seca do GRACE, da NASA: quão seca ou úmida está a zona das raízes (o primeiro metro do solo), a água subterrânea ou a superfície do solo, em percentil da distribuição de 1948 a 2012 do mesmo lugar e da mesma época (5 é mais seco que em 95% do tempo). Um modelo assimila as medidas de gravidade dos satélites GRACE e GRACE-FO. Toda segunda-feira desde 03/02/2003, em células de 0,25° (cerca de 28 km), nas cores do mapa da NASA: para o país, o estado ou a bacia, não para um município. A janela da área, lida do arquivo do mundo e mascarada pelo contorno; sem área, o mundo inteiro. Sem login; só no desktop.",
            "icon": "classify",
            "params": "{\"indicador\"?: \"raizes\" (padrão) | \"subterranea\" | \"superficie\", \"data\"?: dd/mm/aaaa ou aaaa-mm-dd (padrão: a semana mais recente)}",
            "form": [
                {"name": "indicador", "label": "Indicador", "kind": "choice", "required": True, "choices": INDICADORES_GRACE, "default": INDICADORES_GRACE[0], "help": "A zona das raízes fala da seca agrícola; a água subterrânea, da seca longa, nas reservas; a superfície muda em dias, com a chuva."},
                {"name": "data", "label": "Data (dd/mm/aaaa)", "kind": "text", "help": "Vazio: a semana mais recente. Numa data, vale a última segunda-feira até ela."},
            ],
        },
    ),
    (
        "Infraestrutura",
        "Edificações",
        {
            "id": "gba-edificacoes",
            "label": "Edificações e alturas (GlobalBuildingAtlas)",
            "help": "O polígono de cada edificação com a altura estimada, do GlobalBuildingAtlas (Universidade Técnica de Munique; licença CC BY-NC 4.0, sem uso comercial): polígonos do Google, da Microsoft e do OpenStreetMap, e alturas de um modelo de IA sobre imagens PlanetScope de 2019. O RustGIS lê o arquivo do bloco de 5° no lugar, da cópia em GeoParquet do Source Cooperative, e busca só o que a vista mostra: a camada não é recortada pela área. Cores por classe de altura (campo height). A altura é menos precisa na América do Sul. Precisa de um RustGIS que leia GeoParquet no lugar (0.7.0).",
            "icon": "unique",
            "params": "{}",
        },
    ),
    (
        "Infraestrutura",
        "Edificações",
        {
            "id": "enderecos",
            "label": "Endereços (IBGE, CNEFE 2022)",
            "help": "Um ponto por endereço do Censo 2022, do Cadastro Nacional de Endereços para Fins Estatísticos (CNEFE, IBGE), com a rua, o número e o CEP, pelo tema de endereços do Overture Maps. Duas cores: com número e sem número. O RustGIS lê no lugar os arquivos GeoParquet que cobrem a área e busca só o que a vista mostra: a camada não é recortada, é mascarada pelo contorno da área. Aparece só de 1:50.000 para perto, e os números, de 1:3.000. Precisa de um RustGIS que leia GeoParquet no lugar (0.7.0).",
            "icon": "point_layer",
            "params": "{}",
        },
    ),
    (
        "Infraestrutura",
        "Transportes",
        {
            "id": "osm-vias",
            "label": "Ruas e estradas (OpenStreetMap)",
            "help": "As ruas e estradas do OpenStreetMap (© colaboradores do OSM, licença ODbL), do tema de transportes do Overture Maps, pela classe da via: vias expressas, principais, secundárias, terciárias e vicinais, ruas, estradas de terra e trilhas. O RustGIS lê no lugar os arquivos GeoParquet que cobrem a área e busca só o que a vista mostra: a camada não é recortada, é mascarada pelo contorno da área. Aparece só de 1:150.000 para perto (de longe, as Rodovias e estradas do IBGE). Precisa de um RustGIS que leia GeoParquet no lugar (0.7.0).",
            "icon": "line_layer",
            "params": "{}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "chm-amazonia",
            "label": "Altura do dossel da Amazônia a 4,78 m (CTrees)",
            "help": "A altura das árvores em metros, a 4,78 m, só na floresta amazônica: a média de 2020 a 2024 da CTrees, estimada por um modelo de IA (U-Net) sobre as imagens Planet do programa NICFI e calibrada com lidar aéreo da Amazônia. Só a área, lida de cada tile que ela toca (cerca de 20 × 20 km, até 16 por clique), posta na grade do projeto e recortada pelo contorno; numa grade mais grossa, lida da pirâmide do arquivo, que tira a média. Licença: CC BY 4.0 nos metadados do bucket; a página do AWS Open Data diz que segue o contrato do Planet NICFI. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "proportional",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "forty-tipologia",
            "label": "Tipologia florestal 2020 a 10 m (ForTy)",
            "help": "O tipo de floresta em 2020, a 10 m, do Forest Typology (ForTy) da Nature Trace (Google DeepMind e Google Research, com o WRI e o IIASA; licença CC BY 4.0): floresta primária, em regeneração natural, plantada, plantio florestal comercial, culturas arbóreas e agrofloresta, e outros usos. As classes seguem a FAO e o regulamento europeu contra o desmatamento (EUDR), cuja data de corte é 31/12/2020. Só a área, lida de cada bloco que ela toca (cerca de 33 × 45 km), posta na grade do projeto com a classe mais frequente e recortada pelo contorno; numa grade mais grossa, lida da pirâmide do arquivo. Até 16 blocos por clique: um município ou uma microrregião. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área); só no app desktop e no rustgis-cli.",
            "icon": "unique",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "ctrees-biomassa",
            "label": "Biomassa acima do solo por ano (CTrees, 100 m)",
            "help": "A biomassa acima do solo em Mg/ha (toneladas por hectare), a 100 m, num ano de 2000 a 2025, da CTrees (licença CC BY 4.0): estimada por um modelo de IA calibrado com biomassa de lidar aéreo e de parcelas de campo, a partir de séries do Landsat, do radar ALOS PALSAR-2 e das alturas do GEDI e do ICESat-2; ou a incerteza do mapa (o erro-padrão, em Mg/ha). Só a área, lida do arquivo global do ano no bucket público da AWS, posta na grade do projeto e recortada pelo contorno; numa área grande, lida da pirâmide do arquivo, com células maiores. Compare dois anos para ver a perda ou o ganho de biomassa. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "proportional",
            "params": "{\"ano\"?: de 2000 a 2025 (padrão 2025), \"camada\"?: \"biomassa\" (padrão) | \"incerteza\", \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2025, "help": "O ano do mapa, de 2000 a 2025."},
                {"name": "camada", "label": "O que trazer", "kind": "choice", "required": True, "choices": CAMADAS_CTREES, "default": CAMADAS_CTREES[0], "help": "A biomassa estimada, ou a incerteza dela (o erro-padrão): onde a incerteza é alta, a diferença entre dois anos pode ser só ruído."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "gedi-altura",
            "label": "Altura do dossel (GEDI)",
            "help": "A altura do dossel em metros: a altura relativa em que volta 98% da energia da onda do laser (rh98), perto do topo das copas. Do GEDI L4D (ORNL DAAC da NASA): os disparos do laser do GEDI de 2019 a 2023 imputados a cada pixel de 30 m com séries do Landsat. Só a área, lida dos arquivos dos fusos UTM que ela cruza, posta na grade do projeto e recortada pelo contorno. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área); áreas grandes vêm de uma pirâmide do arquivo, com células maiores.",
            "icon": "dra",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "gedi-biomassa",
            "label": "Biomassa acima do solo (GEDI)",
            "help": "A densidade de biomassa acima do solo em Mg/ha (toneladas por hectare), do modelo do GEDI L4A. Do GEDI L4D (ORNL DAAC da NASA): os disparos do laser do GEDI de 2019 a 2023 imputados a cada pixel de 30 m com séries do Landsat. Só a área, lida dos arquivos dos fusos UTM que ela cruza, posta na grade do projeto e recortada pelo contorno. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área); áreas grandes vêm de uma pirâmide do arquivo, com células maiores.",
            "icon": "proportional",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Vegetação",
        {
            "id": "gedi-cobertura",
            "label": "Cobertura do dossel (GEDI)",
            "help": "A cobertura do dossel em %, a fração do chão coberta pelas copas, do GEDI L2B. Do GEDI L4D (ORNL DAAC da NASA): os disparos do laser do GEDI de 2019 a 2023 imputados a cada pixel de 30 m com séries do Landsat. Só a área, lida dos arquivos dos fusos UTM que ela cruza, posta na grade do projeto e recortada pelo contorno. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área); áreas grandes vêm de uma pirâmide do arquivo, com células maiores.",
            "icon": "transparency",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-vv",
            "label": "Radar VV (Sentinel-1, OPERA)",
            "help": "A polarização VV: responde mais à rugosidade e à umidade do solo. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "stretch",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-vh",
            "label": "Radar VH (Sentinel-1, OPERA)",
            "help": "A polarização VH: responde mais ao volume da vegetação. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "stretch",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-razao",
            "label": "Razão VH/VV (Sentinel-1, OPERA)",
            "help": "A razão VH/VV, que separa vegetação de solo exposto; as camadas VV e VH da mesma passagem também ficam no mapa. O radar do Sentinel-1 corrigido do relevo (OPERA RTC-S1, JPL, pelo ASF DAAC da NASA), 30 m, em dB e com o filtro de Lee contra o granulado: os recortes da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, juntados, recortados pela área e postos na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "rgb",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "emit-cenas",
            "label": "Cenas hiperespectrais do EMIT (NASA)",
            "help": "Onde há cena do EMIT, o espectrômetro de imagem da NASA na Estação Espacial Internacional (refletância da superfície em 285 bandas, de 381 a 2.493 nm, 60 m, desde agosto de 2022): o contorno de cada cena que cruza a área (sem área, a vista), na cor da nuvem dela, a menos nublada por cima. Os campos dizem a data, a hora, a nuvem, a versão, o endereço da prévia (aberta) e o do arquivo netCDF (quase 2 GB por cena, com o login Earthdata), que o plugin não baixa. A janela do comando escolhe o limite de nuvem e o ano. A nuvem é da cena inteira, não da área. A versão 002 vale no lugar da 001 na mesma cena. Sem login para a lista.",
            "icon": "polygon_layer",
            "params": "{\"nuvem\"?: de 0 a 100, o limite de nuvem da cena, em % (padrão 30; 100: todas), \"ano\"?: de 2022 em diante (padrão: todos)}",
            "form": [
                {"name": "nuvem", "label": "Nuvem na cena", "kind": "choice", "required": True, "choices": NUVENS_EMIT, "default": NUVENS_EMIT[1], "help": "Só as cenas com até essa fração de nuvem, na cena inteira."},
                {"name": "ano", "label": "Ano", "kind": "text", "help": "Só as cenas desse ano, de 2022 em diante. Vazio: todos os anos."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "nisar-hh",
            "label": "Radar HH (NISAR, banda L)",
            "help": "A polarização HH: responde mais aos troncos, ao solo e às construções. O radar de banda L do NISAR (NASA e ISRO), o GCOV corrigido do relevo, 20 m, em dB e com o filtro sigma de Lee contra o granulado: as cenas da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, tiradas do HDF5 e recortadas pelo retângulo da área pelo Harmony da NASA, juntadas, recortadas pela área e postas na grade do projeto. A banda L (24 cm) atravessa mais as copas que a banda C do Sentinel-1. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 110 km; cerca de 40 s por cena.",
            "icon": "stretch",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "nisar-hv",
            "label": "Radar HV (NISAR, banda L)",
            "help": "A polarização HV: responde mais ao volume da vegetação (galhos e copas). O radar de banda L do NISAR (NASA e ISRO), o GCOV corrigido do relevo, 20 m, em dB e com o filtro sigma de Lee contra o granulado: as cenas da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, tiradas do HDF5 e recortadas pelo retângulo da área pelo Harmony da NASA, juntadas, recortadas pela área e postas na grade do projeto. A banda L (24 cm) atravessa mais as copas que a banda C do Sentinel-1. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 110 km; cerca de 40 s por cena.",
            "icon": "stretch",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "nisar-razao",
            "label": "Razão HV/HH (NISAR, banda L)",
            "help": "A razão HV/HH, que separa vegetação de solo exposto e de cidade; as camadas HH e HV da mesma passagem também ficam no mapa. O radar de banda L do NISAR (NASA e ISRO), o GCOV corrigido do relevo, 20 m, em dB e com o filtro sigma de Lee contra o granulado: as cenas da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, tiradas do HDF5 e recortadas pelo retângulo da área pelo Harmony da NASA, juntadas, recortadas pela área e postas na grade do projeto. A banda L (24 cm) atravessa mais as copas que a banda C do Sentinel-1. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 110 km; cerca de 40 s por cena.",
            "icon": "rgb",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "nisar-tempo",
            "label": "Composição temporal (NISAR, banda L)",
            "help": "A composição temporal do radar NISAR, como a da NASA: a mediana do HH no vermelho (solo exposto, superfícies ásperas, construções), a mediana do HV no verde (vegetação) e o desvio-padrão do HV entre as passagens no azul (vegetação que muda: lavoura que cresce ou é colhida, pasto que seca). A mediana de várias passagens tira o granulado que uma passagem só deixa. Usa as passagens mais recentes da mesma trilha (a mesma visada), uma a cada 12 dias, que cruzam o polígono da área; cada uma é tirada do HDF5 e recortada pelo Harmony da NASA, filtrada (sigma de Lee) e posta na grade do projeto. Só a composição fica no mapa. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 110 km; cerca de 40 s por cena e polarização (4 passagens de 1 cena: uns 6 min).",
            "icon": "rgb",
            "params": "{\"passagens\"?: de 2 a 8, quantas passagens (padrão 4), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "passagens", "label": "Passagens", "kind": "choice", "choices": ["2", "3", "4", "6", "8"], "default": "4", "help": "Quantas passagens da mesma trilha, das mais recentes (uma a cada 12 dias): mais passagens, menos granulado e mais tempo."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-10m",
            "label": "Radar 10 m, mediana no tempo (Sentinel-1)",
            "help": "O retroespalhamento do radar Sentinel-1 (banda C) em 10 m, corrigido do relevo, como no Sentinel-1 Global Backscatter Model: a mediana das passagens mais recentes da mesma órbita (uma a cada 12 dias), que tira o granulado sem borrar as bordas, em dB e em Viridis (água e sombra do relevo escuras, cidade e encostas viradas para o satélite em amarelo). Do Planetary Computer da Microsoft (coleção sentinel-1-rtc, processada pela Catalyst, CC-BY 4.0): só a janela da área de cada cena, lida por faixas, sem login. Sem grade no mapa, cria uma de 10 m; numa grade mais grossa, a janela vem de uma visão reduzida do arquivo. Áreas de até cerca de 220 km; cerca de 20 s por passagem num município (10 passagens: uns 3 min). A posição do dado não é exata: de 13 a 33 m a oeste do Sentinel-2 em oito áreas planas do Brasil, e variável no relevo forte.",
            "icon": "raster_layer",
            "params": "{\"polarizacao\"?: \"VV\" (padrão) ou \"VH\", \"passagens\"?: de 2 a 30, quantas passagens (padrão 10), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 10)}",
            "form": [
                {"name": "polarizacao", "label": "Polarização", "kind": "choice", "choices": ["VV: solo, água e construções", "VH: volume da vegetação"], "default": "VV: solo, água e construções", "help": "VV responde mais ao solo, à água e às construções; VH, ao volume da vegetação."},
                {"name": "passagens", "label": "Passagens", "kind": "choice", "choices": ["4", "6", "10", "15", "20", "30"], "default": "10", "help": "Quantas passagens da mesma órbita, das mais recentes (uma a cada 12 dias): mais passagens, menos granulado e mais tempo, mas misturam estações do ano."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "radar-10m-cor",
            "label": "Radar 10 m, composição colorida (Sentinel-1)",
            "help": "A composição colorida do radar Sentinel-1 em 10 m: a mediana do VV no vermelho, a do VH no verde e VV − VH (a razão VV/VH, em dB) no azul. Cidade e construções em rosa e magenta, mata e cerrado em amarelo e verde, lavoura e solo exposto em lilás, roxo e azul, água preta: separa bem pivôs centrais e talhões. As camadas VV e VH da mediana (as do Radar 10 m, mediana no tempo) também ficam no mapa. Do Planetary Computer da Microsoft (coleção sentinel-1-rtc, CC-BY 4.0), sem login; só a janela da área de cada cena. Áreas de até cerca de 220 km; o dobro do tempo do Radar 10 m (uma leitura por polarização). A posição do dado não é exata: de 13 a 33 m a oeste do Sentinel-2 em oito áreas planas do Brasil, e variável no relevo forte.",
            "icon": "rgb",
            "params": "{\"passagens\"?: de 2 a 30, quantas passagens (padrão 10), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 10)}",
            "form": [
                {"name": "passagens", "label": "Passagens", "kind": "choice", "choices": ["4", "6", "10", "15", "20", "30"], "default": "10", "help": "Quantas passagens da mesma órbita, das mais recentes (uma a cada 12 dias): mais passagens, menos granulado e mais tempo, mas misturam estações do ano."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Uso e cobertura",
        {
            "id": "nisar-cor",
            "label": "Composição colorida (NISAR, banda L)",
            "help": "A composição colorida: HH no vermelho, HV no verde e HH − HV no azul. No estilo das imagens do NISAR da NASA: mata em verde forte, cidade em rosa e magenta, pasto e solo exposto em azul e roxo escuros, água escura; as camadas HH, HV e a razão da mesma passagem também ficam no mapa. O radar de banda L do NISAR (NASA e ISRO), o GCOV corrigido do relevo, 20 m, em dB e com o filtro sigma de Lee contra o granulado: as cenas da passagem mais recente (ou a do dia escolhido na janela do comando) que cruzam o polígono da área, tiradas do HDF5 e recortadas pelo retângulo da área pelo Harmony da NASA, juntadas, recortadas pela área e postas na grade do projeto. A banda L (24 cm) atravessa mais as copas que a banda C do Sentinel-1. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 110 km; cerca de 40 s por cena.",
            "icon": "rgb",
            "params": "{\"data\"?: \"dd/mm/aaaa\", a passagem desse dia (padrão: a mais recente que cobre a área), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Data da passagem", "kind": "text", "help": "A passagem do satélite desse dia, dd/mm/aaaa. Vazio: a mais recente que cobre a área."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-situacao",
            "label": "Perturbação da vegetação: situação (OPERA DIST-S1)",
            "help": "A situação do alerta: a cor diz a etapa (amarelo, primeira detecção; laranja, provisória; vermelho, confirmada; roxo, encerrada, quando a vegetação voltou ao normal) e, mais escura, a confiança alta; sem perturbação fica transparente. As manchas vêm também como polígonos, com borda fina e o nome da classe na legenda; o raster, na grade, fica escondido. Na janela do comando, as cores podem ser as da paleta oficial da OPERA. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile (ou o de até o dia escolhido na janela do comando) que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "unique",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30), \"paleta\"?: \"brasildata\" (padrão: a cor diz a etapa e, mais escura, a confiança alta) | \"oficial\" (a da OPERA)}",
            "form": [
                {"name": "data", "label": "Até a data", "kind": "text", "help": "Os produtos até esse dia, dd/mm/aaaa: o alerta como estava então. Vazio: os mais recentes."},
                {"name": "paleta", "label": "Cores", "kind": "choice", "required": True, "choices": PALETAS, "default": PALETAS[0], "help": "A do BrasilData: a cor diz a etapa do alerta e, mais escura, a confiança alta. A oficial: a paleta da OPERA."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-data",
            "label": "Perturbação da vegetação: data (OPERA DIST-S1)",
            "help": "Quantos dias antes do produto mais recente a perturbação foi vista pela primeira vez: as mais novas em vermelho-escuro. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile (ou o de até o dia escolhido na janela do comando) que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "history",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Até a data", "kind": "text", "help": "Os produtos até esse dia, dd/mm/aaaa: o alerta como estava então. Vazio: os mais recentes."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-confianca",
            "label": "Perturbação da vegetação: confiança (OPERA DIST-S1)",
            "help": "A confiança do alerta, que cresce a cada passagem que confirma a perturbação. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile (ou o de até o dia escolhido na janela do comando) que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "graduated",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Até a data", "kind": "text", "help": "Os produtos até esse dia, dd/mm/aaaa: o alerta como estava então. Vazio: os mais recentes."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Desmatamento",
        {
            "id": "dist-intensidade",
            "label": "Perturbação da vegetação: intensidade (OPERA DIST-S1)",
            "help": "A intensidade: o maior desvio do sinal do radar em relação ao histórico do lugar, desde a primeira detecção. O alerta de perturbação da vegetação do radar do Sentinel-1 (OPERA DIST-S1, JPL, pelo ASF DAAC da NASA), 30 m: o produto mais recente de cada tile (ou o de até o dia escolhido na janela do comando) que cruza o polígono da área, que acumula os alertas desde o começo do monitoramento (04/01/2026), juntado, recortado pela área e posto na grade do projeto. Pede o login Earthdata, que é grátis: o token vai no RustGIS (Início › Dados web › NASA Earthaccess › Earthdata Login), nunca no plugin. Áreas de até cerca de 400 km.",
            "icon": "contrast",
            "params": "{\"data\"?: \"dd/mm/aaaa\", os produtos até esse dia (padrão: os mais recentes), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "data", "label": "Até a data", "kind": "text", "help": "Os produtos até esse dia, dd/mm/aaaa: o alerta como estava então. Vazio: os mais recentes."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Queimadas",
        {
            "id": "inpe-focos-periodo",
            "label": "Focos de calor: período (INPE)",
            "help": "Os focos de calor do INPE de uma data a outra (dia/mês/ano) do mês corrente (UTC): INPE, Programa Queimadas (Sisfogo, IBAMA/PAMGIA). Só o que cruza a área, recortado pela área; sem área, o que está na vista. Os dias vêm da janela do comando; num RustGIS sem janelas de plugin, o mês corrente inteiro.",
            "icon": "time_slider",
            "params": "{\"de\"?: \"dd/mm/aaaa\" ou o dia do mês (1 a 31), \"ate\"?: \"dd/mm/aaaa\" ou o dia do mês (padrão: o mês corrente inteiro)}",
            "form": [
                {"name": "de", "label": "De", "kind": "text", "help": "O primeiro dia, dd/mm/aaaa, do mês corrente: o serviço só guarda o mês corrente. Vazio: o dia 1."},
                {"name": "ate", "label": "Até", "kind": "text", "help": "O último dia, dd/mm/aaaa, do mês corrente. Vazio: o último dia com focos."},
            ],
        },
    ),
    (
        "Vegetação e meio ambiente",
        "Queimadas",
        {
            "id": "risco-fogo",
            "label": "Risco de fogo (INPE)",
            "help": "O perigo meteorológico de fogo, de 0 a 1, do modelo de Risco de Fogo do INPE (Programa Queimadas, versão 11: dias de secura pela chuva, umidade, temperatura, vegetação, relevo e latitude): a média de 2001 a 2025 de um mês, a 1 km. Só a área, lida do arquivo do mês, posta na grade do projeto e recortada pelo contorno, nas cores das cinco classes do INPE: mínimo (abaixo de 0,15), baixo (até 0,40), médio (até 0,70), alto (até 0,95) e crítico. O mês vem da janela do comando, ou do controle Risco de fogo: mês no painel Área de trabalho (setembro até você mudar). Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "classify",
            "params": "{\"mes\"?: de 1 a 12 (padrão: o do painel, ou 9, setembro), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "mes", "label": "Mês", "kind": "choice", "required": True, "choices": MESES, "default": "setembro", "help": "O mês da climatologia: a média de 2001 a 2025 do perigo de fogo nesse mês. Setembro é o auge da estação de fogo no Brasil central; ao norte do equador, como em Roraima, a seca vai de dezembro a março."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Clima passado (ERA5)",
        {
            "id": "era5-chuva",
            "label": "Chuva do ERA5 (reanálise, desde 1940)",
            "help": "A chuva de um período passado, em mm: o total do período ou a média por ano, que num período de 30 anos é a normal do lugar." + FIM_ERA5,
            "icon": "graduated",
            "params": "{\"resumo\"?: \"total\" (padrão) | \"anual\", " + PARAMS_ERA5 + "}",
            "form": [{"name": "resumo", "label": "Resumo", "kind": "choice", "required": True, "choices": RESUMOS_ERA5, "default": RESUMOS_ERA5[0], "help": "A chuva somada no período, ou a média por ano (num período de vários anos)."}] + CAMPOS_ERA5,
        },
    ),
    (
        "Tempo e clima",
        "Clima passado (ERA5)",
        {
            "id": "era5-temperatura",
            "label": "Temperatura do ERA5 (reanálise, desde 1940)",
            "help": "A temperatura do ar a 2 m num período passado, em °C: a média da máxima do dia, da mínima ou da média, ajustada à altitude de cada ponto." + FIM_ERA5,
            "icon": "stretch",
            "params": "{\"tipo\"?: \"maxima\" (padrão) | \"minima\" | \"media\", " + PARAMS_ERA5 + "}",
            "form": [{"name": "tipo", "label": "Temperatura", "kind": "choice", "required": True, "choices": TEMPERATURAS, "default": TEMPERATURAS[2], "help": "A máxima ou a mínima de cada dia, em média no período, ou a média do dia."}] + CAMPOS_ERA5,
        },
    ),
    (
        "Tempo e clima",
        "Clima passado (ERA5)",
        {
            "id": "era5-umidade",
            "label": "Umidade do solo do ERA5 (reanálise, desde 1940)",
            "help": "A umidade da camada de cima do solo (0 a 7 cm) num período passado, em % do volume: a média dos dias." + FIM_ERA5,
            "icon": "dissolve",
            "params": "{" + PARAMS_ERA5 + "}",
            "form": CAMPOS_ERA5,
        },
    ),
    (
        "Tempo e clima",
        "Atmosfera",
        {
            "id": "indicadores-seeg",
            "label": "Emissões de gases por município (SEEG)",
            "help": "As emissões de gases de efeito estufa por município, do SEEG (Observatório do Clima), num retrato de 2025 que vem com o plugin, em toneladas de cada gás no ano: o metano (CH4) total e o da agropecuária (sobretudo a fermentação no estômago do gado), dos resíduos (aterros e esgoto), da mudança de uso da terra (queimadas) e da energia; o CO2 emitido e o líquido (com as remoções pela floresta em pé e pela regeneração, que podem deixá-lo negativo); o óxido nitroso (N2O); e o CO2 equivalente líquido (AR5 do IPCC: CH4 × 28, N2O × 265). São estimativas a partir de dados de atividade (rebanhos, combustíveis, lixo, desmatamento), não medições. Na camada Indicadores por município (os municípios que cruzam a área, até 200, mostrados só dentro dela): os campos se somam aos que ela já tem; o layout faz o mapa do metano.",
            "icon": "field",
            "params": "{}",
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "luzes-noturnas",
            "label": "Luzes noturnas (1992–2024)",
            "help": "As luzes da Terra à noite vistas por satélite, num ano de 1992 a 2024, em células de 30 segundos (cerca de 900 m): a série PCNL, que corrige um pelo outro os sensores DMSP (até 2013) e VIIRS (depois). Mostram cidades, estradas e eletrificação, e servem para acompanhar o crescimento urbano e a economia de municípios e regiões. O valor é relativo, sem unidade (0: sem luz). De Li, Cao e outros (2023), CC BY 4.0: a janela da área no recorte do Brasil, posta na grade do projeto e recortada. Sem login.",
            "icon": "raster_layer",
            "params": "{\"ano\"?: de 1992 a 2024 (padrão: 2024), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2024, "help": "De 1992 a 2024."},
            ],
        },
    ),
    (
        "Sociedade e economia",
        "Economia",
        {
            "id": "luzes-mudanca",
            "label": "Mudança das luzes noturnas",
            "help": "Quanto a luz à noite mudou entre dois anos, de 1992 a 2024: o último menos o primeiro, célula a célula, das mesmas Luzes noturnas (PCNL, 30 segundos, cerca de 900 m). Laranja: mais luz (a cidade que cresce, a eletrificação rural); roxo: menos. Entre 2013 e 2014 o sensor muda, e parte da diferença pode vir da troca (um aviso diz isso): para o mesmo sensor, fique até 2013 ou a partir de 2014.",
            "icon": "raster_layer",
            "params": "{\"inicio\"?: o primeiro ano, de 1992 a 2023 (padrão: 2014), \"fim\"?: o último, até 2024 (padrão: 2024), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "inicio", "label": "Primeiro ano", "kind": "number", "required": True, "default": 2014, "help": "De 1992 a 2023. De 2014 em diante é o mesmo sensor."},
                {"name": "fim", "label": "Último ano", "kind": "number", "required": True, "default": 2024, "help": "Depois do primeiro, até 2024."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Atmosfera",
        {
            "id": "metano-tropomi",
            "label": "Metano no ar (Sentinel-5P)",
            "help": "A coluna média de metano no ar (XCH4, em ppb) medida pelo satélite Sentinel-5P (instrumento TROPOMI, do Copernicus): a média do ano ou de um mês, numa grade de 0,1° (cerca de 11 km) que o BrasilData calcula das órbitas do satélite, só com os pixels de boa qualidade (qa > 0,5): quase toda a Amazônia fica sem dado. Mostra onde há mais metano no ar, não quanto cada lugar emite: o valor cai com a altitude (há menos ar acima) e varia com a estação; manchas altas que se repetem nos meses apontam fontes, como áreas alagadas, pecuária e aterros. Numa área, a janela dela, lida do arquivo do Brasil e mascarada pelo contorno (as células da divisa aparecem cortadas por ele); sem área, o Brasil inteiro, lido pela vista. A camada fica abaixo dos rios, das estradas e das localidades. Contém dados modificados do Copernicus Sentinel (ESA). Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "graduated",
            "params": "{\"periodo\"?: \"média de 2023\" (padrão) | \"março de 2023\" | \"2023-03\"}",
            "form": [
                {
                    "name": "periodo",
                    "label": "Período",
                    "kind": "choice",
                    "required": True,
                    "choices": ["média de 2023"] + [f"{m} de 2023" for m in ("janeiro", "fevereiro", "março", "abril", "maio", "junho", "julho", "agosto", "setembro", "outubro", "novembro", "dezembro")],
                    "default": "média de 2023",
                    "help": "A média do ano, ou de um mês: na estação chuvosa as áreas alagadas emitem mais, e um mês tem menos observações (as nuvens tiram dias)."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Satélites do JAXA",
        {
            "id": "jaxa-chuva",
            "label": "Chuva por satélite (GSMaP, JAXA)",
            "help": "A chuva estimada por satélite do GSMaP (Global Satellite Mapping of Precipitation, a partir dos satélites GPM e de outros, ajustada com pluviômetros), em mm por dia: a média dos meses do período, desde março de 2000, a 0,1° (cerca de 11 km)." + FIM_JAXA,
            "icon": "graduated",
            "params": "{" + PARAMS_JAXA + "}",
            "form": CAMPOS_JAXA,
        },
    ),
    (
        "Tempo e clima",
        "Satélites do JAXA",
        {
            "id": "jaxa-umidade",
            "label": "Umidade do solo (AMSR2, JAXA)",
            "help": "A umidade da camada de cima do solo, em %, do radiômetro de micro-ondas AMSR2 (satélite GCOM-W, passagem do dia), desde julho de 2012, a 0,1° (cerca de 11 km). Os valores são baixos e escuros sob a floresta e altos perto de rios e do mar: use como padrão relativo, para comparar lugares e meses." + FIM_JAXA,
            "icon": "dissolve",
            "params": "{" + PARAMS_JAXA + "}",
            "form": CAMPOS_JAXA,
        },
    ),
    (
        "Tempo e clima",
        "Satélites do JAXA",
        {
            "id": "jaxa-temperatura",
            "label": "Temperatura da superfície (GCOM-C, JAXA)",
            "help": "A temperatura da superfície da terra, em °C, do sensor SGLI (satélite GCOM-C), de dia (por volta das 10h30) ou à noite (por volta das 22h30), desde 2018, a 1/24° (cerca de 4,6 km). Com nuvem, o mês fica com menos observações." + FIM_JAXA,
            "icon": "brightness",
            "params": "{\"turno\"?: \"dia\" (padrão) | \"noite\", " + PARAMS_JAXA + "}",
            "form": [{"name": "turno", "label": "Hora da passagem", "kind": "choice", "required": True, "choices": TURNOS_JAXA, "default": TURNOS_JAXA[0], "help": "De dia, a superfície aquecida pelo sol (o asfalto e o solo exposto ficam quentes); à noite, o calor que ela guarda."}] + CAMPOS_JAXA,
        },
    ),
    (
        "Tempo e clima",
        "Satélites do JAXA",
        {
            "id": "jaxa-evapotranspiracao",
            "label": "Evapotranspiração (GCOM-C, JAXA)",
            "help": "A água que o solo e as plantas devolvem ao ar, em mm por dia: a evapotranspiração do JASMES, do sensor SGLI (satélite GCOM-C), a 0,05° (cerca de 5,5 km), desde abril de 2025 (os meses anteriores não estão no STAC do JAXA Earth)." + FIM_JAXA,
            "icon": "dra",
            "params": "{" + PARAMS_JAXA + "}",
            "form": CAMPOS_JAXA,
        },
    ),
    (
        "Tempo e clima",
        "Satélites do JAXA",
        {
            "id": "jaxa-seca",
            "label": "Índice de seca SPI (GSMaP, JAXA)",
            "help": "O índice de precipitação padronizado (SPI) do GSMaP: quanto a chuva do mês fugiu da normal, em desvios-padrão (abaixo de -1, seca; abaixo de -2, seca extrema; acima de 1, chuva acima do normal), desde junho de 2019, a cerca de 22 km." + FIM_JAXA,
            "icon": "classify",
            "params": "{" + PARAMS_JAXA + "}",
            "form": CAMPOS_JAXA,
        },
    ),
    (
        "Tempo e clima",
        "Previsão",
        {
            "id": "chuva-prevista",
            "label": "Chuva prevista (Open-Meteo)",
            "help": "A chuva prevista somada nos próximos dias, a partir de hoje (horário de Brasília), de um modelo global de previsão do tempo pelo Open-Meteo: o ECMWF IFS (o padrão), o AIFS do ECMWF (de inteligência artificial), o GFS da NOAA, o ICON do DWD ou o melhor do Open-Meteo para o lugar. Os valores diários vêm nos nós de uma grade de 0,25° (cerca de 28 km) que cercam a área e viram um raster, interpolado (bilinear) para a grade do projeto e recortado pela área. Numa área grande, a grade de pontos fica mais aberta (até 400 pontos). Uma rodada só oscila muito na segunda semana: para a faixa provável, use a Chuva do ensemble. O modelo e os dias vêm da janela do comando; num RustGIS sem janelas de plugin, o ECMWF IFS em 15 dias. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0).",
            "icon": "time_next",
            "params": "{\"modelo\"?: \"ecmwf\", \"aifs\", \"gfs\", \"icon\" ou \"melhor\" (padrão: ecmwf), \"dias\"?: de 1 a 16, cortado nos dias do modelo (padrão 15), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {
                    "name": "modelo",
                    "label": "Modelo",
                    "kind": "choice",
                    "required": True,
                    "choices": MODELOS_PREVISAO,
                    "default": MODELOS_PREVISAO[0],
                    "help": "O modelo de previsão. O ECMWF IFS costuma acertar mais; o AIFS é o modelo de IA do ECMWF, mais suave; o GFS é o da NOAA; o ICON, do serviço alemão, vai até 7 dias."
                },
                {
                    "name": "dias",
                    "label": "Dias",
                    "kind": "number",
                    "required": True,
                    "default": 15,
                    "help": "Quantos dias somar, de 1 a 16, contando hoje. Acima dos dias do modelo, vale o máximo dele."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Previsão",
        {
            "id": "chuva-ensemble",
            "label": "Chuva do ensemble: chance e faixa provável (Open-Meteo)",
            "help": "A chuva dos próximos dias nos membros de um ensemble (o mesmo modelo rodado várias vezes com pequenas diferenças), pelo Open-Meteo: o ECMWF ENS (51 membros, o padrão), o AIFS-ENS do ECMWF (51) ou o GEFS da NOAA (31, a 0,5°). Mostra a chance (%) de a chuva somada passar de um limiar, a média dos membros, o cenário seco (percentil 10) ou o chuvoso (percentil 90): mais confiável que uma rodada só, que oscila muito de uma para a outra na segunda semana. Os valores vêm nos nós de uma grade de 0,25° que cercam a área (até 58 pontos no ECMWF e 96 no GEFS; numa área grande, mais abertos) e viram um raster, interpolado para a grade do projeto e recortado pela área. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0).",
            "icon": "graduated",
            "params": "{\"modelo\"?: \"ecmwf\", \"aifs\" ou \"gfs\" (padrão: ecmwf), \"medida\"?: \"chance\", \"media\", \"p10\" ou \"p90\" (padrão: chance), \"limiar\"?: mm, para a chance (padrão 100), \"dias\"?: de 1 a 16 (padrão 15), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {
                    "name": "modelo",
                    "label": "Ensemble",
                    "kind": "choice",
                    "required": True,
                    "choices": MODELOS_ENSEMBLE,
                    "default": MODELOS_ENSEMBLE[0],
                    "help": "O ensemble. O ECMWF ENS é o de referência; o AIFS-ENS é o de IA do ECMWF; o GEFS é o da NOAA, mais grosso (0,5°)."
                },
                {
                    "name": "medida",
                    "label": "O que mostrar",
                    "kind": "choice",
                    "required": True,
                    "choices": MEDIDAS_ENSEMBLE,
                    "default": MEDIDAS_ENSEMBLE[0],
                    "help": "A chance de passar do limiar, em %; ou a chuva, em mm: a média dos membros, o cenário seco (9 em 10 membros dão mais que isso) ou o chuvoso (só 1 em 10 dá mais)."
                },
                {
                    "name": "limiar",
                    "label": "Limiar (mm)",
                    "kind": "number",
                    "required": True,
                    "default": 100,
                    "help": "Para a chance: a chuva somada, em mm, que se quer saber se passa. Por exemplo, 50 mm em 7 dias, ou 150 mm em 15."
                },
                {
                    "name": "dias",
                    "label": "Dias",
                    "kind": "number",
                    "required": True,
                    "default": 15,
                    "help": "Quantos dias somar, de 1 a 16, contando hoje."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Previsão",
        {
            "id": "temperatura-prevista",
            "label": "Temperatura prevista (Open-Meteo)",
            "help": "A temperatura do ar a 2 m prevista para os próximos dias, a partir de hoje (horário de Brasília): a média, nos dias pedidos, da máxima do dia, da mínima ou da média, de um modelo global pelo Open-Meteo (o ECMWF IFS por padrão), ajustada à altitude de cada ponto. Os valores vêm nos nós de uma grade de 0,25° em volta da área e viram um raster, interpolado para a grade do projeto e recortado pela área: o mapa mostra a variação regional, não o frio dos vales nem o calor das cidades. O tipo, o modelo e os dias vêm da janela do comando; sem janelas de plugin, a máxima do ECMWF IFS em 7 dias. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0).",
            "icon": "stretch",
            "params": "{\"tipo\"?: \"maxima\", \"minima\" ou \"media\" (padrão: maxima), \"modelo\"?: \"ecmwf\", \"aifs\", \"gfs\", \"icon\" ou \"melhor\" (padrão: ecmwf), \"dias\"?: de 1 a 16 (padrão 7), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {
                    "name": "tipo",
                    "label": "Temperatura",
                    "kind": "choice",
                    "required": True,
                    "choices": TEMPERATURAS,
                    "default": TEMPERATURAS[0],
                    "help": "A máxima ou a mínima de cada dia, em média nos dias pedidos, ou a média do dia."
                },
                {
                    "name": "modelo",
                    "label": "Modelo",
                    "kind": "choice",
                    "required": True,
                    "choices": MODELOS_PREVISAO,
                    "default": MODELOS_PREVISAO[0],
                    "help": "O modelo de previsão."
                },
                {
                    "name": "dias",
                    "label": "Dias",
                    "kind": "number",
                    "required": True,
                    "default": 7,
                    "help": "Quantos dias, de 1 a 16, contando hoje."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Chuva observada",
        {
            "id": "chirps-mes",
            "label": "Chuva do mês (CHIRPS v3)",
            "help": "A chuva total de um mês, estimada por satélite e corrigida com estações (CHIRPS v3.0, do Climate Hazards Center da Universidade da Califórnia em Santa Bárbara), a 0,05° (cerca de 5,5 km), de 1981 até o último mês fechado: o total sai na terceira semana do mês seguinte. Só a área, lida do arquivo mundial do mês, posta na grade do projeto e recortada pelo contorno. O mês vem da janela do comando; vazio, o mais recente. Só no app desktop: o servidor não aceita pedidos do app web. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "history",
            "params": "{\"mes\"?: \"mm/aaaa\" (padrão: o mais recente), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {
                    "name": "mes",
                    "label": "Mês",
                    "kind": "text",
                    "help": "O mês, mm/aaaa (por exemplo 10/2026), de 01/1981 até o último fechado. Vazio: o mais recente."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Chuva observada",
        {
            "id": "cemaden-chuva",
            "label": "Chuva nas estações (CEMADEN)",
            "help": "A chuva medida agora nas estações da rede do CEMADEN (pluviômetros automáticos, do semiárido e das estações hidrológicas, geotécnicas e agrometeorológicas): os acumulados de 1, 3, 6, 12, 24, 48, 72 e 96 horas e a hora da última leitura (UTC), atualizados a cada 10 minutos, com as cores da chuva em 24 horas. Só as estações dentro da área, recortadas por ela; sem área, as da vista. Os valores vêm do serviço do mapa interativo do CEMADEN, por estado (até 6 estados por vez), e as coordenadas, da lista das estações que vem no plugin: uma estação instalada depois desta versão fica de fora, e o aviso diz quantas. Só no app desktop: o servidor não aceita pedidos do app web.",
            "icon": "point_layer",
            "params": "{}",
        },
    ),
    (
        "Tempo e clima",
        "Chuva observada",
        {
            "id": "pardig-agora",
            "label": "Chuva agora (PARDIG)",
            "help": "A taxa de chuva de agora, em mm/h, estimada por IA a partir de satélites (PARDIG, do Google DeepMind e do Google Research, treinado no radar do satélite GPM da NASA e da JAXA), a 0,1° (cerca de 10 km), a cada 15 minutos. O quadro mais recente sai com umas 5 horas de atraso (de 2 a 8 horas: os quadros chegam em lotes de 6 horas); com a hora na janela do comando, o daquele momento, de 1999 em diante. Só a área, lida do quadro mundial no Google Cloud Storage, posta na grade do projeto e recortada pelo contorno. Dado experimental, não é alerta oficial. Só no app desktop: o servidor não aceita pedidos do app web. Precisa de um RustGIS que leia a janela de um store Zarr (layer.addZarr com bbox).",
            "icon": "history",
            "params": "{\"quando\"?: \"dd/mm/aaaa hh:mm\", no horário de Brasília (padrão: o quadro mais recente), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {
                    "name": "quando",
                    "label": "Data e hora",
                    "kind": "text",
                    "help": "dd/mm/aaaa hh:mm, no horário de Brasília (por exemplo 16/11/2026 07:30), de 01/01/1999 em diante; arredonda para baixo aos 15 minutos. Vazio: o quadro mais recente."
                }
            ]
        },
    ),
    (
        "Tempo e clima",
        "Desastres",
        {
            "id": "cemaden-alertas",
            "label": "Alertas vigentes (CEMADEN)",
            "help": "Os alertas de risco geo-hidrológico vigentes do CEMADEN (risco hidrológico, como enxurradas e inundações, e movimentos de massa, como deslizamentos), nos níveis moderado, alto e muito alto, emitidos para a Defesa Civil: o município inteiro com alerta, nas cores do nível, com o evento, o nível e a hora da emissão e da última atualização (horário de Brasília). Vêm inteiros, sem recorte, os municípios com alerta que têm ao menos 3% dentro da área, porque o alerta vale para o município todo (uma lasca da divisa de um vizinho não conta); sem área, os do retângulo da vista. Sem alerta na área, o aviso diz quantos há no Brasil. Só no app desktop: o servidor não aceita pedidos do app web.",
            "icon": "polygon_layer",
            "params": "{}",
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "sol-radiacao",
            "label": "Radiação solar (NASA POWER)",
            "help": "A irradiação global horizontal na superfície, com todos os céus (nuvens incluídas), em kWh/m² por dia: a média diária de 2001 a 2025, do NASA POWER (CERES SYN1deg, NASA Langley), em células de 1° (cerca de 110 km). As células em volta da área, com uma de margem, viram um raster de 1°, que é interpolado (bilinear) para a grade do projeto e recortado pela área: o mapa mostra como a radiação varia de uma célula para a outra, não detalhes menores que 1°. O período, o ano ou um mês, vem da janela do comando; num RustGIS sem janelas de plugin, a média do ano. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0).",
            "icon": "brightness",
            "params": "{\"mes\"?: de 1 a 12, a média diária desse mês (padrão: a do ano), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "mes", "label": "Período", "kind": "choice", "required": True, "choices": [MEDIA_DO_ANO] + MESES, "default": MEDIA_DO_ANO, "help": "A média diária do ano, ou a de um mês, de 2001 a 2025."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "vento-50m",
            "label": "Vento a 50 m (NASA POWER)",
            "help": "A velocidade média do vento a 50 m de altura, em m/s: a média de 2001 a 2025, do ano ou de um mês, do NASA POWER (reanálise MERRA-2, NASA), em células de 0,625° × 0,5° (cerca de 55 × 65 km). As células em volta da área, com margem, viram um raster, interpolado (bilinear) para a grade do projeto e recortado pela área. Serve para o país, uma região ou um estado: é um modelo de cerca de 50 km, que não mostra o efeito do relevo nem serve para localizar turbinas; numa área com menos de umas 10 células, um aviso diz isso. Sem login. Precisa de um RustGIS com o Polígonos para raster (raster.rasterize, desde a 0.6.0).",
            "icon": "raster_layer",
            "params": "{\"mes\"?: de 1 a 12, a média desse mês (padrão: a do ano), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "mes", "label": "Período", "kind": "choice", "required": True, "choices": [MEDIA_DO_ANO] + MESES, "default": MEDIA_DO_ANO, "help": "A média do ano, ou a de um mês, de 2001 a 2025."},
            ],
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "power-umidade",
            "label": "Umidade do solo nas raízes (NASA POWER)",
            "help": "A umidade do primeiro metro do solo: a média, o valor de um mês ou ano, ou a anomalia (o quanto ele fugiu da média do mesmo mês ou ano)." + FIM_POWER,
            "icon": "classify",
            "params": PARAMS_POWER,
            "form": CAMPOS_POWER,
        },
    ),
    (
        "Tempo e clima",
        "Clima",
        {
            "id": "power-evapotranspiracao",
            "label": "Evapotranspiração (NASA POWER)",
            "help": "A evapotranspiração (a água que o solo e as plantas devolvem ao ar): a média, o valor de um mês ou ano, ou a anomalia (o quanto ele fugiu da média do mesmo mês ou ano). O aviso traz o total em mm. A do JAXA é mais fina (5,5 km), mas só desde 2025." + FIM_POWER,
            "icon": "dra",
            "params": PARAMS_POWER,
            "form": CAMPOS_POWER,
        },
    ),
    (
        "Água",
        "Recursos hídricos",
        {
            "id": "mapbiomas-irrigacao",
            "label": "Sistemas de irrigação (MapBiomas)",
            "help": "Os sistemas de irrigação mapeados pelo MapBiomas a 30 m com imagens Landsat: pivô central (roxo), outros sistemas, sobretudo no Semiárido (azul-claro), e inundação, o arroz irrigado (rosa). O ano, de 1985 a 2025, vem da janela do comando; num RustGIS sem janelas de plugin, do controle MapBiomas: ano, no painel Área de trabalho (2025 até você mudar). Um mapa para ver, do Brasil inteiro, que vale por algumas horas: o MapBiomas não publica um arquivo dele que se leia pela área. Para a área equipada em números, há a do GMIA v5 (FAO); para os pivôs como polígonos, os da ANA.",
            "icon": "web_map",
            "params": "{\"ano\"?: de 1985 a 2025 (padrão: o do painel, ou 2025)}",
            "form": [
                {"name": "ano", "label": "Ano", "kind": "number", "required": True, "default": 2025, "help": "O ano do mapa, de 1985 a 2025."},
            ],
        },
    ),
    (
        "Água",
        "Recursos hídricos",
        {
            "id": "irrigacao-gmia",
            "label": "Área equipada para irrigação (GMIA v5, FAO)",
            "help": "A porcentagem de cada célula de 5′ (cerca de 9 km) equipada para irrigação, por volta de 2005, do Global Map of Irrigation Areas versão 5 (FAO e Universidade de Bonn). Só a área, lida do arquivo mundial da FAO e recortada pelo contorno, na resolução do dado (fora da grade do projeto); numa grade de células de uns 6 km ou mais, posta nela; a escala de cores vai de 0 ao máximo na área. Para a área irrigada por município, há o Censo Agropecuário 2017 (Agricultura). Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "graduated",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "elevacao",
            "label": "Elevação (Copernicus GLO-30)",
            "help": "Copernicus GLO-30 recortado pela área e posto na grade do projeto (Grade do projeto, no grupo Área). Só a área, lida de cada tile de 1° que ela toca (até 250 por clique: um estado), sem login. Sem grade, o primeiro clique cria a padrão da área. No app web, que ainda não lê só a área, tiles inteiros (até 4).",
            "icon": "relief",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30); com grade, só o da grade é aceito}",
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "gem15",
            "label": "Elevação a 15 m (Google GEM-15)",
            "help": "Os modelos de elevação do Google a 15 m (GEM-15, experimental, de imagens estéreo de satélite até 2024, licença CC-BY 4.0): o terreno nu (DTM), a superfície com a vegetação e as construções (DSM) ou a altura acima do terreno (DSM menos DTM). Só a área, lida de cada célula de 1° que ela toca, posta na grade do projeto e recortada pelo contorno; numa grade mais grossa, lida da pirâmide do arquivo. Alturas no geoide EGM96; água e superfícies sem textura ficam sem dado, e o DTM é pior sob mata densa. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "raster_layer",
            "params": "{\"modelo\"?: \"dtm\" (padrão) | \"dsm\" | \"altura\", \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "modelo", "label": "Modelo", "kind": "choice", "required": True, "choices": MODELOS_GEM15, "default": MODELOS_GEM15[0], "help": "O terreno nu serve ao relevo, às curvas de nível e à hidrologia; a superfície inclui o dossel e os prédios; a altura acima do terreno é a diferença das duas (o dossel e as construções, até uns 50 m)."},
            ],
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "anadem",
            "label": "Terreno da América do Sul a 30 m (ANADEM)",
            "help": "O ANADEM, o modelo digital do terreno da América do Sul a 30 m da ANA e do IPH/UFRGS: o Copernicus GLO-30 sem o viés da vegetação, removido por aprendizado de máquina calibrado para o continente (índices do Landsat 8 e do Sentinel-2, alturas do GEDI e do ICESat-2). Altitudes em metros, no SIRGAS 2000. Só a área, lida do arquivo da América do Sul no S3 público da OpenTopography, posta na grade do projeto e recortada pelo contorno; numa área grande, lida da pirâmide do arquivo. Bom para a hidrologia e o relevo sob a mata, onde o Copernicus mede o dossel. Livre e aberto, com a citação. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "relief",
            "params": "{\"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
        },
    ),
    (
        "Meio físico",
        "Relevo",
        {
            "id": "gedtm-relevo",
            "label": "Relevo e derivadas (GEDTM30)",
            "help": "As variáveis do relevo do GEDTM30 da OpenGeoHub (o terreno global a 30 m feito por aprendizado de máquina com os pontos de laser do ICESat-2 e do GEDI; licença CC BY 4.0), as covariáveis de relevo do mapeamento de solos: a elevação do terreno nu (da cópia da OpenTopography), a declividade, o índice topográfico de umidade, a diferença da elevação média em volta, a rugosidade, as curvaturas (mínima, máxima, de perfil, tangencial e anelar), o índice de forma, as aberturas positiva e negativa, o sombreamento, os geomorfons (dez formas do relevo, de pico a depressão) e o fator LS da equação de perda de solo. Algumas também calculadas sobre o terreno de 480 e 960 m (os geomorfons em todas as escalas, de 30 a 960 m): a escala maior mostra a encosta inteira, não os detalhes de 30 m. As aberturas, o índice de forma e a curvatura anelar só vêm nas escalas maiores: o RustGIS não lê por partes os outros arquivos da OpenGeoHub. Só a área, lida do arquivo global da variável, posta na grade do projeto e recortada pelo contorno; numa grade mais grossa, lida da pirâmide (menos os geomorfons). Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "relief",
            "params": "{\"variavel\"?: \"elevacao\" (padrão) | \"declividade\" | \"twi\" | \"dfme\" | \"rugosidade\" | \"curvatura-minima\" | \"curvatura-maxima\" | \"curvatura-perfil\" | \"curvatura-tangencial\" | \"curvatura-anelar\" | \"indice-forma\" | \"abertura-positiva\" | \"abertura-negativa\" | \"sombreamento\" | \"geomorfons\" | \"fator-ls\", \"escala\"?: 30 (padrão), 60, 120, 240, 480 ou 960 m (a que a variável não tem dá a mais próxima que ela tem, com aviso), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "variavel", "label": "Variável", "kind": "choice", "required": True, "choices": VARIAVEIS_GEDTM, "default": VARIAVEIS_GEDTM[0], "help": "A elevação do terreno nu ou uma variável derivada dela. Para mapear solos, as mais usadas são a declividade, o índice topográfico de umidade, a diferença da elevação média, as curvaturas e os geomorfons."},
                {"name": "escala", "label": "Escala do cálculo", "kind": "choice", "required": True, "choices": ESCALAS_GEDTM, "default": ESCALAS_GEDTM[0], "help": "A célula do terreno de que a variável foi calculada. Hoje o RustGIS lê: 30 m na elevação, na declividade, nas curvaturas de perfil e tangencial, no sombreamento e no fator LS; 30, 480 e 960 m no TWI, na diferença da elevação média, na rugosidade e nas curvaturas mínima e máxima; 240, 480 e 960 m na curvatura anelar; 480 e 960 m nas aberturas e no índice de forma; todas nos geomorfons. Outra escala dá a mais próxima, com aviso."},
            ],
        },
    ),
    (
        "Meio físico",
        "Solos",
        {
            "id": "olm-solo",
            "label": "Propriedades do solo a 30 m (OpenLandMap)",
            "help": "Uma propriedade do solo prevista a 30 m pelo OpenLandMap-soildb (OpenGeoHub; Hengl e outros, Earth System Science Data, 2026; licença CC BY 4.0), por aprendizado de máquina espaço-temporal sobre cerca de 400 mil amostras de solo e covariáveis de relevo, clima e vegetação: o carbono orgânico (g/kg), a densidade de carbono (kg/m³), o pH em água, a densidade do solo (g/cm³) e a argila, a areia e o silte (%), de 0 a 30, de 30 a 60 ou de 60 a 100 cm. O carbono e o pH vêm de 5 em 5 anos, de 2000–2005 a 2020–2022; os outros, de 2020–2022. Só a área, lida do arquivo global, posta na grade do projeto e recortada pelo contorno; numa área grande, lida da pirâmide do arquivo. Previsões de um modelo, não análises de laboratório. Precisa de um RustGIS depois da 0.5.0 (leitura da janela da área).",
            "icon": "stretch",
            "params": "{\"propriedade\"?: \"carbono\" (padrão) | \"densidade-carbono\" | \"ph\" | \"densidade\" | \"argila\" | \"areia\" | \"silte\", \"profundidade\"?: \"0-30\" (padrão) | \"30-60\" | \"60-100\", \"periodo\"?: \"2020-2022\" (padrão) | \"2015-2020\" | \"2010-2015\" | \"2005-2010\" | \"2000-2005\" (os anteriores a 2020 só no carbono e no pH), \"celula\"?: sem grade, o tamanho das células da grade que ela cria, em metros (padrão 30)}",
            "form": [
                {"name": "propriedade", "label": "Propriedade", "kind": "choice", "required": True, "choices": PROPRIEDADES_OLM, "default": PROPRIEDADES_OLM[0], "help": "A propriedade do solo, na unidade dela."},
                {"name": "profundidade", "label": "Profundidade", "kind": "choice", "required": True, "choices": PROFUNDIDADES_OLM, "default": PROFUNDIDADES_OLM[0], "help": "A camada do solo, em cm a partir da superfície."},
                {"name": "periodo", "label": "Período", "kind": "choice", "required": True, "choices": PERIODOS_OLM, "default": PERIODOS_OLM[0], "help": "O carbono e o pH vêm de 5 em 5 anos: compare dois períodos para ver a perda ou o ganho de carbono. A densidade do solo e a textura só têm 2020–2022."},
            ],
        },
    ),
    (
        "Meio físico",
        "Solos",
        {
            "id": "solo-pontos",
            "label": "Pontos de solo: análises de laboratório (SoilData)",
            "help": "As camadas de solo descritas e analisadas nos levantamentos e pesquisas publicados no SoilData, o Repositório Brasileiro de Dados do Solo (UTFPR e MapBiomas), pelo Brazilian Soil Dataset 2024 (Samuel-Rosa e Horst; 47 mil camadas de 14 mil perfis com coordenada, de 1958 a 2024): a argila, a areia, o silte, o carbono orgânico, a CTC, o pH ou a densidade do solo da camada que contém a profundidade escolhida, ou a classe do solo (a ordem do SiBCS) de cada perfil. Medidas de laboratório, não previsões, mas espalhadas de modo desigual: muitas em Rondônia e no Rio Grande do Sul, poucas em outros lugares. Cada ponto tem a licença do conjunto de origem: 40% das camadas são CC BY-NC 4.0 (sem uso comercial). Na janela, o RustGIS baixa o arquivo do Brasil (19,8 MB, uns segundos) e o comando continua sozinho quando ele chega.",
            "icon": "point_layer",
            "params": "{\"atributo\"?: \"argila\" (padrão) | \"areia\" | \"silte\" | \"carbono\" | \"ctc\" | \"ph\" | \"densidade\" | \"classe\", \"profundidade\"?: 5, 10 (padrão), 30, 50 ou 100 cm (a camada que contém essa profundidade; a classe não usa)}",
            "form": [
                {"name": "atributo", "label": "Atributo", "kind": "choice", "required": True, "choices": ATRIBUTOS_SOLO, "default": ATRIBUTOS_SOLO[0], "help": "O que colore os pontos. A argila em classes de textura do SiBCS; o pH nas classes de reação da Embrapa; a classe do solo pela ordem do SiBCS, com os nomes antigos (Podzólico, Terra Roxa...) postos na ordem atual mais próxima."},
                {"name": "profundidade", "label": "Profundidade", "kind": "choice", "required": True, "choices": PROFUNDIDADES_SOLO, "default": PROFUNDIDADES_SOLO[1], "help": "A camada do perfil que contém essa profundidade, a partir da superfície. A classe do solo não usa."},
            ],
        },
    ),
    (
        "Meio físico",
        "Solos",
        {
            "id": "mapbiomas-solo-pontos",
            "label": "Pontos de treino do MapBiomas Solo",
            "help": "As amostras de solo com que o MapBiomas treinou os mapas da Coleção 3 (beta) do MapBiomas Solo, publicadas no SoilData: o estoque de carbono orgânico de 0 a 30 cm de cada coleta, em t/ha, com o ano (cerca de 11 mil pontos de 1985 a 2024; as amostras do Inventário Florestal Nacional com a correção por quantis do MapBiomas), ou a argila, a areia, o silte ou a fração grossa da camada que contém a profundidade escolhida (até 100 cm), com a licença do conjunto de origem. Ficam de fora as pseudoamostras de rocha e de areia do modelo. São os dados de treino, não os mapas. Na janela, o RustGIS baixa o arquivo do Brasil (2,2 MB no carbono, 10,4 MB na textura) e o comando continua sozinho quando ele chega.",
            "icon": "point_layer",
            "params": "{\"dado\"?: \"carbono\" (padrão: estoque de 0 a 30 cm) | \"argila\" | \"areia\" | \"silte\" | \"fracao-grossa\", \"profundidade\"?: 5, 10 (padrão), 30, 50 ou 90 cm (na textura: a camada que contém essa profundidade)}",
            "form": [
                {"name": "dado", "label": "Dado", "kind": "choice", "required": True, "choices": DADOS_MBSOLO, "default": DADOS_MBSOLO[0], "help": "O estoque de carbono de 0 a 30 cm de cada coleta (um ponto por coleta, com o ano), ou um dado da textura de uma camada."},
                {"name": "profundidade", "label": "Profundidade", "kind": "choice", "required": True, "choices": PROFUNDIDADES_MBSOLO, "default": PROFUNDIDADES_MBSOLO[1], "help": "Só na textura: a camada do perfil que contém essa profundidade, a partir da superfície."},
            ],
        },
    ),
]
# Os comandos das abas Sobre, só nos botões do fim do painel (fora da aba do RustGIS).
SOBRE_ABAS = [
    {
        "id": cid,
        "label": rotulo,
        "help": f"Mostra {rotulo.lower()} do BrasilData no fim do painel Área de trabalho.",
        "icon": "about",
        "params": "{}",
    }
    for cid, rotulo in ABAS_SOBRE
]
# Comandos só para scripts e a linha de comando, fora da aba.
SO_SCRIPTS = [
    {
        "id": "indicadores",
        "label": "Indicadores por município",
        "help": "Os indicadores das fontes escolhidas na camada Indicadores por município.",
        "icon": "field",
        "params": "{\"fontes\"?: lista entre censo, agro, agroeco, pix, credito, tse, caged e saude (padrão: todas)}",
    },
]


def ler() -> list[dict]:
    with open(ROOT / "fontes.csv", encoding="utf-8", newline="") as f:
        linhas = list(csv.DictReader(f))
    ids = set()
    for n, l in enumerate(linhas, start=2):
        onde = f"fontes.csv, linha {n}"
        if l["id"] in ids or any(l["id"] == c["id"] for c in AREA + DO_PAINEL + SOBRE_ABAS + SO_SCRIPTS + AMOSTRAGEM + [c for _, c in FIM] + [c for _, _, c in EM_MENUS]):
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



# As colunas do fichas.csv; as quatro últimas só nas camadas que não estão no fontes.csv.
COLUNAS_FICHA = ["id", "o_que_e", "periodo", "resolucao", "unidades", "legenda", "cuidados", "classes", "exemplo", "orgao", "citacao", "licenca", "web"]
DO_FONTES = ["orgao", "citacao", "licenca", "web"]
# As áreas dos mapas exemplo com nome: o município de Viçosa e a mesorregião Zona da Mata (MG),
# para os produtos grossos e os temas que Viçosa não tem; Minas Gerais e o Brasil para os limites
# maiores que elas (uma camada que cobre a área toda não mostra nada).
AREAS_EXEMPLO = {
    "vicosa": ("mun:3171303", "Viçosa (MG)"),
    "zona-da-mata": ("meso:3112", "Zona da Mata (MG)"),
    "minas-gerais": ("uf:MG", "Minas Gerais"),
    "ceara": ("uf:CE", "Ceará"),
    "nordeste": ("reg:2", "Região Nordeste"),
    "barreiras": ("micro:29001", "microrregião de Barreiras (BA)"),
    # retângulos, que não são nós do painel (só nas folhas feitas à parte)
    "mantiqueira": ("", "Serra da Mantiqueira (MG, RJ e SP)"),
    "pocos-de-caldas": ("", "maciço de Poços de Caldas (MG e SP)"),
    "vale-do-aco": ("", "Vale do Aço (MG)"),
    "brasil": ("br", "Brasil"),
}
# Os mapas exemplo com a folha feita à parte no RustGIS (o dado é o do plugin; a folha não é a do botão
# Layout, e o scripts/mapas_exemplo.py não os refaz).
FOLHAS_PROPRIAS = {
    "luzes-noturnas",
    "nisar-cor",
    "elevacao",
    "ana-pivos",
    "mapbiomas-cobertura",
    "vento-50m",
    "metano-tropomi",
    "grace-seca",
    "bam-previsao",
    "gem15",
    "gedtm-relevo",
    "mg-solos",
    "olm-solo",
    "forty-tipologia",
    "worldcover",
    "chm-altura",
    "ctrees-biomassa",
    "drenagem",
    "ana-balanco",
    "censo-setores",
    "gba-edificacoes",
    "osm-vias",
    "enderecos",
    "prodes-amazonia",
    "luzes-mudanca",
}
# Os comandos que não trazem camada de dados: sem ficha.
SEM_FICHA = {"area-selecao", "area-desenho", "area-municipio", "grade", "painel", "escolher-area", "layout", "fontes", "atualizacao", "sobre", "sobre-autoria", "sobre-fontes", "sobre-contribuir", "indicadores", "amostra-aleatoria", "amostra-estratificada", "amostra-sistematica"}
# Onde ficam as fichas no repositório.
PAGINAS = "docs/camadas"
# O texto da ficha no painel antes de a primeira camada chegar.
FICHA_VAZIA = "O que é a última camada que chegou ao mapa: a fonte, o período, a resolução, as unidades, a legenda, a licença, a citação e os cuidados de uso. Traga uma camada para ver a ficha dela aqui; a tabela do botão Fontes tem as fichas de todas as camadas do mapa."


def endereco_da_ficha(cid: str) -> str:
    return f"{REPOSITORIO}/blob/main/{PAGINAS}/{cid}.md"


def municipios_do_plugin() -> dict[str, str]:
    with open(ROOT / "municipios.csv", encoding="utf-8", newline="") as f:
        return {l["codigo"]: f"{l['nome']} ({l['uf']})" for l in csv.DictReader(f)}


def como_foi_feito(cid: str) -> str:
    """Como o mapa exemplo da camada foi feito, para a legenda dele."""
    return "com o dado do plugin e a folha feita à parte no RustGIS" if cid in FOLHAS_PROPRIAS else "feito com o plugin"


def area_do_exemplo(ex: str) -> tuple[str, str] | None:
    """O nó da árvore do painel e o nome da área do mapa exemplo (None: sem mapa)."""
    if not ex:
        return None
    if ex in AREAS_EXEMPLO:
        return AREAS_EXEMPLO[ex]
    return ex, municipios_do_plugin()[ex.removeprefix("mun:")]


def ler_fichas(linhas: list[dict], ids: list[str]) -> dict[str, dict]:
    """As fichas por id, conferidas: uma para cada comando de dados, nem mais nem menos."""
    with open(ROOT / "fichas.csv", encoding="utf-8", newline="") as f:
        leitor = csv.DictReader(f)
        if leitor.fieldnames != COLUNAS_FICHA:
            sys.exit(f"fichas.csv: as colunas são {', '.join(COLUNAS_FICHA)}")
        fichas = list(leitor)
    vetoriais = {l["id"]: l for l in linhas}
    mun = municipios_do_plugin()
    por_id: dict[str, dict] = {}
    for n, fi in enumerate(fichas, start=2):
        onde = f"fichas.csv, linha {n}"
        if None in fi or None in fi.values():
            sys.exit(f"{onde}: a linha não tem {len(COLUNAS_FICHA)} colunas (um texto com vírgula precisa de aspas)")
        if fi["id"] in por_id:
            sys.exit(f"{onde}: o id '{fi['id']}' se repete")
        if fi["id"] not in ids or fi["id"] in SEM_FICHA:
            sys.exit(f"{onde}: '{fi['id']}' não é um comando que traz dados")
        for campo in ("o_que_e", "periodo", "resolucao", "unidades", "legenda", "cuidados"):
            if not fi[campo].strip():
                sys.exit(f"{onde}: {campo} está vazio")
        l = vetoriais.get(fi["id"])
        for campo in DO_FONTES:
            if l and fi[campo]:
                sys.exit(f"{onde}: {campo} de uma camada do fontes.csv vem de lá; deixe vazio")
            if not l and not fi[campo].strip():
                sys.exit(f"{onde}: {campo} está vazio (a camada não está no fontes.csv)")
        ex = fi["exemplo"]
        if ex and ex not in AREAS_EXEMPLO and not (ex.startswith("mun:") and ex[4:] in mun):
            sys.exit(f"{onde}: exemplo é vicosa, zona-da-mata, minas-gerais, brasil, mun:<código do IBGE> ou vazio, não '{ex}'")
        if l:
            web = {"sim": "sim", "nao": "não: só no app desktop (o servidor não aceita pedidos do navegador)", "so-web": "só no app web (o servidor só aceita cifras TLS antigas, que o app desktop recusa)"}[l["web"]]
            fi = dict(fi, orgao=l["fonte"].split(",")[0].strip(), citacao=l["fonte"], licenca=l["licenca"], web=web)
        por_id[fi["id"]] = fi
    faltam = [i for i in ids if i not in SEM_FICHA and i not in por_id]
    if faltam:
        sys.exit(f"fichas.csv: faltam as fichas de {', '.join(faltam)}")
    return por_id


def frase(t: str) -> str:
    """O texto com ponto final."""
    t = t.strip()
    return t if t.endswith((".", "!", "?")) else t + "."


def minuscula(t: str) -> str:
    """O texto com a primeira letra minúscula (depois de dois-pontos), menos numa sigla (SiBCS, CNES)."""
    return t if len(t) < 2 or t[1].isupper() else t[0].lower() + t[1:]


def resumo_da_ficha(fi: dict) -> str:
    """O pedaço da ficha que vai na ajuda do botão (o balão e a Busca de comandos)."""
    return (
        f"Período: {fi['periodo']}. Resolução ou escala: {fi['resolucao']}. Unidades: {fi['unidades']}. "
        f"Licença: {fi['licenca']}. Ficha completa e mapa exemplo: {endereco_da_ficha(fi['id'])}"
    )


def texto_da_ficha(nome: str, fi: dict) -> str:
    """A ficha inteira, em texto (o painel do RustGIS aceita até 4.000 caracteres)."""
    t = "\n".join(
        [
            nome,
            frase(fi["o_que_e"]),
            f"Fonte: {fi['orgao']}.",
            f"Período: {frase(fi['periodo'])}",
            f"Resolução ou escala: {frase(fi['resolucao'])}",
            f"Unidades: {frase(fi['unidades'])}",
            f"Legenda: {frase(fi['legenda'])}",
            f"Cuidados: {frase(fi['cuidados'])}",
            f"Licença: {frase(fi['licenca'])}",
            f"Citação: {frase(fi['citacao'])}",
            f"App web: {frase(fi['web'])}",
            f"Ficha e mapa exemplo: {endereco_da_ficha(fi['id'])}",
        ]
    )
    if len(t) > 4000:
        sys.exit(f"a ficha de {fi['id']} tem {len(t)} caracteres; o painel do RustGIS aceita até 4.000")
    return t


# Os tópicos da aba e os subtópicos de cada um, na ordem em que aparecem (o RustGIS aceita até 8
# grupos e 16 itens por grupo). A aba é conferida contra esta lista: um subtópico que não está
# aqui, ou que ficou vazio, para o gerar.py.
TOPICOS = {
    "Área": ["Amostragem"],
    "Território": ["Limites", "Recortes especiais", "Cadastro fundiário", "Terras indígenas"],
    "Sociedade e economia": ["População", "Economia", "Agricultura", "Eleições", "Saúde"],
    "Meio físico": ["Geologia e recursos minerais", "Relevo", "Solos"],
    "Tempo e clima": ["Previsão", "Chuva observada", "Desastres", "Clima", "Clima passado (ERA5)", "Satélites do JAXA", "Atmosfera"],
    "Água": ["Hidrografia", "Bacias", "Recursos hídricos"],
    "Vegetação e meio ambiente": ["Vegetação", "Cubo de dados do INPE", "Uso e cobertura", "Desmatamento", "Queimadas", "Áreas protegidas", "Fiscalização"],
    "Infraestrutura": ["Energia", "Petróleo e gás", "Transportes", "Edificações"],
}
GRUPOS = list(TOPICOS)
# Comandos que também aparecem num segundo subtópico: (tópico, subtópico, id). A Agricultura
# tem o Censo Agropecuário próprio (censo-agro); os censos por município ficam na População. A
# irrigação do GMIA fica na Água e também na Agricultura, ao lado do Censo Agropecuário. Os mapas de
# seca do GDO ficam no Clima e também nos Solos (a umidade do solo), e as cenas do EMIT também. Os do GRACE
# ficam no Clima e também na Água (a água subterrânea).
TAMBEM = [("Sociedade e economia", "Agricultura", "mapbiomas-irrigacao"), ("Sociedade e economia", "Agricultura", "irrigacao-gmia"), ("Tempo e clima", "Clima", "risco-fogo"), ("Meio físico", "Solos", "gdo-seca"), ("Meio físico", "Solos", "emit-cenas"), ("Água", "Recursos hídricos", "grace-seca")]

# O ícone de um subtópico, quando um dos ícones do RustGIS diz mais que a geometria da primeira
# camada (o plugin só pode usar os ícones do RustGIS, pelo nome).
MENU_ICONES = {
    "Amostragem": "point_tool",
    "Limites": "map",
    "Recortes especiais": "clip",
    "Cadastro fundiário": "measure_area",
    "População": "proportional",
    "Economia": "chart",
    "Agricultura": "unique",
    "Eleições": "count",
    "Geologia e recursos minerais": "bands",
    "Relevo": "relief",
    "Solos": "classify",
    "Saúde": "point_layer",
    "Previsão": "time_next",
    "Chuva observada": "history",
    "Clima": "brightness",
    "Clima passado (ERA5)": "time_slider",
    "Satélites do JAXA": "globe_view",
    "Bacias": "dissolve",
    "Recursos hídricos": "graduated",
    "Vegetação": "dra",
    "Cubo de dados do INPE": "bands",
    "Uso e cobertura": "catalog",
    "Desmatamento": "time_slider",
    "Queimadas": "locate",
    "Áreas protegidas": "buffer",
    "Fiscalização": "clear_sel",
    "Transportes": "directions",
}

# Os ícones do RustGIS que um plugin pode usar: a lista `icons::ALL` do RustGIS 0.5.0 e da 0.6.0
# (crates/ui-egui/src/icons.rs). Um nome fora dela vira o ícone genérico de plugin na aba.
ICONES_DO_RUSTGIS = {
    "about", "add_data", "add_folder", "back", "bands", "basemap", "bookmark", "bookmark_export",
    "bookmark_import", "bookmark_new", "bookmark_update", "brightness", "buffer", "calc_geom",
    "calculate", "catalog", "centroid", "chart", "chevron_down", "chevron_left", "chevron_right",
    "classify", "clear_sel", "clip", "close", "colorbar", "command", "contents", "contrast",
    "count", "create", "dark", "def_query", "delete", "directions", "dissolve", "down", "dra",
    "dropdown", "edit", "environments", "erase", "explore", "export", "export_layer", "export_map",
    "eye", "field", "filter", "gamma", "geoprocessing", "globe_lang", "globe_view", "goto_xy",
    "graduated", "graticule", "help", "history", "hull", "identify", "identity", "intersect",
    "label", "label_off", "label_properties", "line_layer", "line_tool", "locate", "logo",
    "maneuver_arrive", "maneuver_depart", "maneuver_left", "maneuver_merge", "maneuver_right",
    "maneuver_roundabout", "maneuver_sharp_left", "maneuver_sharp_right", "maneuver_slight_left",
    "maneuver_slight_right", "maneuver_straight", "maneuver_uturn", "map", "measure",
    "measure_area", "modelbuilder", "near", "new", "new_layout", "new_map", "next_extent", "open",
    "package", "pane", "pane_right", "planet", "plugin", "plugin_add", "point_layer", "point_tool",
    "polygon_layer", "polygon_tool", "prev_extent", "print", "proportional", "python",
    "raster_layer", "redo", "relief", "resample", "rgb", "run", "save", "save_as", "scale_range",
    "search", "select", "select_attr", "select_loc", "settings", "share", "simplify", "single",
    "split", "stats", "stretch", "symbology", "table", "time_first", "time_last", "time_next",
    "time_off", "time_pause", "time_prev", "time_slider", "toolbox", "tools", "transparency",
    "undo", "union", "unique", "up", "vertices", "web_data", "web_map", "zoom_box", "zoom_full",
    "zoom_in", "zoom_out", "zoom_sel", "zoom_world",
}

# O ícone do RustGIS de cada geometria.
ICONES = {"poligono": "polygon_layer", "linha": "line_layer", "ponto": "point_layer"}


def nomes_dos_comandos(linhas: list[dict]) -> dict[str, str]:
    """O nome de cada comando de dados: o da camada (fontes.csv) ou o do botão, numa linha."""
    nomes = {l["id"]: l["nome"] for l in linhas}
    for c in AREA + DO_PAINEL + SOBRE_ABAS + SO_SCRIPTS + AMOSTRAGEM + [c for _, c in FIM] + [c for _, _, c in EM_MENUS]:
        nomes.setdefault(c["id"], c["label"].replace("\n", " "))
    return nomes


def manifesto(linhas: list[dict], fichas: dict[str, dict]) -> str:
    comandos = AREA + DO_PAINEL + SOBRE_ABAS + SO_SCRIPTS
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
                "help": f"{l['nome']}: {minuscula(frase(fichas[l['id']]['o_que_e']))} Fonte: {l['fonte']}. Só o que cruza a área, {onde}; sem área, o que está na vista.{so_desktop} Cuidados: {frase(fichas[l['id']]['cuidados'])} {resumo_da_ficha(fichas[l['id']])}",
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
        # A ajuda escrita à mão (o que é e como pedir), com o resumo da ficha no fim.
        c = dict(c, help=f"{frase(c['help'])} {resumo_da_ficha(fichas[c['id']])}")
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
    comandos.extend(AMOSTRAGEM)
    grupos["Área"].append({"menu": "Amostragem", "icon": MENU_ICONES["Amostragem"], "items": [c["id"] for c in AMOSTRAGEM]})
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
        # No grupo Área, o painel, a seleção, o desenho e a grade vêm antes dos menus.
        itens.sort(key=lambda i: TOPICOS[g].index(i["menu"]) if isinstance(i, dict) else (-1 if i in NA_ABA_AREA else len(TOPICOS[g])))
    for login, quais in (("earthdata", LOGIN_EARTHDATA), ("copernicus", LOGIN_COPERNICUS)):
        faltam = quais - {c["id"] for c in comandos}
        if faltam:
            sys.exit(f"o login {login} nomeia comandos que não existem: {', '.join(sorted(faltam))}")
        for c in comandos:
            if c["id"] in quais:
                c["login"] = login
    usados = [(c["id"], c["icon"]) for c in comandos] + [("painel", PAINEL["icon"])]
    usados += [(i["menu"], i["icon"]) for itens in grupos.values() for i in itens if isinstance(i, dict)]
    fora = [f"{quem} ({icone})" for quem, icone in usados if icone not in ICONES_DO_RUSTGIS]
    if fora:
        sys.exit(f"ícones que o RustGIS não desenha para um plugin: {', '.join(fora)}")
    if len(grupos) > 8:
        sys.exit(f"a aba tem {len(grupos)} grupos; o RustGIS aceita até 8")
    ficha = {"kind": "text", "id": "ficha", "label": "Ficha da camada", "text": FICHA_VAZIA}
    # Sobre: as abas (botões lado a lado) e, embaixo, o texto da aba clicada; abre na Autoria.
    abas = [{"kind": "button", "command": cid} for cid, _ in ABAS_SOBRE]
    texto_sobre = {"kind": "text", "id": "sobre", "label": f"Sobre o BrasilData: {ABAS_SOBRE[0][1]}", "text": AUTORIA}
    painel = dict(PAINEL, items=PAINEL["items"] + [ficha] + abas + [texto_sobre])
    m = {
        "id": "brasildata",
        "name": "BrasilData",
        "version": VERSAO,
        "description": "Dados públicos do Brasil para a área do usuário (IBGE, MMA, ICMBio, INPE, CEMADEN, ANA, ANEEL, ANP, SFB/CAR, INCRA, FUNAI, IBAMA, ANM, DNIT, Ministério da Saúde, Copernicus, NASA/ASF, FAO, WRI, Open-Meteo, CHIRPS, Google DeepMind): só o que cruza a área, recortado por ela, com a fonte e a licença de cada camada.",
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


def catalogo(linhas: list[dict], fichas: dict[str, dict], nomes: dict[str, str]) -> str:
    s = lambda t: json.dumps(t, ensure_ascii=False)  # um literal de texto do Rust
    out = [
        "//! Gerado por `scripts/gerar.py` a partir do `fontes.csv`: não edite à mão.",
        "",
        "use crate::{Ficha, Tema};",
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
    out += ["", "pub const FICHAS: &[Ficha] = &["]
    for cid, fi in fichas.items():
        out += [
            "    Ficha {",
            f"        id: {s(cid)},",
            f"        o_que_e: {s(frase(fi['o_que_e']))},",
            f"        periodo: {s(fi['periodo'])},",
            f"        resolucao: {s(fi['resolucao'])},",
            f"        unidades: {s(fi['unidades'])},",
            f"        legenda: {s(fi['legenda'])},",
            f"        cuidados: {s(fi['cuidados'])},",
            f"        web: {s(fi['web'])},",
            f"        pagina: {s(endereco_da_ficha(cid))},",
            f"        texto: {s(texto_da_ficha(nomes[cid], fi))},",
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
        "permite pedidos do navegador (CORS); *Recorte*: se a camada é recortada pela área. A ficha de cada camada "
        f"(o que é, período, escala, unidades, legenda, cuidados de uso) e um mapa exemplo estão em [{PAGINAS}]({PAGINAS}/README.md).",
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


def onde_na_aba(linhas: list[dict]) -> dict[str, str]:
    """O caminho de cada comando de dados na aba BrasilData (tópico › subtópico)."""
    onde = {l["id"]: l["grupo"] + (f" › {l['menu']}" if l["menu"] else "") for l in linhas}
    for grupo, menu, c in EM_MENUS:
        onde[c["id"]] = grupo + (f" › {menu}" if menu else "")
    return onde


def paginas(linhas: list[dict], fichas: dict[str, dict], nomes: dict[str, str]) -> dict[pathlib.Path, str]:
    """As páginas docs/camadas/<id>.md, com a ficha e o mapa exemplo, e o índice delas."""
    pasta = ROOT / PAGINAS
    aba = onde_na_aba(linhas)
    cel = lambda t: frase(t).replace("|", "\\|")
    out: dict[pathlib.Path, str] = {}
    indice = [
        "# As camadas do BrasilData",
        "",
        "<!-- Gerado por scripts/gerar.py a partir do fontes.csv e do fichas.csv; não edite à mão. -->",
        "",
        f"A ficha de cada uma das {len(fichas)} camadas que os botões da aba BrasilData trazem: o que é, "
        "a fonte, o período, a resolução, as unidades, a legenda, a licença, a citação, os cuidados de "
        "uso, se funciona no app web, e um mapa exemplo feito com o plugin (o botão Layout). Os mapas "
        "exemplo são de Viçosa (MG), ou da Zona da Mata mineira para os produtos de baixa resolução; os "
        "temas que não existem lá vêm de outro município, dito em cada ficha. Os mapas saem do "
        "`scripts/mapas_exemplo.py`.",
        "",
    ]
    for topico in GRUPOS:
        ids = [i for i in fichas if aba.get(i, "").split(" › ")[0] == topico]
        if not ids:
            continue
        indice += [f"## {topico}", "", "| Camada | Onde na aba | O que é |", "|---|---|---|"]
        for cid in ids:
            indice.append(f"| [{nomes[cid]}]({cid}.md) | {aba[cid]} | {cel(fichas[cid]['o_que_e'])} |")
        indice.append("")
    out[pasta / "README.md"] = "\n".join(indice)
    for cid, fi in fichas.items():
        area = area_do_exemplo(fi["exemplo"])
        imagem = pasta / f"{cid}.png"
        if area and imagem.exists():
            mapa = [f"![Mapa exemplo: {nomes[cid]} em {area[1]}]({cid}.png)", "", f"*Mapa exemplo: {area[1]}, {como_foi_feito(cid)}.*"]
        elif area:
            mapa = [f"*O mapa exemplo, de {area[1]}, ainda não foi gerado: rode `scripts/mapas_exemplo.py {cid}`.*"]
        else:
            mapa = ["*Sem mapa exemplo: a camada não chega pelo `rustgis-cli`, que faz os mapas (veja App web).*"]
        linhas_ficha = [
            ("Onde na aba", aba.get(cid, "")),
            ("Fonte", fi["orgao"]),
            ("Período", fi["periodo"]),
            ("Resolução ou escala", fi["resolucao"]),
            ("Unidades", fi["unidades"]),
            ("Legenda", fi["legenda"]),
            ("Cuidados de uso", fi["cuidados"]),
            ("Licença", fi["licenca"]),
            ("Citação", fi["citacao"]),
            ("App desktop", "sim" if not fi["web"].startswith("só no app web") else "não"),
            ("App web", fi["web"]),
        ]
        out[pasta / f"{cid}.md"] = "\n".join(
            [
                f"# {nomes[cid]}",
                "",
                "<!-- Gerado por scripts/gerar.py a partir do fontes.csv e do fichas.csv; não edite à mão. -->",
                "",
                frase(fi["o_que_e"]),
                "",
                *mapa,
                "",
                "| | |",
                "|---|---|",
                *[f"| **{k}** | {cel(v) if len(v) > 40 else v.replace('|', chr(92) + '|')} |" for k, v in linhas_ficha],
                "",
                "[Todas as camadas](README.md)",
                "",
            ]
        )
    return out


def main() -> None:
    linhas = ler()
    nomes = nomes_dos_comandos(linhas)
    fichas = ler_fichas(linhas, list(nomes))
    arquivos = {
        ROOT / "plugin.json": manifesto(linhas, fichas),
        ROOT / "src" / "catalogo.rs": catalogo(linhas, fichas, nomes),
        ROOT / "README.md": readme(linhas),
        **paginas(linhas, fichas, nomes),
    }
    if "--check" in sys.argv:
        velhos = [str(p.relative_to(ROOT)) for p, t in arquivos.items() if not p.exists() or p.read_text(encoding="utf-8") != t]
        # Uma página de uma camada que saiu do fichas.csv também é velha.
        sobra = [str(p.relative_to(ROOT)) for p in (ROOT / PAGINAS).glob("*.md") if p not in arquivos]
        if velhos or sobra:
            sys.exit(f"desatualizados: {', '.join(velhos + sobra)}; rode python scripts/gerar.py")
        print(f"plugin.json, catalogo.rs, README.md e as {len(fichas)} fichas em dia")
        return
    (ROOT / PAGINAS).mkdir(parents=True, exist_ok=True)
    for p in (ROOT / PAGINAS).glob("*.md"):
        if p not in arquivos:
            p.unlink()
    for p, t in arquivos.items():
        p.write_text(t, encoding="utf-8", newline="\n")
        print("escrito", p.relative_to(ROOT))


if __name__ == "__main__":
    main()
