# AMD-053 — Point de reprise, favoris, recherche, inspection des imports et ligne de commande

Version : 1.1 | 2026-10-31 | **Normatif** (R1, choix du propriétaire du 2026-10-31 : reprise des fonctions du rapport de comparaison avec 3dSen 2.0, §4.5, §4.6, §6.1 et §6.2, **sans** reprendre la présentation de la nouvelle interface de 3dSen). **Amende SPEC-07 REQ-LOOP-052** (énoncé et critère), **SPEC-09 REQ-PERS-033, REQ-PERS-041 point 1, REQ-PERS-042, REQ-PERS-050, REQ-PERS-052, REQ-PERS-055, §4.1.9 (`ui.sort_option`), §4.5.2 et §4.5.3**, **SPEC-10 REQ-UI-030** (dernière phrase) ; complète SPEC-10 (REQ-UI-007, REQ-UI-014, REQ-UI-084, inchangée) et AMD-010 (REQ-UI-201, REQ-UI-205, REQ-UI-207, REQ-UI-208).
Historique : 1.0 (rédaction) ; 1.1 (relecture R2-review-AMD-050-053 : 53-1 à 53-16, tous retenus ; pour 53-7, option (a) — les trois textes de REQ-UI-084 restent la seule source normative, les quatre libellés d'import de la v1.0 sont abandonnés et c'est REQ-PERS-041 point 1 qui est amendé ; pour 53-10, la ligne de commande accepte les 36 slots, hors de la limite d'exposition de REQ-UI-208 ; pour 53-11, les trois options sont à tiret simple, les formes à double tiret étant acceptées en alias ; pour 53-14, un emplacement « reprise » illisible est effacé **sans** notification).
Priorités : les cinq exigences sont indépendantes. Si REQ-LOOP-076 (P2) n'est pas retenue, REQ-LOOP-052 reste celle de SPEC-07, l'emplacement `resume` n'existe pas et `-noresume` est sans objet. Si REQ-UI-210 (P2) n'est pas retenue, le champ `favorite` et la valeur 2 de `ui.sort_option` n'existent pas et REQ-UI-211 subsiste seule. Si REQ-UI-211 (P2) n'est pas retenue, la recherche de REQ-UI-201 reste celle d'AMD-010. REQ-UI-213 (P3) est indépendante de toutes.

## 1. Besoin

3dSen 2.0 ajoute des fonctions utiles indépendamment de sa nouvelle présentation : reprendre la
dernière partie au lancement, marquer des jeux favoris, chercher un jeu, **nommer la cause** du
refus d'un profil importé, et lancer directement un jeu ou un emplacement depuis la ligne de
commande. Zaxnes garde son interface (AMD-010) et en reprend les fonctions.

Pour l'import, la lacune n'est pas l'absence de signalement : REQ-PERS-041 point 1 exige déjà une
notification de 4 s, et REQ-PERS-040 exige déjà d'ignorer **et de journaliser** un `.3dn` illisible
trouvé dans le dossier des profils. Ce qui est nouveau est de **choisir le message selon la cause**
du refus, et d'afficher titre et auteur en cas de succès.

Les fonctions visibles sont exprimées sur le **modèle de commandes** d'AMD-010 REQ-UI-205 et sur la
boîte de sélection de jeu/profil de REQ-UI-201, et non sur la grille de tuiles paginée de SPEC-10,
qu'AMD-010 §4 rend informative et P3 : une exigence P2 ne peut pas reposer sur des éléments de
présentation P3-informatifs. Leurs critères d'acceptation sont donc automatiques, comme AMD-010 §5
le prévoit, et la forme visuelle reste libre.

## 2. Exigences

### 2.1 Point de reprise (SPEC-07 §3.6)

**REQ-LOOP-076** (P2) — **Point de reprise** (divergence assumée **(D)** : le programme d'origine
n'écrit aucun savestate à la sortie). À la sortie normale du programme (REQ-LOOP-052), si l'état est
**En cours** ou **En pause** et que l'indicateur **Chargement** est faux, le programme DOIT écrire un
savestate complet (REQ-LOOP-040) avec sa vignette dans l'emplacement `resume` (REQ-PERS-050
amendée), distinct des 36 slots numérotés et du slot rapide. Si l'état est **Arrêté**, l'emplacement
`resume` DOIT être effacé.

**Ordre des opérations à la sortie** (précise REQ-LOOP-052, qui ne le donne pas) :
1. le hook Lua de sauvegarde de chaque script actif, dans l'ordre de REQ-LOOP-063, **avant** la
   capture (REQ-LOOP-041) ;
2. la capture de l'état et l'écriture du savestate de reprise ;
3. le hook Lua de fin et l'arrêt du jeu courant, puis l'arrêt de la boucle d'émulation ;
4. l'écriture des réglages et des métadonnées des emplacements (REQ-LOOP-052), dont l'entrée
   `resume`.

La condition de REQ-LOOP-041 (« l'état n'est pas Arrêté et il y a un jeu courant ») est exactement
celle énoncée ci-dessus ; sa clause « écrire le savestate **entre deux frames émulées** » est
satisfaite par construction, la boucle n'exécutant plus de frame émulée à l'étape 2.

**Vignette** : la capture prise à la dernière pause (REQ-LOOP-006) si elle existe et correspond au
jeu courant ; sinon — sortie depuis l'état En cours, où cette capture peut manquer — une capture de
la vue de jeu est prise au moment de l'écriture, au même format que REQ-PERS-051 (256 × 192 px RGB).

**Au lancement suivant** : si l'emplacement `resume` est occupé et que le profil et la ROM associés
sont toujours disponibles, l'interface DOIT proposer une entrée `Resume` / « Reprendre » portant sa
vignette, en tête de l'énumération des jeux (écran d'accueil de REQ-UI-207, boîte de sélection de
REQ-UI-201) et dans le modèle de commandes sous le nom `resume.last` (REQ-UI-205). L'activer charge
cet emplacement comme un chargement de slot (REQ-LOOP-044).

