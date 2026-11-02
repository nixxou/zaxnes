# AMD-050 — Mode 2D : les pixels de fond universel sont transparents

Version : 1.1 | 2026-10-31 | **Normatif** (R1, à la suite de la comparaison R5 avec 3dSen 2.0). Complète AMD-009 (réponse à Q-022) et AMD-046 (REQ-SCN-074) ; **amende SPEC-02 REQ-CORE-030** (contenu d'un lot de frame) **et REQ-CORE-048** (image de pixels exigée) ; ajoute REQ-CORE-049 (masque d'opacité) et le vecteur V11 à SPEC-02 §5.3.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-050-053 : 50-1 à 50-9, tous retenus ; pour 50-1, option (a) — masque d'opacité de 256 × 240 bits exigé à côté de l'image de pixels, amendant REQ-CORE-048 et le contenu du lot de REQ-CORE-030 ; pour 50-7, l'éclairage du rectangle est tranché plutôt qu'un ticket Q : éclairage de tile, REQ-SCN-028).
Priorités : REQ-CORE-048 amendée (P1) est acquise indépendamment du reste (le mode 2D a besoin de l'image de pixels, transparence ou non). REQ-SCN-076 (P2) dépend de REQ-CORE-049 (P2) : si REQ-CORE-049 n'est pas retenue, REQ-SCN-076 ne l'est pas non plus et le rectangle du mode 2D reste opaque, comme dans AMD-009 et AMD-046.

## 1. Besoin

AMD-009 (Q-022) affiche en mode 2D l'image en pixels du cœur plaquée sur le plan NES, écart accepté
(D) par rapport au programme d'origine qui convertit les tiles. Dans le programme d'origine, les
pixels de valeur 0 d'une tile ou d'un sprite n'ont pas de géométrie : là où l'image NES montre la
couleur de fond universelle, on voit la scène derrière (couleur de fond de REQ-SCN-046, skybox).
Vue de face, la différence ne se voit presque pas ; avec la courbure (AMD-046) ou une caméra
inclinée, l'image plaquée de Zaxnes devient un écran bleu opaque, éclairé et incurvé, alors que le
programme d'origine ne montre que les décors et personnages courbés sur le fond de la scène.

La discriminante est la **valeur** des pixels de fond et de sprite, pas la couleur affichée. Or
l'image de pixels de REQ-CORE-048 ne porte que des **indices de couleur** : un pixel transparent
(fond universel faute de pixel opaque) et un pixel opaque dont la couleur de palette est celle du
fond universel y portent le même indice (REQ-E3D-561 : l'entrée 0 de toute table de palette est la
couleur de fond universelle). L'amendement emporte donc la modification de l'interface du cœur qui
transporte cette information.

## 2. Exigences

### 2.1 Interface du cœur (SPEC-02 §3.5)

**REQ-CORE-048 amendée** (P1) — **Image de pixels.** Le cœur DOIT produire, pour chaque frame
émulée, une image 2D de 256 × 240 pixels portant, pour chaque pixel, l'**indice de couleur NES**
affiché par le PPU (0 à 63) ; là où le PPU n'affiche aucun pixel opaque, c'est l'indice de l'entrée
0 de la palette (la couleur de fond universelle). L'image est transmise avec le lot de frame
(REQ-CORE-030) et n'est valide que pendant l'appel.
- Critère d'acceptation : vecteur V11 (§5.3) ; en mode 2D, l'image plaquée sur le rectangle
  (AMD-009 réponse à Q-022, AMD-046 §2) est identique, pixel pour pixel, à l'image NES de
  référence de la même frame.
- Notes : la mention « Le cœur **PEUT** produire en plus […] Cette image n'est pas exigée » de la
  v1.1 de SPEC-02 est retirée, et la priorité passe de P3 à P1. Le mode 2D (REQ-SCN-055, P1, lu
  avec Q-022 et AMD-046) repose sur cette image : elle n'est plus facultative dès lors que le mode
  2D est dans le périmètre. Elle sert aussi au mode de débogage, usage pour lequel elle avait été
  introduite (AMD-009).

**REQ-CORE-049** (P2) — **Masque d'opacité.** Le cœur DOIT produire, pour la même frame et à côté
de l'image de REQ-CORE-048, un **masque d'opacité** de 256 × 240 **bits**, un bit par pixel. Le bit
vaut **1** quand le pixel porte au moins un pixel opaque, c'est-à-dire quand au moins l'une des deux
conditions suivantes est vraie :

- un pixel de **fond** de valeur ≠ 0 y est produit, le fond étant rendu (bit 3 de PPUMASK à 1) et,
  pour les colonnes x < 8, non masqué (bit 1 de PPUMASK à 1) ;
- un pixel de **sprite** de valeur ≠ 0 y est produit, quel que soit son drapeau « caché »
  (REQ-CORE-042), les sprites étant rendus (bit 4 de PPUMASK à 1) et, pour les colonnes x < 8, non
  masqués (bit 2 de PPUMASK à 1).

Il vaut **0** sinon. L'état de PPUMASK retenu est celui en vigueur au moment où le PPU produit le
pixel (PPUMASK peut changer en cours de frame). Le masque n'est valide que pendant l'appel du rappel
vidéo (REQ-CORE-030).
- Disposition recommandée, utile seulement pour une interface native (libretro, bibliothèque C) :
  32 octets par ligne d'image, lignes de haut en bas ; dans un octet, le bit de poids faible est le
  pixel le plus à gauche ; 7 680 octets au total. Cette disposition n'est pas exigée pour une
  interface interne .NET.
- Critère d'acceptation : vecteur V11 (§5.3).
- Notes : le masque porte exactement l'information que l'image d'indices ne porte pas. `maskX` et
  `maskY` (REQ-CORE-046) sont sans rapport : ils signalent des bandes de bord **peu fiables** à
  l'usage de SPEC-04 et ne rendent aucun pixel transparent. Le cas matériel du « fond de palette »
  (rendu désactivé avec l'adresse PPU dans `$3F00`–`$3FFF`, où le PPU affiche une entrée de palette
  autre que l'entrée 0) est traité comme tout rendu désactivé : bit à 0.

### 2.2 Rendu du rectangle du mode 2D (SPEC-06 §3.16)

**REQ-SCN-076** (P2) — **Transparence du fond universel en mode 2D** : dans l'image du rectangle du
mode 2D (AMD-046 §2), un pixel DOIT être transparent (non dessiné) quand le bit correspondant du
masque d'opacité du cœur (REQ-CORE-049) vaut 0, et dessiné quand il vaut 1. Un pixel opaque de même
couleur que le fond universel reste donc dessiné. La transparence vaut que la courbure
(REQ-SCN-074) soit active ou non ; le rectangle ne projette ni ne reçoit d'ombre (AMD-046 §2, qui
fait foi ; la transparence n'y change rien).

Le pixel transparent DOIT être obtenu par **rejet du fragment**, dans le passage des objets opaques
de REQ-SCN-058 : le rectangle n'entre pas dans la classe transparente de REQ-SCN-040 (ni tri, ni
passage de profondeur séparé, ni mélange alpha), et le résultat NE DOIT PAS dépendre de
`video.anti_alias` (AMD-032 §2, dont AMD-052 fait le défaut).

Le rectangle est **éclairé comme une tile** (REQ-SCN-028, albédo = couleur du pixel).
- Critère d'acceptation (auto) : harnais, mode 2D, rectangle plat, mise en condition
  `scene.use_nes_bg_color` = faux et `Setting.CustomBgColor` = magenta (`#FF00FFFF`) — les réglages
  par défaut ne permettent pas d'observer la transparence, voir les Notes. Image synthétique dont
  une tile de fond a deux pixels de valeur 0 et deux de valeur 3, l'entrée de palette 3 étant égale
  à l'entrée 0 : les deux pixels de valeur 0 donnent la couleur de fond de la scène (magenta, ±2
  niveaux sRGB) ; les deux pixels de valeur 3 sont dessinés avec la couleur du fond universel.
