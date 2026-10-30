# Rapport de vérification — round 1, V2, lot v2-2-lua-camera

**Rôle** : R4, vérificateur indépendant, côté propre
**Date** : 2026-10-30
**Branche vérifiée** : `delta/v2-2-lua-camera`, commit `786f2d6` (« V2-2: 3dSen 2.0 Lua info
table, reset distance, camera changes (AMD-040, AMD-045) »)
**Textes normatifs** : `spec/changes/AMD-040-api-lua-3dsen-2.md` v1.2 du 2026-10-30
(REQ-LUA-112, 113, 115 ; REQ-LUA-114 et les lignes SPEC-09 du §3 qui en dépendent sont hors
périmètre) et `spec/changes/AMD-045-camera-3dsen-2.md` v1.2 du 2026-10-30 (REQ-SCN-069 à 073,
REQ-SCN-007 étape 4 et REQ-SCN-045 amendées). Les deux copies déposées par l'implémenteur dans
`impl/docs/spec/changes/` sont identiques aux originales au caractère près (seules les fins de
ligne diffèrent, LF contre CRLF). Relectures R2 lues pour le raisonnement seulement :
`R2-review-AMD-039-045.md` et `R2-review-AMD-039-045-v1.1.md`.
**Tests écrits pour cette vérification** : `impl/tests/Acceptance/R4V2LuaCameraTests.cs`,
31 méthodes, **38 cas**, aucun ignoré. Les tests de l'implémenteur
(`Amd040LuaTests.cs`, `SceneCameraTests.cs`, `CameraTests.cs`) n'ont pas été modifiés.
**Méthode** : les amendements ont d'abord été dépouillés en critères observables, puis
l'implémentation lue. Chaque preuve passe par ce qu'un observateur extérieur atteint : les
résultats Lua d'un runtime de profil (`LuaRun`), la caméra publique de `Zax.Render` pilotée dans
l'ordre de l'application (`CameraControls.Apply` puis `SceneCamera.Advance`, l'ordre de
`ZaxApplication.HandleCameraInput` / `RenderFrame`), la vue que lit le rendu (`BuildView`), la
section caméra d'une sauvegarde d'état et la boucle réelle de l'hôte (quick save / quick load
par `HostDriver`). Les vecteurs et les nombres sont ceux des amendements.

---

## 1. Décompte par verdict

| Verdict | Lignes |
|---|---|
| **conforme** | 10 |
| **non conforme** | 0 |
| **non testable** | 3 (clauses secondaires, détaillées en §3) |
| **ambigu** | 0 |
| **Total** | **10 exigences, dont 3 portant une clause non testable** |

