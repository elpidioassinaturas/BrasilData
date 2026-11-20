# Seca: umidade do solo, vegetação e SPI (Copernicus GDO)

<!-- Gerado por scripts/gerar.py a partir do fontes.csv e do fichas.csv; não edite à mão. -->

Mapas de seca do Global Drought Observatory do Copernicus: a anomalia da umidade do solo (modelo LISFLOOD), a anomalia da vegetação (fAPAR do VIIRS) e o SPI de 1, 3, 6 e 12 meses do CHIRPS.

*Sem mapa exemplo: a camada não chega pelo `rustgis-cli`, que faz os mapas (veja App web).*

| | |
|---|---|
| **Onde na aba** | Tempo e clima › Clima |
| **Fonte** | Copernicus Emergency Management Service (Centro Comum de Investigação da Comissão Europeia). |
| **Período** | de 10 em 10 dias (o SPI de 6 e 12 meses, por mês): umidade do solo desde 1995, SPI desde 1991, vegetação desde 2012, até a data mais recente. |
| **Resolução ou escala** | 3′ (cerca de 5 km) na umidade do solo, 0,05° no SPI do CHIRPS, cerca de 1 km no fAPAR; o mapa é a imagem do serviço. |
| **Unidades** | sem unidade: anomalias e SPI em desvios-padrão, nas classes de cor do GDO. |
| **Legenda** | A legenda do serviço do GDO, sob a camada no painel Camadas: marrom é mais seco que o normal, azul é mais úmido. |
| **Cuidados de uso** | É um mapa para ver, não para medir: os valores não vêm nele, e ele não é recortado pela área. O Low-Flow Index (vazão baixa nos rios) do GDO só cobre a Europa. |
| **Licença** | CC BY 4.0 |
| **Citação** | Copernicus Emergency Management Service, Global Drought Observatory (GDO), Centro Comum de Investigação da Comissão Europeia. |
| **App desktop** | sim |
| **App web** | sim (o serviço do GDO libera a leitura pelo navegador). |

[Todas as camadas](README.md)
