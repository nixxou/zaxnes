# SPEC-00 — Vue d'ensemble et glossaire
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : toutes

## 1. Objet et périmètre
« Sen » est un émulateur NES pour Windows qui rend les jeux en 3D. À chaque frame, le cœur d'émulation décrit l'image NES sous forme de **tiles** (blocs de 8×8 pixels). Un moteur de conversion reconnaît ces tiles à l'aide d'un **profil** propre au jeu, les transforme en géométrie 3D, puis le rendu les affiche dans une scène éclairée, avec caméra et skybox. Les profils peuvent contenir des scripts Lua qui ajustent ce processus.

**Dans le périmètre** : jouer une ROM NES avec son profil (catalogue embarqué ou profil utilisateur), rendu 3D et mode 2D, caméra, lumière et ombres, skybox, backdrop, mode TV, entrées (clavier, manettes, Zapper à la souris), savestates, rollback, réglages, UI desktop, audio.
**Hors périmètre** : création et édition de profils (éditeur, outils de dessin, UI de scripting), VR, enregistrement vidéo, export de frames et d'OBJ, Steam, toute vérification anti-piratage.

## 2. Carte des documents
| Doc | Sujet | Module |
|---|---|---|
| SPEC-01 | Architecture et contraintes transverses | ARCH |
| SPEC-02 | Cœur NES et contrat des tiles | CORE |
| SPEC-03 | Format des profils, catalogue, association ROM ↔ jeu | PROF |
| SPEC-04 | Pipeline de conversion (frame → patterns, shapes, instances) | E3D 001–399 |
| SPEC-04B | Géométrie et meshes | E3D 400–799 |
| SPEC-05 | Runtime Lua des profils | LUA |
| SPEC-06 | Scène et rendu | SCN |
| SPEC-07 | Boucle d'émulation, savestates, rollback | LOOP |
| SPEC-08 | Entrées | INP |
| SPEC-09 | Réglages et persistance | PERS |
| SPEC-10 | Interface utilisateur | UI |
| SPEC-11 | Audio | AUD |
| SPEC-12 | Critères d'acceptation | ACC |
| DECISIONS-R0 | Arbitrages normatifs des questions ouvertes | — |

## 3. Glossaire
- **Tile** : bloc de 8×8 pixels de l'image NES, décrit par deux plans de bits (couleur 0 à 3 par pixel), une position, une palette, des retournements et un indicateur fond ou sprite (SPEC-02).
- **Image de tile** : le contenu en plans de bits d'une tile, indépendamment de sa position. Deux tiles ont la même image si leurs plans sont identiques.
- **Frame** : l'ensemble des tiles et de la palette produits par le cœur pour une image émulée (SPEC-02, SPEC-04).
- **Palette de frame** : les 32 octets de palette PPU capturés pour la frame ; les couleurs RVB viennent de la table fixe des 64 couleurs NES (SPEC-04).
- **Scène** : l'espace 3D où la frame est convertie, un pixel NES par unité. Hauteur de scène **Hs = 248** unités (31 rangées de tuiles). Le repère (image NES en y ∈ [8, 248], cadre de référence de centre C = (128, 120, 0), rectangle visible) est défini une seule fois par SPEC-04B REQ-E3D-410 (DEC-R2-T2).
- **Pattern** : motif enregistré dans un profil, fait d'une grille de tiles (images) et d'au moins une **variante 3D**. Il est reconnu dans la frame par appariement (SPEC-04).
- **Variante 3D** (aussi « instance de pattern ») : la description géométrique attachée à un pattern : type de géométrie, profondeur, transformations, couleurs, texture, couche (SPEC-03, SPEC-04B).
- **Pattern permanent / temporaire** : un pattern stocké dans le profil, ou appris pendant la partie et non sauvegardé (SPEC-04).
- **Shape** : groupe de tiles adjacentes reconnu comme un objet à l'écran pendant une frame ; elle est exposée à Lua sous le nom `shape2D` (SPEC-04 §2, SPEC-05 §4.1).
- **Instance de frame** : copie, pour une frame, d'une variante 3D du pattern de la shape ; c'est l'objet `shape` exposé à Lua (SPEC-04 REQ-E3D-042, SPEC-05).
- **Couche** : catégorie de profondeur et d'ombre d'un objet (`UNIDENTIFIED`, `UI`, `L1` à `L4`). Elle détermine notamment les ombres projetées et reçues (SPEC-04B, SPEC-06, SPEC-09).
- **Type de géométrie** : valeurs numériques de SPEC-03 (champ 2 de la variante 3D) : 0 `Default`, 1 `HCYLINDER`, 2 `VCYLINDER`, 3 `CUBE`, 4 `HALFHCYLINDER`, 5 `HALFVCYLINDER` (sémantique : SPEC-04B).
- **Algorithme de mesh** : méthode de construction de la surface d'un objet voxelisé (GREEDY, MARCHING, etc., SPEC-04B).
- **Frame épinglée** : une frame conservée comme référence pour certaines règles de conversion (SPEC-04).
- **Tracker** : suivi de l'identité et du mouvement des objets d'une frame à l'autre (SPEC-04).
- **Profil** : fichier protobuf (`.bytes` embarqué ou `.3dn` utilisateur) qui contient patterns, scripts et réglages pour un jeu (SPEC-03).
- **Catalogue** : la liste des profils embarqués (`assets-neutral/profiles.json`, édition `full`).
- **Jeu** : une entrée de la bibliothèque (profil + ROM associée par l'utilisateur). « 2D Mode » est l'entrée spéciale qui joue n'importe quelle ROM sans conversion 3D (SPEC-03, SPEC-06).
- **Script** : code Lua contenu dans un profil, avec des **hooks** (`Start`, `Update`, `UpdateS`, `LateUpdate`, `End`, `Load`, `Save`, `InputProcess`, `BuildPattern`…) appelés à des moments définis (SPEC-05).
- **Réglage** : paramètre du modèle de réglages. Sa portée est U (préférence utilisateur), P (portée par le profil) ou session (SPEC-09).
- **Savestate** : instantané complet permettant de reprendre une partie (cœur, état appris de la conversion, caméra, données persistantes des scripts ; DEC-R2-T5). **Quick save** : l'emplacement rapide (SPEC-07, SPEC-09).
- **Rollback** : retour en arrière à partir d'instantanés réguliers du cœur (SPEC-07).
- **Hôte** : le programme qui pilote les bibliothèques : l'application desktop, ou plus tard le core libretro (SPEC-01).
- **Classe d'écart** : A omission de spec, B erreur de spec, C ambiguïté, D divergence acceptée (PLAN, round 2).

## 4. Conventions
- Coordonnées NES : x vers la droite, y vers le bas, origine en haut à gauche de l'image 256×240.
- Étiquettes de vérifiabilité (DEC-R2-T8, SPEC-12) : **[ROM]** = critère qui exige la ROM fournie par l'utilisateur (vérifiable au round 1 avec elle) ; **[R2]** = critère vérifiable seulement au round 2 par comparaison avec l'original.
- Les numéros d'exigence sont stables ; voir SPEC-GUIDE.

## Historique

- 1.1-draft (AMD-001, relecture R2) : noms Lua `shape` / `shape2D` corrigés ; valeurs numériques des types de géométrie ; définition de la scène renvoyée à REQ-E3D-410 ; contenu du savestate (DEC-R2-T5) ; étiquettes [ROM]/[R2].
