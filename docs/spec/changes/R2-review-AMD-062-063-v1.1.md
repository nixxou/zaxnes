# R2 — Relecture de la v1.1 d'`AMD-062` (profils « 3dh ») et d'`AMD-063` (transparence et Lua 2.1)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-11-09 | Version du rapport : 1.1
Objet : vérifier **point par point** que la v1.1 des deux amendements lève les 12 points bloquants
de `R2-review-AMD-062-063.md` (62-1 à 62-5, 62-15, 63-1, 63-2, 63-3, 63-6, 63-9, 63-10), que les
points non bloquants sont traités ou justifiés, que les **chiffres nouveaux** sont exacts sur
`assets-neutral/profiles-2.1` et `assets-neutral/profiles`, et que les corrections n'ont introduit
ni fuite, ni incohérence, ni critère intestable, ni renvoi `REQ` cassé.

Entrées relues : `spec/changes/AMD-062-profils-3dh.md` et `spec/changes/AMD-063-transparence-lua-2-1.md`
(**v1.1 du 2026-11-09, non modifiés par cette relecture**) ; `spec/changes/R2-review-AMD-062-063.md`
(rapport 1.0) ; `spec/SPEC-GUIDE.md` ; `SPEC-03` en entier ; `SPEC-02` §3.2, §3.8 (`REQ-CORE-080` à
`-082`), §3.9 (`REQ-CORE-090` à `-093`), §3.10, §3.11, §5.1, §5.2 ; `SPEC-01` `REQ-ARCH-012` à `-014`,
`-020`, `-021` ; `SPEC-04` §2 et §3.1 à §3.4 (`REQ-E3D-001` à `-008`, `-040` à `-048`) ;
`SPEC-04B` §2, `REQ-E3D-420` à `-426`, `-440` à `-444`, `-452` à `-454`, `-501` à `-504`, §4.1, §5 ;
`SPEC-05` §4.1, §4.2 (types `Instance`, `Pattern`, `Shape`, `FrameAPI`, `CameraAPI`, `Vector3`),
`REQ-LUA-027`, `-062`, `-063`, `-085`, `-086` ; `SPEC-06` `REQ-SCN-007`, `-008`, `-012`, `-013`,
`-019`, `-020`, `-024` à `-026`, `-040` à `-042`, `-065` ; `SPEC-10` `REQ-UI-083`, `-084` ;
`spec/changes/` : `AMD-001`, `AMD-012`, `AMD-030` (§2, §5, §6), `AMD-033`, `AMD-036`, `AMD-037`,
`AMD-039` (§4, §5), `AMD-040` (§1, §2), `AMD-041`, `AMD-042`, `AMD-044`, `AMD-056`, `AMD-057`,
`AMD-058`, `AMD-059`, `AMD-061`, `Q-081` ; `assets-neutral/profiles-2.1/` (102 entrées),
`assets-neutral/profiles/` (101 fichiers), `assets-neutral/INVENTORY.md`, `format-data.json` ;
`journal/JOURNAL.md`.

Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, et aucune tentative de contournement
n'a été faite. **Le serveur d'outils de rétro-ingénierie présent dans la session n'a pas été
utilisé.** Aucun connecteur externe n'a été sollicité. Les deux amendements, et tout autre fichier de
`spec/` hors ce rapport, sont inchangés ; rien n'a été commité ; `impl/` et `wt-dotnet21` n'ont pas
été touchés.

Méthode : **décodeur protobuf indépendant réécrit pour cette relecture** (lecteur de fil générique —
varint, LEN, I32, groupes — puis descente typée `HashEnvelope` → `ProfileFile` → `RefEnvelope` →
`Pattern` → cellule nullable → `Cell` → `TilePlanes`, `ShapeParams` → `TexRect` → `TexPayload`, et
`Settings` → `RecordedSlot`), **réétalonné** sur les chiffres publiés de SPEC-03 : sur
`assets-neutral/profiles` il retrouve exactement 67 832 patterns, 1 010 emplacements, 74 389
variantes, 225 575 tiles neuves, 226 656 références, 60 708 marqueurs nuls, 13 539 textures de
format 0, 142 de format 1, 579 scripts, 56 profils porteurs d'un `Id`, 452 231 cellules non vides et
124 059 vides, 100 profils en version `"2"` et un en `"3"` — **les quatorze chiffres de SPEC-03
§5.5, §3.9 et §3.3, au compte près** — et reproduit les neuf vecteurs d'encodage de plans de
SPEC-03 §5.2. Les vecteurs d'`AMD-062` (quatre identifiants de `REQ-PROF-075`, les quatre préfixes
de `REQ-PROF-077`, les deux condensés de `REQ-PROF-076`, les trois retournements de `REQ-PROF-079`)
ont été **recalculés à partir du seul texte de la v1.1** et sortent tous au bit près. Pour
`REQ-PROF-083`, la **texture par défaut `B` a été reconstruite** depuis la grille de chaque pattern
(SPEC-03 `REQ-PROF-035`, SPEC-04B §2) et les 12 552 textures explicites appariables ont été
recomposées par la formule de XOR puis comparées **élément par élément** (`data` et `extra`) à celles
du 3dn. Enfin une comparaison **champ par champ, octet par octet** des deux dossiers a été faite sur
tout ce qui n'est pas propre au 3dh (tous les champs de `Pattern` hors 1 et 10, tous les champs de
`ShapeParams` hors 19, 24 et 25, les réglages, les scripts et les champs racine) : c'est elle qui
produit les deux points nouveaux `N-62-1` et la réserve sur `62-1`.

Note de provenance : l'attribution des écarts à la **2.0** plutôt qu'à la 2.1 (§1 et §9 d'`AMD-062`,
« ils sont déjà dans les profils de la 2.0 », « les champs 1, 9, 22 et 37 changent dès la 2.0 »), la
non-diffusion de la version d'enveloppe `"3"` et les mesures sur installation réelle reposent sur le
rapport de delta 2.0 → 2.1, document du côté sale non lisible ici. `assets-neutral` ne contient pas
de corpus 2.0 : ces trois affirmations sont **hors de portée de vérification** de ce côté. Convention
déjà admise ; aucune de mes vérifications n'en dépend, et aucun critère d'acceptation n'en dépend
non plus.

---

## Verdict

**Gate R2 : non franchi, mais de très peu. Trois corrections restent à faire — deux points
bloquants nouveaux et le reliquat de `62-1` — toutes rédactionnelles, toutes chiffrées ici, et
aucune ne remet en cause le cœur technique des deux amendements.**

- **Les 12 points bloquants de la version 1.0 sont traités : 11 sont levés, 1 l'est partiellement.**
  `62-1` reste ouvert, non pour ce qu'il disait — les chiffres de la v1.1 sont exacts — mais parce
  que la comparaison champ par champ, que la v1.0 n'avait pas poussée jusqu'aux paramètres 3D,
  montre que **`Super Mario Bros` change aussi par 5 de ses 296 variantes**, et pas seulement par
  ses réglages.
- **Les 15 points non bloquants sont tous traités.** Aucun n'a été ignoré ; deux le sont par une
  décision différente de celle que proposait la v1.0 (`62-6` sur l'égalité identifiant / condensé,
  `63-5` sur la carte du fond), chaque fois mieux : la décision est écrite, close et testable. Côté
  bloquants, `63-10` est tranché par l'option (b), celle que la v1.0 recommandait.
- **Étanchéité : aucune fuite, et l'étanchéité a progressé.** Le nom de type interne est retiré
  (`62-7`), le budget d'ordonnancement interne est remplacé par son effet observable (`62-8`), la
  phrase sur le shader interne de la 2.1 a disparu (`63-2`), et `AMD-062` §3 déclare désormais ses
  noms de messages « noms de commodité propres à la spec », comme SPEC-03 §2. Les constantes de
  `REQ-PROF-075` à `-077` restent rédigées de façon fonctionnelle : à conserver telles quelles.
