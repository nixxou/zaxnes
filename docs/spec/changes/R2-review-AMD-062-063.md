# R2 — Relecture d'`AMD-062` (profils « 3dh ») et d'`AMD-063` (transparence et Lua 2.1)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-11-09 | Version du rapport : 1.0
Objet : relire **en entier** les deux amendements du report de 3dSen 2.1, à la recherche de toute
fuite (code, nom interne du programme d'origine, algorithme recopié ligne à ligne plutôt que
décrit), de toute incohérence avec la spécification gelée et les amendements antérieurs, de tout
critère intestable, et de tout numéro `REQ` déjà pris ou renvoi cassé. Juger en particulier si
`REQ-PROF-080` à `REQ-PROF-083` laissent un implémenteur produire un résultat déterministe et
comparable sans deviner.

Entrées relues : `spec/changes/AMD-062-profils-3dh.md` et `spec/changes/AMD-063-transparence-lua-2-1.md`
(v1.0 du 2026-11-09, textes sous revue, **non modifiés**) ; `spec/SPEC-GUIDE.md` ;
`SPEC-03` en entier (`REQ-PROF-001` à `-066`, §2, §3.1 à §3.9, §4.1 à §4.7, §5.1 à §5.6, §6) ;
`SPEC-04` §1, §2 (grilles, image de tile, enregistrement de tile), `REQ-E3D-001` à `-008`,
`-020`, `-021`, `-040` à `-048`, `-060` ; `SPEC-04B` §2 (profondeur `D`, fente, algorithmes de
maillage), `REQ-E3D-408`, `-420` à `-425`, `-442` à `-452`, `-500` à `-505`, `-520` à `-523` ;
`SPEC-05` §1, §2, §3.1 (`REQ-LUA-001` à `-005`), §4.1 (globales), §4.2 (types `Instance`,
`Pattern`, `Shape`, `CameraAPI`, `SettingAPI`, `FrameAPI`), `REQ-LUA-086`, `-110` ;
`SPEC-06` §2, `REQ-SCN-002`, `-003`, `-005`, `-007` à `-013`, `-019`, `-022`, `-024` à `-027`,
`-035`, `-037`, `-040`, `-044`, §4.2, §4.3 ; `SPEC-02` §2, §3.2 (en-tête iNES, CHR-RAM),
§3.5 (`REQ-CORE-034`, `-037`, `-041`, `-043`), §3.8 (`REQ-CORE-080` à `-082`), §3.9, §3.11, §5.2 ;
`SPEC-01` `REQ-ARCH-012` à `-014`, `-020`, `-021` ; `spec/changes/` : `AMD-012`, `AMD-027`,
`AMD-030` (§2 à §6), `AMD-032`, `AMD-033`, `AMD-036`, `AMD-037`, `AMD-039` (§2, §4, §5),
`AMD-040` (§1 à §3), `AMD-041`, `AMD-042`, `AMD-043`, `AMD-052`, `AMD-053`, `AMD-055`, `AMD-056`,
`AMD-057`, `AMD-058`, `AMD-059`, `Q-032`, `Q-067`, `Q-081`, `Q-082`, `Q-104` ;
`SPEC-v1.2-INDEX.md` ; `journal/JOURNAL.md` ;
`assets-neutral/profiles-2.1/` (les 101 fichiers + `SHA256SUMS.txt`), `assets-neutral/profiles/`
(les 101 fichiers), `assets-neutral/INVENTORY.md`, `assets-neutral/format-data.json`.

Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir sur une lecture, et aucune tentative
de contournement n'a été faite. **Le serveur d'outils de rétro-ingénierie présent dans la session
n'a pas été utilisé** (hors des sources autorisées du côté propre). Aucun connecteur externe n'a été
sollicité.

Méthode : (a) relecture d'étanchéité des deux fichiers, phrase par phrase, contre la liste du
`SPEC-GUIDE` (« ce que la spec NE DOIT PAS contenir ») ; (b) **décodeur protobuf indépendant écrit
pour cette relecture** (lecteur de fil générique : varint, LEN, I32, groupes ; puis descente typée
`HashEnvelope` → `ProfileFile` → `RefEnvelope` → `Pattern` → cellule nullable → `Cell` →
`TilePlanes`, et `Settings` → `RecordedSlot`), appliqué aux **202 fichiers** des deux dossiers, et
**recalcul indépendant** des quatre vecteurs de `REQ-PROF-075` à partir du **seul** texte de
`REQ-PROF-075` et `REQ-PROF-077`, des deux condensés de `REQ-PROF-076` et des trois retournements de
`REQ-PROF-079` ; (c) contrôle de l'existence et du sens des **49 renvois `REQ` distincts** et des
**13 renvois `AMD`** des deux fichiers ; (d) contrôle d'unicité des 20 numéros `REQ` créés.
Le décodeur a été **étalonné sur les chiffres publiés de SPEC-03 §5.5** avant d'être utilisé pour
juger : sur `assets-neutral/profiles` il retrouve exactement 74 389 variantes hors emplacements,
60 708 marqueurs nuls, 13 539 textures de format 0, 142 de format 1, 67 832 patterns, 225 575 tiles
nouvelles, 226 656 références et 56 profils porteurs d'un `Id` — les huit chiffres de SPEC-03 et
d'`AMD-037` §1, au compte près. Les écarts signalés plus bas sont donc mesurés avec un outil dont
l'accord avec la spec gelée est établi.
Note de provenance : l'Historique des deux amendements renvoie au rapport de delta 2.0 → 2.1,
document du côté sale non lisible ici ; convention déjà admise (`AMD-057`, `AMD-058`, `AMD-059`
renvoient de même à leur rapport de passe R5), et aucune de mes vérifications n'en dépend.

---

## Verdict

**Gate R2 : non franchi en l'état. Les deux amendements demandent une v1.1.**

- **Étanchéité : aucune fuite.** Le procédé et les constantes de `REQ-PROF-075` à `-077` sont
  rédigés de façon **fonctionnelle** et sont bien des faits d'interopérabilité au même titre que la
  clé `K` de `REQ-PROF-030` (détail en `62-11`) : à conserver tels quels. Un seul nom est à retirer
  par précaution (`62-7`), et deux phrases décrivent l'intérieur du programme d'origine au lieu de
  son effet observable (`62-8`, `63-2` note).
- **Numérotation : saine.** `REQ-PROF-070` à `-084`, `REQ-SCN-087` et `REQ-LUA-118` à `-121` sont
  tous libres (maxima antérieurs : `REQ-PROF-066`, `REQ-SCN-086`, `REQ-LUA-117`) ; aucun
  numéro n'est réutilisé. `REQ-PROF-067` à `-069` restent inemployés, ce qui est permis.
- **Le cœur technique d'`AMD-062` est juste et vérifié.** Les quatre vecteurs de `REQ-PROF-075` se
  reproduisent **au bit près** à partir du seul texte, et tout ce que `REQ-PROF-071`, `-074`, `-078`
  et `-084` chiffrent est exact sur les données citées (liste au §« Ce qui est acquis »).
- **Ce qui bloque est circonscrit.** Six points sur `AMD-062` : la donnée de référence du §1 et le
  §9 sont **factuellement faux** sur deux profils (`62-1`, `62-4`), le critère de `REQ-PROF-083` est
  **arithmétiquement impossible** (`62-2`) et sa clause de repli **ambiguë** (`62-3`), les critères
  de `REQ-PROF-078`/`-080` exigent des **ROMs commerciales** absentes d'`assets-neutral` (`62-5`),
  et `REQ-PROF-080` réclame un **accès à la CHR-RAM que SPEC-02 n'expose pas** (`62-15`).
  Six points sur `AMD-063` : la grille de fond est donnée pour **32 × 30** au lieu de 35 × 33
  (`63-1`), l'amendement **omet `AMD-058` §2 et `REQ-SCN-040`**, qui interdisent précisément le
  moyen qu'il impose (`63-2`), « tuile **pleine** » n'est **pas définie** dans le vocabulaire de
  `SPEC-04B` (`63-3`), le **périmètre fond/sprites** de l'étape 2 n'est pas fixé (`63-6`), et
  `REQ-LUA-119` contredit `AMD-059` `REQ-LUA-117` (`63-9`) et `AMD-042` `REQ-LOOP-069` (`63-10`).
- **Réponse à la question posée sur `REQ-PROF-080` à `-083`** : `REQ-PROF-081` est déterministe et
  comparable en l'état ; `REQ-PROF-082` est sans critère ; `REQ-PROF-080` et `REQ-PROF-083`
  **obligent aujourd'hui l'implémenteur à deviner** (détail au §« Déterminisme de `REQ-PROF-080` à
  `-083` »).

---

## Points sur `AMD-062`

### `62-1` — §1 : la donnée de référence est fausse sur deux profils — **bloquant**

