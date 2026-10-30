# Rapport de vérification — round 1, V2, lot v2-5-rollback-reset

**Rôle** : R4, vérificateur indépendant, côté propre
**Date** : 2026-10-30
**Branche vérifiée** : `delta/v2-5-rollback-reset`, commit `35a2649` (« V2-5: rollback restores
PData and camera, soft reset (AMD-042, AMD-044) »)
**Textes normatifs** : `spec/changes/AMD-042-rollback-lua-camera.md` v1.2 du 2026-10-30
(REQ-LOOP-069 à 071, REQ-SCN-024 amendée) et `spec/changes/AMD-044-reset-doux.md` v1.2 du
2026-10-30 (REQ-CORE-122, REQ-LOOP-072, REQ-INP-125, REQ-CORE-006 amendée). Les trois copies
déposées par l'implémenteur dans `impl/docs/spec/changes/` (AMD-042, AMD-044, Q-078) sont
identiques aux originales au caractère près, seules les fins de ligne diffèrent. Relectures R2
lues pour le raisonnement seulement : `R2-review-AMD-039-045.md` et
`R2-review-AMD-039-045-v1.1.md`.
**Hors périmètre** : le recadrage de la caméra à la première image (AMD-045 REQ-SCN-073) relève
du lot v2-2 et n'est pas compté ici ; la clause de REQ-LOOP-070 qui exige que la caméra
rétablie soit **affichée** directement à ses consignes est en revanche dans le périmètre, parce
que REQ-LOOP-070 l'énonce elle-même (voir §3).
**Tests écrits pour cette vérification** : `impl/tests/Acceptance/R4V2RollbackResetTests.cs`,
22 méthodes, **23 cas** (1 théorie à 2 cas), aucun ignoré. Les tests de l'implémenteur
(`RollbackTests.cs`, `SoftResetLoopTests.cs`, `SoftResetTests.cs`, `CpuResetRomTests.cs`,
`LoopTests.cs`, `CoreAccuracyTests.cs`, `InputCoreTests.cs`, `RomSuite.cs`) n'ont pas été
modifiés ni réutilisés : le protocole « shell » des ROM `cpu_reset` est réimplémenté dans mon
fichier à partir du `readme.txt` public de blargg.
**Méthode** : les deux amendements ont d'abord été dépouillés en critères observables, puis
l'implémentation lue. Chaque preuve passe par ce qu'un observateur extérieur atteint : la
boucle réelle de l'hôte (`HostDriver`, par réflexion sur l'assemblage livré), les 64 valeurs de
`PData` et des globales Lua d'un profil écrit par le test, la caméra publique de `Zax.Render`,
le cœur public `Zax.Nes` (registres, mémoires, ports), la barre de menus et l'écran des
contrôles d'AMD-010, la chaîne d'entrée complète (binding → échantillonnage → hotkey →
commande) et deux ROM de test publiques. Les vecteurs et les nombres sont ceux des amendements
(540 / 630 frames, 90°, 3 s, 3 resets, 0x80000).

---

## 1. Décompte par verdict

| Verdict | Lignes |
|---|---|
| **conforme** | 7 |
| **non conforme** | 1 (mineure, dépendance inter-lot) |
| **non testable** | 0 ligne entière (1 clause `[ROM]` de REQ-LOOP-072, détaillée en §4) |
| **ambigu** | 0 |
| **Total** | **8 exigences** |

Une seule non-conformité, sur une clause de REQ-LOOP-070 (l'affichage immédiat de la caméra
rétablie), résorbée par la fusion avec le lot v2-2 — voir §3.

---

## 2. Tableau de conformité

Commande commune à toutes les preuves (Release, premier plan) :

```
dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release \
  --filter "FullyQualifiedName~R4V2RollbackResetTests"
→ Failed!  - Failed: 1, Passed: 22, Skipped: 0, Total: 23, Duration: 7 s
```

