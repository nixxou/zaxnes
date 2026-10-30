# R2 — Relecture d'`AMD-046` v1.1 (courbure en mode 2D)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-30 | Version du rapport : 1.1
Objet : vérifier, point par point, que la **v1.1** d'`AMD-046-courbure-mode-2d.md` lève les points
`46-1` à `46-12` de `R2-review-AMD-046.md` (le point `46-13` est transverse : il porte sur la copie
des amendements dans `impl/docs/spec/`, hors de ce fichier), puis relire la v1.1 **en entier** pour
y chercher toute fuite, incohérence, critère intestable ou référence `REQ` cassée **introduite par
les corrections**.

Entrées relues : `spec/changes/AMD-046-courbure-mode-2d.md` (**v1.1 du 2026-10-30**, texte sous
revue, **non modifié**) ; `R2-review-AMD-046.md` (rapport v1.0, 46-1 à 46-13) ;
`AMD-043-courbure-espace.md` (v1.2 : REQ-SCN-066, -067, -068, REQ-INP-124, §6) ;
`AMD-035-langue-interface.md` ; `AMD-009-q-tickets-l7a.md` et `changes/Q-022.md` (réponse R0
normative) ; `changes/Q-079.md` ; `R2-review-AMD-039-045.md` et sa v1.1 (continuité de 43-13,
N-43-1, T-1, T-2, T-4) ; `SPEC-GUIDE.md` ; `SPEC-00` (glossaire, entrée « Scène ») ; `SPEC-03`
(REQ-PROF-024 et son critère sur `empty.bytes`, REQ-PROF-061, §5.4) ; `SPEC-04` §3.2 (Hs = 248) ;
**`SPEC-04B` REQ-E3D-410 (points 1 à 5) et REQ-E3D-643** ; `SPEC-06` (REQ-SCN-001, -029, -030,
-036, -044, -046, -048, -055, §2, §5.3) ; **`SPEC-08` §3.7 (REQ-INP-060 à 066, REQ-INP-123) et
§5.3** ; `SPEC-09` §4.1 (légende et les onze colonnes), §4.6, REQ-PERS-001/002/080/081/083/084 ;
`SPEC-12` §2 (catégories **auto / [ROM] / [R2] / revue**) ; `SPEC-v1.2-INDEX.md` ; `PLAN.md` §3.
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement ;
les outils MCP de rétro-ingénierie (`rea`) **n'ont pas été utilisés**.

Méthode : (1) relecture de la v1.0 et de la v1.1 côte à côte, point de la relecture par point de la
relecture, avec pour chacun la question « la correction fait-elle ce que le point demandait, et
rien de plus ? » ; (2) relecture de la v1.1 seule, comme un tiers qui ne connaîtrait que la spec, en
cherchant pour chaque tiret quel test automatique il rend exigible ; (3) **recalcul indépendant**
des deux grandeurs que la v1.1 introduit : la définition du rectangle confrontée aux cinq points de
`REQ-E3D-410`, et le vecteur Zapper confronté à la table `SPEC-08` §5.3 et à la formule de
`REQ-SCN-067` ; (4) contrôle de l'unicité de `REQ-SCN-074` et de l'existence de chacun des renvois ;
(5) confrontation de la ligne `SPEC-09` à la légende normative du §4.1 et de la ligne libretro à la
table §4.6 et à `REQ-PERS-083` ; (6) contrôle d'étanchéité sur les **phrases nouvelles** de la v1.1.

Catégories : **LEAK**, **INCONSISTENT**, **UNTESTABLE**, **UNCLEAR**, plus les points
**rédactionnels** et **transverses**, comme dans les rapports précédents.

---

## 0. Verdict

| Document | Verdict | Points 1.0 levés | Nouveaux points | Réserves bloquantes |
|---|---|---|---|---|
| **AMD-046** — courbure en mode 2D (v1.1) | **Accepté — prêt pour la mise en œuvre** | **12 sur 12** (46-1 à 46-12) ; 46-13 transverse, inchangé | 5 (N-46-1 à N-46-5), **tous mineurs** | **aucune** |

**Étanchéité : acquise, inchangée. Aucun point LEAK**, ni dans le texte conservé, ni dans les
phrases ajoutées par la v1.1. Les cinq réserves « avant mise en œuvre » du rapport v1.0 (46-1,
46-2, 46-3, 46-4, 46-6) sont **toutes levées**, et les sept autres points du fichier le sont aussi.
Les corrections n'ont rien cassé : les deux grandeurs nouvelles (le rectangle, le vecteur Zapper)
sont **exactes** au regard de `REQ-E3D-410` et de `SPEC-08` §5.3, vérifiées par recalcul. Les cinq
nouveaux points sont de forme ou de couverture de test : ils rendraient les critères plus serrés,
aucun ne laisse un implémenteur deviner.

