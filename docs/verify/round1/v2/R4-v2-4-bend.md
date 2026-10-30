# Rapport de vérification — round 1, V2, lot v2-4-bend

**Rôle** : R4, vérificateur indépendant, côté propre
**Date** : 2026-10-30
**Branche vérifiée** : `delta/v2-4-bend`, commit `a2aa280` (« V2-4: space bending and its settings (AMD-043) »)
**Textes normatifs** : `spec/changes/AMD-043-courbure-espace.md` **v1.2** du 2026-10-30 et, dans
`spec/changes/AMD-040-api-lua-3dsen-2.md` **v1.2**, les lignes `VisualStyle`,
`GetVisualStyleParameter`, `SetVisualStyleParameter`, `ResetVisualStyleParameters` de
REQ-LUA-114 et tout le §3 (paramètres du style 1, normalisation). Relectures R2 lues pour le
raisonnement seulement : `R2-review-AMD-039-045.md` (43-1 à 43-16) et
`R2-review-AMD-039-045-v1.1.md` (N-43-1 à N-43-4).
**Tests écrits pour cette vérification** : `impl/tests/Acceptance/R4V2BendTests.cs`,
**28 méthodes, 73 cas**, tous exécutés (3 d'entre elles rendent par OpenGL). Les tests de
l'implémenteur (`tests/Acceptance/Amd043Tests.cs`, `tests/Zax.Engine3D.Tests/SpaceBendTests.cs`,
`tests/Zax.Render.Tests/BendTests.cs`, `tests/Zax.App.Tests/OptionsWindowTests.cs`,
`tests/Zax.Settings.Tests/*`) n'ont été ni modifiés ni pris comme référence : les treize vecteurs,
les nombres et les libellés viennent du texte des amendements.
**Méthode** : les treize vecteurs de REQ-SCN-067 ont d'abord été **recalculés indépendamment** à
partir du §3 de l'amendement (feuille de calcul du vérificateur) : les treize lignes de la table
sont reproduites à moins de 0,005 unité, et la valeur `B((0, 8, 0)) = (0,32 ; −17,70 ; 0)` du
critère de REQ-SCN-068 aussi. La table de l'amendement est donc cohérente avec sa propre formule,
ce qui permet de distinguer un écart d'implémentation d'un vecteur erroné. Tout est ensuite
observé de l'extérieur : sommets exportés par le harnais de scène, fichier `settings.json` sur le
disque, résultats Lua, modèle de la fenêtre d'options lu par réflexion sur l'assemblage livré,
état du Zapper, statistiques et captures du rendu OpenGL, et la commande
`Zax.Harness render --bend`.

---

## 1. Décompte par verdict

| Verdict | Lignes |
|---|---|
| **conforme** | 6 |
| **non conforme** | 0 |
| **non testable** | 0 (2 critères `[ROM]` non exécutables ici, voir §4) |
| **ambigu** | 0 |
| **Total** | **6** |

---

## 2. Table de conformité