| REQ-ID | Verdict | Preuve (tests de `R4V2RollbackResetTests`) |
|---|---|---|
| **REQ-LOOP-069** — contenu d'un snapshot | **conforme** | `REQ_LOOP_069_a_snapshot_carries_the_core_the_pdata_and_the_camera` : à 540 frames exactement (avant que le snapshot du compteur 540 ne soit pris), le test relève `PData[0]` et `SceneCamera.Capture()` ; une frame plus tard le snapshot est pris, puis `PData[1]` est forcé à −999 et la consigne d'angles à (45, 90, 0). Le snapshot relu (`RollbackHistory.TakeAt`) porte les **trois** éléments pris au même instant : bloc cœur non vide, bloc `PData` de 256 octets dont la 1ʳᵉ valeur (lue comme `GetPData(0)`) est **celle de la capture** et non −999, et `Camera` **égale** l'état relevé (égalité de valeur du `record struct`, mode + consignes des deux modes + temps restant) — donc ni la lecture tardive ni un hook `Save` n'interviennent. `REQ_LOOP_069_a_snapshot_holds_no_learned_conversion_state` : ce que le snapshot ne contient toujours pas — sur une ROM qui dessine (registre d'images de REQ-E3D-007 non vide), un rollback laisse `Engine.Images.Count` **inchangé** alors que le compteur du cœur recule de 630 à 540 |
| **REQ-SCN-024** (amendée) — temps restant et non échéance | **conforme** | `REQ_SCN_024_the_camera_state_records_the_remaining_transition_time` : fenêtre ouverte (3 s), 1 s écoulée, `Capture()` → `TransitionRemaining` = 2 s ; 5 s de plus → 0 s ; `Restore(state)` → **2 s** et valeur dans [0, 3] : une échéance absolue aurait rendu la fenêtre périmée. `REQ_SCN_024_a_savestate_round_trip_puts_the_remaining_time_back` : le même trajet par la boucle réelle et le fichier (`state.quick_save` 1 s après un passage en FPV, 5 s de lissage, `state.quick_load`) → fenêtre restaurée à 2 s (±0,01) et mode FPV. Le champ est bien le **seul** pris par le rollback (voir REQ-LOOP-069) |
| **REQ-LOOP-070** — restauration | **non conforme** (3 clauses sur 4 conformes ; sévérité **mineure**) | Conformes : `REQ_LOOP_070_a_rollback_restores_the_pdata_of_the_restored_snapshot` — critère entier, run-ahead = 0 (AMD-041 non implémenté sur cette branche, donc nul), profil de test qui écrit `PData[0] = frame` dans `Update`, NTSC, 630 frames (10,5 s) puis rollback → compteur du cœur **540** et `PData[0]` **revenu à la valeur de la capture du snapshot 540** (valeur supérieure à 600 avant le rollback — le `frame` de la dernière frame convertie, 629 — et valeur de la capture après, à ±10⁻⁴). `REQ_LOOP_070_a_rollback_calls_no_script_hook` — `saves`, `loads`, `ends` restent à 0 et `starts` à 1 (celui du lancement) de part et d'autre du rollback : ni `Save` ni `Load` ne sont appelés, les 64 valeurs sont réécrites directement. `REQ_LOOP_070_a_rollback_restores_the_camera_setpoints` — caméra tournée de 90° après la capture, rollback → consigne ry revenue à 0,81° (±10⁻³) et mode TPV. **Non conforme** : `REQ_LOOP_070_the_restored_camera_is_shown_straight_at_its_setpoints` — voir §3 |
| **REQ-LOOP-071** — vidage à la fin d'une partie | **conforme** | `REQ_LOOP_071_the_end_of_a_game_empties_the_history` : critère entier — 600 frames (10 s) → 10 snapshots (≥ 9 demandés), puis `emulation.stop` → **immédiatement** après la fin et **avant tout nouveau lancement**, `Rollback.Count` = 0, `IsEmpty` vrai, état Arrêté et jeu courant effacé. Note : le vidage de fin de partie existait déjà avant ce lot (`EmulationHost.Jobs.cs`, `StopCurrentGame(clearCurrentGame: true)`, présent dès `9a079f0`) ; l'exigence est satisfaite, l'implémenteur n'avait rien à ajouter |
| **REQ-CORE-122** — reset doux du cœur | **conforme** | `REQ_CORE_122_the_public_reset_roms_pass_under_a_soft_reset` (théorie, 2 cas) : critère d'acceptation — `cpu_reset/ram_after_reset.nes` et `cpu_reset/registers.nes` de blargg, pilotées par un runner écrit d'après leur `readme.txt` (signature `DE B0 61` en `$6001-$6003`, `$81` = appui demandé au moins 100 ms plus tard, un appui par demande), **passent** (code 0) sous reset doux. `REQ_CORE_122_a_rom_that_counts_its_resets_reads_three_after_three_presses` : complément du critère — ROM maison qui marque la RAM au démarrage et incrémente `$0010` à chaque entrée par le vecteur de reset → **3** après trois resets doux (et 0 après un reset dur, qui perd la RAM). `REQ_CORE_122_the_cpu_keeps_its_registers_but_for_I_PC_and_S` : I mis à 1, S 0xF0 → 0xED (−3) **sans écrire la pile** (les trois octets sous S sont intacts), PC rechargé depuis `$FFFC/$FFFD`, A/X/Y et C, N, D, Z, V conservés, aucune interruption en attente. `REQ_CORE_122_the_memories_survive_a_soft_reset` : RAM interne (`$0042`, `$0700`), RAM de sauvegarde (`$6123`, carte à pile) et RAM de CHR (`$0010` par `$2006`/`$2007`) conservées, là où le reset dur les efface. `REQ_CORE_122_the_ppu_and_the_apu_go_to_their_after_reset_state` : PPUCTRL et PPUMASK à 0, `$4015` à 0 (canaux coupés), PPUSTATUS, OAMADDR et la RAM de nametable inchangés. `REQ_CORE_122_the_controller_ports_are_cleared` : verrou et registres à décalage remis à zéro (la lecture de `$4016` qui rendait 1 rend 0), **sans effet** sur les boutons tenus, relus à la frame suivante. Le drapeau d'IRQ de frame et celui du DMC sont couverts par `apu_reset/irq_flag_cleared` (suite de l'implémenteur, exécutée : 6/6 ROM `apu_reset` passent sous reset doux) |
| **REQ-CORE-006** (amendée) — compteur de frames | **conforme** | `REQ_CORE_006_the_frame_counter_restarts_at_zero_on_a_soft_reset` : 0 au chargement, 100 après 100 frames, **0 après le reset doux**, 10 après 10 frames, 0 après le reset dur. `REQ_CORE_122_the_lua_frame_global_restarts_without_End_or_Start` : conséquence assumée — `frame` écrit dans `PData[0]` par `Update` dépasse 200 avant le reset doux et revient sous 20 après, **en cours de session**, sans `End` ni `Start` (`starts` = 1, `ends` = 0) |
| **REQ-LOOP-072** — commande | **conforme** (clause `[ROM]` non testable, §4) | `REQ_LOOP_072_a_soft_reset_clears_the_history_and_keeps_the_scripts` : critère « auto, harnais » entier — en jeu, reset doux → `Rollback.Count` = 0, `PData[0]` et une valeur témoin `PData[12] = 1234` **inchangées**, aucun hook `End`, `Start`, `Save` ni `Load`. `REQ_LOOP_072_a_soft_reset_lifts_the_pause_without_relaunching_the_profile` : pause levée (En pause → En cours), compteur du cœur à 0, horloge de cadence réinitialisée (après une seconde passée en pause, la frame rendue suivante n'exécute que 1 à 2 frames émulées au lieu du rattrapage de 5 de REQ-LOOP-012), ROM **non relue** (`RecentRoms` inchangé), état appris de la conversion conservé (`Engine.Images.Count` inchangé), aucun chargement déclenché. `REQ_LOOP_072_the_game_menu_offers_the_soft_reset_next_to_the_hard_one` : la ligne `emulation.soft_reset` est **immédiatement après** `emulation.reset` dans le menu de jeu, inactive hors partie et active en jeu ; libellés d'AMD-044 §3 lus dans les deux tables d'AMD-035 — `Soft Reset` / `Reset doux` — et le libellé affiché est bien celui de la langue en vigueur |
| **REQ-INP-125** — action d'entrée (P3) | **conforme** | `REQ_INP_125_the_soft_reset_is_an_extended_action_with_the_next_free_code` : action `SoftReset` au code **0x80000**, le suivant disponible (2 × `QuickLoad` = 0x40000), unique dans la table, absente de l'octet de manette, et **sans aucune association par défaut** — ni dans les défauts clavier, ni dans ceux de la manette standard, ni dans un `InputBindings` neuf (clavier J1, J2 et manette : `PhysicalControl.None`). `REQ_INP_125_a_bound_soft_reset_triggers_the_command_once_per_press` : critère entier — l'action figure dans l'écran des contrôles (ligne de cible `SoftReset`), une association posée à la main est rendue par l'échantillonnage réel du clavier (`DesktopInputSource`), et l'appui déclenche REQ-LOOP-072 dans la boucle (historique vidé, compteur du cœur revenu à 0) ; maintenu cinq frames de plus, il ne déclenche **qu'un** reset (le cœur continue de compter) |

---

## 3. La non-conformité

### REQ-LOOP-070, dernière phrase — la caméra rétablie n'est pas affichée directement à ses consignes

**Texte** : « La caméra rétablie DOIT être affichée **directement** à ses consignes, sans
lissage depuis la vue d'avant (AMD-045, REQ-SCN-073) », et le critère d'acceptation : « tourner
la caméra de 90° après la capture, rollback → la première image après la restauration montre
l'orientation du moment de la capture ».

**Observé** : les **consignes** reviennent bien (ligne REQ-LOOP-070 du tableau), mais la vue
**affichée** reste celle d'avant le rollback et rejoint les consignes par le lissage de
REQ-SCN-019. Preuve :

```
REQ_LOOP_070_the_restored_camera_is_shown_straight_at_its_setpoints
Assert.Equal() Failure: Values are not within 1 decimal place
Expected: 0.8 (rounded from 0.810000002)
Actual:   90.5 (rounded from 90.5226593)
```

L'angle ry **affiché** sur la première image après la restauration vaut 90,52° (il part de
90,81°, la vue d'avant, et dérive de 0,3° en une frame de 1/60 s) au lieu de 0,81°, l'orientation
du moment de la capture. En clair : au lieu d'être déjà revenu en arrière, le point de vue
glisse visiblement vers l'ancienne orientation pendant ≈ Tₛ = 0,4 s.

**Cause** : `SceneCamera.Restore` (`src/Zax.Render/SceneCamera.cs:252-261`) ne recale pas les
valeurs affichées — sa documentation le dit : « The displayed values are **not** snapped: they
join the restored setpoints through the smoothing of REQ-SCN-019 ». Le rollback
(`EmulationHost.SaveStates.cs:259`) emprunte ce chemin.

**Attendu** : la première image après la restauration montre l'orientation de la capture (ry
affiché = ry de consigne = 0,81°), les vitesses de lissage remises à zéro.

