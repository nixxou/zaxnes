# AMD-041 — Run-ahead

Version : 1.2 | 2026-10-30 | **Normatif** (R1, choix du propriétaire du 2026-10-30). Complète SPEC-02, SPEC-07 et SPEC-09 ; **amende REQ-LOOP-060 (P1), REQ-ARCH-012 et REQ-ARCH-013 (P1)** et précise REQ-ACC-030 ; restreint DEC-07-Q4 au frontend libretro.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-039-045 : 41-1 à 41-10) ; 1.2 (seconde relecture R2-review-AMD-039-045-v1.1 : N-41-1 à N-41-3, T-2).

## 1. Besoin

3dSen 2.0 réduit la latence d'entrée par **run-ahead** : l'image affichée est celle de quelques
frames dans le futur, calculée avec les entrées courantes, puis l'émulation revient à la frame
réelle. Le joueur voit l'effet de sa manette 1 à 3 frames plus tôt. C'est une technique publique
(popularisée par RetroArch, variante « une seule instance »).

Vocabulaire : la **frame réelle** est celle dont l'état persiste ; les **frames en avance** sont
calculées puis abandonnées ; la **frame affichée** est la dernière frame en avance (ou la frame
réelle quand N = 0).

## 2. Exigences

**REQ-LOOP-064** (P2) — **Réglage** : un réglage utilisateur **Run-ahead** DOIT prendre les valeurs
0 (désactivé), 1, 2 ou 3 frames, et valoir **1 par défaut**. Toute valeur hors de [0, 3] est
ramenée dans l'intervalle. Il est persisté (SPEC-09, portée U ; la ligne de SPEC-09 §4.1 est
donnée au §3), modifiable pendant la partie avec effet à la frame réelle suivante, et exposé à Lua
en `Setting.RunAheadFrames` (AMD-040, REQ-LUA-114). L'interface DEVRAIT présenter les valeurs
avec les libellés du §3, la valeur 1 marquée « recommandée ».
- Critère d'acceptation (auto) : racine utilisateur vide → Run-ahead = 1 ;
  `Setting.RunAheadFrames = 7` → 3 ; `Setting.RunAheadFrames = -1` → 0 ; la valeur survit à un
  redémarrage.

**REQ-LOOP-065** (P2) — **Déroulement d'une frame réelle** (amende REQ-LOOP-060, étape b) : avec
Run-ahead = N ≥ 1 et les entrées E lues à l'étape (a) de REQ-LOOP-060 :
1. émuler la frame réelle avec E, **sans** produire d'image pour l'affichage ni de son ;
2. garder en mémoire l'état complet du cœur (état R, REQ-CORE-121) ;
3. émuler N − 1 frames en avance avec les mêmes entrées E, sans image ni son ;
4. émuler une dernière frame en avance avec E, **avec** son et image : c'est la frame affichée ;
5. restaurer l'état R, sans annuler le son déjà produit.

Avec N = 0, la frame est émulée normalement, une seule fois, et c'est la frame affichée.
- Critère d'acceptation (auto, vecteur) : ROM de test qui affiche en permanence, sur l'image d'une
  frame, la valeur qu'avait au **début** de cette frame un compteur incrémenté à chaque frame et l'état du bouton A lu à la frame courante. Run-ahead = 2 ;
  juste avant la frame réelle k, le compteur du cœur vaut k ; A pressé à cette frame : l'image
  affichée montre le compteur k + 2 et A pressé ; après l'étape 5, le compteur du cœur vaut
  **k + 1**, celui de la frame réelle qui vient d'être exécutée.
- Notes (non normatives) : l'état R n'a pas besoin du format portable des savestates ; le coût de
  sa sauvegarde et de sa restauration s'ajoute à chaque frame réelle.

**REQ-LOOP-060** (amendée) — L'ordre d'une frame devient : (a) lecture des entrées des deux
manettes ; (b) exécution de la frame réelle et des frames en avance (REQ-LOOP-065) ;
(c) conversion 3D de la **frame affichée** (SPEC-04) ; (d) hook Lua de mise à jour, sur la frame
affichée (SPEC-05). La capture de snapshot de rollback a toujours lieu avant (a), sur l'état réel.
Avec N = 0, cet ordre est celui d'origine.

**REQ-LOOP-066** (P2) — **Ce qui suit la frame réelle** : l'état du cœur et son compteur de frames
(REQ-CORE-006), l'historique de rollback (REQ-LOOP-030, AMD-042), la partie « cœur » des
savestates et quick saves (REQ-LOOP-040), l'enregistrement des entrées et le pas à pas DOIVENT
porter sur l'état réel R, jamais sur un état en avance. La caméra, les données persistantes des
scripts et l'état appris de la conversion, eux, évoluent avec les frames affichées (§ suivant) :
un savestate les enregistre tels qu'ils sont, ce qui reste cohérent, la suite des frames affichées
étant contiguë. Charger un état, faire un reset ou changer de jeu reprend la frame suivante avec le
même N.
- Critère d'acceptation (auto) : Run-ahead = 3, quick save puis quick load → le compteur de frames
  du cœur est celui de la frame réelle de la sauvegarde ; l'historique capture toujours un snapshot
  à chaque multiple de 60 du compteur réel.

