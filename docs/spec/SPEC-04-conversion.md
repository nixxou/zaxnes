# SPEC-04 — Pipeline de conversion 2D→3D (frame, reconnaissance des patterns, shapes, palettes, tracking)
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-02 (contrat du cœur : enregistrements de tiles, palette, masque), SPEC-03 (format de profil : patterns, instances, réglages embarqués), SPEC-04B (géométrie et meshes), SPEC-05 (API Lua), SPEC-06 (scène, caméra), SPEC-07 (boucle), SPEC-09 (réglages)

## 1. Objet et périmètre

Ce document spécifie ce que le moteur doit produire, pour chaque trame émise par le cœur, **avant** la construction des meshes :

1. la réception d'une trame (enregistrements de tiles et palette) et la construction du **modèle de frame** ;
2. la **reconnaissance des patterns** du profil dans la frame et le regroupement des tiles en **shapes** ;
3. le traitement des tiles **inconnues** (apprentissage de patterns par défaut, patterns temporaires) ;
4. l'état par frame des **instances** liées et leurs propriétés géométriques (boîtes, position) ;
5. la **palette** de frame et la table de couleurs NES ;
6. la **gestion des frames** (tampons, frame épinglée, injection de shapes par script) ;
7. le **tracker** (suivi d'une shape d'une frame à l'autre) ;
8. les **points d'appel des hooks Lua** dans ce pipeline ;
9. les attentes de **performance** et les vecteurs de test.

Hors périmètre :
- la transformation d'un pattern en géométrie, textures et atlas (types de géométrie, profondeur, algorithmes de maillage, couleur vide, déformation) : SPEC-04B. Le présent document fixe seulement **quand** une géométrie est (re)construite et **quels paramètres** reçoit un pattern créé automatiquement ;
- le dessin (matériaux, ombres, transparence, ordre de rendu) : SPEC-06 ;
- l'éditeur de profils (sélection de tiles ou de shapes, ajustement manuel, enregistrement de patterns) : hors périmètre du projet. Les modes de sélection n'ont aucun effet sur le pipeline (REQ-E3D-345) ;
- le mode « ROM » de l'outil de création (fichiers .3dn ouverts directement) : hors périmètre (voir SPEC-07).

Sauf mention contraire, tout ce qui suit décrit le **mode 3D** (jeu lancé depuis un profil). Le **mode 2D** (entrée de bibliothèque « 2D Mode », sans profil de jeu) ne diffère que par les points listés en 3.17.

## 2. Concepts et vocabulaire

Les termes suivants complètent le glossaire de SPEC-00. Coordonnées écran : origine en haut à gauche, x vers la droite, y vers le bas, en pixels. Coordonnées scène : x vers la droite, **y vers le haut**, z en profondeur (voir REQ-E3D-205).

- **Enregistrement de tile** : élément de la liste émise par le cœur à chaque trame (SPEC-02) : plans de bits `low` et `high` (64 bits chacun), position écran `(x, y)` en pixels (entiers signés, le coin haut-gauche de la tile), `flipX`, `flipY`, `hidden` (sprite derrière le fond), attribut de palette `pal` (0–3), indicateur fond/sprite, et pour un sprite son **indice** `ord` (0–127, REQ-CORE-043).
- **Image de tile** : la valeur `(low, high, flipX, flipY)`. Deux tiles ont la même image si et seulement si les quatre composantes sont égales. Une même donnée CHR affichée retournée est donc une **autre** image. L'image est l'unité d'identité de toute la reconnaissance.
- **Pixel d'image** : la valeur 0–3 d'un pixel `(px, py)` (0 ≤ px, py ≤ 7, dans l'orientation affichée) d'une image ; 0 = transparent. Le décodage des plans et l'effet des flips sont fixés par SPEC-02.
- **Grille de fond** : tableau de cellules de `Lg × Hg` = **35 × 33** (la zone utile du cœur, 33 × 31 tiles, plus une marge d'une cellule de chaque côté). Une tile de fond à la position `(x, y)` occupe la cellule `(⌊(x+7)/8⌋ + 1, ⌊(y+7)/8⌋ + 1)` (division entière par défaut vers −∞).
- **Grille de sprites** : grille de même dimension, sans marge : un sprite à `(x, y)` occupe la cellule `(⌊x/8⌋, ⌊y/8⌋)`.
- **Pattern** : entrée du profil (SPEC-03) qui décrit un objet : une grille `W × H` de cellules dont chacune est vide ou contient une image de tile, des paramètres de reconnaissance (`selfConnected`, `skipGeo`, taille de contrainte `(cW, cH)`) et une liste d'au moins une **instance** (appelée « variante 3D » dans SPEC-03 : paramètres 3D : géométrie, profondeur, couche, offset, échelle, rotation, pivot, tags, texture… — SPEC-03/SPEC-04B). Un pattern est **permanent** ou **temporaire**.
- **Ensemble d'images d'un pattern** `Img(P)` : l'ensemble des images distinctes présentes dans sa grille. `|Img(P)|` est son **nombre d'images distinctes**.
- **Registre des patterns** : l'ensemble des patterns permanents et l'ensemble des patterns temporaires actifs pendant la session (chargés du profil ou créés pendant le jeu).
- **Image connue** : image appartenant à `Img(P)` d'au moins un pattern permanent du registre. Sinon l'image est **inconnue** (même si elle figure dans un pattern temporaire).
- **Pattern désactivé** : pattern permanent dont l'indicateur `Disable` (propriété modifiable par script, non sauvegardée, faux par défaut) est vrai. Il n'est jamais candidat (REQ-E3D-093).
- **Shape** : groupe de tiles d'une même frame et d'une même couche (fond ou sprites), lié à exactement un pattern (son **pattern de référence**). Une shape porte autant d'**instances de frame** que son pattern de référence a d'instances.
- **Instance de frame** : copie, valable pour une seule frame, des paramètres d'une instance du pattern de référence (REQ-E3D-190). C'est l'objet que les scripts reçoivent sous le nom `shape` (SPEC-05) ; la shape elle-même est exposée sous le nom `shape2D`.
- **Boîte en tiles** d'une shape : plus petit rectangle de cellules (de la grille de sa couche) contenant toutes ses tiles ; `tStart`, `tEnd` (inclus), `tSize = tEnd − tStart + (1,1)`.
- **Boîte en pixels** d'une shape : `pStart`, `pEnd` (inclus), plus petit rectangle écran contenant tous les pixels non transparents de ses tiles (REQ-E3D-044).
- **Masque** `(mX, mY)` : marges écran (pixels) fournies par le cœur avec chaque trame (SPEC-02), dans lesquelles l'image peut contenir des artefacts de défilement.
- **Frame** : modèle complet issu d'une trame : tiles, shapes ordonnées, instances, palette, masque, compteur.
- **Hauteur scène** `Hs` = 248 (31 × 8 pixels) : constante qui sert à retourner l'axe y (REQ-E3D-205). Le repère de scène complet (image NES en `y ∈ [8, 248]`, cadre de référence, rectangle visible) est défini par SPEC-04B REQ-E3D-410.

## 3. Exigences

### 3.1 Réception d'une trame

**REQ-E3D-001** (P1) — Pour chaque trame émise par le cœur (enregistrements de tiles, palette de 32 octets, masque `(mX, mY)`), le moteur DOIT produire exactement une frame, en exécutant dans cet ordre : (a) construction de la palette de frame (3.11) ; (b) filtrage et placement des tiles (REQ-E3D-002 à 008) ; (c) segmentation du fond (3.5), puis des sprites (3.6) ; (d) hooks Lua de mise à jour (3.14) ; (e) calcul des positions des instances (REQ-E3D-205) ; (f) publication de la frame (3.12).
- Critère d'acceptation : en harnais headless, une trame injectée produit une frame dont le compteur vaut celui de la trame ; les scripts `Update` observent des shapes déjà segmentées et la position (`Pos`) observée après la frame tient compte des offsets modifiés par `UpdateS`.
- Notes : SPEC-07 (REQ-LOOP-060) place cette conversion entre l'exécution de la frame par le cœur et le rendu. Toutes les trames sont converties, y compris en avance rapide.

**REQ-E3D-002** (P1) — Une tile DOIT être écartée si elle sort de la **zone de découpe** définie par le réglage `Clipping` = `(g, b, d, h)` (marges gauche, bas, droite, haut en pixels, valeurs réelles), soit les composantes `x`, `y`, `z`, `w` du vecteur 4 du profil (SPEC-03, champ 4 des réglages) et les indices Lua 0, 1, 2, 3 (DEC-R2-T1) : la tile de position écran `(x, y)` est conservée si et seulement si `x + 8 > g` et `x ≤ 256 − d` et `y + 8 > h` et `y ≤ 248 − b`.
- Critère d'acceptation : avec les valeurs par défaut `(0,1 ; 8,1 ; 8,1 ; 0,1)` : tile de fond en `x = −8` écartée, en `x = −7` conservée ; en `x = 247` conservée, `x = 248` écartée ; en `y = 239` conservée, `y = 240` écartée ; en `y = −7` conservée.
- Notes : les valeurs par défaut et la persistance relèvent de SPEC-09 ; le profil peut les fournir (SPEC-03). Les mêmes marges servent à la découpe du rendu et au cadrage de la scène (SPEC-04B REQ-E3D-410, SPEC-06). Ce filtre, exprimé en coordonnées écran, est plus large que le rectangle visible du rendu : en haut, il conserve les tiles qui touchent les lignes `h` à `8 + h`, que le rendu masque ensuite ; c'est le comportement de l'original.

**REQ-E3D-003** (P1) — Une tile de **sprite** DOIT en outre être écartée si `y ≥ 240` ou si son image est vide (`low = 0` et `high = 0`).
- Critère d'acceptation : sprite en `y = 240` → absent de la frame ; sprite entièrement transparent → absent.

**REQ-E3D-004** (P1) — Une tile de **fond** dont l'image est vide DOIT être écartée si le réglage `EmptyTile` est faux, et conservée comme une tile ordinaire (image vide, 0 pixel non transparent) s'il est vrai.
- Critère d'acceptation : trame dont une cellule de fond est vide ; `EmptyTile` faux → la cellule est « absente » et sépare deux régions (REQ-E3D-041) ; `EmptyTile` vrai → la cellule peut être intégrée à une shape si un pattern contient l'image vide.
- Notes : valeur par défaut faux. 37 profils intégrés l'activent (ex. « Adventures of Lolo », « Castlevania (U) (PRG 1) »).

**REQ-E3D-005** (P1) — Chaque tile de fond conservée DOIT être placée dans la cellule de la grille de fond définie au §2 et conserver `x`, `y`, `pal`, `hidden`. Chaque sprite conservé DOIT conserver `x`, `y`, `pal`, `hidden`, `ord` et sa cellule de grille de sprites.
- Critère d'acceptation : tile de fond en `(−3, 13)` → cellule `(1, 3)` ; en `(5, 16)` → cellule `(2, 3)` ; sprite en `(13, 20)` → cellule `(1, 2)`.

**REQ-E3D-006** (P1) — L'ordre de **germination** des tiles de fond DOIT être : si le réglage `ScanDirection` vaut 0, par `y` croissant puis `x` croissant ; s'il vaut 1, par `x` croissant puis `y` croissant. L'ordre des sprites DOIT être leur ordre d'émission par le cœur.
- Critère d'acceptation : voir TV-E3D-06 (l'ordre des shapes de la frame en dépend).
- Notes : valeur par défaut 0 ; 11 profils intégrés utilisent 1 (ex. « Contra », « Final_Fantasy_USA »).

**REQ-E3D-007** (P1) — Le moteur DOIT maintenir pour la session un **registre d'images** : à chaque image rencontrée (dans une trame ou dans un pattern chargé) correspond une identité unique et stable pour la session, de sorte que la même image vue dans deux frames ou dans un pattern soit reconnue comme identique.
- Critère d'acceptation : une image vue à la frame 1 puis à la frame 1000 est rattachée aux mêmes patterns ; la contrainte de temps est celle de REQ-E3D-360.

**REQ-E3D-008** (P2) — Le registre d'images, les patterns appris et les patterns temporaires DOIVENT être conservés quand le même jeu est relancé sans changer de profil, et effacés (avec le registre des patterns, l'atlas et les scripts) quand on change de jeu ou qu'on quitte le mode 2D. Cet état appris (registre d'images, patterns appris et temporaires) fait en outre partie du savestate (DEC-R2-T5, SPEC-07 REQ-LOOP-040) : au chargement d'un savestate, il est remplacé par l'état enregistré.
- Critère d'acceptation : lancer le jeu A, laisser apprendre un pattern (REQ-E3D-160), revenir au menu, relancer A → le pattern appris est encore actif dès la première frame ; lancer B puis A → il a disparu.