**Effacement et échec**, dans cet ordre de priorité :
1. l'emplacement est effacé à la **sortie** quand l'état est Arrêté ; cette règle l'emporte sur la
   suivante : ignorer la reprise puis quitter sans jouer efface donc le point de reprise ;
2. ignorer l'entrée `Resume` pendant une session ne l'efface pas : elle reste proposée jusqu'à la
   prochaine sortie ;
3. un emplacement `resume` illisible ou incompatible (REQ-LOOP-046, REQ-PERS-056), ou dont le jeu
   n'existe plus dans la bibliothèque (REQ-PERS-053, REQ-PERS-055), est effacé au démarrage :
   l'entrée n'est pas proposée, le démarrage est normal, **sans notification**, et le fait est
   journalisé.

- Critère d'acceptation (auto) : quitter pendant une partie puis relancer propose la reprise ;
  l'accepter restaure l'état (même empreinte de cœur que le savestate, REQ-LOOP-060).
- Critère d'acceptation (auto) : quitter depuis l'état Arrêté → l'emplacement est vide, son fichier
  est absent et l'entrée n'est pas proposée au lancement suivant.
- Critère d'acceptation (auto) : à la sortie en cours de partie, exactement un hook `Save` par
  script actif est appelé avant la capture, puis exactement un hook `End` par script actif, dans
  cet ordre.
- Critère d'acceptation (auto) : tronquer le fichier de l'emplacement `resume` → au démarrage,
  l'entrée n'est pas proposée, le fichier est supprimé, aucune notification n'est affichée, l'écran
  d'accueil de REQ-UI-207 est affiché.
- Notes : sans objet en libretro, où la bibliothèque, les emplacements et le cycle de vie
  appartiennent à l'hôte (REQ-PERS-086). La condition est écrite dans le vocabulaire de SPEC-07
  (états de REQ-LOOP-001 et indicateur Chargement de REQ-LOOP-021) : la formulation « ni en
  chargement ni en fin de partie » de la v1.0 était circulaire, « fin de partie » étant défini par
  REQ-PERS-012 comme « arrêt du jeu, changement de jeu, **sortie** ».

### 2.2 Favoris et recherche (SPEC-10 §3.4, AMD-010)

