# SPEC-04B — Géométrie 3D : voxelisation des patterns, maillage, texturage et placement
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-00 (glossaire), SPEC-02 (contrat de tuile, palette du cœur), SPEC-03 (format de profil : champs des objets 3D et des textures), SPEC-04 (conversion frame → shapes et patterns), SPEC-05 (Lua : hooks et propriétés exposées), SPEC-06 (scène, caméra, matériaux, ombres, échelle de scène), SPEC-07 (temps de frame, indicateur de chargement), SPEC-09 (réglages : couches, ombres, algorithme de rendu)

## 1. Objet et périmètre

Ce document fixe, pour un pattern reconnu (SPEC-04) et les paramètres 3D lus dans le profil (SPEC-03) ou modifiés par script (SPEC-05) :

1. le repère et les unités de la scène, et le placement de chaque objet 3D et de chaque tuile ;
2. la construction du **champ de voxels** d'un pattern pour chaque type de géométrie (`Geo3DType`) et chaque option (`solid`, `fullWarp`, `flip`, `antiShadow`, couleur transparente) ;
3. la conversion du champ de voxels en maillages (algorithmes `GREEDY`, `GREEDY_MULTICOLOR`, `MARCHING`) : sommets, faces, normales, coordonnées de texture ;
4. la résolution des couleurs (palette, textures personnalisées), la transparence, les drapeaux d'ombre, la déformation animée ;
5. le cycle de vie des maillages et le contrat de sortie vers le moteur de rendu (SPEC-06).

Hors périmètre : la reconnaissance des shapes et l'association shape ↔ pattern (SPEC-04) ; le schéma protobuf (SPEC-03, auquel les numéros de champs cités ici renvoient à titre de repère) ; l'éclairage, les shaders, la caméra, le découpage d'écran (SPEC-06) ; l'éditeur de profils et l'export de modèles (exclus du projet).

Toutes les géométries sont décrites par leur **résultat** : ensemble de voxels, puis faces produites. L'implémentation est libre (structures, algorithmes, threads) tant que le résultat est identique dans les tolérances indiquées.

## 2. Concepts et vocabulaire

- **Pixel NES, coordonnées écran** : un pixel est repéré par `(px, py)` en coordonnées écran de SPEC-04 (origine en haut à gauche, `py` vers le bas). **Hauteur scène** `Hs = 248` : constante de retournement de l'axe Y (31 rangées de tuiles de la zone utile du cœur, 31 × 8 = 248). Le **repère de scène** complet (image NES, cadre de référence 256 × 240, rectangle visible, centre `C`, relation inverse) est défini **une seule fois**, par REQ-E3D-410 ; SPEC-04, SPEC-06 et SPEC-08 y renvoient.
- **Tuile** : bloc de 8 × 8 pixels à 2 bits de couleur (SPEC-02). **Tuile 2D distincte** : tuile identifiée selon SPEC-04 (deux tuiles de même contenu après flip sont la même tuile 2D).
- **Shape** : groupe de tuiles d'une frame reconnu comme un tout (SPEC-04). Sa **boîte pixel** est `[pStart, pEnd]` (coins inclusifs, en pixels NES, bornes des pixels non transparents des tuiles). Une shape est de **fond** (BG) ou de **sprite**.
- **Pattern** : modèle de shape mémorisé dans le profil (SPEC-04). Il a une grille de `tw × th` cases de tuiles (`tSize`) ; chaque case contient une tuile 2D ou est vide. L'**image du pattern** a `8·tw × 8·th` pixels.
- **Objet 3D** : jeu de paramètres 3D attaché à un pattern (géométrie, profondeur, couche, transformation, texture…). Un pattern porte **un ou plusieurs** objets 3D (liste ordonnée). Les noms de paramètres utilisés ici sont ceux exposés aux scripts Lua (SPEC-05) : `geo`, `sizeZ`, `solid`, `fullWarp`, `flip`, `antiShadow`, `emptyColor`, `rm`, `layer`, `scale`, `rot`, `offset`, `pivot`, `alpha`, `renderStyle`, `deltaAmp`, `deltaPivot`, `deltaSpeed`, `deltaOffset`, `texOffset`, `texTilting`, `customPalette`, `CData`, `fillrate`, `TagStr`.
- **Instance** : à chaque frame, chaque shape reconnue produit une instance par objet 3D de son pattern ; l'instance copie les paramètres de l'objet 3D de référence (un script peut les modifier pour la frame, SPEC-05) et réutilise ses maillages.
- **Texture de l'objet** : image de `tw_tex × th_tex` texels. Par défaut, c'est l'image du pattern (valeurs 0 à 3) ; un profil peut fournir une **texture personnalisée** (SPEC-03), au format **indexé** (valeur de texel 0 à 7 : 2 bits de couleur + 1 bit supplémentaire) ou **direct** (couleur RGB 24 bits par texel). Toute lecture de texel `(x, y)` se fait **modulo** la taille de la texture : `(x mod tw_tex, y mod th_tex)`.
- **Valeur de texel** `t(x, y)` : entier lu dans la texture (0–7 en indexé, entier RGB en direct).
- **Couleur transparente** `e` : valeur du paramètre `emptyColor` après normalisation (REQ-E3D-424). Un texel de valeur `e` est **transparent**.
- **Profondeur** `D` : paramètre `sizeZ`, entier de 0 à 96.
- **Voxel** : cube unité du champ d'une tuile ; repéré dans la tuile par `(i, j, k)`, `i` = colonne (0 à 7, vers la droite), `j` = rangée (0 à 7, **vers le haut**), `k` = tranche de profondeur (0 à `D − 1`, `k = 0` côté spectateur).
- **Fente** (slot) : en algorithmes GREEDY, chaque voxel porte trois affectations indépendantes, une par axe `a ∈ {X, Y, Z}` ; une affectation est soit « vide », soit les coordonnées `(tx, ty)` d'un texel. En MARCHING, chaque voxel porte une seule affectation.
- **Algorithme de maillage** : `GREEDY` (faces rectangulaires fusionnées sans tenir compte de la couleur), `GREEDY_MULTICOLOR` (fusion limitée aux couleurs identiques), `MARCHING` (surface lissée par cubes marchants).
- **Couche** (`layer`, `ZLayer`) : `UNIDENTIFIED` (0), `UI` (1), `L1` (2), `L2` (3), `L3` (4), `L4` (5). **Alignement** d'une couche : `FRONT`, `CENTER`, `BACK`.

## 3. Exigences

### 3.1 Repère, unités et placement

**REQ-E3D-400** (P1) — La scène DOIT utiliser le repère suivant, dit **repère scène** : axe X vers la droite de l'image, axe Y vers le haut de l'image, axe Z en profondeur, **croissant en s'éloignant du spectateur** placé face à l'image (repère main gauche). L'unité de longueur DOIT être le pixel NES : 1 pixel = 1 unité sur X et Y, et une tranche de profondeur = 1 unité sur Z.
- Critère d'acceptation : une tuile pleine (64 pixels opaques) de profondeur 8, sans transformation, produit une boîte de 8 × 8 × 8 unités (vecteur TV-W1).
- Notes : un moteur de rendu en repère main droite (OpenGL) doit convertir, par exemple par `z' = −z`, et inverser en conséquence l'ordre d'enroulement des triangles et le signe des rotations autour de X et Y (SPEC-06). Toutes les formules de ce document sont exprimées en repère scène.

**REQ-E3D-401** (P1) — Le pixel NES `(px, py)` DOIT correspondre, dans le plan XY de la scène, au carré `[px, px + 1] × [Hs − py − 1, Hs − py]` (avant transformations d'objet et d'échelle de scène).
- Critère d'acceptation : le pixel (0, 0) occupe `x ∈ [0, 1]`, `y ∈ [247, 248]` ; le pixel (255, 239) occupe `x ∈ [255, 256]`, `y ∈ [8, 9]`.
- Notes : même convention que REQ-E3D-205 (SPEC-04) ; voir REQ-E3D-410 pour l'ensemble des relations du repère.

**REQ-E3D-402** (P1) — (Rappel de REQ-E3D-205, SPEC-04, qui fait foi.) La **couche effective** `Le` d'une instance DOIT être sa couche `layer`, sauf si celle-ci vaut `UNIDENTIFIED` : alors `Le = L1` pour une shape de sprite, et `Le` = la couche par défaut des réglages (SPEC-09, valeur d'usine `L1`) pour une shape de fond.
- Critère d'acceptation : un objet de fond `UNIDENTIFIED` suit le changement du réglage de couche par défaut ; un objet de sprite `UNIDENTIFIED` reste en `L1`.

**REQ-E3D-403** (P1) — Chaque couche effective DOIT avoir un décalage de profondeur `Off(Le)` et un alignement, réglables (SPEC-09), de valeurs d'usine : `UI` : 0, `CENTER` ; `L1` : 0, `FRONT` ; `L2` : 12, `FRONT` ; `L3` : 24, `FRONT` ; `L4` : 36, `FRONT`. Le coefficient d'alignement `κ` vaut 1 pour `FRONT`, 0 pour `CENTER`, −1 pour `BACK`. L'intervalle de profondeur de l'objet DOIT être `[zF, zB]` avec `zF = Off(Le) + (D/2)(κ − 1)` et `zB = Off(Le) + (D/2)(κ + 1)`.
- Critère d'acceptation : `L1` `FRONT`, `D = 4` → `[0, 4]` ; `L2` `FRONT`, `D = 4` → `[12, 16]` ; `CENTER`, `D = 4`, décalage 0 → `[−2, 2]` ; `BACK`, `D = 4`, décalage 0 → `[−4, 0]`.
- Notes : `FRONT` aligne la face avant de tous les objets d'une couche sur un même plan, quelle que soit leur profondeur ; les couches successives sont de plus en plus éloignées (12 unités d'écart par défaut). Pour les géométries cylindriques, `D` est la profondeur effective (REQ-E3D-452).

**REQ-E3D-404** (P1) — (Rappel de REQ-E3D-205, SPEC-04, qui fait foi.) La **boîte de placement** d'une instance DOIT être `Bmin = (pStart.x, Hs − pEnd.y − 1, zF)` et `Bmax = (pEnd.x + 1, Hs − pStart.y, zB)`, où `[pStart, pEnd]` est la boîte pixel de la shape de cette frame. Son **point d'ancrage** DOIT être `Pos = Bmin ⊙ (1 − pivot) + Bmax ⊙ pivot + offset` (⊙ = produit composante par composante ; `pivot` vaut (0,5 ; 0,5 ; 0,5) par défaut, `offset` (0, 0, 0)). On note `Ctr` la valeur de `Pos` calculée avec `pivot = (0,5 ; 0,5 ; 0,5)`.
- Critère d'acceptation : shape de boîte pixel (16, 32)–(23, 39), `D = 4`, `L1` `FRONT`, défauts → `Pos = (20, 212, 2)` ; avec `pivot = (0, 0, 0)` → `Pos = (16, 208, 0)` ; avec `offset = (1, −2, 3)` → `Pos` décalé d'autant.

**REQ-E3D-405** (P1) — Chaque tuile `τ` de la shape, de coin supérieur gauche `(tx, ty)` en pixels NES, DOIT être représentée par son maillage (REQ-E3D-540) placé au **centre de tuile** `T = (tx + 4 + offset.x, Hs − ty − 4 + offset.y, Ctr.z)`. Pour une shape de sprite, le rendu ajoute un décalage de profondeur `n = −0,1` (tuile devant le fond) ou `n = +0,1` (tuile de sprite à priorité « derrière le fond », SPEC-02) ; les tuiles de fond ont `n = 0`. Conformément à DEC-06-Q4 et DEC-R2-T4, ce décalage **n'est pas inclus** dans les transformations livrées par le moteur 3D (REQ-E3D-641) : c'est le rendu qui l'applique (SPEC-06, REQ-SCN-037), dans le repère local de l'objet, avant l'échelle et la rotation (même place que dans REQ-E3D-406), et une seule fois. Le moteur 3D livre en revanche, pour chaque tuile, l'indicateur « sprite » et l'indicateur de priorité qui permettent au rendu de choisir `n`.
- Critère d'acceptation : TV-W2 : la transformation livrée pour une tuile de sprite est identique à celle de la même tuile de fond ; après application du décalage par le rendu, la face avant d'une tuile de sprite en `L1 FRONT` est à `z = −0,1`.

