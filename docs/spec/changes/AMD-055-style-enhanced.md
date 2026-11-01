# AMD-055 — Style visuel « Enhanced »

Version : 1.5 | 2026-11-01 | **Normatif** (R1, choix du propriétaire du 2026-10-31 : reprise des pistes §3.1, 3.2 et 3.3 du rapport de comparaison avec 3dSen 2.0). **Amende AMD-040** (§2 : ligne `VisualStyle` de REQ-LUA-114 et note sur les défauts ; §3 : paramètres du style Enhanced, identifiant du style, ligne SPEC-09 de `render.visual_style`), **AMD-043** (§2 REQ-SCN-066 et colonne Lua du §6 : les réglages de courbure agissent dans les deux styles), **AMD-049** (REQ-SCN-075 : phrase sur la grille de voxels), **SPEC-06 REQ-SCN-058** (ordre de rendu d'une frame) et **SPEC-10 REQ-UI-061** (liste des paramètres visibles sur desktop). Complète SPEC-06 et SPEC-09.
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-055 : 55-1 à 55-24 ; décisions du propriétaire du 2026-10-31 : la grille de voxels s'applique au backdrop et se calcule dans le repère local du maillage avant transformation d'objet et avant courbure ; REQ-SCN-083 et REQ-SCN-084 ramenées à une intention paramétrée, en P3, avec des critères de revue ; clés de réglages `render.style.*`, non exposées en libretro ; lecture A pour la courbure en Classic) ; 1.2 (seconde relecture R2-review-AMD-055-v1.1 : N-1 à N-8 ; décisions du propriétaire du 2026-10-31 : **références publiques orientantes** pour REQ-SCN-083 — occlusion ambiante d'écran par hémisphère — et pour REQ-SCN-084 — tracé de rayon dans l'espace écran par marche puis dichotomie ; **mode MSVO** porté de l'implémentation publique sous licence MIT du MiniEngine de Microsoft, avec repli déclaré sur Depth ; N-1 traité par la mesure sur la face avant du backdrop, position fixée ; N-2 aligné sur la lecture de REQ-SCN-084 ; N-3 à N-8 retenus). ; 1.3 (R2-review-AMD-055-v1.2 : N2-1, plaque de `sizeZ` = 1 dans le critère de REQ-SCN-082 ; N2-2, normales plates en MARCHING, REQ-E3D-522) ; 1.4 (vérification R5 du 2026-11-01, Q-094, Q-096, Q-097 : plafond d'assombrissement de REQ-SCN-083 relevé à `min(0,95 ; 0,3 × ao.strength)` d'après la mesure de 3dSen 2.0, invariant de contact ajusté ; verticale de la définition du sol de REQ-SCN-084 en mode caméra locale : normale du plan image) ; 1.5 (Q-099 : intensité des reflets de REQ-SCN-084 divisée par deux, `0,045 × n(ssr.shine)`, d'après la mesure de 3dSen 2.0).
Priorités : REQ-SCN-078 à REQ-SCN-082 sont **P2**, REQ-SCN-083 et REQ-SCN-084 sont **P3**. Si REQ-SCN-078 n'est pas retenue, le style reste sans effet de rendu (AMD-040 REQ-LUA-114 inchangée) et rien de cet amendement ne s'applique. Si une exigence de §2.3 à §2.5 n'est pas retenue, l'effet correspondant n'existe pas, son réglage reste persisté et exposé sans effet (cas analogue à SPEC-09 REQ-PERS-017, §3 point 7) et les autres effets sont inchangés. Si REQ-SCN-081 n'est pas retenue, la phrase d'AMD-049 REQ-SCN-075 sur la grille de voxels reste valable (T-1). Si REQ-SCN-083 et REQ-SCN-084 ne sont retenues ni l'une ni l'autre, l'ordre de frame de REQ-SCN-058 reste celui de SPEC-06, sans les deux passes d'écran (T-1). Le **portage MSVO** de REQ-SCN-083 est lui aussi **P3** : s'il n'est pas retenu, ou tant qu'il n'est pas disponible, `spl.ao` = 1 est rendu comme Depth **(D)** et le reste de REQ-SCN-083 est inchangé (T-1).

## 1. Besoin

3dSen 2.0 propose deux styles de rendu : **Classic** (le rendu de la 1.x, décrit par SPEC-06) et
**Enhanced**. Enhanced garde l'éclairage de Classic et y ajoute, chacun désactivable : une grille
de voxels, des ombres qui suivent la grille de pixels du jeu, une occlusion ambiante d'écran et des
reflets du sol. Le style et ses réglages sont globaux (choix de l'utilisateur, jamais pris dans le
profil) et s'appliquent à chaud. Zaxnes ne dispose aujourd'hui que de Classic ; AMD-040 a gardé le
réglage `VisualStyle` sans effet de rendu.

