# R2 — Relecture d'`AMD-055` (style visuel « Enhanced », v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-055-style-enhanced.md` **en entier** et y chercher toute fuite
(code, pseudo-code, nom interne non nécessaire, algorithme recopié ligne à ligne plutôt que décrit
par ses paramètres et son aspect), toute incohérence avec `SPEC-06` (`REQ-SCN-028`, `-030`, `-031`,
`-040`, `-044`, `-055`, `-058`, `-075`), `AMD-040`, `AMD-043`, `AMD-046`, `AMD-049`, `AMD-050`,
tout critère intestable (chaque critère automatique doit pouvoir être exécuté par un tiers qui ne
connaît que la spec), tout numéro `REQ` déjà pris et toute référence cassée. Jugement demandé en
particulier : **`REQ-SCN-083` et `REQ-SCN-084` laissent-ils un implémenteur produire un résultat
comparable sans deviner ?**

Entrées relues : `spec/changes/AMD-055-style-enhanced.md` (v1.0 du 2026-10-31, texte sous revue,
**non modifié**) ; `spec/SPEC-GUIDE.md` ; `SPEC-06` §2, §3.9 (`REQ-SCN-025` à `-029`), §3.10
(`REQ-SCN-030` à `-033`), §3.11 (`REQ-SCN-034` à `-043`), §3.12 (`REQ-SCN-044`, `-045`), §3.13
(`REQ-SCN-046` à `-048`), §3.14, §3.15 (`REQ-SCN-053`, `-054`), §3.16 (`REQ-SCN-055`), §3.17
(`REQ-SCN-056` à `-059`), §4.1 à §4.3, §5.2, §5.4, §6, Historique ; `SPEC-04B` §2 (définitions
**voxel**, objet 3D, instance, échelle, rotation), `REQ-E3D-410`, `REQ-E3D-568`, `REQ-E3D-641`,
`REQ-E3D-643` ; `SPEC-09` §2 (portées, couche de session), §3.1 (`REQ-PERS-001`, `-003`, `-010`,
`-017`), §3.9 (`REQ-PERS-080` à `-086`), §4.1.1, §4.6 ; `SPEC-10` §3.7 (`REQ-UI-060` à `-063`) ;
`spec/changes/` : `AMD-040` (§2 `REQ-LUA-114`, §3 paramètres de style et ligne `render.visual_style`),
`AMD-043` (`REQ-SCN-066` à `-068`, §6), `AMD-046` (`REQ-SCN-074`, définition du **rectangle du mode
2D**), `AMD-049` (`REQ-SCN-075`), `AMD-050` (`REQ-SCN-076`), `AMD-052` (`video.anti_alias`,
`REQ-PERS-089`, retrait de `REQ-SCN-077`), `AMD-032`, `AMD-035`, `AMD-029`, `Q-079`, `Q-083` ;
`R2-review-AMD-049.md`, `R2-review-AMD-050-053.md`, `R2-review-AMD-050-053-v1.1.md`,
`R2-review-AMD-046-v1.1.md`, `R2-review-AMD-048.md` (conventions de forme, numéros libres) ;
`SPEC-v1.2-INDEX.md`, `TRACEABILITY-CHECK-P6.md`, `DECISIONS-R0.md`, `journal/JOURNAL.md`.
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
**Le serveur d'outils de rétro-ingénierie présent dans la session n'a pas été utilisé** (il est
hors des sources autorisées du côté propre).
Note de provenance : l'en-tête renvoie au « rapport de comparaison avec 3dSen 2.0 », document du
côté sale non lisible ici. C'est la convention déjà admise pour `AMD-049` et `AMD-052` ; ce n'est
pas relevé comme référence cassée, mais aucune de mes vérifications ne s'appuie dessus.

Méthode : (a) recalcul à la main de toutes les valeurs dérivées des tables (gain d'occlusion,
`n(niveau)`, rayon `r`, longueurs, intensités, rayon de flou) ; (b) **recalcul des deux critères
`[ROM]` dans les deux espaces colorimétriques possibles**, pour décider si l'exigence tranche ;
(c) calcul de la largeur à l'écran d'une rainure de grille au cadrage par défaut (`FOV` 70°,
distance de réinitialisation de `REQ-LUA-113`) pour décider si le critère de `REQ-SCN-081` est
exécutable ; (d) confrontation de chaque clause de portée (tiles, backdrop, HUD, tunnel, boîtier
TV, rectangle du mode 2D, objets transparents) aux exigences gelées correspondantes ; (e) relecture
croisée de `AMD-040` §2/§3, `AMD-043` §2/§6 et `AMD-049` `REQ-SCN-075` phrase par phrase, pour
vérifier que les amendements annoncés en en-tête sont bien ceux dont le texte a besoin.

---

## 1. Verdict

**Étanchéité : acquise.** Aucune fuite bloquante. Le texte ne contient ni code, ni pseudo-code, ni
nom interne de classe, de méthode ou de champ privé, ni architecture interne, ni renvoi au code
d'origine. Les effets sont décrits par leurs **paramètres** et leur **aspect**, et le §1 identifie
explicitement les techniques comme publiques et classiques. Le refus de reprendre l'occlusion
« MSVO » est exactement la bonne décision et elle est documentée. Trois réserves faibles seulement :
55-1 (des paramètres d'algorithme **non observables** sont rendus normatifs), 55-2 (l'attribution de
MSVO à une bibliothèque tierce sous licence décrit une dépendance interne du programme d'origine) et
55-3 (constantes sans justification, identifiants de paramètres sans énoncé de leur légitimité).

**Fond : accepté avec réserves, mais une v1.1 est nécessaire avant toute mise en œuvre.**
24 points, dont **14 bloquants**. Les deux plus graves sont une **contradiction frontale avec
`AMD-049` `REQ-SCN-075`** (55-4), qui écrit que la grille de voxels « n'est pas reprise », et une
**incompatibilité arithmétique interne à `REQ-SCN-083`** entre le gain d'occlusion et le plafond de
son critère `[ROM]` (55-10).

**Réponse à la question posée.** Non : **ni `REQ-SCN-083` ni `REQ-SCN-084` ne permettent en l'état
de produire un résultat comparable sans deviner.**
- `REQ-SCN-083` laisse libres quatre choses qui déterminent entièrement l'image : la **pondération
  et la loi d'atténuation** de l'occultation, la **répartition** des échantillons, l'**espace
  colorimétrique** du produit `1 − O`, et le **rayon** du flou bilatéral. Pire, le gain imposé
  (`0,78 × ao.strength`, soit **2,34** aux réglages par défaut) et le plafond du critère `[ROM]`
  (« au plus de 40 % ») n'ont de solution commune que si la moyenne brute au contact tombe dans
  l'intervalle **[0,107 ; 0,171]** — alors qu'une moyenne d'hémisphère pondérée au cosinus vaut
  environ 0,5 au contact d'un angle droit, ce qui sature `O` à 1 (pixel noir). Deux implémenteurs
  honnêtes produiront, l'un des contacts à peine teintés, l'autre des contacts noirs, et au moins
  l'un des deux échouera à un critère de l'exigence elle-même.
- `REQ-SCN-084` laisse libres la **définition mesurable d'une surface de sol** (« l'objet s'étend
  sur au moins 6 unités » : quel objet, quelle étendue, dans quel repère, avant ou après
  courbure), la **loi de croissance du flou** (« croissant de 0,015 × … » sans unité de distance),
  la **loi d'atténuation** en sortie d'écran, l'**espace de mélange**, et le repère de marche
  (écran ou monde). Le mélange à 0,45 est si fort que l'espace colorimétrique décide à lui seul du
  succès du critère `[ROM]` : sur un sol sRGB 60 réfléchissant du blanc, le mélange **linéaire**
  donne sRGB 183 (**+123**, critère échoué) et le mélange **en sRGB** donne 148 (**+88**, critère
  réussi). L'exigence ne dit pas lequel.

