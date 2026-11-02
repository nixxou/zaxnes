# RU-2 — Lua API of 3dSen 2.0 and camera (AMD-040, AMD-042, AMD-045, AMD-059)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-2
(`eb46478`, AMD-040 + AMD-045), V2-5 (`b82d2d3`, AMD-042 + AMD-044) and V2-31 (`8dd1cc5`,
AMD-059) of `dotnetupdated` are read as a reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, before benchmarks, amendments copied | done — `55b6723` |
| 2 | The persisted settings the 2.0 `Setting` members stand for, handheld defaults, `EXCLUDED` | done — `1204603` |
| 3 | AMD-040: the `CS` table and `Camera.ResetDistance` (with the two formulas of AMD-045) | done — `644bf51` |
| 4 | AMD-040: `RunAheadFrames`, `VisualStyle`, the style-parameter methods, the export methods | done — `7121e78` |
| 5 | AMD-059: the four scalar members, `VisualStyleParameters`, the `CS.Nes3D` tables | done — `01cf39b` |
| 6 | AMD-042: a rollback snapshot carries `PData` and the camera (with REQ-SCN-073) | done |
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

### Step 2 — the settings the 2.0 members stand for

Twenty-two keys, none of which has any effect in this lot. They are the *values* the Lua members
of AMD-040 and AMD-059 read and write; each one's effect is its own lot's (table above).

- `zax-settings/src/catalogue.rs`: the identifiers, each in the section of SPEC-09 §4.1 its
  amendment puts it in. `zax-settings/src/table.rs`: the rows, with the default, the scope U, the
  effect and the domain the amendment gives.
- `zax-settings/src/domain.rs` gained what the amendments need and nothing more:
  - `Domain::stepped(min, max, step, decimals)` — the rungs, published as the finite ordered list
    REQ-PERS-081 asks for, **and** the rule `Snap`: `coerce` brings a value into the bounds and
    then onto the nearest rung, ties going to the even one. That is the normalisation of AMD-040
    §3 word for word, and putting it in the domain is what makes « écrire par Lua ou par
    l'interface revient au même » (REQ-LUA-114) true by construction rather than by two copies of
    the same arithmetic. A value between two rungs is snapped, not sent to the default: the
    general rule of REQ-PERS-003 is explicitly replaced here (AMD-055 REQ-SCN-079 header).
  - `Domain::aliased_choice` — `render.visual_style` accepts 1 for Enhanced and reads back 4
    (AMD-055 REQ-SCN-078), wherever the 1 comes from, a script or the settings file.
  - `Domain::snap` is public: the Lua path calls it on the number the script passed, **before**
    the value is truncated to an integer, so `SetVisualStyleParameter(4, "spl.bend_zx", 45)` sees
    45 and not something already rounded.
- `zax-settings/src/styles.rs` (new) is the only place that knows which parameter identifier
  names which setting: the four of AMD-043 REQ-SCN-066 and the twelve of AMD-055 REQ-SCN-079,
  sixteen in all, under the style 4 and its alias 1. Every other identifier, and every other
  style, is "unknown" in the sense of REQ-LUA-114.
- `zax-app/src/ui/options.rs` `EXCLUDED` grew by 21 lines, and `ui/handheld/defaults.rs` by one:
  `emu.run_ahead` is 0 on a handheld (it costs a whole extra emulated frame per shown frame) and
  1 on the desktop, as AMD-041 §3 has it. The other twenty-one cost nothing — they do nothing —
  so they keep the default of their amendment on every platform.
- `docs/spec/SPEC-09-persistance.md`: the rows of §4.1.1, §4.1.4, §4.1.7, §4.1.9 and §4.1.10 and
  the libretro exclusions of §4.6, as the .NET lot V2-31 did for the three keys it added.

Tests: `zax-settings/tests/two_zero_settings.rs` (35 cases) — the rounding of AMD-040 §3 on the
eight values the amendments name, the same rounding reached through the model, the ladders of
AMD-055, the style identifiers 1/4/0/other, the defaults of the twenty-two keys, the library
view that keeps anything, and the published value lists. The transcription of SPEC-09 §4.1
(`tests/common/catalogue_table.rs`) gained the twenty-two rows, so the catalogue is still
checked against a table written out from the specification rather than from the code.

### Step 3 — the `CS` table and the framed distance

**REQ-LUA-112, REQ-LUA-117's shells** — `zax-scripting/src/runtime/cs_table.rs` (new), installed
from `runtime/mod.rs` right after the API globals. `CS` is made of plain Lua tables, not of host
values: nothing of it reaches the binding, which is why it does not appear in the API dump
either (the same as the .NET build after V2-2). Each level is a shell whose fields live in a
table only its metatable reaches:

- a field the amendments do not name reads nil, with no error — the shell itself is empty and
  `__index` is the field table;
- a write anywhere in the tree is dropped (`__newindex` is a function that does nothing), so it
  cannot reach the program and cannot disturb what the next script reads;
- `__metatable` hides the metatable, so `setmetatable` on any level raises and the seal cannot
  be taken off.

**REQ-LUA-113** — `Camera.ResetDistance`, read only. `ScriptCamera` gained `fov`, `set_fov` and
`reset_distance`, with defaults, so a host that binds a camera of its own keeps compiling;
`SceneCameraBridge` (application) and the harness bridge forward the three to the scene camera,
and `DetachedScriptCamera` holds a field of view of its own.

The field of view reaches the camera **before** the view reset answers it, in both places a
reset is answered (`api/settings.rs` `with_settings` and `runtime/mod.rs` `answer_view_reset`),
which is what the .NET lot did in its test harness. Without it a script that writes `Setting.Fov`
would read a framed distance of the previous field of view. The real camera ignores a write of
the value it already holds, so this costs nothing on the host path; it runs on a change of field
of view, never per frame.

**REQ-SCN-069 and REQ-SCN-045 come in with it.** REQ-LUA-113 *is* the unbounded value of
REQ-SCN-069, so the two formulas of AMD-045 landed in this step rather than in step 7:
`geometry::reset_distance` (`d₀ = 132.7964 / tan(FOV / 2)`, replacing `104 / atan(FOV_rad / 2)`)
and `geometry::hud_distance` (`76.6133 / tan(FOV / 2)`, replacing `60 / atan(FOV_rad / 2)`).
`SceneCamera::reset_distance` is the unbounded value; `reset_view` writes it through the
`[0, 512]` bound of REQ-SCN-011, as REQ-SCN-069 asks.

Measured, for R0's capture comparison: at 70° the two formulas agree to 6 · 10⁻⁵ absolute
(189.65291 against 189.65285, 3.6 · 10⁻⁷ relative) and the HUD distance is unchanged
(109.415 both). **No capture of the campaign moves on these two formulas**, every one of them
being taken at the default field of view. A capture at another field of view would move, and
that is the amendment's intent.

Test vector C4 of SPEC-06 §5 changes with the formulas: at 90° it read 156.21 and 90.12, and now
reads the two half-heights themselves (132.80 and 76.61), the tangent of the half angle being 1.
The test says so in place.

Tests: `zax-scripting/tests/scripting/two_zero_api_tests.rs` (7) — the two criteria of
REQ-LUA-112, an unnamed field, a dropped write, the hidden metatable, the criterion of
REQ-LUA-113 at 70°, 45° and 95° with a `Camera.Distance` write in between, the read-only refusal,
and the shape the `punchout` profile reads, run as a `Start` hook with no error.
`zax-render/tests/camera_tests.rs`: the three values of REQ-SCN-069, the agreement with the
former formula at 70°, and the three of REQ-SCN-045. `zax-app/tests/two_zero_camera_tests.rs`
(4) — the two copies of the formula agree at every field of view from 10° to 150° (the crate
`zax-scripting` carries its own, not depending on the renderer), the published `DistanceMax` is
the bound the camera applies, and the bridge hands out the scene camera's framed distance.

API dump: `goldens/lua-api.txt` gained exactly one line, `member CameraAPI.ResetDistance prop
L`, and the count went from 429 to 430 members (495 → 496 lines of file). The two line counts
asserted in the tests (`zax-harness/tests/lua_api_tests.rs`, `zax-goldens/src/lib.rs`) follow.

### Step 4 — the `Setting` members of AMD-040 and the export methods

**REQ-LUA-114** — four members and three methods on `SettingAPI`:

- `RunAheadFrames` and `VisualStyle` are ordinary whole members over `emu.run_ahead` and
  `render.visual_style`. Their normalisation is the setting's domain, so a write of 7 reads 3 and
  a write of 5 reads 0 without a line of arithmetic here, and a value from the settings file
  behaves the same (REQ-PERS-003).
- `GetVisualStyleParameter`, `SetVisualStyleParameter` and `ResetVisualStyleParameters` go
  through `ScriptSettings`, which asks `zax-settings::styles` which setting the identifier names
  and then `Domain::snap` to round. `SetVisualStyleParameter` takes the **number** the script
  passed, not a truncated integer: AMD-040 §3 rounds `(v − a) / step`, so 45 has to reach the
  rounding as 45.
- `ResetVisualStyleParameters` writes each default as a *script* write, so it lands in the
  session layer and the user's own preference comes back at the end of the game (REQ-PERS-010).
  A parameter's default is its own — `ao.strength` goes back to 3, not to 0.