### Contrôle des identifiants et des références

- **`REQ-SCN-074` reste le seul identifiant attribué par l'amendement et reste libre ailleurs** :
  aucune autre occurrence dans `spec/` ; le plus grand `REQ-SCN-` attribué par ailleurs est
  `REQ-SCN-073` (`AMD-045`). La v1.1 n'a **pas** scindé l'exigence en 074 + 075 (la variante que le
  rapport v1.0 jugeait « plus lourde et moins souhaitable ») : la numérotation reste continue.
- **Aucune référence cassée.** Les quinze renvois de la v1.1 existent et disent bien ce que
  l'amendement leur fait dire : `REQ-E3D-410` (SPEC-04B §3.1, points 2 et 3), `REQ-SCN-036`,
  `-048`, `-055`, `-066`, `-067`, `-068`, `REQ-INP-060`, `-063`, `-124`, `REQ-PROF-061`, `Q-022`,
  `Q-079`, `AMD-035`, `AMD-043 §6`. La clé `scene.bend_2d_mode` respecte la forme imposée par
  `REQ-PERS-080` (`[a-z0-9_]+(\.[a-z0-9_\[\]]+)*`) et n'entre en collision avec aucune clé publiée ;
  la core option `3dsen_scene_bend_2d_mode` est exactement celle que `REQ-PERS-083` dérive de cette
  clé.
- **Le motif T-1 est éteint.** En v1.0, une exigence **P3** restreignait `REQ-INP-124` (**P2**) sans
  le déclarer. En v1.1, `REQ-SCN-074` est **P2** (46-8) et l'en-tête porte « **amende AMD-043
  REQ-INP-124** (exception du mode 2D plat) » : même priorité, supersession déclarée. La ligne
  « Priorités » d'`AMD-043` (« si une exigence P2 qui remplace une clause P1 n'est pas retenue, la
  clause d'origine reste valable ») n'a plus à être invoquée ici.
- **Conforme à `AMD-035`** : libellé et aide en anglais **et** en français, anglais par défaut, dans
  la forme à trois colonnes (Élément / Anglais / Français) d'`AMD-043` §6 et d'`AMD-041` §3. Le nom
  employé dans l'énoncé, dans la ligne `SPEC-09` et dans la table de libellés est le **même**
  (« Courber le mode 2D » / `Bend 2D Mode`) : pas de décalage de nommage du type 43-14.

### Contrôle d'étanchéité des phrases nouvelles

Rien à retirer. Les cinq ajouts de la v1.1 qui pourraient en principe renseigner sur l'intérieur du
programme d'origine sont tous dérivés de la spec elle-même ou observables en boîte noire :

1. « le rectangle z = 0, x ∈ [0, 256], y ∈ [8, 248] du repère de REQ-E3D-410, texturé par l'image
   NES 256 × 240 » — **déduction** de `REQ-E3D-410` point 2 et de la réponse R0 de `Q-022`, pas une
   mesure sur l'original (voir §2.1) ;
2. « Il ne projette pas d'ombre et n'en reçoit pas » — fait observable (silhouette présente ou
   absente), du même ordre que `REQ-E3D-409` pour le calque `UI` ;
3. « Le profil « 2D Mode » n'ayant pas de script de ports (REQ-INP-060, REQ-PROF-061) » — fait
   d'**interopérabilité** sur une donnée conservée (`profiles/empty.bytes` contient un seul script,
   `name = "Build"`, `SPEC-03` REQ-PROF-024), catégorie que `SPEC-GUIDE.md` autorise explicitement ;
4. la clause d'ordre reprise de `REQ-SCN-068` — citation d'un texte déjà publié côté propre ;
5. la clé, les libellés et la ligne libretro — **propres à Zaxnes** (divergence **D**), et la colonne
   UI dit maintenant « — dans l'original », ce qui lève justement le risque de laisser croire à un
   libellé observé.

Aucun code, aucun pseudo-code, aucun nom de classe, de méthode, de champ privé ou de fichier source
de l'original ; aucune structure de données ; aucune constante présentée comme « celle de
l'original » au-delà de ce qui est déjà publié (`32 × 32` reste repris du fond de `REQ-SCN-068`).

---

## 1. Point 1.0 → levé / partiellement / non levé