### 3.2 Attributs d'une image de tile

**REQ-E3D-020** (P1) — Pour chaque image, le moteur DOIT calculer, dans l'orientation affichée : le **nombre de pixels** non transparents `n` (0–64) ; l'**ensemble des couleurs** utilisées (sous-ensemble de {1, 2, 3}) ; la **boîte de pixels** `(sx, sy)–(ex, ey)` = min et max des coordonnées des pixels non transparents. Pour une image vide, la boîte vaut par convention `(7, 7)–(0, 0)`.
- Critère d'acceptation : image dont seuls les pixels affichés `(2, 3)` et `(5, 6)` sont non nuls → `n = 2`, couleurs {1}, boîte `(2, 3)–(5, 6)`. Image dont le seul pixel non nul est `(1, 2)` sans flip : avec `flipX` le pixel affiché est `(6, 2)` et la boîte `(6, 2)–(6, 2)` (TV-E3D-10).

**REQ-E3D-021** (P3) — Le moteur DEVRAIT calculer la **classe de densité** d'une image : EMPTY = 0 (`n = 0`), LOW = 1 (1–15), AVERAGE = 2 (16–31), MEDIUM = 3 (32–47), HIGH = 4 (48–63), SOLID = 5 (64) (valeurs numériques mémorisées dans les patterns, SPEC-03) ; et sa **forme 2D** : FULL_RECT si `n ≥ 60`, sinon CHAR si l'image n'utilise qu'une couleur et `n < 48`, sinon RECT si `n` égale l'aire de sa boîte de pixels, sinon UNIDENTIFIED.
- Critère d'acceptation : image pleine d'une couleur → SOLID et FULL_RECT ; lettre monochrome de 20 pixels → AVERAGE et CHAR.
- Notes : cette définition est la seule de la spec (SPEC-04B REQ-E3D-454 y renvoie). En mode 3D ces attributs n'ont aucun effet sur le résultat ; la forme CHAR sert uniquement au mode 2D (REQ-E3D-381). Les densités stockées dans les patterns (SPEC-03) ne sont pas recalculées.

### 3.3 Modèle de frame