Les deux exigences sont réparables : elles ont la bonne forme (paramètres + aspect + critères), il
leur manque cinq ou six phrases chacune. Aucune n'a besoin d'être réécrite.

**Numérotation : correcte.** `REQ-SCN-078` à `-084` sont libres et contigus. Le plus grand
identifiant `SCN` attribué avant cet amendement est `REQ-SCN-077` (`AMD-052`, **retirée**, numéro
conservé), le plus grand *actif* `REQ-SCN-076` (`AMD-050`). `R2-review-AMD-050-053-v1.1.md` §0
confirmait déjà `REQ-SCN-078` libre. Aucun des sept numéros n'apparaît ailleurs dans `spec/`.
Aucun identifiant réutilisé, aucune exigence retirée sans mention.

**Références : trois défauts** (55-19 citation d'éclairage du HUD, 55-20 rectangle du mode 2D sans
renvoi, 55-24 changements documentaires incomplets). Toutes les autres références sont exactes et
vérifiées une à une : `REQ-SCN-028`, `-030`, `-031`, `-040`, `-044`, `-045`, `-055`, `-058`,
`-075`, `REQ-E3D-410`, `REQ-LUA-114`, `AMD-035`, `AMD-040` §3, `AMD-043`.

---

## 2. Étanchéité

### 55-1 — Des paramètres d'algorithme non observables sont rendus normatifs. *Non bloquant.*

`REQ-SCN-083` fixe `S` = 6, 10 ou 16 **échantillons** et le calcul « à demi-résolution sauf en
High » ; `REQ-SCN-084` fixe « 12, 24, 48 et 48 **pas de marche** et 2, 3, 5 et 5 **raffinements** ».
Un nombre d'échantillons, un nombre de pas de marche et un nombre de raffinements de dichotomie ne
sont pas des grandeurs observables : ce sont les **réglages internes du programme d'origine**. Le
`SPEC-GUIDE` autorise « les constantes qui déterminent un résultat observable » ; celles-ci
déterminent un coût et, indirectement, un aspect, mais elles décrivent surtout *comment* l'original
marche. C'est le seul endroit du texte où la frontière « paramètres et aspect » plutôt que
« algorithme » est franchie.

*Correction proposée* : garder ces nombres mais en **DEVRAIT**, présentés comme un ordre de
grandeur (« une implémentation DEVRAIT prendre de l'ordre de 6, 10 et 16 échantillons »), et
raccrocher la qualité à son observable : « l'aspect ne DOIT pas présenter de banding ni de motif de
bruit fixe à l'écran ; en Ultra le tracé ne DOIT pas manquer un objet plus large que 2 pixels
d'écran ». Sinon, les trois tiers de qualité ne sont vérifiables que par lecture du shader, ce
qu'aucun critère ne peut faire.

### 55-2 — L'attribution de « MSVO » à une bibliothèque tierce sous licence. *Non bloquant.*

§1 : « L'occlusion « MSVO » du programme d'origine, **empruntée à une bibliothèque sous licence de
Unity**, n'est pas reprise ». Le **libellé** `MSVO` est légitime : c'est une valeur visible de la
liste d'occlusion, au même titre que `Fantasy` ou `TV 2`, et `REQ-SCN-079` doit le porter pour que
le réglage se relise. En revanche « empruntée à une bibliothèque sous licence de Unity » est un
**fait sur les dépendances internes du programme d'origine** — la catégorie que le `SPEC-GUIDE`
exclut (« l'architecture interne de l'original … sauf effet observable »). Le besoin réel est
juridique : justifier qu'on **ne** reprend **pas** quelque chose.

*Correction proposée* : « L'occlusion « MSVO » du programme d'origine repose sur une technique
tierce qui n'est pas reproductible dans le cadre de cette réimplémentation ; elle n'est pas reprise
et est remplacée par l'occlusion « Depth » **(D)**. » La provenance précise, si le propriétaire veut
la garder pour mémoire, appartient au `PLAN.md` ou à `DECISIONS-R0.md`, pas à l'exigence.
À comparer avec `AMD-049` §1, où le nom du modèle Unity **devait** être cité parce que ses sources
sont publiques et que la spec en reprend le calcul : ici, c'est l'inverse, on ne reprend rien.

### 55-3 — Légitimité des identifiants de paramètres, et constantes sans justification. *Non bloquant.*

Les douze identifiants (`spl.ao`, `ao.strength`, `ssr.shine`, …) sont **admissibles** : ce sont des
arguments de `Setting:Get/SetVisualStyleParameter` (`REQ-LUA-114`) et des clés de réglages
persistés, donc des « faits d'interopérabilité » explicitement autorisés, exactement comme
`spl.bend_zx` dans `AMD-040` §3. Mais l'amendement ne le **dit pas**, et un relecteur ultérieur ne
peut pas distinguer ces identifiants d'un nom interne recopié. Par ailleurs aucune des constantes
(0,62 ; 0,38 ; 3,5 % ; 4–16 % ; 10 / 20 % ; 0,78 ; 8 + 6·(r−1) ; 0,09 ; 40 ; 0,015) ne porte de
justification, alors que le `SPEC-GUIDE` l'exige et que `AMD-043` §3 donne l'exemple à suivre
(« les constantes … sont mesurées sur 3dSen 2.0 … les vecteurs font foi »).

*Correction proposée* : une phrase au §2.1 — « Ces identifiants sont normatifs parce qu'ils sont
des arguments de l'API Lua (`REQ-LUA-114`) et des clés de réglages persistés ; les autres
identifiants de 3dSen 2.0 restent « inconnus » au sens d'`AMD-040` §3 » — et une **Note** sous
chaque exigence de §2.3 à §2.5 : « les constantes sont mesurées sur 3dSen 2.0 ; les critères font
foi ».

---

## 3. Cohérence avec les exigences gelées et les amendements

### 55-4 — `REQ-SCN-081` contredit frontalement `AMD-049` `REQ-SCN-075`, qui n'est pas amendée. **BLOQUANT.**

`AMD-049` `REQ-SCN-075`, normative et en vigueur depuis le 2026-10-31, contient la phrase :
« **La grille de voxels du style « Enhanced » de 3dSen 2.0 n'est pas reprise.** »
`REQ-SCN-081` applique la grille « sur les faces des maillages de tiles **et du backdrop** ».
L'en-tête d'`AMD-055` n'annonce que deux amendements, `AMD-040` et `AMD-043` ; `AMD-049`
n'apparaît ni en en-tête ni au §3. Un implémenteur qui lit les deux amendements dans l'ordre
chronologique est donc tenu par deux exigences de même priorité (P2) qui se contredisent, sans
règle de préséance. Ce n'est pas une imprécision : c'est une décision du propriétaire du 2026-10-31
qui annule une décision du propriétaire du 2026-10-31.

*Correction proposée* : ajouter à l'en-tête « **amende `AMD-049`** (`REQ-SCN-075` : la phrase sur la
grille de voxels) » et au §3 un point : « `AMD-049` `REQ-SCN-075` : la phrase « La grille de voxels
du style « Enhanced » de 3dSen 2.0 n'est pas reprise. » est **supprimée** et remplacée par « En
style Enhanced, la grille de voxels de `REQ-SCN-081` s'applique aux faces du pavé, en modulant
l'albédo `c` avant le calcul ci-dessus. » » — la modulation doit bien viser `c`, car `REQ-SCN-075`
ajoute un terme spéculaire indépendant de l'albédo, qu'une rainure ne doit pas assombrir.
*Vérifié au passage* : le reste de l'articulation avec `AMD-049` est correcte. `REQ-SCN-080` cite
bien `REQ-SCN-075` pour l'éclairage du backdrop, et non `REQ-SCN-028`.