**REQ-LOOP-067** (P2) — **Ce qui voit la frame affichée** : la conversion 2D → 3D, les hooks de
frame Lua (`Update`, `UpdateS`, `LateUpdate`, SPEC-05 §3.4) et l'audio DOIVENT recevoir une seule
frame par frame réelle : la frame affichée. Les hooks Lua NE DOIVENT PAS être appelés pour la frame
réelle ni pour les frames en avance non affichées. Pendant ces hooks, la globale `frame` et
`Script:ReadMem` DOIVENT refléter la frame affichée (compteur et RAM cohérents avec les tiles
converties), bien que l'étape 5 de REQ-LOOP-065 ait lieu pendant (b) : l'implémentation garde
l'état de la frame affichée pour servir ces lectures, ou retarde la restauration après (d) ; l'ordre
est libre tant que la frame réelle suivante repart de R.
- Critère d'acceptation (auto) : un script qui incrémente un compteur dans `Update` augmente de 1
  par frame réelle, quel que soit N ; avec Run-ahead = 2, `Script:ReadMem` sur l'adresse du
  compteur de la ROM de REQ-LOOP-065 renvoie la valeur affichée (k + 2).

**REQ-LOOP-068** (P2) [ROM] — **Zapper** : quand un port est de type `Zapper` (REQ-INP-061), **chaque**
frame émulée, réelle ou en avance, DOIT rendre ses pixels et détecter la lumière sur ses propres
pixels, exactement comme sans run-ahead (SPEC-02, SPEC-08). Ainsi la détection de la frame réelle,
dont l'effet persiste, porte sur l'image que cette frame produit elle-même. Sans Zapper branché,
les frames non affichées PEUVENT omettre le rendu de leurs pixels, pourvu que tout le reste du
comportement du cœur (temps, drapeau de sprite 0, IRQ) reste exact.
- Critère d'acceptation (auto) : la ROM de test Zapper publique de SPEC-08 (ou une ROM maison qui
  affiche une cible et lit la lumière) donne les mêmes résultats avec Run-ahead = 1 qu'avec 0.
- Critère d'acceptation [ROM] : « Duck Hunt » à la souris, un tir sur un canard le touche avec
  Run-ahead = 1 comme avec 0 ; « Super Mario Bros. » garde sa barre d'état fixe avec Run-ahead = 3
  (sprite 0 exact sans rendu des pixels).

**REQ-CORE-121** (P2) — **État en mémoire du cœur** : le cœur DOIT offrir une sauvegarde et une
restauration de son état complet en mémoire (mémoire vive, mapper, CPU, PPU, APU, entrées, compteur
de frames), restaurables à l'identique, et une restauration qui laisse intacte la sortie audio déjà
produite. L'image de la ROM, qui reste chargée, n'en fait pas partie ; la taille de l'état est
bornée.
- Critère d'acceptation (auto) : sauvegarder, émuler 100 frames, restaurer, émuler les mêmes 100
  frames avec les mêmes entrées → images et RAM identiques octet pour octet.

**REQ-ARCH-012 et REQ-ARCH-013** (amendées) — La valeur de Run-ahead fait partie des entrées du
contrat de déterminisme : les suites de frames, de tiles, de résultats de conversion et d'état Lua
ne sont comparées qu'à **N égal** (critère de REQ-LOOP-060 et exports du harnais compris). La suite
des états du cœur réels, elle, ne dépend pas de N.

**REQ-ACC-030** (précisée) — La mesure de performance se fait au réglage par défaut, donc avec
Run-ahead = 1 (deux frames émulées par frame réelle).

## 3. Réglage dans SPEC-09 et libellés

Ligne §4.2 : `RunAheadFrames`, accès L, É, retour entier (AMD-040 §3).

| Clé | Lua | N° | Type | Défaut | Bornes | Portée | Effet | UI | Libretro |
|---|---|---|---|---|---|---|---|---|---|
| `emu.run_ahead` | `RunAheadFrames` | — | entier | 1 | 0 à 3 | U | à la frame réelle suivante | liste « Run-ahead » | non exposé (§4) |

Libellés (AMD-035, anglais par défaut / français) :

| Clé de texte | Anglais | Français |
|---|---|---|
| titre du réglage | `Run-ahead` | `Run-ahead` |
| valeur 0 | `Off` | `Désactivé` |
| valeur 1 | `1 frame (Recommended)` | `1 frame (recommandé)` |
| valeur 2 | `2 frames` | `2 frames` |
| valeur 3 | `3 frames` | `3 frames` |

## 4. Libretro

En libretro, le run-ahead est fourni par l'hôte (RetroArch) à partir de la sérialisation
(REQ-LOOP-063) : le réglage n'y est pas exposé et y vaut 0. DEC-07-Q4 reste vraie pour ce
frontend, et les phrases « le run-ahead n'est pas pris en charge en v1 » de REQ-LOOP-063 et
REQ-LUA-075, qui portent sur libretro, sont **inchangées**.

## 5. Questions

- Aucune.
