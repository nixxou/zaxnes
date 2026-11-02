# R2 — Relecture d'`AMD-048` (section pleine enveloppée : intervalle de profondeur vide, v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-048-cylindre-plein-enveloppe.md` **en entier** et y chercher toute
fuite (code, pseudo-code, nom interne, algorithme recopié), toute incohérence avec `SPEC-04B`
(`REQ-E3D-423`, `REQ-E3D-444` à `-452`, §5.3 `TV-Y1` à `TV-Y8`), `AMD-008` (réponse à `Q-015`) ou
`Q-082`, tout critère intestable et toute référence cassée. **Recalcul à la main**, avec
`REQ-E3D-445`/`-446` et la règle amendée : les **deux cas de `TV-Y4`** et le **nouveau vecteur
`TV-Y9`** (`D′` compris).

Entrées relues : `spec/changes/AMD-048-cylindre-plein-enveloppe.md` (v1.0 du 2026-10-31, texte sous
revue, **non modifié**) ; `SPEC-GUIDE.md` ; `SPEC-04B` §2, §3.2 (`REQ-E3D-420` à `-426`), §3.3
(`REQ-E3D-440` à `-455`), §3.4 (`REQ-E3D-500` à `-524`), §3.5, §3.6 (`REQ-E3D-568`), §4.1, §5
(conventions, §5.1 à §5.5, `TV-Y1` à `TV-Y8` et « autres cas de contrôle »), §6, Annexe A,
Historique ; `spec/changes/Q-013`, `Q-014`, `Q-015`, `Q-081`, `Q-082` ; `AMD-008` (tableau des
réponses L4b), `AMD-021` §3 (confirmations du round 2), `AMD-047` (continuité de forme) ;
`R2-review-v1.md` (jugement demandé 1 : cylindres) et `R2-review-AMD-047.md` ;
`SPEC-v1.2-INDEX.md` (lignes `REQ-E3D-446/447`, `-450`, `-452`) ; `journal/JOURNAL.md` ; côté
`impl`, les deux seules marques de contexte utiles à l'impact
(`src/Zax.Engine3D/VoxelConstruction.cs`, `tests/Zax.Engine3D.Tests/CylinderVectorTests.cs`).
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
Le serveur d'outils de rétro-ingénierie présent dans la session n'a **pas** été utilisé.

Méthode : dépliage **indépendant** de `REQ-E3D-446` (récurrence de contour, régions 1 et 2 avec
leurs deux conditions) et de `REQ-E3D-445` (indices `u⁺`, `u⁻`, `z⁺`, `z⁻`) pour cinq sections —
les deux cas de `TV-Y4`, le segment de 2 pixels à `D = 3` de `TV-Y9`, le cas de contrôle
« rangée `11111111`, `D = 2` » de §5.3, et une rangée pleine à `D = 4` qui n'est couverte par aucun
vecteur — puis application de la règle amendée et comparaison avec les valeurs publiées ; relecture
de chaque critère d'acceptation comme un tiers qui ne connaîtrait que la spec.

Catégories utilisées, comme dans les rapports précédents : **LEAK**, **INCONSISTENT**,
**UNTESTABLE**, **UNCLEAR**, plus les points **rédactionnels**.

---

## 0. Verdict

| Document | Verdict | Points | Réserves bloquantes |
|---|---|---|---|
| **AMD-048** — section pleine enveloppée, intervalle vide (v1.0) | **Accepté avec réserves** | 10 (48-1 à 48-10) | **aucune pour l'étanchéité** ; 2 avant mise en œuvre : **48-1, 48-2** |

**Étanchéité : acquise.** Un seul point LEAK, de faible intensité et purement rédactionnel
(**48-8**) : l'amendement décrit deux fois un *parcours* (« parcourt `z` de `z⁻` à `z⁺` par valeurs
croissantes ») là où seul le *résultat* est observable. Aucun nom interne, aucun identifiant de
classe, de méthode ou de fichier, aucun pseudo-code, aucune structure recopiée ; la règle est
exprimée avec le vocabulaire déjà public de `REQ-E3D-445`/`-446`.

**Le fond est juste, et la réponse à la question posée est oui : `TV-Y4` reste exact, dans ses deux
cas.** Mon recalcul indépendant donne, sous la règle amendée, exactement les 48 voxels de `TV-Y2`
avec le texel `i` pour le premier cas, et exactement les 12 voxels publiés (18 rectangles
inchangés) pour `1123....` à `D = 4`. Le nouveau vecteur `TV-Y9` est exact lui aussi : deux voxels,
`D′ = 2`, maillage `8 × 2 × 1`. **`Q-015` et `Q-082` §4.1 se trompaient tous deux** en déclarant la
lecture « intervalle vide » réfutée par `TV-Y4` : leur raisonnement oublie le point de région 2
`(3, 2)`, dont les bornes ne sont **pas** inversées et qui pose à lui seul les quatre voxels
`(0,1)`, `(0,2)`, `(3,1)`, `(3,2)` attribués dans ces tickets aux deux points inversés.

Ce qui manque n'est donc pas la règle, mais sa **portée** : l'amendement présente le changement
comme ne concernant que « certains points de la région 2 » et ne l'illustre qu'à profondeur
**impaire** (`D = 3`, `A = B = 1`). Or il modifie aussi des sections de profondeur **paire** non
couvertes par les vecteurs — une rangée pleine de 8 pixels à `D = 4`, `solid = fullWarp = vrai`,
perd ses deux colonnes extrêmes et passe de 8 à **6 unités de large** (48-1). C'est le seul point
qui demande une décision plutôt qu'une correction de plume.

---

## 1. Recalculs demandés