### 55-5 — « Cellules de N voxels » : le repère choisi contredit la définition du voxel. **BLOQUANT.**

`REQ-SCN-081` : « l'espace est découpé en cellules de N voxels (**N unités de scène**, alignées sur
le repère de `REQ-E3D-410` multiplié par l'échelle de scène) ». Or `SPEC-04B` §2 définit le voxel
comme le « **cube unité du champ d'une tuile**, repéré dans la tuile par `(i, j, k)` » : c'est une
unité du **repère local** de l'objet. Un objet 3D porte un `scale` et un `rot` (`SPEC-04B` §2,
`REQ-E3D-641`), modifiables par Lua. Conséquences, toutes observables :

- pour un objet d'échelle 2, les cellules « de 4 voxels » valent 2 voxels de l'objet ;
- pour un objet tourné de 30°, les plans de la grille **coupent les faces en biais** : les rainures
  ne suivent plus les voxels, et l'aspect « briques de jouet » promis par l'aide disparaît
  précisément sur les objets qui tournent ;
- deux objets adjacents d'échelles différentes partagent les mêmes rainures de scène, donc la
  grille ne marque plus la structure des maillages.

Les deux lectures sont défendables (la grille de scène est stable quand les sprites bougent ; la
grille locale suit les voxels) mais elles donnent des images différentes, et le texte en nomme une
tout en appelant ses cellules par le nom de l'autre.

*Correction proposée* : trancher explicitement, de préférence pour le **repère local de l'objet**
(c'est ce que l'aide décrit) : « les cellules sont celles du **champ de voxels de l'objet**
(`SPEC-04B` §2), de côté N voxels, alignées sur l'origine de ce champ ; elles subissent ensuite
l'échelle, la rotation et la position de l'objet comme sa géométrie ». Si c'est bien le repère de
scène qui est voulu, écrire « cellules de **N unités de scène** » et supprimer le mot « voxels » du
corps de l'exigence (en le gardant dans le libellé UI, qui est celui de l'original), plus une
**Note** : « sur un objet mis à l'échelle ou tourné, les rainures ne suivent pas les voxels de
l'objet ; c'est voulu ».
La même correction est nécessaire dans `REQ-SCN-082` (« la cellule d'un voxel »), qui ne dit même
pas quel repère ni quel alignement : voir 55-14.

### 55-6 — « Arête de cellule située dans le plan de la face » n'existe pas pour les faces visées par le facteur 0,38. **BLOQUANT.**

`REQ-SCN-081` assombrit « le long de chaque **arête de cellule située dans le plan de la face** »,
puis distingue deux facteurs : 0,62 pour les faces alignées sur un axe
(`|composante de la normale| ≥ 0,9`) et **0,38 sinon**. Mais une arête de cellule — arête d'un cube
de la grille — ne peut appartenir au plan d'une face que si cette face est alignée sur un axe. Pour
une face oblique (faces diagonales des cylindres d'`AMD-048`, surfaces de `MARCHING`, faces d'un
objet tourné), **aucune** arête de cellule n'est dans son plan : la lecture littérale donne « pas de
rainure du tout », et le facteur 0,38 ne s'applique jamais. La clause se contredit donc elle-même.

*Correction proposée* : remplacer par l'intersection, qui couvre les deux cas d'un coup : « le long
de chaque **ligne d'intersection d'un plan de la grille avec le plan de la face** (pour une face
alignée sur un axe, ce sont les arêtes des cellules) ». Préciser aussi le comportement sur les
surfaces de `MARCHING`, qui n'ont pas de face plane : soit la grille ne s'y applique pas, soit elle
s'applique par plan tangent — à trancher, sinon l'aspect de `render.mesh_algo = 1` est indéterminé.

### 55-7 — Rien ne dit où les quatre effets se placent par rapport à la courbure d'`AMD-043`. **BLOQUANT.**

`AMD-043` `REQ-SCN-068` a posé une règle d'ordre explicite pour tous les calculs en coordonnées de
scène (découpe `REQ-SCN-036`, point visé `REQ-SCN-008`, occultation FPV `REQ-SCN-022` : **avant**
courbure) et a amendé `REQ-SCN-028` pour que la normale soit **transportée** par la courbure.
`AMD-055` est muet sur les quatre effets, qui tous dépendent de positions ou de normales :

1. la grille (`REQ-SCN-081`) : position de scène **avant** courbure (rainures droites comme les
   voxels, rainures courbées avec la surface) ou **après** (rainures droites à l'écran, qui glissent
   sur la géométrie) ;
2. le centre de voxel des ombres pixel (`REQ-SCN-082`) : arrondi avant ou après courbure ;
3. le test de sol (`REQ-SCN-084`, `+Y ≥ 0,9`) : sur la normale du maillage ou sur la normale
   transportée — avec une courbure de 90°, un sol plat cesse d'être « vers le haut » sur ses bords,
   donc le reflet s'arrête en cours de route ;
4. l'étendue « au moins 6 unités dans le plan horizontal » : mesurée avant ou après courbure.

L'en-tête annonce pourtant un amendement d'`AMD-043` « (les réglages de courbure restent ceux du
style Enhanced, inchangés) », donc le sujet a été vu, mais seulement du côté des réglages.

*Correction proposée* : une clause d'ordre au §2.2, calquée sur `REQ-SCN-068` : « les quatre effets
de §2.3 à §2.5 sont évalués sur les positions et les normales **après** courbure pour tout ce qui
relève de l'écran (occlusion, reflets, qui travaillent sur la profondeur rendue) et **avant**
courbure pour tout ce qui relève de la grille de voxels (`REQ-SCN-081`, `REQ-SCN-082`), de sorte
que les rainures et les carrés d'ombre suivent la surface courbée ». Puis un tiret de critère
`[ROM]` : « Courbure gauche-droite à 90°, grille à 4 : les rainures suivent l'arc, sans glisser
quand la caméra tourne ».

### 55-8 — `REQ-SCN-058` n'est pas amendée : la place des deux passes d'écran et le sort des objets transparents sont indéterminés. **BLOQUANT.**

`REQ-SCN-058` fixe l'ordre d'une frame : « carte d'ombre ; effacement ou skybox ; objets opaques ;
objets transparents triés ; HUD ; puis l'UI ». `REQ-SCN-083` s'insère « après le rendu des objets
opaques et avant le HUD » — intervalle qui contient **exactement** la passe des objets transparents,
sans dire de quel côté. `REQ-SCN-084` ne se place nulle part. Trois questions restent ouvertes, et
chacune change l'image :

- les objets **transparents** sont-ils assombris par l'occlusion ? Ils écrivent la profondeur
  (passage de profondeur préalable de `REQ-SCN-040`), donc ils **occultent** dans tous les cas ;
  mais s'ils sont dessinés après la passe d'occlusion, ils ne sont pas assombris, et une tile
  transparente paraît plus claire que la tile opaque voisine ;
- l'ordre **occlusion → reflets** ou **reflets → occlusion** : le reflet reflète-t-il une scène déjà
  assombrie ? Les deux produisent des images différentes au contact des objets, là où l'occlusion
  est forte ;
- les reflets peuvent-ils refléter des objets transparents, et le HUD ? « Seule la scène déjà
  rendue peut être reflétée » suggère que oui si les transparents sont déjà dessinés.

*Correction proposée* : amender `REQ-SCN-058` au §3 et donner l'ordre complet : « carte d'ombre ;
effacement ou skybox ; objets opaques ; **reflets du sol (`REQ-SCN-084`)** ; **occlusion ambiante
(`REQ-SCN-083`)** ; objets transparents triés ; HUD ; UI », avec la phrase « les objets transparents
contribuent à la profondeur, donc occultent et sont reflétés quand leur passage de profondeur les y
met, mais ne sont ni assombris par l'occlusion ni porteurs de reflets ». L'ordre exact importe peu
du moment qu'il est écrit ; l'absence de choix, elle, est bloquante.

### 55-9 — L'espace colorimétrique du produit d'occlusion et du mélange des reflets n'est pas fixé, et il décide du résultat. **BLOQUANT.**

`REQ-SCN-083` : « la couleur de chaque pixel … DOIT être multipliée par un facteur `1 − O` ».
`REQ-SCN-084` : « part de la couleur réfléchie **mélangée** à la couleur de la surface ».
Ni l'une ni l'autre ne dit si l'opération a lieu en **linéaire** (ce que fait `REQ-SCN-058`, « le
rendu se fait en espace linéaire ») ou **après encodage sRGB**. `AMD-049` a dû trancher exactement
la même question (point 49-1 de la relecture précédente) ; le même soin est nécessaire ici, et
l'écart est bien plus grand qu'alors, parce que les coefficients sont gros :

