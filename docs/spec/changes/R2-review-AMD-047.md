# R2 — Relecture d'`AMD-047` (marges de clipping négatives, v1.0)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-30 | Version du rapport : 1.0
Objet : relire `spec/changes/AMD-047-clipping-negatif.md` **en entier** et y chercher toute fuite
(code, pseudo-code, nom interne, algorithme recopié), toute incohérence avec `SPEC-09`
(`REQ-PERS-003`, `-007`, `-081`, §4.1.5), `SPEC-06` (§2, `REQ-SCN-003`, `-007`, `-008`, `-036`,
`-039`, `-053`), `SPEC-04` `REQ-E3D-002`, `SPEC-04B` `REQ-E3D-410` ou les amendements récents
(`AMD-043`, `AMD-045`, `AMD-046`), tout critère intestable, tout identifiant `REQ` déjà pris et
toute référence cassée. **Recalcul** du vecteur caméra (T′, E, R) et du critère de tile en x = −8.

Entrées relues : `spec/changes/AMD-047-clipping-negatif.md` (v1.0 du 2026-10-30, texte sous revue,
**non modifié**) ; `SPEC-GUIDE.md` ; `SPEC-09` §2, §3.1 (`REQ-PERS-001` à `-017`), §3.2
(`REQ-PERS-020` à `-025`), §3.9 (`REQ-PERS-080` à `-086`), §4.1 (légende et §4.1.1 à §4.1.6), §4.2,
§4.5.1, §5.6, §6 (Q-2), historique ; `SPEC-06` §1, §2, §3.1 à §3.4, §3.11 à §3.15
(`REQ-SCN-001/002/003/004/005/007/008/009/010/011/012/036/039/044/045/046/048/053/054`) ;
`SPEC-04` §2 (grilles de fond et de sprites) et `REQ-E3D-002` à `-005` ; `SPEC-04B` §2 et
`REQ-E3D-400/401/410` ; `SPEC-03` `REQ-PROF-025/033/040/041`, §4.6 (champ 4) ; `SPEC-05`
`REQ-LUA-045/089` et tables §4 ; `SPEC-12` (aucune occurrence du clipping) ; `DECISIONS-R0.md`
(DEC-R2-T1, DEC-R2-T2, DEC-09-Q2) ; `AMD-001` (T-1), `AMD-011`, `AMD-021` (R2-014),
**`AMD-031` (retrait du boîtier TV)**, `AMD-043` v1.2, `AMD-045` v1.2, `AMD-046` v1.1 ;
`R2-review-AMD-039-045` et sa v1.1, `R2-review-AMD-046` et sa v1.1 (continuité de forme) ;
`SPEC-v1.2-INDEX.md` ; `journal/JOURNAL.md` ; `impl/docs/verify/round1/v2/R4-v2-4-bend.md` ;
`impl/src/Zax.Settings/SettingsCatalogue.cs` (impact de mise en œuvre).
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.

Méthode : ouverture de chacune des exigences citées par l'amendement, puis de celles qui dépendent
des quatre marges sans être citées ; contrôle de l'unicité de `REQ-PERS-087` dans tout `spec/` et
sur **les 42 références** du dépôt `impl` (`git grep` sur `refs/heads` + `refs/remotes`, contrôle
positif avec `REQ-PERS-086`) ; **recalcul indépendant** de T′, E, R, P\* et du filtre de tiles à
partir du cadre de référence de `SPEC-06` §2 / `REQ-E3D-410` et des formules de `REQ-SCN-007`,
`REQ-SCN-008` et `REQ-E3D-002`, la méthode de calcul de P\* étant d'abord validée sur le vecteur
publié de `REQ-SCN-008` ; relecture de chaque critère d'acceptation comme un tiers qui ne
connaîtrait que la spec, en cherchant quel test automatique il rendrait exigible.

Catégories utilisées, comme dans les rapports précédents : **LEAK**, **INCONSISTENT**,
**UNTESTABLE**, **UNCLEAR**, auxquelles s'ajoutent les points **rédactionnels**.

---

## 0. Verdict

| Document | Verdict | Points | Réserves bloquantes |
|---|---|---|---|
| **AMD-047** — marges de clipping négatives (v1.0) | **Accepté avec réserves** | 12 (47-1 à 47-12) | **aucune pour l'étanchéité** ; 3 avant mise en œuvre : **47-1, 47-2, 47-3** |