**REQ-E3D-406** (P1) — La position scène d'un point `v` du maillage local d'une tuile (REQ-E3D-504, REQ-E3D-512) DOIT être `P = Pos + R · (S ⊙ (T + (0, 0, n) − Pos + v + δ))`, où `S` est l'échelle de l'objet (`scale`, éventuellement corrigée par REQ-E3D-408), `R` la rotation de l'objet (REQ-E3D-407) et `δ` la déformation animée (REQ-E3D-568, nulle par défaut). La transformation de scène (échelle globale, conteneur de jeu ou d'interface) s'applique ensuite (SPEC-06).
- Critère d'acceptation : TV-W3 : avec `scale = (2, 1, 1)` et le pivot par défaut, une tuile pleine centrée sur l'ancrage s'étend de 16 unités en X, symétriquement autour de `Pos.x` ; avec `pivot = (0, 0, 0)`, elle s'étend vers +X à partir de `Bmin.x`.
- Notes : l'échelle et la rotation s'appliquent donc autour du point d'ancrage `Pos`, pour toute la shape à la fois.

**REQ-E3D-407** (P1) — La rotation `R` DOIT être construite à partir de `rot = (rx, ry, rz)` en degrés, avec les matrices usuelles `Rx(θ) = [[1,0,0],[0,c,−s],[0,s,c]]`, `Ry(θ) = [[c,0,s],[0,1,0],[−s,0,c]]`, `Rz(θ) = [[c,−s,0],[s,c,0],[0,0,1]]` (`c = cos θ`, `s = sin θ`, appliquées à des vecteurs colonnes en coordonnées scène) : par défaut `R = Rx(rx) · Ry(ry) · Rz(rz)` ; si les étiquettes de l'objet (`TagStr`, liste séparée par des virgules, espaces de bord ignorés) contiennent `_gaxis`, alors `R = Ry(ry) · Rx(rx) · Rz(rz)`.
- Critère d'acceptation : `rot = (0, 0, 90)` transforme le vecteur relatif (8, 0, 0) en (0, 8, 0) ; `rot = (90, 90, 0)` donne `Rx(90)·Ry(90)` sans `_gaxis` et `Ry(90)·Rx(90)` avec, ce qui transforme (0, 0, 1) respectivement en (1, 0, 0) et en (0, −1, 0).

**REQ-E3D-408** (P1) — Pour une instance dont l'algorithme de maillage effectif est `MARCHING`, l'échelle utilisée DOIT être `S = scale ⊙ (sx/(sx+1), sy/(sy+1), sz/(sz+1))` avec `(sx, sy, sz) = (pEnd.x − pStart.x + 1, pEnd.y − pStart.y + 1, D)`.
- Critère d'acceptation : shape d'un seul pixel, `D = 1`, MARCHING : l'octaèdre de rayon 1 (TV-M1) apparaît de rayon 0,5 dans la scène.
- Notes : la surface MARCHING déborde d'environ un demi-voxel de chaque côté (REQ-E3D-521) ; ce facteur ramène l'encombrement à la boîte pixel.

**REQ-E3D-409** (P2) — Une instance de couche `UI` DOIT être placée par les mêmes formules mais rattachée au conteneur d'interface de la scène (SPEC-06) au lieu du conteneur de jeu ; elle ne projette ni ne reçoit d'ombre (REQ-E3D-567) et utilise la classe de matériau « interface » (REQ-E3D-642).
- Critère d'acceptation : passer `layer` d'un objet à `UI` le fait suivre le conteneur d'interface et supprime ses ombres.

