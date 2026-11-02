# R2 — Troisième relecture d'`AMD-055` (style visuel « Enhanced », v1.2)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.2
Objet : vérifier **point par point** que la v1.2 d'`AMD-055-style-enhanced.md` lève les huit points
`N-1` à `N-8` de `R2-review-AMD-055-v1.1.md`, juger la raison donnée pour les points **non
retenus**, puis relire le fichier **en entier** pour chercher toute fuite, incohérence, critère
intestable ou référence `REQ` cassée — qu'elle ait été **introduite par les corrections** ou qu'elle
ait échappé aux deux premières relectures.

Entrées relues : `spec/changes/AMD-055-style-enhanced.md` (v1.2 du 2026-10-31, texte sous revue,
**non modifié**) ; `spec/changes/R2-review-AMD-055-v1.1.md` et `R2-review-AMD-055.md` (rapports v1.1
et v1.0, **non modifiés**) ; `spec/SPEC-GUIDE.md` ; `SPEC-06` §3.9 (`REQ-SCN-025` à `-029`), §3.10
(`REQ-SCN-030` à `-032`), §3.11 (`REQ-SCN-035`, `-040`, `-041`, `-043`), §3.12 (`REQ-SCN-044`,
`-045`), §3.13 (`REQ-SCN-046` à `-048`), §3.14 (`REQ-SCN-049`, `-050`), §3.15 (`REQ-SCN-053`,
`-054`), §3.17 (`REQ-SCN-057` à `-059`, `-063`), §4.1, §4.2, §4.3, §6, Historique ; `SPEC-04B` §2
(vocabulaire, **instance**), `REQ-E3D-405`, `-406`, `-410`, `-452`, `-504`, `-505`, `-521`, `-522`,
`-568`, `-641`, `-643` ; `SPEC-09` §2, §4.1 (légende), §4.1.1, §4.6, `REQ-PERS-003`, `-010`, `-017`,
`-022`, `-080` à `-085` ; `SPEC-10` §3.7 (`REQ-UI-060` à `-062`) ; `SPEC-12` §2 et `REQ-ACC-020`,
`-021`, `-031`, `-050` ; `spec/changes/` : `AMD-012`, `AMD-032` §2, `AMD-035`, `AMD-040` (§1, §2
`REQ-LUA-112` à `-114`, §3 et sa ligne `SPEC-09`), `AMD-043` (§2 `REQ-SCN-066`, §3 `REQ-SCN-067`,
`-068`, §6), `AMD-046` (§2 et §4), `AMD-049` (`REQ-SCN-075`, §3), `AMD-050` (`REQ-SCN-076`),
`AMD-052` (retrait de `REQ-SCN-077`), `Q-016`, `Q-085`, `R2-review-AMD-050-053-v1.1.md` ;
`SPEC-v1.2-INDEX.md` ; `DECISIONS-R0.md` ; `PLAN.md` ; `third_party/README.md` ;
`journal/JOURNAL.md` ; `impl/docs/` (`spec/`, `verify/`).
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
**Le serveur d'outils de rétro-ingénierie présent dans la session n'a pas été utilisé** (il est hors
des sources autorisées du côté propre). Les connecteurs externes non authentifiés n'ont pas été
sollicités.

Méthode : (a) chaque point `N-1` à `N-8` est confronté au texte de la v1.2, **citation contre
citation** ; (b) **recalcul indépendant et complet** de tout ce que la v1.2 chiffre de neuf :
l'enroulement de `REQ-SCN-067` à 90° sur la face avant du backdrop (point par point, puis projection
et écart à la corde), le cadrage du « cadrage large B », les éclairements de `REQ-SCN-075` avec son
terme spéculaire, la nouvelle interpolation `k`/`e`, la table de quatre vecteurs de `REQ-SCN-081`
avec l'albédo corrigé, la géométrie de l'ombre de `REQ-SCN-082`, les marges du « gros plan A » et du
« gros plan R », et la Note de marge du canal rouge ; (c) contrôle de l'existence et du sens des
**44 renvois `REQ` distincts** (200 occurrences) et des 8 renvois `AMD` du fichier ; (d) relecture
d'étanchéité du texte nouveau (§1 références publiques, mode MSVO, §3 point 11).
Note de provenance : inchangée — l'en-tête renvoie au « rapport de comparaison avec 3dSen 2.0 »,
document du côté sale non lisible ici ; convention déjà admise, aucune de mes vérifications ne
s'appuie dessus.

---

## 1. Verdict

**Étanchéité : acquise.** Aucune fuite. Le texte nouveau de la v1.2 est le passage le plus sensible
de l'amendement — il nomme une technique publique par exigence et un dépôt tiers — et il reste dans
les catégories autorisées. En particulier, le **mode MSVO** est écrit proprement : le libellé est
justifié comme valeur visible du réglage, le programme d'origine n'est plus décrit que par une
formulation juridique (« une technique tierce que cette réimplémentation ne peut pas reprendre telle
quelle »), et le remplacement est présenté comme un **choix de Zaxnes** (portage d'une
implémentation publique sous licence MIT), non comme un fait sur l'original. Le fait que « MSVO »
se lise publiquement *Multi-Scale Volumetric Obscurance* et que la filiation de cette famille avec
le MiniEngine de Microsoft soit documentée publiquement rend l'inférence « MSVO ⇒ multi-échelle »
accessible **sans** le côté sale : ce n'est donc pas une fuite. Une seule **réserve faible**, déjà
présente en v1.1 et non aggravée : la phrase du §1 qui attribue au programme d'origine une famille
de techniques (`N2-8`).

**Les huit points de la relecture précédente sont levés : 8 sur 8.** Les deux bloquants le sont au
fond, pas en surface. `N-1` est entièrement refait : le critère de courbure quitte la shape de
32 unités pour la **face avant du backdrop**, la position de la surface est fixée dans les deux
cadrages, le quantificateur est restreint aux rainures **horizontales** avec l'explication
géométrique (« l'enroulement gauche-droite laisse `v` inchangé »), les verticales et la rainure
centrale sont **exclues nommément**, et le nouveau chiffrage est **juste** : j'ai refait l'enroulement
à 90° (`k` = 0,0061359, `f` = 1,026175, `h` = 12,4041), la face avant passe bien de `z` = −13,242 à
`z` = +35,892 pour `u′` = ±118,62, la focale vaut bien 771,2 px, les deux gradients 7,929 et
5,268 px/unité sont exacts et leur différence est bien **2,661**, d'où 21 / 43 / 64 / 85 / 106 / 128
/ 149 px — tous au-dessus du seuil de 15 px, avec une marge confortable. Le cadrage tient aussi :
273,9 unités de largeur visible pour 256, bords courbés à 118,6 pour une demi-largeur de 182,2.
`N-2` est aligné sur la lecture juste, aux **quatre** endroits (§2.2 4ᵉ tiret, §3 point 3, portées de
`REQ-SCN-083` et de `REQ-SCN-084`), avec le cas des tiles « ombre seule » ajouté : plus aucune
contradiction sur un observable. `N-3` reçoit deux configurations nommées (**gros plan A**, **gros
plan R**) et quatre variantes ; j'ai vérifié que les deux invariants insatisfaisables le sont
devenus : le sol de 96 × 96 couvre bien `2 r` = 40 unités (48 > 40) et la marge du canal rouge est
réelle — avec un sol d'albédo linéaire 0,202 et un cube rouge pur, `R_sol` = 0,153 et
`R_reflet` = 0,758, soit un facteur **4,9** et un seuil franchi dès que 25 % de la part rouge est
rapportée. `N-4` à `N-8` sont appliqués tels quels et exacts, y compris l'albédo **0,503** et la
coïncidence des repères `REQ-E3D-504` / `REQ-E3D-522`, que j'ai revérifiée (centre du voxel
`(i, j, k)` = `(i − 3,5 ; j − 3,5 ; k − (D−1)/2)` dans les deux).