**REQ-LUA-115** — `Instance:BuildExportMeshes()` and `ReleaseExportMeshes()`: declared, take
nothing, return nothing. They exist so that a 2.0 script that calls one does not lose every hook
it registered (REQ-LUA-062).

Tests, the criteria of the amendment one by one (6 more in `two_zero_api_tests.rs`, 13 in all):
the two scalar members with their bounds and the 1 → 4 alias, the four values of the
normalisation (500 → 180, 40 → 30, 45 → 60, 15 → 0), the four refusals of an unknown style or
identifier with nothing recorded, the reset of a style and the no-op on an unknown one, the three
methods called with `.` (REQ-LUA-081), and the two export methods on a pattern.

API dump: 7 lines more, 430 → 437 members — `Instance.BuildExportMeshes`,
`Instance.ReleaseExportMeshes`, `SettingAPI.GetVisualStyleParameter`,
`SettingAPI.ResetVisualStyleParameters`, `SettingAPI.RunAheadFrames`,
`SettingAPI.SetVisualStyleParameter`, `SettingAPI.VisualStyle`.

### Step 5 — AMD-059: the four scalar members and the list

**REQ-LUA-116, the four scalars.** `LibraryView` is an ordinary whole member (its setting has no
domain at all, so anything written reads back). The other three carry their own mapping, applied
on the Lua write by a new `mapped` declaration in `api/settings.rs`:

| Member | Mapping | Why not the domain |
|---|---|---|
| `MaxQueuedFrames` | v ≤ 1 → 1, else 2 | the domain's clamp gives the same answer, and the mapping says it in one line beside the other two |
| `RefreshRate` | v ≤ 81 → 72, v ≤ 105 → 90, else 120 | no domain can express "nearest rung, the smaller one at equal distance" |
| `Msaa` | v ≤ 1 → 0, v ≤ 3 → 2, else 4 | the general rule of REQ-PERS-003 sends an out-of-domain value to the **default**, which would read 2 where the amendment asks for 0 or 4 |

The value that reaches the model is therefore always inside its domain, which is what makes "no
numeric write raises an error" true whatever the number — checked from −10⁹ to 10⁹ and on
infinities.

**REQ-LUA-116, `VisualStyleParameters`.** Two new host types, `Liste<ParamStyle>` and
`ParamStyle`, and a write log in `ScriptSettings`.

The design question was what a read hands out. The requirement needs a **snapshot**: "a list
previously read puts exactly its elements back" means nothing if the list is a live view, since
by the time it is written back it already equals the current state. The .NET build kept the
snapshot as an object the binder held alive for as long as Lua referenced it. A Rust `Payload` is
`Copy` and holds no pointer — that is what makes the fast binding of `binding/raw.rs` sound — so
a handle on a host-side snapshot would need a slab that only grows, and a script reading the list
every frame would make it grow without end.

So the payload **is** the snapshot: each entry is the index of the parameter in the table of
`zax_settings::styles` and the index of its value among the rungs of its domain, one byte each,
sixteen entries and a count (33 bytes, under the 48 of `LuaXrState`, so `Payload` did not grow).
A style parameter only ever holds one of its rungs, the domain snapping every write, so the
value and the rung are a faithful round trip — asserted rung by rung for all sixteen parameters.
Consequences, all of them wanted:

- a list a script holds is frozen by construction, and nothing is kept alive on the host side;
- two reads with nothing in between give **equal** lists (`__eq` compares the payloads), which is
  the identity REQ-LUA-043 asks for; a write in between gives a different one;
- the element's `Style` is always 4, the identifier REQ-SCN-078 reads back, so a parameter
  written through the alias 1 shows up under 4;
- a unit test of `zax-settings` keeps the table at or under 256 parameters and every domain at or
  under 256 rungs: a lot that passes either bound has to widen the encoding, and that is where it
  finds out.

The log itself holds only **which** parameters were written and in what order (the order of the
first write); the values are the settings' own, read when the list is read. `nil` in writing
resets every parameter of every style and empties the log; a list puts exactly its elements back,
every other parameter going to its default first; anything else raises, the one write of the
requirement that may.

**REQ-LUA-117** was already done in step 3: the four tables of `CS.Nes3D` went in together,
`CameraControl`, `SettingManager` and `VisualStyle` beside `Info`. The tests of the three are
here.

Tests: 11 more in `two_zero_api_tests.rs` (24 in all) — the four criteria of the scalar members,
the "no numeric write raises" sweep, the criterion of the list (two parameters, `Count` 2,
element 0 with style 4, `spl.bend_zx` and 30, then `nil`), a list read before a write that puts
exactly its elements back, the order of the first write with an update in place, the alias
showing up under 4, the five writes that raise, the identity of REQ-LUA-043, the three tables of
REQ-LUA-117 with their nil fields and their dropped writes, and a script that writes **every**
member the amendment adds and still has its `Update` hook five frames later (REQ-LUA-062, the
point of the amendment).

