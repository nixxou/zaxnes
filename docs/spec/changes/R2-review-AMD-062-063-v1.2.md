# R2 — Relecture ciblée de la v1.2 d'`AMD-062` (profils « 3dh ») et d'`AMD-063` (transparence et Lua 2.1)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-11-09 | Version du rapport : 1.2
Objet : vérifier que la v1.2 lève le reliquat de `62-1` et les 12 points nouveaux de
`R2-review-AMD-062-063-v1.1.md` (`N-62-1` à `N-62-6`, `N-63-1` à `N-63-6`), contrôler les
**chiffres nouveaux** sur `assets-neutral/profiles-2.1` et `assets-neutral/profiles`, puis relire
**les seuls passages modifiés et leurs voisins immédiats** pour y détecter fuite, incohérence,
critère intestable ou renvoi cassé. **Ce n'est pas une relecture complète** : tout ce que la v1.1
déclarait acquis n'a pas été repris.

Entrées : `spec/changes/AMD-062-profils-3dh.md` et `AMD-063-transparence-lua-2-1.md` (**v1.2 du
2026-11-09, non modifiés par cette relecture**) ; `R2-review-AMD-062-063-v1.1.md` ; `SPEC-GUIDE` ;
`SPEC-03` §3.1 à §3.9, §4.1 à §4.7, §5.1 à §5.6 ; `SPEC-02` `REQ-CORE-012`, §5.1, §5.2, §3.8, §4
(tableau des mappers) ; `SPEC-04B` `REQ-E3D-420` à `-426`, `-440` à `-444`, `-452`, §4.1 ;
`SPEC-05` §4.2 (`Instance`, `Shape`, `Pattern`, `PatternAPI`) ; `AMD-012` (`REQ-SCN-065`),
`AMD-039` §5, `AMD-041` `REQ-LOOP-065` à `-067`, `AMD-042` `REQ-LOOP-069` à `-071`, `AMD-057` §2,
`AMD-058`, `AMD-059` ; `assets-neutral/profiles-2.1/` (102 entrées), `assets-neutral/profiles/`
(101 fichiers) ; `journal/JOURNAL.md`.

Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, et aucune tentative de
contournement n'a été faite. **Le serveur d'outils de rétro-ingénierie présent dans la session n'a
pas été utilisé** ; aucun connecteur externe n'a été sollicité. Les deux amendements, et tout autre
fichier de `spec/` hors ce rapport, sont inchangés ; rien n'a été commité ; `impl/` et `wt-dotnet21`
n'ont pas été touchés.

Méthode : **décodeur protobuf réécrit pour cette relecture** (lecteur de fil générique, puis
descente typée `HashEnvelope` → `ProfileFile` → `RefEnvelope` → `Pattern` → cellule nullable →
`Cell` → `TilePlanes`, `ShapeParams` → `TexRect` → `TexPayload`, `Settings` → `RecordedSlot`),
**réétalonné** sur les chiffres publiés de SPEC-03 : sur `assets-neutral/profiles` il retrouve
exactement 579 scripts dans 89 profils, 66 609 / 1 223 patterns, 1 010 emplacements, 74 389
variantes, 452 231 cellules non vides et 124 059 vides, 60 708 / 13 539 / 142 textures, 56 `Id`,
100 profils en version `"2"` et un en `"3"`, la répartition des variantes par pattern
(62 650 / 4 142 / 800 / 240) et 0 champ hors schéma, et il reproduit les neuf vecteurs d'encodage
de plans de SPEC-03 §5.2. Les vecteurs d'`AMD-062` (quatre identifiants de `REQ-PROF-075`, quatre
préfixes de `REQ-PROF-077`, deux condensés de `REQ-PROF-076`, trois retournements de
`REQ-PROF-079`) ont été **recalculés à partir du seul texte de la v1.2** et sortent tous au bit
près. La comparaison des deux dossiers a été refaite **champ par champ** (tuiles par identifiant
recalculé, variantes hors champs 19/24/25, textures de `textureKind = 2` recomposées par XOR et
comparées **élément par élément** avec `extra` absent valant 0, réglages, emplacements, scripts,
champs racine), et recoupée par une comparaison **octet pour octet** des sous-messages de réglages
et de scripts. C'est elle qui produit `N-62-7`.

