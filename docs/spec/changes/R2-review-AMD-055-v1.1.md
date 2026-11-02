# R2 — Seconde relecture d'`AMD-055` (style visuel « Enhanced », v1.1)

Rôle : R2 (filtre anti-fuite, côté PROPRE) | Date : 2026-10-31 | Version du rapport : 1.1
Objet : vérifier **point par point** que la v1.1 d'`AMD-055-style-enhanced.md` lève les 24 points de
`R2-review-AMD-055.md` (v1.0 du rapport), juger la raison donnée pour les points **non retenus**,
puis relire le fichier **en entier** pour chercher toute fuite, incohérence, critère intestable ou
référence `REQ` cassée **introduite par les corrections**.

Entrées relues : `spec/changes/AMD-055-style-enhanced.md` (v1.1 du 2026-10-31, texte sous revue,
**non modifié**) ; `spec/changes/R2-review-AMD-055.md` (v1.0 du rapport, **non modifiée**) ;
`spec/SPEC-GUIDE.md` ; `SPEC-06` §3.9 (`REQ-SCN-025` à `-029`), §3.10 (`REQ-SCN-030` à `-033`),
§3.11 (`REQ-SCN-034` à `-043`), §3.12 (`REQ-SCN-044`, `-045`), §3.13 (`REQ-SCN-046` à `-048`),
§3.15 (`REQ-SCN-053`, `-054`), §3.16 (`REQ-SCN-055`), §3.17 (`REQ-SCN-056` à `-064`), §4.1, §4.2,
§4.3, §5.4, §6, Historique ; `SPEC-04B` §2, `REQ-E3D-400`, `-401`, `-403`, `-406`, `-407`, `-410`,
`-452`, `-504`, `-505`, `-520` à `-523`, `-566` à `-568`, `-640`, `-641` ; `SPEC-09` §2, §4.1
(légende des colonnes), §4.1.1, §4.6, `REQ-PERS-001` à `-005`, `-010`, `-017`, `-022`, `-080` à
`-086` ; `SPEC-10` §3.7 (`REQ-UI-060` à `-065`), §5 V2 ; `SPEC-12` §2 (catégories auto / `[ROM]` /
`[R2]` / **revue**), `REQ-ACC-020`, `-021`, `-031`, `-050` ; `spec/changes/` : `AMD-032` §2,
`AMD-035`, `AMD-040` (§2 `REQ-LUA-112` à `-115`, §3 et ses lignes `SPEC-09`), `AMD-043`
(`REQ-SCN-066` à `-069`, §5, §6), `AMD-046` (§2 définition du **rectangle du mode 2D**,
`REQ-SCN-074`, §3), `AMD-049` (`REQ-SCN-075`, §3), `AMD-050` (`REQ-SCN-076`), `AMD-052`
(`REQ-PERS-089`, retrait de `REQ-SCN-077`), `R2-review-AMD-049.md`,
`R2-review-AMD-050-053-v1.1.md` ; `SPEC-v1.2-INDEX.md` (§ SPEC-02 et SPEC-06, tableaux des
amendements) ; `journal/JOURNAL.md` ; `impl/docs/` (copie partielle de la spec arrêtée au
2026-10-30, `verify/round1/`).
Aucun accès au côté sale : le pare-feu n'a pas eu à intervenir, aucune tentative de contournement.
**Le serveur d'outils de rétro-ingénierie présent dans la session n'a pas été utilisé** (il est hors
des sources autorisées du côté propre).

Méthode : (a) chaque point de la v1.0 est confronté au texte de la v1.1, **citation contre
citation**, y compris pour les phrases que la v1.1 annonce avoir supprimées ou remplacées dans
`AMD-040`, `AMD-043` et `AMD-049` (relues dans les fichiers d'origine, pour vérifier que la phrase
citée y figure bien **mot pour mot**) ; (b) **recalcul indépendant** de toutes les valeurs numériques
nouvelles : conversions sRGB ↔ linéaire, modèle de `REQ-SCN-028` lu avec le facteur d'ombre de
`REQ-SCN-031`, les quatre lignes du tableau de `REQ-SCN-081`, les deux niveaux sRGB de
`REQ-SCN-082` et la géométrie de son ombre, le cadrage du « gros plan G », les trois tables de
niveaux de `REQ-SCN-084`, le rayon d'occlusion, et la **géométrie de la courbure** du second critère
de `REQ-SCN-080` (enroulement de `REQ-SCN-067` calculé point par point, puis projeté) ; (c) contrôle
de la numérotation et de **chaque** renvoi `REQ` du fichier ; (d) relecture d'étanchéité du texte
nouveau (formule de modulation, §4, §5).
Note de provenance : inchangée par rapport à la v1.0 du rapport — l'en-tête renvoie au « rapport de
comparaison avec 3dSen 2.0 », document du côté sale non lisible ici ; convention déjà admise, aucune
de mes vérifications ne s'appuie dessus.

---

## 1. Verdict

**Étanchéité : acquise, et améliorée.** Aucune fuite, ni ancienne ni nouvelle. Les trois réserves
faibles de la v1.0 sont traitées à la source : l'attribution de « MSVO » à une bibliothèque tierce
sous licence a disparu au profit d'une formulation purement juridique (55-2), les identifiants de
paramètres portent désormais l'énoncé de leur légitimité (« arguments de l'API Lua … faits
d'interopérabilité », 55-3), les constantes portent leur justification (« mesurées sur 3dSen 2.0 ;
**les critères font foi** », 55-3), et les nombres d'échantillons, de pas de marche et de
raffinements sont passés en **DEVRAIT** avec la mention explicite « ces nombres ne sont pas
observables … c'est l'aspect qui fait foi » (55-1). La formule de modulation de `REQ-SCN-081` et les
trois pas de `REQ-SCN-082` sont des **descriptions mathématiques d'un résultat**, catégorie
explicitement autorisée par le `SPEC-GUIDE` : aucune structure de code, aucun nom interne, aucune
architecture, aucun renvoi au code d'origine. Le texte nouveau est plus sûr que celui de la v1.0.

**Fond : nettement accepté, mais pas encore franchi.** Sur les 24 points, **22 sont levés** et
**2 partiellement** (55-7 et 55-8) ; les 14 bloquants de la v1.0 sont tous traités au fond, et les
quatre points « non retenus » le sont pour une raison que je juge **recevable** (§3). Le travail de
calibration demandé a été fait et il est **juste** : j'ai recalculé les quatre rapports de
`REQ-SCN-081` (0,62 / 1,00 / 1,05 / 0,384 et sRGB 133 / 166 / 169 / 107), les deux niveaux de
`REQ-SCN-082` (195 et 118, avec les frontières aux coordonnées **6** et **14**), les trois tables de
`REQ-SCN-084` et le rayon d'occlusion — tout tombe juste, à une coquille près (N-7).
L'incompatibilité arithmétique de la v1.0 (55-10 e) a disparu : le plafond `O ≤ 0,1 × ao.strength`
et les invariants sont **simultanément satisfaisables**, vérifié.

Il reste **2 points bloquants nouveaux**, tous deux nés des corrections :

- **N-1** — le critère censé rendre **discriminante** la clause d'ordre vis-à-vis de la courbure
  (second critère de `REQ-SCN-080`) est **arithmétiquement insatisfaisable** : avec une courbure
  gauche-droite, les lignes de rainure **verticales** restent rigoureusement droites, et la plus
  déviante des horizontales ne s'écarte de sa corde que de **2,7 pixels** au cadrage imposé (caméra
  à 60 unités), contre « dépasse 5 pixels » exigé. Une implémentation conforme échoue au critère.
