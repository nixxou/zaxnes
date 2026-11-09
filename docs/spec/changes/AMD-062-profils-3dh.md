# AMD-062 — Profils au format « 3dh » (3dSen 2.1) : lecture, résolution des tuiles, réécriture

Version : 1.2 | 2026-11-09 | **Normatif** (R1, choix du propriétaire du 2026-11-09 : Zaxnes lit
**les deux formats** et ne convertit jamais un profil ; l'import depuis 3dSen marche en 1.x, 2.0
et 2.1). **Amende** SPEC-03 (REQ-PROF-001, 020, 030, 032, 033, 051, 052, 062, 063, §4.1), AMD-030 §5 et §6,
AMD-037 (identifiant lu dans l'enveloppe), AMD-039 §4 et §5 (correction de données). **Complète**
SPEC-02 §3.8 (REQ-CORE-123). Ajoute REQ-PROF-070 à REQ-PROF-084.
Historique : 1.0 (rédaction) ; 1.1 (relecture `R2-review-AMD-062-063.md` : 62-1 à 62-15) ;
1.2 (relecture `R2-review-AMD-062-063-v1.1.md` : 62-1 reliquat, N-62-1 à N-62-6).
Priorité : P1 sauf mention.

## 1. Constat

3dSen 2.1 (2026-11-07) livre ses 101 profils intégrés dans un nouveau format, « 3dh » : le
profil ne contient plus les deux plans de chaque tuile, mais un **identifiant de contenu** de
16 octets. Les tuiles sont retrouvées dans la ROM de l'utilisateur au chargement, ou pendant la
partie pour celles qu'on ne trouve pas dans la ROM (jeux à CHR-RAM qui compressent leurs
tuiles). Les textures explicites des variantes sont stockées en OU exclusif avec la texture par
défaut, que l'on recalcule depuis les tuiles. Tout le reste du profil (patterns, variantes,
scripts, réglages) est inchangé : c'est le schéma de SPEC-03.

Mesuré sur une installation réelle de la 2.1 : l'import de Zaxnes échoue sur les 101 profils
(0 importé, 101 en échec ; il réussit 101 sur 101 sur la 2.0). La 2.1 réécrit en outre au
démarrage les profils utilisateur de **son** dossier de données dans ce format.