Données **non vérifiables de ce côté** (convention déjà admise, aucune de mes vérifications n'en
dépend) : la répartition **2.0 / 2.1** des écarts (§1 : « ils sont déjà dans les profils de la
2.0 », « les champs 1, 9, 22 et 37 changent dès la 2.0 » ; colonne « Version » du tableau des
variantes au §9), la non-diffusion de la version d'enveloppe `"3"`, et les mesures sur installation
réelle (import 0/101, réécriture au démarrage). `assets-neutral` ne contient pas de corpus 2.0.
Aucun critère d'acceptation n'en dépend.

---

## Verdict

**Gate R2 : non franchi, pour une seule correction, et c'est la troisième fois que c'est la même
phrase.** Les 12 points nouveaux de la v1.1 sont **tous levés** — les deux bloquants compris — et
tous les chiffres nouveaux sont exacts. Ce qui reste est le reliquat de `62-1` : la phrase du §1 qui
dit combien de profils sont inchangés et pour qui les réglages changent est **encore fausse**, dans
l'autre sens cette fois. Les mesures sont fournies ci-dessous, et `AMD-039` §5 les confirme déjà de
son côté.

- **`N-62-1` et `N-63-1` (les deux bloquants) sont levés.** Le quatrième écart de l'écrivain 3dh
  (`skipGeo`) est écrit avec les bons chiffres, et l'en-tête renvoie désormais à `REQ-PROF-032` et
  `REQ-PROF-051`. « Tuile pleine » est découplée de l'opacité : la variante n'est plus qualifiée que
  par son algorithme de mesh, l'opacité **et le style de rendu** ne sont testés que sur l'instance
  dessinée, et le critère demandé (`shape.Alpha = 0.7` sur une variante d'opacité 1) est écrit.
- **Les 10 points non bloquants sont levés**, dont `N-62-3` et `N-62-4`, les deux qui faisaient
  échouer un test écrit à la lettre.
- **Tous les chiffres nouveaux sont exacts** : 173 / 191 et 67 / 70 chaînes non dérivables,
  27 330 et 480 pour `skipGeo`, les onze combinaisons d'ordre racine et `(3, 6, 1, 2)` pour
  `empty.bytes`, les cinq variantes de `Super Mario Bros` **champ par champ**, les six réglages du
  §9, 628 variantes d'opacité < 1 et aucune > 1, 43 profils qui écrivent `.Alpha`. Les
  68 127 textures du critère de `REQ-PROF-083` se reproduisent encore **élément par élément**,
  aucun échec.
- **Étanchéité : aucune fuite introduite.** Les passages nouveaux ne décrivent que des faits
  d'interopérabilité (présence d'un champ, ordre des champs, valeurs mesurées) et des membres Lua
  visibles depuis les profils livrés (`.Alpha`). Aucun nom interne, aucun pseudo-code, aucune
  architecture interne, aucune référence au code d'origine.
- **Aucun renvoi cassé.** Les renvois nouveaux ou modifiés existent tous et disent ce que le texte
  leur fait dire : `REQ-PROF-032`, `-051`, `REQ-LOOP-067`, `REQ-LOOP-070`, `REQ-E3D-440`/`-441`,
  `REQ-E3D-420`/`-422`/`-424`, SPEC-02 §5.2 (deuxième vecteur), SPEC-05 §4.2 (`Shape.refPattern`,
  `Instance.Alpha`, `Instance.RenderStyle`), `AMD-057` §2, `REQ-SCN-065` (`AMD-012`).