Aucune non-conformité. Deux observations mineures en §5 (aucune n'est une non-conformité au
texte : elles portent sur un texte d'aide d'outil et sur la fidélité d'un substitut de test).

---

## 2. Tableau de conformité

Commande commune à toutes les preuves (Release, premier plan) :

```
dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release \
  --filter "FullyQualifiedName~R4V2LuaCameraTests"
→ Passed!  - Failed: 0, Passed: 38, Skipped: 0, Total: 38, Duration: 292 ms
```

| REQ-ID | Verdict | Preuve (test de `R4V2LuaCameraTests`) |
|---|---|---|
| **REQ-LUA-112** — table `CS` | **conforme** (critère [R2] non testable) | `R4_the_CS_info_table_carries_a_version_of_at_least_two_and_a_title` : `CS`, `CS.Nes3D`, `CS.Nes3D.Info` sont des tables ; `tonumber(CS.Nes3D.Info.Version:match('^%d+')) >= 2` → vrai (`Version` = `"2.0"`, chaîne) ; `CS.Nes3D.Info.Title ~= '3dSenMaker'` → vrai (`"Zaxnes"`, chaîne) ; `CS.Nes3D.Info.AppId`, `CS.Nes3D.Autre`, `CS.Autre` → `nil`, sans erreur (`run.Log.Errors` vide). `R4_a_write_into_the_CS_tables_changes_nothing` : six écritures (dont `CS.Nes3D = nil`) ne lèvent rien et ne changent ni la lecture suivante, ni ce qu'un second runtime lit. `R4_the_two_point_zero_code_path_of_a_profile_script_runs_without_error` : le chemin 2.0 du §2 (extrait illustratif de l'amendement) exécuté dans `Start` va jusqu'au bout, prend la branche « ≥ 2 » et cadre la caméra avec `Camera.ResetDistance` — aucune erreur Lua, donc aucun hook perdu (REQ-LUA-060 à 064) |
| **REQ-LUA-113** — `Camera.ResetDistance` | **conforme** | `R4_the_lua_reset_distance_is_the_framed_distance_of_the_current_fov` : le critère entier — FOV 70° → 189,65 (±0,05) ; après `Camera.Distance = 50`, `Camera.ResetDistance` inchangé à 189,65 ; `Setting.Fov = 45` → 320,60 ; `Setting.Fov = 95` → 121,69. `R4_the_lua_reset_distance_is_read_only` : accès L seul, l'écriture lève l'erreur de REQ-LUA-042. `R4_the_scene_camera_reset_distance_matches_the_formula_at_every_fov` : aux onze FOV de REQ-SCN-005 (45 à 95 par pas de 5) la caméra réelle donne `132,7964 / tan(FOV/2)` à 2·10⁻³. `R4_the_reset_distance_uses_the_bounded_fov_of_the_camera` : le FOV retenu est celui de la caméra (Q-077), voir §4 |
| **REQ-LUA-115** — export des maillages | **conforme** | `R4_the_export_mesh_methods_exist_and_do_nothing` : `type(pattern.BuildExportMeshes)` et `type(pattern.ReleaseExportMeshes)` → `"function"` ; appelées dans `BuildPattern` sur une instance de profil et hors hook sur une shape de frame, elles ne lèvent rien, le hook va jusqu'au bout (`hooks` = 1), `Offset`, `Scale` et `Alpha` de l'instance sont inchangés, et un argument surnuméraire est toléré |
| **REQ-SCN-069** — distance cadrée | **conforme** (clause de bornage non testable) | `R4_the_framed_distance_follows_the_new_formula` (théorie, 3 cas) : 70° → 189,65 ; 45° → 320,60 ; 95° → 121,69, à ±0,05, sur `SceneConstants.ResetDistance` **et** sur `SceneCamera.ResetDistance`. `R4_the_old_and_the_new_framed_distance_agree_only_at_seventy_degrees` : à 70° les deux formules coïncident à 10⁻³ (189,6529) ; ailleurs elles divergent comme la Note l'annonce (277,93 / 320,60 à 45° ; 150,25 / 121,69 à 95°). `R4_a_view_reset_writes_the_framed_distance_through_the_bound` : la réinitialisation écrit d₀ dans la consigne du **mode courant** (TPV 189,65, FPV laissé à 0) ; la consigne porte bien la borne [0, 512] de REQ-SCN-011 (600 → 512, −3 → 0) ; d₀ suit un changement de FOV sans réinitialisation |
| **REQ-SCN-045** (amendée) — plan HUD | **conforme** | `R4_the_hud_plane_distance_follows_the_new_formula` (théorie, 3 cas) : `d_HUD` = 109,42 à 70°, 184,96 à 45°, 70,20 à 95° (±0,05), sur la constante **et** sur `BuildView(...).HudDistance` ; le facteur 0,5 est inchangé (`HudScale`), l'élément au centre C est sur l'axe à `d_HUD` et un décalage de 40 unités donne 20 unités selon l'axe droite. `R4_the_hud_plane_distance_is_unchanged_at_seventy_degrees` : à 70° la valeur reste celle du texte gelé (109,415), l'ancienne formule dérive ailleurs |
| **REQ-SCN-070** — zoom minimal | **conforme** | `R4_a_user_zoom_in_never_goes_under_sixteen` : le critère entier — ZoomIn maintenu 5 s depuis la vue par défaut (189,65) → consigne 16 ; `Distance = 5` puis ZoomIn 1 s → 5 (aucun effet) ; puis ZoomOut 1 s → 75 (tolérance 2 %). `R4_a_user_zoom_in_stops_on_sixteen_and_spares_the_fpv_zero` : depuis 20 un zoom de 70 unités s'arrête **sur** 16 ; depuis 16 il n'avance plus ; la distance FPV de 0 au lancement reste 0. `R4_the_wheel_stops_at_sixteen_forward_and_is_free_backward` : la molette avant s'arrête à 16 et ne fait rien depuis 5 ; la molette arrière garde la seule borne haute de REQ-SCN-011 (512). `R4_a_script_may_still_post_a_distance_under_sixteen` : l'écriture Lua garde ses bornes [0, 512] — 5 → 5, 0 → 0, 1000 → 512 |
| **REQ-SCN-071** — angles dans un tour | **conforme** | `R4_thirteen_seconds_of_rotation_stay_inside_one_turn_without_a_jump` : RotDown maintenu 13 s (390°) en pas de 1/60 s — à **chaque** frame la consigne rx reste dans ]−180, 180], à la fin elle vaut 30,81 (critère : « ≈ 30 »), et la direction visée ne tourne jamais de plus de 2° en une frame (continuité). `R4_a_setpoint_angle_written_at_270_reads_minus_90_at_the_next_frame` : `Rot.y = 270` relu à la frame suivante → −90 ; l'angle affiché a pris le même tour (−360°) et l'image n'a pas sauté (< 2°). `R4_both_modes_and_the_three_angles_are_wrapped_into_the_half_open_interval` : rx, ry **et** rz des **deux** modes sont ramenés (400 → 40, −400 → −40, 900 → 180, 270 → −90, −270 → 90, 540 → 180) ; l'intervalle est bien semi-ouvert (180 reste 180, −180 devient 180) ; un pas de temps nul replie quand même, « avant chaque mise à jour ». `R4_the_user_can_still_orbit_without_limit` : 13 s de RotRight font parcourir plus de 360° à la caméra |
| **REQ-SCN-072** / **REQ-SCN-007 étape 4** (amendée) — verticale de la caméra | **conforme** | `R4_the_tpv_right_axis_is_across_the_camera_vertical` (théorie, 4 cas) : pour (0,0,30), (20,15,30), (45,0,−60) et (89,5;0;0), l'axe droite est orthogonal à `q·(0,1,0)` (produit scalaire < 10⁻³), la visée reste dirigée vers P* (REQ-SCN-008) et la base reste orthonormée. `R4_the_tpv_right_axis_is_no_longer_across_the_world_vertical` : à rz = 30° le produit `droite · Y_monde` vaut sin 30° = 0,5, alors que l'étape 4 gelée imposait 0. `R4_the_tpv_camera_crosses_the_zenith_smoothly` : premier critère — entre rx = 89,5° et rx = 90,5° les axes droite, haut et avant tournent de moins de 2°, là où la verticale du monde aurait retourné l'image de plus de 150° (180° mesuré sur la même règle de produit vectoriel). `R4_the_roll_is_visible_in_tpv` : second critère — à rx = ry = 0 et rz = 30°, le roulis mesuré (angle entre la verticale caméra et la verticale du monde projetée) vaut 30,0° (±0,2), 0° sans rz, et −30° tourne dans l'autre sens ; la visée ne bouge pas. `R4_tpv_and_the_local_camera_use_the_same_kind_of_vertical` : la caméra locale prend exactement `q·(0,1,0)` de son propre q, et la seule différence restante en TPV est la visée corrigée vers P* |
| **REQ-SCN-073** — caméra rechargée sans glissement | **conforme** (clause « snapshot de rollback » non testable) | `R4_a_reloaded_camera_shows_the_saved_view_on_the_first_picture` : premier critère — sauvegarde en vue par défaut, rotation de 90° (RotRight 3 s) et zoom, convergence du lissage, puis `Restore` : **sans aucun `Advance`**, distance, angles et cible affichés sont déjà les consignes rechargées et l'œil de `BuildView` est celui de la vue sauvegardée ; la frame suivante ne dérive pas (vitesses de lissage remises à zéro). `R4_the_host_quick_load_snaps_the_camera_onto_the_saved_setpoints` : le même trajet par la boucle réelle (`state.quick_save`, déplacement, `state.quick_load`, `Settle`) — la caméra que lit le rendu après le chargement est la sauvegardée, pas celle qui était à l'écran. `R4_the_transition_window_comes_back_as_a_bounded_remaining_time` : sauvegarde 1 s après un passage en FPV → fenêtre restaurée à 2 s ; 7 → 3, −1 → 0, 1,25 → 1,25 (bornage [0, 3] s d'AMD-042). `R4_after_a_load_the_remaining_window_still_smooths_the_target` : second critère — après le chargement, la cible déplacée de 100 unités est rejointe par le lissage (écart restant 40, ±5, après Tₛ = 0,4 s) et, la fenêtre épuisée, la cible est suivie dans la même frame. `R4_the_camera_section_of_a_savestate_round_trips_the_remaining_time` : la section caméra du fichier (`SaveStateFile.WriteCamera`/`ReadCamera`) transporte mode, consignes et temps restant |

---

## 3. Clauses non testables

Trois clauses secondaires n'ont pas de preuve possible dans ce lot. Aucune ne met en cause
l'implémentation ; elles sont signalées pour que R1 sache ce qui reste à couvrir.

1. **REQ-LUA-112, critère [R2]** — « le profil `punchout` de la 2.0 démarre sans erreur Lua et
   cadre la caméra comme dans 3dSen 2.0 ». Les 101 profils d'`assets-neutral/profiles` sont ceux
   de la 1.1.2 (AMD-040 §1 le dit explicitement) ; aucun profil 2.0 n'est présent dans
   `assets-neutral`. Le critère est donc **non testable** tant que le corpus 2.0 n'est pas
   extrait. Substitut retenu : l'extrait illustratif du §2 de l'amendement, exécuté dans `Start`
   d'un profil construit par le test
   (`R4_the_two_point_zero_code_path_of_a_profile_script_runs_without_error`). Il couvre le
   premier membre de la phrase (« démarre sans erreur Lua ») mais pas le second (« cadre la
   caméra comme dans 3dSen 2.0 »), qui demande le profil réel.

2. **REQ-SCN-069, parenthèse de bornage** — « d₀ dépasse 512 sous un FOV de 29,1°, atteignable
   par script ». Le FOV de la caméra reste borné à [45, 95] par REQ-SCN-005, qu'aucun amendement
   ne touche : d₀ ne dépasse jamais 320,60 et la borne [0, 512] ne mord pas. Elle est néanmoins
   **implémentée** (la réinitialisation passe par la consigne, qui applique REQ-SCN-011), ce que
   `R4_a_view_reset_writes_the_framed_distance_through_the_bound` vérifie directement sur la
   consigne. C'est l'objet de Q-077 (§4).

3. **REQ-SCN-073, clause « snapshot de rollback (AMD-042) »** — dans cet état de la branche, le
   rollback ne recharge **pas** la caméra : `EmulationHost.PerformRollback` ne restaure que le
   cœur (commentaire DEC-07-Q3, `src/Zax.App/Loop/EmulationHost.SaveStates.cs:241-249`), et
   `RollbackHistory` ne porte aucune section caméra. La clause est donc vide aujourd'hui, et la
   satisfaire n'est pas violable : rien n'est rechargé, donc rien ne glisse. AMD-042 est hors
   périmètre de ce lot ; **à vérifier dans le lot AMD-042** que le rollback passe bien par
   `SceneCamera.Restore`, seul point d'entrée qui aplatit le lissage (ce que le commentaire de
   `Restore` annonce déjà).

