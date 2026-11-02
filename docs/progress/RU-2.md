# RU-2 — Lua API of 3dSen 2.0 and camera (AMD-040, AMD-042, AMD-045, AMD-059)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-2
(`eb46478`, AMD-040 + AMD-045), V2-5 (`b82d2d3`, AMD-042 + AMD-044) and V2-31 (`8dd1cc5`,
AMD-059) of `dotnetupdated` are read as a reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, before benchmarks, amendments copied | in progress |
| 2 | The persisted settings the 2.0 `Setting` members stand for, handheld defaults, `EXCLUDED` | todo |
| 3 | AMD-040: the `CS` table and `Camera.ResetDistance` | todo |
| 4 | AMD-040: `RunAheadFrames`, `VisualStyle`, the style-parameter methods, the export methods | todo |
| 5 | AMD-059: the four scalar members, `VisualStyleParameters`, the `CS.Nes3D` tables | todo |
| 6 | AMD-042: a rollback snapshot carries `PData` and the camera; the history is emptied at the end of a game | todo |
| 7 | AMD-045: the camera of 3dSen 2.0 | todo |
| 8 | Closing: fmt, clippy, tests in both binding modes, after benchmarks, API dump, journal | todo |

The hash of a step is written in the commit of the step that follows (no amend, protocol of this
lot).

## Requirement map

### AMD-040 — the Lua members of 3dSen 2.0

| Requirement | Rust crate / file |
|---|---|
| REQ-LUA-112 `CS.Nes3D.Info.Version` / `.Title`, sealed tables | `zax-scripting/src/runtime/cs_table.rs` (new), called from `runtime/globals.rs` |
| REQ-LUA-113 `Camera.ResetDistance` | `zax-scripting/src/api/scene.rs`, `environment.rs` (`ScriptCamera::reset_distance`), `zax-app/src/scene_script_bridge.rs`, `zax-harness/src/render/rom.rs` |
| REQ-LUA-114 `Setting.RunAheadFrames` | `zax-scripting/src/api/settings.rs`, `settings.rs`, `zax-settings/src/table.rs` (`emu.run_ahead`) |
| REQ-LUA-114 `Setting.VisualStyle` | same, plus `render.visual_style` and the 1 → 4 alias of AMD-055 REQ-SCN-078 |
| REQ-LUA-114 `Get/Set/ResetVisualStyleParameter(s)` | `zax-settings/src/styles.rs` (new: the parameter table and the normalisation of AMD-040 §3), `zax-scripting/src/settings.rs`, `api/settings.rs` |
| REQ-LUA-115 `Instance:BuildExportMeshes()` / `ReleaseExportMeshes()` | `zax-scripting/src/api/instance.rs` |

### AMD-059 — the remaining members

| Requirement | Rust crate / file |
|---|---|
| REQ-LUA-116 `LibraryView`, `MaxQueuedFrames`, `RefreshRate`, `Msaa` | `zax-scripting/src/settings.rs` (the normalisations), `api/settings.rs`; keys `ui.library_view`, `video.max_queued_frames`, `vr.refresh_rate`, `video.msaa_2_0` in `zax-settings/src/table.rs` |
| REQ-LUA-116 `VisualStyleParameters` and its elements | `zax-settings/src/styles.rs` (the write log), `zax-scripting/src/api/settings.rs`, `binding/payload.rs` (`HostType::StyleParameterList`, `StyleParameter`) |
| REQ-LUA-117 `CS.Nes3D.CameraControl` / `SettingManager` / `VisualStyle` | `zax-scripting/src/runtime/cs_table.rs` |

### AMD-042 — a rollback snapshot carries `PData` and the camera

| Requirement | Rust crate / file |
|---|---|
| REQ-LOOP-069 contents of a snapshot | `zax-app/src/emulation/rollback_history.rs` (`RollbackSnapshot`), `emulation/host/mod.rs` (the capture) |
| REQ-LOOP-070 restoration, no `Save` / `Load` hook, camera shown straight away | `zax-app/src/game_session.rs` (`capture_rollback_data` / `restore_rollback_data`), `emulation/host/save_states.rs` |
| REQ-LOOP-071 the history is emptied at the end of a game | `zax-app/src/emulation/host/mod.rs` |
| REQ-SCN-024 amended: the transition window as a remaining time | `zax-render/src/camera.rs` (already a remaining time; the bound to `[0, 3]` is what changes) |

### AMD-045 — the camera of 3dSen 2.0

| Requirement | Rust crate / file |
|---|---|
| REQ-SCN-069 d₀ = 132.7964 / tan(FOV / 2) × s | `zax-render/src/geometry.rs` (`reset_distance`, `RESET_HALF_HEIGHT`) |
| REQ-SCN-045 amended: d_HUD = 76.6133 / tan(FOV / 2) | `zax-render/src/geometry.rs` (`hud_distance`, `HUD_HALF_HEIGHT`) |
| REQ-SCN-070 the user zoom stops at 16 × s | `zax-render/src/geometry.rs` (`zoom_in`, `MIN_USER_DISTANCE`), `controls.rs` |
| REQ-SCN-071 the setpoint angles stay in ]−180, 180] | `zax-render/src/geometry.rs` (`wrap_angle`), `camera.rs` (`advance`) |
| REQ-SCN-072 the vertical of the camera in TPV | `zax-render/src/camera.rs` (`build_view`, `basis_from_forward`) |
| REQ-SCN-073 a reloaded camera does not slide | `zax-render/src/camera.rs` (`restore`) |

## Notes

### Before benchmarks (the binding micro-benchmarks, step 1)

