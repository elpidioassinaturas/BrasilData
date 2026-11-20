# Altura do dossel da Amazônia a 4,78 m (CTrees)

<!-- Gerado por scripts/gerar.py a partir do fontes.csv e do fichas.csv; não edite à mão. -->

A altura das árvores a 4,78 m na floresta amazônica, média de 2020 a 2024, estimada pela CTrees com um modelo de IA (U-Net) sobre as imagens Planet NICFI, calibrado com lidar aéreo da Amazônia.

*O mapa exemplo, de Teixeirópolis (RO), ainda não foi gerado: rode `scripts/mapas_exemplo.py chm-amazonia`.*

| | |
|---|---|
| **Onde na aba** | Vegetação e meio ambiente › Vegetação |
| **Fonte** | CTrees |
| **Período** | média de 2020 a 2024 |
| **Resolução ou escala** | 4,78 m (tiles de zoom 11 do mosaico Planet NICFI); numa grade mais grossa, a pirâmide do arquivo (até cerca de 38 m). |
| **Unidades** | metros |
| **Legenda** | Escala contínua de 0 (sem árvore) a 35 m ou mais, nas cores da altura do GEDI e do CHMv2. |
| **Cuidados de uso** | Só a floresta amazônica: fora dela não há arquivo. Modelo de IA: compare com a altura do GEDI (laser) e com o CHMv2 da Meta (cerca de 1 m, o mundo inteiro). Até 16 tiles de cerca de 20 km por clique. |
| **Licença** | CC BY 4.0 nos metadados do bucket; a página do AWS Open Data diz que segue o contrato do Planet NICFI. |
| **Citação** | Wagner, F. H. et al. (2025), High Resolution Tree Height Mapping of the Amazon Forest using Planet NICFI Images and LiDAR-Informed U-Net Model, Remote Sensing of Environment (arXiv:2501.10600); CTrees.org, Canopy Tree Height of the Amazon Forest (2025). |
| **App desktop** | sim |
| **App web** | sim (o bucket da AWS libera a leitura pelo navegador); precisa de um RustGIS depois da 0.5.0. |

[Todas as camadas](README.md)