---

## 4. Ticket Q ouvert du lot — Q-077

**Question** : quel FOV alimente d₀ — le réglage `camera.fov`, borné [10 ; 150] pour une écriture
de script (SPEC-09 §4.1), ou la caméra, bornée [45 ; 95] (REQ-SCN-005) ?
**Choix provisoire de l'implémenteur** : celui de la **caméra**.

**Verdict : le choix provisoire est conforme au texte des amendements.** Raisons :

- REQ-LUA-113 définit `Camera.ResetDistance` comme « la distance cadrée d₀ que **donnerait** une
  réinitialisation de vue au FOV courant ». Une réinitialisation écrit d₀ calculée sur le FOV que
  la caméra applique réellement (`BuildView` bâtit la projection avec ce même `_fov`). Lire le
  réglage non borné donnerait au script un nombre qui ne décrit aucun cadrage, ce qui contredirait
  la définition elle-même ; le choix retenu la rend vraie au mot près.
  `R4_the_reset_distance_uses_the_bounded_fov_of_the_camera` le montre : à un réglage de 29° la
  caméra dessine à 45° et `ResetDistance` vaut 320,60.
- « avant le bornage de REQ-SCN-011 » porte bien sur la borne [0, 512] et non sur le FOV :
  `SceneCamera.ResetDistance` n'applique aucun écrêtage de distance, seule `ResetView` passe par
  la consigne. Cette partie de REQ-LUA-113 est donc respectée, et elle est vérifiable.
