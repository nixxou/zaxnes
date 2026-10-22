# R2 — Relecture ciblée de `AMD-021-round2.md`

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-21 | Version du rapport : 1.0
Entrée relue : `spec/changes/AMD-021-round2.md` v1.0.
Référentiel : `spec/SPEC-GUIDE.md`, `SPEC-02`, `SPEC-04`, `SPEC-06`, `SPEC-08`, `SPEC-11`, `SPEC-12`, `DECISIONS-R0.md`, `changes/AMD-012`, `AMD-020`, `Q-004`, `Q-005`, `Q-023`, `Q-048`, `Q-056`.
Méthode : ouverture de chaque exigence citée par AMD-021 ; recalcul de tous les vecteurs numériques (mappers 15 et 28, mélange linéaire, seuil analogique).
Catégories : **LEAK**, **INCONSISTENT**, **UNTESTABLE**, **UNCLEAR**.

**Verdict : ACCEPT WITH FIXES.** Aucune fuite. Deux non-conformités bloquantes (L-B1, L-B2 ci-dessous, toutes deux sur R2-006), cinq exigences de §1 sans critère d'acceptation (contraire à SPEC-GUIDE et à REQ-ACC-050), et six supersessions non déclarées. Rien ne justifie un rejet : toutes les corrections sont rédactionnelles ou consistent à ajouter un critère.

---

## 1. LEAK — aucune fuite

Relecture des trois interdits de SPEC-GUIDE § « Ce que la spec NE DOIT PAS contenir » :

- **Code / pseudo-code suivant la structure de l'original** : absent. Les suites de formules de R2-001, R2-002 et R2-003 sont des descriptions de décodage d'adresse et de calcul de banque, chacune avec un effet directement observable (quelle banque apparaît dans quelle fenêtre, quel mirroring). SPEC-GUIDE et la consigne de relecture les autorisent explicitement (« formules de registre/banque précises pour les mappers NES »). Elles correspondent au comportement public documenté pour Action 53 (28), 100-in-1 Contra Function 16 (15) et la variante Jaleco de 87 (101), tous déjà exigés par REQ-CORE-100.
- **Noms internes de classes, méthodes, champs ou variables privés** : aucun. Les seuls noms propres cités sont des noms d'exigences (`REQ-*`), de tickets (`Q-*`, `DEC-*`), de réglages visibles (Vitesse, FOV, Ombres) et de modèles d'assets (TV 1 / TV 2 / TV 3), qui sont du vocabulaire de la spec gelée.
- **Références au code d'origine** (méthodes, fichiers, numéros de ligne) : aucune.

Deux points relus et **acceptés** après examen :

- **R2-005** emploie « recopie horizontale du registre temporaire » et « incrément vertical de fin de ligne ». Ce sont les termes du matériel NES public (registres `v`/`t`, rechargement au point 257, incrément au point 256) et ils sont déjà le vocabulaire de REQ-CORE-037 et de `Q-004`. Autorisé au titre du « timing PPU en termes de matériel NES public ».
- **R2-015** (« linéariser séparément la couleur et l'intensité de la lumière ») décrit un ordre de calcul plutôt qu'un observable, mais il est borné par un observable chiffré (« écart d'au plus 1 niveau ») et la même information figure déjà, en clair, dans les Notes de REQ-SCN-028 (« l'original applique au facteur d'intensité la même courbe sRGB → linéaire qu'aux couleurs », DEC-06-Q2). Pas de fuite nouvelle.
- **R2-014** (« seul le premier sous-maillage de chaque modèle est dessiné ; l'écran de TV 3 ne l'est pas », « albédo = texture × 0,208 ») porte sur la structure des OBJ d'`assets-neutral/meshes/` et sur une constante colorimétrique, tous deux à effet visible immédiat. REQ-SCN-054 cite déjà ces mêmes meshes et textures. Autorisé.

L'en-tête « Rédigé par R1 à partir du jugement fonctionnel du round 2 » est une mention de provenance, pas un contenu : l'amendement est précisément le canal prévu pour faire passer un résultat de round 2 au côté propre. Aucune action.

---

## 2. INCONSISTENT

### L-B1 (bloquant) — R2-006 contredit les Notes de REQ-CORE-062 et le point 1 de la réponse à `Q-005`

R2-006 écrit : « Pendant le vblank **et** la ligne de pré-rendu, la couleur considérée est l'arrière-plan universel. »

Or :

- les Notes de REQ-CORE-062 disent : « Comme la lumière est mémorisée jusqu'à la fin de la frame, un jeu qui lit le Zapper **pendant le vblank voit encore la lumière de la frame qui vient d'être rendue** » ;
- la réponse R0 normative au point 1 de `Q-005` confirme : « les enregistrements de la frame dessinée **restent lisibles pendant le vblank qui suit** et sont vidés au début de l'image suivante ; pendant le pré-rendu, aucun pixel visible n'est “dépassé” ».

À la lettre, R2-006 annule donc la détection de lumière pendant tout le vblank, c'est-à-dire exactement la fenêtre où les jeux au Zapper lisent le port. C'est l'inverse du comportement exigé par une exigence P1 [ROM] et par une réponse R0 déjà normative, et AMD-021 ne déclare aucune supersession sur ce point (il n'annonce que le point 3 de `Q-005`).