**Références et numérotation : franchi.** `REQ-SCN-078` à `-084` restent libres et contigus, aucun
n'apparaît ailleurs dans `spec/`, `REQ-SCN-077` reste retirée (`AMD-052`). Les **44 renvois `REQ`
distincts** du fichier existent tous et disent ce que le texte leur fait dire ; l'index (§3 point 9)
est désormais complet — les cinq exigences amendées (`REQ-SCN-058`, `REQ-UI-061`, `REQ-SCN-075`,
`REQ-SCN-066`, `REQ-LUA-114`) y sont marquées. Les priorités de l'en-tête correspondent aux énoncés,
et la clause de repli du portage MSVO est bien formée.

**Fond : non franchi — mais pour une raison nouvelle, et non pour un reliquat.** Les points de la
v1.1 étant tous levés, j'ai refait le travail d'arithmétique sur l'**ensemble** du fichier, y compris
ce que les deux rapports précédents avaient déclaré vérifié. Deux défauts en sortent :

- **`N2-1` — le critère (auto) de `REQ-SCN-082` est faux, et ne l'était pas moins en v1.1 : la
  relecture précédente s'est trompée et je dois le dire.** L'ombre d'un **cube** de 8 × 8 × 8 unités
  sous une lumière oblique n'est **pas** « un carré de 8 × 8 unités » : c'est l'enveloppe convexe de
  l'ombre de sa face basse (décalée de 29/3 = 9,667) et de celle de sa face haute (décalée de
  37/3 = 12,333), soit un **hexagone** allant de 5,667 à **16,333** et non de 5,667 à 13,667. La
  frontière annoncée « aux coordonnées entières 6 et 14 » est juste du côté `−x`/`−z` et **fausse**
  du côté `+x`/`+z` (elle tombe à 16), et deux arêtes diagonales viennent s'ajouter. Une
  implémentation conforme échoue au critère. Correction en une ligne (voir §4).
- **`N2-2` — `REQ-SCN-081` demande une normale que `REQ-E3D-522` interdit.** La Modulation prescrit
  la « **normale interpolée** de la surface en MARCHING » et la Note parle de « surface lissée »,
  alors que `REQ-E3D-522` impose des **normales plates par triangle** (« trois sommets propres »,
  « normale plate »), ce que `Q-016` relève explicitement comme excluant les normales moyennées. Un
  implémenteur se trouve donc devant deux textes normatifs contradictoires — exactement la situation
  que la salle blanche interdit de trancher en silence. Conséquence seconde : la justification de la
  correction `N-6` (« pour qu'aucune marche d'albédo n'apparaisse le long d'une surface MARCHING
  lisse ») est **sans objet**, puisque `k` et `e` restent constants par triangle et que des marches
  apparaissent de toute façon aux arêtes.

Aucun des deux ne rouvre une décision du propriétaire : `N2-1` est un choix de scène de test,
`N2-2` le choix d'un mot (et, au plus, une phrase de portée). Six points non bloquants
complètent le tableau (`N2-3` à `N2-9`), dont deux touchent la testabilité.

**Réponse à la question de la v1.0, dernier état.** Elle reste **oui, pour ce que la v1.2 promet** —
et la v1.2 l'améliore sensiblement sans changer le contrat. Les **références publiques orientantes**
sont une bonne décision : elles suppriment le seul vrai coût que faisait peser le choix de ne pas
fixer l'algorithme (l'implémenteur devait choisir une famille de techniques sans indice), sans
reprendre la contrainte qui avait rendu la v1.0 insatisfaisable. Les deux exigences d'écran sont
désormais accompagnées de configurations exécutables par un tiers, donc l'unique réserve de la v1.1
sur ce point (`N-3`) tombe. Reste une maladresse de rédaction, non une ambiguïté de fond : un
`DEVRAIT` déclaré « non normatif » (`N2-7`).

---

## 2. Tableau des huit points de la relecture précédente