API dump: 10 lines more, 437 → 446 members and 44 → 46 types — exactly the figures the .NET lot
V2-31 reported. `CS` is still absent from it, being made of plain Lua tables.

**One pixel of one synthetic capture moved**, and it is the two formulas of step 3 rather than
anything of this step: `goldens/render-synthetic/render.json`, scene `hud`, `backgroundPixels`
305787 → 305788. At 70° the new formulas differ from the old by 2.9 · 10⁻⁵ unit on the HUD plane
and 6.9 · 10⁻⁵ unit on the camera distance (2.6 · 10⁻⁷ and 3.6 · 10⁻⁷ relative), which is enough
to flip a pixel that sat exactly on an edge. The capture is still **100.000 %** of its pixels
within 2 levels of the C# golden, so the PNG is unchanged and only the exact count follows.

### Step 6 — AMD-042: the rollback carries `PData` and the camera

**REQ-LOOP-069.** `RollbackHistory` now holds `RollbackSnapshot { core: Vec<u8>, extras }`, with
`SnapshotExtras { persistent_data: Option<[u8; 256]>, camera: Option<CameraStateData> }`. Both
extras are fixed-size and `Copy`, so the allocation-free steady state of the Rust history
survives: `capture_with` still lends the oldest snapshot's **core buffer** to the next capture
when the history is full, and only replaces its extras. 30 × (core + 256 bytes + a camera state)
is what REQ-LOOP-069 asks for.

The capture site (`emulation/host/mod.rs`, in the frame loop, once every 60 emulated frames)
reads the `PData` block and the camera before taking the core buffer. Neither read runs a script
or lends anything to the Lua state: `GameSession::capture_rollback_data` goes straight to the
runtime through `engine.with_script_host`, which is the path `exchange_model` already used, and
reads 64 table entries. It is inside the `stage_clock` span the profiler already had for the
rollback, so the cost shows up where it belongs.

**REQ-LOOP-070.** `perform_rollback` now calls one `restore_snapshot`: the core, then the 64
`PData` values (`restore_rollback_data`, which writes them the way `SetPData` does), then the
camera through `SceneCamera::restore`. If the core refuses the state, nothing else is restored —
putting a script's data back beside a core that did not move would be worse than not moving at
all. No `Save` and no `Load` hook is called anywhere on this path, which a test checks by
reading what the hooks print.

**REQ-SCN-073 came in with it** rather than waiting for step 7: REQ-LOOP-070 requires the
restored camera to be shown **directly** at its setpoints, so `SceneCamera::restore` now snaps
the displayed values and resets the smoothing velocities, and the transition window comes back as
a remaining time clamped to `[0, 3]` s — which is how AMD-042 amends REQ-SCN-024, it spoke of a
deadline. `restore` is the single path a savestate load, a quick load and a rollback all take, so
the three snap the same way.

**REQ-LOOP-071 already held.** The Rust `stop_current_game(clear_current_game = true)` clears the
history, and that is what the end of a game — the return to the library — goes through. Nothing
was added for it; the acceptance criterion is now a test, which is what the .NET lot did too.

**AMD-042 adds no Lua member**, so `CORE_MEMBERS` is untouched (see the table above).
`separate_core_tests` (5) and `pause_3d_tests` (3) pass: the capture is a read of the core that
is **home** — it sits after `take_core_back()` in the tick, where the frame counter is read
anyway — so emulation-ahead and pipelining see no change.

What DEC-07-Q3 said is now the amendment's: the test
`dec_07_q3_a_rollback_leaves_the_camera_alone` is gone, replaced by
`req_loop_070_a_rollback_puts_the_camera_back_where_it_was`.

Tests: `rollback_tests.rs` 20 (5 new or replaced) — the camera put back and shown straight away,
the `PData` criterion of REQ-LOOP-070 (a profile writing `frame` into `PData[0]`, 10.5 s of play,
rollback, the entry back at 539 and no longer 629 — 539 because the snapshot is taken at the
**start** of the frame 540, before it runs), no `Save` or `Load` hook, the end of a game emptying
the history, and the extras travelling with the recycled core buffer.
`zax-render/tests/camera_tests.rs`: the two criteria of REQ-SCN-073 (a reloaded camera that does
not slide, and the window restored with its remaining time, reached by the smoothing for 2 s and
followed in the same frame afterwards); the REQ-SCN-024 test that asserted the **old** behaviour
("not snapped") now asserts the new one, naming the amendment.