**Correction demandée** : restreindre la clause à la ligne de pré-rendu seule — « pendant la ligne de pré-rendu, la couleur considérée est l'arrière-plan universel ; pendant le vblank, les enregistrements de la frame qui vient d'être rendue restent lus (REQ-CORE-062, Notes ; `Q-005` point 1) » — ou, si l'intention est bien de changer le comportement en vblank, le dire explicitement et expliquer comment le critère [ROM] de REQ-CORE-062 (Duck Hunt, Hogan's Alley, Wild Gunman) peut encore passer.

### L-B2 (bloquant) — R2-006 ne rend pas normatif ce qu'il prétend rendre normatif

R2-006 écrit : « Un pixel d'abscisse ≥ 239 est **toujours vu avec la couleur d'arrière-plan universel** (rend normatif le point 3 de la réponse à `Q-005`). »

Le point 3 de `Q-005` dit autre chose : « la référence **ne détecte jamais de lumière** sur les colonnes x ≥ 239 pour les enregistrements de fond comme de sprite ».

Les deux formulations ne coïncident pas. REQ-CORE-062 classe « claire » toute couleur NES sauf les neuf noirs ; si l'entrée 0 de la palette est claire (cas courant hors fond noir), la formulation de R2-006 fait **détecter** la lumière à x ≥ 239, alors que le point 3 de `Q-005` n'en détecte jamais. La clause énonce donc une règle différente de celle qu'elle cite.

**Correction demandée** : écrire « aucune lumière n'est détectée pour x ≥ 239 » si le point 3 de `Q-005` est l'intention. Préciser au passage que la condition 1 de REQ-CORE-062 (0 ≤ x < 256) est inchangée, la clause ne portant que sur la bande x ∈ [239, 255].

### L-1 — R2-005 laisse en l'état la tolérance d'une ligne de REQ-CORE-037

AMD-021 déclare correctement que R2-005 « remplace la réponse à `Q-004` sur ce point » (le point 2 de cette réponse, qui validait la lecture « au début de la ligne L »). Mais le **point 3** de la même réponse R0 est aussi normatif : « la tolérance d'une ligne de REQ-CORE-037 est **symétrique** : le nouvel état d'un split peut s'appliquer une ligne plus tôt ou une ligne plus tard que sur le matériel ». Le DOIT de R2-005 (« refléter l'état du PPU après toutes les écritures du CPU effectuées pendant la ligne L ») interdit désormais le côté « une ligne plus tard » pour une écriture en milieu de ligne. Les deux textes ne peuvent pas lier en même temps.

