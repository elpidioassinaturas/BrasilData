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
| Limites e indicadores | Municípios, Estados, Indicadores por município (Censos, Pix, crédito rural, eleições, emprego formal) |
| Meio físico | Geologia, Geomorfologia, Solos, Vegetação (BDiA, IBGE) |
| Água | Drenagem, Massas d'água, ANA, Bacias (BHO 2017, UPH, divisão hidrográfica do IBGE) |
| Áreas protegidas | Unidades de conservação, UCs federais, Terras indígenas |
| Fundiário | Imóveis rurais (CAR), INCRA (SIGEF) |
| Desmatamento e embargos | PRODES, DETER, Embargos do IBAMA |
| Energia, minas e transportes | ANEEL, ANP, ANM, Rodovias federais (SNV) |

São 55 camadas de 13 órgãos, listadas em [`fontes.csv`](fontes.csv): serviço, camada, fonte,
licença e se a camada abre no app web. O comando **Fontes** mostra a citação e a licença de
cada camada trazida. Os indicadores são agregados por município: o plugin não traz CPF, CNPJ ou
nomes de pessoas.

Algumas camadas só abrem num dos aplicativos, e a ajuda de cada botão diz quando é o caso:
- 19 das 55 só abrem no app desktop, porque o servidor não manda os cabeçalhos CORS que o navegador exige;
- os imóveis do CAR só abrem no app web, porque o servidor só aceita cifras TLS antigas, que o app desktop não aceita.

## Instalar

- **Pelo registro:** *Plugins › Plugin Manager*, procure BrasilData.
- **Por arquivo:** baixe o `.rgplugin` da [última versão](https://github.com/elpidioassinaturas/rustgis-brasildata/releases)
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
