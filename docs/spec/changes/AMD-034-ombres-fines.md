# AMD-034 — Ombres des traits fins : résolution de la carte d'ombres et biais

Version : 1.0 | 2026-10-23 | **Normatif** (R0, écart signalé par l'utilisateur). Modifie SPEC-06 REQ-SCN-030 (taille de la carte d'ombres) et la règle du biais issue de Q-032.

## 1. Constat (mesuré)

Sur la carte du monde 1 de Super Mario Bros. 3, en 1920 × 1080 avec la caméra par défaut, le texte de la barre de statut est moins lisible que dans l'original.
- **Les couleurs sont déjà justes.** Le bleu du panneau mesure (0, 219, 223) éclairé et (0, 144, 146) à l'ombre, contre (0, 216, 220) et (0, 143, 145) dans l'original : la lumière ambiante, la force des ombres (facteur 0,2) et la direction de la lumière sont correctes et **ne doivent pas changer**.
- **Ce qui diffère, c'est la quantité d'ombre.** Les lettres de profondeur 32 dépassent de 8 unités devant le panneau, et leurs ombres sur le panneau forment une bande sombre continue sous chaque trait dans l'original. Dans Zaxnes, ces ombres se réduisent à des pointillés : 920 pixels de bleu ombré dans la zone de la barre, contre 5 188 dans l'original.
- **La cause est la résolution.** Avec une carte d'ombres de 1024 texels de côté sur la zone couverte, un texel est plus large qu'un trait d'une unité, si bien que l'ombre des traits fins se fragmente. Le biais minimal de 1,5 texel raccourcit encore ces ombres courtes.

## 2. Règle

- La carte d'ombres unique fait **4096 texels de côté** par défaut, au lieu de 1024. Le reste de REQ-SCN-030 ne change pas : ombres franches, une seule carte, zone couverte inchangée.
- Le biais de profondeur vaut **au moins 1 texel** de la carte (au lieu de 1,5), sans descendre sous 0,02 unité. Le décalage selon la normale ne change pas (0,02 unité).
- Si le matériel ne permet pas 4096, on prend la plus grande taille possible (2048, puis 1024) et on le signale une fois dans le journal.
- La lumière ambiante, la force des ombres et la direction de la lumière ne changent pas.

## 3. Critères d'acceptation

- **SMB3, carte du monde 1** : ROM de l'utilisateur, profil intégré « Super Mario Bros 3 (U) (PRG1) », 900 images émulées avec Start appuyé aux images 300 à 305 et 420 à 425, rendu en 1920 × 1080 avec la caméra par défaut.
  - Dans le rectangle x ∈ [490, 1080), y ∈ [865, 950), le nombre de pixels de bleu ombré (rouge < 20, vert et bleu entre 120 et 170) atteint **au moins 4 000**. Référence de l'original : 5 188, dans `verify/reference/smb3-map-original-1080p.png`.
  - Les couleurs éclairée et ombrée du panneau restent à ± 4 près des valeurs du §1.
- **Pas d'acné** : aucune moirure ni ombre parasite sur les surfaces éclairées de Zelda II (scène d'herbe), Excitebike (piste) et Elevator Action, ni sur les scènes synthétiques, avec des captures avant/après.
- **Performance** : la durée médiane d'une image reste sous 4 ms en 1920 × 1080 sur la machine de l'utilisateur, et le test de 60 i/s en 1080p continue de passer quand on le lance seul.