**Sévérité : mineure.** Ce n'est pas un défaut de conception, et il se résorbe sans toucher à ce
lot :

- l'implémenteur a délibérément fait passer la caméra du rollback par le **chemin unique**
  qu'emprunte aussi le chargement d'un savestate (commentaire de `PerformRollback`,
  `EmulationHost.SaveStates.cs:236-239`), ce qui est exactement ce que demandent REQ-LOOP-069
  (« REQ-SCN-024 … en est la référence unique ») et le renvoi de REQ-LOOP-070 vers REQ-SCN-073 ;
- le recalage manquant est précisément ce qu'ajoute REQ-SCN-073, hors périmètre ici mais
  **implémenté dans le lot frère** : sur `delta/v2-2-lua-camera`, `SceneCamera.Restore` se
  termine par `SnapToSetpoints()` (vérifié : `git show delta/v2-2-lua-camera:src/Zax.Render/SceneCamera.cs`,
  ligne 287), et le rapport R4 du lot v2-2 donne REQ-SCN-073 conforme ;
- la fusion des deux lots satisfait donc la clause sans aucune modification de ce lot. Le
  risque est de livrer v2-5 **sans** v2-2 : la clause de REQ-LOOP-070 est alors violée, et c'est
  pour cela qu'elle est comptée en non-conformité ici plutôt qu'en simple observation.