- **Ce qui bloque, en un point.** `N-62-7` — §1 annonce « **98** profils ont exactement le même
  contenu » et « les **réglages** diffèrent pour `Super Mario Bros` ». Mesuré : **97** profils ont
  un modèle décodé identique, et les réglages diffèrent pour **quatre** profils —
  `Super Mario Bros`, `punchout`, `DQ3_DW3` et `Final_Fantasy_USA`. `AMD-039` §5 écrit déjà
  « **97** des 101 profils sont identiques […] les **quatre** qui diffèrent », ce que la correction
  du §1 contredit.

---

## Tableau des points de la version 1.1

| Point | Statut v1.1 | Verdict v1.2 | Vérification |
|---|---|---|---|
| `62-1` — reliquat : §1 et §9, donnée de référence | bloquant, partiellement levé | **partiellement levé** | tableau des 5 variantes exact champ par champ ; égalité « de modèle décodé » écrite ; mais le compte et la liste des profils dont les réglages changent restent faux → `N-62-7` |
| `N-62-1` — `REQ-PROF-084`, écart `skipGeo` | bloquant | **levé** | 4ᵉ tiret écrit ; 67 924 / 67 924 contre 40 502 / 27 330, et 530 / 480 de part et d'autre ; en-tête complété de `REQ-PROF-032` et `-051` |
| `N-63-1` — « pleine » dépendait de l'opacité de la variante | bloquant | **levé** | définition de variante réduite au mesh ; opacité **et** style sur la seule instance ; critère `shape.Alpha = 0.7` ajouté ; `Instance.Alpha` et `Instance.RenderStyle` sont bien LE en SPEC-05 §4.2 |
| `N-62-2` — « toutes résolues » pour 2 fichiers | non bloquant | **levé** | 5 723 / 5 550 / **173** (191 objets tile) et 6 872 / 6 805 / **67** (70 objets) ; clause de réécriture identique écrite |
| `N-62-3` — ordre racine, champ 4 inconditionnel | non bloquant | **levé** | **onze** combinaisons sur les 101, toutes conformes à l'énoncé ; `empty.bytes` = `(3, 6, 1, 2)` |
| `N-62-4` — `REQ-CORE-123`, CHR-RAM de 8 Kio | non bloquant | **levé** (1 résidu) | critère porté à 32 Kio avec le 2ᵉ vecteur de SPEC-02 §5.2, qui donne bien mapper 1 / CHR-RAM 32 Kio ; résidu → `N-62-8` |
| `N-62-5` — « frame émulée » | non bloquant | **levé** | `AMD-041 REQ-LOOP-067` est bien « ce que voit la frame affichée », conversion 2D → 3D comprise |
| `N-62-6` — cause de refus affichée | non bloquant | **levé** | §8 renvoie la cause affichée au 3ᵉ texte de `REQ-UI-084`, les causes à `REQ-PROF-072` |
| `N-63-2` — plage d'opacité | non bloquant | **levé** | 0,1 à 0,8 sur les six profils cités (min `Bubble Bobble` 0,1, max 0,8), le plus souvent 0,5 (193) ou 0,7 (69) ; **628** variantes < 1 sur les 101, **0** > 1 |
| `N-63-3` — typage de `REQ-LUA-120` | non bloquant | **levé** | `Shape.refPattern` existe (SPEC-05 §4.2) ; `GetPatternWithTag` rend une `Instance`, d'où `.pattern` dans le critère |
| `N-63-4` — `REQ-LOOP-070` | non bloquant | **levé** | en-tête et corps citent `REQ-LOOP-069` **et** `-070` ; `REQ-LOOP-070` est bien la restauration |
| `N-63-5` — critères d'`AMD-057` §2 et de `REQ-SCN-065` | non bloquant | **levé** | « objet **opaque** » et « fond uniforme, couleur mélangée attendue » écrits dans `AMD-063`, en-tête complété |
| `N-63-6` — scène synthétique | non bloquant | **levé** | `geo = Default`, `fullWarp = faux`, `emptyColor = 0`, `sizeZ = 4`, aucun pixel de couleur 0 : `REQ-E3D-440` écrit « sur tous les axes » pour tout `k ∈ [0, D)`, donc les trois fentes sont pleines ; « `D` = le `sizeZ` du profil » écrit |