| Opération | En linéaire | En sRGB | Écart |
|---|---|---|---|
| `O = 0,4` sur un pixel quelconque | niveau sRGB × 0,808, soit **−19 %** | **−40 %** | plafond `[ROM]` du double |
| « au plus 40 % » lu sur le niveau sRGB | correspond à `O ≈ 0,71` | `O = 0,40` | `O` admissible du double |
| sol sRGB 60 + reflet blanc, mélange 0,45 | 0,475 lin → sRGB **183** (**+123**) | **148** (**+88**) | critère `[ROM]` échoué / réussi |
| sol sRGB 128 + reflet blanc, mélange 0,45 | 0,569 lin → sRGB **199** (+71) | **185** (+57) | 14 niveaux |

La troisième ligne est décisive : le critère `[ROM]` de `REQ-SCN-084` (« au plus +90 niveaux sRGB »)
est **réussi en mélange sRGB et échoué en mélange linéaire** sur un sol sombre — et le sol de
briques de « Super Mario Bros. » 1-1 est sombre. L'exigence est donc, au choix, fausse ou
ambiguë selon la lecture, et c'est l'implémenteur qui tranche.

*Correction proposée* : écrire dans les deux exigences « **en espace linéaire, avant l'encodage sRGB
de `REQ-SCN-028`** » (lecture cohérente avec `REQ-SCN-058`), puis **recalculer les deux critères
`[ROM]`** sur cette base — le plafond de `REQ-SCN-084` devra probablement monter au-delà de +90, et
celui de `REQ-SCN-083` être exprimé en facteur linéaire (« `O ≤ 0,4` », mesuré comme au 55-13)
plutôt qu'en « 40 % » d'une grandeur non nommée.

### 55-10 — `REQ-SCN-083` est sous-déterminée, et son gain est incompatible avec son propre critère `[ROM]`. **BLOQUANT.**

C'est le point central du rapport. Quatre choses manquent, et une cinquième est arithmétiquement
fausse.

**(a) La loi d'occultation.** « un échantillon est occulté quand la profondeur de la scène à sa
projection est plus proche que lui, la contribution décroissant avec l'écart de profondeur rapporté
à `r` ». « Décroissant » ne fixe ni la forme (linéaire ? inverse ? lissée ?) ni le seuil de
rejet, ni s'il y a une pondération au cosinus de l'angle avec `N`. Ce sont les paramètres qui
fixent le contraste de l'effet.

**(b) La répartition des échantillons.** « répartis dans l'hémisphère orienté par `N` … (échantillons
plus denses près de `P`) » : pas de loi de densité, pas de motif, et surtout **rien sur la variation
par pixel**. Avec 6 à 16 échantillons, un motif fixe donne des bandes, un motif tourné par pixel
donne du bruit que le flou doit absorber : l'aspect n'est pas le même, et aucun critère ne le
tranche.

**(c) Le flou.** « lissé par un flou bilatéral séparable (qui respecte les discontinuités de
profondeur) » : pas de rayon, pas de seuil de profondeur. Un flou de 2 pixels et un flou de 16
pixels donnent deux images très différentes, l'une bruitée, l'autre baveuse.

**(d) L'espace colorimétrique** : voir 55-9.

**(e) Le gain et le plafond ne sont pas compatibles.** Le gain imposé est `0,78 × ao.strength`, soit
**2,34** à `ao.strength = 3` (défaut) et **3,90** à 5. Les deux critères de l'exigence encadrent
alors la moyenne brute `Ō` au pied d'un objet :

- critère auto, contact d'un cube sur un plan : `O ≥ 0,25` ⟹ `Ō ≥ 0,107` ;
- critère `[ROM]`, pied des buissons : assombrissement « au plus de 40 % » ⟹ si on le lit comme
  `O ≤ 0,4` (lecture linéaire), `Ō ≤ 0,171`.

La loi non spécifiée doit donc faire tomber le contact exactement dans la fenêtre **[0,107 ; 0,171]**
— un facteur 1,6 de marge sur une grandeur qu'aucune formule ne fixe. Or la valeur de référence
d'une moyenne d'hémisphère pondérée au cosinus au contact d'un dièdre droit est de l'ordre de
**0,5** : multipliée par 2,34, elle sature `O` à 1 et le pied des buissons devient **noir**,
soit 100 % d'assombrissement contre 40 % permis. Autrement dit, l'implémentation la plus naturelle
**échoue au critère `[ROM]` de l'exigence**, et il faut deviner une normalisation non écrite pour y
arriver. En lecture sRGB du plafond (`O ≤ 0,71`), la fenêtre devient [0,107 ; 0,303], plus large
mais toujours loin de 0,5.

*Correction proposée*, dans cet ordre :
1. fixer la loi, au moins à une famille près : « `O = clamp(0,78 · ao.strength · (1/S) · Σᵢ wᵢ, 0, 1)`
   avec, pour chaque échantillon `i` de direction `dᵢ` et de distance `tᵢ` de `P` :
   `wᵢ = max(0, N·dᵢ) · max(0, 1 − Δᵢ / r)` si l'échantillon est occulté, 0 sinon, `Δᵢ` étant
   l'écart de profondeur entre l'échantillon et la scène » — c'est une description mathématique d'un
   résultat, autorisée par le `SPEC-GUIDE`, et elle ne décrit aucun code ;
2. en déduire la valeur de `Ō` au contact et **recalibrer le gain ou les plafonds** pour que les
   deux critères soient simultanément satisfaisables ; publier la valeur attendue au contact comme
   vecteur, à ±0,05 ;
3. fixer le rayon du flou (« de l'ordre de 2 % de la hauteur d'écran ») et le seuil de profondeur ;
4. dire que le motif d'échantillonnage PEUT varier par pixel, à condition qu'aucun motif fixe ne
   soit visible après le flou.

### 55-11 — `REQ-SCN-084` est sous-déterminée sur cinq points, dont la définition même d'un sol. **BLOQUANT.**

**(a) « l'objet s'étend sur au moins 6 unités × échelle de scène dans le plan horizontal »** :
indéterminé sur quatre plans. *Quel objet* — la shape, l'instance, le maillage, le pattern ?
*Quelle étendue* — la boîte englobante, le `min` des deux côtés, le `max`, la diagonale ? *Dans quel
repère* — local (avant `scale`/`rot`) ou scène, avant ou après courbure (55-7) ? *Pourquoi 6* — une
tuile fait 8 unités de côté (`REQ-E3D-410` : 256 unités pour 256 pixels, tuiles de 8 pixels), donc
le seuil de 6 est **franchi par une tuile seule** et la condition ne filtre presque rien ; si
l'intention était d'exclure le dessus des petits blocs et des sprites, le seuil est mal placé ou
porte sur autre chose que ce que je lis.
*Correction* : « la boîte englobante de l'**instance** dans le plan `xz` du repère de scène, avant
courbure, mesure au moins 6 unités × échelle de scène **dans ses deux dimensions** », plus une Note
qui dit ce que la clause exclut et ce qu'elle garde (un vecteur : « le dessus d'un bloc isolé de
8 × 8 unités reflète / ne reflète pas »).