Rien d'autre dans REQ-LOOP-070 n'est en cause : le cœur, les 64 valeurs de `PData` (écrites dans
l'ordre « cœur, puis `PData`, puis caméra » du texte) et l'état de caméra sont restaurés, et
aucun hook `Save` ou `Load` n'est appelé.

---

## 4. Clause non testable

**REQ-LOOP-072, critère `[ROM]`** — « "Super Mario Bros." en cours de partie, reset doux →
retour à l'écran titre sans écran de chargement ». Aucune ROM commerciale n'est présente du côté
propre (SPEC-12 §2 : les jeux de référence sont fournis par l'utilisateur), le critère est donc
**non testable** ici. Ce qu'il mesure est néanmoins couvert par deux preuves indirectes :
`REQ_LOOP_072_a_soft_reset_lifts_the_pause_without_relaunching_the_profile` montre qu'aucun
chargement n'est déclenché (`IsLoading` faux, ROM non relue, état appris conservé), et les deux
ROM `cpu_reset` montrent qu'un programme qui dépend du vecteur de reset repart bien par
`$FFFC`. Note : la relecture R2 (N-44-3) signalait déjà que le titre de REQ-LOOP-072 ne porte pas
l'étiquette `[ROM]` alors qu'un de ses critères en relève ; la v1.2 l'a ajoutée au titre, ce qui
est conforme à ce qu'elle demandait.

---

## 5. Ticket Q ouvert du lot — Q-078

`spec/changes/Q-078.md` (R3, 2026-10-30) porte sur REQ-CORE-122 : la phrase « PPU, APU et mapper
suivent leur état documenté *after reset* » ne dit pas si deux **comportements** des pages nesdev
en font partie. Les deux choix provisoires **tiennent au texte de la spec**.

1. **Fenêtre d'inhibition des écritures du PPU (≈ 29 658 cycles) non modélisée** — **choix
   accepté**. Les trois exemples que donne l'exigence (« PPUCTRL et PPUMASK à 0, canaux APU
   coupés, `$4015` à 0 ») sont tous des éléments d'**état** ; la fenêtre est une temporisation,
   et l'exigence parle d'« état documenté ». L'argument de cohérence est vérifiable et exact : le
   reset dur (REQ-CORE-004) ne la modélise pas davantage alors que le matériel l'applique aussi
   à la mise sous tension, de sorte que l'ajouter au seul reset doux rendrait les deux chemins
   incohérents. Et aucune des ROM exigées par SPEC-02 §3.12, les deux `cpu_reset` ajoutées
   comprises, ne la mesure : vérifié, les deux passent (§2). Marqueur `// Q-078` bien présent
   dans `Ppu2C02.SoftReset`. Réserve : la page « PPU power up state » range cette phrase **sous**
   la section *after reset* à laquelle l'exigence renvoie, donc l'ambiguïté est réelle et le
   ticket justifié ; si R1 tranche dans l'autre sens, il faudra trancher **en même temps** pour
   le reset dur, comme le ticket le dit.
2. **Séquenceur de frames de l'APU remis à zéro (révision G)** — **choix accepté et vérifié**.
   L'exigence ne nomme pas la révision ; le choix garde les deux resets cohérents (même point de
   départ que le reset dur) et il est **mesuré** par les ROM publiques : les six `apu_reset` de
   blargg passent sous reset doux, dont `4017_timing`, qui mesure précisément le délai, et
   `4017_written`, qui vérifie que le **mode** de `$4017` survit — ce que l'exigence impose par
   ailleurs. Exécution relevée :
   `ApuRomTests.REQ_CORE_122_the_apu_reset_tests_pass_under_a_soft_reset` → 6 cas passés
   (`4015_cleared`, `4017_timing`, `4017_written`, `irq_flag_cleared`, `len_ctrs_enabled`,
   `works_immediately`). Marqueur `// Q-078` bien présent dans `Apu2A03.SoftReset`.