| Point 1.0 | État | Ce que fait la v1.1 |
|---|---|---|
| **46-1** (INCONSISTENT, bloquant) ligne « Priorités » circulaire, dont le repli perd la subdivision | **levé** | La ligne 5 ne se cite plus : « le comportement par défaut (le rectangle du mode 2D se courbe comme la géométrie de la scène de jeu, subdivisé en au moins 32 × 32 cellules, lecture 1 de Q-079) est **P2 et acquis indépendamment du réglage** ; si le réglage « Courber le mode 2D » n'est pas retenu, seule la possibilité de le désactiver (P3) disparaît ». Le repli est désormais cohérent : sans le réglage, il reste la courbure **et** la subdivision ; et la subdivision est effectivement sortie de la clause optionnelle, le premier tiret l'imposant « en mode 2D » sans condition |
| **46-2** (UNCLEAR, bloquant) « le rectangle du mode 2D » n'est défini par aucune exigence | **levé** | Les **deux** moitiés de la correction sont faites : (a) l'en-tête déclare « **SPEC-06 REQ-SCN-055 lu avec AMD-009 Q-022** (dont il reprend l'image en pixels : la parenthèse « tiles converties » de REQ-SCN-055 ne s'applique plus) » — et l'attribue à `Q-022`, qui est bien l'autorité qui a périmé cette parenthèse, non à `AMD-046` ; (b) le §2 ouvre sur une phrase de définition **géométriquement univoque**, vérifiée au §2.1 ci-dessous. L'ambiguïté « image NES (y ∈ [8, 248]) ou cadre de référence (y ∈ [0, 240]) », qui décalait `B(P)` de 8 unités, est tranchée |
| **46-3** (INCONSISTENT, bloquant) restriction non déclarée de `REQ-INP-124`, et clause inexerçable | **levé** | (a) En-tête : « **amende AMD-043 REQ-INP-124** (exception du mode 2D plat) », et le tiret est écrit comme une exception explicite et bilatérale : « quand le réglage est inactif, REQ-INP-124 **NE S'APPLIQUE PAS** et la visée est celle de REQ-INP-063, comme à courbure nulle ; quand le réglage est actif, REQ-INP-124 s'applique sans changement ». (b) Le chemin d'exerçabilité est dit (« un port de type Zapper n'y est obtenu que par l'hôte ou le harnais ») **et** le critère correspondant est ajouté, catégorie *auto* — ce qui vaut mieux que le repli en Note non normative. Le procédé est déjà celui du critère de `REQ-INP-062` (« en harnais, port 2 en `Zapper` »), donc sans précédent à créer |
| **46-4** (UNTESTABLE, bloquant) « identiques à une courbure nulle » : identiques à quel maillage ? | **levé** | La comparaison porte maintenant sur des **positions**, pas sur des maillages : « la position de chaque sommet du rectangle DOIT être sa position plate (B = identité), quels que soient les réglages de REQ-SCN-066 », et le critère dit « tous les sommets exportés sont dans le plan z = 0, à leur position plate à 10⁻³ unité près ». Le maillage de référence n'intervient plus. La v1.1 va même un cran plus loin que la correction proposée (qui laissait la subdivision libre quand le réglage est inactif) en imposant la subdivision dans les deux cas — choix cohérent avec la ligne « Priorités », au prix de la lacune de couverture **N-46-3** |
| **46-5** (UNTESTABLE) ni tolérance, ni antécédent, ni sommet distinct | **levé** | « chaque sommet exporté du rectangle, **de position plate P**, est à moins de **0,05 unité** de B(P) ; le rectangle compte au moins **1 089 sommets distincts** (grille d'au moins 33 × 33) ». Les trois manques sont comblés, la tolérance est celle de `REQ-SCN-067`, et l'arithmétique est juste (32 × 32 cellules ⇒ 33 × 33 = 1 089). Le « soit au moins 2 048 triangles » suggéré n'a pas été repris : sans conséquence, le compte de sommets distincts suffit à interdire la duplication par triangle |
| **46-6** (UNTESTABLE, bloquant) l'ombre du rectangle n'est spécifiée nulle part | **levé** | La première branche de l'alternative est retenue, en une phrase : « Il ne projette pas d'ombre et n'en reçoit pas. » Corollairement, « passe d'ombres comprise » a disparu du premier tiret et « et ombre » du second tiret comme du critère : plus aucune clause vide, plus rien à mesurer qui ne le soit pas. L'héritage de `REQ-SCN-068` (« elle DOIT s'appliquer aussi à la passe d'ombres ») reste importé par le premier tiret mais devient **sans objet** pour cette surface, sans contradiction. Réserve de forme seulement : **N-46-1** |
| **46-7** (UNCLEAR) « le fond (backdrop) et le reste de la scène » inopérant | **levé** | Remplacé mot pour mot par la rédaction proposée : « si un backdrop est présent (REQ-SCN-048, piloté par Lua), il reste soumis à REQ-SCN-068, que le réglage soit actif ou non ». « Et le reste de la scène » a disparu, et l'ambiguïté du mot « fond » avec `REQ-SCN-046` (couleur de fond) et le fond converti n'existe plus |
| **46-8** (UNCLEAR, priorité) P3 pour un comportement par défaut | **levé** | `REQ-SCN-074` est cotée **(P2)**, et la ligne « Priorités » précise que seule la possibilité de désactiver est **P3** — la première des deux formes proposées, la plus légère. Cohérent avec `REQ-SCN-068` (P2) qu'elle complète et avec `Q-022` (P2) |
| **46-9** (UNCLEAR, à trancher maintenant) `scene.bend_2d` se lit comme un troisième axe | **levé** | Clé renommée **`scene.bend_2d_mode`** partout où elle apparaît (ligne `SPEC-09` et, par dérivation `REQ-PERS-083`, core option `3dsen_scene_bend_2d_mode`). Plus aucune occurrence de `scene.bend_2d` : la contrainte « une clé publiée NE DOIT JAMAIS être renommée » (`REQ-PERS-080`) est respectée avant publication, ce qui était tout l'enjeu du point |
| **46-10** (INCONSISTENT) trois écarts à la légende du §4.1 et à la table §4.6 | **levé**, les trois | 1. libretro → **`on/off`**, convention des booléens du §4.1 (`render.shadow`) ; 2. UI → « — dans l'original (réglage propre à Zaxnes) ; dans Zaxnes : case « Bend 2D Mode » / « Courber le mode 2D », groupe « Géométrie » (AMD-043 §6), après « Centre de l'arc » », ce qui respecte la définition de la colonne (« libellé **observé** ») ; 3. la colonne **« Comportement »** est présente et la colonne s'appelle bien **« Domaine »** : la table compte les **onze** colonnes du §4.1. La ligne §4.6 est ajoutée (`3dsen_scene_bend_2d_mode`, `enabled, disabled`, défaut `enabled`, catégorie `Scène`), dans la forme exacte de `3dsen_render_shadow` |
| **46-11** (UNCLEAR) clause d'ordre de `REQ-SCN-068` (N-43-1) | **levé** | Premier tiret : « y compris la clause d'ordre de REQ-SCN-068 : la découpe par le rectangle visible (REQ-SCN-036) est évaluée sur les positions avant courbure, de sorte que l'ensemble des lignes et colonnes NES visibles ne dépend pas du réglage » ; et le critère reprend « Les lignes NES visibles sont les mêmes dans les deux cas, pour les marges par défaut ». Le point le plus visible du mode 2D est donc écrit, non laissé à l'implicite. Réserve mineure sur le renvoi lui-même : **N-46-4** |
| **46-12** (rédactionnel) quatre détails de forme | **levé**, les quatre | Marqueur **(D)** dans l'énoncé (« réglage propre à Zaxnes, divergence acceptée **(D)** ») ; table de libellés à **trois** colonnes ; critère de perception **facultatif** ajouté (« l'image est courbée d'un seul tenant, sans facette visible ni fente ») ; ligne **Historique** présente (« 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-046 : 46-1 à 46-13) »). Réserve sur la **catégorie** de ce critère facultatif : **N-46-5** |
| **46-13** (transverse, continuité de T-4) l'amendement n'existe pas dans `impl/docs/spec/` | **hors du fichier — inchangé** | Rien à corriger ici, par construction : le point porte sur l'arbre miroir du côté propre, pas sur l'amendement. État au 2026-10-30 : `impl/docs/spec/` s'arrête toujours à `AMD-038`, et `SPEC-v1.2-INDEX.md` ne connaît toujours pour le mode 2D que « AMD-009 Q-022 (D) » (ligne « mode 2D ») et ne mentionne ni `AMD-043` ni `AMD-046`. À traiter au prochain cycle de synchronisation, avec `AMD-039` à `AMD-046` et les deux rapports R2 |

*Note de forme* : la ligne Historique écrit « 46-1 à 46-13 » alors que 46-13 ne peut pas être levé
par ce fichier. C'est la convention des amendements précédents (`AMD-043` cite « 43-1 à 43-16 »,
qui comprenaient eux aussi des points transverses) : on lit « en réponse au rapport », pas « tous
clos ici ». Sans conséquence.