**REQ-UI-210** (P2) — **Favoris** (divergence assumée **(D)** : l'original n'a pas de favoris).
Chaque entrée de la bibliothèque DOIT pouvoir être marquée ou démarquée comme favorite par une
commande `favorite.toggle` du modèle de commandes (REQ-UI-205), applicable à l'entrée désignée et
offerte dans la boîte de sélection de jeu/profil (REQ-UI-201) et sur l'écran d'accueil
(REQ-UI-207). Le marquage DOIT être persisté par entrée dans le fichier de bibliothèque (champ
`favorite`, §4.5.2 amendé). L'énumération des jeux DOIT offrir un filtre « Favoris seulement »
(commande `library.filter_favorites`, à état coché) et l'ordre de tri (`ui.sort_option`,
REQ-UI-014) DOIT gagner l'option « Favoris d'abord » (valeur 2). Une entrée favorite DOIT être
**distinguée visuellement** des autres ; la forme du marqueur est libre.
- Critère d'acceptation (auto) : `favorite.toggle` sur un jeu, quitter, relancer → l'entrée est
  toujours favorite, et `library.json` porte `"favorite": true` pour elle seule.
- Critère d'acceptation (auto) : 2 favoris sur 10 entrées, `library.filter_favorites` coché →
  l'énumération donne exactement les 2 favoris ; décoché → les 10.
- Critère d'acceptation (auto) : `ui.sort_option` = 2 → l'énumération donne les favoris d'abord,
  puis les non-favoris, chaque groupe par titre croissant (REQ-PERS-033 amendée) ; les listes de
  slots (REQ-PERS-057) sont inchangées.
- Notes : sans objet en libretro (REQ-PERS-086).

**REQ-UI-211** (P2) — **Recherche** (divergence assumée **(D)**) : la recherche par titre déjà
offerte par la boîte de sélection de jeu/profil (AMD-010 REQ-UI-201) DOIT filtrer les entrées dont
le titre **contient** le texte saisi, dès la saisie, sans tenir compte de la casse ni des accents, et
se combiner avec le filtre des favoris (REQ-UI-210) : une entrée est retenue si elle satisfait les
deux. La comparaison DOIT se faire, sur le texte saisi comme sur le titre, après **décomposition
NFD**, retrait des marques diacritiques (catégorie Unicode `Mn`) et mise en minuscules
**invariante de culture**. La même recherche DOIT être offerte sur l'écran d'accueil (REQ-UI-207)
quand il énumère les jeux. Il n'est pas créé de seconde recherche : c'est celle de REQ-UI-201 qui
est précisée.
- Critère d'acceptation (auto) : la saisie « mario » garde les titres contenant « Mario » et
  « MARIO » ; la saisie « pokemon » garde « Pokémon » ; la saisie « bros » garde « Mario Bros. ».
- Critère d'acceptation (auto) : un champ vide n'exclut aucune entrée ; combinée au filtre des
  favoris, la saisie « mario » ne garde que les favoris dont le titre contient « mario ».
- Notes : la normalisation est écrite parce que deux implémentations libres divergeraient sur « œ »,
  « ß » ou « Pokémon ». Sans objet en libretro (REQ-PERS-086).

### 2.3 Inspection des profils importés (SPEC-10 §3.9, SPEC-09 §3.4)

**REQ-UI-212** (P2) — **Cause du refus d'un import de profil.** Avant d'ajouter un profil importé
(`.3dn`) à la bibliothèque (REQ-PERS-041), le programme DOIT le lire **selon SPEC-03, sans charger
ses patterns ni ses scripts** (seuls les champs racine 1, 7, 8, 9, 10 et 11 sont nécessaires,
REQ-PROF-062) et, en cas de refus, afficher la notification (gravité normale, 4 s, REQ-UI-084)
dont le texte correspond à la **cause** :

| Cause | Texte affiché (REQ-UI-084, seule source normative) |
|---|---|
| `Type` ≠ `"3dn"` (REQ-PROF-033) | `This file is not a 3dn file` |
| `Ver` = `"1"` ou non analysable (REQ-PROF-021) | `3dn format version <v>  is not supported` (deux espaces avant « is ») |
| protobuf mal formé, ou fichier illisible (REQ-PROF-001, REQ-PROF-033) | `<chemin>has an obsolate or wrong profile format. This profile is not compatible with the current version of Zaxnes.` (sans espace après le chemin, AMD-029 §3) |
| `Id` absent ou vide (REQ-PROF-063, REQ-PERS-041 point 1) | le même texte que la ligne précédente |

