# R2 — Relecture d'`AMD-046` (courbure en mode 2D, v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-30 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-046-courbure-mode-2d.md` **en entier** et y chercher toute fuite
(code, pseudo-code, nom interne, algorithme recopié), toute incohérence avec `AMD-043`, `AMD-035`,
`REQ-SCN-055`, `Q-022`, `REQ-INP-063` / `REQ-INP-124` ou `SPEC-09`, tout critère intestable, tout
identifiant `REQ` déjà pris et toute référence cassée.

Entrées relues : `spec/changes/AMD-046-courbure-mode-2d.md` (v1.0 du 2026-10-30, texte sous revue,
**non modifié**) ; `AMD-043-courbure-espace.md` (v1.2, qu'il complète) ; `AMD-035-langue-interface.md` ;
`AMD-009-q-tickets-l7a.md` (réponse à Q-022) ; `changes/Q-022.md` ; `Q-079` lu sur la branche
`delta/v2-4-bend` du dépôt `impl` (`git show delta/v2-4-bend:docs/spec/changes/Q-079.md`) ;
`R2-review-AMD-039-045.md` et `R2-review-AMD-039-045-v1.1.md` (continuité des points 43-x, N-43-x,
T-1 à T-4) ; `SPEC-GUIDE.md` ; `SPEC-03` (REQ-PROF-061, `empty.bytes`) ; `SPEC-04` §3.17 ;
`SPEC-06` §3.9 à §3.17 (REQ-SCN-028/030/036/044/045/046/047/048/055/058) ; `SPEC-08` §3.7 et §5.3
(REQ-INP-060 à 066, REQ-INP-123) ; `SPEC-09` §2, §4.1 (légende et lignes booléennes), §4.6,
REQ-PERS-001/002/003/013/080/081/083/084 ; `SPEC-v1.2-INDEX.md` ; `PLAN.md` §3 et la liste des
catégories d'écart. Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune
tentative de contournement.

Méthode : ouverture de chacune des 11 exigences citées par l'amendement ; contrôle de l'unicité de
`REQ-SCN-074` dans tout `spec/` **et** dans toutes les branches du dépôt `impl` (`git grep` sur
l'ensemble des révisions) ; relecture de chaque tiret de `REQ-SCN-074` comme un tiers qui ne
connaîtrait que la spec, en cherchant quel test automatique il rendrait exigible ; confrontation de
la ligne `SPEC-09` à la légende normative du §4.1 et à la table libretro du §4.6 ; vérification de
la cohérence arithmétique des deux chiffres de subdivision (32 × 32 cellules ⇒ 33 × 33 = 1 089
sommets distincts et 2 048 triangles, chiffre que Q-079 annonce).

Catégories utilisées, comme dans les rapports précédents : **LEAK**, **INCONSISTENT**,
**UNTESTABLE**, **UNCLEAR**, auxquelles s'ajoutent les points **rédactionnels** et **transverses**.

---

## 0. Verdict

| Document | Verdict | Points | Réserves bloquantes |
|---|---|---|---|
| **AMD-046** — courbure en mode 2D (v1.0) | **Accepté avec réserves** | 13 (46-1 à 46-13) | **aucune pour l'étanchéité** ; 5 avant mise en œuvre : **46-1, 46-2, 46-3, 46-4, 46-6** |

**Étanchéité : acquise. Aucun point LEAK.** Le fond de la décision (le mode 2D se courbe par
défaut, avec une option pour le garder plat) est sain, cohérent avec la lecture 1 de Q-079 et avec
la liste close d'exclusions d'`AMD-043` §5. Ce qui manque est de la **précision normative**, pas de
l'étanchéité : l'amendement exige de subdiviser et de courber un objet — « le rectangle du mode
2D » — que la spec ne définit nulle part, et deux de ses tirets (visée, ombre) ne sont aujourd'hui
exerçables par aucun test.

### Contrôle des identifiants et des références

- **`REQ-SCN-074` est libre.** Le plus grand identifiant `REQ-SCN-` attribué est `REQ-SCN-073`
  (`AMD-045`) ; aucune occurrence de `REQ-SCN-074` dans `spec/` hors `AMD-046`, ni dans aucune
  branche ni révision du dépôt `impl` (y compris `delta/v2-1` à `delta/v2-5`). La numérotation est
  donc continue, sans trou ni collision (REQ-GUIDE « un identifiant attribué n'est jamais réutilisé »).
- **Aucune référence cassée.** Les 11 renvois de l'amendement existent et désignent bien ce qu'il
  leur fait dire : `REQ-SCN-055` (SPEC-06 §3.16), `REQ-SCN-066`, `-067`, `-068` (AMD-043 §2 à §3),
  `REQ-INP-063` et `REQ-INP-064` par transitivité, `REQ-INP-124` (AMD-043 §4), `Q-022`, `Q-079`,
  `AMD-035`, `AMD-043 §6` (le groupe « Géométrie » et la ligne « Centre de l'arc » y sont bien), et
  la catégorie **D** de `PLAN.md` (« Divergence acceptée : amélioration, ou comportement hors
  périmètre ») est la bonne.
- **Conforme à `AMD-035`** : libellé et aide donnés en anglais **et** en français, anglais par
  défaut. Le nom employé dans l'énoncé (« Courber le mode 2D ») est **le même** que celui de la
  table de libellés — l'amendement ne reproduit donc pas le décalage de nommage relevé en 43-14 sur
  `AMD-043` (« Bend gauche-droite » dans l'énoncé contre « Courbure gauche-droite » en §6).

### Contrôle d'étanchéité (détail)

Rien à retirer. Pour mémoire, l'amendement ne contient ni code, ni pseudo-code, ni structure de
données, ni nom de classe, de méthode, de champ privé ou de fichier source de l'original. Les trois
seuls faits venus du côté sale sont des faits **observables en boîte noire**, que `SPEC-GUIDE.md`
autorise explicitement (« le comportement observable », « les valeurs par défaut ») :

1. « Dans le programme d'origine, le mode 2D se courbe comme la scène 3D » — résultat visible à
   l'écran, mesurable en lançant une ROM en « 2D Mode » avec un angle non nul ;
2. « Le réglage n'a pas d'accès Lua : le programme d'origine n'en a pas » — absence d'un nom dans
   l'API exposée aux scripts, catégorie de fait d'interopérabilité que le guide autorise ;
3. la clé `scene.bend_2d` et ses libellés sont **propres à Zaxnes** (divergence D) et ne décrivent
   donc rien de l'original.

Le chiffre `32 × 32` n'est pas présenté comme une constante mesurée sur l'original : il est repris
du fond (backdrop) de `REQ-SCN-068`, ce qui est le bon réflexe — une constante annoncée comme
« celle de l'original » aurait, elle, renseigné sur l'intérieur du programme.

---

## 1. Points

### 46-1 (INCONSISTENT, bloquant avant mise en œuvre) — la ligne « Priorités » est circulaire et perd la subdivision

Ligne 4 : « si `REQ-SCN-074` n'est pas retenue, le mode 2D se courbe toujours (**premier tiret de
REQ-SCN-074**) et l'option n'existe pas. » Le repli désigne un tiret de l'exigence qu'on suppose
non retenue : si `REQ-SCN-074` tombe, son premier tiret tombe avec elle, et avec lui la
**subdivision en au moins 32 × 32 cellules**, qui y est enfermée. Le repli produit alors un
rectangle courbé à 4 sommets, c'est-à-dire un quadrilatère gauche sans courbure visible : l'inverse
du comportement que le propriétaire a choisi de conserver. C'est aussi, en pratique, la seule
conséquence mesurable de Q-079 (2 → 2 048 triangles).

*Correction* : énoncer le repli sans se citer, et sortir la subdivision de la clause optionnelle.
Par exemple : « Priorités : le comportement par défaut (le rectangle du mode 2D se courbe comme la
géométrie de la scène de jeu, subdivisé en au moins 32 × 32 cellules — lecture 1 de Q-079) est
acquis indépendamment du réglage ; si le réglage « Courber le mode 2D » n'est pas retenu, seule la
possibilité de le désactiver disparaît. »

### 46-2 (UNCLEAR, bloquant avant mise en œuvre) — « le rectangle du mode 2D » n'est défini par aucune exigence

`REQ-SCN-074` exige de subdiviser, de courber et d'exporter les sommets d'un objet qu'aucun
document normatif ne définit :

- `REQ-SCN-055` (P1, SPEC-06 **gelé** en 1.1) dit encore l'inverse de ce que l'amendement suppose :
  « l'aspect obtenu (**tiles converties avec la géométrie par défaut**) est défini par SPEC-04 » ;
- la seule phrase normative qui installe l'image en pixels est la ligne Q-022 d'`AMD-009` : « le
  mode 2D affiche l'image en pixels du cœur plaquée sur le plan NES **(D)** ; REQ-E3D-381 n'est plus
  utilisée pour ce rendu » — sans dire que cette image est portée par **un** rectangle, ni quelle
  est son étendue, ni dans quel plan ;
- `SPEC-04` §3.17 (REQ-E3D-380, -381) ne décrit rien de ce rendu, et `SPEC-00` n'a pas d'entrée
  de glossaire pour cet objet.

Un tiers qui ne connaîtrait que la spec ne peut donc ni implémenter ni tester « les sommets exportés
du rectangle » : il ne sait pas combien il y a de rectangles, où ils sont, ni si leur étendue est
celle de l'image NES (x ∈ [0, 256], y ∈ [8, 248], `REQ-E3D-410`) ou celle du cadre de référence
(y ∈ [0, 240]). Les deux choix décalent l'image de 8 unités, donc changent `B(P)` (le centre de
courbure `C` est à y = 120) : le critère d'acceptation n'est pas univoque.

*Correction* : (a) ajouter à l'en-tête « Complète **SPEC-06 REQ-SCN-055**, lu avec AMD-009 Q-022
(dont il reprend l'image en pixels, la parenthèse « tiles converties » de REQ-SCN-055 ne s'appliquant
plus) » ; (b) ouvrir le §2 par une phrase de définition, par exemple : « Le **rectangle du mode 2D**
est la surface plane qui porte l'image en pixels du cœur en mode 2D (AMD-009 Q-022) : le rectangle
z = 0, x ∈ [0, 256], y ∈ [8, 248] du repère de REQ-E3D-410, texturé par l'image NES 256 × 240. »

### 46-3 (INCONSISTENT, bloquant avant mise en œuvre) — le tiret « visée du Zapper » restreint `REQ-INP-124` (P2) sans le déclarer, et n'est exerçable par rien

Deux problèmes distincts dans le même tiret.

**(a) Supersession non déclarée.** `REQ-INP-124` (P2, AMD-043 §4) s'énonce « **quand une courbure
est active** » et n'a pas d'exception de mode : avec `bend_zx` = 90° et « Courber le mode 2D »
inactif, elle impose encore la visée par les candidats `B(P)`, alors que l'image affichée est plate.
`REQ-SCN-074` (P3) tranche l'inverse — à juste titre — mais son en-tête n'annonce que « complète
AMD-043 (REQ-SCN-068) et SPEC-09 ». C'est exactement le motif **T-1** du rapport précédent (une
exigence P2 remplacée par une clause de priorité moindre, sans déclaration), et il faut en plus
noter que `REQ-INP-124` **continue** de s'appliquer en mode 2D quand le réglage est actif.

**(b) Clause inexerçable.** En mode 2D, aucun port ne peut être de type `Zapper` : le type de port
vaut `Manette` au démarrage, n'est pas persisté, et n'est changé **que** par un script de profil
(`REQ-INP-060`) ; or le profil du mode 2D est `empty.bytes`, relu à chaque démarrage
(`REQ-PROF-061`), dont le seul script est le script de construction (`name = "Build"`,
source `function BuildPattern()`, critère de SPEC-03 REQ-PROF-024). Le Zapper n'est donc jamais actif (`REQ-INP-061`) dans le mode où le
tiret s'applique, et l'amendement ne donne aucun critère d'acceptation pour lui.

*Correction* : (a) en-tête → « **amende AMD-043 REQ-INP-124** (exception du mode 2D) », et rédiger
le tiret comme une exception explicite : « en mode 2D, quand le réglage est inactif, REQ-INP-124 NE
S'APPLIQUE PAS : la visée est celle de REQ-INP-063, comme à courbure nulle. Quand le réglage est
actif, REQ-INP-124 s'applique sans changement. » (b) Dire par quel chemin la clause est exerçable —
« en mode 2D, un port de type `Zapper` ne peut être obtenu que par l'hôte ou le harnais, le profil
« 2D Mode » n'ayant pas de script de ports (REQ-INP-060, REQ-PROF-061) » — et ajouter le critère
correspondant : « harnais, mode 2D, port 2 forcé en `Zapper`, `ZapperOffset` nul, Courbure
gauche-droite 90°, réglage inactif : pointeur sur le point de scène (128, 120, 0) → position
transmise (128, 128) ±1 (vecteur §5.3 de SPEC-08) ; réglage actif : résultat de REQ-INP-124 à 0,5
unité près. » À défaut, déplacer le tiret en **Note** non normative.

### 46-4 (UNTESTABLE, bloquant avant mise en œuvre) — « sommets identiques à une courbure nulle » : identiques à quel maillage ?

Deuxième tiret et second membre du premier critère : réglage inactif ⇒ « sommets et ombre
**identiques à une courbure nulle** ». La subdivision n'est imposée que par le premier tiret, donc
seulement quand le réglage est **actif**. « Courbure nulle » laisse donc deux maillages de
référence possibles — le rectangle non subdivisé (4 sommets) ou la grille 33 × 33 — et deux
implémentations opposées passent ou échouent selon celui que le testeur choisit. Une comparaison
ensembliste de sommets échoue d'un côté, réussit de l'autre.

*Correction* : comparer des **positions**, pas des maillages : « quand le réglage est inactif, la
position de chaque sommet du rectangle DOIT être sa position plate (B = identité), quels que soient
les réglages de REQ-SCN-066 ; la subdivision est alors libre. » Critère correspondant : « réglage
inactif, Courbure gauche-droite 90° : tous les sommets exportés sont dans le plan z = 0 et leurs
(x, y) sont ceux du rectangle plat, à 10⁻³ unité près ».

### 46-5 (UNTESTABLE) — le premier critère n'a ni tolérance, ni antécédent, ni notion de sommet distinct

« les sommets exportés du rectangle sont **à B(P) près** et en comptent **au moins 33 × 33** » :

- « à B(P) près » ne fixe pas de tolérance, alors que `REQ-SCN-067` en fixe une pour ses propres
  vecteurs (±0,05 unité) et que l'amendement dépend d'un calcul en virgule flottante ;
- l'antécédent `P` n'est pas nommé : il faut dire que `P` est la position **plate** du sommet
  considéré ;
- « en comptent au moins 33 × 33 » compte des sommets par un produit : un exportateur qui duplique
  les sommets par triangle en livre 6 144 pour 32 × 32 cellules et satisfait le critère sans aucune
  subdivision régulière.

*Correction* : « chaque sommet exporté, de position plate P, est rendu à moins de **0,05 unité** de
B(P) ; le rectangle compte au moins **1 089 sommets distincts** (grille d'au moins 33 × 33), soit au
moins 2 048 triangles. »

### 46-6 (UNTESTABLE, bloquant avant mise en œuvre) — l'« ombre » du rectangle du mode 2D n'est spécifiée nulle part

Le premier tiret impose la courbure « passe d'ombres comprise » et le second exige une « ombre
identique à une courbure nulle ». Mais la spec ne dit pas si le rectangle du mode 2D **participe**
à la passe d'ombres : `REQ-SCN-030` conditionne l'ombre effective au réglage `Shadow` **et** au
drapeau du motif **et** au drapeau du calque (`layer[i].cast_shadow` / `recv_shadow`), or en mode 2D
il n'y a ni motif ni calque pour cette surface (profil sans motif, `REQ-SCN-055`). Si le rectangle
ne projette ni ne reçoit d'ombre, les deux clauses sont vides et le critère ne peut rien mesurer ;
s'il en projette une, il faut dire sous quels drapeaux.

*Correction* : trancher en une phrase, par exemple « le rectangle du mode 2D ne projette pas
d'ombre et n'en reçoit pas ; la clause « passe d'ombres comprise » du premier tiret porte sur les
autres objets éventuellement présents » — et retirer alors « et ombre » du second tiret et du
critère ; ou, si l'intention est l'inverse, « le rectangle projette et reçoit l'ombre comme une
surface du calque L1 », avec un critère d'ombre observable (silhouette d'un objet de test sur le
rectangle courbé, puis plat).

### 46-7 (UNCLEAR) — la clause « le fond (backdrop) et le reste de la scène » est inopérante en mode 2D

Second tiret : « Le fond (backdrop) et le reste de la scène suivent toujours REQ-SCN-068. » En mode
2D, aucun des objets visés n'existe : le backdrop est « un pavé optionnel **piloté uniquement par
Lua** » (`REQ-SCN-048`) et le profil du mode 2D n'a pas de script de frame ; il n'y a aucun motif,
donc aucun HUD ; la skybox et le boîtier TV sont de toute façon exclus de la courbure par
`REQ-SCN-068`. La phrase ne rend donc rien exigible, et son mot « fond » est ambigu : `SPEC-06`
distingue la **couleur de fond** (`REQ-SCN-046`), le **backdrop** (`REQ-SCN-048`) et le **fond
converti** (tiles de fond), et `REQ-SCN-068` emploie « fond (backdrop) » pour le pavé.

*Correction* : « si un backdrop est présent (REQ-SCN-048, piloté par Lua), il reste soumis à
REQ-SCN-068, que le réglage soit actif ou non » ; supprimer « et le reste de la scène », ou le
remplacer par l'énumération de ce qui est réellement concerné.

### 46-8 (UNCLEAR, priorité) — P3 pour un comportement par défaut visible sur tous les jeux sans profil

`REQ-SCN-074` est cotée **P3** (confort) en entier. Or son premier tiret ne décrit pas une
commodité : il tranche l'aspect du mode 2D, qui est l'entrée par défaut dès qu'aucun profil ne
correspond (Q-022 est coté P2 pour cette raison même), et il complète `REQ-SCN-068`, cotée **P2**.
Selon la grille du `SPEC-GUIDE` (« P2 importante : fonction visible »), le défaut et la subdivision
relèvent de P2 ; seul l'interrupteur relève de P3.

*Correction* : coter l'exigence **P2** et préciser dans la ligne « Priorités » que seule la
possibilité de désactiver est P3 ; ou scinder en deux exigences (`REQ-SCN-074` P2 pour le défaut et
la subdivision, `REQ-SCN-075` P3 pour le réglage) — la seconde forme est plus lourde et moins
souhaitable.

### 46-9 (UNCLEAR, à trancher maintenant) — la clé `scene.bend_2d` se lit comme un troisième axe de courbure

`AMD-043` §6 a publié `scene.bend_lr` et `scene.bend_tb` (gauche-droite, haut-bas). Dans cette
série, `scene.bend_2d` se lit spontanément comme « courbure selon un axe 2d », et non comme « faut-il
courber le mode 2D ». Le point mérite d'être tranché **avant publication** parce que `REQ-PERS-080`
interdit de renommer une clé publiée (« une clé publiée NE DOIT JAMAIS être renommée ») : l'erreur
serait définitive. Le format de la clé est par ailleurs valide au regard de l'expression de
`REQ-PERS-080` et n'entre en collision avec aucune clé existante.

*Correction* : préférer `scene.bend_2d_mode` (ou `scene.bend_picture`), qui ne peut pas se lire
comme un axe.

### 46-10 (INCONSISTENT, ligne SPEC-09) — trois écarts à la légende normative du §4.1 et à la table §4.6

1. **Colonne « libretro »** : l'amendement écrit « option, 2 valeurs ». La légende du §4.1 définit
   cette colonne comme la **discrétisation** des core options, et la convention des booléens y est
   `on/off` (`render.shadow`, `emu.auto_tv_mode`), avec les valeurs `enabled` / `disabled` dans la
   table §4.6 ; `REQ-PERS-081` et `REQ-PERS-083` exigent des **libellés** d'énumération, pas un
   nombre de valeurs.
2. **Colonne « UI »** : la légende la définit comme « présence dans le panneau de réglages desktop
   **de l'original** (libellé observé) ». Le réglage n'existant pas dans l'original (divergence D),
   y écrire « case « Bend 2D Mode » » laisse croire à un libellé observé.
3. **Colonne « Comportement »** : les tables du §4.1 en comportent onze ; l'amendement en donne dix
   (et nomme « Bornes » la colonne « Domaine »). La forme est celle d'`AMD-043` §6, que la relecture
   précédente a acceptée (point 43-13 « levé ») — ce n'est donc pas un blocage, mais la colonne
   « Comportement » est l'endroit naturel pour « sans effet en mode 3D ».

*Correction* : libretro → `on/off` ; ajouter une ligne à la table §4.6 (`3dsen_scene_bend_2d_mode`,
valeurs `enabled, disabled`, défaut `enabled`, catégorie `Scène`) ; UI → « — dans l'original
(réglage propre à Zaxnes) ; dans Zaxnes : case « Bend 2D Mode » / « Courber le mode 2D », groupe
« Géométrie » (AMD-043 §6), après « Centre de l'arc » » ; ajouter la colonne « Comportement »
(« courbe le rectangle du mode 2D, REQ-SCN-074 ; sans effet en mode 3D »).

### 46-11 (UNCLEAR) — la clause d'ordre de `REQ-SCN-068` (N-43-1) appliquée au rectangle du mode 2D

Le premier tiret dit que la courbure s'applique « comme à la géométrie de la scène de jeu
(REQ-SCN-068) ». Lue strictement, cette formule importe aussi la clause d'ordre qu'`AMD-043` v1.2 a
ajoutée pour clore **N-43-1** : la découpe par le rectangle visible (`REQ-SCN-036`), le point visé
`P*` (`REQ-SCN-008`) et le segment d'occultation FPV (`REQ-SCN-022`) sont évalués **avant**
courbure. C'est le point le plus visible du mode 2D : si la découpe était appliquée après courbure,
l'ensemble des **lignes NES visibles** changerait en basculant le réglage, alors que les marges de
clipping sont inchangées. Mieux vaut l'écrire que de compter sur l'implicite.

*Correction* : ajouter au premier tiret « — y compris la clause d'ordre de REQ-SCN-068 : la découpe
par le rectangle visible est évaluée sur les positions avant courbure, de sorte que l'ensemble des
lignes et colonnes NES visibles ne dépend pas du réglage » ; et au premier critère « les lignes NES
visibles sont les mêmes avec le réglage actif et inactif, pour les marges par défaut ».

### 46-12 (rédactionnel) — trois détails de forme

- **Marqueur de divergence** : le §1 annonce une divergence « catégorie D du plan », mais l'énoncé
  de `REQ-SCN-074` ne porte pas le marqueur **(D)** que la spec utilise dans le corps des exigences
  (`REQ-INP-063`, `REQ-INP-060`, Q-022). Les tables de traçabilité de l'index s'appuient sur ce
  marqueur. *Correction* : « **REQ-SCN-074** (P2) — **Courbure du mode 2D** (réglage propre à
  Zaxnes, divergence acceptée **(D)**) ».
- **Table de libellés** : `AMD-043` §6 et `AMD-041` §3 emploient trois colonnes
  (Élément / Anglais / Français) ; `AMD-046` en emploie deux, clé et libellé fondus dans la même
  cellule. *Correction* : aligner sur la forme à trois colonnes.
- **Critère `[ROM]`** : les deux critères sont automatiques, ce qui est bien (aucune étiquette
  `[ROM]` abusive, contrairement au reproche T-2 du rapport précédent). Un critère `[ROM]`
  **facultatif** serait néanmoins utile pour la perception (« 2D Mode » sur une ROM quelconque,
  courbure 90° : l'image est courbée d'un seul tenant, sans facette visible ni fente), car
  `32 × 32` ne garantit l'absence de facettes que par convention.
- **En-tête** : l'absence de ligne « Historique » est normale en v1.0 ; à ajouter en v1.1 comme
  l'ont fait `AMD-039` à `AMD-045`.

### 46-13 (transverse, continuité de T-4) — l'amendement n'existe pas dans l'arbre miroir du côté propre

`impl/docs/spec/` s'arrête à `AMD-038` : ni `AMD-039` à `AMD-046` ni `AMD-043` ne s'y trouvent,
alors que `Q-079` y est bien présent sur `delta/v2-4-bend`. Le côté propre qui mettra en œuvre
`REQ-SCN-074` et refermera `// Q-079` (`Zax.Render/Gl/NesPictureTexture.cs`,
`Zax.Render/SceneRenderer.cs`) lit cet arbre. Le point n'est pas imputable à `AMD-046` — c'est le
reliquat **T-4** du rapport précédent — mais il le concerne directement.

*Correction* : copier `AMD-039` à `AMD-046` (et ce rapport) dans `impl/docs/spec/changes/` lors du
prochain cycle de synchronisation, et mentionner `AMD-043`/`AMD-046` dans `SPEC-v1.2-INDEX.md`
(tables « mode 2D » et « divergences »), qui ne connaît encore pour le mode 2D que
« AMD-009 Q-022 (D) ».

---

## 2. Ce qui est juste et qu'il faut garder tel quel

Pour que la v1.1 ne défasse rien :

- **Le fond de la décision** est cohérent avec la lecture 1 de Q-079 et avec le raisonnement
  d'`AMD-043` §5 : la première phrase de `REQ-SCN-068` porte l'intention (« la géométrie de la scène
  de jeu ») et la liste des exclusions est close ; laisser le rectangle plat **par défaut** aurait
  ajouté une quatrième exclusion non déclarée. Le défaut « se courbe » est le bon.
- **La subdivision alignée sur le fond** (32 × 32) évite d'inventer une seconde convention.
- **« en mode 3D, le réglage n'a aucun effet »** est bien énoncé, et le troisième membre du premier
  critère (« basculer le réglage ne change aucun sommet exporté ») le rend vérifiable. Il serait
  utile d'ajouter, en note, que cela ne rouvre pas un chemin de rendu séparé au sens de
  `REQ-SCN-055` : seule la subdivision d'une surface change, le pipeline reste unique.
- **L'absence d'accès Lua** est justifiée et cohérente : `REQ-SCN-066` n'expose en
  `Setting:Get/SetVisualStyleParameter` que les quatre paramètres qui existent dans l'original
  (`spl.*`), et un réglage propre à Zaxnes n'a pas à s'y greffer.
- **Les deux critères de persistance** (« la valeur survit à un redémarrage », « elle vaut « actif »
  sur une installation neuve ») sont exactement ce que `REQ-PERS-001` et `REQ-PERS-002` rendent
  testable. À garder mot pour mot.
- **La fermeture de Q-079** est explicite, avec les deux lectures nommées : le §4 est conforme à
  l'usage des amendements précédents.

---

## 3. Conclusion — gate R2

**Oui : `AMD-046` est publiable au côté propre.**

- **Étanchéité : acquise.** Aucun point **LEAK** : pas de code, pas de pseudo-code, aucun nom
  interne de l'original, aucun renvoi à sa structure ou à ses sources. Les trois faits venus du
  côté sale (le mode 2D se courbe, l'absence d'accès Lua, le défaut « actif ») sont observables en
  boîte noire et autorisés par `SPEC-GUIDE.md`. Rien dans ce rapport ne justifie de retenir la
  publication.
- **Verdict : accepté avec réserves**, 13 points, dont **aucun bloquant pour l'étanchéité** et
  **cinq à traiter avant la mise en œuvre**, parce qu'en l'état un implémenteur devrait deviner :
  **46-2** (l'objet « rectangle du mode 2D » n'est défini par aucune exigence, et `REQ-SCN-055`
  gelée dit encore l'inverse), **46-3** (restriction non déclarée de `REQ-INP-124`, et clause de
  visée qu'aucun port `Zapper` ne peut exercer en mode 2D), **46-6** (participation du rectangle à
  la passe d'ombres non spécifiée), **46-4** (maillage de référence de « identiques à une courbure
  nulle ») et **46-1** (ligne « Priorités » circulaire, dont le repli perd la subdivision).
- **Avant le gel de la spec**, par ordre d'importance : 46-2, 46-3, 46-6, 46-4, 46-1, puis **46-9**
  (la clé est définitive au sens de `REQ-PERS-080` : à trancher maintenant), **46-5** et **46-11**
  (tolérance, sommets distincts, clause d'ordre — ils rendent les critères incontestables),
  **46-8** (priorité P2), **46-10** (ligne `SPEC-09` et table §4.6). **46-7**, **46-12** et
  **46-13** sont rédactionnels ou transverses.
- **Aucune exigence gelée n'a besoin d'être rouverte au-delà de ce que les corrections demandent de
  déclarer** : `REQ-SCN-055` (sa parenthèse « tiles converties », déjà périmée depuis
  `AMD-009` Q-022) et `REQ-INP-124` (exception du mode 2D plat). Ces deux déclarations sont de
  forme, pas de fond : elles n'inversent aucun comportement choisi par le propriétaire.

Le fichier `AMD-046-courbure-mode-2d.md` n'a pas été modifié par cette relecture ; rien n'a été
commité.