**(b) La loi de flou** : « rayon de flou croissant de `0,015 × (10 − n(ssr.clarity)) / 5` (en
fraction de la hauteur d'écran) **avec la distance parcourue par le rayon** ». La quantité donnée
est un **rayon**, pas un taux : « croissant de R avec la distance » peut se lire « atteint R à la
longueur maximale », « croît de R par unité de scène » ou « croît de R par pixel d'écran ». Les
trois diffèrent d'un facteur 200 (longueur maximale par défaut : `40 × 5 = 200` unités). Aux
réglages par défaut, la première lecture donne 1,5 % de la hauteur d'écran, soit ≈ 16 pixels à
1080p — plausible ; les deux autres sont absurdes, ce qui aide, mais l'exigence doit le dire.
*Correction* : « le rayon de flou croît **linéairement** avec la distance parcourue par le rayon,
de 0 au point d'origine jusqu'à `0,015 × (10 − n(ssr.clarity)) / 5` de la hauteur d'écran à la
longueur maximale ». Et supprimer le `/ 5`, reste visible d'une normalisation : `0,003 × (10 − n)`
dit la même chose sans faire chercher.

**(c) L'atténuation** : « atténuée vers 0 quand le rayon sort de l'écran ou atteint sa longueur
maximale » — ni la largeur de la bande d'atténuation (en fraction d'écran) ni sa forme ne sont
données. C'est ce qui décide si les reflets ont un bord franc disgracieux ou non, c'est-à-dire
l'aspect.
*Correction* : « l'intensité est multipliée par `lissé(d/0,1)`, `d` étant la distance de la
projection au bord d'écran le plus proche en fraction de la plus petite dimension d'écran, et par
`1 − (distance parcourue / longueur maximale)` ».

**(d) Le repère de marche** : 12 à 48 « pas de marche » le long d'un rayon — pas uniformes en
**espace écran** ou en **espace monde** ? À 200 unités de portée et 24 pas, la différence décide si
un objet proche est manqué ou non. (Voir aussi 55-1 : ces nombres devraient être indicatifs.)

**(e) L'espace de mélange** : voir 55-9, qui est déterminant pour le critère `[ROM]`.

**Vérification au passage, favorable** : les trois tables de niveaux sont cohérentes et monotones
dans le bon sens. `n = 2·niveau − 1` donne bien 1, 3, 5, 7, 9 ; aux défauts (3) : longueur
`40 × 5 = 200` unités, intensité `0,09 × 5 = 0,45`, flou `0,015 × (10 − 5)/5 = 0,015` ;
`ssr.clarity` croissant **réduit** le flou (cohérent avec « Netteté ») et `ssr.shine` croissant
augmente l'intensité, jusqu'à 0,81 au niveau 5 — ce qui est beaucoup, mais c'est un choix assumé.
Le dédoublement « 48 et 48 pas, 5 et 5 raffinements » pour High et Ultra est intentionnel et
clairement énoncé (seule la résolution change). Rien à corriger de ce côté.

### 55-12 — `spl.voxel_grid` : la valeur du réglage est un indice, mais l'exigence la prend pour `N`. **BLOQUANT.**

`REQ-SCN-079` : « `spl.voxel_grid` | 0 Off, **1 = 2, 2 = 4, 3 = 8** voxels ». `REQ-SCN-081` :
« **Grille de voxels (`spl.voxel_grid` = N de 2, 4 ou 8)** », et son critère dit « **N = 4** ».
Lu littéralement, `REQ-SCN-081` demande que le réglage **vaille** 2, 4 ou 8, ce qui sort de son
domaine `[0 ; 3]` ; la normalisation d'`AMD-040` §3 ramènerait une écriture de 4 à 3 et de 8 à 3,
donc un script qui suit `REQ-SCN-081` obtient 8 voxels en croyant en demander 4. Le même numéro
désigne deux grandeurs dans deux exigences voisines.

*Correction proposée* : « Grille de voxels, de côté `N = 2^spl.voxel_grid` voxels (1 → 2, 2 → 4,
3 → 8 ; 0 → effet désactivé) », et dans le critère « `spl.voxel_grid = 2`, soit `N = 4` ».

### 55-13 — Les critères « automatiques » de `REQ-SCN-081` et `REQ-SCN-083` ne sont pas exécutables par un tiers. **BLOQUANT.**

Trois obstacles indépendants, pour les deux exigences.

**(a) Ils portent sur des grandeurs internes.** « au centre d'une rainure, **albédo** × 0,62 ± 0,03 »
et « les pixels du plan … sont assombris (**O** ≥ 0,25) » : ni l'albédo modulé ni le facteur `O` ne
sont observables. Un tiers ne peut mesurer qu'une couleur de pixel dans une capture. (Même réserve
que 49-6 de la relecture d'`AMD-049`, et la même correction s'applique.)
*Correction* : exprimer chaque critère comme un **rapport entre deux captures** de la même scène,
l'une avec l'effet à Off, l'autre à On, mesuré en **linéaire** au pixel nommé : « le rapport des
valeurs linéaires (grille On / grille Off) vaut 0,62 ± 0,03 au pixel central de la rainure ».

**(b) La configuration d'éclairage et de cadrage n'est pas donnée.** Une face « blanche (albédo 1) »
n'est pas blanche à l'écran : avec la lumière par défaut `(1, −1, 1)`, intensité 1, ambiante 0,1812,
une face de normale `(0, 0, −1)` sort à `0,1812 + 0,577 = 0,758` en linéaire. L'éclaircissement de
10 % ne serait d'ailleurs **pas observable** si la configuration saturait (`REQ-SCN-028` sature à 1).
Le critère doit nommer la lumière, l'intensité et l'orientation, comme le fait `SPEC-06` §5.2.

**(c) Au cadrage par défaut, la rainure est plus fine qu'un pixel d'écran.** À `FOV` 70° et à la
distance de réinitialisation (`REQ-LUA-113` : 189,65), la hauteur visible vaut
`2 × 189,65 × tan 35° = 265,6` unités, soit **4,07 pixels par unité** à 1080p. Une rainure de
`N = 4` mesure `2 × 3,5 % × 4 = 0,28` unité, soit **1,14 pixel** — et la transition est elle-même
« lissée sur environ un pixel d'écran ». « Au centre d'une rainure » n'est donc mesurable sur
**aucun** pixel, et à `N = 2` la rainure fait 0,57 pixel : la grille devient un motif de crénelage.
Le critère doit fixer la distance de caméra (ou la hauteur visible) de façon qu'une rainure couvre
au moins 5 pixels, et l'exigence gagnerait une **Note** sur l'aspect : la grille n'est lisible qu'en
vue rapprochée, ce qui est normal pour un effet de « briques de jouet », mais mérite d'être écrit.

**(d) Et l'anticrénelage.** `AMD-052` `REQ-PERS-089` met `video.anti_alias` à **1** (MSAA 4×,
`AMD-032` §2) sur une installation neuve. Toute mesure au pixel sur un détail de 1 pixel dépend donc
du MSAA. Les critères doivent dire `video.anti_alias = 0`, comme `AMD-050` a dû le faire pour
`REQ-SCN-076`.

### 55-14 — Le critère de `REQ-SCN-082` ne discrimine pas, et l'alignement des cellules manque. **BLOQUANT.**