Données de référence pour les tests : `assets-neutral/profiles-2.1/` (les 101 profils 3dh et
`SHA256SUMS.txt`). Les noms sont ceux de `assets-neutral/profiles/`, à ceci près que deux y
gardent l'apostrophe que `profiles/` écrit `_` (`Castlevania II - Simon's Quest (USA)`,
`Hogan's Alley`) : un test qui apparie les deux dossiers remplace `'` par `_`.

Par rapport à leurs homologues de `assets-neutral/profiles/` (profils 1.x), une fois tuiles et
textures résolues, et au sens du **modèle décodé** (le 3dh écrit en outre le champ 12 `skipGeo` de
tous ses patterns, que le 3dn omet souvent : REQ-PROF-084) :
- **97** profils ont exactement le même contenu décodé ;
- les **quatre** autres : `Super Mario Bros` change par **5 de ses 296 variantes** (tableau au §9 :
  `fullWarp` de l'une en 2.0 ; `sizeZ` et `offset.z` de trois autres en 2.1 ; données libres
  `CData[4..7]` des cinq) et par 6 de ses réglages ; `DQ3_DW3` (+71 patterns, +71 variantes) et
  `Final_Fantasy_USA` (+21 patterns, +22 variantes) gagnent des patterns, **en 2.0** ; les
  **scripts** diffèrent pour `DQ3_DW3`, `Final_Fantasy_USA` et `punchout` (2.0, AMD-040) ; les
  **réglages** diffèrent pour ces trois-là (dès la 2.0) **et** pour `Super Mario Bros`, le seul
  écart appliqué par REQ-PROF-041 étant `Light.Direction` de `DQ3_DW3` ((1 ; −2,5 ; 2) →
  (1 ; 1,5 ; 2)) ;
- les **98** profils autres que `Super Mario Bros`, `DQ3_DW3` et `Final_Fantasy_USA` ont les mêmes
  patterns, variantes, tuiles et textures.
Entre 2.0 et 2.1, seul `Super Mario Bros` change (4 variantes, `Volume`, premier emplacement
mémorisé, §9) : tout le reste de ces écarts date de la 2.0.
Les **99** profils autres que `DQ3_DW3` et `Final_Fantasy_USA` sont dits **appariables** : leurs
patterns, variantes et tuiles se correspondent un à un, dans l'ordre du fichier.
AMD-039 §5 est corrigé en ce sens : ses quatre profils « ne changent que par leurs scripts » est
faux pour `DQ3_DW3` et `Final_Fantasy_USA`, qui gagnent aussi des patterns, et pour
`Super Mario Bros`, dont les scripts sont inchangés et les réglages changent ; leurs réglages
changent aussi, pour les quatre.

## 2. Principe retenu

- Zaxnes accepte un profil 3dn **ou** 3dh partout où il accepte un profil (catalogue, profils
  utilisateur, import d'un fichier, import depuis 3dSen). Le format se reconnaît au contenu, pas
  à l'extension.
- Un profil n'est **jamais converti** d'un format à l'autre, ni à l'import, ni au chargement, ni
  au démarrage. Un fichier n'est réécrit que sur action explicite (export, éditeur), et alors dans
  **son propre format** (REQ-PROF-084).
- La résolution des tuiles d'un profil 3dh se fait avec la ROM lancée ; elle n'est jamais
  nécessaire pour l'import, qui reste instantané.

## 3. Reconnaissance et enveloppe

**REQ-PROF-070** — Reconnaissance. Un profil dont les cinq premiers octets sont
`0A 03 33 64 68` (champ 1 = la chaîne `"3dh"`) est un profil **3dh** et DOIT être lu selon le
présent amendement. Tout autre fichier est lu comme un profil 3dn (SPEC-03, inchangé).
REQ-PROF-020 est amendée : le champ 1 d'un 3dn vaut toujours `"3dn"` ; un fichier qui commence
par la signature 3dh n'est **pas** refusé par REQ-PROF-020.
- Critère d'acceptation (auto) : les 101 fichiers de `assets-neutral/profiles-2.1` sont reconnus
  3dh, les 101 de `assets-neutral/profiles` sont reconnus 3dn.

**REQ-PROF-071** — Enveloppe. Le message racine d'un 3dh est `HashEnvelope` :

| N° | Nom | Fil | Type | Sens |
|---|---|---|---|---|
| 1 | `Type` | LEN | string | `"3dh"` |
| 2 | `Version` | LEN | string | version de l'enveloppe (REQ-PROF-072) |
| 3 | `Payload` | LEN | octets | le profil proprement dit (REQ-PROF-074) |
| 4 | `Hashes` | LEN | string, répété | condensés hérités, 64 chiffres hexadécimaux minuscules (REQ-PROF-076) |
| 6 | `PayloadHash` | LEN | string | SHA-256 de `Payload`, 64 chiffres hexadécimaux minuscules |
| 7 | `Id` | LEN | string | identifiant du profil |
| 8 | `Title` | LEN | string | titre |
| 9 | `Maker` | LEN | string | auteur du profil |
| 10 | `GameDev` | LEN | string | éditeur du jeu |
| 11 | `Des` | LEN | string | description |
| 12 | `Ids` | LEN | octets | identifiants de tuile, 16 octets chacun, concaténés (REQ-PROF-075) |

Le champ 5 n'est jamais écrit. Les règles générales de SPEC-03 §3.1 (champs dans n'importe quel
ordre, champ inconnu ignoré, absence = nul) s'appliquent. Comme en SPEC-03 §2, `HashEnvelope` et
les noms de champs de ce document sont des noms de commodité propres à la spec, sans portée
normative : seuls les numéros de champs et les types de fil comptent. La chaîne
`"3dSen.TileId.v1"` de REQ-PROF-075 est, elle, une donnée du format.

**Manifeste.** L'ensemble des chaînes du profil est `Hashes` ∪ { forme texte (REQ-PROF-075) de
chaque identifiant de `Ids` }. Toutes les comparaisons au manifeste (doublons ici, présence en
REQ-PROF-074, égalité en REQ-PROF-081, résolution en REQ-PROF-078 et 080) se font sur ces chaînes.
- Critère d'acceptation (auto) : `BalloonFight.bytes` (2.1) a les champs 1, 2, 3, 6, 7, 12 dans
  cet ordre, `Version = "5"`, 330 identifiants (5 280 octets), et `PayloadHash` égal au SHA-256 de
  `Payload`.

**REQ-PROF-072** — Validation de l'enveloppe. Le profil DOIT être refusé (REQ-PROF-033 :
chargement en bloc ou rien, profil vide, jeu qui démarre) si :
- `Version` n'est pas `"1"`, `"2"` ou `"5"` (la version `"3"` n'a jamais été diffusée ; la
  version `"4"` avait des identifiants qui ne se résolvent plus) ; la version `"4"` est refusée
  avec le message `Hash profile envelope version 4 is no longer supported: its tile identifiers cannot be resolved. Import the original profile again.`,
  les autres avec `Unsupported hash profile envelope` ;
- `Payload` est absent (`Unsupported hash profile envelope`) ;
- `PayloadHash` est présent et diffère du SHA-256 de `Payload` (`Damaged hash profile payload`) ;
- le manifeste est invalide (`Invalid hash profile manifest`) : longueur de `Ids` non multiple
  de 16 ; une entrée de `Hashes` qui n'est pas 64 chiffres hexadécimaux minuscules (en version
  `"2"` et `"5"`, une entrée de 32 chiffres est aussi acceptée et vaut un identifiant de tuile
  sous forme texte, REQ-PROF-075) ; une même chaîne présente deux fois dans le manifeste (§3).
Ces textes sont des messages de **journal** (anglais, non traduits). À l'import, la cause affichée
d'un 3dh refusé est le troisième texte de REQ-UI-084 (« … has an obsolete or wrong profile format
… »), comme pour un 3dn illisible.
- Critère d'acceptation (auto) : on modifie un octet de `Payload` → refus, message « Damaged » ; on
  remplace `Version` par `"4"` → refus avec le message de la version 4 ; par `"3"` → `Unsupported
  hash profile envelope` ; on duplique les 16 premiers octets de `Ids` → `Invalid hash profile
  manifest` ; on ajoute à `Hashes` la forme texte du premier identifiant de `Ids` → même refus ;
  dans chaque cas le jeu démarre sans profil.
- Notes : les 101 fichiers de `profiles-2.1` sont en version `"5"`, avec `Hashes` vide et aucune
  tuile en champ 4. Les branches « versions 1 et 2 », « entrées texte » et « condensés hérités »
  se testent sur des enveloppes **synthétiques**.

**REQ-PROF-073** (P2) — En-tête de bibliothèque. Pour la liste des jeux (REQ-PROF-062) et
l'import (REQ-PROF-063), les métadonnées d'un 3dh sont lues dans l'**enveloppe** : champs 7 à 11,
mêmes numéros et même sens que l'en-tête d'un 3dn. Le `Payload` n'est pas décodé pour cela.
- Critère d'acceptation (auto) : l'en-tête de `Super Mario Bros.bytes` (2.1) donne `Id =
  "Jhj6E7iOysxT"`, comme son homologue 3dn ; un 3dh dont un octet du `Payload` a été modifié
  (refusé au chargement, REQ-PROF-072) apparaît malgré tout dans la liste des jeux avec l'`Id` et
  le `Title` de son enveloppe.
- Notes : les 101 enveloppes portent un `Id` ; 56 des 101 homologues 3dn en portent un, égal
  (56 sur 56) ; les 45 autres n'en ont pas (AMD-037, §8).

## 4. Contenu du profil

**REQ-PROF-074** — `Payload` est un profil au schéma de SPEC-03 (`ProfileFile`), avec trois
différences :
1. **Tuile** (`TilePlanes`, §4.2 de SPEC-03) : les champs 1 et 2 (plans encodés) sont
   **absents** ; le champ 3 (`flip`) est présent ; la tuile porte à la place **soit** le champ 5
   (LEN, 16 octets : identifiant de tuile, REQ-PROF-075), **soit** le champ 4 (LEN, string :
   condensé hérité de 64 chiffres, ou identifiant de 32 chiffres en version `"2"`/`"5"`,
   REQ-PROF-076). Une tuile qui porte quand même les champs 1 et 2 est une tuile ordinaire.
   La valeur des champs 4/5, mise sous forme texte, DOIT être une chaîne du manifeste (§3) ;
   sinon le profil est refusé (`Tile hash absent from profile manifest: <chaîne>`).
2. **Variante** (`ShapeParams`, §4.4 de SPEC-03) : le champ 19 (`tex`) est absent ; deux champs
   nouveaux le remplacent :

   | N° | Nom | Fil | Type | Sens |
   |---|---|---|---|---|
   | 24 | `xorTex` | LEN | `TexRect` (même encodage que le champ 19) | texture XOR texture par défaut |
   | 25 | `textureKind` | VARINT | int32 | 0 = pas de texture, 1 = texture par défaut, 2 = texture explicite stockée en XOR |

   Correspondance avec SPEC-03 REQ-PROF-035 : `textureKind = 0` ≡ champ 19 absent ;
   `textureKind = 1` ≡ marqueur nul ; `textureKind = 2` ≡ texture explicite, reconstruite par
   REQ-PROF-083. Un autre `textureKind`, ou `2` sans champ 24, est un profil invalide
   (`Invalid profile XOR texture`).
3. **Ordre des champs racine** : les champs 3, 4, 5, 6 sont écrits d'abord, en ordre croissant,
   puis les autres champs racine (1, 2, 7 à 11) dans l'ordre croissant (REQ-PROF-084). Le lecteur
   accepte tout ordre.
`Type` et `Ver` du payload gardent leur sens de SPEC-03 (`"3dn"`, `"2"`/`"3"`, drapeau
`IsV2Format`) : le payload d'un 3dh est un 3dn valide à ces trois différences près.
- Critère d'acceptation (auto) : dans `Mega Man 2.bytes` (2.1), toutes les tuiles ont les
  champs {3, 5} et aucune n'a 1 ou 2 ; 1 809 variantes ont le champ 25 (1 663 de valeur 1,
  146 de valeur 2), 146 ont le champ 24, aucune le champ 19 ; le payload a `Type = "3dn"`,
  `Ver = "2"`. Sur les 101 fichiers : 225 843 tuiles, toutes de champs {3, 5} ; 74 482 variantes
  hors emplacements mémorisés, partagées en 60 777 de `textureKind = 1`, 13 559 de `textureKind =
  2` avec un champ 24 de format 0 et de taille `tSize × 8`, 4 de format 0 et d'autre taille, 142 de
  format 1 ; les 1 010 variantes des emplacements mémorisés n'ont pas de champ 25 (pas de
  texture, comme en SPEC-03).
- Critère d'acceptation (auto) : ordre des champs racine du payload, sur les 101 fichiers :
  3 (s'il y a des scripts), 4 (s'il y a des patterns permanents), 5 (s'il y en a de temporaires),
  6, puis 1, 2, puis 7 à 10 quand ils sont présents (onze combinaisons sur les 101 ;
  `empty.bytes` donne 3, 6, 1, 2).

## 5. Identifiant de tuile

Notations : une tuile **brute** est faite de 16 octets `b[0..15]`, ceux que l'on trouve dans la
ROM : plan bas en `b[0..7]`, plan haut en `b[8..15]`. `low` et `high` sont ces deux moitiés lues
comme entiers 64 bits non signés **petit-boutistes**. Ce sont exactement les plans en clair de
SPEC-03 REQ-PROF-030 (après décodage par `K`), pour une tuile **non retournée** (`flip` est
stocké à part). Toute l'arithmétique est modulo 2^64.

**REQ-PROF-075** — L'identifiant d'une tuile brute est le couple (`L`, `H`) de deux entiers de 64
bits :
- `D = SHA-256("3dSen.TileId.v1" ‖ b[0..15])` (15 octets ASCII puis les 16 octets bruts) ;
- `L = P | (u32le(D[0..3]) << 32)`, où `P` est le préfixe de REQ-PROF-077 ;
- `H = u64le(D[4..11])`.
Forme binaire (champ 5, `Ids`) : `L` sur 8 octets petit-boutistes puis `H` sur 8 octets
petit-boutistes. Forme texte : 32 chiffres hexadécimaux minuscules, `H` d'abord puis `L`, chacun
sur 16 chiffres.
- Critère d'acceptation (auto) — vecteurs :

| Tuile brute `b[0..15]` (hex) | Préfixe `P` | Forme binaire (hex) | Forme texte |
|---|---|---|---|
| `00000000000000000000000000000000` | `06e28b2d` | `2d8be206b548f95b38bf8a3e2d7f50f9` | `f9507f2d3e8abf385bf948b506e28b2d` |
| `ffffffffffffffffffffffffffffffff` | `5343027f` | `7f024353624f541ff1301725c2461716` | `161746c2251730f11f544f625343027f` |
| `000102030405060708090a0b0c0d0e0f` | `870dcae2` | `e2ca0d87ba7804d4c5cccdc426124e91` | `914e1226c4cdccc5d40478ba870dcae2` |
| `ffffffffffffffff0000000000000000` | `1dbbccea` | `eaccbb1d67be65f246453106c3c0a5d3` | `d3a5c0c306314546f265be671dbbccea` |

**REQ-PROF-076** (P2) — Condensé hérité : `SHA-256(b[0..15])` en 64 chiffres hexadécimaux
minuscules. Une tuile portant un condensé hérité se résout comme une tuile portant un
identifiant (REQ-PROF-077), en comparant ce condensé.
- Critère d'acceptation (auto) : la tuile nulle a le condensé
  `374708fff7719dd5979ec875d56cd2286f6d3cf7ec317a3b25632aab28ec37bb` ; la tuile `00 01 … 0f`,
  `be45cb2605bf36bebde684841a28f0fd43c69850a3dce5fedba69928ee3a8991`.

**REQ-PROF-077** — Préfixe `P` (32 bits), fonction de (`low`, `high`), qui sert de filtre rapide
avant le SHA-256 (constantes d'interopérabilité, au même titre que `K`) :
1. `(m_lo, m_hi)` = produit 128 bits non signé de `(low XOR high XOR 0x59973F0033362349)` par
   `0x9E3779B185EBCA87` ;
2. `m_lo = m_lo + 0x03C0000000000000` ;
3. `h2 = high XOR 0xC202797692D63D58` ; `m_hi = m_hi + h2 + (h2 AND 0xFFFFFFFF) × 0x85EBCA76` ;
4. `m_lo = m_lo XOR byteswap64(m_hi)` (inversion de l'ordre des 8 octets) ;
5. `v = m_lo × 0xC2B2AE3D27D4EB4F` ; `v = v XOR (v >> 37)` ; `v = v × 0x165667919E3779F9` ;
   `v = v XOR (v >> 32)` ;
6. `P = v AND 0xFFFFFFFF`.
Les étapes 1 à 5 sont une fonction de mélange mesurée, à reproduire au bit près ; leur forme n'a
pas de sens propre. Par construction (REQ-PROF-075), `P` est aussi formé des 32 bits de poids
faible de `L` : une implémentation PEUT ne calculer le SHA-256 que pour les tuiles dont le préfixe
figure dans le manifeste.
- Critère d'acceptation (auto) : la colonne « Préfixe » des vecteurs de REQ-PROF-075.

## 6. Résolution des tuiles

**REQ-PROF-078** — Résolution à partir de la ROM. Quand un profil 3dh est chargé pour une ROM,
avant la première frame convertie :
1. on délimite dans le fichier iNES de la ROM la zone PRG puis la zone CHR (en-tête de 16
   octets, trainer de 512 octets sauté si le bit 2 de l'octet 6 est posé ; tailles NES 2.0 si
   `(octet 7 AND 0x0C) = 0x08`, y compris la notation exposant-multiplicateur quand le quartet
   haut vaut 15 ; zones tronquées à la taille du fichier) ;
2. on parcourt la PRG puis la CHR **à chaque position d'octet** `o` telle que `o + 16` ne dépasse
   pas la fin de la zone (positions non alignées comprises) ; pour la fenêtre `b = fichier[o..o+15]`
   on calcule les identifiants des **quatre** retournements de `b` (REQ-PROF-079), et chaque
   identifiant (ou condensé hérité) présent dans le manifeste et pas encore résolu reçoit la
   tuile brute correspondante (la version retournée) ;
3. on peut arrêter dès que tout le manifeste est résolu.
Une ROM qui n'est pas iNES ne résout rien (toutes les tuiles restent en attente). Une ROM
compressée (`.zip`) est lue comme pour le lancement.
- Critère d'acceptation (auto) : pour chacun des 99 profils appariables (§1), un fichier iNES
  **synthétique** (en-tête de SPEC-02 §5.1) dont la zone CHR contient, chacune à une position
  **non alignée** sur 16, les 16 octets bruts de chaque tuile du 3dn homologue → tout le manifeste
  du 3dh est résolu ; le même fichier privé d'une tuile → cette tuile seule reste en attente ; une
  tuile placée **retournée** (`f = 1`, REQ-PROF-079) est aussi résolue.
- Critère d'acceptation (ROM, facultatif, ignoré si la ROM est absente) : `Super Mario Bros.bytes`
  avec `Super Mario Bros. (World).nes` → 481 sur 481 ; `Legend Of Zelda.bytes` avec
  `Legend of Zelda, The (USA).nes` → 1 035 sur 1 035 ; `Mega Man 2.bytes` avec
  `Mega Man 2 (USA).nes` → 5 353 sur 5 497 (les 144 autres attendent la partie, REQ-PROF-080).

**REQ-PROF-079** — Retournements d'une tuile brute (`f` = bit 0 miroir horizontal, bit 1 miroir
vertical, comme `flip`) : le miroir vertical inverse l'ordre des 8 octets **de chaque plan** ; le
miroir horizontal inverse l'ordre des 8 bits de chaque octet ; `f = 3` applique les deux.
- Critère d'acceptation (auto) : `00 01 … 0f` donne, pour `f = 1`,
  `008040c020a060e0109050d030b070f0` ; pour `f = 2`, `07060504030201000f0e0d0c0b0a0908` ;
  pour `f = 3`, `e060a020c0408000f070b030d0509010`.

**REQ-PROF-080** — Résolution pendant la partie. Tant qu'il reste des tuiles en attente, à
chaque frame **affichée** (AMD-041 REQ-LOOP-067 : celle que la conversion traite), **avant** la
reconnaissance des patterns (SPEC-04) :
- les tuiles de la frame (fond et sprites, plans en clair, non retournés) et, si la cartouche a
  de la CHR-RAM, **tous** les blocs de 16 octets alignés de la CHR-RAM (lue par REQ-CORE-123),
  sont confrontés au manifeste, avec leurs quatre retournements, comme en REQ-PROF-078. La règle
  est sans état : aucune base de comparaison d'une frame à l'autre. Une implémentation PEUT éviter
  de recalculer un bloc inchangé, à condition que l'ensemble résolu à chaque frame soit celui du
  parcours complet ;
- une tuile résolue devient une tuile ordinaire : elle participe à la reconnaissance **de cette
  même frame**, et les patterns, variantes et textures qui la contiennent sont recalculés avant le
  rendu de cette frame.
La résolution est monotone : une tuile résolue ne revient jamais en attente pendant la session,
ni sur un retour arrière, un run-ahead, un chargement d'état ou un reset. L'ensemble des tuiles
résolues est une connaissance de session, comme le cache (REQ-PROF-082) : il ne fait partie ni
des savestates ni des snapshots de rollback. Il ne change le résultat d'une frame que pour les
patterns dont une tuile **qui n'est pas à l'écran** est encore en attente (REQ-PROF-081) : une
tuile visible est toujours résolue avant la reconnaissance de sa frame. REQ-ARCH-012 s'entend donc
à ensemble résolu initial égal (cache vide pour les tests) ; deux exécutions de même ROM, profil,
état initial, entrées et run-ahead donnent les mêmes frames.
- Critère d'acceptation (auto) : cœur piloté par le harnais avec une ROM synthétique à CHR-RAM ;
  le profil 3dh a deux tuiles absentes de la ROM ; à la frame 10 le test écrit l'une en CHR-RAM
  (alignée, retournée verticalement), à la frame 20 il écrit l'autre et l'affiche en sprite → la
  première est résolue dès la frame 10 et pas avant, la seconde à la frame 20, avant la
  reconnaissance de cette frame, et son pattern est reconstruit avant le rendu ; deux exécutions
  donnent les mêmes captures ; un rollback de 5 frames après la frame 20 ne remet aucune des deux
  en attente.
- Critère d'acceptation (auto) : chemin « écran » seul — moteur sans source de CHR-RAM, la tuile
  n'arrive que par les tuiles de la frame → résolue avant la reconnaissance de cette frame, et
  son pattern est reconnu dans cette frame. (Sur une cartouche dont toute la CHR est de la RAM,
  une tuile visible est aussi un bloc aligné de la CHR-RAM : les deux chemins se testent donc
  séparément, Q-178 §1.)
- Critère d'acceptation (ROM, facultatif) : `Mega Man 2.bytes` avec `Mega Man 2 (USA).nes` : 5 353
  chaînes résolues au chargement ; après l'intro et l'entrée dans un niveau, **toute tuile
  dessinée** est résolue (les 144 chaînes restantes ne correspondent à aucune tuile de ce dump et
  restent en attente sans effet visible, Q-178 §2) ; sur une frame capturée par `--dump-meshes`,
  toute tuile dessinée dont le pattern n'a plus de tuile en attente a le même maillage qu'avec
  `assets-neutral/profiles/Mega Man 2.bytes`.

**REQ-PROF-081** — Tuile en attente. Jusqu'à sa résolution, une tuile en attente :
- n'est égale à aucune tuile de l'écran (elle ne peut pas servir à reconnaître un pattern) ;
- est égale à une autre tuile en attente si et seulement si elles portent la même chaîne de
  manifeste (§3) et le même `flip` (REQ-PROF-032 étendue) ; une tuile portée par son condensé
  hérité et la même tuile portée par son identifiant sont donc distinctes tant qu'elles sont en
  attente, et égales une fois résolues (mêmes plans) ;
- dans la texture par défaut de son pattern, laisse ses 8 × 8 pixels à 0, et sa cellule n'a pas
  de maillage ; une variante dont au moins une tuile est en attente est construite avec la
  géométrie `Default` sur ses cellules résolues seulement (pas de `fullWarp`), puis reconstruite
  avec sa géométrie propre quand la dernière tuile est résolue.
- Critère d'acceptation (auto) : un pattern 2 × 1 dont une tuile est en attente se dessine avec
  sa seule cellule résolue ; après résolution, comme le même pattern lu depuis un 3dn.

**REQ-PROF-082** (P2) — Cache. Une implémentation DEVRAIT conserver les tuiles résolues d'un
profil (par `Id` de profil, dans le dossier de données de l'utilisateur, format libre) et les
appliquer au chargement suivant avant la résolution par la ROM. Le cache ne contient que des
couples chaîne de manifeste → 16 octets, chacun vérifiable (la chaîne se recalcule des 16 octets) ;
il est sans effet sur le format du profil. Un cache illisible, ou une entrée dont la chaîne ne se
recalcule pas, est ignoré. Les tests de déterminisme se font avec un cache vide.
- Critère d'acceptation (auto) : un 3dh résolu avec la ROM synthétique de REQ-PROF-078, puis
  rechargé avec une ROM qui ne contient aucune de ses tuiles, a toutes ses tuiles résolues avant
  la première frame ; un cache tronqué ou de contenu arbitraire est ignoré sans erreur (toutes les
  tuiles repartent en attente) ; les octets du fichier de profil sont inchangés.

## 7. Textures

**REQ-PROF-083** — Reconstruction d'une texture de variante 3dh (une fois les tuiles du pattern
résolues) :
- `textureKind = 0` : pas de texture ;
- `textureKind = 1` : texture par défaut du pattern (SPEC-03 REQ-PROF-035, calculée depuis les
  tuiles résolues ; une cellule en attente donne des pixels 0) ;
- `textureKind = 2` : avec `X` = champ 24 (largeur, hauteur, format et tableaux `data`, `extra`
  de SPEC-03 §4.5) et `B` = texture par défaut (format 0) : la texture a les dimensions et le
  format de `X` ; pour chaque indice `i` de 0 à `len(X.data) − 1` :
  `data[i] = X.data[i] XOR (i < len(B.data) ? B.data[i] : 0)` (sur 32 bits) et
  `extra[i] = X.extra[i] XOR (i < len(B.data) ? B.extra[i] : 0)` (sur 16 bits ; un élément de
  `X.extra` absent, tableau absent ou plus court que `X.data`, vaut 0 ; `B`, de format 0,
  a `len(B.extra) = len(B.data)`). Le XOR se fait **tableau contre tableau**, quels que soient les
  formats et les tailles.
  Tant qu'une tuile du pattern est en attente **et** que les dimensions **ou** le format de `X`
  diffèrent de ceux de `B`, la variante utilise `B` (mesuré : 146 variantes concernées sur les 101
  profils, 142 de format 1 et 4 de taille différente).
- Critère d'acceptation (auto) : sur les **99** profils appariables (§1), après résolution complète
  des tuiles à partir des tuiles du 3dn homologue, la k-ième variante (ordre du fichier, hors
  emplacements mémorisés) a la texture de la k-ième variante du 3dn : 55 575 de `textureKind = 1`
  ont pour homologue un marqueur nul ; 12 418 de format 0 et même taille, 130 de format 1 et 4 de
  taille différente ont pour homologue une texture explicite égale (`data` et `extra`), soit
  68 127 variantes. `DQ3_DW3` et `Final_Fantasy_USA` ne s'apparient pas (patterns ajoutés en
  2.0) : seule leur partition de REQ-PROF-074 est vérifiée.

## 8. Réécriture, export, import

**REQ-PROF-084** (P2) — Réécriture sans perte d'un 3dh. Un profil 3dh chargé puis réécrit DOIT
rester un 3dh. L'écrivain canonique 3dh :
- enveloppe : champs 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12 dans cet ordre, chaînes nulles et listes
  vides omises ; `Version = "5"` ; `Ids` = identifiants sous forme binaire **triés par leur forme
  texte** (ordre ordinal) ; `Hashes` = condensés hérités et identifiants texte encore référencés,
  triés de même ; `PayloadHash` recalculé ;
- payload : écrivain canonique de SPEC-03 REQ-PROF-051, avec quatre écarts : l'ordre racine de
  REQ-PROF-074 point 3 ; la tuile écrite `3` puis `5` (ou `4`) ; la variante `… 23, 24, 25` (24
  omis si `textureKind ≠ 2`) ; le champ 12 (`skipGeo`) de **chaque** pattern des listes 4 et 5
  écrit, même faux, là où l'écrivain canonique d'un 3dn reproduit l'absence lue (27 330 patterns
  des 101 profils 1.x) ; les patterns des emplacements mémorisés (`Settings.14`/`15`) gardent de
  part et d'autre l'absence lue (480 sur 1 010) ;
- une tuile résolue est réécrite par son identifiant (jamais par ses plans), une tuile en attente
  par la valeur lue ; une texture de variante est réécrite selon `textureKind` : 1 si elle est
  identique à la texture par défaut, 2 (XOR) sinon, 0 si absente.
- Critère d'acceptation (auto) : décoder puis réécrire chacun des 101 fichiers de
  `assets-neutral/profiles-2.1` redonne un fichier identique octet pour octet (101 sur 101), sans
  ROM (aucune tuile résolue) et avec les ROMs synthétiques de REQ-PROF-078 pour les 99 profils
  appariables (toutes tuiles résolues) ; pour `DQ3_DW3` et `Final_Fantasy_USA`, 173 et 66 chaînes (une 67e est le miroir d'une tuile connue, résolue par l'un des quatre retournements, Q-178 §3)
  du manifeste restent en attente faute de tuile brute connue, et la réécriture est identique dans
  les deux cas.
- Notes : l'export d'un 3dn (REQ-PROF-052) reste un 3dn. Écrire un 3dh à partir d'un 3dn n'est
  pas demandé (P3, option d'export éventuelle, non implémentée dans ce lot).

**REQ-PROF-062 / 063** (amendées) — Les fichiers examinés et importés sont reconnus par leur
contenu (REQ-PROF-070) ; le dossier des profils utilisateur est parcouru pour les extensions
`.3dn` **et** `.3dh` ; le sélecteur d'import propose les deux. Un fichier importé est copié **tel
quel** (pas de conversion). L'`Id` d'un 3dh est celui de l'enveloppe ; AMD-037 (identifiant
dérivé du contenu quand l'`Id` est absent ou vide) s'applique au champ 7 de l'enveloppe. Les
dialogues d'AMD-056 s'appliquent sans changement : les causes de refus d'un 3dh sont celles de
REQ-PROF-072, la cause **affichée** est le troisième texte de REQ-UI-084 (§3).

**AMD-030 §5 et §6, AMD-039 §4** (amendés) — Import depuis une installation de 3dSen :
- le contrôle d'un TextAsset accepte un 3dn (SPEC-03) ou un 3dh (REQ-PROF-070 à 074, sans
  résolution des tuiles : enveloppe, manifeste et payload doivent se décoder) ; le fichier est
  écrit sans modification sous `<m_Name>.3dn` comme avant ;
- critère (installation réelle 2.1, facultatif) : 101 importés, 0 en échec ; un second import :
  101 déjà présents ;
- critère (sans installation) : un bundle synthétique dont le TextAsset est
  `assets-neutral/profiles-2.1/BalloonFight.bytes` importe ce fichier octet pour octet ;
- critères 1.x et 2.0 inchangés : les trois générations s'importent ;
- le fichier écrit garde l'extension `.3dn` même quand son contenu est un 3dh : c'est voulu, le
  format se reconnaît au contenu (REQ-PROF-070) ;
- un 2.0 puis un 2.1 importés tour à tour : pour les 56 jeux dont le 3dn porte un `Id`, les deux
  fichiers ont le même `Id` (règles d'AMD-030 §2 et de REQ-PROF-062 inchangées) ; pour les 45
  autres, le 3dn reçoit l'identifiant dérivé d'AMD-037 et le 3dh celui de son enveloppe : deux
  entrées distinctes de la bibliothèque, ce qui est accepté.

## 9. Données de référence et écarts assumés

- Profil `Super Mario Bros.bytes`, réglages (champ racine 6), 1.x contre 2.1 :

  | Champ | Nom (SPEC-03 §4.6) | 1.x | 2.1 | Appliqué (REQ-PROF-041) |
  |---|---|---|---|---|
  | 1 | `Light` (intensité seule) | 0,716150 | 1,011272 | non |
  | 9 | `VSync` | 0 | 1 | non |
  | 12 | `Volume` | 0,141679 | 0,425790 | non |
  | 14 | `SavedPattern3D`, 1er emplacement (`Config1`) | — | autre variante | oui (préréglage d'éditeur, sans effet sur le rendu, REQ-PROF-044) |
  | 22 | `Core` | 2 | 0 | non |
  | 37 | `SameSpPalette` | absent | 0 | oui (vaut le défaut) |

  Les champs 1, 9, 22 et 37 changent dès la 2.0 ; 12 et 14 changent en 2.1. Aucun de ces
  réglages n'a d'effet observable en jeu.
- Profil `Super Mario Bros.bytes`, variantes (ordre du fichier), 1.x contre 2.1 :

  | Variante (ordre du fichier, index 0-based) | Pattern (`tSize`) | Champ | 1.x | 2.1 | Version | Effet |
  |---|---|---|---|---|---|---|
  | 149 | 4 × 4 | 6 `fullWarp` | vrai | faux | 2.0 | visible (REQ-E3D-440/441) |
  | 168, 174, 232 | 2 × 2, 2 × 2, 3 × 2 | 1 `sizeZ` | 50 | 52 | 2.1 | visible |
  | 168, 174, 232 | idem | 10 `offset.z` | 12,0 | 14,0 | 2.1 | visible |
  | 149, 168, 173, 174, 232 | — | 17 `deltaOffset`, ligne 3 (`CData[4..7]`) | 1,5707964 | 1,570796 | 2.0 (149), 2.1 (autres) | données de script |

  Ces changements arrivent avec le profil importé ; Zaxnes n'a rien à coder pour eux.
- **(D) Pas de conversion.** 3dSen 2.1 convertit au démarrage les profils utilisateur de son
  dossier (original gardé en `.bak`) et convertit tout profil importé. Zaxnes ne convertit jamais
  (choix du propriétaire) : un 3dn reste lisible sans ROM et partageable tel quel.
- **(D) Reconstruction immédiate.** 3dSen peut montrer pendant quelques frames l'ancien maillage
  d'un pattern dont une tuile vient d'être résolue ; Zaxnes le reconstruit avant le rendu de la
  frame (REQ-PROF-080), ce qui garde le déterminisme.
- **(D) CHR-RAM sans état.** 3dSen ne regarde que les blocs de CHR-RAM qui ont changé depuis sa
  dernière observation ; Zaxnes parcourt toute la CHR-RAM tant qu'il reste des tuiles en attente
  (REQ-PROF-080). Même ensemble résolu, sauf pour la tuile entièrement nulle (16 octets à 0)
  tant que son bloc de CHR-RAM n'a jamais changé : Zaxnes la résout depuis la CHR-RAM, 3dSen
  seulement quand elle passe à l'écran.
- Le format du cache de 3dSen n'est pas repris (REQ-PROF-082).

## 10. SPEC-02 : lecture de la CHR-RAM

**REQ-CORE-123** (P1) — **Lecture de la CHR-RAM.** Le cœur DOIT offrir, entre deux frames, la
lecture seule de la CHR-RAM de la cartouche : sa taille en octets (0 si la cartouche n'a pas de
CHR-RAM) et ses octets, adressés linéairement à partir de 0, dans l'ordre de la mémoire de la
cartouche (indépendamment des banques actuellement visibles par le PPU). La lecture est sans effet
de bord sur l'émulation et sur son déterminisme.
- Critère d'acceptation (auto) : ROM synthétique en **mapper 0** avec l'octet 5 de l'en-tête à 0
  (CHR-RAM de 32 Kio par REQ-CORE-012, sans banques : `$0000`–`$1FFF` du PPU correspondent aux
  positions 0 à `0x1FFF`) ; le programme écrit par `$2006`/`$2007` les 16 octets `00 01 … 0f` à
  l'adresse PPU `$0130` → après la frame, la lecture donne taille 32 768 et ces 16 octets aux
  positions `0x130` à `0x13F` ; une ROM à CHR-ROM donne
  taille 0 ; deux exécutions, l'une lisant la CHR-RAM à chaque frame et l'autre jamais, donnent les
  mêmes frames.

## Historique

- 1.0 (2026-11-09) : rédaction (R1) après l'étude du delta 2.0 → 2.1 ; vecteurs vérifiés côté
  sale sur les 101 profils et sur trois ROMs.
- 1.1 (2026-11-09) : relecture R2. Données de référence corrigées (62-1, 62-4, 62-12) ; critères
  sans ROM commerciale (62-2, 62-5, 62-14) ; repli de texture univoque (62-3) ; CHR-RAM : nouvelle
  REQ-CORE-123 et règle sans état, connaissance de session et déterminisme (62-15) ; manifeste
  comparé sous forme texte, enveloppes synthétiques (62-6) ; noms de commodité, retrait du nom de
  cache et du budget interne (62-7, 62-8) ; messages de journal et cause d'import (62-9) ; `Id`
  des 45 profils sans `Id` (62-10) ; nits de REQ-PROF-077 (62-11) ; critère observable de
  REQ-PROF-073 (62-13).
- 1.2 (2026-11-09) : relecture R2b. Égalité de modèle décodé et 5 variantes de Super Mario Bros
  (62-1) ; quatrième écart de l'écrivain 3dh, `skipGeo`, et en-tête (N-62-1) ; réécriture des deux
  profils non appariables (N-62-2) ; ordre racine conditionnel (N-62-3) ; CHR-RAM de 32 Kio dans
  le critère de REQ-CORE-123 (N-62-4) ; frame affichée (N-62-5) ; cause affichée (N-62-6).
- 1.2a (2026-11-09) : relecture R2c. §1 repris mot pour mot de la proposition de N-62-7 (97 / 98
  profils, réglages de quatre profils) ; critère de REQ-CORE-123 en mapper 0 (N-62-8) ; index
  0-based au §9 (N-62-9).
- 1.2b (2026-11-09) : réponse R0 à Q-178 (lot V2-32) : critères de REQ-PROF-080 réécrits (deux
  chemins testés séparément, 144 chaînes de Mega Man 2 jamais affichées) ; 66 au lieu de 67.