`cargo test -p zax-scripting --release --test scripting perf -- --ignored --nocapture
--test-threads=1`, this machine, nothing else running. Both binding modes, the second with
`ZAXNES_FAST_LUA=1`.

| Measurement | safe | fast |
|---|---|---|
| empty loop | 3 ns | 3 ns |
| `Camera.Distance` read | 279 ns | 205 ns |
| `Camera.Distance` write | 187 ns | 121 ns |
| `Setting.SceneRatio` read | 165 ns | 79 ns |
| `Script:ReadMem` | 493 ns | 327 ns |
| unknown member read | 115 ns | 130 ns |
| `Vector3(1, 2, 3)` | 391 ns | 469 ns |
| vector component read | 110 ns | 55 ns |
| vector component write | 131 ns | 63 ns |
| `BuildPattern`, 50 instances | 1.72 µs | 1.59 µs |
| `BuildPattern`, 500 instances | 1.59 µs | 1.49 µs |
| `Update` per hook call | 46.48 µs | 19.06 µs |
| `LateUpdate` per hook call | 46.52 µs | 19.00 µs |
| empty `UpdateS` | 0.83 µs | 0.83 µs |
| frame with `Update` + `LateUpdate` | 71.8 µs | 46.0 µs |
| … with 60 `UpdateS` on top | 219.7 µs | 151.8 µs |
| 60 shapes walking 60 shapes | 6914.8 µs | 3051.3 µs |

### What this lot adds to a per-frame path: nothing

- Member lookup is a `HashMap<&[u8], usize>` per type (`binding/table.rs`), so the new members
  cost the existing ones nothing: no scan grows. The one bound to watch is `method_key`
  (`binding/raw.rs`), which packs `type_index * 1024 + position`: a type must stay under 1024
  members, and `SettingAPI` goes from 95 to 104.
- **Both binding paths come for free in Rust.** Unlike the .NET build, where the fast path had
  its own table, `binding/raw.rs` reads the very `TypeDef` the safe path reads (`api()`,
  `definition.member(key)`), so a member declared once answers in both modes with the same
  function. There is nothing to declare twice; the tests are still run in both modes.
- No new setting is read per frame, per tile or per pixel: the 22 keys of step 2 have **no**
  effect in this lot (see below), and the four the camera reads it already read.
- The camera work of AMD-045 adds, per displayed frame, one `wrap_angle` per setpoint angle (6
  multiplications and a `ceil`) and one cross product in TPV — no allocation, no branch on a
  setting.

### `CORE_MEMBERS` (`zax-app/src/game_session.rs`): nothing to add, member by member

`CORE_MEMBERS` names the API members that read or drive the emulated console; a profile naming
none of them may have its next frame started early (emulation ahead, pipelining,
pause-3D-in-2D, catch-up-last-3D). The members this lot adds, one by one:

| Member | Reads or drives the core? | Why |
|---|---|---|
| `CS.Nes3D.Info.Version`, `.Title` | no | two constant strings in a Lua table |
| `CS.Nes3D.CameraControl.DistanceMax`, `SettingManager.DefaultMsaa`, `VisualStyle.*` | no | constant numbers in Lua tables |
| `Camera.ResetDistance` | no | the camera's field of view, which is a setting of the host |
| `Setting.RunAheadFrames`, `VisualStyle`, `LibraryView`, `MaxQueuedFrames`, `RefreshRate`, `Msaa` | no | settings of the model; none of them is the core's state |
| `Setting:Get/Set/ResetVisualStyleParameter(s)`, `VisualStyleParameters` | no | settings of the model |
| `Instance:BuildExportMeshes`, `ReleaseExportMeshes` | no | they do nothing at all (REQ-LUA-115) |

**AMD-042 adds no Lua member.** The amendment is about what a rollback snapshot carries
(`PData` and the camera) and about when the history is emptied; it adds nothing to SPEC-05 §4,
so there is no "rollback member" to weigh. The capture and the restore run in the host, between
two emulated frames, never from a script. `separate_core_tests` and `pause_3d_tests` are run all
the same, because the capture moved inside the frame loop.

Adding any of the above to `CORE_MEMBERS` would cost the handhelds their pipelining on every
profile that reads a setting — which is most of them — for no gain.

### The members whose effect belongs to a later lot

The Lua surface is complete here — the value is persisted, the normalisation is applied, and no
numeric write raises — but nothing of the program reads these settings yet:

| Member / setting | Surface | Effect |
|---|---|---|
| `Setting.RunAheadFrames` (`emu.run_ahead`) | done | RU-3 (AMD-041 run-ahead) |
| `Setting.MaxQueuedFrames` (`video.max_queued_frames`) | done | RU-3 (AMD-052 REQ-LOOP-077) |
| `Setting.Msaa` (`video.msaa_2_0`) | done | none ever: AMD-059 §3 keeps `video.anti_alias` as the anti-aliasing **(D)** |
| `Setting.RefreshRate` (`vr.refresh_rate`) | done | none ever: no virtual reality in Zaxnes **(D)** |
| `Setting.LibraryView` (`ui.library_view`) | done | none ever: one library layout |
| `Setting.VisualStyle` (`render.visual_style`) | done, with the 1 → 4 alias | RU-5 (AMD-055 Enhanced) |
| the four curvature parameters (`scene.bend_lr`, `scene.bend_tb`, `scene.arc_h`, `scene.arc_center`) | done | the AMD-043 lot (curvature of space) |
| the twelve Enhanced parameters (`render.style.*`) | done | RU-5 (AMD-055) |

Every one of them is in `ui/options.rs` `EXCLUDED` until the lot that gives it an effect takes
it out: an option the user can move and that changes nothing would be worse than no option.
`vr.refresh_rate` needs no line, the `Vr` theme having no tab.