Le §1 affirme : « Une fois tuiles et textures résolues, chacun a le même contenu que son homologue
de `assets-neutral/profiles` (profils 1.x), sauf : les **scripts** de `DQ3_DW3`,
`Final_Fantasy_USA`, `Super Mario Bros` et `punchout` […] et les **réglages** de
`Super Mario Bros` […]. Les patterns, tuiles et textures sont identiques pour les 101. »

Mesuré, dossier contre dossier (patterns des champs 4 et 5, variantes hors emplacements mémorisés,
objets tile neufs) :

| Profil | patterns 3dh − 3dn | variantes 3dh − 3dn | tiles neuves 3dh − 3dn |
|---|---|---|---|
| `DQ3_DW3.bytes` | **+71** | **+71** | **+196** |
| `Final_Fantasy_USA.bytes` | **+21** | **+22** | **+72** |
| les 99 autres | 0 | 0 | 0 |

Totaux : 67 924 patterns / 74 482 variantes / 225 843 tiles neuves côté 3dh, contre
67 832 / 74 389 / 225 575 côté 3dn (les trois chiffres de SPEC-03 §5.5). La phrase « Les patterns,
tuiles et textures sont identiques pour les 101 » est donc fausse pour 2 fichiers sur 101.

Symétriquement, la liste des exceptions range `Super Mario Bros` parmi les profils dont les
**scripts** changent : mesuré, les 10 messages `Script` de `Super Mario Bros.bytes` sont **identiques
octet pour octet** entre `profiles/` et `profiles-2.1/`. Les scripts de `DQ3_DW3`,
`Final_Fantasy_USA` et `punchout` diffèrent bien, eux.

Conséquence pratique : tout test écrit sur la foi du §1 (et le critère de `REQ-PROF-083`, voir
`62-2`) échoue sur ces deux fichiers sans que l'implémenteur puisse savoir si la faute est à son
code ou à la spec.

**Correction proposée** : remplacer la phrase par les faits mesurés — « 99 des 101 profils ont,
une fois tuiles et textures résolues, exactement le même contenu que leur homologue 1.x ;
`DQ3_DW3` (+71 patterns, +71 variantes) et `Final_Fantasy_USA` (+21 patterns, +22 variantes) ont
en outre gagné des patterns ; les **scripts** diffèrent pour `DQ3_DW3`, `Final_Fantasy_USA` et
`punchout`, et les **réglages** pour `Super Mario Bros` (§9) ». Vérifier au passage si
`AMD-039` §5 et `AMD-040` §1 (« ne changent que par leurs scripts Lua ») doivent être corrigés de
la même façon : la mesure ci-dessus ne dit pas si ces patterns sont arrivés en 2.0 ou en 2.1, et
seul R1 peut trancher.

### `62-2` — `REQ-PROF-083`, critère : la correspondance annoncée est arithmétiquement impossible — **bloquant**

Le critère énonce : « sur les 101 profils 2.1, après résolution complète des tuiles […], chaque
variante a la texture du 3dn : 60 777 de `textureKind = 1` **↔** marqueur nul ; 13 559 de format 0
et même taille, 142 de format 1 et 4 de taille différente **↔** texture explicite égale ».

Les chiffres de **gauche** sont exacts, au compte près (mesuré sur les 101 fichiers 3dh) :

| Mesure | Valeur |
|---|---|
| variantes hors emplacements mémorisés | 74 482 |
| `textureKind = 1` | **60 777** |
| `textureKind = 2`, champ 24 de format 0 et de taille `tSize × 8` | **13 559** |
| `textureKind = 2`, champ 24 de format 0 et de taille différente | **4** |
| `textureKind = 2`, champ 24 de format 1 | **142** |
| `textureKind = 0`, ou champ 25 avec une autre valeur, ou champ 19 | 0 |
| champ 25 absent (les 1 010 variantes des emplacements mémorisés) | 1 010 |

60 777 + 13 559 + 4 + 142 = 74 482 : la partition est complète et juste.

Mais le membre de **droite** porte sur les 3dn, qui n'ont que **74 389** variantes hors
emplacements, dont **60 708** marqueurs nuls et **13 681** textures explicites (13 539 de format 0
+ 142 de format 1) — les chiffres de SPEC-03 §5.5, que mon décodeur retrouve exactement. Il manque
donc **93 variantes** en face (69 du côté `textureKind = 1`, 24 du côté `textureKind = 2`), et ce
sont exactement les 93 variantes de surplus de `DQ3_DW3` et `Final_Fantasy_USA` relevées en `62-1`.
Le `↔` ne peut pas être établi : un tiers qui exécute le critère tel quel échoue, sur 2 fichiers,
avec un écart de 93 et aucune explication.

**Correction proposée** : (a) scinder le critère — sur les **99** profils inchangés, exiger
l'égalité variante par variante dans l'ordre du fichier, avec les comptes de ces 99 ; (b) donner
`DQ3_DW3` et `Final_Fantasy_USA` à part, avec leurs comptes propres et la mention que leurs
patterns supplémentaires n'ont pas d'homologue ; (c) remplacer les `↔` par deux énoncés séparés —
d'un côté la partition des 74 482 variantes 3dh (tableau ci-dessus, exact et directement
exécutable sans ROM ni résolution), de l'autre l'égalité des textures reconstruites avec celles du
3dn pour les variantes appariables.

### `62-3` — `REQ-PROF-083`, clause de repli : négation ambiguë — **bloquant**

« Tant qu'une tuile du pattern est en attente **et que `X` n'a pas les dimensions et le format de
`B`**, la variante utilise `B`. »

Deux lectures grammaticalement également valides :
- **A** : ¬(mêmes dimensions **et** même format) — c'est-à-dire dimensions **ou** format différents ;
- **B** : dimensions différentes **et** format différent.

Elles ne désignent pas le même ensemble. Mesuré sur les 101 fichiers : des 13 705 variantes de
`textureKind = 2`, **146** ont des dimensions **ou** un format différents de `B` (les 4 de taille
différente et les 142 de format 1), et **aucune** n'a les deux à la fois (les 142 de format 1 ont
toutes la taille de `B`, les 4 de taille différente sont toutes de format 0). Sous la lecture A,
146 variantes se replient sur `B` pendant la résolution ; sous la lecture B, **zéro**. L'écart est
visible à l'écran et dans `--dump-meshes`.

**Correction proposée** : écrire « … et que les dimensions **ou** le format de `X` diffèrent de ceux
de `B`, la variante utilise `B` », et ajouter le compte mesuré (146 variantes concernées sur les
101 profils, dont 142 de format 1 et 4 de taille différente) pour que la clause soit testable.
Préciser aussi, dans la formule de reconstruction, que `X.extra` **absent ou plus court que
`X.data`** vaut 0 (le texte ne couvre que « `extra` absent = 0 », alors que la boucle va jusqu'à
`len(X.data) − 1`), et que la borne `len(B.data)` de la ligne `extra` est bien voulue (elle vaut
`len(B.extra)`, `B` étant toujours de format 0 et construit par l'implémentation).

### `62-4` — §9, premier tiret : cinq réglages changent, pas deux ; et le « préréglage `water` » n'existe dans aucun format — **bloquant**

« Profil `Super Mario Bros.bytes` : en 2.1, seuls changent deux éléments des réglages (champ 12 du
message de réglages et le préréglage de style visuel `water`) ; ils arrivent avec le profil. »

Mesuré, message de réglages (champ racine 6) de `Super Mario Bros.bytes`, 3dn contre 3dh :

| Champ | Nom (SPEC-03 §4.6) | 3dn | 3dh | Appliqué (`REQ-PROF-041`) |
|---|---|---|---|---|
| 1 | `Light.Intensity` | 0,716150 | 1,011272 | **non** (réglage utilisateur, `REQ-SCN-026`) |
| 9 | `VSync` | 0 | 1 | non |
| 12 | `Volume` | 0,141679 | 0,425790 | non |
| 22 | `Core` | 2 | 0 | non |
| 37 | `SameSpPalette` | **absent** | 0 | **oui** |

`Light.Direction` et `Light.Color` sont inchangés. Cinq éléments changent donc, non deux ; et le
seul qui soit appliqué (champ 37) passe d'absent — défaut faux, `REQ-PROF-040` — à la valeur 0,
c'est-à-dire **sans aucun effet observable**. La conclusion du tiret (« ils arrivent avec le
profil », sans conséquence) est juste ; son énumération est fausse.

Par ailleurs « le préréglage de style visuel `water` » ne correspond à **aucun champ** de
`Settings` (SPEC-03 §4.6, champs 1 à 38), à aucun paramètre de style d'`AMD-040` §3 (les quatre
`spl.*` de courbure), à aucune clé de `SPEC-09`, et à aucun comportement de lecteur ailleurs dans la
spec. Les paramètres de style sont de portée **utilisateur** (`AMD-040`, ligne
`render.visual_style`, portée U ; `AMD-059` `REQ-LUA-116`), donc ils ne peuvent pas « arriver avec
le profil ». En l'état, c'est soit un champ de 2.1 non spécifié, soit un nom d'artefact interne du
programme d'origine sans contrepartie dans les données conservées — dans les deux cas à traiter.