Les deux effets d'écran du programme d'origine reposent sur des techniques publiques et classiques
(occlusion ambiante par échantillonnage d'un hémisphère à partir de la profondeur, reflets tracés
dans l'espace écran). Le propriétaire a tranché de **ne pas** en fixer l'algorithme : REQ-SCN-083 et
REQ-SCN-084 énoncent une **intention**, des **paramètres normatifs** et des **invariants
vérifiables**, laissent le rendu exact à l'implémenteur, et sont en **P3** ; leur aspect est jugé par
revue visuelle (catégorie **revue** de SPEC-12). La grille de voxels et les ombres pixel, elles, sont
entièrement spécifiées et restent en P2.

**Références publiques** (décision du propriétaire du 2026-10-31). Pour épargner à l'implémenteur
le choix d'une famille de techniques, les deux exigences **nomment** une technique publique de
référence : REQ-SCN-083 DEVRAIT suivre l'occlusion ambiante d'écran **par hémisphère** telle qu'elle
est décrite publiquement (par exemple le tutoriel SSAO de **John Chapman, 2013**) et REQ-SCN-084
DEVRAIT suivre le **tracé de rayon dans l'espace écran par marche linéaire puis affinement par
dichotomie** (famille décrite par **McGuire et Mara, « Efficient GPU Screen-Space Ray Tracing »,
JCGT 3(4), 2014**). Ces références **orientent sans être normatives** : elles ne changent ni la
priorité, ni la liberté d'implémentation, et ce sont les **paramètres et les invariants** de
l'amendement qui font foi.

L'occlusion « MSVO » du programme d'origine repose sur une technique tierce que cette
réimplémentation ne peut pas reprendre telle quelle. Le libellé `MSVO` reste exposé, parce que c'est
une valeur visible du réglage, qui doit se relire ; le mode est rendu, lui, par une occlusion
ambiante d'écran **multi-échelle** portée d'une implémentation publique sous **licence MIT**
(REQ-SCN-083, *Mode MSVO*), et **à défaut** comme l'occlusion « Depth » — repli déclaré **(D)**.

## 2. Exigences

### 2.1 Réglage du style

**REQ-SCN-078** (P2) — **Styles.** `render.visual_style` (AMD-040 §3) DOIT désormais choisir le
rendu : **Classic** (rendu de SPEC-06, inchangé) ou **Enhanced** (§2.2 à 2.5). L'identifiant du
style Enhanced est **4** dans le programme d'origine ; Zaxnes DOIT accepter 4 et 1 comme
identifiants d'Enhanced, en écriture (`Setting.VisualStyle`, réglage) comme en argument des
méthodes `Get/Set/ResetVisualStyleParameter(s)` (REQ-LUA-114), et DOIT renvoyer **4** en lecture de
`Setting.VisualStyle` quand Enhanced est actif ; 0 reste Classic ; toute autre valeur écrite donne 0,
y compris quand elle vient du fichier de réglages (SPEC-09 REQ-PERS-003, règle de la colonne
« Domaine » du §4). Le changement de style ou d'un de ses réglages s'applique à la frame rendue
suivante, sans recharger le jeu. Les réglages du style ne viennent jamais du profil.

La **courbure** d'AMD-043 (REQ-SCN-066 à REQ-SCN-068) n'est **pas** un effet du style : elle agit
en Classic comme en Enhanced **(D)**, de sorte qu'un changement de style ne fasse pas perdre un
réglage de courbure (§3 point 4).
- Critère d'acceptation (auto) : `Setting.VisualStyle = 4` → relu 4 ; `= 1` → relu 4 ;
  `Setting:SetVisualStyleParameter(4, "spl.bend_zx", 30)` → vrai ; `Setting:SetVisualStyleParameter(1,
  "ao.strength", 5)` → vrai ; `= 2` → relu 0 ; fichier de réglages contenant `render.visual_style = 1`
  → relu 4, `= 7` → relu 0.
- Critère d'acceptation (auto) : `Setting.VisualStyle = 0` avec Courbure gauche-droite à 60° : la
  scène reste courbée (sommets exportés à B(P) près, AMD-043 REQ-SCN-067).

**REQ-SCN-079** (P2) — **Paramètres d'Enhanced** (portée U, persistés sous les clés du §4,
normalisation d'AMD-040 §3 : valeur bornée puis arrondie au pas, pas 1) ; ils s'ajoutent aux quatre
paramètres de courbure d'AMD-043 :

| Id | Libellé (EN / FR) | Valeurs | Défaut |
|---|---|---|---|
| `spl.ao` | Ambient Occlusion / Occlusion ambiante | 0 Off, 1 MSVO (occlusion multi-échelle de REQ-SCN-083 ; rendue comme Depth tant que son portage n'est pas disponible, **D**), 2 Depth | 0 |
| `ao.strength` | Strength / Force | 1 à 5 | 3 |
| `ao.radius` | Radius / Rayon | 1 à 5 | 3 |
| `ao.thickness` | Thickness / Épaisseur | 1 à 5 (sans effet observable hors du mode MSVO, voir ci-dessous) | 3 |
| `ao.quality` | AO Quality / Qualité de l'occlusion | 0 Low, 1 Medium, 2 High | 1 |
| `spl.pixel_shadow` | Pixel Shadows / Ombres pixel | 0 Off, 1 On | 0 |
| `spl.voxel_grid` | Voxel Grid / Grille de voxels | 0 Off, 1, 2, 3 — côté de cellule `N = 2^spl.voxel_grid` voxels (1 → 2, 2 → 4, 3 → 8) | 0 |
| `spl.reflections` | Reflections / Reflets | 0 Off, 1 On | 0 |
| `ssr.shine` | Floor Shine / Brillance du sol | 1 à 5 | 3 |
| `ssr.clarity` | Clarity / Netteté | 1 à 5 | 3 |
| `ssr.length` | Length / Longueur | 1 à 5 | 3 |
| `ssr.quality` | Reflection Quality / Qualité des reflets | 0 Low, 1 Medium, 2 High, 3 Ultra | 2 |

Ces douze identifiants sont normatifs parce qu'ils sont des **arguments de l'API Lua**
(REQ-LUA-114) et, pour cette raison, des faits d'interopérabilité ; les autres identifiants de
paramètres de 3dSen 2.0 restent « inconnus » au sens d'AMD-040 §3.

`ao.thickness` est conservé, persisté et exposé. Il **n'a aucun effet observable** tant que le mode
MSVO est rendu comme Depth (repli **(D)** de REQ-SCN-083), parce que l'occlusion Depth ne l'utilise
pas ; il est alors présenté **grisé (D)**, comme les réglages de SPEC-09 REQ-PERS-017 — par analogie
seulement, puisque REQ-PERS-017 énumère des réglages sans effet **dans l'original**, ce qui n'est pas
le cas ici. Quand le portage MSVO est disponible, `ao.thickness` **agit** en mode MSVO
(REQ-SCN-083) et n'est plus grisé pour `spl.ao` = 1.

Présentation (voir §3 point 6 et §4) : les paramètres sont présentés dans la grille de paramètres
desktop (SPEC-10 REQ-UI-061 amendée), à la suite de la ligne `Style`, dans un groupe repliable
« Style » ; les réglages d'occlusion ne sont visibles que si `spl.ao` ≠ 0, ceux des reflets que si
`spl.reflections` = 1. Aides (EN / FR) : occlusion « Soft shade in corners and hollows. » /
« Ombre douce dans les coins et les creux. » ; ombres pixel « Shadows that follow the game's pixel
grid. » / « Des ombres qui suivent la grille de pixels du jeu. » ; grille « Grooves every 2, 4 or 8
voxels, like toy bricks. » / « Des rainures tous les 2, 4 ou 8 voxels, comme des briques de
jouet. » ; reflets « The floor reflects the scene like a polished surface. » / « Le sol reflète la
scène comme une surface polie. ».
- Critère d'acceptation (auto) : chaque paramètre, **écrit depuis l'UI ou depuis le modèle de
  réglages**, survit à un redémarrage ; `ao.strength` écrit à 9 → 5, à 1,4 → 1, à 0 → 1 ;
  `spl.voxel_grid` écrit à 4 → 3 ; `spl.ao` écrit à 1,4 → 1.
- Critère d'acceptation (auto) : une écriture **par Lua** suit SPEC-09 REQ-PERS-010 (couche de
  session) : `Setting:SetVisualStyleParameter(4, "ao.strength", 5)` pendant une partie, puis fin de
  partie → `ao.strength` revient à la préférence de l'utilisateur.
- Critère d'acceptation (auto) : changer `ao.thickness` de 1 à 5, `spl.ao` = **2**, tous les autres
  réglages fixés : la capture est identique au pixel près. (En mode MSVO effectif, c'est le critère
  inverse qui s'applique, REQ-SCN-083.)

### 2.2 Éclairage de base et ordre des effets

**REQ-SCN-080** (P2) — En Enhanced, l'éclairage des tiles, du backdrop et du HUD est celui de
Classic (tiles : REQ-SCN-028 ; backdrop : REQ-SCN-075 ; HUD : REQ-SCN-029 pour l'éclairage et
REQ-SCN-044 pour le rendu), auquel s'appliquent les effets de §2.3 à 2.5 quand ils sont activés.
Tous les effets désactivés, une capture Enhanced est identique à une capture Classic.

**Ordre des calculs**, par analogie avec la clause d'ordre d'AMD-043 REQ-SCN-068 :
- la grille de voxels (REQ-SCN-081) et les ombres pixel (REQ-SCN-082) sont évaluées **avant la
  courbure**, dans le repère local du maillage (REQ-E3D-504, REQ-E3D-522) pour les tiles et dans le
  repère de scène pour le backdrop : rainures et carrés d'ombre **suivent** donc la surface courbée,
  comme ils suivent l'échelle et la rotation de l'objet ;
- l'occlusion ambiante (REQ-SCN-083) et les reflets (REQ-SCN-084) travaillent sur la profondeur et
  la couleur **rendues**, donc **après** la courbure ; le test de sol et l'étendue de REQ-SCN-084
  sont en revanche évalués sur la normale et la boîte englobante **avant** courbure, de sorte
  qu'une courbure forte n'interrompe pas un reflet en cours de surface ;
- l'ordre de frame de REQ-SCN-058 devient : carte d'ombre ; effacement ou skybox ; objets opaques ;
  **reflets du sol (REQ-SCN-084)** ; **occlusion ambiante (REQ-SCN-083)** ; objets transparents
  triés ; HUD ; puis l'UI (§3 point 3) ;
- **ce qui occulte et ce qui est reflété** découle de cet ordre, et de lui seul. Les **objets
  transparents** (REQ-SCN-040) reçoivent la grille de voxels mais **pas** les ombres pixel (ils ne
  reçoivent déjà pas d'ombre, REQ-SCN-032) ; comme ils sont dessinés **après** les deux passes
  d'écran — leur passage de profondeur compris, la passe transparente étant indivisible par objet
  (REQ-SCN-040 lu avec AMD-012) —, ils **n'occultent pas** pour l'occlusion ambiante, ne sont **pas
  reflétés**, et ne sont ni assombris par l'occlusion ni porteurs de reflets. Le **HUD**, dessiné
  encore après, non plus. La **skybox**, dessinée avant les objets opaques, **peut être reflétée**
  mais **n'occulte jamais** : elle est à distance infinie, derrière toute la scène (REQ-SCN-049),
  donc jamais à moins de `2 r` d'un point de la scène. Le **tunnel**, les **modèles de téléviseur**
  (REQ-SCN-053, REQ-SCN-054) et le **rectangle du mode 2D** (AMD-046 §2), dessinés avec les objets
  opaques, **occultent** et **peuvent être reflétés**, sans être eux-mêmes modifiés par les deux
  effets. Les tiles de style « **ombre seule** » (REQ-SCN-041), qui ne sont pas dessinées, n'écrivent
  aucune profondeur : elles n'occultent pas et ne sont pas reflétées, mais elles projettent leur
  ombre comme en Classic ;
- en Enhanced, la scène PEUT être rendue dans une **cible intermédiaire** puis composée dans la
  cible fournie par l'hôte ; celle-ci n'est jamais relue (REQ-SCN-057). Les deux effets d'écran
  travaillent sur la couleur et la profondeur **résolues** (après résolution du MSAA éventuel de
  `video.anti_alias`, AMD-032 §2) et sont appliqués à la résolution de la cible de rendu courante,
  elle-même éventuellement réduite par `video.res_scale` (REQ-SCN-063).
- Critère d'acceptation (auto) : même scène, Classic et Enhanced tous effets à Off : captures
  identiques à ±2 niveaux sRGB par canal.
- Critère d'acceptation (auto), **clause d'ordre vis-à-vis de la courbure** : **cadrage large B** de
  REQ-SCN-081, Courbure gauche-droite (`spl.bend_zx`) à **90°**. La position d'une ligne de rainure
  est relevée, colonne de pixels par colonne de pixels, au **minimum local de luminance** ; sa corde
  joint ses deux extrémités dans la capture. Chaque ligne de rainure **horizontale** de la face avant
  du backdrop située à au moins 8 et au plus 56 unités du centre C — soit `y` = 64, 72, …, 112 et
  `y` = 128, 136, …, 176 — est **courbée** : son écart maximal à sa corde dépasse **15 pixels**, et
  vaut `2,661 × |y − 120|` pixels, soit 21, 43, 64, 85, 106, 128 et 149 px pour `|y − 120|` = 8, 16,
  24, 32, 40, 48 et 56 (tolérance : 3 pixels ou 3 %, la plus grande des deux). Le coefficient se
  recalcule ainsi, avec REQ-SCN-067 (`L` = 256, `s` = 1, θ = 90°, donc `k` = 0,0061359,
  `f` = 1,026175 et `h` = 12,4041) et une focale de `(1080/2)/tan 35° = 771,2` pixels : la face
  avant (`w` = −0,5) passe de `z` = −13,242 en son centre à `z` = +35,892 à ses bords (`u` = ±128,
  portés à `u′` = ±118,62), soit des profondeurs de 97,258 et 146,392 depuis la caméra, donc 7,929
  et 5,268 pixels par unité de `|y − 120|` ; l'écart à la corde est leur différence, 2,661. À
  courbure nulle, la même ligne est **droite** à moins de 1 pixel. La rainure horizontale qui passe
  par le centre (`y` = 120) et les rainures **verticales** restent droites dans les deux captures et
  sont exclues de la mesure : l'enroulement gauche-droite de REQ-SCN-067 laisse la coordonnée `v`
  inchangée, une ligne à `u` constant reste donc un segment droit dans la scène. Voir la Note
  « portée du critère de courbure » de REQ-SCN-081 pour ce que ce critère discrimine.

### 2.3 Grille de voxels et ombres pixel

**REQ-SCN-081** (P2) — **Grille de voxels** (`spl.voxel_grid` ≠ 0 ; côté de cellule
`N = 2^spl.voxel_grid` voxels, soit 2, 4 ou 8). La grille module **l'albédo** des faces des
maillages de tiles et des faces du pavé du backdrop, avant tout calcul d'éclairage (REQ-SCN-028
pour les tiles ; terme `c` de REQ-SCN-075 pour le backdrop, de sorte que la rainure n'assombrisse
pas le terme spéculaire).

**Repère.** Pour une tile, la position `p` utilisée est celle du point dans le **repère local du
maillage de la tile** (SPEC-04B REQ-E3D-504 pour GREEDY et GREEDY_MULTICOLOR, REQ-E3D-522 pour
MARCHING — les deux repères coïncident : `x`, `y` ∈ [−4, 4], `z` ∈ [−D/2, D/2], `D` étant la
profondeur de construction), prise **avant** la transformation d'objet (déformation animée, échelle,
rotation, position, REQ-E3D-406) et **avant** la courbure : la grille suit donc l'objet. Les **arêtes
de cellule** sont les coordonnées multiples de `N` dans ce repère : pour `N = 8` et `N = 4`, une
arête passe par le centre de la tile ; pour `N = 2`, les arêtes sont aux coordonnées paires. Pour le
backdrop, `p` est la position du point sur le pavé en **unités de scène** (repère de scène de
REQ-E3D-410 multiplié par l'échelle de scène, avant courbure) et les arêtes sont aux coordonnées
multiples de `N × échelle de scène`.

**Modulation.** Soit `n` la normale unitaire du maillage au point considéré (normale de la face en
GREEDY et GREEDY_MULTICOLOR, normale plate du triangle en MARCHING (REQ-E3D-522)), exprimée dans le même
repère que `p`. On pose, pour chacun des trois axes `a` :

- poids `w_a = clamp(4 × (1 − |n_a|), 0, 1)` : un axe perpendiculaire à la face a un poids nul et
  **ne crée aucune rainure** ; les axes du plan de la face ont un poids 1 ;
- distance à l'arête la plus proche, en **fraction de cellule** :
  `d_a = min_{j entier} |p_a − j·N| / N`, donc `d_a ∈ [0 ; 0,5]` ;
- **alignement sur un axe** : avec `m = max_a |n_a|` et `t = clamp((m − 0,8) / 0,1, 0, 1)`, on pose
  un facteur de rainure `k = 0,38 + 0,24 · t` et un éclaircissement maximal `e = 0,20 − 0,10 · t`.
  Une face alignée sur un axe (`m ≥ 0,9`, donc `t` = 1 : toutes les faces GREEDY, REQ-E3D-505)
  donne `k = 0,62` et `e = 0,10` ; une face à 45° ou plus oblique (`m ≤ 0,8`, donc `t` = 0) donne
  `k = 0,38` et `e = 0,20` ; entre les deux, `k` et `e` sont interpolés linéairement en `t` ;
- facteur de l'axe `a` :
  - **rainure**, pour `d_a ≤ 0,035` (bande de 3,5 % de la cellule de part et d'autre de l'arête) :
    `f_a = 1 − w_a · (1 − k)` ;
  - **arête claire**, pour `0,035 ≤ d_a ≤ 0,16` :
    `f_a = 1 + w_a · e · (0,16 − d_a) / 0,125`, maximale au bord de la rainure et décroissant vers
    l'intérieur de la cellule, où elle atteint exactement 1 en `d_a = 0,16` ;
  - `f_a = 1` ailleurs ;
  - la transition en `d_a = 0,035` (de la rainure à l'arête claire) DOIT être lissée sur environ un
    pixel d'écran ; en `d_a = 0,16`, le facteur vaut déjà 1 et la transition est donc continue ;
- l'albédo est multiplié par `f_x · f_y · f_z`.

La grille NE DOIT PAS s'appliquer au HUD (REQ-SCN-044, -045), à la skybox, au tunnel ni aux modèles
de téléviseur (REQ-SCN-053, REQ-SCN-054), ni au rectangle du mode 2D (AMD-046 §2, transparence
AMD-050 REQ-SCN-076). Elle s'applique aux tiles opaques **et** transparentes (REQ-SCN-080).
- **Gros plan G** (configuration commune aux critères de §2.3, exécutable par un tiers) : harnais,
  mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle
  de scène 1, courbure nulle, TV Box = None, skybox `Color` noire, valeurs lues en sRGB 8 bits puis
  converties en linéaire. Surface mesurée : une shape de **4 × 4 tiles pleines** (32 × 32 unités),
  de profondeur de construction `D` = 8, **centrée sur C = (128, 120, 0)** (REQ-E3D-410), **sans
  rotation ni mise à l'échelle** (sauf mention contraire d'un critère) : elle occupe donc
  `x` ∈ [112, 144], `y` ∈ [104, 136], `z` ∈ [−4, 4], et la face mesurée est sa **face avant**, de
  normale (0, 0, −1), dans le plan `z` = −4. Texture personnalisée au **format direct** de couleur
  RVB (188, 188, 188), soit un albédo linéaire de **0,503 ± 0,002**. Lumière blanche
  `Light.Direction` = (1, −1, 1), intensité 1. Caméra en **(128 ; 120 ; −34)**, soit à **30 unités**
  du centre de la face mesurée, sur la normale de cette face, visant ce centre, FOV 70° : la hauteur
  visible vaut `2 × 30 × tan 35° = 42,0` unités, soit **25,7 pixels par unité de scène** ; à `N = 4`
  une rainure couvre `2 × 0,035 × 4 = 0,28` unité, soit **7,2 pixels**.
- **Cadrage large B** (configuration du critère de courbure de REQ-SCN-080, exécutable par un
  tiers) : harnais, mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0
  (Native), échelle de scène 1, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**,
  `spl.pixel_shadow` = 0, `spl.ao` = 0, `spl.reflections` = 0, `spl.voxel_grid` = **3** (donc
  `N` = 8), **aucune shape dans la scène**. Surface mesurée : le **backdrop** (REQ-SCN-048), avec
  `Backdrop.Enable` = vrai, `Backdrop.Color` = RVB (188, 188, 188) (albédo linéaire 0,503), tailles
  et décalages **par défaut** (256 × 240 × 1, décalages nuls) : sa face avant est le rectangle
  `z` = −0,5, `x` ∈ [0, 256], `y` ∈ [0, 240], de normale (0, 0, −1), et les arêtes de cellule y sont
  aux coordonnées de scène **multiples de 8** en `x` et en `y`. Lumière blanche
  `Light.Direction` = (1, −1, 1), intensité 1 (la face avant est éclairée partout, `N·L` allant de
  0,816 au bord gauche à 0 au bord droit à 90° de courbure : chaque rainure reste repérable, d'au
  moins 15 niveaux sRGB sous ses voisines). Caméra en **(128 ; 120 ; −110,5)**, soit **110 unités**
  devant la face avant, visant C = (128, 120, 0), FOV 70° : la hauteur visible sur la face vaut
  `2 × 110 × tan 35° = 154,0` unités (**7,01 pixels par unité**) et la largeur visible 273,9 unités,
  donc toute la largeur de la face (256 unités) est dans le cadre, avec 8,9 unités de marge de
  chaque côté ; à 90° de courbure, les bords de la face sont à 118,6 unités de l'axe pour une
  demi-largeur visible de 182,2 unités, donc toujours dans le cadre.
- Critère d'acceptation (auto) : gros plan G, face de normale (0, 0, −1), `render.shadow` = faux,
  `spl.pixel_shadow` = 0, `spl.voxel_grid` = 2 (donc `N = 4`). Points donnés dans le repère local de
  **la tile adjacente au centre de la face du côté `+x`, `+y`** (REQ-E3D-504 ; son centre est en
  (132, 132) dans le repère de scène). Rapport des **valeurs linéaires** (grille On / grille Off) :

  | Point local | Situation | Rapport attendu | sRGB attendu (On) | sRGB attendu (Off) |
  |---|---|---|---|---|
  | (0 ; 2) | centre d'une rainure, un seul axe | 0,62 ± 0,03 | 133 ± 3 | 166 ± 3 |
  | (2 ; 2) | centre de cellule | 1,00 ± 0,01 | 166 ± 3 | 166 ± 3 |
  | (0,4 ; 2) | arête claire, `d = 10 %` | 1,048 ± 0,02 | 169 ± 3 | 166 ± 3 |
  | (0 ; 0) | croisement de deux rainures | 0,384 ± 0,03 | 107 ± 3 | 166 ± 3 |

  En Classic, et en Enhanced avec `spl.voxel_grid` = 0, les quatre points valent 166 ± 3 : le
  critère est donc discriminant.
- Critère d'acceptation (auto) : gros plan G, `spl.voxel_grid` = 1, 2 puis 3 : l'écartement des
  lignes de rainure mesuré à l'écran vaut respectivement **2, 4 et 8 unités de scène** (±0,1), et
  dans les trois cas une ligne passe par le centre de chaque tile ; à `spl.voxel_grid` = 1 et 2,
  une ligne passe aussi par la jonction de deux tiles voisines.
- Critère d'acceptation (auto) : même scène avec `scale = (2, 1, 1)` sur l'objet : l'écartement des
  rainures mesuré à l'écran **double** sur l'axe x (la grille suit l'objet) ; avec
  `rot = (0, 0, 30)`, les rainures tournent de 30° avec la face.
- Critère de revue (SPEC-12) : « Super Mario Bros. » 1-1, `spl.voxel_grid` = 2, en vue rapprochée :
  les blocs et le sol montrent des rainures régulières qui donnent l'aspect de briques de jouet ;
  le ciel, le score et le boîtier TV éventuel sont inchangés.
- Notes : les constantes (3,5 % ; 16 % ; 0,62 ; 0,38 ; 10 % ; 20 % ; le facteur 4 du poids `w` ; les
  seuils 0,8 et 0,9 de l'interpolation d'alignement) sont mesurées sur 3dSen 2.0 ; **les critères
  font foi**. Au croisement de deux arêtes les facteurs se multiplient (0,62² = 0,384) : le coin de
  cellule est plus sombre que la rainure, c'est voulu. La grille n'est **lisible qu'en vue
  rapprochée** : au cadrage par défaut (FOV 70°, distance de réinitialisation de REQ-LUA-113, soit
  189,65 unités), une rainure de `N = 4` ne couvre que 1,1 pixel à 1080p et la grille se réduit à une
  texture fine ; c'est normal pour un effet de « briques de jouet ». Comme les arêtes sont aux
  multiples de `N` du repère local de chaque tile et que les centres de tiles voisines sont distants
  de 8 unités, les rainures se raccordent d'une tile à la suivante pour `N` = 2, 4 et 8. En MARCHING,
  la surface est faite de triangles plans à normale plate (REQ-E3D-522) : la formule s'applique par
  triangle, et `k` et `e` peuvent changer d'un triangle à l'autre.
- Note, **portée du critère de courbure** (REQ-SCN-080) : ce critère établit que la grille est
  **peinte sur la géométrie** — une grille évaluée sur la position **projetée**, ou en espace écran,
  donnerait des lignes droites et uniformément espacées dans les deux captures, et échouerait donc.
  Il ne distingue pas, en revanche, une évaluation **avant** d'une évaluation **après** l'enroulement
  de REQ-SCN-067 : cet enroulement est presque une isométrie le long de la surface (sur toute la
  largeur de la face, à 90°, l'écart cumulé entre la position matérielle et la position courbée
  n'atteint que 9,4 unités, soit un peu plus d'une cellule à `N` = 8, et il n'est observable que par
  le nombre de lignes de rainure, 33 contre 29, dont les plus extérieures sont sous le pixel). Cette
  partie de la clause d'ordre est donc **déclarée** plutôt que mesurée ; la partie qui porte sur la
  transformation d'objet, elle, est mesurée par les deux critères `scale` et `rot` ci-dessus.

**REQ-SCN-082** (P2) — **Ombres pixel** (`spl.pixel_shadow` = 1) : sur les faces des maillages de
**tiles opaques** seulement, l'ombre (REQ-SCN-030, REQ-SCN-031) DOIT être évaluée non pas au
fragment, mais au **centre du voxel** du fragment, obtenu ainsi :

1. prendre la position `p` du point dans le repère local du maillage de la tile (REQ-E3D-504,
   REQ-E3D-522), avant la transformation d'objet et avant la courbure, comme en REQ-SCN-081 ;
2. pour les **deux axes du plan de la face**, c'est-à-dire les deux axes de plus petit `|n_a|`
   (`n` = normale du maillage dans ce repère ; en cas d'égalité, l'axe de plus petit indice dans
   l'ordre `x`, `y`, `z`), ramener la coordonnée au centre du voxel qui la contient : la grille des
   voxels est celle de REQ-E3D-504, donc `p_x ← ⌊p_x⌋ + 0,5`, `p_y ← ⌊p_y⌋ + 0,5` et
   `p_z ← ⌊p_z + D/2⌋ + 0,5 − D/2` (sur `z`, les bords de voxel sont décalés de `D/2` ; pour `D`
   pair c'est aussi « partie entière + 0,5 ») ; le troisième axe est laissé tel quel, de sorte que
   le point reste sur la face. Sur une face GREEDY (normale `±X`, `±Y` ou `±Z`, REQ-E3D-505), cette
   règle sélectionne exactement les deux axes dont `|n_a|` = 0 ; sur une surface MARCHING, dont la
   normale est quelconque, elle sélectionne toujours deux axes, sans plaque où l'effet disparaîtrait ;
3. transformer ce point **comme un sommet** (déformation animée, puis échelle, rotation et position
   de l'objet, dans l'ordre de REQ-E3D-406, puis courbure), appliquer le biais de profondeur et le
   décalage selon la normale de REQ-SCN-031, et y lire la carte d'ombre.

L'ombre change donc par carrés d'un voxel, comme la grille de pixels du jeu. L'effet est sans
objet sur le backdrop, le HUD, le tunnel et les modèles de téléviseur (REQ-SCN-053, REQ-SCN-054),
le rectangle du mode 2D (AMD-046 §2) et les objets transparents (REQ-SCN-040, qui ne reçoivent pas
d'ombre).
- Critère d'acceptation (auto) : harnais, capture 1920 × 1080, `video.anti_alias` = 0,
  `spl.voxel_grid` = 0, `render.shadow` = vrai, albédo et couleurs du gros plan G. Sol : une shape
  de 4 × 4 tiles pleines (32 × 32 unités) avec `rot = (90, 0, 0)`, donc de normale de scène
  (0, 1, 0). Cube : une tile pleine de `sizeZ` = 1 (8 × 8 × 1 unités), empreinte verticale centrée
  sur le centre du sol, face basse à **29 unités** au-dessus du sol. Lumière blanche
  `Light.Direction` = (1, −3, 1), intensité 1 : l'ombre de la plaque est un carré d'environ 8 × 8 unités dont le
  centre est à (+29/3 ; +29/3) = (+9,667 ; +9,667) du centre du sol dans le plan du sol, et ses
  bords tombent donc à 5,667 et 13,667 unités du centre du sol, **au milieu d'un voxel**. Caméra à
  30 unités du centre de l'ombre, élévation 45°, azimut 0 (depuis −z), FOV 70°.
  - avec `spl.pixel_shadow` = 1 : pour chaque cellule de 1 voxel du sol entièrement visible, tous
    les pixels de la capture qui s'y projettent ont la **même** valeur à ±2 niveaux sRGB (sRGB 195
    ± 3 éclairé, 118 ± 3 ombré) ; la frontière entre cellules ombrées et cellules éclairées suit
    **exactement** des bords de cellule, aux coordonnées entières 6 et 14 unités du centre du sol ;
  - avec `spl.pixel_shadow` = 0 (et en Classic) : au moins une cellule de chaque bord de l'ombre
    contient à la fois des pixels ombrés et des pixels éclairés. **C'est cette dernière clause qui
    rend le critère discriminant** : les ombres de REQ-SCN-030 sont déjà dures, la netteté du bord
    ne distingue pas les deux styles.
- Critère de revue (SPEC-12) : « Super Mario Bros. » 1-1, `spl.pixel_shadow` = 1 : les ombres des
  blocs sur le sol et sur le décor sont découpées en carrés d'un pixel NES, alignés sur les pixels
  du jeu ; le score et le ciel sont inchangés.
- Notes : le point d'évaluation se déplace de 0,7 unité au plus le long de la face ; le biais de
  REQ-SCN-031 est appliqué **à ce point déplacé**, et non au fragment, afin que l'acné d'ombre
  reste évitée. L'alignement des cellules est exactement celui de REQ-SCN-081 (repère de
  REQ-E3D-504), avec `N` = 1 voxel.

### 2.4 Occlusion ambiante

**REQ-SCN-083** (P3) — **Occlusion ambiante** (`spl.ao` ≠ 0 ; 1 MSVO : voir *Mode MSVO* ci-dessous).

*Intention.* Les coins, les creux et le pied des objets reçoivent moins de lumière ambiante.
L'effet est **doux**, sans contour net, ne dépend que de la géométrie visible, et ne noircit jamais
complètement une surface.

*Mise en œuvre.* L'implémentation DEVRAIT utiliser une technique publique et classique d'occlusion
ambiante d'écran par échantillonnage d'un **hémisphère orienté par la normale**, reconstruite à
partir de la **profondeur** de la scène, suivie d'un lissage qui respecte les discontinuités de
profondeur. Elle DEVRAIT suivre la technique décrite publiquement par le **tutoriel SSAO de John
Chapman (2013)** : noyau d'échantillons tiré dans l'hémisphère de la normale et **resserré près du
point** considéré, **rotation du noyau par pixel**, **test de portée** qui écarte les échantillons
trop éloignés en profondeur, puis **flou**. Cette référence **oriente sans être normative** : la loi
de pondération et d'atténuation de l'occultation, la répartition et la variation par pixel des
échantillons, le rayon et le seuil du lissage restent des **choix d'implémentation libres**, et ce
sont les paramètres et les invariants ci-dessous qui font foi. Le motif d'échantillonnage PEUT
varier par pixel, à condition qu'aucun motif fixe ne reste visible après lissage.

*Paramètres normatifs.*
- La passe s'applique **après le rendu des objets opaques et des reflets, et avant les objets
  transparents** (REQ-SCN-058 amendée, REQ-SCN-080).
- Elle multiplie la couleur de chaque pixel de la scène par un facteur `1 − O`, `O ∈ [0, 1]`,
  **en espace linéaire, avant l'encodage sRGB de REQ-SCN-028**.
- **Rayon** d'influence : `r = (8 + 6 × (ao.radius − 1)) × échelle de scène` unités de scène, soit
  8, 14, 20, 26 et 32 pour les réglages 1 à 5 (20 par défaut).
- **Force** : `O` est proportionnel à `ao.strength` sous un **plafond d'assombrissement**
  `O ≤ min(0,95 ; 0,3 × ao.strength)` en tout pixel, soit 0,3 / 0,6 / 0,9 / 0,95 / 0,95 pour les
  réglages 1 à 5 (0,9 par défaut). (v1.4 : mesuré sur 3dSen 2.0, l'occlusion par défaut assombrit
  au contact jusqu'à plus de 85 % en linéaire ; voir Q-097.) `O` DOIT être nul là où aucune géométrie occultante ne se
  trouve à moins de `2 r`.
- `ao.thickness` n'a aucun effet hors du mode MSVO (REQ-SCN-079).
- L'implémentation DEVRAIT prendre de l'ordre de **6, 10 et 16 échantillons** par pixel pour
  `ao.quality` Low, Medium et High, et calculer l'occlusion à la **moitié de chaque dimension de la
  cible de rendu courante** sauf en High. Ces nombres ne sont pas observables : ils ne sont donnés
  qu'en ordre de grandeur, et c'est l'aspect (ci-dessous) qui fait foi.
- **Portée** : l'effet NE DOIT PAS modifier la skybox, le HUD (REQ-SCN-029, REQ-SCN-044), le
  tunnel ni les modèles de téléviseur (REQ-SCN-053, qui les veut « non éclairés, sans ombre », et
  REQ-SCN-054), le rectangle du mode 2D (AMD-046 §2) ni les objets transparents (REQ-SCN-040).
  **Occultent** en revanche toutes les surfaces déjà dessinées, donc présentes dans la profondeur,
  au moment de la passe : tiles opaques, backdrop, tunnel, modèles de téléviseur et rectangle du
  mode 2D. N'occultent pas : les objets transparents et le HUD, dessinés après la passe, les tiles
  « ombre seule » (REQ-SCN-041), qui n'écrivent pas de profondeur, et la skybox, à distance infinie
  (REQ-SCN-080, 4ᵉ tiret).

*Mode MSVO* (`spl.ao` = 1). Le libellé est une valeur visible du réglage, qui doit se relire (§1).
Ce mode DEVRAIT (P3) être rendu par une occlusion ambiante d'écran **multi-échelle** : l'occultation
est estimée à plusieurs résolutions de la profondeur puis combinée, ce qui donne à la fois un
contact fin et un assombrissement large à coût modéré. L'implémentation DEVRAIT pour cela **porter
l'implémentation publique sous licence MIT** de l'effet d'occlusion ambiante multi-échelle du
**MiniEngine de Microsoft** (dépôt public `DirectX-Graphics-Samples`), **adaptée à OpenGL 3.3 en
passes de fragments** (OpenGL 3.3 n'a pas de nuanceurs de calcul). La licence MIT est compatible
avec la GPL-3.0 sous laquelle la réimplémentation est distribuée.
- **Conditions.** Le portage DOIT être placé sous `third_party/`, porter l'en-tête de licence MIT
  de son origine, et faire l'objet d'une entrée dans `third_party/README.md` (origine, licence,
  statut dans la salle blanche, règles d'usage), sur le modèle de l'entrée `3DNes_Engine` (§3
  point 11).
- **Repli (D).** Tant que ce portage n'est pas disponible, `spl.ao` = 1 DOIT être rendu exactement
  comme `spl.ao` = 2 (Depth) : repli déclaré **(D)**, le libellé `MSVO` restant exposé et relu.
- **Paramètres.** `ao.radius` et `ao.strength` gardent la signification et les valeurs normatives
  ci-dessus (rayon 8 / 14 / 20 / 26 / 32 ; plafond `O ≤ min(0,95 ; 0,3 × ao.strength)`). `ao.thickness` **agit**
  dans ce mode, et dans celui-là seulement : il règle l'**épaisseur supposée** des occulteurs
  connus seulement par leur profondeur. Qualitativement, à `ao.thickness` = 1 un échantillon plus
  proche que le point considéré est traité comme un **écran mince**, qui n'occulte que son voisinage
  immédiat en profondeur ; à 5, il est traité comme un **volume plein** s'étendant vers le fond, qui
  occulte donc davantage derrière lui. À géométrie fixe, l'assombrissement NE DOIT PAS décroître
  quand `ao.thickness` croît. `ao.quality` DEVRAIT faire varier le nombre d'échelles et
  d'échantillons ; ces nombres ne sont pas observables, comme ci-dessus.
- **Conformité.** Les invariants et les critères de revue de cette section valent **aussi** pour ce
  mode, à la seule exception de l'invariant d'identité entre `spl.ao` = 1 et `spl.ao` = 2, qui ne
  vaut que sous le repli.

*Gros plan A* (configuration commune aux invariants de §2.4, exécutable par un tiers) : harnais,
mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle de
scène 1, courbure nulle, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**, **tous les
autres effets de §2.3 à §2.5 à Off** (`spl.voxel_grid` = 0, `spl.pixel_shadow` = 0,
`spl.reflections` = 0), `spl.ao` = 2 et les quatre réglages `ao.*` à leur défaut (`ao.strength` = 3,
`ao.radius` = 3 donc `r` = 20, `ao.thickness` = 3, `ao.quality` = 1). Lumière blanche
`Light.Direction` = (1, −1, 1), intensité 1. Valeurs lues en sRGB 8 bits puis converties en linéaire.
- **Sol** : une shape de **12 × 12 tiles pleines** (96 × 96 unités), `D` = 8, avec
  `rot = (90, 0, 0)`, donc de normale de scène (0, 1, 0), centrée sur C ; texture personnalisée au
  format direct de couleur RVB (188, 188, 188), soit un albédo linéaire de 0,503 ± 0,002. Le sol
  s'étend ainsi à 48 unités du centre, soit plus de `2 r` = 40 unités.
- **Cube** : une tile pleine de `sizeZ` = 8 (8 × 8 × 8 unités), même texture, posée sur le sol, son
  empreinte verticale centrée sur le centre du sol.
- **Caméra** : à **80 unités** du centre du sol, dans la direction (−0,5 ; +0,5 ; −0,707) depuis ce
  centre (à gauche, au-dessus et devant : élévation 30°), visant ce centre, FOV 70° — vue de trois
  quarts du cube.
- **Variante A-h** : même configuration avec le réglage `Skybox` = 1 (Gradient) et un motif HUD
  visible au centre de l'image (REQ-SCN-044), pour l'invariant de portée.

*Invariants (auto).* Tous sont des **rapports de valeurs linéaires** entre deux captures du gros
plan A qui ne diffèrent que par le réglage indiqué.
- Gros plan A **sans le cube** (un plan seul, sans autre géométrie) : le rapport
  (occlusion On / Off) vaut 1,00 ± 0,02 en tout pixel du sol.
- Gros plan A : sur les pixels du sol au contact de la base du cube, le rapport est **strictement
  inférieur à 1** et **supérieur ou égal à 0,10**, à un niveau sRGB 8 bits près (plafond
  `O ≤ min(0,95 ; 0,3 × ao.strength)` = 0,9) ; en un
  pixel du sol situé à au moins `2 r` = 40 unités du cube et visible dans la capture (par exemple à
  40 unités du cube, du côté opposé à la caméra), il vaut 1,00 ± 0,02.
- `ao.strength` = 5 : le rapport est partout ≥ 0,50 ; `ao.strength` = 1 : partout ≥ 0,90. Le rapport
  est monotone décroissant en `ao.strength` au pixel de contact.
- Sous le repli **(D)**, `spl.ao` = 1 et `spl.ao` = 2 donnent des captures identiques à ±2 niveaux
  sRGB. Avec le portage MSVO disponible : `spl.ao` = 1, changer `ao.thickness` de 1 à 5 modifie au
  moins un pixel de plus de 2 niveaux sRGB, et le rapport au pixel de contact ne **croît** pas.
- `spl.ao` = 0 : capture identique à une capture Classic de la même scène, à ±2 niveaux sRGB
  (REQ-SCN-080 ; les autres effets sont à Off dans le gros plan A).
- Variante A-h : les pixels de la skybox et du HUD sont identiques à ±1 niveau sRGB entre
  `spl.ao` = 0 et `spl.ao` = 2.

*Critères de revue* (catégorie **revue** de SPEC-12, captures archivées dans `verify/`).
- Gros plan A : l'assombrissement est **progressif**, sans contour net, sans bande ni motif de bruit
  fixe, et ne varie pas quand la caméra est immobile.
- « Super Mario Bros. » 1-1, occlusion Depth, réglages par défaut : le pied des buissons, des
  collines et des blocs est visiblement assombri sans devenir noir ; le ciel et le HUD sont
  inchangés. Même critère en mode MSVO quand le portage est disponible.
- Les trois valeurs de `ao.quality` donnent le même aspect général, Low pouvant être plus bruité ;
  aucune ne doit montrer de bande ni de motif fixe.

### 2.5 Reflets du sol

**REQ-SCN-084** (P3) — **Reflets du sol** (`spl.reflections` = 1).

*Intention.* Le sol reflète la scène déjà rendue comme une surface polie ; le reflet s'estompe
avec la distance parcourue et au bord de l'écran, sans bord franc.

*Mise en œuvre.* L'implémentation DEVRAIT utiliser une technique publique et classique de **tracé
de rayon réfléchi dans l'espace écran** : **marche linéaire** le long de la projection du rayon,
puis **affinement par dichotomie** autour du premier franchissement de la profondeur — la famille
décrite par **McGuire et Mara, « Efficient GPU Screen-Space Ray Tracing », JCGT 3(4), 2014**. Cette
référence **oriente sans être normative** : le repère et la loi de progression de la marche, le
nombre et la forme des affinements, la forme et la largeur exactes des atténuations, et la forme du
noyau de flou restent des **choix d'implémentation libres**, et ce sont les paramètres et les
invariants ci-dessous qui font foi.

*Définition du sol.* Une **surface de sol** est une face dont la normale, exprimée dans le **repère
de scène avant courbure**, a une composante **+Y ≥ 0,9**, et qui appartient à une **instance**
(SPEC-04B §2) dont la **boîte englobante dans le plan `xz` du repère de scène, avant courbure**,
mesure au moins `6 × échelle de scène` unités **dans ses deux dimensions**. Un bloc isolé de
8 × 8 unités dans le plan `xz` (une shape d'une seule tile, `D` = 8, sans rotation ni mise à
l'échelle) réfléchit donc. En revanche la dimension en `z` est la **profondeur** de l'objet : la
même shape avec `D` = 3, ou avec `scale = (1 ; 1 ; 0,5)`, mesure 3 ou 4 unités en `z` et ne
réfléchit pas. Un sol de jeu, qui s'étend sur de nombreuses tiles, dépasse le seuil dans les deux
dimensions dès que sa profondeur atteint 6 unités.
*Caméra locale* (REQ-SCN-009, v1.4). Quand le profil active la caméra locale, le plan de l'image
NES est posé à plat et tient lieu de sol : la verticale de cette définition est alors la **normale
du plan image tournée vers la caméra**, `(0 ; 0 ; −1)` dans le repère de scène avant courbure, à
la place de `+Y`, **quelle que soit l'inclinaison de la caméra** ; l'emprise se mesure de même
dans le plan `xy`. En caméra ordinaire, la définition ci-dessus s'applique sans changement (le gros
plan R et ses invariants sont inchangés). Mesure qui fonde cette règle : sur « Punch-Out!! »,
profil à caméra locale inclinée de −60°, la référence fait réfléchir le tapis du ring (normale de
scène (0 ; 0 ; −1)), environ 10,5 % de la frame changeant à `spl.reflections` = 1.

*Paramètres normatifs*, avec `n(niveau) = 2 · niveau − 1` (niveaux 1 à 5 → `n` de 1 à 9) :
- **longueur maximale** du rayon : `40 × n(ssr.length) × échelle de scène` unités de scène, soit
  40, 120, 200, 280 et 360 pour les réglages 1 à 5 (200 par défaut) ;
- **intensité** du reflet (part de la couleur réfléchie mélangée à la couleur de la surface) :
  `0,045 × n(ssr.shine)`, soit 0,045 / 0,135 / 0,225 / 0,315 / 0,405 (0,225 par défaut ; v1.5, voir Q-099). Le mélange se
  fait **en espace linéaire, avant l'encodage sRGB de REQ-SCN-028** ;
- **flou** : le rayon de flou croît **linéairement** avec la distance parcourue par le rayon, de 0
  au point d'origine jusqu'à `0,003 × (10 − n(ssr.clarity))` de la **hauteur d'écran** à la
  longueur maximale, soit 0,027 / 0,021 / 0,015 / 0,009 / 0,003 pour les réglages 1 à 5 (0,015,
  soit 16 pixels à 1080p, par défaut) ;
- **atténuation** : l'intensité DOIT décroître continûment vers 0 quand le rayon approche sa
  longueur maximale et quand sa projection approche le bord de l'écran. La largeur et la forme de
  ces deux transitions sont libres, mais **aucun bord franc de reflet ne DOIT être visible** ;
- **qualité** : l'implémentation DEVRAIT prendre de l'ordre de 12, 24, 48 et 48 pas de marche et 2,
  3, 5 et 5 raffinements pour Low, Medium, High et Ultra, et tracer à la moitié de chaque dimension
  de la cible de rendu courante sauf en Ultra. Ces nombres ne sont pas observables (voir
  REQ-SCN-083) ;
- seule la **scène déjà rendue** peut être reflétée : rien de ce qui est hors champ ou caché ;
- **portée** : la skybox, le HUD, le tunnel, les modèles de téléviseur et le rectangle du mode 2D
  ne sont jamais des surfaces de sol et ne sont jamais modifiés par les reflets ; les objets
  transparents ne portent pas de reflets (REQ-SCN-080). Sont **reflétés** ceux de ces éléments qui
  sont déjà dessinés au moment de la passe — skybox, tunnel, modèles de téléviseur, rectangle du
  mode 2D et tiles opaques ; ne le sont pas les objets transparents et le HUD, dessinés après, ni
  les tiles « ombre seule » (REQ-SCN-041), qui ne sont pas dessinées (REQ-SCN-080, 4ᵉ tiret).

*Gros plan R* (configuration commune aux invariants de §2.5, exécutable par un tiers) : harnais,
mode 3D, capture **1920 × 1080**, `video.anti_alias` = 0, `video.res_scale` = 0 (Native), échelle de
scène 1, courbure nulle, TV Box = None, skybox `Color` noire, `render.shadow` = **faux**, **tous les
autres effets de §2.3 à §2.5 à Off** (`spl.voxel_grid` = 0, `spl.pixel_shadow` = 0, `spl.ao` = 0),
`spl.reflections` = 1 et les quatre réglages `ssr.*` à leur défaut (`ssr.shine` = 3, donc une
intensité de 0,225 ; `ssr.clarity` = 3 ; `ssr.length` = 3, donc 200 unités ; `ssr.quality` = 2).
Lumière blanche `Light.Direction` = (1, −1, 1), intensité 1. Valeurs lues en sRGB 8 bits puis
converties en linéaire.
- **Sol** : une shape de **8 × 8 tiles pleines** (64 × 64 unités), `D` = 8, avec `rot = (90, 0, 0)`,
  donc de normale de scène (0, 1, 0), centrée sur C : sa boîte englobante `xz` mesure 64 × 64
  unités, c'est donc une surface de sol. Texture personnalisée au format direct de couleur
  RVB **(124, 124, 124)**, soit un albédo linéaire de **0,202 ± 0,002** — un sol sombre, pour que
  l'invariant du canal rouge ait de la marge (voir Notes).
- **Cube rouge** : une tile pleine de `sizeZ` = 8 (8 × 8 × 8 unités), texture au format direct de
  couleur RVB (255, 0, 0), soit un albédo linéaire de (1 ; 0 ; 0), posée sur le sol, son empreinte
  verticale centrée sur le centre du sol.
- **Caméra** : à **40 unités** du centre du sol, dans la direction (−0,5 ; +0,5 ; −0,707) depuis ce
  centre, visant ce centre, FOV 70° : les faces du cube de normales (0, 0, −1) et (−1, 0, 0) sont
  visibles et éclairées (`N·L` = 0,577, REQ-SCN-025).
- **Variante R-m** (mur) : le sol est remplacé par la même shape **sans rotation**, dont la face
  avant a la normale de scène (0, 0, −1) ; le cube rouge est posé devant elle, à 1 unité.
- **Variante R-s** (étendue) : le sol est remplacé par une shape d'**une seule tile pleine**,
  `D` = 8, **sans rotation** (8 × 8 × 8 unités : sa face supérieure a la normale (0, 1, 0) et sa
  boîte `xz` mesure 8 × 8), centrée sur C ; le cube rouge est conservé, **à côté** du bloc : sa base
  au niveau de la face supérieure du bloc, sa face (−1, 0, 0) à 2 unités du bord `+x` du bloc, de
  sorte que le reflet de la face rouge atteigne la face supérieure du bloc. La même variante est
  reprise avec `scale = (1 ; 1 ; 0,5)` sur le bloc (4 unités en `z`).
- **Variante R-h** : gros plan R avec le réglage `Skybox` = 1 (Gradient) et un motif HUD visible au
  centre de l'image (REQ-SCN-044), pour l'invariant de portée.

*Invariants (auto).* Chaque invariant compare deux captures de la configuration indiquée qui ne
diffèrent que par le réglage cité.
- Gros plan R : sur les pixels du sol visibles au contact de la base du cube, le canal rouge
  **linéaire** augmente d'au moins **10 %** par rapport à la même capture avec
  `spl.reflections` = 0.
- Variante R-m : aucun pixel de la face avant du mur (normale (0, 0, −1)) n'est modifié (±1 niveau
  sRGB).
- Variante R-s : le bloc de 8 × 8 unités **est modifié** sur au moins un pixel de sa face
  supérieure ; le même bloc à `scale = (1 ; 1 ; 0,5)` (4 unités en `z`, donc moins de
  `6 × échelle de scène` dans une de ses deux dimensions) n'est modifié sur **aucun** pixel.
- Gros plan R : `ssr.shine` croissant de 1 à 5 fait croître de façon monotone l'écart entre les
  captures avec et sans reflets ; `ssr.length` croissant allonge la zone modifiée ; `ssr.clarity`
  croissant réduit la largeur de transition mesurée sur un bord de reflet.
- `spl.reflections` = 0 : capture identique à une capture Classic de la même scène, à ±2 niveaux
  sRGB (REQ-SCN-080 ; les autres effets sont à Off dans le gros plan R).
- Variante R-h : les pixels de la skybox et du HUD sont identiques à ±1 niveau sRGB entre
  `spl.reflections` = 0 et 1.

*Critères de revue* (catégorie **revue** de SPEC-12, captures archivées dans `verify/`).
- Gros plan R : le reflet est net près du contact, se brouille et s'efface avec la distance, et n'a
  pas de bord franc, ni au bout du rayon ni au bord de l'écran.
- « Super Mario Bros. » 1-1, reflets à On, réglages par défaut : le sol de briques montre un reflet
  reconnaissable des buissons et de Mario, plus marqué au contact, sans que le sol perde sa
  texture ; le ciel et le HUD sont inchangés.
- Les quatre valeurs de `ssr.quality` donnent le même aspect général ; en Ultra, le tracé ne manque
  aucun objet plus large que 2 pixels d'écran.

- Notes (§2.4 et §2.5) : les constantes (8 + 6 (`ao.radius` − 1), le plafond de 50 %, 40, 0,09,
  0,003, le seuil +Y ≥ 0,9 et les 6 unités de la boîte englobante) sont mesurées sur 3dSen 2.0 ;
  **les critères font foi**. Le gain `0,78 × ao.strength` et le plafond « au plus de 40 % » de la
  v1.0 sont retirés : ils n'avaient pas de solution commune (55-10). Les plafonds et invariants
  ci-dessus les remplacent et sont simultanément satisfaisables, parce qu'ils ne contraignent plus
  qu'une **borne** et une **monotonie**, pas une valeur calculée par une loi non spécifiée.
- Note, **marge de l'invariant du canal rouge** : le mélange valant
  `(1 − i) · surface + i · reflet` avec `i` = 0,225, la condition « +10 % » s'écrit
  `R_reflet ≥ 1,444 · R_sol`. Avec les albédos du gros plan R et le modèle de REQ-SCN-028, le sol
  donne `R_sol = 0,202 × (0,1812 + 0,577) = 0,153` et la face rouge réfléchie
  `R_reflet = 1,000 × (0,1812 + 0,577) = 0,759`, soit un facteur **4,9**. Il suffit donc que la part
  rouge effectivement rapportée au contact dépasse **30 %** de sa valeur pleine : le seuil de 10 %
  reste franchi malgré l'atténuation et le flou, quelle que soit leur forme. C'est pour cette marge
  que le sol du gros plan R est sombre (albédo 0,202) et le cube rouge pur.

## 3. Changements documentaires

1. **AMD-040 §3** : la liste des paramètres du style Enhanced devient celle de REQ-SCN-079 plus les
   quatre de courbure ; l'identifiant du style est 4 (1 accepté) ; la ligne SPEC-09 de
   `render.visual_style` (« mémorisé, sans effet de rendu ») devient « imm. — effet immédiat sur le
   rendu, REQ-SCN-078 », domaine « 0, 4 (1 accepté en écriture, relu 4 ; toute autre valeur → 0) ».
   Le titre « Paramètres du style 1 » devient « Paramètres du style Enhanced (identifiant 4, 1
   accepté) ».
2. **AMD-040 §2** : la ligne `VisualStyle` de la table de REQ-LUA-114 devient « 0 = Classic,
   4 = Enhanced (1 accepté en écriture, relu 4) ; défaut 0 ; toute autre valeur écrite donne 0.
   Persisté (portée U, ligne §3), **effet immédiat sur le rendu** (SPEC-06 REQ-SCN-078) ». La note
   « Le défaut des quatre paramètres est 0 : un script ne distingue donc pas « inconnu » de « valeur
   nulle » en lisant » est restreinte aux **quatre paramètres de courbure** : huit des douze
   paramètres de REQ-SCN-079 ont un défaut non nul. Le critère `Setting.VisualStyle = 5` → 0 reste
   valide.
3. **SPEC-06 REQ-SCN-058** : l'ordre de rendu devient « carte d'ombre ; effacement ou skybox ;
   objets opaques (tiles, backdrop, tunnel et modèles TV) ; **en style Enhanced, les reflets du sol
   (REQ-SCN-084) puis l'occlusion ambiante (REQ-SCN-083)** ; objets transparents triés ; HUD ; puis
   l'UI de SPEC-10 par-dessus », suivi de « Les objets transparents, dessinés après les deux passes
   d'écran de REQ-SCN-080 (leur passage de profondeur compris), n'occultent pas pour l'occlusion
   ambiante et ne sont pas reflétés ; ils ne sont pas davantage assombris par l'occlusion ni
   porteurs de reflets. Il en va de même du HUD, dessiné encore après, et des tiles « ombre seule »
   (REQ-SCN-041), qui n'écrivent aucune profondeur ; la skybox peut être reflétée mais n'occulte
   jamais (REQ-SCN-080). » Le reste de l'exigence (espace linéaire, 10 bits par canal) est inchangé.
4. **AMD-043 REQ-SCN-066** : « Dans Zaxnes, ils agissent quel que soit `Setting.VisualStyle`
   (Zaxnes n'a qu'un rendu) » devient « Dans Zaxnes, ils agissent quel que soit
   `Setting.VisualStyle`, en Classic comme en Enhanced **(D)** » ; « et se lisent et s'écrivent aussi
   par `Setting:Get/SetVisualStyleParameter(1, id, …)` » devient « … `(4, id, …)` (1 accepté,
   REQ-SCN-078) » ; l'en-tête de colonne « Id Lua (AMD-040 §3, **style 1**) » devient « Id Lua
   (AMD-040 §3, **style Enhanced : identifiant 4, 1 accepté**) ». **AMD-043 §6** : la colonne
   « Lua » devient `GetVisualStyleParameter(4, "spl.bend_zx")` et de même pour les trois autres
   lignes (1 accepté).
5. **AMD-049 REQ-SCN-075** : la phrase « La grille de voxels du style « Enhanced » de 3dSen 2.0
   n'est pas reprise. » est **supprimée** et remplacée par « En style Enhanced, la grille de voxels
   de REQ-SCN-081 s'applique aux faces du pavé, en modulant l'albédo `c` avant le calcul ci-dessus
   (le terme spéculaire n'est pas assombri par une rainure). » Le reste de REQ-SCN-075, ses trois
   critères et ses vecteurs sont inchangés : à `spl.voxel_grid` = 0, qui est le défaut, ils ne sont
   pas affectés.
6. **SPEC-10 REQ-UI-061** (P1) : la grille de paramètres desktop reçoit, **après la ligne
   `Skybox`**, une ligne `Style` (liste : `Classic`, `Enhanced`) ; quand `Style` = `Enhanced`, les
   douze paramètres de REQ-SCN-079 apparaissent à la suite, dans un groupe repliable « Style »,
   avec les libellés du §4 et les règles de visibilité de REQ-SCN-079. La phrase du critère
   d'acceptation « Les douze libellés sont présents, et aucun autre paramètre n'apparaît » devient
   « Les douze libellés sont présents, plus la ligne `Style` et, quand `Style` = `Enhanced`, les
   douze paramètres de SPEC-06 REQ-SCN-079 **apparaissant selon ses règles de visibilité** (quatre
   avec les valeurs par défaut — occlusion, ombres pixel, grille, reflets —, les douze avec
   `spl.ao` ≠ 0 et `spl.reflections` = 1) ; aucun autre paramètre n'apparaît. » La ligne `Style` et
   le groupe sont une addition de Zaxnes au panneau décrit par REQ-UI-061 **(D)** : le programme
   d'origine les place ailleurs dans son interface, que SPEC-10 ne décrit pas.
7. **SPEC-09** : les treize lignes du §4 de cet amendement entrent dans la table §4.1.1 (vidéo et
   rendu) ; la liste « Non exposés en libretro » du §4.6 reçoit « `render.visual_style` et
   `render.style.*` (réglages de style, non exposés) ». **REQ-PERS-017 n'est pas amendée** : son
   énumération vise les réglages sans effet **dans l'original**, ce qui n'est pas le cas de
   `render.style.ao_thickness` (il agit dans le mode MSVO, REQ-SCN-083) ; REQ-SCN-079 la cite donc
   **par analogie** seulement.
8. **SPEC-06** : nouvelle section **§3.11bis « Style visuel Enhanced »**, après REQ-SCN-043,
   contenant REQ-SCN-078 à REQ-SCN-084 ; **§4.1** (réglages utilisateur concernés) reçoit treize
   lignes : `render.visual_style` et les douze clés `render.style.*`, avec leurs bornes et leur
   exigence.
9. **SPEC-v1.2-INDEX** : REQ-SCN-078 à REQ-SCN-084 ajoutées à la liste de SPEC-06 (§3.11bis), ligne
   `AMD-055` ajoutée au tableau des amendements ; **REQ-SCN-058**, **REQ-UI-061**, **REQ-SCN-075**
   (déjà amendée par AMD-049), **REQ-SCN-066** (AMD-043) et **REQ-LUA-114** (AMD-040) marquées
   amendées — la table de l'index étant tenue par exigence.
10. Les textes (libellés, aides) du §4 entrent dans les tables d'AMD-035.
11. **`third_party/README.md`** : si le portage MSVO de REQ-SCN-083 est retenu, il reçoit une entrée
    « occlusion ambiante multi-échelle (MIT) » sur le modèle de l'entrée `3DNes_Engine` : origine
    (dépôt public `DirectX-Graphics-Samples` de Microsoft, MiniEngine), licence **MIT** (compatible
    GPL-3.0), contenu, statut dans la salle blanche (source publique sous licence, lisible et
    réutilisable par le côté propre), et règles d'usage — la spec prime, et tout fichier qui reprend
    ou adapte ce code porte l'en-tête d'attribution MIT de son origine. Tant que cette entrée
    n'existe pas, le repli **(D)** de REQ-SCN-083 s'applique.

## 4. Lignes de SPEC-09 et libellés

Format d'AMD-043 §6, complété de la colonne « Comportement » de SPEC-09 §4.1.1. Toutes les clés sont
de portée **U**, d'effet **imm.** (au plus tard à la frame rendue suivante) et **non exposées en
libretro** (colonne « — », SPEC-09 REQ-PERS-083) : ce sont des réglages de style, que l'hôte
libretro n'a pas à piloter. Le domaine donne les bornes et le pas, pour que la normalisation
d'AMD-040 §3 et la validation de REQ-PERS-003 s'appliquent sans deviner.

Colonne **UI** : la légende de SPEC-09 §4.1 en fait la présence dans le panneau de réglages desktop
**de l'original**, et l'original place le style et ses paramètres **ailleurs** dans son interface,
que SPEC-10 ne décrit pas (§3 point 6). La colonne porte donc, sur la forme d'AMD-046 §4,
« — dans l'original (hors du panneau de REQ-UI-061) ; dans Zaxnes : … », abrégé ci-dessous en
« — / Zaxnes : … ». Tous les éléments « Zaxnes » sont dans le groupe repliable « Style », après la
ligne `Style`, elle-même après la ligne `Skybox`.

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | Libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `render.visual_style` | `VisualStyle` | — | entier | 0 | 0 Classic, 4 Enhanced (1 accepté en écriture, relu 4) ; autre → 0 | U | imm. | — / Zaxnes : liste « Style » | — | choix du rendu, SPEC-06 REQ-SCN-078 |
| `render.style.ao` | `GetVisualStyleParameter(4, "spl.ao")` | — | entier | 0 | 0 à 2, pas 1 | U | imm. | — / Zaxnes : liste « Ambient Occlusion » | — | occlusion ambiante d'écran, REQ-SCN-083 ; 1 (MSVO) rendu comme 2 tant que son portage n'est pas disponible **(D)** |
| `render.style.ao_strength` | `… "ao.strength"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. | — / Zaxnes : curseur « Strength » | — | plafond d'assombrissement `O ≤ min(0,95 ; 0,3 × valeur)`, REQ-SCN-083 |
| `render.style.ao_radius` | `… "ao.radius"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. | — / Zaxnes : curseur « Radius » | — | rayon `8 + 6 × (valeur − 1)` unités de scène, REQ-SCN-083 |
| `render.style.ao_thickness` | `… "ao.thickness"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. (sans effet hors du mode MSVO) | — / Zaxnes : curseur « Thickness », grisé sauf en mode MSVO effectif | — | épaisseur supposée des occulteurs en mode MSVO ; aucun effet avec l'occlusion Depth, REQ-SCN-083 |
| `render.style.ao_quality` | `… "ao.quality"` | — | entier | 1 | 0 à 2, pas 1 | U | imm. | — / Zaxnes : liste « AO Quality » | — | nombre d'échantillons et résolution de la passe (non observables), REQ-SCN-083 |
| `render.style.pixel_shadow` | `… "spl.pixel_shadow"` | — | entier | 0 | 0 à 1, pas 1 | U | imm. | — / Zaxnes : case « Pixel Shadows » | — | ombre évaluée au centre du voxel, REQ-SCN-082 |
| `render.style.voxel_grid` | `… "spl.voxel_grid"` | — | entier | 0 | 0 à 3, pas 1 | U | imm. | — / Zaxnes : liste « Voxel Grid » | — | rainures tous les `N = 2^valeur` voxels, REQ-SCN-081 |
| `render.style.reflections` | `… "spl.reflections"` | — | entier | 0 | 0 à 1, pas 1 | U | imm. | — / Zaxnes : case « Reflections » | — | reflets du sol dans l'espace écran, REQ-SCN-084 |
| `render.style.ssr_shine` | `… "ssr.shine"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. | — / Zaxnes : curseur « Floor Shine » | — | intensité `0,045 × (2 × valeur − 1)`, REQ-SCN-084 |
| `render.style.ssr_clarity` | `… "ssr.clarity"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. | — / Zaxnes : curseur « Clarity » | — | flou maximal `0,003 × (10 − (2 × valeur − 1))` de la hauteur d'écran, REQ-SCN-084 |
| `render.style.ssr_length` | `… "ssr.length"` | — | entier | 3 | 1 à 5, pas 1 | U | imm. | — / Zaxnes : curseur « Length » | — | longueur maximale `40 × (2 × valeur − 1)` unités de scène, REQ-SCN-084 |
| `render.style.ssr_quality` | `… "ssr.quality"` | — | entier | 2 | 0 à 3, pas 1 | U | imm. | — / Zaxnes : liste « Reflection Quality » | — | pas de marche, raffinements et résolution de la passe (non observables), REQ-SCN-084 |

Les clés respectent la forme de REQ-PERS-080 (`[a-z0-9_]+(\.[a-z0-9_\[\]]+)*`) ; le préfixe
`render.style.` les regroupe et les distingue des réglages de rendu existants. Chaque réglage
fournit la liste finie ordonnée de ses valeurs avec libellés (REQ-PERS-081, table ci-dessous), même
s'il n'est pas exposé en libretro, et porte la métadonnée d'effet `immédiat` (REQ-PERS-084).

Libellés et aides (AMD-035 ; anglais par défaut / français) :

| Élément | Anglais | Français |
|---|---|---|
| groupe | `Style` | `Style` |
| Style visuel | `Style` — « Classic keeps the original look. Enhanced adds optional effects. » ; valeurs `Classic`, `Enhanced` | `Style` — « Classic garde l'aspect d'origine. Enhanced ajoute des effets optionnels. » ; valeurs `Classic`, `Enhanced` |
| Occlusion ambiante | `Ambient Occlusion` — « Soft shade in corners and hollows. » ; valeurs `Off`, `MSVO`, `Depth` | `Occlusion ambiante` — « Ombre douce dans les coins et les creux. » ; valeurs `Désactivée`, `MSVO`, `Depth` |
| Force | `Strength` — « How dark the shading gets. » | `Force` — « Intensité de l'assombrissement. » |
| Rayon | `Radius` — « How far the shading reaches. » | `Rayon` — « Portée de l'assombrissement. » |
| Épaisseur | `Thickness` — « No effect with Depth occlusion. » | `Épaisseur` — « Sans effet avec l'occlusion Depth. » |
| Qualité de l'occlusion | `AO Quality` — « More samples, smoother shading. » ; valeurs `Low`, `Medium`, `High` | `Qualité de l'occlusion` — « Plus d'échantillons, ombre plus lisse. » ; valeurs `Faible`, `Moyenne`, `Élevée` |
| Ombres pixel | `Pixel Shadows` — « Shadows that follow the game's pixel grid. » | `Ombres pixel` — « Des ombres qui suivent la grille de pixels du jeu. » |
| Grille de voxels | `Voxel Grid` — « Grooves every 2, 4 or 8 voxels, like toy bricks. » ; valeurs `Off`, `Every 2 voxels`, `Every 4 voxels`, `Every 8 voxels` | `Grille de voxels` — « Des rainures tous les 2, 4 ou 8 voxels, comme des briques de jouet. » ; valeurs `Désactivée`, `Tous les 2 voxels`, `Tous les 4 voxels`, `Tous les 8 voxels` |
| Reflets | `Reflections` — « The floor reflects the scene like a polished surface. » | `Reflets` — « Le sol reflète la scène comme une surface polie. » |
| Brillance du sol | `Floor Shine` — « How strong the reflection is. » | `Brillance du sol` — « Intensité du reflet. » |
| Netteté | `Clarity` — « Higher values keep the reflection sharp. » | `Netteté` — « Une valeur élevée garde le reflet net. » |
| Longueur | `Length` — « How far the reflection reaches. » | `Longueur` — « Portée du reflet. » |
| Qualité des reflets | `Reflection Quality` — « More steps, fewer gaps in the reflection. » ; valeurs `Low`, `Medium`, `High`, `Ultra` | `Qualité des reflets` — « Plus de pas, moins de trous dans le reflet. » ; valeurs `Faible`, `Moyenne`, `Élevée`, `Ultra` |

Les deux réglages de qualité portent des libellés **distincts** (`AO Quality` et
`Reflection Quality`) : ils peuvent être visibles en même temps dans le même panneau.

## 5. Questions

- Aucune. Les trois points qui relevaient d'un choix du propriétaire sont tranchés dans le texte :
  le **repère de la grille** est le repère local du maillage de la tile (REQ-SCN-081, et repère de
  scène pour le backdrop), la **courbure agit dans les deux styles** (lecture A, REQ-SCN-078 et §3
  point 4), et les **techniques des deux effets d'écran** sont orientées par des références
  publiques non normatives, le mode **MSVO** étant rendu par un portage sous licence MIT avec repli
  déclaré sur Depth (§1, REQ-SCN-083).
- Points de la relecture R2 (rapport v1.0) **non retenus** : la famille de loi d'occultation
  proposée au 55-10 (point 1) et la formule d'atténuation de bord proposée au 55-11 c ne sont pas
  reprises, parce que le propriétaire a décidé de ne pas **fixer** l'algorithme des deux effets
  d'écran : REQ-SCN-083 et REQ-SCN-084 restent une intention paramétrée, en P3, dont l'aspect est
  jugé par revue. Depuis la v1.2, ces choix sont **orientés** par une référence publique nommée
  (Chapman 2013 ; McGuire et Mara 2014) sans devenir normatifs pour autant ; le repère de la marche
  (55-11 d) reste donc un choix d'implémentation, désormais documenté par la référence. Les lignes
  libretro suggérées au 55-15 ne sont pas ajoutées : les réglages de style ne sont pas exposés en
  libretro (§4). La reformulation du 55-6 (« ligne d'intersection d'un plan de la grille avec le
  plan de la face ») est remplacée par la formulation par **poids d'axe** de REQ-SCN-081, qui couvre
  du même coup les faces obliques, les objets tournés et les surfaces `MARCHING`, sans cas
  particulier.
- Points de la seconde relecture R2 (rapport v1.1, N-1 à N-8) : **tous retenus**. N-1 par la
  **première** correction proposée (mesure sur la face avant du backdrop, 256 × 240), avec la
  position de la surface mesurée désormais fixée dans le gros plan G **et** dans le cadrage large B,
  le quantificateur restreint aux rainures horizontales (l'enroulement gauche-droite laisse `v`
  inchangé) et un nouveau chiffrage ; une Note dit en outre ce que ce critère discrimine et ce
  qu'il ne discrimine pas. N-2 par l'alignement des trois endroits sur la lecture de REQ-SCN-084,
  avec le cas des tiles « ombre seule » ajouté. N-3 par les configurations nommées **gros plan A**
  et **gros plan R** et leurs variantes (A-h, R-m, R-s, R-h), un sol de 96 × 96 unités pour `2 r`,
  un sol sombre et un cube rouge pur pour la marge du canal rouge, et un objet à refléter dans la
  variante R-s. N-4, N-5 (règle des deux axes de plus petit `|n_a|`), N-6 (interpolation de `k` et
  `e` entre 0,8 et 0,9 ; arête claire contiguë à la rainure, de `d_a` = 0,035 à 0,16, divisée par
  0,125), N-7 (albédo 0,503 ; tile nommée) et N-8 (colonne UI sur la forme d'AMD-046, colonne
  « Comportement » ajoutée, REQ-PERS-017 citée par analogie plutôt qu'amendée, index complété,
  en-tête de table d'AMD-043, renvois REQ-E3D-522 et ordre de REQ-E3D-406) sont appliqués tels
  quels.
