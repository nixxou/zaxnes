# R2 — Relecture ciblée de la spec v1.1-draft

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-20 | Version du rapport : v1.1
Entrées : `changes/R2-review-v1.md` (§18, items 1–25), `changes/AMD-001-r2-fixes.md`, `DECISIONS-R0.md` (v1.0 + arbitrages DEC-R2-\*), les 14 SPEC en version 1.1-draft.
Méthode : relecture **ciblée** (pas de relecture ligne à ligne). Ouverture de chaque exigence citée par §18 et par les lignes vérifiées d'AMD-001 ; relecture **intégrale** de SPEC-06 et SPEC-09 ; recalcul des vecteurs numériques de caméra, de HUD, de tunnel TV, de découpe et de visée Zapper.

Catégories inchangées : **LEAK**, **UNTESTABLE**, **INCONSISTENT**, **UNCLEAR**.

---

## 1. Items bloquants de R2-review-v1 §18 : verdict item par item

### Fuites (items 1 à 12)

| # | Item | Exigence relue | Verdict |
|---|---|---|---|
| 1 | 02-1 | SPEC-02 REQ-CORE-120 | **RESOLVED** — la phrase sur la bibliothèque native livrée et jamais chargée a disparu ; il ne reste que l'observable (liste à une entrée, nom du cœur en tête du savestate, message de refus). |
| 2 | 04-1 | SPEC-04 REQ-E3D-280 | **RESOLVED** — la mention des trois tampons est remplacée par « l'implémentation est libre tant que la règle est respectée ». |
| 3 | 04-2 | SPEC-04 REQ-E3D-342 | **RESOLVED** — notes réduites à « conservé pour la compatibilité des données ; il n'a aucun effet » ; plus de « jamais exécutée ». |
| 4 | 04-3 | SPEC-04 REQ-E3D-207 | **RESOLVED** — « `Age` vaut toujours 1 ; aucun profil embarqué ne lit cette valeur » ; plus de frames de début/fin de vie. |
| 5 | 04B-1 | SPEC-04B REQ-E3D-603 | **RESOLVED** — parenthèse de pré-allocation supprimée. |
| 6 | 04B-2 | SPEC-04B REQ-E3D-601 | **RESOLVED** — « dans la référence, une apparition retardée d'une ou deux frames est observable » ; plus de tâche de fond. |
| 7 | 04B-3 | SPEC-04B REQ-E3D-564 | **RESOLVED** — reformulation retenue : « ces capacités correspondent à celles de la référence, qui interrompt le chargement au-delà ». Les deux constantes restantes (512 par côté, 64 × 512 × 512 d'occupation totale) sont exprimées comme des **capacités exigées**, donc comme un résultat observable : conforme. |
| 8 | 04B-4 | SPEC-04B §6 Q-2 | **RESOLVED** — reformulation retenue mot pour mot ; plus de programme de shader ni d'outil d'export. |
| 9 | 04B-5 | SPEC-04B REQ-E3D-507 | **RESOLVED** — exigence **retirée** (solution plus propre que celle proposée) ; la ligne 213 porte la justification de résultat (REQ-E3D-523 compare les maillages indépendamment de l'ordre des faces). |
| 10 | 07-1 | SPEC-07 REQ-LOOP-033 | **RESOLVED** — reformulation retenue mot pour mot. |
| 11 | 11-1 | SPEC-11 §6 Q5 | **RESOLVED** — « le clip associé aux curseurs Vitesse et FOV n'a pas pu être déterminé en boîte noire » ; plus de liaison de scène ni d'argument non lisible. |
| 12 | 10-1 | SPEC-10 REQ-UI-011, 122 | **RESOLVED** — toutes les dimensions sont en proportions du cadre ou de la surface de référence 800×450, déclarée « disposition indicative ». |

**Bilan fuites : 12 / 12 corrigées. Aucune nouvelle fuite détectée dans les passages réécrits.**

### Incohérences bloquantes (items 13 à 24)

| # | Item | Verdict et vérification |
|---|---|---|
| 13 | T-1 / 09-1 / D-3 — sens de `Clipping` | **RESOLVED** — convention unique 0 gauche, 1 bas, 2 droite, 3 haut dans SPEC-04 REQ-E3D-002, SPEC-04B REQ-E3D-410 pt 4, SPEC-06 §2 et REQ-SCN-003, SPEC-09 §4.1.5. SPEC-09 n'est plus en contradiction avec elle-même : la table nomme `Clipping[3]` « marge haute » et la formule écrit `y + 8 > c3 (bord haut)`. Q-2 close. |
| 14 | T-2 / 06-1 / D-2 — repère de scène | **RESOLVED pour la géométrie, NEW ISSUE sur la branche Zapper.** La définition unique (SPEC-04B REQ-E3D-410) est cohérente, SPEC-06 §2 la reprend sans la redéfinir, et les quatre critères numériques de REQ-E3D-410 se vérifient. Recalculs faits : `R` et `T′` pour les clippings par défaut et « Super Mario Bros », `P*`, `E` en C1/C2/C3, `d₀` (70° → 189,65 ; 45° → 277,93 ; 95° → 150,25 ; 90° → 156,21), `d_HUD` (109,42 ; 90,12), les cinq pavés du tunnel et le repère TV de §5.3 : **tous exacts**. Le choix de conserver `C = (128, 120, 0)` et le cadre `[0, 240]` (contre ma proposition `C = (128, 128, 0)`) est accepté : il est désormais explicite, justifié et testable, et il porte le décalage de 8 unités là où il est observable. Reste la visée du Zapper : voir **N-1** au §3. |
| 15 | T-3 / 07-4 / 11-2 — cadence hôte | **RESOLVED** — cadence nominale 60/50 images/s dans SPEC-02 REQ-CORE-015, SPEC-07 §2 et REQ-LOOP-010, SPEC-11 REQ-AUD-004 et 007 (800 / 960 échantillons). Plus aucune occurrence de « 798,7 » ; « 60,0988 Hz » ne subsiste qu'à deux endroits, comme contre-exemple explicite. |
| 16 | T-4 / 04B-6 / D-1 — décalage ±0,1 | **RESOLVED** — propriétaire unique : le rendu. REQ-E3D-405 et 641 livrent les deux indicateurs sans le décalage, REQ-SCN-037 l'applique « une seule fois », TV-W2 distingue la transformation livrée (`z ∈ [0, 4]`) de la boîte affichée (`z ∈ [−0,1 ; 3,9]`), REQ-E3D-406 place `n` au bon endroit de la chaîne. |
| 17 | T-5 / 01-1 / 09-2 — périmètre du savestate | **RESOLVED sur le fond** — l'état appris est inclus (REQ-ARCH-013, REQ-LOOP-040, REQ-PERS-056 pt 6, REQ-E3D-008), la borne de taille est valable pour la session, la capacité fixe réservée est permise. Le périmètre **libretro** est resté en arrière : voir **N-2** au §3. |
| 18 | T-6 / 05-1 / 07-2 — ordre de restauration | **RESOLVED** — REQ-LOOP-044 charge le profil à l'étape 4 et affecte `PData` à l'étape 5, avec renvoi à REQ-LUA-072 et critère discriminant (« elles ne sont pas remises à zéro par le chargement du profil de B »). |
| 19 | 04-4 — collision `BL` / `TR` | **RESOLVED** — REQ-E3D-205 emploie `Bmin`/`Bmax` et renvoie explicitement aux grandeurs exposées ; SPEC-04 §4.4 définit `BL`/`TR` d'instance avec leur profondeur `∓s/2` et la mise en garde croisée ; le critère donne les deux jeux de valeurs (TV-LUA-05). |
| 20 | 03-1 — REQ-PROF-025 | **RESOLVED** — listes 14/15 reconstituées à 5 emplacements, divergence (D) assumée, critère conforme à DEC-03-Q03. |
| 21 | 10-2 — onglet initial | **RESOLVED** — REQ-UI-002 et REQ-UI-010 : `GAME` au premier affichage (DEC-10-Q6, D). |
| 22 | 10-3 — bouton « Site » | **RESOLVED** — retiré dans REQ-UI-039 ; aucune autre occurrence du bouton en dehors de la mention de retrait et de Q-5. |
| 23 | 04B-7 / 06-6 / D-6 — table `NES_RGB` | **RESOLVED** — propriétaire unique SPEC-04 §4.1 (table présente) ; REQ-E3D-560, REQ-SCN-034 et SPEC-06 §5.2 y renvoient. Plus de renvoi circulaire. |
| 24 | 08-4 — polarité de la gâchette | **RESOLVED** — SPEC-08 §4.1 : bit 4 = 1 relâchée, 0 pressée, autres bits nuls, exemples `0x10` / `0x00` ; REQ-INP-062 reprend la convention dans un critère exécutable en harnais. |

### Point de statut (item 25)

| # | Item | Verdict |
|---|---|---|
| 25 | REQ-E3D-521 / 522 | **RESOLVED** — numérotation des coins et des arêtes rendue **normative** (DEC-R2-521), indice de configuration `Σ 2^i` défini, REQ-E3D-522 appuyée dessus (« coin intérieur de valeur `c` de plus grand numéro »), justification remplacée par une justification de reproductibilité. |

**Bilan §18 : 25 items, 25 traités, 24 pleinement résolus ; l'item 14 est résolu pour la géométrie mais laisse une incohérence résiduelle sur la visée du Zapper (N-1).**

---

## 2. Nouveau verdict sur les deux documents rejetés

### SPEC-06 — Scène et rendu : **ACCEPT** (deux retouches de forme, non bloquantes)

Relecture intégrale (437 lignes).

- **LEAK : aucune.** Les notes qui citent la référence décrivent toutes un **écart ou une constante observable** : biais d'arrondi de 0,5 unité (REQ-SCN-008), crénelage de la carte d'ombre (REQ-SCN-030), facteur `(c0 − c2)/256` du repère TV (REQ-SCN-054), absence d'effet d'`AntiAlias` et de `GraphicQuality` (REQ-SCN-062), facteur de résolution limité à 1 sous Windows (REQ-SCN-063). Les skyboxes (REQ-SCN-050/051) restent des définitions mathématiques du résultat ; REQ-SCN-051 a gagné un critère **auto** sur cubemaps de test, ce qui lève l'UNTESTABLE 06-7.
- **Cohérence du repère : vérifiée** (voir item 14). Le document ne redéfinit plus le repère : §2 renvoie à REQ-E3D-410 et §4.3 justifie `C` par ce renvoi.
- **Numérique : vérifié.** Outre les vecteurs §5.1 et §5.3, j'ai recalculé la hauteur visible à FOV 70 (265,6 / 472,2), la proportion du HUD (78 %), la proportion de l'image NES à l'écran (87 % avec le clipping de SMB), le lissage critique (40,6 après 0,4 s ; 1,7 après 1,2 s), les cinq lignes de §5.2 et le critère (b) de REQ-SCN-051 (`srgb(2,2975 × lin(F0))` ≈ sRGB 0,306) : **tous conformes**.
- **Incohérences v1.0 levées** : 06-2 (REQ-SCN-059 « continue d'être lissée », scène non affichée), 06-3 (1024 texels), 06-4 (VSync 0/1/2, aligné sur SPEC-09 §4.1.1 et REQ-PERS-013), 06-5 (REQ-SCN-043 réduit à un renvoi normatif vers REQ-E3D-568, `h₀` compris), 06-6, 06-9 (textures et cubemap déclarées présentes dans `assets-neutral/`), 06-10 (REQ-SCN-063 : conservé dans le modèle, absent de l'UI, application facultative).
- Retouches de forme (non bloquantes) : (a) REQ-SCN-030 place un **DOIT normatif** (« 1024 texels ») dans son bloc *Notes* — à remonter dans le corps de l'exigence ; (b) §4.3 écrit « Ambiante | sRGB 118 (0,4627) » alors que REQ-SCN-027 et §5.2 raisonnent en linéaire 0,1812 — préciser « 0,4627 en sRGB normalisé ».
- UNCLEAR mineur restant : REQ-SCN-017 conditionne l'orbite à « sans touche Maj enfoncée » sans qu'aucun document ne dise ce que fait Maj + glissement.

### SPEC-09 — Persistance : **ACCEPT WITH FIXES** (N-2 et N-3 ci-dessous)

Relecture intégrale (649 lignes).

- **LEAK : aucune.** §4.4 reste une description de l'arborescence et de la clé de registre **visibles de l'utilisateur** (statut déjà validé en v1.0, §19) ; la forme « journal d'exécution » est corrigée.
- T-1 et T-5 levés dans le corps du document (voir items 13 et 17) ; 09-3 (onglet Save aligné sur REQ-UI-051), 09-4 (racine portable), 09-5 (défauts d'ombre de premier lancement « non reproduits (D) »), 09-6 (REQ-PERS-066 en DOIT), 09-7 (domaine [0 ; 248]), 09-8 (renvoi normatif à REQ-PROF-041), 09-9 (au moins un cœur, exactement un en v1) : tous **RESOLVED**.
- Vecteurs §5.1, §5.2, §5.4, §5.5 et la réduction de jaquette de REQ-PERS-036 (1024×1024 → 192×192) recalculés : conformes.
- Deux incohérences subsistent, toutes deux introduites ou laissées par l'amendement : **N-2** (périmètre de `retro_serialize`) et **N-3** (réglages P exposés en core options).
- UNCLEAR mineur : §4.2 liste `SceneScale` en « L, É » alors que §4.1.10 lui donne la portée **D**, définie au §2 comme « lecture seule » ; REQ-PERS-013 pt 3 la rend bien assignable (l'écriture se reporte sur `SceneRatio`). Une demi-phrase dans §2 ou §4.1.10 lèverait l'ambiguïté avec REQ-PERS-007 (« les membres marqués “lecture” NE DOIVENT PAS être assignables »).

---

## 3. Passages neufs ou réécrits : cohérence croisée

Vérifiés : SPEC-04B REQ-E3D-410, SPEC-08 REQ-INP-063 et §5.3, SPEC-12 §2 et REQ-ACC-050, DECISIONS-R0 « Arbitrages après la relecture R2 » (DEC-R2-T1…T9, D4, D5, D7, 12-2, 521), DEC-07-Q2 et DEC-08-4 mis à jour.

**Repère de scène, indices de `Clipping`, cadence 60/50, périmètre du savestate : cohérents** (voir §1, items 13, 14, 15, 17). Les décisions DEC-R2-\* sont toutes répercutées et citées à l'endroit où elles s'appliquent ; DEC-07-Q2 porte désormais ×1,5 avec la justification par la formule de REQ-LOOP-015. **Aucune fuite dans ces passages** : REQ-E3D-410 n'énonce que des relations géométriques et des constantes de cadrage observables.

SPEC-12 : les quatre catégories (auto / [ROM] / [R2] / revue) sont définies au §2, les étiquettes sont posées dans les titres d'exigences (je les ai retrouvées sur REQ-CORE-033/039/062, REQ-E3D-233/252/322/426/566, REQ-SCN-030/044/046/050/051, REQ-INP-060/062/064/066, REQ-LUA-088, REQ-ACC-020/021/031), et REQ-ACC-050 rend la porte P4 atteignable (verdicts « en attente de ROM » et « vérifiable au round 2 » comptés comme justifiés, reportés dans la matrice de traçabilité). **T-7, T-8 et 12-1 sont levés.**

### N-1 (INCONSISTENT, bloquant) — Visée du Zapper : la formule normative contredit tous ses propres vecteurs

`DEC-08-4` a été révisée et tranche explicitement : la visée vise **le pixel NES affiché sous le pointeur**, `py = tronc(248 − y)` dans le repère de REQ-E3D-410, le décalage de 8 lignes de la référence n'étant pas reproduit **(D)** ; la décision « prime sur la formule fidèle de REQ-INP-063 et clôt SPEC-08 Q-9 ». Le corps de REQ-INP-063 a bien été mis à jour (`py = tronc(248 − (y − Oy) / e)`, écart (D) annoncé). **Mais cinq passages ont gardé la relation fidèle `py = tronc(240 − y)` :**

1. le **critère d'acceptation de REQ-INP-063** lui-même : « pointeur sur `C = (128, 120, 0)` → (128, 120) ; sur le centre de l'image NES `(128, 128)` → (128, 112) ; sur le coin haut-gauche de l'image `(0, 248)` → (0, −8) » ;
2. les **six lignes du tableau §5.3** de SPEC-08, qui reprennent les mêmes valeurs ;
3. **SPEC-08 §6 Q-9**, toujours ouverte, qui présente la relation fidèle comme celle qui est spécifiée et la relation `248` comme une divergence que R0 « peut décider » ;
4. la ligne **Historique** de SPEC-08 (« relation vérifiée dans l'original (py = tronc(240 − (y − Oy)), décalage de 8 lignes) et vecteurs §5.3 recalculés ») ;
5. **SPEC-04B REQ-E3D-410 point 5** : « La visée du Zapper n'utilise **pas** cette relation mais celle de SPEC-08 REQ-INP-063 (fidèle à l'original, décalée de 8 lignes) » — or REQ-INP-063 utilise désormais exactement la relation inverse du point 5.

Les six vecteurs sont donc tous décalés de **exactement 8 lignes** par rapport à la formule qui les précède. Un implémenteur qui part des critères code la relation fidèle ; un implémenteur qui part de la formule et de `DEC-08-4` code l'autre. Dans Duck Hunt, 8 lignes font la différence entre toucher un canard et le manquer : l'écart est pleinement observable.

Correction (mécanique, aucune analyse nouvelle) : recalculer les vecteurs avec `py = tronc(248 − (y − Oy)/e)` — `C` → (128, 128) ; centre de l'image → (128, 120) ; `(0, 248)` → (0, 0) ; coin haut-gauche de `R` avec les marges (0 ; 7,9 ; 0 ; 0,1), point `(0 ; 239,9)` → (0, 8) ; `C` avec `ZapperOffset = (10, 0, 0)` → (118, 128) — ; réécrire REQ-E3D-410 point 5 (la visée **utilise** la relation inverse du point 5, le décalage de 8 lignes de la référence n'étant pas reproduit (D)) ; marquer Q-9 « close par DEC-08-4 (D) » ; corriger la ligne Historique.

### N-2 (INCONSISTENT, bloquant) — Périmètre de `retro_serialize` : parties 3 à 5 contre parties 3 à 6

`DEC-R2-T5` inclut l'état appris de la conversion dans le savestate **précisément pour garantir la sérialisation libretro** ; SPEC-07 REQ-LOOP-063 exige « le même périmètre qu'un savestate (REQ-LOOP-040) » ; REQ-PERS-056 borne la taille « des parties 3 à 6 » en invoquant l'exigence libretro. Mais :

- **SPEC-09 REQ-PERS-086** : « la sérialisation (`retro_serialize`) DOIT contenir les parties **3 à 5** de REQ-PERS-056 précédées d'une signature et d'une version » — ce qui exclut la partie 6 ;
- **SPEC-09 §4.5.4** décrit le fichier comme « les sections “cœur”, “Lua” (64 réels double précision), “caméra”, chacune précédée de sa longueur », sans section pour l'état appris, et ajoute que « toute section de longueur incohérente rend le fichier invalide ».

En l'état, la sérialisation libretro perd l'état appris, donc le rejeu à l'identique que `DEC-R2-T5` cherchait à garantir (REQ-ARCH-012/013, REQ-ACC-040, REQ-LOOP-062) — exactement le défaut T-5 d'origine, déplacé dans le frontend libretro. Correction : « parties 3 à 6 » dans REQ-PERS-086, et une quatrième section dans §4.5.4.

### N-3 (INCONSISTENT, bloquant, portée limitée) — Réglages de portée P exposés en core options

REQ-PERS-085 : « Les réglages P et S NE DOIVENT PAS être exposés comme core options ». Le dernier paragraphe de §4.6 le répète (« Non exposés en libretro : … tous les réglages P, S, D, VR, éditeur et film »). Or :

- `video.graphic_quality` est de portée **P** (DEC-09-Q1) et sa colonne « libretro » du §4.1.1 vaut « Low/Medium/High » ; le tableau §4.6 liste `3dsen_video_graphic_quality` ;
- `emu.tv_mode` est de portée **P** (Q-1, DEC-09-Q1) et sa colonne « libretro » vaut « NTSC/PAL/DENDY » ; le tableau §4.6 liste `3dsen_emu_tv_mode`.

Le tableau de §4.6 contredit donc le paragraphe qui le suit immédiatement, et REQ-PERS-085. Un implémenteur doit deviner si la table d'options générée (REQ-PERS-083, « génération automatique depuis le modèle ») doit contenir ces deux entrées. Correction : soit exclure les deux lignes, soit inscrire dans REQ-PERS-085 l'exception nommée (« à l'exception de `video.graphic_quality` et `emu.tv_mode`, exposés parce que l'hôte libretro n'a pas d'autre moyen de les choisir ») et corriger la phrase de §4.6 en conséquence.

---

## 4. Contrôle par échantillon de 20 constats non bloquants d'AMD-001

Tirés dans neuf documents. Tous **RESOLVED** ; aucun n'a introduit de fuite.

| Constat | Vérification faite | Verdict |
|---|---|---|
| 00-1 | SPEC-00 : `shape2D` = shape, `shape` = instance de frame | RESOLVED |
| 00-2 | SPEC-00 : types de géométrie 0 `Default` … 5 `HALFVCYLINDER` | RESOLVED |
| 02-5 | REQ-CORE-037 : « règle de l'original, sans motif fonctionnel connu … change le contenu des lots (V6) » | RESOLVED |
| 02-6 | REQ-CORE-044 : départage par numéro OAM à X égal, tri non stable de la référence classé (D) | RESOLVED |
| 03-3 | SPEC-03 §2 : noms de messages déclarés de commodité, sans portée normative | RESOLVED |
| 03-4 | REQ-PROF-003 : reformulée en règle de lecture, critère sur `empty.bytes` | RESOLVED |
| 04-5 | REQ-E3D-002 : `(g, b, d, h)` ↔ `x, y, z, w` ↔ indices 0–3, avec note sur l'écart de 8 lignes avec `R` | RESOLVED |
| 04-8 | REQ-E3D-381 : résultat normatif = bande retenue ; sort des tiles libérées « non spécifié » | RESOLVED |
| 04-9 | REQ-E3D-361 : budget = durée d'une frame de rendu cible, aligné sur REQ-E3D-600 et REQ-LOOP-061 | RESOLVED |
| 04-11 | REQ-E3D-007 : « temps constant amorti » déplacé en recommandation dans les notes de REQ-E3D-360 | RESOLVED |
| 04B-9 | REQ-E3D-455 : `fillrate` cité explicitement, « seul ce hook permet de le fixer avant la construction » | RESOLVED |
| 04B-10 | REQ-E3D-442 : recherche dans `[y, y + k]`, clause d'arrêt (« cet entier existe toujours ») | RESOLVED |
| 04B-11 | REQ-E3D-443 : boîte sur `Wp × Hp` cases vides comprises, ouverte à l'endroit des cases vides (REQ-E3D-421) | RESOLVED |
| 04B-12 | REQ-E3D-568 : `OriPos − Pd = (−q.x, h₀ − q.y, 0)` documenté, déclaré fidèle, conséquence chiffrée (« à un pixel près ») | RESOLVED |
| 05-5 | REQ-LUA-004 : garantie normative restreinte à l'ASCII imprimable, collation Unicode recommandée au-delà | RESOLVED |
| 05-7 | SPEC-05 §4.2 : « pattern de frame » défini (grille `TSize`, instances, durée de vie, `layer`/`geo`/`sizeZ` de l'instance 0) | RESOLVED |
| 06-9 | REQ-SCN-054 : les six textures et la cubemap déclarées présentes dans `assets-neutral/` | RESOLVED |
| 08-6 | REQ-INP-100 : horloge d'affichage, explicitement hors périmètre de REQ-ARCH-012 | RESOLVED |
| 10-9 | REQ-UI-002 : clause de fermeture assouplie par renvoi à REQ-UI-007, 035, 095 | RESOLVED |
| 11-3 | REQ-AUD-020 et 022 : fondus de 200/60 s, indépendants de la cadence, critères discriminants 60 / 120 i/s | RESOLVED |

---

## 5. Constats non bloquants supplémentaires (à corriger quand c'est pratique)

1. **UNCLEAR** — Collision de notation sur `s` : SPEC-02 REQ-CORE-072 appelle `s` le **facteur** de vitesse (son critère « s = 2 » signifie k = 2), SPEC-07 REQ-LOOP-015 appelle `s` le **réglage** entier de `−8` à `+8` (où s = 2 donne k = 1,5). REQ-LOOP-016 jongle correctement avec les deux (« le réglage `s = 2`, soit `k = 1,5` »), mais la lecture croisée est piégeuse. Renommer le facteur de SPEC-02 en `k`.
2. **INCONSISTENT mineur** — `DECISIONS-R0.md` : `DEC-06-Q3` (« carte d'ombre de 512 texels par défaut ») est laissée telle quelle, alors que `DEC-R2-D4` déclare la remplacer. `DEC-03-Q02` a reçu un barré dans la même situation ; appliquer le même traitement.
3. **INCONSISTENT mineur (document de suivi)** — `AMD-001` §1 et §4 point 1 décrivent encore la visée du Zapper comme « fidèle », avec Q-9 ouverte pour R0, ce que `DEC-08-4` a tranché depuis. AMD-001 n'est pas normatif, mais la table des constats sert de point d'entrée : y signaler la révision.
4. Les deux retouches de forme de SPEC-06 et l'UNCLEAR `SceneScale` de SPEC-09, déjà décrits au §2.

---

## 6. Verdicts par document (v1.1-draft)

| Document | Verdict | Motif |
|---|---|---|
| SPEC-00 | **ACCEPT** | 00-1 et 00-2 corrigés |
| SPEC-01 | **ACCEPT** | T-5 et 01-2/03 corrigés |
| SPEC-02 | **ACCEPT** | LEAK 02-1 levée, T-3 appliquée ; réserve de notation sur `s` (§5.1) |
| SPEC-03 | **ACCEPT** | 03-1 à 03-5 corrigés |
| SPEC-04 | **ACCEPT** | 3 LEAK levées, 04-4 et 04-5 corrigés |
| SPEC-04B | **ACCEPT WITH FIXES** | 5 LEAK levées, T-4 et point 25 résolus ; **N-1** (REQ-E3D-410 point 5) |
| SPEC-05 | **ACCEPT** | T-6, T-7, 05-2 à 05-7 corrigés |
| SPEC-06 | **ACCEPT** | *(était REJECT)* repère unique, vecteurs recalculés exacts, aucune fuite ; 2 retouches de forme |
| SPEC-07 | **ACCEPT** | LEAK 07-1 levée, T-3/T-5/T-6, 07-5 (×1,5), 07-6 (DEC-R2-D5), 07-7 |
| SPEC-08 | **ACCEPT WITH FIXES** | 08-1 à 08-7 corrigés ; **N-1** (critère de REQ-INP-063, §5.3, Q-9, Historique) |
| SPEC-09 | **ACCEPT WITH FIXES** | *(était REJECT)* T-1 et T-5 levés dans le corps ; **N-2** et **N-3** |
| SPEC-10 | **ACCEPT** | LEAK 10-1 levée, 10-2 à 10-10 corrigés |
| SPEC-11 | **ACCEPT** | LEAK 11-1 levée, T-3, 11-3 à 11-5 corrigés |
| SPEC-12 | **ACCEPT** | catégories de vérification définies, REQ-ACC-050 atteignable, 12-2 à 12-4 corrigés |
| DECISIONS-R0 | **ACCEPT** | D-1 à D-7 tranchés ; réserve de forme sur `DEC-06-Q3` (§5.2) |

## 7. Comptes

| Catégorie | v1.0 | v1.1 restants |
|---|---|---|
| LEAK | 12 | **0** |
| INCONSISTENT bloquants | 12 (+ 1 point de statut) | **3** (N-1, N-2, N-3) |
| UNTESTABLE systémiques | 2 (T-7, T-8) | 0 |
| UNCLEAR / forme non bloquants | 36 | 7 (§2 et §5) |

---

## 8. Verdict final

**PAS ENCORE PRÊT POUR LE GEL — trois corrections obligatoires, toutes mécaniques.**

Les 12 fuites et 12 des 13 incohérences bloquantes de la v1.0 sont effectivement corrigées, SPEC-06 et SPEC-09 passent du rejet à l'acceptation, et les deux problèmes systémiques de testabilité sont réglés par les catégories de SPEC-12. Il ne reste **aucune fuite**. Ce qui bloque encore le gel, ce sont trois contradictions qui obligeraient un implémenteur à deviner, et dont la correction ne demande aucune analyse nouvelle :

1. **N-1** — Visée du Zapper : recalculer les 6 vecteurs de REQ-INP-063 et de SPEC-08 §5.3 avec `py = tronc(248 − (y − Oy)/e)`, réécrire REQ-E3D-410 point 5, clore Q-9 par `DEC-08-4`, corriger l'Historique de SPEC-08.
2. **N-2** — REQ-PERS-086 : « parties 3 à **6** » ; ajouter la section de l'état appris dans SPEC-09 §4.5.4.
3. **N-3** — Trancher l'exposition libretro de `video.graphic_quality` et `emu.tv_mode` (exception nommée dans REQ-PERS-085, ou retrait des deux lignes de §4.6 et de la colonne « libretro » du §4.1).

Ces trois corrections appliquées, et sans qu'une nouvelle relecture complète soit nécessaire, la spec est **PRÊTE À GELER en v1.1**. Les constats du §5 et les retouches de forme du §2 peuvent être repris plus tard sans bloquer l'implémentation.

---

*Rapport produit à partir de la seule spécification, de `PLAN.md` et de `assets-neutral/INVENTORY.md`. Aucun fichier de spec n'a été modifié.*