Critère : « un cube projetant son ombre sur une face de 8 × 8 voxels : chaque voxel de la face
reçoit une ombre uniforme (pleine ou nulle), **sans bord de pénombre à l'intérieur d'un voxel** ».
Or `REQ-SCN-030` impose déjà des **ombres dures** (« bords nets, sans filtrage doux ») : il n'y a
jamais de pénombre, ni en Classic ni en Enhanced. **Le rendu Classic passe ce critère sans
modification**, donc le critère ne teste pas l'exigence. L'observable propre aux ombres pixel est
que la **frontière de l'ombre coïncide avec les bords des voxels**, et non qu'elle soit nette.
Deuxième manque : l'exigence dit « arrondie au centre de la cellule d'un voxel dans ce plan » sans
dire dans quel repère ni sur quel alignement (même défaut qu'au 55-5), et sans dire ce que devient
le biais de profondeur de 0,02 unité de `REQ-SCN-031` quand le point d'évaluation se déplace de
jusqu'à 0,7 unité le long de la face.

*Correction proposée* : « Critère (auto) : cube de 8 × 8 × 8 unités au-dessus d'une face de sol de
32 × 32 unités, lumière à 45°, grille désactivée. Pour chaque cellule de 1 voxel de la face, les
pixels de la capture qui s'y projettent ont tous la **même** valeur (±2 niveaux sRGB), et la
frontière entre cellules ombrées et cellules éclairées suit les bords des cellules à ±0 unité ; en
Classic (`spl.pixel_shadow = 0`), au moins une cellule contient à la fois des pixels ombrés et des
pixels éclairés » — la dernière clause est ce qui rend le critère discriminant. Ajouter : « le point
d'évaluation est le centre de la cellule, dans le même repère et avec le même alignement que
`REQ-SCN-081` ; le biais de `REQ-SCN-031` est appliqué à ce point ».

### 55-15 — Douze réglages persistés sans clé stable ni ligne de `SPEC-09`. **BLOQUANT.**

`REQ-SCN-079` déclare douze paramètres « portée U, persistés », et son critère exige qu'ils
survivent à un redémarrage. Or aucun n'a de **clé stable**, et le §3 n'ajoute **aucune ligne** à
`SPEC-09` §4.1.1 ni §4.6. C'est en contradiction avec cinq exigences P1/P2 de `SPEC-09` :
`REQ-PERS-001` (le modèle contient chacun des réglages des tables §4.1, avec clé, type, défaut,
domaine et portée), `REQ-PERS-080` (forme et stabilité de la clé), `REQ-PERS-081` (liste finie
ordonnée de valeurs avec libellés), `REQ-PERS-083` (clé de core option et colonne « libretro ») et
`REQ-PERS-084` (métadonnée d'effet). `AMD-043` §6 et `AMD-046` §3 ont fait exactement ce travail
pour leurs réglages ; `AMD-055` ne le fait pas, et en l'état « survit à un redémarrage » n'est pas
testable puisque rien ne dit sous quelle clé la valeur est écrite. Le §3 point 1 ne traite que la
ligne existante `render.visual_style`, et pour celle-ci il faut aussi dire comment `REQ-PERS-003`
valide un fichier qui contient `1` (→ Enhanced, relu 4) et une valeur hors domaine (→ 0).

*Correction proposée* : un §4 « Lignes de `SPEC-09` et libellés », avec treize lignes au format de
`AMD-043` §6 — par exemple `render.style_ao`, `render.style_ao_strength`, `render.style_ao_radius`,
`render.style_ao_thickness`, `render.style_ao_quality`, `render.style_pixel_shadow`,
`render.style_voxel_grid`, `render.style_reflections`, `render.style_ssr_shine`,
`render.style_ssr_clarity`, `render.style_ssr_length`, `render.style_ssr_quality` — chacune avec
type, défaut, domaine **avec bornes et pas explicites** (pour que la normalisation d'`AMD-040` §3
s'applique sans deviner, par exemple à l'écriture de 1,4 dans `spl.ao`), portée U, effet « imm. »,
colonne UI, colonne libretro, et la ligne `zaxnes_…` correspondante au §4.6 (`AMD-029`). Ajouter
aussi, comme `SPEC-06` §4.1 le fait pour les autres réglages de rendu, les treize lignes dans cette
table.

### 55-16 — L'UI décrite contredit `REQ-UI-061` (P1) et il manque la moitié des textes. **BLOQUANT.**

`REQ-SCN-079` : « Ils sont présentés dans **le groupe d'options du rendu**, sous « Style », visibles
quand Enhanced est actif ». Trois problèmes.

1. **`REQ-UI-061` (P1)** énumère les paramètres visibles sur desktop et conclut : « Les douze
   libellés sont présents, et **aucun autre paramètre n'apparaît** ». Treize contrôles nouveaux (le
   choix de style, plus les douze paramètres) ne peuvent pas apparaître sans amender cette exigence
   — qui est P1, donc prioritaire sur la P2 d'`AMD-055`. Le §3 n'en dit rien. (`AMD-043` a frôlé le
   même écueil mais s'est protégé par un **DEVRAIT** ; `AMD-055` est à l'indicatif, donc normatif.)
2. **« le groupe d'options du rendu » n'existe nulle part.** `SPEC-10` §3.7 décrit une grille de
   paramètres sans groupes ; les seuls groupes définis sont « Géométrie » (`AMD-043` §6) et
   « Scène » (`AMD-046`, table libretro). La référence est donc vide, et le placement indéterminé.
3. **Textes manquants** (alors que le §3 point 2 les envoie dans les tables d'`AMD-035`, qui exigent
   EN/FR) : le libellé du groupe « Style » en français ; les libellés du choix de style
   (`Classic`/`Enhanced`) et son aide ; les libellés des valeurs d'énumération — `Off`, `MSVO`,
   `Depth` ; `Low`/`Medium`/`High`/`Ultra` ; et surtout ce qu'affiche `spl.voxel_grid` (« 2 », « 2
   voxels », « Every 2 voxels » ?). Enfin `ao.quality` et `ssr.quality` portent **le même libellé
   « Quality / Qualité »** et peuvent être visibles en même temps : deux lignes identiques dans le
   même panneau.

*Correction proposée* : amender explicitement `REQ-UI-061` au §3 (« la grille de paramètres reçoit,
après « Skybox », une ligne `Style` ; quand `Style = Enhanced`, les paramètres de `REQ-SCN-079`
apparaissent à la suite, repliables »), nommer le groupe et compléter la table de textes, avec des
libellés distincts pour les deux qualités (« AO Quality » / « Reflection Quality »).

### 55-17 — Ce qu'`AMD-055` amende dans `AMD-043` n'est pas écrit, et la courbure en Classic reste indécise. **BLOQUANT.**

L'en-tête annonce « **amende … `AMD-043`** (les réglages de courbure restent ceux du style Enhanced,
inchangés) », mais le §3 « Changements documentaires » ne contient **aucun** point pour `AMD-043` :
l'amendement annoncé n'a pas de texte. Et la phrase de l'en-tête se lit de deux façons
incompatibles :

- lecture A : les quatre identifiants de courbure restent rattachés au **style Enhanced** pour
  l'API Lua (donc `Get/SetVisualStyleParameter(4 ou 1, "spl.bend_zx", …)`), mais la courbure
  continue de s'appliquer **dans les deux styles**, comme aujourd'hui ;
- lecture B : la courbure devient un effet du **style Enhanced** et cesse d'agir en Classic.

`REQ-SCN-078` pousse vers A (« Classic : rendu de `SPEC-06`, inchangé », et la courbure est dans
`SPEC-06` amendé) ; l'en-tête, vers B. L'écart est entièrement observable : en lecture B, un
utilisateur qui a réglé une courbure la perd en passant en Classic. Par ailleurs, la phrase
d'`AMD-043` `REQ-SCN-066` « ils agissent quel que soit `Setting.VisualStyle` (**Zaxnes n'a qu'un
rendu**) » devient fausse dans sa parenthèse, et la colonne Lua de `AMD-043` §6 (`…Parameter(1, …)`)
devrait mentionner 4.