Aucun autre ticket Q n'est ouvert pour ce lot.

---

## 6. Suites exécutées

Release, premier plan, une suite à la fois (règle de `CLAUDE.md`).

| Suite | Commande | Résultat |
|---|---|---|
| Mes tests | `dotnet test tests/Acceptance/… --filter "FullyQualifiedName~R4V2RollbackResetTests"` | `Failed: 1, Passed: 22, Skipped: 0, Total: 23` — l'échec est la non-conformité du §3 |
| Acceptation complète | `dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release` | `Failed: 2, Passed: 1397, Skipped: 15, Total: 1414, Duration: 3 m 44 s` |
| `Zax.App.Tests` | `dotnet test tests/Zax.App.Tests/Zax.App.Tests.csproj -c Release` | `Failed: 0, Passed: 413, Skipped: 0, Total: 413` |
| `Zax.Nes.Tests` | `dotnet test tests/Zax.Nes.Tests/Zax.Nes.Tests.csproj -c Release` | `Failed: 0, Passed: 677, Skipped: 4, Total: 681` |

Le second échec de la suite d'acceptation est **hors lot** et non reproductible isolément :
`AcceptanceGateTests.Converting_a_full_frame_fits_the_time_budget` (« worst conversion 17,125 ms
exceeds one frame ») est une barrière de performance sensible à la charge CPU ; relancée seule,
elle passe (`Passed: 1, Total: 1, Duration: 2 s`). Elle ne touche ni AMD-042 ni AMD-044.