---

## 2. Les deux contrôles ciblés

### 2.1 La définition du rectangle contre `REQ-E3D-410` — **exacte**

La v1.1 écrit : « le rectangle z = 0, x ∈ [0, 256], y ∈ [8, 248] du repère de REQ-E3D-410, texturé
par l'image NES 256 × 240 ».

- `REQ-E3D-410` **point 2** : « L'image visible de 256 × 240 pixels (`py` de 0 à 239) occupe donc
  `x ∈ [0, 256]`, `y ∈ [8, 248]`, `z = 0` ; son centre géométrique est `(128, 128, 0)`. » Les trois
  bornes et le plan de la v1.1 sont **mot pour mot** ceux du point 2. Rien n'est inventé.
- Le choix est le bon des deux : `Q-022` (réponse R0 normative) dit « l'image en pixels 256×240
  produite par le cœur […] **plaquée sur le plan NES selon le repère de REQ-E3D-410** ». Le « plan
  NES » du point 2, et non le **cadre de référence** du point 3 (`y ∈ [0, 240]`, centre
  `C = (128, 120, 0)`), qui sert au rectangle visible, à la cible de caméra, au HUD, au backdrop et
  au boîtier TV. L'amendement ne confond pas les deux : il place le rectangle sur l'image
  (point 2) et laisse `REQ-SCN-067` centrer la courbure sur `C` (point 3). C'est cohérent avec
  `Q-079`, qui décrivait « un quad unique de 256 × 240 unités […] posé sur le plan NES de
  REQ-E3D-410 ».
