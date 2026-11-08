# BrasilData

Plugin do [RustGIS](https://github.com/opengeos/RustGIS) que traz dados públicos do Brasil
para a área do usuário: só o que cruza a área, recortado por ela, com a fonte e a licença de cada
camada. Funciona no aplicativo desktop e no app web (WebAssembly), sem instalar mais nada.

*A RustGIS plugin that brings Brazilian public data (IBGE, ANA, INPE, ICMBio, INCRA, ANEEL, ANP,
ANM, IBAMA, DNIT and others) into the user's area, clipped to it, with each layer's source and
licence. It runs on the desktop and in the web app. The interface is in Portuguese.*

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
as fontes dos dados e como contribuir, num aviso curto e, inteiro, em **Sobre o BrasilData**, no
fim do painel.

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

A **Grade do projeto** é o sistema de coordenadas e o tamanho das células em que os rasters chegam
(hoje, a Elevação, o Radar e a Perturbação da vegetação). No painel, escolha a célula (de 10 m a 1 km) e o sistema e clique em **Grade do
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
| Território | Limites (IBGE: Brasil, Grandes Regiões, estados, regiões intermediárias e imediatas, meso e microrregiões, municípios, distritos, subdistritos e bairros), Recortes especiais (IBGE: Amazônia Legal, Semiárido, MATOPIBA, SUDENE, municípios costeiros, regiões metropolitanas, aglomerações, concentrações e arranjos urbanos, hierarquia urbana), Cadastro fundiário (SFB, INCRA), Terras indígenas (FUNAI) |
| Sociedade e economia | População (IBGE: setores censitários, censos, áreas de ponderação, área urbana e rural, favelas e comunidades urbanas), Economia (Banco Central, Novo CAGED), Agricultura (Banco Central, IBGE), Eleições (TSE) e Todos os indicadores |
| Saúde | Estabelecimentos (CNES, DATASUS) |
| Meio físico | Geologia e recursos minerais (IBGE, ANM, MapBiomas), Relevo (IBGE, Copernicus), Solos (IBGE) |
| Água | Hidrografia (IBGE, ANA), Bacias (IBGE, ANA), Recursos hídricos (ANA) |
| Vegetação e meio ambiente | Vegetação (IBGE), Uso e cobertura (MapBiomas; radar Sentinel-1 do ASF/NASA), Desmatamento (INPE: PRODES e DETER; alertas de perturbação OPERA DIST-S1 do ASF/NASA), Queimadas (INPE: focos de calor das últimas 48 horas, dos últimos 7 dias e do mês corrente; um período com datas, por script: `plugin:brasildata/inpe-focos-periodo={"de": "01/11/2026", "ate": "06/11/2026"}`), Áreas protegidas (MMA, ICMBio), Fiscalização (IBAMA) |
| Infraestrutura | Energia (ANEEL), Petróleo e gás (ANP), Transportes (DNIT) |

As camadas vetoriais estão listadas em [`fontes.csv`](fontes.csv) e na tabela [Fontes](#fontes),
abaixo: serviço, camada, fonte, licença e se a camada abre no app web. O comando **Fontes**
põe a citação e a licença de cada camada trazida na tabela **BrasilData: fontes**, que abre embaixo
do mapa e fica salva com o projeto. Os indicadores são agregados por município: o plugin não traz CPF, CNPJ ou
nomes de pessoas.

Algumas camadas só abrem num dos aplicativos, e a ajuda de cada botão diz quando é o caso:
- 19 das 82 só abrem no app desktop, porque o servidor não manda os cabeçalhos CORS que o navegador exige;
- os imóveis do CAR só abrem no app web, porque o servidor só aceita cifras TLS antigas, que o app desktop não aceita.

## Fontes

<!-- fontes: início (gerado por scripts/gerar.py a partir do fontes.csv; não edite à mão) -->
82 camadas de 14 fontes, na ordem do `fontes.csv`. *App web*: se o servidor permite pedidos do navegador (CORS); *Recorte*: se a camada é recortada pela área.

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
| Meio físico › Geologia e recursos minerais | Geologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geol_area` | uso livre com citação da fonte | sim | sim |
| Meio físico › Relevo | Geomorfologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geom_area` | uso livre com citação da fonte | sim | sim |
| Meio físico › Solos | Solos (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:pedo_area` | uso livre com citação da fonte | sim | sim |
| Vegetação e meio ambiente › Vegetação | Vegetação (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:vege_area` | uso livre com citação da fonte | sim | sim |
| Água › Hidrografia | Drenagem (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_trecho_drenagem_l` | uso livre com citação da fonte | sim | sim |
| Água › Hidrografia | Massas d'água (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_massa_dagua_a` | uso livre com citação da fonte | sim | sim |
| Água › Bacias | Ottobacias (BHO 2017 1:50.000, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017 1:50.000), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/SPR/BHO2017_50K_AREADRENAGEM/FeatureServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Hidrografia | Trechos de drenagem (BHO 2017, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/Atlas2020/Bho_2017_BR/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Bacias | Unidades de planejamento hídrico (UPH, ANA) | ANA, Unidades de Planejamento Hídrico (UPH), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/DADOSABERTOS/Unidade_de_Planejamento_H%C3%ADdrico_UPH/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | não |
| Água › Bacias | Macrorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_840_MacroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Mesorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_835_MesoRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Microrregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_830_MicroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Recursos hídricos | Disponibilidade hídrica superficial (ANA) | ANA, Disponibilidade Hídrica Superficial, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Disponibilidade_Hidrica_Superficial/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Balanço hídrico quantitativo por curso d'água (ANA) | ANA, Balanço Hídrico Quantitativo por Curso de Água, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Balanco_Hidrico_Quantitativo_por_Curso_de_Agua/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Estações da Rede Hidrometeorológica Nacional (ANA) | ANA, Rede Hidrometeorológica Nacional, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Rede_Hidrometeorologica_Nacional/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Captações superficiais (ANA) | ANA, Captações Superficiais, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Captacoes_Superficiais/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Pivôs centrais mapeados (ANA) | ANA, Pivôs Mapeados, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Pivos_Mapeados/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › Recursos hídricos | Municípios atingidos por seca em 2015 (ANA) | ANA, Municípios Atingidos por Eventos de Seca em 2015, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Municipios_Atingidos_por_Eventos_de_Seca_em_2015/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação (CNUC 07/2026) | MMA, Cadastro Nacional de Unidades de Conservação (CNUC), 07/2026 | [WFS](https://geoservicos.inde.gov.br/geoserver/MMA/ows) `MMA:cnuc_26_07_31` | não declarada no serviço | sim | sim |
| Vegetação e meio ambiente › Áreas protegidas | Unidades de conservação federais (ICMBio) | ICMBio, Limites das Unidades de Conservação Federais, 08/2026 | [WFS](https://geoservicos.inde.gov.br/geoserver/ICMBio/ows) `ICMBio:limiteucsfederais_a` | não declarada no serviço | sim | sim |
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
| Vegetação e meio ambiente › Fiscalização | Áreas embargadas (IBAMA) | IBAMA, Termos de embargo (SISCOM, PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/SISCOM/publico/MapServer) `3` | dados abertos; licença não declarada no serviço | sim | sim |
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
| Infraestrutura › Transportes | Rodovias federais (SNV vigente, DNIT) | DNIT, Sistema Nacional de Viação (SNV), versão vigente | [WFS](https://servicos.dnit.gov.br/dnitgeo/geoserver/ows) `vgeo:vw_snv_rod` | sem restrições declaradas | sim | sim |
| Meio físico › Geologia e recursos minerais | Área minerada por ano (MapBiomas, 1985–2024) | MapBiomas, Monitor da Mineração, área minerada, 1985 a 2024 (campos: ano, substância, se está num processo do SIGMINE, se está em área restrita e área) | [WFS](https://plataforma.geoserver.mapbiomas.org/geoserver/pto/ows) `pto:mining_age` | CC BY 4.0, pelos termos do MapBiomas; não declarada no serviço | sim | sim |
| Território › Recortes especiais | Amazônia Legal (IBGE 2024) | IBGE, Amazônia Legal, Malha Municipal 2024 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2024_040_amazleg_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Semiárido (IBGE 2025) | IBGE, Semiárido, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_080_semiarido_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | MATOPIBA (IBGE 2025) | IBGE, MATOPIBA, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_190_matopiba_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Área de atuação da SUDENE (IBGE 2025) | IBGE, Área de atuação da SUDENE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_070_areasudene_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Municípios costeiros (IBGE 2025) | IBGE, Municípios costeiros, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_280_municipioscosteiros_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Recorte metropolitano (IBGE 2025) | IBGE, Recorte metropolitano, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_211_recortemetrop_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Aglomerações urbanas (IBGE 2025) | IBGE, Aglomerações urbanas, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_230_aglomurb_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Concentrações urbanas (IBGE 2025) | IBGE, Concentrações urbanas, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_090_concurb_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Arranjos populacionais (IBGE 2025) | IBGE, Arranjos populacionais, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_100_arranjpop_agreg` | uso livre com citação da fonte | sim | não |
| Território › Recortes especiais | Hierarquia urbana das cidades (IBGE 2025) | IBGE, Hierarquia urbana das cidades, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_200_hierarqurb_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Áreas de ponderação (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_710_apond_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Área urbana para fins censitários (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_705_areaurbanapormunicipiopfc_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Área rural para fins censitários (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_700_arearuralpormunicipiopfc_agreg` | uso livre com citação da fonte | sim | não |
| Sociedade e economia › População | Favelas e comunidades urbanas (IBGE, Censo 2022) | IBGE, Censo Demográfico 2022, malha de setores e agregados | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2022_670_fcu_agreg` | uso livre com citação da fonte | sim | não |
<!-- fontes: fim -->

Fora da tabela, com a citação no comando **Fontes**:
- **Elevação:** Copernicus DEM GLO-30 (© DLR e.V. e © Airbus Defence and Space GmbH, sob o
  programa Copernicus da União Europeia e da ESA), baixado do espelho da
  [OpenTopography](https://opentopography.org), que abre também no app web;
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
- **Setores censitários:** IBGE, Censo Demográfico 2022, agregados por setores;
- **Uso e cobertura:** Projeto MapBiomas, Coleção 11 (1985 a 2025, 30 m), pela API da plataforma
  MapBiomas, como mapa de mosaicos com as cores da legenda oficial. É um mapa para ver, do Brasil
  inteiro: não é recortado pela área nem traz os valores das classes, porque o RustGIS 0.5.0 não
  lê só a janela da área de um COG remoto (opengeos/RustGIS#1204). O endereço do Earth Engine
  vale por algumas horas; um novo clique pede outro. O ano, de 1985 a 2025, se escolhe no controle
  MapBiomas: ano, no painel Área de trabalho, e cada ano vira uma camada. Abre também no app web.
  **Recortado pela área:** com uma área, de 2019 a 2024 e num RustGIS que lê a janela de um COG
  remoto (depois da 0.5.0, no desktop e na linha de comando), vem o raster das classes: Coleção 10,
  30 m, do espelho do Source Cooperative (o bucket oficial recusa acesso anônimo), só a janela da
  área, recortada pelo contorno e posta na grade do projeto com a classe mais frequente. Os valores
  são os códigos da legenda do MapBiomas, para medir áreas por classe. Áreas grandes vêm de uma
  pirâmide do arquivo, com células maiores;
- **Estrutura da vegetação (GEDI):** Seo, E., S.P. Healey, Z. Yang, R.O. Dubayah, T. De Conto e
  J. Armston. 2025. GEDI L4D Imputed Waveforms, Version 2. ORNL DAAC.
  https://doi.org/10.3334/ORNLDAAC/2455. Altura do dossel (rh98), biomassa acima do solo e cobertura
  do dossel a 30 m, dos disparos do laser do GEDI de 2019 a 2023 imputados com o Landsat. Pede o login
  Earthdata e um RustGIS que leia só a janela da área (depois da 0.5.0): cada arquivo cobre um fuso UTM;
- **Estabelecimentos de saúde:** Ministério da Saúde, CNES, competência 04/2026, pelo
  [geobr](https://github.com/ipea/geobr) do Ipea (GeoParquet do Brasil, 98 MB, lido aos pedaços).
  Ficam os ativos, sem os de pessoa física e de empresário individual, cujo nome é o de uma
  pessoa, e sem CPF, CNPJ ou nome de diretor. Só no app desktop: o GitHub não manda CORS. O
  zip do próprio Ministério (OpenDataSUS) vem em Latin-1, que o RustGIS 0.5.0 não lê.

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
- **Tabelas embutidas:** `municipios.csv`, `malhas.bin`, `tse.csv` e `caged.csv` são refeitas pelos
  `scripts/baixar_*.py` (`baixar_malhas.py` depois de `baixar_municipios.py`).
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