Notations de `REQ-E3D-445` : `C = s + f`, `A = L`, `B = min(D − 1, L)`, `Cz = D − 1`,
`u⁺ = ⌊(C + p)/2⌋`, `u⁻ = C − u⁺` ; `z⁺ = ⌊(Cz + q)/2⌋` en région 1,
`z⁺ = ⌊(Cz + q − 1)/2⌋` en région 2, `z⁻ = Cz − z⁺`. Contour de `REQ-E3D-446` déplié point par
point, avec les deux conditions (transition calculée sur la nouvelle coordonnée majeure et
l'ancienne mineure ; rétention `B²p ≤ A²q` en région 1, `A²q ≤ B²p` en région 2). Types pleins :
tous les quadrants actifs, donc la règle amendée écrit `(u⁺, z)` avec le texel `u⁺` et `(u⁻, z)`
avec le texel `u⁻` (texels de `Q2` et `Q0`, `REQ-E3D-447`) : chaque colonne reçoit son propre
texel, ce qui est bien « comme une extrusion découpée par l'ellipse ».

### 1.1 `TV-Y4`, premier cas — rangée `11111111`, `VCYLINDER`, `D = 8` : **exact**

`s = 0`, `f = 7`, `L = 7` → `C = 7`, `A = 7`, `B = 7`, `Cz = 7`, `A²B² = 2401`.

- Région 1 : `(0,7)`, `(1,7)`, `(2,7)`, `(3,7)`, `(4,6)`, `(5,5)` — la transition bascule à
  `p = 4` (2646 ≥ 2401), `p = 5` (2450 ≥ 2401), `p = 6` (2450 ≥ 2401), et `(6,4)` est écarté par la
  rétention (`294 ≤ 196` faux).
- Région 2 (symétrique, `A = B`) : `(7,0)`, `(7,1)`, `(7,2)`, `(7,3)`, `(6,4)`, `(5,5)`.

| Point | Région | `u⁺` / `u⁻` | `z⁺` / `z⁻` | Règle amendée |
|---|---|---|---|---|
| `(0,7)`, `(1,7)`, `(2,7)` | 1 | 3/4, 4/3, 4/3 | 7 / 0 | `z = 0…7` → colonnes 3 et 4 pleines |
| `(3,7)` | 1 | 5/2 | 7 / 0 | `z = 0…7` → colonnes 5 et 2 pleines |
| `(4,6)` | 1 | 5/2 | 6 / 1 | `z = 1…6` (déjà couvert) |
| `(5,5)` | 1 | 6/1 | 6 / 1 | `z = 1…6` → colonnes 6 et 1, `k = 1…6` |
| `(7,0)` | 2 | 7/0 | 3 / 4 | **rien** (`z⁻ > z⁺`) |
| `(7,1)` | 2 | 7/0 | 3 / 4 | **rien** |
| `(7,2)`, `(7,3)` | 2 | 7/0 | 4 / 3 | `z = 3…4` → colonnes 7 et 0, `k = 3, 4` |
| `(6,4)`, `(5,5)` | 2 | 6/1 | 5 / 2 | `z = 2…5` (déjà couvert) |

Union : colonnes 2–5 sur `k ∈ [0,7]` (32 voxels), colonnes 1 et 6 sur `k ∈ [1,6]` (12), colonnes 0
et 7 sur `k ∈ {3,4}` (4) → **48 voxels**, exactement l'ensemble de `TV-Y2`
(`i = 0, 7 : k ∈ {3,4}` ; `i = 1, 6 : k ∈ [1,6]` ; `i = 2…5 : k ∈ [0,7]`), chacun portant le texel
de sa colonne. **Rien ne change** par rapport à `AMD-008` : ce que les points inversés `(7,0)` et
`(7,1)` écrivaient est écrit à l'identique par `(7,2)` et `(7,3)`, qui partagent leurs `u`.

### 1.2 `TV-Y4`, second cas — rangée `1123....`, `D = 4` : **exact**

`s = 0`, `f = 3`, `L = 3` → `C = 3`, `A = 3`, `B = 3`, `Cz = 3`, `A²B² = 81`.
Région 1 : `(0,3)`, `(1,3)`, `(2,3)` (`(3,2)` écarté par la rétention, `27 ≤ 18` faux).
Région 2 : `(3,0)`, `(3,1)`, `(3,2)` (`(2,3)` écarté, `27 ≤ 18` faux) — mêmes trois points que
`Q-015`.

| Point | Région | `u⁺` / `u⁻` | `z⁺` / `z⁻` | Règle amendée |
|---|---|---|---|---|
| `(0,3)` | 1 | 1/2 | 3 / 0 | `z = 0…3` → colonnes 1 et 2 pleines |
| `(1,3)`, `(2,3)` | 1 | 2/1 | 3 / 0 | `z = 0…3` (mêmes colonnes) |
| `(3,0)` | 2 | 3/0 | 1 / 2 | **rien** |
| `(3,1)` | 2 | 3/0 | 1 / 2 | **rien** |
| `(3,2)` | 2 | 3/0 | **2 / 1** | `z = 1…2` → `(3,1)`, `(3,2)`, `(0,1)`, `(0,2)` |

Résultat : `(0,1)`, `(0,2)` texel 0 ; `(1, 0…3)` texel 1 ; `(2, 0…3)` texel 2 ; `(3,1)`, `(3,2)`
texel 3 — **les 12 voxels publiés, au texel près**, donc les **18 rectangles** aussi (l'ensemble des
voxels est inchangé, `REQ-E3D-501`/`-502` donnent le même compte).

**Conclusion sur la question posée : `TV-Y4` reste exact, dans ses deux cas, sous la règle
amendée.** Le critère d'acceptation « `TV-Y4` inchangé » d'`AMD-048` est vrai. Le point `(3, 2)`,
que ni `Q-015` ni `Q-082` §4.1 ne prennent en compte, est la raison : en profondeur **paire** les
deux premiers points de la région 2 (`q = 0` et `q = 1`) ont les bornes inversées, mais le
troisième (`q = 2`), quand il est retenu, a exactement le même `u` et le même intervalle
`[m, m + 1]` — il écrit donc les mêmes voxels.

### 1.3 Nouveau vecteur `TV-Y9` — `HCYLINDER`, `D = 3`, segment de 2 pixels : **exact**

Colonne dont le segment occupe deux rangées consécutives : `L = 1` → `C = 2r + 1`, `A = 1`,
`B = min(2, 1) = 1`, `Cz = 2`, `A²B² = 1`. Ni `A` ni `B` nul : `REQ-E3D-451` ne s'applique pas.

- Région 1 : `(0,1)` ; transition vers `p = 1` : `1·1·0 + 1·1·0 = 0 ≥ 1` faux → `(1,1)`, retenu
  (`1 ≤ 1`) ; `p = 2` : `1·2·1 = 2 ≥ 1` vrai → `(2,0)`, écarté (`2 ≤ 0` faux). **`{(0,1), (1,1)}`**.
- Région 2 : `(1,0)` ; `q = 1` : `0 + 0 > 1` faux → `(1,1)`, retenu (`1 ≤ 1`) ; `q = 2` :
  `2 > 1` vrai → `(0,2)`, écarté (`2 ≤ 0` faux). **`{(1,0), (1,1)}`**.

| Point | Région | `u⁺` / `u⁻` | `z⁺` / `z⁻` | Règle amendée |
|---|---|---|---|---|
| `(0,1)` | 1 | `r` / `r+1` | 1 / 1 | `z = 1` → les deux rangées, tranche 1 |
| `(1,1)` | 1 | `r+1` / `r` | 1 / 1 | idem |
| `(1,0)` | 2 | `r+1` / `r` | **0 / 2** | **rien** (intervalle vide) |
| `(1,1)` | 2 | `r+1` / `r` | 1 / 1 | idem région 1 |

**Exactement deux voxels**, sur la seule tranche médiane, chacun avec le texel de sa propre rangée
(le texel de `Q2` vaut `u⁺`, celui de `Q0` vaut `u⁻`, et `{u⁺, u⁻} = {r, r + 1}`). La note
d'`AMD-048` (région 1 `(0,1)`, `(1,1)` avec `z⁻ = z⁺ = 1` ; région 2 `(1,0)` avec `z⁺ = 0`,
`z⁻ = 2` et `(1,1)` avec `z⁻ = z⁺ = 1`) est **juste au chiffre près**.