Soit **2 bloquants levés sur 2**, **10 non bloquants levés sur 10**, et le reliquat de `62-1`
toujours ouvert.

### Chiffres nouveaux, mesurés

| Affirmation de la v1.2 | Mesuré |
|---|---|
| 102 entrées dont `SHA256SUMS.txt` ; appariement `'` → `_` | 102 ; 101 sommes justes sur 101 ; 101/101 appariés ; les 2 noms à apostrophe confirmés |
| `DQ3_DW3` +71 patterns, +71 variantes | +71 / +71 (1 830 contre 1 759 ; 2 188 contre 2 117) |
| `Final_Fantasy_USA` +21 patterns, +22 variantes | +21 / +22 (3 905 contre 3 884 ; 4 167 contre 4 145) |
| scripts différents pour `DQ3_DW3`, `Final_Fantasy_USA`, `punchout` | exact, et pour **eux seuls** (comparaison octet pour octet du champ racine 3) |
| `Super Mario Bros` : 5 de ses 296 variantes | 296 variantes ; **5** concernées ; `fullWarp` vrai → faux sur **une** ; `sizeZ` 50 → 52 et `offset.z` 12,0 → 14,0 sur **trois** ; `deltaOffset` ligne 3 (`CData[4..7]`) 1,5707964 → 1,570796 sur les **cinq** — le tableau du §9 est exact ligne par ligne, `tSize` compris (4 × 4 ; 2 × 2, 2 × 2, 3 × 2) |
| §9, six réglages de `Super Mario Bros` | exact : `Light.Intensity` 0,7161504 → 1,0112723 ; `VSync` 0 → 1 ; `Volume` 0,1416787 → 0,4257903 ; emplacement 1 `Config1` (`sizeZ` 8 → 52, `geo` 4 → 0, `rm` 0 → 1) ; `Core` 2 → 0 ; `SameSpPalette` absent → 0 ; et ce sont les **seuls** six champs de `Settings` qui changent pour lui |
| `REQ-PROF-084` : `skipGeo` de 27 330 patterns, 480 sur 1 010 | 3dh 67 924 / 67 924 présents ; 3dn 40 502 présents, **27 330** absents ; emplacements **480** absents des deux côtés (et 27 330 + 480 = 27 810, le chiffre de `REQ-PROF-003`) |
| `REQ-PROF-084` : `Ids` trié et sans doublon | trié par forme texte en ordre ordinal **101/101**, sans doublon **101/101**, 179 468 identifiants tous référencés |
| `REQ-PROF-074` : comptes sur les 101 | 225 843 objets tile, tous de champs `{3, 5}` ; 74 482 variantes hors emplacements = 60 777 + 13 559 + 4 + 142 ; champ 19 : **0** ; les 1 010 variantes d'emplacements sans 19/24/25 ; repli : **146** |
| `REQ-PROF-083` : 55 575 / 12 418 / 130 / 4 = 68 127 | exact à l'unité, et **68 127 / 68 127 égales** après reconstruction (`data` **et** `extra`) |
| `REQ-PROF-071`, `-072` : enveloppe | `BalloonFight` 330 identifiants (5 280 octets), `PayloadHash` juste ; version `"5"` 101/101, `Hashes` vide 101/101, champ 5 absent partout, 0 tuile en champ 4, `Id` présent 101/101 ; deux ordres d'enveloppe seulement, conformes à l'ordre canonique |
| `AMD-063` §1 : 43 profils écrivent `.Alpha` | **43** profils, **62** occurrences, mêmes scripts dans les deux dossiers |
| `AMD-063` §1 : aucun profil n'utilise les 3 membres 2.1 | **0** occurrence de `HasPlaceholders`, `IsSolidTile`, `RotOverride`, `CameraControl` dans les 1 158 scripts |