| Point | Sujet (v1.1) | Verdict | Vérification |
|---|---|---|---|
| **N-1** | critère de courbure de `REQ-SCN-080` arithmétiquement insatisfaisable (rainures verticales droites par construction ; 2,7 px au lieu de 5 ; position de la shape non fixée) | **levé** | Première correction proposée retenue : la mesure passe à la **face avant du backdrop** (256 × 240, `Backdrop.Enable`, albédo 0,503), dans un **cadrage large B** nommé et chiffré. Les trois défauts sont traités : (a) le quantificateur ne porte plus que sur les rainures **horizontales** à 8–56 unités du centre, les **verticales** et la rainure `y` = 120 étant **exclues** avec leur raison (« l'enroulement gauche-droite de REQ-SCN-067 laisse la coordonnée `v` inchangée ») ; (b) le seuil passe à **15 px** pour des écarts calculés de 21 à 149 px — recalculés un par un, exacts (coefficient 2,661 = 7,929 − 5,268, lui-même issu de `z` = −13,242 au centre et +35,892 aux bords, `u′` = ±118,62) ; (c) la **position** de la surface est fixée dans le cadrage large B **et** dans le gros plan G (« centrée sur C = (128, 120, 0) », « elle occupe `x` ∈ [112, 144] … »). Le cadrage est vérifié : largeur visible 273,9 pour 256 unités, marge de 8,9 de chaque côté, bords courbés à 118,6 < 182,2 ; les lignes mesurées restent dans le cadre (±444 px pour ±540). Le protocole de mesure est écrit (« colonne de pixels par colonne de pixels, au minimum local de luminance », corde entre extrémités), et la lisibilité est garantie : j'ai calculé le contraste rainure/voisines avec `REQ-SCN-075` **terme spéculaire compris**, de 31,6 niveaux sRGB au centre à 17,6 au bord droit où `N·L` = 0 — la valeur annoncée de 0,816 à gauche et 0 à droite est exacte. La **Note « portée du critère de courbure »** est en outre un ajout honnête : elle déclare ce que le critère **ne** discrimine pas (avant / après enroulement), au lieu de le laisser croire. |
| **N-2** | objets transparents, skybox et HUD déclarés occultants contre l'ordre de frame fixé par le même amendement | **levé** | Les trois endroits visés sont alignés sur la lecture de `REQ-SCN-084`, et un quatrième (portée de `REQ-SCN-084`) est rendu homogène : « comme ils sont dessinés **après** les deux passes d'écran — leur passage de profondeur compris, la passe transparente étant indivisible par objet (REQ-SCN-040 lu avec AMD-012) —, ils **n'occultent pas** …, ne sont **pas reflétés**, et ne sont ni assombris par l'occlusion ni porteurs de reflets ». Vérifié contre `AMD-012` (« passage de profondeur et tri … **par objet entier** ») ✓. Le **HUD** est traité (« dessiné encore après, non plus »), la **skybox** correctement scindée (« peut être reflétée mais n'occulte jamais : elle est à distance infinie, derrière toute la scène, REQ-SCN-049 ») ✓, le **tunnel**, les **modèles TV** et le **rectangle du mode 2D** gardent leur statut d'occulteurs non modifiés ✓, et le cas voisin suggéré est ajouté : les tiles « **ombre seule** » (`REQ-SCN-041`, non dessinées) n'écrivent aucune profondeur, n'occultent pas, ne sont pas reflétées, **mais projettent leur ombre** — ce qui est exactement ce que dit `REQ-SCN-041` ✓. `REQ-SCN-058` amendée (§3 point 3) porte la même clause, mot pour mot compatible. Plus aucune contradiction : les deux observables de 55-8 sont tranchés. |
| **N-3** | invariants « auto » de §2.4 et §2.5 sans configuration nommée ; deux d'entre eux insatisfaisables | **levé** | Deux configurations complètes sur le modèle du gros plan G : **gros plan A** (1920 × 1080, `video.anti_alias` = 0, `video.res_scale` = 0, échelle 1, courbure nulle, TV Box None, skybox noire, `render.shadow` faux, **tous les autres effets à Off** nommément, `spl.ao` = 2 et les quatre `ao.*` à leur défaut, albédos **linéaires** donnés, lumière et caméra chiffrées) et **gros plan R** (idem, sol sombre 0,202, cube rouge pur), plus les variantes **A-h**, **R-m**, **R-s**, **R-h**. Les trois insatisfaisabilités sont levées, vérifiées par recalcul : (1) le sol passe à **96 × 96** unités, soit 48 du centre, donc au-delà de `2 r` = 40 ✓ ; (2) la condition « +10 % du canal rouge » devient `R_reflet ≥ 1,222 · R_sol`, soit 0,187 contre une valeur pleine de 0,758 : il suffit que **25 %** de la part rouge soit rapportée, et la **Note de marge** le démontre dans le texte même ✓ (j'ai refait les deux éclairements : 0,202 × 0,7586 = 0,153 et 1,000 × 0,7586 = 0,759 ✓, `N·L` = 0,577 sur le sol comme sur les deux faces visibles du cube ✓) ; (3) la **variante R-s** fournit l'objet à refléter qui manquait, avec un placement qui fonctionne — j'ai tracé le rayon réfléchi depuis la face supérieure du bloc : il atteint bien la face `(−1, 0, 0)` du cube rouge, laquelle est face à la caméra, donc présente dans l'image, ce que `REQ-SCN-084` exige (« seule la **scène déjà rendue** peut être reflétée ») ✓. La phrase « tous les autres effets à Off » demandée est là, dans les deux configurations ✓. |
| **N-4** | critère amendé de `REQ-UI-061` ignorant les règles de visibilité | **levé** | §3 point 6 : « … les douze paramètres de SPEC-06 REQ-SCN-079 **apparaissant selon ses règles de visibilité** (quatre avec les valeurs par défaut — occlusion, ombres pixel, grille, reflets —, les douze avec `spl.ao` ≠ 0 et `spl.reflections` = 1) ; aucun autre paramètre n'apparaît. » Recompté : avec les défauts, 4 paramètres sur 12 restent visibles (les 4 `ao.*` et les 4 `ssr.*` sont masqués) ✓. La phrase du critère d'origine (« Les douze libellés sont présents, et aucun autre paramètre n'apparaît ») est bien celle de `SPEC-10`, retrouvée mot pour mot ✓, et la ligne `Style` est bien placée après `Skybox`, qui est la 11ᵉ ligne de la table, avant `Full Screen` ✓. |
| **N-5** | `REQ-SCN-082` muette sur `MARCHING`, règle d'axes pouvant n'en sélectionner aucun | **levé** | Seconde correction proposée retenue : « les **deux axes de plus petit `\|n_a\|`** (… en cas d'égalité, l'axe de plus petit indice dans l'ordre `x`, `y`, `z`) ». La règle sélectionne **toujours** exactement deux axes, y compris pour une normale proche de (0,577 ; 0,577 ; 0,577) où l'ancienne n'en sélectionnait aucun ✓, elle est déterministe en cas d'égalité ✓, et elle **coïncide** avec l'ancienne sur toute face GREEDY (`\|n_a\|` = 0 sur les deux axes du plan, `REQ-E3D-505`) ✓ — la phrase qui l'affirme est exacte. Plus de plaque où l'effet disparaîtrait. Réserve de rédaction induite : `N2-3`. |
| **N-6** | seuil d'alignement à 0,9 produisant une marche d'albédo de 10 % ; bande `0,035 < d_a < 0,04` | **levé** | Les deux volets sont appliqués. (a) interpolation : `t = clamp((m − 0,8)/0,1, 0, 1)`, `k = 0,38 + 0,24 t`, `e = 0,20 − 0,10 t` — vérifié aux deux bouts : `t` = 1 donne bien `k` = 0,62 / `e` = 0,10 (face alignée, `m` = 1) et `t` = 0 donne `k` = 0,38 / `e` = 0,20 (face à 45°, `m` = 0,7071 ≤ 0,8) ✓, et la formule est continue en `m`. (b) l'**arête claire** devient contiguë à la rainure : `0,035 ≤ d_a ≤ 0,16` avec `(0,16 − d_a)/0,125`, qui vaut `1 + w·e` en `d_a` = 0,035 et exactement 1 en `d_a` = 0,16 ✓ — la bande parasite à `f_a` = 1 a disparu et il ne reste qu'**une** transition à lisser, celle que le texte nomme. La table des quatre vecteurs suit : (0,4 ; 2) donne `1 + 0,10 × 0,06/0,125` = **1,048** ✓, conforme à la valeur affichée. Deux réserves induites : `N2-2` (la justification MARCHING tombe) et `N2-5` (provenance des constantes). |
| **N-7** | coquilles du gros plan G (albédo 0,500 ; « tile centrale » inexistante) | **levé**, avec une coquille de remplacement | Albédo : « soit un albédo linéaire de **0,503 ± 0,002** » ✓ — recalculé, sRGB 188 vaut 0,50297 en linéaire, et j'ai revérifié que les quatre vecteurs sRGB du critère restent justes avec cette valeur (166,1 / 133,5 / 169,6 / 106,9, tous dans ±3) ✓. Tile nommée : « la tile **adjacente au centre de la face du côté `+x`, `+y`** » ✓ — mais les coordonnées de scène données entre parenthèses sont fausses : `N2-4`. |
| **N-8** | résidus documentaires (colonne UI, `REQ-PERS-017`, index, en-tête `AMD-043`, renvois `REQ-E3D-522` et `REQ-E3D-406`) | **levé**, les cinq | (a) **Colonne UI** : §4 ouvre par un paragraphe qui cite la légende de `SPEC-09` §4.1 et adopte la forme d'`AMD-046` §4 — « — dans l'original (hors du panneau de REQ-UI-061) ; dans Zaxnes : … », abrégée dans la table ✓ ; la colonne **« Comportement »** manquante est ajoutée, et la table compte bien les **onze** colonnes du §4.1.1 (vérifié contre la ligne `video.anti_alias`) ✓. (b) **`REQ-PERS-017`** : §3 point 7 dit explicitement qu'elle **n'est pas amendée**, avec la bonne raison (son énumération vise les réglages sans effet **dans l'original**, ce qui n'est pas le cas de `ao.thickness`, qui agit en mode MSVO), et `REQ-SCN-079` la cite **par analogie** ✓ — c'est la seconde branche que le rapport proposait, et la meilleure. (c) **Index** : §3 point 9 marque les cinq exigences amendées ✓. (d) **En-tête d'`AMD-043`** : « l'en-tête de colonne « Id Lua (AMD-040 §3, **style 1**) » devient « … **style Enhanced : identifiant 4, 1 accepté** » » ✓, citation exacte retrouvée dans `AMD-043` §2. (e) **Renvois** : `REQ-E3D-522` est désormais cité partout à côté de `REQ-E3D-504`, avec la mention « les deux repères coïncident » que j'ai revérifiée ✓, et l'énumération du pas 3 de `REQ-SCN-082` suit maintenant l'ordre de `REQ-E3D-406` (« déformation animée, puis échelle, rotation et position ») ✓, conforme à `P = Pos + R · (S ⊙ (… + v + δ))`. |

**Récapitulatif** : **8 levés sur 8**, 0 partiellement, 0 non levé. Les deux bloquants de la v1.1
sont traités au fond et leur nouveau chiffrage est exact.

**Contrôle de non-régression sur les 24 points de la v1.0.** Je les ai rejoués sur la v1.2 : 23
restent levés. Un seul change de verdict, et par ma faute, non par celle de la v1.2 : **55-14**
(« critère de `REQ-SCN-082` non discriminant, alignement manquant ») passe de *levé* à
**partiellement levé** — la clause discriminante, l'alignement sur `REQ-E3D-504` avec `N` = 1 voxel,
le biais appliqué au point déplacé et les deux niveaux sRGB (195,1 et 118,3, recalculés) sont bien
là et justes, mais la géométrie de l'ombre est fausse (`N2-1`). Les quatre points « non retenus » de
la v1.0 restent justifiés, et mieux qu'avant (§3).

---

## 3. Jugement des points « non retenus » et des décisions nouvelles

Le §5 de la v1.2 déclare **tous** les points `N-1` à `N-8` retenus — je l'ai vérifié, c'est exact.
Les quatre points de la v1.0 restent non retenus ; leur raison est **inchangée sur le fond et
renforcée sur la forme**. Je les juge toujours recevables, et deux d'entre eux le sont mieux :

1. **La famille de loi d'occultation (55-10 point 1) et le repère de la marche (55-11 d) —
   raison recevable, désormais presque sans coût.** Le propriétaire maintient qu'il ne fixe pas
   l'algorithme, mais il **nomme une référence publique** par exigence (Chapman 2013 pour
   l'hémisphère, McGuire et Mara 2014 pour la marche puis la dichotomie) et, pour la seconde, cite
   un article avec sa revue, son volume et son année. C'est la meilleure des trois réponses
   possibles : elle supprime le vrai défaut que 55-10 visait (l'implémenteur devait inventer une
   famille de techniques sans indice) sans réintroduire la contrainte arithmétique qui avait rendu
   la v1.0 insatisfaisable, et sans faire de la spec un pseudo-code. Elle est conforme au
   `SPEC-GUIDE`, qui admet la documentation publique comme source et encourage les renvois à du code
   public sous licence. Réserve de rédaction seulement : `N2-7`.
2. **La formule d'atténuation de bord (55-11 c) — raison recevable, garde-fou inchangé.** `DOIT`
   décroître continûment vers 0, « aucun bord franc ne DOIT être visible », jugé par revue
   (`SPEC-12`, `REQ-ACC-050` : « conforme si la revue est archivée ») ✓.
3. **Les lignes libretro (55-15) — raison recevable.** `REQ-PERS-083` prévoit la colonne « — »,
   `SPEC-09` §4.6 a la liste « Non exposés en libretro » que §3 point 7 complète correctement,
   `REQ-PERS-085` n'interdit rien pour les réglages U, et `REQ-PERS-081` est maintenue malgré la
   non-exposition ✓.
4. **La reformulation du 55-6 — remplacée par mieux** (poids d'axe), jugé en v1.1 ✓.

**Décisions nouvelles de la v1.2, jugées.**

- **Références publiques orientantes** : **bonne décision**, acceptée sans réserve de fond. Elles ne
  touchent ni la priorité, ni la liberté, ni les critères ; le texte le dit deux fois et le §5 le
  répète.
- **Portage MSVO sous licence MIT, avec repli déclaré sur Depth** : **décision recevable**, et bien
  bordée côté spec (P3, repli `(D)`, conditions de licence, entrée `third_party/README.md`, invariant
  d'identité sous le repli, invariant inverse quand le portage est là, grisage de `ao.thickness`
  conditionnel). La compatibilité **MIT → GPL-3.0** est correcte, et la remarque « OpenGL 3.3 n'a pas
  de nuanceurs de calcul » est exacte (les *compute shaders* arrivent en 4.3) : le portage « en
  passes de fragments » est donc une nécessité réelle, pas une précaution. Une réserve de procédure,
  non de fond : l'autorisation d'une **nouvelle source tierce** doit être inscrite là où le côté
  propre lit ses règles, pas seulement dans `third_party/README.md` (`N2-6`).

Aucun point n'a été écarté sans raison, et aucune raison donnée n'est un expédient.

---

## 4. Points nouveaux

### N2-1 — Le critère (auto) de `REQ-SCN-082` décrit l'ombre d'un cube comme celle d'une plaque plate. **BLOQUANT.**

Le critère écrit : « Cube : une tile pleine de `sizeZ` = 8 (8 × 8 × 8 unités), empreinte verticale
centrée sur le centre du sol, face basse à **29 unités** au-dessus du sol. Lumière blanche
`Light.Direction` = (1, −3, 1), intensité 1 : **l'ombre du cube est un carré de 8 × 8 unités** dont
le centre est à (+29/3 ; +29/3) = (+9,667 ; +9,667) …, et ses bords tombent donc à **5,667 et
13,667** unités du centre du sol, au milieu d'un voxel », puis : « la frontière entre cellules
ombrées et cellules éclairées suit **exactement** des bords de cellule, aux coordonnées entières
**6 et 14** unités du centre du sol ».

L'ombre d'un **solide** sous une lumière directionnelle est la projection de tout le solide, donc
l'**enveloppe convexe** des projections de ses sommets — pas celle de sa seule face basse. Avec
`d` = (1, −3, 1), le décalage horizontal d'un point situé à la hauteur `h` au-dessus du sol vaut
`(h/3 ; h/3)` :

| Face du cube | Hauteur au-dessus du sol | Décalage | Carré d'ombre (par axe) |
|---|---|---|---|
| basse | 29 | 9,667 | [5,667 ; 13,667] |
| haute | 37 | 12,333 | [8,333 ; 16,333] |

L'ombre est donc l'enveloppe convexe de ces deux carrés : un **hexagone** de sommets
(5,667 ; 5,667), (13,667 ; 5,667), (16,333 ; 8,333), (16,333 ; 16,333), (8,333 ; 16,333),
(5,667 ; 13,667). Conséquences : la boîte de l'ombre mesure **10,667 unités** de côté et non 8 ; le
bord n'est à 5,667 que du côté `−x` et `−z` ; du côté `+x` et `+z` il est à **16,333**, donc la
frontière de cellules tombe à la coordonnée **16** (le centre 15,5 est dans l'ombre, 16,5 non) et non
à 14 ; et deux **arêtes diagonales** relient les deux régimes, le long desquelles la frontière forme
un escalier de cellules. Une implémentation parfaitement conforme à `REQ-SCN-082` échoue donc au
critère sur deux de ses quatre bords.

Ce défaut est **antérieur à la v1.2** : il figurait déjà dans la v1.1, et les deux relectures R2
précédentes l'ont confirmé à tort (la v1.1 du rapport écrit « bords à 5,667 et 13,667 ✓ … frontières
aux coordonnées entières 6 et 14 ✓ »). L'erreur vient d'avoir projeté la seule face basse. Le reste
du critère est juste : les deux niveaux sRGB (195,1 et 118,3, recalculés avec `REQ-SCN-028`,
`REQ-SCN-027` et le facteur 0,2 de `REQ-SCN-031`) sont exacts, l'alignement des cellules sur les
entiers depuis le centre du sol l'est aussi (4 × 4 tiles, centres de tiles aux offsets ±4 et ±12,
bords de voxel aux entiers), et la clause discriminante repose sur le fait que le bord **non snappé**
tombe au milieu d'un voxel, ce qui reste vrai en 5,667.

*Corrections proposées*, dans l'ordre de préférence :

1. **remplacer le cube par une plaque** : « une tile pleine de `sizeZ` = **1** (8 × 8 × 1 unités) ».
   Les deux décalages deviennent 9,667 et 10,000, l'ombre va de 5,667 à 14,000 par axe, les cellules
   de centre dans cet intervalle sont ombrées, et les frontières retombent **exactement** sur 6 et
   14 comme le texte l'annonce (recalculé ; les deux minuscules arêtes diagonales n'affectent qu'un
   coin de cellule, hors des bords mesurés). C'est la correction d'un seul chiffre, et elle préserve
   mot pour mot le reste du critère, les niveaux sRGB compris ;
2. ou **garder le cube et restreindre la mesure** : « la frontière des deux bords situés du côté
   `−x` et `−z` du centre du sol suit exactement des bords de cellule, à la coordonnée entière
   **6** », en remplaçant « l'ombre du cube est un carré de 8 × 8 unités » par la description
   hexagonale (et en supprimant la coordonnée 14) ;
3. dans les deux cas, écrire le raisonnement en une phrase (« l'ombre d'un solide est l'enveloppe
   convexe des ombres de ses faces basse et haute »), pour qu'une relecture ultérieure n'ait pas à
   le redécouvrir.

### N2-2 — `REQ-SCN-081` prescrit une normale interpolée que `REQ-E3D-522` interdit. **BLOQUANT (clause MARCHING).**

Deux passages de `REQ-SCN-081` :

- Modulation : « Soit `n` la normale unitaire du maillage au point considéré (normale de la face en
  GREEDY et GREEDY_MULTICOLOR, **normale interpolée de la surface en MARCHING**) » ;
- Notes : « En MARCHING, **aucune face n'est plane** : la formule s'applique par point, avec la
  **normale interpolée**, et les rainures suivent la **surface lissée**. »

Or `SPEC-04B` `REQ-E3D-522` (P2) impose : « Chaque triangle DOIT avoir **trois sommets propres**
portant la **normale plate** `N = (B − A) × (C − A) / ‖…‖` ». `Q-016` le relève explicitement :
une construction à normales moyennées est « ce que REQ-E3D-522 interdit (« trois sommets propres »,
« normale plate ») ». En MARCHING, chaque triangle **est** plan et porte une normale constante ; il
n'y a pas de normale interpolée à lire, et la surface rendue est **facettée**, non lissée.

Trois conséquences :

- un implémenteur a devant lui deux textes normatifs contradictoires sur la même grandeur, sur un
  point qui change le résultat (la normale détermine `w_a`, donc quelles familles de rainures
  existent, et `m`, donc `k` et `e`) ;
- la **justification de la correction `N-6`** tombe : « entre les deux, `k` et `e` sont interpolés,
  **pour qu'aucune marche d'albédo n'apparaisse le long d'une surface MARCHING lisse** ». Avec des
  normales plates, `m` est constant par triangle, donc `k` et `e` le sont aussi, et des marches
  d'albédo apparaissent **à chaque arête de triangle**, interpolation ou pas. L'interpolation reste
  un progrès (elle réduit l'amplitude des marches de 10 % à au plus 10 % répartis), mais elle
  n'atteint pas le but affiché ;
- la Note « aucune face n'est plane » est fausse telle quelle.

*Correction proposée* : remplacer « normale interpolée de la surface en MARCHING » par « **normale
plate du triangle en MARCHING (REQ-E3D-522)** », et réécrire la Note (« en MARCHING la surface est
facettée : la formule s'applique par point avec la normale plate du triangle, et les rainures
suivent la surface facettée ; `k` et `e` étant constants par triangle, l'interpolation d'alignement
ne supprime pas les marches aux arêtes, elle en réduit l'amplitude »). Réécrire du même coup la
raison de l'interpolation dans la Modulation, qui ne peut plus être « qu'aucune marche n'apparaisse »
mais « que l'amplitude de la marche reste faible ». Si le propriétaire préfère une surface lissée,
c'est `REQ-E3D-522` qu'il faut amender, et alors `Q-016` doit être rouvert — mais rien dans
`AMD-055` n'appelle ce choix.

### N2-3 — Le pas 2 de `REQ-SCN-082` sort le point d'évaluation de la surface en MARCHING. *Non bloquant.*

La règle `N-5` est juste, mais les deux phrases qui l'accompagnent ne valent que pour une face
alignée sur un axe :

- « le troisième axe est laissé tel quel, **de sorte que le point reste sur la face** » ;
- Notes : « le point d'évaluation se déplace de 0,7 unité au plus **le long de la face** ».

Pour une face GREEDY (normale `±X`, `±Y`, `±Z`), les deux axes ramenés sont bien dans le plan de la
face et le troisième la quitte : le point reste dessus ✓. Pour une surface MARCHING de normale
quelconque, ramener deux coordonnées au centre du voxel **décolle** le point de la surface : le
déplacement atteint `√(0,5² + 0,5²)` = 0,707 unité, dont une composante **normale** non nulle. La
borne de 0,7 unité reste donc exacte en module, mais « le long de la face » et « reste sur la face »
sont faux, et le point peut se retrouver jusqu'à ~0,7 unité **devant ou derrière** la surface — très
au-delà du biais de 0,02 unité de `REQ-SCN-031`, donc avec un risque réel d'ombre décollée ou
d'acné sur les surfaces MARCHING, c'est-à-dire précisément ce que la Note dit vouloir éviter.

*Correction* : soit exclure MARCHING de `REQ-SCN-082` (option la plus simple, déjà envisagée en
`N-5`, et cohérente avec un effet qui imite la grille de pixels d'un décor cubique), soit garder la
règle et corriger les deux phrases, en bornant explicitement la composante normale du déplacement
(par exemple « en MARCHING, le point ramené est reprojeté sur le plan du triangle » — une ligne).
Les critères du fichier n'en dépendent pas : ils portent tous sur des faces GREEDY.

### N2-4 — Coquille de coordonnées dans la tile nommée par la correction `N-7`. *Non bloquant.*

« Points donnés dans le repère local de **la tile adjacente au centre de la face du côté `+x`,
`+y`** (REQ-E3D-504 ; son centre est en **(132, 132)** dans le repère de scène). » La shape du gros
plan G occupe `x` ∈ [112, 144] et `y` ∈ [104, 136] ; les centres de tiles sont donc en
`x` ∈ {116, 124, 132, 140} et `y` ∈ {108, 116, 124, 132}. La tile adjacente au centre de la face
(128 ; 120) du côté `+x`, `+y` a son centre en **(132, 124)**, non (132, 132) — (132, 132) est la
tile du **coin** `+x`, `+y`. Les valeurs mesurées sont identiques (les quatre tiles candidates
partagent texture et repère local, et les points du tableau sont donnés en coordonnées locales),
mais un lecteur qui localise la tile par ses coordonnées de scène ne mesure pas celle que la phrase
désigne. *Correction* : « son centre est en (132, 124) ».

### N2-5 — La rampe d'interpolation de `k` et `e` est présentée comme mesurée alors qu'elle est un choix de la réimplémentation. *Non bloquant.*

La Note de `REQ-SCN-081` range « les seuils **0,8 et 0,9** de l'interpolation d'alignement » parmi
les constantes « mesurées sur 3dSen 2.0 ». Or l'interpolation est née en v1.2, en réponse à `N-6`,
avec une justification de conception (« pour qu'aucune marche d'albédo n'apparaisse ») et non de
mesure ; la v1.1 avait un **basculement franc** au seuil 0,9. Ce que les deux rapports ont traité
comme mesuré, ce sont les deux **valeurs d'arrivée** (0,62 / 0,10 pour une face alignée, 0,38 / 0,20
pour une face oblique) ; la rampe et sa largeur, elles, sont un lissage ajouté par Zaxnes. Les
attribuer à une mesure fausse la traçabilité et, surtout, empêche de les discuter comme un écart.

*Correction* : laisser 0,9 dans la liste des constantes mesurées, en sortir **0,8** et la rampe, et
écrire une phrase : « l'interpolation entre `m` = 0,8 et `m` = 0,9 est un lissage propre à Zaxnes
**(D)**, destiné à limiter la marche d'albédo entre les deux régimes ; les deux régimes eux-mêmes
sont mesurés ». Voir aussi `N2-2` : la raison affichée de ce lissage doit être corrigée.

### N2-6 — L'autorisation de la nouvelle source tierce n'est inscrite qu'à un seul endroit. *Non bloquant, mais à régler avant la mise en œuvre.*

`REQ-SCN-083` demande de « **porter l'implémentation publique sous licence MIT** … du **MiniEngine
de Microsoft** (dépôt public `DirectX-Graphics-Samples`) », et §3 point 11 prévoit en conséquence une
entrée dans **`third_party/README.md`**. C'est nécessaire mais pas suffisant : les sources que le
côté propre a le droit de lire sont énumérées ailleurs — `PLAN.md` les fixe (et porte une ligne
dédiée, datée, pour l'ajout de `third_party/3DNes_Engine`), et les instructions de travail du côté
propre reprennent cette liste close. Le `SPEC-GUIDE`, de son côté, n'autorise nommément des renvois
qu'à `third_party/3DNes_Engine`. Un implémenteur qui suit ses règles à la lettre **n'a pas le droit**
d'aller chercher le dépôt que `REQ-SCN-083` lui demande de porter, et il n'a pas non plus à décider
seul d'étendre la liste.

Rien n'est bloqué pour autant : le portage est **P3** et son repli `(D)` est déclaré, donc un lot
d'implémentation peut livrer `spl.ao` = 1 rendu comme Depth sans attendre cette décision.

*Correction* : ajouter à §3 un point qui nomme les trois inscriptions à faire — ligne de `PLAN.md`
(nouvelle source publique sous licence, datée, comme pour `3DNes_Engine`), mention dans la liste de
sources des instructions du côté propre, et extension de la phrase du `SPEC-GUIDE` sur les renvois au
code public sous licence — en plus de l'entrée `third_party/README.md` déjà prévue. Et préciser que
l'entrée doit exister **avant** que le portage commence, ce que la dernière phrase du point 11 dit
déjà pour le repli.

### N2-7 — Un `DEVRAIT` déclaré « non normatif » : les deux lectures sont incompatibles. *Non bloquant.*

`REQ-SCN-083` : « Elle **DEVRAIT** suivre la technique décrite publiquement par le tutoriel SSAO de
John Chapman (2013) : … Cette référence **oriente sans être normative** ». Même construction dans
`REQ-SCN-084` avec McGuire et Mara. Le `SPEC-GUIDE` définit `DEVRAIT` comme « recommandé (un écart
doit être justifié) » : c'est donc bien une obligation faible, et dire dans la même phrase qu'elle
n'est pas normative annule ce qu'elle vient de poser. Le reste du paragraphe tranche d'ailleurs dans
l'autre sens (« la loi de pondération …, le rayon et le seuil du lissage restent des **choix
d'implémentation libres** »), ce qui est l'intention réelle.

*Correction* : garder le contenu et changer le mode — « **à titre indicatif**, l'implémentation peut
suivre … » ou « l'implémentation **PEUT** suivre … », en conservant la phrase « ce sont les
paramètres et les invariants qui font foi ». Deuxièmement : la référence Chapman est un tutoriel en
ligne sans identifiant stable, là où McGuire et Mara est cité avec revue, volume et année ; comme la
spec doit se suffire à elle-même pour un tiers, soit lui donner une référence pérenne, soit se
contenter de l'**énumération** qui suit déjà dans le texte (noyau hémisphérique resserré, rotation
par pixel, test de portée, flou) — laquelle est, elle, autosuffisante.

### N2-8 — Réserve faible d'étanchéité : la seule affirmation du fichier sur les techniques internes de l'original. *Non bloquant.*

§1 : « Les **deux effets d'écran du programme d'origine** reposent sur des techniques publiques et
classiques (occlusion ambiante par échantillonnage d'un **hémisphère** à partir de la profondeur,
reflets tracés dans l'**espace écran**). » C'est la seule phrase du fichier qui énonce un fait sur
l'intérieur de l'original, et la moitié n'en est pas observable : le tracé en espace écran se déduit
d'un observable que l'amendement écrit par ailleurs (« rien de ce qui est hors champ ou caché » n'est
reflété), mais l'échantillonnage d'un hémisphère ne se déduit d'aucune capture. La phrase était déjà
là en v1.1 et je l'avais acceptée ; la v1.2 ne l'aggrave pas — au contraire, son nouveau paragraphe
présente correctement Chapman et McGuire comme des **choix du côté propre**, ce qui rend la phrase du
§1 inutile à l'argument.

*Correction suggérée* (une suppression) : « Les deux effets d'écran reposent sur des techniques
publiques et classiques » — sans « du programme d'origine ». Le besoin est intact et la réserve
disparaît. Le reste du §1, y compris le paragraphe MSVO, est **étanche** : il ne dit du programme
d'origine que ce qui est juridique (« une technique tierce que cette réimplémentation ne peut pas
reprendre telle quelle ») et attribue tout le reste à Zaxnes.

### N2-9 — Résidus documentaires et de testabilité. *Non bloquants.*

- **Ligne `render.visual_style` en double.** §3 point 1 n'amende de la ligne `SPEC-09` d'`AMD-040`
  §3 que les colonnes « Effet » et « Domaine ». Mais `AMD-040` §3 porte aussi `UI` = « aucune » et
  `Libretro` = « non exposé », alors que la ligne homologue du §4 d'`AMD-055` porte
  « — / Zaxnes : liste « Style » ». Deux textes normatifs donnent donc deux contenus pour la même
  ligne de `SPEC-09` §4.1.1. *Correction* : écrire au point 1 que **la ligne du §4 du présent
  amendement remplace** celle d'`AMD-040` §3.
- **Aide de `Thickness` fausse sous le repli.** §4 : « `Thickness` — « No effect with Depth
  occlusion. » ». Sous le repli `(D)`, `spl.ao` = 1 (MSVO) est rendu *comme* Depth, et le réglage n'a
  pas d'effet non plus — l'aide affirme donc le contraire de ce que l'utilisateur observe avec le
  réglage sur `MSVO`. *Correction* : « No effect until multi-scale occlusion is available. » /
  « Sans effet tant que l'occlusion multi-échelle n'est pas disponible. », ou une aide qui couvre les
  deux cas.
- **Règle de grisage à aligner.** La prose de `REQ-SCN-079` (« il est alors présenté **grisé (D)** »
  … « Quand le portage MSVO est disponible, `ao.thickness` **agit** … et n'est plus grisé pour
  `spl.ao` = 1 ») se lit comme si le grisage cessait à l'arrivée du portage, alors que la règle juste
  est celle du §4 : « grisé **sauf en mode MSVO effectif** » (donc encore grisé avec `spl.ao` = 2).
  Une demi-phrase à reprendre dans `REQ-SCN-079`.
- **Le chiffre « au moins 15 niveaux sRGB » du cadrage large B est juste à une colonne près.** J'ai
  recalculé le contraste rainure/voisines au pire endroit (bord droit, `N·L` = 0, terme spéculaire
  nul) : 83,5 contre 65,9, soit **17,6** niveaux ✓. Mais si la colonne de pixels mesurée tombe
  elle-même dans une rainure **verticale** (`f_x` = 0,62, cas fréquent près du bord droit où 8 unités
  de scène se compriment en 7 pixels), les deux valeurs deviennent 65,9 et 51,4, soit **14,5**
  niveaux. *Correction* : annoncer « au moins 14 niveaux sRGB », ou réserver la mesure aux colonnes
  hors rainure verticale. Sans incidence sur le critère lui-même, qui repose sur un **minimum local**
  et non sur un seuil de contraste.
- **Un invariant de `REQ-SCN-084` n'est pas exécutable tel quel.** « `ssr.clarity` croissant réduit
  la **largeur de transition mesurée sur un bord de reflet** » : ni le bord, ni la ligne de mesure,
  ni la définition de la largeur (10–90 % ?) ne sont nommés, alors que les deux invariants voisins
  (`ssr.shine`, `ssr.length`) sont mesurables tels quels. *Correction* : nommer le bord et la
  mesure, par exemple « la largeur 10–90 % de l'extinction du reflet mesurée le long de la colonne
  de pixels du centre de l'image, au bout du rayon ».
- **Skybox reflétée : permissif ou normatif ?** §2.2 dit que la skybox « **peut** être reflétée » ;
  la portée de `REQ-SCN-084` dit que « **sont reflétés** … skybox, tunnel, modèles de téléviseur … ».
  Comme une marche en espace écran ne franchit jamais une profondeur infinie, refléter le ciel
  demande un traitement à part, qui doit rester **facultatif**. *Correction* : « la skybox **PEUT**
  être reflétée » dans la portée de `REQ-SCN-084` aussi.
- **`REQ-SCN-058` ne nomme toujours pas le rectangle du mode 2D** parmi les objets opaques, alors que
  les portées de `REQ-SCN-083` et `-084` s'appuient sur le fait qu'il est « dessiné avec les objets
  opaques ». Le manque était déjà relevé dans `R2-review-AMD-050-053-v1.1` ; `AMD-055` amendant
  justement `REQ-SCN-058`, c'est l'occasion de l'ajouter à la parenthèse (« tiles, backdrop, tunnel,
  modèles TV **et, en mode 2D, le rectangle du mode 2D (AMD-046 §2)** »).
- **Repli manquant pour le critère d'ordre.** Le second critère de `REQ-SCN-080` (P2) s'appuie sur le
  « cadrage large B » **et** sur l'existence de la grille, tous deux définis par `REQ-SCN-081`. Si
  `REQ-SCN-081` n'est pas retenue, le critère devient intestable alors que la clause d'ordre reste
  écrite. Une incise `(T-1)` dans la ligne « Priorités : » suffirait.

---

## 5. Ce qui est juste dans la v1.2 et doit être conservé tel quel

- Le **nouveau critère de courbure** (`REQ-SCN-080`, cadrage large B) est le meilleur critère du
  fichier : il nomme sa configuration, fixe la position de la surface, donne son protocole de mesure,
  énonce une **valeur attendue par ligne** avec une tolérance double (3 px ou 3 %), exclut nommément
  ce qui ne peut pas bouger, et **publie son propre calcul** — de sorte qu'une relecture ultérieure
  n'a qu'à le refaire, ce que j'ai fait, point par point, sans trouver d'écart. C'est le modèle à
  reprendre pour tout critère géométrique à venir.
- La **Note « portée du critère de courbure »** : déclarer ce qu'un critère **ne** discrimine pas
  (ici : avant / après enroulement, parce que l'enroulement est presque une isométrie le long de la
  surface) vaut mieux qu'un critère qui laisse croire qu'il tranche tout. Le raisonnement est juste :
  33 lignes de rainure contre 29 si la grille était évaluée après l'enroulement, pour un écart cumulé
  de 9,4 unités (128 − 118,62) — et les lignes extérieures sont effectivement sous le pixel, la
  compression horizontale atteignant 0,9 px/unité près du bord droit.
- Le **traitement de l'ordre de frame** (`N-2`) : la même clause est écrite quatre fois sans varier,
  avec sa raison à chaque fois, et le cas limite des tiles « ombre seule » a été ajouté de la propre
  initiative du rédacteur. C'est de la rédaction robuste.
- Les **gros plans A et R** et leurs quatre variantes : non seulement ils rendent les invariants
  exécutables, mais le gros plan R est **dimensionné pour sa marge** (sol sombre, cube rouge pur) et
  cette marge est **démontrée dans une Note chiffrée**. Un amendement qui prouve la satisfaisabilité
  de ses propres invariants est exactement ce qu'il faut à la matrice de traçabilité.
- Le **mode MSVO** : intention, repli déclaré `(D)`, conditions de licence, invariant d'identité sous
  le repli **et** invariant inverse quand le portage est là, grisage conditionnel, clause de priorité
  dédiée. Rien n'y est laissé à l'interprétation, et le repli garantit qu'aucun lot d'implémentation
  ne dépend du portage.
- La **correction `N-6` dans sa partie arête claire** : rendre la bande claire contiguë à la rainure
  supprime d'un coup la discontinuité parasite et la seconde transition à lisser, et la table des
  vecteurs suit (1,048).
- Le **§4** (treize lignes aux onze colonnes de `SPEC-09` §4.1.1, colonne UI à la forme d'`AMD-046`,
  libellés et aides EN/FR complets) et le **§5** (choix tranchés, points non retenus avec leur
  raison, points retenus avec leur traitement) restent au niveau des amendements les mieux tenus de
  `spec/changes/`.
- La **compatibilité ascendante** de `REQ-SCN-078` : rejouée une fois de plus contre les critères
  d'`AMD-040` (`Set(0, …)` → faux, `Set(1, "spl.bend_zx", 500)` → vrai avec `Get` → 180,
  `Set(7, …)` → faux, `VisualStyle = 5` → 0) et contre la normalisation du §3 (9 → 5, 1,4 → 1,
  0 → 1, 4 → 3) : aucun critère existant ne devient faux.

---

## 6. Conclusion — gate R2

- **Filtre anti-fuite : franchi.** Aucune fuite. Le passage le plus exposé de la v1.2 — les
  références publiques et le portage MSVO sous licence MIT — est rédigé du bon côté de la frontière :
  il décrit des choix de Zaxnes et des sources publiques, jamais l'intérieur de l'original. Une seule
  réserve faible, héritée de la v1.1 et corrigible par la suppression de trois mots (`N2-8`).
- **Références et numérotation : franchi.** `REQ-SCN-078` à `-084` libres et contigus,
  `REQ-SCN-077` toujours retirée, priorités cohérentes avec les énoncés, **44 renvois `REQ`
  distincts** et 8 renvois `AMD` vérifiés un à un : tous existent et disent ce que le texte leur fait
  dire, y compris les citations faites pour remplacement dans `AMD-040`, `AMD-043`, `AMD-049`,
  `SPEC-06` et `SPEC-10`. Index complet. Un seul doublon de ligne à résoudre (`N2-9`, premier tiret).
- **Arithmétique : franchi pour tout le texte nouveau, une erreur ancienne mise au jour.** Tout ce
  que la v1.2 chiffre de neuf est **juste** : enroulement à 90° (`k`, `f`, `h`, `z` = −13,242 et
  +35,892, `u′` = ±118,62), focale 771,2 px, gradients 7,929 et 5,268, coefficient 2,661 et les sept
  écarts 21 à 149 px, cadrage (154,0 / 273,9 / 182,2 / 8,9 unités), `N·L` de 0,816 à 0, contraste de
  31,6 à 17,6 niveaux sRGB, albédos linéaires 0,503 et 0,202, interpolation de `k` et `e`, arête
  claire 1,048, vecteurs sRGB 166 / 133 / 169 / 107, marges du gros plan A (48 > 40) et du gros plan
  R (`R_sol` = 0,153, `R_reflet` = 0,758, facteur 4,9, seuil à 25 %), rayon réfléchi de la variante
  R-s. Le seul chiffrage **faux** du fichier est la géométrie de l'ombre du critère de `REQ-SCN-082`
  (`N2-1`), qui datait de la v1.1 et que les deux relectures précédentes avaient validée à tort.
- **Fond : non franchi, de très peu.** 8 points sur 8 levés, 23 des 24 points de la v1.0 toujours
  levés, les quatre points non retenus mieux justifiés qu'avant. Restent **2 points bloquants** :
  `N2-1` (le critère auto de `REQ-SCN-082` n'est pas satisfaisable : l'ombre d'un cube n'est pas un
  carré, la frontière du côté `+x`/`+z` tombe à 16 et non à 14) et `N2-2` (`REQ-SCN-081` prescrit une
  normale interpolée que `REQ-E3D-522` interdit, ce qui prive en outre la correction `N-6` de sa
  justification). Les deux se corrigent respectivement par **un chiffre** (`sizeZ` = 1) et par **un
  mot** (normale plate) plus une Note réécrite ; ni l'un ni l'autre ne rouvre une décision du
  propriétaire, ni ne touche une exigence déjà déclarée prête.
- **Réponse à la question du propriétaire, dernier état : oui, et plus facilement qu'en v1.1.** Les
  références publiques orientantes lèvent la dernière objection pratique à `REQ-SCN-083` et
  `REQ-SCN-084` — l'implémenteur n'a plus à inventer une famille de techniques — sans réintroduire la
  contrainte qui avait rendu la v1.0 insatisfaisable, et les gros plans A et R rendent les invariants
  exécutables par un tiers, ce qui était la dernière réserve de la v1.1. Un implémenteur peut
  travailler **sans deviner** : rien d'observable ne manque, et rien d'observable ne se contredit —
  sous réserve des deux clauses de `N2-1` et `N2-2`, qui sont l'une un critère et l'autre un mot.
- **Est-il prêt pour la mise en œuvre ?** **Pas tout à fait, mais une v1.3 de quelques lignes
  suffit, et le chantier peut démarrer dès maintenant sur l'essentiel.** Sont **prêts tels quels** :
  `REQ-SCN-078`, `REQ-SCN-079`, `REQ-SCN-080` (clause d'ordre et son critère de courbure, entièrement
  vérifié), `REQ-SCN-083` et `REQ-SCN-084` avec leurs configurations, le §3 et le §4 — soit la
  quasi-totalité du lot, y compris les deux effets d'écran en P3 et le repli `(D)` du mode MSVO.
  Ce qui bloque est circonscrit à **deux clauses de §2.3** : le critère auto de `REQ-SCN-082`
  (`N2-1`) et la normale MARCHING de `REQ-SCN-081` (`N2-2`).
- **Recommandation** : v1.3 traitant **`N2-1`** et **`N2-2`**, puis `N2-3` (qui touche la
  satisfaisabilité de `REQ-SCN-082` en MARCHING), `N2-6` (procédure, à régler avant que le portage
  MSVO commence — pas avant le reste) et `N2-9` tirets 4 et 5 (testabilité), puis `N2-4`, `N2-5`,
  `N2-7`, `N2-8` et le reste de `N2-9` au fil de la plume. Une quatrième relecture R2 n'aurait à
  recalculer que la nouvelle géométrie d'ombre de `REQ-SCN-082` ; tout le reste du fichier est
  acquis, références et arithmétique comprises.

Le fichier `AMD-055-style-enhanced.md` n'a pas été modifié par cette relecture, ni aucun autre
fichier de `spec/` hors ce rapport et la ligne de journal ; rien n'a été commité.