`D′` : `REQ-E3D-452` donne `D′ = max (B + 1) = 2` pour un objet dont tous les segments sont de ce
type. **Exact** (voir toutefois 48-7 : `D′` n'est pas ce que la boîte locale mesure).

Maillage, pour une tuile entière de telles colonnes : les voxels occupent `i ∈ [0,8)`, deux rangées,
`k = 1` ; `REQ-E3D-504` garde le `D` de construction, donc
`z ∈ [1 − 3/2, 2 − 3/2] = [−0,5 ; 0,5]` → `x` 8 unités, `y` 2 unités, `z` **1 unité** :
le `8 × 2 × 1` annoncé est **exact** (et la boîte locale vaut `(−4 ; −1 ; −0,5) … (4 ; 1 ; 0,5)` si
le segment est aux rangées d'image 3 et 4 ; 6 rectangles, 12 triangles).

### 1.4 Contrôle croisé : les autres vecteurs de §5.3 sont insensibles

- `TV-Y1`, `TV-Y3`, `TV-Y6`, `TV-Y7`, `TV-Y8` : contour (`solid = faux`) → `REQ-E3D-448`, que
  l'amendement ne touche pas.
- `TV-Y2` : `solid` sans `fullWarp` → `REQ-E3D-449`, intact.
- `TV-Y5` : cas dégénéré `REQ-E3D-451`, intact.
- « Autres cas de contrôle », rangée `11111111`, `D = 2` (`A = 7`, `B = 1`, `Cz = 1`) : région 1 =
  `(p,1)` pour `p = 0…7`, région 2 = `{(7,0)}` seul. En mode enveloppé la région 1 atteint
  `u⁺ = f = 7` et `u⁻ = s = 0` avec `z ∈ [0,1]` : les 16 voxels sont posés **avant** la région 2, et
  le point inversé `(7,0)` n'apportait rien de plus. Insensible, texels compris.
- Rangée `111.....`, `D = 8` : contour, intact.

### 1.5 Le cas que l'amendement ne dit pas : profondeur **paire**, section aplatie

Rangée pleine `11111111`, `VCYLINDER`, **`D = 4`**, `solid = fullWarp = vrai` :
`C = 7`, `A = 7`, `B = min(3,7) = 3`, `Cz = 3`, `A²B² = 441`.

- Région 1 : `(0,3)`, `(1,3)`, `(2,3)`, `(3,3)`, `(4,3)` (transitions fausses jusqu'à
  `p = 4` : 402 < 441), puis `p = 5` : 474 ≥ 441 → `(5,2)` ; `p = 6` : 368 < 441 → `(6,2)` ;
  `p = 7` : 476 ≥ 441 → `(7,1)`, **écarté par la rétention** (`63 ≤ 49` faux).
  Le plus grand `u⁺` atteint par la région 1 est donc `⌊(7+6)/2⌋ = **6**`.
- Région 2 : `(7,0)` ; `q = 1` : `0 + 378 > 441` faux → `(7,1)`, retenu (`49 ≤ 63`) ; `q = 2` :
  `98 + 378 = 476 > 441` vrai → `(6,2)`, **écarté** (`98 ≤ 54` faux). **`{(7,0), (7,1)}`**, et les
  deux ont `z⁺ = 1`, `z⁻ = 2` : **bornes inversées**.

| Règle | Voxels | Boîte locale |
|---|---|---|
| `AMD-008` (`Q-015`) | **24** : colonnes 2–5 `k ∈ [0,3]`, colonnes 1 et 6 `k ∈ [1,2]`, colonnes 0 et 7 `k ∈ [1,2]` | `x ∈ [−4, 4]` (**8 unités**) |
| `AMD-048` | **20** : les mêmes, **sans les colonnes 0 et 7** | `x ∈ [−3, 3]` (**6 unités**) |

Les deux colonnes extrêmes disparaissent entièrement, parce qu'aucun autre point ne les atteint :
la région 1 s'arrête à `u⁺ = 6` et la région 2 n'a que ses deux points inversés. Le cylindre est
**2 unités plus étroit**, sur un cas aussi banal qu'une rangée pleine à `sizeZ = 4`. Les deux
lectures donnent ici le **même nombre de rectangles** (18 de part et d'autre, recalculé
face par face) : seul l'ensemble des voxels et la boîte discriminent — un test qui ne compterait
que les faces ne verrait rien.

Critère général, utile à l'amendement : en profondeur **paire** (`Cz = 2m + 1`) les deux points
`q = 0` et `q = 1` de la région 2 ont `z⁺ = m`, `z⁻ = m + 1` et deviennent muets ; leurs voxels ne
survivent que si la région 2 retient `(A, 2)` — ce qui demande **`2A ≤ B²`** — ou si la région 1
atteint `u⁺ = f`. `TV-Y4` est dans le premier cas (`2·7 ≤ 49`, `2·3 ≤ 9`), le cas de contrôle
`D = 2` dans le second, la rangée pleine à `D = 4` dans **aucun des deux** (`14 > 9`). En
profondeur **impaire** (`Cz = 2m`) seul `q = 0` est muet, et il perd toujours les tranches
`m − 1` et `m + 1` des colonnes extrêmes : c'est précisément la correction voulue pour les cordes.

---

## 2. Points bloquants avant mise en œuvre

### 48-1 — (INCONSISTENT / UNTESTABLE) La portée réelle de la règle n'est ni annoncée, ni vérifiée, ni verrouillée : la profondeur **paire** change aussi — **bloquant**

`AMD-048` §1 affirme :

> « La différence ne se voit que pour les points dont `z⁻ > z⁺`, c'est-à-dire certains points de la
> région 2 (REQ-E3D-446). Elle est visible sur les cordes du ring de « Punch-Out!! » […] »

La première phrase est vraie (la région 1 n'a jamais `z⁻ > z⁺`, puisque
`z⁺ = ⌊(Cz + q)/2⌋ ≥ Cz/2`), mais elle laisse croire que l'écart se limite aux profondeurs
impaires du type observé. Le §1.5 ci-dessus montre le contraire : **une rangée pleine de 8 pixels à
`D = 4`, `solid = fullWarp = vrai`, perd ses colonnes 0 et 7** et passe de 8 à 6 unités de large.
Le seul élément de preuve cité est un relevé à `D = 3` avec `A = B = 1` ; **aucun** vecteur de §5.3
ne couvre le cas discriminant, et aucune observation n'est rapportée pour la profondeur paire.
La classe concernée n'est pas marginale : Annexe A compte 10 589 objets à `sizeZ = 4`, 5 092
`VCYLINDER` à `fullWarp` et 13 475 objets `solid` ; le critère `2A > B²` est atteint dès qu'un
segment plein de 6 pixels ou plus est construit à `D = 4` (et dès 26 pixels à `D = 8`, ce qui
concerne les patterns larges — celui des cordes fait 240 pixels de large). `Q-015` chiffrait déjà
l'enjeu dans l'autre sens (« sans la lecture retenue, 5 092 objets `VCYLINDER` à `fullWarp` perdent
une partie de leurs voxels ») : c'est exactement cette perte qu'`AMD-048` rend normative, sans le
dire.

**Correction proposée**, en trois parties :

1. Dire la portée dans l'exigence ou ses Notes, par exemple : « Les points concernés sont ceux de la
   région 2 dont les bornes sont inversées : `q = 0` si `D` est impair, `q = 0` et `q = 1` si `D`
   est pair. Leurs voxels ne sont réécrits par un autre point que si la région 2 retient `(A, 2)`
   (`2A ≤ B²`) ou si la région 1 atteint `u⁺ = f` ; sinon la section perd les colonnes extrêmes.
   `TV-Y4` est dans le premier cas, le cas de contrôle `D = 2` de §5.3 dans le second. »
2. Ajouter à §5.3 le vecteur de profondeur paire qui manque, **TV-Y10** (numéro libre) :
   « Rangée `11111111`, `VCYLINDER`, `D = 4`, `solid = fullWarp = vrai` → 20 voxels : `i = 2…5`,
   `k ∈ [0,3]` ; `i = 1` et `i = 6`, `k ∈ {1,2}` ; **aucun voxel en `i = 0` et `i = 7`** ; chaque
   voxel `(i, k)` porte le texel `i` ; `D′ = 4` ; boîte locale `x ∈ [−3, 3]`, `y ∈ [3, 4]`,
   `z ∈ [−2, 2]` ; 18 rectangles (le compte ne discrimine pas : comparer les voxels et la boîte). »
3. Faire confirmer ce cas par un différentiel R5 sur l'original — c'est le même type de contrôle que
   celui qui a produit l'amendement, et il est bien plus visible que les cordes (deux unités de
   largeur). Si le différentiel n'est pas possible tout de suite, le dire : « la conséquence en
   profondeur paire est une extrapolation de la même règle, non encore observée », et classer le
   risque, comme `AMD-008` l'a fait pour `Q-014`/`Q-016` (« à confirmer au round 2 »).

**Bloquant : oui.** Pas pour l'étanchéité, mais parce que l'amendement est normatif P1 et renverse,
sans le dire et sans vecteur, le résultat d'une classe d'objets beaucoup plus large que celle qui
l'a motivé. Un implémenteur qui applique `AMD-048` à la lettre rétrécira des cylindres que personne
n'a relevés ; un implémenteur qui applique l'esprit (« ne corriger que la profondeur impaire »)
écrira une autre règle. Les deux lectures ne peuvent pas rester ouvertes.

### 48-2 — (UNCLEAR) `TV-Y9` : le plan des voxels n'est pas celui de §5.3, et le vecteur devient faux hors de `r = 3` — **bloquant**

`AMD-048` écrit :

> « une colonne dont le segment occupe les rangées `r` et `r + 1` (`L = 1`) → exactement deux
> voxels, `(r, 1)` et `(r + 1, 1)` dans le plan (rangée, profondeur) »

Mais §5.3 fixe une autre convention pour les types H : « les voxels sont donnés […] en `(j, k)`
pour les types H (colonne `i` fixée) », et `j` est l'indice de voxel, lié à la rangée d'image par
`j = 7 − y` (`REQ-E3D-421`) — c'est ce que font `TV-Y3` (rangées d'image 0 à 3 → voxels `j = 4…7`)
et `TV-Y8` (« face plane en bas, rangée 3 de l'image, `j = 4` »). Pour un segment aux rangées
d'image `r` et `r + 1`, les voxels sont donc en `j = 7 − r` et `j = 6 − r`, et la paire
`{r, r + 1}` ne coïncide avec `{6 − r, 7 − r}` que pour `r = 3`. Le vecteur est donc juste si l'on
lit `r` comme une coordonnée le long de la ligne (`u`, comme `Q-082` §2), et **faux dès `r ≠ 3`**
si on le lit avec la convention de la section qui va l'accueillir. Un test automatique écrit
depuis le texte, avec `r = 5`, attendrait `j = 5, 6` là où la règle donne `j = 2, 3` : il échouerait
sur une implémentation correcte. L'ensemble des voxels, lui, est le même dans les deux lectures
(« les deux rangées du segment, tranche médiane »), donc le défaut est de notation — mais dans un
critère d'acceptation normatif.

**Correction proposée** : fixer le segment et écrire le vecteur dans la convention de §5.3, par
exemple : « **TV-Y9** — Colonne 0, rangées 3 et 4 = 1 et 2, reste 0, `HCYLINDER`, `D = 3`,
`solid = fullWarp = vrai` → deux voxels, `(j, k) →` rangée : `(4,1)→3`, `(3,1)→4` ; `D′ = 2` ;
6 rectangles ; boîte locale `x ∈ [−4, −3]`, `y ∈ [−1, 1]`, `z ∈ [−0,5 ; 0,5]`. Pour une tuile
entière de telles colonnes (8 pixels de large), 6 rectangles et une boîte `8 × 2 × 1`
(`x ∈ [−4, 4]`, `y ∈ [−1, 1]`, `z ∈ [−0,5 ; 0,5]`). » Deux valeurs de texel distinctes rendent en
outre vérifiable le « texel de sa propre rangée », ce que deux rangées de même couleur ne font pas.
**Bloquant : oui**, correction d'une ligne, mais c'est un critère d'acceptation P1 ambigu dont une
des deux lectures produit des valeurs attendues fausses.

---

## 3. Points non bloquants

### 48-3 — (UNCLEAR) « `TV-Y4` inchangé » est affirmé sans l'argument, alors que `Q-015` **et** `Q-082` affirment le contraire — **non bloquant**

L'amendement se contente de « `TV-Y4` inchangé (les deux cas restent exacts sous cette règle) ».
Or les deux tickets qu'il remplace et qu'il clôt disent, noir sur blanc, que cette règle est
réfutée par `TV-Y4` : `Q-015` (« `TV-Y4` lève le doute […] y compris par les points où
`z⁺ < z⁻` ») et `Q-082` §4.1 (« elle est **réfutée** par `TV-Y4` […] dont les voxels `(0,1)`,
`(0,2)`, `(3,1)`, `(3,2)` ne viennent que des points de région 2 `(3,0)` et `(3,1)`, tous deux de
bornes inversées »). Les deux raisonnements ont la **même erreur** : ils oublient le troisième point
de la région 2, `(3, 2)`, dont les bornes `(z⁺, z⁻) = (2, 1)` ne sont pas inversées et qui pose à
lui seul ces quatre voxels (§1.2 ci-dessus — `Q-015` le liste pourtant dans son propre tableau).
Laisser trois documents normatifs en contradiction apparente, sans dire lequel se trompe et
pourquoi, c'est exposer la règle à une réouverture : le prochain lecteur qui relira `Q-015` y
trouvera une « réponse R0 normative » et un argument de test qui invitent à revenir en arrière.

**Correction proposée** : ajouter une Note d'une phrase à `REQ-E3D-450` amendée — « `TV-Y4` reste
exact parce que les points de région 2 à bornes inversées y sont suivis d'un point de même `u` et
de même intervalle (`(3, 2)` pour `D = 4`, `(7, 2)` et `(7, 3)` pour `D = 8`) ; l'argument contraire
de `Q-015` et de `Q-082` §4.1 omettait ce point » — et porter une ligne de clôture en bas de
`Q-015.md` et de `Q-082.md` renvoyant à `AMD-048`. **Bloquant : non**, mais c'est le point qui
protège le mieux la décision.

### 48-4 — (INCONSISTENT) La seconde lecture de `Q-082` n'est pas écartée, alors qu'elle l'est — par `Q-014` et `AMD-021` — **non bloquant**

`Q-082` §4 propose **deux** lectures qui donnent la section d'une unité pour les cordes, et retient
la seconde (« région 2 démarrant à `(A, 1)` », donc le point `(A, 0)` sans contribution) comme
la seule survivante ; son §5.2 construit même un différentiel pour elle. `AMD-048` adopte la
première sans mentionner la seconde. Elles ne sont pas équivalentes : la seconde touche le contour
lui-même, donc **aussi** `REQ-E3D-448` et `REQ-E3D-449`, et laisse la profondeur paire intacte ;
la première ne touche que `REQ-E3D-450` mais change la profondeur paire (48-1). Le choix mérite
d'être justifié d'autant qu'il est, lui, démontrable : la seconde lecture est **réfutée par
`Q-014`**, dont la réponse R0 a été **confirmée au round 2** (`AMD-021` §3, « `Q-014` (cas de
contrôle des cylindres) », différentiel sur un cylindre plein de profondeur 2). `Q-014` établit que
pour une rangée pleine à `D = 2` les colonnes extrêmes portent les texels **inversés**, ce qui
n'arrive que si le point `(A, 0)` de la région 2 **écrit** — et à `D = 2` la région 2 se réduit
justement à ce seul point (`(7,1)` échoue à la rétention, `49 ≤ 7` faux). Si la région 2 commençait
à `q = 1`, elle serait vide dans ce cas et les texels ne seraient pas inversés.

**Correction proposée** : une phrase au §1 ou §3 — « La lecture alternative de `Q-082` §4.2
(région 2 sans son point `q = 0`) est écartée : elle viderait la région 2 pour une rangée pleine à
`D = 2`, alors que `Q-014`, confirmé au round 2 (`AMD-021` §3), y relève les texels inversés des
colonnes extrêmes, qui ne viennent que de ce point. » **Bloquant : non**, mais sans elle `Q-082`
reste ouvert dans les faits : son §5.2 demande un différentiel pour une hypothèse qu'`AMD-048`
abandonne en silence.

### 48-5 — (UNCLEAR) `REQ-E3D-448` et `REQ-E3D-449` restent intacts : les quatre couples de drapeaux ne s'accordent plus — **non bloquant**

`Q-082` §2 relève que, pour un segment de 2 pixels à `D = 3`, les **quatre** couples
(`solid`, `fullWarp`) donnent le même champ de voxels, la section pleine `2 × 3`. Après `AMD-048`,
ce n'est plus vrai : le couple `solid = fullWarp = vrai` donne 2 voxels (1 unité de profondeur), les
trois autres continuent d'en donner 6 (3 unités), via les points quadrants de `REQ-E3D-448` ou les
miroirs de `REQ-E3D-449` — que l'amendement ne touche pas. C'est cohérent (le relevé ne porte que
sur des cordes `solid` + `fullWarp`), mais ce n'est pas dit, et c'est la moitié de la question 1 de
`Q-082` (« `REQ-E3D-446` et `REQ-E3D-445` sont-ils complets pour `D` impair ? »). En l'état, un
lecteur peut comprendre qu'`AMD-048` corrige « la section des cylindres de profondeur impaire »
en général.

**Correction proposée** : une Note — « `REQ-E3D-448` (contour) et `REQ-E3D-449` (section pleine non
enveloppée) sont inchangés : le point de région 2 à bornes inversées y pose ses voxels comme avant,
et le même segment de 2 pixels à `D = 3` y occupe toujours les trois tranches. Aucune observation ne
porte sur ces deux modes à profondeur impaire ; l'asymétrie est assumée. » **Bloquant : non.**

### 48-6 — (UNTESTABLE) `TV-Y9` manque des données que §5.3 fournit partout ailleurs — **non bloquant**

Au-delà de la convention (48-2), le vecteur ne fixe ni la rangée du segment, ni les valeurs de
texel, ni le nombre de rectangles, ni la boîte locale, là où `TV-Y1` à `TV-Y8` donnent les trois
dernières systématiquement (« 24 voxels ; maillage `GREEDY` : 64 rectangles ; `D′ = 8` »,
« 18 rectangles », « boîte locale `x ∈ [−1, 0]` »). Le « `8 × 2 × 1` unités » est une dimension,
pas une position : c'est la position (`z ∈ [−0,5 ; 0,5]`, centrée sur `D = 3` et non sur `D′ = 2`)
qui montre que `REQ-E3D-504` s'applique avec la profondeur de construction. Enfin, le cas réel
mériterait le critère `[ROM]` qu'`AMD-047` emploie, avec le profil nommé comme un fichier
vérifiable : « `[ROM]` : `assets-neutral/profiles/punchout.bytes`, pattern `HCYLINDER` de
`sizeZ = 3` (cordes du ring, rangées 5-6, 13-14, 21-22 de la texture) : chaque tuile de corde donne
6 rectangles et une boîte locale `(−4 ; −3 ; −0,5) … (4 ; −1 ; 0,5)`. »

**Correction proposée** : compléter `TV-Y9` comme indiqué en 48-2, plus le critère `[ROM]`
ci-dessus. **Bloquant : non.**

### 48-7 — (UNCLEAR) `D′ = 2` est une déduction de `REQ-E3D-452`, pas une observation : la question 3 de `Q-082` reste ouverte — **non bloquant**

`AMD-048` annonce `D′ = 2` dans le même souffle que le maillage `8 × 2 × 1`, ce qui peut se lire
comme si la mesure confirmait `D′`. Elle ne le peut pas : `REQ-E3D-452` dit expressément que « le
champ de voxels et le centrage local des maillages restent ceux de la profondeur initiale `D` ».
La boîte locale de 1 unité est donc **indépendante** de `D′` ; `D′` n'est observable que par le
placement (`REQ-E3D-403`, `-404`, `-408`), la déformation (`REQ-E3D-568`) et la valeur de `sizeZ`
relue par un script (`Q-013`, `SPEC-05`) — et `Q-081` rappelle que le `depth` exporté par l'outil
est le `D′` de l'objet entier. `D′ = 2` est donc correct **par la règle** (§1.3), mais la question 3
de `Q-082` (« la corde est-elle, dans l'original, de profondeur effective `D′ = 2` ? ») n'est pas
tranchée par le relevé cité.

**Correction proposée** : scinder la phrase — « `D′ = 2` par `REQ-E3D-452` (`B + 1`), le centrage
local restant celui de `D = 3` (`REQ-E3D-504`) : le maillage `8 × 2 × 1` ne dépend pas de `D′` » —
et, au §3, répondre explicitement à la question 3 de `Q-082` (confirmée, ou laissée au round 2).
**Bloquant : non.**

### 48-8 — (LEAK, faible) Le texte décrit un **parcours** là où seul le résultat est observable — **non bloquant**

Deux formulations décrivent le déroulement interne plutôt que le comportement :

> §1 : « Le programme d'origine (1.x et 2.0, comportement identique) **parcourt** `z` de `z⁻` à
> `z⁺` par valeurs croissantes »
> §2 : « pour chaque entier `z` de `z⁻` à `z⁺` **par valeurs croissantes** (bornes incluses ;
> aucun si `z⁻ > z⁺`) »

`SPEC-GUIDE` interdit au document « du pseudo-code qui suit la structure de l'original » et
« l'architecture interne de l'original […] sauf effet observable ». Ici, l'ordre **n'a aucun effet
observable** : dans un point de contour donné, le texel écrit ne dépend pas de `z`
(`REQ-E3D-447` le fixe par quadrant), donc l'ordre des `z` est invisible y compris sous la règle
« la dernière écriture l'emporte » de `REQ-E3D-423` ; seul l'ordre **des points** est normatif, et
`REQ-E3D-446` le dit déjà (« l'ordre n'est normatif que pour la couleur des voxels écrits plusieurs
fois »). Le sens croissant est donc la seule partie de la phrase qui ne serve à rien d'autre qu'à
décrire la mécanique interne. Le reste du document est propre : aucun nom de classe, de méthode ou
de champ, aucun numéro de ligne, aucun nom de fichier d'origine, aucune récurrence recopiée (celles
de `REQ-E3D-445`/`-446` sont déjà publiques et jugées recevables par `R2-review-v1.md`, jugement
demandé 1).

**Correction proposée** : exprimer la condition en ensemble, le résultat reste identique — §2 :
« pour chaque entier `z` tel que `z⁻ ≤ z ≤ z⁺` (l'ensemble est **vide** si `z⁻ > z⁺`) » ; §1 :
« Le programme d'origine (1.x et 2.0, comportement identique) **n'écrit aucun voxel** pour les
points de contour dont `z⁻ > z⁺`. » Ajouter, si l'on veut lever toute hésitation : « l'ordre
d'énumération des `z` est libre : le texel d'un point ne dépend pas de `z` ». **Bloquant : non**
(aucune information non publique n'est divulguée), mais c'est la seule retouche d'étanchéité à
faire.

### 48-9 — (rédactionnel) Hygiène documentaire : index, §5.3, historique, phrase vide — **non bloquant**

`AMD-048` ne dit pas où ses changements atterrissent, là où `AMD-047` consacre un §3 à cette
mécanique. Restent à déclarer :

- `SPEC-v1.2-INDEX.md` ligne `REQ-E3D-450` : elle annonce encore « **AMD-008 Q-015** : intervalle
  `[min(z⁻,z⁺), max(z⁻,z⁺)]`, bornes incluses », exactement ce qu'`AMD-048` remplace ;
- §5.3 de `SPEC-04B` doit **recevoir** `TV-Y9` (et `TV-Y10` si 48-1 est suivi) : l'amendement crée
  le vecteur dans un critère d'acceptation sans dire qu'il entre dans la section des vecteurs ;
- l'Historique de `SPEC-04B` (qui s'arrête à 1.1-draft/`AMD-001`) ne mentionnera ni `AMD-048` ni
  les amendements intermédiaires — au minimum, y porter cette ligne ;
- « Le reste de `REQ-E3D-450` est inchangé » est **vide de contenu** : le §2 réénonce l'exigence
  entière, texels, masque de quadrants et renvoi à `REQ-E3D-423` compris. Soit la supprimer, soit
  la remplacer par ce qui est effectivement conservé (« le critère d'acceptation `TV-Y4` est
  conservé, complété par `TV-Y9` ») ;
- §1, « des cordes **trois fois plus épaisses** » : c'est la **profondeur** qui est triplée
  (3 unités au lieu de 1), la largeur et la hauteur ne changent pas ; « trois fois plus
  profondes » évite l'ambiguïté. De même, nommer le fichier du profil
  (`assets-neutral/profiles/punchout.bytes`) rend le relevé vérifiable côté propre.

**Correction proposée** : un §3 « Changements documentaires » sur le modèle d'`AMD-047` §3, listant
les quatre premiers points, et les deux retouches de plume. **Bloquant : non.**

### 48-10 — (UNCLEAR) Deux conséquences non couvertes par un vecteur : `antiShadow` et les demi-cylindres — **non bloquant**

Un point de région 2 muet emporte avec lui tout ce que la région 2 apportait de spécifique :

- `antiShadow` ne modifie les texels que « pour les points de **région 2** seulement »
  (`REQ-E3D-447`). En mode enveloppé, les points à bornes inversées ne recolorent donc plus rien ;
  1 856 objets ont un `antiShadow` effectif sur cylindre, et `TV-Y6`, seul vecteur `antiShadow`,
  est en mode contour : rien ne verrouille `solid + fullWarp + antiShadow`.
- Pour les types demi (`A = 2L`, `B = min(D − 1, 2L)`, `C = 2s` ou `2f`), la règle amendée vaut
  aussi, avec une seule écriture par `z` (un seul côté de quadrants actif) : un point muet fait
  disparaître la totalité de sa contribution. Le cas est moins exposé (`B` y est plus grand, donc
  `2A ≤ B²` plus souvent vrai), mais `TV-Y8` est en mode contour : là non plus rien ne verrouille.

**Correction proposée** : une Note signalant les deux cas comme non verrouillés (et, si un
différentiel est lancé pour 48-1, y ajouter un demi-cylindre plein enveloppé). **Bloquant : non.**

---

## 4. Étanchéité (bilan LEAK)

| Contrôle | Résultat |
|---|---|
| Code ou pseudo-code suivant la structure de l'original | **Aucun** |
| Noms internes (classes, méthodes, champs, variables) | **Aucun** |
| Références au code d'origine (fichiers, lignes, méthodes) | **Aucune** |
| Architecture interne (découpage, threads) | **Aucune** |
| Algorithme recopié | **Aucun** — la récurrence de contour reste celle de `REQ-E3D-445`/`-446`, déjà jugée recevable (algorithme public, verrouillé par les vecteurs) |
| Description d'un déroulement interne sans effet observable | **1 occurrence** (48-8, « parcourt `z` […] par valeurs croissantes »), reformulable sans perte |

L'amendement reste du côté « résultat » : il énonce une condition sur les bornes d'un point de
contour et ses conséquences en voxels, en texels et en dimensions de maillage, toutes observables
par un tiers qui ne connaît que la spec. La mention « 1.x et 2.0, comportement identique » est un
fait d'interopérabilité recevable, et n'expose rien de la mise en œuvre.

---

## 5. Conclusion — gate R2

- **Étanchéité : acquise.** Aucun point LEAK bloquant ; une seule phrase à reformuler (**48-8**),
  sans conséquence normative. Rien dans ce rapport ne justifie de retenir la publication pour des
  raisons de salle blanche.
- **Le fond est juste, et la question posée a une réponse nette : `TV-Y4` reste exact, dans ses deux
  cas.** Mes deux recalculs indépendants retrouvent, sous la règle amendée, les 48 voxels de
  `TV-Y2` avec le texel `i` (`D = 8`) et les 12 voxels / 18 rectangles de `1123....` (`D = 4`),
  parce que les points de région 2 à bornes inversées y sont doublés par un point de même `u` et de
  même intervalle. Le nouveau vecteur **`TV-Y9` est exact** : deux voxels sur la tranche médiane,
  chacun avec le texel de sa rangée, `D′ = 2` par `REQ-E3D-452`, maillage `8 × 2 × 1` par
  `REQ-E3D-504` (centré sur `D = 3`). La règle retenue est de surcroît la **seule** des deux
  candidates de `Q-082` qui survive à tous les vecteurs **et** à `Q-014` confirmé au round 2
  (48-4) ; `Q-015` et `Q-082` §4.1 se trompaient en la déclarant réfutée (48-3).
- **Verdict : accepté avec réserves**, 10 points, dont **aucun bloquant pour l'étanchéité** et
  **deux à traiter avant la mise en œuvre** : **48-1** (la règle change aussi la profondeur
  **paire** — une rangée pleine à `D = 4` perd ses deux colonnes extrêmes et rétrécit de 8 à
  6 unités : à annoncer, à verrouiller par un vecteur, et à faire confirmer par un différentiel ou
  à déclarer comme extrapolation) et **48-2** (`TV-Y9` écrit dans une convention de coordonnées qui
  n'est pas celle de §5.3, et devient faux dès que le segment n'est pas aux rangées 3 et 4).
  **48-1** est le seul point qui demande une décision ; **48-2** est une correction d'une ligne.
- **Ensuite, par ordre d'importance** : **48-3** et **48-4** (ils ferment la contradiction avec
  `Q-015`/`Q-082` et écartent la lecture concurrente — sans eux, la décision se rouvrira),
  **48-5** (dire que `REQ-E3D-448`/`-449` ne changent pas, donc que les quatre couples de drapeaux
  ne s'accordent plus), **48-7** (`D′` déduit et non observé, question 3 de `Q-082`), **48-6** et
  **48-10** (vecteurs incomplets, cas non verrouillés), puis **48-9**, rédactionnel.
- **Aucune exigence gelée n'a besoin d'être rouverte** au-delà de ce que l'amendement déclare déjà
  (`REQ-E3D-450` et son critère), plus l'ajout de `TV-Y9`/`TV-Y10` à §5.3, la ligne
  `REQ-E3D-450` de `SPEC-v1.2-INDEX.md` et les clôtures de `Q-015.md` et `Q-082.md`.
  `REQ-E3D-445`, `-446`, `-447`, `-448`, `-449`, `-451` et `-452` sont **inchangés** par cette
  lecture, ce que mon recalcul confirme vecteur par vecteur (§1.4).
- **Impact de mise en œuvre, pour mémoire** : le moteur applique aujourd'hui la réponse de
  `Q-015` (`Q-082` §3) ; les deux emplacements marqués `// Q-015` / `// Q-082` et les tests de
  `CylinderVectorTests` qui verrouillent la section de 3 unités à `D = 3` sont les points à
  reprendre, et le vecteur de 48-1 est le test à ajouter. Rien n'a été modifié côté `impl`.

Le fichier `AMD-048-cylindre-plein-enveloppe.md` n'a pas été modifié par cette relecture ; rien
n'a été commité.