---

## Points nouveaux

### `N-62-7` — §1 : « 98 profils » et « les réglages diffèrent pour `Super Mario Bros` » — **bloquant**

C'est la suite directe de `62-1`. La v1.2 écrit :

> - **98** profils ont exactement le même contenu ;
> - `Super Mario Bros` change par **5 de ses 296 variantes** […] et par ses réglages ; […]
> - les **scripts** diffèrent pour `DQ3_DW3`, `Final_Fantasy_USA` et `punchout` (2.0, AMD-040) ;
> - les **réglages** diffèrent pour `Super Mario Bros` (§9).

Mesuré, modèle décodé contre modèle décodé (`skipGeo` mis à part, seul écart d'encodage) :
**97** profils sur 101 sont identiques. Les **quatre** qui diffèrent, et par quoi :

| Profil | patterns / variantes | scripts | réglages (champs de `Settings`) |
|---|---|---|---|
| `Super Mario Bros` | 5 variantes | identiques | **6** : 1, 9, 12, 14, 22, 37 (§9) |
| `punchout` | identiques | différents | **6** : 1 (`Intensity` 0,946318 → 1,0112723), 9, 12 (0,5183012 → 0,215933), 21 (`Skybox` 1 → −1), 22 (`Core` 2 → 0), 37 (absent → 0) |
| `DQ3_DW3` | +71 / +71 | différents | **8** : 1 (`Light.Direction` **(1 ; −2,5 ; 2)** → **(1 ; 1,5 ; 2)** et `Intensity`), 9, 10 (`FrameRate` 60 → 90), 12, et 4 des 5 emplacements de `SavedPattern3D` |
| `Final_Fantasy_USA` | +21 / +22 | différents | **8** : 1 (`Intensity`), 3 (`CustomBgColor` −13 057 796 → −16 777 216), 9, 12, 21 (2 → −1), 37, et 2 emplacements |

Deux conséquences :

1. La phrase « **98** profils ont exactement le même contenu » est **fausse** au sens littéral de
   « contenu » : `punchout` est dans les 98 alors que ses scripts **et** ses réglages diffèrent. Elle
   est exacte si l'on garde la portée de la v1.0/v1.1 — « les mêmes **patterns, variantes, tuiles et
   textures** » : 98 = 101 − `Super Mario Bros` − `DQ3_DW3` − `Final_Fantasy_USA`. La v1.2 a élargi
   le mot sans élargir le périmètre.
2. La phrase « les **réglages** diffèrent pour `Super Mario Bros` » est **fausse** comme énoncé
   exhaustif : ils diffèrent pour quatre profils. Et un de ces écarts est **appliqué** par
   `REQ-PROF-041`, donc visible : `Light.Direction` de `DQ3_DW3`. (Les autres ne sont pas appliqués ;
   `SameSpPalette` prend le défaut ; les emplacements sont des préréglages d'éditeur,
   `REQ-PROF-044`.) La conclusion du §9 « aucun de ces réglages n'a d'effet observable en jeu »
   reste juste **pour les six de `Super Mario Bros`**, pas pour les quatre profils.

Cela contredit en outre `AMD-039` §5, que le §1 prétend corriger : ce §5 écrit déjà « **97** des 101
profils sont identiques octet pour octet […] Les **quatre** qui diffèrent (`DQ3_DW3`,
`Final_Fantasy_USA`, `Super Mario Bros`, `punchout`) ne changent que par leurs scripts Lua ». Les
quatre noms sont exactement les miens et le compte exactement le mien ; ce qui reste faux dans ce
§5, et que la ligne « Corrigé par AMD-062 §1 » ne corrige pas, c'est « ne changent que par leurs
scripts » — vrai pour aucun des quatre, puisque les réglages changent pour les quatre.

Le point est purement rédactionnel, les chiffres sont ici, et aucun critère d'acceptation des deux
amendements n'en dépend (`REQ-PROF-078` et `-083` se restreignent aux 99 appariables,
`REQ-PROF-084` porte sur les 101 fichiers tels quels).