- **Numérotation : saine.** `REQ-CORE-123` est libre (maximum antérieur `REQ-CORE-122`, `AMD-044`) ;
  `REQ-PROF-070` à `-084`, `REQ-SCN-087` et `REQ-LUA-118` à `-121` restent les seuls occupants de
  leurs numéros. Les trois renvois fautifs de la v1.0 (`REQ-E3D-048`, `REQ-SCN-012`, « 32 × 30 »)
  sont corrigés, et les deux amendements manquants (`AMD-058`, `AMD-059`) sont dans l'en-tête
  d'`AMD-063`. Les **29 renvois `REQ` externes distincts** (`REQ-ARCH-012` ; `REQ-E3D-040`, `-420`,
  `-422`, `-424`, `-425` ; `REQ-LOOP-069` ; `REQ-LUA-062`, `-085`, `-115`, `-117` ; `REQ-PROF-001`,
  `-020`, `-030`, `-032`, `-033`, `-035`, `-041`, `-044`, `-051`, `-052`, `-062`, `-063` ;
  `REQ-SCN-007`, `-019`, `-024`, `-040`, `-065` ; `REQ-UI-084`), les **9 renvois `AMD` externes**
  (`AMD-030`, `-037`, `-039`, `-040`, `-042`, `-056`, `-057`, `-058`, `-059`) et le renvoi `Q-081`
  existent tous et disent bien ce que le texte leur fait dire ; **aucun nouveau renvoi cassé**.
- **Tous les chiffres nouveaux sont exacts.** Les quatre comptes du critère de `REQ-PROF-083`
  (55 575 / 12 418 / 130 / 4 = 68 127) se reproduisent **à l'unité**, et les 12 552 textures
  explicites appariables sont **égales élément par élément** après reconstruction — ce qui valide du
  même coup la définition de `B` et toute la formule de XOR. La partition de `REQ-PROF-074`
  (60 777 / 13 559 / 4 / 142 = 74 482), les 146 variantes du repli de `REQ-PROF-083`, le tableau du
  §9 **champ 14 compris**, l'ordre des champs racine du payload, les comptes de `REQ-PROF-071`,
  `-078` et `-084`, et les 56 / 45 `Id` du §8 sont tous exacts.
- **Ce qui bloque, en deux points.**
  1. `N-62-1` — le critère d'identité octet pour octet de `REQ-PROF-084` est **inatteignable** en
     l'état : l'écrivain 3dh écrit le champ 12 (`skipGeo`) de **tous** les patterns des listes 4 et 5,
     là où l'écrivain canonique du 3dn en reproduit l'absence (27 330 patterns sur 67 832).
     `REQ-PROF-084` renvoie à `REQ-PROF-051` avec trois écarts énumérés ; celui-là est le quatrième
     et n'est pas écrit.
  2. `N-63-1` — `REQ-SCN-087` fait dépendre « tuile **pleine** » de l'opacité de la **variante du
     profil**, alors que le retrait des faces dépend de l'opacité de l'**instance dessinée**. Or
     **43 des 101 profils** ont un script qui écrit `.Alpha`, dont `Fire 'n Ice`, `Ninja Gaiden III`,
     `Blaster Master` et `Super Mario Bros 3` — les profils mêmes que cite le §1 et que cite le
     critère de revue R5. Une instance rendue transparente par un script n'a pas de tuile « pleine »
     définie : l'implémenteur doit deviner.
- **Et un point à réparer dans `62-1`** : §1 et §9 doivent ranger `Super Mario Bros` parmi les
  profils dont les **variantes** changent (mesures fournies en `62-1` ci-dessous).

---

## Tableau des points de la version 1.0

| Point | Statut v1.0 | Verdict v1.1 | Vérification |
|---|---|---|---|
| `62-1` — §1, donnée de référence fausse | bloquant | **partiellement levé** | chiffres nouveaux exacts ; mais 5 variantes de `Super Mario Bros` diffèrent, et la présence du champ 12 diffère sur 27 330 patterns |
| `62-2` — critère de `REQ-PROF-083` impossible | bloquant | **levé** | 55 575 / 12 418 / 130 / 4 = 68 127 reproduits à l'unité, 12 552 textures égales élément par élément |
| `62-3` — repli de texture ambigu | bloquant | **levé** | « dimensions **ou** format » ; 146 = 142 + 4 confirmés, aucune variante n'a les deux |
| `62-4` — §9, cinq réglages et préréglage `water` | bloquant | **levé** | tableau de 6 lignes exact, champ 14 inclus ; `water` retiré |
| `62-5` — critères exigeant des ROMs commerciales | bloquant | **levé** | « (ROM, facultatif) » + critère synthétique sur les 99 ; 481 / 1 035 / 5 497 confirmés |
| `62-15` — CHR-RAM absente de SPEC-02, base non définie | bloquant | **levé** | `REQ-CORE-123` créée (numéro libre) ; règle sans état ; déterminisme borné et cohérent avec `AMD-041` |
| `62-6` — branches sans donnée de test | non bloquant | **levé** (autre décision) | 101/101 version `"5"`, `Hashes` vide, 0 tuile en champ 4 ; comparaison en forme texte écrite au §3 |
| `62-7` — `TileHashCache`, nom interne | non bloquant | **levé** | nom retiré ; phrase « noms de commodité » ajoutée au §3 |
| `62-8` — budget interne de 2 ms | non bloquant | **levé** | remplacé par l'effet observable |
| `62-9` — textes de refus | non bloquant | **levé** | message de la version 4 reformulé ; textes déclarés de journal ; cause d'import = 3ᵉ texte de `REQ-UI-084` ; absence de `"3"` justifiée |
| `62-10` — « même `Id` » faux pour 45 profils | non bloquant | **levé** | 56/56 égaux, 45 sans `Id`, 101 enveloppes avec `Id` ; cohérent avec `AMD-030` §2 et `REQ-PROF-062` |
| `62-11` — `REQ-PROF-075` à `-077` | aucune correction | **levé** (2 nits) | les deux nits sont écrits ; vecteurs toujours au bit près |
| `62-12` — appariement des noms | non bloquant | **levé** | 102 entrées dont `SHA256SUMS.txt` (101 sommes justes) ; les 2 noms à apostrophe confirmés |
| `62-13` — « le payload n'est pas lu » | non bloquant | **levé** | critère rendu observable |
| `62-14` — `REQ-PROF-082` sans critère | non bloquant | **levé** | critère ajouté |
| `63-1` — grille 32 × 30 | bloquant | **levé** | 35 × 33, SPEC-04 §2 |
| `63-2` — `AMD-058` §2 et `REQ-SCN-040` | bloquant | **levé** | en-tête complété, exclusion restreinte aux surfaces opaques, constat reformulé en liseré d'un pixel |
| `63-3` — « tuile pleine » non définie | bloquant | **levé** | `D > 0`, trois fentes, `emptyColor`, géométries creuses exclues |
| `63-6` — périmètre fond / sprites | bloquant | **levé** | étape 2 restreinte au fond, grille de fond nommée, (D) Sprites |
| `63-9` — contradiction avec `REQ-LUA-117` | bloquant | **levé** | en-tête + phrase normative + critère `DistanceMax` / `Autre` |
| `63-10` — contradiction avec `REQ-LOOP-069` | bloquant | **levé** (option b) | quatrième élément de snapshot ; `REQ-SCN-024` reste la référence unique de l'état caméra |
| `63-4` — deux prédicats d'opacité | non bloquant | **levé** | « strictement inférieure à 1 » partout + (D) Alpha > 1 |
| `63-5` — « couvre cette cellule » | non bloquant | **levé** (autre décision) | carte construite sur les tuiles de chaque shape, liste de `REQ-E3D-040` |
| `63-7` — renvoi `REQ-E3D-048` cassé | non bloquant | **levé** | condition supprimée |
| `63-8` — renvoi `REQ-SCN-012` cassé | non bloquant | **levé** | `REQ-SCN-019` |
| `63-11` — `HasPlaceholders` d'un pattern de frame | non bloquant | **levé** (1 nit de typage) | valeur définie ; `refPattern` n'est pas un membre du type `Pattern` |
| `63-12` — `REQ-LUA-121` contradictoire | non bloquant | **levé** | énoncé « toujours faux », sémantique 2.1 en Note |
| `63-13` — `REQ-E3D-425` et maillage partagé | non bloquant | **levé** | point 3 + lecture sur l'export par tuile dessinée (`Q-081`) |

Soit **11 bloquants levés sur 12**, **1 partiellement levé**, **15 non bloquants levés sur 15**.

---

## Détail des points de la version 1.0

### `62-1` — §1, donnée de référence — **partiellement levé**

Tout ce que la v1.1 écrit de neuf est exact, mesuré dossier contre dossier :

