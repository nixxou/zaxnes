# AMD-042 — Le rollback restaure les données des scripts et la caméra

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Amende SPEC-07 REQ-LOOP-030 (Notes) à 032 et SPEC-06 REQ-SCN-024 (« échéance » → « temps restant ») ; **complète DEC-07-Q3** : l'extension du rollback, option P3, devient une exigence P2, limitée à `PData` et à la caméra (l'état appris reste hors snapshot).
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 42-1 à 42-5) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-42-1, N-42-2).

## 1. Constat

Dans 3dSen 1.1.2, un snapshot de rollback ne contient que l'état du cœur (REQ-LOOP-030, Notes).
Après un retour en arrière, les données persistantes des scripts (`PData`, SPEC-05 §3.9) et la
caméra restent à leur valeur d'après : un profil qui compte, mémorise ou suit quelque chose dans
`PData` se désynchronise du jeu, et la vue ne revient pas en arrière avec lui. 3dSen 2.0 corrige
cela.

## 2. Exigences

**REQ-LOOP-069** (P2) — **Contenu d'un snapshot** (remplace la dernière phrase des Notes de
REQ-LOOP-030) : chaque snapshot de l'historique DOIT contenir :
- l'état complet du cœur (comme avant ; un état en mémoire, REQ-CORE-121, suffit) ;
- les **64 valeurs de `PData`** au moment de la capture, lues comme `GetPData(i)` les lit ;
- l'**état de la caméra** tel que l'enregistre un savestate, défini par REQ-SCN-024 (amendée
  ci-dessous), qui en est la référence unique.

Le snapshot ne contient toujours ni l'état appris de la conversion, ni les frames converties,
ni les maillages. Mémoire : 30 × (état du cœur + 64 nombres + état de caméra).
- Critère d'acceptation : voir REQ-LOOP-070.

**REQ-SCN-024** (amendée) — L'état de caméra d'un savestate enregistre le **temps restant** de la
fenêtre de transition de mode (entre 0 et 3 s) au lieu de son échéance : une échéance absolue
serait périmée au rechargement. Au chargement, ce temps restant est restauré (AMD-045,
REQ-SCN-073).

**REQ-LOOP-070** (P2) — **Restauration** (complète REQ-LOOP-032 étape 2 et REQ-LOOP-033) :
restaurer un snapshot DOIT, en plus du cœur, réécrire les 64 valeurs de `PData` (comme
`SetPData`) puis rétablir l'état de la caméra. Les hooks `Save` et `Load` des scripts NE DOIVENT
PAS être appelés par le rollback (ils restent réservés aux savestates et à la sérialisation,
REQ-LUA-075). La caméra rétablie DOIT être affichée **directement** à ses consignes, sans lissage
depuis la vue d'avant (AMD-045, REQ-SCN-073).
- Critère d'acceptation (auto) : Run-ahead = 0 (AMD-041) ; un profil de test écrit
  `PData[0] = frame` dans `Update` ; NTSC, jouer 10,5 s, rollback → juste après la restauration,
  `PData[0]` a retrouvé la valeur qu'il avait au moment où le snapshot restauré a été capturé
  (celui du compteur 540, REQ-LOOP-032), et non plus environ 630 ; tourner la caméra de 90° après
  la capture, rollback → la première image après la restauration montre l'orientation du moment
  de la capture.

**REQ-LOOP-071** (P2) — **Vidage** (complète REQ-LOOP-031) : l'historique DOIT aussi être vidé à
la **fin** d'une partie (retour à la bibliothèque), et pas seulement au lancement suivant.
- Critère d'acceptation (auto, harnais) : jouer 10 s (au moins 9 snapshots), terminer la partie
  → immédiatement après la fin, l'historique compte 0 snapshot, avant tout nouveau lancement.

## 3. Ce qui ne change pas

- La cadence de capture (un snapshot au début de chaque frame dont le compteur réel est un
  multiple de 60), la profondeur (30 snapshots) et le pas d'un appui sur Rollback
  (REQ-LOOP-032 : l'avant-dernier snapshot). Note : 3dSen 2.0 recule d'un snapshot de plus au
  premier appui (le troisième en partant de la fin) ; ce changement n'est **pas** repris, le pas
  de REQ-LOOP-032 reste la référence.
- Avec run-ahead (AMD-041), les snapshots portent sur l'état réel du cœur ; `PData` et la caméra
  sont pris tels qu'ils sont au moment de la capture.

## 4. Questions

- Aucune.
