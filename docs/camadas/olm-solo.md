# Propriedades do solo a 30 m (OpenLandMap)

<!-- Gerado por scripts/gerar.py a partir do fontes.csv e do fichas.csv; não edite à mão. -->

Uma propriedade do solo a 30 m do OpenLandMap-soildb (OpenGeoHub): carbono orgânico, densidade de carbono, pH em água, densidade do solo, argila, areia ou silte, de 0 a 30, de 30 a 60 ou de 60 a 100 cm, prevista por aprendizado de máquina espaço-temporal sobre cerca de 400 mil amostras de solo do mundo inteiro.

*O mapa exemplo, de Viçosa (MG), ainda não foi gerado: rode `scripts/mapas_exemplo.py olm-solo`.*

| | |
|---|---|
| **Onde na aba** | Meio físico › Solos |
| **Fonte** | OpenGeoHub (Land & Carbon Lab) |
| **Período** | carbono e pH de 5 em 5 anos, de 2000–2005 a 2020–2022; densidade do solo e textura, 2020–2022. |
| **Resolução ou escala** | 0,00025° (cerca de 30 m); numa área grande, a pirâmide do arquivo (60 m, 120 m...). |
| **Unidades** | g/kg (carbono), kg/m³ (densidade de carbono), pH, g/cm³ (densidade do solo) e % (argila, areia e silte). |
| **Legenda** | Escala contínua: o carbono do bege ao marrom-escuro, de 0 a 60 g/kg; o pH do vermelho (ácido, 4) ao verde (alcalino, 8,5); a densidade do solo, de 0,8 a 1,7 g/cm³; a argila, a areia e o silte, de 0 a 80 %. |
| **Cuidados de uso** | Previsões de um modelo, não análises de laboratório: mostram padrões, e o Brasil tem menos amostras que a Europa e os Estados Unidos. A diferença do carbono entre dois períodos só vale onde é grande. Para o mapa de solos por classes, os Solos do IBGE (BDiA). |
| **Licença** | CC BY 4.0 |
| **Citação** | Hengl, T. et al., OpenLandMap-soildb: global soil information at 30 m spatial resolution for 2000–2022+, Earth System Science Data 18, 989–1036 (2026); dados: Consoli, D. et al., doi:10.5281/zenodo.15470431. |
| **App desktop** | sim |
| **App web** | sim (o servidor da OpenGeoHub libera a leitura pelo navegador); precisa de um RustGIS depois da 0.5.0. |

[Todas as camadas](README.md)