- Conséquence sur le critère, qui était l'enjeu de 46-2 : `B(P)` est désormais univoque, le décalage
  de 8 unités étant tranché. Vérification de l'ordre de grandeur, par recalcul de `REQ-SCN-067` pour
  Courbure gauche-droite = 90° (k = θ/256 = 6,13592·10⁻³ ; φ = 22,5° ; f = 1,02618 ;
  h = 2 sin²(φ/2)/k = 12,4058) : le centre du cadre `C` est rendu en `(128 ; 120 ; −12,73)`, et le
  milieu du bord supérieur du rectangle, `P = (128, 248, 0)`, en `(128 ; 248 ; −12,73)` — la même
  formule donne `(128 ; 120 ; −8,43)` à 60°, valeur **publiée** dans la table de vecteurs de
  `REQ-SCN-067`, ce qui confirme que la formule a été appliquée correctement. Les tolérances du
  critère (0,05 unité côté actif, 10⁻³ côté inactif) sont du bon ordre devant ces déplacements de
  plusieurs unités : le critère discrimine réellement les deux états.
- Deux omissions **bénignes** dans la définition : le facteur d'échelle de scène `s` n'est pas
  rappelé (`REQ-SCN-002` ; `s` = 1 sur desktop, et `REQ-SCN-067` l'intègre déjà dans `k` et dans le
  pivot), et l'orientation de la texture n'est pas redite (elle découle de `REQ-SCN-001` : le pixel
  `(px, py)` occupe `[px, px+1] × [247 − py, 248 − py]`). Aucune des deux ne rend un critère
  ambigu ; aucune ne justifie une v1.2.

### 2.2 Le vecteur Zapper contre `SPEC-08` §5.3 — **exact**

La v1.1 écrit : « réglage inactif : un pointeur sur le point de scène (128, 120, 0) donne la
position transmise (128, 128) ±1 (vecteur de SPEC-08 §5.3) ».

- `SPEC-08` §5.3, **première ligne** de la table (« port 2 en `Zapper`, caméra par défaut, décalage
  nul, échelle 1 ; relation de REQ-INP-063 selon DEC-08-4, recalculée en v1.1 ») : pointeur sur
  `C = (128, 120)` → position transmise **(128, 128) ±1**, octet du port `0x10`. La citation est
  **exacte**, y compris la tolérance.
- Recalcul indépendant par `REQ-INP-063` : `px = tronc((x − Ox)/e) = 128` et
  `py = tronc(248 − (y − Oy)/e) = tronc(248 − 120) = 128` avec `ZapperOffset` nul et `e` = 1. Même
  résultat que `REQ-E3D-410` point 5 (le point `(x, y)` montre le pixel `(⌊x⌋, ⌊Hs − y⌋)`).
  Cohérent aussi avec le critère de `REQ-INP-063` (« pointeur sur le point de scène `C` → (128,
  128) »). Le vecteur retenu est exactement celui qui exerce la clause : il vise **`C`**, le centre
  de courbure, donc le seul point dont `B(P)` a la même projection `(x, y)` que `P` — c'est-à-dire
  le point où une implémentation qui oublierait l'exception passerait quand même ; mais la position
  transmise reste discriminante, puisque sous courbure active le **pointeur** doit être placé
  ailleurs (voir N-46-2).
