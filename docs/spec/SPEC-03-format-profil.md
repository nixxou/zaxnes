# SPEC-03 — Format des profils de jeu (`.bytes` / `.3dn`), catalogue et association ROM ↔ profil
Version : 1.1 | Statut : gelé (2026-10-20)
Dépend de : SPEC-02 (contrat de tile du cœur), SPEC-04 / SPEC-04B (sémantique des patterns et de la géométrie), SPEC-05 (scripts Lua), SPEC-06 (scène), SPEC-09 (persistance), SPEC-10 (UI)

## 1. Objet et périmètre

Ce document définit **exactement** comment lire et réécrire sans perte :
- les 101 profils embarqués (`assets-neutral/profiles/*.bytes`) ;
- les profils utilisateur (fichiers `.3dn`) ;
- le catalogue des profils embarqués (`assets-neutral/profiles.json`) ;
- la façon dont un jeu de la bibliothèque est relié à un fichier ROM.

Il donne le schéma protobuf complet (numéros de champs, types de fil, présence, valeurs par défaut), les conventions de fil non standard (références partagées, cellules nullables, sous-type par inclusion), l'encodage réversible des plans de tiles, les règles de version, et la signification **courte** de chaque champ. Le comportement que ces champs pilotent (détection de patterns, construction des meshes, scripts, scène) est défini dans SPEC-04, SPEC-04B, SPEC-05 et SPEC-06.

Hors périmètre : l'éditeur de profils (création, modification, sauvegarde interactive), le mode « ROM brute » de l'éditeur, la VR. Les champs que ces fonctions laissent dans les fichiers DOIVENT néanmoins être lus et conservés (REQ-PROF-004).

## 2. Concepts et vocabulaire

| Terme | Sens dans ce document |
|---|---|
| **Profil** | Fichier binaire protobuf décrivant la conversion 3D d'un jeu NES : patterns, variantes 3D, scripts Lua, réglages de scène. Extension `.bytes` pour les profils embarqués extraits, `.3dn` pour les profils utilisateur ; même format. |
| **Tile** | Motif NES 8×8 à 2 bits par pixel, identifié par ses deux plans de bits (bas et haut) et ses drapeaux de miroir (SPEC-02). |
| **Plan** | Entier de 64 bits contenant un plan de bits d'une tile : bit `y*8 + (7-x)` = bit du pixel (x, y), x et y dans 0..7, (0,0) en haut à gauche, sans miroir appliqué. |
| **Pattern** | Assemblage rectangulaire de tiles (grille `tSize.x × tSize.y`, cellules éventuellement vides) reconnu à l'écran et converti en objet 3D (SPEC-04). |
| **Variante 3D** | Jeu de paramètres 3D d'un pattern (profondeur, géométrie, couche, transformations, déformation, texture, tags). Un pattern a une ou plusieurs variantes ; la première est la variante principale. Côté Lua, une variante est l'objet `pattern` manipulé par les scripts (SPEC-05). |
| **Pattern permanent / temporaire** | Drapeau `permanent` d'un pattern. Les deux familles sont rangées dans deux listes distinctes du fichier. |
| **Enveloppe de référence** | Convention de fil permettant de partager un même objet (tile, pattern, variante) entre plusieurs emplacements du fichier (§3.2). |
| **Cellule nullable** | Élément de la grille d'un pattern, qui peut être vide (§3.2). |
| **Réglages embarqués** | Copie complète des réglages de l'application au moment où le profil a été écrit (lumière, couches, découpe, etc.), dont une partie est appliquée au chargement (§3.7). |
| **Clé d'encodage** | Constante 64 bits `0x000708FACD884A5A` (`format-data.json`, `tile_encoding_key`) utilisée pour masquer les plans dans le fichier. |

Noms de messages : `ProfileFile`, `RefEnvelope`, `ShapeParams`, `TexRect`, `TexPayload`, `RecordedSlot`, `Cell`, `TilePlanes` et les autres noms de messages de ce document sont des **noms de commodité propres à la spec**, sans portée normative : ils ne circulent pas sur le fil protobuf (seuls les numéros de champs et les types de fil comptent) et l'implémentation est libre de nommer ses types.

Notation : `LEN` = type de fil 2 (longueur préfixée), `VARINT` = 0, `I32` = 5 (4 octets little-endian), `SGROUP`/`EGROUP` = 3/4. « tag » = varint `(numéro << 3) | type de fil`.

## 3. Exigences

### 3.1 Règles générales d'encodage

**REQ-PROF-001** (P1) — Un profil DOIT être lu comme un message protobuf (encodage binaire standard) dont le message racine est `ProfileFile` (§4.1). Le lecteur DOIT accepter les champs dans n'importe quel ordre et DOIT ignorer un champ inconnu sans échouer (règle protobuf), mais aucun des 101 profils embarqués n'en contient.
- Critère d'acceptation : le décodage des 101 fichiers de `assets-neutral/profiles` réussit ; un décodeur instrumenté qui compte les numéros de champs hors schéma en rencontre 0 ; un fichier de test auquel on ajoute un champ inconnu (numéro 99, tout type de fil) se décode avec le même modèle.

**REQ-PROF-002** (P1) — Types scalaires sur le fil :
- `int32`, `enum`, `bool`, `byte`, `sbyte`, `short` : VARINT. Une valeur négative est étendue en signe sur 64 bits (varint de 10 octets), sans zigzag. `bool` vaut 0 ou 1.
- `int64` : VARINT du complément à deux sur 64 bits (10 octets si négatif), sans zigzag.
- `float` : I32, IEEE-754 simple précision little-endian. Les bits DOIVENT être conservés tels quels (pas de normalisation des NaN ni de −0).
- `string` : LEN, UTF-8.
- Message imbriqué : LEN, sauf mention contraire (cellules nullables : groupe).
- Critère d'acceptation : `-1` en int32 s'encode `ff ff ff ff ff ff ff ff ff 01` ; `1.0f` s'encode `00 00 80 3f`.

**REQ-PROF-003** (P1) — Présence : un champ de type message ou chaîne absent du fichier signifie « valeur nulle » (différente de « chaîne vide »). Un scalaire absent prend la valeur par défaut indiquée dans les tables du §4 (colonne « Défaut si absent ») ; sauf mention contraire, c'est 0 / faux. Le lecteur DOIT donc accepter l'absence de n'importe quel scalaire, cas fréquent dans les profils embarqués (ex. : `skipGeo` absent dans 27 810 patterns, champ 24 des réglages absent dans 23 profils) ; l'écrivain canonique, lui, écrit toujours les scalaires (REQ-PROF-051).
- Critère d'acceptation : dans `empty.bytes`, les champs de réglages 24 à 38 sont absents ; après chargement, `EmptyTile` = faux, `SceneRatio` = 1, `Fov` = 70 (défauts de REQ-PROF-040).