- Critère d'acceptation (auto) : même mise en condition. Bits 3 et 4 de PPUMASK à 0 : la capture est
  exactement la couleur de fond de la scène, sur les 256 × 240 pixels du rectangle. Bit 1 de PPUMASK
  à 0, fond rendu, fond de la tile non nul : les 8 colonnes de **gauche** sont transparentes et le
  reste du rectangle est inchangé ; avec `maskX` = 8 (REQ-CORE-046, mirroring horizontal), les 8
  colonnes de droite et les bords haut et bas restent dessinés.
- Critère d'acceptation (auto) : même scène rendue avec `video.anti_alias` = 0, 1 et 2 : l'ensemble
  des pixels dessinés du rectangle est identique dans les trois cas, et aucun pixel de bord de
  sprite n'est partiellement transparent.
- Critère d'acceptation [ROM] : « Super Mario Bros. », niveau 1-1 en mode 2D, Skybox = `Color`,
  `Setting.CustomBgColor` = magenta, Courbure gauche-droite à 90° : au pixel central d'une zone de
  ciel, la couleur lue vaut la couleur personnalisée à ±2 niveaux sRGB ; les pixels de la barre de
  statut (noir opaque, indice `$0F`) restent dessinés. Contrôle visuel : le ciel bleu n'est pas
  dessiné sur le rectangle, on voit la scène derrière, et seuls le décor, les personnages et le
  texte sont courbés.