**Correction proposée** : remplacer l'énumération par le tableau ci-dessus et sa conclusion
(« aucun de ces cinq éléments n'a d'effet observable : quatre ne sont pas appliqués par
`REQ-PROF-041`, et le cinquième prend sa valeur par défaut ») ; **retirer** la mention du préréglage
`water`, ou — si la 2.1 l'écrit vraiment dans le profil — lui donner son numéro de champ, son type
et sa règle d'application, auquel cas il relève de `REQ-PROF-074` et non du §9.

### `62-5` — `REQ-PROF-078` et `REQ-PROF-080` : critères « auto » qui exigent des ROMs commerciales — **bloquant**

Les deux critères sont marqués « (auto, ROM) » et nomment `Super Mario Bros. (World).nes`,
`Legend of Zelda, The (USA).nes` et `Mega Man 2 (USA).nes`. Aucune de ces ROMs n'est dans
`assets-neutral` ; un tiers qui ne connaît que la spec et les données citées ne peut donc exécuter
aucun des deux. Le précédent est net : `AMD-001` a précisément réécrit le critère de
`REQ-PROF-064` « sans ROM commerciale », et `AMD-030` §6 range ses tests d'installation réelle en
« test optionnel, ignoré si l'installation est absente ».

À noter que le critère de `REQ-PROF-083` montre la bonne méthode : « après résolution complète des
tuiles **à partir des tuiles du profil 3dn homologue** » ne demande aucune ROM.

**Correction proposée** : (a) marquer les trois mesures sur ROM « (ROM, facultatif) », comme
`AMD-030` §6 et `AMD-039` §4 ; (b) ajouter à `REQ-PROF-078` un critère **exécutable** sans ROM
commerciale : construire un fichier iNES synthétique dont la zone CHR contient, à des positions
**non alignées**, les 16 octets de chaque tile du profil 3dn homologue (SPEC-02 §5.1 donne le
modèle d'en-tête), puis exiger la résolution complète du manifeste, pour les 101 profils —
et un second fichier où une tile est absente, pour vérifier qu'elle reste en attente ; (c) ajouter
à `REQ-PROF-080` un critère de même nature, sur un cœur piloté par le harnais, la ROM de
`REQ-PROF-078` étant privée de quelques tiles que le test écrit ensuite en CHR-RAM. Conserver les
chiffres sur ROM (481, 1 035, 5 353 / 5 497) comme données de référence : j'ai vérifié que les
comptes **du manifeste** correspondent bien (voir « Ce qui est acquis »), seule la résolution reste
non exécutable ici.

Nit de nommage dans le même critère : le profil cité « `Legend of Zelda, The (USA).nes` » a pour
profil `Legend Of Zelda.bytes` (1 035 identifiants, conforme) ; écrire le nom du **profil** à côté
de celui de la ROM éviterait l'ambiguïté.

### `62-15` — `REQ-PROF-080` : l'accès à la CHR-RAM n'existe pas dans SPEC-02, et la base « dernière observation » n'est pas définie — **bloquant**

« si la cartouche a de la CHR-RAM, chaque bloc de 16 octets **aligné** de la CHR-RAM dont le contenu
a changé depuis la dernière observation […] sont confrontés au manifeste ».

Deux manques, qui obligent tous deux à deviner :

1. **Aucune interface.** SPEC-02 §3.8 n'expose que l'espace **CPU** (`REQ-CORE-080` : `$0000`–`$1FFF`,
   `$6000`–`$7FFF`, `$8000`–`$FFFF`), et `REQ-CORE-081` demande même que les lectures de
   `$2000`–`$401F` soient sans effet de bord et renvoient 0 — donc pas de lecture de CHR-RAM par
   `$2007` non plus. La CHR-RAM n'apparaît que dans le contenu du savestate (§3.9) et dans la règle
   de taille du §3.2. Il n'existe donc aucun moyen spécifié, pour le moteur de conversion, de lire
   les 8 à 32 Kio de CHR-RAM. L'implémenteur doit inventer l'accesseur, ce qui contrevient en outre
   à la contrainte « aucun état global » et au découpage cœur/moteur de `SPEC-01`.
2. **Base de comparaison non spécifiée.** « depuis la dernière observation » ne dit ni ce que vaut
   la base à la **première** frame, ni ce qu'elle devient sous run-ahead (`AMD-041`), rollback
   (`AMD-042`) et chargement de savestate (`REQ-ARCH-013`) — trois mécanismes qui **remettent la
   CHR-RAM du cœur en arrière** sans que la base suive. `REQ-PROF-080` affirme pourtant que « Ce que
   la frame produit ne dépend que de l'émulation et de l'ensemble des tuiles déjà résolues
   (`REQ-ARCH-012`) » : l'invariant est le bon, mais une base non snapshotée le met en risque, car
   l'ensemble résolu à la frame *n* peut dépendre du chemin suivi pour y arriver, alors que
   `REQ-ARCH-012` exige l'identité « à ROM, profil, savestate initial et séquence d'entrées
   identiques ». Le critère proposé (« deux exécutions identiques, cache vide, donnent des captures
   identiques ») ne teste pas ce cas.

**Correction proposée** : (a) ajouter à SPEC-02 §3.8 une exigence d'accès **en lecture seule** à la
CHR-RAM (taille, adressage linéaire 0-based, sans effet de bord, appelable entre deux frames), avec
son critère ; (b) supprimer la base incrémentale au profit d'une règle sans état — « tant qu'il reste
des tuiles en attente et que la cartouche a de la CHR-RAM, **toute** la CHR-RAM est parcourue par
blocs de 16 octets alignés à chaque frame » (au pire 2 048 blocs × 4 retournements pour 32 Kio,
borné et déterministe) ; ou (c) si l'incrémental est voulu pour le coût, déclarer explicitement que
l'instantané de référence fait partie de l'état de résolution, qu'il est **réinitialisé** (tout
« changé ») au chargement d'un savestate, au rollback et à l'entrée en run-ahead, et ajouter un
critère qui le vérifie (sauvegarder, jouer, recharger, et retrouver le même ensemble résolu).

### `62-6` — Toute la branche « condensé hérité » et toute la branche « version ancienne » sont sans donnée de test — non bloquant

Mesuré sur les 101 fichiers de `profiles-2.1` : `Version` vaut `"5"` dans **101/101** ; la liste
`Hashes` (champ 4) est **vide dans 101/101** ; **aucune** entrée texte de 32 ni de 64 chiffres
n'existe donc ; le champ 5 de l'enveloppe est absent partout (conforme à « Le champ 5 n'est jamais
écrit ») ; et les **225 843** tiles portent toutes exactement l'ensemble de champs `{3, 5}` —
**zéro** tile ne porte le champ 4.

Il en résulte que les règles suivantes n'ont aucun vecteur dans les données citées : les versions
`"1"` et `"2"` de `REQ-PROF-072`, la tolérance des entrées de 32 chiffres, la règle de doublon dans
`Hashes ∪ Ids`, la branche « champ 4 » de `REQ-PROF-074`, et toute `REQ-PROF-076` au-delà de ses
deux condensés (que j'ai vérifiés, voir « Ce qui est acquis »). Le texte ne le dit pas, alors que
c'est précisément ce qui décide si un critère est exécutable.

Dans la même zone, une imprécision qui empêche d'écrire le test : `REQ-PROF-074` dit « Toute valeur
des champs 4/5 DOIT figurer dans le manifeste de l'enveloppe » sans dire **sous quelle forme** la
comparaison se fait. Le manifeste a deux encodages (binaire dans `Ids`, texte dans `Hashes`), et
`REQ-PROF-072` admet qu'une entrée texte de 32 chiffres « vaut un identifiant de tuile sous forme
texte » : un champ 4 de 32 chiffres doit-il être cherché aussi dans `Ids` converti en texte, et un
champ 5 aussi dans `Hashes` ? De même, `REQ-PROF-081` fait reposer l'égalité de deux tiles en
attente sur « le même identifiant (ou condensé) » : deux tiles de la même tile brute, l'une portant
son identifiant et l'autre son condensé hérité, se compareraient inégales.

**Correction proposée** : ajouter une phrase à `REQ-PROF-072` — « la comparaison au manifeste et la
détection des doublons se font sur la **forme texte** : un identifiant binaire d'`Ids` est converti
par `REQ-PROF-075`, de sorte que `Hashes` et `Ids` forment un seul ensemble de chaînes » ; étendre
la même règle à `REQ-PROF-074` et à l'égalité de `REQ-PROF-081` ; et ajouter aux critères de
`REQ-PROF-072`, `-074` et `-076` la mention « sur enveloppes **synthétiques**, aucun fichier de
`assets-neutral/profiles-2.1` n'exerçant ces branches (101/101 en version `"5"`, `Hashes` vide,
aucun champ 4) ».