- Les conditions du vecteur (caméra par défaut, échelle 1) ne sont pas redites par l'amendement.
  Sans conséquence : `REQ-INP-063` précise que la relation vaut pour « le point de scène situé sous
  le pointeur » quelle que soit la caméra (critère à 30°), et `s` = 1 sur desktop. Le critère est
  donc exerçable tel quel.
- Les conditions nécessaires sont toutes posées : port 2 forcé en `Zapper` (donc Zapper **actif** au
  sens de `REQ-INP-061`), `ZapperOffset` nul, courbure 90°, jeu en cours (mode 2D). Rien ne manque
  pour exécuter le test.

---

## 3. Nouveaux points (tous mineurs, aucun bloquant)

### N-46-1 (rédactionnel / traçabilité) — la règle d'ombre vit hors de toute exigence

« Il ne projette pas d'ombre et n'en reçoit pas » est **normatif** (c'est la décision qui lève
46-6), mais la phrase est placée dans le paragraphe de **définition** du §2, hors de
`REQ-SCN-074` : elle ne porte donc aucun identifiant, n'est couverte par aucun critère
d'acceptation, n'entrera pas dans la matrice de traçabilité, et — contrairement au reste de
`REQ-SCN-074` — n'est pas scopée par le réglage (elle vaut aussi quand il est inactif, ce qui est
sans doute l'intention). Elle ne dit pas non plus si elle reproduit l'original ou si elle est une
divergence **(D)** ; le marqueur (D) de l'énoncé porte sur le réglage, pas sur elle.

*Correction* : en faire un tiret de `REQ-SCN-074` — « le rectangle du mode 2D NE DOIT ni projeter
ni recevoir d'ombre, quel que soit le réglage (comme le calque `UI`, REQ-E3D-409) ; la clause
d'ombres de REQ-SCN-068 porte sur les autres objets éventuellement présents » — et dire en Note si
c'est le comportement de l'original ou un choix (D). Un critère *auto* est facile si on en veut un :
en mode 2D, le nombre d'objets de la passe d'ombres est nul.

### N-46-2 (UNTESTABLE, mineur) — la moitié « réglage actif » du critère Zapper ne nomme ni pointeur ni résultat

Le critère dit : « réglage actif : le résultat est celui de REQ-INP-124 à 0,5 unité près ». Deux
petits défauts : (a) la position du **pointeur** n'est pas donnée pour ce cas — sous courbure
active, « un pointeur sur le point de scène (128, 120, 0) » ne veut plus dire la même chose, puisque
c'est l'**image** `B(128, 120, 0) = (128 ; 120 ; −12,73)` qui se trouve sous le pointeur ; (b) « à
0,5 unité près » se superpose à la tolérance de 0,5 unité que `REQ-INP-124` fixe déjà, ce qui peut se
lire comme 1,0 unité cumulée.

Le résultat attendu est pourtant calculable sans ambiguïté : les candidats de `REQ-INP-124` sont les
`P` du plan `z = 0` dont `B(P)` est sur le rayon, le plus proche est `C`, et `REQ-INP-063` en fait
**(128, 128)** — la même valeur que dans le cas inactif. Le critère est donc dérivable, pas
intestable ; c'est pourquoi le point n'est pas bloquant.

*Correction* : « réglage actif, le pointeur placé sur l'image courbée `B(128, 120, 0)` : la position
transmise est la même, (128, 128) ±1, dans la tolérance de 0,5 unité de REQ-INP-124 ».

### N-46-3 (UNTESTABLE, mineur) — la subdivision inconditionnelle n'est exercée par aucun critère

Le premier tiret impose la subdivision « en mode 2D », sans condition ; mais seul le critère du cas
**actif** compte les sommets (≥ 1 089 distincts). Le critère du cas **inactif** ne vérifie que des
positions : un rectangle à **4 sommets** y satisfait pleinement. Une implémentation qui ne
subdiviserait qu'en courbure active passerait donc tous les critères tout en violant le premier
tiret — exactement le genre d'écart que la ligne « Priorités » (« acquis indépendamment du
réglage ») veut interdire.

*Correction* : ajouter au critère du cas inactif « …et le rectangle compte toujours au moins 1 089
sommets distincts » ; ou, à l'inverse, revenir à la rédaction du rapport v1.0 (« la subdivision est
alors libre ») et ne l'imposer que dans le cas actif. Les deux ferment la lacune ; la première est
plus fidèle à la ligne « Priorités ».

### N-46-4 (UNCLEAR, mineur) — `REQ-SCN-036` est rédigée pour des **tiles**, et le rectangle n'en est pas une