**Étanchéité : acquise. Aucun point LEAK.** Le fond de la décision est sain et les deux
recalculs demandés **tombent juste au chiffre près** : T′ = (128 ; 80 ; 0), E = (128 ;
−149,937 ; −132,755), R = [−128, 384] × [−80, 240], et le critère de tile en x = −8 discrimine
bien les deux jeux de marges. Ce qui manque n'est pas de l'étanchéité mais de l'hygiène de
renvois : l'amendement « précise » une exigence **retirée du produit** (`REQ-SCN-053`, `AMD-031`),
laisse dans `SPEC-09` §4.1.5 une phrase qui dit exactement le contraire de la nouvelle exigence, et
appuie un critère d'acceptation sur un format de fichier qui n'existe pas.

---

## 1. Points bloquants avant mise en œuvre

### 47-1 — (INCONSISTENT) Le tiret du « tunnel du boîtier TV » vise une exigence retirée par `AMD-031` — **bloquant**

`AMD-047` annonce en en-tête qu'il « précise SPEC-06 … `REQ-SCN-053` », et son §2 exige :

> « le tunnel du boîtier TV (REQ-SCN-053) suit ses formules, recalculées d'après R ; »

Or **`AMD-031`** (2026-10-22, normatif, R0) retire le boîtier TV *de bout en bout* : « plus de
réglage, plus de modèle, plus de rendu du boîtier ni de son tunnel ». Le retrait est fait, non
seulement décidé : le journal du 2026-10-23 (lot REM-TV) liste la suppression de
`src/Zax.Render/{TvBox,TvModel}.cs`, de l'entrée de catalogue `scene.tv_box` et des tests associés,
et `impl/docs/verify/round1/v2/R4-v2-4-bend.md` §5 avait déjà relevé la même référence morte dans
`AMD-043` (« La troisième exclusion de `REQ-SCN-068` est sans objet : `AMD-031` a retiré le boîtier
TV »). Le tiret demande donc d'entretenir une géométrie qui n'existe plus.

Deuxième symptôme, qui montre que le renvoi n'a pas été exercé : **les formules de `REQ-SCN-053` ne
survivent pas à des marges négatives**. Avec le jeu de marges de l'amendement lui-même
(−128 ; −80 ; −128 ; 0), les cinq pavés du tunnel donnent

| Pavé | Intervalle publié | Valeur obtenue | État |
|---|---|---|---|
| gauche | x ∈ [−8, c0] | [−8, −128] | **inversé** |
| droite | x ∈ [256 − c2, 264] | [384, 264] | **inversé** |
| bas | y ∈ [−4, c1] | [−4, −80] | **inversé** |
| haut | y ∈ [240 − c3, 244] | [240, 244] | correct |
| fond | [0, 256] × [0, 240] | inchangé | correct |

« Suit ses formules » n'a donc pas de résultat défini : une implémentation qui normalise les bornes
poserait trois panneaux opaques **à l'intérieur** de la zone que la marge négative vient de
découvrir — l'effet exactement inverse de l'intention. De même `REQ-SCN-054` (repère TV d'origine
`C + ((c0 − c2)/256, (c1 − c3)/240, −16)`, échelles `(256 − c0 − c2)/256 = 2,0` et
`(240 − c1 − c3)/240 ≈ 1,33`) dépend des quatre marges et n'est pas même mentionné.

**Correction proposée** : supprimer le tiret et retirer `REQ-SCN-053` de la liste de l'en-tête.
Si l'on veut garder la trace du raisonnement, une Note suffit : « Le boîtier TV et son tunnel
(`REQ-SCN-053`, `REQ-SCN-054`) ont été retirés par `AMD-031` ; les marges n'ont plus d'effet
décoratif. » Aucune autre exigence de `SPEC-06` ne consomme les marges hors de celles déjà citées
(contrôle fait sur `REQ-SCN-001/003/007/008/009/036/039/044/045/046/048`).
**Bloquant : oui** (référence cassée dans une exigence P1, et seule clause de l'amendement dont le
résultat est indéfini).

### 47-2 — (INCONSISTENT) `SPEC-09` §4.1.5 continue d'affirmer le bornage après l'amendement — **bloquant**

Le §3 d'`AMD-047` n'amende que la colonne « Domaine » des quatre lignes du tableau. Mais le
paragraphe de prose qui suit immédiatement ce tableau, dans la **même sous-section**, se termine
par :