---

## 7. Observations mineures (hors verdict)

1. **Les tests de l'implémenteur dans l'espace du vérificateur.** Le commit modifie quatre
   fichiers de `tests/Acceptance/`, qui est réservé au vérificateur par le `Description` du
   `.csproj` (« Implementers must not add tests here »). Les modifications sont correctes sur le
   fond (mise à jour des signatures de `RollbackHistory`, 20ᵉ action, nouvelle théorie
   `cpu_reset`), mais elles brouillent la séparation des rôles : ces tests ne sont plus
   indépendants de l'implémentation qu'ils contrôlent. C'est une remarque de procédure pour R1,
   pas une non-conformité au texte.
2. **`REQ-LOOP-033` et la restauration étendue.** REQ-LOOP-070 « complète REQ-LOOP-032 étape 2
   **et REQ-LOOP-033** ». Le panneau d'historique de REQ-LOOP-033 (P3, facultatif, absent de
   l'interface livrée) n'existe pas : `RollbackHistory.TakeAt` n'est appelé par aucun chemin de
   l'hôte. La clause est donc vide pour l'instant, mais `TakeAt` rend le snapshot **complet**
   (cœur + `PData` + caméra), donc un panneau futur héritera de la restauration étendue sans
   travail supplémentaire.
3. **Taille d'un snapshot.** La mémoire annoncée par REQ-LOOP-069 (« 30 × (état du cœur +
   64 nombres + état de caméra) ») correspond : le `record` `RollbackSnapshot` ne porte que ces
   trois champs, le bloc `PData` fait 256 octets et l'état de caméra 64 octets de valeurs.
   Aucune exigence ne chiffre une borne à vérifier.
