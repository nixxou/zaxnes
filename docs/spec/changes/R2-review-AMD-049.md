# R2 — Relecture d'`AMD-049` (éclairage du backdrop, v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-049-eclairage-backdrop.md` **en entier** et y chercher toute fuite
(code, pseudo-code, nom interne non nécessaire, algorithme recopié du programme d'origine plutôt
que décrit comme comportement ou repris d'une source publique), toute incohérence avec `SPEC-06`
(`REQ-SCN-025`, `-026`, `-027`, `-028`, `-030`, `-031`, `-048`) ou les amendements récents, tout
critère intestable, tout numéro `REQ` déjà pris et toute référence cassée. **Recalcul à la main**
des deux vecteurs automatiques, et jugement sur la cohérence interne des formules (bornes, clamp,
unités, linéaire/sRGB) et sur leur applicabilité sans deviner.

Entrées relues : `spec/changes/AMD-049-eclairage-backdrop.md` (v1.0 du 2026-10-31, texte sous
revue, **non modifié**) ; `SPEC-GUIDE.md` ; `SPEC-06` §2 (cadre de référence), §3.4
(`REQ-SCN-009`), §3.9 (`REQ-SCN-025` à `-029`), §3.10 (`REQ-SCN-030` à `-033`), §3.11
(`REQ-SCN-034` à `-043`), §3.12 (`REQ-SCN-044`, `-045`), §3.13 (`REQ-SCN-046` à `-048`), §3.14
(`REQ-SCN-049`), `REQ-SCN-053`/`-054` (matériaux TV), `REQ-SCN-058` (ordre de rendu), §4.2/§4.3
(tables de réglages), §5.2 (vecteurs d'ombrage), §5.4 (captures de référence R1 à R7), §6, Historique ;
`spec/changes/` : `AMD-040` (`VisualStyle`, styles Classic/Enhanced), `AMD-043` (`REQ-SCN-066` à
`-068`, `REQ-INP-124`), `AMD-045`, `AMD-046` (`REQ-SCN-074`), `AMD-047`, `AMD-048` (forme et
en-têtes), `AMD-009` (`Q-021`), `AMD-012`, `AMD-019` (`Q-045`), `AMD-021` (`R2-010`, `R2-014`,
`R2-015`), `Q-021`, `Q-023`, `Q-038`, `Q-045`, `Q-072`, `Q-079`, `Q-082` ; `R2-review-AMD-046.md`
et `R2-review-AMD-046-v1.1.md` (seules autres occurrences de `REQ-SCN-075`),
`R2-review-AMD-039-045-v1.1.md` (clause de priorité T-1), `R2-review-AMD-047.md`,
`R2-review-AMD-048.md`, `R2-review-AMD-021.md` (`R2-015`) ; `SPEC-03` §4.1, §4.6, §4.7 (format
`Settings`/`Light`/`Argb`), `SPEC-05` (`REQ-LUA-090`, types `Color`), `SPEC-09` (`REQ-PERS-008`,
ligne `camera.local_cam`), `SPEC-04` §4.1 (palette NES), `SPEC-12`, `SPEC-v1.2-INDEX.md`,
`TRACEABILITY-CHECK-P6.md`, `DECISIONS-R0.md`, `journal/JOURNAL.md` ;
`assets-neutral/profiles/punchout.bytes` (réglages embarqués et scripts Lua du profil cité par le
critère `[ROM]`, lus avec le format de `SPEC-03`) ; `third_party/3DNes_Engine/` (aucun shader :
cette source ne couvre pas l'éclairage).
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
Le serveur d'outils de rétro-ingénierie présent dans la session n'a **pas** été utilisé.

Méthode : réécriture **indépendante** du modèle de `REQ-SCN-075` dans un petit script de calcul
(écrit depuis le texte de l'amendement seul, sans regarder l'implémentation), puis (a) recalcul des
deux vecteurs automatiques terme par terme, (b) recalcul du vecteur `[ROM]` « Punch-Out!! » à
partir des réglages réellement portés par le profil d'`assets-neutral`, pour cinq orientations de
caméra et pour les deux lectures possibles de `E`, (c) recalcul de l'ancien modèle diffus sur la
même scène pour mesurer l'écart annoncé au §1, (d) recalcul du critère d'acceptation de
`REQ-SCN-048` (panneau noir) sous le nouveau modèle, (e) étude des bornes des cinq facteurs et des
cas dégénérés. Chaque critère a ensuite été relu comme le ferait un tiers qui ne connaîtrait que la
spec.

Catégories utilisées, comme dans les rapports précédents : **LEAK**, **INCONSISTENT**,
**UNTESTABLE**, **UNCLEAR**, plus les points **rédactionnels**.

---

## 0. Verdict

| Document | Verdict | Points | Réserves bloquantes |
|---|---|---|---|
| **AMD-049** — éclairage du backdrop (v1.0) | **Accepté avec réserves** | 12 (49-1 à 49-12) | **aucune pour l'étanchéité** ; 3 avant mise en œuvre : **49-1, 49-2, 49-3** |

- **Étanchéité : rien à signaler.** Aucun code, aucun pseudo-code, aucun nom interne de classe, de
  méthode, de champ ou de variable du programme d'origine, aucune référence à ses fichiers source.
  Le modèle est donné en notation mathématique et sourcé explicitement sur les **sources publiques
  des shaders intégrés de Unity**, ce que `SPEC-GUIDE` §« Ce que la spec PEUT contenir » autorise
  (documentation publique, description mathématique d'un résultat, constantes justifiées).
- `REQ-SCN-075` est **libre** : le plus grand identifiant attribué est `REQ-SCN-074` (`AMD-046`), et
  `REQ-SCN-075` n'apparaît ailleurs que comme **suggestion non retenue** : point 46-8 de
  `R2-review-AMD-046.md` (scinder `REQ-SCN-074` en deux exigences, « forme plus lourde et moins
  souhaitable ») et le bilan de `R2-review-AMD-046-v1.1.md` (« elle économise un identifiant »).
  La version 1.1 d'`AMD-046` a délibérément gardé un seul identifiant. Aucun conflit.
- **Les deux vecteurs automatiques sont exacts** : je retrouve, à partir du seul texte de
  l'amendement, `Fd = 1`, `Vis = 0,25`, `D = 0,3314`, `Spec = 0,2603`, `F = 0,04`,
  `0,96 × 1,1812 = 1,133952`, le terme additif `0,01041`, et `0,2372` puis `0,0753` pour
  `c = 0,2` (§1.1). Le vecteur `[ROM]` est lui aussi exact, **mais seulement sous des hypothèses
  que l'amendement ne donne pas** (49-1, 49-2).
- **Les formules sont cohérentes entre elles** (§1.3) : un seul espace de couleur (linéaire), tous
  les produits scalaires clampés, toutes les quantités sans dimension, aucune division par zéro
  possible aux valeurs fixées. **Deux points manquent pour qu'un implémenteur n'ait rien à
  deviner** : la définition exacte de `E` (49-1) et l'encodage de sortie (écrêtage + sRGB, 49-4).
  Le reste est applicable tel quel.
- **Référence cassée : aucune.** `REQ-SCN-025`, `-027`, `-031`, `-048` sont citées à bon escient et
  la phrase remplacée existe mot pour mot dans `SPEC-06` ligne 241.

---

## 1. Recalculs

### 1.1 Vecteurs automatiques (les deux sont exacts)

Configuration commune : `N = (0, 0, −1)`, œil sur l'axe donc `V = (0, 0, −1)`,
`Light.Direction = (0, 0, 1)` donc `L = (0, 0, −1)`, `H = (0, 0, −1)`, lumière blanche d'intensité
1, `Amb = 0,1812`. Les trois conventions se recoupent : `Light.Direction` est bien le **sens de
propagation** (`REQ-SCN-025`), la lumière vient donc du côté de la caméra et frappe la face avant
du pavé de plein fouet ; la caméra par défaut regarde vers les `z` croissants (`AMD-043` §3), donc
`V = (0, 0, −1)` est bien « l'œil sur l'axe ». Pas de piège de signe.

| Grandeur | Amendement | Recalcul | Verdict |
|---|---|---|---|
| `nl`, `nv`, `nh`, `lh` | 1 | 1, 1, 1, 1 | ✔ |
| `F90 = 0,5 + 2 lh² ρ` | (non publié) | 2,48 | ✔ |
| `Fd` | 1 | 1,0000000 (les deux termes en `(1 − 1)⁵` s'annulent) | ✔ |
| `Vis` | 0,25 | 0,2499988 (le `10⁻⁵` du dénominateur) → 0,25 | ✔ |
| `D` | 0,3314 | 0,96059601 / (π × 0,96059601²) = 0,3313671 | ✔ |
| `Spec` | 0,2603 | 0,2499988 × 0,3313671 × π × 1 = 0,2602538 | ✔ |
| `F` | 0,04 | 0,04 (`(1 − lh)⁵ = 0`) | ✔ |
| `Spec × F` | 0,01041 | 0,01041015 | ✔ |
| `0,96 × (Amb + 1)` | 1,1812 (× 0,96) | 0,96 × 1,1812 = 1,1339520 | ✔ |
| Vecteur 1, `c = 0,2` | 0,2372 | 0,2372010 | ✔ (bien dans ±0,001) |
| Vecteur 2, `s = 0,2` | `c·0,96·(0,1812 + 0,2) + 0,2·0,01041` | 0,96 × 0,3812 = 0,3659520 ; 0,2 × 0,01041015 = 0,00208203 | ✔ |
| Vecteur 2, `c = 0,2` | 0,0753 | 0,0752724 | ✔ |

Les deux vecteurs sont donc **justes et mutuellement cohérents** : le second ne diffère du premier
que par le facteur d'ombre, appliqué **au seul terme direct** (diffus et spéculaire), l'ambiante
restant intacte — ce qui est exactement ce que demande `REQ-SCN-031` (« l'ambiante n'est pas
affectée »). Le choix `α = ρ² = 0,9801` est cohérent avec le reste (`α² = 0,96059601` apparaît bien
deux fois dans `D`).

### 1.2 Vecteur `[ROM]` « Punch-Out!! » : la valeur est juste, l'énoncé ne permet pas de la retrouver

Le profil `punchout.bytes` d'`assets-neutral` porte, dans ses réglages embarqués,
`Light.Direction = (1, 1, 2)` et `Light.Color = −1` (blanc) ; mais son script `scene` écrit, dans
la scène de combat (celle où le backdrop est affiché) :
`Backdrop.Color = Palette.FrameColor(0)`, `Backdrop.Enable = true`, `Setting.LocalCam = true`,
`Light.Color = Color(1, 0.8, 0.8)`. « La lumière rosée du profil » du §1 vient donc d'un
**script**, pas des réglages embarqués, et « vue de dessus » est le mode `LocalCam`/`TopDownCam`
de `REQ-SCN-009`, lui aussi forcé par le script. Le pavé mesure alors
(248 ; 256 ; 64) centré en `C + (0 ; −68 ; 31)`.

Avec ces valeurs, l'albédo `0x0078FC` (entrée 0 de la palette, `SPEC-04` §4.1, ligne `$10`) vaut en
linéaire `(0 ; 0,187821 ; 0,973445)`, et je retrouve :

| Surface mesurée | Plongée de la caméra | `E` = linéarisé | `E` = brut (0,8) |
|---|---|---|---|
| **face avant** du pavé, `N = (0, 0, −1)` | 0° | (24,3 ; 99,9 ; 208,7) | (24,3 ; 110,7 ; 229,5) |
| idem | 15° | (24,7 ; 100,0 ; 208,8) | (24,7 ; 110,7 ; 229,5) |
| idem | **30°** | **(25,6 ; 100,1 ; 208,8)** | (25,6 ; 110,9 ; 229,6) |
| idem | 45° | (27,4 ; 100,4 ; 209,0) | (27,4 ; 111,2 ; 229,8) |
| idem | 60° | (30,4 ; 101,4 ; 210,2) | (30,4 ; 112,3 ; 231,2) |
| **face du dessus**, `N = (0, 1, 0)` | quelconque | (0 ; 50,6 ; 114,3) | (0 ; 50,6 ; 114,3) |

La valeur publiée (26 ; 100 ; 209) ± 3 est donc **exacte**, et même robuste : elle est tenue de 0°
à environ 55° de plongée (au-delà, le rouge sort de la tolérance par le haut). C'est une bonne
nouvelle pour un modèle qui dépend de `V`. Mais elle n'est retrouvable qu'en **devinant** trois
choses que l'énoncé ne dit pas : la face mesurée, la couleur de lumière en vigueur, et la
linéarisation de celle-ci. Les deux lectures erronées ne sont pas marginales : la face horizontale
— lecture littérale de « le sol du ring » — donne (0 ; 51 ; 114), et `E` non linéarisé donne
(26 ; 111 ; 230). D'où 49-1 et 49-2.

### 1.3 Cohérence interne des formules (bornes, clamp, unités, linéaire/sRGB)

- **Un seul espace de couleur**, annoncé (« en linéaire ») et tenu : `c` est explicitement converti
  (« `Backdrop.Color` converti en linéaire »), `Amb` est donné en linéaire par `REQ-SCN-027`
  (0,1812, que je retrouve à 0,18115 depuis sRGB 118). Il **manque seulement** le chemin de retour
  (écrêtage + encodage sRGB), que `REQ-SCN-048` héritait de `REQ-SCN-028` et que le remplacement
  coupe → 49-4. La mention « ±0,001 **avant écrêtage** » prouve que l'auteur a l'écrêtage en tête.
- **Unités** : les cinq facteurs (`Fd`, `Vis`, `D`, `Spec`, `F`) sont sans dimension ; `Amb` et `E`
  sont des éclairements relatifs ; `c` est une réflectance. L'addition `Amb + s·E·Fd·nl` est donc
  homogène, et le `+ s·E·Spec·F` final, hors de la parenthèse `c · 0,96 · (…)`, est bien un terme
  **additif non teinté par l'albédo** — ce que §1 annonce (« un reflet spéculaire faible, de la
  couleur de la lumière »). Cohérent.
- **Clamps** : les quatre produits scalaires sont bornés explicitement ; `nv = |N·V| ∈ [0, 1]`,
  `nl`, `nh`, `lh ∈ [0, 1]`, donc `(1 − x)⁵ ∈ [0, 1]` partout. `F90 ∈ [0,5 ; 2,48]`, donc
  `Fd ∈ [0,25 ; 6,1504]` : le diffus n'est **pas** simplement « réduit de 4 % », il peut être
  multiplié par 6 en incidence rasante → 49-7. `F ∈ [0,04 ; 1]`.
- **Pas de division par zéro** aux valeurs fixées : le dénominateur de `D` vaut
  `nh²(α² − 1) + 1 ≥ α² = 0,9606 > 0` pour tout `nh ∈ [0, 1]` ; celui de `Vis` ne s'annule qu'en
  `nl = nv = 0`, où `Spec` est de toute façon multiplié par `nl = 0`. Les deux garde-fous
  (`10⁻⁵`, `max(0, …)`) sont donc **numériquement inertes** → 49-8.
- **Seul vrai trou de définition** : `H = (L + V)` normalisé n'existe pas quand `L = −V` → 49-8.
- **`s` multiplie aussi le spéculaire**, ce qui est le bon comportement au sens de `REQ-SCN-031`
  (« le terme de lumière **directe** DOIT être multiplié par 0,2 ») : le spéculaire est de la
  lumière directe. Cohérent, et le vecteur 2 le vérifie.
- **Applicabilité** : hors `E` (49-1) et l'encodage de sortie (49-4), un implémenteur peut écrire
  la formule sans aucune décision libre — j'ai pu le faire moi-même depuis le seul texte, et
  retomber sur tous les chiffres publiés. C'est le principal point positif du document.

### 1.4 Effet réel du changement, mesuré

Sur l'exemple même du §1 (face avant du pavé de « Punch-Out!! », plongée 30°) :

| Modèle | Rouge | Vert | Bleu |
|---|---|---|---|
| `REQ-SCN-028` (diffus, texte actuel) | 0 | 99,7 | 211,7 |
| `REQ-SCN-075` (ce document) | 25,6 | 100,1 | 208,8 |

Le vert ne bouge pas, le bleu perd 3 niveaux, et **le rouge passe de 0 à 26**. L'écart dominant
n'est donc pas le « 4 % plus sombre » mais l'apparition de la teinte de la lumière là où l'albédo
n'en a aucune → 49-7.

Sur le critère d'acceptation de `REQ-SCN-048` (panneau **noir**, lumière par défaut `(1, −1, 1)`,
blanche, intensité 1, vue frontale) : `nl = 0,5774`, `Spec × F = 0,007534`, soit **sRGB ≈ 21** hors
ombre et **≈ 5** dans l'ombre. Deux conséquences, toutes deux à écrire (49-3) : le panneau n'est
plus noir, et — à l'inverse — les « ombres qui se dessinent dessus » deviennent **enfin
observables**, alors qu'avec le modèle diffus un panneau d'albédo nul reste noir partout et le
critère actuel est strictement invérifiable.

---

## 2. Points

### 49-1 (UNCLEAR, **bloquant**) — `E` n'est pas défini : sRGB ou linéaire ?

`REQ-SCN-075` pose « `E` = couleur de la lumière × intensité (`REQ-SCN-025`) », alors que
`REQ-SCN-028` — dont le backdrop est précisément détaché par ce remplacement — écrit
« `lumière_lin` la couleur de la lumière multipliée par l'intensité, **convertie en linéaire** »,
avec ses Notes (`DEC-06-Q2` : l'intensité subit la même courbe, ≈ intensité^2,2) et `AMD-021`
`R2-015` (linéariser séparément couleur et intensité est accepté, ≤ 1 niveau). Le préambule « en
linéaire » peut se lire comme couvrant ce point, ou comme ne décrivant que l'espace de calcul.

Ce n'est pas une subtilité : les deux lectures diffèrent de **11 niveaux sur le vert et 21 sur le
bleu** sur le vecteur `[ROM]` lui-même (§1.2), et `Light.Intensity` est un réglage **utilisateur**
réglable de 0,50 à 1,50 (`REQ-SCN-026`), donc l'écart est visible en usage normal. Les deux
vecteurs automatiques (lumière **blanche**, intensité **1**) ne départagent pas les lectures : ils
valent 1 dans les deux cas. Seul le vecteur `[ROM]` tranche — mais il n'est pas calculable depuis
l'amendement (49-2), donc l'ambiguïté n'est levée pour personne.

*Correction* : ajouter, après la définition de `E`, « `E` est la même grandeur que `lumière_lin` de
`REQ-SCN-028` : couleur de la lumière multipliée par l'intensité, **convertie en linéaire**
(`DEC-06-Q2` ; la linéarisation séparée de la couleur et de l'intensité reste acceptée,
`AMD-021` `R2-015`) ».

### 49-2 (UNTESTABLE, **bloquant**) — le critère `[ROM]` n'est pas reproductible tel qu'il est écrit

« « Punch-Out!! », vue de dessus, profil du jeu : le sol du ring (backdrop de couleur NES
`0x0078FC`) éclairé, mesuré en sRGB, vaut (26 ; 100 ; 209) ±3. » La valeur est **juste** (§1.2),
mais le nouveau modèle dépend de `V` — contrairement au modèle diffus qu'il remplace — et l'énoncé
ne fixe ni la face mesurée, ni la lumière en vigueur, ni l'orientation de la caméra :

1. « le sol du ring » se lit naturellement comme une surface **horizontale**, donc la face du
   dessus du pavé, `N = (0, 1, 0)` : avec `Light.Direction = (1, 1, 2)` (le profil), `N·L < 0`,
   cette face ne reçoit que l'ambiante et vaut **(0 ; 51 ; 114)**. Un vérificateur qui mesure là
   conclut à une non-conformité de 150 niveaux. La valeur publiée est celle de la **face avant**,
   `N = (0, 0, −1)` (le pavé fait 64 unités de profondeur et descend jusqu'à `y = −76` : c'est sa
   face avant qui tient l'écran sous une caméra plongeante).
2. la couleur de lumière utilisée n'est pas celle des réglages embarqués (blanc) mais celle
   qu'écrit le **script** du profil, `Color(1, 0.8, 0.8)` ; le §1 dit « la lumière rosée du
   profil » sans dire qu'elle vient d'un script, et le critère ne la donne pas du tout.
3. « vue de dessus » désigne en fait le mode `Setting.LocalCam`/`TopDownCam` de `REQ-SCN-009`,
   forcé par le même script — et non un angle. L'angle de plongée restant libre, il faut dire
   jusqu'où la valeur tient : d'après mon recalcul, de 0° à ≈ 55° (au-delà, le rouge dépasse 29).

*Correction* : réécrire le critère ainsi : « « Punch-Out!! », scène de combat (le script du profil
y met `Backdrop.Enable = true`, `Backdrop.Color = Palette.FrameColor(0)` = NES `0x0078FC`,
`Light.Color = Color(1, 0.8, 0.8)`, `Setting.LocalCam = true`), lumière du profil
`Light.Direction = (1, 1, 2)`, intensité 1, ombres actives, hors ombre : la **face avant** du pavé
(`N = (0, 0, −1)`), mesurée en sRGB, vaut (26 ; 100 ; 209) ±3 pour toute plongée de caméra de 0° à
45°. » Si l'on veut garder une formulation visuelle, la faire suivre de « c'est la grande surface
bleue qui tient le sol du ring à l'image ».

### 49-3 (INCONSISTENT, **bloquant**) — le critère d'acceptation de `REQ-SCN-048` n'est pas amendé, et il change

Le §3 ne touche qu'une phrase de `REQ-SCN-048` ; son critère d'acceptation reste :
« `Backdrop.Color = Color(0,0,0)` … fait apparaître … un **panneau noir** … sur lequel se dessinent
les ombres des couches avant. » Sous `REQ-SCN-075`, un albédo nul ne donne plus du noir : il reste
le terme `s · E · Spec · F`, soit **sRGB ≈ 21** hors ombre et **≈ 5** dans l'ombre avec la lumière
par défaut (§1.4). Comme `Backdrop.Color` vaut **noir par défaut** (`REQ-SCN-048`, `REQ-LUA-090`),
tout backdrop activé sans couleur explicite devient un panneau gris foncé visible sur un fond noir
(`REQ-SCN-046`) : c'est un changement observable que ni le §1 ni le §2 n'annoncent.

Symétriquement, c'est `REQ-SCN-075` qui **rend le critère vérifiable** : avec le modèle diffus
actuel, un panneau d'albédo nul est noir partout et les ombres ne peuvent pas « s'y dessiner ».
L'amendement corrige donc un critère invérifiable sans le dire.

*Correction* : ajouter au §3 un point 3 : « `SPEC-06` `REQ-SCN-048`, critère d'acceptation :
« un panneau noir » devient « un panneau presque noir (sRGB ≈ 21 hors ombre, ≈ 5 dans l'ombre, avec
la lumière par défaut et l'intensité 1, `REQ-SCN-075`) », le reste inchangé » ; et une phrase au §1
ou dans une Note du §2 : « un albédo nul n'est plus rendu noir : le reflet spéculaire subsiste, le
backdrop par défaut apparaît gris très foncé. »

### 49-4 (UNCLEAR, non bloquant) — l'encodage de sortie n'est plus hérité, et l'alpha n'est pas tranché

`REQ-SCN-048` renvoyait à `REQ-SCN-028`, qui finit par « Le résultat est encodé en sRGB à
l'affichage, **saturé à 1**, sans opérateur de tone mapping ». Le remplacement supprime ce renvoi
pour le backdrop, et `REQ-SCN-035` ne rattrape pas le cas (elle ne vise que « les couleurs de
palette et de textures couleur » — `Backdrop.Color` n'est ni l'une ni l'autre). Le texte dit bien
« en linéaire » et « avant écrêtage », donc l'intention est claire, mais la clause normative manque.
Par ailleurs le type `Color` de Lua a **quatre** composantes (`SPEC-05` §4.2, réels 0–1) et
`REQ-LUA-090` parle de « couleur noire **opaque** » : comme `REQ-SCN-075` redonne tout le calcul,
autant y dire que l'alpha de `Backdrop.Color` est ignoré (le pavé est rendu avec les opaques,
`REQ-SCN-058`).

*Correction* : ajouter en fin de §2 : « Le résultat est écrêté à 1 et encodé en sRGB à l'affichage,
sans tone mapping, comme en `REQ-SCN-028` ; la composante alpha de `Backdrop.Color` est ignorée (le
pavé est opaque, `REQ-SCN-058`). »

### 49-5 (UNCLEAR, non bloquant) — `N` n'est pas défini, et la clause de normale d'`AMD-043` ne s'attache plus

`REQ-SCN-075` dit « pour chaque point de normale unitaire `N` » sans dire d'où vient `N`. Pour un
pavé à six faces planes, la réponse est évidente — sauf quand la **courbure** d'`AMD-043` est
active : `REQ-SCN-068` met explicitement « le fond (backdrop) » dans le périmètre de la courbure,
subdivise ses faces « en au moins 32 × 32 cellules », et amende la parenthèse de `REQ-SCN-028` pour
dire que « la normale `N` utilisée pour l'ombrage DEVRAIT être celle du maillage **transportée par
la courbure** ». Cette clause est accrochée à `REQ-SCN-028` ; en détachant le backdrop de
`REQ-SCN-028`, `AMD-049` la détache aussi du backdrop, qui est pourtant le seul objet nommé par les
deux textes. Chaîne de renvois cassée, sans contradiction frontale.

*Correction* : « `N` est la normale du maillage du pavé, transportée par la courbure quand elle est
active (`AMD-043` `REQ-SCN-068`) ».

### 49-6 (UNTESTABLE, non bloquant) — les deux critères automatiques ne sont pas mesurables sur une capture

Les deux critères sont exprimés en **linéaire avant écrêtage**, avec une tolérance de ±0,001. À
0,2372, ±0,001 vaut **0,26 niveau** d'une sortie 8 bits ; à 0,0753, 0,50 niveau. Aucune capture
sRGB ne permet donc de les vérifier : il faut soit une implémentation de référence du calcul, soit
une lecture en virgule flottante. Il manque aussi comment **obtenir** `c = 0,2` : `Backdrop.Color`
est une couleur sRGB (`REQ-SCN-048`) de composantes réelles, donc il faut écrire
`Color(0.484529, …)` pour `c = 0,2` exactement (une valeur 8 bits 124 donne `c = 0,20156`, hors
tolérance).

*Correction* : garder les valeurs linéaires comme référence du calcul, et ajouter la forme
mesurable : « en sortie sRGB, pour `Backdrop.Color = Color(0.484529, 0.484529, 0.484529)`
(soit `c = 0,2`), la capture vaut (134, 134, 134) ±1 hors ombre et (78, 78, 78) ±1 dans l'ombre ».
(Valeurs recalculées : 133,69 et 77,54.) Préciser aussi que le critère se vérifie en harnais, sur
un pavé dont une face est frontale.

### 49-7 (rédactionnel, non bloquant) — le §1 décrit mal l'écart qu'il motive

Deux imprécisions dans « la part diffuse est réduite de 4 % et un reflet spéculaire faible … s'y
ajoute … 4 % plus sombre et légèrement teinté par la lumière rosée » :

1. le diffus n'est pas « réduit de 4 % » en général : il est multiplié par `0,96 · Fd` avec
   `Fd ∈ [0,25 ; 6,15]` (§1.3) ; `Fd = 1` seulement près de l'incidence normale. En incidence
   rasante le backdrop devient **plus clair**, pas plus sombre ;
2. sur l'exemple cité, l'écart mesuré n'est pas celui-là (§1.4) : le vert est **identique**, le
   bleu perd 3 niveaux, et le rouge passe de **0 à 26**. Le « légèrement » sous-estime donc le seul
   écart réellement visible, et le « 4 % plus sombre » décrit un canal qui ne change pas.

Le §1 n'est pas normatif, mais c'est lui qui justifie l'amendement, et un relecteur qui tente de
recouper les chiffres se trompe de cible.

*Correction* : « Par rapport à un éclairage diffus, la part diffuse est multipliée par 0,96 (y
compris l'ambiante) et modulée par un facteur de rétrodiffusion qui vaut 1 près de l'incidence
normale et croît en incidence rasante ; s'y ajoute un reflet spéculaire faible, de la couleur de la
lumière, indépendant de l'albédo. L'écart est visible quand le backdrop occupe une grande surface :
sur le sol du ring de « Punch-Out!! » en vue de dessus, le bleu perd 3 niveaux et le rouge, nul
dans l'albédo, monte à 26 sous la lumière rosée du script du profil. »

### 49-8 (rédactionnel, non bloquant) — garde-fous numériques inertes, et un cas dégénéré non traité

- Le `+ 10⁻⁵` au dénominateur de `Vis` et le `max(0, …)` de `Spec` ne déterminent **aucun résultat
  observable** : à `α = 0,9801`, le dénominateur ne s'annule qu'en `nl = nv = 0`, où `Spec` est
  multiplié par `nl = 0`, et `Vis · D · π > 0` toujours (§1.3). `SPEC-GUIDE` ne demande que les
  constantes « qui déterminent un résultat observable, avec leur justification » : ces deux-là sont
  des protections de calcul reprises de la source publique, à justifier comme telles ou à retirer.
  Le `10⁻⁵` a au moins l'intérêt de rendre la formule totale ; le `max(0, …)` n'en a aucun ici.
- En revanche un cas **n'est pas** couvert : `H = (L + V)` normalisé n'est pas défini quand
  `L = −V` (l'œil regarde exactement dans le sens de propagation de la lumière). La configuration
  n'apparaît que sur une face invisible ou exactement de profil (`N·V = −N·L`, donc `nl = 0` dès
  que la face est vue), mais une implémentation naïve y produit un NaN.

*Correction* : remplacer les deux garde-fous par une Note (« le `10⁻⁵` et le `max(0, …)` ne
protègent que des cas dégénérés et n'ont pas d'effet mesurable ») et ajouter une puce : « si
`L + V = 0`, `H` est indéfini et `Spec` vaut 0 ».

### 49-9 (UNTESTABLE, non bloquant) — « pas de reflet de l'environnement » n'a aucun critère

« Il n'y a pas de reflet de l'environnement (skybox) » est la clause la plus facile à rater en
implémentation — c'est le terme que les modèles de ce genre ajoutent par défaut — et c'est la seule
du §2 sans critère d'acceptation. Les trois critères existants tournent tous avec la skybox par
défaut, qui n'affiche rien (`REQ-SCN-046`, `REQ-SCN-049`) : aucun n'exerce la clause.

*Correction* : ajouter « Critère d'acceptation (auto) : la couleur du backdrop est identique, au
bruit de quantification près, pour les quatre valeurs du réglage Skybox (`REQ-SCN-049`), en
particulier Gradient et Fantasy. »

### 49-10 (INCONSISTENT, non bloquant) — pas de clause de priorité, alors que P2 remplace du P1

`REQ-SCN-075` est **P2** et remplace une clause **P1** de `REQ-SCN-048`. `AMD-043`, `AMD-044`,
`AMD-045` et `AMD-046` portent pour ce cas une ligne « Priorités » explicite (règle T-1 :
« si une exigence P2 qui remplace une clause P1 n'est pas retenue, la clause d'origine reste
valable »), et `R2-review-AMD-039-045-v1.1` relève cette configuration comme devant être annoncée.
Sans cette ligne, la matrice de traçabilité ne sait pas quoi faire d'un `REQ-SCN-075` non
implémenté : exiger le diffus, ou rien.

*Correction* : ajouter sous l'en-tête : « Priorités : `REQ-SCN-075` est P2 ; si elle n'est pas
retenue, la clause d'origine de `REQ-SCN-048` (éclairage diffus, `REQ-SCN-028`) reste valable
(T-1). »

### 49-11 (rédactionnel, non bloquant) — le §3 point 2 (« Index ») est ambigu, et le placement cache le modèle

« Index : `REQ-SCN-075` ajoutée à `SPEC-06` §3.13 » mélange deux choses : le **placement** de la
nouvelle exigence dans `SPEC-06` et une éventuelle ligne dans `SPEC-v1.2-INDEX.md`. Or ce dernier
est marqué « **Documentation seulement** », figé au 2026-10-21, et son §2 ne connaît aucun
amendement après `AMD-023` : y insérer une ligne `REQ-SCN-075` seule, sans ligne `AMD-049` au §2 ni
rattrapage d'`AMD-024` à `AMD-048`, produirait un index incohérent. Par ailleurs §3.13 (« Couleur
de fond et backdrop ») est le bon endroit pour l'objet, mais un lecteur qui cherche les modèles
d'éclairage lit §3.9 (« Lumière ») et §3.11 (« Matériaux et ombrage des tiles »), où rien ne signale
qu'il existe désormais un **second** modèle.

*Correction* : (a) préciser « `SPEC-06` §3.13 : `REQ-SCN-075` insérée après `REQ-SCN-048` » et
traiter `SPEC-v1.2-INDEX.md` à part (ou renoncer, comme `AMD-043` à `AMD-048`) ; (b) ajouter un
renvoi d'une ligne à `REQ-SCN-028` : « Le backdrop suit `REQ-SCN-075` et non cette exigence. »

### 49-12 (rédactionnel, non bloquant) — citations et justification de la rugosité

- L'en-tête cite « phrase « éclairé de façon diffuse (`REQ-SCN-028`) » », le §2 cite « éclairé de
  façon diffuse (`REQ-SCN-028`, albédo = `Backdrop.Color`) » et le §3 cite « **Il DOIT être** éclairé
  de façon diffuse (…) ». Seule la troisième est le texte exact de `SPEC-06` ligne 241 : harmoniser
  sur elle les trois occurrences.
- Le remplacement proposé garde « (albédo = `Backdrop.Color`) » alors que `REQ-SCN-075` définit déjà
  `c` ; la redondance est inoffensive mais la parenthèse peut tomber.
- `ρ = 0,99` n'est justifié que par « presque mat » au §1. Pour qu'un tiers puisse recouper la
  constante avec la source publique, donner aussi le paramètre d'auteur correspondant
  (lissage = 1 − ρ = 0,01) et, éventuellement, nommer le fichier d'en-tête des shaders intégrés de
  Unity où le modèle est publié : c'est une référence à une **source publique**, que `SPEC-GUIDE`
  encourage (au même titre que les renvois à `third_party/3DNes_Engine`, qui ne contient aucun
  shader et ne couvre donc pas ce point).

---

## 3. Contrôles menés sans résultat (pour mémoire)

- **Fuite** : aucune. Pas de code, pas de pseudo-code suivant une structure ligne à ligne, pas de
  nom de classe, méthode, champ ou variable privés, pas de référence à un fichier source du
  programme d'origine, pas de numéro de ligne, pas d'architecture interne. Les seuls noms propres
  sont `Backdrop.*`, `Light.*`, `RecvShadow` (contrat Lua, visible depuis les données conservées,
  `REQ-PERS-008`), « Standard » et « Unity » (source publique, citée comme telle), « Enhanced » et
  « 3dSen 2.0 » (nom de style public, déjà fixé par `AMD-040` : 0 = Classic, 1 = Enhanced — la
  mention est **cohérente**). La notation `nl`/`nv`/`nh`/`lh`/`Vis`/`D`/`F`/`F90` est celle de la
  source publique, pas celle de l'original.
- **Numéro `REQ`** : `REQ-SCN-075` libre (voir §0) ; aucun identifiant réutilisé ; aucune exigence
  retirée sans mention.
- **Cohérence avec les exigences citées** : `REQ-SCN-025` (sens de propagation, `L = −Direction`,
  blanc, intensité 1) ✔ ; `REQ-SCN-026` (intensité = réglage utilisateur 0,50–1,50) — le seul
  impact est 49-1 ; `REQ-SCN-027` (ambiante 0,1812 linéaire, uniforme, indépendante de la skybox et
  du profil) ✔, recoupée à 0,18115 depuis sRGB 118 ; `REQ-SCN-028` reste intacte **pour les tiles**
  et pour le HUD (`REQ-SCN-029`, `REQ-SCN-044`), ce qui est voulu ; `REQ-SCN-030` (ombres dures,
  réglage Ombres) ✔, la condition « si `RecvShadow` et les ombres sont actifs » reprend exactement
  `REQ-SCN-048` ; `REQ-SCN-031` (facteur 0,2, ambiante non affectée) ✔ ; `REQ-SCN-048` : le reste de
  l'exigence est explicitement préservé (« pavé, ombres reçues, pas d'ombre projetée, pas de
  découpe, propriétés Lua »), seul son **critère** est oublié (49-3).
- **Vecteurs et captures existants** : §5.2 de `SPEC-06` ne contient que de l'ombrage de **tile** —
  aucun vecteur à corriger. Les captures de référence §5.4 (R1 à R7) portent toutes sur « Super
  Mario Bros », sans backdrop : rien à reprendre. `SPEC-12` et `TRACEABILITY-CHECK-P6.md` ne citent
  ni `REQ-SCN-048` ni le backdrop. `REQ-SCN-058` (le backdrop est dessiné avec les opaques) reste
  vrai. `REQ-LUA-090` (remise à zéro en fin de partie) n'est pas touchée.
- **Amendements récents** : pas de conflit avec `AMD-012` (transparence par objet : le backdrop est
  opaque), `AMD-046` (`REQ-SCN-074` ne parle du backdrop que pour le laisser sous `REQ-SCN-068`),
  `AMD-047` (marges négatives : le backdrop n'est pas découpé), `AMD-048` (géométrie). Le seul
  accrochage est celui d'`AMD-043` relevé en 49-5.
- **Observation hors périmètre, pour information** : `REQ-SCN-054` dit déjà que les modèles TV sont
  « éclairés par la lumière de la scène (`REQ-SCN-028`, **avec spécularité selon leur matériau**) »,
  spécularité rendue « informative » par `AMD-021` `R2-014`. Il y a donc maintenant trois régimes
  dans `SPEC-06` : tiles et HUD strictement diffus, backdrop en Standard chiffré, TV en spéculaire
  informatif. Ce n'est pas une incohérence (trois objets différents, trois exigences distinctes),
  mais si la comparaison R5 a montré que le boîtier TV suit le même modèle que le backdrop, c'est
  le moment de le dire dans un amendement séparé — pas dans celui-ci.
- **Portabilité libretro** (`SPEC-GUIDE` §Contraintes transverses) : le modèle est un calcul local
  par fragment, sans état global, sans dépendance de plateforme et sans ressource externe (pas de
  reflet d'environnement). Aucune contrainte violée.

---

## 4. Conclusion (gate R2)

- **Étanchéité : passée sans réserve.** `AMD-049` ne contient aucune fuite. Le modèle est décrit
  comme un résultat mathématique, sourcé sur une publication publique (sources des shaders intégrés
  de Unity) et non sur le programme d'origine, et il se suffit à lui-même comme le §1 l'annonce.
  Aucune tentative d'accès au côté sale n'a été nécessaire ni faite.
- **Exactitude : les deux vecteurs automatiques sont justes**, terme par terme (§1.1), et
  mutuellement cohérents. Le vecteur `[ROM]` est juste aussi (§1.2) et plus robuste qu'on pouvait
  le craindre pour un modèle dépendant du point de vue (tenu de 0° à ≈ 55° de plongée) — mais il
  n'est pas reproductible depuis l'énoncé.
- **Cohérence interne : bonne.** Un seul espace de couleur, tous les produits scalaires clampés,
  des grandeurs homogènes, aucune division par zéro aux constantes fixées, le facteur d'ombre
  appliqué au bon terme. **Un implémenteur peut appliquer les formules sans deviner, à deux
  exceptions près** : la définition de `E` (49-1) et l'encodage de sortie (49-4). C'est le cœur de
  ce rapport.
- **Verdict : accepté avec réserves**, 12 points, dont **aucun bloquant pour l'étanchéité** et
  **trois à traiter avant la mise en œuvre** :
  **49-1** (dire si `E` est linéarisé — 11 et 21 niveaux d'écart, et les vecteurs automatiques ne
  tranchent pas), **49-2** (le critère `[ROM]` doit nommer la face mesurée, la lumière du script et
  la plage de plongée, faute de quoi la lecture littérale « le sol du ring » donne (0 ; 51 ; 114)),
  **49-3** (amender aussi le critère d'acceptation de `REQ-SCN-048` : un backdrop d'albédo nul
  n'est plus noir mais sRGB ≈ 21, et le backdrop par défaut est noir).
  49-1 et 49-3 sont des corrections d'une à deux phrases ; 49-2 demande de reprendre un critère.
- **Ensuite, par ordre d'importance** : **49-4** (écrêtage et encodage sRGB, plus l'alpha — sans
  lui l'exigence ne dit pas ce qu'elle produit à l'écran), **49-5** (normale, et raccrochage de la
  clause de courbure d'`AMD-043`), **49-6** (critères automatiques non mesurables en l'état),
  **49-9** (la clause « pas de reflet d'environnement » n'est exercée par aucun critère),
  **49-10** (clause de priorité T-1), puis **49-7**, **49-8**, **49-11** et **49-12**,
  rédactionnels.
- **Aucune exigence gelée n'a besoin d'être rouverte** au-delà de ce que l'amendement déclare déjà
  (la phrase d'éclairage de `REQ-SCN-048`), plus — et c'est l'ajout demandé par 49-3 — le **critère
  d'acceptation** de `REQ-SCN-048`, plus le renvoi d'une ligne à ajouter à `REQ-SCN-028` (49-11).
  `REQ-SCN-025`, `-026`, `-027`, `-030`, `-031`, `-035`, `-058`, les vecteurs de §5.2 et les
  captures de §5.4 sont **inchangés**, ce que mes recalculs confirment.
- **Impact de mise en œuvre, pour mémoire** : le changement est localisé au matériau du backdrop ;
  les tiles, le HUD et le boîtier TV ne bougent pas. Les deux vecteurs automatiques sont
  directement transposables en tests une fois 49-6 appliqué. Rien n'a été modifié côté `impl`.

Le fichier `AMD-049-eclairage-backdrop.md` n'a pas été modifié par cette relecture ; rien n'a été
commité.
