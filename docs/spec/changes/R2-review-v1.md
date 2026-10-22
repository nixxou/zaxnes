# R2 — Revue anti-fuite et cohérence de la spec v1.0-draft

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-20 | Version du rapport : v1
Documents relus : SPEC-00, 01, 02, 03, 04, 04B, 05, 06, 07, 08, 09, 10, 11, 12, `DECISIONS-R0.md`.
Références de cadrage : `SPEC-GUIDE.md`, `PLAN.md` §3.3, `assets-neutral/INVENTORY.md`.

Catégories : **LEAK** (bloquant), **UNTESTABLE**, **INCONSISTENT**, **UNCLEAR**.
Convention : « bloquant » = doit être corrigé avant le gel de la spec v1.0.

---

## 0. Points transverses (à traiter en priorité)

### T-1 (INCONSISTENT, bloquant) — Sens des composantes de `Clipping` : SPEC-09 contredit SPEC-04, SPEC-05 et SPEC-06

SPEC-09 §4.1.5 :
> `clip.top` | `Clipping[1]`, `Clipping.y` | … marge haute
> `clip.bottom` | `Clipping[3]`, `Clipping.w` | … marge basse

SPEC-05 §4.2 (type `Découpe`) dit exactement l'inverse : `y` = « marge basse », `w` = « marge haute ». SPEC-06 REQ-SCN-003 : « i = 0 gauche, 1 bas, 2 droite, 3 haut ». SPEC-04 REQ-E3D-002 : `Clipping = (g, b, d, h)`.

SPEC-09 est de plus **contradictoire avec elle-même** : sa propre formule (§4.1.5, fin) écrit `y + 8 > c3` (test du bord **haut** dans le repère écran de SPEC-02) alors que sa table nomme `c3` « marge basse ».

Conséquence observable : 38 occurrences de `Setting.Clipping[]` dans le corpus Lua ; une inversion applique une marge de 8 px au mauvais bord. `DEC-09-Q2` ne tranche pas (il dit « `clip.top` désigne le haut de l'écran » sans fixer l'indice).

Proposition : retenir la convention majoritaire (indice 0 = gauche, 1 = **bas**, 2 = droite, 3 = **haut**), renommer les clés SPEC-09 dans cet ordre, et clore SPEC-09 Q-2 devenue sans objet.

### T-2 (INCONSISTENT, bloquant) — Hauteur de scène : la « correction transverse » de DECISIONS-R0 ne couvre pas SPEC-06

`DECISIONS-R0` impose Hs = 248 et dit remplacer « la valeur 240 de REQ-SCN-001 … partout où elle désigne la hauteur de la scène ». Cette règle de substitution mécanique laisse SPEC-06 incohérent :

- §2 : « **Plan NES** : rectangle z = 0, x ∈ [0, 256], **y ∈ [0, 240]** » → devrait être y ∈ [8, 248] ;
- §2 et REQ-SCN-003 : `R = [c0, 256 − c2] × [c1, 240 − c3]` → `248 − c3` ;
- REQ-SCN-001 : « zone [px, px + 1] × [239 − py, 240 − py] » et son critère « [0, 8] × [232, 240] » contredisent REQ-E3D-401 ([247 − py, 248 − py]) de 8 unités ;
- **le centre C = (128, 120, 0)** (REQ-SCN-012, §4.3, REQ-SCN-045) n'est **pas** corrigé par la règle (120 ≠ 240) : il reste 8 unités sous le centre réel de l'image, ce qui décale la vue par défaut, la cible caméra, le HUD et le boîtier TV ;
- tous les vecteurs numériques de §5.1 (T′, E, P\*) et §5.3 sont calculés dans l'ancien repère ;
- REQ-SCN-053 (tunnel TV) utilise `240 − c3`, `244 − c3`, `[0, 240]` ;
- SPEC-08 REQ-INP-063 vise « y de 0 (haut) à 240 » sans donner la conversion scène → pixel NES.

Proposition : publier un amendement qui réécrit explicitement SPEC-06 §2, REQ-SCN-001, -003, -012, -045, -053, §4.3 et §5.1/§5.3 avec Hs = 248 et C = (128, 128, 0), plutôt qu'une règle de substitution globale.

### T-3 (INCONSISTENT, bloquant) — Cadence hôte : 60 fps (SPEC-02) contre 60,0988 Hz (SPEC-07/SPEC-11)

SPEC-02 REQ-CORE-015, colonne « Cadence nominale de l'hôte » : NTSC → **60 fps** ; REQ-CORE-070 cale l'audio sur `fréquence / cadence nominale` (800 échantillons à 48 kHz) et justifie :
> caler la production sur la cadence nominale (60 ou 50 fps) plutôt que sur la fréquence exacte du matériel (60,0988 Hz) garde l'audio synchrone avec l'hôte

SPEC-07 §2 et REQ-LOOP-010 font tourner l'hôte à la **fréquence nominale 60,0988 Hz** ; SPEC-11 REQ-AUD-007 attend « ≈ **798,7** trames par `retro_run` » (= 48000/60,0988), contre 800 dans SPEC-02.

Conséquence : production de 48 079 éch./s contre 48 000 consommés, soit une dérive de 0,16 % — exactement le phénomène que REQ-AUD-004/005 cherchent à éviter. Les trois documents doivent utiliser la même grandeur.

### T-4 (INCONSISTENT, bloquant) — Décalage ±0,1 des sprites : SPEC-04B contredit DECISIONS-R0

`DEC-06-Q4` : « Les positions livrées par SPEC-04B **n'incluent pas** le décalage de ±0,1 des sprites : c'est le rendu (SPEC-06) qui l'applique. »
SPEC-04B REQ-E3D-405 dit l'inverse :
> Ce décalage est inclus dans les transformations livrées au rendu (REQ-E3D-641), qui ne l'applique pas une seconde fois

TV-W2 intègre le décalage dans la boîte scène (`z ∈ [−0,1 ; 3,9]`). REQ-SCN-037 reste permissif (« Si SPEC-04B intègre déjà ce décalage … »). Il faut un seul propriétaire ; en l'état, le risque est l'application double ou nulle.

### T-5 (INCONSISTENT, bloquant) — Savestate : REQ-ARCH-013 exige l'état du moteur 3D, REQ-LOOP-040 l'interdit

SPEC-01 REQ-ARCH-013 : « Un savestate DOIT contenir … (cœur, **état du moteur de conversion**, état Lua …) », avec pour critère de rejouer M frames à l'identique.
SPEC-07 REQ-LOOP-040 : « Il NE DOIT PAS contenir l'état de la scène 3D » ; SPEC-09 REQ-PERS-056 de même.

Or SPEC-04 REQ-E3D-008 fait **survivre** le registre d'images, les patterns appris et les patterns temporaires à un chargement d'état (ils ne sont effacés qu'au changement de jeu). Après un aller-retour de savestate, la segmentation (REQ-E3D-092, passe 2 contre passe 3) dépend donc de l'historique d'apprentissage et le rejeu n'est pas reproductible — ce qui met en défaut REQ-ARCH-012/013, REQ-ACC-040 et la sérialisation libretro de REQ-LOOP-062.

Proposition : soit inclure le registre appris dans le savestate, soit restreindre explicitement la garantie de déterminisme et amender REQ-ARCH-013.

### T-6 (INCONSISTENT, bloquant) — Ordre de restauration d'un savestate : SPEC-07 contre SPEC-05

SPEC-05 REQ-LUA-072 : `End` → **rechargement éventuel du profil** → restauration du cœur → affectation de `PData[1..64]` → `Start` → `Load`.
SPEC-07 REQ-LOOP-044 : 3) arrêt ; **4) restaurer le cœur, les données persistantes et la caméra** ; **5) charger le profil si nécessaire**, `Start`, `Load`.

Dans l'ordre de SPEC-07, le chargement du profil (étape 5) crée un nouvel état Lua dont `PData` est remis à 64 zéros (REQ-LUA-011), ce qui efface la restauration faite à l'étape 4. Les deux étapes doivent être inversées dans REQ-LOOP-044.

### T-7 (UNTESTABLE, systémique) — Critères d'acceptation qui renvoient à l'outillage du côté sale