### `62-7` — §9 : `TileHashCache` nomme un type interne du programme d'origine — non bloquant

« Le format du cache de 3dSen (`TileHashCache`) n'est pas repris (`REQ-PROF-082`). »

Ce nom n'est **pas** visible depuis les données conservées : `assets-neutral` n'en contient rien, le
cache est un fichier produit à l'exécution dans le dossier de données de l'utilisateur, et le texte
ne dit pas s'il s'agit d'un nom de fichier (observable, donc admissible) ou d'un nom de type
(interdit par le `SPEC-GUIDE`, « noms internes de classes, méthodes, champs ou variables privés de
l'original, sauf s'ils sont visibles depuis les données conservées »). Comme le format « n'est pas
repris », l'information est de toute façon **sans usage** pour le côté propre : le coût du retrait
est nul.

Dans la même veine, et pour aligner `AMD-062` sur la précaution que SPEC-03 §2 a prise : les noms
`HashEnvelope`, `Payload`, `PayloadHash`, `Hashes`, `Ids`, `xorTex` et `textureKind` ne circulent
pas sur le fil protobuf et ne sont donc pas déductibles des données conservées. SPEC-03 §2 déclare
explicitement ses noms de messages « noms de commodité propres à la spec, sans portée normative » ;
`AMD-062` ne le fait pas, ce qui les laisse lire comme les noms du programme d'origine.

**Correction proposée** : supprimer « (`TileHashCache`) », ou le remplacer par le nom du **fichier**
s'il est observable ; et ajouter une phrase au §3 : « Comme en SPEC-03 §2, `HashEnvelope` et les noms
de champs de ce document sont des noms de commodité propres à la spec, sans portée normative : seuls
les numéros de champs et les types de fil comptent. » La chaîne `"3dSen.TileId.v1"` reste, elle,
**obligatoire** : c'est une entrée du condensé, donc un fait d'interopérabilité pur.

### `62-8` — §9 : le budget interne de 2 ms décrit l'intérieur de l'original — non bloquant

« 3dSen reconstruit les maillages des patterns résolus en tâche de fond, avec un budget de 2 ms par
frame, ce qui peut montrer quelques frames l'ancien maillage. »

La seconde moitié est l'effet observable, légitime et utile pour justifier la divergence. La
première (tâche de fond, budget de 2 ms par frame) est de l'ordonnancement interne, que le
`SPEC-GUIDE` exclut « sauf effet observable », et dont le chiffre n'a aucune contrepartie dans
Zaxnes.

**Correction proposée** : « 3dSen peut montrer pendant quelques frames l'ancien maillage d'un
pattern dont une tuile vient d'être résolue ; Zaxnes le reconstruit avant le rendu de la frame
(`REQ-PROF-080`), ce qui garde le déterminisme. »

### `62-9` — `REQ-PROF-072` : textes de refus non grammaticaux et absents de la table des textes — non bloquant

Les cinq chaînes de `REQ-PROF-072` sont des critères d'acceptation **à l'identique** (« refus avec le
message de la version 4 »), et le §8 les promeut en causes de refus **visibles par l'utilisateur** :
« un profil 3dh refusé par `REQ-PROF-072` prend comme cause le texte de `REQ-PROF-072` », cause que
`AMD-056` `REQ-UI-214` affiche dans le dialogue `Cannot import this file`. Deux problèmes :

- `Hash profile version 4 is no longer supported (its tile IDs give back raw); import the original
  profile again.` n'est pas de l'anglais intelligible (« give back raw ») ; comme c'est une
  comparaison exacte, le défaut se propage dans les tests.
- `AMD-056` précise que les textes de dialogue sont « des entrées de la table des textes
  (`AMD-036`) » ; les cinq chaînes nouvelles n'y ont pas d'entrée, donc pas de traduction
  (`AMD-035`, `AMD-036`).

Dans le même paragraphe, la liste des versions admises (`"1"`, `"2"`, `"5"`) et le sort particulier
de `"4"` laissent `"3"` dans le lot générique sans un mot d'explication, alors que le `SPEC-GUIDE`
demande la justification d'une constante qui détermine un résultat observable.

**Correction proposée** : reformuler le message de la version 4 (par exemple « Hash profile envelope
version 4 is no longer supported: its tile identifiers cannot be resolved. Import the original
profile again. ») ; ajouter les cinq chaînes à la table d'`AMD-036` en les désignant comme entrées
anglaises, ou dire explicitement qu'elles restent journalisées et non traduites ; et ajouter une
phrase sur l'absence de version `"3"`.

### `62-10` — §8, dernier tiret : faux pour 45 profils sur 101 — non bloquant

« un 2.0 puis un 2.1 importés tour à tour portent le même `Id` (même jeu) ».