**REQ-PROF-004** (P1) — Le modèle chargé DOIT conserver tous les champs du schéma, y compris ceux qui ne servent qu'à des fonctions hors périmètre (éditeur, VR), pour permettre une réécriture sans perte (REQ-PROF-050).
- Critère d'acceptation : décoder puis réécrire chaque profil embarqué avec l'écrivain canonique (REQ-PROF-051) redonne un fichier identique octet par octet.

### 3.2 Références partagées et cellules nullables

**REQ-PROF-010** (P1) — Enveloppe de référence. Les champs marqués `ref` dans le schéma (listes de patterns `ProfileFile.3` et `.4`, variantes `Pattern.10`, tiles des cellules `Cell.1`) ne contiennent pas directement le message mais une **enveloppe** `RefEnvelope` (LEN) :

| N° | Type | Sens |
|---|---|---|
| 1 | VARINT int32 | Clé d'un objet **déjà vu** dans le fichier. Si présent, l'enveloppe ne contient rien d'autre et désigne cet objet (même identité). |
| 2 | VARINT int32 | Clé attribuée à un **nouvel** objet. |
| 10 | LEN | Le message de l'objet (type donné par le schéma du champ porteur). Présent si et seulement si le champ 2 est présent. |

Les champs 3, 4 et 8 de cette convention (type dynamique) n'apparaissent jamais dans les profils ; un lecteur PEUT les rejeter.
- Critère d'acceptation : dans `BalloonFight.bytes`, le premier élément du champ racine 4 (longueur 611) commence par `10 01 52 de 04` : nouvel objet de clé 1, puis champ 10 de longueur 606 contenant le pattern.

**REQ-PROF-011** (P1) — Clés. Les clés sont **communes à tout le fichier** (tous types confondus : tiles, patterns, variantes, y compris ceux des réglages embarqués) et attribuées **séquentiellement à partir de 1** dans l'ordre d'écriture, au moment où l'enveloppe d'un nouvel objet commence, donc **avant** les objets qu'il contient (ordre préfixe). La clé 0 est réservée (objet racine) et n'apparaît pas. Une référence (champ 1) ne peut désigner qu'un objet dont l'enveloppe a déjà commencé.
- Critère d'acceptation : dans `BalloonFight.bytes`, pattern clé 1 → ses 8 tiles clés 2 à 9 → sa variante clé 10 ; le pattern suivant a la clé 11, sa tile la clé 12, sa variante la clé 13. Sur les 101 profils, toutes les clés nouvelles sont strictement séquentielles (vérifié).
- Notes : un lecteur DOIT résoudre une référence vers le même objet en mémoire (même identité), pas vers une copie. Les tiles sont massivement partagées : 225 575 tiles nouvelles et 226 656 références vers des tiles déjà vues dans l'ensemble des profils. Aucune variante ni aucun pattern n'est partagé dans les profils embarqués, mais le lecteur DOIT l'accepter.

**REQ-PROF-012** (P1) — Cellules nullables. La grille `Pattern.1` est une liste répétée dont chaque élément est un **groupe** (SGROUP … EGROUP, même numéro de champ 1) :
- groupe vide (`0b 0c`) = cellule vide (pas de tile) ;
- sinon le groupe contient exactement un champ 1 (LEN) = `RefEnvelope` vers un message `Cell`.
Le nombre d'éléments vaut `tSize.x × tSize.y`, en ordre ligne par ligne : la cellule (cx, cy) est à l'index `cx + cy × tSize.x`.
- Critère d'acceptation : sur les 101 profils : 452 231 cellules non vides et 124 059 cellules vides ; dans 67 832 patterns sur 67 832 ayant une grille, le nombre de cellules vaut `tSize.x × tSize.y`.