Un tiers qui ne connaît que la spec (R4) ne peut pas exécuter ces outils, qui résident dans un emplacement interdit au côté propre :

| Exigence | Critère cité |
|---|---|
| REQ-PROF-001, REQ-PROF-051 | « validation dirty-side » |
| REQ-E3D-060 | « vérifié côté outillage » |
| REQ-LUA-004 | vérification par un script d'ordre des scripts |
| REQ-LUA-051 | outil de contrôle d'API Lua, renvoi au §5.1 |
| REQ-LUA-100 | sonde de compilation Lua 5.4 |
| REQ-LUA-101 | sonde de dialecte Lua |

Tous sont reformulables sans perte en critères exécutables sur `assets-neutral/profiles` (« une analyse statique des 579 scripts résout 100 % des accès membres contre le §4 »). SPEC-05 §5.1 devrait être marqué **informatif**.

### T-8 (UNTESTABLE, systémique) — Critères qui exigent une ROM commerciale ou le programme d'origine

SPEC-12 §2 rappelle qu'aucune ROM commerciale n'est livrée. Or de nombreux critères P1 en dépendent : REQ-CORE-033, -039 (Punch-Out!!), -062 (Duck Hunt / Hogan's Alley / Wild Gunman), -114, -115 (101 titres), REQ-E3D-233 (« DQ3_DW3 »), -252 (« punchout »), -322 (« Mega Man 2 »), REQ-E3D-426, -565, -566, REQ-LUA-110/111 (SMB3), REQ-SCN-030, -046, SPEC-06 §5.4 (R1–R7), REQ-ACC-020/021.
S'y ajoutent les critères qui exigent une **comparaison avec le programme d'origine** (SPEC-06 §5.4, REQ-SCN-051 « tolérance perceptuelle ») : permis en boîte noire (PLAN §3.3 règle 6), mais non reproductible par un vérificateur qui ne dispose pas du programme.

Ce n'est pas un défaut de rédaction, mais il faut que SPEC-12 et la matrice de traçabilité **classent explicitement ces exigences comme « vérifiables en round 2 seulement »**, sans quoi la porte P4 (« aucune exigence non testable non justifiée », REQ-ACC-050) est inatteignable.

### T-9 (INCONSISTENT) — Questions ouvertes périmées par DECISIONS-R0

Les §6 suivants proposent encore une solution que DECISIONS-R0 a déjà tranchée autrement ou dans l'autre sens ; ils doivent être marqués « clos par DEC-… » :
SPEC-03 Q-02 (propose le tri ordinal ; `DEC-03-Q02` impose une collation culturelle invariante — REQ-LUA-004 fait foi), SPEC-03 Q-03 (`DEC-03-Q03`), Q-04 (`DEC-03-Q04`), SPEC-04 Q-01/Q-04/Q-05, SPEC-04B Q-1/Q-2/Q-4/Q-5/Q-6, SPEC-05 Q-05-2/3/4/5, SPEC-06 Q1 (cubemaps réextraits : `INVENTORY.md` confirme les 6 faces), Q2, Q5, Q6, SPEC-07 Q1/Q2/Q3/Q4, SPEC-08 Q1/Q2/Q3/Q4/Q6/Q7/Q8, SPEC-09 Q-1/Q-3/Q-4/Q-5/Q-6/Q-7, SPEC-10 Q-3/Q-4/Q-5/Q-6/Q-7, SPEC-11 Q1/Q2/Q3/Q5.

---

## 1. SPEC-00 — Glossaire

**INCONSISTENT 00-1** — Le glossaire inverse les noms Lua des deux objets de frame.
> **Shape** : groupe de tiles adjacentes … c'est l'objet `shape` exposé à Lua

SPEC-04 REQ-E3D-042 et SPEC-05 §4.1 disent le contraire : l'**instance de frame** est exposée sous le nom `shape`, la shape elle-même sous `shape2D`. Corriger SPEC-00 (et traiter la question Q-07 de SPEC-04, qui signale déjà le flottement de vocabulaire).

**UNCLEAR 00-2** — La liste « Type de géométrie : `Default`, `CUBE`, `VCYLINDER`, `HCYLINDER`, `HALFVCYLINDER`, `HALFHCYLINDER` » n'est pas dans l'ordre de l'énumération (SPEC-03 champ 2 : 0 Default, 1 HCYLINDER, 2 VCYLINDER, 3 CUBE, 4 HALFHCYLINDER, 5 HALFVCYLINDER). Un lecteur peut y voir un ordre normatif : ajouter les valeurs numériques ou renvoyer à SPEC-03.

Aucune fuite.

## 2. SPEC-01 — Architecture

**INCONSISTENT 01-1** — REQ-ARCH-013 contre REQ-LOOP-040 / REQ-PERS-056 : voir **T-5** (bloquant).

**INCONSISTENT 01-2** — REQ-ARCH-033 : « en mode portable … elles sont placées **à côté de l'exécutable** », alors que SPEC-09 REQ-PERS-071 impose un sous-dossier `userdata\`. Aligner.

**UNCLEAR 01-3** — REQ-ARCH-013 exige que la taille maximale d'un savestate « DOIT pouvoir être annoncée avant la sérialisation » ; ni SPEC-07 ni SPEC-09 ne disent comment la borner, alors que le bloc du cœur contient **l'image complète de la ROM** (REQ-CORE-090), de taille variable. Préciser que la borne vaut pour une ROM chargée (ce que dit REQ-LOOP-062, mais pas REQ-ARCH-013).

Aucune fuite.

## 3. SPEC-02 — Cœur NES

**LEAK 02-1 (mineur)** — REQ-CORE-120 :
> Une bibliothèque native fceumm est livrée avec la référence, mais elle n'est jamais chargée.

Inventaire de livraison et comportement de chargement interne de l'original, sans effet observable : l'exigence (« l'implémentation DOIT fournir un seul cœur ») se suffit à elle-même. Reformulation : supprimer la phrase.

**Borderline accepté** — Les notes « la référence lit les 2 bits bas de l'octet 10 » (REQ-CORE-013), « dans la référence, un dépassement arrête l'émulation » (REQ-CORE-032), « la référence effectue ici une vraie lecture de bus » (REQ-CORE-081) décrivent un **comportement observable** divergent et justifient une divergence acceptée : conformes au guide. La note de REQ-CORE-040 (« le moteur 3D de référence place les tiles d'après x et y ») est informative et inutile, mais sans portée.

**UNTESTABLE 02-2** — REQ-CORE-070 : « pas de **repliement audible** sur un balayage de fréquence du canal pulse ». « Audible » n'est pas mesurable. Proposer un seuil (p. ex. produits de repliement ≤ −60 dBFS sur un balayage 100 Hz–10 kHz).

**UNTESTABLE 02-3** — REQ-CORE-039, critère : « correspondent **visuellement** à l'image 2D d'un émulateur de référence public … à la même frame ». Verdict subjectif + ROM commerciale. Proposer un vecteur synthétique sur les bascules `$FD`/`$FE` (réalisable avec une ROM construite comme au §5.1).

**UNTESTABLE 02-4** — REQ-CORE-002 : « revue d'interface » comme seul critère d'une exigence P1.

**UNCLEAR 02-5** — REQ-CORE-037, exception sans justification :
> si L = 0 et que le mirroring courant est vertical, aucune rangée n'est émise pour cette ligne

Le guide demande qu'une règle déterminant un résultat observable porte sa justification. Ici, un implémenteur ne peut pas distinguer une règle voulue d'une erreur de transcription ; V6 la confirme mais ne l'explique pas.

**UNCLEAR 02-6** — REQ-CORE-044 : « pour MMC2 et MMC4 … les sprites d'une même capture sont émis par X croissant ; l'ordre est **non spécifié à X égal** ». Un ordre non spécifié contredit le déterminisme exigé par REQ-ARCH-012 / REQ-CORE-092 (deux exécutions doivent donner des exports identiques octet à octet). Fixer un départage (numéro OAM croissant).

**UNCLEAR 02-7** — REQ-CORE-061 renvoie la polarité de la gâchette à SPEC-08 (« Convention de l'hôte de référence (SPEC-08) ») mais **SPEC-08 ne la définit nulle part** (§3.7 et §4.1 donnent l'état logique, jamais le bit). Seule `DEC-02-Q1` la fixe. À inscrire dans SPEC-08 §4.1.

## 4. SPEC-03 — Format de profil

Document le plus solide de l'ensemble : format d'interopérabilité pur, vecteurs d'octets vérifiables, aucun nom interne non exposé. **Aucune fuite.**

**INCONSISTENT 03-1** — REQ-PROF-025 impose de refuser un profil dont les éléments `SavedPattern3D` (14) ou `SaveAnimPattern3D` (15) manquent ; `DEC-03-Q03` impose au contraire de « reconstituer 5 emplacements par défaut ». Amender REQ-PROF-025.

**INCONSISTENT 03-2** — REQ-PROF-024 dit seulement « trié par `name` croissant » ; la règle normative est REQ-LUA-004 (collation non ordinale) et §6 Q-02 propose encore l'ordinal. Renvoyer explicitement à REQ-LUA-004 (voir **T-9**).

**UNCLEAR 03-3** — Les noms de messages (`ProfileFile`, `RefEnvelope`, `ShapeParams`, `TexRect`, `TexPayload`, `RecordedSlot`, `Cell`, `TilePlanes`) ne circulent pas sur le fil protobuf : ce sont donc des noms **de spec**, pas des faits d'interopérabilité. Ils paraissent bien neutralisés, mais le document devrait le dire une fois (« noms de commodité, sans portée normative »), pour qu'un relecteur ultérieur ne les prenne pas pour des noms de types de l'original.

**UNCLEAR 03-4** — REQ-PROF-003 : « les écrivains d'origine écrivent toujours les scalaires … un scalaire absent indique donc un fichier écrit par une version plus ancienne ». Formulé comme un constat sur le producteur ; la règle normative utile (défauts du §4) est ailleurs. Sans gravité.

**UNCLEAR 03-5** — REQ-PROF-052 fait générer un `Id` à partir de « 9 octets **aléatoires cryptographiques** » ; aucune exigence ne dit que cet aléa est exclu du périmètre de déterminisme de REQ-ARCH-012. Le préciser (export hors boucle de jeu).

## 5. SPEC-04 — Pipeline de conversion

### Jugement demandé : les règles d'appariement sont-elles une spécification de résultat ?

**Verdict : ACCEPTABLES** pour REQ-E3D-090, -091, -092, -131, -160.
Motifs : (a) le résultat est intrinsèquement **dépendant de l'ordre** — la spec le dit (« le rétrécissement de C est glouton et dépend de l'ordre d'exploration : c'est voulu et nécessaire à la compatibilité des profils ») ; (b) cet ordre est **directement observable** par les scripts, puisque REQ-E3D-220 en fait l'ordre de `Frame:GetShape(i)` et de toutes les requêtes « premier élément » ; (c) le texte ne contient ni code, ni nom interne, ni structure de classes, et il est accompagné de vecteurs (TV-E3D-01..08) qui fixent le résultat. Une formulation purement déclarative perdrait de l'information normative.
Réserve de forme (non bloquante) : préférer « la shape est l'ensemble défini par le processus déterministe suivant » à une énumération d'étapes impératives numérotées, et présenter la file FIFO comme une définition plutôt que comme une procédure.

**LEAK 04-1 (bloquant, correction triviale)** — REQ-E3D-280, Notes :
> l'original utilise trois tampons de frame (conversion, affichage, épinglage). L'implémentation est libre

Architecture interne de l'original sans effet observable — le guide l'interdit explicitement, et l'exigence énonce déjà la règle observable. **Reformulation : supprimer la première phrase.**

**LEAK 04-2 (bloquant, correction triviale)** — REQ-E3D-342, Notes :
> l'original contient une affectation automatique de couches qui n'est jamais exécutée.

Description de code mort. L'observable est déjà dans l'exigence (« `Layer.Auto` n'a aucun effet »). **Reformulation : « Le réglage est conservé pour la compatibilité des données ; il n'a aucun effet. »**

**LEAK 04-3 (bloquant, correction triviale)** — REQ-E3D-207, Notes :
> l'original n'actualise jamais les frames de début/fin de vie

Révèle des champs internes (frames de naissance et de mort d'une shape) qui n'apparaissent ni dans les données conservées ni dans l'API Lua. **Reformulation : « `Age` vaut toujours 1 ; aucun profil embarqué ne lit cette valeur. »**

**Borderline accepté** — REQ-E3D-323 (« l'original reconnaît ce hook mais ne l'invoque jamais ») : `DetectShape` est un nom **visible de Lua**, et « jamais invoqué » est l'observable. Conforme.

**INCONSISTENT 04-4 (bloquant)** — Collision de notation sur `BL`/`TR`.
REQ-E3D-205 : `BL = (pStart.x, Hs − pEnd.y − 1, z0)` avec `z0 = Off(Le) + (s/2)(κ − 1)`.
§4.4 : « instance `BL` / `TR` : idem [BottomLeft/TopRight] avec y diminué de 1 », soit `z = −s/2`.
Pour L1 FRONT (`Off = 0`, `κ = 1`) : `z0 = 0` mais `−s/2 = −4` (confirmé par TV-LUA-05 : `BL = (16, 200, −4)`, alors que le critère de REQ-E3D-205 donne `BL = (16, 208, 0)`). Le même symbole désigne deux valeurs. SPEC-04B a évité le piège en nommant `Bmin`/`Bmax` (REQ-E3D-404) : adopter cette notation dans REQ-E3D-205.

**INCONSISTENT 04-5** — REQ-E3D-002 nomme les marges `(g, b, d, h)` sans jamais dire quelle composante du `Vec4` (SPEC-03 champ 4) est laquelle ; la correspondance n'existe que dans SPEC-05 §4.2, que l'implémenteur du lot L4 n'a pas de raison de lire. Voir **T-1**.

**INCONSISTENT 04-6** — REQ-E3D-021 (SPEC-04) et REQ-E3D-454 (SPEC-04B) définissent **deux fois** les classes de densité et de forme 2D, avec des libellés différents (`AVERAGE`/`MEDIUM` d'un côté, « médiane »/« moyenne » de l'autre). Les valeurs numériques coïncident, les noms non. De plus la note de REQ-E3D-454 affirme qu'elles « servent à la conversion (SPEC-04 : couches automatiques) », alors que REQ-E3D-342 établit que les couches automatiques n'ont aucun effet et que REQ-E3D-021 dit que la forme CHAR ne sert qu'au mode 2D. Garder une seule définition (SPEC-04) et y renvoyer.

**UNCLEAR 04-7** — REQ-E3D-300 (tracker). Le parcours est annoncé sur « les shapes **de la couche** dans l'ordre de la frame », mais le cas 1 ajoute à E « tous les patterns non inclus des shapes de la frame d'indice ≤ celui de s (**toutes couches**) ». Mélanger un parcours restreint à une couche et une indexation globale est un artefact de boucle, pas une règle de résultat : soit E accumule au fil d'un parcours **toutes couches**, soit la clause « toutes couches » est à supprimer. À reformuler en énoncé de résultat (quel ensemble E après la frame).

**UNCLEAR 04-8 (frôle la transcription)** — REQ-E3D-381 (mode 2D) :
> extraire la première bande contiguë de lignes entièrement pleines …, libérer les autres tiles, étendre la bande par 4-voisinage …

et sa note : « les tiles libérées et déjà dépassées dans l'ordre de germination ne sont pas réexaminées dans cette frame (non affichées) ; comportement d'origine, non critique ». C'est la seule règle d'appariement qui décrit un **effet de bord d'implémentation** (position d'un curseur de boucle) explicitement qualifié de non critique. L'exigence étant P3, proposer : spécifier le résultat (quelle bande est retenue) et déclarer **non spécifié** le sort des tiles libérées.

**UNCLEAR 04-9** — REQ-E3D-361 : « le transfert … DEVRAIT être limité à un budget de temps par frame de rendu » — budget non chiffré (SPEC-04B REQ-E3D-600 et SPEC-07 REQ-LOOP-061 disent « la durée d'une frame de rendu cible » : à aligner).

**UNTESTABLE 04-10** — REQ-E3D-345 : « Critère d'acceptation : sans objet ». Exigence sans critère ; la transformer en note informative.

**UNTESTABLE 04-11** — REQ-E3D-007 : « en temps constant amorti » n'est pas directement observable ; ne garder que le renvoi à REQ-E3D-360.

## 6. SPEC-04B — Géométrie

### Jugement demandé 1 : REQ-E3D-446 à 449 (cylindres)

**Verdict : ACCEPTABLES comme spécification de résultat, avec réserve de forme.**
- REQ-E3D-446 décrit une **récurrence entière** (tracé d'ellipse par point milieu en coordonnées doublées), algorithme **public** et nommé comme tel dans les notes. Le jeu de voxels produit est une discrétisation particulière qu'aucune formulation « c'est une ellipse » ne permettrait de retrouver : l'information est donc **nécessaire au résultat**, et les vecteurs TV-Y1..Y8 la verrouillent. Pas de code, pas de nom interne.
- REQ-E3D-449 référence « l'algorithme de **Bresenham classique** » avec sa paramétrisation (erreur initiale `⌊Δmaj/2⌋`, axe majeur, sens de parcours) : même analyse. Le document prend soin de séparer ce qui est normatif (la **forme**, P1) de ce qui ne l'est que partiellement (la **couleur** des voxels intérieurs, P2, écart classé D).
- REQ-E3D-447/448 (masques de quadrants, choix du texel, `antiShadow`) sont des règles de résultat pur.

Réserve (non bloquante) : l'**ordre** des points de contour n'a d'effet observable que par les écrasements de REQ-E3D-423 — déjà dégradé en P2 par REQ-E3D-449. Le dire une fois explicitement dans REQ-E3D-446 (« l'ordre n'est normatif que pour la couleur des voxels recouverts ») éviterait qu'un lecteur futur lise cette section comme une procédure obligatoire.

### Jugement demandé 2 : REQ-E3D-521 (numérotation des coins en marching cubes)

**Verdict : ACCEPTABLE sur le fond, mais UNCLEAR sur son statut normatif.**
La numérotation donnée est celle des tables publiques de Lorensen–Cline transposée aux axes de la scène (coins 0–3 sur la face `β = 0`, 4–7 sur `β = 1`) : c'est un **fait de correspondance** nécessaire pour réutiliser une table publique, pas une transcription.
En revanche le texte se contredit :
> pour une correspondance triangle à triangle avec l'original, numéroter les coins … ; la triangulation des cas ambigus est **sinon libre**

alors que REQ-E3D-522 rend la **coordonnée de texture** (donc la couleur affichée) dépendante de cette numérotation :
> celui de plus **grand numéro de coin** pour une correspondance exacte

La numérotation a donc un effet observable et doit être normative, ou bien REQ-E3D-522 doit donner une règle qui n'en dépend pas. À trancher. Accessoirement, la justification « pour une correspondance … avec l'original » est à remplacer par une justification de résultat (« pour que la triangulation des configurations ambiguës soit reproductible »).

### Fuites

**LEAK 04B-1 (bloquant, correction triviale)** — REQ-E3D-603 :
> (l'original **pré-alloue 1 024 objets et 2 048 tuiles** et étend ces réserves à la demande)

Dimensionnement interne sans effet observable ; l'exigence dit déjà « NE DOIT PAS être limité par une capacité fixe ». **Supprimer la parenthèse.**

**LEAK 04B-2 (bloquant, correction triviale)** — REQ-E3D-601, Notes : « l'original construit **en tâche de fond** ». Thread interne ; l'observable (retard d'une à deux frames) est déjà dans l'exigence. **Supprimer.**

**LEAK 04B-3 (bloquant, correction partielle)** — REQ-E3D-564, Notes :
> l'original range les textures dans un **atlas de 64 couches de 512 × 512** et **échoue (exception)** quand il est plein

Les deux dimensions justifient les capacités exigées (acceptable au titre des « constantes qui déterminent un résultat observable »), mais « atlas de 64 couches » et « échoue (exception) » décrivent la structure et la gestion d'erreur internes. **Reformulation : « Ces capacités correspondent à celles de la référence, qui interrompt le chargement au-delà. »**

**LEAK 04B-4 (bloquant)** — §6 Q-2 :
> Le rendu de l'original l'applique dans un **programme de shader non lisible** ; **l'outil d'export** de l'original l'applique après la transformation complète.

Référence à deux composants internes de l'original, dont un explicitement hors périmètre (l'export de frames). **Reformulation : « L'espace d'application de la déformation n'a pas pu être déterminé en boîte noire ; REQ-E3D-568 retient “avant échelle et rotation”. »**

**LEAK 04B-5 (bloquant)** — REQ-E3D-507 :
> **Ordre d'émission GREEDY** (pour des **tampons identiques à l'original**) … l'ordre n'a pas d'effet visible

Exigence normative (P2) dont la seule justification est la correspondance avec les tampons de sommets internes de l'original. **Reformulation : rendre l'exigence informative (« l'implémentation PEUT émettre les faces dans cet ordre, ce qui facilite la comparaison automatique des exports »).**

**Borderline accepté** — REQ-E3D-523 (« l'original stocke … en flottants 16 bits ») et REQ-E3D-524 (« l'original utilise des index 16 bits ; le dépassement produit des faces erronées ») justifient respectivement une tolérance et une divergence classée D, à partir d'un effet observable. REQ-E3D-503 (« dans l'original, une position sans cellule compte comme une valeur 0 ») décrit un résultat. Conformes, à reformuler au besoin en « la référence produit … ».

**Modèle des « fentes » : accepté.** §2 et REQ-E3D-422 introduisent trois affectations par voxel (une par axe). C'est la construction la plus discutable du document, mais elle a un **effet directement observable** (quelles faces existent : REQ-E3D-422 et TV-C1 le montrent pour `CUBE`), et elle factorise six règles de géométrie. Conforme au guide (« description mathématique d'un résultat »).

### Autres

**INCONSISTENT 04B-6 (bloquant)** — REQ-E3D-405 contre `DEC-06-Q4` : voir **T-4**.

**INCONSISTENT 04B-7** — REQ-E3D-560 : « `NES_RGB` est la table … définie par **SPEC-02 (ou SPEC-06)** ». Elle n'est ni dans SPEC-02 ni dans SPEC-06 : elle est au §4.1 de **SPEC-04**, conformément à `DEC-04B-Q1`. SPEC-06 REQ-SCN-034 renvoie en plus à SPEC-04B, et SPEC-06 §5.2 à « la palette NES de SPEC-04B ». Trois renvois circulaires vers un tableau qui n'existe qu'une fois. Corriger les trois.

**INCONSISTENT 04B-8** — REQ-E3D-567, Notes (« à la toute première exécution, l'original part de “L1 projette sans recevoir” ») et SPEC-09 §4.1.6 (défauts `cast_shadow` i = 0,1 vrais ; `recv_shadow` faux pour i = 2 au premier lancement) décrivent un état que `DEC-04B-Q5` exclut explicitement de reproduire. Supprimer ces défauts de premier lancement ou les marquer « non reproduits (D) ».

**UNCLEAR 04B-9** — `fillrate` (REQ-E3D-449, §4.1) est « exécution seulement », mais il intervient à la **construction** du champ de voxels. Rien ne dit qu'il doit être posé par le hook de construction avant celle-ci : REQ-E3D-455 ne le cite pas dans sa liste, alors que REQ-LUA-030 le mentionne (« `pattern.fillrate = 0.05` … avant leur construction »). À expliciter dans REQ-E3D-455.

**UNCLEAR 04B-10** — REQ-E3D-442 : « `y*` est le plus grand entier `≤ y + k` tel que `t(x, y*) ≠ e` (lecture **modulo** la hauteur de texture) ». Avec une lecture modulo, « le plus grand entier ≤ » n'a pas de borne inférieure : le comportement n'est pas défini si la colonne est entièrement transparente (boucle infinie possible). Ajouter la clause d'arrêt.

**UNCLEAR 04B-11** — REQ-E3D-443, note : « La boîte couvre toute la grille du pattern, cases vides comprises **seulement si la tuile existe** ». Phrase non interprétable.

**UNCLEAR 04B-12** — `GetDeform`/`GetDeformation` (SPEC-05 REQ-LUA-083) n'utilisent pas le même point d'origine que la déformation réellement appliquée (REQ-E3D-568) : `Pd` vaut `deltaPivot ⊙ sz + (pStart.x, Hs − pEnd.y − h₀, −D/2)` tandis que `OriPos` vaut `BottomLeft ⊙ (1−q) + TopRight ⊙ q`, ce qui en diffère de `q ⊙ (1,1,0)` (et de `h₀`). Un script qui prédit le déplacement rendu obtient donc un résultat légèrement faux. Dire si l'écart est voulu (fidélité) ou s'il faut unifier.

**UNTESTABLE 04B-13** — REQ-E3D-643, critère : « voir SPEC-06 » (aucun critère propre). REQ-E3D-566 et REQ-E3D-453 renvoient à « l'annexe A » / « les profils de l'annexe A » sans désigner de cas précis.

## 7. SPEC-05 — Runtime Lua

Document d'interopérabilité ; les noms, alias, fautes de frappe tolérées et constantes sont **légitimes** (API visible des profils). **Aucune fuite bloquante.**

**Borderline accepté** — REQ-LUA-028, Notes : « l'original **recycle ces objets** ; une lecture tardive y renvoie des valeurs d'une autre shape ». Décrit un mécanisme interne, mais l'effet est observable et motive la divergence exigée. À reformuler en « la référence réutilise ces objets, si bien qu'une lecture tardive y renvoie … ». REQ-LUA-101 et §5.1 décrivent des constats de **boîte noire** sur la bibliothèque Lua livrée (`_VERSION`, présence de `bit`/`jit`), permis par PLAN §3.3 règle 6.

**INCONSISTENT 05-1 (bloquant)** — Ordre de restauration : voir **T-6**.

**INCONSISTENT 05-2** — REQ-LUA-095 : « `Video.SceneScale` (L : 1 ; **écriture ignorée** tant que `Setting.VrRuntime` = 0) » contre SPEC-06 REQ-SCN-002 : « Une écriture DOIT être acceptée sans effet géométrique ; **si la valeur écrite diffère de 1 … elle DOIT déclencher une réinitialisation de vue** ». Deux profils (« flappy ») écrivent 0,02 au démarrage : l'effet (caméra réinitialisée ou non) est observable.

**UNTESTABLE 05-3** — REQ-LUA-051, REQ-LUA-100, REQ-LUA-101 : voir **T-7**.

**UNTESTABLE 05-4** — REQ-LUA-111 : « ne DEVRAIENT pas allouer … au point de provoquer des pauses de ramasse-miettes visibles (une pause > 4 ms par minute de jeu est un défaut) » : critère exploitable, mais l'instrumentation exigée (« imputable à la couche Lua ») n'est pas définie.

**UNCLEAR 05-5** — REQ-LUA-004 : la collation est définie par trois classes (« ponctuation, espace et symboles < chiffres < lettres ») sans table de classement exhaustive. Les exemples couvrent l'ASCII ; le comportement pour les noms non ASCII (profils utilisateur) reste indéfini. Donner la règle complète ou restreindre explicitement la garantie à l'ASCII.

**UNCLEAR 05-6** — REQ-LUA-083 (4) : `Overlap(rect)` est défini « par rapport au rectangle d'origine `(BottomLeft.x, BottomLeft.y)` et de taille `(size.x, size.y)` », alors que `OverlapRect` est « comme `Overlap(r)` avec l'origine `BL` ». `BottomLeft` et `BL` diffèrent d'une unité en y (SPEC-04 §4.4) : confirmer que l'écart d'un pixel est voulu.

**UNCLEAR 05-7** — Le type `Instance` expose `pattern` comme « le pattern **propre à sa shape** (grille = boîte en tiles de la shape) » (§4.2). Ce « pattern propre » n'est défini nulle part ailleurs (ni dans SPEC-03, ni dans SPEC-04) : sa grille, ses variantes et sa durée de vie sont à spécifier, d'autant que `Shape.layer`, `Shape.geo` et `Shape.sizeZ` en dépendent.

## 8. SPEC-06 — Scène et rendu

**LEAK** : aucune. Les descriptions de skybox (REQ-SCN-050/051) sont des **définitions mathématiques du résultat** (couleur en fonction de la direction), explicitement autorisées ; REQ-SCN-019 nomme le filtre public (« smooth damp ») plutôt que de le présenter comme interne. REQ-SCN-008, note (« l'original ajoute un biais d'arrondi de 0,5 unité ») décrit un écart observable couvert par une tolérance : conforme.

**INCONSISTENT 06-1 (bloquant)** — Hauteur de scène et centre C : voir **T-2**.

**INCONSISTENT 06-2** — REQ-SCN-047 (« Quand le jeu n'est pas en cours (menu, pause), la scène 3D … NE DOIT PAS être affichée ») contre REQ-SCN-059 (« La caméra continue d'être lissée **et rendue** pendant les pauses »). SPEC-07 REQ-LOOP-003 et SPEC-10 REQ-UI-003 confirment que la scène est masquée en pause. Corriger REQ-SCN-059 (« continue d'être lissée »).

**INCONSISTENT 06-3** — REQ-SCN-030, note : « La réimplémentation DEVRAIT utiliser au moins **1024** texels » contre `DEC-06-Q3` : « Carte d'ombre de **512** texels par défaut ».

**INCONSISTENT 06-4** — §4.1 donne au réglage VSync le domaine « 0 désactivée, 1 activée » alors que SPEC-09 §4.1.1 et REQ-PERS-013 en définissent **trois** valeurs (0, 1, 2 = un rafraîchissement sur deux).

**INCONSISTENT 06-5** — REQ-SCN-043 redéfinit la déformation animée déjà normée par REQ-E3D-568, avec une notation différente (Π, F, Φ) et **sans** le décalage `h₀` dépendant de la version de format du profil (SPEC-03 `Ver = "2"`), qui concerne 100 profils sur 101. Désigner SPEC-04B comme normatif et réduire REQ-SCN-043 à un renvoi.

**INCONSISTENT 06-6** — REQ-SCN-034 et §5.2 renvoient la table des 64 couleurs à SPEC-04B : voir **04B-7** et `DEC-04B-Q1`.

**UNTESTABLE 06-7** — REQ-SCN-051 (skybox Fantasy) : critère « comparaison **visuelle** avec l'original sur 3 jeux … Tolérance **perceptuelle** (pas d'exigence au pixel) ». Non vérifiable par un tiers qui ne dispose pas du programme d'origine. Proposer au minimum des valeurs de référence pour quelques directions et quelques palettes de fond.

**UNTESTABLE 06-8** — §5.4 (R1–R7) : toutes les captures de référence supposent Super Mario Bros. et une comparaison avec l'original ; REQ-SCN-030 et REQ-SCN-046 également. Voir **T-8**.

**UNCLEAR 06-9** — REQ-SCN-054 cite les textures `Tv_3_4_diff/_normal`, `Tv_1_2_diff/_normal/_gloss` et la cubemap générique `Cubemap`. `INVENTORY.md` ne mentionne que « textures de la TV CRT (diffuse, normal, AO, émissive, rugosité) » et une « cubemap générique 64×64 ». Vérifier que les jeux de textures de TV 1 et TV 2 sont bien présents dans `assets-neutral/textures/`, faute de quoi REQ-SCN-054 est irréalisable pour deux de ses trois valeurs.

**UNCLEAR 06-10** — REQ-SCN-063 (facteur de résolution, P3, « PEUT ») contre SPEC-09 §4.1.1, qui présente `video.res_scale` comme un réglage utilisateur ordinaire avec libellé UI, et SPEC-10 REQ-UI-062, qui l'exclut de l'UI (`DEC-10-Q2`). Statut à unifier.

## 9. SPEC-07 — Boucle

**LEAK 07-1 (bloquant, correction triviale)** — REQ-LOOP-033, Notes :
> ce panneau existe **dans le code d'origine** mais n'est pas présent dans l'interface livrée.

Référence explicite au code de l'original — interdite par le guide (« des références au code d'origine »). **Reformulation : « L'interface livrée ne comporte pas de panneau d'historique ; l'implémentation PEUT en ajouter un. »**

**Borderline accepté** — REQ-LOOP-007 (« l'original ne conserve qu'une commande en attente ») et REQ-LOOP-032 (« le comportement de l'original dans ces cas limites est défectueux ») décrivent des comportements observables et motivent des divergences.

**INCONSISTENT 07-2 (bloquant)** — REQ-LOOP-044, étapes 4 et 5 : voir **T-6**.
**INCONSISTENT 07-3 (bloquant)** — REQ-LOOP-040 contre REQ-ARCH-013 : voir **T-5**.
**INCONSISTENT 07-4 (bloquant)** — Fréquence nominale contre cadence nominale : voir **T-3**.

**INCONSISTENT 07-5** — Avance rapide : REQ-LOOP-016 (« `k` vaut 2 tant que la hotkey est maintenue ; au relâchement, `k = 2` persiste encore pendant **30 frames** ») et SPEC-08 REQ-INP-103 (« une demande lance, **si aucune avance rapide n'est déjà en cours**, une avance rapide de 30 images ») décrivent deux mécanismes différents : dans le second, le reliquat après relâchement vaut 0 à 30 frames, pas 30. Choisir un modèle.

**INCONSISTENT 07-6** — `DEC-07-Q4` : « `retro_serialize` et `retro_unserialize` produisent et restaurent un savestate complet, **hooks Lua `Save`/`Load` compris** » contre REQ-LOOP-063 et REQ-LUA-075, qui interdisent ces hooks pour le rembobinage et le netplay. Préciser dans la décision qu'elle vise les savestates manuels.

**UNCLEAR 07-7** — REQ-LOOP-012 fixe le rattrapage à 5 frames et REQ-LOOP-011 une granularité de 1 ms, mais aucune exigence ne dit ce qui se passe quand `k` est grand (s = +8, k = 3) : la limite de 5 frames de retard est-elle exprimée en frames émulées ou en périodes `1/f` ? T5 suppose `f` courant ; à expliciter.

## 10. SPEC-08 — Entrées

**LEAK** : aucune.

**INCONSISTENT 08-1** — REQ-INP-041, Notes : « une implémentation **DEVRAIT** alors la retirer de l'autre joueur » alors que `DEC-08-1` en fait une divergence **acceptée**, donc normative : passer à DOIT.

**INCONSISTENT 08-2** — REQ-INP-060 : « Le type de chaque port vaut `Manette` au démarrage du programme et n'est pas persisté », sans rien dire du changement de jeu ; `DEC-09-Q6` impose le retour à `Manette`/`Manette` « à chaque chargement d'un jeu **différent** » (D). À reporter dans REQ-INP-060 et dans SPEC-09 REQ-PERS-012, qui ne mentionne que le démarrage du programme.

**INCONSISTENT 08-3** — Avance rapide : voir **07-5**.

**UNCLEAR 08-4** — La **polarité du bit de gâchette** du Zapper n'est définie nulle part dans SPEC-08, alors que SPEC-02 REQ-CORE-061 la lui attribue (« Convention de l'hôte de référence (SPEC-08) »). Seule `DEC-02-Q1` la fixe (bit 4 = 1 gâchette relâchée). À inscrire au §4.1.

**UNCLEAR 08-5** — REQ-INP-063 : la visée est obtenue par intersection avec « le **plan de l'image NES** », exprimée « en pixels NES : x de 0 à 256, y de 0 (haut) à 240 », sans donner la conversion depuis l'ordonnée de scène. Avec Hs = 248 (voir **T-2**), les formules `py = Hs − 1 − y` et `py = Hs − y` ne donnent pas le même pixel. À expliciter.

**UNCLEAR 08-6** — REQ-INP-100 évalue les hotkeys « à chaque image **affichée** » alors que REQ-INP-005 échantillonne les entrées « avant chaque image **émulée** ». Les deux horloges diffèrent (60/90/120 Hz contre ≈ 60,1 Hz) : préciser que les fronts montants de hotkeys sont détectés sur l'horloge d'affichage et n'entrent pas dans le périmètre de déterminisme de REQ-ARCH-012.

**UNCLEAR 08-7** — `DEC-06-Q6` prévoit un arbitrage « clic Zapper contre caméra **pour le même bouton** », mais SPEC-08 attribue des boutons différents (droit pour la gâchette, REQ-INP-062 ; gauche pour l'orbite, REQ-INP-091). La décision est sans objet : le signaler pour éviter qu'un implémenteur cherche un conflit inexistant.

## 11. SPEC-09 — Persistance

**LEAK** : aucune au sens strict. §4.4 (« Disposition observable de l'original ») décrit des **fichiers et une clé de registre visibles par l'utilisateur** — observation en boîte noire autorisée par PLAN §3.3 règle 6, et le document la marque « informatif ». Réserve de forme : la ligne « `Player.log` | journal du **moteur** » laisse transparaître le moteur d'origine sans aucune utilité ; écrire « journal d'exécution ».

**INCONSISTENT 09-1 (bloquant)** — `clip.top` / `clip.bottom` : voir **T-1**.
**INCONSISTENT 09-2 (bloquant)** — REQ-PERS-056 contre REQ-ARCH-013 : voir **T-5**.

**INCONSISTENT 09-3** — REQ-PERS-057 : « l'onglet Save [liste] **les 36 slots** » contre SPEC-10 REQ-UI-051 : « l'onglet SAVE DOIT lister **le premier slot vide** …, suivi de tous les slots non vides » (confirmé par V4). Aligner SPEC-09 sur SPEC-10.

**INCONSISTENT 09-4** — REQ-PERS-071 (racine portable = `userdata\`) contre REQ-ARCH-033 (« à côté de l'exécutable ») : voir **01-2**.

**INCONSISTENT 09-5** — §4.1.6 : défauts d'ombre « au premier lancement » contraires à `DEC-04B-Q5` : voir **04B-8**.

**INCONSISTENT 09-6** — REQ-PERS-066 (`ui.last_browse_dir`, P3, « DEVRAIT ») contre SPEC-10 REQ-UI-105 (P2, « DOIT être mémorisé de façon persistante »).

**UNCLEAR 09-7** — §4.1.5 donne à `clip.top` le domaine `[0 ; 240]` alors que la formule de filtrage utilise 248 (`y ≤ 248 − c1`). Avec Hs = 248, le domaine devrait être `[0 ; 248]`.

**UNCLEAR 09-8** — REQ-PERS-005 impose de remplacer « **tous** les réglages de portée P » par ceux du profil, alors que SPEC-03 REQ-PROF-041 en donne une **liste explicite** et énumère ce qui est ignoré. Les deux ensembles semblent coïncider, mais aucune exigence ne l'affirme. Ajouter un renvoi croisé normatif (« la portée P du §4.1 est exactement la liste de REQ-PROF-041 »).

**UNCLEAR 09-9** — REQ-PERS-003, règle 2 : « `emu.core` → **reste de la division par le nombre de cœurs disponibles** ». Avec un seul cœur la règle est triviale ; avec zéro cœur elle est indéfinie. Sans portée pratique, mais à borner.

## 12. SPEC-10 — UI

**LEAK 10-1 (mineur, non bloquant)** — REQ-UI-011 (« référence : 118 contre 96 unités, pour une hauteur de 38 ») et REQ-UI-122 (« le cadre du menu mesure environ 768×675 unités du **canevas de 1200×725** ») donnent les dimensions du canevas d'interface de l'original. Ces grandeurs sont mesurables sur une capture, et le document déclare la disposition « indicative » et l'exigence P3 : pas de fuite caractérisée, mais la notion de « canevas » relève de la construction interne de l'UI d'origine, déclarée hors périmètre (« on ne vise pas l'identité au pixel près », §1). Reformulation suggérée : exprimer ces proportions en fractions de la surface logique de référence 800×450 (REQ-UI-130).

**INCONSISTENT 10-2 (bloquant)** — REQ-UI-010 et REQ-UI-002 : « Au premier affichage après le lancement du programme, l'onglet actif est `SETTING` » contre `DEC-10-Q6` : « Onglet initial : **GAME** (D) ».

**INCONSISTENT 10-3 (bloquant)** — REQ-UI-039 (bouton **Site**, info-bulle `Go to custom profile website`), repris par REQ-UI-140 et REQ-UI-122, contre `DEC-10-Q5` : « Bouton “Site” **retiré** (D) ». Retirer les trois mentions.

**INCONSISTENT 10-4** — REQ-UI-076 décrit un bloc « à propos » de **trois** lignes (produit, version, auteur) et laisse le contenu ouvert (Q-3), alors que `DEC-10-Q3` impose l'identité de la reconstruction (« Sen »), la **licence GPL v3** et les crédits à 3dSen et à 3DNes_Engine (Geod Studio) — ce qui n'entre pas dans trois lignes. À réécrire.

**INCONSISTENT 10-5** — Texte exact divergent entre REQ-UI-084 (« `<chemin>has an obsolate…` (sans espace après le chemin) ») et SPEC-09 REQ-PERS-041 (« `<chemin> has an obsolate…` »). Les deux documents présentent ces chaînes comme normatives : il faut une seule source.

**INCONSISTENT 10-6** — REQ-UI-051 et V4 contre REQ-PERS-057 : voir **09-3**.

**INCONSISTENT 10-7** — REQ-UI-105 (P2, DOIT) contre REQ-PERS-066 (P3, DEVRAIT) : voir **09-6**.

**UNTESTABLE 10-8** — REQ-UI-120 et REQ-UI-122 : « Critère d'acceptation : **revue visuelle** » pour deux exigences P2/P3 ; acceptable si la traçabilité les marque comme telles.

**UNCLEAR 10-9** — REQ-UI-002 affirme « et aucune autre transition n'existe », alors que REQ-UI-007 (lancement par `-id=`), REQ-UI-035 et REQ-UI-095 introduisent d'autres chemins. Assouplir la clause de fermeture.

**UNCLEAR 10-10** — REQ-UI-062 exclut de l'UI `Graphic` (qualité graphique), mais SPEC-09 §4.1.1 lui donne un libellé UI « Graphic : Low/Medium/High » et une exposition libretro, et `DEC-09-Q1` conserve sa portée P. Préciser : réglage conservé dans le modèle, absent de l'UI desktop.

## 13. SPEC-11 — Audio

**LEAK 11-1 (mineur)** — §6 Q5 : « Clip joué par les curseurs Vitesse et FOV des réglages (**liaison définie dans la scène, argument non lisible**) ». Révèle le mode de câblage interne de l'UI d'origine. **Reformulation : « Le clip associé aux curseurs Vitesse et FOV n'a pas pu être déterminé en boîte noire. »**

**INCONSISTENT 11-2 (bloquant)** — Nombre d'échantillons par frame : REQ-AUD-007 attend « ≈ 798,7 trames par `retro_run` » contre les 800 de REQ-CORE-070. Voir **T-3**.

**INCONSISTENT 11-3** — REQ-AUD-020 et REQ-AUD-022 imposent une pente « `V / 200` **par frame de rendu** » (donc 3,33 s à 60 i/s, 2,2 s à 90 i/s) et proposent en note « une durée fixe ≈ 2,5 s ± 1 s », alors que `DEC-11-Q2` fixe « un fondu de **durée fixe** : 200/60 s ≈ 3,33 s ». Texte normatif et décision divergent ; le critère d'acceptation (« 3,3 s ± 0,3 s à 60 i/s ») ne les discrimine pas.

**UNCLEAR 11-4** — REQ-AUD-004 définit le rapport de rééchantillonnage à partir de « la **fréquence d'horloge de l'APU** de la région × k », alors que REQ-CORE-070 le définit à partir de la **cadence nominale** et REQ-CORE-072 à partir de `fréquence / (cadence × s)`. Trois formulations pour une même grandeur ; n'en garder qu'une (SPEC-02 étant propriétaire du cœur).

**UNCLEAR 11-5** — REQ-AUD-005 autorise un ajustement du rapport de rééchantillonnage de ± 0,5 % « sans modification audible de hauteur », mais REQ-CORE-070 fixe un calage rigide : dire explicitement que le cœur doit exposer un rapport ajustable.

## 14. SPEC-12 — Acceptation

**LEAK** : aucune.

**UNTESTABLE 12-1 (structurel)** — REQ-ACC-050 exige « aucune exigence “non testable” non justifiée » à la porte P4, alors que plusieurs dizaines d'exigences P1 ne sont vérifiables qu'avec une ROM commerciale ou avec le programme d'origine (voir **T-8**). SPEC-12 doit introduire une troisième catégorie de verdict (« vérifiable en round 2 seulement ») et la rendre compatible avec REQ-ACC-050, sinon la porte est formellement inatteignable.

**INCONSISTENT 12-2** — REQ-ACC-031 (« 4 ms **en moyenne** ») et REQ-E3D-360 (« **99ᵉ centile** ≤ 4 ms ») expriment la même cible avec deux statistiques différentes ; `DEC-04-Q06` dit « cible (P2) » sans trancher.

**UNCLEAR 12-3** — REQ-ACC-020 cite « Quarth (algorithme MARCHING) » comme jeu de référence ; l'annexe A de SPEC-04B ne liste Quarth que parmi les profils `rm = 2`. Confirmer que la ROM correspondante est attendue de l'utilisateur (voir **T-8**).

**UNCLEAR 12-4** — REQ-ACC-021 exclut « les écarts classés D » du calcul de SSIM, sans dire comment les exclure d'une mesure par image.

## 15. DECISIONS-R0

**LEAK** : aucune.

**INCONSISTENT D-1 (bloquant)** — `DEC-06-Q4` (décalage des sprites) contredit REQ-E3D-405 : voir **T-4**. Comme DECISIONS prime, c'est SPEC-04B qui doit être amendé — ou la décision révisée.

**INCONSISTENT D-2 (bloquant)** — « Correction transverse » sur Hs : règle de substitution insuffisante, voir **T-2**. En particulier, elle ne corrige pas `C = (128, 120, 0)`.

**INCONSISTENT D-3** — `DEC-09-Q2` nomme `clip.top` sans fixer son indice : voir **T-1**.

**INCONSISTENT D-4** — `DEC-06-Q3` (512 texels) contre la note de REQ-SCN-030 (≥ 1024) : voir **06-3**.

**INCONSISTENT D-5** — `DEC-07-Q4` (hooks `Save`/`Load` inclus dans `retro_serialize`) contre REQ-LOOP-063 et REQ-LUA-075 : voir **07-6**.

**UNCLEAR D-6** — `DEC-04-Q03` (« Les valeurs du masque relèvent de SPEC-02 ») et `DEC-04B-Q1` (table NES portée par SPEC-04) sont des décisions de **propriété de document** ; elles ne sont pas répercutées dans les renvois des documents concernés (REQ-E3D-560, REQ-SCN-034, SPEC-06 §5.2). À appliquer.

**UNCLEAR D-7** — `DEC-08-6` (« le réglage “bouton Menu émulé” est conservé dans le modèle mais n'a pas d'effet ») et SPEC-09 REQ-PERS-017 (« elle PEUT appliquer l'effet décrit par son libellé ») divergent : la décision interdit l'effet, la spec l'autorise.

---

## 16. Verdicts par document

| Document | Verdict | Motif principal |
|---|---|---|
| SPEC-00 | **ACCEPT WITH FIXES** | 00-1 (inversion `shape` / `shape2D`) |
| SPEC-01 | **ACCEPT WITH FIXES** | T-5 (périmètre du savestate), 01-2 |
| SPEC-02 | **ACCEPT WITH FIXES** | LEAK 02-1 (une phrase), T-3, 02-5/06/07 |
| SPEC-03 | **ACCEPT WITH FIXES** | 03-1 (contre `DEC-03-Q03`), 03-2 |
| SPEC-04 | **ACCEPT WITH FIXES** | 3 LEAK de notes (04-1/2/3), 04-4 (collision `BL`), 04-7/08 |
| SPEC-04B | **ACCEPT WITH FIXES** | 5 LEAK (04B-1..5), T-4, 04B-7, statut de REQ-E3D-521 |
| SPEC-05 | **ACCEPT WITH FIXES** | T-6, 05-2, T-7 |
| SPEC-06 | **REJECT** | T-2 : repère et constantes de scène incohérents dans tout le document (§2, REQ-SCN-001/003/012/045/053, §4.3, §5.1, §5.3) ; + 06-2, 06-5 |
| SPEC-07 | **ACCEPT WITH FIXES** | LEAK 07-1, T-3, T-5, T-6, 07-5 |
| SPEC-08 | **ACCEPT WITH FIXES** | 08-4 (polarité Zapper non définie), 08-1/02/05 |
| SPEC-09 | **REJECT** | T-1 : `clip.top` / `clip.bottom` inversés par rapport à SPEC-04/05/06 **et** par rapport à sa propre formule ; + 09-3, T-5 |
| SPEC-10 | **ACCEPT WITH FIXES** | 10-2 et 10-3 (contraires à `DEC-10-Q5/Q6`), 10-4, 10-5 |
| SPEC-11 | **ACCEPT WITH FIXES** | 11-2 (T-3), 11-3, LEAK 11-1 |
| SPEC-12 | **ACCEPT WITH FIXES** | 12-1 (porte P4 inatteignable en l'état) |
| DECISIONS-R0 | **ACCEPT WITH FIXES** | D-1, D-2, D-3 |

## 17. Comptes par catégorie

| Catégorie | 00 | 01 | 02 | 03 | 04 | 04B | 05 | 06 | 07 | 08 | 09 | 10 | 11 | 12 | DEC | Transv. | **Total** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| LEAK | 0 | 0 | 1 | 0 | 3 | 5 | 0 | 0 | 1 | 0 | 0 | 1 | 1 | 0 | 0 | — | **12** |
| UNTESTABLE | 0 | 0 | 3 | 0 | 2 | 1 | 2 | 2 | 0 | 0 | 0 | 1 | 0 | 1 | 0 | 2 | **14** |
| INCONSISTENT | 1 | 2 | 0 | 2 | 3 | 3 | 1 | 6 | 5 | 3 | 6 | 6 | 2 | 1 | 5 | 6 | **52** |
| UNCLEAR | 1 | 1 | 3 | 3 | 3 | 4 | 3 | 2 | 1 | 4 | 3 | 2 | 2 | 2 | 2 | 0 | **36** |

Les points transverses T-1 à T-8 sont comptés une fois dans la colonne « Transv. » et rappelés, sans être recomptés, dans chaque document concerné.

## 18. Liste des items bloquants

**Fuites (12) — toutes corrigeables par suppression ou reformulation d'une note, sans toucher aux exigences :**

1. **02-1** — SPEC-02 REQ-CORE-120 : mention d'une bibliothèque native livrée et jamais chargée.
2. **04-1** — SPEC-04 REQ-E3D-280 : « l'original utilise trois tampons de frame ».
3. **04-2** — SPEC-04 REQ-E3D-342 : affectation automatique de couches « jamais exécutée » (code mort).
4. **04-3** — SPEC-04 REQ-E3D-207 : « frames de début/fin de vie » jamais actualisées (champs internes).
5. **04B-1** — SPEC-04B REQ-E3D-603 : pré-allocation de 1 024 objets et 2 048 tuiles.
6. **04B-2** — SPEC-04B REQ-E3D-601 : « l'original construit en tâche de fond ».
7. **04B-3** — SPEC-04B REQ-E3D-564 : « atlas de 64 couches » et « échoue (exception) ».
8. **04B-4** — SPEC-04B §6 Q-2 : « programme de shader non lisible », « l'outil d'export de l'original ».
9. **04B-5** — SPEC-04B REQ-E3D-507 : exigence normative justifiée par « des tampons identiques à l'original ».
10. **07-1** — SPEC-07 REQ-LOOP-033 : « ce panneau existe dans le code d'origine ».
11. **11-1** — SPEC-11 §6 Q5 : « liaison définie dans la scène, argument non lisible ».
12. **10-1** *(mineur, à arbitrer)* — SPEC-10 REQ-UI-011/122 : dimensions du canevas d'UI d'origine.

**Incohérences bloquantes (12) :**

13. **T-1 / 09-1 / D-3** — Sens des composantes de `Clipping` (SPEC-09 contre SPEC-04/05/06 et contre sa propre formule).
14. **T-2 / 06-1 / D-2** — Hs = 248 non répercuté dans SPEC-06 (plan NES, rectangle visible, centre `C`, tunnel TV, tous les vecteurs §5.1 et §5.3).
15. **T-3 / 07-4 / 11-2** — Cadence hôte 60 fps (SPEC-02) contre 60,0988 Hz (SPEC-07/SPEC-11) ; 800 contre 798,7 échantillons par frame.
16. **T-4 / 04B-6 / D-1** — Décalage ±0,1 des sprites : REQ-E3D-405 contre `DEC-06-Q4`.
17. **T-5 / 01-1 / 09-2** — Périmètre du savestate : REQ-ARCH-013 contre REQ-LOOP-040 et REQ-PERS-056, avec conséquence sur le déterminisme (REQ-ARCH-012, REQ-ACC-040).
18. **T-6 / 05-1 / 07-2** — REQ-LOOP-044 restaure `PData` avant de recharger le profil, qui le remet à zéro (REQ-LUA-011/072).
19. **04-4** — Collision de notation `BL` / `TR` entre REQ-E3D-205 et SPEC-04 §4.4 (confirmée par TV-LUA-05).
20. **03-1** — REQ-PROF-025 contre `DEC-03-Q03`.
21. **10-2** — Onglet initial SETTING contre `DEC-10-Q6` (GAME).
22. **10-3** — Bouton « Site » maintenu contre `DEC-10-Q5` (retiré).
23. **04B-7 / 06-6 / D-6** — Renvois circulaires vers la table des 64 couleurs NES (SPEC-04B → SPEC-02/06, SPEC-06 → SPEC-04B) alors que `DEC-04B-Q1` la place dans SPEC-04.
24. **08-4** — Polarité du bit de gâchette du Zapper : SPEC-02 la délègue à SPEC-08, qui ne la définit pas.

**Point de statut à trancher (non comptabilisé comme fuite) :**

25. **REQ-E3D-521 / REQ-E3D-522** — La numérotation des coins de cellule est déclarée facultative (« sinon libre ») alors que REQ-E3D-522 fait dépendre la coordonnée de texture du « plus grand numéro de coin ». Rendre la numérotation normative, ou rendre REQ-E3D-522 indépendante.

## 19. Points explicitement validés (pas de fuite)

- **SPEC-04B REQ-E3D-446 à 449 (cylindres)** : spécification de résultat recevable. Les récurrences entières sont celles d'algorithmes **publics** (ellipse par point milieu, Bresenham), nommés comme tels, et la discrétisation produite est l'information normative elle-même. Pas de code, pas de nom interne, résultat verrouillé par TV-Y1..Y8.
- **SPEC-04B REQ-E3D-521 (marching cubes)** : la numérotation des coins est une **mise en correspondance** avec les tables publiques Lorensen–Cline, donc un fait d'interopérabilité et non une transcription (statut normatif à clarifier, point 25 ci-dessus).
- **SPEC-04 REQ-E3D-090/091/092/131/160 (règles d'appariement)** : spécification de résultat recevable, car le résultat dépend de l'ordre d'exploration et que cet ordre est observable depuis Lua (REQ-E3D-220).
- **SPEC-04B §2, modèle des « fentes » par axe** : abstraction de spécification à effet observable direct, et non une structure de données recopiée.
- **SPEC-03 dans son ensemble** : format d'interopérabilité (numéros de champs, clé d'encodage, conventions de fil), explicitement autorisé.
- **SPEC-05 §4** : noms, alias et constantes de l'API Lua — contrat d'interopérabilité ; les constats de dialecte sont des observations de boîte noire autorisées.
- **SPEC-06 REQ-SCN-050/051** : définitions mathématiques d'une image en fonction de la direction de vue ; REQ-SCN-019 nomme un filtre public.
- **SPEC-09 §4.4** : disposition de fichiers observable par l'utilisateur (réserve de forme sur « journal du moteur »).

---

*Rapport produit à partir de la seule spécification, de `PLAN.md` et de `assets-neutral/INVENTORY.md`. Aucun fichier de spec n'a été modifié.*