> « Le domaine [0 ; 248] des marges verticales correspond à la hauteur de la grille de conversion ;
> une valeur hors domaine est bornée (REQ-PERS-003). »

Après `AMD-047`, `SPEC-09` §4.1.5 contiendrait donc, à trois lignes d'intervalle, « réel fini » et
« une valeur hors domaine est bornée ». Un implémenteur qui lit la section en entier doit deviner
laquelle des deux l'emporte — c'est précisément le défaut que `REQ-PERS-001` (« type, défaut et
domaine sont égaux à ceux du tableau ») rend testable, et donc visible.

**Correction proposée** : ajouter au §3 de l'amendement la phrase de remplacement, par exemple :
« Dernière phrase du paragraphe de §4.1.5 remplacée par : *Les quatre marges acceptent tout réel
fini, négatif compris (`REQ-PERS-087`) ; `[0 ; 256]` et `[0 ; 248]` ne sont plus des bornes mais
les dimensions de référence de l'image et de la grille de conversion.* »
**Bloquant : oui** (contradiction interne à la section amendée).

### 47-3 — (UNTESTABLE) Le « fichier de réglages de profil » du premier critère n'existe pas — **bloquant**

Premier critère d'acceptation (auto) :

> « Un fichier de réglages de profil contenant `clip.left = -128` est relu à −128. »

Aucun format de ce nom n'existe dans la spec. Les deux porteurs possibles se disqualifient :

1. **`settings.json`** (`SPEC-09` §4.5.1) est indexé par clé stable, donc c'est bien lui qui
   contiendrait la clé `clip.left` — mais `REQ-PERS-020` dit que les réglages de portée P « NE
   DOIVENT PAS être nécessaires au démarrage : leur présence éventuelle dans le fichier **PEUT être
   ignorée** », et `clip.left` est de portée P. Une implémentation parfaitement conforme échoue
   donc au critère.
2. **Le profil** (`SPEC-03` §4.6) porte bien la valeur, mais sous la forme du champ 4 (`Clipping`,
   message `Vec4`) appliqué par `REQ-PROF-041` — il ne contient aucune clé textuelle `clip.left`.

**Correction proposée** : viser le profil, qui est le vrai chemin d'entrée des marges négatives :
« Un profil dont le champ 4 du message de réglages (`SPEC-03` §4.6, `Clipping`) vaut
(−128 ; −80 ; −128 ; 0) donne, après chargement (`REQ-PROF-041`), `Setting.Clipping[0] = −128` et
`Setting.Clipping[1] = −80`. » Si l'on tient à couvrir aussi `settings.json`, il faut le dire
explicitement et lever la permission de `REQ-PERS-020` pour ces quatre clés — ce qui serait un
amendement supplémentaire, et n'est pas nécessaire au besoin exposé au §1.
**Bloquant : oui** (critère d'une exigence P1 qui contredit une autre exigence P1).

---

## 2. Points non bloquants

### 47-4 — (INCONSISTENT) « au-delà de l'image NES il n'y a simplement pas de tile à garder » est faux, et l'amendement se contredit trois lignes plus bas

Le §2 affirme que la conservation des tiles et la découpe « gardent ce qui tombe dans R ; au-delà
de l'image NES il n'y a simplement pas de tile à garder ». Son propre troisième critère exige
pourtant qu'une tile **en x = −8** soit conservée et rendue : x ∈ [−8, 0] est hors de l'image NES
(`x ∈ [0, 256]`, `REQ-E3D-410` point 2). La zone capturée par le cœur déborde de l'image de huit
pixels sur chaque bord :

- fond : grille de 33 × 31 tiles (`SPEC-04` §2), soit x écran ∈ [−8, 248] et y écran ∈ [−8, 239] ;
  une tile en x = −8 occupe la scène x ∈ [−8, 0], une tile en y écran = −8 occupe y ∈ [248, 256]
  (`REQ-E3D-410` point 2 : « déborde au-dessus de y = 248 »), une tile en y écran = 239 descend
  jusqu'à y = 1, sous le bas de l'image ;
- sprites : x ∈ [0, 255], y < 240 (`REQ-E3D-003`), donc jusqu'à x = 263 en scène.

Conséquence utile pour l'implémenteur, et qui manque : une marge négative ne découvre **au plus que
les ~8 unités de bordure de chaque côté** ; au-delà, R ne contient effectivement rien.

**Correction proposée** : remplacer la clause par « au-delà de la zone capturée par le cœur
(`SPEC-04` §2 : fond 33 × 31 tiles, x écran ∈ [−8, 248], y écran ∈ [−8, 239] ; sprites
x ∈ [0, 255], y < 240) il n'y a pas de tile à garder : une marge négative ne découvre que les huit
unités de bordure partiellement défilées de chaque côté ».
**Bloquant : non** (mais la phrase actuelle fait douter du troisième critère).

### 47-5 — (UNCLEAR) L'« exception » à `REQ-PERS-003` est inutile et trop large

Le §2 pose : « `REQ-PERS-003` ne s'applique pas à ces quatre marges, sauf pour une valeur non finie
(NaN, ±∞), remplacée par la valeur par défaut de la marge **(D)** ».

Or `REQ-PERS-003` appliquée **sans exception** au nouveau domaine produit déjà exactement ce
comportement : « une valeur hors domaine … DOIT être ramenée dans le domaine selon la règle de la
colonne “Domaine” ; **à défaut de règle, la valeur par défaut DOIT être utilisée** ». Avec le
domaine « réel fini », seules NaN et ±∞ sont hors domaine, aucune règle de bornage n'est donnée,
donc le défaut s'applique. Le seul changement nécessaire est donc celui du §3 (colonne
« Domaine »).

