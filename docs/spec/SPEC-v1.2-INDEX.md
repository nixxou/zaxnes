# Index de la spécification v1.2 — où trouver les amendements d'une exigence

Version : 1.2 | 2026-10-21 | **Documentation seulement** : ce fichier n'ajoute, ne retire et ne
modifie **aucune** exigence. Il dit, pour chaque document et chaque exigence, quelles décisions
R0 et quels amendements `AMD-NNN` s'y appliquent, et dans quel ordre ils se lisent.

Rédigé pendant la phase P6 (clôture) du `PLAN.md`. Les fichiers `SPEC-*.md` sont **gelés** en
v1.1 et ne sont pas touchés.

---

## 1. Ce qui est normatif, et dans quel ordre

Le corpus normatif se lit dans cet ordre, du plus fort au plus faible :

| Rang | Source | Nature |
|---|---|---|
| 1 | `changes/AMD-002` … `changes/AMD-023` | amendements postérieurs au gel ; **le plus récent l'emporte** sur un amendement antérieur qui traite le même point (voir §4) |
| 2 | `DECISIONS-R0.md` | décisions R0, gelées avec la v1.1 ; primes sur le texte d'une SPEC en cas de conflit |
| 3 | `SPEC-00` … `SPEC-12`, version **1.1** | le texte des exigences |

`changes/AMD-001` est d'une autre nature : c'est le **compte rendu d'application** de la
relecture R2 au texte des SPEC. Ses 117 lignes ont été écrites *dans* les fichiers `SPEC-*.md`
pour produire la v1.1 ; il ne se superpose donc pas à la v1.1, il l'explique. On le consulte pour
savoir **pourquoi** une exigence dit ce qu'elle dit, pas pour savoir ce qu'elle dit.

`changes/Q-NNN.md` sont des **tickets de question** ouverts par les implémenteurs ou par le
vérificateur. Un ticket n'est pas normatif ; c'est la réponse R0 qui l'est, et elle vit dans un
`AMD-NNN` (ou, pour `Q-062`, dans `AMD-023` et dans le ticket lui-même).

`changes/R2-review-*.md` sont des rapports de relecture : informatifs.

---

## 2. Table des amendements

| Fichier | Date | Objet | Exigences touchées (résumé) |
|---|---|---|---|
| `AMD-001-r2-fixes.md` | 2026-10-20 | application de la relecture R2 au texte v1.1 (compte rendu) | les 14 documents ; voir §3 |
| `AMD-002-q-tickets-l1a.md` | 2026-10-20 | Q-001 à Q-003 | REQ-CORE-011, -015, -016, -091 ; **crée REQ-LOOP-018** |
| `AMD-003-q-tickets-l1b.md` | 2026-10-20 | Q-004, Q-005 | REQ-CORE-037, -038, -062 |
| `AMD-004-q-tickets-l1c.md` | 2026-10-20 | Q-006 | REQ-CORE-074, REQ-AUD-012 |
| `AMD-005-q-tickets-l1d.md` | 2026-10-20 | Q-007 (MMC5) | REQ-CORE-037, -039, -041, -100 |
| `AMD-006-q-tickets-l2.md` | 2026-10-20 | Q-008 à Q-010 | REQ-PROF-051, -062, -064, REQ-PERS-072 |
| `AMD-007-q-tickets-l4a.md` | 2026-10-20 | Q-011, Q-012 | REQ-E3D-060, -065, -005/-006/-041 |
| `AMD-008-q-tickets-l4b.md` | 2026-10-20 | Q-013 à Q-018 | REQ-E3D-407, -446/-447, -450, -452, -522, -562 |
| `AMD-009-q-tickets-l7a.md` | 2026-10-21 | Q-019 à Q-022 | REQ-SCN-028, -030, -040, REQ-E3D-381 |
| `AMD-010-ui-classique.md` | 2026-10-21 | interface d'émulateur classique | **crée REQ-UI-200 à REQ-UI-209** ; rend la présentation de SPEC-10 informative (P3) |
| `AMD-011-q-tickets-l7b.md` | 2026-10-21 | Q-023 à Q-026 | REQ-SCN-031, -051, -053/-054, SPEC-06 §5.1 |
| `AMD-012-transparence-par-objet.md` | 2026-10-21 | transparence par objet, jointures | REQ-SCN-040 (révise AMD-009) ; **crée REQ-SCN-065** |
| `AMD-013-q-tickets-l6.md` | 2026-10-21 | Q-027 à Q-031 | REQ-LOOP-017, -023, -042/-043, REQ-UI-204, -207, REQ-PERS-051 |
| `AMD-014-q-tickets-l7c.md` | 2026-10-21 | Q-032 | REQ-SCN-031, REQ-SCN-065 |
| `AMD-015-q-tickets-l8.md` | 2026-10-21 | Q-033 | REQ-INP-042, -045, -047 |
| `AMD-016-q-tickets-l3.md` | 2026-10-21 | Q-034 à Q-036 | REQ-PERS-080, SPEC-09 §4.1.1/§4.6, REQ-UI-200/-201 |
| `AMD-017-q-tickets-l9.md` | 2026-10-21 | Q-037, Q-038 | REQ-UI-200 (Vidéo) |
| `AMD-018-q-tickets-p4a.md` | 2026-10-21 | Q-039 à Q-043 | REQ-CORE-073, -112, -113, REQ-E3D-360, -381, REQ-ACC-011, -030, -031 |
| `AMD-019-q-tickets-p4b.md` | 2026-10-21 | Q-044 à Q-047 | REQ-AUD-004/-007, REQ-CORE-070/-071, REQ-SCN-008, -035, -051, -064, REQ-LUA-110/-111 |
| `AMD-020-q-tickets-p4c.md` | 2026-10-21 | Q-048 à Q-053 | REQ-INP-030/-031 (**remplacé par AMD-021**), -106, -123, REQ-PERS-036/-074, REQ-UI-036/-201/-204, SPEC-09 §4.1.10 |
| `AMD-021-round2.md` | 2026-10-21 | amendements issus du round 2 | REQ-CORE-022, -037, -062, -100, REQ-E3D-091, REQ-INP-030/-031, REQ-SCN-028, -032, -040, -054, REQ-AUD-041 |
| `AMD-022-q-tickets-r2fixes.md` | 2026-10-21 | Q-058, Q-059.1 à .5 | précise AMD-021 : R2-001, R2-005, R2-006, R2-008, R2-014 ; tests d'acceptation de SPEC-12 |
| `AMD-023-q-tickets-p5r.md` | 2026-10-21 | Q-060 à Q-062 | REQ-ACC-020, -021, -030, -041, REQ-CORE-115 |