**Correction proposée**, au §1 : « **97** profils ont exactement le même contenu décodé ; les
**quatre** autres : `Super Mario Bros` change par 5 de ses 296 variantes (§9) et par 6 de ses
réglages ; `DQ3_DW3` (+71 patterns, +71 variantes) et `Final_Fantasy_USA` (+21 patterns,
+22 variantes) gagnent des patterns ; les **scripts** diffèrent pour `DQ3_DW3`,
`Final_Fantasy_USA` et `punchout` ; les **réglages** diffèrent pour ces trois-là **et** pour
`Super Mario Bros`, le seul écart appliqué par `REQ-PROF-041` étant `Light.Direction` de `DQ3_DW3`
((1 ; −2,5 ; 2) → (1 ; 1,5 ; 2)). Les **98** profils autres que `Super Mario Bros`, `DQ3_DW3` et
`Final_Fantasy_USA` ont les mêmes patterns, variantes, tuiles et textures. » — et compléter la
correction d'`AMD-039` §5 d'un « leurs réglages changent aussi, pour les quatre ».

### `N-62-8` — `REQ-CORE-123`, critère : l'état de la banque CHR du MMC1 à la mise sous tension n'est fixé nulle part — non bloquant

Le critère nouveau est exécutable (le 2ᵉ vecteur de SPEC-02 §5.2 donne bien mapper 1 et
CHR-RAM 32 Kio), mais il repose sur la parenthèse « MMC1 à la mise sous tension : banque CHR 0 en
`$0000` ». SPEC-02 ne fixe pas l'état initial des registres du MMC1 (§4 ne décrit que le mapper, et
`REQ-CORE-004` ne va pas jusqu'aux registres de cartouche), et la documentation publique le donne
comme largement indéterminé. Un tiers qui choisit un autre état initial voit le critère échouer sur
l'adresse, pas sur la fonction mesurée.

**Correction proposée** : construire la ROM du critère en **mapper 0** avec l'octet 5 à 0
(`REQ-CORE-012` donne alors 32 Kio de CHR-RAM, sans banques, donc adressage identité), ou bien
ajouter à SPEC-02 l'état de mise sous tension des registres du MMC1.

### `N-62-9` — §9, tableau des variantes : la base de la numérotation n'est pas dite — non bloquant

Les indices du tableau (`149` ; `168, 174, 232` ; `149, 168, 173, 174, 232`) sont des indices
**0-based** dans la suite des variantes du fichier : mesuré, les variantes concernées sont la
150ᵉ, la 169ᵉ, la 174ᵉ, la 175ᵉ et la 233ᵉ. Le tableau dit « ordre du fichier » sans fixer la base ;
un test écrit en 1-based désigne quatre variantes voisines et passe à côté (la 174ᵉ, par exemple,
change bien — mais seulement par `deltaOffset`).

Nit de la même famille, déjà signalé en v1.1 et toujours là : la colonne « 1.x » du champ 14 porte
« — », qui se lit « absent » alors que l'emplacement est présent avec une autre variante
(mesuré : `Config1`, `sizeZ = 8`, `geo = 4`, `rm = 0`).

**Correction proposée** : « Variante (ordre du fichier, **index 0-based**) » ; et pour le champ 14,
remplacer « — » par « `Config1`, autre variante ».

---

## Ce qui est acquis (revérifié pour cette passe)