L'exception, elle, a un effet de bord non voulu : `REQ-PERS-022` renvoie à `REQ-PERS-003` pour
remplacer une valeur isolée invalide **« (type ou domaine) »** dans `settings.json`. Écrire que
`REQ-PERS-003` « ne s'applique pas » à ces quatre clés les prive aussi du contrôle de **type**
(`"clip.left": "abc"`).

**Correction proposée** : supprimer la clause d'exception et retirer « et `REQ-PERS-003`
(exception) » de l'en-tête ; porter la règle dans la colonne « Domaine » du §3 :
« réel fini ; non fini (NaN, ±∞) → valeur par défaut (`REQ-PERS-087`) ». Le comportement voulu est
inchangé, et le contrôle de type reste acquis.
**Bloquant : non.**

### 47-6 — (UNCLEAR) L'énumération des chemins d'écriture oublie `Clipping.target`

`REQ-PERS-087` énumère « `clip.left`, `clip.bottom`, `clip.right`, `clip.top` ;
`Setting.Clipping[0]` à `[3]`, `x`, `y`, `z`, `w` ». La table §4.2 de `SPEC-09` expose un cinquième
chemin, en **lecture et écriture** :

> `| Clipping | target | L, É | les quatre marges sous forme de vecteur 4 |`

Un script qui écrit `Clipping.target` doit manifestement bénéficier de la même règle. Laisser ce
membre hors de l'énumération, dans une exigence rédigée comme une liste close, invite à le borner.

**Correction proposée** : ajouter `Clipping.target` à l'énumération (« …, `x`, `y`, `z`, `w`,
`target` »).
**Bloquant : non.**

### 47-7 — (UNTESTABLE) Le critère de la tile en x = −8 est juste, mais incomplet pour un test automatique

**Recalcul** à partir de `SPEC-09` §4.1.5 et de `SPEC-04` `REQ-E3D-002` (filtre identique dans les
deux documents : `x + 8 > c0`, `x ≤ 256 − c2`, `y + 8 > c3`, `y ≤ 248 − c1`) :

| Marges | x + 8 > c0 | x ≤ 256 − c2 | Verdict |
|---|---|---|---|
| (−16 ; 8,1 ; 8,1 ; 0,1) | 0 > −16 **vrai** | −8 ≤ 247,9 vrai | **conservée** |
| (0,1 ; 8,1 ; 8,1 ; 0,1) | 0 > 0,1 **faux** | — | **écartée** |

Le critère est donc **exact** et discriminant, et sa seconde moitié coïncide avec le critère déjà
publié de `REQ-E3D-002` (« tile de fond en x = −8 écartée, en x = −7 conservée »). Avec le domaine
actuel, −16 serait ramené à 0 et donnerait aussi « écartée » : le critère teste bien la nouveauté.
Deux manques pour un critère marqué « (auto) » :

1. **y n'est pas donné** : les deux conditions verticales doivent être satisfaites pour que la tile
   survive, et le rendu la découpe en plus à R = [−16 ; 247,9] × [8,1 ; 239,9] (`REQ-SCN-036`).
