# AMD-012 — Transparence par objet et jointures entre tiles

Version : 1.0 | 2026-10-21 | **Normatif** (complète la spec v1.1 gelée ; révise AMD-009 pour Q-020)

| Point | Document | Amendement |
|---|---|---|
| Q-020 (révisé) | SPEC-06 REQ-SCN-040 | Passage de profondeur et tri des objets transparents **par objet entier**, et non par tile : les faces internes entre les tiles d'un objet translucide ne doivent pas être visibles. |
| Jointures | SPEC-04B REQ-E3D-505, SPEC-06 | **Nouvelle exigence REQ-SCN-065 (P2)** : en vue par défaut, deux tiles adjacentes d'une même surface plane et de même couleur NE DOIVENT PAS montrer de ligne visible à leur jointure (fissure, z-fighting entre faces confondues, discontinuité d'ombre). Critère : capture hors écran d'une surface de 4×4 tiles identiques, éclairée et recevant l'ombre → aucune colonne ni ligne de pixels à la jointure ne s'écarte de plus de 2 niveaux de la couleur de la surface. |