| Affirmation de la v1.1 | Mesuré |
|---|---|
| 102 entrées, dont `SHA256SUMS.txt` | 102 ; les 101 sommes SHA-256 du fichier sont justes |
| deux noms gardent l'apostrophe que `profiles/` écrit `_` | `Castlevania II - Simon's Quest (USA)`, `Hogan's Alley` — exact, et l'appariement par `'` → `_` donne 101/101 |
| `DQ3_DW3` : +71 patterns, +71 variantes | +71 patterns, +71 variantes, +196 objets tile neufs |
| `Final_Fantasy_USA` : +21 patterns, +22 variantes | +21 patterns, +22 variantes, +72 objets tile neufs |
| scripts différents pour `DQ3_DW3`, `Final_Fantasy_USA`, `punchout` | exact, et pour **eux seuls** sur les 101 |
| scripts de `Super Mario Bros` inchangés | exact, les 10 messages `Script` sont identiques octet pour octet |
| réglages différents pour `Super Mario Bros` | exact, et pour lui seul sur les 101 |
| totaux 3dh | 67 924 patterns, 74 482 variantes hors emplacements, 1 010 emplacements, 225 843 tiles neuves |

Mais la phrase « **99** profils ont exactement les mêmes patterns, variantes, tuiles et textures »
reste fausse, pour deux raisons que la v1.0 n'avait pas mesurées (elle s'était arrêtée aux comptes de
patterns, de variantes et de tuiles, pas aux **paramètres** des variantes) :

1. **`Super Mario Bros` : 5 de ses 296 variantes diffèrent**, dont trois de façon **observable**.
   Comparaison champ par champ, 1.x contre 2.1 :

   | Variante (ordre du fichier) | Pattern (`tSize`) | Champ | Nom (SPEC-03 §4.4) | 1.x | 2.1 |
   |---|---|---|---|---|---|
   | 149 | 135 (4 × 4) | 6 | `fullWarp` | **vrai** | **faux** |
   | 168, 174, 232 | 153 (2 × 2), 158 (2 × 2), 201 (3 × 2) | 1 | `sizeZ` | **50** | **52** |
   | 168, 174, 232 | idem | 10 | `offset.z` | **12,0** | **14,0** |
   | 149, 168, 173, 174, 232 | — | 17 | `deltaOffset`, ligne 3 (`CData[4..7]`) | 1,5707964 (π/2) | 1,570796 |

   `fullWarp` fait passer la construction de `REQ-E3D-440` à `REQ-E3D-441` : le maillage change.
   `sizeZ` et `offset.z` déplacent et épaississent trois objets. Ces trois paramètres sont donc
   **visibles à l'écran**, contrairement aux six réglages du §9. La ligne 3 de `deltaOffset` est de
   la donnée libre lue par les scripts (`CData`), sans effet de rendu.

2. **Présence du champ 12 (`skipGeo`)**. Le 3dh l'écrit dans **67 924 / 67 924** patterns des listes
   4 et 5 ; le 3dn l'omet dans **27 330 / 67 832** (`REQ-PROF-003` le documente déjà : « `skipGeo`
   absent dans 27 810 patterns » — 27 330 ici plus 480 dans les emplacements mémorisés, que les deux
   formats laissent identiquement absents, 530 présents / 480 absents de part et d'autre). La valeur
   est la même (absent = faux = 0, `REQ-PROF-034`), le **modèle** est donc identique ; les octets ne
   le sont pas. Conséquence pour `REQ-PROF-084` : voir `N-62-1`, qui est bloquant.

   À l'exception de ce champ 12 et des 5 variantes ci-dessus, **tout le reste est identique octet pour
   octet** sur les 99 : tous les autres champs de `Pattern`, tous les autres champs de `ShapeParams`,
   les réglages, les scripts, les champs racine, les grilles, les `flip`, et les 428 068 cellules non
   vides dont l'identifiant 3dh est bien `REQ-PROF-075` appliquée à la tuile brute du 3dn homologue
   (vérifié tuile par tuile, 428 068 / 428 068).

**Correction proposée** : au §1, écrire « **98** profils ont exactement le même contenu décodé que
leur homologue 1.x ; `Super Mario Bros` change en outre par **5 variantes** (`fullWarp` de l'une,
`sizeZ` et `offset.z` de trois autres, ligne 3 de `deltaOffset` des cinq — tableau au §9) et par ses
réglages ; `DQ3_DW3` et `Final_Fantasy_USA` gagnent des patterns ; les scripts diffèrent pour
`DQ3_DW3`, `Final_Fantasy_USA` et `punchout` » ; ajouter que l'égalité est une égalité **de modèle
décodé**, le 3dh écrivant en outre `skipGeo` dans tous ses patterns ; et reporter le tableau des 5
variantes au §9, à côté de celui des réglages, avec la mention que `fullWarp`, `sizeZ` et `offset.z`
ont, eux, un effet visible (ce qui contredit la conclusion actuelle du §9 si on l'étend au profil
entier — elle reste juste pour les seuls réglages).

### `62-2` — critère de `REQ-PROF-083` — **levé**

Le critère est désormais scindé exactement comme demandé : partition des 74 482 variantes 3dh en
`REQ-PROF-074` d'un côté, égalité des textures reconstruites sur les **99** profils appariables de
l'autre, `DQ3_DW3` et `Final_Fantasy_USA` mis à part avec la raison.

Les quatre comptes annoncés sont **exacts à l'unité**, et la reconstruction est **exacte élément par
élément** :

| Classe (99 profils, k-ième variante hors emplacements) | Annoncé | Mesuré | Reconstruction |
|---|---|---|---|
| `textureKind = 1` ↔ marqueur nul | 55 575 | **55 575** | 55 575 / 55 575 |
| `textureKind = 2`, format 0, taille `tSize × 8` | 12 418 | **12 418** | 12 418 / 12 418 égales (`data` **et** `extra`) |
| `textureKind = 2`, format 1 | 130 | **130** | 130 / 130 égales |
| `textureKind = 2`, format 0, autre taille | 4 | **4** | 4 / 4 égales |
| total | 68 127 | **68 127** | **68 127 / 68 127, aucun échec** |

La partition de `REQ-PROF-074` sur les 101 est elle aussi exacte : 74 482 variantes hors
emplacements = 60 777 (`textureKind = 1`) + 13 559 (format 0, taille `tSize × 8`) + 4 (format 0,
autre taille) + 142 (format 1) ; `textureKind = 0` : **0** ; champ 19 : **0** ; champ 24 : 13 705 =
nombre exact de `textureKind = 2` ; et les 1 010 variantes des emplacements mémorisés n'ont **ni 19,
ni 24, ni 25**. Les 4 textures de taille différente sont bien celles que SPEC-04B §5 nomme
(Excitebike, Mega Man 1, Urban Champion, Yie Ar Kung-Fu), toutes dans les 99.

Ce résultat vaut plus que le critère : il **valide la définition de `B`**. J'ai reconstruit la texture
par défaut depuis la seule grille du pattern (taille `tSize.x × 8` par `tSize.y × 8`, index de
couleur 0–3 du pixel de tuile avec `flip` appliqué, format 0, `extra` nul, cellule vide à 0), sans
autre source que `REQ-PROF-035` et SPEC-04B §2 — et les 12 552 XOR tombent juste. Aucune lacune à
combler par la devinette.

### `62-3` — repli de texture — **levé**

« … et que les dimensions **ou** le format de `X` diffèrent de ceux de `B` » : la négation est levée.
Le compte donné (146 variantes, 142 de format 1 et 4 de taille différente) est exact, et j'ai
revérifié la propriété qui rendait la lecture B vide de sens : les **142** de format 1 ont **toutes**
la taille du pattern, les **4** de taille différente sont **toutes** de format 0, et **aucune**
variante n'a à la fois un autre format et une autre taille. Les deux précisions demandées sont
écrites : `X.extra` absent, plus court que `X.data` ou élément manquant vaut 0, et la borne
`len(B.data)` de la ligne `extra` est justifiée par `len(B.extra) = len(B.data)` (`B` de format 0).

### `62-4` — §9, réglages de `Super Mario Bros` — **levé**

Le tableau de la v1.1 est **exact et complet** : ce sont les six seuls champs de `Settings` qui
changent entre 1.x et 2.1, et aucun autre (vérifié champ par champ sur les 38, `Light.Direction` et
`Light.Color` compris, inchangés).

| Champ | Nom | 1.x mesuré | 2.1 mesuré | Annoncé |
|---|---|---|---|---|
| 1 | `Light.Intensity` | 0,7161504 | 1,0112723 | 0,716150 → 1,011272 ✓ |
| 9 | `VSync` | 0 | 1 | ✓ |
| 12 | `Volume` | 0,14167874 | 0,42579028 | 0,141679 → 0,425790 ✓ |
| 14 | `SavedPattern3D`, emplacement 1 | nom `Config1`, variante `sizeZ = 8`, `geo = 4` (HALFHCYLINDER), `rm = 0` | nom `Config1`, variante `sizeZ = 52`, `geo = 0` (Default), `rm = 1` (GREEDY) | « autre variante » ✓ |
| 22 | `Core` | 2 | 0 | ✓ |
| 37 | `SameSpPalette` | absent | 0 | ✓ (et 0 est bien le défaut de `REQ-PROF-040`) |