**REQ-E3D-410** (P1) — **Repère de scène : définition unique** (DEC-R2-T2). Toutes les spécifications DOIVENT utiliser les relations suivantes, dans le repère scène de REQ-E3D-400, échelle de scène égale à 1 :
1. **Grille de conversion** : 32 colonnes et 31 rangées de tuiles (`Hs = 31 × 8 = 248`).
2. **Image NES** : le pixel `(px, py)` occupe `[px, px + 1] × [Hs − py − 1, Hs − py]` (REQ-E3D-401). L'image visible de 256 × 240 pixels (`py` de 0 à 239) occupe donc `x ∈ [0, 256]`, `y ∈ [8, 248]`, `z = 0` ; son centre géométrique est `(128, 128, 0)`. Une tuile de fond capturée partiellement au-dessus de l'image (`y` écran négatif, SPEC-02) déborde au-dessus de `y = 248`.
3. **Cadre de référence** : rectangle `z = 0`, `x ∈ [0, 256]`, `y ∈ [0, 240]`, de centre **`C = (128, 120, 0)`**. C'est dans ce cadre que sont exprimés le rectangle visible (point 4), la cible par défaut de la caméra, le plan du HUD, le backdrop et le boîtier TV (SPEC-06). Il est **décalé de 8 unités vers le bas** par rapport à l'image NES : `C` est 8 unités sous le centre de l'image. Ce décalage est celui de l'original et DOIT être conservé (il fixe le cadrage par défaut observable).
4. **Rectangle visible** : avec les marges de découpe `Clipping = (c0, c1, c2, c3)` (indices 0 gauche, 1 bas, 2 droite, 3 haut, DEC-R2-T1), `R = [c0, 256 − c2] × [c1, 240 − c3]` en coordonnées scène. En pixels NES, les colonnes visibles sont `px ∈ [c0, 256 − c2]` et les lignes visibles `py ∈ [8 + c3, 248 − c1]` : la marge haute `c3` se compte à partir de la ligne 8 de l'image, la marge basse `c1` à partir du bas de la ligne 247 (8 lignes sous le bas de l'image). Avec les marges par défaut `(0,1 ; 8,1 ; 8,1 ; 0,1)`, les lignes 0 à 8,1 et les colonnes 247,9 à 256 sont masquées, aucune ligne du bas ne l'est.
5. **Relation inverse** : un point `(x, y)` du plan `z = 0` montre le pixel NES `(⌊x⌋, ⌊Hs − y⌋)`. La visée du Zapper utilise cette même relation (SPEC-08 REQ-INP-063, DEC-08-4) (fidèle à l'original, décalée de 8 lignes).
- Critère d'acceptation : le pixel (0, 0) occupe `y ∈ [247, 248]`, le pixel (0, 239) `y ∈ [8, 9]` ; le centre de l'image est `(128, 128)` ; avec les marges `(0 ; 7,9 ; 0 ; 0,1)`, `R = [0, 256] × [7,9 ; 239,9]`, les lignes NES 0 à 7 sont entièrement masquées et la ligne 239 entièrement visible ; avec les marges par défaut, le centre de `R` est `(124, 124)`.
- Notes : la grille de 31 rangées vient de la capture des rangées de fond partiellement visibles (SPEC-02, REQ-CORE-038). Le filtrage des tiles par les marges (SPEC-04 REQ-E3D-002) est exprimé en coordonnées écran et est plus large que `R` de 8 lignes en haut.

### 3.2 Champ de voxels

**REQ-E3D-420** (P1) — Pour chaque objet 3D et chaque **tuile 2D distincte** de son pattern, le moteur DOIT construire un champ de 8 × 8 × `D` voxels. Si `D = 0`, aucun voxel n'est construit et la tuile n'a pas de géométrie (maillage vide).
- Critère d'acceptation : `sizeZ = 0` → maillages vides, rien n'est dessiné pour cet objet ; les autres objets 3D du même pattern restent visibles.

**REQ-E3D-421** (P1) — Le pixel `(x, y)` de l'image du pattern (`y = 0` en haut) DOIT correspondre, dans la tuile de la case `(⌊x/8⌋, ⌊y/8⌋)`, aux voxels `(i, j) = (x mod 8, 7 − (y mod 8))` pour tout `k`. Les règles de construction (3.3) écrivent des affectations sur des **coordonnées pattern** `(x, y, k)` ; une écriture sur une case vide de la grille, ou hors de la grille, est sans effet.
- Critère d'acceptation : TV-G1 : le pixel (0, 0) du pattern est le voxel (0, 7, ·) de la tuile (0, 0), dont le maillage occupe le coin supérieur gauche de la tuile.

**REQ-E3D-422** (P1) — En algorithmes GREEDY, une écriture « sur tous les axes » DOIT affecter les trois fentes du voxel ; une écriture « sur l'axe a » n'affecte que la fente `a`. En MARCHING, toute écriture DOIT affecter l'unique fente du voxel. Les fentes non écrites sont vides.
- Critère d'acceptation : géométrie `CUBE` (REQ-E3D-443) en GREEDY : un voxel de la face avant n'a que sa fente Z remplie, et ne génère donc que des faces perpendiculaires à Z (TV-C1).

**REQ-E3D-423** (P1) — Une écriture qui affecte le texel `(tx, ty)` à une fente DOIT rendre cette fente **vide** si `t(tx, ty) = e`. Plusieurs écritures sur la même fente DOIVENT se résoudre par « la dernière écrite l'emporte », dans l'ordre canonique de chaque règle de construction (3.3), y compris quand la dernière écriture rend la fente vide.
- Critère d'acceptation : TV-C2 : en `CUBE`, une face dont les texels sont transparents est trouée à ces positions.

**REQ-E3D-424** (P1) — La couleur transparente DOIT être `e = emptyColor` si `emptyColor ≥ 0`, et `e = emptyColor + 5` si `emptyColor < 0`. Avec une texture indexée, les valeurs de texel vont de 0 à 7 ; `e = 4` signifie donc « aucune des couleurs 0 à 3 n'est transparente » (seuls les texels portant le bit supplémentaire et valant exactement 4 le sont). Avec une texture directe, seul un texel de valeur entière exactement égale à `e` est transparent (en pratique le texel de valeur 0 quand `e = 0`).
- Critère d'acceptation : TV-G5 : `emptyColor = 4`, pixel de couleur 0 → voxel plein, rendu avec la couleur 0 de la palette.

**REQ-E3D-425** (P1) — Quand une même tuile 2D occupe plusieurs cases du pattern, ces cases DOIVENT partager un champ de voxels unique (donc un maillage unique) ; les écritures effectuées pour chacune de ces cases s'appliquent toutes à ce champ, dans l'ordre canonique (REQ-E3D-423).
- Critère d'acceptation : TV-C3 : pattern de 2 × 1 cases portant deux fois la même tuile pleine, `CUBE`, `D = 2` : chaque tuile affichée possède à la fois la paroi gauche (x = −4) et la paroi droite (x = +4), soit 10 faces par tuile, au lieu de 8 si les tuiles étaient différentes.
- Notes : comportement visible (parois intérieures) ; il DOIT être reproduit pour les profils embarqués.

**REQ-E3D-426** (P1) — La texture d'un objet DOIT être sa texture personnalisée si le profil en fournit une (SPEC-03 : texture de taille non nulle), sinon l'image du pattern. Toutes les lectures de texel pendant la construction DOIVENT se faire modulo la taille de la texture (§2).
- Critère d'acceptation : en harnais, sans ROM : texture personnalisée 8 × 8 sur un pattern de 8 × 192 pixels (cas présent dans le profil embarqué « Mega Man 1 » de `assets-neutral/profiles`, ou pattern synthétique équivalent) : les maillages des 24 tuiles portent les mêmes coordonnées de texture modulo 8, chaque bloc de 8 rangées relisant la même texture.

### 3.3 Types de géométrie

Notation : `Wp = 8·tw`, `Hp = 8·th` (taille de l'image du pattern), `D` = profondeur, `t` = valeur de texel, `e` = couleur transparente. « Écrire `(x, y, k) ← (tx, ty)` » signifie affecter le texel `(tx, ty)` aux coordonnées pattern `(x, y, k)`.

**REQ-E3D-440** (P1) — `geo = Default` (0), `fullWarp = faux` : **extrusion**. Pour chaque pixel `(x, y)` de l'image du pattern tel que `t(x, y) ≠ e`, et pour chaque `k ∈ [0, D)`, le moteur DOIT écrire `(x, y, k) ← (x, y)` sur tous les axes. Ordre canonique : `x` croissant, puis `y` croissant, puis `k` croissant.
- Critère d'acceptation : TV-G1 à TV-G4 : chaque pixel opaque devient un prisme de section 1 × 1 et de profondeur `D`, dont toutes les faces portent la couleur du pixel.

**REQ-E3D-441** (P1) — `geo = Default`, `fullWarp = vrai`, algorithme GREEDY ou GREEDY_MULTICOLOR : **extrusion enveloppée**. Pour chaque pixel `(x, y)` avec `t(x, y) ≠ e` et chaque `k ∈ [0, D)`, le moteur DOIT écrire, dans cet ordre : sur l'axe Y `(x, y, k) ← (x, D − 1 − k)` ; sur l'axe X `(x, y, k) ← (D − 1 − k, y)` ; sur l'axe Z `(x, y, k) ← (x, y)`. Ordre canonique comme REQ-E3D-440.
- Critère d'acceptation : TV-G6 : un pixel isolé en (0, 0), `D = 2`, dont la texture n'a qu'un texel opaque : les faces avant et arrière sont complètes, les faces latérales n'existent que sur la moitié arrière (`z ∈ [0, 1]`).
- Notes : les faces du dessus et du dessous montrent les `D` premières rangées de la texture, les faces gauche et droite ses `D` premières colonnes, la rangée ou colonne `D − 1` étant côté spectateur.

**REQ-E3D-442** (P2) — `geo = Default`, `fullWarp = vrai`, algorithme MARCHING : pour chaque pixel `(x, y)` avec `t(x, y) ≠ e` et chaque `k ∈ [0, D)`, le moteur DOIT écrire `(x, y, k) ← (x, y*)` où `y*` est le plus grand entier de l'intervalle `[y, y + k]` tel que `t(x, y*) ≠ e` (lecture modulo la hauteur de texture). Cet entier existe toujours, puisque `t(x, y) ≠ e` : la recherche ne descend jamais sous `y`.
- Critère d'acceptation : colonne de 3 pixels de couleurs 1, 2, 3 de haut en bas, `D = 3` : la tranche `k = 1` du pixel du haut prend la couleur 2, la tranche `k = 2` la couleur 3.

**REQ-E3D-443** (P1) — `geo = CUBE` (3) : **boîte texturée**. Le moteur DOIT écrire, dans cet ordre :
1. pour `x ∈ [0, Wp)` puis `k ∈ [0, D)` : sur l'axe Y, `(x, Hp − 1, k) ← (x, k)` puis `(x, 0, k) ← (x, D − 1 − k)` ;
2. pour `y ∈ [0, Hp)` puis `k ∈ [0, D)` : sur l'axe X, `(0, y, k) ← (D − 1 − k, y)` puis `(Wp − 1, y, k) ← (D − 1 − k, y)` ;
3. pour `x ∈ [0, Wp)` puis `y ∈ [0, Hp)` : sur l'axe Z, `(x, y, D − 1) ← (x, y)` puis `(x, y, 0) ← (x, y)`.
Aucun autre voxel n'est écrit : l'intérieur de la boîte reste vide.
- Critère d'acceptation : TV-C1 : tuile pleine de couleur 1, `D = 4` : 12 faces (6 extérieures, 6 intérieures tournées vers l'intérieur de la boîte), boîte de 8 × 8 × 4.
- Notes : la face du dessous (rangée du bas de l'image) montre les rangées 0 à `D − 1` de la texture, la face du dessus les mêmes rangées dans l'ordre inverse ; les faces gauche et droite montrent les colonnes `D − 1` à 0 ; les faces avant et arrière montrent l'image entière. La boîte couvre toute l'image du pattern (`Wp × Hp`, cases vides comprises) ; les écritures qui tombent dans une case vide de la grille sont sans effet (REQ-E3D-421), si bien que la boîte est ouverte à l'endroit des cases vides.

**REQ-E3D-444** (P1) — `geo = VCYLINDER` (2), `HCYLINDER` (1), `HALFVCYLINDER` (5), `HALFHCYLINDER` (4) : **segments**. Pour les types `V`, le moteur DOIT parcourir chaque rangée `y` de l'image du pattern, `x` croissant, et y découper les **segments** maximaux de pixels consécutifs de valeur `≠ e` ; pour les types `H`, chaque colonne `x`, `y` croissant. Les rangées (types V) sont traitées par `y` croissant, les colonnes (types H) par `x` croissant, les segments d'une même ligne dans l'ordre de parcours. Chaque segment, de premier pixel `s` et dernier pixel `f` le long de la ligne (`L = f − s`), est remplacé par une **section elliptique** dans le plan (ligne, profondeur) contenant la ligne (REQ-E3D-445 à 451). Seule cette ligne de l'image reçoit des voxels.
- Critère d'acceptation : TV-Y1 : une rangée de 8 pixels en `VCYLINDER`, `D = 8` → anneau de 24 voxels dans la rangée `j = 7`.
- Notes : `VCYLINDER` produit des cylindres d'axe vertical (sections horizontales) ; `HCYLINDER` des cylindres d'axe horizontal.

**REQ-E3D-445** (P1) — **Paramètres de la section** (coordonnées doublées, en demi-pixels). Le moteur DOIT utiliser :
- types pleins (`VCYLINDER`, `HCYLINDER`) : centre le long de la ligne `C = s + f`, demi-axe `A = L`, demi-axe en profondeur `B = min(D − 1, L)` ;
- types demi (`HALF…`) : `A = 2L`, `B = min(D − 1, 2L)`, et le centre `C` vaut `2s` pour `HALFVCYLINDER` sans `flip` et pour `HALFHCYLINDER` avec `flip`, `2f` sinon ;
- dans tous les cas, centre en profondeur `Cz = D − 1`.
Les indices de voxels sont obtenus par `u⁺ = ⌊(C + p)/2⌋`, `u⁻ = C − u⁺` (le long de la ligne) et, en profondeur, `z⁺ = ⌊(Cz + q)/2⌋`, `z⁻ = Cz − z⁺` pour un point de contour `(p, q)` de région 1, `z⁺ = ⌊(Cz + q − 1)/2⌋`, `z⁻ = Cz − z⁺` pour un point de région 2 (REQ-E3D-446).
- Critère d'acceptation : TV-Y1 à TV-Y8.
- Notes : la section est centrée sur la tranche médiane `(D − 1)/2` ; un demi-cylindre a pour diamètre en profondeur jusqu'à `2L + 1` voxels, sa face plane (non fermée) étant à l'extrémité `s` ou `f` qui porte le centre.

**REQ-E3D-446** (P1) — **Contour de la section**. Le contour DOIT être la suite ordonnée de points entiers `(p, q)` suivante (`A, B > 0`) :
- *Région 1* : `(p₀, q₀) = (0, B)` ; pour passer d'un point au suivant, `p` augmente de 1 et `q` diminue de 1 si `B²·p(p − 1) + A²·q(q − 1) ≥ A²B²` (calculé avec le nouveau `p` et l'ancien `q`), sinon `q` est inchangé. Un point n'est retenu (et la région se poursuit) que tant que `B²·p ≤ A²·q`.
- *Région 2* : `(p₀, q₀) = (A, 0)` ; pour passer au suivant, `q` augmente de 1 et `p` diminue de 1 si `A²·q(q − 1) + B²·p(p − 1) > A²B²` (avec le nouveau `q` et l'ancien `p`), sinon `p` est inchangé. Un point n'est retenu que tant que `A²·q ≤ B²·p`.
L'ordre de traitement DOIT être : tous les points de la région 1 dans l'ordre, puis ceux de la région 2. Chaque point engendre quatre **points quadrants** : `Q0 = (u⁺, z⁺)`, `Q1 = (u⁺, z⁻)`, `Q2 = (u⁻, z⁺)`, `Q3 = (u⁻, z⁻)` (dans le plan ligne × profondeur, REQ-E3D-445), traités dans l'ordre 0, 1, 2, 3.
- Critère d'acceptation : `A = 7, B = 7` (rangée de 8, `D = 8`) : l'union des points quadrants est exactement l'anneau de TV-Y1.
- Notes : c'est un tracé d'ellipse par point milieu en coordonnées doublées ; les vecteurs TV-Y fixent le résultat attendu. L'**ordre** des points n'est normatif que pour la couleur des voxels écrits plusieurs fois (REQ-E3D-423) ; l'ensemble des voxels produits n'en dépend pas.

**REQ-E3D-447** (P1) — **Masque et texels des quadrants**. Les quadrants actifs DOIVENT être : tous pour les types pleins ; `Q0, Q1` (côté `u⁺`) pour `HALFVCYLINDER` sans `flip` et `HALFHCYLINDER` avec `flip` ; `Q2, Q3` (côté `u⁻`) pour `HALFVCYLINDER` avec `flip` et `HALFHCYLINDER` sans `flip`. Le texel affecté à un quadrant est, sur la ligne courante, la colonne (types V) ou la rangée (types H) :
- types pleins : `u⁻` pour `Q0` et `Q3`, `u⁺` pour `Q1` et `Q2` — la moitié avant (côté spectateur, `z⁻`) montre l'image à l'endroit, la moitié arrière la montre en miroir ;
- types demi : la position propre du quadrant (`u⁺` pour `Q0, Q1`, `u⁻` pour `Q2, Q3`).
Avec `antiShadow = vrai`, pour les points de **région 2** seulement, `u⁺` et `u⁻` sont remplacés dans ce choix de texel (pas dans la position des voxels) par `⌊(C + q)/2⌋` et `⌊(C − q + 1)/2⌋` respectivement.
- Critère d'acceptation : TV-Y1 (texels miroir à l'arrière) ; TV-Y6 (`antiShadow` : les voxels d'extrémité prennent les couleurs des colonnes 2 et 5 au lieu de 0 et 7).
- Notes : sans `antiShadow`, les flancs du cylindre (région 2, proches des extrémités du segment) prennent la couleur des pixels d'extrémité, souvent un contour sombre ; `antiShadow` les colore avec des pixels plus centraux.

**REQ-E3D-448** (P1) — **Contour seul** (`solid = faux`). Pour chaque point quadrant actif `(u, z)` dont le texel affecté a une valeur **différente de 0**, le moteur DOIT écrire sur tous les axes le voxel (coordonnées pattern) `(u, ligne, z)` pour les types V, `(ligne, u, z)` pour les types H, avec ce texel. `fullWarp` est sans effet.
- Critère d'acceptation : TV-Y1, TV-Y3, TV-Y7 (texels de valeur 0 ignorés même si `e ≠ 0`).

**REQ-E3D-449** (P1) — **Section pleine** (`solid = vrai`, `fullWarp = faux`). Pour chaque point du contour, dans l'ordre, le moteur DOIT tracer le segment discret allant du centre `M = (⌊C/2⌋, ⌊Cz/2⌋)` au point `Q0`, par l'algorithme de Bresenham classique : axe majeur = celui de plus grand écart, parcouru de l'extrémité de plus petite coordonnée majeure à l'autre, erreur initiale `⌊Δmaj/2⌋`, pas mineur quand l'erreur devient négative. Un point du segment n'est tracé que si `|m − mQ| / Δmaj < fillrate`, où `m` est sa coordonnée majeure, `mQ` celle de `Q0` et `Δmaj` l'écart majeur (`fillrate` = 1 par défaut : seul le centre est exclu ; si `Δmaj = 0`, rien n'est tracé). Chaque point tracé `(a, b)` engendre, pour chaque quadrant actif dont le texel affecté (REQ-E3D-447, celui du point de contour courant) est non nul, l'écriture sur tous les axes de `(a, b)`, `(a, Cz − b)`, `(C − a, b)`, `(C − a, Cz − b)` respectivement pour `Q0, Q1, Q2, Q3`.
- Critère d'acceptation : TV-Y2 : la forme (ensemble des voxels pleins) est exacte ; voir la note pour les couleurs.
- Notes : les écritures successives se recouvrent ; la couleur des voxels intérieurs et d'une partie de la surface dépend de cet ordre. La **forme** est normative (P1) ; la **couleur** des voxels non situés sur le contour DEVRAIT suivre exactement l'ordre décrit (P2) — un écart de couleur limité à ces voxels est classé D.

**REQ-E3D-450** (P1) — **Section pleine enveloppée** (`solid = vrai`, `fullWarp = vrai`). Pour chaque point du contour et pour chaque `z ∈ [z⁻, z⁺]` (bornes du point), le moteur DOIT écrire sur tous les axes : si `Q0` et `Q1` sont actifs, `(u⁺, z)` avec le texel de `Q2` ; si `Q2` et `Q3` sont actifs, `(u⁻, z)` avec le texel de `Q0`. Aucun test « texel non nul » n'est appliqué (seule la règle de transparence REQ-E3D-423 vaut).
- Critère d'acceptation : TV-Y4 : chaque colonne de la section est remplie avec la couleur de son propre pixel (types pleins), comme une extrusion découpée par l'ellipse.

**REQ-E3D-451** (P1) — **Cas dégénéré**. Si `A = 0` (segment d'un seul pixel) ou `B = 0` (en particulier `D = 1`), le moteur DOIT écrire un seul voxel, sur tous les axes, en position `(⌊C/2⌋, ⌊Cz/2⌋)` du plan de la section, avec le texel situé à cette position le long de la ligne (soumis seulement à REQ-E3D-423).
- Critère d'acceptation : TV-Y5 : rangée de 8 pixels, `VCYLINDER`, `D = 1` → un seul voxel, en colonne 3 ; pixel isolé, `D = 5` → un voxel en tranche `k = 2`.

**REQ-E3D-452** (P1) — **Profondeur effective des cylindres**. Après la construction d'un objet de type cylindrique, la profondeur utilisée pour le placement (REQ-E3D-403, 404, 408) et pour la déformation (REQ-E3D-568) DOIT devenir `D' = max (B + 1)` sur l'ensemble des segments construits (`D' = 0` s'il n'y en a aucun). Le champ de voxels et le centrage local des maillages restent ceux de la profondeur initiale `D` (REQ-E3D-504).
- Critère d'acceptation : TV-Y3 : colonne de 4 pixels, `HCYLINDER`, `sizeZ = 8` → `D' = 4` ; demi-cylindre d'un segment de 4 pixels, `sizeZ = 8` → `D' = 7`.
- Notes : la valeur `sizeZ` lue par un script après construction est `D'` (SPEC-05).

**REQ-E3D-453** (P2) — **Paramètres d'un pattern créé à l'exécution** (complément de REQ-E3D-161 et du tableau 4.3 de SPEC-04, qui font foi). Quand SPEC-04 crée un nouveau pattern (shape inconnue du profil), son objet 3D unique DOIT recevoir : `geo = Default`, `sizeZ = 3`, `layer = L1`, `scale = (1,1,1)`, `rot = (0,0,0)`, `offset = (0,0,0)`, `pivot = (0,5 ; 0,5 ; 0,5)`, `alpha = 1`, `rm = 0`, `renderStyle = 1`, `solid = flip = fullWarp = antiShadow = faux`, `emptyColor = 0`, `deltaAmp = (0,0,0)`, `deltaPivot = (0,5 ; 0,5 ; 0,5)`, `deltaSpeed` nul, `deltaOffset` = π/2 sur les lignes 0 à 2 et 0 sur la ligne 3, `customPalette = −1`, texture = image du pattern, aucune étiquette.
- Critère d'acceptation : une shape non décrite par le profil apparaît extrudée de 3 unités, dans la couche `L1`.

**REQ-E3D-454** (P3) — **Classifications 2D (informatives)**. Les classes de densité et de forme 2D d'une tuile sont celles de SPEC-04 REQ-E3D-021, qui fait foi (noms et seuils : EMPTY 0, LOW 1, AVERAGE 2, MEDIUM 3, HIGH 4, SOLID 5 ; FULL_RECT, CHAR, RECT, UNIDENTIFIED). En complément, pour une shape de `tw × th` cases dont `nt` cases occupées et `np` pixels non nuls : densité de tuiles = 5 si `nt = tw·th`, sinon `⌈4·nt/(tw·th)⌉` ; densité de pixels = 5 si `np = 64·tw·th`, sinon `⌈4·np/(64·tw·th)⌉`.
- Critère d'acceptation : shape de 2 × 1 cases dont une seule occupée, 30 pixels non nuls → densité de tuiles `⌈4·1/2⌉ = 2`, densité de pixels `⌈4·30/128⌉ = 1`.
- Notes : ces classes n'ont **aucun effet sur les maillages** ni sur la conversion en mode 3D (SPEC-04 REQ-E3D-021 et REQ-E3D-342) ; elles sont mémorisées dans les patterns (SPEC-03).

**REQ-E3D-455** (P1) — Avant la construction du champ de voxels d'un objet 3D, le moteur DOIT exécuter le hook Lua de construction des scripts liés à l'objet (SPEC-05) ; les paramètres modifiés par ce hook (géométrie, profondeur, options, ainsi que le paramètre d'exécution `fillrate` de REQ-E3D-449) DOIVENT être ceux utilisés pour la construction. `fillrate` n'est pas mémorisé dans le profil : seul ce hook permet de le fixer avant la construction.
- Critère d'acceptation : un script dont le hook de construction fixe `pattern.sizeZ = 8` produit des maillages de profondeur 8 pour les objets liés.

### 3.4 Maillage

**REQ-E3D-500** (P1) — L'algorithme de maillage effectif d'un objet DOIT être déterminé par `rm` : 1 → `GREEDY`, 2 → `MARCHING`, 3 → `GREEDY_MULTICOLOR`, toute autre valeur (dont 0) → l'algorithme choisi dans les réglages globaux (SPEC-09, valeur d'usine `GREEDY`).
- Critère d'acceptation : objet `rm = 0` : changer le réglage global change son rendu ; objet `rm = 2` : toujours lissé.

**REQ-E3D-501** (P1) — **Faces GREEDY**. Pour une tuile, un axe `a` et un sens `σ ∈ {−, +}`, une **cellule de face** existe entre le voxel `v` et son voisin `w = v + σ·1ₐ` si et seulement si la fente `a` de `v` est non vide et, soit `w` est hors de la tuile (hors de `[0,8) × [0,8) × [0,D)`), soit la fente `a` de `w` est vide. La cellule porte le texel de la fente `a` de `v`, la normale `σ·1ₐ`, et se situe sur le plan séparant `v` de `w`.
- Critère d'acceptation : TV-G1 : un voxel isolé produit exactement 6 cellules de face.
- Notes : les voxels des tuiles voisines **ne sont pas** consultés : chaque maillage de tuile est fermé sur lui-même (TV-G7, faces à la jonction de deux tuiles pleines).

**REQ-E3D-502** (P1) — **Partition en rectangles**. Les cellules de face de même axe, même sens et même plan DOIVENT être regroupées en rectangles selon la règle suivante, avec `u = axe (a+1) mod 3`, `v = axe (a+2) mod 3` (X = 0, Y = 1, Z = 2) : on parcourt les cellules du plan par `v` croissant puis `u` croissant ; à la première cellule non encore couverte, le rectangle s'étend d'abord le long de `u` sur la longueur maximale de cellules consécutives compatibles, puis le long de `v` tant que la rangée suivante, sur toute cette longueur, est compatible ; ses cellules sont alors couvertes. En `GREEDY`, deux cellules sont compatibles dès qu'elles existent toutes deux.
- Critère d'acceptation : TV-G2 (deux pixels de couleurs différentes, 6 rectangles), TV-G3 (tuile pleine, 6 rectangles), TV-G4 (forme en L, 10 rectangles).

**REQ-E3D-503** (P1) — **GREEDY_MULTICOLOR**. La règle de REQ-E3D-502 DOIT s'appliquer avec la compatibilité suivante : la valeur de texel de la cellule est égale à celle de la cellule de départ du rectangle.
- Critère d'acceptation : TV-G2 en multicolore → 10 rectangles ; TV-G8 (rayures de couleurs 1 et 2) → 34 rectangles au lieu de 6.
- Notes (P3, comportement hérité) : dans l'original, une position sans cellule compte comme une valeur 0 ; si la cellule de départ a la valeur 0 (possible seulement quand `e ≠ 0`), le rectangle peut donc s'étendre sur des positions sans cellule. Une implémentation PEUT ne pas reproduire ce cas ; l'écart est classé D.

**REQ-E3D-504** (P1) — **Positions GREEDY**. Les sommets d'un rectangle DOIVENT être ses quatre coins sur le plan, exprimés dans le repère local de la tuile : un coin de voxel d'indices entiers `(i, j, k)` (coin inférieur-gauche-avant du voxel `(i, j, k)`) a pour position locale `(i − 4, j − 4, k − D/2)`, où `D` est la profondeur utilisée pour construire le champ.
- Critère d'acceptation : TV-G1 : le voxel (0, 7, 0), `D = 1`, occupe `x ∈ [−4, −3]`, `y ∈ [3, 4]`, `z ∈ [−0,5 ; 0,5]`.

**REQ-E3D-505** (P1) — **Normales et orientation**. Chaque sommet d'un rectangle DOIT porter la normale unitaire de sa cellule (`±X`, `±Y`, `±Z`). Chaque rectangle DOIT être émis comme deux triangles `(A, B, C)` tels que `(B − A) × (C − A)`, calculé avec la formule usuelle du produit vectoriel sur les coordonnées du repère scène, ait le sens de la normale.
- Critère d'acceptation : pour tous les triangles de TV-G1 à TV-G8, le produit vectoriel est colinéaire de même sens que la normale.
- Notes : dans le repère main gauche de la scène, ces triangles apparaissent dans le sens horaire vus de l'extérieur (convention de face avant à fournir au moteur de rendu, SPEC-06). La découpe d'un rectangle en deux triangles est libre.

**REQ-E3D-506** (P1) — **Coordonnées de texture GREEDY**, en unités de texel (`(u, v) = (colonne, rangée)`, rangée 0 = haut de la texture ; un texel `(tx, ty)` couvre `[tx, tx+1] × [ty, ty+1]`). Soit un rectangle de `jw × kh` cellules dont les cellules de coin sont `cBL = (u₀, v₀)`, `cBR = (u₀ + jw − 1, v₀)`, `cTL = (u₀, v₀ + kh − 1)`, `cTR = (u₀ + jw − 1, v₀ + kh − 1)` (indices le long des axes `u`, `v` de REQ-E3D-502), de texels `tBL, tBR, tTL, tTR`, et dont les sommets géométriques sont `BL = (u₀, v₀)`, `BR = (u₀ + jw, v₀)`, `TL = (u₀, v₀ + kh)`, `TR = (u₀ + jw, v₀ + kh)`. Chaque sommet reçoit d'abord le texel de la cellule de coin de même nom, puis on DOIT ajouter 1 :
- à la composante colonne : si `tTR.x ≥ tBL.x`, à `TR` et à (`BR` si l'axe est Z, `TL` sinon) ; sinon à `BL` et à (`TL` si l'axe est Z, `BR` sinon) ;
- à la composante rangée : si `tTR.y ≥ tBL.y`, à `TR` et à (`TL` si l'axe est Z, `BR` sinon) ; sinon à `BL` et à (`BR` si l'axe est Z, `TL` sinon).
Les coordonnées sont ensuite interpolées linéairement sur le rectangle.
- Critère d'acceptation : TV-G1, TV-G4 (valeurs listées).
- Notes : quand l'affectation des texels est affine sur le rectangle (extrusion, faces avant), chaque point d'une face montre exactement le texel de la cellule située dessous. En `GREEDY`, la fusion ignore les couleurs : si l'affectation n'est pas affine (cylindres, faces enveloppées), le rendu montre l'interpolation entre les texels des coins ; ce comportement DOIT être reproduit.

**REQ-E3D-507** — *retirée* (v1.1, AMD-001). L'ordre d'émission des faces n'a pas d'effet observable ; REQ-E3D-523 compare les maillages indépendamment de l'ordre des faces. L'ordre de listage des vecteurs du §5 est une simple convention de présentation.

**REQ-E3D-520** (P2) — **MARCHING : grille duale**. Pour une tuile, le moteur DOIT considérer les cellules cubiques dont les 8 coins sont des centres de voxels `(i + α, j + β, k + γ)`, `α, β, γ ∈ {0, 1}`, pour `i ∈ [−1, 7]`, `j ∈ [−1, 7]`, `k ∈ [−1, D − 1]`. Un coin hors de la tuile DOIT être lu dans la tuile voisine du pattern à la position correspondante (affectation de cette tuile) ; hors de l'image du pattern, hors de `[0, D)` en profondeur ou sur une case vide, le coin est vide. Les cellules `i = −1` ne sont traitées que si la tuile est dans la colonne la plus à gauche du pattern. Pour une tuile présente plusieurs fois, le voisinage est lu autour de l'occurrence la plus proche du côté considéré (choix libre en cas d'égalité).
- Critère d'acceptation : TV-M3 : deux tuiles pleines adjacentes forment une surface continue sans paroi à leur jonction.
- Notes : les cellules `j = −1` et `j = 7` sont toujours traitées : la jonction verticale entre deux tuiles est générée par chacune d'elles (triangles en double, invisibles). Ces doublons PEUVENT être omis.

**REQ-E3D-521** (P2) — **MARCHING : surfaces par couleur**. Dans chaque cellule, la valeur d'un coin est la valeur de texel de son affectation, un coin vide valant 0. Pour chaque valeur `c ≠ e` présente parmi les coins, le moteur DOIT produire la surface séparant les coins de valeur `c` (intérieurs) des autres (extérieurs) selon la méthode des cubes marchants : un sommet sur chaque arête de la cellule joignant un coin intérieur à un coin extérieur, **placé exactement sur le coin extérieur**, et des triangles reliant ces sommets selon la triangulation classique des 256 configurations (Lorensen–Cline). Une cellule dont les 8 coins valent `c` ne produit rien.
- Critère d'acceptation : TV-M1 : un pixel isolé, `D = 1` → octaèdre régulier de « rayon » 1 centré sur le centre du voxel, 8 triangles. TV-M2 : deux pixels adjacents de même couleur → 16 triangles.
- Notes : chaque couleur a sa propre surface ; deux régions de couleurs différentes en contact produisent chacune une surface. Quand `e ≠ 0`, les coins vides comptent comme valeur 0 et engendrent eux aussi des surfaces (comportement hérité, P3).
- **Numérotation normative** (DEC-R2-521) : les coins de cellule DOIVENT être numérotés 0 à 7 dans l'ordre `(0,0,1), (1,0,1), (1,0,0), (0,0,0), (0,1,1), (1,1,1), (1,1,0), (0,1,0)` (en `(α, β, γ)`) et les arêtes dans l'ordre classique `0–1, 1–2, 2–3, 3–0, 4–5, 5–6, 6–7, 7–4, 0–4, 1–5, 2–6, 3–7` ; l'indice de configuration est `Σ 2^i` sur les coins intérieurs `i`, et la triangulation de chaque configuration est celle des tables publiques de Lorensen–Cline indexées ainsi. Cette numérotation rend reproductible la triangulation des configurations ambiguës et le choix du texel de REQ-E3D-522.

**REQ-E3D-522** (P2) — **MARCHING : attributs**. Les positions locales DOIVENT être `centre de voxel − (3,5 ; 3,5 ; (D − 1)/2)`, le centre du voxel `(i, j, k)` étant `(i, j, k)` en indices. Chaque triangle DOIT avoir trois sommets propres portant la normale plate `N = (B − A) × (C − A) / ‖(B − A) × (C − A)‖` orientée vers l'extérieur de la région de couleur `c` (règle d'orientation de REQ-E3D-505) ; un triangle d'aire nulle porte la normale (0, 0, 0) et PEUT être omis. Les coordonnées de texture des trois sommets DOIVENT être le centre `(tx + 0,5 ; ty + 0,5)` du texel affecté au coin intérieur de valeur `c` de **plus grand numéro** dans la numérotation normative de REQ-E3D-521.
- Critère d'acceptation : TV-M1 : normales `(±1, ±1, ±1)/√3`, toutes orientées vers l'extérieur.

**REQ-E3D-523** (P2) — **Tolérances de comparaison des maillages**. Deux maillages DOIVENT être considérés conformes si les ensembles de faces (positions des coins à 10⁻³ près, normale, coordonnées de texture à 10⁻³ texel près) sont identiques, indépendamment de l'ordre des faces, du partage des sommets et de la découpe des rectangles en triangles. En MARCHING, les triangles d'aire nulle et les doublons de REQ-E3D-520 sont exclus de la comparaison.
- Critère d'acceptation : les vecteurs du §5 sont vérifiés selon cette règle.
- Notes : l'original stocke positions, normales et coordonnées en flottants 16 bits ; toutes les valeurs produites (multiples de 0,5 de valeur absolue ≤ 48, coordonnées de texel ≤ 512) y sont exactes. L'usage de flottants 32 bits est conforme.

**REQ-E3D-524** (P2) — Le moteur DOIT accepter des maillages de tuile de plus de 65 535 sommets (index 32 bits ou découpage).
- Critère d'acceptation : test unitaire du mailleur sur un champ injecté en damier 3D (8 × 8 × 96, un voxel sur deux plein) : 18 432 rectangles (73 728 sommets), tous présents.
- Notes : l'original utilise des index 16 bits ; le dépassement (rarissime) y produit des faces erronées, qu'il n'est pas demandé de reproduire (classe D).

### 3.5 Shapes multi-tuiles et objets multiples

**REQ-E3D-540** (P1) — Pour une instance, le moteur DOIT dessiner, pour **chaque tuile** de la shape de la frame, le maillage de la tuile 2D correspondante de l'objet 3D de référence, placé selon REQ-E3D-405 et 406. Une tuile 2D qui apparaît plusieurs fois dans la shape est dessinée autant de fois, avec le même maillage.
- Critère d'acceptation : shape de 2 × 2 tuiles dont trois identiques : 4 tuiles dessinées, 2 maillages distincts.
- Notes : la géométrie d'une shape est donc l'assemblage des maillages de tuiles ; les tuiles ne sont pas fusionnées (sauf continuité de surface en MARCHING, REQ-E3D-520).

**REQ-E3D-541** (P1) — Quand un pattern porte plusieurs objets 3D, chaque instance de shape associée DOIT produire une instance par objet 3D, chacune avec ses propres paramètres, sa propre texture, son propre champ de voxels et ses propres maillages.
- Critère d'acceptation : un pattern à deux objets 3D (par exemple un `Default` en `L1` et un `CUBE` en `L2`) affiche deux géométries superposées pour chaque occurrence de la shape.

**REQ-E3D-542** (P1) — Les maillages DOIVENT dépendre uniquement de l'objet 3D de référence (paramètres au moment de la construction, texture, tuiles du pattern). Les modifications de paramètres d'une instance pendant une frame (scripts, SPEC-05) DOIVENT affecter le placement, la transformation, la couleur, l'alpha, les ombres et la déformation de cette instance, mais **pas** ses maillages.
- Critère d'acceptation : un script qui modifie `sizeZ` d'une instance déplace sa boîte de placement (REQ-E3D-403) sans changer l'épaisseur de ses maillages.

**REQ-E3D-543** (P1) — Une instance dont `renderStyle = 0` (objet désactivé) NE DOIT PAS être dessinée. Une instance dont la texture ou les maillages ne sont pas encore prêts (REQ-E3D-601) NE DOIT PAS être dessinée.
- Critère d'acceptation : `renderStyle = 0` → aucune géométrie ni ombre pour cet objet.

### 3.6 Texture, couleur, transparence, ombres, déformation

**REQ-E3D-560** (P1) — **Texture indexée**. La couleur affichée en un point d'une face DOIT être `NES_RGB[pal[4·P + (t mod 4)]]`, où `t` est la valeur du texel échantillonné, `pal` la palette de la frame affichée (REQ-E3D-561) et `P` la table : `customPalette` si `customPalette ≥ 0`, sinon `palette de la tuile + 4` pour une shape de sprite, `palette de la tuile` pour une shape de fond (la palette est celle de chaque tuile de la frame, SPEC-02). `NES_RGB` est la table de 64 couleurs RVB définie au §4.1 de SPEC-04 (DEC-04B-Q1).
- Critère d'acceptation : TV-K1 : texel 2 d'une tuile de fond de palette 1 → couleur `NES_RGB[pal[6]]` ; même tuile avec `customPalette = 9` → `NES_RGB[pal[38]]`.
- Notes : le bit supplémentaire des textures personnalisées n'influe pas sur la couleur (il ne sert qu'à la transparence et à la fusion multicolore).

**REQ-E3D-561** (P1) — (Rappel de REQ-E3D-250, SPEC-04, qui fait foi.) La palette de frame `pal` DOIT compter 16 tables de 4 entrées (64 indices de couleur NES), construites à partir des 32 octets de palette du cœur (SPEC-02) : `pal[i] = core[0]` si `i mod 4 = 0`, sinon `pal[i] = core[i mod 32]`. Les tables 8 à 15 reproduisent donc les tables 0 à 7, et l'entrée 0 de toute table est la couleur de fond universelle.
- Critère d'acceptation : `pal[36] = core[0]`, `pal[38] = core[6]`, `pal[22] = core[22]`.

**REQ-E3D-562** (P1) — **Texture directe**. La couleur affichée DOIT être la couleur RVB stockée dans le texel (opaque), sans palette ; un texel de valeur entière 0 DOIT afficher la couleur de fond universelle (`NES_RGB[core[0]]`) **telle qu'elle est au moment où la texture est chargée** dans le moteur de rendu (elle ne suit pas les changements ultérieurs de palette).
- Critère d'acceptation : texture directe avec un texel 0x00FF0000 → rouge pur, quelle que soit la palette ; changer la couleur de fond après le chargement ne modifie pas les texels 0 déjà chargés.

**REQ-E3D-563** (P1) — L'échantillonnage des textures DOIT être au plus proche voisin, sans filtrage ni mipmap. La coordonnée de texel échantillonnée en un point de coordonnées interpolées `(u, v)` DOIT être `(⌊u'⌋ mod tw_tex, ⌊v'⌋ mod th_tex)` avec `(u', v') = (u·texTilting.x + texOffset.x·tw_tex, v·texTilting.y + texOffset.y·th_tex)` ; valeurs par défaut `texTilting = (1, 1)`, `texOffset = (0, 0)`.
- Critère d'acceptation : `texOffset = (0,5 ; 0)` sur une texture de 16 texels de large décale l'image de 8 texels avec répétition ; `texTilting = (2, 1)` répète l'image deux fois en largeur.
- Notes : `texOffset` et `texTilting` sont des paramètres d'exécution (scripts), non mémorisés dans le profil. Le comportement en bord (répétition) est cohérent avec les lectures modulo de la construction ; voir Q-3.

**REQ-E3D-564** (P2) — Le moteur DOIT accepter, par objet 3D, des textures jusqu'à 512 × 512 texels, et une occupation totale simultanée d'au moins 64 × 512 × 512 texels pour l'ensemble des objets du profil chargé. Le comportement au-delà n'est pas spécifié mais NE DOIT PAS interrompre l'émulation.
- Critère d'acceptation : chargement des 101 profils embarqués sans erreur de capacité de texture.
- Notes : ces capacités correspondent à celles de la référence, qui interrompt le chargement au-delà ; une texture de plus de 512 texels de côté n'y est pas acceptée.

**REQ-E3D-565** (P1) — La résolution des couleurs indexées DOIT se faire **au rendu**, à chaque frame affichée : un changement de palette (palette de la frame ou `customPalette`) DOIT être visible à la frame affichée suivante sans reconstruction de maillage.
- Critère d'acceptation : un jeu qui fait clignoter sa palette (cycle de couleurs) fait changer la couleur des objets 3D à la même cadence que l'image 2D, sans indicateur de chargement.

**REQ-E3D-566** (P1) — **Alpha et style de rendu**. Une instance (hors couche UI) DOIT utiliser la classe de matériau **opaque** si `alpha = 1` ou `renderStyle = 2`, et **transparente** sinon, avec l'opacité `alpha` (0 à 1). Avec `renderStyle = 2`, l'instance NE DOIT PAS être visible mais DOIT projeter son ombre (classe « ombre seule »).
- Critère d'acceptation : en harnais, une instance d'`alpha = 0,5` apparaît dans la liste de REQ-E3D-641 avec la classe « transparente » et l'alpha 0,5 ; une instance de `renderStyle = 2` (quel que soit `alpha`) avec la classe « ombre seule » et le drapeau de projection vrai ; une instance d'`alpha = 1` et `renderStyle = 1` avec la classe « opaque ».

**REQ-E3D-567** (P1) — **Ombres**. Une instance DOIT projeter une ombre si et seulement si : les ombres sont activées globalement (SPEC-09), l'instance n'est pas en couche UI, sa classe de matériau est opaque ou ombre seule, ses étiquettes ne contiennent pas `!cast`, et la couche effective autorise la projection (réglage SPEC-09). Elle DOIT recevoir les ombres si et seulement si : ombres activées, pas en couche UI, étiquettes sans `!receiv` ni `!recv`, et la couche effective autorise la réception. Les autorisations par couche valent par défaut : `L1` projette et reçoit ; `L2`, `L3`, `L4` reçoivent sans projeter ; `UI` ni l'un ni l'autre.
- Critère d'acceptation : objet `L2` sans étiquette → reçoit l'ombre d'un objet `L1` mais n'en projette pas ; étiquette `!cast` sur un objet `L1` → plus d'ombre portée.
- Notes : seul l'état stable ci-dessus est exigé (DEC-04B-Q5) ; l'état particulier de la toute première exécution de l'original n'est pas reproduit **(D)**.

**REQ-E3D-568** (P2) — **Déformation animée**. Si `deltaAmp ≠ (0,0,0)`, chaque sommet `v` d'une tuile `τ` DOIT être déplacé de `δ`, avec :
- `sz = (pEnd.x − pStart.x + 1, pEnd.y − pStart.y + 1, D)` ;
- `Pd = deltaPivot ⊙ sz + (pStart.x, Hs − pEnd.y − h₀, −D/2)`, où `h₀ = 0` pour un profil de version 2 et `h₀ = 1` sinon (SPEC-03) ;
- `p = (tx + 4, Hs − ty − 4, 0) − Pd + v` et `nrm = p ⊘ sz` (division composante par composante) ;
- pour chaque composante `r ∈ {x, y, z}` (lignes 0, 1, 2 des matrices) : `δ_r = deltaAmp_r · sz_r · sin(nrm.x·Sp[r][0] + Of[r][0]) · sin(nrm.y·Sp[r][1] + Of[r][1]) · sin(nrm.z·Sp[r][2] + Of[r][2]) · sin(time·Sp[r][3] + Of[r][3])`, où `Sp = deltaSpeed`, `Of = deltaOffset` (matrices 4 × 4, `[ligne][colonne]`) et `time` le temps de la frame affichée en secondes (compteur de frames émulées / 60, SPEC-07).
Le déplacement `δ` est ajouté à `v` avant l'échelle et la rotation de l'objet (REQ-E3D-406).
- Critère d'acceptation : TV-D1 : `deltaAmp = (0, 0,1, 0)`, `deltaSpeed[1][0] = 2π`, autres vitesses nulles, décalages par défaut : chaque sommet monte de `0,1·sy·sin(2π·nrm.x + π/2)·1·1·sin(π/2)`, onde fixe d'une période sur la largeur de la shape.
- Notes : avec les valeurs par défaut (vitesses nulles, décalages π/2), `δ = deltaAmp ⊙ sz` : une translation constante. L'espace d'application (avant échelle et rotation) est fixé par DEC-04B-Q2. Le pivot `Pd` diffère du point `OriPos` exposé aux scripts (SPEC-05 REQ-LUA-083) : `OriPos − Pd = (−q.x, h₀ − q.y, 0)` avec `q = deltaPivot` ; cet écart est celui de l'original et DOIT être conservé (fidélité) : `GetDeform` ne prédit donc le déplacement rendu qu'à un décalage d'au plus un pixel près.

**REQ-E3D-569** (P3) — La ligne 3 des matrices `deltaSpeed` et `deltaOffset` (huit valeurs, exposées aux scripts sous le nom `CData[0..7]` : indices 0–3 = ligne 3 de `deltaSpeed`, 4–7 = ligne 3 de `deltaOffset`) NE DOIT PAS influer sur la géométrie ni sur la déformation ; elle DOIT être conservée et mémorisée avec l'objet 3D (SPEC-03, SPEC-05).
- Critère d'acceptation : modifier `CData[5]` ne change aucun sommet ; la valeur est relue identique après sauvegarde du profil.

### 3.7 Cycle de vie des maillages

**REQ-E3D-600** (P1) — Les maillages d'un objet 3D DOIVENT être construits : au chargement du profil, pour tous les objets 3D de tous ses patterns ; à la création d'un pattern pendant le jeu (REQ-E3D-453) ; et pour tous les objets dont l'algorithme effectif dépend du réglage global (en pratique, tous les objets de tous les patterns) quand ce réglage change. Pendant une construction de masse (chargement, changement d'algorithme), l'avancement exposé à l'indicateur de chargement (SPEC-07) DOIT valoir `(objets dont le champ et les maillages sont calculés + objets dont les maillages sont transmis au rendu) / (2 × objets à construire)`.
- Critère d'acceptation : au chargement d'un profil, la barre de progression va de 0 à 100 % et le jeu démarre quand tous les maillages sont prêts ; changer l'algorithme global relance une construction complète avec sa barre.

