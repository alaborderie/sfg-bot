---
name: shared-rating-rubric
description: Barème partagé de notation Good/Average/Poor appliqué à tous les rôles. Composé automatiquement par AnalysisPipeline après le référentiel par compétence.
type: analysis-shared
---

## Barème de notation (obligatoire)

Attribue la note en pesant la partie ENTIÈRE, pas seulement la phase de lane :

- **Good** — le joueur a rempli les objectifs critiques de son rôle (voir le référentiel par compétence) et son impact global sur la partie a été clairement positif. Une défaite n'interdit pas Good si la performance individuelle était au-dessus du lot.
- **Average** — performance en demi-teinte : une phase de jeu ratée mais compensée ailleurs (ex : lane perdue mais bon impact mid/late), ou des chiffres corrects sans impact décisif sur la partie.
- **Poor** — les objectifs critiques du rôle sont ratés ET l'impact global a été négatif (morts évitables répétées, absence des objectifs, aucun plan de rattrapage visible).

Règles de cohérence — à respecter strictement :
- Une victoire où le joueur a un impact positif net (bonne participation aux kills, objectifs pris, dégâts utiles) ne peut JAMAIS être notée Poor.
- Une défaite ne force jamais Poor : juge la performance individuelle dans son contexte.
- Une lane perdue puis convertie en victoire par le scaling ou les teamfights vaut au minimum Average.
- Une lane dominée mais jamais convertie (pas de pression tourelles, pas de présence sur les objectifs) plafonne à Average, même en cas de victoire.
- La note doit rester cohérente avec les repères chiffrés du référentiel par compétence, pondérés par l'importance de chaque compétence pour le rôle.