**Correction demandée** : ajouter une phrase disant que, pour les écritures du CPU en milieu de ligne, la tolérance d'une ligne de REQ-CORE-037 ne s'applique plus, et indiquer si `DEC-02-Q4` (« tolérance d'une ligne maintenue ; différentiel SMB, SMB3, Zelda », qui est précisément le différentiel dont R2-005 est le résultat) est de ce fait clos.

### L-2 — R2-008 modifie une exigence P1 en se présentant comme une « note »

REQ-E3D-091 est **P1** et fixe l'ordre des candidats comme « l'ordre d'inscription au registre des patterns permanents » ; `DEC-04-Q02` réaffirme « égalité de candidats : ordre d'inscription » et renvoie la validation au round 2. R2-008 introduit une règle différente (le nouveau pattern reprend « la place du dernier pattern retiré » de cette image). C'est un changement de fond d'une exigence P1, étiqueté « note, P3 », sans mention de supersession.

**Correction demandée** : étiqueter R2-008 « REQ-E3D-091 amendée, P1 », et indiquer que `DEC-04-Q02` et la question ouverte Q-02 de SPEC-04 §6 sont closes par ce résultat.

### L-3 — R2-011 contredit la constante de gâchette de REQ-INP-031

R2-011 écrit « zone morte 0,2 et saturation 0,9 pour une gâchette ». REQ-INP-031 dit « les gâchettes DEVRAIENT avoir une zone morte de **0,1** », et n'énonce la saturation 0,9 et le ré-étalement que pour les sticks. L'analyse de `Q-048` utilise elle-même 0,1 (« donne 0,44 après la zone morte de 0,1 »).

La supersession d'`AMD-020` pour `Q-048` est, elle, correctement déclarée (en-tête **et** corps de R2-011), et la clôture de `Q-056` est cohérente : la mise en forme redevient bien le point d'application du seuil, donc la saturation et le ré-étalement redeviennent observables, ce qui est exactement ce que `Q-056` demandait.

**Correction demandée** : choisir — soit « zone morte 0,1 » (conforme à REQ-INP-031), soit déclarer REQ-INP-031 amendée à 0,2 / 0,9 pour les gâchettes. Le critère de REQ-INP-030 (« gâchette enfoncée à moitié → `ZoomIn` enfoncé ») passe dans les deux lectures (0,429 avec 0,2/0,9 ; 0,5 avec 0,1/0,9, tous deux ≥ 0,34), donc aucun critère existant ne se casse — mais la constante doit être unique.

### L-4 — R2-013 contredit les Notes de REQ-AUD-041 sans le dire

Les Notes de REQ-AUD-041 (P2) disent : « les curseurs de la page Réglages (**dont Vitesse et FOV**) déclenchent aussi un son d'interface à chaque changement ; le clip exact associé au curseur de vitesse n'a pas pu être déterminé (Q5), le son de validation est retenu par défaut. » R2-013 énonce l'inverse pour ces deux réglages. C'est une bonne clôture de `DEC-11-Q5`, mais elle est étiquetée « note, P3 » alors qu'elle remplace la note d'une exigence P2.

Second point : « Tous les autres réglages de la fenêtre d'options déclenchent le son de validation » **élargit** le tableau de REQ-AUD-041, qui énumère limitativement (volume, lumière, échelle, listes déroulantes, cases à cocher).

**Corrections demandées** : (a) écrire « remplace la note de REQ-AUD-041 » et garder P2 ; (b) dire que la ligne « validation » du tableau est étendue à tout réglage de la page Réglages sauf Vitesse et FOV ; (c) employer le vocabulaire de la spec — SPEC-10/SPEC-11 disent « page Réglages », `AMD-010` définit l'interface classique ; « fenêtre d'options » n'est défini nulle part.

### L-5 — R2-014 contredit deux énoncés de REQ-SCN-054

1. REQ-SCN-054 se termine par « les modèles sont éclairés par la lumière de la scène … et **ne reçoivent pas d'ombre** », et ne dit rien d'une projection. R2-014 écrit « les touches de TV 3 **projettent et reçoivent** les ombres ». C'est un changement, pas un « complément ».
2. REQ-SCN-054 donne pour TV 3 « albédo = `2000s_CRT_TV_black_AO` × **gris 0,21** ». R2-014 donne « albédo = texture × **0,208** ». Deux constantes différentes, sans indication de celle qui prime.

**Corrections demandées** : marquer explicitement que la phrase « ne reçoivent pas d'ombre » de REQ-SCN-054 est amendée (corps des téléviseurs : ni projection ni réception ; touches de TV 3 : les deux), et que 0,208 remplace 0,21 (ou conserver 0,21).

### L-6 — R2-015 est classé à la fois en divergence et en confirmation

§2 inscrit R2-015 (linéarisation séparée de la couleur et de l'intensité, REQ-SCN-028) comme divergence acceptée **D**, tandis que §3 confirme `DEC-06-Q2`, qui est précisément la décision prescrivant ce comportement (Notes de REQ-SCN-028). Ou bien c'est conforme (confirmation), ou bien c'est un écart (D) : il faut trancher, faute de quoi la matrice de traçabilité recevra deux verdicts contradictoires pour la même exigence.

### L-7 (mineur) — R2-007 définit la conformité par renvoi à l'implémentation

« Au-delà de la 8ᵉ lecture d'un port manette, la valeur renvoyée reste **celle de la reconstruction** » : un tiers qui ne connaît que la spec ne peut pas savoir ce qui est attendu. REQ-CORE-022 renvoie au protocole public de nesdev, qui documente la valeur des lectures suivantes. **Correction** : nommer la valeur (par exemple « bit 0 = 1 ») en gardant le classement D, ou écrire que le comportement est un choix d'implémentation libre.

---

## 3. UNTESTABLE / critères manquants

SPEC-GUIDE : « Chaque exigence porte un critère d'acceptation **vérifiable par un tiers qui ne connaît que la spec**. » REQ-ACC-050 conditionne le gate à l'absence d'exigence « non testable » non justifiée. Les items de §2 (divergences) et §3 (confirmations) n'ont pas à porter de critère ; ceux de **§1** si.

| Item | État | Correction demandée |
|---|---|---|
| **R2-003** (mapper 101) | **aucun critère** | Ajouter un vecteur, p. ex. : CHR synthétique de 32 Kio dont chaque banque de 8 Kio porte son numéro ; `$6000 ← $02` → banque 2 ; `$E000 ← $07` → banque 3 (valeur & 3) ; `$5FFF ← $01` sans effet. |
| **R2-006** | **aucun critère** | Après règlement de L-B1/L-B2 : couleur universelle `$30` (claire), pointeur en (250, 100), lecture à la ligne 200 → pas de lumière ; même lecture faite pendant le vblank sur un pixel x < 239 couvert par un enregistrement clair → lumière détectée. |
| **R2-008** | **aucun critère** | L'ordre des candidats n'est observable qu'à travers le mesh finalement retenu pour une image. Donner l'observable : image I avec patterns P1 puis P2 inscrits ; retirer P1, inscrire P3 contenant I → le mesh retenu pour I est celui de P3 et non celui de P2 ; si aucun retrait n'a eu lieu, P3 passe après P2. |
| **R2-013** | **aucun critère** | Ajouter l'observable : déplacer le curseur Vitesse ou FOV → aucun son d'interface ; basculer la case Ombres → son de validation à chaque changement, au volume courant. |
| **R2-014** | **aucun critère** pour ses quatre points | Au moins les trois premiers sont vérifiables « auto » / « revue » sur `assets-neutral/` : capture hors écran avec TV 1 puis TV 3 → aucun pixel translucide dans le corps du téléviseur ; avec TV 3, l'écran du modèle n'est pas dessiné (le rectangle visible n'est pas recouvert) ; une zone du corps de TV 3 hors émission mesure la couleur de la texture × 0,208 ± 2 niveaux. |
| **R2-014**, 5ᵉ point | **non vérifiable** : « PEUT être approché par **tout modèle de reflet raisonnable** » | « Raisonnable » n'est pas mesurable. Soit écrire que l'aspect spéculaire des modèles TV est un choix d'implémentation libre (ce que SPEC-GUIDE § « Liberté laissée aux implémenteurs » accorde déjà, sans besoin d'exigence), soit donner une borne de revue (zone de brillance dans tel rectangle de la vue de référence, écart ≤ N niveaux). |
| **R2-005**, critère 2 | **partiellement non calculable** : « le nombre de rangées par frame ne dépasse pas **celui d'une image sans split** » | L'écran titre et la démo de SMB 3 comportent toujours un split : la référence de comparaison n'existe pas dans l'échantillon. Donner un nombre (une frame NTSC sans écriture de `$2006` en milieu de ligne contient 30 lignes de Y fin 1 → « ≤ 30 rangées de fond par frame »). Le reste du critère est bon, et l'étiquette [ROM] est légitime (SPEC-12 §2 ; SMB 3 est dans REQ-ACC-020). |
| R2-005, critère 1 | **bon** | Discriminant et cohérent avec REQ-CORE-038 (rangée capturée à L = 100 avec Y fin 1 → `y` = 99, dans la plage −1…238). La seconde phrase (passer à Y fin 2 pendant une ligne de Y fin 1 → aucune rangée) teste bien le déplacement de l'instant. |
| R2-010 | **bon** | La « scène synthétique d'ombre de REQ-SCN-030 » existe bien (critère de REQ-SCN-030 : tile pleine en L1 au-dessus d'une tile plate en L2 recevant les ombres, lumière (1, −1, 1)). Remarque sans conséquence : la moitié « HUD » de R2-010 est redondante — REQ-SCN-032 conditionne déjà la réception à « elle n'appartient pas au HUD » et REQ-SCN-029 dit « le HUD ne projette ni ne reçoit d'ombre ». Le passage de DEVRAIT à NE DOIT PAS pour les transparents est, lui, un durcissement correctement annoncé. |
| R2-001 | **bon**, recalculé | `F4C0E0` mis à part, j'ai revérifié les quatre vecteurs : t = bits 4-5 du mode, m = 2^(t+1) − 1. (ext 5, mode 0) : m = 1, E = 10, I = 15 → `$8000` = (10 & ~1) \| (30 & 1) = 10, `$C000` = 11 ✓. (ext 2, mode `$18`, int 3) : mode PRG 2, t = 1, m = 3, E = 4 → 4 / ((4 & ~3) \| 3) = 7 ✓. (ext 2, mode `$1C`, int 3) : mode PRG 3 → ((4 & ~3) \| 3) = 7 / (E + 1) = 5 ✓. CHR ← `$10`, mode 0 : bit 1 du mode nul, donc bit 0 du mode ← bit 4 de `$10` = 1 → mode bits 0-1 = 1 → écran B ✓. |
| R2-002 | **bon**, recalculé | `$8001 ← $05` : mode 1, B = 5 → 5 / dernière banque 16 Kio d'une ROM de 1 Mio = 63 ✓. `$8002 ← $85` : mode 2, B = 5, h = 1 → banque 8 Kio 2·5 + 1 = 11 dans les quatre fenêtres ✓. `$8000 ← $40` : bit 6 = 1 → horizontal ✓. Mise sous tension (mode 0, B = 0) → banques 0 et 1 = 32 premiers Kio ✓. |
| R2-009 | **bon**, recalculé | `F4C0E0` est bien la couleur NES `$35` du tableau §4.1 de SPEC-04. Avec l'ambiante de REQ-SCN-027 (0,1812 en linéaire) et REQ-SCN-028 : albédo linéaire (0,9047 ; 0,5273 ; 0,7455) × (0,1812 + 0,577) = (0,6859 ; 0,3998 ; 0,5653) ; × α = 0,5 devant un fond noir → (0,3430 ; 0,1999 ; 0,2826) → sRGB (158,2 ; 123,5 ; 144,9), soit **(158, 124, 145) ± 2** ✓. Le critère discrimine fortement : un mélange fait en sRGB donnerait (108, 85, 99). Deux remarques (voir ci-dessous). |

**Deux remarques sur le critère de R2-009** (à corriger, sans remettre en cause le fond) :

1. L'ambiante est une **entrée** du calcul et n'est pas citée. Ajouter « ambiante de REQ-SCN-027 » ; sinon un tiers ne peut pas reproduire les trois nombres.
2. La seconde clause — « R = 158 ± 2 devant un fond sRGB (0, 120, 131) » — **ne discrimine rien** : la composante R de ce fond vaut 0, donc le résultat est identique en espace linéaire et en espace sRGB, et la clause répète la première. Utiliser une composante non nulle du fond : avec le même motif devant (0, 120, 131), **G = 147 ± 2** en mélange linéaire (contre 134 en mélange sRGB).

---

## 4. UNCLEAR (mineur, à préciser sans blocage)

- **U-1, R2-001** : « Une écriture en `$5000-$5FFF` choisit le registre cible par **ses** bits 7 et 0 ». « Ses » peut se lire comme les bits de l'adresse ou de la valeur ; seule la lecture « valeur écrite » est compatible avec les codes `$00`/`$01`/`$80`/`$81` qui suivent. Écrire « par les bits 7 et 0 de **la valeur écrite** ».
- **U-2, R2-001** : à la mise sous tension, externe = 63 donne E = 126 banques de 16 Kio, au-delà de toute ROM de moins de 2 Mio (une ROM de 1 Mio n'en compte que 64). Ni R2-001 ni REQ-CORE-100 ne disent comment un numéro de banque hors ROM est résolu (masquage à la taille de la ROM, bus flottant). Le critère évite le cas, mais la règle est nécessaire pour implémenter l'état de mise sous tension. Ajouter « les numéros de banque sont masqués à la taille de la ROM » (ou la règle retenue).
- **U-3, R2-014** : « Seul le premier sous-maillage de **chaque modèle** est dessiné » laisse ouvert si les cinq meshes `Button_*_LOD0` attachés par REQ-SCN-054 comptent comme des « modèles » distincts — donc toujours dessinés — ou comme des sous-maillages de TV 3, donc supprimés, ce qui contredirait le point suivant (« les touches de TV 3 projettent et reçoivent les ombres »). Écrire « de chacun des meshes listés au tableau et au paragraphe de REQ-SCN-054 ».
- **U-4, R2-010** : « quels que soient les drapeaux de son motif et de sa couche » ne vise que la **réception**. La règle de **projection** de REQ-SCN-032 (« elle est rendue opaque ») exclut déjà les transparents, et l'exception « ombre seule » de REQ-SCN-041 reste (un motif de style 2 est toujours rendu en opaque, donc hors du champ de R2-010). Une phrase le confirmant éviterait une lecture où R2-010 supprimerait aussi l'ombre portée des motifs « ombre seule ».

---

## 5. Points relus et conformes, sans action

- **R2-012** (à travers un objet transparent, les transparents situés derrière restent visibles) est cohérent avec REQ-SCN-040 (« seule la surface la plus proche de **chaque objet** transparent est visible », tri du plus lointain au plus proche) et avec `AMD-012` (passage de profondeur et tri **par objet entier**). Le classement D est correct : la restriction porte sur l'objet, pas sur la scène.
- **R2-004** (comportement nesdev pour le mapper 27 ; écarts de largeur CHR et de mirroring VRC2 sans effet sur les cartouches commerciales) est cohérent avec REQ-CORE-100, qui exige « le comportement documenté sur nesdev » et liste déjà 27 et 21/22/23/25.
- **Mappers 15, 27, 28, 101, 225 et 228 figurent tous déjà au tableau de REQ-CORE-100** : R2-001 à R2-004 et §3 précisent des cartes exigées, ils n'en ajoutent aucune. Aucune interaction avec REQ-CORE-102.
- **R2-006, autres points** (pas de cache par bande de 8 lignes) est conforme : REQ-CORE-062 écrit « le résultat **PEUT** être recalculé une seule fois par groupe de 8 lignes ». Le classement D est même superflu, l'option n'étant pas exigée.
- **Conventions d'identifiants** : les `R2-NNN` sont déclarés « uniquement pour la traçabilité » et chaque item renvoie à un `REQ-<MOD>-NNN` existant ; aucun identifiant de REQ n'est réutilisé ni recyclé. Conforme à SPEC-GUIDE § « Identifiants ». Les trous de numérotation en §1 (004, 006 partiel, 007, 012, 015) sont expliqués par §2.
- **Portabilité libretro** (SPEC-GUIDE § « Contraintes transverses ») : aucun item n'introduit d'état global, de dépendance de plateforme ni de non-déterminisme. R2-005 et R2-008 ajoutent de l'état au cœur et au registre de patterns, qui reste local à l'objet et sérialisable, donc compatible avec les savestates déterministes de REQ-ARCH-012/013.

---

## 6. Synthèse des corrections demandées

**Bloquantes (gate)**

1. **L-B1** — R2-006 : restreindre la clause « arrière-plan universel » à la ligne de pré-rendu, ou déclarer la supersession des Notes de REQ-CORE-062 et du point 1 de `Q-005`.
2. **L-B2** — R2-006 : reformuler en « aucune lumière n'est détectée pour x ≥ 239 », conformément au point 3 de `Q-005` qu'il cite.
3. **C-1 à C-6** — ajouter un critère d'acceptation à R2-003, R2-006, R2-008, R2-013 et R2-014, et rendre mesurable (ou libre) le point spéculaire de R2-014.

**Supersessions à déclarer** : L-1 (tolérance de REQ-CORE-037 / `Q-004` point 3 et `DEC-02-Q4`), L-2 (REQ-E3D-091 P1 et `DEC-04-Q02`), L-3 (zone morte de gâchette de REQ-INP-031), L-4 (Notes et tableau de REQ-AUD-041), L-5 (ombres et constante 0,21 de REQ-SCN-054), L-6 (classement de R2-015 : D ou confirmation).

**Rédactionnelles** : L-7, U-1 à U-4, et les deux remarques sur le critère de R2-009 (citer REQ-SCN-027 ; remplacer la clause R non discriminante par G = 147 ± 2).

**LEAK : 0.** Le document est publiable au côté propre en l'état ; les corrections ci-dessus portent sur la cohérence et la testabilité, pas sur l'étanchéité.