---

## 3. Index par exigence

Lecture d'une ligne : l'exigence, puis **toutes** les sources qui la modifient, de la plus
ancienne à la plus récente. `(v1.1)` signale un point déjà écrit dans le texte gelé par AMD-001 :
on n'a rien à superposer, la ligne n'est là que pour le chemin de provenance. `(D)` est une
divergence volontaire assumée ; `→` marque un remplacement (voir §4).

### SPEC-00 — Glossaire

| Exigence / section | Amendements |
|---|---|
| glossaire (shape / instance de frame) | AMD-001 00-1 (v1.1) |
| valeurs numériques des couches | AMD-001 00-2 (v1.1) |

### SPEC-01 — Architecture

| Exigence | Amendements |
|---|---|
| REQ-ARCH-012 | AMD-001 03-5 (v1.1 : l'aléa d'un profil est hors périmètre) ; DEC-R2-T5 |
| REQ-ARCH-013 | **DEC-R2-T5 (D)** : l'état appris de la conversion fait partie du savestate ; AMD-001 T-5, 01-1, 01-3 (v1.1 : borne valable pour la session) ; AMD-001 §4.3 : une capacité fixe réservée est permise, sa valeur n'est pas fixée |
| REQ-ARCH-033 | AMD-001 01-2, 09-4 (v1.1 : racine portable `userdata\`, REQ-PERS-071 fait foi) |

### SPEC-02 — Cœur NES

| Exigence | Amendements |
|---|---|
| REQ-CORE-002 | AMD-001 02-4 (v1.1 : matrice opération → test) |
| REQ-CORE-004 | DEC-02-Q3 : reset = remise sous tension ; RAM pile conservée |
| REQ-CORE-011 | **AMD-002 Q-001** : cause typée + message technique, seules les chaînes en police de code sont littérales ; **AMD-002 Q-002** : instance laissée sans ROM après échec |
| REQ-CORE-013 | DEC-02-Q2 (D) : règle publique |
| REQ-CORE-015 | **AMD-002 Q-003** : la tolérance de durée s'applique dès la 2ᵉ frame après un changement de région |
| REQ-CORE-016 | **AMD-002 Q-003** (même règle) ; DEC-02-Q2 |
| REQ-CORE-022 | AMD-021 §2 **R2-007 (D)** : au-delà de la 8ᵉ lecture, la valeur de la reconstruction est conservée |
| REQ-CORE-033 | DEC-R2-T8 : étiquette [ROM] (Punch-Out!!) ; AMD-023 Q-060 : observables automatiques |
| REQ-CORE-037 | AMD-003 Q-004 (scroll du bord gauche, tolérance d'une ligne) → **AMD-021 R2-005** : la rangée reflète l'état de **fin** de ligne, la tolérance d'une ligne ne s'applique plus à l'instant de capture ; AMD-005 Q-007 (MMC5 : table CHR unique) ; AMD-022 Q-059.1 : critère 2 précisé ; AMD-001 02-5 (v1.1 : justification) |
| REQ-CORE-038 | AMD-003 Q-004 ; AMD-021 R2-005 (même remplacement que -037) |
| REQ-CORE-039 | AMD-005 Q-007 (MMC5) ; AMD-001 02-3 (v1.1 : vecteur synthétique MMC2, part Punch-Out!! en [ROM]) |
| REQ-CORE-040 | AMD-007 Q-012 (recouvrement de cellule : le dernier du lot l'emporte) |
| REQ-CORE-041 | AMD-005 Q-007 (MMC5, 8×16 compris) |
| REQ-CORE-044 | AMD-001 02-6 (v1.1, **D**) : départage par numéro OAM, l'original trie sans stabilité |
| REQ-CORE-046 | DEC-04-Q03 : les valeurs du masque relèvent de SPEC-02 |
| REQ-CORE-061 | DEC-02-Q1 : bit 4 = 1 gâchette relâchée ; AMD-001 02-7 (v1.1 : polarité écrite dans SPEC-08 §4.1) |
| REQ-CORE-062 | AMD-003 Q-005 (points 1 et 3) → **AMD-021 R2-006** : pendant le vblank et la ligne de pré-rendu, seule la couleur d'arrière-plan universel est vue ; x ≥ 239 vu avec cette couleur, **exigé** ; AMD-021 §2 R2-006 autres points **(D)** : pas de cache par bande de 8 lignes ; AMD-022 Q-059.5 : la ligne de post-rendu est hors du balayage visible |
| REQ-CORE-070 | AMD-001 02-2 (v1.1 : repliement ≤ −60 dBFS) ; AMD-019 Q-044 : moyenne sur ≥ 600 frames ; AMD-001 11-5 : ajustement fin ±0,5 % |
| REQ-CORE-071 | AMD-019 Q-044 : la lecture reste tout ou rien, la moyenne est indépendante du tampon |
| REQ-CORE-072 | AMD-001 07-5 (v1.1) : avance rapide **×1,5** et non ×2 ; DEC-07-Q2 (libellé corrigé) |
| REQ-CORE-073 | **AMD-018 Q-039** : crête à crête mesurée à 440 Hz sur les plateaux |
| REQ-CORE-074 | **AMD-004 Q-006** : seule la file d'échantillons en attente est vidée au chargement d'un état |
| REQ-CORE-081 | AMD-001 §3 (borderline, inchangé) |
| REQ-CORE-100 | AMD-005 Q-007 (MMC5) ; **AMD-021 R2-001** (mapper 28), **R2-002** (15), **R2-003** (101) ; AMD-021 §2 R2-004 **(D)** (mapper 27 / VRC2-VRC4) ; AMD-022 Q-059.3 : numéro de banque hors image ramené modulo le nombre de banques (**toutes** les cartes) ; AMD-021 §4.2 : critère de R2-003 |
| REQ-CORE-112 | **AMD-018 Q-040** : protocoles de résultat des ROMs de test |
| REQ-CORE-113 | **AMD-018 Q-040** (même protocole) |
| REQ-CORE-114, -115 | DEC-R2-T8 : étiquettes ; **AMD-023 Q-062** : la clause « sans plantage » de -115 est instruite comme un défaut (voir §6) |
| REQ-CORE-120 | AMD-001 02-1 (v1.1 : phrase de fuite supprimée) |
| audio d'extension | DEC-02-Q5 : non requis (P3) |

### SPEC-03 — Format de profil

| Exigence | Amendements |
|---|---|
| REQ-PROF-001 | DEC-R2-T7 ; AMD-001 T-7 (v1.1 : critères exécutables sur `assets-neutral/profiles`) |
| REQ-PROF-003 | AMD-001 03-4 (v1.1 : reformulé en règle de lecture) |
| REQ-PROF-024 | **DEC-03-Q02** : tri des scripts par comparaison **culturelle invariante** (la proposition ordinale est explicitement rejetée) ; AMD-001 03-2 (v1.1 : renvoi à REQ-LUA-004) |
| REQ-PROF-025 | **DEC-03-Q03 (D)** : profil sans listes 14/15 → 5 emplacements reconstitués ; AMD-001 03-1 (v1.1) |
| REQ-PROF-041 | AMD-001 09-8 (v1.1 : REQ-PERS-005 y renvoie) |
| REQ-PROF-051 | **AMD-006 Q-010** : présence des scalaires, identité octet par octet prioritaire ; DEC-R2-T7 ; AMD-001 T-7 |
| REQ-PROF-052 | AMD-001 03-5 (v1.1 : aléa hors REQ-ARCH-012) |
| REQ-PROF-062 | **AMD-006 Q-008** : profils utilisateur dans `profiles/`, import `CustomProfiles` optionnel |
| REQ-PROF-064 | **AMD-006 Q-009** : association ROM ↔ jeu dans `library.json` ; DEC-R2-T8 ; AMD-001 T-8 (rendu exécutable sans ROM commerciale) |
| `TVMode` / région | **DEC-03-Q01** puis **AMD-013 Q-031** : région automatique active → la région de la ROM l'emporte sur le `TVMode` du profil **(D)** ; inactive → ordre fidèle |
| édition du catalogue | DEC-03-Q04 : seule l'édition `full` est utilisée |

### SPEC-04 — Conversion

| Exigence | Amendements |
|---|---|
| REQ-E3D-002 | **DEC-R2-T1** : `Clipping` 0 = gauche, 1 = basse, 2 = droite, 3 = haute ; AMD-001 T-1, 04-5 (v1.1 : correspondance (g,b,d,h) ↔ x,y,z,w ↔ 0–3) |
| REQ-E3D-005, -006, -041 | **AMD-007 Q-012** : deux enregistrements de fond du même lot dans la même cellule → le dernier l'emporte |
| REQ-E3D-007 | AMD-001 04-11 (v1.1 : « temps constant amorti » retiré de l'exigence) |
| REQ-E3D-008 | **DEC-R2-T5** : l'état appris est sérialisé ; AMD-001 T-5 |
| REQ-E3D-021 | AMD-001 04-6 (v1.1 : définition unique dans SPEC-04, SPEC-04B REQ-E3D-454 y renvoie) |
| REQ-E3D-060 | **AMD-007 Q-011** : au chargement, patterns enregistrés tels quels, doublon ignoré (le premier reste) ; DEC-R2-T7 ; AMD-001 T-7 |
| REQ-E3D-065 | **AMD-007 Q-011** : les règles de remplacement ne concernent que l'apprentissage en partie |
| REQ-E3D-091 | DEC-04-Q02 (ordre d'inscription pur) → **AMD-021 R2-008** : après retrait, le nouveau pattern prend la place du dernier retiré ; priorité de REQ-E3D-091 (et non P3) ; AMD-022 Q-059.4 : la place libérée est consommée par le premier pattern qui l'occupe ; AMD-021 §4.2 : critère |
| REQ-E3D-205 | AMD-001 04-4 (v1.1 : `Bmin`/`Bmax`, `BL`/`TR`) |
| REQ-E3D-207 | AMD-001 04-3 (v1.1 : `Age` vaut toujours 1) |
| REQ-E3D-233 | DEC-04-Q04/Q05 : les deux bizarreries conservées ; DEC-R2-T8 |
| REQ-E3D-252, -322 | DEC-R2-T8 : étiquettes [ROM]/[R2] |
| REQ-E3D-280 | AMD-001 04-1 (v1.1 : mention des trois tampons supprimée) |
| REQ-E3D-300 | AMD-001 04-7 (v1.1 : reformulé en résultat ; la règle de l'original est conservée) |
| REQ-E3D-342 | AMD-001 04-2 (v1.1 : conservé pour la compatibilité, aucun effet) |
| REQ-E3D-345 | AMD-001 04-10 (v1.1 : devient une note informative) |
| REQ-E3D-360 | **DEC-R2-12-2** : moyenne ≤ 4 ms **et** 99ᵉ centile ≤ 8 ms ; **AMD-018 Q-042** : protocole de mesure (profil `Final_Fantasy_USA`, trame pleine synthétique) ; DEC-04-Q06 : cible P2 |
| REQ-E3D-361 | AMD-001 04-9 (v1.1 : budget = durée d'une frame de rendu cible) |
| REQ-E3D-381 | **AMD-009 Q-022 (D)** : le mode 2D affiche l'image en pixels du cœur, -381 n'est plus utilisée pour ce rendu ; **AMD-018 Q-041** : critère remplacé par la règle de croissance de la passe 2 ; AMD-001 04-8 |
| `DetectShape` | DEC-04-Q01 : jamais appelé (fidèle) |
| table `NES_RGB` | DEC-04B-Q1 : portée par SPEC-04 ; AMD-001 04B-7, 06-6, D-6 |
| vocabulaire | DEC-04-Q07 : glossaire de SPEC-00 |

### SPEC-04B — Géométrie

| Exigence | Amendements |
|---|---|
| REQ-E3D-401, -405 | **DEC-R2-T4** et DEC-06-Q4 : le décalage ±0,1 des sprites est appliqué **par le rendu** ; AMD-001 T-4, 04B-6, D-1 (v1.1) |
| REQ-E3D-406 | voir REQ-E3D-410 ; **la citation de « REQ-E3D-512 » est une référence morte** (§5) |
| REQ-E3D-407 | **AMD-008 Q-018** : `_gaxis` reconnu comme élément exact de liste (41 objets) |
| REQ-E3D-410 | **exigence créée par AMD-001 T-2** (v1.1) sous **DEC-R2-T2** : repère de scène unique, image en y ∈ [8, 248], cadre de référence [0,256]×[0,240], centre C = (128, 120, 0) ; la règle « 240 → 248 » de la v1.0 est retirée |
| REQ-E3D-426 | DEC-R2-T8 ; AMD-001 04B-13 (v1.1 : critère propre au harnais) |
| REQ-E3D-442, -443 | AMD-001 04B-10, 04B-11 (v1.1) |
| REQ-E3D-446, -447, §5.3 | **AMD-008 Q-014** : les règles priment sur l'exemple, rectifié (colonnes 0 et 7) ; AMD-021 §3 : confirmé au round 2 |
| REQ-E3D-450 | **AMD-008 Q-015** : intervalle `[min(z⁻,z⁺), max(z⁻,z⁺)]`, bornes incluses |
| REQ-E3D-452 | **AMD-008 Q-013** : `D'` écrit dans `sizeZ` |
| REQ-E3D-454 | AMD-001 04-6 (v1.1 : renvoie à SPEC-04 REQ-E3D-021) |
| REQ-E3D-455 | AMD-001 04B-9 (v1.1 : `fillrate` fixé par le hook de construction) |
| REQ-E3D-504, -505 | **AMD-012** : jointures — crée **REQ-SCN-065** dans SPEC-06 |
| REQ-E3D-507 | AMD-001 04B-5 (v1.1) : **exigence retirée** |
| REQ-E3D-521, -522 | **DEC-R2-521** : la numérotation des coins devient normative ; **AMD-008 Q-016** : orientation par coins intérieurs et extérieurs (construction 2) ; AMD-001 point 25 ; AMD-021 §3 : confirmé |
| REQ-E3D-560 | AMD-001 04B-7, D-6 (v1.1 : table `NES_RGB` = SPEC-04 §4.1) |
| REQ-E3D-562 | **AMD-008 Q-017** : couleur de fond paramétrable, `NES_RGB[0x0F]` avant la première frame |
| REQ-E3D-564 | AMD-001 04B-3 (v1.1) |
| REQ-E3D-566 | DEC-R2-T8 ; AMD-001 04B-13 (v1.1) |
| REQ-E3D-567 | AMD-001 04B-8, 09-5 (v1.1, **D**) : état de premier lancement non reproduit |
| REQ-E3D-568 | **DEC-04B-Q2** : déformation dans le repère local, avant échelle et rotation (confirmé AMD-021 §3) ; AMD-001 04B-12 (v1.1 : écart `OriPos − Pd` documenté et voulu) |
| REQ-E3D-601, -603 | AMD-001 04B-2, 04B-1 (v1.1) |
| REQ-E3D-641, -643 | AMD-001 T-4, 04B-13 (v1.1) |
| textures hors bornes | DEC-04B-Q3 : répétition modulo dans l'emplacement |
| découpage par marges | DEC-04B-Q4 : dans le plan XY de la scène, avant la caméra |
| ombres par couche | DEC-04B-Q5 **(D)** : seul l'état stable est reproduit |
| couleur 0 héritée | DEC-04B-Q6 **(D)** : non obligatoire (P3) |

### SPEC-05 — Lua

| Exigence | Amendements |
|---|---|
| REQ-LUA-004 | DEC-03-Q02 (tri culturel invariant) ; DEC-R2-T7 ; AMD-001 05-5 (v1.1 : garantie normative restreinte aux noms ASCII) |
| REQ-LUA-028 | AMD-001 §3 (v1.1 : reformulation « objets réutilisés ») |
| REQ-LUA-051, -100, -101 | DEC-R2-T7 ; AMD-001 T-7, 05-3 (v1.1 : §5.1 marqué informatif, noms d'outils retirés) |
| REQ-LUA-072 | **DEC-R2-T6** : profil rechargé **avant** l'affectation de `PData` ; DEC-R2-T8 |
| REQ-LUA-074 | DEC-R2-T8 |
| REQ-LUA-075 | **DEC-R2-D5** : `Save`/`Load` appelés par `retro_serialize`/`retro_unserialize` ; AMD-001 07-6, D-5 |
| REQ-LUA-083 | AMD-001 04B-12, 05-6 (v1.1 : écarts `OriPos − Pd` et `Overlap` d'un pixel, voulus) |
| REQ-LUA-086, -088 | DEC-R2-T8 ; AMD-001 T-2 (v1.1 : repère de scène) |
| REQ-LUA-095 | AMD-001 05-2 (v1.1 : écrire `Video.SceneScale` réinitialise la vue) |
| REQ-LUA-110, -111 | DEC-R2-T8 ; **AMD-019 Q-047** : partie « auto » mesurée sur trame pleine synthétique (protocole de Q-042), partie jeux de référence étiquetée [ROM] ; AMD-001 05-4 (v1.1 : instrumentation définie) |
| SPEC-05 §4 (tables) | **AMD-020 Q-052** : `EngineMode`, `IsProfileMode`, `IsRomMode` **requis** en lecture seule ; DEC-05-Q6 : seuls les membres du §4 sont requis |
| SPEC-05 §4.2 `Pattern` | AMD-001 05-7 (v1.1 : « pattern de frame » défini) |
| dialecte et runtime | DEC-05-Q1 : Lua 5.4 (KeraLua) avec les contre-mesures de SPEC-05 |
| compteur `frame` | DEC-05-Q2 **(D)** : part de 0 au lancement du jeu |
| état Lua | DEC-05-Q3 **(D)** : recréé à chaque changement de profil ; DEC-05-Q5 : le reset conserve les drapeaux d'erreur |
| lecture de palette | DEC-05-Q4 : décalée d'une frame (fidèle) |

### SPEC-06 — Scène et rendu

| Exigence | Amendements |
|---|---|
| §2, REQ-SCN-001, -003, -012, -045, -048, §4.3, §5.1, §5.3 | **DEC-R2-T2** ; AMD-001 T-2, 06-1, D-2 (v1.1) : repère défini dans REQ-E3D-410, C = (128, 120, 0), R = [c0, 256−c2] × [c1, 240−c3] ; les vecteurs caméra et TV de la v1.0 sont **inchangés** |
| REQ-SCN-002 | AMD-001 05-2 (v1.1) |
| REQ-SCN-008 | **AMD-019 Q-046** : « cas dégénéré » = l'œil confondu avec un sommet (une des deux distances nulle) → le visé corrigé est le **milieu du segment** |
| REQ-SCN-011 | AMD-019 Q-046 (distance bornée à [0, 512]) |
| REQ-SCN-022 | **AMD-021 R2-010** : l'occultation FPV ne reçoit aucune ombre |
| REQ-SCN-028 | **AMD-009 Q-021** : normales transformées par la transposée de l'inverse ; DEC-06-Q2 (confirmé AMD-021 §3) ; AMD-021 §2 **R2-015 (D)** : linéariser séparément couleur et intensité est accepté (≤ 1 niveau) |
| REQ-SCN-030 | DEC-06-Q3 (512 texels) → **DEC-R2-D4** : **1024 texels** par défaut ; **AMD-009 Q-019** : boîte d'ombre = intersection des projeteurs et d'une sphère de 512 unités autour de l'œil ; DEC-R2-T8 ; AMD-001 06-3, 06-8, D-4 |
| REQ-SCN-031 | **AMD-011 Q-025** : carte alimentée par les faces arrière, l'absence d'acné prime sur le biais ; **AMD-014 Q-032** : biais `max(0,02 ; 1,5 × texel)` ; AMD-021 §3 : Q-025 et Q-032 confirmés |
| REQ-SCN-032 | **AMD-021 R2-010** : le « DEVRAIT » devient **« NE DOIT PAS »** — une tile transparente et une tile de HUD ne reçoivent aucune ombre |
| REQ-SCN-034 | AMD-001 04B-7, 06-6, D-6 (v1.1 : table `NES_RGB` = SPEC-04 §4.1) |
| REQ-SCN-035 | **AMD-019 Q-045** : la courbe sRGB de §2 fait foi (≈ 4,672), tolérance ±2 % |
| REQ-SCN-037 | AMD-001 T-4 (v1.1 : décalage ±0,1 appliqué ici) |
| REQ-SCN-040 | AMD-009 Q-020 (objet transparent = tile) → **AMD-012** : passage de profondeur et tri **par objet entier** ; **AMD-021 R2-009** : mélange en espace **linéaire** (HUD et occultation FPV compris) ; AMD-021 §2 **R2-012 (D)** : transparents derrière un transparent restent visibles |
| REQ-SCN-043 | AMD-001 06-5 (v1.1 : renvoi normatif à REQ-E3D-568) |
| REQ-SCN-044, -046, -050 | DEC-R2-T8 ; AMD-001 06-8 (v1.1 : critères synthétiques) |
| REQ-SCN-047 | **AMD-013 Q-029** : en pause la scène reste affichée, figée ; à l'arrêt, écran d'accueil (REQ-UI-207) |
| REQ-SCN-049, -051 | **AMD-011 Q-024** : teinte Fantasy, choix provisoire validé ; **AMD-019 Q-045** : « ×2 » n'était qu'une approximation, la courbe fait foi ; AMD-001 06-7 ; AMD-021 §3 : Q-024 confirmé (lecture A) |
| REQ-SCN-053, -054 | **AMD-011 Q-023** : matériaux TV, écarts décoratifs D ; **AMD-021 R2-014** : TV 1 et TV 3 opaques, premier sous-maillage seul, albédo TV 3 = texture × 0,208 + émission, touches de TV 3 projettent et reçoivent l'ombre, corps non ; spéculaire **informatif** ; **AMD-022 Q-059.2** : corps sans ombre, critère §4.2 corrigé ; AMD-001 06-9 |
| REQ-SCN-059 | AMD-001 06-2 (v1.1 : « continue d'être lissée ») |
| REQ-SCN-063 | AMD-001 06-10 (v1.1 : réglage conservé dans le modèle, absent de l'UI desktop) |
| REQ-SCN-064 | DEC-R2-T8 ; **AMD-019 Q-047** : étiquette **[ROM]** pour la mesure sur les jeux ; **AMD-023 Q-062** : le banc de mesure doit tourner sans exception |
| REQ-SCN-065 | **exigence créée par AMD-012** (P2 : aucune ligne visible à la jointure de deux tiles) ; **AMD-014 Q-032** : résidu de 1 à 2 px sur translucides obliques accepté (P3) |
| cubemaps | DEC-06-Q1 : 6 faces réextraites, conventions dans `cubemaps.json` |
| écritures Lua sur réglages | DEC-06-Q5 **(D)** et DEC-09-Q3 : couche de session non persistée |
| glisser à la souris | DEC-06-Q6 : SPEC-08 fixe le bouton ; le clic Zapper prime sur la caméra ; AMD-001 08-7 |
| mode 2D | **AMD-009 Q-022 (D)** : image en pixels du cœur plaquée sur le plan NES |

### SPEC-07 — Boucle

| Exigence | Amendements |
|---|---|
| §2, §5, REQ-LOOP-010, -015 | **DEC-R2-T3** : cadence nominale 60 (NTSC) / 50 (PAL, Dendy) images/s ; AMD-001 T-3, 07-4 (v1.1) |
| REQ-LOOP-003, -006 | **AMD-013 Q-027** (vignette d'un emplacement en cours de jeu) et **Q-029** (scène figée en pause) ; AMD-010 §1 : la présentation de SPEC-10 devient informative |
| REQ-LOOP-007 | AMD-001 §3 (borderline, inchangé) |
| REQ-LOOP-012 | AMD-001 07-7 (v1.1 : limite en frames émulées, période `1/f` au facteur `k`) |
| REQ-LOOP-016 | AMD-001 07-5 (v1.1) : avance rapide **×1,5**, demandes réarmées, reliquat 0–30 frames ; DEC-07-Q2 |
| REQ-LOOP-017 | **AMD-013 Q-031** : région automatique active → la ROM l'emporte sur le `TVMode` du profil **(D)** ; DEC-03-Q01 |
| **REQ-LOOP-018** | **exigence créée par AMD-002 Q-002** (P2) : l'ouverture d'une ROM depuis une partie en cours valide le fichier **avant** d'interrompre la partie |
| REQ-LOOP-023 | **AMD-013 Q-028** : `.zip` pris en charge, `.7z` refusé avec `Uncompression is not successful` ; AMD-020 Q-051 : le filtre ne propose pas `.7z` |
| REQ-LOOP-032, -033 | AMD-001 07-1, §3 (v1.1) |
| REQ-LOOP-040 | **DEC-R2-T5 (D)** : le savestate inclut l'état appris de la conversion ; AMD-001 T-5, 07-3 |
| REQ-LOOP-042, -043 | **AMD-013 Q-027** ; DEC-07-Q5 **(D)** : la quick save peut capturer une vignette |
| REQ-LOOP-044 | **DEC-R2-T6** : ordre de restauration de SPEC-05 (REQ-LUA-072) ; AMD-001 T-6, 05-1, 07-2 |
| REQ-LOOP-061 | AMD-001 04-9 (v1.1) |
| REQ-LOOP-063 | **DEC-R2-D5** : `retro_serialize`/`retro_unserialize` appellent `Save`/`Load` ; AMD-001 07-6, D-5 |
| vitesse | DEC-07-Q1 : −8..+8 (confirmé AMD-021 §3) |
| rollback | DEC-07-Q3 : cœur seul (fidèle), extension P3 |
| libretro | DEC-07-Q4 : savestate complet ; pas de run-ahead en v1 |

### SPEC-08 — Entrées

| Exigence | Amendements |
|---|---|
| REQ-INP-030, -031 | AMD-020 Q-048 (course brute) → **AMD-021 R2-011 remplace AMD-020** : le seuil 0,34 s'applique à la valeur **mise en forme** (zone morte radiale 0,2, saturation 0,9, ré-étalement linéaire) ; AMD-021 §4.1 : zone morte de **gâchette 0,2** (et non 0,1) ; Q-056 est sans objet ; DEC-08-8 |
| REQ-INP-041 | AMD-001 08-1 (v1.1) : **DOIT** (DEC-08-1, **D**) — une manette attribuée est retirée de l'autre joueur |
| REQ-INP-042, -045, -047 | **AMD-015 Q-033** : capture sur front montant, une commande déjà enfoncée est ignorée jusqu'à son relâchement ; DEC-08-2 |
| REQ-INP-060, -062, -064, -066 | DEC-R2-T8 ; AMD-001 08-2 (v1.1) : DEC-09-Q6 **(D)** — les types de port reviennent à Manette au chargement d'un jeu différent |
| REQ-INP-063 | **DEC-08-4 (D)** : la visée tient compte de l'échelle de scène et vise le **pixel affiché** (`py = tronc(248 − (y − Oy)/e)`) ; le décalage de 8 lignes de l'original n'est pas reproduit ; AMD-001 T-2, 08-5 |
| REQ-INP-100 | AMD-001 08-6 (v1.1 : horloge d'affichage, hors déterminisme) |
| REQ-INP-103 | AMD-001 07-5, 08-3 (v1.1) : avance rapide **×1,5** |
| REQ-INP-106 | **AMD-020 Q-049** : AMD-010 et AMD-013 priment — raccourcis de menu de REQ-UI-204, `Ctrl+Fn` pour charger un état ; les touches nues de §4.2 gardent leur rôle de jeu et de caméra |
| REQ-INP-123 | **AMD-020 Q-053** : critère recalculé sur la relation amendée de REQ-INP-063 ; AMD-001 T-2 |
| §4.1 (octet Zapper) | DEC-02-Q1 ; AMD-001 02-7, 08-4 (v1.1 : bit 4 = 1 relâchée) |
| molette | DEC-08-3 : 5 unités par cran |
| touches | DEC-08-5 : codes physiques ; SPEC-10 affiche le libellé de la disposition |
| bouton Menu émulé | DEC-08-6 et **DEC-R2-D7** : conservé dans le modèle, **aucun effet** ; AMD-001 D-7 |
| hotkeys en pause | DEC-08-7 (confirmé AMD-021 §3 : QuickLoad en pause relance le jeu) |

### SPEC-09 — Persistance

| Exigence | Amendements |
|---|---|
| §4.1.1 | **AMD-016 Q-035** : `video.res_scale` n'est pas exposé en core option (§4.6 l'emporte), il reste dans le modèle ; AMD-001 06-10, 10-10 |
| §4.1.5 | **DEC-R2-T1** et **DEC-09-Q2** : `clip.bottom` = `Clipping[1]`, `clip.top` = `Clipping[3]` ; AMD-001 T-1, 09-1, 09-7 (domaine des marges verticales [0 ; 248]), D-3 |
| §4.1.6 | AMD-001 04B-8, 09-5 (v1.1) |
| §4.1.10 | **AMD-020 Q-052** : `EngineMode`, `IsProfileMode`, `IsRomMode` requis en Lua avec cette sémantique |
| §4.4 | AMD-001 §3 (v1.1 : « journal d'exécution ») |
| §4.6 | **AMD-016 Q-035** |
| REQ-PERS-003 | AMD-001 09-9 (v1.1 : au moins un cœur, exactement un en v1) |
| REQ-PERS-005 | AMD-001 09-8 (v1.1 : renvoi normatif à REQ-PROF-041) |
| REQ-PERS-011 | AMD-001 05-2 (v1.1) |
| REQ-PERS-012 | AMD-001 08-2 (v1.1) : DEC-09-Q6 **(D)** |
| REQ-PERS-017 | **DEC-R2-D7** : aucun effet ; AMD-001 D-7 |
| REQ-PERS-036 | **AMD-020 Q-050 (D)** : jaquettes ramenées en **P3** |
| REQ-PERS-041 | AMD-001 10-5 (v1.1 : texte sans espace, SPEC-10 seule source) |
| REQ-PERS-051 | **AMD-013 Q-027** : vignette 256 × 192 |
| REQ-PERS-056 | **DEC-R2-T5 (D)** : l'état appris est dans le savestate ; AMD-001 T-5, 09-2 |
| REQ-PERS-057 | AMD-001 09-3, 10-6 (v1.1 : aligné sur REQ-UI-051) |
| REQ-PERS-066 | AMD-001 09-6, 10-7 (v1.1 : P2 DOIT) |
| REQ-PERS-071 | **DEC-09-Q7** : nom « Sen », `%LOCALAPPDATA%\Sen`, `portable.txt`, dossier system libretro `sen` ; AMD-001 01-2, 09-4 |
| REQ-PERS-072 | **AMD-006 Q-008** |
| REQ-PERS-074 | **AMD-020 Q-050 (D)** : P3 |
| REQ-PERS-080 | **AMD-016 Q-034** : grammaire des clés étendue aux crochets, `[a-z0-9_\[\]]+(\.[a-z0-9_\[\]]+)*` |
| `games.json` | **AMD-016 Q-036** : profil par fichier de ROM et ROMs récentes dans `games.json` (racine utilisateur) |
| empreinte ROM | DEC-09-Q4 : pas de contrôle au chargement, information journalisée |
| préréglages libretro | DEC-09-Q5 : huit couleurs de fond |

### SPEC-10 — UI

| Exigence | Amendements |
|---|---|
| **tout SPEC-10** | **AMD-010 §1 et §4** : les **fonctions** restent obligatoires, la **présentation et la navigation** (onglets, pagination, tuiles, navigation directionnelle, style) deviennent **informatives** et passent en **P3** ; la navigation des menus à la manette n'est pas requise |
| REQ-UI-002, -007, -010, -035, -095 | AMD-001 10-2 (DEC-10-Q6 **D** : onglet initial GAME, rendu sans objet par AMD-010 §4), 10-9 (clause de fermeture assouplie) |
| REQ-UI-011, -122 | AMD-001 10-1 (v1.1 : dimensions en proportions) |
| REQ-UI-036 | **AMD-020 Q-051** : filtre `.nes`, `.zip`, « Tous les fichiers » ; `.7z` non proposé |
| REQ-UI-039, -121, -122, -140 | AMD-001 10-3 (DEC-10-Q5 **D** : bouton « Site » retiré) |
| REQ-UI-051 | AMD-001 09-3 |
| REQ-UI-062 | AMD-001 06-10, 10-10 (v1.1 : `Graphic` conservé dans le modèle, absent de l'UI desktop) |
| REQ-UI-076 | AMD-001 10-4 (DEC-10-Q3 : bloc « à propos » = identité « Sen », GPL v3, crédits à 3dSen et à 3DNes_Engine) |
| REQ-UI-084 | AMD-001 10-5 (v1.1 : texte exact) |
| REQ-UI-120, -122 | AMD-001 10-8 (v1.1 : catégorie « revue » de SPEC-12, mesure de couleur) |
| **REQ-UI-200** | **créée par AMD-010** (P1, barre de menus) ; **AMD-016 Q-036** (`games.json`) ; **AMD-017 Q-037 (D)** : bascule 3D/2D à chaud sans relance ; **AMD-017 Q-038** : Mode caméra (TPV/FPV exclusifs) ; **AMD-013 Q-027** (menu « Sauvegarder l'état ») |
| **REQ-UI-201** | **créée par AMD-010** (P1, ouvrir une ROM et résolution du profil) ; **AMD-016 Q-036** ; **AMD-020 Q-051** (filtres) |
| **REQ-UI-202** | **créée par AMD-010** (P1, fenêtre d'options) ; AMD-021 **R2-013** pour les sons des réglages |
| **REQ-UI-203** | **créée par AMD-010** (P1, configuration des touches) ; AMD-015 Q-033 (règles de capture) |
| **REQ-UI-204** | **créée par AMD-010** (P1, raccourcis par défaut) ; **AMD-013 Q-030** : charger un état = **`Ctrl+Fn`**, sauvegarder = `Maj+Fn`, `Fn` seul reste aux touches caméra (AMD-010 corrigé) ; **AMD-020 Q-049** (priorité sur REQ-INP-106) ; AMD-013 Q-027 |
| **REQ-UI-205** | **créée par AMD-010** (P1, modèle de commandes indépendant de l'UI) |
| **REQ-UI-206** | **créée par AMD-010** (P1, notifications OSD avec les textes de SPEC-10) |
| **REQ-UI-207** | **créée par AMD-010** (P2, écran d'accueil) ; **AMD-013 Q-029** : écran d'accueil à l'arrêt |
| **REQ-UI-208** | **créée par AMD-010** (P2, 10 emplacements + état rapide **(D)**) |
| **REQ-UI-209** | **créée par AMD-010** (P1, plein écran : barre masquée, raccourcis actifs) |
| export OBJ | DEC-10-Q1 : exclu |
| plateforme | DEC-10-Q2 : Windows en v1, une seule méthode de rendu, pas de `Tex Scale` |
| navigation tuile par tuile | DEC-10-Q4 : P3 ; rendue sans objet par AMD-010 §4 |
| compteur FPS | DEC-10-Q7 : option, désactivée par défaut |

### SPEC-11 — Audio

| Exigence | Amendements |
|---|---|
| REQ-AUD-004 | **DEC-R2-T3** ; **AMD-019 Q-044** : moyenne sur ≥ 600 frames, indépendante du tampon ; AMD-001 11-4 (v1.1 : formule unique, renvoi à REQ-CORE-070/072) |
| REQ-AUD-005 | DEC-11-Q3 ; AMD-001 11-5 (v1.1 : ajustement fin ±0,5 %) |
| REQ-AUD-007 | **DEC-R2-T3** : 800 (NTSC) / 960 (PAL) échantillons par frame à 48 kHz ; **AMD-019 Q-044** ; AMD-001 T-3, 11-2 |
| REQ-AUD-011 | AMD-001 07-5 (v1.1) : avance rapide **×1,5** |
| REQ-AUD-012 | **AMD-004 Q-006** : seule la file en attente est vidée au chargement d'un état |
| REQ-AUD-020, -022 | **DEC-11-Q2 (D)** : fondu de durée fixe 200/60 s ≈ 3,33 s ; AMD-001 11-3 (v1.1 : critères discriminants) |
| REQ-AUD-041 | **AMD-021 R2-013** (P3, **remplace les notes et le tableau** et clôt DEC-11-Q5) : Vitesse et FOV ne déclenchent aucun son ; tous les autres réglages jouent le son de validation à chaque changement |
| reprise de la musique | DEC-11-Q1 : à la position de suspension (confirmé AMD-021 §3) |
| fréquence du périphérique | DEC-11-Q4 : adaptation acceptée |

### SPEC-12 — Acceptation

| Exigence | Amendements |
|---|---|
| §2 (catégories) | **DEC-R2-T7** (tout critère exécutable par le côté propre) et **DEC-R2-T8** (étiquettes **[ROM]** et **[R2]**) ; AMD-001 12-1, 10-8 (catégorie « revue »), 12-3 |
| REQ-ACC-011 | **AMD-018 Q-043** : frame synthétique définie (deux tiles de fond distinctes et un sprite) |
| REQ-ACC-020 | DEC-R2-T8 ; **AMD-023 Q-060** : les critères décrits comme une inspection humaine deviennent des mesures automatiques + captures archivées ; **AMD-023 Q-061** : la part MARCHING est couverte par Tetris, SMB et Metroid rendus avec MARCHING forcé, faute de ROM NES de Quarth ; **AMD-023 Q-062** : la clause « sans plantage » est instruite comme un défaut ; AMD-001 T-8 |
| REQ-ACC-021 | AMD-001 12-4 (retrait ou masquage des régions affectées par un écart D), T-8 ; **AMD-023 Q-060** |
| REQ-ACC-030 | **AMD-018 Q-042** : protocole de mesure des budgets ; **AMD-023 Q-062** : le banc doit tourner sans exception |
| REQ-ACC-031 | **DEC-R2-12-2** : moyenne ≤ 4 ms **et** 99ᵉ centile ≤ 8 ms ; **AMD-018 Q-042** ; AMD-001 12-2, T-8 |
| REQ-ACC-041 | **AMD-023 Q-062** : une exception, même rare, est incompatible avec cette exigence |
| REQ-ACC-050 | **DEC-R2-T8** : les exigences [ROM]/[R2] comptent comme justifiées ; AMD-001 12-1 ; AMD-023 Q-060 |
| tests d'acceptation | **AMD-022 Q-058** : mise à jour aux valeurs d'AMD-021 §4.1 |

---

## 4. Chaîne des remplacements

Un amendement postérieur peut annuler un amendement antérieur. Les cas réels, dans l'ordre :

| Point | D'abord | Puis | Enfin |
|---|---|---|---|
| Repère de scène | règle « 240 → 248 » (v1.0) | **DEC-R2-T2** + REQ-E3D-410 (AMD-001) | — |
| Ombre : taille de la carte | DEC-06-Q3 (512 texels) | **DEC-R2-D4** (1024 texels) | — |
| Instant de capture d'une rangée de fond | DEC-02-Q4 + AMD-003 Q-004 point 3 (tolérance d'une ligne) | **AMD-021 R2-005** (état de fin de ligne) | AMD-022 Q-059.1 (critère) |
| Zapper : vblank et bande droite | AMD-003 Q-005 points 1 et 3 | **AMD-021 R2-006** | AMD-022 Q-059.5 (ligne de post-rendu) |
| Transparence : granularité | AMD-009 Q-020 (par tile) | **AMD-012** (par objet) | AMD-021 R2-009 (mélange linéaire) |
| Ordre des candidats après retrait | DEC-04-Q02 (ordre d'inscription) | **AMD-021 R2-008** | AMD-022 Q-059.4 |
| Seuil des commandes analogiques | AMD-020 Q-048 (course brute) | **AMD-021 R2-011** (valeur mise en forme) | — |
| Zone morte de gâchette | REQ-INP-031 (0,1) | **AMD-021 §4.1** (0,2) | — |
| Visée du Zapper | REQ-INP-063 (formule fidèle, 8 lignes au-dessus) | **DEC-08-4 (D)** (pixel affiché) | AMD-020 Q-053 (critère recalculé) |
| Sons des réglages | notes et tableau de REQ-AUD-041 | **AMD-021 R2-013** | — |
| Matériaux TV | REQ-SCN-054 (« les TV ne reçoivent pas d'ombre », albédo 0,21) | **AMD-021 R2-014** (albédo 0,208, touches de TV 3 ombrées) | AMD-022 Q-059.2 |
| Raccourcis d'emplacement | AMD-010 REQ-UI-204 (`Fn`) | **AMD-013 Q-030** (`Ctrl+Fn`) | AMD-020 Q-049 (priorité sur SPEC-08) |
| `retro_serialize` et hooks Lua | proposition R2 (savestates manuels seuls) | **DEC-R2-D5** (DEC-07-Q4 prime) | — |
| Avance rapide | « ×2 » (v1.0, DEC-07-Q2) | **AMD-001 07-5** (×1,5, constat vérifié) | — |
| Fantasy « ×2 » | REQ-SCN-051 (×4,595) | **AMD-019 Q-045** (la courbe sRGB fait foi, ≈ 4,672 ±2 %) | — |
| Jaquettes | REQ-PERS-036/-074 (P2) | **AMD-020 Q-050** (P3, D) | — |
| Présentation de l'UI | SPEC-10 (onglets, tuiles) | **AMD-010** (émulateur classique ; SPEC-10 présentation informative, P3) | — |
| Mode 2D | REQ-E3D-381 | **AMD-009 Q-022 (D)** (image en pixels du cœur) | AMD-018 Q-041 (critère de -381 remplacé) |

---

## 5. Exigences créées après le gel de la v1.1

Elles n'apparaissent dans aucun fichier `SPEC-*.md` : on les lit dans leur amendement.

| Exigence | Priorité | Créée par | Objet |
|---|---|---|---|
| REQ-LOOP-018 | P2 | `AMD-002` (Q-002) | valider une ROM avant d'interrompre la partie en cours |
| REQ-E3D-410 | P1 | `AMD-001` T-2, sous DEC-R2-T2 — **écrite dans SPEC-04B v1.1** | repère de scène unique |
| REQ-SCN-065 | P2 | `AMD-012` | aucune ligne visible à la jointure de deux tiles |
| REQ-UI-200 … REQ-UI-209 | P1 sauf -207 et -208 (P2) | `AMD-010` | interface d'émulateur classique |

**Référence morte.** `SPEC-04B` REQ-E3D-406 cite « REQ-E3D-504, REQ-E3D-512 » ; **REQ-E3D-512
n'est définie nulle part** — le maillage local d'une tuile est spécifié par REQ-E3D-500 à -507 et
REQ-E3D-520 à -524. C'est un renvoi erroné du texte gelé, sans effet normatif : il est consigné
dans `changes/Q-063.md` et dans `TRACEABILITY-CHECK-P6.md`. Les fichiers gelés ne sont pas
modifiés.

---

## 6. Points encore ouverts

| Ticket | Sujet | État |
|---|---|---|
| `Q-059` | cinq points d'AMD-021 restés ouverts | répondus par `AMD-022` (Q-059.1 à .5) |
| `Q-062` | `NullReferenceException` intermittente dans le banc REQ-ACC-030 | réponse R0 dans `AMD-023` : défaut à corriger en P6. **Instruit et refermé par le lot P6** : cause extérieure au programme (ticket §11 et §12) ; aucune correction dans `src/`, un durcissement et quatre tests de charge |
| `Q-063` | renvoi `REQ-E3D-512` sans cible | ouvert par le lot P6 ; documentation seulement |

Les questions `Q-001` à `Q-058`, `Q-060` et `Q-061` sont répondues par les amendements du §2.
Les questions ouvertes dans les §6 des SPEC et tranchées par `DECISIONS-R0.md` y sont marquées
« close par DEC-… » (DEC-R2-T9).

---

## 7. Couverture de la matrice de traçabilité

Le relevé automatique trouve **676** chaînes de la forme `REQ-…` dans `spec/` et dans les
amendements ; trois n'en sont pas (les familles `REQ-INP-03x` et `REQ-INP-04x`, et le renvoi mort
`REQ-E3D-512` du §5). **Les 673 exigences qui restent ont chacune exactement une ligne** dans
`verify/round1/traceability.csv`, et la matrice ne contient ni doublon ni identifiant orphelin.
Le détail et le script de contrôle sont dans `TRACEABILITY-CHECK-P6.md`.