**REQ-E3D-601** (P1) — Un objet 3D dont les maillages ne sont pas encore prêts NE DOIT PAS être dessiné (REQ-E3D-543) ; sa shape reste reconnue et les scripts la voient. Pour un pattern créé pendant le jeu, les maillages DEVRAIENT être prêts au plus tard 2 frames émulées après sa création.
- Critère d'acceptation : une shape nouvelle apparaît en 3D dans les 2 frames suivant sa première détection.
- Notes : dans la référence, une apparition retardée d'une ou deux frames est observable ; ce retard est toléré.

**REQ-E3D-602** (P1) — Aucun changement de palette, de position, de transformation, d'alpha, de couche, d'étiquette d'ombre ou de déformation NE DOIT provoquer de reconstruction de maillage (REQ-E3D-542, 565).
- Critère d'acceptation : en jeu, sans création de pattern, l'indicateur de chargement reste inactif et aucun maillage n'est reconstruit (compteur de constructions inchangé).

**REQ-E3D-603** (P2) — Quand un pattern est retiré (SPEC-04 : pattern temporaire expiré, déchargement du profil), ses maillages et l'espace de texture de ses objets DOIVENT être libérés. Le nombre d'objets et de tuiles dessinés par frame NE DOIT PAS être limité par une capacité fixe.
- Critère d'acceptation : une frame de 1 500 shapes instanciées sur 3 000 tuiles se dessine entièrement.

