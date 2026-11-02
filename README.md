# BrasilData

Plugin do [RustGIS](https://github.com/opengeos/RustGIS) que traz dados públicos do Brasil
para a área do usuário: só o que cruza a área, recortado por ela, com a fonte e a licença de cada
camada. Funciona no aplicativo desktop e no app web (WebAssembly), sem instalar mais nada.

*A RustGIS plugin that brings Brazilian public data (IBGE, ANA, INPE, ICMBio, INCRA, ANEEL, ANP,
ANM, IBAMA, DNIT and others) into the user's area, clipped to it, with each layer's source and
licence. It runs on the desktop and in the web app. The interface is in Portuguese.*

## O que traz

A aba **BrasilData** tem um painel para escolher a área (um município, a seleção ou a vista) e os
grupos:

| Grupo | Menus |
|---|---|
| Limites e indicadores | Municípios, Estados, Indicadores por município (Censos, Pix, crédito rural, eleições, emprego formal), Saúde (estabelecimentos do CNES) |
| Meio físico | Geologia, Geomorfologia, Solos, Vegetação (BDiA, IBGE), Elevação (Copernicus GLO-30) |
| Água | Drenagem, Massas d'água, ANA, Bacias (BHO 2017, UPH, divisão hidrográfica do IBGE) |
| Áreas protegidas | Unidades de conservação, UCs federais, Terras indígenas |
| Fundiário | Imóveis rurais (CAR), INCRA (SIGEF) |
| Desmatamento e embargos | PRODES, DETER, Embargos do IBAMA |
| Energia, minas e transportes | ANEEL, ANP, ANM, Rodovias federais (SNV) |

As camadas vetoriais estão listadas em [`fontes.csv`](fontes.csv) e na tabela [Fontes](#fontes),
abaixo: serviço, camada, fonte, licença e se a camada abre no app web. O comando **Fontes**
mostra a citação e a licença de cada camada trazida. Os indicadores são agregados por município: o plugin não traz CPF, CNPJ ou
nomes de pessoas.

Algumas camadas só abrem num dos aplicativos, e a ajuda de cada botão diz quando é o caso:
- 19 das 55 só abrem no app desktop, porque o servidor não manda os cabeçalhos CORS que o navegador exige;
- os imóveis do CAR só abrem no app web, porque o servidor só aceita cifras TLS antigas, que o app desktop não aceita.

## Fontes

<!-- fontes: início (gerado por scripts/gerar.py a partir do fontes.csv; não edite à mão) -->
55 camadas de 13 fontes, na ordem do `fontes.csv`. *App web*: se o servidor permite pedidos do navegador (CORS); *Recorte*: se a camada é recortada pela área.

| Grupo | Camada | Fonte | Serviço | Licença | App web | Recorte |
|---|---|---|---|---|---|---|
| Limites e indicadores | Municípios (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_030_munic` | uso livre com citação da fonte | sim | não |
| Limites e indicadores | Estados (IBGE 2025) | IBGE, Malha Municipal 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2025_020_uf_agreg` | uso livre com citação da fonte | sim | não |
| Meio físico | Geologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geol_area` | uso livre com citação da fonte | sim | sim |
| Meio físico | Geomorfologia (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:geom_area` | uso livre com citação da fonte | sim | sim |
| Meio físico | Solos (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:pedo_area` | uso livre com citação da fonte | sim | sim |
| Meio físico | Vegetação (BDiA 1:250.000) | IBGE, Banco de Dados de Informações Ambientais (BDiA) | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `BDIA:vege_area` | uso livre com citação da fonte | sim | sim |
| Água | Drenagem (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_trecho_drenagem_l` | uso livre com citação da fonte | sim | sim |
| Água | Massas d'água (BC250 2025) | IBGE, Base Cartográfica Contínua 1:250.000 (BC250), 2025 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CCAR:BC250_2025_hid_massa_dagua_a` | uso livre com citação da fonte | sim | sim |
| Água › Bacias | Ottobacias (BHO 2017 1:50.000, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017 1:50.000), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/SPR/BHO2017_50K_AREADRENAGEM/FeatureServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Bacias | Trechos de drenagem (BHO 2017, ANA) | ANA, Base Hidrográfica Ottocodificada (BHO 2017), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/Atlas2020/Bho_2017_BR/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | sim |
| Água › Bacias | Unidades de planejamento hídrico (UPH, ANA) | ANA, Unidades de Planejamento Hídrico (UPH), SNIRH | [ArcGIS REST](https://www.snirh.gov.br/arcgis/rest/services/DADOSABERTOS/Unidade_de_Planejamento_H%C3%ADdrico_UPH/MapServer) `0` | reprodução permitida, desde que citada a fonte | sim | não |
| Água › Bacias | Macrorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_840_MacroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Mesorregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_835_MesoRegHidro` | uso livre com citação da fonte | sim | não |
| Água › Bacias | Microrregiões hidrográficas (DHN 2021, IBGE) | IBGE, Divisão Hidrográfica Nacional (DHN), 2021 | [WFS](https://geoservicos.ibge.gov.br/geoserver/wfs) `CGMAT:qg_2021_830_MicroRegHidro` | uso livre com citação da fonte | sim | não |
| Água › ANA | Disponibilidade hídrica superficial (ANA) | ANA, Disponibilidade Hídrica Superficial, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Disponibilidade_Hidrica_Superficial/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › ANA | Balanço hídrico quantitativo por curso d'água (ANA) | ANA, Balanço Hídrico Quantitativo por Curso de Água, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Balanco_Hidrico_Quantitativo_por_Curso_de_Agua/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › ANA | Estações da Rede Hidrometeorológica Nacional (ANA) | ANA, Rede Hidrometeorológica Nacional, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Rede_Hidrometeorologica_Nacional/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › ANA | Captações superficiais (ANA) | ANA, Captações Superficiais, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Captacoes_Superficiais/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › ANA | Pivôs centrais mapeados (ANA) | ANA, Pivôs Mapeados, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Pivos_Mapeados/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Água › ANA | Municípios atingidos por seca em 2015 (ANA) | ANA, Municípios Atingidos por Eventos de Seca em 2015, SNIRH | [ArcGIS REST](https://portal1.snirh.gov.br/server/rest/services/dados_abertos/Municipios_Atingidos_por_Eventos_de_Seca_em_2015/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Áreas protegidas | Unidades de conservação (CNUC 07/2026) | MMA, Cadastro Nacional de Unidades de Conservação (CNUC), 07/2026 | [WFS](https://geoservicos.inde.gov.br/geoserver/MMA/ows) `MMA:cnuc_26_07_31` | não declarada no serviço | sim | sim |
| Áreas protegidas | Unidades de conservação federais (ICMBio) | ICMBio, Limites das Unidades de Conservação Federais, 08/2026 | [WFS](https://geoservicos.inde.gov.br/geoserver/ICMBio/ows) `ICMBio:limiteucsfederais_a` | não declarada no serviço | sim | sim |
| Áreas protegidas | Terras indígenas (FUNAI, via Infra S.A.) | FUNAI, Terras Indígenas (cópia publicada pela Infra S.A.) | [ArcGIS REST](https://geo.infrasa.gov.br/server/rest/services/Terras_Ind%C3%ADgenas/MapServer) `0` | reprodução permitida citando a fonte (FUNAI) | sim | sim |
| Fundiário | Imóveis rurais (CAR) | Serviço Florestal Brasileiro, Cadastro Ambiental Rural (SICAR); dados autodeclarados, sem os cancelados | [WFS](https://geoserver.car.gov.br/geoserver/sicar/wfs) `sicar:sicar_imoveis_{uf}` | licença não declarada no serviço | só no app web | sim |
| Fundiário › INCRA | Parcelas certificadas (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Fundiário › INCRA | Parcelas particulares (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Fundiário › INCRA | Assentamentos e lotes (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Fundiário › INCRA | Glebas públicas (SIGEF) | INCRA, Sistema de Gestão Fundiária (SIGEF), Geoportal INCRA | [WFS](https://geoportal.incra.gov.br/geoserver/ows) `geonode:sigef_geo` | licença não declarada no serviço | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Cerrado) | INPE, PRODES Cerrado (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-cerrado-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Caatinga) | INPE, PRODES Caatinga (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-caatinga-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Amazônia Legal) | INPE, PRODES Amazônia Legal (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-legal-amz:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Mata Atlântica) | INPE, PRODES Mata Atlântica (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-mata-atlantica-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Pantanal) | INPE, PRODES Pantanal (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-pantanal-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › PRODES | Desmatamento anual (PRODES Pampa) | INPE, PRODES Pampa (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `prodes-pampa-nb:yearly_deforestation` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › DETER | Alertas de desmatamento (DETER Cerrado) | INPE, DETER Cerrado (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `deter-cerrado-nb:deter_cerrado` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos › DETER | Alertas de desmatamento (DETER Amazônia) | INPE, DETER Amazônia (TerraBrasilis) | [WFS](https://terrabrasilis.dpi.inpe.br/geoserver/ows) `deter-amz:deter_amz` | CC BY-SA 4.0 | sim | sim |
| Desmatamento e embargos | Áreas embargadas (IBAMA) | IBAMA, Termos de embargo (SISCOM, PAMGIA) | [ArcGIS REST](https://pamgia.ibama.gov.br/server/rest/services/SISCOM/publico/MapServer) `3` | dados abertos; licença não declarada no serviço | sim | sim |
| Energia, minas e transportes › ANEEL | Usinas hidrelétricas (ANEEL) | ANEEL, SIGEL, Usinas Hidrelétricas - UHE | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `2` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANEEL | Pequenas centrais hidrelétricas (ANEEL) | ANEEL, SIGEL, Pequenas Centrais Hidrelétricas - PCH | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `3` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANEEL | Centrais geradoras hidrelétricas (ANEEL) | ANEEL, SIGEL, Centrais Geradoras Hidrelétricas - CGH | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `4` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANEEL | Centrais geradoras eólicas (ANEEL) | ANEEL, SIGEL, Central Geradora Eólica - EOL | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `0` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANEEL | Centrais solares fotovoltaicas (ANEEL) | ANEEL, SIGEL, Centrais Geradoras Solares Fotovoltaicas - UFV | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `21` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANEEL | Usinas termelétricas (ANEEL) | ANEEL, SIGEL, Usinas Termelétricas - UTE | [ArcGIS REST](https://sigel.aneel.gov.br/arcgis/rest/services/PORTAL/Camadas_Downloads/FeatureServer) `18` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Bacias sedimentares (ANP) | ANP, GeoMaps, Bacias Sedimentares | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:bacias_gishub_db` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Blocos exploratórios (ANP) | ANP, GeoMaps, Blocos Exploratórios | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:bloco_view_geoanp` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Campos de produção (ANP) | ANP, GeoMaps, Campos de Produção | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:campos_gishub_db` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Poços de petróleo e gás (ANP) | ANP, GeoMaps, Poços | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `sisroc:well_view_geoanp` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Gasodutos de transporte (ANP) | ANP, GeoMaps, Gasodutos de Transporte | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:gasodutos_de_transporte` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Oleodutos de transporte (ANP) | ANP, GeoMaps, Oleodutos de Transporte | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:oleodutos_de_transporte` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANP | Postos de combustível (ANP) | ANP, GeoMaps, Postos de Combustível | [WFS](https://geomaps.anp.gov.br/geoserver/ows) `geoanp:postos_de_combustivel` | dados abertos; licença não declarada no serviço | não (sem CORS) | sim |
| Energia, minas e transportes › ANM | Processos minerários ativos (ANM) | ANM, SIGMINE, processos minerários ativos | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `0` | dados abertos, com crédito à ANM | sim | sim |
| Energia, minas e transportes › ANM | Áreas de bloqueio (ANM) | ANM, SIGMINE, áreas de bloqueio | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `3` | dados abertos, com crédito à ANM | sim | sim |
| Energia, minas e transportes › ANM | Reservas garimpeiras (ANM) | ANM, SIGMINE, reservas garimpeiras | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/SIGMINE/dados_anm/MapServer) `4` | dados abertos, com crédito à ANM | sim | sim |
| Energia, minas e transportes › ANM | Barragens de mineração (ANM, SIGBM) | ANM, Sistema Integrado de Gestão de Barragens de Mineração (SIGBM), painel público | [ArcGIS REST](https://geo.anm.gov.br/arcgis/rest/services/Producao/Barragens_Dashboard_Publico/FeatureServer) `0` | dados abertos, com crédito à ANM | sim | sim |
| Energia, minas e transportes | Rodovias federais (SNV vigente, DNIT) | DNIT, Sistema Nacional de Viação (SNV), versão vigente | [WFS](https://servicos.dnit.gov.br/dnitgeo/geoserver/ows) `vgeo:vw_snv_rod` | sem restrições declaradas | sim | sim |
<!-- fontes: fim -->

Fora da tabela, com a citação no comando **Fontes**:
- **Elevação:** Copernicus DEM GLO-30 (© DLR e.V. e © Airbus Defence and Space GmbH, sob o
  programa Copernicus da União Europeia e da ESA), baixado do espelho da
  [OpenTopography](https://opentopography.org), que abre também no app web;
- **Indicadores por município:** IBGE (Censo Demográfico 2022, Censo Agropecuário 2017), Banco
  Central (Pix, crédito rural), TSE e Novo CAGED;
- **Setores censitários:** IBGE, Censo Demográfico 2022, agregados por setores;
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
- **Tabelas embutidas:** `municipios.csv`, `tse.csv` e `caged.csv` são refeitas pelos
  `scripts/baixar_*.py`.
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
