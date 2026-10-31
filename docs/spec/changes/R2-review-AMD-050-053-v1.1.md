# R2 — Relecture de la version 1.1 d'AMD-050, AMD-051, AMD-052 et AMD-053

Rôle R2 (filtre anti-fuite, côté PROPRE) | 2026-10-31 | Gate R2 (seconde passe)

Objet : vérifier, point par point, que la v1.1 des quatre amendements lève chacun des 45 points de
`R2-review-AMD-050-053.md` (v1.0 de la relecture), puis relire les quatre fichiers **en entier** à la
recherche de fuites, d'incohérences, de critères intestables ou de références REQ cassées
**introduites par les corrections**.

Documents relus, en entier, dans leur version 1.1 du 2026-10-31 :

- `AMD-050-mode-2d-fond-transparent.md` (REQ-CORE-048 amendée, REQ-CORE-049, REQ-SCN-076, vecteur V11) ;
- `AMD-051-robustesse-conversion.md` (REQ-E3D-644, REQ-E3D-645 **retirée**, REQ-E3D-646, REQ-LOOP-073, REQ-PERS-088) ;
- `AMD-052-cadence-file-gpu-msaa.md` (REQ-LOOP-074, -075, -077, REQ-PERS-089, REQ-SCN-077 **retirée**) ;
- `AMD-053-reprise-favoris-ligne-de-commande.md` (REQ-LOOP-076, REQ-UI-210 à 213).

Référence appliquée : `spec/SPEC-GUIDE.md`.

Sources consultées pour cette seconde passe : `spec/SPEC-GUIDE` ; `spec/SPEC-02` (§3.5 complet
`REQ-CORE-030` à `-048`, §4.1, §5.1 à §5.3, §6) ; `spec/SPEC-03` (`REQ-PROF-001` à `-004`, `-021`,
`-022`, `-033`, `-041`, `-050` à `-052`, `-060` à `-064`, §4.1, §4.6, §4.7, §5.5) ; `spec/SPEC-04`
(`REQ-E3D-008`) ; `spec/SPEC-04B` (§3.4 `REQ-E3D-523/524`, §3.5 `REQ-E3D-540` à `-543`, §3.6
`REQ-E3D-560` à `-569`, §3.7 `REQ-E3D-600` à `-603`, `REQ-E3D-453`, `-640/641`) ; `spec/SPEC-06`
(plan du §3, `REQ-SCN-028`, `-031`, `-040`, `-043`, `-046` à `-048`, `-055` à `-058`, `-063`) ;
`spec/SPEC-07` (**en entier** : §2, §3.1 à §3.7, §4, §5, §6, Historique) ; `spec/SPEC-08`
(`REQ-INP-103`) ; `spec/SPEC-09` (`REQ-PERS-005`, `-006`, `-012`, `-017`, `-020` à `-024`, `-033`,
`-034`, `-037`, `-040` à `-042`, `-050` à `-059`, `-071`, `-075`, `-085`, `-086`, §4.1.1, §4.1.9,
§4.5.1 à §4.5.4, §4.6, §5.1) ; `spec/SPEC-10` (`REQ-UI-007`, `-014`, `-030`, `-080` à `-085`,
`-100` à `-105`, §4, §5) ; `spec/SPEC-12` (`REQ-ACC-030`, `-031`, `-041`, `-050`) ;
`spec/SPEC-v1.2-INDEX` ; `spec/changes/` (`AMD-010`, `AMD-026`, `AMD-029`, `AMD-032`, `AMD-035`,
`AMD-036`, `AMD-043`, `AMD-046`, `AMD-050` à `AMD-053`, `R2-review-AMD-050-053`) ; `impl/docs/`
(miroirs, pour le contrôle des numéros libres) ; `journal/JOURNAL.md`.

Aucun accès au côté sale ; le serveur d'outils de rétro-ingénierie présent dans la session n'a pas
été utilisé ; aucune tentative de contournement du pare-feu. **Aucun amendement modifié**, aucun
fichier d'`impl` modifié, rien commité.

---

## 0. Résultat d'ensemble

**Les 45 points de la première relecture sont tous retenus par les auteurs, et tous effectivement
levés : aucun « partiellement levé », aucun « non levé », aucun point écarté.** Les 19 bloquants
sont traités au fond, pas en surface : REQ-E3D-645 est réellement retirée, REQ-SCN-077 réellement
déplacée, les textes amendés d'exigences en vigueur (REQ-CORE-030/048, REQ-LOOP-012/013/052,
REQ-PERS-033/041/042/050/052/055, REQ-UI-030, lignes SPEC-09 §4.1.1/§4.1.9/§4.5.2/§4.5.3/§4.6) sont
donnés mot pour mot ou sous forme de ligne de table, et les critères non exécutables sont remplacés
par des critères chiffrés.

La relecture complète des quatre fichiers fait apparaître **21 points nouveaux**, dont **2
bloquants**, tous deux introduits par une correction et tous deux réparables par une clause :

| Amendement | Points v1.0 | Levés | Points nouveaux | Bloquants résiduels |
|---|---|---|---|---|
| AMD-050 | 9 | 9 | 6 | 0 |
| AMD-051 | 12 | 12 | 5 | **1 (51-N1)** |
| AMD-052 | 13 | 13 | 5 | **1 (52-N1)** |
| AMD-053 | 16 | 16 | 5 | 0 |

**Contrôles transverses.**

**0.a — Numéros REQ : toujours libres.** Les trois numéros créés par la v1.1 — `REQ-CORE-049`
(AMD-050), `REQ-LOOP-077` et `REQ-PERS-089` (AMD-052) — ont été cherchés dans tout `spec/` **et**
dans `impl/docs/` : chacun n'apparaît que dans l'amendement qui le crée. `REQ-CORE-050`,
`REQ-LOOP-078`, `REQ-PERS-090`, `REQ-SCN-078` et `REQ-E3D-647` sont libres : la numérotation reste
sans trou et sans collision. Les deux retraits respectent le SPEC-GUIDE (« une exigence supprimée
garde son numéro avec la mention “retirée” ») : `REQ-E3D-645` est inscrite comme retirée en SPEC-04B
§3.7 et `REQ-SCN-077` en SPEC-06 §3.17, avec dans les deux cas le motif et le renvoi au remplaçant.
**Aucun point.**