| REQ-ID | Verdict | Preuve |
|---|---|---|
| **REQ-SCN-066** — réglages de courbure (4 réglages, portée U, persistés, pas de 30°, à chaud, aussi par `Get/SetVisualStyleParameter(1, …)`, groupe « Géométrie » avancé) | **conforme** | `R4V2BendTests.REQ_SCN_066_the_four_settings_are_the_lines_of_section_6` (4 clés, `SettingKind.Int`, défaut 0, `SettingScope.User`, `SettingEffect.Immediate`, échelle −180…180 de 13 barreaux, 13 / 2 valeurs libretro) ; `…_a_value_off_the_ladder_is_normalised` (14 cas, **45 → 60** du critère) ; `…_the_curvature_survives_a_restart` (écrit par `AppSettings`, relu par un `SettingsStore` neuf : `"scene.bend_lr": 60`, `"scene.arc_center": 1` dans `settings.json`) ; `…_the_settings_apply_hot_in_the_running_host` (`HostDriver`, écriture sur le modèle → `Scene.Settings.Bend` immédiatement, et retour à l'identité) ; `…_the_settings_are_the_style_parameters_of_REQ_LUA_114` (aller-retour interface ↔ Lua sur les 4 identifiants, et l'effet est le même pour `Setting.VisualStyle` = 0 et = 1) ; `…_the_options_window_shows_a_geometry_group` (groupe `options.group.geometry`, marqueur « avancé », libellés et aides du §6 en anglais **et** en français). Commande : `dotnet test tests/Acceptance -c Release --filter R4V2BendTests`. |
| **REQ-SCN-067** — transformation des points `B(P)` | **conforme** | `…_the_thirteen_vectors_of_the_amendment` : **13 cas**, les treize lignes de la table à ±0,05 unité (dont la ligne « bornage actif » (−180, 0, 0) sur `P = (256, 120, −200)` → `(132,53 ; 120 ; −64,00)`). `…_the_settings_of_the_scene_drive_the_very_same_transform` : la même transformation obtenue par les quatre réglages du rendu. `…_is_the_identity_at_zero_and_continuous_in_the_angle` : identité à θ = 0 sur quatre points, 0,01° et 0,5° déplacent presque rien sur chacun des trois axes, 30° et 29,99° donnent la même image à 0,5 unité (le seuil de l'implémentation est 10⁻⁴ rad, la borne que l'amendement autorise). `…_a_positive_angle_bulges_and_a_negative_one_wraps` : z croît aux bords pour θ > 0, décroît pour θ < 0. `…_the_depth_is_held_at_ninety_five_percent_of_the_radius` : au-delà de 0,95/k l'image ne bouge plus, **dans les deux sens de courbure**. |
| **REQ-SCN-068** — portée de la courbure | **conforme** | `…_the_tiles_are_bent_and_the_hud_is_not` (courbure gauche-droite 90° : le sommet de la tile en `(256, 120, 0)` est à `B(P)` à 10⁻² près et s'est déplacé de plus de 10 unités ; tous les sommets convertis sont à l'image de leur position non courbée ; le HUD exporté est **identique** à celui d'une courbure nulle, positions et normales) ; `…_the_hud_and_the_skybox_stay_flat` (rendu OpenGL, frame HUD + skybox dégradé : image **pixel pour pixel identique** avec les trois axes à 180 / −90 / 120 et centre au bord) ; `…_the_shadows_follow_the_bent_surface` (rendu OpenGL, ombre isolée en rendant deux fois la même frame ombres allumées puis éteintes : sous un arc de 90° l'ombre se déplace dans la même direction que son projecteur et d'une amplitude comparable, ce qu'une passe d'ombres non courbée ne donnerait pas) ; `…_the_backdrop_bends_and_is_subdivided` (≥ 6 × 33 × 33 sommets, courbé, jamais découpé par R) ; `…_the_clipping_is_evaluated_before_bending` (arc 60° au bord, marges par défaut : `B((0, 8, 0)) = (0,32 ; −17,70 ; 0)`, la tile du coin reste exportée, une partie est dans R **avant** courbure, et toute son image courbée est hors de R) ; `…_the_camera_target_is_evaluated_before_bending` (P\*, œil, cible et matrice vue-projection inchangés pour quatre courbures) ; `…_the_fpv_occlusion_is_evaluated_before_bending` (rendu OpenGL, FPV : `Statistics.Occluders` identique pour trois courbures) ; `…_the_curvature_comes_after_the_animated_deformation` (instance déformée : la position avant courbure est `M · (v + δ)` et la position dessinée est `B(M · (v + δ))`, donc la courbure vient après REQ-SCN-043 et après la transformation propre de l'objet). Harnais : `Zax.Harness render --frames synthetic:clip --bend 90,0,0,0` → `10 colours  84 opaque  84 casters` contre `5 colours  84 opaque  84 casters` sans courbure (la géométrie et l'ombrage changent, l'ensemble des tiles gardées et des projecteurs ne change pas). |
| **REQ-SCN-028 amendée** — normale transportée par la courbure | **conforme** | `…_the_shading_normal_is_transported_by_the_curvature` : à 90° la normale du sommet `(256, 120, 0)` reste unitaire, n'est plus la normale du maillage (produit scalaire < 0,95 avec elle), et est **perpendiculaire aux images de deux tangentes indépendantes** de la face (différences centrées à travers `B`, cosinus < 10⁻²) — c'est-à-dire la normale de la surface courbée. La parenthèse d'origine reste vraie pour la déformation animée : sans courbure, les normales exportées d'une instance déformée sont **exactement** celles de la même frame construite sans aucune déformation (`…_the_curvature_comes_after_the_animated_deformation`). |
| **REQ-INP-124** — Zapper à la souris sur une scène courbée | **conforme** | `…_a_ray_through_the_image_aims_within_half_a_unit` : **11 cas**, les onze vecteurs de REQ-SCN-067 sans bornage, visée à moins de 0,5 unité de `P` avec z₀ = z de `P` (le critère d'acceptation de l'amendement). `…_the_aim_is_within_half_a_unit_over_the_whole_window` : **11 cas**, une grille de 7 × 7 points couvrant toute la fenêtre de visée, pour onze courbures (les trois axes, les deux centres d'arc et trois combinaisons) ; les échantillons où le bornage de profondeur de REQ-SCN-067 s'applique sont écartés, la visée n'y étant pas spécifiée (AMD-043 §4) — la condition `w · k > 0,95` est recalculée dans le test à partir du §3 de l'amendement, pas demandée à l'implémentation. `…_the_aiming_window_is_the_image_widened_by_sixteen_units` : `|x − 128| ≤ 144 · s`, `|y − 128| ≤ 136 · s`, bornes incluses, à l'échelle près. `…_off_screen_and_no_candidate_keep_the_rules_of_REQ_INP_064` : un candidat hors fenêtre est transmis tel quel (pixel NES négatif, tir hors écran) ; un rayon sans aucun candidat laisse la dernière position valide en place. `…_the_pixel_is_the_one_of_REQ_INP_063` : les coordonnées NES sont bien celles de REQ-INP-063 (à un pixel près sur trois points), et sans courbure la visée est exactement celle de REQ-INP-063. |
| **REQ-LUA-114** (partie `VisualStyle`) — `VisualStyle`, `GetVisualStyleParameter`, `SetVisualStyleParameter`, `ResetVisualStyleParameters`, normalisation AMD-040 §3 | **conforme** | `…_the_visual_style_and_its_parameters` : défaut 0, `= 1` → 1, `= 5` → 0, `= -1` → 0 ; `SetVisualStyleParameter(1, "spl.bend_zx", 500)` → vrai puis lecture 180 ; 40 → 30, **45 → 60**, **15 → 0** (égalité à l'entier **pair** : 7,5 barreaux → 8, 6,5 → 6), −500 → −180 ; style 0 et style 7 inconnus → faux, identifiant `"inconnu"` → faux et lecture 0 ; `ResetVisualStyleParameters(1)` remet les quatre paramètres à 0, `(7)` ne fait rien et ne lève rien ; `…_a_value_off_the_ladder_is_normalised` ajoute 14 cas de normalisation dont 75 → 60 et 105 → 120, qu'un arrondi « au plus loin de zéro » ne donnerait pas. `…_the_visual_style_is_persisted_and_changes_no_drawing` : `render.visual_style` est une préférence U de défaut 0, survit au redémarrage, n'est pas exposée à libretro et n'apparaît pas dans la fenêtre d'options ; `RenderSettings` n'a aucun membre de style, donc aucun dessin ne peut en dépendre. |

---

## 3. Non-conformités

**Aucune.** Les 73 cas écrits à partir du texte des deux amendements passent tous.

---

## 4. Clauses partielles et remarques

1. **Critères `[ROM]` non exécutables ici.** REQ-SCN-068 (« Courbure gauche-droite à 90°, *Super
   Mario Bros.* : arc continu, sans fente ni ombre décollée, score plat et lisible ») et
   REQ-INP-124 (« *Duck Hunt* à la souris : viser un canard à moins de 32 unités du bord droit le
   touche ») demandent des ROMs commerciales et une observation visuelle ; `tests/roms/` ne
   contient que des ROMs de test publiques. Les critères **auto** des deux exigences, eux, sont
   vérifiés. Les verdicts ci-dessus portent donc sur les critères auto et sur le texte normatif,
   pas sur ces deux observations.
2. **Force de la preuve sur la passe d'ombres.** Mon test montre que l'ombre **suit** la géométrie
   courbée (même direction, amplitude comparable), ce qui écarte le défaut le plus probable — une
   passe d'ombres restée plate. Il ne mesure pas la position géométrique exacte de l'ombre ; une
   vérification à la ROM (critère `[ROM]` ci-dessus) resterait utile. À noter que le test de
   l'implémenteur sur ce point (`BendTests.REQ_SCN_068_the_shadow_pass_follows_the_bent_surface`)
   n'affirme que « la passe tourne encore et il reste des projecteurs » : il ne détecterait pas
   une passe d'ombres non courbée. C'est une faiblesse de test, pas une non-conformité.
3. **`SceneScale` et la courbure.** `k = θ / (L · s)` et le pivot `−120 · s` dépendent de
   l'échelle de scène. L'hôte ne pousse `SceneScale` vers la scène que lorsqu'un des quatre
   réglages de courbure change (`EmulationHost.PublishBending`). Sans effet observable : sur
   desktop `s` vaut 1 et une écriture de `Video.SceneScale` n'a aucun effet géométrique
   (SPEC-05 REQ-LUA-095, SPEC-06 REQ-SCN-002). Simple remarque, aucune exigence du lot n'est en
   cause.
4. **Subdivision.** REQ-SCN-068 ne chiffre la subdivision que pour le fond (≥ 32 × 32 cellules par
   face) ; c'est ce que fait le fond, et aussi le quad du mode 2D (voir §5). Les maillages des
   tiles convertis font 8 unités de côté et n'entrent pas dans « les surfaces planes assez grandes
   pour se courber visiblement ».
5. **Boîtier TV.** La troisième exclusion de REQ-SCN-068 est sans objet : AMD-031 a retiré le
   boîtier TV du programme.
6. **Processus.** `tests/Acceptance/Zax.Acceptance.Tests.csproj` porte dans sa `Description` la
   mention « Reserved for the round-1 verifier (role R4) … Implementers must not add tests here ».
   Le commit de l'implémenteur y a ajouté `Amd043Tests.cs` (500 lignes). Je ne l'ai ni lu comme
   référence avant d'écrire mes propres tests, ni modifié ; je le signale parce que la convention
   du projet est enfreinte, pas parce que la conformité au cahier des charges en souffre.

---

## 5. Ticket Q ouvert du lot

**Q-079 — « REQ-SCN-068 ne dit pas si le quad du mode 2D est courbé »**
Choix provisoire retenu par l'implémenteur : **lecture 1** — le quad unique de 256 × 240 du mode
2D est courbé comme le reste de la scène de jeu et subdivisé en 32 × 32 cellules.

**Avis : le choix provisoire est conforme au texte de l'amendement.** Trois raisons.

1. La première phrase de REQ-SCN-068 est l'exigence : « la courbure DOIT s'appliquer à la
   géométrie de la scène de jeu ». En mode 2D ce quad **est** la géométrie de la scène de jeu : il
   remplace les tiles convertis, il est posé sur le plan NES du cadre de référence (REQ-E3D-410)
   et il porte l'image du jeu. L'énumération qui suit (« tiles et sprites convertis … et fond »)
   décrit le cas ordinaire, celui où un profil existe.
2. La liste des **exclusions** est close et explicite, et le §5 de l'amendement le dit en propres
   termes : « La courbure du HUD, de la skybox et du boîtier TV est tranchée en REQ-SCN-068. »
   Laisser le quad plat ajouterait une quatrième exclusion que l'amendement ne nomme pas, alors
   qu'il a pris la peine de fermer la liste.
3. La subdivision de 32 × 32 satisfait le « DEVRAIENT être subdivisées » de REQ-SCN-068 : le quad
   est exactement une « surface plane assez grande pour se courber visiblement », et le chiffre
   retenu est celui que l'amendement fixe pour l'autre grande surface plane de la scène, le fond.
   Rien dans le texte ne demande davantage.

Vérifié : `R4V2BendTests.REQ_SCN_068_the_two_dimensional_quad_follows_the_provisional_choice_of_Q079`
— en mode 2D, un seul objet exporté, courbé, de 33 × 33 sommets (donc 2 × 32 × 32 = 2 048
triangles, le chiffre annoncé par le ticket), et sa courbure n'est pas un simple basculement de
plan. Le ticket reste légitimement ouvert : l'amendement ne nomme pas le quad, et seul le
propriétaire peut ratifier la lecture. Mais la question ne cache aucune non-conformité.

---

## 6. Exécutions

Toutes en **Release**, au premier plan, une suite à la fois.

| Commande | Résultat |
|---|---|
| `dotnet test tests/Acceptance -c Release --filter "FullyQualifiedName~R4V2BendTests"` | **73 / 73** réussis |
| `dotnet test tests/Acceptance -c Release` (suite complète) | **1477** réussis, 15 ignorés, 1492 au total, 3 min 39 s |
| `dotnet test tests/Zax.Engine3D.Tests -c Release` | **217 / 217** réussis |
| `dotnet test tests/Zax.Render.Tests -c Release` | **155 / 155** réussis |
| `dotnet test tests/Zax.Settings.Tests -c Release` | **267 / 267** réussis |
| `dotnet test tests/Zax.Scripting.Tests -c Release` | **51 / 51** réussis |
| `dotnet test tests/Zax.App.Tests -c Release` | **404 / 404** réussis |
| `Zax.Harness render --frames synthetic:clip --size 192x144 --bend 90,0,0,0` | `clip  10 colours  84 opaque  0 alpha  0 HUD  84 casters` |
| `Zax.Harness render --frames synthetic:clip --size 192x144 --bend 0,0,0` | `clip   5 colours  84 opaque  0 alpha  0 HUD  84 casters` |

Les 15 tests ignorés de la suite d'acceptation sont ceux qui demandent les ROMs de référence ou
les campagnes de charge (`Requires=OriginalAssets`, `Q062StressTests`, `FinalRomCaptureTests`) ;
aucun ne concerne ce lot.

---

## 7. Dossiers consultés

`spec/` (`changes/AMD-043-courbure-espace.md`, `changes/AMD-040-api-lua-3dsen-2.md`,
`changes/Q-079.md`, `changes/R2-review-AMD-039-045.md`,
`changes/R2-review-AMD-039-045-v1.1.md`, `SPEC-05-lua.md`), `impl/` (sources du commit `a2aa280`,
`tests/`, `tools/Zax.Harness`), `verify/round1/v2/` (rapports des lots voisins, pour la forme),
`PLAN.md`. Aucun accès réseau. Ni le programme d'origine, ni ses binaires, ni les sources
décompilées, ni les outils du côté sale n'ont été consultés.