La clause d'ordre ajoutée (46-11) et le critère « les lignes NES visibles sont les mêmes dans les
deux cas » supposent que le rectangle du mode 2D **est découpé** par le rectangle visible `R`. Or
`REQ-SCN-036` s'énonce « tout fragment **de tile** (opaque ou transparente, hors HUD) […] », et en
mode 2D il n'y a pas de tile : l'exigence qui porte réellement le découpage de l'**image** est
`SPEC-04B` **`REQ-E3D-643`** (« le découpage de l'image par les marges réglables […] DOIT être
appliqué par le rendu dans le plan XY de la scène, après transformation, sur la base du rectangle
visible `R` de REQ-E3D-410 ; le moteur 3D ne modifie pas les maillages pour cela »). Le renvoi n'est
donc pas *faux* — `REQ-SCN-068` nomme `REQ-SCN-036`, et `AMD-046` ne fait que la citer — mais il
laisse un tiers se demander si la clause s'applique à un objet qui n'est pas un tile.

Le critère reste **robuste** dans les deux lectures (si rien n'est découpé, l'ensemble des lignes
visibles est le même dans les deux états, et le critère passe), d'où le caractère mineur du point.
Deux conséquences pratiques méritent néanmoins d'être écrites : (a) `REQ-E3D-643` interdit de
découper en modifiant le maillage, donc le compte de 1 089 sommets ne dépend pas des marges ;
(b) comme `REQ-E3D-643` applique la découpe **dans le plan XY de la scène, après transformation**,
c'est bien la clause d'ordre qui garantit que les lignes visibles ne bougent pas.

*Correction* : « la découpe par le rectangle visible (REQ-SCN-036, appliquée à l'image par
REQ-E3D-643) est évaluée sur les positions avant courbure ». Et, pour le critère, nommer
l'observable : capture hors écran avec une image de test dont chaque ligne NES est distincte,
comme le fait le critère de `REQ-E3D-643`.

### N-46-5 (rédactionnel) — le critère facultatif est étiqueté `[ROM]` alors qu'il relève de **revue**

« Critère d'acceptation **[ROM]** (facultatif) : « 2D Mode » sur une ROM quelconque […] l'image est
courbée d'un seul tenant, sans facette visible ni fente ». Deux écarts à `SPEC-12` §2 : (a) ce
critère n'a besoin d'**aucune** ROM commerciale — le mode 2D est l'entrée par défaut d'une ROM sans
profil, et la mesure de `Q-022` a justement été faite en harnais sur la ROM de test publique
`240pee` —, donc il n'entre pas dans la définition de `[ROM]` (« exige la ROM fournie par
l'utilisateur ») ; (b) « sans facette visible ni fente » est un **verdict visuel**, c'est-à-dire la
catégorie **revue** (« verdict par revue visuelle documentée, capture et justification archivées
dans `verify/` »). De plus, `SPEC-12` §2 veut qu'« une exigence porte l'étiquette [ROM] […] dans
son titre quand au moins une partie de son critère en relève » : si l'étiquette était maintenue,
`REQ-SCN-074` devrait porter `[ROM]`, ce qu'elle ne fait pas — et porterait à tort, ce qui était le
reproche **T-2** du rapport précédent.