**0.b — Étanchéité : acquise pour les quatre, et améliorée.** Le LEAK faible de la v1.0 (51-11,
« file d'envoi au GPU » et « envoyé plus tard au lieu d'être oublié ») a disparu : le §1 d'AMD-051 et
les deux exigences sont désormais énoncés en effets observables (« les tuiles d'un pattern dont la
texture ne peut pas être allouée ne sont simplement pas dessinées », « une tuile apparaît dès que son
maillage et sa texture sont disponibles et n'est jamais perdue »). L'expression « file d'envoi » ne
subsiste nulle part. Les seuls noms propres ajoutés par la v1.1 sont admis par le SPEC-GUIDE :
`video.max_queued_frames` et `favorite` (clé de réglage persisté et champ d'un fichier de la
réimplémentation), `glFenceSync`/`glClientWaitSync` (API publique, documentation publique),
`--atlas-budget` (option du harnais de la réimplémentation), libellés d'interface et options de
ligne de commande utilisateur-visibles. Le masque d'opacité de REQ-CORE-049 et sa disposition
recommandée (32 octets par ligne, bit de poids faible à gauche) sont une **interface de la
réimplémentation**, que le programme d'origine n'a pas : ce n'est pas un fait de rétro-ingénierie, et
la clause « non exigée pour une interface interne .NET » laisse le choix libre. **Aucun point.**

**0.c — Index.** Les quatre amendements portent maintenant, dans leur §« Changements
documentaires », les lignes à écrire dans `spec/SPEC-v1.2-INDEX.md` (point 0.c de la première
relecture). L'écriture elle-même reste du ressort de R1 ; l'index est explicitement
« documentation seulement ». Un seul libellé est inexact (**52-N3**).

**0.d — Forme.** Les quatre fichiers portent désormais une ligne **Historique** et une clause de
**Priorités** relative, y compris AMD-050 et AMD-052 qui en étaient dépourvus. La clause d'AMD-052
(« Si REQ-LOOP-075 n'est pas retenue, la liste de REQ-LOOP-013 reste celle de SPEC-07 ») est le
modèle à suivre ; AMD-050 ne la donne pas pour le point 5 ajouté à REQ-CORE-030 (**50-N5**). Les
quatre portent « Questions — Aucune », ce qui est exact : aucun des 45 points n'a été renvoyé à un
ticket Q, y compris 50-7 (éclairage du rectangle), tranché dans le texte.

---

## 1. AMD-050 v1.1 — Mode 2D : pixels de fond universel transparents

### Verdict : **prêt pour la mise en œuvre** — 9 points sur 9 levés, 0 bloquant résiduel, 6 points nouveaux non bloquants.

La correction est la bonne : l'amendement emporte avec lui la modification de SPEC-02 (option (a) de
50-1), et il le fait proprement — l'information manquante est isolée dans une exigence nouvelle
(REQ-CORE-049, masque d'opacité), l'image de pixels devient exigée (REQ-CORE-048 amendée, P3 → P1),
le contenu du lot de frame est complété (REQ-CORE-030) et un vecteur de test est ajouté. REQ-SCN-076
s'exprime maintenant sur le masque, non plus sur une couleur, et tranche les trois questions ouvertes
(rejet de fragment, indépendance de l'anticrénelage, éclairage).

**Vecteur V11 vérifié à la main, à partir des conventions de SPEC-02 §5.1 et §5.3.** La tile 1 de la
table `$0000` a bien `plan0 = 0x0101…01` et `plan1 = 0xFEFE…FE`, donc, avec la convention de
REQ-CORE-034 (bit 7 = colonne 0), des pixels de valeur **2 en colonnes 0 à 6 et 1 en colonne 7** :
l'affirmation de l'amendement est exacte, et les valeurs 1 et 2 sont bien celles dont les entrées de
palette `$3F01` et `$3F02` sont écrasées au second tiret. PPUMASK `$0A` = bits 1 et 3 (fond rendu,
colonne de gauche affichée, sprites éteints) ✓ ; `$08` = bit 1 à 0 → 8 premiers bits à 0 par ligne ✓ ;
7 680 octets = 256 × 240 / 8 ✓. Le dernier tiret est exact aussi : OAM sprite 0 = (Y `$20`, T `$01`,
A `$00`, X `$10`) avec PPUCTRL `$00` (sprites 8×8, table `$0000`) donne bien un rectangle opaque
x ∈ [16, 23], y ∈ [33, 40] (y = Y + 1, REQ-CORE-042).

### Points de la première relecture

| Point | Statut | Vérification |
|---|---|---|
| **50-1** (BLOQUANT) exigence non implémentable avec une image d'indices | **levé** | Option (a) : `REQ-CORE-049` (P2) ajoute un masque de 256 × 240 **bits**, défini par les valeurs de pixel de fond et de sprite et par les bits 1 à 4 de PPUMASK, transmis avec le lot (`REQ-CORE-030` amendée, points 4 et 5) ; REQ-SCN-076 ne lit plus que ce masque. Le cas indiscernable est devenu un tiret du vecteur V11 (palette entièrement égale à l'entrée 0 → image uniforme, masque à 1). |
| **50-2** priorité incohérente avec `REQ-CORE-048` | **levé** | P3 → **P1**, phrase « Cette image n'est pas exigée » retirée, justification écrite (le mode 2D de REQ-SCN-055 P1 repose dessus). |
| **50-3** confusion avec `maskX`/`maskY` | **levé** | REQ-CORE-049 ne parle que des bits 1 et 2 pour `x < 8` ; les Notes disent explicitement que `maskX`/`maskY` « ne rendent aucun pixel transparent » ; un critère le vérifie. |
| **50-4** rendu entièrement désactivé non discuté | **levé** | Note ajoutée (rectangle entièrement transparent, clignotement assumé) **et** critère (bits 3 et 4 à 0 → capture entièrement égale au fond de scène). Le cas matériel du « fond de palette » est tranché (bit à 0) au lieu d'être écarté — voir **50-N2**. |
| **50-5** critère (auto) inatteignable | **levé** | Mise en condition explicite `scene.use_nes_bg_color` = faux, `Setting.CustomBgColor` = magenta `#FF00FFFF`, avec la Note qui explique pourquoi (REQ-PERS-005 point 3 + REQ-PERS-006, vérifiés). Conforme à la table de `REQ-SCN-046` (Skybox Default, `BgColor` faux → `CustomBgColor`). |
| **50-6** duplication de la clause d'ombre | **levé** | Remplacée par un renvoi : « le rectangle ne projette ni ne reçoit d'ombre (AMD-046 §2, **qui fait foi** ; la transparence n'y change rien) ». |
| **50-7** « à l'éclairage près » sans fondement | **levé** | Tranché dans le texte : « le rectangle est **éclairé comme une tile** (REQ-SCN-028, albédo = couleur du pixel) », avec Note disant que ni REQ-SCN-055, ni Q-022, ni AMD-046 §2 ne le fixaient, et que le modèle Standard d'AMD-049 ne s'applique pas. Cohérent avec `AMD-043` §3, qui amende déjà REQ-SCN-028 pour la normale transportée par la courbure : le cas du rectangle courbé est donc couvert. Conséquence non tirée dans un critère : **50-N3**. |
| **50-8** mélange / anticrénelage | **levé** | « rejet du fragment, dans le passage des objets opaques de REQ-SCN-058 », exclusion explicite de la classe transparente de REQ-SCN-040 (ni tri, ni passage de profondeur, ni mélange alpha), « NE DOIT PAS dépendre de `video.anti_alias` » **et** un critère à trois valeurs de réglage. |
| **50-9** critère [ROM] non mesurable | **levé** | Critère chiffré (pixel central d'une zone de ciel = couleur personnalisée ±2 niveaux sRGB ; barre de statut indice `$0F` restant dessinée) ; la phrase visuelle est conservée comme contrôle. |

### Points nouveaux

**50-N1 — Le masque ignore la limite de 8 sprites par ligne, l'image non : les deux peuvent se
contredire. (non bloquant, le plus important des six)**
`REQ-CORE-048` amendée dit « l'indice de couleur NES **affiché par le PPU** » : l'image suit donc le
matériel, et un neuvième sprite d'une ligne n'y apparaît pas. `REQ-CORE-049` dit « un pixel de
**sprite** de valeur ≠ 0 **y est produit** », sans mentionner la limite matérielle — alors que
`REQ-CORE-041` Notes prévient précisément que « la limite matérielle de 8 sprites par ligne ne
s'applique pas aux **enregistrements** » mais que « le comportement émulé […] suit, lui, le
matériel ». Deux lectures sont donc possibles, et la mauvaise produit en mode 2D un pixel **dessiné
avec la couleur du fond universel** là où le masque dit « opaque » et l'image dit « transparent » :
exactement l'artefact que REQ-SCN-076 veut supprimer.
*Correction proposée* : écrire « un pixel de sprite de valeur ≠ 0 est **affiché** par le PPU à cette
position (mêmes règles que l'image de REQ-CORE-048, limite de 8 sprites par ligne comprise) », et
ajouter un tiret à V11 : neuf sprites alignés sur la même rangée → le masque vaut 0 là où le
neuvième seul serait opaque. Formuler l'invariant qui compte : **un pixel de masque à 1 implique un
pixel d'image différent de l'entrée 0 de la palette**, sauf palette dégénérée (tiret 2 de V11).

**50-N2 — Fidélité du contenu de l'image non bornée : niveaux de gris, accentuation, fond de
palette. (non bloquant)**
`REQ-CORE-047` (P2) ne protège que « les enregistrements » et « la palette transmise » des bits de
niveaux de gris (bit 0 de PPUMASK) et d'accentuation (bits 5 à 7) ; le vecteur V10 parle de « lot
identique » en détaillant « (enregistrements, palette, marges) ». Or `REQ-CORE-030` amendée fait
désormais entrer l'image et le masque **dans le lot**, et `REQ-CORE-048` devient P1 avec un critère
d'identité pixel pour pixel. Trois cas restent sans réponse : l'image applique-t-elle le bit de
niveaux de gris (le matériel, oui : indice ET `$30`) ? les bits d'accentuation (effet d'affichage) ?
et le « fond de palette » (rendu désactivé avec l'adresse PPU dans `$3F00`–`$3FFF`), pour lequel
REQ-CORE-049 Notes tranche le **masque** (bit 0) mais où REQ-CORE-048 impose sans réserve « l'indice
de l'entrée 0 », ce qui est un écart au matériel non étiqueté **(D)** ?
*Correction proposée* : une phrase dans REQ-CORE-048 (« les bits de niveaux de gris et
d'accentuation NE DOIVENT PAS modifier l'image, comme ils ne modifient ni les enregistrements ni la
palette, REQ-CORE-047 ; le cas du fond de palette donne l'indice de l'entrée 0 **(D)** ») et
l'extension explicite du périmètre de V10 à l'image et au masque.

**50-N3 — Le premier critère (auto) de REQ-SCN-076 contredit l'éclairage que la même exigence vient
de fixer. (non bloquant)**
Le critère attend que « les deux pixels de valeur 3 **sont dessinés avec la couleur du fond
universel** », sans tolérance ni condition d'éclairage. Mais le rectangle est maintenant **éclairé**
(REQ-SCN-028 : `albedo × (ambiante + max(0, N·L) × lumière × S)`), tandis que la couleur de fond de
la scène est posée « sans éclairage » (critère de `REQ-SCN-046`). La couleur lue sur un pixel
dessiné ne sera donc égale à la couleur de palette que si le facteur d'éclairage vaut exactement 1 :
le critère n'est pas vérifiable tel quel, alors que le critère voisin (pixels de valeur 0 = magenta
±2 niveaux sRGB) l'est.
*Correction proposée* : soit donner les conditions d'éclairage (par exemple « lumière par défaut,
`light.intensity` = 1, direction par défaut : la couleur lue vaut la couleur du fond universel
éclairée, à ±2 niveaux sRGB »), soit n'exiger du critère que la discriminante utile : « les deux
pixels de valeur 3 **diffèrent** de la couleur de fond de la scène et les deux pixels de valeur 0
l'égalent à ±2 niveaux ».

**50-N4 — Précisions manquantes dans le second critère (auto) et dans V11. (mineur)**
(a) Le second critère pose « Bit 1 de PPUMASK à 0, fond rendu » sans donner la valeur complète de
PPUMASK : si les sprites sont rendus avec le bit 2 à 1, les 8 colonnes de gauche ne sont pas
transparentes. (b) Sa fin — « avec `maskX` = 8 (REQ-CORE-046, **mirroring horizontal**), les 8
colonnes de droite **et les bords haut et bas** restent dessinés » — ne teste rien pour les bords
haut et bas : en mirroring horizontal `maskY` vaut **0** (table de REQ-CORE-046). (c) V11 ne donne
pas le scroll, alors que V1 le fait (« scroll (0, 0) ») et que §5.3 ne le met pas dans ses défauts ;
V11 ne dit pas non plus si les écritures de palette du second tiret restent en vigueur aux tirets
suivants.
*Correction proposée* : donner PPUMASK en hexadécimal comme le fait V11 ; pour (b), utiliser le
mirroring **vertical** (`maskY` = 8, `maskX` = 8 par la dernière clause de REQ-CORE-046) ; ajouter
« scroll (0, 0) » et « chaque tiret repart de la mise en place du §5.1 » à V11.

**50-N5 — `REQ-CORE-030` (P1) reçoit un point qui dépend d'une exigence P2, sans clause de repli.
(mineur)**
Le §3 point 1 ajoute à la liste du lot « 5. le **masque d'opacité** (REQ-CORE-049) » ; REQ-CORE-030
est P1 et REQ-CORE-049 est P2. La clause de priorité de l'en-tête ne traite que REQ-SCN-076. AMD-052
fait l'inverse, et c'est le bon modèle : « Si REQ-LOOP-075 (P2) n'est pas retenue, la liste de
REQ-LOOP-013 reste celle de SPEC-07 ».
*Correction proposée* : ajouter à la clause de Priorités « si REQ-CORE-049 n'est pas retenue, le
point 5 de REQ-CORE-030 n'est pas ajouté ».

**50-N6 — Le critère de `REQ-CORE-048` sort du périmètre de SPEC-02 et invoque une référence non
définie. (mineur)**
« en mode 2D, l'image plaquée sur le rectangle […] est identique, pixel pour pixel, à **l'image NES
de référence** de la même frame » : d'une part cette référence n'est définie nulle part (SPEC-02
n'en parle que sous la forme « un émulateur de référence public », et seulement dans un critère
[ROM], REQ-CORE-039) ; d'autre part le critère d'une exigence du **cœur** se mesure ici à travers le
chemin de rendu de SPEC-06. V11 suffit pour SPEC-02.
*Correction proposée* : garder « vecteur V11 (§5.3) » comme critère, et déplacer la clause du mode
2D en Note, ou l'étiqueter **[ROM]** avec le même renvoi qu'en REQ-CORE-039.

*Remarque de forme, sans point* : l'énumération des objets opaques de `REQ-SCN-058` (« tiles,
backdrop, tunnel et modèles TV ») ne nomme pas le rectangle du mode 2D auquel REQ-SCN-076 renvoie ;
une incise (« le rectangle du mode 2D compte parmi les tiles ») éviterait la question.

### Conclusion AMD-050 (gate R2)

Les neuf points sont levés, l'étanchéité est acquise, les numéros sont libres, l'arithmétique du
vecteur V11 est exacte, et aucune contradiction avec SPEC-02, SPEC-06, AMD-046, AMD-043 ou AMD-049
n'est introduite. **Gate R2 franchi, sans réserve bloquante.** Les six points nouveaux sont des
précisions de rédaction et de critères ; **50-N1** devrait être traité avant la mise en œuvre, car
il décide d'un pixel visible.

---

## 2. AMD-051 v1.1 — Robustesse de la conversion, du chargement et des écritures

### Verdict : **accepté avec une réserve bloquante** — 12 points sur 12 levés, mais **1 bloquant nouveau (51-N1)**, réparable par une clause.

Le retrait de REQ-E3D-645 est fait dans les règles (numéro conservé, mention « retirée », motif
écrit, renvoi au critère du damier de REQ-E3D-524 comme seul vecteur). REQ-E3D-644 et REQ-E3D-646
sont réécrites en effets observables, sans mécanisme interne, avec leurs renvois à REQ-E3D-564,
REQ-E3D-601 et REQ-E3D-603. REQ-PERS-088 ne duplique plus REQ-PERS-021 : elle en étend le périmètre,
et son unique critère ne porte que sur les fichiers nouvellement couverts. REQ-LOOP-073 a son texte
de notification, sa traduction, sa ligne de table SPEC-07 §4 et son état cible. Le nom du harnais est
correct (`Zax.Harness`, AMD-029 §1 ; la première relecture écrivait encore `Sen.Harness`).

### Points de la première relecture

| Point | Statut | Vérification |
|---|---|---|
| **51-1** (BLOQUANT) REQ-E3D-645 contre REQ-E3D-524 | **levé** | REQ-E3D-645 **retirée**, numéro conservé, motif et clause de sortie vide expliqués ; §3 point 2 confirme REQ-E3D-524 inchangée ; §3 point 3 inscrit le retrait en SPEC-04B §3.7 ; la clause de cadrage (maillage de tuile vs maillage agrégé) est tranchée implicitement puisque le seul vecteur redevient celui de REQ-E3D-524. Réserve de **fait** : la priorité citée est fausse (**51-N3**). |
| **51-2** (BLOQUANT) « file d'envoi au GPU » | **levé** | Les deux exigences sont énoncées par leurs effets (texture non allouable → tuiles non dessinées, autres patterns normaux, frame suivante produite ; objet non prêt → dessiné à la première frame rendue qui suit, sans être oublié). L'expression n'apparaît plus, ni dans le §1. |
| **51-3** (BLOQUANT) critère en « N patterns » | **levé** | Critère en **texels** avec l'option de harnais `--atlas-budget <texels>`, Note expliquant pourquoi « N patterns » n'est pas mesurable, et §3 point 8 ajoutant l'option à la liste des moyens de vérification. |
| **51-4** (BLOQUANT) « n'avance plus » indéfini | **levé** *(mais voir 51-N1)* | L'absence d'avancement est définie par `REQ-E3D-600`, avec la Note rappelant les 100 ms de REQ-LOOP-021 point 6 (vérifié dans SPEC-07). La seconde branche de la condition introduit en revanche un défaut nouveau. |
| **51-5** (BLOQUANT) ni texte exact, ni état cible | **levé** | `Loading timed out.` / « Délai de chargement dépassé. » **(D)**, inscrit dans SPEC-07 §4 (format de la table vérifié : Événement / Texte / Type / Durée) et dans la table d'AMD-035 ; état cible **Arrêté** avec jeu courant effacé et indicateur Chargement à faux, comme REQ-LOOP-023, et écran d'accueil de REQ-UI-207. Incohérence de gravité : **51-N2**. |
| **51-6** doublon avec REQ-PERS-021 | **levé** | REQ-PERS-088 s'énonce comme extension de périmètre (« L'écriture atomique de REQ-PERS-021 DOIT s'appliquer de la même façon à … ») ; la Note dit explicitement qu'il n'existe qu'une source normative par comportement ; §3 point 7 confirme REQ-PERS-021 inchangée. |
| **51-7** liste de fichiers incomplète | **levé** | « tout fichier persistant au sens de REQ-PERS-024 — réglages, bibliothèque, index des slots, bindings — et aux fichiers de savestate (REQ-PERS-056) » : conforme mot pour mot à l'énumération de REQ-PERS-024. L'ordre des opérations `*.v<N>.bak` et `*.corrupt-<horodatage>` est écrit, cohérent avec REQ-PERS-022, -024 et -037. |
| **51-8** REQ-E3D-646 déjà exigée, critère contre une recommandation | **levé** | L'exigence renvoie à REQ-E3D-601 « qui fait foi pour la mise en attente » et ne garde que deux clauses ; le retard de 10 frames est déclaré « **cas de test**, pas un comportement visé » ; la constante de 600 frames est justifiée (10 s en NTSC, SPEC-07 §2 — vérifié : cadence nominale 60 images/s par DEC-R2-T3). |
| **51-9** journal non borné | **levé** | « au plus une fois par pattern et au plus 100 fois par partie ; au-delà, une ligne de résumé périodique (au plus une par minute de jeu) », avec la justification (patterns temporaires, REQ-E3D-008, REQ-E3D-453 — tous deux vérifiés). Imprécision résiduelle : **51-N4**. |
| **51-10** tension avec REQ-E3D-603 / REQ-E3D-564 | **levé** | REQ-E3D-644 s'ouvre sur « Précise la phrase “Le comportement au-delà n'est pas spécifié…” de REQ-E3D-564, **sans rouvrir REQ-E3D-603** : cette exigence n'autorise pas à plafonner le nombre d'objets ou de tuiles dessinés par frame » ; la clause de Priorités le redit. |
| **51-11** (LEAK faible) mécanismes au §1 | **levé** | §1 entièrement réécrit en effets observables. |
| **51-12** second critère non assertif | **levé** | « l'état est Arrêté, l'indicateur Chargement est faux, et une commande du modèle de commandes (AMD-010 REQ-UI-205) s'exécute dans la seconde qui suit ». |

### Points nouveaux

**51-N1 — La seconde branche de REQ-LOOP-073 avorte des chargements légitimes : elle casse
REQ-LOOP-021 (P1). (BLOQUANT)**
Le texte déclenche la panne « si l'avancement […] ne change pas pendant 10 s, **ou si une étape de
REQ-LOOP-021 (ou de REQ-LOOP-044) ne se termine pas dans les 10 s** ». Or l'**étape 6** de
REQ-LOOP-021 est précisément l'attente de la génération des maillages jusqu'à 100 %, dont **aucune
exigence ne borne la durée** : `REQ-ACC-031` exclut explicitement la « construction initiale des
meshes » de son budget, `REQ-E3D-600` prévoit une construction de masse pour *tous* les objets de
*tous* les patterns d'un profil (1 854 patterns pour `Contra.bytes`, SPEC-03 §5.6), et `REQ-UI-091`
décrit une barre de progression dont l'intérêt est justement d'accompagner une attente longue. Une
implémentation conforme à la lettre de REQ-LOOP-073 effacerait donc le jeu courant et passerait à
Arrêté au bout de 10 s sur un gros profil ou une machine lente, alors que le chargement progresse
normalement : c'est la négation d'une exigence **P1**. La première branche (avancement figé pendant
10 s) suffit d'ailleurs au critère d'acceptation annoncé, qui fait bloquer l'étape 6 par le harnais.
*Correction proposée* : restreindre la seconde branche aux étapes qui n'exposent **aucune** mesure
d'avancement — « ou si une étape de REQ-LOOP-021 (ou de REQ-LOOP-044) **autre que l'attente des
maillages** ne se termine pas dans les 10 s » — ou, plus simplement, exiger la conjonction : « si
l'étape en cours expose un avancement (REQ-E3D-600), seule l'absence de changement pendant 10 s
compte ». Ajouter en Notes que la durée totale d'une construction de masse n'est pas bornée
(REQ-ACC-031), pour qu'un implémenteur ne refasse pas le raccourci.
*Remarque d'honnêteté* : la formulation fautive est celle que la première relecture proposait au
point 51-4 ; l'erreur est de mon côté, elle n'en reste pas moins à corriger.

**51-N2 — La gravité de la notification de REQ-LOOP-073 est contradictoire dans l'amendement
lui-même. (non bloquant)**
Le point 1 de l'exigence écrit « afficher la **notification d'erreur** `Loading timed out.`
(**gravité normale**, 4 s, SPEC-10 REQ-UI-080/081) », et le §3 point 5 ajoute à la table de SPEC-07
§4 la ligne « | … | `Loading timed out.` | **erreur** | 4 s | ». `REQ-UI-080` fait de « normale » et
« erreur » deux gravités distinctes (blanc / rouge), et `REQ-UI-081` en tire une règle de
remplacement : une notification normale est **ignorée** si une erreur est déjà affichée. Les deux
rédactions ne décrivent donc pas le même comportement observable.
*Correction proposée* : retenir **erreur**, par cohérence avec les échecs de lancement de
REQ-LOOP-023 (ligne « erreurs de lancement / chargement | … | **erreur** » de SPEC-07 §4) et parce
que l'événement conduit à l'état Arrêté ; la durée de 4 s est celle du défaut de REQ-UI-080.

**51-N3 — `REQ-E3D-524` est citée (deux fois) comme P1 : elle est P2. (non bloquant, mais factuel)**
SPEC-04B §3.4 porte « **REQ-E3D-524** (P2) ». L'amendement écrit « Elle contredisait frontalement
REQ-E3D-524 **(P1)** » et la clause de Priorités s'appuie sur la même lecture. La décision de
retirer REQ-E3D-645 **n'est pas remise en cause** : la contradiction de critères (73 728 sommets
« tous présents » contre 70 000 « non dessiné ») subsiste entre deux exigences de même priorité, et
le second motif — une clause de sortie qui rend l'exigence vide pour toute implémentation à indices
32 bits — est indépendant de la priorité. Seul le texte est à corriger.
*Correction proposée* : « REQ-E3D-524 (P2) » aux deux endroits. (L'erreur vient de la première
relecture, qui l'annonçait comme P1.)

**51-N4 — Le plafond de journal partagé entre REQ-E3D-644 et REQ-E3D-646 n'est pas mesurable.
(non bloquant)**
REQ-E3D-646 écrit « au plus une fois par objet (et, **au-delà de 100 lignes par partie**, sous la
forme de résumé de REQ-E3D-644) ». Deux inconnues : le compteur de 100 lignes est-il **commun** aux
deux exigences ou propre à chacune ; et le résumé de REQ-E3D-644 est défini comme donnant « le
nombre cumulé de **patterns** concernés », grandeur qui ne décrit pas des **objets** restés en
attente. Le critère d'acceptation ne couvre que le cas à une ligne ; le cas au-delà de 100 n'est pas
testable.
*Correction proposée* : donner à REQ-E3D-646 son propre plafond et son propre résumé (« au plus 100
diagnostics par partie, puis au plus une ligne par minute de jeu donnant le nombre cumulé d'objets
concernés »), et ajouter un critère (101 objets retardés de 700 frames → 100 diagnostics puis une
ligne de résumé).

**51-N5 — Deux critères reposent sur des capacités de harnais non déclarées. (mineur)**
Le point 51-3 a été traité en déclarant l'option `--atlas-budget` dans le §3 ; la même rigueur
manque pour « un objet dont la construction est **retardée de 10 frames par le harnais** »
(REQ-E3D-646) et pour « le harnais **bloque l'étape 6** de REQ-LOOP-021 » (REQ-LOOP-073) : aucune
option du harnais ne fournit ces deux comportements, dans AMD-051 ni ailleurs.
*Correction proposée* : ajouter au §3 point 8 les deux options correspondantes (par exemple
`--delay-mesh <frames>` et `--stall-load <ms>`), au même titre que `--atlas-budget`.

### Conclusion AMD-051 (gate R2)

Les douze points sont levés, dont les cinq bloquants, et le LEAK faible a disparu. Mais la réponse à
51-4 a introduit **51-N1**, qui rendrait une implémentation conforme incapable de charger un gros
profil sur une machine lente — une exigence P2 qui annule une P1. **Gate R2 franchi sous réserve de
51-N1**, dont la correction tient en une incise ; 51-N2 (gravité) et 51-N3 (priorité citée) sont à
corriger dans la même passe, étant l'un et l'autre des contradictions internes au texte.

---

## 3. AMD-052 v1.1 — Cadence, file de frames GPU, anticrénelage par défaut

### Verdict : **accepté avec une réserve bloquante** — 13 points sur 13 levés, mais **1 bloquant nouveau (52-N1)**, réparable en une ligne.

Les trois sujets sont désormais au même niveau de maturité. REQ-LOOP-012 est amendée **texte et
critère** et le vecteur T5 est donné sous sa forme de ligne de table, avec son arithmétique.
REQ-SCN-077 est retirée et remplacée par REQ-LOOP-077, avec la clause « sans objet en libretro » et
un critère exécutable en GL 3.3 core. Le défaut d'anticrénelage vise enfin le texte qui le porte —
la ligne `video.anti_alias` de SPEC-09 §4.1.1 — sous un identifiant propre, REQ-PERS-089 (P3), avec
sa divergence **(D)** assumée. Vérifications de fond : la ligne de SPEC-09 §4.1.1 reproduite est
exacte (seules la colonne « Défaut » et la fin de « Comportement » changent) ; le format de la table
§4.6 est respecté (Core option / Valeurs / Défaut / Catégorie) et la catégorie « Rendu » y existe
bien ; le groupe « Vidéo » est bien l'un des six groupes d'AMD-010 REQ-UI-202 ; `Zax.Harness render`
rapporte 4 échantillons à `--anti-alias 1`, conformément au critère d'AMD-032 §2.

### Points de la première relecture

| Point | Statut | Vérification |
|---|---|---|
| **52-1** (BLOQUANT) REQ-LOOP-012 amendée à moitié | **levé** | §3 points 1 et 2 : énoncé amendé (« dans la limite de 2 frames de retard », « ne dépasse jamais 2 », « au-delà de 2 frames DOIT être abandonné ») **et** critère amendé (« au plus 2 frames en rafale ; ≈ 28 frames de moins »). Arithmétique revérifiée : 500 ms → 30 dues, 2 exécutées, 28 perdues ✓. |
| **52-2** (BLOQUANT) grandeur bornée ambiguë | **levé** | « **le nombre de frames dues et non encore exécutées NE DOIT jamais dépasser 2** », reprise mot pour mot de REQ-LOOP-012, avec la Note qui dit pourquoi (les deux lectures diffèrent d'une frame). |
| **52-3** (BLOQUANT) REQ-LOOP-075 casse l'avance rapide | **levé** *(mais voir 52-N1)* | Variante stricte : « l'**armement, l'expiration et le réarmement** d'une avance rapide (REQ-LOOP-016) NE DOIVENT PAS la réinitialiser » ; §3 point 3 précise que REQ-LOOP-016 ne figure pas dans la liste de REQ-LOOP-013. Le second critère reste inexécutable. |
| **52-4** (BLOQUANT) mauvais document, contre REQ-SCN-057 | **levé** | `REQ-SCN-077` **retirée** (SPEC-06 §3.17, mention « retirée ») ; `REQ-LOOP-077` la remplace, portée « frontend desktop », « sans objet en libretro, où la présentation et la chaîne d'échange appartiennent à l'hôte (REQ-PERS-086) et où le moteur dessine dans une cible fournie par l'hôte (REQ-SCN-057) » ; `video.max_queued_frames` ajouté à la liste « Non exposés en libretro » de §4.6, aux côtés de `video.vsync` et `video.frame_rate` ✓. |
| **52-5** (BLOQUANT) critère non vérifiable | **levé** | Critère binaire en GL 3.3 core (`glFenceSync` + `glClientWaitSync` à délai nul, signalé avant la première commande de la frame *n + 1* à 1, non exigé à 2), conforme à REQ-SCN-056. Le critère de latence reste comparatif sans seuil : **52-N4**. |
| **52-6** (BLOQUANT) amendement inopérant | **levé** | `REQ-PERS-089` vise la **ligne de SPEC-09 §4.1.1** (défaut `0` → `1`), la ligne complète est reproduite, REQ-PERS-017 est déclarée inchangée, et une Note (rédaction) explique pourquoi la v1.0 était inopérante (AMD-032 §2 déclare §4.1.1 inchangé). |
| **52-7** clause sans identifiant REQ | **levé** | `REQ-PERS-089` (P3), avec clause de priorité relative (« si elle n'est pas retenue, le défaut reste `0` et rien d'autre ne change »). |
| **52-8** divergence non étiquetée | **levé** | « divergence assumée **(D)** » dans l'énoncé, **(D)** dans la ligne SPEC-09, Note de justification reprenant l'argument d'AMD-032 §2. |
| **52-9** « groupe Rendu » cassé | **levé** | Colonne UI → « groupe **Vidéo** (AMD-010 REQ-UI-202), après “Render Rate” ». |
| **52-10** ligne libretro manquante | **levé** | Ligne `zaxnes_video_anti_alias | None, FFAA, SMAA | FFAA | Rendu` ajoutée au §4.6 avec le préfixe d'AMD-029, et `video.max_queued_frames` déclaré non exposé (colonne libretro « — »). |
| **52-11** interaction avec la cadence d'émulation | **levé** | « L'attente NE DOIT PAS décaler la cadence des frames émulées (REQ-LOOP-010) et ne constitue pas un retard au sens de REQ-LOOP-074 ; à 1, le saut de rendu de REQ-LOOP-014 reste applicable », plus un critère (600 ± 1 frames à 1). |
| **52-12** vecteur T5 hors table | **levé** | §3 point 5 donne la ligne telle quelle ; la Note donne l'arithmétique (60 dues, 2 exécutées, 58 abandonnées). Recontrôlé exact. |
| **52-13** étanchéité (pour mémoire) | **confirmé** | §4 conservé ; rien d'interne n'est ajouté par la v1.1. |

### Points nouveaux

**52-N1 — Le second critère de REQ-LOOP-075 ne peut pas être satisfait : il mesure un histogramme à
`k = 1` pendant une avance rapide à `k = 1,5`. (BLOQUANT)**
Le critère demande, pendant un maintien de 5 s de la hotkey, que « les critères de REQ-LOOP-016
(≈ 450 frames émulées ±5 %) **et de REQ-LOOP-011 (95 % des intervalles dans
[14,6 ms ; 18,6 ms] hors rattrapage)** restent vérifiés ». Or le critère de `REQ-LOOP-011` est posé
« à `k = 1` NTSC », et pendant l'avance rapide `k = 1,5` : la cadence visée est de 90 frames
émulées par seconde, soit un intervalle nominal de **11,1 ms**, hors de l'intervalle exigé. Une
implémentation conforme échoue donc à ce critère par construction — le défaut que 52-1 reprochait à
REQ-LOOP-012.
Dans la même Note, le chiffrage est faux : « un maintien produit donc deux changements d'état
(expiration puis réarmement) par salve, **soit 2 à 4 par seconde** ». Une salve de 30 frames
émulées à `k = 1,5` dure **≈ 0,33 s** — `REQ-INP-103` écrit ce chiffre noir sur blanc (« un appui
bref donne 30 images à ×1,5, environ 0,33 s ») — donc ≈ 3 salves et **≈ 6 changements d'état par
seconde**. L'argument n'en est que plus fort, mais la constante doit être juste (SPEC-GUIDE :
justification des constantes).
*Correction proposée* : citer l'**énoncé** de REQ-LOOP-011 et non son critère — « l'écart entre deux
frames émulées consécutives reste dans `1/f ± 2 ms` avec `f = 90` (REQ-LOOP-011) pendant le
maintien, et dans `1/f ± 2 ms` avec `f = 60` après le relâchement » — et corriger la Note en
« ≈ 3 salves, soit ≈ 6 changements d'état par seconde ».
*Remarque d'honnêteté* : l'une et l'autre approximations viennent du point 52-3 de la première
relecture, qui parlait de « ≈ 20 ms × 30 » (valeur à `k = 1`) et renvoyait à « l'histogramme
d'intervalles de REQ-LOOP-011 » sans préciser qu'il est posé à `k = 1`.

**52-N2 — Deux sources normatives pour la même borne de rattrapage. (non bloquant)**
REQ-LOOP-012 est amendée (§3 points 1 et 2) **et** REQ-LOOP-074 (P1) énonce la même règle, avec la
même grandeur « reprise mot pour mot » et les mêmes critères. C'est exactement le défaut que le point
51-6 reprochait à REQ-PERS-088 — et qu'AMD-051 a corrigé en ne réénonçant rien : « pour qu'il
n'existe qu'une source normative par comportement ». Les deux textes divergeront à la première
reformulation.
*Correction proposée* : choisir. Soit REQ-LOOP-074 est l'unique énoncé et REQ-LOOP-012 porte la
mention « remplacée par REQ-LOOP-074 » (le numéro restant attribué), soit l'amendement se limite à
réécrire REQ-LOOP-012 et REQ-LOOP-074 n'est pas créée. La première solution est préférable : elle
garde la trace de la décision dans un numéro propre.

**52-N3 — Libellé d'index inexact. (mineur)**
Le §3 point 11 annonce « priorité **et** critère de REQ-LOOP-012 mis à jour » dans
`SPEC-v1.2-INDEX`. Rien dans l'amendement ne change la priorité de REQ-LOOP-012, qui est P1 avant
comme après.
*Correction proposée* : « énoncé et critère de REQ-LOOP-012 mis à jour ».

**52-N4 — Critères de REQ-LOOP-077 : un comparatif sans seuil et deux capacités de harnais non
déclarées. (mineur)**
(a) « la latence mesurée […] est **plus courte** à 1 qu'à 2 » : sans écart minimal, sans nombre de
mesures et sans statistique, deux campagnes bruitées peuvent conclure différemment. (b) « y compris
quand le rendu est **volontairement ralenti par le harnais** » et, pour REQ-LOOP-075, « le
**compteur de réinitialisations d'horloge** reste à 0 » supposent deux instruments que ni AMD-052 ni
le harnais ne déclarent (même remarque qu'en 51-N5).
*Correction proposée* : « la médiane sur 600 frames est plus courte d'au moins une demi-période
`1/f` à 1 qu'à 2 » ; et déclarer les deux instruments du harnais (par exemple
`--slow-render <ms>` et le compteur exposé par l'API de test de REQ-LOOP-001).

**52-N5 — Clause d'AMD-032 §2 à déclarer périmée. (mineur)**
AMD-032 §2 affirme « la persistance, l'aller-retour des profils (champ 32), **la table libretro** et
l'API Lua ne bougent pas ». AMD-052 §3 point 8 réécrit bel et bien la table libretro (ligne ajoutée,
défaut `FFAA`). L'amendement liste les invariants dans REQ-PERS-089 (« le domaine, les libellés, le
champ de profil 32 et l'API Lua ») en omettant prudemment la table libretro, mais ne dit pas que la
clause d'AMD-032 §2 est dépassée sur ce point. Au rang 1 du corpus normatif (« le plus récent
l'emporte », SPEC-v1.2-INDEX §1) la lecture est bonne ; une phrase l'évitera à un tiers.
*Correction proposée* : ajouter en Notes « la clause “la table libretro ne bouge pas” d'AMD-032 §2
reste vraie pour les valeurs et la catégorie ; seule la colonne “Défaut” change, par REQ-PERS-089 ».

### Conclusion AMD-052 (gate R2)

Les treize points sont levés, dont les six bloquants, et les deux vérifications arithmétiques
annoncées (T5 et le critère de 500 ms) sont exactes. La réponse à 52-3 laisse en revanche un critère
qu'une implémentation conforme échouerait (**52-N1**) et une constante fausse dans la même Note.
**Gate R2 franchi sous réserve de 52-N1.** 52-N2 (double source normative) mérite d'être traité dans
la même passe : c'est, à l'identique, le défaut que la première relecture a fait corriger sur
REQ-PERS-088.

---

## 4. AMD-053 v1.1 — Reprise, favoris, recherche, inspection des imports, ligne de commande

### Verdict : **prêt pour la mise en œuvre** — 16 points sur 16 levés, 0 bloquant résiduel, 5 points nouveaux non bloquants.

C'est la révision la plus lourde des quatre, et la plus aboutie. Les quatre contradictions sont
devenues des amendements explicites (REQ-LOOP-052, REQ-UI-030, REQ-PERS-041 point 1, et le
rattachement au modèle de commandes plutôt qu'à la présentation neutralisée par AMD-010) ; le modèle
de persistance de l'emplacement `resume` et des favoris est écrit en entier, lignes SPEC-09 à
l'appui ; REQ-UI-212 est refondue sur les causes réelles de SPEC-03 et renonce à ses quatre libellés
au profit des trois textes de REQ-UI-084. Vérifications de fond : REQ-PERS-050, -052, -042, -055,
-033, -057, §4.5.2 (`schemaVersion` 1 → 2, champ `favorite`, autres champs inchangés), §4.5.3 (37 →
38 entrées), §4.1.9 (`ui.sort_option`, champ de profil 36) et REQ-UI-030 sont cités exactement ;
`REQ-PROF-041` confirme que `SortOption` fait partie des valeurs de profil **ignorées** au
chargement, donc que la clause de portée U est juste ; `REQ-PROF-050`/`-051` (aller-retour octet par
octet des 101 profils) ne sont pas menacés par la règle « la valeur 2 n'est jamais écrite dans le
champ 36 », qui ne concerne que l'écriture des réglages courants de `REQ-PROF-052` ; `REQ-PERS-056`
points 1 et 2 fournissent bien la signature et la version qui donnent un sens à « incompatible ».

**Point de fait corrigé au passage.** La première relecture citait le troisième texte de REQ-UI-084
avec « …the current version of **3dSen**. ». AMD-053 écrit « …the current version of **Zaxnes**. » :
c'est l'amendement qui a raison — `AMD-029` §3 dit « C'est la seule modification du texte exact ».

### Points de la première relecture

| Point | Statut | Vérification |
|---|---|---|
| **53-1** (BLOQUANT) contre REQ-LOOP-052 (P1) | **levé** | §3 points 1 et 2 : énoncé et critère amendés (« sans sauvegarde automatique de la partie **dans les slots numérotés ni dans le slot rapide** ; l'emplacement `resume` […] est écrit **(D)** »). Ordre des opérations donné en quatre étapes, conforme à l'énoncé de REQ-LOOP-052 (hook de fin, arrêt de la boucle, réglages et métadonnées, fermeture) et à REQ-LOOP-063 pour l'ordre des hooks ; la condition et la clause « entre deux frames émulées » de REQ-LOOP-041 sont traitées. |
| **53-2** (BLOQUANT) aucun modèle de persistance | **levé** | REQ-PERS-050 amendée (38 emplacements), §4.5.3 à 38 entrées avec `schemaVersion` 2 et migration, nom de fichier et vignette dans l'exemple, exclusion des listes Load/Save (REQ-PERS-052 étendue), effacement avec le jeu (REQ-PERS-042), réconciliation au démarrage (REQ-PERS-055), source de la vignette à la sortie depuis En cours (capture de REQ-LOOP-006 sinon capture prise à l'écriture, format de REQ-PERS-051). |
| **53-3** (BLOQUANT) contre REQ-UI-030 (P1) | **levé** | §3 point 14 remplace la dernière phrase mot pour mot, avec **(D)** et la mention « l'original n'a ni l'un ni l'autre » ; REQ-UI-211 déclare explicitement « Il n'est pas créé de seconde recherche : c'est celle de REQ-UI-201 qui est précisée » (AMD-010 REQ-UI-201 porte bien « avec une recherche par titre »). |
| **53-4** (BLOQUANT) appui sur une présentation P3 | **levé** | Favoris et filtre exprimés sur le modèle de commandes (`favorite.toggle`, `library.filter_favorites`, `library.search`, `resume.last`, ajoutés à REQ-UI-205 au §3 point 17) et sur la boîte de REQ-UI-201 ; « le marquage est libre » ; §1 explique le principe ; critères automatiques. Les mots « tuile » et « grille paginée » ont disparu. |
| **53-5** (BLOQUANT) favoris et tri sans représentation ni ordre | **levé** | (a) champ `favorite` au §4.5.2, `schemaVersion` 2, migration et `library.json.v1.bak` (REQ-PERS-024, REQ-PERS-088) ; (b) ligne `ui.sort_option` réécrite, domaine « 2 Favorites first », et la question d'interopérabilité du champ 36 tranchée explicitement **(D)** ; (c) REQ-PERS-033 amendée (favoris puis non-favoris, chaque groupe par titre croissant) et « sans effet sur les listes de slots (REQ-PERS-057), qui suivent “A-Z” quand `ui.sort_option` vaut 2 » ; clause libretro « sans objet » (REQ-PERS-086). |
| **53-6** (BLOQUANT) format inventé | **levé** | « le lire **selon SPEC-03, sans charger ses patterns ni ses scripts** » ; table des causes réelles (`Type`, `Ver`, protobuf mal formé, `Id`) avec renvois à REQ-PROF-001, -021, -033, -063 ; Note qui enterre « signature absente » et « données tronquées ». Liste de champs incomplète : **53-N1**. |
| **53-7** (BLOQUANT) textes concurrents de REQ-UI-084 | **levé** | Option (a) : les trois textes de REQ-UI-084 restent « seule source normative », reproduits au caractère près (y compris les **deux espaces** et l'absence d'espace après le chemin) ; REQ-UI-084 déclarée inchangée au §3 point 15 ; REQ-PERS-041 point 1 amendé pour choisir selon la cause ; les quatre libellés de la v1.0 sont nommément abandonnés ; « Aucune divergence (D) n'est donc nécessaire ici » — exact. |
| **53-8** justification inexacte | **levé** | §1 réécrit : « la lacune n'est pas l'absence de signalement : REQ-PERS-041 point 1 exige déjà une notification de 4 s, et REQ-PERS-040 exige déjà d'ignorer **et de journaliser** […]. Ce qui est nouveau est de **choisir le message selon la cause** ». |
| **53-9** titre et auteur | **levé** | « titre = `Title` (champ 8), à défaut le nom du fichier sans extension (REQ-PERS-034) ; auteur = `Maker` (champ 9) », champs facultatifs (REQ-PROF-022), auteur vide **omis**, avec critère. Cas non traité : **53-N5**. |
| **53-10** `-slot` 1 à 36 contre REQ-UI-208 | **levé** | Écrit : « La ligne de commande n'est pas tenue par la limite d'exposition de AMD-010 REQ-UI-208 (10 emplacements dans les menus) et accepte les 36 slots que REQ-PERS-050 gère » ; §3 point 17 confirme REQ-UI-208 inchangée. |
| **53-11** convention de préfixe | **levé** | « Ces trois options rejoignent la famille des options **reprises du programme d'origine**, à tiret simple comme le `-id=` de REQ-UI-007 ; les formes à double tiret […] DOIVENT être acceptées comme **alias** **(D)** », avec la liste des options propres à Zaxnes (AMD-032 §5, AMD-030, AMD-026, REQ-PERS-071 — toutes vérifiées). |
| **53-12** REQ-UI-213 sous-spécifiée | **levé** | (a) « chargé **après** l'étape 7 de REQ-LOOP-021 (l'état étant passé à En cours) » ; (b) slot vide → ignoré et journalisé, slot d'un autre jeu → le jeu du slot est lancé (REQ-PERS-052) et `-id` sans effet ; (c) « `-slot` **implique** `-noresume` » ; (d) `-emustats` écrit dans le **fichier de journal** et non sur la console parente (AMD-032 §5), en anglais (AMD-035 §2), critère 600 ± 1 lignes. Interaction nouvelle : **53-N2**. |
| **53-13** condition circulaire | **levé** | « si l'état est **En cours** ou **En pause** et que l'indicateur **Chargement** est faux », avec la Note qui explique la circularité de la v1.0 (REQ-PERS-012 définit « fin de partie » comme incluant la sortie). |
| **53-14** effacement et échec | **levé** | Trois règles numérotées **dans cet ordre de priorité**, la première l'emportant explicitement sur la deuxième ; emplacement illisible ou incompatible → effacé, entrée non proposée, démarrage normal, **sans notification**, fait journalisé ; critère associé. Précisions à ajouter : **53-N3**. |
| **53-15** normalisation de la recherche | **levé** | « après **décomposition NFD**, retrait des marques diacritiques (catégorie Unicode `Mn`) et mise en minuscules **invariante de culture** », critères « pokemon » → « Pokémon » et « bros » → « Mario Bros. ». |
| **53-16** étanchéité (pour mémoire) | **confirmé** | §5 conservé ; les ajouts de la v1.1 (`favorite`, `resume`, `resume.last`, `library.search`) sont des données et des commandes de la réimplémentation. |

### Points nouveaux

**53-N1 — La liste des champs racine nécessaires à REQ-UI-212 omet celui que sa propre table
exige. (non bloquant)**
L'exigence écrit « seuls les champs racine **1, 7, 8, 9, 10 et 11** sont nécessaires,
REQ-PROF-062 » — c'est bien le sous-ensemble que REQ-PROF-062 et SPEC-03 §4.1 appellent
« l'en-tête ». Mais la deuxième ligne de la table des causes porte sur **`Ver`, qui est le champ 2**
(REQ-PROF-021, SPEC-03 §4.1). Avec la liste citée, la cause « `Ver` = `"1"` ou non analysable » est
inévaluable.
*Correction proposée* : « les champs racine **1, 2, 7, 8, 9, 10 et 11** (en-tête de REQ-PROF-062,
plus le champ `Ver` nécessaire à la table ci-dessous) », et ajouter au §3 la précision
correspondante pour la phrase de SPEC-03 §4.1 (« L'en-tête lu pour la bibliothèque est le
sous-ensemble {1, 7, 8, 9, 10, 11} ») ou dire explicitement que l'import en lit un de plus.

**53-N2 — `-noresume` et la première règle d'effacement se contredisent. (non bloquant)**
`REQ-UI-213` dit : « `-noresume` : l'entrée `Resume` […] n'est pas proposée à ce lancement ;
l'emplacement `resume` n'est **ni lu ni effacé** ». La règle 1 de REQ-LOOP-076 dit : l'emplacement
« est effacé à la **sortie** quand l'état est Arrêté ; cette règle l'emporte sur la suivante ».
Lancer avec `-noresume` puis quitter sans jouer tombe sous les deux : le point de reprise est-il
préservé (lecture de `-noresume`) ou détruit (lecture de la règle 1) ? Comme `-slot` implique
`-noresume`, le cas est fréquent : lancer un slot, quitter depuis Arrêté, et perdre la reprise.
*Correction proposée* : trancher en une phrase, de préférence « `-noresume` suspend aussi la règle 1
pour la durée de ce lancement : l'emplacement `resume` n'est ni lu, ni écrit, ni effacé », ce qui est
la seule lecture conforme à l'intention de l'option.

**53-N3 — Le contrôle de l'emplacement `resume` au démarrage n'est pas dit « complet », et sa
relation à REQ-LOOP-046 n'est pas écrite. (non bloquant)**
Le critère « **tronquer** le fichier de l'emplacement `resume` → au démarrage, l'entrée n'est pas
proposée, le fichier est supprimé » n'est satisfiable que si le fichier est **validé en entier** au
démarrage : un savestate tronqué dans son corps garde une signature et une version valides
(REQ-PERS-056 points 1 et 2), et seule la règle « toute section de longueur incohérente rend le
fichier invalide » (§4.5.4) permet de le détecter. Il faut donc dire que ce contrôle est fait, et ce
qu'il coûte à chaque démarrage. Par ailleurs, rien n'est dit du cas où le fichier passe le contrôle
mais échoue à l'activation de `Resume` : `REQ-LOOP-046` (P1) impose alors la notification
`Corrupted save state.` et l'état Arrêté, ce qui n'est pas « sans notification ». (Il n'y a pas de
contradiction : la réconciliation silencieuse de REQ-PERS-055 est le bon précédent pour le
démarrage ; mais les deux chemins doivent être distingués.)
*Correction proposée* : « l'emplacement `resume` est validé en entier au démarrage (signature,
version et cohérence des longueurs, §4.5.4) ; s'il échoue, il est effacé sans notification. Si le
chargement de l'entrée `Resume` échoue malgré ce contrôle, REQ-LOOP-046 s'applique (notification
`Corrupted save state.`, état Arrêté). » Et vérifier au passage que le critère de REQ-LOOP-052
(« l'application se ferme en moins de 2 s ») reste tenable avec une capture et un savestate complet
écrits à la sortie.

**53-N4 — L'écran d'accueil de REQ-UI-207 n'énumère pas les jeux. (mineur)**
`AMD-010` REQ-UI-207 décrit « un écran d'accueil simple : “Fichier ▸ Ouvrir une ROM” et la liste des
**ROMs récentes** ». REQ-UI-210 y exige pourtant la commande de favori et le filtre, et REQ-LOOP-076
y place l'entrée `Resume` « en tête de l'**énumération des jeux** ». Seule REQ-UI-211 prend la
précaution d'écrire « **quand il énumère les jeux** ».
*Correction proposée* : reprendre la même garde dans REQ-UI-210 et REQ-LOOP-076, ou amender
REQ-UI-207 pour dire que l'écran d'accueil énumère aussi la bibliothèque — en le déclarant, puisque
c'est un ajout à AMD-010.

**53-N5 — Notification d'import dans le cas de l'identifiant d'un profil embarqué. (mineur)**
`REQ-PERS-041` point 4 : « identifiant d'un profil **embarqué** → aucun changement ». REQ-UI-212
n'énonce que deux issues : refus (table des causes) ou acceptation (titre et auteur affichés). Le
troisième cas, où le fichier est valide mais n'est pas ajouté, n'a pas de notification définie — et
pour un profil embarqué les métadonnées affichées viennent du catalogue, pas des champs
(`REQ-PROF-060`, `REQ-PROF-022`).
*Correction proposée* : une ligne (« identifiant d'un profil embarqué : aucun changement
(REQ-PERS-041 point 4) ; la notification affiche le titre du **catalogue** (REQ-PROF-060) et indique
que le profil n'est pas remplacé »), ou dire explicitement que ce cas n'affiche rien.

### Conclusion AMD-053 (gate R2)

Les seize points sont levés, dont les sept bloquants, et aucune des corrections n'a cassé de
référence : les quatorze lignes de SPEC-09, SPEC-10 et AMD-010 citées sont exactes, y compris le
texte de REQ-UI-084 tel qu'AMD-029 §3 le modifie. **Gate R2 franchi, sans réserve bloquante.** Les
cinq points nouveaux sont des précisions ; **53-N1** (champ 2 manquant) et **53-N2** (`-noresume`)
sont les deux à traiter avant la mise en œuvre, l'un parce qu'une cause de la table est sinon
inévaluable, l'autre parce qu'un tiers ne peut pas choisir entre deux règles.

---

## 5. Récapitulatif et réponse à la question du gate R2

| Amendement | Verdict v1.1 | Points v1.0 | Levés | Nouveaux | Bloquants résiduels | Prêt pour la mise en œuvre ? |
|---|---|---|---|---|---|---|
| AMD-050 (REQ-CORE-048/049, REQ-SCN-076, V11) | **accepté** | 9 | **9/9** | 6 | — | **Oui** |
| AMD-051 (REQ-E3D-644/646, REQ-LOOP-073, REQ-PERS-088 ; -645 retirée) | **accepté sous réserve** | 12 | **12/12** | 5 | **51-N1** | **Non** — une clause à corriger |
| AMD-052 (REQ-LOOP-074/075/077, REQ-PERS-089 ; REQ-SCN-077 retirée) | **accepté sous réserve** | 13 | **13/13** | 5 | **52-N1** | **Non** — une ligne de critère à corriger |
| AMD-053 (REQ-LOOP-076, REQ-UI-210 à 213) | **accepté** | 16 | **16/16** | 5 | — | **Oui** |

**Les 45 points de la première relecture sont levés, aucun n'est resté partiel, aucun n'a été écarté
— il n'y a donc aucune « raison de non-retenue » à juger.** Les quatre révisions ont traité les
bloquants au fond : les deux exigences à retirer ou déplacer le sont effectivement, les onze
exigences en vigueur que le lot contredisait sont amendées explicitement et reproduites, et tous les
critères jugés inexécutables sont remplacés par des critères chiffrés ou binaires. L'étanchéité est
acquise pour les quatre, et le seul LEAK faible de la v1.0 a disparu : la v1.1 est, de ce point de
vue, strictement meilleure.

**Deux amendements ne sont pas encore prêts**, chacun pour un défaut introduit par une correction, et
chacun réparable sans toucher au comportement visé :

- **51-N1** — la seconde branche de `REQ-LOOP-073` (« une étape de REQ-LOOP-021 ne se termine pas
  dans les 10 s ») avorte un chargement légitime : l'attente des maillages de l'étape 6 n'est bornée
  par aucune exigence (`REQ-ACC-031` l'exclut explicitement de son budget). À restreindre aux étapes
  sans mesure d'avancement.
- **52-N1** — le second critère de `REQ-LOOP-075` exige qu'un histogramme posé à `k = 1`
  (`REQ-LOOP-011`) reste vérifié pendant une avance rapide à `k = 1,5` : inexécutable. À réécrire sur
  l'énoncé de REQ-LOOP-011 (`1/f ± 2 ms`, `f = 90`), et à corriger la constante de la Note (≈ 3
  salves et ≈ 6 changements d'état par seconde, `REQ-INP-103`).

À traiter dans la même passe, sans bloquer : **51-N2** (gravité normale contre « erreur » dans la
table SPEC-07 §4), **51-N3** (`REQ-E3D-524` citée P1, elle est P2), **52-N2** (double source
normative pour la borne de rattrapage, le défaut même que 51-6 a fait corriger ailleurs), **50-N1**
(le masque d'opacité doit suivre les mêmes règles d'affichage que l'image, limite de 8 sprites par
ligne comprise), **53-N1** (champ 2 `Ver` absent de la liste des champs lus) et **53-N2**
(`-noresume` contre la règle d'effacement à la sortie).

Trois erreurs de la **première relecture** ont été propagées ou corrigées par les auteurs, et sont
rectifiées ici : `REQ-E3D-524` est P2 et non P1 (51-N3) ; le harnais s'appelle `Zax.Harness`
(AMD-029), comme AMD-051 l'écrit ; le troisième texte de `REQ-UI-084` dit bien « …version of
Zaxnes. » (AMD-029 §3), comme AMD-053 l'écrit. Les deux formulations fautives que la première
relecture avait elle-même proposées (le délai par étape de 51-4 et le renvoi à l'histogramme de
REQ-LOOP-011 de 52-3) sont les deux bloquants résiduels : elles sont signalées comme telles.