Un profil **accepté** DOIT afficher dans la notification d'import son titre et, s'il n'est pas vide,
son auteur : titre = `Title` (champ 8), à défaut le nom du fichier sans extension (REQ-PERS-034) ;
auteur = `Maker` (champ 9). Les deux champs sont facultatifs (REQ-PROF-022) ; un auteur vide est
**omis** de la notification.
- Critère d'acceptation (auto) : importer un fichier texte quelconque → le texte
  `<chemin>has an obsolate…` et rien n'est ajouté à la bibliothèque.
- Critère d'acceptation (auto) : importer un protobuf bien formé dont `Type` = `"nes"` →
  `This file is not a 3dn file` ; un profil dont `Ver` = `"1"` → `3dn format version 1  is not
  supported` (deux espaces) ; un profil sans `Id` → le texte `has an obsolate…`.
- Critère d'acceptation (auto) : importer un profil valide d'`Id` inconnu, `Title` = « Foo »,
  `Maker` = « Bar » → la notification porte « Foo » et « Bar » ; le même sans `Maker` → elle ne
  porte que « Foo » ; le même sans `Title`, fichier `Baz.3dn` → elle porte « Baz ».
- Notes : les quatre libellés de la v1.0 (`Unreadable file`, `Not a profile (missing signature)`,
  `Unknown profile version`, `Truncated profile`) sont **abandonnés**. Un profil est un message
  protobuf nu (REQ-PROF-001 : champs dans n'importe quel ordre, champ inconnu ignoré) : il n'a ni
  en-tête ni signature, « signature absente » ne correspond à rien, et « données tronquées » ne se
  distingue pas d'un « protobuf mal formé ». REQ-UI-084 et REQ-PERS-041 point 1 restent la seule
  source normative des textes, et AMD-036 en a fait les entrées anglaises de la table des textes,
  identiques caractère par caractère. Aucune divergence (D) n'est donc nécessaire ici.

### 2.4 Ligne de commande (SPEC-10 §3.1)

**REQ-UI-213** (P3) — **Ligne de commande** (complète REQ-UI-007) : le programme DEVRAIT accepter
aussi `-slot=<n>`, `-noresume` et `-emustats`. Ces trois options rejoignent la famille des options
**reprises du programme d'origine**, à tiret simple comme le `-id=` de REQ-UI-007 ; les formes à
double tiret `--slot=<n>`, `--no-resume` et `--emu-stats` DOIVENT être acceptées comme **alias**
**(D)**, par cohérence avec les options propres à Zaxnes (`--help`, `--list-profiles`,
`--import-3dsen` d'AMD-032 §5 et AMD-030 ; `--fov`, `--vsync`, `--skybox`, … d'AMD-026 ;
`--userdata` de REQ-PERS-071).

- `-slot=<n>`, `n` ∈ {`1`, …, `36`, `quick`} : le slot est chargé **après** l'étape 7 de
  REQ-LOOP-021 (l'état étant passé à En cours), comme un chargement de slot ordinaire
  (REQ-LOOP-044). La ligne de commande n'est pas tenue par la limite d'exposition de AMD-010
  REQ-UI-208 (10 emplacements dans les menus) et accepte les 36 slots que REQ-PERS-050 gère.
  Si le slot est **vide**, il est ignoré, le jeu de `-id` reste lancé et le fait est journalisé.
  Si le slot appartient à **un autre jeu**, c'est le jeu du slot qui est lancé, comme pour le slot
  rapide (REQ-PERS-052), et `-id` est alors sans effet. `-slot` **implique** `-noresume`.
- `-noresume` : l'entrée `Resume` de REQ-LOOP-076 n'est pas proposée à ce lancement ; l'emplacement
  `resume` n'est ni lu ni effacé.
- `-emustats` : le programme DOIT journaliser **une ligne par frame émulée** (temps d'émulation de
  la frame), en **anglais** (AMD-035 §2), dans le **fichier de journal** et non sur la console
  parente, qu'un lancement normal n'emprunte pas (AMD-032 §5).
- Une valeur invalide (slot hors domaine, identifiant inconnu) est ignorée et le programme démarre
  normalement.
- Critère d'acceptation (auto) : `-id=<jeu> -slot=3`, slot 3 occupé par ce jeu → le jeu démarre puis
  le slot 3 est chargé (empreinte de cœur égale à celle du savestate) ; `-noresume` n'affiche pas
  l'entrée `Resume` et laisse l'emplacement `resume` occupé.
- Critère d'acceptation (auto) : `-id=<jeu A> -slot=5`, slot 5 appartenant au jeu B → le jeu B est
  lancé à l'état du slot 5 ; `-id=<jeu> -slot=7` avec le slot 7 vide → le jeu démarre normalement et
  une ligne de journal signale le slot vide ; `-slot=99` → option ignorée, démarrage normal.
- Critère d'acceptation (auto) : `-emustats`, NTSC, 10 s → 600 ± 1 lignes dans le fichier de
  journal, et aucune écriture sur la console parente.

## 3. Changements documentaires

1. SPEC-07 REQ-LOOP-052, énoncé amendé : « … DOIT être exécutée **sans confirmation** et **sans
   sauvegarde automatique de la partie dans les slots numérotés ni dans le slot rapide** ;
   l'emplacement `resume` de REQ-LOOP-076 est écrit **(D)** ». Le reste (hook de fin, arrêt de la
   boucle, écriture des réglages et des métadonnées, fermeture) est inchangé et son ordre est
   précisé par REQ-LOOP-076.
2. SPEC-07 REQ-LOOP-052, critère amendé : « … aucun savestate n'est créé **dans les slots numérotés
   ni dans le slot rapide** ; l'emplacement `resume` est écrit quand REQ-LOOP-076 s'applique … », le
   reste inchangé.
3. SPEC-07 §3.6 : REQ-LOOP-076 ajoutée après REQ-LOOP-052.
4. SPEC-09 REQ-PERS-050 amendée : « L'implémentation DOIT gérer **36 slots numérotés** `1` à `36`,
   **un slot rapide** `quick` **et un emplacement de reprise** `resume` (REQ-LOOP-076), **communs à
   tous les jeux** », le reste de l'énoncé (identifiant, identifiant du jeu associé, vignette, date)
   inchangé et applicable aux 38 emplacements.
5. SPEC-09 REQ-PERS-052 : la clause d'exclusion du slot rapide est étendue — « Le slot rapide **et
   l'emplacement `resume`** n'apparaissent dans aucune des listes Load/Save » ; l'emplacement
   `resume` n'est écrit que par REQ-LOOP-076 et chargé que par la commande `resume.last`.
6. SPEC-09 REQ-PERS-042 : « effacer tous les slots associés à ce jeu (y compris le slot rapide **et
   l'emplacement `resume`**) ».
7. SPEC-09 REQ-PERS-055 : la cohérence au démarrage s'applique aussi à l'emplacement `resume` (jeu
   absent de la bibliothèque → vidé et fichier supprimé ; fichier manquant → vidé), en plus de la
   clause d'illisibilité de REQ-LOOP-076.
8. SPEC-09 REQ-PERS-033 amendée : un troisième ordre est ajouté — « **Favorites first** » : les
   entrées favorites d'abord, puis les non-favorites, **chaque groupe par titre croissant** (le
   critère de « A-Z ») ; cet ordre est **sans effet sur les listes de slots** (REQ-PERS-057), qui
   suivent « A-Z » quand `ui.sort_option` vaut 2.
9. SPEC-09 REQ-PERS-041 point 1 amendé : « fichier illisible ou sans identifiant → notification
   (4 s) avec le texte exact défini par SPEC-10 REQ-UI-084, **seule source normative**, **choisi
   selon la cause du refus d'après la table de REQ-UI-212** (le chemin y est suivi directement de
   `has an obsolate…`, sans espace) ».
