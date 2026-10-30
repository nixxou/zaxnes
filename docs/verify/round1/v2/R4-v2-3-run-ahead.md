# Rapport de vérification — round 1, V2, lot v2-3-run-ahead

**Rôle** : R4, vérificateur indépendant, côté propre
**Date** : 2026-10-30
**Branche vérifiée** : `delta/v2-3-run-ahead`, commit `ac0e85c` (« V2-3: run-ahead (AMD-041) »)
**Textes normatifs** : `spec/changes/AMD-041-run-ahead.md` **v1.2** du 2026-10-30 (la dernière :
la seconde relecture R2 a fait passer AMD-041 en 1.2 pour N-41-1, N-41-2, N-41-3 et T-2) et,
dans `spec/changes/AMD-040-api-lua-3dsen-2.md` **v1.2**, la seule ligne `RunAheadFrames` de
REQ-LUA-114 (§2 et §3 « §4.2 »). Relectures R2 lues pour le raisonnement seulement :
`R2-review-AMD-039-045.md` (41-1 à 41-10) et `R2-review-AMD-039-045-v1.1.md` (41-1 à 41-10
levés, N-41-1 à N-41-3).
**Tests écrits pour cette vérification** : `impl/tests/Acceptance/R4V2RunAheadTests.cs`,
23 méthodes, **39 cas**, tous exécutés. Les tests de l'implémenteur
(`tests/Zax.App.Tests/RunAheadTests.cs`, `RunAheadRom.cs`, `tests/Zax.Nes.Tests/CoreStateTests.cs`)
n'ont été ni lus comme référence ni modifiés : les vecteurs viennent du texte de l'amendement.
**Méthode** : trois ROMs NES construites par le vérificateur (`R4RunAheadRom`), et l'observation
par l'extérieur — fichier de réglages, résultats Lua, état du cœur (REQ-CORE-121), image
256 × 240 que le cœur compose, palette que la conversion publie, exports du harnais. La boucle
de bureau est pilotée par `HostDriver`, qui atteint `EmulationHost` par réflexion : aucun
fichier de `src/` n'est touché.

La ROM de REQ-LOOP-065 respecte la convention que N-41-2 demande d'écrire : **le compteur
qu'affiche une image est celui du début de la frame**. Son gestionnaire de NMI s'exécute au tout
début d'une frame émulée (le cœur arrête une frame au premier dot du blanking vertical, donc
l'interruption est servie par l'appel suivant, avant les 240 lignes visibles) et il écrit la
couleur de fond universelle **avant** d'incrémenter. Le rendu reste éteint : toute l'image vaut
cette couleur, et « l'image de la frame » tient dans un octet — cinq bits bas le compteur,
bit 5 l'état du bouton A tel que cette frame-là l'a lu.

---

## 1. Décompte par verdict

| Verdict | Lignes |
|---|---|
| **conforme** | 10 |
| **non conforme** | 0 |
| **non testable** | 0 (3 clauses partielles, voir §3) |
| **ambigu** | 0 |
| **Total** | **10** |

**Aucune non-conformité.** Trois observations mineures en §4, aucune bloquante.

---

## 2. Tableau par exigence

Commande commune (foreground, Release) :

```
dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release --no-build \
  --filter "FullyQualifiedName~R4V2RunAheadTests"
→ Passed!  - Failed: 0, Passed: 39, Skipped: 0, Total: 39
```