- L'écart ne concerne qu'une **parenthèse illustrative** de REQ-SCN-069 (« atteignable par
  script »), qui décrit 3dSen 2.0 ; élargir le bornage de la caméra à [10 ; 150] modifierait
  REQ-SCN-005, qu'aucun amendement n'amende. La question 1 du §3 de Q-077 devrait donc être
  tranchée dans le sens de sa question 2 : noter la parenthèse comme sans portée dans Zaxnes.
- Le détail numérique relevé par Q-077 est exact et confirmé par
  `R4_a_view_reset_writes_the_framed_distance_through_the_bound` : à 29,1° d₀ vaut 511,6 (**sous**
  512) ; le seuil réel est 29,07° (d₀ ≈ 512,2) et à 29° d₀ = 513,5. Le « 29,1° » de REQ-SCN-069
  est donc un arrondi par excès du seuil, à corriger si l'amendement est revu.

Aucun autre ticket Q n'est ouvert pour ce lot.

---

## 5. Observations mineures (hors non-conformité)

- **M-1 (mineur, documentation d'outil)** — `tools/Zax.Harness/RenderExport.cs:53` affiche encore
  dans son aide « `--distance <d>` Camera setpoint distance (default 104 / atan(FOV/2)) », c'est-
  à-dire l'ancienne formule que REQ-SCN-069 remplace. Le code, lui, est correct : il écrit
  `scene.Scene.Camera.Fov = fov`, ce qui déclenche la réinitialisation et donc la nouvelle
  formule. Seul le texte d'aide est périmé. Aucun effet sur le produit.
- **M-2 (mineur, fidélité du substitut de test)** — `DetachedScriptCamera.Fov`
  (`src/Zax.Scripting/ScriptEnvironment.cs:178`) n'est **pas** borné et alimente une seconde copie
  de la formule de REQ-SCN-069, `ScriptCamera.FramedDistance`. Sur ce substitut, un script qui
  écrit `Setting.Fov = 29` lirait `Camera.ResetDistance` ≈ 513,5, alors que l'application lit
  320,60 (la caméra y est bornée [45, 95], Q-077). Le produit n'emprunte jamais ce chemin —
  l'application passe par `SceneCameraBridge` → `SceneCamera.ResetDistance`, et le banc
  `Zax.Harness` par la caméra réelle — mais un test d'acceptation écrit contre le substitut peut
  « prouver » hors de [45, 95] un comportement que l'application n'a pas.
  `R4_the_reset_distance_uses_the_bounded_fov_of_the_camera` consigne les deux valeurs. Remède
  possible, au choix de R1 : borner `DetachedScriptCamera.Fov` comme la caméra, ou faire lire à
  `ScriptCamera.FramedDistance` la constante de `Zax.Render` au lieu d'en dupliquer le nombre.
- **Remarque de couverture** — les lignes SPEC-09 du §3 d'AMD-040 (`render.visual_style`,
  `VisualStyle`, `RunAheadFrames`, `GetVisualStyleParameter`, `SetVisualStyleParameter`,
  `ResetVisualStyleParameters`) dépendent toutes de REQ-LUA-114, hors périmètre de ce lot : rien
  n'y était à vérifier, et rien n'y a été implémenté.