10. SPEC-09 §4.1.9, ligne `ui.sort_option` réécrite (domaine et comportement) :

| Clé | Lua | N° | Type | Défaut | Domaine | Portée | Effet | UI | libretro | Comportement |
|---|---|---|---|---|---|---|---|---|---|---|
| `ui.sort_option` | `SortOption` | 36 | entier | 0 | 0 A-Z, 1 Recent, **2 Favorites first** | U | imm. | liste « Sort by » de la bibliothèque | — | tri de la bibliothèque et des slots, SPEC-10, REQ-PERS-033/057 ; la valeur **2 n'est jamais écrite dans le champ de profil 36** (0 est écrit à sa place) et une valeur 2 lue dans un profil est ignorée comme toute valeur de portée U (REQ-PERS-005 point 2) **(D)** |

11. SPEC-09 §4.5.2 `library.json` : champ `favorite` (booléen, défaut `false`) ajouté aux entrées de
    `games` ; `schemaVersion` porté à **2** ; un fichier en version 1 est migré à la lecture
    (`favorite` = `false` partout) et la copie `library.json.v1.bak` est conservée (REQ-PERS-024,
    REQ-PERS-088) :

```json
{
  "schemaVersion": 2,
  "notice": "Do not edit the id field.",
  "games": [
    { "id": "10", "title": "Digdug", "romPath": "D:\\roms\\digdug.nes", "lastPlayed": 1760950000, "cover": "Digdug_3dsen_10.png", "favorite": false }
  ]
}
```

