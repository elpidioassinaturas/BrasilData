# BrasilData

Plugin do [RustGIS](https://github.com/opengeos/RustGIS) que traz dados públicos do Brasil
para a área do usuário: só o que cruza a área, recortado por ela, com a fonte e a licença de cada
camada. Funciona no aplicativo desktop e no app web (WebAssembly), sem instalar mais nada.

*A RustGIS plugin that brings Brazilian public data (IBGE, ANA, INPE, ICMBio, INCRA, ANEEL, ANP,
ANM, IBAMA, DNIT and others) and global datasets read only over the user's area (Copernicus, NASA,
JAXA, ECMWF ERA5, MapBiomas, OpenGeoHub, OpenTopography, Google, Meta, CTrees) into that area,
clipped to it, with each layer's source and licence. It runs on the desktop and in the web app. The
interface is in Portuguese.*

**Página do plugin:** [elpidioassinaturas.github.io/BrasilData](https://elpidioassinaturas.github.io/BrasilData/),
com o catálogo das camadas (busca, filtros, fichas e mapas exemplo) e como instalar.

> **Sentiu falta de algum dado?** O BrasilData cresce com as sugestões de quem usa. Para pedir uma
> fonte nova, [abra uma issue](https://github.com/elpidioassinaturas/BrasilData/issues/new) com o
> nome do dado, o órgão que o publica, o endereço do serviço (WFS, ArcGIS REST, API ou arquivo)
> e para que você o usaria. Se quiser contribuir com a fonte, acrescente uma linha ao
> [`fontes.csv`](fontes.csv), rode `python scripts/gerar.py` e abra um pull request: cada linha
> vira um botão na aba. Correções, avisos de serviço fora do ar e relatos de uso também são
> bem-vindos.
>
> *Missing a dataset? Open an issue with the data, its publisher and its service address, or add a
> line to `fontes.csv` and send a pull request.*

## O que traz

A aba **BrasilData** vai de tópicos a subtópicos e camadas: cada grupo é um tópico, cada menu é
um subtópico e os itens do menu são as camadas, com a fonte entre parênteses. Os tópicos seguem as
categorias temáticas da ISO 19115 e do Perfil MGB da INDE, juntadas para caber nos 8 grupos que o
RustGIS aceita. O grupo Área tem o painel **Área de trabalho**, a Grade do projeto, o comando
Fontes, a verificação de atualização e o **Sobre**: a autoria, a versão, a licença, o repositório,
as fontes dos dados e como contribuir. A janela do botão mostra a autoria ao abrir; no fim do
painel, as abas **Autoria**, **Fontes dos dados** e **Como contribuir** mostram cada parte num clique.

Os comandos que têm escolhas abrem uma **janela** com os seus parâmetros; o **Executar** roda o
comando e mostra embaixo o que ele fez. São eles:

| Comando | Parâmetros na janela |
|---|---|
| Área: município (pela Busca de comandos) | o nome ou o código do IBGE e a UF |
| Grade do projeto | a célula em metros e o sistema (automático, Albers do IBGE, Policônica ou outro código EPSG) |
| Layout | o formato (folha A4, A3 ou A2, figura de 1, 1,5 ou 2 colunas, página ou meia página de tese), a orientação (automática, retrato ou paisagem) e o estilo da folha |
| Amostragem (aleatória, estratificada, sistemática) | o número de pontos ou o espaçamento, o sorteio, os estratos e o arquivo de pontos (mais abaixo) |
| Uso e cobertura e Sistemas de irrigação (MapBiomas) | o ano, e no uso e cobertura, se vem só a área |
| Cobertura da terra 100 m (Copernicus) | o ano, de 2015 a 2019 |
| Cobertura da terra a 10 m (ESA WorldCover) | o ano, 2020 ou 2021 |
| Luzes noturnas | o ano, de 1992 a 2024 |
| Mudança das luzes noturnas | o primeiro e o último ano |
| Cenas hiperespectrais do EMIT (NASA) | o limite de nuvem da cena e o ano |
| NDVI a 300 m, de 10 em 10 dias (Copernicus) | a data (vazia: a mais recente) |
| Índice de vegetação (cubo do INPE) | o satélite (Sentinel-2, Landsat, CBERS-4 WFI ou MODIS), o índice (NDVI ou EVI), o período e o resumo das datas (média, máxima, mínima ou desvio-padrão) |
| Biomassa acima do solo por ano (CTrees) | o ano e se vem a biomassa ou o erro-padrão dela |
| Radar VV, VH e VH/VV (Sentinel-1); HH, HV, HV/HH e composição (NISAR) | a data da passagem (vazio: a mais recente) |
| Perturbação da vegetação (DIST-S1) | a data dos produtos (vazio: os mais recentes) e, na situação, as cores |
| Focos de calor: período | o primeiro e o último dia, no mês corrente |
| Risco de fogo | o mês |
| Radiação solar | a média do ano ou um mês |
| Seca (Copernicus GDO) | o indicador (anomalia da umidade do solo, anomalia da vegetação ou SPI de 1, 3, 6 ou 12 meses) e a data |
| Chuva, temperatura e umidade do solo do ERA5 | o período e, na chuva, o total ou a média por ano; na temperatura, a máxima, a mínima ou a média do dia |
| Satélites do JAXA (chuva, umidade do solo, temperatura da superfície, evapotranspiração, SPI) | o período, o resumo dos meses e, na temperatura, a passagem de dia ou de noite |
| Chuva prevista | o modelo (ECMWF IFS, AIFS, GFS, ICON ou o melhor do Open-Meteo) e os dias |
| Chuva do ensemble | o ensemble (ECMWF ENS, AIFS-ENS ou GEFS), o que mostrar (a chance de passar de um limiar, a média ou os cenários seco e chuvoso), o limiar e os dias |
| Temperatura prevista | a máxima, a mínima ou a média do dia, o modelo e os dias |
| Chuva do mês (CHIRPS) | o mês (vazio: o mais recente) |
| Chuva agora (PARDIG) | a data e a hora, no horário de Brasília (vazio: o quadro mais recente) |
| Elevação a 15 m (GEM-15) | o modelo: terreno nu (DTM), superfície (DSM) ou altura acima do terreno |
| Relevo e derivadas (GEDTM30) | a variável (elevação, declividade, TWI, curvaturas, aberturas, geomorfons e outras) e a escala do cálculo, de 30 a 960 m |
| Propriedades do solo a 30 m (OpenLandMap) | a propriedade (carbono, densidade de carbono, pH, densidade do solo, argila, areia ou silte), a profundidade e o período |
| Pontos de solo (SoilData) | o atributo (argila, areia, silte, carbono orgânico, CTC, pH, densidade do solo ou classe do solo) e a profundidade (5, 10, 30, 50 ou 100 cm) |
| Pontos de treino do MapBiomas Solo | o dado (estoque de carbono de 0 a 30 cm, argila, areia, silte ou fração grossa) e, na textura, a profundidade (5, 10, 30, 50 ou 90 cm) |
| Geologia do SGB | a escala (1:1.000.000, mapa do estado, 1:250.000, 1:100.000, 1:50.000 ou 1:25.000) |

As janelas vêm no RustGIS 0.7.0 ou mais novo, que tem as janelas de comando de plugin
(opengeos/RustGIS#1547). Num RustGIS sem elas, os botões rodam com o que está no painel **Área de
trabalho** (a célula e o sistema da grade, o ano do MapBiomas, o mês do risco de fogo) ou com o
padrão, e um script passa os mesmos parâmetros: `plugin:brasildata/layout={"perfil": "a3", "estilo": "panel"}`.

A área de trabalho pode ser:

- **um limite político**, clicando no nome na árvore **Limites políticos** do painel **Área de
  trabalho**: o Brasil, uma grande região, um estado, uma mesorregião, uma microrregião ou um
  município. O triângulo abre o ramo, e a caixa busca pelo nome. O RustGIS guarda até 5.000 nós
  por árvore, e o Brasil tem 5.570 municípios: os municípios aparecem (e entram na busca) quando
  você escolhe a grande região deles ou qualquer lugar dentro dela. Os limites vêm dentro do plugin
  (`malhas.bin`, da API de malhas do IBGE), e a escolha funciona com o site do IBGE fora do ar ou
  sem internet. Meso e microrregiões são as da divisão do IBGE de 1989. Uma grande região é o
  conjunto dos seus estados; o Brasil é o contorno do país do IBGE, mais simples (cerca de 4.000
  vértices), porque os 27 estados detalhados não cabem numa chamada de plugin. Os municípios vêm
  embutidos na malha intermediária do IBGE, mais leve. No primeiro tema ou raster pedido para um
  município, o RustGIS baixa o limite oficial dele, o da Malha Municipal 2025 (a mesma camada de
  **Limites › Municípios**), e o plugin troca o contorno da área por ele: tudo o que se recorta
  depois segue a divisa oficial. Em Viçosa, a malha intermediária se afastava dela até 815 m. Se o
  WFS do IBGE falhar, vale a malha máxima da API do IBGE (até 144 m da oficial em Viçosa); sem rede,
  a intermediária. Na janela, o limite chega depois do clique: clique de novo. Para o contorno
  oficial de um estado, meso ou microrregião, traga a camada de **Limites** e use a seleção. Por script:
  `plugin:brasildata/escolher-area={"no": "reg:2"}` (`br`, `reg:1` a `reg:5`, `uf:PI`,
  `meso:2202`, `micro:22003`, `mun:2211001`);
- **um desenho**: **Área: desenho** liga a ferramenta (clique nos cantos e dê um duplo clique para
  fechar o polígono) e **Usar o desenho** faz dele a área;
- **a seleção**: os polígonos selecionados da camada ativa, com **Área: seleção**;
- **a vista**: sem área, os temas pedem o que está na vista do mapa.

A **Grade do projeto** é o sistema de coordenadas e o tamanho das células em que os rasters chegam:
- **relevo:** a Elevação, a Elevação a 15 m do GEM-15, o ANADEM e o GEDTM30 com as derivadas;
- **solo:** as propriedades do OpenLandMap;
- **vegetação:** a altura do dossel (CHMv2, CTrees e GEDI), a biomassa (CTrees e GEDI), a tipologia
  florestal (ForTy), o índice de vegetação do cubo do INPE e a cobertura da terra do Copernicus;
- **radar:** o Sentinel-1, o NISAR e a Perturbação da vegetação;
- **clima:** o ERA5, os satélites do JAXA, o CHIRPS, a Radiação solar e o Risco de fogo;
- **água:** a Área equipada para irrigação.

Os arquivos globais (OpenLandMap, GEDTM30, ANADEM, CTrees, ForTy, Copernicus) são lidos só na
janela da área, e numa área grande da pirâmide do arquivo, com células maiores. Quando a célula da
grade é menor que a do dado, o aviso diz que o valor foi reamostrado. O mapa passa a ser desenhado no sistema dela quando você escolhe a área ou faz a grade:
uma projeção de mapa-múndi, como o Eckert IV, deixaria a área deformada (a Bahia, 27% mais
estreita). No painel, escolha a célula (de 10 m a 1 km) e o sistema e clique em **Grade do
projeto**:

- **Automático:** o UTM SIRGAS 2000 do fuso da área, ou o Albers do IBGE (EPSG:10857) quando a
  área passa de um fuso;
- **Policônica:** a Policônica do Brasil, SIRGAS 2000 (EPSG:5880). O RustGIS 0.5.0 ainda não
  projeta rasters nela; as versões seguintes, sim;
- **Albers (IBGE):** a Cônica Equivalente de Albers do IBGE, SIRGAS 2000 (EPSG:10857), que preserva
  as áreas.

A grade fica no mapa como a camada **BrasilData: grade**: o retângulo da área, com o sistema e a
célula nos atributos. Ela é salva com o projeto. Os cantos das células caem nos múltiplos da célula,
então rasters na mesma grade têm as mesmas linhas e colunas. Uma área nova refaz a grade com as
mesmas escolhas. Sem grade, a primeira Elevação cria a padrão: 30 m, sistema automático. Por script:
`plugin:brasildata/grade={"celula": 90, "crs": "albers"}`; o `crs` aceita também um código EPSG em
metros.

A **Amostragem** (menu do grupo Área, e no fim do painel **Área de trabalho**) sorteia pontos
para ir a campo dentro da área de trabalho:

- **Aleatória simples:** os pontos ao acaso, todos com a mesma chance;
- **Estratificada:** os estratos são as classes de uma camada de polígonos do mapa (clique antes
  nela no painel Camadas, por exemplo **BrasilData: Solos**). Cada estrato recebe pontos na
  proporção da sua área dentro da área de trabalho, com pelo menos um, e os pontos caem ao acaso
  dentro dele. O campo das classes é escolhido sozinho (`ordem`, `classe`, `legenda`… antes dos
  outros; códigos e campos só numéricos ficam de fora), e o aviso lista os outros que serviriam;
- **Sistemática:** uma grade de pontos com o espaçamento do painel (de 50 m a 10 km) e a origem
  sorteada, numerada em zigue-zague de norte a sul.

No painel ficam o número de pontos (de 5 a 500), o espaçamento e o número do sorteio: o mesmo
sorteio, na mesma área e com o mesmo método, dá os mesmos pontos, e outro número dá outra
amostra. Cada método tem a sua camada (**BrasilData: amostragem aleatória**, **estratificada** e
**sistemática**), com o rótulo (P01, P02…), o método, o sorteio, o estrato ou o espaçamento, e a
longitude e a latitude em graus (SIRGAS 2000), e a tabela abre embaixo do mapa. Os pontos são
sorteados pelo RustGIS (`analysis.simulate`), uniformes em longitude e latitude: numa área do
tamanho de um estado, a diferença para uniforme na superfície fica abaixo de 3%. Limites: até 500
pontos por amostra, e até 2.000 pontos da grade no retângulo da área. Por script:
`plugin:brasildata/amostra-estratificada={"pontos": 60, "camada": "BrasilData: Solos (BDiA 1:250.000)", "campo": "legenda", "minimo": 2, "sorteio": 3}`
e `plugin:brasildata/amostra-sistematica={"espacamento": 2000}`.

Cada método abre uma **janela** com os seus parâmetros:
- **Aleatória simples:** o número de pontos e o sorteio;
- **Estratificada:** a camada e o campo dos estratos, o número de pontos, o mínimo por estrato e o
  sorteio;
- **Sistemática:** o espaçamento em metros e o sorteio.

Todas têm também o **arquivo de pontos**. Só o nome (`amostragem_aleatoria.gpkg`, o padrão) grava o
arquivo na pasta do projeto; um caminho completo grava onde ele diz. Os formatos são .gpkg, .shp,
.geojson, .csv, .kml e .gpx, este para o GPS. O GeoPackage e o Shapefile saem em SIRGAS 2000, e um
GeoPackage que já existe recebe a amostra nova. Num projeto ainda não salvo, só o nome não tem
pasta: os pontos ficam no mapa, e o aviso pede para salvar o projeto ou dar o caminho completo.
Vazio, os pontos ficam só no mapa. A janela vem num RustGIS com as janelas de comando de plugin
(opengeos/RustGIS#1547); no RustGIS 0.6.0, os botões usam o painel, e o arquivo vai por script:
`plugin:brasildata/amostra-aleatoria={"pontos": 40, "arquivo": "campo.gpx"}`.

O botão **Layout** (grupo Área) monta uma folha pronta para exportar com o dado principal do
BrasilData, que é a camada visível mais acima. A folha traz:
- o nome da área no título e o que é o dado no subtítulo;
- a barra de cores com a unidade, ou a legenda das classes;
- as fontes no rodapé;
- a projeção da grade, a grade em metros e a localização.

Cada tipo de camada recebe o seu tratamento:
- **Indicadores por município e setores censitários:** saem num mapa coroplético da variável
  principal, em 5 classes (quantis com 30 feições ou mais, quebras naturais com menos). O
  subtítulo diz qual é a variável: densidade, estabelecimentos agropecuários, Pix, crédito de
  custeio, abstenção, saldo do CAGED ou mortalidade infantil.
- **Polígonos de referência maiores que a área** (municípios vizinhos, regiões, bacias): ficam com
  um véu claro, o contorno e o nome de cada feição, sem pintar a folha inteira.

Um novo clique refaz a folha. Por script:
`plugin:brasildata/layout={"perfil": "a3", "estilo": "banner"}`.

O **Radar (Sentinel-1, OPERA)**, em Uso e cobertura, traz o retroespalhamento do Sentinel-1
corrigido do relevo (OPERA RTC-S1, 30 m), do ASF DAAC da NASA: a polarização **VV**, que responde
mais à rugosidade e à umidade do solo, a **VH**, que responde mais ao volume da vegetação, e a
**Razão VH/VV**, que separa vegetação de solo exposto (ela deixa as duas polarizações no mapa
também). Como chega:

1. A API de busca do ASF lista os recortes (*bursts*, cerca de 20 × 85 km) que cruzam o retângulo
   da área, os mais recentes primeiro.
2. O RustGIS seleciona os que cruzam o polígono da área (`selectByLocation`), e não só o retângulo
   dela: na Zona da Mata mineira, 38 dos 49 recortes de uma passagem.
3. Vale a passagem mais recente que cobre a área quase tão bem quanto a melhor; por script, a de um
   dia: `plugin:brasildata/radar-razao={"data": "26/10/2026"}`.
4. Cada recorte é projetado na grade do projeto, cortado ao retângulo da área e juntado aos outros;
   o mosaico é recortado pelo polígono da área.

Cada recorte passa pelo filtro de Lee (5 × 5), que tira o granulado (*speckle*) do radar, antes de
ir para a grade, e o resultado chega em **dB** (10 · log10), como o radar costuma ser visto. A razão
VH/VV, em dB, é a diferença VH − VV. O cinza tem limites fixos, os mesmos para qualquer data e
lugar: VV de −10 a −3 dB, VH de −16 a −9 dB e a razão de −8 a −3 dB. A água fica preta, e as
cidades, brancas. O radar pede o **login
Earthdata**, que é grátis: crie um token em [urs.earthdata.nasa.gov](https://urs.earthdata.nasa.gov)
(Generate Token) e cole-o no RustGIS, em Início › Dados web › NASA Earthaccess › Earthdata Login. O
token fica com o RustGIS, que o manda só aos servidores da NASA; o plugin nunca o vê. Na janela, os
recortes (cerca de 7 MB cada) chegam depois do clique: quando aparecerem no mapa, clique de novo.
Áreas de até cerca de 400 km de lado (uma mesorregião). Só no app desktop, por enquanto.

A **Perturbação da vegetação (OPERA DIST-S1)**, em Desmatamento, traz os alertas de perturbação
da vegetação que o JPL tira do mesmo radar: cada passagem do Sentinel-1 é comparada com o histórico
do lugar, e onde o sinal muda além do normal (desmate, queimada, tempestade, colheita) nasce um
alerta, que as passagens seguintes confirmam ou encerram. São quatro camadas:

- **situação**: a cor diz a etapa e, mais escura, a confiança alta.

  | | Primeira detecção | Provisória (2 passagens) | Confirmada (3 ou mais) | Encerrada (voltou ao normal) |
  |---|---|---|---|---|
  | Alta confiança | 4, amarelo | 5, laranja | 6, vermelho-escuro | 8, roxo |
  | Baixa confiança | 1, amarelo-claro | 2, pêssego | 3, salmão | 7, lilás |

  A primeira detecção, ainda não confirmada, é a mais comum (num tile de Viçosa, MG, em
  28/02/2026, 12% da área, contra 0,35% de alertas confirmados) e fica clara, sem cobrir o resto.
  As cores foram escolhidas para continuar distintas com daltonismo (a menor distância de cor entre
  duas classes, ΔE, é 25,6 na visão normal, 17,8 na deuteranopia, 21,5 na protanopia e 9,5 na
  tritanopia). A paleta oficial da OPERA, feita para fundo preto, vem por script:
  `plugin:brasildata/dist-situacao={"paleta": "oficial"}` (nela, a primeira detecção fica
  translúcida, e o 4 e o 7 quase se confundem na protanopia, ΔE 5,9). As manchas vêm também
  como polígonos (**Perturbação da vegetação, manchas**), com a cor da classe, uma borda fina da
  própria cor mais escura, que deixa as manchas claras legíveis sobre um mapa claro, e o nome da
  classe na legenda ("6 · Confirmada, alta confiança"). Elas saem do mosaico cortado só pelo
  retângulo da área e são recortadas depois pelo limite, como os temas: na divisa, seguem a linha,
  e não os degraus de 30 m das células. As manchas com borda saem para áreas de até cerca de 5
  milhões de células da grade (uns 4.500 km² a 30 m, de um município a uma microrregião); numa
  área maior, fica só o raster. O raster, na grade, fica no projeto,
  escondido; a legenda dele mostra só os códigos, porque o RustGIS 0.5.0 ainda não dá nomes às
  classes de um raster. Uma legenda com os
  nomes sai de `scripts/legenda_dist.py <saída> [brasildata|oficial]` (PNG e SVG);
- **data**: quantos dias antes do produto mais recente a perturbação foi vista pela primeira vez;
- **confiança**, que cresce a cada passagem que confirma o alerta (escala fixa de 2,5 a 100);
- **intensidade**: o maior desvio do sinal em relação ao histórico (escala fixa de 2,5 a 12).

Cada produto cobre um tile MGRS (cerca de 110 × 110 km) e acumula os alertas desde o começo do
monitoramento até a data dele: vale o mais recente de cada tile que cruza o polígono da área. Por
script, os produtos até um dia: `plugin:brasildata/dist-situacao={"data": "15/02/2026"}`. Os tiles
vizinhos podem ser de dias diferentes; o nome da camada leva a data do mais recente, e o aviso diz o
intervalo. Como a primeira detecção é a da última passagem de cada tile, os tiles aparecem às vezes
como retângulos na camada da situação. **O ASF só tem produtos de 04/01 a 28/02/2026 por
enquanto** (conferido em 08/11/2026). Como o radar, pede o login Earthdata e é só para o app
desktop; na janela, os tiles (cerca de 2 MB cada na situação e na data, 9 MB na confiança e na
intensidade) chegam depois do clique.

Os **Trechos de drenagem (BHO 2017, ANA)** mostram os rios conforme o zoom. De longe aparecem só os
rios principais; à medida que o mapa aproxima, os menores aparecem também. A hierarquia é a área de
drenagem a montante de cada trecho (`nuareamont`): de 1:20 milhões para longe, os rios com 50 mil
km² ou mais; perto de 1:1 milhão, os com 500 km² ou mais; de 1:300 mil para perto, todos os que
vieram. A ordem de Strahler, que a BHO também traz, não serve para isso: ela cresce com o detalhe
do mapeamento, e o Rio Amazonas (ordem 9) ficaria abaixo de rios do Sudeste de ordem 10. O plugin
traz no máximo 30 mil trechos: numa área grande, só os rios a partir de uma área de drenagem (no
Brasil inteiro, 10 mil km²), e o aviso diz qual; numa área menor, vêm os rios menores.

O estilo que você der a uma camada do BrasilData fica com ela: a legenda (o campo, as classes, a
paleta), as cores, os rótulos e a transparência. Quando o tema vem de novo, para outra área ou outra
vista, a camada nova recebe o estilo da antiga; o estilo também fica salvo no projeto, com a camada.
Numa legenda por classes, os limites das classes ficam como estavam: para recalculá-los com os dados
novos, aplique a simbologia de novo. Os rasters (MapBiomas, relevo, radar, perturbação) ainda não guardam o
estilo.

| Tópico | Subtópicos |
|---|---|
| Território | Limites (IBGE: Brasil, Grandes Regiões, estados, regiões intermediárias e imediatas, meso e microrregiões, municípios, distritos, subdistritos, bairros e localidades), Recortes especiais (IBGE: Amazônia Legal, Semiárido, MATOPIBA, SUDENE, municípios costeiros, regiões metropolitanas, aglomerações, concentrações e arranjos urbanos, hierarquia urbana; zonas do ZEE-MG), Cadastro fundiário (SFB, INCRA), Terras indígenas (FUNAI) |
| Sociedade e economia | População (IBGE: setores censitários, censos, áreas de ponderação, área urbana e rural, favelas e comunidades urbanas), Economia (Pix do Banco Central, Novo CAGED), Agricultura (crédito rural do Banco Central, Censo Agropecuário e agroecologia do IBGE; sistemas de irrigação do MapBiomas; área equipada para irrigação do GMIA v5, FAO), Eleições (TSE), Saúde (estabelecimentos do CNES; indicadores do SIM, do SINASC e do CNES, DATASUS) e Todos os indicadores |
| Meio físico | Geologia e recursos minerais (IBGE, ANM, Feam, MapBiomas; geologia do SGB em seis escalas, nas cores do SGB, ocorrências minerais, afloramentos, geocronologia, petrografia, fósseis, bacias sedimentares e geodiversidade), Riscos geológicos (setores de risco e suscetibilidade a inundação, movimento de massa, enxurrada e corrida de massa, do SGB), Relevo (geomorfologia do IBGE; elevação do Copernicus GLO-30; GEM-15 do Google a 15 m; terreno da América do Sul do ANADEM, ANA e IPH/UFRGS; GEDTM30 da OpenGeoHub, com a elevação e 15 derivadas, como declividade, TWI, curvaturas, aberturas e geomorfons), Solos (IBGE; solos, aptidão, infiltração, água disponível e erosão de MG, Sisema e ZEE-MG; propriedades do solo a 30 m do OpenLandMap-soildb; pontos de solo com as análises de laboratório do SoilData, UTFPR e MapBiomas, e os pontos de treino do MapBiomas Solo; seca do Copernicus GDO) |
| Tempo e clima | Previsão (pelo Open-Meteo: a chuva prevista do ECMWF IFS, do AIFS, do GFS e do ICON, a chance e a faixa provável de chuva dos ensembles ECMWF ENS, AIFS-ENS e GEFS, e a temperatura prevista), Chuva observada (o total do mês do CHIRPS v3; a taxa de chuva de agora do PARDIG, Google DeepMind; a chuva medida nas estações do CEMADEN), Desastres (alertas vigentes do CEMADEN), Clima (seca do Copernicus GDO; radiação solar, umidade do solo nas raízes e evapotranspiração do NASA POWER; climatologia do Risco de Fogo do INPE), Clima passado (chuva, temperatura e umidade do solo da reanálise ERA5, desde 1940), Satélites do JAXA (chuva do GSMaP, umidade do solo do AMSR2, temperatura da superfície e evapotranspiração do GCOM-C, SPI) |
| Água | Hidrografia (IBGE, ANA), Bacias (IBGE, ANA, IGAM), Recursos hídricos (ANA; poços tubulares do SIAGAS, SGB; estresse hídrico do Aqueduct 4.0, WRI; outorgas e usos insignificantes de MG, IGAM; sistemas de irrigação do MapBiomas; área equipada para irrigação do GMIA v5, FAO) |
| Vegetação e meio ambiente | Vegetação (IBGE; altura do dossel do GEDI, NASA, do CHMv2 a 1 m, Meta e WRI, e da Amazônia a 4,78 m, CTrees; biomassa do GEDI e da CTrees por ano; tipologia florestal 2020 a 10 m do ForTy), Cubo de dados do INPE (NDVI e EVI do Sentinel-2, Landsat, CBERS-4 e MODIS), Uso e cobertura (MapBiomas; cobertura da terra 100 m do Copernicus; radar Sentinel-1 e NISAR do ASF/NASA), Desmatamento (INPE: PRODES e DETER; supressões autorizadas em MG, IEF; alertas de perturbação OPERA DIST-S1 do ASF/NASA), Queimadas (INPE: focos de calor das últimas 48 horas, dos últimos 7 dias, do mês corrente e de um período, e a climatologia mensal do Risco de Fogo), Áreas protegidas (MMA, ICMBio, IEF), Fiscalização (IBAMA; licenças ambientais de MG, Semad) |
| Infraestrutura | Energia (ANEEL), Petróleo e gás (ANP), Transportes (DNIT), Edificações (polígonos e alturas do GlobalBuildingAtlas) |

As camadas vetoriais estão listadas em [`fontes.csv`](fontes.csv) e na tabela [Fontes](#fontes),
abaixo: serviço, camada, fonte, licença e se a camada abre no app web. O comando **Fontes**
põe a citação e a licença de cada camada trazida na tabela **BrasilData: fontes**, que abre embaixo
do mapa e fica salva com o projeto. Os indicadores são agregados por município: o plugin não traz CPF, CNPJ ou
nomes de pessoas.

Algumas camadas só abrem num dos aplicativos, e a ajuda de cada botão diz quando é o caso:
- 19 das 99 camadas vetoriais só abrem no app desktop, porque o servidor não manda os cabeçalhos CORS que o navegador exige; dos rasters, o cubo do INPE, os satélites do JAXA e o ForTy também só abrem no desktop, e a ficha de cada um diz se abre no app web;
- os imóveis do CAR só abrem no app web, porque o servidor só aceita cifras TLS antigas, que o app desktop não aceita.

## Fontes

<!-- fontes: início (gerado por scripts/gerar.py a partir do fontes.csv; não edite à mão) -->
117 camadas de 16 fontes, na ordem do `fontes.csv`. *App web*: se o servidor permite pedidos do navegador (CORS); *Recorte*: se a camada é recortada pela área. A ficha de cada camada (o que é, período, escala, unidades, legenda, cuidados de uso) e um mapa exemplo estão em [docs/camadas](docs/camadas/README.md).

| Grupo | Camada | Fonte | Serviço | Licença | App web | Recorte |
|---|---|---|---|---|---|---|
| Território › Limites | Brasil (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_000_brasil_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Grandes Regiões (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_010_grandreg_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Estados (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_020_uf_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Regiões geográficas intermediárias (IBGE 2025) | IBGE, Divisão Regional do Brasil em Regiões Geográficas (2017), na Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_120_reggeoginterm_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Regiões geográficas imediatas (IBGE 2025) | IBGE, Divisão Regional do Brasil em Regiões Geográficas (2017), na Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_110_reggeogimed_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Mesorregiões geográficas (IBGE, divisão de 1989) | IBGE, Divisão do Brasil em Mesorregiões e Microrregiões Geográficas (1989-1990), na Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_170_mesoreg_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Microrregiões geográficas (IBGE, divisão de 1989) | IBGE, Divisão do Brasil em Mesorregiões e Microrregiões Geográficas (1989-1990), na Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_180_microreg_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Municípios (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_030_munic` | uso livre com citação da fonte | sim | não |
| Território › Limites | Distritos (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_630_distrito_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Subdistritos (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_640_subdistrito_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Bairros (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_650_bairro_agreg` | uso livre com citação da fonte | sim | não |
| Território › Limites | Localidades (IBGE, BC250 2025) | IBGE, Base Cartográfica Contínua do Brasil 1:250.000 (BC250), versão 2025, Posicionamento Geográfico de Localidade | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_lml_posic_geo_localidade_p` | uso livre com citação da fonte | sim | sim |
| Meio físico › Geologia e recursos minerais | Geologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geol_area` | uso livre com citação da fonte | sim | sim |
| Meio físico › Relevo | Geomorfologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geom_area` | uso livre com citação da fonte | sim | sim |
| Meio físico › Solos | Solos (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:pedo_area` | uso livre com citação da fonte | sim | sim |
| Meio físico › Solos | Mapa de solos de Minas Gerais (UFV, Cetec, UFLA e Feam, 2010) | IDE-Sisema, Mapa de solos do Estado de Minas Gerais (UFV, Cetec, UFLA e Feam, 2010) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1502_mg_mapa_solos_pol` | licença não declarada no serviço | sim | sim |
| Meio físico › Solos | Aptidão agrícola das terras de MG (Embrapa, 1:500.000) | IDE-Sisema, Aptidão agrícola das terras de Minas Gerais (Embrapa, 1:500.000) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1504_mg_solos_aptidao_agricola_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Meio físico › Solos | Capacidade de infiltração dos solos de MG (SGB/CPRM) | IDE-Sisema, Capacidade de infiltração dos solos, do Mapa Hidrogeológico de Minas Gerais (SGB/CPRM, 1:500.000) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1504_mg_infil_solos_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Meio físico › Solos | Capacidade de água disponível nos solos de MG (ANA e UFPR) | IDE-Sisema, Capacidade de água disponível nos solos (ANA e UFPR, Atlas Irrigação) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1504_mg_capacidade_agua_solos_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Meio físico › Solos | Vulnerabilidade do solo à erosão (ZEE-MG) | IDE-Sisema, Zoneamento Ecológico-Econômico de Minas Gerais (UFLA e Sisema, 2005 a 2008) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2401_mg_vulnerabilidade_solos_erosao_pol` | licença não declarada no serviço | sim | sim |
| Meio físico › Solos | Risco à erosão (ZEE-MG) | IDE-Sisema, Zoneamento Ecológico-Econômico de Minas Gerais (UFLA e Sisema, 2005 a 2008) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2401_mg_risco_potencial_erosao_pol` | licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Vegetação | Vegetação (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:vege_area` | uso livre com citação da fonte | sim | sim |
| Água › Hidrografia | Drenagem (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025; ordem de Strahler e drenagem a montante: BrasilData | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_trecho_drenagem_l` | uso livre com citação da fonte | sim | sim |
| Água › Hidrografia | Massas d'água (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_massa_dagua_a` | uso livre com citação da fonte | sim | sim |
| Água › Bacias | Ottobacias (BHO 2017 1:50.000, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017 1:50.000), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/SPR/BHO2017_50K_AREADRENAGEM/FeatureServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Hidrografia | Trechos de drenagem (BHO 2017, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/Atlas2020/Bho_2017_BR/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Bacias | Unidades de planejamento hídrico (UPH, ANA) | ANA, Unidades de Planejamento Hídrico (UPH), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/DADOSABERTOS/Unidade_de_Planejamento_H%C3%ADdrico_UPH/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | não |
| Água › Bacias | Macrorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_840_MacroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Mesorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_835_MesoRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Microrregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_830_MicroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Circunscrições hidrográficas de MG (IGAM) | IDE-Sisema, Circunscrições Hidrográficas de Minas Gerais (IGAM) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1108_mg_circunscricoes_hidrograficas_pol` | licença não declarada no serviço | sim | sim |
| Água › Recursos hídricos | Disponibilidade hídrica superficial (ANA) | ANA, Disponibilidade Hídrica Superficial, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Disponibilidade_Hidrica_Superficial/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Balanço hídrico quantitativo por curso d'água (ANA) | ANA, Balanço Hídrico Quantitativo por Curso de Água, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Balanco_Hidrico_Quantitativo_por_Curso_de_Agua/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Estações da Rede Hidrometeorológica Nacional (ANA) | ANA, Rede Hidrometeorológica Nacional, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Rede_Hidrometeorologica_Nacional/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Captações superficiais (ANA) | ANA, Captações Superficiais, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Captacoes_Superficiais/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Pivôs centrais mapeados (ANA) | ANA, Pivôs Mapeados, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Pivos_Mapeados/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Municípios atingidos por seca em 2015 (ANA) | ANA, Municípios Atingidos por Eventos de Seca em 2015, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Municipios_Atingidos_por_Eventos_de_Seca_em_2015/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Estresse hídrico de referência (Aqueduct 4.0, WRI) | Kuzma, S., M.F.P. Bierkens, S. Lakshman, T. Luo, L. Saccoccia, E.H. Sutanudjaja e R. Van Beek. 2023. Aqueduct 4.0: Updated decision-relevant global water risk indicators. Technical Note. Washington, DC: World Resources Institute. doi:10.46830/writn.23.00061. Estresse hídrico de referência (bws): retirada de água sobre a oferta renovável, por sub-bacia HydroBASINS de nível 6 e unidade administrativa, pelo serviço Aqueduct Water Risk (Baseline Annual) do ArcGIS Living Atlas (Esri). Campos: bws_raw, a razão; bws_score, de 0 a 5; bws_cat, a categoria (-1 árido e de pouco uso, -9999 sem dado); estresse, a categoria em português | [ArcGIS REST](https://services.arcgis.com/P3ePLMYs2RVChkJx/arcgis/rest/services/aqueduct_water_risk/FeatureServer) `1` | CC BY 4.0 (WRI) | sim | sim |
| Água › Recursos hídricos | Outorgas de uso de recursos hídricos de MG (IGAM) | IDE-Sisema, Outorgas de direito de uso de recursos hídricos de Minas Gerais (IGAM, 01/1989 a 04/2024) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2103_mg_outorgas_uso_recursos_hidricos_pto` | licença não declarada no serviço | sim | sim |
| Água › Recursos hídricos | Usos insignificantes de recursos hídricos de MG (IGAM) | IDE-Sisema, Cadastros de uso insignificante de recursos hídricos de Minas Gerais (IGAM, 01/2021 a 03/2025) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2103_mg_uso_insignificante_recursos_hidricos_pto` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação (CNUC) | MMA, Cadastro Nacional de Unidades de Conservação (CNUC), na cópia publicada pela Infra S.A. (geo.infrasa.gov.br); a do MMA e do ICMBio, na INDE (geoservicos.inde.gov.br), está fora do ar desde 26/11/2026 | [ArcGIS REST](https://geo.infrasa.gov.br/server/rest/services/Hosted/ucs_cnuc/FeatureServer) `0` | não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação federais (CNUC) | MMA, Cadastro Nacional de Unidades de Conservação (CNUC), as de esfera federal (geridas pelo ICMBio), na cópia publicada pela Infra S.A.; a do MMA e do ICMBio, na INDE (geoservicos.inde.gov.br), está fora do ar desde 26/11/2026 | [ArcGIS REST](https://geo.infrasa.gov.br/server/rest/services/Hosted/ucs_cnuc/FeatureServer) `0` | não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação estaduais de MG (IEF) | IDE-Sisema, Unidades de conservação estaduais de Minas Gerais (IEF) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2010_mg_unidades_conservacao_estaduais_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação municipais de MG (IEF) | IDE-Sisema, Unidades de conservação municipais de Minas Gerais (IEF) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2010_mg_unidades_conservacao_municipais_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Território › Terras indígenas | Terras indígenas (FUNAI, via Infra S.A.) | FUNAI, Terras Indígenas (cópia publicada pela Infra S.A.) | [ArcGIS REST](https://geo.infrasa.gov.br/server/rest/services/Terras_Ind%C3%ADgenas/MapServer) `0` | reprodução permitida citando a fonte (FUNAI) | sim | sim |
| Território › Cadastro fundiário | Imóveis rurais (CAR) | Serviço Florestal Brasileiro, Cadastro Ambiental Rural (SICAR); dados autodeclarados, sem os cancelados | [WFS](https://geoserver.car.gov.br/geoserver/sicar/wfs) `sicar:sicar_imoveis_{uf}` | licença não declarada no serviço | só no app web | sim |
| Território › Cadastro fundiário | Parcelas certificadas (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Território › Cadastro fundiário | Parcelas particulares (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Território › Cadastro fundiário | Assentamentos e lotes (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Território › Cadastro fundiário | Glebas públicas (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Cerrado) | INPE, PRODES Cerrado (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-cerrado-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Caatinga) | INPE, PRODES Caatinga (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-caatinga-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Amazônia Legal) | INPE, PRODES Amazônia Legal (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-legal-amz:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Mata Atlântica) | INPE, PRODES Mata Atlântica (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-mata-atlantica-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Pantanal) | INPE, PRODES Pantanal (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-pantanal-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Desmatamento anual (PRODES Pampa) | INPE, PRODES Pampa (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-pampa-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Alertas de desmatamento (DETER Cerrado) | INPE, DETER Cerrado (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `deter-cerrado-nb:deter_cerrado` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Alertas de desmatamento (DETER Amazônia) | INPE, DETER Amazônia (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `deter-amz:deter_amz` | CC BY-SA 4.0 | sim | sim |
| Vegetação e meio ambiente › Desmatamento | Áreas autorizadas para supressão de vegetação em MG (IEF, desde 07/2022) | IDE-Sisema, Áreas autorizadas para intervenção ambiental com supressão de vegetação nativa (IEF, Feam e municípios conveniados, desde 07/2022) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2105_mg_area_autorizada_intervencao_ambiental_ief_pol` | uso irrestrito (licenceUnrestricted nos metadados da IDE-Sisema) | sim | sim |
| Vegetação e meio ambiente › Fiscalização | Áreas embargadas (IBAMA) | IBAMA, Termos de embargo (SISCOM, PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/SISCOM/publico/MapServer) `3` | dados abertos; licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Fiscalização | Licenças ambientais emitidas em MG (Semad, SLA) | IDE-Sisema, Licenças ambientais do Sistema de Licenciamento Ambiental (Semad, desde 11/2019) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2101_mg_empreendimentos_licenciados_pto` | licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Queimadas | Focos de calor das últimas 48 horas (INPE) | INPE, Programa Queimadas, focos de calor do mês corrente (Sisfogo, IBAMA/PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/sisfogo_focos/focos_inpe_br_municipios_ibge_2022_focos_datetime_mes_atual/FeatureServer) `0` | dados abertos; licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Queimadas | Focos de calor dos últimos 7 dias do mês (INPE) | INPE, Programa Queimadas, focos de calor do mês corrente (Sisfogo, IBAMA/PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/sisfogo_focos/focos_inpe_br_municipios_ibge_2022_focos_datetime_mes_atual/FeatureServer) `0` | dados abertos; licença não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Queimadas | Focos de calor do mês corrente (INPE) | INPE, Programa Queimadas, focos de calor do mês corrente (Sisfogo, IBAMA/PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/sisfogo_focos/focos_inpe_br_municipios_ibge_2022_focos_datetime_mes_atual/FeatureServer) `0` | dados abertos; licença não declarada no serviço | sim | sim |
| Infraestrutura › Energia | Usinas hidrelétricas (ANEEL) | ANEEL, SIGEL, Usinas Hidrelétricas - UHE | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `2` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Energia | Pequenas centrais hidrelétricas (ANEEL) | ANEEL, SIGEL, Pequenas Centrais Hidrelétricas - PCH | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `3` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Energia | Centrais geradoras hidrelétricas (ANEEL) | ANEEL, SIGEL, Centrais Geradoras Hidrelétricas - CGH | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `4` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Energia | Centrais geradoras eólicas (ANEEL) | ANEEL, SIGEL, Central Geradora Eólica - EOL | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Energia | Centrais solares fotovoltaicas (ANEEL) | ANEEL, SIGEL, Centrais Geradoras Solares Fotovoltaicas - UFV | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `21` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Energia | Usinas termelétricas (ANEEL) | ANEEL, SIGEL, Usinas Termelétricas - UTE | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `18` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Bacias sedimentares (ANP) | ANP, GeoMaps, Bacias Sedimentares | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:bacias_gishub_db` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Blocos exploratórios (ANP) | ANP, GeoMaps, Blocos Exploratórios | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:bloco_view_geoanp` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Campos de produção (ANP) | ANP, GeoMaps, Campos de Produção | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:campos_gishub_db` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Poços de petróleo e gás (ANP) | ANP, GeoMaps, Poços | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:well_view_geoanp` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Gasodutos de transporte (ANP) | ANP, GeoMaps, Gasodutos de Transporte | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:gasodutos_de_transporte` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Oleodutos de transporte (ANP) | ANP, GeoMaps, Oleodutos de Transporte | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:oleodutos_de_transporte` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Infraestrutura › Petróleo e gás | Postos de combustível (ANP) | ANP, GeoMaps, Postos de Combustível | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:postos_de_combustivel` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Meio físico › Geologia e recursos minerais | Processos minerários ativos (ANM) | ANM, SIGMINE, processos minerários ativos | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `0` | dados abertos, com crédito à ANM | sim | sim |
| Meio físico › Geologia e recursos minerais | Áreas de bloqueio (ANM) | ANM, SIGMINE, áreas de bloqueio | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `3` | dados abertos, com crédito à ANM | sim | sim |
| Meio físico › Geologia e recursos minerais | Reservas garimpeiras (ANM) | ANM, SIGMINE, reservas garimpeiras | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `4` | dados abertos, com crédito à ANM | sim | sim |
| Meio físico › Geologia e recursos minerais | Barragens de mineração (ANM, SIGBM) | ANM, Sistema Integrado de Gestão de Barragens de Mineração (SIGBM), painel público | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/Producao/Barragens_Dashboard_Publico/FeatureServer) `0` | dados abertos, com crédito à ANM | sim | sim |
| Meio físico › Geologia e recursos minerais | Barragens de rejeitos e resíduos de MG (Feam, ano base 2024) | IDE-Sisema, Barragens da Política Estadual de Segurança de Barragens (Feam, ano base 2024) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_1901_mg_barragens_rejeitos_residuos_pto` | licença não declarada no serviço | sim | sim |
| Infraestrutura › Transportes | Rodovias e estradas (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_rod_trecho_rodoviario_l` | uso livre com citação da fonte | sim | sim |
| Infraestrutura › Transportes | Rodovias federais (SNV vigente, DNIT) | DNIT, Sistema Nacional de Viação (SNV), versão vigente | [WFS](https://servicos.dnit.gov.br/dnitgeo/geoserver/ows) `vgeo:vw_snv_rod` | sem restrições declaradas | sim | sim |
| Meio físico › Geologia e recursos minerais | Área minerada por ano (MapBiomas, 1985–2024) | MapBiomas, Monitor da Mineração, área minerada, 1985 a 2024 (campos: ano, substância, se está num processo do SIGMINE, se está em área restrita e área) | [WFS](https://plataforma.geoserver.mapbiomas.org/geoserver/pto/ows) `pto:mining_age` | CC BY 4.0, pelos termos do MapBiomas; não declarada no serviço | sim | sim |
| Território › Recortes especiais | Amazônia Legal (IBGE 2024) | IBGE, Amazônia Legal, Malha Municipal 2024 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2024_040_amazleg_agreg` | uso livre com citação da fonte | sim | sim |
| Território › Recortes especiais | Semiárido (IBGE 2025) | IBGE, Semiárido, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_080_semiarido_agreg` | uso livre com citação da fonte | sim | sim |
| Território › Recortes especiais | MATOPIBA (IBGE 2025) | IBGE, MATOPIBA, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_190_matopiba_agreg` | uso livre com citação da fonte | sim | sim |
| Território › Recortes especiais | Área de atuação da SUDENE (IBGE 2025) | IBGE, Área de atuação da SUDENE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_070_areasudene_agreg` | uso livre com citação da fonte | sim | sim |
| Território › Recortes especiais | Municípios costeiros (IBGE 2025) | IBGE, Municípios costeiros, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_280_municipioscosteiros_agreg` | uso livre com citação da fonte | sim | sim |
| Território › Recortes especiais | Recorte metropolitano (IBGE 2025) | IBGE, Recorte metropolitano, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_211_recortemetrop_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Aglomerações urbanas (IBGE 2025) | IBGE, Aglomerações urbanas, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_230_aglomurb_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Concentrações urbanas (IBGE 2025) | IBGE, Concentrações urbanas, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_090_concurb_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Arranjos populacionais (IBGE 2025) | IBGE, Arranjos populacionais, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_100_arranjpop_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Hierarquia urbana das cidades (IBGE 2025) | IBGE, Hierarquia urbana das cidades, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_200_hierarqurb_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Zonas ecológico-econômicas (ZEE-MG) | IDE-Sisema, Zoneamento Ecológico-Econômico de Minas Gerais (UFLA e Sisema, 2005 a 2008) | [WFS](https://geoserver.meioambiente.mg.gov.br/wfs) `IDE:ide_2401_mg_zonas_ecologico_economicas_pol` | licença não declarada no serviço | sim | sim |
| Sociedade e economia › População | Áreas de ponderação (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_710_apond_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Área urbana para fins censitários (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_705_areaurbanapormunicipiopfc_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Área rural para fins censitários (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_700_arearuralpormunicipiopfc_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Favelas e comunidades urbanas (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_670_fcu_agreg` | uso livre com citação da fonte | sim | não |
| Meio físico › Geologia e recursos minerais | Ocorrências e recursos minerais (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Ocorrências e recursos minerais | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:ocorrencias_recursos_minerais` | domínio público (opendata.sgb.gov.br: livre para uso) | sim | sim |
| Meio físico › Geologia e recursos minerais | Afloramentos descritos em campo (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Afloramentos | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:afloramentos` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geologia e recursos minerais | Análises geocronológicas (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Geocronologia | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:geocronologia` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geologia e recursos minerais | Lâminas petrográficas (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Petrografia | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:petrografia` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geologia e recursos minerais | Ocorrências fossilíferas (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Ocorrências fossilíferas | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:ocorrencias_fossiliferas` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geologia e recursos minerais | Bacias sedimentares (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Bacias sedimentares | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `geosgb:bacias_sedimentares` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geologia e recursos minerais | Geodiversidade do Brasil, 1:1.000.000 (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Mapa de Geodiversidade do Brasil, 1:1.000.000 | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:geodiversidade_1m` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Riscos geológicos | Setores de risco geológico (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Setorização de áreas de risco geológico | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:risco_geologico` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Riscos geológicos | Suscetibilidade a inundação (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Cartas de suscetibilidade a movimentos gravitacionais de massa e inundações | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:suscet_inundacao` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Riscos geológicos | Suscetibilidade a movimento de massa (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Cartas de suscetibilidade a movimentos gravitacionais de massa e inundações | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:suscet_movimento_de_massa` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Riscos geológicos | Bacias suscetíveis a enxurrada (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Cartas de suscetibilidade a movimentos gravitacionais de massa e inundações | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:suscet_enxurrada` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Riscos geológicos | Bacias suscetíveis a corrida de massa (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Cartas de suscetibilidade a movimentos gravitacionais de massa e inundações | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `gestao-territorial:suscet_corrida_de_massa` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Água › Recursos hídricos | Poços do SIAGAS (SGB) | SGB (Serviço Geológico do Brasil), SIAGAS, Sistema de Informações de Águas Subterrâneas | [WFS](https://geoservicos.sgb.gov.br/geoserver/ows) `hidrogeologia:siagas` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geofísica | Estações gravimétricas (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Levantamentos geofísicos terrestres: gravimetria | [ArcGIS REST](https://geoportal.sgb.gov.br/server/rest/services/geofisica/geofisica_terrestre/MapServer) `1` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geofísica | Estações magnetotelúricas (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Levantamentos geofísicos terrestres: magnetotelúrico | [ArcGIS REST](https://geoportal.sgb.gov.br/server/rest/services/geofisica/geofisica_terrestre/MapServer) `2` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geofísica | Levantamentos hiperespectrais (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Levantamentos hiperespectrais aerotransportados | [ArcGIS REST](https://geoportal.sgb.gov.br/server/rest/services/geofisica/levantamento_hiperespectral/MapServer) `0` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
| Meio físico › Geofísica | Cartas de anomalias 1:100.000 (SGB) | SGB (Serviço Geológico do Brasil), GeoSGB, Cartas de anomalias aerogeofísicas 1:100.000 | [ArcGIS REST](https://geoportal.sgb.gov.br/server/rest/services/Cartas_de_Anomalia/MapServer) `0` | dados abertos do SGB; licença não declarada no serviço | sim | sim |
<!-- fontes: fim -->

Fora da tabela, com a citação no comando **Fontes**:
- **Elevação:** Copernicus DEM GLO-30 (© DLR e.V. e © Airbus Defence and Space GmbH, sob o
  programa Copernicus da União Europeia e da ESA), lido do
  [Planetary Computer](https://planetarycomputer.microsoft.com/dataset/cop-dem-glo-30) da Microsoft,
  só a janela da área de cada tile de 1°, sem login; no app web, até 4 tiles inteiros;
- **Elevação a 15 m:** GEM-15, Near-Global Surface and Terrain Elevation Models at 15 m, v1, do
  Google Research (experimental; preprint no EarthArXiv,
  [doi:10.31223/X55N75](https://doi.org/10.31223/X55N75)), licença CC-BY 4.0, lido dos COGs públicos
  do Google Cloud Storage (`gs://climate-and-sustainability/gem-15`): só a janela da área em cada
  célula de 1°. Alturas no geoide EGM96; precisa de um RustGIS depois da 0.5.0;
- **Altura do dossel a 1 m:** Canopy Height Maps v2 (CHMv2) da Meta e do World Resources
  Institute, 2026 ([arXiv:2603.06382](https://arxiv.org/abs/2603.06382)), licença CC BY 4.0, lido
  dos COGs públicos do bucket `dataforgood-fb-data` (AWS): só a janela da área em cada tile de
  zoom 10, da pirâmide do arquivo (médias) quando a grade é mais grossa. O 0 é sem árvore e também
  sem dado; precisa de um RustGIS depois da 0.5.0;
- **Radar:** OPERA Radiometric Terrain Corrected SAR Backscatter from Sentinel-1, versão 1
  (OPERA_L2_RTC-S1_V1), do projeto OPERA do JPL, distribuído pelo
  [ASF DAAC](https://asf.alaska.edu) da NASA
  ([doi:10.5067/SNWG/OPERA_L2_RTC-S1_V1](https://doi.org/10.5067/SNWG/OPERA_L2_RTC-S1_V1)), com
  dados Copernicus Sentinel-1 modificados (ESA);
- **Perturbação da vegetação:** OPERA Land Surface Disturbance Alert from Sentinel-1, versão 1
  (OPERA_L3_DIST-ALERT-S1_V1), do projeto OPERA do JPL, distribuído pelo
  [ASF DAAC](https://asf.alaska.edu) da NASA
  ([doi:10.5067/SNWG/OPL3DISTALS1-V1](https://doi.org/10.5067/SNWG/OPL3DISTALS1-V1)), com dados
  Copernicus Sentinel-1 modificados (ESA);
- **Indicadores por município:** IBGE (Censo Demográfico 2022, Censo Agropecuário 2017), Banco
  Central (Pix, crédito rural), TSE e Novo CAGED. O **Censo Agropecuário 2017** (Agricultura) traz
  os estabelecimentos e a agricultura familiar, a utilização das terras (lavouras, pastagens e
  matas, em ha), os bovinos, o pessoal ocupado, os tratores, a irrigação, o uso de agrotóxicos, a
  energia elétrica, a orientação técnica e o valor da produção (mil R$). A **Agroecologia**, do
  mesmo censo, traz a produção orgânica, a associação a cooperativas e entidades, as práticas de
  conservação, o plantio direto, a adubação orgânica ou química, quem não usou agrotóxicos e a área
  de sistemas agroflorestais. A tabela das fontes diz o que é cada campo;
- **Indicadores de saúde (Saúde):** Ministério da Saúde, DATASUS, pelo FTP de disseminação, por
  local de residência:
  - **SINASC:** os nascidos vivos e a parte com baixo peso, por cesárea, de mãe adolescente e com 7
    ou mais consultas de pré-natal;
  - **SIM:** os óbitos, a taxa por mil habitantes e a parte por causas externas, circulatórias,
    neoplasias e respiratórias, pelos capítulos da CID-10, além da mortalidade infantil por mil
    nascidos vivos, somada em 3 anos;
  - **CNES:** os leitos existentes e os do SUS, também por mil habitantes;
  - **População:** a estimativa do IBGE (tabela 6579).

  Os microdados vêm em .dbc num FTP, que o plugin não lê. Por isso eles vêm num retrato embutido
  (`saude.csv`), refeito por `scripts/baixar_saude.py` quando o DATASUS fecha um ano novo;
- **Setores censitários:** IBGE, Censo Demográfico 2022, agregados por setores. Os campos V0001 a
  V0007 do IBGE chegam com nomes legíveis: `pessoas`, `domicilios`, `dom_particulares`,
  `dom_coletivos`, `media_moradores`, `prop_imputados` (proporção de 0 a 1) e `dom_part_ocupados`.
  O plugin acrescenta a densidade, `dens_hab_km2`;
- **Uso e cobertura:** Projeto MapBiomas, Coleção 11 (1985 a 2025, 30 m), pela API da plataforma
  MapBiomas, como mapa de mosaicos com as cores da legenda oficial. É um mapa para ver, do Brasil
  inteiro: não é recortado pela área nem traz os valores das classes, porque o RustGIS 0.5.0 não
  lê só a janela da área de um COG remoto (opengeos/RustGIS#1204). O endereço do Earth Engine
  vale por algumas horas; um novo clique pede outro. O ano, de 1985 a 2025, se escolhe no controle
  MapBiomas: ano, no painel Área de trabalho, e cada ano vira uma camada. Abre também no app web.
  **Recortado pela área:** com uma área, de 2019 a 2024 e num RustGIS que lê a janela de um COG
  remoto, vem o raster das classes. Isso vale na linha de comando desde o RustGIS 0.6.0, e no app
  desktop a partir da versão com opengeos/RustGIS#1581. Num RustGIS que não lê a janela, o plugin
  avisa e traz o mapa de mosaicos. O raster é o da Coleção 10, 30 m,
  do espelho do Source Cooperative (o bucket oficial recusa acesso anônimo), só a janela da
  área, recortada pelo contorno e posta na grade do projeto com a classe mais frequente. Os valores
  são os códigos da legenda do MapBiomas, para medir áreas por classe. Logo abaixo do raster vem a
  camada das classes (em 2024, **Uso e cobertura da terra em 2024 (MapBiomas)**), com um polígono
  por classe, o nome e a cor da legenda oficial. É ela que põe os nomes na legenda do Layout, com o
  nome dela como título, e no identificar, porque a legenda do RustGIS não lista as classes de um
  raster. Áreas grandes vêm de uma pirâmide do arquivo, com células maiores;
- **Estrutura da vegetação (GEDI):** Seo, E., S.P. Healey, Z. Yang, R.O. Dubayah, T. De Conto e
  J. Armston. 2025. GEDI L4D Imputed Waveforms, Version 2. ORNL DAAC.
  https://doi.org/10.3334/ORNLDAAC/2455. Altura do dossel (rh98), biomassa acima do solo e cobertura
  do dossel a 30 m, dos disparos do laser do GEDI de 2019 a 2023 imputados com o Landsat. Pede o login
  Earthdata e um RustGIS que leia só a janela da área (na linha de comando desde a 0.6.0; no
  desktop, a partir da versão com opengeos/RustGIS#1581): cada arquivo cobre um fuso UTM;
- **Radiação solar:** NASA Langley Research Center, projeto POWER (API de climatologia v2.10.1).
  A irradiação global horizontal na superfície, com todos os céus, em kWh/m² por dia: a média
  diária de 2001 a 2025, do CERES SYN1deg, em células de 1° (cerca de 110 km). As células em volta
  da área, com uma de margem, viram um raster de 1° (`raster.rasterize`, desde o RustGIS 0.6.0),
  interpolado para a grade do projeto e recortado pela área: mostra a variação regional, por isso
  serve a áreas grandes (um estado) mais que a um município. Por script, um mês:
  `plugin:brasildata/sol-radiacao={"mes": 7}`;
- **Umidade do solo nas raízes e evapotranspiração:** NASA Langley Research Center, projeto POWER
  (APIs mensal e de climatologia v2.10), da reanálise MERRA-2 (NASA GMAO; Gelaro et al. 2017,
  doi:10.1175/JCLI-D-16-0758.1). A umidade do primeiro metro de solo (0 seco, 1 saturado) e a
  evapotranspiração da terra, em mm por dia, em células de 0,625° × 0,5°: a média de 2001 a 2025, um
  mês ou ano desde 1981 e a anomalia dele. As células em volta da área viram um raster, como na
  radiação solar; o mar não tem dado. Os meses saem
  com alguns meses de atraso; o ano, só completo. Por script:
  `plugin:brasildata/power-umidade={"medida": "anomalia", "ano": 2024, "mes": 9}` e
  `plugin:brasildata/power-evapotranspiracao={"ano": 2024}`;
- **Previsão do BAM:** CPTEC/INPE, Modelo Atmosférico Global Brasileiro (BAM, TQ0666L064, cerca de
  20 km; Figueroa et al. 2016, doi:10.1175/WAF-D-16-0062.1), do
  [servidor de dados do CPTEC](https://dataserver.cptec.inpe.br/dataserver_modelos/bam/). A chuva somada
  ou a temperatura média de 1 a 10 dias da rodada das 00 UTC, de registros de 6 horas baixados um a um
  (cerca de 4 MB cada) de arquivos GRIB do globo. Precisa do RustGIS 0.8.0, no desktop; na janela do
  0.8.0, o comando só termina uns 10 minutos depois de os registros chegarem (a correção foi enviada ao
  RustGIS). Por script:
  `plugin:brasildata/bam-previsao={"variavel": "chuva", "dias": 5}`;
- **Seca (GRACE):** NASA Goddard Space Flight Center, GRACE-DA-DM Global 3.0, distribuído pelo
  National Drought Mitigation Center da [Universidade de Nebraska](https://nasagrace.unl.edu) (Li et
  al. 2019, doi:10.1029/2018WR024618). O percentil semanal, desde 2003, da umidade do primeiro metro do
  solo, da água subterrânea e da umidade da superfície, em células de 0,25°. Sem login; o servidor não
  libera a leitura pelo navegador, e o botão funciona no desktop. Por script:
  `plugin:brasildata/grace-seca={"indicador": "subterranea", "data": "23/11/2026"}`;
- **Previsão do tempo:** Open-Meteo (Zippenfenig, P. 2023. Open-Meteo.com Weather API,
  doi:10.5281/zenodo.7970649; CC BY 4.0; a API gratuita é para uso não comercial), com os modelos
  IFS e AIFS e os ensembles ENS e AIFS-ENS do ECMWF (CC BY 4.0), o GFS e o GEFS da NOAA/NCEP
  (domínio público) e o ICON do DWD (CC BY 4.0). Os valores diários vêm nos nós de uma grade de
  0,25° (0,5° no GEFS) que cercam a área e viram um raster (`raster.rasterize`, desde o RustGIS
  0.6.0), interpolado para a grade do projeto e recortado pela área. Numa área grande, a grade de
  pontos fica mais aberta: até 400 pontos numa previsão e até 58 (ECMWF) ou 96 (GEFS) num
  ensemble, o que o orçamento de processamento de um clique do plugin aguenta. Os dias contam a
  partir de hoje, no horário de Brasília; um último dia incompleto na rodada fica de fora, com um
  aviso. Se o Open-Meteo demorar mais do que o RustGIS espera (20 s por pedido, 60 s por clique),
  o comando avisa e o clique seguinte, depois de 30 s, só pede o que faltou. Por script:
  `plugin:brasildata/chuva-prevista={"modelo": "gfs", "dias": 7}`,
  `plugin:brasildata/chuva-ensemble={"medida": "chance", "limiar": 150}` e
  `plugin:brasildata/temperatura-prevista={"tipo": "minima", "dias": 3}`;
- **Clima passado (ERA5):** a reanálise ERA5 do ECMWF (Hersbach et al. 2023, Copernicus Climate
  Change Service, doi:10.24381/cds.adbb2d47; licença do Copernicus: uso livre com a citação), pela
  API histórica do Open-Meteo (CC BY 4.0; a API gratuita é para uso não comercial): a chuva (total
  ou média por ano), a temperatura do ar a 2 m (máxima, mínima ou média do dia, ajustada à
  altitude) e a umidade do solo de 0 a 7 cm, de um dia a 30 anos desde 1940. Os valores diários
  vêm nos nós da grade de 0,25° que cercam a área e viram um raster, como na previsão. O cubo
  ERA5 do catálogo do RustGIS (ARCO-ERA5, em Zarr) guarda uma hora do globo por pedaço e levaria
  uns 23 minutos por mês: por isso o plugin usa o Open-Meteo. A API gratuita pesa cada ponto pelos
  dias pedidos, até uns 500 por clique: num período longo ou numa área grande, a grade abre até
  1°, e acima disso o comando diz o período que cabe. As respostas são lidas como texto
  (`host.fetch` com `"json": false`, do RustGIS 0.7.0): montar o JSON de 30 anos passaria do
  orçamento de processamento de um clique. Sem período, o ano passado inteiro. Por script:
  `plugin:brasildata/era5-chuva={"resumo": "anual", "inicio": "01/01/1991", "fim": "31/12/2020"}`,
  `plugin:brasildata/era5-temperatura={"tipo": "minima"}` e `plugin:brasildata/era5-umidade={}`;
- **Chuva do mês:** Climate Hazards Center (UC Santa Barbara), CHIRPS v3.0 (Funk et al. 2015,
  Scientific Data 2, 150066, doi:10.1038/sdata.2015.66): a chuva total do mês por satélite e
  estações, a 0,05°, de 1981 até o último mês fechado. Só a janela da área, lida do COG mundial
  do mês (depois do RustGIS 0.5.0), posta na grade do projeto e recortada pela área. Só no app
  desktop: o servidor não manda CORS. O README da v3 não declara licença; o Climate Hazards
  Center distribui o CHIRPS como domínio público. Por script:
  `plugin:brasildata/chirps-mes={"mes": "10/2026"}`;
- **Chuva nas estações e alertas:** CEMADEN/MCTI (Centro Nacional de Monitoramento e Alertas de
  Desastres Naturais), rede observacional de estações ("data from the CEMADEN/MCTI observational
  network") e alertas de risco geo-hidrológico, sem login. A chuva vem do serviço do mapa
  interativo (`getJson2.php`, por estado, atualizado a cada 10 minutos): os acumulados de 1 a 96
  horas de cada estação, com as coordenadas das tabelas que vêm no plugin (`cemaden.csv` e o
  índice `cemaden.bin`, de `scripts/baixar_cemaden.py`, para rodar de novo a cada versão: uma
  estação instalada depois fica de fora, e o aviso diz quantas). A área pode cruzar até 6 estados
  e ter até 1.500 estações no retângulo. Os alertas vigentes vêm do painel de alertas
  (`wsAlertas2`), um por município, com o contorno do município da malha embutida, inteiro: entram
  os municípios com ao menos 3% dentro da área. O CEMADEN não declara licença para os dados (o
  portal é CC BY-ND 3.0): cite a fonte. Só no app desktop: os servidores não mandam CORS. As séries
  históricas ficam na PED do CEMADEN (ped.cemaden.gov.br, cadastro gratuito), fora do plugin;
- **Chuva agora:** Google DeepMind e Google Research, PARDIG (Shalev et al. 2026,
  arXiv:2611.13941, CC BY 4.0): a taxa de chuva estimada por IA a partir de satélites, treinada no
  radar do GPM, a 0,1° e a cada 15 minutos, de 1999 em diante. O quadro mais recente sai com umas
  5 horas de atraso; o plugin, que não tem relógio, acha-o pela listagem do bucket público
  `gs://pardig_1` (uma bisseção de cerca de dez pedidos pequenos). Só a janela da área, lida do
  store Zarr do quadro pelo próprio RustGIS (`layer.addZarr` com `bbox`), passada a mm/h, posta na
  grade do projeto e recortada pela área. Só no app desktop: o bucket não manda CORS. Dado
  experimental, não é alerta oficial. Por script:
  `plugin:brasildata/pardig-agora={"quando": "16/11/2026 07:30"}`;
- **Estresse hídrico:** WRI, Aqueduct 4.0 (Kuzma et al. 2023, CC BY 4.0), pelo serviço Aqueduct
  Water Risk do ArcGIS Living Atlas: a retirada de água sobre a oferta renovável, por sub-bacia
  HydroBASINS de nível 6 e unidade administrativa. O plugin acrescenta o campo `estresse`, com as
  categorias em português, e as cores do Aqueduct;
- **Área equipada para irrigação:** Siebert et al. 2013, Global Map of Irrigation Areas versão 5
  (FAO e Universidade de Bonn, CC BY 4.0): a porcentagem de cada célula de 5′ (cerca de 9 km)
  equipada para irrigação, por volta de 2005, lida só na janela da área do arquivo mundial da FAO
  (um RustGIS que leia a janela, como o GEDI). As cores vão de 0 a 40%, com mais tons até 10%;
- **Risco de fogo:** INPE, Programa Queimadas, modelo de Risco de Fogo versão 11 (Setzer,
  Sismanoglu e Santos 2019): o perigo meteorológico de fogo, de 0 a 1, pelos dias de secura (chuva
  do IMERG), a umidade e a temperatura do GFS, a vegetação, o relevo e a latitude. A climatologia
  de um mês, a média de 2001 a 2025, a 0,01° (cerca de 1 km), lida só na janela da área do
  arquivo do mês (um RustGIS que leia a janela, como o GEDI). As cores são as cinco classes do
  INPE: mínimo (abaixo de 0,15), baixo (até 0,40), médio (até 0,70), alto (até 0,95) e crítico.
  O mês vem da janela do comando ou do controle **Risco de fogo: mês** do painel (setembro até você
  mudar); por script, `plugin:brasildata/risco-fogo={"mes": 3}`. É perigo modelado, não
  ocorrência: para o fogo de fato, os focos de calor. O INPE não declara uma licença; cite o
  Programa Queimadas e o método;
- **Sistemas de irrigação:** Projeto MapBiomas, mapeamento anual dos sistemas de irrigação (pivô
  central, inundação e outros sistemas; 1985 a 2025, 30 m), pela API da plataforma. Como o mapa de
  uso e cobertura do Brasil inteiro, é um mapa para ver, não recortado pela área: o MapBiomas não
  publica um arquivo dele que se leia pela janela. O ano é o do controle MapBiomas: ano;
- **Estabelecimentos de saúde:** Ministério da Saúde, CNES, competência 04/2026, pelo
  [geobr](https://github.com/ipea/geobr) do Ipea (GeoParquet do Brasil, 98 MB, lido aos pedaços).
  Ficam os ativos, sem os de pessoa física e de empresário individual, cujo nome é o de uma
  pessoa, e sem CPF, CNPJ ou nome de diretor. Só no app desktop: o GitHub não manda CORS. O
  zip do próprio Ministério (OpenDataSUS) vem em Latin-1, que o RustGIS 0.5.0 não lê;
- **Terreno da América do Sul (ANADEM):** ANA (2025), ANADEM: A Digital Terrain Model for South
  America, distribuído pela OpenTopography (doi:10.5069/G9736P4G); Laipelt, L. et al., Remote
  Sensing 16(13), 2321 (2024). É o Copernicus GLO-30 sem o viés da vegetação, a 30 m, lido do S3
  público da OpenTopography, sem chave. Livre e aberto, com a citação;
- **Relevo e derivadas (GEDTM30):** Ho, Y.-F. et al., PeerJ 13, e19673 (2025),
  doi:10.7717/peerj.19673; dados: Ho, Y.-F. e Hengl, T., doi:10.5281/zenodo.14900181 (CC BY 4.0).
  A elevação vem da cópia da OpenTopography (doi:10.5069/G9BV7DT1), e as derivadas, dos servidores
  da OpenGeoHub. Algumas variáveis só vêm nas escalas de 240, 480 ou 960 m, porque o RustGIS ainda
  não lê esses arquivos por partes (opengeos/RustGIS#2569); a janela diz qual escala veio;
- **Propriedades do solo (OpenLandMap-soildb):** Hengl, T. et al., Earth System Science Data 18,
  989–1036 (2026); dados: Consoli, D. et al., doi:10.5281/zenodo.15470431 (CC BY 4.0).
  - **Propriedades e profundidades:** carbono orgânico, densidade de carbono, pH, densidade do solo,
    argila, areia e silte, a 30 m, de 0 a 30, de 30 a 60 e de 60 a 100 cm.
  - **Períodos:** carbono e pH de 5 em 5 anos desde 2000.
  - **Servidor:** o da OpenGeoHub é lento, e uma área grande pode passar do tempo limite de 30 s
    do RustGIS (opengeos/RustGIS#2570);
- **Cobertura da terra 100 m:** Buchhorn, M. et al., Copernicus Global Land Service: Land Cover
  100m, coleção 3, de 2015 a 2019, pelo Zenodo (CC BY 4.0);
- **Cobertura da terra a 10 m (ESA WorldCover):** © ESA WorldCover project 2020 ou 2021 / Contains
  modified Copernicus Sentinel data processed by ESA WorldCover consortium; Zanaga, D. et al.,
  doi:10.5281/zenodo.5571936 (2020) e doi:10.5281/zenodo.7254221 (2021), CC BY 4.0, pelo Planetary
  Computer da Microsoft, sem login. Os dois anos usam versões diferentes do algoritmo e não se
  subtraem;
- **Cenas hiperespectrais do EMIT:** EMIT L2A Surface Reflectance and Uncertainty and Masks 60 m V002
  (doi:10.5067/EMIT/EMITL2ARFL.002) e V001 (doi:10.5067/EMIT/EMITL2ARFL.001), NASA LP DAAC; contornos
  e nuvem do catálogo CMR da NASA, sem login (as imagens pedem o Earthdata Login);
- **NDVI a 300 m (Copernicus):** Copernicus Land Monitoring Service, NDVI 300 m de 10 em 10 dias,
  versão 3 (União Europeia), pelo Copernicus Data Space Ecosystem, com o login que o RustGIS guarda;
- **Índice de vegetação (cubo do INPE):** Brazil Data Cube, cubos de 16 dias do Sentinel-2, do
  Landsat, do CBERS-4 WFI e do MODIS; Ferreira et al. 2020, Remote Sensing 12(24), 4033. Só no
  app desktop;
- **Seca (Copernicus GDO):** Copernicus Emergency Management Service, Global Drought Observatory,
  do Centro Comum de Investigação da Comissão Europeia (CC BY 4.0). O mapa é a imagem do serviço
  WMS, sem recorte pela área (opengeos/RustGIS#2530);
- **Clima passado (ERA5) e satélites do JAXA:** a reanálise vem pela API histórica do Open-Meteo
  (acima). O GSMaP, o AMSR2 e o GCOM-C vêm pelo JAXA Earth API, com o crédito "JAXA" e o nome do
  conjunto. Só no app desktop;
- **Vegetação por IA:**
  - **Altura do dossel da Amazônia a 4,78 m:** Wagner, F. H. et al. (2025) e CTrees;
  - **Biomassa acima do solo a 100 m, de 2000 a 2025:** Yang, Y., Saatchi, S. et al., CTrees,
    doi:10.82924/7VMB-ZV66 (CC BY 4.0);
  - **Tipologia florestal 2020 a 10 m:** Neumann, M. et al., Scientific Data (2026),
    doi:10.1038/s41597-026-08299-5 (CC BY 4.0). Só no app desktop;
- **Edificações:** Zhu, X. X. et al. (2025), GlobalBuildingAtlas, Earth System Science Data 17,
  6647, pelo Source Cooperative. CC BY-NC 4.0, sem uso comercial; os polígonos do OpenStreetMap
  seguem a ODbL;
- **Endereços:** IBGE, Censo 2022, CNEFE, processado pelo AddressForAll e pelo OpenAddresses, pelo tema
  de endereços do [Overture Maps](https://overturemaps.org) (dados abertos do IBGE, com citação);
- **Radar NISAR:** NISAR Geocoded Polarimetric Covariance Product, versão 1, da NASA/JPL e da ISRO,
  pelo ASF DAAC (doi:10.5067/NIL2GCOV-V1). Banda L a 20 m, nas polarizações HH e HV, com a razão e
  a composição colorida. Pede o login Earthdata, como o Sentinel-1.

A ficha de cada camada raster, com o período, a resolução, a legenda, os cuidados e um mapa
exemplo, está em [docs/camadas](docs/camadas/README.md).

## Instalar

- **Pelo registro:** *Plugins › Plugin Manager*, procure BrasilData.
- **Por arquivo:** baixe o `.rgplugin` da [última versão](https://github.com/elpidioassinaturas/BrasilData/releases)
  e use *Plugins › Plugin Manager › Settings › Install From File…*, ou
  `rustgis-cli plugins install brasildata-<versão>.rgplugin`.
- **Atualizar:** o botão **Verificar atualização** (grupo Área) compara a
  versão instalada com a última release e diz como atualizar. Instalado pelo registro, o plugin
  se atualiza em *Gerenciador de plugins › Instalado › Atualizar*; instalado por arquivo, baixe o
  `.rgplugin` novo e instale-o de novo. Um plugin não pode se atualizar sozinho: o RustGIS não
  deixa um plugin rodar os comandos de instalação.

Precisa do RustGIS 0.5.0 ou mais novo: o `bbox` dos serviços de feições (opengeos/RustGIS#924),
os eixos do WFS corrigidos (opengeos/RustGIS#1071) e o `raster.project` da elevação entraram nela.
Os rasters lidos só na janela da área (o GEDI, o OpenLandMap, o GEDTM30, o ANADEM, a CTrees, o
Copernicus e outros) precisam de um RustGIS depois da 0.5.0, e as janelas dos comandos, da 0.7.0 ou
mais novo.

## Desenvolver

```sh
rustup target add wasm32-unknown-unknown
python scripts/gerar.py           # plugin.json e src/catalogo.rs a partir de fontes.csv
python scripts/package.py         # compila, confere como o registro confere e empacota em dist/
python scripts/smoke.py --cli caminho/para/rustgis-cli
```

O `package.py` e o `smoke.py` vêm do
[modelo de plugin](https://github.com/opengeos/rustgis-plugin-template) e precisam do Python
3.12 ou mais novo. No Windows, rode o `smoke.py` com `PYTHONUTF8=1`.

- **Fontes:** uma camada nova é uma linha em `fontes.csv`, seguida de `python scripts/gerar.py`.
  O CI roda `gerar.py --check` e falha se o `plugin.json` ou o `catalogo.rs` não estiverem em dia.
- **Página:** `python scripts/site.py` gera a página em `_site/` a partir do README (o topo e as
  seções Instalar, Autoria e Licença), do `plugin.json`, do `fontes.csv`, do `fichas.csv` e de
  `docs/camadas`; o estilo e a busca ficam em `site/`. O workflow *Pages* a publica a cada push na
  `main`, e o CI confere que ela é gerada.
- **Tabelas embutidas:** `municipios.csv`, `malhas.bin`, `tse.csv`, `caged.csv` e `saude.csv` são
  refeitas pelos `scripts/baixar_*.py` (`baixar_malhas.py` depois de `baixar_municipios.py`; o
  `baixar_saude.py` roda com `uv run --python 3.12 --with pyreaddbc --with dbfread` e baixa cerca de
  900 MB do FTP do DATASUS).
- **Versão do RustGIS no CI:** a variável `RUSTGIS_VERSION` do repositório escolhe a versão em
  que o plugin roda; sem ela, é a mais recente.

## Publicar uma versão

1. Suba a versão em `scripts/gerar.py` (`VERSAO`) e no `Cargo.toml`, e rode `python scripts/gerar.py`.
2. `git tag v<versão> && git push origin v<versão>`. O workflow *Release* compila, confere,
   roda o plugin no RustGIS e cria a release com o `.rgplugin` e a entrada do registro.
3. Abra um PR no [opengeos/rustgis-plugins](https://github.com/opengeos/rustgis-plugins) com
   `registry/brasildata.json`, com a categoria `Data`.

## Autoria

Elpídio Filho, do Laboratório de Geoprocessamento e Pedometria (LABGEO) da Universidade Federal
de Viçosa (UFV). O botão **Sobre** (grupo Área) mostra a autoria dentro do RustGIS.

## Licença

Código sob a licença MIT ([LICENSE](LICENSE)). Os dados são dos órgãos que os publicam, cada um
com a sua licença, listada em `fontes.csv` e no comando Fontes.