| REQ-ID | Verdict | Preuve (test, observation) |
|---|---|---|
| **REQ-LOOP-064** — réglage Run-ahead | **conforme** | `REQ_LOOP_064_an_empty_user_root_gives_one_frame_of_run_ahead` : racine utilisateur vide → `emu.run_ahead` = 1, portée U, `LuaPath = RunAheadFrames`. `REQ_LOOP_064_a_value_outside_zero_to_three_is_brought_back` (9 cas, −7/−1/0/1/2/3/4/7/`int.MaxValue` → 0/0/0/1/2/3/3/3/3) : hors de [0, 3] la valeur est **ramenée**, jamais remplacée par le défaut. `REQ_LOOP_064_the_value_survives_a_restart` : `"emu.run_ahead": 3` dans le fichier de réglages, relu à 3 par une seconde instance. `REQ_LOOP_064_the_four_values_carry_the_labels_of_the_amendment` : domaine = les quatre entiers 0 à 3, libellés `Off` / `1 frame (Recommended)` / `2 frames` / `3 frames` et `Désactivé` / `1 frame (recommandé)` / `2 frames` / `3 frames`, titre `Run-ahead` dans les deux langues (§3). `REQ_LOOP_064_a_change_during_the_game_applies_at_the_next_real_frame` : N passé de 0 à 2 en cours de partie, l'image saute de `c−1` à `c+2` **sur la frame réelle suivante**, le cœur n'avançant que d'une. `AMD_041_the_setting_is_not_exposed_to_libretro` : `IsLibretroExposed` faux (§4) |
| **REQ-LOOP-065** — déroulement d'une frame réelle | **conforme** | `REQ_LOOP_065_the_vector_of_the_amendment` : vecteur de l'amendement au pied de la lettre, Run-ahead = 2, compteur `k` juste avant la frame réelle, A pressé → image = `((k+2) & 0x1F) | 0x20` (compteur k+2 **et** A pressé), compteur du cœur = `k+1` après l'étape 5, `NesCore.FrameCount` + 1 exactement. La même assertion vérifie que la frame réelle a vu le **même** E : l'octet `$12` qui survit à l'étape 5 porte le bit A. `REQ_LOOP_065_the_displayed_frame_is_n_frames_ahead` (N = 0, 1, 2, 3, 6 frames réelles chacune) : image = `c + N`, compteur = `c + 1`, cœur + 1 — et à N = 0 la frame est émulée une seule fois et c'est l'affichée. `REQ_LOOP_067_one_real_frame_yields_one_frame_of_audio` (N = 0, 1, 3) : 800,0 échantillons par frame réelle (48 000 / 60,0988 ≈ 798,7, écart < 5 %), donc les étapes 1 et 3 sont bien muettes |
| **REQ-LOOP-060** (amendée) | **conforme** | (a) une seule lecture d'entrées par frame réelle, partagée par les N + 1 frames : `REQ_LOOP_065_the_vector_of_the_amendment` (bit A présent sur la frame réelle **et** sur l'affichée). (b) voir REQ-LOOP-065. (c) `REQ_LOOP_060_the_conversion_receives_the_displayed_frame` : l'entrée 0 de la palette publiée est le fond universel de la frame **affichée** (`c + 2`), une seule frame convertie par frame réelle, suite contiguë. (d) `REQ_LOOP_067_read_mem_in_a_hook_sees_the_displayed_frame`. Snapshot de rollback avant (a) sur l'état réel : `REQ_LOOP_066_the_history_follows_the_real_counter` |
| **REQ-LOOP-066** — ce qui suit la frame réelle | **conforme** | `REQ_LOOP_066_a_quick_save_carries_the_real_frame` : Run-ahead = 3, 120 frames réelles → `FrameCount` = 120 (et non 480) ; quick save, 45 frames de plus (165), quick load → `FrameCount` = 120 et le compteur de la ROM est celui de la sauvegarde. `REQ_LOOP_066_the_history_follows_the_real_counter` : 300 frames réelles → **5** snapshots (compteurs réels 0, 60, 120, 180, 240) à N = 0 **comme** à N = 3. `REQ_LOOP_066_a_load_or_a_reset_carries_on_with_the_same_n` : après un quick load et après un reset, N vaut toujours 2 et l'image est toujours deux frames en avance |
| **REQ-LOOP-067** — ce qui voit la frame affichée | **conforme** | `REQ_LOOP_067_the_frame_hooks_run_once_per_real_frame` (N = 0, 1, 2, 3) : un compteur incrémenté dans `Update` et un autre dans `LateUpdate` augmentent de **exactement 40** sur 40 frames réelles, et la globale `frame` de +40 : aucun appel pour la frame réelle ni pour les frames en avance non affichées. `REQ_LOOP_067_read_mem_in_a_hook_sees_the_displayed_frame` : à Run-ahead = 2, `Script:ReadMem` dans `Update` renvoie l'octet que la frame **affichée** a mis sur son image (`k + 2`), alors qu'après l'étape 5 le cœur est revenu à `k + 1` — vérifié sur 5 frames consécutives. Audio : `REQ_LOOP_067_one_real_frame_yields_one_frame_of_audio` |
| **REQ-LOOP-068** — Zapper | **conforme** | `REQ_LOOP_068_the_zapper_gives_the_same_result_with_run_ahead` : ROM maison (écran blanc, compteur 16 bits des lectures de `$4017` dont le bit 3 est à 0), port 2 en `Zapper` par le script du profil, souris au centre de la cible de rendu, 10 frames réelles → **même visée et même compte** à Run-ahead = 1 qu'à 0, et compte non nul (la lumière est bien vue). `REQ_LOOP_068_sprite_zero_and_timing_stay_exact_without_pixels` : seconde phrase de l'exigence — ROM avec fond opaque, sprite 0 en X = 100 / Y = 60, rendu activé, qui mesure en itérations CPU le délai du début du blanking au hit de sprite 0 ; après 60 frames réelles, **le délai mesuré et l'état complet du cœur sont identiques à N = 0, 1, 2 et 3** (c'est le critère « barre d'état fixe de Super Mario Bros. » rendu automatique) |
| **REQ-CORE-121** — état en mémoire du cœur | **conforme** | `REQ_CORE_121_a_restore_replays_a_hundred_frames_byte_for_byte` : sauvegarder, émuler 100 frames (séquence d'entrées variable), restaurer, réémuler les mêmes 100 frames → les 100 images 256 × 240 **et** les 100 × 2 KiB de RAM identiques octet pour octet. `REQ_CORE_121_the_state_is_bounded_and_holds_no_rom_image` : `state.Length ≤ MaxCoreStateSize()`, `MaxCoreStateSize() < MaxStateSize()`, état < 32 KiB donc sans l'image de la ROM ; compteur de frames, RAM, mapper, CPU, PPU, APU et entrées reviennent. `REQ_CORE_121_a_restore_leaves_the_audio_already_produced_alone` : après la restauration en mémoire 800 échantillons survivent, là où un `TryLoadState` de savestate en laisse 0 (REQ-CORE-074, Q-006) |
| **REQ-ARCH-012** (amendée) | **conforme** | `REQ_ARCH_012_the_real_core_states_do_not_depend_on_n` : même ROM, même séquence d'entrées, 90 frames réelles → l'état complet du cœur (REQ-CORE-121) est **identique octet pour octet** à N = 0, 1, 2 et 3. `REQ_ARCH_012_the_displayed_sequence_depends_on_n` : la suite des frames affichées, elle, dépend de N, et exactement de N (décalage de 1 à N = 1, de 3 à N = 3), donc les comparer à N différent n'aurait pas de sens. Exports du harnais : `Zax.Harness render --rom … --run-ahead 2 --dump-frame f.json` écrit `"runAhead": 2` en tête du fichier de frame **et** dans `render.json`, dont la description dit « after 30 real frames at run-ahead 2 » |
| **REQ-ARCH-013** (amendée) | **conforme** | Même preuve que REQ-ARCH-012 pour la clause « à N égal » et les exports du harnais. N n'entre pas dans le savestate et n'a pas à y entrer : `REQ_LOOP_066_a_load_or_a_reset_carries_on_with_the_same_n` montre qu'un chargement reprend avec le N courant, ce que REQ-LOOP-066 exige. La borne de taille de l'état en mémoire est vérifiée par `REQ_CORE_121_the_state_is_bounded_and_holds_no_rom_image` |
| **REQ-LUA-114** — ligne `RunAheadFrames` | **conforme** | `REQ_LUA_114_run_ahead_frames_is_read_and_written_from_lua` : `type(Setting.RunAheadFrames)` = `number`, lecture = 1 par défaut, `= 7` → 3, `= -1` → 0, `= 2` → 2 et le modèle de réglages porte 2. Les deux critères (AMD-040 §2 et AMD-041 §2) sont couverts par le même test |

---

## 3. Clauses non couvertes par un test automatique

Elles ne changent le verdict d'aucune exigence et sont signalées pour la traçabilité.

1. **REQ-LOOP-066, « l'enregistrement des entrées » et « le pas à pas »** — ni l'un ni l'autre
   n'existent dans Zaxnes : SPEC-07 §1 exclut explicitement « enregistrement et relecture de
   movies », et aucune exigence de la spec ne décrit un pas à pas. Les deux clauses sont donc
   **vides** ici, pas violées. Les quatre autres porteurs de la phrase (état du cœur, compteur,
   historique, partie « cœur » des savestates et quick saves) sont testés.
2. **REQ-LOOP-068, critères `[ROM]`** (Duck Hunt à la souris, barre d'état de Super Mario Bros.)
   — ils demandent des ROMs commerciales, hors de ce qu'un test automatique peut porter. Le
   critère « auto » de l'exigence est couvert, et le mécanisme que le critère Super Mario Bros.
   met en jeu — exactitude du drapeau de sprite 0 et du temps quand les pixels ne sont pas
   composés — est vérifié de bout en bout par `REQ_LOOP_068_sprite_zero_and_timing_stay_exact_without_pixels`.
3. **REQ-ACC-030 (précisée)** — « la mesure de performance se fait au réglage par défaut, donc
   avec Run-ahead = 1 ». Hors périmètre du lot (l'amendement ne fait que la préciser) et hors
   de ce qu'un test unitaire mesure ; le point de départ est bon, le réglage vaut 1 sur une
   racine utilisateur vide. La campagne REQ-ACC-030 devra être relancée à ce réglage.

---

## 4. Observations (aucune n'est une non-conformité)

- **O-1 (mineur)** — Le harnais `Zax.Harness render` a `--run-ahead 0` **par défaut**, alors
  que le réglage de l'application vaut 1. Ce n'est pas une contradiction : AMD-041 §3 fixe le
  défaut du **réglage** `emu.run_ahead`, pas celui d'un outil de développement, et garder 0 par
  défaut préserve la comparabilité des captures déjà produites — ce que REQ-ARCH-012 amendée
  demande justement, puisque N est écrit dans `render.json` et dans le fichier de frame. À
  garder en tête seulement si une mesure de REQ-ACC-030 passait un jour par le harnais.
- **O-2 (mineur)** — `GameSession.RunAheadFrames` vaut 0 sur une session fraîchement créée, et
  c'est la boucle de bureau qui y recopie le réglage avant chaque frame réelle. C'est le
  comportement qu'AMD-041 §4 demande du frontend libretro (« le réglage n'y est pas exposé et
  y vaut 0 ») ; côté bureau, `REQ_LOOP_064_a_change_during_the_game_applies_at_the_next_real_frame`
  montre que la recopie a lieu au bon moment.
- **O-3 (mineur)** — Avec un port en `Zapper`, la frame réelle **rend ses pixels** mais ne
  produit ni rappel vidéo ni son (`FrameOutput.Pixels`). C'est exactement la lecture que la
  seconde relecture R2 donne de l'étape 1 (« n'interdit que l'image **pour l'affichage** ») et
  c'est ce que REQ-LOOP-068 exige ; le tampon de pixels n'est pas dans le bloc d'état, donc
  l'étape 5 ne le ramène pas en arrière et l'image que l'hôte montre reste celle de la frame
  affichée (vérifié par toutes les assertions sur `PixelFrame`).
- **Point relu sans test dédié** — `SettingDomain.Coerce` change de condition (`Rule !=
  Unbounded` → `Rule == UseDefault`) pour laisser passer le nouveau domaine `ClampedChoice`.
  Les seuls domaines à liste existants sont `Choice` (règle `UseDefault`, inchangée) et
  `Switch` (règle `Unbounded`, inchangée) : aucune régression possible, et les 273 tests de
  `Zax.Settings.Tests` plus les 1 413 d'acceptation le confirment.

---

## 5. Tickets Q du lot

Recherche sur les **79** tickets de `spec/changes/` (`Q-001` à `Q-079`) des mots-clés
`run-ahead`, `run_ahead`, `RunAhead`, `REQ-LOOP-064` à `REQ-LOOP-068` et `REQ-CORE-121` :
**aucun ticket n'est ouvert ni fermé pour ce lot**, ce que l'énoncé annonçait. AMD-041 §5 et
AMD-040 §4 déclarent tous deux « Aucune » question ouverte, et l'implémenteur n'en a pas créé.

Le seul ticket que le code du lot cite est **Q-006** (clos), sur ce que « l'audio repart du
silence » efface à la restauration d'un savestate. La citation est juste : l'implémentation s'y
appuie pour **opposer** les deux restaurations — celle d'un savestate vide la file d'attente
(REQ-CORE-074, lecture 1 de Q-006), celle de REQ-CORE-121 la laisse intacte, comme le texte de
l'exigence le demande explicitement. `REQ_CORE_121_a_restore_leaves_the_audio_already_produced_alone`
vérifie les deux branches dans le même test.

---

## 6. Suites exécutées (Release, foreground, une à la fois)

| Projet | Commande | Résultat |
|---|---|---|
| Acceptation (mes tests seuls) | `dotnet test tests/Acceptance/… --filter "FullyQualifiedName~R4V2RunAheadTests"` | `Passed! - Failed: 0, Passed: 39, Skipped: 0, Total: 39` |
| Acceptation (complète) | `dotnet test tests/Acceptance/Zax.Acceptance.Tests.csproj -c Release` | `Passed! - Failed: 0, Passed: 1413, Skipped: 15, Total: 1428, Duration: 3 m 45 s` |
| Cœur NES | `dotnet test tests/Zax.Nes.Tests/… -c Release` | `Passed! - Failed: 0, Passed: 672, Skipped: 4, Total: 676` |
| Application | `dotnet test tests/Zax.App.Tests/… -c Release` | `Passed! - Failed: 0, Passed: 433, Skipped: 0, Total: 433` |
| Réglages | `dotnet test tests/Zax.Settings.Tests/… -c Release` | `Passed! - Failed: 0, Passed: 273, Skipped: 0, Total: 273` |
| Scripts | `dotnet test tests/Zax.Scripting.Tests/… -c Release` | `Passed! - Failed: 0, Passed: 52, Skipped: 0, Total: 52` |

Harnais, pour les exports de REQ-ARCH-012/013 :

```
Zax.Harness render --out <tmp> --rom tests/roms/240pee/240pee.nes --run 30 \
  --run-ahead 2 --dump-frame <tmp>/frame.json
→ frame.json : { "runAhead": 2, "palette": [...] }
→ render.json : { "scene": "240pee", "description": "240pee.nes after 30 real frames at
   run-ahead 2, profile '2D Mode'", …, "runAhead": 2, … }
```

---

## 7. Conclusion

Le lot `v2-3-run-ahead` est **conforme** à AMD-041 v1.2 et à la ligne `RunAheadFrames` de
REQ-LUA-114 d'AMD-040 v1.2, sur les dix exigences du périmètre. Les trois vecteurs chiffrés de
l'amendement (compteur `k + 2` / `k + 1` à N = 2, les 100 frames rejouées de REQ-CORE-121, les
cinq snapshots aux multiples de 60 du compteur réel à N = 3) passent tels qu'ils sont écrits,
et le point le plus délicat de l'amendement — l'exactitude du cœur quand les pixels ne sont pas
composés, qui porte la barre d'état de Super Mario Bros. — est vérifié par une ROM à sprite 0.
Aucune non-conformité, aucun ticket Q à arbitrer.