### 3.8 Contrat de sortie vers le rendu

**REQ-E3D-640** (P1) — Le moteur 3D DOIT exposer, pour chaque maillage de tuile, une description abstraite indépendante de la plateforme : liste de sommets (position locale en float, normale, coordonnées de texture en unités de texel de la texture de l'objet), liste de triangles (indices), une seule sous-partie, topologie triangles, boîte englobante locale. La couleur n'est pas portée par les sommets.
- Critère d'acceptation : le harnais headless (PLAN, `Sen.Harness`) exporte ces descriptions et les compare aux vecteurs du §5 selon REQ-E3D-523.

**REQ-E3D-641** (P1) — Pour chaque frame affichée, le moteur 3D DOIT produire la liste ordonnée des **tuiles à dessiner**, chacune avec : référence du maillage ; matrice de transformation complète (REQ-E3D-406, hors transformation de scène) ; conteneur (jeu ou interface) ; classe de matériau (REQ-E3D-642) ; alpha ; référence de la texture de l'objet et son format (indexé ou direct) ; table de palette `P` (REQ-E3D-560, sans objet en direct) ; `texOffset` et `texTilting` ; drapeaux projeter / recevoir l'ombre (REQ-E3D-567) ; indicateurs « tuile de sprite » et « priorité derrière le fond » (décalage `n` appliqué par le rendu, REQ-E3D-405) ; si la déformation est active, les paramètres `deltaAmp`, `deltaSpeed`, `deltaOffset`, `sz` et la position de la tuile relative au pivot de déformation (`(tx + 4, Hs − ty − 4, 0) − Pd`, REQ-E3D-568). La palette de frame (64 couleurs RVB résolues) et le temps de frame sont fournis une fois par frame.
- Critère d'acceptation : pour TV-W1 à TV-W3, la liste exportée contient les valeurs attendues.
- Notes : la déformation peut être appliquée par le moteur de rendu (shader) ou par le moteur 3D ; le résultat doit être celui de REQ-E3D-568.

**REQ-E3D-642** (P1) — Les classes de matériau DOIVENT être : **opaque** ; **transparente** (mélange alpha, pas d'ombre portée) ; **interface** (couche UI) ; **ombre seule** (invisible, projette l'ombre). Leur aspect (éclairage, shaders) relève de SPEC-06.
- Critère d'acceptation : chaque tuile de la liste de REQ-E3D-641 porte exactement une de ces quatre classes.

**REQ-E3D-643** (P2) — Le découpage de l'image par les marges réglables (SPEC-06, SPEC-09) DOIT être appliqué par le rendu dans le plan XY de la scène, après transformation, sur la base du rectangle visible `R` de REQ-E3D-410 ; le moteur 3D ne modifie pas les maillages pour cela.
- Critère d'acceptation : en harnais, les transformations et maillages exportés (REQ-E3D-640, 641) sont identiques pour deux réglages de marges différents ; dans la capture hors écran, un objet à cheval sur le bord gauche de `R` (REQ-E3D-410) est tranché net dans le plan `x = c0` (SPEC-06 REQ-SCN-036 et REQ-SCN-039).

## 4. Formats et interfaces

### 4.1 Paramètres d'un objet 3D utilisés par la géométrie

Les numéros de champ renvoient au message « objet 3D » de SPEC-03, qui fait foi.

| Paramètre (nom Lua) | Champ | Type | Effet (ce document) |
|---|---|---|---|
| `sizeZ` | 1 | entier 0–96 | profondeur `D` (3.2, 3.3) ; remplacée par `D'` après un cylindre |
| `geo` | 2 | `Geo3DType` : 0 `Default`, 1 `HCYLINDER`, 2 `VCYLINDER`, 3 `CUBE`, 4 `HALFHCYLINDER`, 5 `HALFVCYLINDER` | règle de construction (3.3) |
| `antiShadow` | 3 | booléen | texels de région 2 des cylindres (REQ-E3D-447) |
| `flip` | 4 | booléen | côté des demi-cylindres (REQ-E3D-445, 447) |
| `solid` | 5 | booléen | section pleine des cylindres (REQ-E3D-449, 450) |
| `fullWarp` | 6 | booléen | extrusion enveloppée (REQ-E3D-441, 442), remplissage par colonne des cylindres (REQ-E3D-450) |
| `layer` | 7 | `ZLayer` 0–5 | couche (REQ-E3D-402, 403) |
| `scale` | 8 | vecteur 3 | échelle `S` |
| `rot` | 9 | vecteur 3 (degrés) | rotation `R` |
| `offset` | 10 | vecteur 3 | décalage de l'ancrage |
| `pivot` | 11 | vecteur 3 (fractions de la boîte) | ancrage `Pos` |
| `alpha` | 12 | flottant 0–1 | opacité, classe de matériau |
| `rm` | 13 | octet | algorithme de maillage (REQ-E3D-500) |
| `deltaAmp` | 14 | vecteur 3 | amplitude de déformation |
| `deltaPivot` | 15 | vecteur 3 | pivot de déformation |
| `deltaSpeed` | 16 | matrice 4 × 4 | vitesses de déformation (lignes 0–2) ; ligne 3 = `CData[0..3]` |
| `deltaOffset` | 17 | matrice 4 × 4 | phases de déformation (lignes 0–2) ; ligne 3 = `CData[4..7]` |
| `renderStyle` | 18 | entier | 0 désactivé, 1 normal, 2 ombre seule |
| texture | 19 | texture (SPEC-03) | texture personnalisée ; taille nulle = image du pattern |
| `TagStr` | 20 | chaîne | étiquettes `!cast`, `!receiv`/`!recv`, `_gaxis` (et liens de scripts, SPEC-05) |
| `emptyColor` | 23 | entier signé | couleur transparente (REQ-E3D-424) |
| `customPalette`, `texOffset`, `texTilting`, `fillrate` | — | exécution seulement | REQ-E3D-560, 563, 449 (défauts −1, (0,0), (1,1), 1) |

### 4.2 Description abstraite d'un maillage (sortie)

```
MeshTuile {
  sommets : liste de { position : (x, y, z) float, normale : (nx, ny, nz) float, uv : (u, v) float en texels }
  triangles : liste de (a, b, c) indices, orientés selon REQ-E3D-505
  boite : (min, max) locale
}
TuileADessiner {
  maillage, transformation 4×4, conteneur ∈ {jeu, interface},
  materiau ∈ {opaque, transparent, interface, ombreSeule}, alpha,
  texture (référence, format ∈ {indexé, direct}, tw_tex, th_tex), tablePalette P (0–15 ou aucune),
  texOffset, texTilting, projetteOmbre, recoitOmbre, sprite, derriereFond,
  deformation? { amp, vitesses 4×4, phases 4×4, sz, posTuileRelPivot }
}
Frame3D { tuiles : liste de TuileADessiner, paletteRVB : 64 couleurs, temps : secondes }
```

## 5. Vecteurs de test

Conventions : image de tuile donnée en 8 rangées (rangée 0 en haut), `.` = 0, chiffres = valeur ; sauf mention, pattern d'une seule tuile, `emptyColor = 0`, `GREEDY`, texture = image du pattern. Les positions sont locales à la tuile (REQ-E3D-504, 522). Un rectangle est noté `normale : [sommet (position) uv]` dans l'ordre BL, TL, BR, TR. Les comptes de faces (« rectangles ») sont ceux de REQ-E3D-502 ; un rectangle = 4 sommets, 2 triangles.

### 5.1 Extrusion (`Default`)

**TV-G1** — Pixel (0, 0) = 1, reste 0, `D = 1` → 6 rectangles, boîte `x ∈ [−4, −3]`, `y ∈ [3, 4]`, `z ∈ [−0,5 ; 0,5]` :
- `−X` : (−4, 3, −0,5) (0,0) ; (−4, 3, 0,5) (1,0) ; (−4, 4, −0,5) (0,1) ; (−4, 4, 0,5) (1,1)
- `−Y` : (−4, 3, −0,5) (0,0) ; (−3, 3, −0,5) (1,0) ; (−4, 3, 0,5) (0,1) ; (−3, 3, 0,5) (1,1)
- `−Z` : (−4, 3, −0,5) (0,0) ; (−4, 4, −0,5) (0,1) ; (−3, 3, −0,5) (1,0) ; (−3, 4, −0,5) (1,1)
- `+X` : (−3, 3, −0,5) (0,0) ; (−3, 3, 0,5) (1,0) ; (−3, 4, −0,5) (0,1) ; (−3, 4, 0,5) (1,1)
- `+Y` : (−4, 4, −0,5) (0,0) ; (−3, 4, −0,5) (1,0) ; (−4, 4, 0,5) (0,1) ; (−3, 4, 0,5) (1,1)
- `+Z` : (−4, 3, 0,5) (0,0) ; (−4, 4, 0,5) (0,1) ; (−3, 3, 0,5) (1,0) ; (−3, 4, 0,5) (1,1)
Même pixel, `D = 4` : mêmes 6 rectangles, `z ∈ [−2, 2]`.

**TV-G2** — Rangée 0 = `12......`, `D = 2` :
- `GREEDY` : 6 rectangles, boîte `x ∈ [−4, −2]`, `y ∈ [3, 4]`, `z ∈ [−1, 1]` ; rectangle `−Z` : (−4,3,−1) (0,0) ; (−4,4,−1) (0,1) ; (−2,3,−1) (2,0) ; (−2,4,−1) (2,1) (les deux couleurs sur une seule face, uv interpolées) ; `+X` : (−2,3,−1) (1,0) ; (−2,3,1) (2,0) ; (−2,4,−1) (1,1) ; (−2,4,1) (2,1).
- `GREEDY_MULTICOLOR` : 10 rectangles (1 par face `±X`, 2 par face `±Y`, `±Z`) ; `−Z` : (−4,3,−1)…(−3,4,−1) uv (0,0)–(1,1) puis (−3,3,−1)…(−2,4,−1) uv (1,0)–(2,1).

**TV-G3** — Tuile pleine de 1, `D = 8` → 6 rectangles, boîte `[−4, 4]³`. Avec `D = 2` et rangées `12121212` : 6 rectangles en `GREEDY`.

**TV-G4** — Forme en L (rangée 0 = `11......`, rangée 1 = `1.......`), `D = 1` → 10 rectangles (`−X` 1, `−Y` 2, `−Z` 2, `+X` 2, `+Y` 1, `+Z` 2). Rectangle `−X` : (−4, 2, −0,5) (0,2) ; (−4, 2, 0,5) (1,2) ; (−4, 4, −0,5) (0,0) ; (−4, 4, 0,5) (1,0). Rectangle `+Y` : (−4, 4, −0,5) (0,0) ; (−2, 4, −0,5) (2,0) ; (−4, 4, 0,5) (0,1) ; (−2, 4, 0,5) (2,1).

**TV-G5** — Rangée 0 = `10......` (le reste 0), `emptyColor = 4`, `D = 1` → toute la tuile est pleine : 6 rectangles, boîte `[−4, 4] × [−4, 4] × [−0,5 ; 0,5]`.

**TV-G6** — `fullWarp`, pixel (0, 0) = 1, reste 0, `D = 2` → 6 rectangles : `±Z` couvrent `z = ∓1` sur `x ∈ [−4, −3]`, `y ∈ [3, 4]` ; `±X` et `±Y` couvrent seulement `z ∈ [0, 1]` (exemple `−X` : (−4,3,0) (0,0) ; (−4,3,1) (1,0) ; (−4,4,0) (0,1) ; (−4,4,1) (1,1)).

**TV-G7** — Pattern 2 × 1 de deux tuiles pleines **différentes** (couleurs distinctes), `D = 1` → chaque tuile : 6 rectangles, boîte `[−4, 4]² × [−0,5 ; 0,5]` (les faces `+X` de la tuile gauche et `−X` de la tuile droite existent toutes deux).

**TV-G8** — Rangées `12121212` × 8, `D = 2`, `GREEDY_MULTICOLOR` → 34 rectangles (`±X` : 1 chacune ; `±Y`, `±Z` : 8 chacune).

### 5.2 Boîte (`CUBE`)

**TV-C1** — Tuile pleine de 1, `D = 4` → 12 rectangles (2 par normale), boîte `[−4, 4] × [−4, 4] × [−2, 2]`.

**TV-C2** — Anneau (rangées 0 et 7 pleines, colonnes 0 et 7 pleines, intérieur 0), `D = 4` → 40 rectangles (`±X` : 6 chacune ; `±Y` : 6 chacune ; `±Z` : 8 chacune).

**TV-C3** — Pattern 2 × 1, `D = 2`, tuiles pleines : même tuile 2D dans les deux cases → 1 maillage de 10 rectangles (`±X` : 2 chacune, `±Y` : 2 chacune, `±Z` : 1 chacune) ; tuiles différentes → 2 maillages de 8 rectangles (`±X` : 1, `±Y` : 2, `±Z` : 1).

**TV-C4** — Rangée 0 = `11111111`, reste 0, `D = 2` → 10 rectangles :
- `−X` (x = −4) et `+X` (x = −3) : `y ∈ [3, 4]`, `z ∈ [−1, 1]`, uv (2,0),(0,0),(2,1),(0,1) dans l'ordre BL, TL, BR, TR ;
- `−X` (x = 3) et `+X` (x = 4) : idem ;
- `−Y` (y = −4) et `+Y` (y = −3) : `x ∈ [−4, 4]`, `z ∈ [−1, 0]` (dessous : seule la rangée 0 de texture est opaque) ;
- `−Y` (y = 3) et `+Y` (y = 4) : `x ∈ [−4, 4]`, `z ∈ [0, 1]` (dessus) ;
- `−Z` (z = −1) et `+Z` (z = 1) : `x ∈ [−4, 4]`, `y ∈ [3, 4]`.

### 5.3 Cylindres

Les voxels sont donnés dans le plan de la section, en `(i, k)` pour les types V (rangée `j` fixée) et `(j, k)` pour les types H (colonne `i` fixée), avec le texel affecté (colonne pour V, rangée pour H). Toutes les fentes d'un voxel portent le même texel.

**TV-Y1** — Rangée 0 = `11111111`, `VCYLINDER`, `D = 8`, contour (`solid = faux`, `fullWarp` indifférent). Voxels de la rangée `j = 7` → texel : (0,3)→0, (0,4)→7, (1,1)→1, (1,2)→1, (1,5)→6, (1,6)→6, (2,0)→2, (2,1)→2, (2,6)→5, (2,7)→5, (3,0)→3, (3,7)→4, (4,0)→4, (4,7)→3, (5,0)→5, (5,1)→5, (5,6)→2, (5,7)→2, (6,1)→6, (6,2)→6, (6,5)→1, (6,6)→1, (7,3)→7, (7,4)→0. 24 voxels ; maillage `GREEDY` : 64 rectangles ; `D' = 8`.

**TV-Y2** — Même rangée, `solid = vrai`, `fullWarp = faux` → 48 voxels : `i = 0` et `i = 7` : `k ∈ {3, 4}` ; `i = 1` et `i = 6` : `k ∈ [1, 6]` ; `i = 2…5` : `k ∈ [0, 7]`. Maillage : 30 rectangles. Texels de contrôle (ordre REQ-E3D-449) : (1,3)→0, (1,4)→7, (3,1)→2, (4,1)→5, (2,2)→1.

**TV-Y3** — Colonne 0, rangées 0 à 3 = 1, reste 0, `HCYLINDER`, `sizeZ = 8` → voxels de la colonne `i = 0`, `(j, k)` → texel (rangée) : (4,3)→3, (4,4)→0, (5,2)→2, (5,5)→1, (6,2)→1, (6,5)→2, (7,3)→0, (7,4)→3 ; 24 rectangles ; `D' = 4`. Colonne pleine de 8, `sizeZ = 8` : 24 voxels symétriques de TV-Y1, 64 rectangles. Colonne pleine de 8, `sizeZ = 4` : `(j, k)` = (0,1)→0, (0,2)→7, (1,1)→6, (1,2)→1, (2,0)→5, (2,3)→2, (3,0)→4, (3,3)→3, (4,0)→3, (4,3)→4, (5,0)→2, (5,3)→5, (6,1)→1, (6,2)→6, (7,1)→7, (7,2)→0 ; `D' = 4`.

**TV-Y4** — Rangée `11111111`, `VCYLINDER`, `D = 8`, `solid = fullWarp = vrai` → mêmes 48 voxels que TV-Y2, chaque voxel `(i, k)` portant le texel `i`. Rangée `1123....`, `D = 4`, `solid = fullWarp = vrai` → voxels (0,1), (0,2) texel 0 ; (1,0…3) texel 1 ; (2,0…3) texel 2 ; (3,1), (3,2) texel 3 ; 18 rectangles.

**TV-Y5** — Rangée `11111111`, `VCYLINDER`, `D = 1` → un seul voxel (3, 0), texel 3 ; boîte locale `x ∈ [−1, 0]`. Pixel isolé (0, 0) = 1, `VCYLINDER`, `D = 5` → voxel (0, 2), texel 0 ; `D' = 1`.

**TV-Y6** — Rangée `12333321`, `VCYLINDER`, `D = 8`, contour : mêmes positions que TV-Y1 et mêmes texels ; avec `antiShadow = vrai` : positions identiques, texels modifiés pour les voxels de région 2 : (0,3)→2, (0,4)→5, (7,3)→5, (7,4)→2 (les autres inchangés).

**TV-Y7** — Rangée `1101....`, `emptyColor = 4`, `VCYLINDER`, `D = 4`, contour → le segment couvre les 8 pixels (aucun n'est transparent), mais seuls les points quadrants de texel non nul sont écrits : (0,2)→0, (1,1)→1, (3,0)→3, (4,3)→3, (6,2)→1, (7,1)→0.

**TV-Y8** — Demi-cylindres, segment de 4 pixels (`1111....` en rangée 0, ou colonne 0 rangées 0 à 3), `sizeZ = 8`, contour, `D' = 7` :
- `HALFVCYLINDER` : (0,1)→0, (0,6)→0, (1,1)→1, (1,6)→1, (2,1)→2, (2,2)→2, (2,5)→2, (2,6)→2, (3,3)→3, (3,4)→3 (face plane ouverte en `i = 0`) ;
- `HALFVCYLINDER`, `flip` : (0,3)→0, (0,4)→0, (1,1)→1, (1,2)→1, (1,5)→1, (1,6)→1, (2,1)→2, (2,6)→2, (3,1)→3, (3,6)→3 (face plane en `i = 3`) ;
- `HALFHCYLINDER` (colonne) : `(j, k)` = (4,1)→3, (4,6)→3, (5,1)→2, (5,6)→2, (6,1)→1, (6,2)→1, (6,5)→1, (6,6)→1, (7,3)→0, (7,4)→0 (face plane en bas, rangée 3 de l'image, `j = 4`).
Maillages : 26 rectangles chacun.

Autres cas de contrôle : rangée `111.....`, `VCYLINDER`, `D = 8` → voxels (0,3)→0, (0,4)→2, (1,3)→1, (1,4)→1, (2,3)→2, (2,4)→0, `D' = 3` ; rangée `11111111`, `D = 2` → 16 voxels (`k = 0` : texel `i` ; `k = 1` : texel `7 − i`), 6 rectangles.

### 5.4 Surfaces lissées (`MARCHING`)

**TV-M1** — Pixel (0, 0) = 1, `D = 1` → 8 triangles, 24 sommets, octaèdre centré en (−3,5 ; 3,5 ; 0) de sommets (−4,5 ; 3,5 ; 0), (−2,5 ; 3,5 ; 0), (−3,5 ; 2,5 ; 0), (−3,5 ; 4,5 ; 0), (−3,5 ; 3,5 ; ±1) ; normales `(±1, ±1, ±1)/√3` vers l'extérieur ; uv (0,5 ; 0,5) pour tous les sommets. Exemple de triangle : (−4,5 ; 3,5 ; 0), (−3,5 ; 3,5 ; −1), (−3,5 ; 2,5 ; 0), normale `(−1, −1, −1)/√3`.

**TV-M2** — Rangée 0 = `11......`, `D = 1` → 16 triangles, boîte `x ∈ [−4,5 ; −1,5]`, `y ∈ [2,5 ; 4,5]`, `z ∈ [−1, 1]`.

**TV-M3** — Tuile pleine de 1, `D = 2` → 380 triangles, boîte `[−4,5 ; 4,5]² × [−1,5 ; 1,5]`. Deux tuiles pleines adjacentes horizontalement : aucune face dans le plan de jonction `x = 4` de la tuile gauche.

**TV-M4** — Pixel (0, 0) = 1, `emptyColor = 4`, `D = 1` → 16 triangles dont 8 d'aire nulle (comportement hérité de REQ-E3D-521 ; seuls les 8 triangles non dégénérés sont comparés).

### 5.5 Placement, couleur, déformation

**TV-W1** — Shape de fond d'une tuile pleine, coin (16, 32), `D = 4`, `L1 FRONT`, paramètres par défaut, `GREEDY` → `Pos = (20, 212, 2)`, `T = (20, 212, 2)` ; boîte scène `x ∈ [16, 24]`, `y ∈ [208, 216]`, `z ∈ [0, 4]`. En `L2` : `z ∈ [12, 16]`. Couche alignée `CENTER` (décalage 0) : `z ∈ [−2, 2]` ; `BACK` : `z ∈ [−4, 0]`.

**TV-W2** — Même tuile, shape de sprite → transformation livrée par le moteur 3D identique à TV-W1 (`z ∈ [0, 4]`), indicateur « sprite » vrai ; boîte affichée après le décalage du rendu (REQ-SCN-037) : `z ∈ [−0,1 ; 3,9]` ; sprite à priorité « derrière le fond » → `z ∈ [0,1 ; 4,1]`.

**TV-W3** — TV-W1 avec `scale = (2, 1, 1)` → `x ∈ [12, 28]` ; avec en plus `pivot = (0, 0, 0)` → `Pos = (16, 208, 0)`, `x ∈ [16, 32]`, `y ∈ [208, 216]`, `z ∈ [0, 4]` ; TV-W1 avec `rot = (0, 0, 90)` → la tuile reste dans `x ∈ [16, 24]`, `y ∈ [208, 216]` (carré), son pixel (0, 0) passe du coin supérieur gauche au coin inférieur gauche.

**TV-W4** — TV-W1 en `MARCHING`, pixel isolé au lieu de tuile pleine (shape 1 × 1 pixel en (16, 32)) : `S = (0,5 ; 0,5 ; 0,5)` ; octaèdre de rayon 0,5 centré en (16,5 ; 215,5 ; 0,5).

**TV-K1** — Palette du cœur `core[0..31]` = 0x0F, 0x01…0x1F (valeurs quelconques distinctes) ; tuile de fond de palette 1, texel 2 → `NES_RGB[core[6]]` ; même tuile de sprite (palette 1) → `NES_RGB[core[22]]` ; texel 0 → `NES_RGB[core[0]]` dans tous les cas ; `customPalette = 9`, texel 2 → `NES_RGB[pal[38]] = NES_RGB[core[6]]`.

**TV-D1** — Shape 16 × 16 pixels de coin (0, 0), `D = 4`, profil version 2, `deltaPivot = (0,5 ; 0,5 ; 0,5)`, `deltaAmp = (0 ; 0,1 ; 0)`, `deltaSpeed[1][0] = 2π`, autres vitesses 0, `deltaOffset` par défaut, `time = 0` → `sz = (16, 16, 4)`, `Pd = (8, 241, 0)` ; un sommet dont `nrm.x = 0,25` est déplacé de `δy = 0,1 × 16 × sin(2π·0,25 + π/2) = 0` ; à `nrm.x = 0` : `δy = 1,6` ; à `nrm.x = 0,5` : `δy = −1,6`.

## 6. Questions ouvertes

- **Q-1** — *close par DEC-04B-Q1* : la table `NES_RGB` est portée par SPEC-04 (§4.1).
- **Q-2** — *close par DEC-04B-Q2* : l'espace d'application de la déformation n'a pas pu être déterminé en boîte noire ; REQ-E3D-568 retient « avant échelle et rotation ». L'écart éventuel n'est visible que si `scale ≠ 1` ou `rot ≠ 0` sur un objet déformé ; vérification différentielle au round 2.
- **Q-3** — *close par DEC-04B-Q3* : texture répétée hors de ses bornes (modulo dans son emplacement, REQ-E3D-563).
- **Q-4** — *close par DEC-04B-Q4* : découpage dans le plan XY de la scène, avant la caméra (REQ-E3D-643, REQ-E3D-410).
- **Q-5** — *close par DEC-04B-Q5* : état stable seul, écart classé D (REQ-E3D-567).
- **Q-6** — *close par DEC-04B-Q6* (non obligatoires, P3, D) : les deux cas hérités de couleur 0 (`GREEDY_MULTICOLOR` avec `e ≠ 0`, REQ-E3D-503 ; MARCHING avec `e ≠ 0`, REQ-E3D-521) sont-ils à reproduire ? Proposition : non obligatoire (P3, classe D), sauf si un profil embarqué en dépend visiblement (aucun objet embarqué n'a `rm = 3` ; 11 047 ont `emptyColor = 4`, dont quelques-uns en `rm = 2`).

## Annexe A — Usage des types de géométrie dans les 101 profils embarqués

Décompte, côté sale, de tous les objets 3D des patterns permanents et temporaires des 101 profils (`assets-neutral/profiles`). 67 832 patterns, 74 389 objets 3D ; 5 182 patterns portent plusieurs objets 3D. Version de format : 100 profils en version 2, 1 en version 3.

| `geo` | Objets | Profils | Exemples de profils (plus grand nombre d'objets) |
|---|---|---|---|
| `Default` | 63 633 | 100 | Final_Fantasy_USA, Super Spike V Ball (U), Fire 'n Ice (U) |
| `VCYLINDER` | 7 121 | 99 | Rockman 3, Super Mario Bros 3 (U), Felix the Cat |
| `HCYLINDER` | 2 373 | 87 | Rockman 3, Felix the Cat, Life Force (U) |
| `CUBE` | 872 | 64 | Super Mario Bros 3 (U), Adventures of Lolo 3 (U), Duck Tales (U) |
| `HALFHCYLINDER` | 246 | 36 | gradius, Blaster Master (U), Felix the Cat |
| `HALFVCYLINDER` | 144 | 30 | Ninja Gaiden III, Life Force (U), Jackal (U) |

| Option | Objets | Profils | Remarques / exemples |
|---|---|---|---|
| `solid` | 13 475 | 98 | dont `VCYLINDER` 6 439, `HCYLINDER` 1 957, `Default` 4 604 (sans effet), `CUBE` 106 (sans effet) |
| `fullWarp` | 10 518 | 83 | dont `Default` 3 380 (REQ-E3D-441), `VCYLINDER` 5 092 ; Life Force (U), Blaster Master (U) |
| `antiShadow` | 2 568 | 69 | effectif sur cylindres : 1 856 ; Blaster Master (U), Super Mario Bros 3 (U) |
| `flip` | 311 | 38 | effectif sur demi-cylindres : 140 ; Blaster Master (U), Micro Mages |
| `rm` = 0 / 1 / 2 / 3 | 73 356 / 400 / 633 / 0 | — / 22 / 24 / 0 | `MARCHING` : Quarth, Spy Hunter (U), Super Bat Puncher Demo ; `GREEDY` forcé : Road Fighter (Japan), Legend Of Zelda |
| `emptyColor` = 0 / 1 / 2 / 3 / 4 | 62 717 / 271 / 109 / 245 / 11 047 | — / 21 / 17 / 26 / 90 | `e = 4` : Adventures of Lolo 3 (U), Rockman 3, Ninja Gaiden III |
| texture personnalisée indexée / directe | 13 539 / 142 | — / 34 | directe : Lunar Pool, Blaster Master (U), Depths ; 4 textures de taille différente du pattern (Excitebike, Mega Man 1, Urban Champion, Yie Ar Kung-Fu) |
| déformation (`deltaAmp ≠ 0`) | 289 | 45 | Jackal (U), Ninja Gaiden III, Spy Hunter (U) |
| `alpha < 1` | 628 | 46 | Felix the Cat, Super Mario Bros 3 (U), Fire 'n Ice (U) |
| `renderStyle` = 0 / 1 / 2 | 1 639 / 72 744 / 6 | — | ombre seule : Chip 'n Dale Rescue Rangers (U), Fire 'n Ice (U) |
| couche `UNIDENTIFIED` / `UI` / `L1` / `L2` / `L3` / `L4` | 30 762 / 156 / 32 456 / 9 534 / 1 318 / 163 | — / 23 / — | `UI` : pooyan, flappy_tpv, Shatterhand |
| rotation non nulle / échelle ≠ 1 / décalage ≠ 0 | 3 248 / 784 / 15 080 | — | |
| étiquettes `!cast` / `!receiv` / `!recv` / `_gaxis` | 980 / 160 / 406 / 42 | — / — / — / 7 | `_gaxis` : City Connection, Spy Hunter (U), Donkey Kong |

Répartition de `sizeZ` (objets) : 0 : 51 ; 1 : 2 007 ; 2 : 26 398 ; 3 : 13 084 ; 4 : 10 589 ; 8 : 5 895 ; 16 : 3 224 ; 48 : 2 172 ; 32 : 1 687 ; 6 : 1 974 ; 12 : 1 499 ; 24 : 799 ; autres valeurs de 5 à 96 : le reste (maximum observé 96).

## Historique

- 1.1-draft (AMD-001, relecture R2) : repère de scène unique (REQ-E3D-410, DEC-R2-T2) ; décalage ±0,1 des sprites confié au rendu (REQ-E3D-405, TV-W2, DEC-R2-T4) ; notes LEAK reformulées (REQ-E3D-564, 601, 603, Q-2) ; REQ-E3D-507 retirée ; numérotation MARCHING normative (REQ-E3D-521/522) ; clauses précisées (442, 443, 446, 454, 455, 566, 426, 567, 568, 643) ; renvoi NES_RGB vers SPEC-04 ; questions closes par DECISIONS-R0.