*Correction* : « Critère d'acceptation (**revue**, facultatif) : en mode 2D sur une ROM de test
publique, Courbure gauche-droite à 90°, réglage actif : l'image est courbée d'un seul tenant, sans
facette visible ni fente ; capture archivée dans `verify/` ». Aucune étiquette de titre n'est alors
nécessaire. (Le rapport v1.0 avait lui-même suggéré la formulation `[ROM]` : c'est cette suggestion
qui était imprécise, la correction de R1 l'a suivie fidèlement.)

### Nits sans conséquence

- La colonne libretro de la ligne `SPEC-09` est écrite « Libretro » (majuscule) là où le §4.1 écrit
  « libretro » ; `AMD-043` §6 fait déjà de même.
- Le critère Zapper ne redit pas « caméra par défaut, échelle 1 », conditions du vecteur §5.3 (voir
  §2.2) ; exerçable sans cela.
- La ligne Historique cite « 46-1 à 46-13 » alors que 46-13 est transverse (voir §1).

---

## 4. Ce qui est juste et qu'il faut garder tel quel

Pour qu'une éventuelle v1.2 ne défasse rien :

- **La définition du rectangle** (§2.1) : elle est exacte, minimale, et entièrement dérivée de
  `REQ-E3D-410` point 2 et de `Q-022`. À ne pas « améliorer » en y mêlant le cadre de référence.
- **La déclaration d'amendement de `REQ-INP-124`** et la formulation **bilatérale** de l'exception
  (inactif → `REQ-INP-063` ; actif → `REQ-INP-124` sans changement) : c'est ce qui éteint le motif
  T-1 et ce qui évite qu'un implémenteur traite le mode 2D comme un cas particulier permanent.
- **La cotation P2 avec la nuance P3 dans la ligne « Priorités »** : plus légère qu'une scission en
  `REQ-SCN-074` / `REQ-SCN-075`, et elle économise un identifiant.
- **La clé `scene.bend_2d_mode`** : définitive au sens de `REQ-PERS-080`, et désormais non
  confondable avec un troisième axe de courbure. Ne plus y toucher après publication.
- **Les deux critères de persistance**, inchangés depuis la v1.0 (« la valeur survit à un
  redémarrage », « elle vaut « actif » sur une installation neuve ») : exactement ce que
  `REQ-PERS-001` et `REQ-PERS-002` rendent testables.
- **La Note « cela ne crée pas de chemin de rendu séparé au sens de REQ-SCN-055 : seule la
  subdivision d'une surface change, le pipeline reste unique »** : elle protège une exigence P1
  gelée contre une lecture erronée de l'amendement. Elle a été ajoutée exactement comme le rapport
  v1.0 le suggérait.
- **La clôture de `Q-079`** au §4, avec les deux lectures nommées, et **la ligne §4.6** de la table
  libretro, qui manquait aux lignes d'`AMD-043` §6.

---

## 5. Conclusion — gate R2

**Oui : `AMD-046` v1.1 est prêt pour la mise en œuvre.**

- **Étanchéité : acquise.** **Aucun point LEAK**, ni conservé ni introduit. Les faits venus du côté
  sale restent les trois mêmes faits observables en boîte noire que la v1.0 (le mode 2D se courbe,
  l'absence d'accès Lua, le défaut « actif »), auxquels la v1.1 n'ajoute que des **déductions de la
  spec** (le rectangle), un fait d'interopérabilité sur une donnée conservée (le profil
  `empty.bytes` n'a qu'un script `BuildPattern`) et des éléments **propres à Zaxnes** (clé,
  libellés, ligne libretro). Rien ne justifie de retenir la publication.
- **Les 12 points du fichier sont levés**, dont les **cinq réserves bloquantes** du rapport v1.0 :
  46-2 (définition du rectangle, vérifiée exacte contre `REQ-E3D-410`), 46-3 (supersession de
  `REQ-INP-124` déclarée et clause rendue exerçable, avec un critère *auto* dont le vecteur est
  vérifié exact contre `SPEC-08` §5.3), 46-6 (ombre tranchée), 46-4 (positions au lieu de
  maillages), 46-1 (ligne « Priorités » non circulaire). Les sept autres (46-5, 46-7 à 46-12) sont
  levés aussi, la plupart dans la rédaction exacte proposée.
- **Aucune réserve bloquante ne subsiste.** Les cinq nouveaux points sont de forme ou de couverture
  de test : **N-46-3** (la subdivision inconditionnelle n'est exercée par aucun critère) et
  **N-46-2** (la moitié « actif » du critère Zapper ne nomme ni pointeur ni valeur attendue)
  resserreraient les critères ; **N-46-1** (la règle d'ombre hors exigence) et **N-46-5** (critère
  facultatif à reclasser en **revue**) relèvent de la traçabilité ; **N-46-4** (renvoyer aussi à
  `REQ-E3D-643`) de la précision du renvoi. Un implémenteur n'a plus rien à deviner : les cinq
  peuvent être traités au prochain passage éditorial, avant le gel de la spec, sans bloquer le
  début des travaux. Ordre d'importance : N-46-3, N-46-2, N-46-1, N-46-5, N-46-4.
- **Aucune exigence gelée n'a besoin d'être rouverte** au-delà des deux déclarations déjà écrites
  dans l'en-tête (`REQ-SCN-055`, dont la parenthèse « tiles converties » était périmée depuis
  `AMD-009` Q-022 ; `REQ-INP-124`, exception du mode 2D plat). Ces deux déclarations sont de forme,
  pas de fond.
- **Reste ouvert, hors de ce fichier : 46-13** (T-4). `impl/docs/spec/` s'arrête à `AMD-038` et
  `SPEC-v1.2-INDEX.md` ne mentionne pour le mode 2D que « AMD-009 Q-022 (D) ». Le côté propre qui
  mettra en œuvre `REQ-SCN-074` et refermera `// Q-079` lit cet arbre : il faut y copier `AMD-039` à
  `AMD-046` et les deux rapports R2, et compléter les tables « mode 2D » et « divergences » de
  l'index avec `AMD-043` et `AMD-046`.

Le fichier `AMD-046-courbure-mode-2d.md` n'a pas été modifié par cette relecture ; rien n'a été
commité.