- Notes :
  - Vue de face, sans courbure et avec la couleur de fond de la scène égale au fond NES, l'image
    reste visuellement inchangée et la transparence n'est **pas** observable. C'est le cas de la
    configuration par défaut : REQ-PERS-005 point 3 force `scene.use_nes_bg_color`
    (`Setting.BgColor`) à vrai à chaque chargement de profil, et le profil « 2D Mode » est rechargé
    à chaque lancement (REQ-PERS-006). D'où la mise en condition explicite des critères ci-dessus.
  - Quand le fond et les sprites sont tous deux désactivés (bits 3 et 4 de PPUMASK à 0 : blanking
    forcé, fréquent pendant les écritures VRAM de transition), le rectangle entier est transparent
    et seule la scène est visible. C'est le comportement attendu — le programme d'origine n'a alors
    aucune géométrie non plus — et le rectangle peut donc clignoter sur ces frames.
  - L'éclairage du rectangle n'était fixé ni par REQ-SCN-055, ni par Q-022, ni par AMD-046 §2. Il
    est tranché ici : éclairage de tile (REQ-SCN-028), qui est celui du rendu décrit au §1. Le
    modèle Standard réservé au backdrop (AMD-049 REQ-SCN-075) ne s'applique pas au rectangle.

## 3. Changements documentaires

1. SPEC-02 REQ-CORE-030 : deux points sont ajoutés à la liste du contenu d'un lot de frame :
   « 4. l'**image de pixels** de 256 × 240 indices de couleur NES (REQ-CORE-048) ;
   5. le **masque d'opacité** de 256 × 240 bits (REQ-CORE-049) ». La phrase « Les données ne sont
   valides que pendant l'appel » les couvre comme les autres.
2. SPEC-02 §3.5.5 REQ-CORE-048 : énoncé, priorité (P3 → P1) et critère d'acceptation remplacés par
   ceux du §2.1.
3. SPEC-02 §3.5.5 : REQ-CORE-049 ajoutée après REQ-CORE-048.
4. SPEC-02 §5.3, vecteur ajouté (conventions de §5.1 et §5.3 ; la tile 1 de la table `$0000` a tous
   ses pixels non nuls : valeur 2 pour les colonnes 0 à 6 de chaque rangée, valeur 1 pour la
   colonne 7) :
   - **V11 — image de pixels et masque d'opacité.** Nametable `$2000` entièrement remplie de la
     tile 1, OAM à `$FF`.
     - PPUMASK = `$0A` (fond rendu, colonne de gauche du fond affichée, sprites éteints) → masque
       entièrement à 1 (7 680 octets à `0xFF`) ; image = indices des entrées de palette 1 et 2.
     - Palette modifiée en `$3F00` ← `$0F`, `$3F01` ← `$0F`, `$3F02` ← `$0F` (les trois couleurs
       égales à la couleur de fond universelle) → l'**image** est uniformément `$0F`, le **masque**
       reste entièrement à 1. C'est le cas que l'image seule ne permet pas de distinguer.
     - PPUMASK = `$08` (bit 1 à 0 : 8 pixels de gauche du fond masqués) → dans chaque ligne du
       masque, les 8 premiers bits valent 0 et les 248 autres 1.
     - PPUMASK = `$00` → masque entièrement à 0 ; image uniformément égale à l'indice de l'entrée 0.
     - Nametable remise à la tile 0 (motif nul), PPUMASK = `$1E`, OAM sprite 0 = (Y `$20`, T `$01`,
       A `$00`, X `$10`) → masque à 1 exactement sur le rectangle x ∈ [16, 23], y ∈ [33, 40], à 0
       partout ailleurs.
5. SPEC-06 §3.16 : REQ-SCN-076 ajoutée après REQ-SCN-055 (le rectangle du mode 2D est défini par
   AMD-046 §2).
6. SPEC-v1.2-INDEX : REQ-CORE-049 ajoutée à la liste de SPEC-02 (§3.5), REQ-SCN-076 à celle de
   SPEC-06 (§3.16), et la priorité de REQ-CORE-048 mise à jour (P3 → P1).

## 4. Questions

- Aucune.