**REQ-E3D-040** (P1) — Une frame DOIT contenir : la grille de fond, la liste des sprites, la liste **ordonnée** des shapes (toutes les shapes de fond d'abord, puis toutes les shapes de sprites), le nombre de shapes de fond, la liste ordonnée des instances de frame, la sous-liste des instances **scriptables** (REQ-E3D-220), la palette de frame (64 entrées), le masque, le compteur de frame et le temps.
- Critère d'acceptation : l'API de test expose ces éléments ; les shapes d'indice < nombre de shapes de fond sont toutes des shapes de fond.

**REQ-E3D-041** (P1) — Deux tiles de fond sont **voisines** si leurs cellules diffèrent d'une unité sur exactement un axe (4-voisinage). Une cellule sans tile conservée dans la trame courante n'appartient à aucune shape et ne relie rien.
- Critère d'acceptation : deux tiles séparées par une cellule vide (avec `EmptyTile` faux) ne sont jamais dans la même shape de fond.

**REQ-E3D-042** (P1) — Chaque tile conservée DOIT appartenir à au plus une shape. Une tile qui n'appartient à aucune shape à la fin de la segmentation n'est pas affichée.
- Critère d'acceptation : la somme des tiles des shapes est ≤ au nombre de tiles conservées ; voir REQ-E3D-093 pour le cas d'une tile non affichée.

**REQ-E3D-043** (P1) — Le **compteur** d'une frame DOIT valoir le nombre de frames émulées depuis le démarrage du jeu (SPEC-07) et son **temps** valoir `compteur / 60` secondes, quelle que soit la région.
- Critère d'acceptation : 600e frame d'un jeu PAL → `time = 10,0`.
- Notes : les scripts lisent ces valeurs sous les noms `frame` et `time` (SPEC-05).

**REQ-E3D-044** (P1) — Pour chaque shape, le moteur DOIT calculer : la boîte en tiles (§2) ; la boîte en pixels, comme min de `(x + sx, y + sy)` et max de `(x + ex, y + ey)` sur ses tiles (boîtes de pixels d'image, REQ-E3D-020) ; le nombre de tiles ; l'ensemble de ses images distinctes `Img(S)` ; sa palette (REQ-E3D-047) ; son indicateur `hidden` (REQ-E3D-048) ; son **centre** `((pStart + pEnd + (1,1)) / 2)` en coordonnées écran.
- Critère d'acceptation : shape de deux tiles de fond pleines en `(16, 32)` et `(24, 32)` → `pStart = (16, 32)`, `pEnd = (31, 39)`, centre `(24, 36)`.

**REQ-E3D-045** (P2) — L'indicateur **Inside** d'une shape DOIT être vrai si et seulement si `pStart.x > mX` et `pEnd.x + mX < 255` et `pStart.y > mY` et `pEnd.y + mY < 239`. L'indicateur **AtBorder** est sa négation. Une shape est **grande** si `tSize.x ≥ 28` ou `tSize.y ≥ 26`. Une shape est **terminée** si elle est Inside ou grande.
- Critère d'acceptation : masque `(8, 0)` ; shape avec `pStart.x = 8` → AtBorder ; avec `pStart.x = 9`, `pEnd.x = 200`, `pStart.y = 1`, `pEnd.y = 230` → Inside.

**REQ-E3D-046** (P2) — L'indicateur **Completed** d'une shape DOIT être vrai si et seulement si `|Img(S)| = |Img(P)|` où P est son pattern de référence.
- Critère d'acceptation : TV-E3D-05, shape `{A, B}` liée à `[A B C]` → Completed faux.

**REQ-E3D-047** (P1) — La **palette d'une shape** DOIT valoir l'attribut `pal` de la première tile ajoutée (la tile germe), augmenté de 4 pour une shape de sprites (valeurs 0–3 pour le fond, 4–7 pour les sprites).
- Critère d'acceptation : shape de sprites dont le germe a `pal = 2` → palette 6.
- Notes : chaque tile garde son propre `pal` pour la coloration (REQ-E3D-253) ; la palette de shape sert aux scripts et aux requêtes.

**REQ-E3D-048** (P2) — L'indicateur `hidden` d'une shape de sprites DOIT être le OU des indicateurs `hidden` de ses tiles ; celui d'une shape de fond est toujours faux.
- Critère d'acceptation : shape de 4 sprites dont un seul est `hidden` → shape `hidden`.

### 3.4 Registre des patterns, clés et égalité

**REQ-E3D-060** (P1) — Au chargement d'un profil, chaque pattern permanent et temporaire (SPEC-03) DOIT être inscrit au registre, dans l'ordre : tous les permanents dans l'ordre du fichier, puis tous les temporaires dans l'ordre du fichier. Pour chaque pattern, le moteur calcule `Img(P)`, la boîte de pixels du pattern (REQ-E3D-064) et ses relations d'adjacence (REQ-E3D-061).
- Critère d'acceptation : le nombre de patterns inscrits égale le nombre d'entrées du profil ; pour chacun des 101 profils de `assets-neutral/profiles`, le nombre de patterns inscrits est égal au nombre de patterns permanents plus temporaires du fichier (aucun profil intégré ne contient deux patterns permanents de même `Img`).

**REQ-E3D-061** (P1) — Pour un pattern P de grille `W × H` (cellule `(i, j)`, `0 ≤ i < W`, `0 ≤ j < H`), le moteur DOIT définir deux relations sur les images :
- **adjacence horizontale** `R(P)` : `(X, Y) ∈ R(P)` s'il existe une cellule `(i, j)` d'image X et la cellule `(i', j)` d'image Y, avec `i' = i + 1` ; si `selfConnected` est vrai, aussi `i' = (i + 1) mod W` (la dernière colonne est suivie de la première) ;
- **adjacence verticale** `D(P)` : idem avec `(i, j)` et `(i, j')`, `j' = j + 1` (ou `(j + 1) mod H` si `selfConnected`).
Les cellules vides ne produisent aucune paire. Les relations portent sur les **images**, pas sur les positions : une paire vaut partout où elle apparaît.
- Critère d'acceptation : P = `[A B]` (1 ligne) : `selfConnected` faux → `R = {(A,B)}`, `D = ∅` ; vrai → `R = {(A,B), (B,A)}`, `D = {(A,A), (B,B)}`. P = 1×1 `[A]`, `selfConnected` vrai → `R = D = {(A,A)}`.
- Notes : `selfConnected` vaut vrai si le champ est absent du profil (SPEC-03). 6 352 patterns intégrés sur 66 609 le mettent à faux (ex. « Felix the Cat », « punchout »).

**REQ-E3D-062** (P1) — Le moteur DOIT dire qu'un pattern P **autorise** l'ajout d'une tile N voisine d'une tile T (déjà dans la shape), dans la direction d (position de N par rapport à T), si et seulement si :
1. soit `skipGeo` est faux et : d = droite et `(img T, img N) ∈ R(P)` ; d = gauche et `(img N, img T) ∈ R(P)` ; d = bas et `(img T, img N) ∈ D(P)` ; d = haut et `(img N, img T) ∈ D(P)` ;
2. soit `skipGeo` est vrai et `img N ∈ Img(P)` (direction indifférente) ;
3. **et** la boîte en tiles de la shape augmentée de N a une largeur ≤ `cW` **et** une hauteur ≤ `cH`.
- Critère d'acceptation : TV-E3D-02, 03, 04.
- Notes : la taille de contrainte vaut `(99, 99)` (sans contrainte) par défaut ; 668 patterns intégrés en ont une autre (ex. « Ninja Gaiden III », « Jackal (U) », « Micro Mages »). 2 310 patterns ont `skipGeo` vrai (« Life Force (U) » : 1 171).

**REQ-E3D-063** (P1) — Deux patterns DOIVENT être considérés comme **égaux** (même clé dans le registre) si et seulement si leurs ensembles d'images distinctes sont égaux. La disposition, la taille de grille, le nombre d'occurrences et les paramètres sont ignorés. La recherche « pattern permanent de clé K » ou « pattern temporaire de clé K » DOIT être en temps constant amorti.
- Critère d'acceptation : patterns `[A B]` et `[B; A; B]` (3 lignes) → égaux.

**REQ-E3D-064** (P2) — La **boîte de pixels d'un pattern** DOIT être calculée dans le repère de sa grille (la cellule `(i, j)` couvre les pixels `8i…8i+7`, `8j…8j+7`) comme min/max des boîtes de pixels de ses images non vides. On en déduit `BottomLeft(P) = (pStart.x, 8H − pEnd.y − 1)` et `TopRight(P) = (pEnd.x, 8W − pStart.y − 1)`.
- Critère d'acceptation : pattern 1×1 d'une image pleine → `pStart = (0,0)`, `pEnd = (7,7)`, `BottomLeft = (0, 0)`.
- Notes : la formule de `TopRight` utilise bien `W` (et non `H`) pour l'axe y ; c'est le comportement observé par les scripts, conservé tel quel (voir Q-04).

**REQ-E3D-065** (P1) — Quand un pattern **permanent** de clé K est ajouté au registre (chargement ou apprentissage), le moteur DOIT : retirer le pattern permanent de même clé s'il existe ; retirer tout pattern temporaire T tel que `Img(T) ⊆ Img(K)`. Quand un pattern **temporaire** de clé K est ajouté, le moteur DOIT retirer tout pattern temporaire T tel que `Img(K) ⊆ Img(T)`.
- Critère d'acceptation : registre contenant le temporaire `{A}` ; apprentissage du permanent `{A, B}` → le temporaire `{A}` disparaît.

### 3.5 Segmentation du fond

**REQ-E3D-090** (P1) — **Croissance contrainte.** À partir d'une tile germe G et d'un ensemble initial de **candidats** C (patterns), le moteur DOIT construire une shape ainsi :
1. la shape contient G ; une file FIFO contient G ;
2. tant que la file n'est pas vide, on retire sa tête T et on examine ses voisines dans l'ordre **droite, bas, haut, gauche** ;
3. une voisine N n'est examinée que si elle existe dans la trame et n'appartient encore à aucune shape (y compris la shape en cours) ; si le réglage `SameBgPalette` est vrai, il faut aussi `pal(N) = pal(T)` ;
4. soit C' ⊆ C l'ensemble des candidats qui autorisent N depuis T (REQ-E3D-062), dans l'ordre de C ; si C' est non vide, N est ajoutée à la shape et à la file, et C devient C' ; sinon N reste libre (elle pourra être réexaminée depuis une autre tile avec le C alors courant).
À la fin, le **pattern choisi** est le candidat de C ayant le plus petit `|Img(P)|` ; à égalité, le premier dans l'ordre de C.
- Critère d'acceptation : TV-E3D-01 à 05.
- Notes : le rétrécissement de C est glouton et dépend de l'ordre d'exploration : c'est voulu et nécessaire à la compatibilité des profils. Une shape reconnue peut ne contenir qu'une partie des images de son pattern (REQ-E3D-046).

**REQ-E3D-091** (P1) — L'**ordre initial des candidats** d'une image DOIT être l'ordre d'inscription au registre des patterns permanents qui la contiennent (REQ-E3D-060, puis patterns appris dans l'ordre de création).
- Critère d'acceptation : deux patterns `[A B]` (inscrit 1er) et `[A C]` (2e), même nombre d'images ; frame « A » seule → liée à `[A B]`.
- Notes : voir Q-02 sur la stabilité de cet ordre après suppression de patterns.

**REQ-E3D-092** (P1) — La segmentation du fond DOIT se faire en **trois passes successives** sur les tiles de fond, chacune dans l'ordre de germination (REQ-E3D-006) :
- **Passe 1 (grandes régions connues)** : pour chaque tile encore libre dont l'image est connue, croissance contrainte (REQ-E3D-090) avec pour candidats les patterns permanents non désactivés contenant son image. La shape obtenue est **retenue** si (elle est **pleine** (REQ-E3D-094) et compte au moins 8 tiles et au moins 2 images distinctes) ou si elle est grande (REQ-E3D-045) ; sinon elle est **rejetée**. Les tiles d'une shape rejetée restent indisponibles jusqu'à la fin de la passe 1, puis sont libérées.
- **Passe 2 (tiles inconnues)** : pour chaque tile encore libre dont l'image est **inconnue au moment où elle est visitée**, création d'une shape d'une seule tile, liée selon REQ-E3D-160.
- **Passe 3 (reste connu)** : pour chaque tile encore libre dont l'image est connue, croissance contrainte comme en passe 1, toujours retenue.
Les shapes sont ajoutées à la frame dans l'ordre où elles sont retenues.
- Critère d'acceptation : TV-E3D-06 (ordre des shapes : passe 1, puis 2, puis 3).
- Notes : « inconnue au moment de la visite » : si une tile inconnue de la passe 2 a fait apprendre un pattern permanent pour son image (REQ-E3D-160), les tiles suivantes de même image sont connues et sont traitées en passe 3 (TV-E3D-07).

**REQ-E3D-093** (P1) — Une tile dont l'image est connue mais dont **tous** les patterns permanents sont désactivés NE DOIT être liée à aucune shape dans cette frame (elle n'est pas affichée), sauf si elle a été intégrée à la shape d'un autre germe.
- Critère d'acceptation : pattern `[A]` seul pattern de A, `Disable` mis à vrai par un script → les tiles A isolées disparaissent à la frame suivante ; remis à faux → elles réapparaissent.
- Notes : utilisé par « Jackal (U) » pour activer des variantes de décor selon la scène.

**REQ-E3D-094** (P1) — Une shape est **pleine** si aucune cellule strictement intérieure à sa boîte en tiles (ni sur la première/dernière ligne, ni sur la première/dernière colonne) et n'appartenant pas à la shape n'est **encerclée**, c'est-à-dire n'a, dans la boîte, une tile de la shape à la fois quelque part à sa gauche sur sa ligne, à sa droite sur sa ligne, au-dessus sur sa colonne et au-dessous sur sa colonne.
- Critère d'acceptation : anneau 3×3 de 8 tiles (centre absent) → non pleine ; forme en U → pleine ; ligne simple → pleine.

**REQ-E3D-095** (P1) — En passe 3 (et en passe 1 pour une shape retenue) la shape DOIT être liée au pattern choisi (REQ-E3D-090) et ajoutée à la frame comme scriptable.
- Critère d'acceptation : TV-E3D-01.

### 3.6 Segmentation des sprites

**REQ-E3D-130** (P1) — Deux sprites T et N sont **adjacents avec tolérance t** (entier ≥ 0, pixels) dans la direction d déterminée ainsi, en testant dans l'ordre et en retenant la première qui convient : droite si `|y_N − y_T| ≤ t` et `|x_T + 8 − x_N| ≤ t` ; gauche si `|y_N − y_T| ≤ t` et `|x_N + 8 − x_T| ≤ t` ; bas si `|x_N − x_T| ≤ t` et `|y_T + 8 − y_N| ≤ t` ; haut si `|x_N − x_T| ≤ t` et `|y_N + 8 − y_T| ≤ t`. Sinon ils ne sont pas adjacents.
- Critère d'acceptation : T `(100, 50)`, N `(108, 52)` : t = 0 → non adjacents ; t = 2 → N à droite de T.

**REQ-E3D-131** (P1) — La segmentation des sprites DOIT se faire en deux passes, après celle du fond, chacune dans l'ordre d'émission :
- **Passe S1 (groupes inconnus)** : pour chaque sprite libre dont l'image est **inconnue au moment de la visite**, création d'une shape par croissance libre : en partant du germe (file FIFO), pour chaque sprite T retiré de la file, chaque sprite N encore libre, pris dans l'ordre d'émission, est ajouté (et mis en file) si `pal(N) = pal(T)`, si N est adjacent à T avec tolérance 0, et si la cellule de grille de sprites de N n'est pas déjà occupée par un sprite appartenant à une shape de cette frame. Les sprites connus voisins sont donc absorbés eux aussi. La shape est ensuite liée selon REQ-E3D-160.
- **Passe S2 (sprites connus)** : pour chaque sprite encore libre, croissance contrainte au sens de REQ-E3D-090 avec les différences suivantes : les voisins sont **tous** les sprites libres pris dans l'ordre d'émission, l'adjacence et sa direction sont celles de REQ-E3D-130 avec `t = SpriteTolerance` ; la condition de palette s'applique si `SameSpPalette` est vrai ; la boîte en tiles utilise la grille de sprites ; les candidats initiaux sont les patterns permanents non désactivés contenant l'image du germe. Si ce germe n'a aucun candidat, il reste libre (non affiché). La shape est toujours retenue, liée au pattern choisi et ajoutée.
- Critère d'acceptation : TV-E3D-08.
- Notes : valeurs par défaut `SpriteTolerance = 0`, `SameSpPalette = faux`. Profils intégrés : tolérance 0 à 7 (ex. 7 pour « Chip 'n Dale », 4 pour « Contra ») ; `SameSpPalette` vrai pour « DQ3_DW3 » et « Jackal (U) ».

**REQ-E3D-132** (P1) — Quand un sprite rejoint une shape, sa cellule de grille de sprites DOIT être marquée occupée par lui pour le reste de la frame (dernier arrivé gagnant en passe S2, où l'occupation n'est pas testée).
- Critère d'acceptation : deux sprites inconnus de même palette dans la même cellule et un troisième adjacent : la passe S1 ne peut pas rattacher le second au groupe du premier par la cellule commune.

### 3.7 Tiles inconnues : apprentissage et patterns temporaires

**REQ-E3D-160** (P1) — Une shape créée en passe 2 (fond) ou S1 (sprites) DOIT être liée ainsi :
1. si la shape est **terminée** (REQ-E3D-045) : s'il existe un pattern permanent de même clé (`Img(S)`), elle lui est liée ; sinon un **pattern permanent est appris** (REQ-E3D-161) et elle lui est liée ;
2. sinon (shape en bord d'écran et non grande) : s'il existe un pattern temporaire de même clé, elle lui est liée ; sinon, s'il existe un pattern temporaire T avec `Img(S) ⊆ Img(T)`, elle est liée au premier tel T dans l'ordre du registre temporaire ; sinon un **pattern temporaire est appris** et elle lui est liée.
- Critère d'acceptation : TV-E3D-07.
- Notes : les patterns temporaires ne sont jamais candidats à la croissance contrainte ; ils évitent seulement qu'un objet partiellement visible en bord d'écran devienne un pattern permanent. Les profils intégrés contiennent 1 223 patterns temporaires.

**REQ-E3D-161** (P1) — Un pattern **appris** à partir d'une shape S DOIT avoir : une grille de taille `tSize(S)` dont la cellule `(i, j)` contient l'image de la tile de S située en `tStart + (i, j)` (vide s'il n'y en a pas) ; `selfConnected` vrai, `skipGeo` faux, contrainte `(99, 99)` ; une seule instance, avec les paramètres 3D par défaut du tableau 4.3 (géométrie « Default », profondeur 3, couche L1). Il est inscrit au registre (REQ-E3D-065), ses relations sont calculées, et sa géométrie est construite (REQ-E3D-322, SPEC-04B).
- Critère d'acceptation : sprite inconnu isolé au milieu de l'écran → un pattern permanent 1×1 apparaît dans le registre, profondeur 3, couche L1 ; à la frame suivante, la même image est connue.
- Notes : en mode bibliothèque, les patterns appris ne sont pas écrits dans le profil (aucune sauvegarde de profil dans le périmètre).

### 3.8 Liaison, instances de frame et propriétés

**REQ-E3D-190** (P1) — Lier une shape à un pattern P DOIT créer, pour chaque instance k de P, une instance de frame dont **tous** les paramètres 3D sont copiés de l'instance k de P (géométrie, profondeur, couche, offset, échelle, rotation, pivot, amplitude et matrices de déformation, alpha, mode de rendu, style de rendu, texture, couleur vide, indicateurs de tags…), avec la palette personnalisée remise à « aucune » (−1). Les tags et l'association aux scripts sont **partagés** avec l'instance de P.
- Critère d'acceptation : TV-E3D-13 : un script `UpdateS` qui ajoute 5 à `shape.Offset.x` à chaque frame donne un offset de 5 (et non 5, 10, 15…) dans chaque frame.
- Notes : une modification d'instance de frame ne vaut que pour la frame courante ; pour un effet durable, un script modifie le pattern (via le registre, SPEC-05).

**REQ-E3D-191** (P2) — Une instance de frame dont le **style de rendu** vaut 0 (`Enable` faux) DOIT être conservée dans la frame mais non dessinée.
- Critère d'acceptation : script qui met `shape.Enable = false` → l'objet disparaît pour cette frame et reste interrogeable par les scripts.

**REQ-E3D-192** (P2) — La **palette d'une instance** (`Palette`) DOIT valoir sa palette personnalisée si elle est ≥ 0, sinon la palette de sa shape.
- Critère d'acceptation : `shape.Palette = 9` → les tiles sont colorées avec la ligne 9 de la palette de frame (REQ-E3D-253).

**REQ-E3D-205** (P1) — Après les hooks Lua, le moteur DOIT calculer pour chaque instance de frame sa **position** `Pos` et son **centre** `Center` :
- `Bmin = (pStart.x, Hs − pEnd.y − 1, z0)`, `Bmax = (pEnd.x + 1, Hs − pStart.y, z1)` (notation de SPEC-04B REQ-E3D-404 ; à ne pas confondre avec les grandeurs `BL` et `TR` exposées aux scripts, §4.4, dont la profondeur vaut `∓s/2`) ;
- `z0 = O + (s/2)(a − 1)`, `z1 = O + (s/2)(a + 1)`, où s est la profondeur de l'instance, O le décalage z et a le coefficient d'alignement de sa **couche effective** (FRONT = 1, CENTER = 0, BACK = −1, réglages `Layer` de SPEC-09) ;
- la couche effective est celle de l'instance, ou si elle vaut UNIDENTIFIED : L1 pour une shape de sprites, la couche par défaut du réglage `Layer.DefLayer` pour une shape de fond ;
- `Pos = Bmin ⊙ (1 − pivot) + Bmax ⊙ pivot + offset` (produit composante par composante), `Center` = même formule avec pivot `(0,5 ; 0,5 ; 0,5)`.
- Critère d'acceptation : shape d'une tile pleine en `(16, 32)`, profondeur 3, couche L1 (décalage 0, FRONT), pivot `(0,5 ; 0,5 ; 0,5)`, offset nul → `Bmin = (16, 208, 0)`, `Bmax = (24, 216, 3)`, `Pos = (20, 212, 1,5)` ; les grandeurs exposées valent alors `BL = (16, 208, −1,5)` et `TR = (23, 215, 1,5)` (REQ-E3D-206, TV-LUA-05).
- Notes : valeurs par défaut des couches : UI décalage 0 CENTER ; L1 0, L2 12, L3 24, L4 36, toutes FRONT ; couche par défaut L1. Le placement des meshes autour de `Pos` relève de SPEC-04B/SPEC-06.

**REQ-E3D-206** (P2) — Le moteur DOIT exposer pour chaque shape et instance les grandeurs dérivées du tableau 4.4, calculées exactement par ces formules (valeurs lues par les scripts).
- Critère d'acceptation : pour la shape du critère de REQ-E3D-205 : shape `BottomLeft = (16, 208)`, `TopRight = (23, 215)` ; instance `BottomLeft = (16, 209, −1,5)`, `BL = (16, 208, −1,5)`, `size = (8, 8, 3)`.

**REQ-E3D-207** (P3) — L'**âge** d'une shape (`Age`) DOIT valoir 1.
- Critère d'acceptation : toute shape renvoie `Age = 1`.
- Notes : `Age` vaut toujours 1 ; aucun profil embarqué ne lit cette valeur. Conservé pour compatibilité.

### 3.9 Ordre des shapes et instances scriptables

**REQ-E3D-220** (P1) — Les instances **scriptables** d'une frame DOIVENT être exactement les instances des shapes produites par la segmentation (3.5, 3.6), dans l'ordre des shapes, puis par indice d'instance croissant. Les shapes ajoutées par script (3.10) ou restaurées d'une frame épinglée NE DOIVENT PAS être scriptables : elles sont dessinées mais n'apparaissent ni dans les requêtes par tag ou par indice ni dans les appels `UpdateS`.
- Critère d'acceptation : `Frame:GetShape(0)` renvoie la première instance de la première shape de fond retenue ; une shape créée par `Frame:CreateNewShape` n'est pas renvoyée par `Frame:GetShapesWithTag("*")`.
- Notes : l'ordre détermine le résultat des requêtes « premier élément » des scripts (SPEC-05). Aucun tri supplémentaire (par position ou par aire) n'est appliqué.

### 3.10 Shapes injectées par script et frame épinglée

**REQ-E3D-230** (P2) — `Frame:CreateNewShape(P, pos, pal)` (P = pattern ou instance de pattern, `pos` = vecteur scène, `pal` = palette 4–7) DOIT ajouter à la frame en cours une shape de sprites non scriptable, liée à P, dont la tile issue de la cellule `(i, j)` de P (cellules non vides seulement) est placée à `x = 8i + ⌊pos.x⌉₀`, `y = 8j + (Hs − (⌊pos.y⌉₀ + 8H) + 1)`, avec `pal − 4` comme attribut de palette, non `hidden` (`⌊·⌉₀` : troncature vers zéro) ; les offsets de toutes ses instances reçoivent en plus `(pos.x − ⌊pos.x⌉₀, −(pos.y − ⌊pos.y⌉₀), pos.z)`. Si P est une instance d'indice k, seule l'instance k est activée et c'est elle qui est renvoyée ; sinon la shape est renvoyée.
- Critère d'acceptation : TV-E3D-14.

**REQ-E3D-231** (P2) — `Frame:AddNewShape(P, pos, pal)` DOIT faire de même avec les tiles placées à `x = 8i`, `y = 8j`, et ajouter à l'offset de chaque instance `(pos.x, pos.y − Hs + 8H, pos.z)`.
- Critère d'acceptation : pattern 1×1 plein, `pos = (40, 100, 0)` → `Pos` (pivot centré, profondeur 3, couche L1) = `(44, 104, 1,5)`.

**REQ-E3D-232** (P2) — `Frame:InsertShape(P, pos, pal)` DOIT équivaloir à `CreateNewShape(P, pos − BottomLeft(P), pal)` (REQ-E3D-064).
- Critère d'acceptation : pattern 1×1 plein : `InsertShape` et `CreateNewShape` donnent la même tile pour la même `pos`.

**REQ-E3D-233** (P2) [ROM] — **Frame épinglée.** `Frame:PinNewFrame()` DOIT figer la dernière frame publiée (REQ-E3D-280) jusqu'à `Frame:ReleasePinnedFrame()`. `Frame:AddRectFromPinnedFrame(x, y, w, h)` ajoute un rectangle (coordonnées scène) à la liste de la frame en cours ; `Frame:RestoreFromPinnedFrame()` ajoute à la frame en cours, comme shapes non scriptables, chaque shape de la frame épinglée dont la boîte `(pStart, pEnd − pStart + 1)` chevauche l'un de ces rectangles (sans doublon) ; `RestoreFromPinnedFrame(rect)` ajoute le rectangle puis les shapes qui le chevauchent. Sans frame épinglée, ces appels n'ont aucun effet. Au dessin, une tile d'une shape restaurée n'est dessinée que si sa case 8×8 (coordonnées scène) chevauche un des rectangles.
- Critère d'acceptation : en harnais, deux trames synthétiques : épingler la première, puis dans la seconde `RestoreFromPinnedFrame(rect)` avec un rectangle chevauchant une shape S de la première → S est ajoutée à la seconde frame comme shape non scriptable et seules ses tiles dont la case 8 × 8 chevauche le rectangle sont dessinées. [ROM] « DQ3_DW3 » : à l'ouverture d'une boîte de dialogue, le décor masqué par la boîte reste visible derrière elle tant que la boîte est ouverte.
- Notes : le test de chevauchement compare des rectangles mixtes (pixels écran pour les shapes, scène pour les rectangles) ; ce comportement est conservé tel quel (voir Q-05).

### 3.11 Palettes

**REQ-E3D-250** (P1) — La **palette de frame** DOIT avoir 64 entrées (16 lignes × 4 colonnes) d'indices de couleur NES (0–63), construites à partir des 32 octets `p` fournis par le cœur : entrée `e` = `p[0]` si `e mod 4 = 0`, sinon `p[e mod 32]`. Les lignes 0–3 servent au fond, 4–7 aux sprites ; les lignes 8–15 sont des copies des lignes 0–7, utilisables comme palettes personnalisées.
- Critère d'acceptation : TV-E3D-09.

**REQ-E3D-251** (P1) — La conversion d'un indice NES en couleur RVB DOIT utiliser la table fixe 4.1 (opaque).
- Critère d'acceptation : indice `$16` → `#FC3800` ; `$0F` → `#000000`.

**REQ-E3D-252** (P2) [ROM] — Les scripts DOIVENT pouvoir lire et écrire une entrée de palette de frame : `Palette.FrameColorIndex(e)` / `Palette.GetColorIndex(e)` lisent la frame **publiée** (celle en cours d'affichage) ; avec un second argument non nul, la frame **en cours de conversion**. `Palette.SetColorIndex(e, v, sel)` écrit selon la même règle. `Palette.FrameColor(e)` renvoie la couleur RVB de la frame publiée. Une écriture ne vaut que pour la frame visée (la palette est reconstruite à chaque trame).
- Critère d'acceptation : en harnais, `Palette.SetColorIndex(5, 0x30, 1)` dans `Update` → l'entrée 5 de la frame convertie vaut 0x30 pour cette frame et retrouve la valeur du cœur à la frame suivante ; sans second argument, l'écriture vise la frame publiée. [ROM] « punchout » : un script écrit `SetColorIndex(l*4+1, v, 1)` → la ligne l de la frame courante est recolorée dans cette frame uniquement.
- Notes : dans une implémentation sans file de rendu séparée, « frame publiée » = la frame précédemment convertie (décalage d'une frame à conserver pour la compatibilité des scripts).

**REQ-E3D-253** (P1) — Chaque tile d'une shape DOIT être colorée, pour une texture au format **indexé**, avec la ligne de palette : palette personnalisée de l'instance si ≥ 0, sinon `pal` de la tile (+ 4 pour un sprite) ; la valeur de pixel 0–3 donne la colonne. Pour une texture au format **direct**, les couleurs de la texture sont utilisées sans palette.
- Critère d'acceptation : shape de fond de deux tiles de `pal` 1 et 2 (avec `SameBgPalette` faux) → chaque tile garde ses propres couleurs.
- Notes : les formats de texture (indexé = 0, direct = 1) sont définis par SPEC-03/SPEC-04B.

**REQ-E3D-254** (P2) — La couleur de fond de scène DOIT être la couleur de l'entrée 0 de la palette de frame publiée lorsque le réglage `BgColor` est vrai et qu'aucun skybox n'est actif, sinon la couleur personnalisée du réglage (SPEC-06/SPEC-09).
- Critère d'acceptation : `p[0] = $21`, `BgColor` vrai, pas de skybox → fond `#38C0FC`.

### 3.12 Gestion des frames

**REQ-E3D-280** (P1) — Le rendu DOIT toujours afficher la frame **publiée la plus récente** ; la conversion d'une nouvelle frame ne DOIT jamais modifier une frame en cours d'affichage ni la frame épinglée. Une frame publiée qui n'a pas été affichée avant la publication suivante PEUT être sautée.
- Critère d'acceptation : en avance rapide, l'affichage montre des frames de compteur croissant, jamais une frame partiellement construite.
- Notes : l'implémentation est libre tant que la règle est respectée et que le résultat reste déterministe (SPEC-07).

**REQ-E3D-281** (P2) — Au démarrage d'un jeu, après un reset ou un chargement d'état (SPEC-07), toutes les frames DOIVENT être vidées (aucune shape, compteur 0) et la frame épinglée relâchée.
- Critère d'acceptation : après chargement d'un savestate, aucune shape de l'ancienne session n'est dessinée.

### 3.13 Tracker

**REQ-E3D-300** (P2) — Le moteur DOIT fournir un **tracker** qui, démarré sur une shape S0, renvoie à chaque frame la shape qui la continue, selon les règles suivantes. État : position `q` (centre de S0, REQ-E3D-044), couche (sprites si S0 est une shape de sprites, sinon fond), ensemble **inclus** I (initialement le pattern de référence de S0), ensemble **exclu** E (initialement vide). La frame range ses shapes dans une liste ordonnée unique : toutes les shapes de fond, puis toutes les shapes de sprites (ordre de REQ-E3D-220). À chaque frame, en parcourant les seules shapes de la couche suivie, dans l'ordre de cette liste, avec un candidat c (aucun au départ) et un seuil `δ = 9` pixels :
1. si le pattern de la shape s est dans I : E reçoit le pattern (s'il n'est pas dans I) de **chaque shape qui précède s dans la liste ordonnée, ou qui est s** — pour un suivi de sprites, cela comprend donc toutes les shapes de fond de la frame —, ainsi que le pattern de c (s'il existe) ; s est retenue ; fin du parcours ;
2. sinon si son pattern est dans E : on passe ;
3. sinon si la distance euclidienne entre q et son centre est < δ : le pattern de c (s'il existe) est ajouté à E, c := s, δ := cette distance ;
4. sinon son pattern est ajouté à E.
À la fin, si une shape est retenue (cas 1) ou si c existe, q devient son centre ; si elle a été trouvée par proximité (cas 3), son pattern est ajouté à I. Le tracker renvoie cette shape, ou rien (q inchangé).
- Critère d'acceptation : TV-E3D-12.
- Notes : utilisé par la caméra subjective qui suit un personnage (SPEC-06). Les ensembles ne sont vidés qu'au redémarrage du tracker. Résultat après une frame : E contient, en plus de son contenu antérieur, les patterns écartés par les cas 1, 3 et 4 rencontrés avant l'arrêt du parcours ; la règle « toutes les shapes qui précèdent s » est celle de l'original.

### 3.14 Hooks Lua dans le pipeline

**REQ-E3D-320** (P1) — Après la segmentation d'une frame (REQ-E3D-001 d), le moteur DOIT, dans cet ordre : mettre à jour les globales `frame` et `time` ; appeler `Update` de chaque script actif (ordre des scripts : SPEC-05) ; constituer pour chaque script possédant `UpdateS` la liste des instances scriptables qui lui sont **liées** (REQ-E3D-321), dans l'ordre de REQ-E3D-220 ; appeler `UpdateS` de chaque script ayant au moins une instance liée, une fois par instance, avec les globales `shape` (instance) et `shape2D` (sa shape) ; appeler `LateUpdate` de chaque script. Les shapes créées par les scripts pendant ces appels sont ajoutées à la frame en cours et dessinées avec elle.
- Critère d'acceptation : un script qui crée une shape dans `Update` la voit dessinée dans la même frame ; un script qui met `Disable` à vrai sur un pattern dans `Update` n'affecte la segmentation qu'à partir de la frame suivante.

**REQ-E3D-321** (P1) — Une instance DOIT être liée à un script si la chaîne de tags du script contient le caractère `*`, ou si l'un des tags du script est égal à l'un des tags de l'instance (tags : éléments non vides de la chaîne séparée par des virgules, espaces de bord retirés, comparaison exacte sensible à la casse).
- Critère d'acceptation : script de tags `"enemy, boss"` ; instance de tags `"boss"` → liée ; `"Boss"` → non liée ; script `"*"` → liée à toutes.

**REQ-E3D-322** (P1) [ROM] — Le hook `BuildPattern` DOIT être appelé pour chaque instance de pattern liée au script, avec la globale `pattern` = cette instance, **avant** la construction de sa géométrie (SPEC-04B), à chaque construction : au chargement du profil (pour tous les patterns, avant l'appel de `Start`), à l'apprentissage d'un pattern (REQ-E3D-161), et lors de la reconstruction de tous les patterns après un changement d'algorithme de rendu (REQ-E3D-340). Les modifications faites par le hook sont permanentes pour le pattern.
- Critère d'acceptation : en harnais, sans ROM : au chargement d'un profil dont un script `BuildPattern` fixe `pattern.sizeZ = 8` (par exemple « empty » avec sa règle `*`), le hook est appelé une fois par instance liée avant `Start`, et la géométrie construite a la profondeur 8. [ROM] « Mega Man 2 » : le hook met `pattern.texTilting.y = 1` → la valeur est en place dès la première frame affichée.

**REQ-E3D-323** (P1) — Le hook `DetectShape`, s'il est défini par un script, NE DOIT PAS être appelé par le pipeline de conversion : sa présence n'a aucun effet sur la segmentation ni sur la liaison.
- Critère d'acceptation : le profil « Super Bat Puncher Demo » (seul profil intégré à le définir) produit les mêmes shapes que si la fonction était absente.
- Notes : l'original reconnaît ce hook mais ne l'invoque jamais ; l'invoquer changerait le rendu de ce profil. Voir Q-01.

**REQ-E3D-324** (P2) — Une erreur Lua levée par un hook appelé dans ce pipeline NE DOIT PAS interrompre la conversion : la frame est terminée et publiée, et le script fautif est traité selon SPEC-05 (désactivé jusqu'à rechargement).
- Critère d'acceptation : script `UpdateS` qui lève une erreur → la frame suivante s'affiche normalement, sans cet effet.

### 3.15 Algorithmes de rendu, densité, sélection, couches automatiques

**REQ-E3D-340** (P1) — Le moteur DOIT gérer l'**algorithme de rendu** (énumération : GREEDY = 0, MARCHING = 1, GREEDY_MULTICOLOR = 2) : un réglage global (défaut GREEDY, réglage utilisateur, jamais pris du profil) et une valeur par instance `rm` (0 = utiliser le réglage global, 1 = GREEDY, 2 = MARCHING, 3 = GREEDY_MULTICOLOR). Un changement du réglage global DOIT entraîner la reconstruction de la géométrie de tous les patterns du registre (avec `BuildPattern`, REQ-E3D-322) et la progression de chargement (REQ-E3D-362).
- Critère d'acceptation : basculer le réglage → indicateur Chargement (SPEC-07 REQ-LOOP-024), puis tous les objets reconstruits ; une instance avec `rm = 2` reste MARCHING quel que soit le réglage.
- Notes : la signification géométrique de chaque algorithme est fixée par SPEC-04B ; le mode MARCHING applique aussi une correction d'échelle au dessin (SPEC-04B/SPEC-06).

**REQ-E3D-341** (P3) — Les classes de densité des patterns stockées dans le profil (SPEC-03) DOIVENT être conservées et exposées en lecture, sans effet sur le pipeline.
- Critère d'acceptation : relire un pattern → mêmes valeurs qu'au chargement.

**REQ-E3D-342** (P3) — Le réglage de **couches automatiques** (`Layer.Auto`, défaut faux) NE DOIT avoir aucun effet sur la couche des shapes.
- Critère d'acceptation : activer le réglage → aucune couche ne change.
- Notes : le réglage est conservé pour la compatibilité des données ; il n'a aucun effet.

**REQ-E3D-345** (P3) — *Informative* : les modes de sélection de l'éditeur (shape ou tile) et la mise en évidence de sélection sont hors périmètre ; ils n'ont aucun effet sur la conversion. (Note sans critère propre ; pas d'exigence à vérifier.)

### 3.16 Performance

**REQ-E3D-360** (P1) — La conversion d'une frame (étapes a à c et e de REQ-E3D-001, hors temps des scripts) DEVRAIT prendre en moyenne au plus 4 ms, et au plus 8 ms au 99ᵉ centile, sur la machine de référence (SPEC-12, DEC-R2-12-2), pour une trame pleine (33 × 31 tiles de fond, 64 sprites) et un profil de 4 000 patterns permanents ; elle DOIT tenir dans la période d'une frame émulée (16,6 ms).
- Critère d'acceptation : banc headless sur le profil « Final_Fantasy_USA » (3 884 patterns) et une trame pleine synthétique, 1 000 conversions : moyenne ≤ 4 ms et 99ᵉ centile ≤ 8 ms.
- Notes : pour tenir ce budget, obtenir les candidats d'une image (REQ-E3D-007, 063) et tester une adjacence DEVRAIENT se faire en temps constant amorti.

**REQ-E3D-361** (P1) — La construction de la géométrie des patterns DOIT être asynchrone par rapport à la conversion : une shape dont le pattern n'a pas encore de géométrie prête est conservée dans la frame (scriptable) mais n'est pas dessinée ; le transfert des géométries prêtes vers le rendu DEVRAIT être limité, à chaque frame de rendu, à un budget de temps égal à la durée d'une frame de rendu cible (même règle que SPEC-04B REQ-E3D-600 et SPEC-07 REQ-LOOP-061).
- Critère d'acceptation : au chargement d'un gros profil, le jeu tourne ; les objets apparaissent au fil de la construction, sans gel de plus de 100 ms.

**REQ-E3D-362** (P2) — La **progression de chargement** (0 à 1) DOIT valoir `(construites + transférées) / (2 × à_construire)` pendant le chargement d'un profil ou une reconstruction, et 1 hors chargement ou si aucun pattern n'est à construire.
- Critère d'acceptation : profil de 100 patterns : 50 construits, 0 transféré → 0,25 ; tout transféré → 1.
- Notes : consommée par SPEC-07 (attente des 100 %) et SPEC-10 (barre de progression).

### 3.17 Mode 2D

**REQ-E3D-380** (P3) — En mode 2D (entrée « 2D Mode »), le profil vide est rechargé à chaque démarrage et tout le registre est effacé à la fin du jeu (REQ-E3D-008), et le transfert des géométries n'est pas limité en temps.
- Critère d'acceptation : deux lancements successifs en mode 2D commencent avec un registre vide.

**REQ-E3D-381** (P3) — En mode 2D, la passe 2 du fond DOIT faire croître la shape d'une tile inconnue non CHAR (REQ-E3D-021) par 4-voisinage (ordre droite, bas, haut, gauche, file FIFO) en ajoutant chaque voisine libre, inconnue, non CHAR, de même `pal` que la tile courante et dont l'image n'est pas déjà dans la shape. Si la shape obtenue est grande et ne remplit pas sa boîte en tiles, le moteur DOIT en extraire la première bande contiguë de lignes entièrement pleines (si `tSize.x ≥ tSize.y`, sinon de colonnes), libérer les autres tiles, étendre la bande par 4-voisinage aux tiles libres dont l'image est déjà dans la bande, puis lier la bande (REQ-E3D-160). La passe S1 DOIT, de même, n'ajouter que des sprites inconnus dont l'image n'est pas déjà dans la shape.
- Critère d'acceptation : en mode 2D, un mur de briques à deux images alternées (A B A B…) donne des shapes de 2 tiles (chaque image une fois), et non une seule grande shape.
- Notes : le résultat normatif est la bande retenue (première bande contiguë de lignes ou de colonnes entièrement pleines, étendue comme indiqué). Le sort, dans la frame courante, des tiles libérées par l'extraction (nouvelle shape ou absence d'affichage) n'est **pas spécifié**.

## 4. Formats et interfaces

### 4.1 Table des couleurs NES (indice → RVB, hexadécimal RRGGBB)

| Indices | +0 | +1 | +2 | +3 | +4 | +5 | +6 | +7 | +8 | +9 | +A | +B | +C | +D | +E | +F |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| $00 | 788084 | 0000FC | 0000C4 | 4028C4 | 94008C | AC0028 | AC1000 | 8C1800 | 503000 | 007800 | 006800 | 005800 | 004058 | 000000 | 000000 | 000000 |
| $10 | BCC0C4 | 0078FC | 0088FC | 6848FC | DC00D4 | E40060 | FC3800 | E46018 | AC8000 | 00B800 | 00A800 | 00A848 | 008894 | 2C2C2C | 000000 | 000000 |
| $20 | FCF8FC | 38C0FC | 6888FC | 9C78FC | FC78FC | FC589C | FC7858 | FCA048 | FCB800 | BCF818 | 58D858 | 58F89C | 00E8E4 | 606060 | 000000 | 000000 |
| $30 | FCF8FC | A4E8FC | BCB8FC | DCB8FC | FCB8FC | F4C0E0 | F4D0B4 | FCE0B4 | FCD884 | DCF878 | B8F878 | B0F0D8 | 00F8FC | C8C0C0 | 000000 | 000000 |

Justification : c'est la table qui détermine toutes les couleurs affichées par les profils au format indexé ; elle doit être reproduite à l'identique.

### 4.2 Réglages consommés par le pipeline

Valeur par défaut = valeur si le champ est absent du profil ou des réglages (SPEC-03/SPEC-09).

| Réglage | Type | Défaut | Effet (exigence) |
|---|---|---|---|
| `Clipping` (g, b, d, h) | 4 réels | 0,1 ; 8,1 ; 8,1 ; 0,1 | découpe des tiles (002) |
| `EmptyTile` | booléen | faux | tiles de fond vides (004) |
| `ScanDirection` | entier 0/1 | 0 | ordre de germination du fond (006) |
| `SameBgPalette` | booléen | faux | même `pal` exigée pour croître au fond (090) |
| `SameSpPalette` | booléen | faux | même `pal` exigée en passe S2 (131) |
| `SpriteTolerance` | entier ≥ 0 | 0 | tolérance d'adjacence en passe S2 (130, 131) |
| `RenderMode` | énum | GREEDY | algorithme global (340) |
| `Layer` : décalage, alignement par couche ; `DefLayer` ; `Auto` | — | voir 205 ; L1 ; faux | position z (205), couche par défaut, sans effet (342) |
| `BgColor`, couleur personnalisée | booléen, couleur | vrai, noir | couleur de fond (254) |

### 4.3 Paramètres d'une instance de pattern appris

| Paramètre | Valeur |
|---|---|
| géométrie | Default (valeur 0 de l'énumération de SPEC-04B : Default, HCYLINDER, VCYLINDER, CUBE, HALFHCYLINDER, HALFVCYLINDER) |
| profondeur | 3 |
| couche | L1 (énumération : UNIDENTIFIED = 0, UI = 1, L1 = 2, L2 = 3, L3 = 4, L4 = 5) |
| échelle / rotation / offset | (1,1,1) / (0,0,0) / (0,0,0) |
| pivot ; pivot de déformation | (0,5 ; 0,5 ; 0,5) ; (0,5 ; 0,5 ; 0,5) |
| amplitude de déformation | (0,0,0) ; matrices de vitesse et de phase : valeurs par défaut de SPEC-04B |
| alpha ; style de rendu ; `rm` | 1 ; 1 ; 0 |
| couleur vide ; anti-ombre, flip, solide, enveloppe complète | 0 ; faux |
| tags | aucun |
| texture | texture par défaut issue des images (SPEC-04B), format indexé |

### 4.4 Grandeurs dérivées exposées (coordonnées scène sauf mention)

| Grandeur | Définition |
|---|---|
| shape `Start`, `End`, `Size` | `pStart`, `pEnd`, `pEnd − pStart + (1,1)` (pixels écran) |
| shape `TStart`, `TEnd`, `TSize` | boîte en tiles (cellules de la grille de sa couche) |
| shape `BottomLeft` / `BL` | `(pStart.x, Hs − pEnd.y − 1)` |
| shape `TopRight` / `TR` | `(pEnd.x, Hs − pStart.y − 1)` |
| shape `Bg`, `Hidden`, `Palette`, `TileCount`, `Inside`, `AtBorder`, `Completed`, `Age` | REQ-E3D-040 à 048, 207 |
| instance `BottomLeft` | `(pStart.x, Hs − pEnd.y, −s/2)` |
| instance `TopRight` | `(pEnd.x, Hs − pStart.y, s/2)` |
| instance `BL` / `TR` | `(pStart.x, Hs − pEnd.y − 1, −s/2)` / `(pEnd.x, Hs − pStart.y − 1, s/2)` (à distinguer de `Bmin`/`Bmax` de REQ-E3D-205) |
| instance `size` / `Size` | `(pEnd.x − pStart.x + 1, pEnd.y − pStart.y + 1, s)` |
| instance `Pos`, `Center` | REQ-E3D-205 |
| instance `Palette`, `Enable`, `Index` | REQ-E3D-192, 191 ; indice de l'instance dans son pattern |

## 5. Vecteurs de test

Conventions : A, B, C… = images distinctes pleines (64 pixels, couleur 1) sauf mention ; une **ligne de fond** « r : A B A » signifie des tiles de fond aux positions écran `(8c, 8r)` pour c = 0, 1, 2 (cellules `(c+1, r+1)`) ; masque `(0, 0)` ; réglages par défaut ; aucun script. `[A B]` = pattern permanent 2×1 ; `[A B; C D]` = 2×2. « sc » = `selfConnected`.

**TV-E3D-01 — Région d'un pattern périodique.** Profil : `[A B]` sc vrai. Trame : ligne 5 : A B A B A B (6 tiles, au milieu de l'écran : décaler à c = 10…15). Attendu : 1 shape de fond, 6 tiles, pattern `[A B]`, `Completed` vrai. (Passe 1 la rejette car < 8 tiles, passe 3 la retient.) Variante 8 tiles A B A B A B A B : même résultat, retenue dès la passe 1.

**TV-E3D-02 — selfConnected faux.** Profil : `[A B]` sc faux. Même trame (6 tiles). Attendu : 3 shapes `{A,B}`, `{A,B}`, `{A,B}` (de gauche à droite). Variante sc vrai → 1 shape (TV-E3D-01). Variante trame « B A » : sc faux → 2 shapes `{B}`, `{A}` (la paire (B, A) n'existe pas) ; sc vrai → 1 shape.

**TV-E3D-03 — Taille de contrainte.** Profil : `[A]` sc vrai, contrainte `(2, 99)`. Trame : ligne 5 : A A A A A (c = 10…14). Attendu : 3 shapes de 2, 2 et 1 tiles, toutes liées à `[A]`. Variante contrainte `(99, 1)` et une colonne de 3 A → 3 shapes d'une tile.

**TV-E3D-04 — skipGeo.** Profil : `[A B]` sc faux, `skipGeo` vrai. Trame : A en `(80, 40)`, B en `(80, 48)` (B sous A). Attendu : 1 shape `{A, B}`. Variante `skipGeo` faux : 2 shapes.

**TV-E3D-05 — Rétrécissement des candidats et plus petit pattern.** Profil (dans cet ordre) : P1 = `[A B]`, P2 = `[A B C]`, tous deux sc faux. Trames (ligne 5, c = 10…) :
- « A B » → 1 shape liée à P1 (`|Img|` 2 < 3), `Completed` vrai ;
- « A B C » → 1 shape liée à P2 ;
- « C A B » → 2 shapes : `{C}` liée à P2 (C n'a pas de voisin droit dans P2), `Completed` faux ; puis `{A, B}` liée à P1.

**TV-E3D-06 — Ordre des passes.** Profil : Q = `[A B; B A]` sc vrai ; `[D]` sc vrai. Trame : damier 4×2 aux lignes 2–3, c = 4…7 (ligne 2 : A B A B ; ligne 3 : B A B A) ; une tile inconnue U en `(160, 0)` (c = 20, ligne 0) ; une tile D en `(160, 120)`. Masque `(0, 0)`. Attendu, ordre des shapes : (1) le damier, 8 tiles, Q (passe 1) ; (2) `{U}` (passe 2 ; U touche le bord haut `pStart.y = 0` non > 0 → AtBorder → pattern **temporaire** appris) ; (3) `{D}` (passe 3). Avec `ScanDirection = 1` : même ordre (les passes priment sur l'ordre de germination).

**TV-E3D-07 — Apprentissage.** Profil vide. Trame 1 : ligne 10 : U U U (c = 10…12), U inconnue. Attendu trame 1 : `{U}` (c = 10, passe 2, pattern permanent appris P_U 1×1) puis `{U, U}` (c = 11–12, passe 3, liée à P_U). Trame 2 identique : 1 shape de 3 tiles liée à P_U (passe 3). Variante U en bord (c = 0, ligne 10, masque `(8, 0)`) : trame 1 → pattern temporaire T_U ; trames suivantes : la tile reste inconnue et est liée à T_U par clé exacte, aucun pattern permanent n'est créé.

**TV-E3D-08 — Sprites.** Sprites (ordre d'émission) : S1 A `(100, 50)` pal 0 ; S2 B `(108, 50)` pal 0 ; S3 C `(116, 52)` pal 0.
- Profil vide : passe S1 → shape `{S1, S2}` (adjacence exacte), puis `{S3}` ; palettes 4 et 4 ; patterns appris `{A, B}` (2×1) et `{C}`.
- Profil `[A B C]` sc faux, `SpriteTolerance = 2` : passe S2 → 1 shape `{S1, S2, S3}`. Avec `SpriteTolerance = 0` : `{S1, S2}` et `{S3}`, toutes deux liées à `[A B C]`.
- Profil `[A B C]`, tolérance 2, S2 en pal 1 et `SameSpPalette` vrai : `{S1}`, `{S2}`, `{S3}` ; avec `SameSpPalette` faux : 1 shape de palette 4.

**TV-E3D-09 — Palette de frame.** `p[0] = $0F`, `p[1] = $30`, `p[4] = $12`, `p[5] = $16`, `p[17] = $27`, autres 0. Attendu : entrée 0 → $0F ; 1 → $30 ; 4 → $0F (et non $12) ; 5 → $16 ; 16 → $0F ; 17 → $27 ; 37 → $16 ; 49 → $27. RVB de l'entrée 5 : `#FC3800` ; de l'entrée 17 : `#FCA048`.

**TV-E3D-10 — Identité d'image et flips.** Image X : `low` = un seul bit allumé donnant le pixel `(1, 2)` non flippé, `high = 0`. Profil `[X]`. Sprite d'image X avec `flipX` vrai → image distincte X' (pixel affiché en `(6, 2)`, boîte `(6, 2)–(6, 2)`) ; X' est inconnue : le sprite fait apprendre un nouveau pattern et n'est pas lié à `[X]`.

**TV-E3D-11 — Découpe.** Réglages par défaut. Fond en `x = −8` et `x = 248` écartés ; sprite en `y = 240` écarté ; fond en `(−7, −7)` conservé dans la cellule `(1, 1)`.

**TV-E3D-12 — Tracker.** Départ sur une shape de sprites T (pattern PT, centre (100, 100)). Frame 2, shapes de sprites dans l'ordre : X (PX, centre (104, 100)), Y (PT, centre (140, 100)) → renvoie Y ; PX ∈ E ; q = (140, 100). Frame 3 : X seule en (141, 100) → PX exclu → rien ; q inchangé. Frame 4 : Z (PZ, centre (145, 103)) → distance 5,83 < 9 → Z ; PZ ajouté à I ; q = (145, 103).

**TV-E3D-13 — Instance réinitialisée par frame.** Profil : `[A]` avec tag « t » ; script de tags « t » dont `UpdateS` fait `shape.Offset.x = shape.Offset.x + 5`. Trame identique répétée 3 fois. Attendu : `Pos.x` identique aux trois frames, égal à la valeur sans script + 5.

**TV-E3D-14 — Shape injectée.** Pattern P 1×1 d'une image pleine. `Frame:CreateNewShape(P, (10,5 ; 20,25 ; 3), 5)` pendant `Update`. Attendu : une shape de sprites non scriptable ; tile en `(10, 221)` avec `pal` 1 ; shape `pStart = (10, 221)`, `pEnd = (17, 228)`, `BottomLeft = (10, 19)` ; offset d'instance `(0,5 ; −0,25 ; 3)` ; `Pos` (profondeur 3, couche L1, pivot centré) = `(14,5 ; 22,75 ; 4,5)`.

### Profils intégrés qui exercent chaque règle

| Règle | Profils (assets-neutral/profiles) |
|---|---|
| `selfConnected` faux (061) | Felix the Cat, punchout, Super Spike V Ball (U), Jackal (U) |
| contrainte de taille (062) | Ninja Gaiden III, Jackal (U), Rockman 3, Micro Mages, Castlevania (U) (PRG 1) |
| `skipGeo` (062) | Life Force (U), Ninja Gaiden III, Kage (J), Final_Fantasy_USA |
| `ScanDirection = 1` (006) | Contra, Final_Fantasy_USA, Adventures of Lolo 2/3, Jackal (U), Digdug |
| `SameBgPalette` vrai (090) | 48 profils, ex. Arkanoid (U), Batman, Mega Man 2 |
| `SpriteTolerance` > 0 (131) | Chip 'n Dale (7), Felix the Cat (7), Contra (4), Jackal (U) (5) |
| `SameSpPalette` vrai (131) | DQ3_DW3, Jackal (U) |
| `EmptyTile` vrai (004) | Adventures of Lolo, Castlevania (U) (PRG 1), Final_Fantasy_USA |
| `Disable` par script (093) | Jackal (U) |
| shapes injectées (230–232) | Chip 'n Dale, City Connection, Jackal (U), Road Fighter, Tiny Toon Adventures (CreateNewShape) ; Moai-kun, Tetris (AddNewShape) |
| frame épinglée (233) | DQ3_DW3 |
| `BuildPattern` (322) | Mega Man 2, Legend Of Zelda, Hudson's Adventure Island (U) |
| `DetectShape` non appelé (323) | Super Bat Puncher Demo |
| écriture de palette (252) | punchout |

## 6. Questions ouvertes

- **Q-01** — *Close par DEC-04-Q01* (jamais appelé, fidèle). `DetectShape` (REQ-E3D-323) : l'original ne l'appelle jamais. Faut-il conserver ce comportement (fidélité, choix actuel) ou l'implémenter (intention probable de l'auteur, mais change « Super Bat Puncher Demo ») ? Décision produit à prendre ; la spec suit la fidélité.
- **Q-02** — *Close par DEC-04-Q02* (ordre d'inscription, différentiel). Ordre des candidats à égalité de `|Img|` (REQ-E3D-091) après retrait de patterns (par `Pattern.RemoveWithTag` ou remplacement par apprentissage) : l'original dépend de l'ordre interne d'une table de hachage. La spec impose l'ordre d'inscription ; à valider par le juge (round 2) sur des profils à patterns concurrents.
- **Q-03** — *Close par DEC-04-Q03* (SPEC-02 REQ-CORE-046 fait foi). Valeurs exactes du masque `(mX, mY)` par type de miroir et effet du recadrage gauche du PPU : relèvent de SPEC-02 ; le présent document les consomme seulement (REQ-E3D-045).
- **Q-04** — *Close par DEC-04-Q04* (conservé, fidèle). `TopRight(P)` d'un pattern (REQ-E3D-064) utilise la largeur de grille pour l'axe y. Conservé pour compatibilité ; aucun profil intégré ne semble en dépendre pour des grilles non carrées (à confirmer par SPEC-05).
- **Q-05** — *Close par DEC-04-Q05* (conservé, fidèle). Chevauchement frame épinglée (REQ-E3D-233) : rectangles en coordonnées scène comparés à des boîtes en pixels écran (axe y inversé). Fidélité conservée ; seul « DQ3_DW3 » l'utilise. Vérifier visuellement en round 2.
- **Q-06** — *Close par DEC-04-Q06 et DEC-R2-12-2* (moyenne ≤ 4 ms, 99ᵉ centile ≤ 8 ms). Le seuil de 4 ms (REQ-E3D-360) est une cible du projet, pas une mesure de l'original ; à confirmer avec SPEC-12.
- **Q-07** — *Close par DEC-04-Q07* (glossaire de SPEC-00 corrigé en v1.1). Coordination de vocabulaire : « instance de frame » (ici) = objet `shape` exposé à Lua (SPEC-05) ; « instance de pattern » = « variante 3D » de SPEC-03 (`Pattern.10`). À aligner avec le glossaire de SPEC-00.

## Historique

- 1.1-draft (AMD-001, relecture R2) : notes LEAK reformulées (REQ-E3D-207, 280, 342) ; notation Bmin/Bmax dans REQ-E3D-205 et §4.4 (collision BL/TR) ; composantes de Clipping et écart filtre/rendu (REQ-E3D-002, DEC-R2-T1/T2) ; état appris dans le savestate (REQ-E3D-008, DEC-R2-T5) ; classes 2D définies ici seulement ; tracker reformulé en résultat (REQ-E3D-300) ; REQ-E3D-345 informative ; budgets REQ-E3D-360/361 (DEC-R2-12-2) ; REQ-E3D-381 ; critères exécutables et étiquettes [ROM] (060, 233, 252, 322) ; questions closes.