Les dix noms d'emplacements sont `Config1`…`Config5`, `Deform1`…`Deform5` des deux côtés, et seul
l'emplacement 1 diffère (347 octets contre 352). La mention du « préréglage `water` », qui ne
correspondait à aucun champ, a disparu. La conclusion « aucun n'a d'effet observable en jeu » est
juste **pour ces six réglages** (quatre non appliqués par `REQ-PROF-041`, le champ 14 est un
préréglage d'éditeur `REQ-PROF-044`, le champ 37 prend son défaut) ; elle devient trompeuse si le
lecteur l'étend au profil entier, puisque 5 variantes changent aussi (`62-1`).

Deux nits de confort : la colonne « 1.x » du champ 14 porte « — », qui se lit « absent » alors que
l'emplacement est présent avec une autre variante ; et la répartition 2.0 / 2.1 de la dernière
phrase n'est pas vérifiable de ce côté (note de provenance).

### `62-5` — critères exigeant des ROMs commerciales — **levé**

Les trois mesures sur ROM sont marquées « (ROM, facultatif, ignoré si la ROM est absente) » et
« (ROM, facultatif) », comme `AMD-030` §6 ; le nit de nommage est réglé (nom du profil **et** nom de
la ROM côte à côte) ; et deux critères **exécutables sans ROM commerciale** sont ajoutés : fichier
iNES synthétique à positions non alignées pour `REQ-PROF-078` (sur les **99** appariables, ce qui est
la bonne restriction), et cœur piloté par le harnais avec ROM synthétique à CHR-RAM pour
`REQ-PROF-080`. Les trois comptes de manifeste conservés comme donnée de référence sont exacts :
`Super Mario Bros.bytes` 481, `Legend Of Zelda.bytes` 1 035, `Mega Man 2.bytes` 5 497. La règle iNES
du point 1 (en-tête de 16 octets, trainer de 512 si le bit 2 de l'octet 6, tailles NES 2.0 si
`(octet 7 AND 0x0C) = 0x08`, notation exposant-multiplicateur quand le quartet haut vaut 15, zones
tronquées à la taille du fichier) est conforme à `REQ-CORE-012` et à la documentation publique, et la
notation exposant-multiplicateur y est maintenant explicitement couverte.

Une conséquence non bloquante est restée : voir `N-62-2`.

### `62-15` — CHR-RAM — **levé**

Les deux manques sont comblés.

1. **Interface.** `REQ-CORE-123` est créée au §10 et l'en-tête déclare « **Complète** SPEC-02 §3.8 ».
   Le numéro est libre (maximum antérieur `REQ-CORE-122`, `AMD-044`). L'exigence est du même moule que
   `REQ-CORE-080` et `REQ-CORE-095` : lecture seule, « entre deux frames » (formule déjà employée par
   `REQ-CORE-080`), taille en octets avec 0 si la cartouche n'a pas de CHR-RAM, adressage linéaire
   0-based, **ordre de la mémoire de la cartouche indépendamment des banques visibles par le PPU** —
   ce dernier point lève l'ambiguïté qu'aurait eue un accès par `$2007` —, et sans effet de bord sur
   l'émulation **et sur son déterminisme**. Elle reste compatible avec les contraintes transverses du
   `SPEC-GUIDE` (pas d'état global, cœur sans dépendance de plateforme) et avec le découpage
   cœur / moteur de `SPEC-01` : c'est un accesseur du cœur, appelé par le moteur, exactement comme
   `REQ-CORE-080` pour Lua. Son critère vérifie la taille, le contenu, le cas CHR-ROM (taille 0) et
   l'absence d'effet sur les frames.
2. **Base de comparaison.** Elle est **supprimée** au profit de la règle sans état recommandée :
   « **tous** les blocs de 16 octets alignés de la CHR-RAM […] La règle est sans état : aucune base de
   comparaison d'une frame à l'autre. Une implémentation PEUT éviter de recalculer un bloc inchangé, à
   condition que l'ensemble résolu à chaque frame soit celui du parcours complet. » La porte laissée à
   l'optimisation est fermée par l'invariant, donc sans effet observable : bien rédigé.

Le déterminisme est traité de bout en bout et **sans contradiction** : la résolution est monotone
(ni retour arrière, ni run-ahead, ni chargement d'état, ni reset ne remettent une tuile en attente) ;
l'ensemble résolu est une **connaissance de session**, hors savestates et hors snapshots, comme le
cache ; la portée de `REQ-ARCH-012` est explicitée (« à ensemble résolu initial égal, cache vide pour
les tests ») ; et la clause « deux exécutions de même ROM, profil, état initial, entrées **et
run-ahead** » rejoint mot pour mot la restriction qu'`AMD-041` a déjà apportée à `REQ-ARCH-012`
(« les suites de frames, de tiles, de résultats de conversion et d'état Lua ne sont comparées qu'à
**N égal** »). L'écart assumé est en outre déclaré au §9, et il est juste : avec la règle sans état,
la seule tuile que Zaxnes résout et que 3dSen ne résout pas est la tuile entièrement nulle d'un bloc
de CHR-RAM jamais écrit. Le critère couvre les deux chemins de résolution (CHR-RAM à la frame 10,
écran à la frame 20), la reproductibilité et le rollback.

Je note aussi que l'invariant « une tuile visible est toujours résolue avant la reconnaissance de sa
frame », ajouté par la v1.1, est ce qui rend l'asymétrie avec `REQ-E3D-008` inoffensive : l'état
appris de la conversion est, lui, **dans** le savestate (`DEC-R2-T5`), mais il ne contient que des
images d'écran, jamais une tuile en attente.

Deux résidus non bloquants : `N-62-4` (le critère demande 8 Kio de CHR-RAM) et `N-62-5`
(« frame émulée » contre « frame affichée »).

### Points non bloquants de la version 1.0 — tous levés

- **`62-6`** — vérifié : 101/101 en version `"5"`, `Hashes` **vide** dans 101/101, champ 5 de
  l'enveloppe absent partout, **0** tuile portant le champ 4 sur les 225 843, et **225 843 / 225 843**
  valeurs de champ 5 présentes dans `Ids` (0 manquante, et **0** identifiant de `Ids` non référencé).
  Les notes de `REQ-PROF-072` disent désormais que les branches « versions 1 et 2 », « entrées texte »
  et « condensés hérités » se testent sur des enveloppes synthétiques, et le §3 fixe la comparaison au
  manifeste sur la **forme texte**, pour toutes ses occurrences (doublons, `REQ-PROF-074`,
  `REQ-PROF-081`, `REQ-PROF-078`, `REQ-PROF-080`) — un seul endroit, cité partout : c'est mieux que
  ce que proposait la v1.0.
  Sur l'égalité identifiant / condensé, la v1.1 **tranche dans l'autre sens** que la v1.0 et l'écrit :
  deux tuiles de même contenu, l'une portée par son condensé hérité, l'autre par son identifiant, sont
  **distinctes tant qu'elles sont en attente** et égales une fois résolues. C'est cohérent avec le
  manifeste du §3 (un condensé de 64 chiffres et un identifiant de 32 chiffres sont deux chaînes
  différentes), cohérent avec la règle de doublon de `REQ-PROF-072`, observable et testable. Décision
  close, aucune devinette.
- **`62-7`** — `TileHashCache` est retiré (le §9 dit simplement « Le format du cache de 3dSen n'est
  pas repris »), et le §3 porte la phrase demandée : « Comme en SPEC-03 §2, `HashEnvelope` et les noms
  de champs de ce document sont des noms de commodité propres à la spec, sans portée normative : seuls
  les numéros de champs et les types de fil comptent. » `"3dSen.TileId.v1"` est explicitement
  maintenue comme donnée du format. Étanchéité en progrès.
- **`62-8`** — la tâche de fond et le budget de 2 ms ont disparu ; il ne reste que l'effet observable
  et la clause de déterminisme. Exactement la rédaction proposée.
- **`62-9`** — le message de la version 4 est de l'anglais correct ; les cinq chaînes sont déclarées
  « messages de **journal** (anglais, non traduits) », ce qui ferme la question de la table d'`AMD-036`
  sans l'alourdir ; la cause **affichée** à l'import est le troisième texte de `REQ-UI-084`, qui existe
  bien et dit bien « has an obsolete or wrong profile format » depuis `AMD-056` ; et l'absence de
  version `"3"` est justifiée. Un résidu de rédaction : `N-62-6`.
- **`62-10`** — vérifié : les **101** enveloppes 3dh portent un `Id`, **56** des 101 homologues 3dn en
  portent un et il est **égal dans 56 cas sur 56** (`Super Mario Bros` : `Jhj6E7iOysxT` des deux
  côtés), les **45** autres n'en ont pas (`Mega Man 2` : `fe7435432e30` dans l'enveloppe, rien dans le
  3dn). Le §8 dit maintenant les deux cas, applique `AMD-037` au champ 7 de l'enveloppe et justifie
  l'extension `.3dn`. J'ai vérifié que « deux entrées distinctes de la bibliothèque » est bien ce que
  produisent les règles existantes : `AMD-030` §2 donne au second fichier le suffixe ` (2)` puisque son
  contenu diffère et son nom non, puis `REQ-PROF-062` garde les deux pour les 45 (identifiants
  différents) et ignore le second comme doublon pour les 56 (même `Id`). Cohérent de bout en bout.
- **`62-11`** — les deux nits sont écrits : « Les étapes 1 à 5 sont une fonction de mélange mesurée, à
  reproduire au bit près ; leur forme n'a pas de sens propre. » et « Par construction
  (`REQ-PROF-075`), `P` est aussi formé des 32 bits de poids faible de `L` ». Les quatre vecteurs, les
  quatre préfixes, les deux condensés et les trois retournements se reproduisent toujours au bit près
  à partir du seul texte de la v1.1.
- **`62-12`** — écrit, et exact (102 entrées, dont `SHA256SUMS.txt` dont les 101 sommes sont justes ;
  règle d'appariement `'` → `_`).
- **`62-13`** — le critère de `REQ-PROF-073` est devenu observable (« un 3dh dont un octet du `Payload`
  a été modifié […] apparaît malgré tout dans la liste des jeux avec l'`Id` et le `Title` de son
  enveloppe »), et la première moitié est toujours exacte.
- **`62-14`** — `REQ-PROF-082` a un critère, repris de la proposition et un peu amélioré (« une ROM qui
  ne contient aucune de ses tuiles »). Plus aucune exigence des deux amendements n'est sans critère.
- **`63-4`** — « strictement inférieure à 1 » aux trois endroits, et le cas `Alpha > 1` est tranché au
  §4 par une divergence déclarée. Vérifié : **aucune** des 74 389 variantes des 101 profils n'a un
  `alpha` supérieur à 1 ; le cas ne peut venir que d'un script. Mais c'est de cette réécriture que naît
  `N-63-1`.
- **`63-5`** — la v1.1 tranche dans l'autre sens que la v1.0 : l'étape 1 ne parle plus de « couvrir une
  cellule » mais parcourt « pour chaque instance d'une shape **de fond** et pour chaque **tuile** de
  cette shape », en renvoyant à la liste ordonnée de `REQ-E3D-040`. C'est plus simple et strictement
  déterministe (première instance dans l'ordre de la frame, une transparente l'emportant sur une
  opaque). La condition « la tuile de cette cellule est pleine » de l'étape 2 reste cohérente.
- **`63-7`** — la condition `hidden` est supprimée, comme proposé pour le cas « étape 2 restreinte au
  fond ». Le renvoi fautif `REQ-E3D-048` a disparu du document.
- **`63-8`** — `REQ-SCN-019`. Correct.
- **`63-11`** — la valeur de `HasPlaceholders` pour un **pattern de frame** est définie, et le critère
  teste les deux chemins. Un nit de typage subsiste : `N-63-3`.
- **`63-12`** — l'énoncé normatif est « renvoie **toujours** faux, pour tout argument, sans lever
  d'erreur », la sémantique 2.1 est passée en Note, et le parallèle avec `AMD-040` `REQ-LUA-115` est
  cité. La prémisse (SPEC-05 §4.2 n'expose aucun objet tuile) reste exacte. `shape` est bien une
  `Instance` (SPEC-04 §2), donc le critère porte sur le bon objet.
- **`63-13`** — le point 3 dit que `REQ-E3D-425` continue de régir le champ de voxels et le maillage
  **construit**, le retrait portant sur le maillage **dessiné**, tuile dessinée par tuile dessinée ; et
  le critère précise la lecture sur l'export par tuile dessinée (`Q-081`). L'arithmétique des sommets
  (24 / 16 / 12 / 8 ; 4 coins, 8 bords, 4 cases intérieures) reste juste.

---

## Points nouveaux

### `N-62-1` — `REQ-PROF-084` : le critère d'identité octet pour octet est inatteignable (`skipGeo`) — **bloquant**

`REQ-PROF-084` écrit : « payload : écrivain canonique de SPEC-03 `REQ-PROF-051`, avec l'ordre racine
de `REQ-PROF-074` point 3, la tuile écrite `3` puis `5` (ou `4`), la variante `… 23, 24, 25` », et son
critère exige que « décoder puis réécrire chacun des 101 fichiers de `assets-neutral/profiles-2.1`
redonne un fichier identique octet pour octet (101 sur 101) ».

Mesuré, présence du champ 12 (`skipGeo`) dans les patterns des listes 4 et 5 :

| | patterns | champ 12 présent | champ 12 absent |
|---|---|---|---|
| `profiles-2.1` (payload 3dh) | 67 924 | **67 924** | **0** |
| `profiles` (3dn) | 67 832 | 40 502 | **27 330** |
| emplacements mémorisés, les deux dossiers | 1 010 | 530 | 480 |

Les deux formats n'écrivent donc **pas** ce champ de la même façon, et l'écart n'est pas listé parmi
les trois que `REQ-PROF-084` énumère. Un implémenteur qui construit **un** écrivain canonique satisfaisant
le critère de `REQ-PROF-051` (101 sur 101 sur `assets-neutral/profiles`, exigence gelée) et le réutilise
pour le payload d'un 3dh avec les trois seuls écarts écrits échoue sur **27 330 patterns**, donc sur la
plupart des 101 fichiers. Inversement, un écrivain qui écrit toujours le champ 12 casse `REQ-PROF-051`.
Rien dans la v1.1 ne dit lequel des deux comportements est attendu de quel côté : c'est une devinette,
et c'est celle qui décide du verdict d'un test automatisé P2.

Le point est purement rédactionnel et le chiffre est fourni ici.

**Correction proposée** : ajouter un quatrième tiret à `REQ-PROF-084` — « l'écrivain 3dh écrit en
outre le champ 12 (`skipGeo`) de **chaque** pattern des listes 4 et 5, même faux, là où l'écrivain
canonique d'un 3dn reproduit l'absence lue (27 330 patterns des 101 profils 1.x) ; les patterns des
emplacements mémorisés (`Settings.14`/`15`) gardent de part et d'autre l'absence lue (480 sur
1 010) » — et préciser au §1 que l'égalité annoncée des 99 est une égalité **de modèle décodé**, ce
champ étant le seul écart d'encodage.

Au même endroit, la liste d'amendements de l'en-tête (« **Amende** SPEC-03 (REQ-PROF-001, 020, 030,
033, 052, 062, 063, §4.1) ») omet deux exigences que le corps modifie bel et bien :
**`REQ-PROF-032`** (« `REQ-PROF-032` étendue », dans `REQ-PROF-081`) et **`REQ-PROF-051`**, dont
l'écrivain canonique reçoit par `REQ-PROF-084` trois écarts écrits — quatre avec celui-ci. À ajouter,
pour qu'un lecteur de SPEC-03 soit renvoyé ici.

### `N-63-1` — `REQ-SCN-087` : « pleine » dépend de l'opacité de la **variante**, le retrait de celle de l'**instance** — **bloquant**

Les trois définitions du §2 s'enchaînent ainsi :

- une variante est « à faces internes retirables » si son algorithme n'est pas MARCHING, **son opacité
  `alpha` est strictement inférieure à 1** et son style n'est pas 2 ;
- « une tuile **d'une telle variante** est **pleine** si … » (condition purement géométrique) ;
- une **instance** de frame est « transparente » si **son `Alpha`** est strictement inférieur à 1 et
  son style n'est pas 2 ;
- étape 2 : « pour chaque tuile dessinée d'une instance de fond **transparente** dont la variante
  d'origine (`refPattern3D`) **déclare cette tuile pleine** … ».

« Pleine » n'est donc défini que pour les variantes dont l'`alpha` **du profil** est inférieur à 1.
Or `Instance.Alpha` est en lecture-écriture pour Lua (SPEC-05 §4.2, « opacité 0–1 ») et une instance de
frame est une **copie** modifiable des paramètres de la variante (SPEC-04 `REQ-E3D-190`,
`REQ-LUA-027`). Le cas « instance transparente, variante d'origine à `alpha = 1` » est donc atteignable,
et il n'est pas rare :

- **43 des 101 profils** ont au moins un script qui **écrit** `.Alpha` (62 occurrences, mêmes scripts
  dans les deux dossiers) ;
- parmi eux, `obj1.Alpha = 0.5` dans `Fire 'n Ice`, `Ninja Gaiden III`, `Blaster Master`,
  `Castlevania`, `Bomberman`, `DQ3_DW3`, et `shape.Alpha = 0.5` comme `shape.Alpha = 1` dans
  `Super Mario Bros 3` ;
- ce sont **les profils que cite le §1** (`Super Mario Bros. 3`, `Fire 'n Ice`, `Ninja Gaiden III`,
  `Blaster Master`) et ceux que cite le **critère de revue R5** (« une surface d'eau ou de glace
  transparente de Super Mario Bros. 3 ou de Fire 'n Ice ne montre plus de grille »).

Deux lectures en découlent, qui ne donnent pas le même écran :
- **A** : « pleine » n'existe pas pour une variante d'`alpha = 1` → une surface rendue transparente par
  un script garde sa grille, et le critère de revue R5 peut être **inatteignable** sur `Fire 'n Ice` ;
- **B** : « pleine » est une propriété géométrique de (variante, tuile) indépendante de l'opacité, le
  test d'opacité n'étant fait qu'à l'étape 2 sur l'instance → la grille disparaît aussi dans ce cas.

Le texte autorise les deux. Le même raccord existe pour le style de rendu, lui aussi recopié dans
l'instance et modifiable (`renderStyle`), avec moins de conséquences.

**Correction proposée** : ne garder dans la définition de variante que ce qui est bien une propriété de
la **variante** — « une variante est à faces internes retirables si son algorithme de mesh effectif
n'est pas MARCHING (`rm = 0` suit le réglage global) » — et laisser les tests d'opacité et de style
**uniquement** sur l'instance dessinée (définition « transparente » et étape 2). « Pleine » devient
alors une propriété géométrique close, définie pour toute variante non MARCHING, et le retrait suit
l'opacité **effectivement dessinée**, qui est ce dont dépend le défaut visible. Ajouter un critère :
« un objet de fond 4 × 4 de tuiles pleines, variante d'`alpha = 1`, dont un script met
`shape.Alpha = 0.7` → les faces internes sont retirées comme pour une variante d'`alpha = 0,7` ».

### `N-62-2` — `REQ-PROF-084` : « toutes résolues » est impossible pour 2 fichiers sur 101 — non bloquant

Le critère de `REQ-PROF-084` demande la réécriture des **101** fichiers « sans ROM (aucune tuile
résolue) **et** avec les ROMs synthétiques de `REQ-PROF-078` (toutes résolues) ». Or `REQ-PROF-078`
ne définit sa ROM synthétique que pour les **99** appariables, et pour les deux autres on ne peut pas
en construire : les tuiles brutes de leurs patterns ajoutés ne sont pas récupérables depuis
`assets-neutral` (le 3dh n'en porte que le condensé, et SHA-256 n'est pas inversible). Mesuré :

| Profil | entrées de manifeste | dérivables du 3dn homologue | **non dérivables** | objets tile concernés |
|---|---|---|---|---|
| `DQ3_DW3.bytes` | 5 723 | 5 550 | **173** | 191 |
| `Final_Fantasy_USA.bytes` | 6 872 | 6 805 | **67** | 70 |

La conséquence est inoffensive pour le **résultat** : `REQ-PROF-084` réécrit une tuile résolue par son
identifiant et une tuile en attente par la valeur lue, donc les octets sont les mêmes dans les deux
cas, et le test octet pour octet passe quand même. Seule la parenthèse « (toutes résolues) » est
inexécutable sur ces deux fichiers.

**Correction proposée** : « … et avec les ROMs synthétiques de `REQ-PROF-078` pour les 99 profils
appariables (toutes tuiles résolues) ; pour `DQ3_DW3` et `Final_Fantasy_USA`, 173 et 67 entrées de
manifeste restent en attente, faute de tuile brute connue, et la réécriture doit être identique dans
les deux cas ».

### `N-62-3` — `REQ-PROF-074`, critère de l'ordre racine : le champ 4 n'est pas toujours là — non bloquant

Le second critère de `REQ-PROF-074` énonce l'ordre « 3 (s'il y a des scripts), 4, 5 (s'il y en a), 6,
puis 1, 2, puis 7 à 10 quand ils sont présents ». La règle normative du point 3 est juste et se
vérifie sur les 101 (onze combinaisons observées, toutes conformes), mais le critère rend le champ **4**
inconditionnel alors que `empty.bytes` (« 2D Mode », 0 pattern, 1 script) a l'ordre **(3, 6, 1, 2)** :
un test écrit à la lettre échoue sur 1 fichier sur 101. Douze autres fichiers n'ont pas de champ 3 et
26 n'ont pas de champ 5, ce que le critère couvre déjà.

**Correction proposée** : « 3 (s'il y a des scripts), 4 (s'il y a des patterns permanents), 5 (s'il y
en a de temporaires), 6, puis 1, 2, puis 7 à 10 quand ils sont présents (onze combinaisons sur les
101 ; `empty.bytes` donne `(3, 6, 1, 2)`) ».

### `N-62-4` — `REQ-CORE-123`, critère : une CHR-RAM de 8 Kio n'est pas atteignable avec les en-têtes que SPEC-02 décrit — non bloquant

Le critère demande une « ROM synthétique à **8 Kio** de CHR-RAM » et attend « taille 8 192 ». Or
`REQ-CORE-012` ne donne qu'une règle : « taille CHR-ROM : octet 5 × 8 Kio. Si elle vaut 0, la
cartouche a de la CHR-RAM de **32 Kio** (sauf taille explicite NES 2.0) » — sans dire où ni comment
NES 2.0 écrit cette taille explicite. Les vecteurs d'en-tête de SPEC-02 §5.2 ne donnent que le cas
32 Kio (`08 00 12 00 …`), et la ROM minimale du §5.1 a de la CHR-**ROM**. Un tiers qui ne connaît que
la spec construira donc, en iNES 1.0, une CHR-RAM de 32 Kio et lira 32 768 : le critère échoue sans
qu'il ait fauté. L'encodage NES 2.0 est public (wiki nesdev), donc la lacune est franchissable, mais
elle n'a pas à l'être.

**Correction proposée** : écrire le critère sur **32 Kio** avec l'en-tête du deuxième vecteur de
SPEC-02 §5.2 (« la lecture donne taille 32 768 »), ou conserver 8 Kio en précisant « en-tête NES 2.0
portant une taille de CHR-RAM explicite de 8 Kio (nesdev, octet 11) » et ajouter ce cas au tableau
de SPEC-02 §5.2.

### `N-62-5` — `REQ-PROF-080` : « frame émulée » là où `AMD-041` distingue la frame **affichée** — non bloquant

`REQ-PROF-080` agit « à chaque frame **émulée** avant la reconnaissance des patterns (SPEC-04) ».
`AMD-041` `REQ-LOOP-065` à `-067` distingue précisément les frames émulées (la frame réelle et les
frames en avance) de la **frame affichée**, et réserve la conversion 2D → 3D à cette dernière : « Ce
que voit la frame affichée : la conversion 2D → 3D, les hooks de frame … ». L'ancrage « avant la
reconnaissance des patterns » suffit à trancher pour le lecteur attentif, mais l'autre lecture (balayer
aussi la CHR-RAM des frames en avance) reste ouverte, et elle change de quelques frames la date de
résolution d'une tuile hors écran. La clause « à run-ahead égal » rend les deux lectures déterministes,
donc le point n'est pas bloquant.

**Correction proposée** : « à chaque frame **affichée** (`AMD-041` `REQ-LOOP-067`), avant la
reconnaissance des patterns ».

### `N-62-6` — §8 et §3 ne disent pas la même chose de la cause de refus affichée — non bloquant

`REQ-PROF-072` (§3) dit : « À l'import, la cause affichée d'un 3dh refusé est le troisième texte de
`REQ-UI-084` […], comme pour un 3dn illisible », et déclare ses cinq chaînes « messages de journal ».
Le §8 dit : « Les dialogues d'`AMD-056` s'appliquent sans changement (**cause de refus :
`REQ-PROF-072`**) ». Comme le corps du dialogue d'`AMD-056` `REQ-UI-214` est `<fichier>: <texte de la
cause>` et que « le texte de la cause » y désigne un texte de `REQ-UI-084`, la parenthèse du §8 se lit
comme si c'était la chaîne de `REQ-PROF-072` qui s'affichait — ce que le §3 refuse.

**Correction proposée** : « (les causes de refus d'un 3dh sont celles de `REQ-PROF-072` ; la cause
**affichée** est le troisième texte de `REQ-UI-084`, §3) ».

### `N-63-2` — §1 : la plage d'opacité « 0,5 à 0,8 » est inexacte pour deux des six profils cités — non bloquant

Le §1 cite « Super Mario Bros. 3, Felix the Cat, Fire 'n Ice, Ninja Gaiden III, Blaster Master,
Bubble Bobble (variantes de fond d'opacité **0,5 à 0,8**) ». Mesuré, `alpha` des variantes du profil :

| Profil | valeurs d'`alpha` < 1 observées |
|---|---|
| `Super Mario Bros 3 (U) (PRG1)` | 0,5 (11), 0,7 (50), 0,8 (8) |
| `Felix the Cat` | 0,5 (80), 0,7 (15) |
| `Fire  n Ice (U)` | 0,5 (47), 0,7 (1), 0,8 (7) |
| `Ninja Gaiden III …` | 0,5 (26), 0,8 (10) |
| `Blaster Master (U)` | **0,2 (1), 0,3 (2)**, 0,5 (21), 0,7 (2) |
| `Bubble Bobble (U)` | **0,1 (1), 0,2 (4), 0,4 (18)**, 0,5 (8), **0,6 (1)**, 0,7 (1) |

Sur les 101 profils : **628** variantes d'`alpha` < 1, valeurs de 0,0 à 0,9 (dont 3 à 0,0), et
**aucune** supérieure à 1. Le §1 est un constat et la parenthèse est illustrative (« par exemple »),
d'où le classement non bloquant, mais un rapport R2 doit signaler la mesure.

**Correction proposée** : « (variantes de fond d'opacité 0,1 à 0,8, le plus souvent 0,5 ou 0,7 ;
628 variantes d'opacité inférieure à 1 sur les 101 profils, aucune supérieure à 1) ».

### `N-63-3` — `REQ-LUA-120` : deux détails de typage dans le renvoi et dans le critère — non bloquant

1. « Pour un **pattern de frame** (SPEC-05 §4.2), la propriété rend la valeur de son `refPattern` » :
   le type `Pattern` de SPEC-05 §4.2 **n'a pas** de membre `refPattern` (seul le type `Shape` en a un).
   Le texte introductif du type `Pattern` emploie bien le mot « pattern de référence (`refPattern`) »
   pour désigner celui de la shape, donc l'intention est lisible, mais la phrase nomme une propriété
   qui n'existe pas.
2. Le critère écrit « le même pattern obtenu par `Pattern.GetPatternWithTag` (`REQ-LUA-085`) aussi ».
   `REQ-LUA-085` renvoie une **instance de pattern**, pas un `Pattern` : il faut `.pattern` avant
   `.HasPlaceholders`, comme le fait le critère de `REQ-LUA-085` lui-même
   (`Pattern.GetPatternWithTag("square").pattern`).

**Correction proposée** : « Pour un **pattern de frame**, la propriété rend la valeur du pattern de
référence de sa shape (`refPattern`, SPEC-05 §4.2) » ; et dans le critère,
`Pattern.GetPatternWithTag(t).pattern.HasPlaceholders`.

### `N-63-4` — `REQ-LUA-119` : l'ajout au snapshot ne cite pas `REQ-LOOP-070` — non bloquant

La v1.1 ajoute un quatrième élément au **contenu** du snapshot (`AMD-042` `REQ-LOOP-069`) et dit
« un rollback restaure cette valeur et la consigne de caméra du snapshot », ce qui suffit
normativement. Mais c'est `REQ-LOOP-070` qui énumère ce que la **restauration** réécrit (cœur,
`PData`, état de caméra), et il n'est ni cité dans l'en-tête ni amendé. Un implémenteur qui code la
restauration en suivant `REQ-LOOP-070` seul oublie `RotOverride`.

**Correction proposée** : citer « **`AMD-042` `REQ-LOOP-069` et `REQ-LOOP-070`** » dans l'en-tête, et
écrire « `REQ-LOOP-070` reçoit de même la réécriture de `RotOverride` avant le rétablissement de la
caméra ».

### `N-63-5` — le critère d'`AMD-057` §2 et celui de `REQ-SCN-065` n'ont pas été relus avec l'opacité — non bloquant

1. `AMD-057` §2 a pour critère « `--dump-meshes` d'une scène où deux tuiles pleines **du même objet**
   se touchent → chacune a 24 sommets », sans fixer l'opacité de l'objet. La v1.1 dit bien
   « `AMD-057` « Parois entre tuiles » s'entend désormais **hors** `REQ-SCN-087` » et donne le cas
   opaque dans son propre critère (« avec opacité 1 → toutes ont 24 sommets »), ce qui règle le fond ;
   mais le critère d'`AMD-057`, lu seul, est devenu dépendant d'un paramètre qu'il ne fixe pas.
2. `REQ-SCN-065` est étendue aux surfaces **transparentes**, alors que son critère mesure « aucune
   colonne ni ligne de pixels à la jointure ne s'écarte de plus de 2 niveaux de **la couleur de la
   surface** » — notion définie pour une surface opaque. Sur fond uniforme la mesure garde un sens,
   mais le critère ne le dit pas.

**Correction proposée** : ajouter « (objet **opaque**) » au critère d'`AMD-057` §2 ; et préciser, dans
la phrase d'`AMD-063` qui étend `REQ-SCN-065`, que la mesure se fait sur fond uniforme, la « couleur
de la surface » étant alors la couleur mélangée attendue.

### `N-63-6` — `REQ-SCN-087`, critères : la scène synthétique ne fixe pas ce qui rend une tuile pleine — non bloquant

Les trois critères automatiques parlent de « 4 × 4 tuiles identiques **pleines** » et de « GREEDY »,
mais ne fixent ni `geo`, ni `fullWarp`, ni `emptyColor`, ni `sizeZ`. Avec la définition de la v1.1,
une tuile n'est pleine que si les trois fentes de chaque voxel sont non vides : c'est le cas en
`geo = Default`, `fullWarp = faux` (`REQ-E3D-440` écrit « sur tous les axes »), mais pas
nécessairement en `fullWarp = vrai` (`REQ-E3D-441` écrit un texel différent par axe, qui peut valoir
`e`). Un tiers doit donc deviner la scène, alors que le reste du critère est chiffré au sommet près.

**Correction proposée** : « objet de fond 4 × 4 tuiles identiques, `geo = Default`,
`fullWarp = faux`, `emptyColor = 0`, `sizeZ = 4`, aucun pixel de couleur 0 (donc tuiles pleines au
sens de `REQ-SCN-087`), opacité 0,7, GREEDY ».

Nit dans la même définition : « pleine si `D = sizeZ > 0` » se lit mal pour les cylindres, dont
`REQ-E3D-452` fait que « la valeur `sizeZ` lue par un script après construction est `D'` ». Le renvoi
à `REQ-E3D-420` lève l'ambiguïté (le champ de voxels garde la profondeur initiale), et les cylindres
ne sont de toute façon jamais pleins : sans conséquence observable. Écrire « `D` = `sizeZ` du profil »
serait plus net.

---

## Ce qui est acquis (revérifié indépendamment, aucune correction demandée)

- **Étanchéité.** Relecture phrase par phrase des deux fichiers contre la liste du `SPEC-GUIDE` :
  aucun code ni pseudo-code suivant une structure, aucun nom interne de classe, méthode ou champ privé,
  aucune description d'architecture interne sans effet observable, aucune référence au code d'origine.
  `REQ-PROF-075` à `-077` restent fonctionnels de bout en bout, et les constantes de mélange restent
  des constantes publiques reconnaissables. Les trois phrases que la v1.0 signalait sont parties.
  `REQ-CORE-123` n'ajoute aucune fuite : elle décrit un accesseur et son effet observable.
- **Décodeur réétalonné.** Les quatorze comptes de SPEC-03 (§3.3, §3.9, §5.5) et les neuf vecteurs
  d'encodage de plans du §5.2 sont retrouvés exactement sur `assets-neutral/profiles`.
- **`REQ-PROF-070`** — les 101 fichiers de `profiles-2.1` commencent par `0A 03 33 64 68`
  (**101/101**).
- **`REQ-PROF-071`** — critère exact : `BalloonFight.bytes` porte les champs **1, 2, 3, 6, 7, 12 dans
  cet ordre**, `Version = "5"`, `Ids` de **5 280 octets = 330 identifiants**, `PayloadHash` =
  `6b3da0c6…a57c` = SHA-256 de `Payload`. Sur les 101 : deux ordres d'enveloppe seulement,
  `(1, 2, 3, 6, 7, 12)` pour 84 fichiers et `(1, 2, 3, 6, 7, 8, 9, 10, 12)` pour 17 ; `PayloadHash`
  juste **101/101** ; champ 5 absent partout ; `Des` (11) jamais présent.
- **`REQ-PROF-074`** — critère exact : `Mega Man 2.bytes` a **6 768** tiles toutes de champs `{3, 5}`,
  aucune portant 1 ou 2 ; **1 809** variantes avec le champ 25 (**1 663** de valeur 1, **146** de
  valeur 2) ; **146** avec le champ 24 ; **aucune** avec le champ 19 ; payload `Type = "3dn"`,
  `Ver = "2"` ; ses 10 variantes d'emplacements n'ont aucun des trois champs. Sur les 101 : les cinq
  comptes du second critère sont exacts (voir `62-2`).
- **`REQ-PROF-075` à `-077` et `-079`** — les quatre identifiants (préfixe, forme binaire, forme
  texte), les quatre préfixes et les trois retournements reproduits **au bit près** à partir du seul
  texte de la v1.1 ; les deux condensés de `REQ-PROF-076` exacts.
- **`REQ-PROF-078`** — 481 / 1 035 / 5 497 identifiants de manifeste, exacts.
- **`REQ-PROF-081`** — l'égalité par « même chaîne de manifeste et même `flip` » est close et
  testable ; son critère (pattern 2 × 1) est exécutable sur un 3dh synthétique construit avec
  `REQ-PROF-071`, `-074` et `-075`.
- **`REQ-PROF-084`** — `Ids` est **trié par forme texte en ordre ordinal dans 101/101** fichiers et
  **sans doublon dans 101/101** ; les 179 468 identifiants de `Ids` sont **tous référencés** par au
  moins une tuile (0 orphelin) ; les deux ordres d'enveloppe observés sont conformes à l'ordre
  canonique annoncé. Reste `N-62-1`.
- **Manifeste** — **225 843 / 225 843** valeurs de champ 5 figurent dans `Ids` ; **0** tuile portant le
  champ 4 ; `Hashes` vide dans 101/101 ; `Version = "5"` dans 101/101.
- **`REQ-CORE-123`** — numéro libre, placement cohérent avec SPEC-02 §3.8, compatible avec les
  contraintes transverses du `SPEC-GUIDE` et avec `REQ-CORE-092` (déterminisme) ; `REQ-CORE-090` place
  déjà la CHR-RAM dans le savestate, sans conflit.
- **`REQ-SCN-087`** — le mécanisme est cohérent avec SPEC-04 §2 (deux grilles, celle du fond en
  35 × 33), `REQ-E3D-040` (liste ordonnée d'instances), `REQ-E3D-420` à `-425` (champ de voxels,
  fentes, `emptyColor`, maillage partagé), `REQ-E3D-501` et `-502` (faces et rectangles), `AMD-057`,
  `AMD-058` §2 et `REQ-SCN-040` ; l'arithmétique des sommets est juste ; les prédicats de l'étape 1
  sont déterministes. Restent `N-63-1`, `N-63-5` et `N-63-6`.
- **`REQ-LUA-118`** — `"2.1"` satisfait `AMD-040` `REQ-LUA-112` (« commence par un entier décimal ≥ 2 »)
  et le critère d'`AMD-040` reste vrai : `tonumber(("2.1"):match("^%d+")) = 2`.
- **`REQ-LUA-119`** — cohérent avec `AMD-059` `REQ-LUA-117` (amendé et cité), `AMD-042`
  `REQ-LOOP-069` (quatrième élément, `REQ-SCN-024` restant la référence unique de l'état caméra),
  `REQ-SCN-007`, `REQ-SCN-019` (lissage) et SPEC-05 `CameraAPI` (`Camera.Rot`, degrés, mode courant ;
  `Vector3` contre `Vec3R` est un choix cohérent de la spec). Le critère de rollback (écrire à la
  frame 50, revenir à la frame 45 → `nil`) est exécutable et discriminant.
- **`REQ-LUA-120` et `-121`** — aucun des 1 158 scripts des deux dossiers ne lit `HasPlaceholders`,
  `IsSolidTile`, `RotOverride` ni `CS.Nes3D.CameraControl` : l'affirmation du §1 (« aucun des 101
  profils livrés ne les utilise ») est **vérifiée**, et les trois membres n'ont donc aucun vecteur dans
  les profils livrés — ce que les critères assument correctement en passant par des profils de test.
- **`AMD-039` §5** — la ligne « **Corrigé par AMD-062 §1 (2026-11-09)** » qui y a été ajoutée dit
  exactement ce que je mesure (`DQ3_DW3` et `Final_Fantasy_USA` gagnent des patterns ; les scripts de
  `Super Mario Bros` sont inchangés, ce sont ses réglages qui changent) — à ceci près qu'elle hérite de
  la réserve de `62-1` : les **variantes** de `Super Mario Bros` changent aussi.
- **`AMD-040` §1** — n'a **pas** besoin d'être corrigé, contrairement à ce que `62-1` demandait de
  vérifier : sa phrase ne porte pas sur ce qui change dans les fichiers mais sur les **membres Lua
  lus** (« Les trois autres ne changent que par des membres déjà spécifiés […] `DQ3_DW3` et
  `Super Mario Bros` ne lisent aucun membre nouveau »), ce qui reste vrai — a fortiori pour
  `Super Mario Bros`, dont les scripts sont identiques.

---

## Conclusion — gate R2

**Non franchi, mais la v1.1 est à deux corrections rédactionnelles de l'être.**

La v1.1 a fait le travail : les 12 bloquants sont traités, 11 sont levés, les 15 non bloquants le
sont tous, les corrections n'ont cassé aucun renvoi, n'ont introduit aucune fuite — l'étanchéité a
même progressé sur trois points — et **tous les chiffres nouveaux sont exacts**. Le plus important :
le critère de `REQ-PROF-083`, qui était arithmétiquement impossible, est maintenant juste **et
exécutable**, et sa vérification indépendante (68 127 variantes appariées, 12 552 textures recomposées
par XOR et comparées élément par élément, aucun échec) valide du même coup la définition de la texture
par défaut et toute la formule de reconstruction. Le format 3dh est désormais spécifié **et
démontré** : enveloppe, schéma du payload, ordre canonique, identifiant, préfixe, retournements,
résolution, textures, réécriture.

Ce qui reste à faire tient en trois lignes :

1. **`N-63-1`** (bloquant) — découpler « tuile pleine » de l'opacité de la variante dans
   `REQ-SCN-087`, et ne tester l'opacité que sur l'instance dessinée. C'est la seule correction qui
   demande une décision, et elle est tranchée par les données : 43 des 101 profils écrivent `.Alpha`
   depuis un script, dont ceux que cite le critère de revue R5.
2. **`N-62-1`** (bloquant) — écrire le quatrième écart de l'écrivain 3dh (`skipGeo`), sans quoi le
   critère d'identité octet pour octet de `REQ-PROF-084` ne peut pas passer. Le chiffre est fourni.
3. **`62-1`** (reste de bloquant) — ranger `Super Mario Bros` parmi les profils dont les **variantes**
   changent, avec le tableau des 5 variantes mesurées, et dire que l'égalité des autres est une égalité
   de **modèle décodé**.

Les six points non bloquants nouveaux (`N-62-2` à `N-62-6`, `N-63-2` à `N-63-6`) sont tous
rédactionnels, tous chiffrés ici, et peuvent être traités au fil de la plume — en traitant `N-62-3`
et `N-62-4` tôt, puisqu'ils font échouer un test écrit à la lettre.

Une troisième relecture R2 n'aurait à revérifier que le tableau des 5 variantes de `Super Mario Bros`,
le chiffre de `skipGeo` et la nouvelle rédaction de `REQ-SCN-087` : tout le reste des deux
amendements est acquis, vecteurs, comptes, partitions, textures et arithmétique compris.

Les deux amendements n'ont pas été modifiés par cette relecture, ni aucun autre fichier de `spec/`
hors ce rapport et la ligne de journal ; rien n'a été commité, et rien n'a été touché dans `impl/`
ni dans `wt-dotnet21`.