*Correction proposée* : trancher pour la lecture A (elle préserve un réglage utilisateur et évite
une divergence) et l'écrire au §3 : « `AMD-043` `REQ-SCN-066` : « Dans Zaxnes, ils agissent quel que
soit `Setting.VisualStyle` (Zaxnes n'a qu'un rendu) » devient « Dans Zaxnes, ils agissent quel que
soit `Setting.VisualStyle`, en Classic comme en Enhanced **(D)** » ; la colonne Lua du §6 devient
`GetVisualStyleParameter(4, …)` (1 accepté) ». Ajouter un tiret au critère de `REQ-SCN-078` :
« `Setting.VisualStyle = 0` avec Courbure gauche-droite à 60° : la scène reste courbée ».

### 55-18 — `AMD-040` `REQ-LUA-114` et sa note ne sont pas amendés. *Non bloquant.*

Le §3 point 1 ne vise que le §3 d'`AMD-040` et la ligne `SPEC-09`. Restent inexacts, dans le §2
d'`AMD-040` :
- la ligne `VisualStyle` de la table de `REQ-LUA-114` : « 0 = Classic, **1 = Enhanced** ; … **sans
  effet sur le rendu de Zaxnes (qui n'a qu'un rendu)** » — les deux moitiés sont désormais fausses ;
- la note « Le défaut des quatre paramètres est 0 : un script ne distingue donc pas « inconnu » de
  « valeur nulle » en lisant » — avec huit paramètres de défaut non nul (`ao.strength` = 3, …), elle
  ne vaut plus que pour les quatre de courbure.

*Correction proposée* : deux points au §3 — réécrire la ligne (« 0 = Classic, 4 = Enhanced (1
accepté en écriture, relu 4) ; effet immédiat sur le rendu, `REQ-SCN-078` ») et restreindre la note
aux quatre paramètres de courbure. Le critère `Setting.VisualStyle = 5 → 0` d'`AMD-040` reste
valide, je l'ai vérifié.

### 55-19 — `REQ-SCN-080` cite la mauvaise exigence pour l'éclairage du HUD. *Non bloquant.*

« l'éclairage des tiles, du backdrop et du HUD est celui de Classic (`REQ-SCN-028`,
`REQ-SCN-075`) ». L'éclairage du HUD est celui de **`REQ-SCN-029`** (même modèle, mais direction
exprimée dans le repère de la caméra) et son rendu celui de **`REQ-SCN-044`**. En citant
`REQ-SCN-028`, l'exigence dit involontairement que le HUD s'éclaire comme une tile, ce qui le
ferait changer de teinte quand la caméra tourne.

*Correction* : « (tiles : `REQ-SCN-028` ; backdrop : `REQ-SCN-075` ; HUD : `REQ-SCN-029` et
`REQ-SCN-044`) ».

### 55-20 — Portée des effets : le tunnel, le boîtier TV et le rectangle du mode 2D. *Non bloquant.*

`REQ-SCN-081` et `REQ-SCN-082` listent soigneusement leurs exclusions (HUD, skybox, rectangle du
mode 2D ; backdrop pour les ombres pixel), mais `REQ-SCN-083` ne dit que « la scène (pas de la
skybox, ni du HUD) » et `REQ-SCN-084` ne dit rien du tout. D'où trois trous :

- le **tunnel** et les **modèles de téléviseur** (`REQ-SCN-053`, `-054`) sont des objets opaques de
  la scène : l'occlusion s'y applique-t-elle ? Le tunnel est « non éclairé, sans ombre » par
  `REQ-SCN-053` ; l'assombrir par occlusion contredit l'esprit de cette clause, et des angles
  sombres apparaîtraient dans les cinq pavés qui entourent la scène ;
- le **rectangle du mode 2D** : exclu de la grille et des ombres pixel, mais pas de l'occlusion ni
  des reflets. Un plan seul n'est pas assombri (c'est le troisième tiret du critère de
  `REQ-SCN-083`) et sa normale ne pointe pas vers le haut, donc l'effet est probablement nul ; il
  faut tout de même l'écrire, ne serait-ce que pour l'asymétrie avec §2.3 ;
- les tiles **transparentes** reçoivent-elles des rainures ? La portée « faces des maillages de
  tiles » les inclut.

Par ailleurs « le rectangle du mode 2D » est cité **sans référence** dans `REQ-SCN-081` et
`REQ-SCN-082` ; il est défini par **`AMD-046` §2** (et sa transparence par `AMD-050`
`REQ-SCN-076`). `AMD-050` prend soin d'écrire « (AMD-046 §2) » à chaque mention.

*Correction* : compléter les deux listes d'exclusion (« ni au tunnel et aux modèles de téléviseur
de `REQ-SCN-053`/`-054`, ni au rectangle du mode 2D (`AMD-046` §2) ») et ajouter les renvois.

### 55-21 — `ao.thickness` n'a aucun effet observable : le dire. *Non bloquant.*

`spl.ao = 1` (MSVO) est « rendu comme Depth » et `ao.thickness` est « sans effet en Depth » : le
réglage n'a donc **jamais** d'effet dans Zaxnes. Il est pourtant présenté, aidé et visible comme les
autres. `SPEC-09` a un précédent exact et une formulation éprouvée pour ce cas (`REQ-PERS-017` :
« Réglages sans effet observable »).

*Correction* : « `ao.thickness` est conservé, persisté et exposé, **sans aucun effet observable**
dans Zaxnes (l'occlusion MSVO n'étant pas reprise) ; il est présenté grisé **(D)** », et un tiret de
critère « changer `ao.thickness` de 1 à 5 ne change aucun pixel de la capture ». Sans cela, un
implémenteur cherchera un effet qui n'existe pas, et un testeur ne pourra pas écrire de test.

### 55-22 — Cible de rendu, MSAA et demi-résolution. *Non bloquant.*

Les deux effets d'écran lisent la profondeur et la couleur de la scène. Trois points d'articulation
avec le pipeline gelé méritent une phrase, sans quoi chaque implémenteur tranchera seul :

- `REQ-SCN-057` : le rendu dessine « dans une cible fournie par l'hôte … **sans relire l'image
  produite** ». L'intention de cette clause est d'interdire la relecture CPU du framebuffer hôte,
  pas l'échantillonnage d'une cible intermédiaire, mais `REQ-SCN-084` (« seule la scène déjà rendue
  peut être reflétée ») l'effleure. À écrire : « en Enhanced, la scène PEUT être rendue dans une
  cible intermédiaire puis composée dans la cible de l'hôte ; celle-ci n'est jamais relue » ;
- `video.anti_alias` = 1 par défaut (`AMD-052` `REQ-PERS-089`) = MSAA 4× (`AMD-032` §2) : dire que
  l'occlusion et les reflets sont calculés sur la profondeur **résolue** et appliqués après
  résolution, et fixer `video.anti_alias = 0` dans les critères au pixel (voir 55-13 d) ;
- « demi-résolution » : par rapport à la cible de rendu, qui peut déjà être réduite par
  `video.res_scale` (`REQ-SCN-063`) — préciser « la moitié de chaque dimension de la cible de
  rendu courante ».

### 55-23 — « Survit à un redémarrage » contre la couche de session de `REQ-PERS-010`. *Non bloquant.*

Critère de `REQ-SCN-079` : « chaque paramètre survit à un redémarrage ». Mais `REQ-PERS-010`
(DEVRAIT, **(D)** assumée) traite toute écriture Lua sur un réglage U comme une **couche de
session**, rétablie à la fin de la partie, et « la préférence persistée ne change que par l'UI ».
Un tiers qui exécute le critère par `Setting:SetVisualStyleParameter` constatera donc l'inverse de
ce qui est écrit. (`AMD-040` a la même tension — « écrire par Lua ou par l'interface revient au
même » — mais `AMD-055` la rend testable, donc visible.)