2. « conservée **et rendue** » mélange deux exigences distinctes (`REQ-E3D-002` pour la
   conservation, `REQ-SCN-036` pour le fragment). Les nommer rend le test opposable.

**Correction proposée** : « marges (−16 ; 8,1 ; 8,1 ; 0,1) : une tile de fond émise en
(x, y) = (−8, 100) est conservée par `REQ-E3D-002` et aucun de ses fragments n'est éliminé par
`REQ-SCN-036` (elle occupe la scène x ∈ [−8, 0] ⊂ R = [−16 ; 247,9]) ; avec
(0,1 ; 8,1 ; 8,1 ; 0,1) la même tile est écartée par `REQ-E3D-002`. »
**Bloquant : non.**

### 47-8 — (UNCLEAR) Le critère caméra est exact ; trois précisions le rendraient opposable

**Recalcul** à partir de `REQ-SCN-007` et du cadre de référence de `SPEC-06` §2 (`C = (128, 120, 0)`,
repère main gauche, Y vers le haut), consignes rx = −60°, ry = rz = 0, d = 265,51,
marges (−128 ; −80 ; −128 ; 0) :

- **étape 2** : T′ = T + ((c0 − c2)/2, (c1 − c3)/2, 0) = (128 + 0 ; 120 − 40 ; 0) = **(128 ; 80 ; 0)** ✔
- **étape 3** : f = (cos rx·sin ry, −sin rx, cos rx·cos ry) = (0 ; 0,8660254 ; 0,5) ;
  E = T′ − d·f = (128 ; 80 − 229,937 ; −132,755) = **(128 ; −149,937 ; −132,755)** ✔
  (l'amendement annonce (128,0 ; −149,9 ; −132,8) ±0,5)
- **rectangle visible** : R = [c0, 256 − c2] × [c1, 240 − c3] = **[−128, 384] × [−80, 240]** ✔

Les trois valeurs publiées sont donc confirmées, signes compris (contrôle de la convention sur le
vecteur publié de `REQ-SCN-007` : rx = 30°, ry = −20°, d = 189,65, clipping (0 ; 7,9 ; 0 ; 0,1)
⇒ E = (184,17 ; 218,73 ; −154,34), conforme au (184,2 ; 218,7 ; −154,3) de la spec).
`AMD-045` ne gêne pas : `REQ-SCN-072` ne remplace que l'**étape 4** de `REQ-SCN-007`, les étapes 2
et 3 utilisées ici sont intactes. Restent trois précisions :

1. **d = 265,51 n'est la distance de réinitialisation d'aucun FOV atteignable** : ni par
   `REQ-SCN-012` (104/atan(FOV_rad/2) ⇒ FOV ≈ 47,4°), ni par `AMD-045` `REQ-SCN-069`
   (132,7964/tan(FOV/2) ⇒ FOV ≈ 53,2°), alors que le FOV va de 45° à 95° par pas de 5°. C'est
   légitime (d est une consigne libre bornée à [0, 512], `REQ-SCN-011`), mais il faut l'écrire,
   sinon le lecteur cherche le FOV correspondant.
2. `REQ-SCN-007` calcule à partir des valeurs **lissées** (`REQ-SCN-019`) : un test « (auto) » doit
   préciser « après stabilisation du lissage » (ou vitesses de lissage nulles).
3. **Le critère s'arrête avant l'observable.** Ce que l'amendement veut corriger — « la cible de la
   caméra est 40 unités trop haut » — se voit par l'orientation, donc par **P\*** (`REQ-SCN-008`),
   qui dépend des quatre coins de R et change beaucoup ici. Recalcul avec la méthode de
   `REQ-SCN-008` (validée au préalable sur le vecteur publié de cette exigence,
   (123,36 ; 124,56) contre (123,4 ; 124,6) ±0,6) : A = (−128, −80), B = (384, −80),
   Cc = (−128, 240), D = (384, 240) ; |A−E| = |B−E| = 296,73, |Cc−E| = |D−E| = 485,0 ;
   M1 = (128, −80), M2 = (128, 240), M3 = (66,34 ; 41,47), M4 = (189,66 ; 41,47) ⇒
   **P\* ≈ (128 ; 41,5) ±0,6**. Aucun cas dégénéré : E n'est confondu avec aucun sommet et les deux
   droites se coupent franchement (l'une verticale, l'autre horizontale).

**Correction proposée** : « caméra TPV, **lissage stabilisé**, consignes rx = −60°, ry = 0, rz = 0,
distance de consigne d = 265,51 (valeur libre, bornes de `REQ-SCN-011`), cible par défaut
C = (128, 120, 0), marges (−128 ; −80 ; −128 ; 0) : T′ = (128 ; 80 ; 0),
E ≈ (128,0 ; −149,9 ; −132,8) ±0,5, **P\* ≈ (128 ; 41,5) ±0,6 (`REQ-SCN-008`)** et
R = [−128, 384] × [−80, 240]. »
**Bloquant : non.**

### 47-9 — (INCONSISTENT) Le §1 dit « la vue s'élargit », ce que la spec ne produit pas

« la vue s'élargit et la caméra se recentre vers l'avant du ring » attribue aux marges un effet de
**champ** qu'aucune exigence ne leur donne : la distance de cadrage ne dépend pas de R
(`REQ-SCN-012` : d₀ = 104/atan(FOV_rad/2) ; `AMD-045` `REQ-SCN-069` : d₀ = 132,7964/tan(FOV/2) —
deux constantes du seul FOV), et le FOV est un réglage utilisateur. Élargir R n'écarte donc pas la
caméra d'un pouce. Plus précisément, avec c0 = c2 = −128 la composante horizontale de T′ ne bouge
pas du tout ((c0 − c2)/2 = 0).

Ce que les marges négatives font réellement, et qui suffit au besoin :

- **c1 = −80** descend la cible de 40 unités (T′.y = 80 au lieu de 120) — c'est le « 40 unités trop
  haut » du §1, exact ;
- **c0 = c2 = −128** change la forme de R, donc le point visé P\* (`REQ-SCN-008`) : avec les
  mêmes consignes caméra, P\* passe de **(128 ; 20,4)** pour des marges (0 ; −80 ; 0 ; 0) à
  **(128 ; 41,5)** pour (−128 ; −80 ; −128 ; 0) — 21 unités d'écart de visée ;
- la découpe (`REQ-SCN-036`) cesse de rogner les bordures : au plus 8 unités par côté (47-4).

**Correction proposée** : remplacer « la vue s'élargit et la caméra se recentre » par « la cible
effective et le point visé descendent vers l'avant du ring, et la découpe cesse de rogner les
bordures ». Ni le champ de vision ni la distance ne changent.
**Bloquant : non** (justification seulement ; l'exigence du §2, elle, est correcte).

### 47-10 — (UNCLEAR) « R vide : non spécifié » laisse une exigence P1 sans comportement

« une marge pour laquelle R serait vide (c0 + c2 ≥ 256 ou c1 + c3 ≥ 240) n'est pas spécifiée ». Les
deux inégalités sont **arithmétiquement justes** (R est vide ssi c0 ≥ 256 − c2 et ssi
c1 ≥ 240 − c3), et le cas n'est pas nouveau : l'ancien domaine [0 ; 256] l'autorisait déjà
(c0 = c2 = 200). Mais `REQ-PERS-087` est P1 et le cas est atteignable par un script
(`Clipping[0] = 1e30`), donc « non spécifié » devient une invitation à planter — alors que
`SPEC-GUIDE.md` demande bornes et erreurs.

**Correction proposée** : ajouter « l'image rendue PEUT alors être vide ; l'implémentation NE DOIT
PAS lever d'erreur ni cesser de rendre les autres éléments (HUD, fond, skybox) ».
**Bloquant : non.**

### 47-11 — (rédactionnel) Renvois normatifs et justification du classement (D)

1. `DEC-R2-T2` fait de **`SPEC-04B` `REQ-E3D-410` point 4** la définition **unique** du rectangle
   visible ; `SPEC-06` §2 le rappelle, `SPEC-09` §4.1.5 aussi. L'amendement cite « (SPEC-06 §2) »
   pour R et « (SPEC-09 §4.1.5, SPEC-04 REQ-E3D-002) » pour la conservation des tiles. Citer
   d'abord la source normative (`REQ-E3D-410` point 4 ; `REQ-E3D-002`) évite d'amender un rappel.
2. Le marquage **(D)** de la règle NaN est conforme à l'usage du dépôt (« divergence volontaire
   assumée », légende de `SPEC-v1.2-INDEX.md` §3), mais rien ne dit de quoi on diverge. Une
   demi-phrase rend la divergence auditable : « le comportement du programme d'origine sur une
   valeur non finie n'est pas connu et n'est pas reproduit ».
**Bloquant : non.**

### 47-12 — (rédactionnel) `REQ-SCN-003` est annoncée « précisée » sans texte de remplacement

L'en-tête annonce une précision de `REQ-SCN-003`, `-007`, `-008`, `-036` et `-053`, mais seul le §3
donne des lignes de remplacement, et seulement pour `SPEC-09`. Pour `REQ-SCN-003`, une phrase est
pourtant périmée : « Valeurs typiques des profils : 0,1, 7,9, 8,1, 16,1, 24,1 » — l'amendement
documente deux valeurs de plus (−128 et −80).

**Correction proposée** : « `REQ-SCN-003`, liste des valeurs typiques complétée : *…, 24,1, et des
valeurs négatives (−128, −80) pour quelques profils (`REQ-PERS-087`)*. » Contrôlé par ailleurs :
`REQ-SCN-007` (étape 2), `REQ-SCN-008`, `REQ-SCN-036` et **`REQ-SCN-039`** (R recalculé à chaque
frame depuis le clipping courant) s'appliquent tels quels à des marges négatives et n'ont besoin
d'aucun texte de remplacement — ce que l'amendement peut dire en une ligne plutôt que de les
annoncer « précisés ».
**Bloquant : non.**

---

## 3. Contrôles effectués sans constat

| Contrôle | Résultat |
|---|---|
| **Étanchéité** (code, pseudo-code, noms internes, algorithme recopié, renvoi aux sources d'origine) | **Aucun constat.** L'amendement ne contient aucune ligne de code ni de pseudo-code, aucun nom de classe, méthode, champ ou variable interne, aucun numéro de ligne ni nom de fichier source. Les seuls identifiants cités (`clip.left`…, `Setting.Clipping[i]`, `Clipping.x`) sont des **faits d'interopérabilité déjà publiés** (clés stables `REQ-PERS-080`, API Lua `REQ-PERS-008`), explicitement autorisés par `SPEC-GUIDE.md`. Les faits venus du côté sale — « le programme d'origine ne borne pas ces marges », les valeurs (−128, −80) du profil de « Punch-Out!! » — sont observables en boîte noire et de même nature que les inventaires déjà présents dans `SPEC-09` §5.6. |
| **`REQ-PERS-087` libre** | **Oui.** Dernier numéro attribué : `REQ-PERS-086`. Aucune occurrence dans `spec/` hors `AMD-047`, ni dans `impl/docs/`, ni sur **aucune des 42 références** du dépôt `impl` (`git grep` sur `refs/heads` + `refs/remotes` ; contrôle positif : `REQ-PERS-086` ressort sur 5 références). Numérotation `PERS` correcte : l'exigence porte sur le domaine d'un réglage. |
| **Priorité P1** | Justifiée au sens de `SPEC-GUIDE.md` (« sans elle, un profil ne fonctionne pas ») : le profil de « Punch-Out!! » est inexploitable avec les marges bornées. |
| **`REQ-PERS-081`** (domaine énumérable) | L'affirmation du §3 (« pas d'UI, pas de libretro : la discrétisation de `REQ-PERS-081` ne les concerne pas ») est **exacte** : `REQ-PERS-081` ne vise que les réglages de portée **U**, les quatre marges sont de portée **P**, et `REQ-PERS-085` interdit d'exposer les réglages P en core options. Colonnes « UI » et « libretro » du §4.1.5 déjà à « — ». |
| **`REQ-PERS-007`** (exposition Lua) | Compatible sans changement : `Clipping[i]` borne l'**indice** (0 à 3), jamais la valeur. `REQ-LUA-045` ne lève d'erreur d'indice que pour `Setting.Layer`, les listes, shapes et patterns. |
| **`REQ-PERS-002`, `-005`, `REQ-PROF-040/041`** | Inchangés : les défauts (0,1 ; 8,1 ; 8,1 ; 0,1) restent dans le domaine, et le chemin profil → réglages est le même. |
| **`SPEC-03`** | Le champ 4 est un `Vec4` de flottants (§4.6) : −128 et −80 sont représentables exactement et l'aller-retour est fidèle. Aucune clause de bornage dans `REQ-PROF-025/033/040/041` à amender. |
| **`AMD-043` / `AMD-046`** (courbure) | **Aucun conflit.** `REQ-SCN-068` impose d'évaluer `REQ-SCN-036` et `REQ-SCN-008` sur les positions **avant** courbure : les marges négatives agissent donc dans le même repère, inchangé. La fenêtre de visée de `REQ-INP-124` (|x − 128| ≤ 144·s, |y − 128| ≤ 136·s) est définie à partir de l'image NES et non de R : elle n'est pas affectée. |
| **`AMD-045`** (caméra) | **Aucun conflit** : `REQ-SCN-072` ne remplace que l'étape 4 de `REQ-SCN-007` ; les étapes 2 et 3 utilisées par `AMD-047` sont intactes. `REQ-SCN-069` ne fait pas dépendre d₀ de R (voir 47-9). |
| **`SPEC-12`** | Aucune occurrence du clipping : rien à amender côté acceptation. |
| **Impact de mise en œuvre** | Localisé : `impl/src/Zax.Settings/SettingsCatalogue.cs:689-705` pose aujourd'hui `Domain = SettingDomain.Range(0d, maximum)` pour les quatre clés. Le reste de la chaîne (profil, Lua, rendu) ne borne pas. |

---

## 4. Conclusion — gate R2

**Oui : `AMD-047` est publiable au côté propre, et prêt pour la mise en œuvre une fois 47-1, 47-2
et 47-3 appliqués.**

- **Étanchéité : acquise.** Aucun point **LEAK** : pas de code, pas de pseudo-code, aucun nom
  interne de l'original, aucun renvoi à sa structure ni à ses sources. Les trois faits venus du
  côté sale (l'original ne borne pas les marges, le profil de « Punch-Out!! » écrit −128/−80, la
  conséquence visible à l'écran) sont observables en boîte noire et autorisés par `SPEC-GUIDE.md`.
  Rien dans ce rapport ne justifie de retenir la publication.
- **Le fond est juste.** Les deux recalculs demandés confirment l'amendement **au chiffre près** :
  T′ = (128 ; 80 ; 0), E = (128 ; −149,937 ; −132,755) et R = [−128, 384] × [−80, 240] d'une part ;
  la tile en x = −8 conservée avec c0 = −16 et écartée avec c0 = 0,1 d'autre part. Aucune formule
  de `REQ-SCN-007`, `-008`, `-036`, `-039`, `REQ-E3D-002` ou `REQ-E3D-410` ne dégénère sur des
  marges négatives — P\* compris, que j'ai calculé pour le cas de l'amendement (≈ (128 ; 41,5)).
- **Verdict : accepté avec réserves**, 12 points, dont **aucun bloquant pour l'étanchéité** et
  **trois à traiter avant la mise en œuvre**, parce qu'en l'état un implémenteur devrait deviner :
  **47-1** (l'amendement « précise » `REQ-SCN-053`, retiré du produit par `AMD-031`, et la seule
  clause dont le résultat soit indéfini sur des marges négatives), **47-2** (`SPEC-09` §4.1.5
  affirmerait encore « une valeur hors domaine est bornée » dans la section même qu'on amende) et
  **47-3** (critère appuyé sur un format de fichier inexistant, qui contredit `REQ-PERS-020`).
  Les trois corrections sont **mécaniques** : une suppression, une phrase de remplacement, une
  reformulation de critère. Elles n'appellent pas de nouveau tour de décision du propriétaire.
- **Ensuite, par ordre d'importance** : **47-4** et **47-7** (ils rendent le critère de tile
  incontestable et lèvent la contradiction interne), **47-8** (P\* et lissage dans le critère
  caméra), **47-5** (l'exception à `REQ-PERS-003` est inutile et coûte le contrôle de type),
  **47-6** (`Clipping.target`), **47-10** (pas de plantage quand R est vide), puis **47-9**,
  **47-11** et **47-12**, rédactionnels.
- **Aucune exigence gelée n'a besoin d'être rouverte** au-delà de ce que les corrections demandent
  de déclarer : la colonne « Domaine » et la dernière phrase de `SPEC-09` §4.1.5, et la liste des
  valeurs typiques de `REQ-SCN-003`. Aucune ne renverse un comportement choisi par le propriétaire.

Le fichier `AMD-047-clipping-negatif.md` n'a pas été modifié par cette relecture ; rien n'a été
commité.
