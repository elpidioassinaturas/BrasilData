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
RustGIS aceita. O grupo Área tem o painel **Área de trabalho**, a Grade do projeto e o comando
Fontes.

A área de trabalho pode ser:

- **um limite político**, clicando no nome na árvore **Limites políticos** do painel **Área de
  trabalho**: o Brasil, uma grande região, um estado, uma mesorregião, uma microrregião ou um
  município. O triângulo abre o ramo, e a caixa busca pelo nome. O RustGIS guarda até 5.000 nós
  por árvore, e o Brasil tem 5.570 municípios: os municípios aparecem (e entram na busca) quando
  você escolhe a grande região deles ou qualquer lugar dentro dela. Os limites vêm dentro do plugin
  (`malhas.bin`, da API de malhas do IBGE), e a escolha funciona com o site do IBGE fora do ar ou
  sem internet. Meso e microrregiões são as da divisão do IBGE de 1989. Uma grande região é o
  conjunto dos seus estados; o Brasil é o contorno do país do IBGE, mais simples (cerca de 4.000
  vértices), porque os 27 estados detalhados não cabem numa chamada de plugin. As fronteiras são
  generalizadas: para o contorno exato, traga a camada de **Limites** e use a seleção. Por script:
  `plugin:brasildata/escolher-area={"no": "reg:2"}` (`br`, `reg:1` a `reg:5`, `uf:PI`,
  `meso:2202`, `micro:22003`, `mun:2211001`);
- **um desenho**: **Área: desenho** liga a ferramenta (clique nos cantos e dê um duplo clique para
  fechar o polígono) e **Usar o desenho** faz dele a área;
- **a seleção**: os polígonos selecionados da camada ativa, com **Área: seleção**;
- **a vista**: sem área, os temas pedem o que está na vista do mapa.

A **Grade do projeto** é o sistema de coordenadas e o tamanho das células em que os rasters chegam
(hoje, a Elevação). No painel, escolha a célula (de 10 m a 1 km) e o sistema e clique em **Grade do
projeto**:

- **Automático:** o UTM SIRGAS 2000 do fuso da área, ou a Policônica (EPSG:5880) quando a área
  passa de um fuso;
- **Policônica:** a Policônica do Brasil, SIRGAS 2000 (EPSG:5880);
- **Albers (IBGE):** a Cônica Equivalente de Albers do IBGE, SIRGAS 2000 (EPSG:10857), que preserva
  as áreas.

A grade fica no mapa como a camada **BrasilData: grade**: o retângulo da área, com o sistema e a
célula nos atributos. Ela é salva com o projeto. Os cantos das células caem nos múltiplos da célula,
então rasters na mesma grade têm as mesmas linhas e colunas. Uma área nova refaz a grade com as
mesmas escolhas. Sem grade, a primeira Elevação cria a padrão: 30 m, sistema automático. Por script:
`plugin:brasildata/grade={"celula": 90, "crs": "albers"}`; o `crs` aceita também um código EPSG em
metros.

| Tópico | Subtópicos |
|---|---|
| Território | Limites (IBGE: Brasil, Grandes Regiões, estados, regiões intermediárias e imediatas, meso e microrregiões, municípios, distritos, subdistritos e bairros), Recortes especiais (IBGE: Amazônia Legal, Semiárido, MATOPIBA, SUDENE, municípios costeiros, regiões metropolitanas, aglomerações, concentrações e arranjos urbanos, hierarquia urbana), Cadastro fundiário (SFB, INCRA), Terras indígenas (FUNAI) |
| Sociedade e economia | População (IBGE: setores censitários, censos, áreas de ponderação, área urbana e rural, favelas e comunidades urbanas), Economia (Banco Central, Novo CAGED), Agricultura (Banco Central, IBGE), Eleições (TSE) e Todos os indicadores |
| Saúde | Estabelecimentos (CNES, DATASUS) |
| Meio físico | Geologia e recursos minerais (IBGE, ANM, MapBiomas), Relevo (IBGE, Copernicus), Solos (IBGE) |
| Água | Hidrografia (IBGE, ANA), Bacias (IBGE, ANA), Recursos hídricos (ANA) |
| Vegetação e meio ambiente | Vegetação (IBGE), Uso e cobertura (MapBiomas), Desmatamento (INPE: PRODES e DETER), Queimadas (INPE: focos de calor das últimas 48 horas, dos últimos 7 dias e do mês corrente; um período com datas, por script: `plugin:brasildata/inpe-focos-periodo={"de": "01/11/2026", "ate": "06/11/2026"}`), Áreas protegidas (MMA, ICMBio), Fiscalização (IBAMA) |
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
  vale por algumas horas; um novo clique pede outro. Abre também no app web;
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

## Licença

Código sob a licença MIT ([LICENSE](LICENSE)). Os dados são dos órgãos que os publicam, cada um
com a sua licença, listada em `fontes.csv` e no comando Fontes.