*Correction* : « chaque paramètre, **écrit depuis l'UI ou le modèle de réglages**, survit à un
redémarrage ; une écriture par Lua suit `REQ-PERS-010` (couche de session) ». Et un tiret pour le
cas Lua, qui mérite d'être verrouillé : « `Setting:SetVisualStyleParameter(4, "ao.strength", 5)`
pendant une partie, puis fin de partie → `ao.strength` revient à la préférence utilisateur ».

### 55-24 — Clause de priorités absente et changements documentaires incomplets. *Non bloquant.*

- **Priorités** : `AMD-043`, `AMD-046`, `AMD-049` et `AMD-052` portent tous une ligne « Priorités : »
  sous l'en-tête (demandée par les relectures précédentes, point 49-10 pour le dernier en date).
  `AMD-055` n'en a pas, alors qu'il contient sept exigences P2 dont une (`REQ-SCN-081`) **remplace
  une clause** d'une autre P2 (55-4) et une autre (`REQ-SCN-078`) modifie une P2 d'`AMD-040`. Sans
  cette ligne, la matrice de traçabilité ne sait pas quoi faire d'un `AMD-055` partiellement retenu.
  *Correction* : « Priorités : les sept exigences sont P2. Si `REQ-SCN-078` n'est pas retenue, le
  style reste sans effet de rendu (`AMD-040` `REQ-LUA-114` inchangée) et rien de cet amendement ne
  s'applique. Si une exigence de §2.3 à §2.5 n'est pas retenue, l'effet correspondant n'existe pas,
  son réglage reste persisté sans effet, et les autres effets sont inchangés ; si `REQ-SCN-081`
  n'est pas retenue, la phrase d'`AMD-049` `REQ-SCN-075` sur la grille de voxels reste valable (T-1). »
- **Placement et index** : le §3 ne dit pas **où** les sept exigences entrent dans `SPEC-06`, ni ne
  met l'index à jour, alors que `AMD-049` §3 point 3 et `AMD-050` §3 points 5–6 ont établi la
  convention. *Correction* : « `SPEC-06` : nouvelle section §3.11bis « Style visuel Enhanced »
  après `REQ-SCN-043`, contenant `REQ-SCN-078` à `-084` ; `REQ-SCN-058` amendée (55-8) ; `SPEC-06`
  §4.1 : treize lignes ajoutées (55-15) ; `SPEC-v1.2-INDEX` : `REQ-SCN-078` à `-084` ajoutées à la
  liste de `SPEC-06`, ligne `AMD-055` ajoutée au tableau des amendements ».
- Le §4 « Questions » dit « Aucune ». Après correction, au moins deux points relèvent d'un choix du
  propriétaire et non d'une précision rédactionnelle : la lecture A ou B de 55-17, et le repère de
  la grille de 55-5. Ils devraient y figurer, ou être tranchés dans le texte.

---

## 4. Ce qui est juste, et mérite d'être conservé tel quel

Pour que la v1.1 ne défasse pas ce qui va :

- **`REQ-SCN-078`** est bien construite : l'asymétrie « accepte 4 et 1, relit 4 » est exactement ce
  qu'il faut pour que les scripts 2.0 et les réglages existants fonctionnent, son critère est
  exécutable tel quel, et la clause « jamais pris dans le profil » s'accorde avec la portée U.
- **`REQ-SCN-080`** est la meilleure exigence de l'amendement : elle rattache Enhanced à Classic au
  lieu de redécrire un pipeline, et son critère (« tous effets à Off → captures identiques à ±2
  niveaux sRGB ») est à la fois exécutable, discriminant et peu coûteux. C'est le filet de sécurité
  de tout l'amendement.
- Le **refus de MSVO** et son remplacement par Depth, marqué **(D)**, sont la bonne décision, prise
  pour la bonne raison et documentée au bon endroit.
- Les **tables de niveaux** de `REQ-SCN-084` sont cohérentes et monotones (vérifié, 55-11), et le
  choix de ne faire varier que la résolution entre High et Ultra est explicite.
- La **désactivation individuelle** de chaque effet, la portée globale U et l'application à chaud
  sont uniformes dans tout le texte, sans exception cachée.
- Aucune exigence gelée n'a besoin d'être **rouverte** au-delà de ce que les points ci-dessus
  demandent : `REQ-SCN-058` (55-8), `REQ-UI-061` (55-16), `REQ-SCN-075` (55-4), plus les tables de
  `SPEC-09` §4.1.1/§4.6 et `SPEC-06` §4.1 (55-15). `REQ-SCN-028`, `-029`, `-030`, `-031`, `-034`
  à `-043`, `-044`, `-045`, `-046` à `-055`, `-056`, `-057`, `-059`, les vecteurs de `SPEC-06` §5.2
  et §5.3 et les captures de §5.4 sont **inchangés**.

---

## 5. Conclusion — gate R2

- **Filtre anti-fuite : franchi.** Aucune fuite. Pas de code, pas de pseudo-code calqué, pas de nom
  interne, pas d'architecture, pas de renvoi au code d'origine. Les trois réserves (55-1, 55-2,
  55-3) sont des corrections d'une à trois phrases et ne remettent pas en cause le texte.
- **Numérotation et références : franchi**, sauf trois renvois à corriger (55-19, 55-20, 55-24).
  `REQ-SCN-078` à `-084` sont libres et contigus ; `REQ-SCN-077` reste retirée.
- **Fond : non franchi en l'état.** 24 points, dont **14 bloquants** :
  **55-4** (contradiction avec `AMD-049`), **55-5** et **55-6** (le repère et la géométrie de la
  grille), **55-7** (ordre avec la courbure), **55-8** (`REQ-SCN-058` et les objets transparents),
  **55-9** (espace colorimétrique, qui fait basculer un critère `[ROM]`), **55-10**
  (`REQ-SCN-083` sous-déterminée **et** arithmétiquement incompatible avec son propre critère
  `[ROM]`), **55-11** (`REQ-SCN-084` sous-déterminée sur cinq points), **55-12**
  (`spl.voxel_grid`), **55-13** et **55-14** (critères automatiques non exécutables ou non
  discriminants), **55-15** (réglages persistés sans clé), **55-16** (UI contre `REQ-UI-061` P1),
  **55-17** (amendement d'`AMD-043` annoncé mais non écrit, courbure en Classic indécise).
- **Réponse à la question du propriétaire : non.** `REQ-SCN-083` et `REQ-SCN-084` ne permettent pas,
  en v1.0, de produire un résultat comparable sans deviner. Les deux manquent de la même chose :
  une loi (pondération et atténuation pour l'occlusion, croissance du flou et atténuation de bord
  pour les reflets), un espace colorimétrique, et une définition mesurable de leur domaine
  d'application (quels pixels, quel sol). Pour `REQ-SCN-083` s'ajoute une erreur de calibration :
  le gain 2,34 par défaut et le plafond « au plus 40 % » du critère `[ROM]` n'ont de solution
  commune que dans une fenêtre étroite que rien n'impose, et l'implémentation la plus naturelle
  échoue au critère. C'est réparable sans changer la structure : cinq à six phrases par exigence,
  plus un recalcul des deux critères `[ROM]`.
- **Recommandation** : v1.1 traitant d'abord 55-4, 55-10 et 55-11 (les trois qui engagent le
  propriétaire : une décision annulée, une calibration à refaire, une définition à poser), puis
  55-5 à 55-9 et 55-12 à 55-17 (rédactionnels mais bloquants), puis les non bloquants. Une
  seconde relecture R2 sera nécessaire sur 55-10 et 55-11, qui demandent de nouvelles formules et
  de nouveaux critères — donc de nouveaux recalculs.

Le fichier `AMD-055-style-enhanced.md` n'a pas été modifié par cette relecture ; rien n'a été
commité.