---

## 6. Exécutions

Toutes les commandes ont été lancées au **premier plan**, en **Release**, une suite à la fois
(règle de `impl/CLAUDE.md`). Aucune campagne GPU prolongée.

| Commande | Résultat |
|---|---|
| `dotnet build -c Release` | `Build succeeded. 0 Warning(s) 0 Error(s)` |
| `dotnet test tests/Acceptance/… -c Release --filter "FullyQualifiedName~R4V2LuaCameraTests"` | `Failed: 0, Passed: 38, Skipped: 0, Total: 38, Duration: 292 ms` |
| `dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release` | `Failed: 0, Passed: 1426, Skipped: 15, Total: 1441, Duration: 3 m 43 s` — les 15 ignorés sont les tests de ROM utilisateur et de charge (`FinalRomTests`, `FinalRomCaptureTests`, `Q062StressTests`, `RomAndRemainderTests`), sans rapport avec le lot |
| `dotnet test tests/Zax.Render.Tests/… -c Release` | `Failed: 0, Passed: 155, Skipped: 0, Total: 155, Duration: 2 s` |
| `dotnet test tests/Zax.Scripting.Tests/… -c Release` | `Failed: 0, Passed: 51, Skipped: 0, Total: 51, Duration: 25 s` |
| `dotnet test tests/Zax.App.Tests/… -c Release` | `Failed: 0, Passed: 402, Skipped: 0, Total: 402, Duration: 7 s` |

Aucune régression : les quatre projets touchés par le commit passent entièrement.

---

## 7. Conclusion

Les trois exigences Lua et les cinq exigences de caméra du lot, ainsi que les deux exigences
amendées (REQ-SCN-007 étape 4 et REQ-SCN-045), sont **conformes** aux textes v1.2 d'AMD-040 et
d'AMD-045, critères d'acceptation automatiques inclus, aux tolérances que les amendements
indiquent. **Aucune non-conformité**, donc aucun blocage. Trois clauses secondaires restent non
testables dans ce lot (corpus 2.0 absent, bornage [0, 512] sans portée sous REQ-SCN-005,
rollback hors périmètre) et deux observations mineures sont laissées à R1. Le choix provisoire de
Q-077 est conforme au texte ; il mérite d'être figé par une note d'amendement plutôt que par un
élargissement de REQ-SCN-005.