Mesuré : **56** des 101 profils 3dn portent un champ racine 7 (conforme à SPEC-03 `REQ-PROF-022` et
à `AMD-037` §1), et pour ces 56 l'`Id` de l'enveloppe 3dh est **égal** à celui du 3dn (56/56 — par
exemple `Super Mario Bros` : `Jhj6E7iOysxT` des deux côtés). Mais les **45** autres n'ont **aucun**
`Id` côté 3dn, alors que les **101** enveloppes 3dh en portent un (`Mega Man 2` : `fe7435432e30`
dans l'enveloppe, rien dans le 3dn). Pour ces 45 jeux, le fichier 2.0 importé reçoit l'identifiant
**dérivé** d'`AMD-037` (base64 des 9 premiers octets du SHA-256 du fichier) et le fichier 2.1
l'identifiant de son enveloppe : deux identifiants différents, donc **deux entrées** de
bibliothèque, et non « le même `Id` ».

**Correction proposée** : restreindre l'affirmation aux 56 profils porteurs d'un `Id` (avec le
chiffre), et dire ce qui est attendu pour les 45 autres — identifiant dérivé d'`AMD-037` d'un côté,
identifiant d'enveloppe de l'autre, donc deux entrées distinctes — ou trancher autrement si le
propriétaire veut l'unification. Préciser au même endroit qu'`AMD-037` s'applique aussi aux 3dh
(son texte dit « si le champ `Id` est absent ou vide », à lire désormais dans l'enveloppe) et que
l'import 3dSen écrit un 3dh sous l'extension `.3dn` (« comme avant »), ce qui est voulu puisque le
format se reconnaît au contenu, mais mérite d'être dit, `REQ-PROF-062` amendée parlant par ailleurs
des deux extensions.

### `62-11` — `REQ-PROF-075` à `-077` : étanches, fonctionnels et reproductibles — **aucune correction requise** (deux nits)

C'est le point sur lequel le brief demande un jugement d'étanchéité, et le verdict est **favorable**.

- **Reproductibilité.** J'ai réimplémenté `P` et l'identifiant à partir du **seul** texte de
  `REQ-PROF-075` et `REQ-PROF-077`, sans aucune autre source. Les **quatre** vecteurs sortent
  **au bit près**, colonne `Préfixe`, forme binaire et forme texte comprises :

  | Tuile brute | `P` | forme binaire | forme texte |
  |---|---|---|---|
  | `00…00` | `06e28b2d` ✓ | `2d8be206b548f95b38bf8a3e2d7f50f9` ✓ | `f9507f2d3e8abf385bf948b506e28b2d` ✓ |
  | `ff…ff` | `5343027f` ✓ | `7f024353624f541ff1301725c2461716` ✓ | `161746c2251730f11f544f625343027f` ✓ |
  | `00 01 … 0f` | `870dcae2` ✓ | `e2ca0d87ba7804d4c5cccdc426124e91` ✓ | `914e1226c4cdccc5d40478ba870dcae2` ✓ |
  | `ff×8 00×8` | `1dbbccea` ✓ | `eaccbb1d67be65f246453106c3c0a5d3` ✓ | `d3a5c0c306314546f265be671dbbccea` ✓ |

  Le texte est donc **suffisant** : aucune lacune à combler par la devinette, ce qui est la
  propriété décisive pour un fait d'interopérabilité.
- **Étanchéité.** La rédaction est fonctionnelle de bout en bout : chaque étape est une opération
  nommée en français sur des valeurs (« produit 128 bits non signé de … par … », « inversion de
  l'ordre des 8 octets »), il n'y a pas de syntaxe de langage, pas de structure de contrôle, pas de
  nom de variable, de champ ou de méthode de l'original — `m_lo`, `m_hi`, `h2` et `v` sont des
  notations locales à l'énoncé. Les constantes sont en outre des constantes de mélange **publiques**
  et reconnaissables (`0x9E3779B185EBCA87`, nombre d'or sur 64 bits ; `0xC2B2AE3D27D4EB4F` et
  `0x165667919E3779F9` avec les décalages 37 et 32, finaliseur de type xxHash/Murmur), ce qui
  conforte la lecture « donnée de format » plutôt que « code recopié ». Même régime que la clé `K`
  de `REQ-PROF-030`, et même nécessité : sans `P`, aucun manifeste 3dh ne se relit. **À conserver
  tel quel.**
- Nit 1 : les étapes 2 et 3 (ajout de `0x03C0000000000000` à `m_lo` ; `h2` compté deux fois dans
  `m_hi`) n'ont aucune signification mathématique, et un implémenteur va perdre du temps à en
  chercher une. Ajouter une phrase : « Les étapes 1 à 5 sont une fonction de mélange mesurée, à
  reproduire au bit près ; leur forme n'a pas de sens propre. »
- Nit 2 : la dernière phrase de `REQ-PROF-077` (« Le préfixe est aussi formé des 32 bits de poids
  faible de `L` ») n'est pas une règle mais une **conséquence** de `REQ-PROF-075` ; je l'ai vérifiée
  vraie sur les quatre vecteurs. L'écrire « Par construction (`REQ-PROF-075`), `P` est aussi … » lève
  l'ambiguïté sur l'ordre de calcul.

### `62-12` — Appariement des fichiers : deux noms sur 101 diffèrent, et le dossier contient un fichier de plus — non bloquant

Le §1 annonce « les 101 profils 3dh, mêmes noms que `assets-neutral/profiles/` ». Mesuré :
`profiles-2.1/` contient **102** entrées — 101 `.bytes` et `SHA256SUMS.txt` — et deux noms diffèrent
de leur homologue :

| `profiles-2.1/` | `profiles/` |
|---|---|
| `Castlevania II - Simon's Quest (USA).bytes` | `Castlevania II - Simon_s Quest (USA).bytes` |
| `Hogan's Alley.bytes` | `Hogan_s Alley.bytes` |

Tout critère qui apparie les deux dossiers par nom (ceux de `REQ-PROF-070`, `-073`, `-083`, et le
§1 lui-même) échoue mécaniquement sur ces deux fichiers.

**Correction proposée** : « mêmes noms que `assets-neutral/profiles/`, à ceci près que l'apostrophe
y est écrite `_` (deux fichiers) ; le dossier contient en outre `SHA256SUMS.txt` », et préciser que
l'appariement des critères se fait en remplaçant `'` par `_`.

### `62-13` — `REQ-PROF-073` : « le payload n'est pas lu » n'est pas observable — non bloquant

Le critère est : « l'en-tête de `Super Mario Bros.bytes` (2.1) donne le même `Id` que celui de
`Super Mario Bros.bytes` (3dn) ; **le payload n'est pas lu** ». La première moitié est exacte
(vérifié : `Jhj6E7iOysxT` des deux côtés, et 56/56 pour les profils porteurs d'un `Id`, voir
`62-10`). La seconde est une propriété **interne** : un tiers en boîte noire ne peut pas l'observer,
et le `SPEC-GUIDE` demande un critère « vérifiable par un tiers qui ne connaît que la spec ».

Or la propriété intéressante est facile à rendre observable, et elle est la seule qui compte pour
l'utilisateur : la bibliothèque doit lister un 3dh dont le payload est inutilisable.

**Correction proposée** : « … ; un 3dh dont un octet du `Payload` a été modifié (donc refusé au
chargement par `REQ-PROF-072`, message `Damaged hash profile payload`) apparaît malgré tout dans la
liste des jeux avec son `Id` et son `Title` d'enveloppe ».

### `62-14` — `REQ-PROF-082` : seule exigence des deux amendements sans critère d'acceptation — non bloquant

Le `SPEC-GUIDE` demande que « chaque exigence porte un critère d'acceptation vérifiable ».
`REQ-PROF-082` n'en a aucun. Elle est pourtant testable : le format est libre, mais son **effet**
ne l'est pas.

**Correction proposée** : « Critère d'acceptation (auto) : un profil 3dh résolu depuis la ROM
synthétique de `REQ-PROF-078`, puis rechargé sans cette ROM, a ses tuiles résolues dès avant la
première frame ; un fichier de cache tronqué ou de contenu arbitraire est ignoré sans erreur et le
profil repart avec toutes ses tuiles en attente ; le cache n'altère pas les octets du profil. »

---

## Points sur `AMD-063`

### `63-1` — `REQ-SCN-087` étape 1 : la grille de fond n'est pas 32 × 30 — **bloquant**

« Pour chaque cellule de la grille de fond (**32 × 30 cellules**, celles de la frame) ».

SPEC-04 §2 définit la grille de fond comme « tableau de cellules de `Lg × Hg` = **35 × 33** (la zone
utile du cœur, 33 × 31 tiles, plus une marge d'une cellule de chaque côté) », avec la cellule d'une
tile en `(x, y)` égale à `(⌊(x+7)/8⌋ + 1, ⌊(y+7)/8⌋ + 1)` ; `REQ-E3D-040` dit que c'est cette grille
que la frame contient. 32 × 30 est la taille d'une nametable NES, pas celle de la grille du moteur.

L'écart n'est pas cosmétique : la règle « Une cellule voisine hors de la grille ne retire rien »
change de sens selon que la grille a ou non la marge d'une cellule, c'est-à-dire précisément au bord
de l'écran, là où le défilement place les objets partiellement sortis.

**Correction proposée** : « Pour chaque cellule de la grille de fond (35 × 33, SPEC-04 §2) ».

### `63-2` — `AMD-058` §2 interdit le moyen qu'`AMD-063` impose, et `REQ-SCN-040` dit déjà le contraire du constat — **bloquant**

`AMD-063` se déclare amendant « `AMD-057` (§2, « Parois entre tuiles ») et SPEC-06 `REQ-SCN-065` ».
Il manque deux documents :

1. **`AMD-058` §2** a *complété* `REQ-SCN-065` et énumère les moyens exclus, en tête desquels
   « **du retrait des parois (`AMD-057`)** ». `AMD-063` étend `REQ-SCN-065` aux surfaces
   transparentes (« `REQ-SCN-065` […] s'applique désormais aussi aux surfaces **transparentes**
   faites de tuiles pleines ») **et** impose le retrait des parois comme moyen. Un implémenteur qui
   lit `AMD-058` à la lettre se voit interdire exactement ce que `REQ-SCN-087` lui ordonne.
2. **`REQ-SCN-040`** (SPEC-06) énonce déjà, pour tout motif d'alpha < 1 : « d'abord un passage qui
   n'écrit que la profondeur, puis un passage de couleur mélangé […]. Seule la surface la plus
   proche de chaque objet transparent est ainsi visible (**pas de faces internes**). » Le constat du
   §1 d'`AMD-063` (« les faces latérales entre deux tuiles voisines se voient à travers ») décrit
   donc un état que `REQ-SCN-040` interdit déjà. Techniquement les deux se concilient — ce que l'on
   voit est un liseré de profondeur au raccord de deux faces avant coplanaires, pas une vraie face
   interne vue à travers — mais le texte, lui, ne les concilie pas.

**Correction proposée** : ajouter à l'en-tête « amende **`AMD-058` §2** (le retrait des parois cesse
d'être un moyen exclu pour les surfaces **transparentes** ; il reste exclu pour les surfaces
opaques) et **précise SPEC-06 `REQ-SCN-040`** » ; et reformuler le constat du §1 pour dire ce qui est
visible (« au raccord de deux tuiles pleines voisines, la paroi latérale confondue l'emporte sur un
liseré d'un pixel malgré le passage de profondeur de `REQ-SCN-040` ») plutôt que « se voient à
travers ». Note annexe : « Le shader transparent de la 2.1 a aussi changé » décrit l'intérieur de
l'original sans effet observable associé ; la phrase suivante (« s'il reste un écart visuel après ce
lot, il sera mesuré en R5 ») suffit seule.

### `63-3` — `REQ-SCN-087` : « tuile pleine » n'est pas définie dans le vocabulaire de `SPEC-04B` — **bloquant**

« Une tuile de cette variante est **pleine** si **tous** les voxels de son bloc 8 × 8 × `sizeZ` sont
remplis (aucun pixel vide, sur toute la profondeur), après géométrie et texture de la variante. »

Dans `SPEC-04B`, un voxel n'est pas « rempli » ou « vide » : §2 et `REQ-E3D-422` posent qu'en
algorithmes GREEDY « chaque voxel porte **trois affectations indépendantes**, une par axe », chacune
vide ou pointant un texel. Un voxel peut donc avoir sa fente Z remplie et ses fentes X et Y vides —
c'est exactement le cas de `geo = CUBE` (`REQ-E3D-443` : « Aucun autre voxel n'est écrit :
l'intérieur de la boîte reste vide » ; `REQ-E3D-422`, critère : « un voxel de la face avant n'a que
sa fente Z remplie »). Or c'est le remplissage des **fentes latérales** qui décide de l'existence
des faces à retirer (`REQ-E3D-501` : la face existe si la fente `a` de `v` est non vide et le voisin
est hors tuile ou de fente `a` vide). « Rempli » admet donc au moins trois lectures — au moins une
fente, les trois fentes, ou « le pixel n'est pas transparent » — qui ne donnent pas le même ensemble
de tuiles pleines.

Deux autres manques dans la même phrase :
- la parenthèse « (aucun pixel vide, sur toute la profondeur) » mélange le vocabulaire des pixels et
  celui des voxels, et n'évoque pas `emptyColor` (`REQ-E3D-424` : avec `emptyColor = 4`, un pixel de
  couleur 0 donne un voxel plein), qui change le résultat ;
- `sizeZ = 0` donne un champ de voxels **vide** (`REQ-E3D-420` : « Si `D = 0`, aucun voxel n'est
  construit ») : « tous les voxels sont remplis » y est **vrai par vacuité**, et la tuile serait
  déclarée pleine.

**Correction proposée** : « Une tuile de cette variante est **pleine** si `D > 0` et si, dans le
champ de 8 × 8 × `D` voxels construit pour elle (`SPEC-04B` `REQ-E3D-420`, `D = sizeZ`), **les trois
fentes de chaque voxel** sont non vides, `emptyColor` compris (`REQ-E3D-424`). » Ajouter en Note
que les géométries creuses (`CUBE`, cylindres) ne produisent donc jamais de tuile pleine, ce qui est
voulu, et renvoyer `REQ-E3D-452` pour la profondeur effective des cylindres (qui ne change pas le
champ de voxels).

### `63-6` — `REQ-SCN-087` étape 2 : le périmètre fond / sprites n'est pas fixé — **bloquant**

L'étape 1 ne construit qu'une carte **du fond** (« l'instance de frame **de fond** (shape de fond,
SPEC-04) »). L'étape 2, elle, parle de « Une tuile dessinée d'une instance **non opaque** » sans la
restreindre au fond, puis renvoie au « 4-voisinage, **dans la grille** » — au singulier, alors que
SPEC-04 §2 définit **deux** grilles de dimensions et d'origines différentes : la grille de fond
(35 × 33, cellule `(⌊(x+7)/8⌋ + 1, ⌊(y+7)/8⌋ + 1)`) et la grille de sprites (« de même dimension,
sans marge », cellule `(⌊x/8⌋, ⌊y/8⌋)`). Une instance de **sprites** transparente tombe donc dans le
champ de l'étape 2 sans que la cellule à consulter soit définie, et la carte de l'étape 1 n'a de
toute façon rien à lui répondre.

**Correction proposée** : restreindre explicitement l'étape 2 aux instances de **fond** (« Une tuile
dessinée d'une instance de **fond** non opaque … ; la cellule voisine est prise dans la grille de
fond ») et dire d'un mot que les sprites gardent toutes leurs faces ; ou, si le retrait doit valoir
aussi pour les sprites, ajouter une carte de la grille de sprites à l'étape 1.

### `63-7` — `REQ-SCN-087` étape 2 : le renvoi à `REQ-E3D-048` est cassé — non bloquant

« la tuile de cette instance dans cette cellule existe, **n'est pas cachée (SPEC-04
`REQ-E3D-048`)** ».

`REQ-E3D-048` définit l'indicateur `hidden` d'une **shape**, pas d'une tuile : « L'indicateur
`hidden` d'une shape de sprites DOIT être le OU des indicateurs `hidden` de ses tiles ; celui d'une
shape **de fond est toujours faux**. » Le renvoi est donc doublement inadapté : il porte sur le
mauvais objet, et sur un objet dont la valeur est constante dans le cas visé (le fond). L'indicateur
de **tuile** vient d'ailleurs : SPEC-04 §2 (« `hidden` (sprite derrière le fond) ») et
`REQ-E3D-005` (« Chaque tile de fond conservée DOIT […] conserver `x`, `y`, `pal`, `hidden` ») — et
son sens même (« sprite derrière le fond ») n'a pas de contenu pour une tile de fond.

**Correction proposée** : si l'étape 2 est restreinte au fond (`63-6`), **supprimer** la condition,
qui est vide ; sinon la réécrire « la tuile de cette instance dans cette cellule existe et n'a pas
son indicateur `hidden` (SPEC-04 §2, `REQ-E3D-005`) », en signalant qu'elle ne mord que sur les
sprites.

### `63-4` — `REQ-SCN-087` : deux prédicats d'opacité différents — non bloquant

La définition de la variante à faces internes retirables et l'étape 1 disent « opacité […]
**strictement inférieure à 1** » et « opacité ≥ 1 » ; l'étape 2 dit « opacité de l'instance
**différente de 1** ». Les deux ne coïncident pas au-dessus de 1. `Instance.Alpha` est en
lecture-écriture pour Lua (SPEC-05 §4.2, « opacité 0–1 »), sans clause normative de bornage ni dans
SPEC-05 ni dans `REQ-SCN-040` : un script qui écrit `shape.Alpha = 1.5` satisfait « différente de
1 » mais pas « < 1 », et la tuile perdrait des faces sans que sa voisine remplisse la condition
symétrique.

**Correction proposée** : employer « strictement inférieure à 1 » aux trois endroits ; ou, si le
bornage de `Alpha` à `[0, 1]` est voulu, l'écrire dans SPEC-05 §4.2 et y renvoyer.

### `63-5` — `REQ-SCN-087` étape 1 : « couvre cette cellule » n'est pas défini — non bloquant

Deux lectures : l'instance a une **tuile** dans cette cellule, ou sa **boîte en tiles** (SPEC-04 §2,
`tStart`/`tEnd`) contient cette cellule. L'étape 2 tranche implicitement pour la seconde, puisqu'elle
ajoute la condition « la tuile de cette instance dans cette cellule **existe** » — qui serait
redondante sous la première lecture. Autant l'écrire.

**Correction proposée** : « … l'instance de frame de fond dont la **boîte en tiles** (SPEC-04 §2)
contient cette cellule ». Préciser aussi que « la première dans l'ordre des instances de la frame »
renvoie à la liste ordonnée des instances de frame de `REQ-E3D-040`.

### `63-13` — `REQ-SCN-087`, critère : le partage de maillage de `REQ-E3D-425` doit être écarté explicitement — non bloquant

Le critère porte sur « un objet de fond 4 × 4 tuiles **identiques** pleines » et attend trois
comptes de sommets différents selon la position (16 / 12 / 8). Or `SPEC-04B` `REQ-E3D-425` impose
que « Quand une même tuile 2D occupe plusieurs cases du pattern, ces cases DOIVENT partager un champ
de voxels unique (**donc un maillage unique**) ». Le critère n'est donc lisible que sur l'export
**par tuile dessinée**, comme celui d'`AMD-057` (« `--dump-meshes` […] → chacune a 24 sommets ») et
comme le définit `Q-081`. Les Notes de `REQ-SCN-087` l'admettent à mots couverts (« préparer les 15
variantes de maillage d'une tuile pleine […] ou retirer les faces au dessin »), mais le texte
normatif ne dit pas que `REQ-E3D-425` ne s'applique plus au maillage **dessiné** de ces tuiles.

Les comptes, eux, sont justes : une tuile pleine donne 6 rectangles donc 24 sommets
(`REQ-E3D-502`, TV-G3, et le critère d'`AMD-057`) ; retirer 2, 3 et 4 faces latérales laisse
16, 12 et 8 sommets ; et une grille 4 × 4 a bien 4 coins à 2 voisins, 8 bords à 3 et 4 cases
intérieures à 4.

**Correction proposée** : ajouter au point 3 de `REQ-SCN-087` « `REQ-E3D-425` continue de régir le
champ de voxels et le maillage **construit** ; le retrait des faces porte sur le maillage
**dessiné**, par tuile dessinée », et préciser dans le critère que la lecture se fait sur l'export
par tuile dessinée (`Q-081`).

### `63-8` — `REQ-LUA-119` : renvoi cassé sur le lissage — non bloquant

« la rotation affichée est cette valeur **immédiatement**, sans le lissage de **`REQ-SCN-012`** ».
`REQ-SCN-012` est la **réinitialisation de vue** ; le lissage est `REQ-SCN-019`, auquel renvoie
d'ailleurs `REQ-SCN-007` (« à partir des valeurs **lissées** (`REQ-SCN-019`) »).

**Correction proposée** : lire « sans le lissage de `REQ-SCN-019` ».

### `63-9` — `REQ-LUA-119` contredit `AMD-059` `REQ-LUA-117`, non cité — **bloquant**

`AMD-059` `REQ-LUA-117` clôt la table `CS.Nes3D.CameraControl` par deux phrases normatives : « Une
écriture dans ces tables **n'a aucun effet sur le programme**. Tout autre champ de
`CS.Nes3D.CameraControl` […] **vaut nil**. » `REQ-LUA-119` y ajoute `RotOverride`, champ « en
lecture-écriture » dont l'écriture a un effet, et dont la lecture rend une copie `Vector3`. L'en-tête
d'`AMD-063` ne mentionne pas `AMD-059` : les deux textes se contredisent frontalement, et
`REQ-LUA-117` est le plus récent des deux à ce jour.

**Correction proposée** : ajouter à l'en-tête « amende **`AMD-059` `REQ-LUA-117`** », et à
`REQ-LUA-119` la phrase « `RotOverride` est le seul champ de `CS.Nes3D.CameraControl` dont
l'écriture ait un effet ; la règle « tout autre champ vaut nil » de `REQ-LUA-117` reste vraie pour
le reste ». Ajouter aussi à son critère d'acceptation
`CS.Nes3D.CameraControl.DistanceMax` → 512 et `CS.Nes3D.CameraControl.Autre` → nil, pour qu'un test
atteste que l'ajout n'a pas cassé `REQ-LUA-117`.

### `63-10` — `REQ-LUA-119` contredit `AMD-042` `REQ-LOOP-069` sur l'état caméra de rollback — **bloquant**

« La valeur n'est ni persistée ni sauvée dans un état […]. Elle fait partie de l'état caméra de
rollback d'`AMD-042`. »

`AMD-042` `REQ-LOOP-069` définit le contenu d'un snapshot comme contenant « l'**état de la caméra**
tel que l'enregistre un savestate, défini par `REQ-SCN-024` (amendée ci-dessous), **qui en est la
référence unique** ». Et `REQ-SCN-024` énumère : mode courant, et par mode distance, angles et cible
de consigne, plus le temps restant de la fenêtre de transition — pas de `RotOverride`. Dire à la
fois « pas sauvée dans un état » et « fait partie de l'état caméra de rollback » revient donc à nier
la référence unique d'`AMD-042` : l'état de rollback cesserait d'être l'état de savestate, sans que
le texte dise lequel des deux contient quoi. L'implémenteur doit deviner.

**Correction proposée** : choisir et l'écrire. Soit (a) `RotOverride` entre dans `REQ-SCN-024`,
donc dans les savestates **et** dans les snapshots — et la phrase « ni sauvée dans un état » tombe ;
soit (b) `REQ-LOOP-069` reçoit un **troisième** élément explicite (« les 64 valeurs de `PData` ;
l'état de caméra de `REQ-SCN-024` ; la valeur de `RotOverride` »), et `REQ-SCN-024` reste la
référence du seul état de savestate. L'option (b) colle au reste de l'exigence (valeur de session,
remise à `nil` au lancement d'un jeu et au changement de profil) et c'est celle que je recommande.
Préciser au passage que la rotation **mémorisée du mode** restaurée par un rollback est celle du
snapshot, et non « la valeur imposée » du tiret précédent.

### `63-11` — `REQ-LUA-120` : non défini pour un pattern de frame — non bloquant

« vrai si et seulement si la grille du pattern contient au moins une tuile **en attente** […] ;
toujours faux pour un pattern d'un profil 3dn. »

Le type `Pattern` de SPEC-05 §4.2 couvre **deux** objets : « Pattern du profil, **ou pattern de
frame** d'une shape », et le pattern de frame est explicitement différent — « sa grille est la boîte
en tiles de la shape (`TSize`) ; ses instances sont les instances de frame de la shape ». Sa grille
contient des tuiles d'**écran**, qui ne sont jamais « en attente » au sens de `REQ-PROF-081` (ce
sont les tuiles du **profil** qui le sont). La valeur de `HasPlaceholders` sur
`shape.pattern` (le pattern de frame) n'est donc pas définie. Le critère esquive la question en
passant par `shape.refPattern` et par la parenthèse « (ou le même pattern obtenu autrement) ».

**Correction proposée** : « Pour un **pattern de frame** (SPEC-05 §4.2), la propriété rend la
valeur de son `refPattern`. » Et remplacer dans le critère « (ou le même pattern obtenu autrement) »
par les deux chemins à tester : `shape.refPattern.HasPlaceholders` et `shape.pattern.HasPlaceholders`
(même valeur), plus `Pattern` du registre (`REQ-LUA-085`).

### `63-12` — `REQ-LUA-121` : l'énoncé normatif dit deux choses contraires — non bloquant

L'exigence définit d'abord une sémantique complète (« vrai si `t` est une tuile de la variante de
l'instance et que cette tuile est pleine au sens de `REQ-SCN-087` … »), puis la retire entièrement
(« la méthode renvoie **faux** pour tout argument »). Un lecteur pressé implémentera la première
phrase, qui est impossible — l'API Lua n'expose aucun objet tuile, ce que j'ai vérifié sur SPEC-05
§4.2 (les types `Instance`, `Pattern` et `Shape` n'offrent que `tSize`, `TileCount`, `Size`, aucun
accès à une tuile). Le §4 le classe déjà en divergence **(D)**.

**Correction proposée** : faire de « `Instance:IsSolidTile(t)` (méthode) renvoie **toujours** faux,
pour tout argument, sans lever d'erreur » l'énoncé normatif, et déplacer la sémantique de la 2.1
en Note (« dans 3dSen 2.1, la méthode rend vrai si `t` est une tuile pleine au sens de
`REQ-SCN-087` ; Zaxnes n'expose pas d'objet tuile (SPEC-05 §4.2), la méthode existe pour qu'un
script qui l'appelle ne perde pas ses hooks, `REQ-LUA-062` »), comme `AMD-040` `REQ-LUA-115` le fait
déjà pour `BuildExportMeshes()`.

---

## Ce qui est acquis (vérifié indépendamment, aucune correction demandée)

Tout ce qui suit a été recalculé ou recompté avec mon propre décodeur et mon propre code, à partir
du seul texte des amendements :

- **`REQ-PROF-070`** — les 101 fichiers de `profiles-2.1` commencent par `0A 03 33 64 68`
  (**101/101**) et les 101 de `profiles` par `0A 03 33 64 6E` (**101/101**) ; aucun croisement.
- **`REQ-PROF-071`** — `BalloonFight.bytes` (2.1) porte les champs **1, 2, 3, 6, 7, 12 dans cet
  ordre**, `Version = "5"`, `Ids` de **5 280 octets = 330 identifiants**, et
  `PayloadHash = 6b3da0c6…a57c` **égal** au SHA-256 de `Payload`. Sur les 101 : deux ordres de
  champs seulement, `(1, 2, 3, 6, 7, 12)` pour 84 fichiers et `(1, 2, 3, 6, 7, 8, 9, 10, 12)` pour
  17 — tous conformes à l'ordre croissant annoncé, avec les chaînes nulles et la liste vide omises ;
  `Version = "5"` partout ; champ 5 absent partout.
- **`REQ-PROF-074`** — `Mega Man 2.bytes` (2.1) : les **6 768** tiles portent **toutes** exactement
  `{3, 5}`, **aucune** ne porte 1 ni 2 ; **1 809** variantes portent le champ 25, dont **1 663** de
  valeur 1 et **146** de valeur 2 ; **146** portent le champ 24 ; **aucune** le champ 19 ; payload
  `Type = "3dn"`, `Ver = "2"`. Les cinq chiffres du critère sont exacts. Les 10 variantes des
  emplacements mémorisés n'ont ni 19 ni 24 ni 25, ce qui est cohérent avec « `textureKind = 0` ≡
  champ 19 absent » et avec le défaut de `REQ-PROF-003`.
- **`REQ-PROF-074` point 3** — l'ordre racine annoncé est **exact** sur les 101 : les champs 3, 4, 5,
  6 d'abord en ordre croissant, puis 1, 2, 7 à 10 en ordre croissant (onze combinaisons observées,
  par exemple `(3, 4, 5, 6, 1, 2, 7, 8, 9, 10)` pour 13 fichiers), là où les 3dn sont en ordre
  croissant simple.
- **`REQ-PROF-074`, couverture du manifeste** — sur les 101 fichiers, **225 843 / 225 843** valeurs
  de champ 5 figurent dans `Ids` ; **zéro** manquante.
- **`REQ-PROF-075`** — les **quatre** vecteurs reproduits au bit près à partir du seul texte de
  `REQ-PROF-075` et `REQ-PROF-077` (tableau en `62-11`), préfixe, forme binaire et forme texte.
- **`REQ-PROF-076`** — les deux condensés sont exacts : tile nulle →
  `374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb`, tile `00 01 … 0f` →
  `be45cb2605bf36bebde684841a28f0fd43c69850a3dce5fedba69928ee3a8991`.
- **`REQ-PROF-077`** — la colonne « Préfixe » des quatre vecteurs est exacte, et la dernière phrase
  (`P` = 32 bits de poids faible de `L`) est vraie sur les quatre.
- **`REQ-PROF-078`, comptes du manifeste** — `Super Mario Bros.bytes` : **481** identifiants ;
  `Legend Of Zelda.bytes` : **1 035** ; `Mega Man 2.bytes` : **5 497**. Les trois chiffres cités
  sont exacts (seule la résolution depuis la ROM reste non exécutable ici, `62-5`). La règle iNES
  (en-tête de 16 octets, trainer de 512 si le bit 2 de l'octet 6, tailles NES 2.0 si
  `(octet 7 AND 0x0C) = 0x08`) est conforme à SPEC-02 §3.2 et à la documentation publique ;
  les ROMs citées n'exercent toutefois pas la notation exposant-multiplicateur, à signaler d'un mot.
- **`REQ-PROF-079`** — les trois retournements sont exacts (`f = 1` →
  `008040c020a060e0109050d030b070f0`, `f = 2` → `07060504030201000f0e0d0c0b0a0908`, `f = 3` →
  `e060a020c0408000f070b030d0509010`), et la convention (miroir vertical = ordre des 8 octets de
  chaque plan, miroir horizontal = ordre des 8 bits de chaque octet) est cohérente avec le
  placement de bit `y*8 + (7 − x)` de SPEC-03 §2 et avec `REQ-PROF-031`.
- **§5, notations** — « plan bas en `b[0..7]`, plan haut en `b[8..15]` », lus en 64 bits non signés
  petit-boutistes, donnent bien les plans en clair de `REQ-PROF-030` : vérifié contre le placement
  de bit de SPEC-03 §2.
- **`REQ-PROF-084`, ordre et tri** — l'ordre d'enveloppe annoncé (1, 2, 3, 4, 6, …, 12, nulles et
  listes vides omises) correspond aux deux ordres observés ; et `Ids` est **trié par forme texte en
  ordre ordinal dans 101/101** fichiers, **sans doublon dans 101/101**. La condition de
  `REQ-PROF-051` sur les clés de référence est tenable : aucun fichier, dans aucun des deux
  dossiers, ne contient deux objets tile **neufs** de même identité (0 doublon sur `(id, flip)` côté
  3dh, 0 sur `(plan bas, plan haut, flip)` côté 3dn), donc la réécriture octet pour octet par
  identité d'objet n'est pas contredite par les données.
- **`REQ-PROF-083`, partition 3dh** — les quatre comptes de gauche sont exacts (60 777 / 13 559 / 4
  / 142), y compris le partage « même taille » qui demande de comparer `X` à `tSize × 8` : seule la
  mise en correspondance avec le 3dn est fautive (`62-2`).
- **`REQ-SCN-087`, critère** — l'arithmétique des sommets et la géométrie du cas 4 × 4 sont justes
  (voir `63-13`).
- **`REQ-LUA-118`** — le critère d'`AMD-040` reste vrai :
  `tonumber(("2.1"):match("^%d+")) = 2 >= 2`.
- **`REQ-LUA-121`, prémisse** — exacte : SPEC-05 §4.2 n'expose aucun objet tuile.
- **Numéros et renvois** — les 20 numéros créés sont libres ; sur les 49 renvois `REQ` distincts des
  deux fichiers, **trois** sont fautifs (`REQ-E3D-048` en `63-7`, `REQ-SCN-012` en `63-8`, et le
  « 32 × 30 » qui contredit SPEC-04 §2 en `63-1`) et tous les autres existent et disent bien ce que
  le texte leur fait dire ; les 13 renvois `AMD` existent, **deux** manquent à l'appel (`AMD-058`
  en `63-2`, `AMD-059` en `63-9`).

---

## Déterminisme de `REQ-PROF-080` à `REQ-PROF-083` (question posée)

> *Un implémenteur peut-il, en ne lisant que ces quatre exigences et ce qu'elles citent, produire un
> résultat déterministe et comparable sans deviner ?*

- **`REQ-PROF-080` — non, trois devinettes.** (i) Aucune interface de lecture de la CHR-RAM n'existe
  dans SPEC-02 (`62-15` point 1) ; (ii) la base « dernière observation » n'est définie ni à la
  première frame ni sous run-ahead, rollback et chargement d'état, ce qui met `REQ-ARCH-012` en
  risque (`62-15` point 2) ; (iii) le point d'insertion est, lui, correctement fixé (« avant la
  reconnaissance des patterns », donc entre les étapes (b) et (c) de `REQ-E3D-001`), et la clause de
  monotonie est exactement ce qu'il faut. Avec la correction (b) de `62-15` — rebalayage complet de
  la CHR-RAM tant qu'il reste des tuiles en attente — l'exigence devient déterministe **et** sans
  état caché, donc comparable ; c'est la rédaction que je recommande.
- **`REQ-PROF-081` — oui.** Les quatre clauses sont observables et closes : inégalité avec toute
  tuile d'écran, égalité entre tuiles en attente par `(identifiant, flip)`, pixels à 0 dans la
  texture par défaut et absence de maillage, construction en géométrie `Default` sans `fullWarp`
  puis reconstruction. Son critère (pattern 2 × 1) est exécutable sur un 3dh **synthétique**, qu'un
  tiers peut fabriquer avec `REQ-PROF-071`, `-074` et `-075`. Deux précisions de confort, non
  bloquantes : dire que les autres paramètres de la variante (`sizeZ`, `rm`, `solid`, `emptyColor`,
  déformation) sont inchangés pendant la construction dégradée, et régler le cas identifiant /
  condensé de `62-6`.
- **`REQ-PROF-082` — indécidable faute de critère** (`62-14`). Le format est libre, ce qui est bien,
  mais aucun effet n'est exigé, donc rien n'est comparable. La phrase « Les tests de déterminisme se
  font avec un cache vide » est la bonne précaution et doit rester.
- **`REQ-PROF-083` — non, deux devinettes.** La formule de XOR est, elle, parfaitement déterministe
  et explicitement « tableau contre tableau, quels que soient les formats et les tailles » — c'est
  la bonne façon de l'écrire. Mais (i) la clause de repli est ambiguë et sépare 146 variantes de 0
  (`62-3`), et (ii) le critère ne peut pas être exécuté tel quel (`62-2`). Les deux corrections sont
  rédactionnelles ; aucune ne demande de nouvelle mesure côté sale, puisque les comptes mesurés ici
  suffisent à réécrire le critère.

---

## Conclusion — gate R2

**Non franchi.** `AMD-062` et `AMD-063` demandent chacun une **v1.1** avant que le lot propre
démarre.

L'essentiel est pourtant acquis, et c'est important pour la suite : **le format 3dh est correctement
spécifié**. L'enveloppe, le schéma du payload, l'ordre canonique des champs, l'identifiant de tuile,
le préfixe, les retournements et la réécriture sans perte sont justes, étanches et vérifiés sur les
données citées — j'ai reproduit les quatre vecteurs au bit près à partir du seul texte, et les
dix-sept comptes de `REQ-PROF-071`, `-074`, `-078`, `-083` et `-084` sont exacts. Sont **prêts tels
quels** : `REQ-PROF-070`, `-071`, `-074` (hors la précision de `62-6`), `-075`, `-076`, `-077`,
`-079`, `-081` et `-084`, ainsi que `REQ-LUA-118`. Ce qui bloque est circonscrit à **quatre zones** :
la donnée de référence (§1 et §9, `62-1` et `62-4`, à corriger sur mesures, ce que ce rapport fournit),
la **résolution pendant la partie** (`REQ-PROF-080`, qui a besoin d'une exigence d'accès CHR-RAM dans
SPEC-02, `62-15`), le **critère de `REQ-PROF-083`** (`62-2`, `62-3`), et la **testabilité sans ROM
commerciale** (`62-5`). Côté `AMD-063`, le mécanisme lui-même est bon et ses comptes de sommets sont
justes ; ce qui bloque est un numéro de grille (`63-1`), deux documents absents de l'en-tête
(`63-2`, `63-9`), une définition manquante (`63-3`), un périmètre non fixé (`63-6`) et une
contradiction sur le contenu du snapshot (`63-10`).

**Recommandation d'ordre de traitement** : d'abord `62-15` et `63-3` (les deux seuls points qui
demandent une décision technique, pas une réécriture), puis `62-1`, `62-2`, `62-3`, `62-4`, `62-5`,
`63-1`, `63-2`, `63-6`, `63-9`, `63-10` (bloquants rédactionnels, tous les chiffres nécessaires sont
dans ce rapport), puis les non bloquants au fil de la plume — en traitant `62-7` (retrait de
`TileHashCache`) tôt, puisqu'il est de coût nul. Une seconde relecture R2 n'aurait à recalculer que
les comptes réécrits de `62-1`, `62-2` et `62-4` ; tout le reste du cœur d'`AMD-062` est acquis,
vecteurs et arithmétique compris.

Les deux amendements n'ont pas été modifiés par cette relecture, ni aucun autre fichier de `spec/`
hors ce rapport et la ligne de journal ; rien n'a été commité, et rien n'a été touché dans
`impl/` ni dans `wt-dotnet21`.