**REQ-PROF-013** (P1) — Sous-type par inclusion. Le message `TexRect` (rectangle + texture) porte son contenu de texture dans le champ 10 (message `TexPayload`) ; un écrivain DOIT écrire le champ 10 **avant** les champs 1 et 2 (seule exception à l'ordre croissant, REQ-PROF-051). Le lecteur accepte tout ordre.
- Critère d'acceptation : la texture « nulle » d'une variante s'écrit `9a 01 08 52 02 10 00 08 00 10 00`.

### 3.3 Message racine

**REQ-PROF-020** (P1) — Le champ 1 (`Type`) DOIT valoir exactement `"3dn"`. Toute autre valeur rend le fichier invalide (chargement refusé, REQ-PROF-033). Un champ 1 absent n'est pas une erreur.
- Critère d'acceptation : les 101 profils commencent par `0a 03 33 64 6e` ; un fichier dont le champ 1 vaut `"3dx"` est refusé.

**REQ-PROF-021** (P1) — Le champ 2 (`Ver`) est une chaîne de version. Le lecteur DOIT en retirer tout caractère autre que chiffres, `.` et `$`, puis :
- `"1"` → format non supporté, chargement refusé ;
- `"2"` → profil « format V2 » : drapeau mémorisé et exposé aux scripts sous le nom `IsV2Format` (SPEC-05) ; il modifie le point de référence du pivot de déformation (SPEC-04B, `deltaPivot`) ;
- toute autre valeur (dont `"3"`, version écrite par la dernière version d'origine) → format courant, `IsV2Format` = faux.
- Critère d'acceptation : 100 profils embarqués sont en `"2"`, un seul (`Jackal (U).bytes`) en `"3"`. Une version `"v1"` est refusée ; `"2.0"` n'est pas V2.

**REQ-PROF-022** (P2) — Métadonnées : champs 7 `Id`, 8 `Title`, 9 `Maker` (auteur du profil), 10 `GameDev` (éditeur du jeu), 11 `Des` (description). Toutes facultatives. Pour un profil **embarqué**, l'identité, le titre, l'éditeur et l'auteur affichés viennent du catalogue (REQ-PROF-060), **pas** de ces champs.
- Critère d'acceptation : `Jackal (U).bytes` contient `Title = "Tennis"` mais s'affiche « Jackal » (catalogue id `"101"`) ; 56 profils embarqués ont un `Id`, 17 un `Title`/`Maker`/`GameDev`, aucun un `Des`.

**REQ-PROF-023** (P1) — Patterns : le champ 4 (`patterns`) liste les patterns rangés comme permanents, le champ 5 (`tempPatterns`) les temporaires, chacun en enveloppe de référence. À la lecture, la famille effective d'un pattern est donnée par **son propre champ `permanent`** (Pattern.4), pas par la liste qui le contient. Un même objet pattern rencontré deux fois (par référence) n'est chargé qu'une fois.
- Critère d'acceptation : total sur les 101 profils : 66 609 entrées en champ 4, 1 223 en champ 5 ; toutes celles du champ 4 ont `permanent = vrai`.

**REQ-PROF-024** (P1) — Scripts : le champ 3 (`scripts`) liste les scripts Lua (message `Script`, sans enveloppe de référence). Après chargement, l'ensemble des scripts DOIT être trié par `name` selon la collation non ordinale de SPEC-05 REQ-LUA-004, qui fait foi (DEC-03-Q02) ; c'est l'ordre d'exécution des hooks.
- Critère d'acceptation : 579 scripts dans 89 profils ; `empty.bytes` contient un script `name = "Build"`, `TagStr = "*"`, dont la source commence par `function BuildPattern()`.

**REQ-PROF-025** (P1) — Réglages : le champ 6 contient les réglages embarqués (message `Settings`, §4.6, appliqués selon §3.7). Le profil DOIT être refusé (REQ-PROF-033) si le champ 6 est absent ou si, dans ce message, l'un des éléments `Light` (1), `Clipping` (4), `Layer` (6) est absent. Si l'élément `SavedPattern3D` (14) ou `SaveAnimPattern3D` (15) est absent, le profil DOIT être accepté et l'élément manquant reconstitué avec 5 emplacements par défaut (`Config1` à `Config5`, `Deform1` à `Deform5`) (DEC-03-Q03, divergence acceptée **(D)** : l'original refuse ce profil).
- Critère d'acceptation : un profil dont le message de réglages ne contient pas le champ 4 se charge comme un profil vide (REQ-PROF-033) ; un profil sans élément 14 se charge normalement et expose 5 emplacements `Config1`…`Config5`.
- Notes : les 101 profils embarqués ont les cinq éléments.

### 3.4 Tiles et encodage des plans

**REQ-PROF-030** (P1) — Encodage des plans. Les champs `TilePlanes.1` (plan bas) et `TilePlanes.2` (plan haut) ne contiennent pas le plan en clair. Avec `K = 0x000708FACD884A5A` et `r = K & 0x3F = 26` :
- stocké = `rotr64(plan XOR K, 26)` ;
- plan = `rotl64(stocké, 26) XOR K` ;
où `rotr64`/`rotl64` sont des rotations sur 64 bits non signés ; la valeur stockée est ensuite écrite comme un int64 (complément à deux, REQ-PROF-002). La transformation est une bijection ; elle s'applique **uniquement** à ces deux champs, partout où le message `TilePlanes` apparaît.
- Critère d'acceptation : vecteurs du §5.2 (ex. : plan 0 ↔ stocké `0x6212968001C23EB3` = 7 066 876 241 827 217 075, qui est la valeur stockée la plus fréquente).

**REQ-PROF-031** (P1) — Le champ `TilePlanes.3` (`flip`) vaut `bit0 = miroir horizontal`, `bit1 = miroir vertical`. La couleur (0..3) du pixel affiché (x, y) d'une tile est `2×bitH + bitL`, avec `bitP = (plan >> (yy*8 + 7 - xx)) & 1`, `xx = flipX ? 7-x : x`, `yy = flipY ? 7-y : y`.
- Critère d'acceptation : la tile clé 12 de `BalloonFight.bytes` (§5.3) donne le dessin attendu.

**REQ-PROF-032** (P1) — Identité de tile : deux tiles sont **la même tile** si et seulement si (plan bas, plan haut, flip) sont égaux, indépendamment de l'enveloppe qui les porte. Les propriétés dérivées d'une tile (nombre de pixels, boîte englobante, couleurs présentes, densité, connexité) ne sont pas stockées et DOIVENT être recalculées (SPEC-04).
- Critère d'acceptation : deux cellules portant des enveloppes différentes mais des valeurs égales sont traitées comme la même tile par la reconnaissance (test SPEC-04).

**REQ-PROF-033** (P1) — Échec de chargement : si le fichier est invalide (protobuf mal formé, `Type` ≠ `"3dn"`, version `"1"`, réglages incomplets, référence vers une clé inconnue, clé non séquentielle), le chargement DOIT échouer **en bloc** : aucun pattern ni script partiel n'est conservé, l'émulation continue sans conversion 3D spécifique (profil vide), et l'erreur est journalisée. Le jeu ne DOIT pas planter.
- Critère d'acceptation : un profil tronqué à la moitié de sa taille se charge comme un profil vide, le jeu démarre.

### 3.5 Patterns, variantes 3D et textures

**REQ-PROF-034** (P1) — Le lecteur DOIT construire, pour chaque pattern, la grille de tiles (REQ-PROF-012), la liste ordonnée des variantes (`Pattern.10`, au moins une), et conserver tous les autres champs (§4.3). Défauts si absents : `constraintSize = (99, 99)`, `selfConnected = vrai`, `skipGeo = faux`.
- Critère d'acceptation : 68 842 patterns au total (dont 1 010 dans les emplacements mémorisés des réglages) ; parmi les 67 832 patterns des champs 4 et 5, répartition du nombre de variantes : 1 → 62 650, 2 → 4 142, 3 → 800, ≥4 → 240.

**REQ-PROF-035** (P1) — Texture d'une variante (`ShapeParams.19`, message `TexRect`) :
- champ absent → pas de texture (cas des patterns des emplacements mémorisés) ;
- **marqueur nul** (`w = 0`, `h = 0`, `TexPayload` avec format 0 et aucune donnée) → la variante utilise la **texture par défaut** du pattern, calculée à partir des tiles (taille `tSize.x×8` × `tSize.y×8`, chaque pixel prenant l'index de couleur de la tile, voir SPEC-04) ;
- sinon texture explicite de `w × h` pixels, pixel (x, y) à l'index `n = y×w + x` (y = 0 en haut).
Format `0` (indexé) : `data` contient `ceil(w×h/16)` entiers ; bits `2(n%16)` et `2(n%16)+1` de `data[n/16]` = bits 0-1 de l'index ; `extra` (même longueur) : bit `n%16` de `extra[n/16]` = bit 2 de l'index (index 0..7). Format `1` (direct) : `data` contient `w×h` entiers ARGB (`a<<24 | r<<16 | g<<8 | b`), la valeur 0 signifiant « couleur de fond de la palette ». `extra` absent = tout à zéro.
- Critère d'acceptation : 74 389 textures, dont 60 708 marqueurs nuls ; pour les 13 681 autres, la longueur de `data` (et de `extra` s'il est présent) est exactement celle indiquée (vérifié sur 100 %).

**REQ-PROF-036** (P1) — Tags de variante (`ShapeParams.20`) et de script (`Script.6`) : chaîne de tags séparés par des virgules, espaces de bord retirés, éléments vides ignorés. Le tag `*` sur un script le lie à toutes les variantes ; sinon un script est lié aux variantes partageant au moins un tag (SPEC-05). Tags spéciaux de variante reconnus : `!cast`, `!receiv`, `!recv`, `!static`, `_gaxis` (effets : SPEC-06).
- Critère d'acceptation : `TagStr = " char, ,!cast "` donne les tags `char` et `!cast`.

**REQ-PROF-037** (P2) — `ShapeParams.23` (`emptyColor`, sbyte) : une valeur lue négative DOIT être ramenée dans 0..4 en ajoutant 5. (Aucune valeur négative dans les profils embarqués.)
- Critère d'acceptation : une valeur stockée `-1` est exposée comme `4`.

### 3.6 Scripts

**REQ-PROF-038** (P1) — Message `Script` : `name` (1) et `Name` (5) portent le même nom (écrits tous deux) ; à la lecture, le nom est celui du champ 1, le champ 5 étant redondant. `enable` (2), `error` (3), `Source` (4 : code Lua complet), `TagStr` (6). La compilation et l'exécution relèvent de SPEC-05 ; une erreur de compilation n'empêche pas le chargement du profil.
- Critère d'acceptation : sur les 579 scripts, champ 1 = champ 5 dans 100 % des cas ; 556 `enable = vrai`, 23 faux ; `error` toujours faux.

### 3.7 Réglages embarqués

**REQ-PROF-040** (P1) — Défauts des réglages si absents du message `Settings` : `Fov = 70`, `Skybox = -1`, `Mirroring = vrai`, `SceneRatio = 1`, `SkipFrame = vrai`, `BGMusic = vrai`, tous les autres scalaires 0 / faux. Dans `LayerParams`, `CastShadow`/`RecvShadow` absents = faux.
- Critère d'acceptation : voir REQ-PROF-003.

**REQ-PROF-041** (P1) — Application au chargement d'un profil (mode bibliothèque). Les valeurs suivantes du profil REMPLACENT les réglages courants de l'utilisateur : `Light.Direction`, `Light.Color`, `Clipping`, `MapEnable`, `Layer` (pour chaque couche `Offset`, `Aligned` ; `DefLayer` ; `Auto`), `GraphicQuality`, `SavedPattern3D`, `SaveAnimPattern3D`, `SameBgPalette`, `SameSpPalette`, `SpriteTolerance`, `ScanDirection`, `EmptyTile`, `LocalCam`, `TVMode`. De plus `BgColor` est forcé à vrai. Toutes les autres valeurs du profil sont **ignorées** et les réglages de l'utilisateur sont conservés : `Light.Intensity`, `RenderMode`, `VrRuntime`, `Skybox`, `Core`, `GameSpeed`, `Shadow`, `Volume`, `FrameRate`, `VSync`, `ControlMethod`, `SceneRatio`, `EmulatedMenuButton`, `RenderMethod`, `ResScaleIndex`, `SkipFrame`, `BGMusic`, `AntiAlias`, `Mirroring`, `AutoTVMode`, `TVBox`, `SortOption`, `CustomBgColor`, `Fov`.
- Critère d'acceptation : utilisateur avec `Volume = 0.3`, `Light.Intensity = 2` ; chargement de `empty.bytes` → `Volume = 0.3`, `Light.Intensity = 2`, `Light.Direction = (1, 0.2, 1)`, `Clipping = (4.1, 16.1, 4.1, 0.1)`, `GraphicQuality = 2`.
- Notes : un champ « appliqué » absent du profil prend le défaut de REQ-PROF-040, pas la valeur de l'utilisateur. Ce que deviennent ces valeurs dans le fichier de réglages persistant : SPEC-09.

**REQ-PROF-042** (P1) — Ombres des couches : quelle que soit la valeur stockée, après lecture, la couche d'index i (0..5) a `CastShadow = (i == 2)` et `RecvShadow = (i >= 2)`.
- Critère d'acceptation : sur les 606 couches stockées, 119 ont `CastShadow = vrai` ; après chargement, exactement une couche par profil (index 2) l'a.

**REQ-PROF-043** (P2) — `Layers.1` contient 6 éléments `LayerParams`, indexés par la valeur de l'énumération de couche (0 non identifiée, 1 UI, 2 à 5 = L1 à L4). Les 101 profils en ont 6. Un nombre différent DOIT être signalé comme profil invalide (REQ-PROF-033).

**REQ-PROF-044** (P2) — Emplacements mémorisés : `Settings.14` (`SavedPattern3D`, 5 entrées, noms par défaut `Config1`…`Config5`) et `Settings.15` (`SaveAnimPattern3D`, 5 entrées, `Deform1`…`Deform5`) sont des listes de `RecordedSlot` (nom + pattern **sans enveloppe de référence**, dont la grille est absente et `tSize = (0,0)`, avec une variante). Ce sont des préréglages de l'éditeur ; ils DOIVENT être conservés.
- Critère d'acceptation : 1 010 emplacements au total ; dans `empty.bytes`, les noms de 14 sont `Road, BG, Top, Loop, Body`.

### 3.8 Écriture et réécriture sans perte

**REQ-PROF-050** (P2) — L'implémentation DOIT pouvoir réécrire un profil chargé sans perte d'information au sens du schéma (toutes valeurs, ordres de listes, identités partagées).
- Critère d'acceptation : décoder → écrire → décoder donne un modèle égal champ par champ, pour les 101 profils.

**REQ-PROF-051** (P2) — Écrivain canonique (pour l'identité octet par octet) : champs en ordre de numéro croissant (sauf `TexRect` : 10, puis 1, 2) ; scalaires toujours écrits, y compris nuls ; chaînes écrites si non nulles (même vides) ; messages et chaînes nuls omis ; listes vides omises ; listes répétées **non compactées** (pas de « packed ») ; varints minimaux ; clés de référence attribuées selon REQ-PROF-011 en fonction de l'identité des objets ; floats recopiés bit à bit ; `TexPayload.3` omis si toutes ses valeurs sont nulles.
- Critère d'acceptation : test automatisé sur `assets-neutral/profiles` : décoder puis réécrire chacun des 101 profils avec l'écrivain canonique redonne un fichier identique octet par octet (101 sur 101).

**REQ-PROF-052** (P3) — Normalisations à l'écriture d'un **nouveau** profil (hors éditeur, mais si l'implémentation exporte) : `Type = "3dn"`, `Ver = "3"` ; si `Id` est vide, en générer un nouveau de 12 caractères (alphabet base64 standard `A–Z a–z 0–9 + /`, obtenu en encodant 9 octets aléatoires cryptographiques en base64 ; cet aléa est hors du périmètre de déterminisme de REQ-ARCH-012, car l'export se fait hors de la boucle de jeu et n'influence ni l'émulation ni la conversion) ; si `Title` est la chaîne vide, y mettre le nom de fichier de la ROM ; scripts écrits dans l'ordre trié ; patterns permanents en champ 4, temporaires en champ 5 ; réglages = réglages courants complets ; variante dont la texture est identique à la texture par défaut → marqueur nul.
- Critère d'acceptation : un profil exporté sans `Id` reçoit un `Id` de 12 caractères base64 ; relu, il donne les mêmes patterns.

### 3.9 Catalogue embarqué, profils utilisateur, association ROM

**REQ-PROF-060** (P1) — Catalogue : `profiles.json` liste, par édition (`full` : 101 entrées, `demo` : 20 entrées), des objets `{id, title, publisher, description, author, file}`. L'application DOIT utiliser l'édition `full`. L'`id` (chaîne) est l'identifiant de jeu dans la bibliothèque ; `title`, `publisher`, `author`, `description` sont les métadonnées affichées ; `file` désigne les octets du profil. L'édition `demo` est une donnée conservée, non utilisée (ses `id` recouvrent ceux de `full` avec d'autres jeux).
- Critère d'acceptation : la bibliothèque contient 101 jeux embarqués ; l'id `"47"` est « 2D Mode » (`profiles/empty.bytes`).

**REQ-PROF-061** (P1) — Mode 2D : un jeu dont le titre de profil est exactement `"2D Mode"` est le jeu « 2D » : il s'affiche sous ce nom quel que soit le titre personnalisé, utilise une image d'attente propre, et son profil est relu à chaque démarrage (SPEC-04 pour le rendu 2D).
- Critère d'acceptation : lancer « 2D Mode » avec n'importe quelle ROM fonctionne et n'affiche aucun pattern 3D.

**REQ-PROF-062** (P1) — Profils utilisateur : au démarrage, tous les fichiers `*.3dn` du dossier `CustomProfiles` situé dans le dossier de données de l'utilisateur (SPEC-09) sont examinés. Pour chacun, seuls les champs racine 1, 7, 8, 9, 10, 11 sont nécessaires (en-tête ; les autres champs peuvent être sautés). Un fichier dont `Type` ≠ `"3dn"` ou illisible est ignoré (journalisé). Le jeu ajouté a pour id `Id`, pour titre `Title` ou, s'il est vide/absent, le nom du fichier sans extension. Si l'id est déjà présent (profil embarqué ou profil utilisateur lu avant), le fichier est ignoré avec un message « profil dupliqué ».
- Critère d'acceptation : un fichier `Foo.3dn` sans `Title` et d'`Id` `"abc123def456"` apparaît sous le titre « Foo » ; un second fichier de même `Id` est ignoré ; un fichier d'`Id` `"1"` est ignoré (collision avec Arkanoid).
- Notes : l'ordre d'énumération des fichiers décide du gagnant entre doublons ; l'implémentation DEVRAIT trier les noms de fichiers (ordinal) pour être déterministe.

**REQ-PROF-063** (P2) — Import d'un profil : le fichier choisi est lu comme en REQ-PROF-062 ; un `Id` absent ou vide DOIT provoquer le refus avec le message « profil obsolète ou au mauvais format, incompatible avec cette version ». Sinon le fichier est copié (même nom, écrasement) dans `CustomProfiles` : si l'id est nouveau, le jeu est ajouté ; si l'id appartient à un profil utilisateur, l'ancien fichier est supprimé et remplacé ; si l'id appartient à un profil embarqué, rien n'est fait.
- Critère d'acceptation : importer deux fois le même fichier ne crée qu'un jeu.

**REQ-PROF-064** (P1) — Association ROM : il n'y a **aucune** reconnaissance automatique de la ROM (pas de hachage, pas de CRC, pas de base de jeux). La bibliothèque est une liste de **profils** ; l'utilisateur choisit un jeu, et si aucun chemin de ROM n'est mémorisé pour ce jeu (ou s'il force le choix), un sélecteur de fichier (`.nes`, `.zip`) s'ouvre ; le chemin choisi est mémorisé pour ce jeu (clé = id du profil) dans `rom.json` (SPEC-09) et réutilisé aux lancements suivants. N'importe quelle ROM peut être associée à n'importe quel profil ; la conversion n'est correcte que si la ROM correspond au jeu du profil.
- Critère d'acceptation : associer une ROM de test publique (par exemple la ROM minimale de SPEC-02 §5.1) au profil Arkanoid lance cette ROM avec les réglages de scène d'Arkanoid, sans contrôle d'identité de la ROM ; le chemin est mémorisé dans `rom.json` sous l'id `"1"`.
- Notes : format exact de `rom.json` (liste `Items` d'objets `{id, romPath, title, lastTime}`) : SPEC-09. Une entrée dont l'id ne correspond à aucun profil connu est supprimée au démarrage.

**REQ-PROF-065** (P2) — Suppression d'un jeu : seuls les jeux issus de profils utilisateur sont supprimables ; la suppression efface le fichier `.3dn` et l'image de couverture associée (SPEC-09).
- Critère d'acceptation : l'action « supprimer » est indisponible sur un jeu embarqué.

**REQ-PROF-066** (P3) — Fichier `.3dn` de l'éditeur (informatif, hors périmètre) : le mode éditeur lit et écrit `<dossier de l'application>/3dn/<nom du fichier ROM, extension remplacée par .3dn>`, en renommant l'ancienne version en `.bak`. Le format est identique à celui des profils.

## 4. Formats et interfaces

### 4.1 `ProfileFile` (racine)

| N° | Nom (visible Lua) | Fil | Type | Rép. | Défaut si absent | Sens |
|---|---|---|---|---|---|---|
| 1 | `Type` | LEN | string | | null | Signature, doit être `"3dn"` |
| 2 | `Ver` | LEN | string | | null | Version (`"2"`, `"3"`), REQ-PROF-021 |
| 3 | `scripts` | LEN | `Script` | oui | vide | Scripts Lua |
| 4 | `patterns` | LEN | `RefEnvelope<Pattern>` | oui | vide | Patterns permanents |
| 5 | `tempPatterns` | LEN | `RefEnvelope<Pattern>` | oui | vide | Patterns temporaires |
| 6 | (réglages) | LEN | `Settings` | | null | Réglages embarqués |
| 7 | `Id` | LEN | string | | null | Identifiant de profil (12 car.) |
| 8 | `Title` | LEN | string | | null | Titre |
| 9 | `Maker` | LEN | string | | null | Auteur du profil |
| 10 | `GameDev` | LEN | string | | null | Éditeur du jeu |
| 11 | `Des` | LEN | string | | null | Description |

L'en-tête lu pour la bibliothèque (profils utilisateur) est le sous-ensemble {1, 7, 8, 9, 10, 11}.

### 4.2 `RefEnvelope<T>` et cellule nullable

`RefEnvelope<T>` : voir REQ-PROF-010 (1 = clé existante, 2 = nouvelle clé, 10 = message T).
Cellule de grille : groupe `SGROUP(1) [ 1:LEN RefEnvelope<Cell> ] EGROUP(1)`.

`Cell` :

| N° | Nom | Fil | Type | Sens |
|---|---|---|---|---|
| 1 | (plans) | LEN | `TilePlanes` | Données de la tile |

`TilePlanes` :

| N° | Nom | Fil | Type | Sens |
|---|---|---|---|---|
| 1 | (plan bas) | VARINT | int64 | Plan bas **encodé** (REQ-PROF-030) |
| 2 | (plan haut) | VARINT | int64 | Plan haut **encodé** |
| 3 | `flip` | VARINT | int32 | bit0 miroir H, bit1 miroir V |

### 4.3 `Pattern`

| N° | Nom (visible Lua) | Fil | Type | Rép. | Défaut si absent | Sens (détail SPEC-04) |
|---|---|---|---|---|---|---|
| 1 | `tile2DGrid` | SGROUP | cellule nullable | oui | vide | Grille ligne par ligne |
| 2 | `constraintSize` | LEN | `IVec2` | | (99, 99) | Contrainte de taille ; (99,99) = sans contrainte, (100,100) = contrainte « V1 » |
| 3 | `tSize` | LEN | `IVec2` | | null | Taille en tiles (largeur, hauteur) |
| 4 | `permanent` | VARINT | bool | | faux | Pattern permanent / temporaire |
| 5 | `pixelCount` | VARINT | int32 | | 0 | Nombre de pixels (valeur mémorisée, non recalculée) |
| 6 | `colorFlags` | VARINT | enum (drapeaux) | | 0 | bit1/bit2/bit3 = couleur 1/2/3 présente |
| 7 | `tDensity` | VARINT | enum | | 0 | Densité en tiles : 0 EMPTY, 1 LOW, 2 AVERAGE, 3 MEDIUM, 4 HIGH, 5 SOLID (valeurs 6 et 8 observées une fois chacune : à conserver) |
| 8 | `pDensity` | VARINT | enum | | 0 | Densité en pixels, même énumération |
| 10 | `pattern3DList` | LEN | `RefEnvelope<ShapeParams>` | oui | vide | Variantes 3D, la première est la principale |
| 11 | `selfConnected` | VARINT | bool | | vrai | Option de connexité interne |
| 12 | `skipGeo` | VARINT | bool | | faux | Ne pas analyser la géométrie |

### 4.4 `ShapeParams` (variante 3D)

| N° | Nom (visible Lua) | Fil | Type | Sens (détail SPEC-04B / SPEC-06) |
|---|---|---|---|---|
| 1 | `sizeZ` | VARINT | int32 | Profondeur en voxels (max 96) |
| 2 | `geo` | VARINT | enum | 0 Default, 1 HCYLINDER, 2 VCYLINDER, 3 CUBE, 4 HALFHCYLINDER, 5 HALFVCYLINDER |
| 3 | `antiShadow` | VARINT | bool | |
| 4 | `flip` | VARINT | bool | |
| 5 | `solid` | VARINT | bool | |
| 6 | `fullWarp` | VARINT | bool | |
| 7 | `layer` | VARINT | enum | 0 UNIDENTIFIED, 1 UI, 2 L1, 3 L2, 4 L3, 5 L4 |
| 8 | `scale` | LEN | `Vec3` | Échelle |
| 9 | `rot` | LEN | `Vec3` | Rotation (degrés) |
| 10 | `offset` | LEN | `Vec3` | Décalage |
| 11 | `pivot` | LEN | `Vec3` | Pivot (fraction de la taille) |
| 12 | `alpha` | I32 | float | Opacité |
| 13 | `rm` | VARINT | byte | Algorithme de mesh : 0 = réglage global, 1 GREEDY, 2 MARCHING, 3 GREEDY_MULTICOLOR |
| 14 | `deltaAmp` | LEN | `Vec3` | Amplitude de déformation |
| 15 | `deltaPivot` | LEN | `Vec3` | Pivot de déformation |
| 16 | `deltaSpeed` | LEN | `Mat4` | Vitesses de déformation ; ligne 3 = données libres `CData[0..3]` |
| 17 | `deltaOffset` | LEN | `Mat4` | Phases de déformation ; ligne 3 = `CData[4..7]` |
| 18 | `renderStyle` | VARINT | int32 | 0 masqué, 1 normal, 2 ombre seule |
| 19 | `tex` | LEN | `TexRect` | Texture (REQ-PROF-035) |
| 20 | `TagStr` | LEN | string | Tags (REQ-PROF-036) |
| 23 | `emptyColor` | VARINT | sbyte | Couleur des pixels vides (REQ-PROF-037) |

Défaut si absent : 0 / faux / null. Champs 21 et 22 : jamais utilisés.

### 4.5 Textures

`TexRect` : 10 `TexPayload` (LEN, écrit en premier) ; 1 `w` (VARINT int32, pixels) ; 2 `h` (VARINT int32).
`TexPayload` : 1 `data` (VARINT int32, **répété non compacté**) ; 2 `format` (VARINT enum : 0 indexé, 1 direct) ; 3 `extra` (VARINT short étendu en int32, répété non compacté, omis si tout est nul).

### 4.6 `Settings` (réglages embarqués)

| N° | Nom (visible Lua) | Fil | Type | Défaut | Appliqué (REQ-PROF-041) | Sens |
|---|---|---|---|---|---|---|
| 1 | `Light` | LEN | `Light` | null | partiel | Lumière principale |
| 2 | `BgColor` | VARINT | bool | faux | forcé vrai | Fond = couleur de palette |
| 3 | `CustomBgColor` | LEN | `Argb` | null | non | Couleur de fond personnalisée |
| 4 | `Clipping` | LEN | `Vec4` | null | oui | Marges de découpe de l'image (SPEC-06) |
| 5 | `MapEnable` | VARINT | bool | faux | oui | |
| 6 | `Layer` | LEN | `Layers` | null | oui | Couches de profondeur |
| 7 | `GraphicQuality` | VARINT | int32 | 0 | oui | |
| 8 | `VrRuntime` | VARINT | int32 | 0 | non | |
| 9 | `VSync` | VARINT | int32 | 0 | non | |
| 10 | `FrameRate` | VARINT | int32 | 0 | non | |
| 11 | `GameSpeed` | I32 | float | 0 | non | |
| 12 | `Volume` | I32 | float | 0 | non | |
| 13 | `RenderMode` | VARINT | enum | 0 | non | 0 GREEDY, 1 MARCHING, 2 GREEDY_MULTICOLOR |
| 14 | `SavedPattern3D` | LEN | `RecordedSlot` | rép. | oui | Préréglages de forme |
| 15 | `SaveAnimPattern3D` | LEN | `RecordedSlot` | rép. | oui | Préréglages de déformation |
| 16 | `ControlMethod` | VARINT | int32 | 0 | non | |
| 17 | `Shadow` | VARINT | bool | faux | non | |
| 18 | `SameBgPalette` | VARINT | bool | faux | oui | (SPEC-04) |
| 19 | `SpriteTolerance` | VARINT | int32 | 0 | oui | (SPEC-04) |
| 20 | `ScanDirection` | VARINT | int32 | 0 | oui | (SPEC-04) |
| 21 | `Skybox` | VARINT | int32 | -1 | non | |
| 22 | `Core` | VARINT | int32 | 0 | non | |
| 23 | `Mirroring` | VARINT | bool | vrai | non | |
| 24 | `EmptyTile` | VARINT | bool | faux | oui | (SPEC-04) |
| 25 | `LocalCam` | VARINT | bool | faux | oui | (SPEC-06) |
| 26 | `SceneRatio` | I32 | float | 1 | non | |
| 27 | `EmulatedMenuButton` | VARINT | bool | faux | non | |
| 28 | `RenderMethod` | VARINT | int32 | 0 | non | |
| 29 | `ResScaleIndex` | VARINT | int32 | 0 | non | |
| 30 | `SkipFrame` | VARINT | bool | vrai | non | |
| 31 | `BGMusic` | VARINT | bool | vrai | non | |
| 32 | `AntiAlias` | VARINT | int32 | 0 | non | |
| 33 | `AutoTVMode` | VARINT | bool | faux | non | |
| 34 | `TVMode` | VARINT | enum | 0 | oui | 0 NTSC, 1 PAL, 2 MULTI, 3 DENDY |
| 35 | `TVBox` | VARINT | int32 | 0 | non | |
| 36 | `SortOption` | VARINT | int32 | 0 | non | |
| 37 | `SameSpPalette` | VARINT | bool | faux | oui | (SPEC-04) |
| 38 | `Fov` | I32 | float | 70 | non | Jamais présent dans les profils embarqués |

`Light` : 1 `Direction` (`Vec3`), 2 `Color` (`Argb`), 3 `Intensity` (float). Seuls 1 et 2 sont appliqués.
`Layers` : 1 couches (`LayerParams`, répété, 6 éléments), 2 `DefLayer` (enum couche), 3 `Auto` (bool).
`LayerParams` : 1 `Offset` (float), 2 `Aligned` (enum : 0 FRONT, 1 CENTER, 2 BACK), 3 `CastShadow` (bool), 4 `RecvShadow` (bool) — 3 et 4 réécrasés à la lecture (REQ-PROF-042).
`RecordedSlot` : 1 nom (string), 3 pattern (`Pattern`, **message direct**, pas d'enveloppe). Le champ 2 n'existe pas.

### 4.7 Types de base

| Message | Champs |
|---|---|
| `IVec2` | 1 x (int32), 2 y (int32) |
| `Vec3` | 1 x, 2 y, 3 z (float, I32) |
| `Vec4` | 1 x, 2 y, 3 z, 4 w (float) |
| `Mat4` | 16 floats, champ `1 + 4×ligne + colonne` = élément (ligne, colonne), ligne et colonne dans 0..3 (champ 1 = m00, 2 = m01, 5 = m10, 16 = m33) |
| `Argb` | 1 valeur (int32) = `a<<24 | r<<16 | g<<8 | b` ; à la lecture la composante alpha est ignorée (couleur opaque) |

Ces mêmes types (couleur sous forme d'un int32 ARGB, vecteur sous forme de 3 floats) servent dans les autres fichiers protobuf de l'application (SPEC-09).

## 5. Vecteurs de test

### 5.1 Petits messages → octets

| Entrée | Octets (hex) |
|---|---|
| Début de tout profil : `Type="3dn"`, `Ver="2"` | `0a 03 33 64 6e 12 01 32` |
| `Script` partiel `name="Build"`, `enable=vrai`, `error=faux` | `0a 05 42 75 69 6c 64 10 01 18 00` |
| `Vec3(1,1,1)` en champ 8 | `42 0f 0d 00 00 80 3f 15 00 00 80 3f 1d 00 00 80 3f` |
| Cellule vide | `0b 0c` |
| Cellule référant la tile existante de clé 12 | `0b 0a 02 08 0c 0c` |
| Cellule, nouvelle tile clé 12 (plans §5.3) | `0b 0a 1c 10 0c 52 18 0a 16 08 b3 f1 a8 ce 80 d0 a5 89 62 10 b3 f1 b4 fa c0 d0 a5 89 62 18 00 0c` |
| Texture marqueur nul en champ 19 | `9a 01 08 52 02 10 00 08 00 10 00` |
| int32 `-16777216` (`Argb` noir opaque) | `80 80 80 f8 ff ff ff ff ff 01` |

### 5.2 Encodage des plans (K = `0x000708FACD884A5A`, rotation 26)

| Plan en clair | Valeur stockée (hex) | Stockée (int64 décimal) |
|---|---|---|
| `0x0000000000000000` | `0x6212968001C23EB3` | 7066876241827217075 |
| `0xFFFFFFFFFFFFFFFF` | `0x9DED697FFE3DC14C` | −7066876241827217076 |
| `0x0000000000000001` | `0x621296C001C23EB3` | 7066876516705124019 |
| `0x8000000000000000` | `0x621296A001C23EB3` | 7066876379266170547 |
| `0x00000000000000FF` | `0x6212A94001C23EB3` | 7066896857670237875 |
| `0x0102040810204080` | `0x6A02B68041433CB7` | 7638868579940777143 |
| `0x071F3F7F7FFFFFFF` | `0x9DED6941C60DE16C` | −7066876509057851028 |
| `0xC0F0F8FCFCFEFEFE` | `0x5DAD29303DFC018C` | 6750096703673336204 |
| `0xC3818100008181C3` | `0x4272E670E1A27EB3` | 4788142726344507059 |

Note : les valeurs stockées ≥ 2^63 sont écrites en int64 négatif (ex. `0x9DED6941C60DE16C` = 11379867564651700588 non signé = −7066876509057851028). Les trois dernières lignes sont les plans hauts des tiles de clés 2, 3 et 4 de `BalloonFight.bytes` (leurs plans bas valent 0).

### 5.3 Extrait décodé : `BalloonFight.bytes` (137 266 octets)

- `Type="3dn"`, `Ver="2"`, `Id="Lc89vRjlYrUL"`, pas de `Title`, 0 script.
- 237 patterns en champ 4, 2 en champ 5 ; 296 variantes ; 516 tiles distinctes (par identité d'objet).
- Pattern 0 : clé 1, `tSize=(8,1)`, 8 cellules non vides (clés 2 à 9).
- Pattern 1 : clé 11 ; `constraintSize=(99,99)`, `tSize=(1,1)`, `permanent=vrai`, `pixelCount=0`, `colorFlags=12`, `tDensity=5`, `pDensity=1`, `selfConnected=faux`, `skipGeo=faux`.
  - Cellule (0,0) : tile clé 12, stockées bas `7066876241961957555`, haut `7066876259234298035`, `flip=0` → plans en clair bas `0x0020201800000000`, haut `0x103A3C1800000000`, dessin (lignes y = 0..7, chiffres = couleur) :
    `00000000 / 00000000 / 00000000 / 00000000 / 00033000 / 00322200 / 00322020 / 00020000`.
  - Variante unique clé 13 : `sizeZ=3`, `geo=0`, `layer=0`, `scale=(1,1,1)`, `rot=(0,0,0)`, `offset=(0,0,0)`, `pivot=(0.5,0.5,0.5)`, `alpha=1`, `rm=0`, `deltaAmp=(0,0,0)`, `deltaPivot=(0.5,0.5,0.5)`, `deltaSpeed` = 16 zéros, `deltaOffset` = 16 × π/2 (float `0x3FC90FDB`), `renderStyle=1`, texture = marqueur nul, `TagStr=""`, `emptyColor=0`.
- Réglages : champs 1 à 37 présents (38 absent).

### 5.4 Extrait décodé : `empty.bytes` (3 780 octets, « 2D Mode »)

- `Ver="2"`, pas d'`Id`, 1 script (`Build`, `TagStr="*"`, `enable=vrai`), 0 pattern.
- Réglages présents : champs 1 à 23 seulement. `Light.Direction=(1, 0.2, 1)` (float le plus proche de 0.2), `Light.Color=-1` (blanc), `Light.Intensity≈0.85911`, `BgColor=vrai`, `CustomBgColor=-16777216`, `Clipping=(4.1, 16.1, 4.1, 0.1)`, `DefLayer=2`, `Auto=faux`, `GraphicQuality=2`, `FrameRate=120`, `Volume=1`, `RenderMode=0`, `Skybox=0`, `Core=2`, `Mirroring=vrai`.
- Couches (Offset, Aligned, Cast, Recv) stockées : (0,0,F,F) (0,0,F,F) (0,2,T,T) (0,0,F,T) (−4,0,F,T) (−4,0,F,T).
- `SavedPattern3D` noms : `Road, BG, Top, Loop, Body` ; `SaveAnimPattern3D` : `Deform1…Deform5`. Pattern du premier emplacement : `constraintSize=(99,99)`, `tSize=(0,0)`, `permanent=faux`, pas de grille, 1 variante, `selfConnected=vrai`.

### 5.5 Comptes globaux (101 profils embarqués)

| Grandeur | Valeur |
|---|---|
| Fichiers décodés sans erreur ni champ inconnu | 101 |
| Fichiers réécrits identiques octet par octet (REQ-PROF-051) | 101 |
| Version `"2"` / `"3"` | 100 / 1 |
| Profils avec scripts / scripts | 89 / 579 |
| Patterns (champ 4 / champ 5 / emplacements des réglages) | 66 609 / 1 223 / 1 010 |
| Variantes 3D (hors emplacements) | 74 389 |
| Tiles nouvelles / références à des tiles existantes | 225 575 / 226 656 |
| Textures marqueur nul / indexées / directes | 60 708 / 13 539 / 142 |
| Profils par ensemble de champs de réglages présents | {1..23} : 10 ; {1..15, 17..23} : 13 ; {1..24} : 3 ; {1..26} : 10 ; {1..32} : 9 ; {1..34} : 8 ; {1..36} : 29 ; {1..37} : 19 |

### 5.6 Petits profils (pour tests rapides)

| Fichier | Taille | Scripts | Patterns ch. 4 / ch. 5 | Variantes | Tiles distinctes |
|---|---|---|---|---|---|
| `empty.bytes` | 3 780 | 1 | 0 / 0 | 0 | 0 |
| `Puzznic (U).bytes` | 44 334 | 0 | 74 / 0 | 75 | 311 |
| `flappy_fpv.bytes` | 66 849 | 11 | 67 / 0 | 77 | 149 |
| `Super Mario Bros.bytes` | 179 515 | 10 | 257 / 1 | 296 | 783 |
| `Contra.bytes` | 1 120 081 | 3 | 1 854 / 65 | 1 966 | 5 365 |

## 6. Questions ouvertes

- **Q-01** — *Close par DEC-03-Q01* (détection depuis la ROM, puis `TVMode` du profil au chargement d'un jeu différent). `TVMode` fait partie des valeurs appliquées depuis le profil (REQ-PROF-041) alors que la région peut aussi être fixée automatiquement à partir de la ROM avant le chargement du profil ; l'ordre exact et l'effet sur le cœur sont à trancher avec SPEC-07 / SPEC-09.
- **Q-02** — *Close par DEC-03-Q02* : la proposition d'un tri ordinal est **rejetée** ; le tri est la collation non ordinale de SPEC-05 REQ-LUA-004 (l'ordre ordinal changerait l'exécution d'« Excitebike »). à confirmer par SPEC-05 pour les profils utilisateur.
- **Q-03** — *Close par DEC-03-Q03* (reconstitution, REQ-PROF-025). Profil sans listes 14/15 (aucun cas réel) : refuser (comportement d'origine) ou reconstituer 5 emplacements par défaut (`Config1..5`, `Deform1..5`) ? Proposition : reconstituer, car c'est sans effet hors éditeur.
- **Q-04** — *Close par DEC-03-Q04* (édition `full` seule). L'édition `demo` du catalogue n'est pas utilisée ; à confirmer par R0 qu'aucune variante « démo » n'est attendue.

## Historique

- 1.1-draft (AMD-001, relecture R2) : critères de REQ-PROF-001/051 rendus exécutables (DEC-R2-T7) ; REQ-PROF-025 accepte l'absence des listes 14/15 (DEC-03-Q03, D) ; tri des scripts renvoyé à REQ-LUA-004 ; REQ-PROF-003 et 052 précisées ; noms de messages déclarés non normatifs ; critère de REQ-PROF-064 sans ROM commerciale ; questions closes.