- **N-2** — le sort des **objets transparents** est écrit deux fois de deux façons contradictoires :
  §2.2 (4ᵉ tiret), §3 point 3 et la portée de `REQ-SCN-083` disent qu'ils « contribuent à la
  profondeur, donc ils occultent et peuvent être reflétés », alors que l'ordre de frame que le même
  amendement fixe place **toute** la passe transparente (y compris son passage de profondeur,
  `REQ-SCN-040` lu avec `AMD-012`) **après** les deux passes d'écran. La portée de `REQ-SCN-084`
  (« s'ils sont déjà dessinés au moment de la passe ») dit l'inverse, et c'est elle qui est juste.
  C'est exactement la question que 55-8 demandait de trancher, revenue sous une autre forme.

Les deux se corrigent en **une clause chacun**, sans rouvrir aucune décision du propriétaire.

**Réponse à la question de la v1.0 (« `REQ-SCN-083` et `REQ-SCN-084` laissent-ils produire un
résultat comparable sans deviner ? »).** La v1.1 **change la question**, et c'est la bonne décision.
Les deux exigences ne promettent plus un résultat *comparable* mais un résultat *conforme*, et
celui-là, oui, un implémenteur peut le produire **sans deviner** : ce qui est libre est déclaré
libre, les paramètres observables sont complets (rayon, plafond, longueur, intensité, loi de flou,
espace colorimétrique, portée, définition mesurable du sol), et les invariants automatiques plus les
critères de revue bornent le résultat. Deux implémentations ne donneront pas la même image ; le
propriétaire l'assume explicitement (§1, §5), le `SPEC-GUIDE` l'autorise (« tout ce que la spec ne
fixe pas est un choix d'implémentation libre ») et `SPEC-12` fournit la catégorie **revue** et la
règle de conformité correspondante (`REQ-ACC-050` : « conforme si la revue est archivée »).
*Vérification au passage, favorable* : les deux effets valant **0 par défaut**, les captures
différentielles de `REQ-ACC-021` au round 2 (vue par défaut) ne sont pas touchées par ce choix — le
SSIM n'a pas à classer ces effets en écart D.
Réserve unique sur ce point : les invariants « auto » de §2.4 et §2.5 n'ont **pas de configuration
nommée**, contrairement au « gros plan G » de §2.3, et deux d'entre eux ne sont pas satisfaisables
pour certains choix légitimes de scène (N-3).

**Numérotation : correcte, revérifiée.** `REQ-SCN-078` à `-084` sont libres et contigus ; aucun de
ces sept numéros n'apparaît ailleurs dans `spec/` ; `REQ-SCN-077` reste **retirée** (`AMD-052`), son
numéro conservé. Les priorités annoncées en en-tête (`-078` à `-082` P2, `-083` et `-084` P3)
correspondent **exactement** aux priorités portées par les énoncés. La clause « Priorités : » et ses
replis `(T-1)` suivent la convention d'`AMD-046`, `AMD-049` et `AMD-052`.

**Références : exactes.** J'ai vérifié une à une les 45 références du fichier. Toutes existent et
disent ce que le texte leur fait dire, y compris les cinq phrases citées pour être remplacées
(`AMD-040` §2 ligne `VisualStyle` et note des défauts ; `AMD-043` `REQ-SCN-066` et colonne Lua du
§6 ; `AMD-049` `REQ-SCN-075` phrase sur la grille ; `SPEC-06` `REQ-SCN-058` ; `SPEC-10`
`REQ-UI-061` et la phrase « Les douze libellés sont présents, et aucun autre paramètre
n'apparaît »), que j'ai retrouvées **mot pour mot** dans les fichiers d'origine. Les trois défauts
de référence de la v1.0 sont levés : 55-19 (`REQ-SCN-029` + `REQ-SCN-044` pour le HUD), 55-20
(`AMD-046` §2 cité à chaque mention du rectangle du mode 2D), 55-24 (placement `SPEC-06` §3.11bis
et mise à jour de l'index). Deux renvois seulement mériteraient une précision, sans être faux
(N-8 : `REQ-E3D-522` pour `MARCHING`, index incomplet).

---

## 2. Tableau des 24 points de la première relecture

| Point | Sujet (v1.0) | Verdict | Vérification |
|---|---|---|---|
| **55-1** | paramètres d'algorithme non observables rendus normatifs | **levé** | `REQ-SCN-083` : « L'implémentation **DEVRAIT** prendre de l'ordre de 6, 10 et 16 échantillons … Ces nombres ne sont pas observables : ils ne sont donnés qu'en ordre de grandeur, et c'est l'aspect … qui fait foi » ; `REQ-SCN-084` idem pour 12/24/48/48 pas et 2/3/5/5 raffinements. Les observables de remplacement sont là : revue « sans bande ni motif de bruit fixe », « en Ultra, le tracé ne manque aucun objet plus large que 2 pixels d'écran » — exactement la correction proposée. |
| **55-2** | « MSVO empruntée à une bibliothèque sous licence de Unity » | **levé** | Remplacé **mot pour mot** par la formulation proposée (« repose sur une technique tierce qui n'est pas reproductible dans le cadre de cette réimplémentation ») ; le libellé `MSVO` est conservé **avec sa justification** (« c'est une valeur visible du réglage, qui doit se relire »). Plus aucun fait sur les dépendances internes de l'original. |
| **55-3** | légitimité des identifiants, constantes sans justification | **levé** | §2.1 : « Ces douze identifiants sont normatifs parce qu'ils sont des **arguments de l'API Lua** (REQ-LUA-114) et, pour cette raison, des faits d'interopérabilité ; les autres … restent « inconnus » au sens d'AMD-040 §3 » ✓. Notes de `REQ-SCN-081` et Notes finales de §2.4/§2.5 : « les constantes … sont mesurées sur 3dSen 2.0 ; **les critères font foi** » ✓ (modèle d'`AMD-043` §3). `REQ-SCN-082` n'introduit aucune constante nouvelle : rien à justifier. |
| **55-4** | contradiction frontale avec `AMD-049` `REQ-SCN-075` | **levé** | En-tête : « **AMD-049** (REQ-SCN-075 : phrase sur la grille de voxels) » ✓. §3 point 5 supprime la phrase **citée exactement** telle qu'elle figure dans `AMD-049` et la remplace par la rédaction proposée, qui vise bien l'**albédo `c`** — j'ai relu la formule de `REQ-SCN-075` : `c` n'apparaît que dans `c · 0,96 · (Amb + s·E·Fd·nl)`, le terme spéculaire en est indépendant, donc « le terme spéculaire n'est pas assombri par une rainure » est exact ✓. Repli `(T-1)` prévu si `REQ-SCN-081` n'est pas retenue ✓. |
| **55-5** | repère des « cellules de N voxels » contre la définition du voxel | **levé** | Section **Repère** explicite : repère **local du maillage** (`REQ-E3D-504`, `x, y ∈ [−4, 4]`, `z ∈ [−D/2, D/2]`), **avant** transformation d'objet et **avant** courbure pour les tiles ; **unités de scène** pour le backdrop. Vérifié contre `REQ-E3D-504` (coin de voxel `(i, j, k)` → `(i−4, j−4, k−D/2)`) : les bords de voxel tombent bien sur les entiers en `x`/`y` et sur les entiers décalés de `D/2` en `z` ✓. Les deux critères « `scale = (2,1,1)` → écartement doublé » et « `rot = (0,0,30)` → rainures tournées de 30° » rendent le choix **vérifiable**, ce que la v1.0 ne permettait pas. |
| **55-6** | « arête de cellule dans le plan de la face » inexistante pour le facteur 0,38 | **levé, autrement** (point non retenu, raison recevable) | La reformulation « ligne d'intersection » est remplacée par la formulation par **poids d'axe** `w_a = clamp(4(1 − \|n_a\|), 0, 1)`. Je la juge **meilleure** : elle couvre d'un coup les faces alignées (deux familles de rainures, un axe de poids nul), les faces obliques, les objets tournés et `MARCHING` (Note dédiée), sans cas particulier, et elle est continue en `n`. Le facteur 0,38 est désormais **atteignable** : les faces `GREEDY` étant toujours alignées sur un axe (`REQ-E3D-505`), la branche oblique ne s'applique qu'aux surfaces `MARCHING`, ce qui est cohérent. Réserve mineure nouvelle sur le seuil 0,9 : N-6. |
| **55-7** | place des quatre effets vis-à-vis de la courbure d'`AMD-043` | **partiellement levé** | La clause d'ordre demandée est écrite (§2.2, « par analogie avec la clause d'ordre d'AMD-043 REQ-SCN-068 ») et tranche les quatre cas : grille et ombres pixel **avant** courbure ; occlusion et reflets **après** (profondeur et couleur rendues) ; test de sol et boîte englobante de `REQ-SCN-084` **avant** courbure. Le sens choisi est cohérent avec `REQ-SCN-068` (« la courbure n'intervient qu'au placement final des sommets »), vérifié. **Mais** le critère censé rendre la clause discriminante est faux : **N-1 (bloquant)**. |
| **55-8** | `REQ-SCN-058` non amendée, place des passes, objets transparents | **partiellement levé** | L'ordre complet est écrit et `REQ-SCN-058` est **réellement amendée** (§3 point 3, citation exacte, « Le reste de l'exigence … est inchangé ») ✓ ; l'ordre **reflets → occlusion** est tranché, donc « le reflet reflète une scène non encore assombrie » est déterminé ✓ ; la question « le HUD est-il reflété ? » est tranchée par « s'ils sont déjà dessinés au moment de la passe » ✓. **Mais** la clause sur les objets transparents contredit l'ordre que le même amendement vient de fixer : **N-2 (bloquant)**. |
| **55-9** | espace colorimétrique du produit et du mélange | **levé** | `REQ-SCN-083` : « en espace linéaire, avant l'encodage sRGB de REQ-SCN-028 » ✓. `REQ-SCN-084`, intensité : même phrase ✓. Conséquence tirée jusqu'au bout : le plafond « 40 % » et le critère « +90 niveaux sRGB » — les deux énoncés que la lecture faisait basculer — ont **disparu** au profit de grandeurs linéaires (`O ≤ 0,1 × ao.strength` ; « le canal rouge **linéaire** augmente d'au moins 10 % »), et tous les critères de §2.3 sont exprimés en **rapports de valeurs linéaires**. Le piège de la v1.0 est fermé. |
| **55-10** | `REQ-SCN-083` sous-déterminée **et** gain incompatible avec son propre critère | **levé** (a, b, c libres par décision ; d corrigé ; **e corrigé**) | (e) : le gain `0,78 × ao.strength` et le plafond « 40 % » sont **retirés**, la Note le dit et renvoie au point 55-10. Recalcul : `O ≤ 0,1 × ao.strength` donne 0,1/0,2/0,3/0,4/0,5 ✓, « rapport ≥ 0,70 » au contact aux défauts ✓, « ≥ 0,50 » à 5 ✓, « ≥ 0,90 » à 1 ✓ ; la proportionnalité à `ao.strength` et le plafond sont le **même** énoncé (`O = 0,1·strength·g`, `g ≤ 1`), donc **aucune fenêtre impossible** ne subsiste et la monotonie exigée en découle. (a)(b)(c) restent libres : point non retenu, raison jugée au §3. (d) : voir 55-9. |
| **55-11** | `REQ-SCN-084` sous-déterminée sur cinq points | **levé** (a, b, e corrigés ; c, d libres par décision) | (a) *Définition du sol* : « face dont la normale, dans le repère de scène **avant courbure**, a une composante +Y ≥ 0,9, appartenant à une **instance** dont la boîte englobante dans le plan `xz` … mesure au moins `6 × échelle de scène` **dans ses deux dimensions** », avec deux exemples chiffrés (`D` = 8 réfléchit, `D` = 3 ou `scale.z` = 0,5 → 4 unités ne réfléchit pas) et un invariant discriminant ✓ — les quatre indéterminations (quel objet, quelle étendue, quel repère, avant/après courbure) sont closes. (b) *Flou* : « croît **linéairement** … de 0 au point d'origine jusqu'à `0,003 × (10 − n(ssr.clarity))` de la hauteur d'écran à la longueur maximale » ; le `/5` parasite est supprimé ✓. Recalculé : 0,027 / 0,021 / 0,015 / 0,009 / 0,003, défaut 0,015 = **16,2 px à 1080p** ✓. (e) linéaire ✓. (c)(d) libres : §3. |
| **55-12** | `spl.voxel_grid` : indice pris pour `N` | **levé** | Table : « 0 Off, 1, 2, 3 — côté de cellule `N = 2^spl.voxel_grid` voxels (1 → 2, 2 → 4, 3 → 8) » ; `REQ-SCN-081` : même formule ; critères : « `spl.voxel_grid` = 2 (donc `N` = 4) », « écrit à 4 → 3 ». Plus aucune confusion possible entre la valeur du réglage et le côté de cellule. |
| **55-13** | critères « automatiques » non exécutables | **levé** | (a) tout est exprimé en **rapport de deux captures** (grille On / Off) en valeurs linéaires ✓. (b) le **« gros plan G »** nomme tout : harnais, mode 3D, 1920 × 1080, `video.anti_alias` = 0, `video.res_scale` = 0, échelle 1, courbure nulle, TV Box = None, skybox noire, texture au format direct RVB (188,188,188), lumière (1, −1, 1) intensité 1, caméra à 30 unités sur la normale, FOV 70° ✓. (c) recalculé : hauteur visible `2 × 30 × tan 35° = 42,0` unités → **25,71 px/unité**, rainure `2 × 0,035 × 4 = 0,28` unité → **7,2 px** ✓ (> 5 px demandés) ; et la Note porte l'aveu demandé sur le cadrage par défaut (189,65 → **1,14 px**, recalculé à partir de `REQ-SCN-004` : hauteur visible 265,6 unités) ✓. (d) `video.anti_alias` = 0 explicite, comme `AMD-050` ✓. Coquille résiduelle : N-7. |
| **55-14** | critère de `REQ-SCN-082` non discriminant, alignement manquant | **levé** | La clause discriminante demandée est là, et commentée : « avec `spl.pixel_shadow` = 0 (et en Classic) : au moins une cellule de chaque bord de l'ombre contient à la fois des pixels ombrés et des pixels éclairés. **C'est cette dernière clause qui rend le critère discriminant** ». L'alignement est fixé (« exactement celui de REQ-SCN-081 … avec `N` = 1 voxel ») et le biais de `REQ-SCN-031` est appliqué **au point déplacé** ✓. Arithmétique recalculée entièrement : lumière (1, −3, 1), `t = 29/3` → centre d'ombre à (+9,667 ; +9,667), bords à **5,667** et **13,667** (au milieu d'un voxel ✓), cellules ombrées de centre dans [5,667 ; 13,667] → frontières aux coordonnées entières **6** et **14** ✓ ; niveaux : albédo 0,5029, `N·L = 0,9045`, ambiante 0,1812, facteur d'ombre 0,2 (`REQ-SCN-031`) → éclairé 0,5460 lin → sRGB **195,0** ✓, ombré 0,1821 lin → sRGB **118,3** ✓. Les deux vecteurs annoncés (195 ± 3, 118 ± 3) sont justes. |
| **55-15** | douze réglages persistés sans clé stable ni ligne `SPEC-09` | **levé** | §4 donne les **treize** lignes au format d'`AMD-043` §6, clés `render.style.*`, type, défaut, **domaine avec bornes et pas**, portée U, effet imm., UI, libretro « — » ; la forme de `REQ-PERS-080` est vérifiée dans le texte ; `REQ-PERS-081` (liste finie ordonnée avec libellés) et `REQ-PERS-084` (métadonnée d'effet) sont traitées explicitement ; §3 points 7 et 8 font entrer les lignes dans `SPEC-09` §4.1.1, la liste de §4.6 et `SPEC-06` §4.1 ; la validation `REQ-PERS-003` du fichier (`1` → 4, hors domaine → 0) est dans le domaine de `render.visual_style` **et** dans un critère ✓. Lignes libretro non retenues : raison jugée au §3 (recevable). Résidus de forme : N-8. |
| **55-16** | UI contre `REQ-UI-061` (P1), textes manquants | **levé** | §3 point 6 amende `REQ-UI-061` **nommément**, y compris la phrase de son critère, place la ligne `Style` **après `Skybox`** (vérifié : c'est bien la 11ᵉ ligne de la table, avant `Full Screen`), nomme le groupe (« Style », repliable) et marque l'addition **(D)**. Le §4 fournit **tous** les textes manquants : groupe en FR, `Classic`/`Enhanced` et leur aide, `Off`/`MSVO`/`Depth`, `Low`/`Medium`/`High`/`Ultra`, et l'affichage de `spl.voxel_grid` (« Every 2 voxels » / « Tous les 2 voxels ») ; les deux qualités portent des libellés **distincts** (`AO Quality` / `Reflection Quality`), avec la phrase qui le justifie ✓. Les règles de visibilité (`spl.ao` ≠ 0, `spl.reflections` = 1) sont ajoutées — d'où une retouche à faire sur la phrase du critère : N-4. |
| **55-17** | amendement d'`AMD-043` annoncé sans texte ; courbure en Classic | **levé** | **Lecture A** retenue et écrite deux fois : `REQ-SCN-078` (« La courbure … n'est **pas** un effet du style : elle agit en Classic comme en Enhanced **(D)**, de sorte qu'un changement de style ne fasse pas perdre un réglage de courbure ») et §3 point 4, qui réécrit la phrase de `REQ-SCN-066` et la colonne Lua d'`AMD-043` §6 (citations exactes vérifiées). Le tiret de critère demandé est là : « `Setting.VisualStyle = 0` avec Courbure gauche-droite à 60° : la scène reste courbée » ✓. Compatibilité ascendante vérifiée contre les critères d'`AMD-040` : `Set(0, …)` → faux reste vrai (Classic n'a toujours aucun paramètre), `Set(1, "spl.bend_zx", 500)` → vrai et `Get` → 180 restent valides, `Set(7, …)` → faux reste valide ✓. |
| **55-18** | `AMD-040` `REQ-LUA-114` et sa note non amendés | **levé** | §3 point 2 réécrit la ligne `VisualStyle` (les deux moitiés fausses sont corrigées) et **restreint la note aux quatre paramètres de courbure**, avec la raison (« huit des douze paramètres de REQ-SCN-079 ont un défaut non nul » — recompté : `ao.strength`, `ao.radius`, `ao.thickness`, `ao.quality`, `ssr.shine`, `ssr.clarity`, `ssr.length`, `ssr.quality` = **8** ✓). Le critère `VisualStyle = 5` → 0 est déclaré valide, et il l'est ✓. |
| **55-19** | mauvaise exigence pour l'éclairage du HUD | **levé** | « tiles : REQ-SCN-028 ; backdrop : REQ-SCN-075 ; HUD : REQ-SCN-029 pour l'éclairage et REQ-SCN-044 pour le rendu » — correction appliquée mot pour mot. |
| **55-20** | portée : tunnel, boîtier TV, rectangle du mode 2D, renvois | **levé** | Les quatre listes d'exclusion sont complètes et **homogènes** (`REQ-SCN-081`, `-082`, `-083`, `-084`), le tunnel et les modèles TV y sont avec le renvoi `REQ-SCN-053`/`-054` et la raison (« qui les veut non éclairés, sans ombre » — vérifié dans `REQ-SCN-053` : « non éclairés et sans ombre » ✓), le rectangle du mode 2D y est **avec son renvoi `AMD-046` §2** à chaque mention ✓, et le cas des tiles transparentes est tranché (« Elle s'applique aux tiles opaques **et** transparentes »). La phrase « toutes ces surfaces … occultent » est en revanche fausse pour deux d'entre elles : N-2. |
| **55-21** | `ao.thickness` sans effet observable | **levé** | Phrase dédiée (« **sans aucun effet observable** … présenté grisé **(D)** »), renvoi à `REQ-PERS-017`, mention dans la table des paramètres, aide « No effect with Depth occlusion. », ligne §4 « imm. (sans effet, REQ-PERS-017) … grisé », et le critère demandé : « changer `ao.thickness` de 1 à 5 … la capture est identique au pixel près » ✓. Réserve de forme : `REQ-PERS-017` est une **liste fermée** de trois réglages, non amendée (N-8). |
| **55-22** | cible de rendu, MSAA, demi-résolution | **levé** | Les trois phrases demandées sont au dernier tiret de §2.2 : cible intermédiaire autorisée (PEUT) avec « celle-ci n'est jamais relue (REQ-SCN-057) » ; travail sur couleur et profondeur **résolues** (« après résolution du MSAA éventuel … AMD-032 §2 » — vérifié : `AMD-032` §2 donne bien 1 = MSAA 4× et résout « avant que quoi que ce soit n'échantillonne la texture ») ; « moitié de chaque dimension de la **cible de rendu courante**, elle-même éventuellement réduite par `video.res_scale` (REQ-SCN-063) » ✓. |
| **55-23** | « survit à un redémarrage » contre `REQ-PERS-010` | **levé** | Les deux critères demandés sont là : écriture « depuis l'UI ou depuis le modèle de réglages » → survit au redémarrage ; écriture **par Lua** → couche de session de `REQ-PERS-010`, avec le scénario « pendant une partie, puis fin de partie → retour à la préférence » ✓. |
| **55-24** | clause de priorités absente, changements documentaires incomplets | **levé** | Ligne « Priorités : » complète, avec les replis `(T-1)` pour `REQ-SCN-081` (phrase d'`AMD-049`) et pour l'ordre de frame si aucune des deux passes d'écran n'est retenue ✓. §3 points 8 et 9 : placement `SPEC-06` **§3.11bis** après `REQ-SCN-043` (vérifié : §3.11 s'achève bien sur `REQ-SCN-043`), treize lignes en §4.1, index mis à jour ✓. §5 ne dit plus « Aucune » à tort : les deux choix du propriétaire y sont **nommés comme tranchés** et les points non retenus y sont listés avec leur raison — c'est mieux que ce que le rapport demandait. Index incomplet de deux lignes : N-8. |

**Récapitulatif** : 22 levés, 2 partiellement levés (55-7, 55-8, par le fait de N-1 et N-2),
0 non levé. Les 14 bloquants de la v1.0 sont traités au fond, pas en surface.

---

## 3. Jugement des points « non retenus »

Le §5 de la v1.1 déclare quatre points non retenus (en partie) et donne sa raison. Je les juge
**tous recevables**, pour des raisons différentes.

1. **La famille de loi d'occultation (55-10 point 1) — raison recevable.** Le propriétaire a décidé
   de ne pas fixer l'algorithme des deux effets d'écran et en a tiré **toutes** les conséquences :
   passage en **P3**, énoncé en « intention + paramètres + invariants », aspect jugé en catégorie
   **revue** de `SPEC-12`, déclaration explicite au §1 que « le rendu exact » est laissé à
   l'implémenteur, et surtout **retrait du gain et du plafond qui rendaient la fenêtre impossible**.
   C'est cohérent : le défaut signalé en 55-10 n'était pas l'absence de loi *en soi*, mais le fait
   qu'une loi non écrite était **contrainte** par deux critères incompatibles. Supprimer la
   contrainte vaut autant qu'écrire la loi, et coûte moins au round 2 : les deux effets étant à Off
   par défaut, `REQ-ACC-021` n'est pas affectée.
2. **La formule d'atténuation de bord (55-11 c) — raison recevable, avec un garde-fou.** Le texte ne
   donne ni largeur ni forme, mais impose « l'intensité **DOIT** décroître continûment vers 0 » et
   « **aucun bord franc de reflet ne DOIT être visible** », puis le fait juger par revue. Un `DOIT`
   vérifié par revue est admissible depuis `SPEC-12` §2 et `REQ-ACC-050`. L'aspect, qui était le
   vrai enjeu du point, est donc couvert.
3. **Le repère de la marche (55-11 d) — raison recevable.** Conséquence directe du même choix ; le
   nombre de pas étant devenu un `DEVRAIT` non observable, fixer le repère de la marche n'aurait
   plus rien contraint d'observable.
4. **Les lignes libretro (55-15) — raison recevable et mieux motivée que la suggestion.** « Les
   réglages de style ne sont pas exposés en libretro » est un choix légitime : `REQ-PERS-083` prévoit
   la colonne « — », `SPEC-09` §4.6 a déjà une liste « Non exposés en libretro » que §3 point 7
   complète correctement, `REQ-PERS-085` n'impose rien pour les réglages U, et la v1.1 prend soin de
   maintenir `REQ-PERS-081` (listes de valeurs avec libellés) **même** pour les réglages non exposés.
   La suggestion de la v1.0 (lignes `zaxnes_…`) était la moins bonne des deux options.
5. **La reformulation du 55-6 — remplacée par mieux.** Jugée au tableau ci-dessus : la formulation
   par poids d'axe est supérieure à l'« intersection de plans » que je proposais.

Aucun point n'a été écarté sans raison, et aucune raison donnée n'est un expédient.

---

## 4. Points nouveaux

### N-1 — Le critère de courbure de `REQ-SCN-080` est arithmétiquement insatisfaisable. **BLOQUANT.**

Second critère de `REQ-SCN-080` : « gros plan G de REQ-SCN-081 élargi à la shape entière (caméra à
60 unités), `spl.voxel_grid` = 2, Courbure gauche-droite à 90° : **chaque** ligne de rainure de la
shape est courbée dans la capture — son écart maximal à la corde qui joint ses extrémités **dépasse
5 pixels** —, alors qu'à courbure nulle la même ligne est droite à moins de 1 pixel ».

C'est la pièce qui rend discriminante la clause d'ordre de 55-7 : l'intention est juste, le chiffrage
ne l'est pas. Recalcul complet avec `REQ-SCN-067` (`L` = 256, `s` = 1, θ = 90°) :

- `k = (π/2)/256 = 0,0061359` rad/unité, rayon `1/k = 162,97` unités ; `φ = θ/4 = 0,3927`,
  `f = φ/sin φ = 1,02617`, `h = 2 sin²(φ/2)/k = 12,405` ;
- shape de 4 × 4 tiles, donc `u ∈ [−16, 16]` : l'arc vu ne couvre que `32 k = 0,196` rad = **11,25°**
  des 90° de courbure. Flèche en profondeur entre le centre et le bord :
  `f · (1 − cos(16k))/k = 1,02617 × 0,78493 = 0,806` unité ;
- focale en pixels à 1080p et FOV 70° : `(1080/2)/tan 35° = 771,2` px.

**Premier défaut, fatal : les rainures verticales restent exactement droites.** L'enroulement de
`REQ-SCN-067` agit sur le couple (`u`, `w`) et **laisse `v` inchangé** pour une courbure
gauche-droite. Une ligne de rainure verticale est à `u` constant : après courbure elle est à `u′` et
`w′` constants, donc c'est un **segment droit** dans la scène, qui se projette en un segment droit.
Son écart à sa corde est nul, à courbure 90° comme à courbure nulle. Le quantificateur « **chaque**
ligne de rainure » ne peut donc jamais être satisfait, par construction, quelle que soit la scène.

**Second défaut : même pour les rainures horizontales, le seuil n'est pas atteint au cadrage
imposé.** Écart à la corde `= f_px · v · (1/z_centre − 1/z_bord)` :

| Rainure horizontale | Caméra à 60 unités (cadrage du critère) | Caméra à 30 unités (gros plan G) |
|---|---|---|
| `v = ±16` (bord de la shape) | **2,73 px** | 10,76 px |
| `v = ±12` | 2,05 px | 8,07 px |
| `v = 0` (par le centre) | **0 px** | **0 px** |

Au cadrage demandé, la ligne la plus déviante atteint **2,7 px**, soit un peu plus de la moitié du
seuil, et la rainure qui passe par le centre de l'image ne dévie pas du tout, à aucune distance :
une implémentation parfaitement conforme à la clause d'ordre **échoue au critère**. Et le critère est
en outre non reproductible : le « gros plan G » ne fixe pas la **position de la shape dans la scène**,
alors que l'angle vu dépend de `u · k`, donc de cette position (une shape centrée sur `u = 112` voit
sa flèche devenir en partie latérale et l'écart monter vers 6 px).

*Corrections proposées*, dans l'ordre de préférence :

1. mesurer sur une surface qui couvre une bonne part du cadre de référence — la face avant du
   **backdrop** (256 × 240, `Backdrop.Enable`, qui porte désormais la grille par §3 point 5) ou une
   shape de 32 × 30 tiles : la flèche passe à `f · (1 − cos(128k))/k = 48,98` unités, soit des
   dizaines de pixels d'écart, et le critère devient franc ;
2. ou garder la shape de 32 unités en **fixant son centre sur `C`**, en revenant à la caméra à
   **30 unités** et en restreignant le quantificateur : « les lignes de rainure **horizontales**
   situées à au moins 12 unités du centre de la face s'écartent de leur corde de plus de 5 pixels ;
   les rainures verticales restent droites » (8,1 px calculés à `v = 12`, marge suffisante) ;
3. dans les deux cas, écrire la position de la shape dans la scène dans le « gros plan G », puisque
   deux critères en dépendent désormais.

### N-2 — Les objets transparents, la skybox et le HUD ne peuvent pas « occulter » dans l'ordre que l'amendement fixe. **BLOQUANT.**

Trois endroits de la v1.1 disent la même chose, et elle est incompatible avec l'ordre de frame que
le même amendement établit :

- §2.2, 4ᵉ tiret : « les **objets transparents** (REQ-SCN-040) … contribuent à la profondeur
  (passage de profondeur préalable de REQ-SCN-040), donc ils **occultent** pour l'occlusion ambiante
  et **peuvent être reflétés** » ;
- §3 point 3 : « Les objets transparents contribuent à la profondeur, donc ils occultent et peuvent
  être reflétés … » ;
- `REQ-SCN-083`, *Portée* : « Toutes ces surfaces contribuent en revanche à la profondeur, donc elles
  **occultent** » — la liste comprenant la skybox, le HUD, le tunnel, les modèles TV, le rectangle du
  mode 2D et les objets transparents.

Or l'ordre fixé par §2.2 et §3 point 3 est : « objets opaques ; **reflets** ; **occlusion** ; objets
transparents triés ; HUD ; UI ». Le passage de profondeur des objets transparents **fait partie de la
passe transparente** (`REQ-SCN-040` : « d'abord un passage qui n'écrit que la profondeur, puis un
passage de couleur mélangé », et `AMD-012` l'a rendu **par objet entier**) : il a donc lieu **après**
les deux passes d'écran. Au moment où l'occlusion est calculée, aucune profondeur d'objet transparent
n'est écrite ; au moment où les reflets sont tracés, aucun objet transparent n'est dessiné. Même
constat pour le **HUD**, dessiné deux passes plus tard, et pour la **skybox**, qui est à distance
infinie et derrière toute la scène (`REQ-SCN-049`) — rien ne se trouve à moins de `2 r` d'elle.

La portée de `REQ-SCN-084` dit l'inverse, et c'est **elle qui est juste** : « Tous peuvent en
revanche **être reflétés s'ils sont déjà dessinés au moment de la passe** ». L'amendement se
contredit donc sur un observable (y a-t-il de l'occlusion ambiante au pied d'une tile transparente,
et le reflet du sol montre-t-il les objets transparents ?) — exactement la question que 55-8
demandait de trancher.

Avancer le passage de profondeur des transparents avant les deux passes **n'est pas** une solution
neutre : une profondeur transparente écrite globalement en amont ferait rejeter, au passage de
couleur, les fragments situés derrière un autre objet transparent, ce qui casse `AMD-021` R2-012
(« les transparents derrière un transparent restent visibles »). La correction simple est donc
l'autre :

*Correction proposée* : aligner les trois endroits sur la lecture de `REQ-SCN-084` — « les objets
transparents ne sont pas encore dessinés quand les deux passes d'écran ont lieu : ils **n'occultent
pas**, ne sont **pas reflétés**, et ne sont ni assombris ni porteurs de reflets ; le HUD, dessiné
après, non plus ; la skybox, à distance infinie, n'occulte jamais. Le tunnel, les modèles de
téléviseur et le rectangle du mode 2D, dessinés avec les objets opaques, **occultent** et peuvent
être reflétés sans être modifiés. » Une phrase de plus réglerait au passage un cas voisin non traité :
les tiles de style « **ombre seule** » (`REQ-SCN-041`, non dessinées) n'écrivent pas de profondeur et
n'occultent donc pas.

### N-3 — Les invariants « auto » de §2.4 et §2.5 n'ont pas de configuration, et deux d'entre eux ne sont pas satisfaisables. *Non bloquant, mais à corriger avant la mise en œuvre.*

§2.3 a le « gros plan G », nommé, chiffré, exécutable par un tiers. §2.4 et §2.5 n'ont rien
d'équivalent : « un cube posé sur un plan, vu de trois quarts », « un cube rouge posé sur un plan de
sol gris de 32 × 32 unités » — sans albédo, sans lumière, sans caméra, sans résolution, sans
`video.anti_alias`, et sans dire que les autres effets sont à Off (ce qui est vrai par défaut, mais
deux invariants affirment l'identité à une capture Classic, ce qui est faux si la grille est active).
La plupart des invariants sont des **rapports** On/Off et survivent à cette imprécision. Deux n'y
survivent pas :

- **`REQ-SCN-084`, premier invariant** : « le canal rouge **linéaire** augmente d'au moins **10 %** ».
  Le mélange étant `(1 − i)·surface + i·reflet` avec `i = 0,45`, le canal rouge du sol **baisse** si
  le rouge réfléchi est inférieur à son propre niveau : la condition s'écrit
  `0,55·R_sol + 0,45·R_reflet ≥ 1,10·R_sol`, soit `R_reflet ≥ 1,222 · R_sol`. Avec un sol gris
  d'albédo linéaire 0,5 éclairé comme le gros plan G (`R_sol = 0,381`), il faut
  `R_reflet ≥ 0,466`, soit un cube d'albédo rouge **linéaire ≥ 0,61** — ce qu'un « rouge » de palette
  NES ne garantit pas. Deux testeurs de bonne foi obtiendront deux verdicts opposés sur la même
  implémentation.
- **`REQ-SCN-084`, troisième invariant** : « un bloc isolé de 8 × 8 unités, lui, **est modifié** ».
  Un bloc isolé ne reflète que ce qui l'entoure ; si la scène ne contient rien d'autre, le rayon sort
  de l'écran ou atteint sa longueur maximale, l'intensité est atténuée « vers 0 » et aucun pixel n'est
  modifié. L'invariant exige donc implicitement un objet à refléter, qu'il ne nomme pas.
- Accessoirement, `REQ-SCN-083` demande « à `2 r` du cube, il vaut 1,00 ± 0,02 », soit **40 unités**
  aux réglages par défaut : il faut un plan d'au moins 80 unités de côté, alors que l'invariant
  voisin de §2.5 travaille sur 32 × 32.

*Correction proposée* : définir un « **gros plan A** » (occlusion) et un « **gros plan R** »
(reflets) sur le modèle du gros plan G — résolution, `video.anti_alias` = 0, albédos **linéaires**
donnés, lumière et caméra nommées, plan de 96 × 96 unités pour A, et pour R un sol gris (albédo
linéaire 0,2 suffit : la condition devient `R_reflet ≥ 0,186`, largement tenue) avec un cube d'albédo
rouge pur et un objet à refléter — puis ajouter « tous les autres effets de §2.3 à §2.5 sont à Off »
en tête des deux séries d'invariants.

### N-4 — Le critère amendé de `REQ-UI-061` ignore les règles de visibilité qu'`AMD-055` vient d'introduire. *Non bloquant.*

§3 point 6 fait devenir le critère de `REQ-UI-061` (P1) : « Les douze libellés sont présents, plus la
ligne `Style` et, quand `Style` = `Enhanced`, **les douze paramètres de SPEC-06 REQ-SCN-079** ; aucun
autre paramètre n'apparaît. » Mais `REQ-SCN-079` ajoute, dans la même v1.1, que « les réglages
d'occlusion ne sont visibles que si `spl.ao` ≠ 0, ceux des reflets que si `spl.reflections` = 1 » —
et les deux valent **0 par défaut**. Sur une installation neuve, passer en Enhanced n'affiche donc
que **4** des douze paramètres (occlusion, ombres pixel, grille, reflets) : le critère tel qu'il est
réécrit **échoue** sur une implémentation conforme. Le §3 point 6 le sait (« avec … les règles de
visibilité de REQ-SCN-079 ») mais la phrase du critère ne le dit pas.

*Correction* : « … les douze paramètres de SPEC-06 REQ-SCN-079 **apparaissent selon ses règles de
visibilité** (quatre avec les valeurs par défaut, les douze avec `spl.ao` ≠ 0 et
`spl.reflections` = 1) ; aucun autre paramètre n'apparaît. »

### N-5 — `REQ-SCN-082` ne dit rien de `MARCHING`, et sa règle d'axes peut ne sélectionner aucun axe. *Non bloquant.*

`REQ-SCN-082` s'applique « sur les faces des maillages de **tiles opaques** », sans exclure
`render.mesh_algo = 1`. Son pas 2 ne ramène au centre du voxel que « les axes dont `|n_a| < 0,5` ».
Pour une face `GREEDY` (normale `±X`, `±Y`, `±Z`, `REQ-E3D-505`) cela donne toujours exactement deux
axes, et le résultat est celui qui est décrit. Sur une surface `MARCHING`, la normale est quelconque :
on peut avoir trois axes ramenés (normale proche de l'une des diagonales des faces) ou **aucun**
(normale proche de `(0,577 ; 0,577 ; 0,577)`, où les trois `|n_a|` valent 0,577 ≥ 0,5) — et dans ce
dernier cas l'effet **disparaît** sur ces fragments, par plaques. `REQ-SCN-081` a pris soin de traiter
le cas `MARCHING` dans ses Notes ; `REQ-SCN-082` devrait faire de même.

*Correction* : soit « en `MARCHING`, les ombres pixel ne s'appliquent pas » (le plus simple, et
cohérent avec le fait que l'effet imite la grille de pixels d'un décor cubique), soit « les deux axes
de plus petit `|n_a|` sont ramenés au centre du voxel » (règle qui sélectionne toujours deux axes et
qui coïncide avec la règle actuelle sur toute face `GREEDY`).

### N-6 — Le seuil « alignée sur un axe » à 0,9 produit une marche visible sur les surfaces `MARCHING`. *Non bloquant.*

`REQ-SCN-081` bascule `k` de 0,62 à 0,38 et `e` de 0,10 à 0,20 selon que `max_a |n_a|` est au-dessus
ou au-dessous de **0,9**. Sur une surface lisse de `MARCHING`, la normale varie continûment : le long
du contour où `max_a |n_a| = 0,9`, le facteur de rainure saute de `1 − 0,4 × 0,38 = 0,848` à
`1 − 0,4 × 0,62 = 0,752`, soit une **marche de 10 %** d'albédo le long d'une ligne de la surface,
alors que le reste de la formule est continu (le poids `w_a` est conçu pour l'être). C'est un
artefact observable, et il est contradictoire avec le critère de revue de `REQ-SCN-083` qui proscrit
les bandes — ici sur un autre effet.

*Correction* : interpoler les deux constantes sur un intervalle, par exemple
`t = clamp((max_a |n_a| − 0,8)/0,1, 0, 1)`, `k = 0,38 + 0,24 t`, `e = 0,20 − 0,10 t` (les valeurs
mesurées restent atteintes pour une face alignée et pour une face à 45°) — ou, à défaut, l'écrire
dans les Notes comme un écart accepté.
*Même famille, plus petit* : la bande `0,035 < d_a < 0,04`, où `f_a = 1`, produit une discontinuité de
+10 % à `d_a = 0,04` que la clause de lissage (« les deux transitions, en `d_a = 0,035` et
`d_a = 0,16` ») ne couvre pas. Faire commencer l'arête claire à `d_a = 0,035` ou lisser **trois**
transitions.

### N-7 — Coquilles du « gros plan G ». *Non bloquant.*

- « texture personnalisée au **format direct** de couleur RVB (188, 188, 188), soit un albédo
  linéaire de **0,500 ± 0,002** » : recalculé avec la courbe sRGB de `REQ-SCN-035` (celle qui donne
  bien 0,1812 pour sRGB 118, `REQ-SCN-027`, vérifié), sRGB 188 vaut **0,5029** en linéaire ; sRGB 187
  vaut 0,4969. Aucun niveau 8 bits ne tombe dans 0,500 ± 0,002. **Les vecteurs sRGB du critère ne
  changent pas** (je les ai recalculés avec 0,5029 : 166,0 / 133,5 / 169,7 / 106,9 — tous dans leur
  tolérance de ±3), seul le libellé de l'albédo est faux. *Correction* : « soit un albédo linéaire de
  **0,503 ± 0,002** ».
- « Points donnés dans le repère local de **la tile centrale** » : une shape de 4 × 4 tiles n'a pas
  de tile centrale (son centre est une jonction de quatre tiles). Les quatre tiles candidates sont
  équivalentes ici (même texture, même repère local), donc la mesure est la même ; la phrase doit
  néanmoins nommer la tile, par exemple « la tile adjacente au centre de la face, côté `+x`, `+y` ».

### N-8 — Résidus documentaires. *Non bloquant.*

- **Colonnes du §4.** `SPEC-09` §4.1 définit la colonne **UI** comme « présence dans le panneau de
  réglages desktop **de l'original** (libellé observé), « — » = absent », et §3 point 6 d'`AMD-055`
  dit justement que « le programme d'origine les place ailleurs dans son interface, que SPEC-10 ne
  décrit pas ». Les treize lignes du §4 remplissent pourtant cette colonne avec le placement **de
  Zaxnes**. `AMD-046` a rencontré le même cas et a trouvé la bonne forme, à reprendre : « — dans
  l'original (réglage propre à Zaxnes) ; dans Zaxnes : … ». Le tableau omet par ailleurs la dernière
  colonne de `SPEC-09` §4.1.1 (« Comportement »), qui accueille d'habitude le renvoi à l'exigence.
- **`REQ-PERS-017`** est une **énumération fermée** de trois réglages (`scene.map_enable`,
  `video.anti_alias`, `input.emulated_menu_button`). `render.style.ao_thickness` devient le
  quatrième : soit §3 ajoute un point « `REQ-PERS-017` reçoit `render.style.ao_thickness` », soit le
  texte de `REQ-SCN-079` cite l'exigence **par analogie** (« comme les réglages de REQ-PERS-017 »)
  plutôt qu'en disant « C'est le cas prévu par SPEC-09 REQ-PERS-017 ».
- **Index.** §3 point 9 marque `REQ-SCN-058` et `REQ-UI-061` comme amendées, mais pas
  `REQ-SCN-075` (`AMD-049`), `REQ-SCN-066` (`AMD-043`) ni `REQ-LUA-114` (`AMD-040`), qui le sont
  aussi — la table de l'index est tenue par exigence.
- **En-tête de table d'`AMD-043` `REQ-SCN-066`** : la colonne « Id Lua (`AMD-040` §3, **style 1**) »
  reste à « style 1 » alors que §3 point 1 renomme le §3 d'`AMD-040` en « Paramètres du style
  Enhanced (identifiant 4, 1 accepté) ». Un mot à ajouter au §3 point 4.
- **Deux précisions de renvoi**, sans incidence : les positions locales des surfaces `MARCHING` sont
  données par `REQ-E3D-522` (« centre de voxel − (3,5 ; 3,5 ; (D−1)/2) ») et non par `REQ-E3D-504`
  que `REQ-SCN-081`/`-082` citent seul ; j'ai vérifié que **les deux repères coïncident** (le centre
  du voxel `(i, j, k)` de `REQ-E3D-504` est bien `(i−3,5 ; j−3,5 ; k−(D−1)/2)`), donc le texte est
  juste, mais citer les deux épargnerait la vérification au lecteur. De même, le pas 3 de
  `REQ-SCN-082` énumère « échelle, rotation, décalage, déformation animée, puis courbure » alors que
  `REQ-E3D-568` ajoute la déformation `δ` **avant** l'échelle et la rotation ; la phrase
  « transformer ce point **comme un sommet** » suffit à lever l'ambiguïté, l'énumération gagnerait à
  suivre l'ordre de `REQ-E3D-406`.

---

## 5. Ce qui est juste dans la v1.1 et doit être conservé tel quel

- Le **basculement de `REQ-SCN-083`/`-084` en « intention + paramètres + invariants », P3, aspect en
  revue** est la bonne réponse au point le plus lourd du rapport précédent, et il est fait
  proprement : §1 l'annonce et le justifie, les priorités le portent, `SPEC-12` fournit la catégorie,
  les Notes disent ce qui est retiré et **pourquoi** (« ils n'avaient pas de solution commune
  (55-10) »). Un amendement qui documente sa propre correction est exactement ce qu'il faut à la
  matrice de traçabilité.
- La **formule de modulation de `REQ-SCN-081`** : lisible, continue en `n`, vérifiable point par
  point, et accompagnée d'un tableau de quatre vecteurs **et** de la clause « en Classic, et en
  Enhanced avec `spl.voxel_grid` = 0, les quatre points valent 166 ± 3 : le critère est donc
  discriminant ». C'est le modèle à suivre pour §2.4 et §2.5 (N-3).
- Le **critère de `REQ-SCN-082`**, dont j'ai refait toute la géométrie et les deux niveaux : il est
  exécutable, discriminant, et son commentaire explique en une phrase pourquoi il l'est.
- Les **Notes de `REQ-SCN-081`** : raccord des rainures d'une tile à la suivante (vérifié : les
  centres de tiles sont distants de 8, multiple de 2, 4 et 8), coin de cellule plus sombre
  (0,62² = 0,384, vérifié), lisibilité limitée à la vue rapprochée (1,14 px au cadrage par défaut,
  vérifié), comportement en `MARCHING`. Ces quatre notes répondent par avance aux questions d'un
  implémenteur.
- Le **§4** (treize lignes + libellés EN/FR complets) et le **§5** (choix tranchés, points non
  retenus avec leur raison) : les deux sections que la v1.0 n'avait pas, et elles sont au niveau des
  amendements les mieux tenus de `spec/changes/`.
- La **compatibilité ascendante** de `REQ-SCN-078` (accepter 4 et 1, relire 4) est préservée jusque
  dans les critères déjà écrits d'`AMD-040`, que j'ai rejoués un par un : aucun ne devient faux.

---

## 6. Conclusion — gate R2

- **Filtre anti-fuite : franchi.** Aucune fuite, aucune réserve. Les trois réserves faibles de la
  v1.0 (55-1, 55-2, 55-3) sont levées à la source, et le texte nouveau — formule de modulation,
  trois pas des ombres pixel, tables de niveaux, §4 — reste dans les catégories autorisées par le
  `SPEC-GUIDE` (comportement observable, faits d'interopérabilité, descriptions mathématiques d'un
  résultat, constantes justifiées).
- **Numérotation et références : franchi.** `REQ-SCN-078` à `-084` libres et contigus,
  `REQ-SCN-077` toujours retirée, priorités cohérentes avec les énoncés, 45 renvois vérifiés un à un,
  cinq phrases citées pour remplacement retrouvées mot pour mot dans leurs fichiers d'origine. Deux
  précisions de confort seulement (N-8).
- **Arithmétique : franchi.** Tout ce que la v1.1 chiffre est juste — rapports 0,62 / 1,00 / 1,05 /
  0,384, sRGB 133 / 166 / 169 / 107, 195 / 118, frontières d'ombre à 6 et 14, rainure de 7,2 px,
  1,14 px au cadrage par défaut, rayons 8 / 14 / 20 / 26 / 32, longueurs 40 / 120 / 200 / 280 / 360,
  intensités 0,09 / 0,27 / 0,45 / 0,63 / 0,81, flous 0,027 / 0,021 / 0,015 / 0,009 / 0,003 et
  16,2 px à 1080p, plafonds 0,1 à 0,5 — à **une** coquille de libellé près (N-7, albédo 0,503 et non
  0,500) et à **une** exception : le seul chiffrage **faux** est le seuil de 5 pixels du critère de
  courbure (N-1).
- **Fond : non franchi, de peu.** 22 des 24 points levés, 2 partiellement, les quatre points non
  retenus justifiés. Restent **2 points bloquants nouveaux**, tous deux conséquences des
  corrections : **N-1** (le critère qui rend discriminante la clause d'ordre de la courbure est
  insatisfaisable : rainures verticales droites par construction, 2,7 px au lieu de 5 pour la plus
  déviante des horizontales, position de la shape non fixée) et **N-2** (l'amendement affirme et nie
  tour à tour que les objets transparents occultent et sont reflétés ; la skybox et le HUD sont dans
  la même phrase à tort).
- **Réponse à la question du propriétaire, actualisée : oui, pour ce que la v1.1 promet.**
  `REQ-SCN-083` et `REQ-SCN-084` ne visent plus un résultat comparable d'une implémentation à
  l'autre — le propriétaire l'a tranché, l'a écrit et en a tiré les conséquences (P3, revue,
  invariants, retrait du gain incompatible). Pour ce contrat-là, un implémenteur peut travailler
  **sans deviner** : rien d'observable ne manque, et rien d'observable ne se contredit. La seule
  réserve est que leurs invariants automatiques ne sont pas encore exécutables par un tiers faute de
  configuration nommée (N-3).
- **Est-il prêt pour la mise en œuvre ?** **Pas encore, mais une v1.2 de quelques lignes suffit.**
  `REQ-SCN-078`, `-079`, `-081` et `-082`, le §3 et le §4 sont **prêts tels quels** : un lot
  d'implémentation pourrait démarrer sur eux sans attendre. Ce qui bloque est circonscrit à deux
  clauses : le second critère de `REQ-SCN-080` (N-1) et le paragraphe sur les objets transparents,
  répété en trois endroits (N-2, §2.2 4ᵉ tiret, §3 point 3, portée de `REQ-SCN-083`). Ni l'un ni
  l'autre ne rouvre une décision du propriétaire : N-1 est un choix de scène et de seuil, N-2 est le
  choix de la rédaction déjà juste (celle de `REQ-SCN-084`).
- **Recommandation** : v1.2 traitant **N-1** et **N-2**, puis **N-3** et **N-4** (qui touchent des
  critères, donc la testabilité, et dont la correction est mécanique), puis N-5 à N-8 au fil de la
  plume. Une troisième relecture R2 n'aurait à vérifier que N-1 (nouveau chiffrage, donc nouveau
  recalcul) et les critères touchés par N-3 ; le reste du fichier est acquis.

Le fichier `AMD-055-style-enhanced.md` n'a pas été modifié par cette relecture, ni aucun autre
fichier de `spec/` hors ce rapport et la ligne de journal ; rien n'a été commité.