12. SPEC-09 §4.5.3 `saves/index.json` : **38 entrées exactement** (`1`…`36`, `quick`, `resume`) ;
    `schemaVersion` porté à **2**, un fichier en version 1 étant migré à la lecture par l'ajout
    d'une entrée `resume` vide (REQ-PERS-024). Entrée ajoutée à l'exemple :
    `{ "id": "resume", "gameId": "10", "savedAt": 1760950200, "file": "resume.state", "thumbnail": "resume.png" }`.
13. SPEC-09 REQ-PERS-086 / §4.6 : en libretro, la bibliothèque, les emplacements et les jaquettes
    étant gérés par l'hôte, l'emplacement `resume`, les favoris et la recherche sont **sans objet**.
14. SPEC-10 REQ-UI-030, dernière phrase remplacée : « Il n'y a ni champ de recherche ni filtre »
    devient « L'énumération des jeux offre une recherche par titre (REQ-UI-211) et un filtre des
    favoris (REQ-UI-210) **(D)** ; l'original n'a ni l'un ni l'autre. »
15. SPEC-10 REQ-UI-084 : **inchangée** (ses trois textes restent la seule source normative, AMD-036).
    SPEC-10 REQ-UI-014 : inchangée ; la troisième option du tri vient de la ligne `ui.sort_option`
    amendée au point 10 et ne s'applique pas aux listes de slots.
16. SPEC-10 : REQ-UI-210 et REQ-UI-211 ajoutées au §3.4, REQ-UI-212 au §3.9, REQ-UI-213 au §3.1
    (après REQ-UI-007).
17. AMD-010 REQ-UI-205 : les commandes `favorite.toggle`, `library.filter_favorites`,
    `library.search` et `resume.last` sont ajoutées au modèle de commandes. REQ-UI-208 est
    inchangée : sa limite d'exposition ne s'applique pas à la ligne de commande (REQ-UI-213).
18. SPEC-v1.2-INDEX : REQ-LOOP-076 ajoutée à la liste de SPEC-07, REQ-UI-210 à REQ-UI-213 à celle de
    SPEC-10 ; mentions de mise à jour pour REQ-LOOP-052, REQ-PERS-033, REQ-PERS-041, REQ-PERS-042,
    REQ-PERS-050, REQ-PERS-052, REQ-PERS-055 et REQ-UI-030.

## 4. Libellés (AMD-035 ; anglais par défaut / français)

| Élément | Anglais | Français |
|---|---|---|
| entrée de reprise | `Resume` | `Reprendre` |
| marquage d'un favori | `Favorite` | `Favori` |
| filtre | `Favorites only` | `Favoris seulement` |
| option de tri | `Favorites first` | `Favoris d'abord` |
| champ de recherche | `Search` | `Rechercher` |

Les messages d'échec d'import ne figurent pas dans cette table : ce sont les trois textes de
REQ-UI-084, déjà inscrits comme entrées anglaises par AMD-036, avec leurs traductions françaises.

## 5. Étanchéité (pour mémoire)

L'en-tête écarte explicitement la présentation de la nouvelle interface de 3dSen. Les noms cités
(`-slot`, `-noresume`, `-emustats`, `Resume`, `Favorites only`, …) sont des chaînes
utilisateur-visibles, admises au même titre que le `-id=` de REQ-UI-007 ; `Type`, `Ver`, `Id`,
`Title` et `Maker` sont des faits d'interopérabilité du format conservé. Aucun nom interne, aucun
code, aucun algorithme recopié.

## 6. Questions

- Aucune.