- **Étanchéité.** Relecture des passages nouveaux contre la liste du `SPEC-GUIDE` : le quatrième
  tiret de `REQ-PROF-084`, le tableau des variantes du §9, la définition réécrite de `REQ-SCN-087`,
  le critère de `REQ-CORE-123` et les deux phrases nouvelles du §1 d'`AMD-063` ne contiennent que du
  comportement observable, des faits d'interopérabilité et des membres Lua visibles depuis les
  profils. Rien à retirer.
- **Testabilité.** Les six critères touchés (`REQ-PROF-074` ordre racine, `REQ-PROF-084`
  réécriture, `REQ-CORE-123`, `REQ-SCN-087` × 3, `REQ-LUA-120`) sont tous chiffrés et exécutables
  sans ROM commerciale, à la réserve de `N-62-8`.
- **Numérotation.** Aucun identifiant nouveau dans la v1.2 ; `REQ-CORE-123`, `REQ-PROF-070` à
  `-084`, `REQ-SCN-087` et `REQ-LUA-118` à `-121` restent les seuls occupants de leurs numéros.
- **Cohérence interne d'`AMD-063`.** Avec la nouvelle définition, « pleine » est une propriété
  géométrique de (variante, tuile) close pour toute variante non MARCHING ; l'étape 1 (carte du
  fond) et l'étape 2 (retrait) ne lisent plus l'opacité que sur l'instance ; l'écart assumé
  « `Alpha` > 1 est opaque » reste cohérent avec « strictement inférieure à 1 » ; `AMD-057`,
  `AMD-058` §2, `REQ-SCN-040` et `REQ-SCN-065` sont cités avec le bon périmètre.

---

## Conclusion — gate R2

**Non franchi, pour une seule phrase.** La v1.2 fait tout le reste : les deux points bloquants de la
v1.1 sont levés — dont `N-63-1`, le seul qui demandait une décision, et elle est tranchée dans le
sens que les données imposaient —, les dix non bloquants le sont tous, aucun renvoi n'est cassé,
aucune fuite n'est introduite, et **tous les chiffres nouveaux sont exacts**, y compris les plus
coûteux à établir : les cinq variantes de `Super Mario Bros` champ par champ, les 173 et 67 chaînes
non dérivables, les 27 330 `skipGeo`, les onze ordres racine, et les 68 127 textures recomposées
sans un échec.

Ce qui reste est le reliquat de `62-1`, à son troisième tour : la phrase du §1 qui compte les
profils inchangés et nomme ceux dont les réglages changent. La v1.1 avait corrigé « 99 » en « 98 »
tout en élargissant « mêmes patterns, variantes, tuiles et textures » en « même contenu », et en
reprenant de la v1.1 de ce rapport une affirmation que je donnais pour vérifiée et qui ne l'était
pas : « les réglages ne changent que pour `Super Mario Bros` ». Mesuré cette fois sous deux angles
indépendants (champ par champ, puis octet pour octet sur le sous-message de réglages), ils changent
pour **quatre** profils, et `AMD-039` §5 le disait déjà. La rédaction proposée en `N-62-7` ferme les
deux lectures d'un coup : 97 au sens du contenu décodé, 98 au sens des patterns, variantes, tuiles
et textures, et la liste exacte des quatre.

Les deux points non bloquants nouveaux (`N-62-8`, `N-62-9`) sont rédactionnels et chiffrés ;
`N-62-8` mérite d'être traité tôt, puisqu'il fait reposer un critère P1 sur un état de registre que
la spec ne fixe pas.

Une quatrième relecture R2 n'aurait à vérifier que la nouvelle rédaction du §1, la base de
numérotation du tableau du §9 et la ROM du critère de `REQ-CORE-123` : tout le reste des deux
amendements est acquis.

Les deux amendements n'ont pas été modifiés par cette relecture, ni aucun autre fichier de `spec/`
hors ce rapport et la ligne de journal ; rien n'a été commité, et rien n'a été touché dans `impl/`
ni dans `wt-dotnet21`.
