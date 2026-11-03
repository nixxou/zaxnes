# RO-5 — honest anti-aliasing labels, and finer steps for the arc centre

Branch `reworkoptions`, from RO-3 (`72c3f33`). Two owner requests, independent of each other:

1. the **anti-aliasing labels** say what the renderer really does — MSAA 4× and MSAA 8× instead of
   the specification's names FFAA and SMAA — everywhere the user reads them, and **nothing** else
   moves (the stored values, the settings file, the 3dSen import, the Lua API, the libretro value
   strings);
2. `scene.arc_center` takes **five** values, 0, 0.25, 0.5, 0.75 and 1, the intermediate ones
   placing the arc's pivot at the matching linear interpolation between the two heights AMD-043
   defines — with everything written for 3dSen behaving exactly as before.

Build directory: `rust/target-ro`.
`CARGO_TARGET_DIR=G:/3dsen/cleanroom/impl/rust/target-ro`, `--features zax-app/bundled` on
Windows (the workspace has no `SDL2.lib` on the library path).

## Checklist

| # | Step | State |
|---|---|---|
| 1 | The map of both parts, the decisions they force, the test plan | done — `e3a7b9d` |
| 2 | Part 1: the labels | done — `7cd57f7` |
| 3 | Part 2: the arc centre | done — `38b6b84` |
| 4 | Closing: the notes, Q-107, `OPTIONS-LAYOUT.md`, fmt, clippy, the full tests, the release build | done — this commit |

## Notes

### Uncommitted work found at the start (step 1)

`git status` showed the same three untracked things RO-1 to RO-3 each found and left alone, for the
same reasons: `tests/` at the repository root (the .NET test tree, removed by `d3680e4` and still
on disk) and the two scratch probes `rust/crates/zax-render/tests/dump_sources_tmp.rs` and
`probe_tmp.rs`, both headed "Scratch probe, not committed" and gated on `ZAX_PROBE`. Nothing of
RO-5 was found half-done. A wildcard `git add` under `rust/` sweeps the two probes into the index
every time (RO-1 and RO-2 each had to take them back out in a commit of their own), so this
lot stages files **by name**.

### The base the lot sits on, and the withdrawn lot (step 3)

The lot was first written on top of RO-4. The owner **withdrew RO-4** (patches always hooked); R0
reset the branch to RO-3's head `72c3f33`, kept RO-4 on `ro4-abandoned`, cherry-picked steps 1 and
2 as `e3a7b9d` and `7cd57f7`, and re-applied the working tree of step 3, which applied cleanly.
Checked before continuing, as R0 asked:

- `emu.patches_always_hooked` and every spelling of it: **nowhere** in `impl/` — not in the
  catalogue, not in `SettingKeys.reference.txt`, not in the two goldens, not in these notes;
- `docs/progress/RO-4.md` and `docs/spec/changes/Q-108.md` are gone with the reset;
- `git diff 72c3f33 HEAD` is the two string tables, `anti_alias_labels_tests.rs` and this file and
  **nothing else**, so the key lists and the two goldens are letter for letter RO-3's;
- the three places in these notes that named RO-4 — the base commit above and two sentences about
  the untracked files — are rewritten; the base commit that the measurements of part 2 compare
  against is `72c3f33`, which is the commit the pivot arithmetic is bit-compared with.

The step-2 note below records "zax-app: 759 passed". That figure was measured on the RO-4 base,
whose `patches_always_hooked_tests.rs` is 677 lines of tests now withdrawn; on this base the same
suite is **748 passed, 0 failed** with part 2 included. Nothing was lost — the figure's base moved.

### Step 1 — part 1: where the anti-aliasing labels come from

One place, and it is not the one the brief names. `table.rs::anti_alias_labels` is
`[opt(0, "None"), opt(1, "FFAA"), opt(2, "SMAA")]`, and that vector is used **twice**:

```rust
.with_domain(Domain::choice(anti_alias_labels.clone()))
.with_libretro(anti_alias_labels)
```

The **domain** labels are not interface text. They are:

| Reader | What it does with the label |
|---|---|
| `json::to_json` → `enum_json` | `video.anti_alias` is `SettingKind::Enum`, so the **settings file** holds the label: `"anti_alias": "FFAA"` |
| `json::from_json` → `enum_from_json` | reads a file back by label, through `Domain::value_of` |
| `core_options.rs` | the libretro value strings are `descriptor.libretro_values()`, which are these |
| `text::label_of_option(&option.label)` | looks the label up as `option.<label>` in the string table, and **falls back to the label itself** when there is no entry |

So changing the domain labels would rewrite the settings file and the libretro option values, which
the brief forbids. What the user sees goes through `text::label_of_option`, and there is **no**
`option.FFAA` or `option.SMAA` entry today — which is exactly why the user reads "FFAA".

**The change is therefore two string-table entries per language and nothing else.** Every place the
brief lists already funnels through `label_of_option`:

| Place | Path |
|---|---|
| the desktop options window | `zax_ui/options.rs::draw_enum_labelled` → `text::label_of_option` |
| the console global pages | `handheld/options.rs::value_text` → `handheld/advanced.rs::value_text` → `label_of_option` |
| the console game pages | `game_options.rs::console_value_text` → `options::value_text` → the same |
| the desktop *Game options* menu | `zax_ui/menu.rs::draw_game_rows` → `options::value_text` → the same |

The **tooltips** (`detail.video.anti_alias`) and the short help are their own texts and are rewritten
by hand, in both languages.

### Step 1 — part 1: the libretro value strings, and which half was changed

`core_options::CoreOption` carries `values: Vec<String>` — **one** string per value, which is at
once what RetroArch persists in its `.opt` file and what it displays. There is no second,
display-only field (libretro's `retro_core_option_v2` has one; this implementation does not expose
it). So "change only what is displayed" is not available here: changing the string would change the
persisted key and reset a user's choice.

> **Decision: the libretro values stay `None` / `FFAA` / `SMAA`.** Nothing changed for libretro.
> Recorded in Q-107 §7 and in `OPTIONS-LAYOUT.md`, with the note that giving libretro a display
> label of its own is a lot of its own (it would touch `core_options.rs` and the libretro shim).

### Step 2 — the labels, which turned out to be four string-table entries

Exactly what step 1's map predicted, and nothing more:

- `option.FFAA` → "MSAA 4×" and `option.SMAA` → "MSAA 8×", in **both** language tables (the two
  names are the same in French: a sample count is not a word). `option.None` already read "None" /
  "Aucun", so the three levels now read "None / MSAA 4× / MSAA 8×" and
  "Aucun / MSAA 4× / MSAA 8×".
- `detail.video.anti_alias` rewritten in both languages: the sample counts, then **3dSen calls them
  FFAA and SMAA, and so do the settings file and the libretro option; Zaxnes draws them as what
  they are**. The old sentence "The two names are the ones the specification gives the two levels"
  is gone — it was true and useless, and the new one tells the user where they will meet FFAA
  again. The default line now says "MSAA 4× on the desktop, None on a handheld".

No code changed. The four interfaces the brief lists all read the value through
`text::label_of_option`, which looks a domain label up as `option.<label>` and falls back to the
label itself — which is why the user read "FFAA" in the first place. `video.anti_alias` has **no**
short `help.*` sentence (`options::help_of` returns `None` for it), so the tooltip is the paragraph
alone and there was nothing else to rewrite.

The × is U+00D7, inside the atlas's first glyph range (`ui/fonts.rs::GLYPH_RANGES`,
`0x0020..0x00FF`) and inside the built-in font's Latin-1, so the console draws it — unlike the
mockup's `›` of RO-3. `text::speed_label` has been drawing a × on the same console page since
AMD-041.

`crates/zax-app/tests/anti_alias_labels_tests.rs` (new, 6 tests): the three levels in both
languages through the window's drop-down list **and** through `advanced::value_text` (which is the
console's global pages, the console's game pages and the desktop *Game options* menu, all three);
the row still where it was; the stored values, the domain labels, the default, field 32 and the
Lua path untouched; the settings file still holding `"SMAA"` and a file holding `"None"` / `"FFAA"`
/ `"SMAA"` reading back as 0 / 1 / 2; the libretro values still `None` / `FFAA` / `SMAA` and
`try_apply("SMAA")` still applying; the tooltip naming all four of FFAA, SMAA, MSAA 4× and MSAA 8×.

`zax-app`: **759 passed, 0 failed** (753 before, plus the 6 new). `cargo fmt --check` and
`cargo clippy -p zax-app --all-targets -- -D warnings` clean. (Figure measured on the RO-4 base;
see "The base the lot sits on" above — 748 on this base, the withdrawn lot's tests being gone.)

### Step 3 — part 2 as built, and the two surprises

The nine decisions of step 1 were carried out as written; nothing had to be reopened. The shape of
the change, file by file:

| File | What it does |
|---|---|
| `zax-settings/src/table.rs` | `ARC_CENTER_RUNGS` / `ARC_CENTER_LABELS`, the kind `Float`, the default `Float(0.0)`, the domain `Domain::grid(&RUNGS, 2).with_labels(..)`, the libretro list from the same two arrays |
| `zax-engine3d/src/space_bend.rs` | `arc_at_edge: bool` → `arc_center: f32`; the pivot is `EDGE_PIVOT_OFFSET · scale · fraction`, the fraction clamped and a non-number read as 0 |
| `zax-app/src/scene_script_bridge.rs` | `as_int() != 0` → `as_float()` |
| `zax-app/src/ui/options.rs` | `LABELLED_FRACTIONS` and `is_labelled_fraction` |
| `zax-app/src/ui/zax_ui/options.rs` | the `enumerated` predicate takes the list in; `draw_enum_labelled` writes a `Float` when the kind is `Float` |
| `zax-app/src/ui/handheld/advanced.rs` | `value_text` shows the rung's name for a labelled fraction |
| `zax-scripting/src/settings.rs` | the three reads become `as_double()`; `set_visual_style_parameter` and `reset_visual_style_parameters` write a `Float` when the descriptor is one |
| `zax-scripting/src/values.rs` | `number_format::to_f32`, the `(float)` cast beside the `(int)` one |
| `zax-harness/src/render/mod.rs` | `--bend`'s fourth part parses a number in `[0, 1]` |

**Surprise 1: the near end needed writing out, not multiplying.** `EDGE_PIVOT_OFFSET · s · 0.0` is
**`−0.0`**, the offset being negative. Numerically that is the same pivot everywhere it is used —
`−0.0 == 0.0`, and the shader's arithmetic agrees — but the uniform the renderer uploads would have
carried a different bit pattern than before the lot, which is not what "pixel-identical at 0 and 1"
should have to rest on. So `fraction <= 0.0` yields a literal `0.0`, and the test compares
`to_bits()`, not `==`, at both ends. The far end needs nothing: `x · 1.0 == x` exactly.

**Surprise 2: the two goldens did not move at all.** The test plan expected the RO-3 option tree
golden to gain the new values. `rust/goldens/handheld-option-tree.txt` and
`desktop-options-shown.txt` hold **keys**, never values (`scene.arc_center` appears three times
across the two, bare), so five rungs read exactly as two did. Nothing to update, and the two
goldens' own tests confirm it.

The one call-site cost step 1 predicted came due: 32 `SpaceBend::new(…, false|true, …)` in the
tests of four crates became `0.0` / `1.0`. Two of them read better as a conversion than as a
literal — `space_bend_tests.rs`'s own `bend()` helper, which keeps the amendment's two-value view
for the thirteen vectors of REQ-SCN-067, and the `#[case]` list of `zapper_input_tests.rs`, whose
`at_edge: bool` cases are REQ-INP-124's own.

#### What the tests check, and what they found

New: `space_bend_tests::ro_5_the_pivot_is_linear_between_the_two_heights` (both ends bit for bit
and the whole struct equal to the boolean's, the five pivots `0, −30, −60, −90, −120` strictly
monotone, the scene scale still dividing them, the clamping of a value out of range or not a
number, and two neighbouring rungs never giving the same image);
`bend_tests::ro_5_the_five_arc_centres_export_five_pictures_and_the_two_ends_are_unchanged` (the
exported vertices at the two ends identical to those built from the two pivots REQ-SCN-066 states,
the ten pairs of rungs all distinct, the middle rung's lowest vertex between the two ends');
`two_zero_settings::ro_5_the_arc_centre_snaps_on_the_quarters` (the whole table of decision 6,
0 and 1 exact, the ties away from zero, and the script's grid equal to the menu's at 201 probes
from −0.5 to 1.5); `settings::ro_5_a_script_writes_and_reads_the_five_rungs_of_the_arc_centre`,
`ro_5_a_reset_takes_the_arc_centre_back_to_the_screen_centre`,
`ro_5_the_write_log_carries_an_intermediate_arc_centre_there_and_back` (REQ-LUA-116's round trip
putting 0.75 back exactly, at rung 3 of 5); `bend_options_tests::ro_5_…_named_list…` and the two
rewritten cases there (the five rungs, their labels in both languages, the pad walking the five a
quarter at a time and stopping at each end, the three other stepped reals staying sliders);
`bend_settings_tests::ro_5_a_stored_arc_centre_loads_unchanged_and_the_quarters_survive_a_restart`
(the file still reading `"scene.arc_center": 1`, each quarter surviving a save and a load, a
hand-edited 0.4 landing on 0.5); `game_config::ro_5_an_intermediate_arc_centre_of_a_game_survives_a_restart`
(`0`, `0.25`, `0.75`, `1` as the exact JSON text, read back through `games.json`);
`core_options` extended (the five values, the default still `Screen Center`, and each of the five
labels applying through `try_apply`); `render_report_tests::ro_5_the_bend_centre_takes_any_fraction`
(the quarters, a 0.1 off the grid, and `1.5` / `-0.1` / `edge` / empty all refused).

The decisive compatibility check is the settings file's: `"scene.arc_center": 1` is still the text
written, so a profile, a `games.json` or a settings file from before this lot is read back bit for
bit — which is what decision 1 promised and the one thing a `Float` kind could have broken.

Release tests of every touched crate, one at a time, foreground: `zax-settings` **255 passed**,
`zax-engine3d` **235 passed, 6 ignored**, `zax-scripting` **170 passed, 12 ignored**, `zax-app`
**748 passed, 1 ignored**, `zax-harness` **49 passed**, `zax-render` **287 passed, 1 ignored** —
**0 failed** anywhere. `cargo fmt --check` and
`cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` clean.

Per-frame cost: none. `SpaceBend::new` runs when a curvature setting changes
(`scene_script_bridge::bend_of`), the pivot it computes is the uniform `uBendPivot` that was
already uploaded once per change, and the three shader paths the brief names — the scene pass, the
shadow pass and the 2D-mode rectangle — read that one uniform, so **no shader source changed**.
The clamp and the branch are two instructions on a path taken when a menu row moves.

### Step 1 — part 2: what `scene.arc_center` is at `72c3f33`

| | |
|---|---|
| descriptor | `SettingKind::Int`, `SettingValue::Int(0)`, scope `User`, effect `Immediate`, theme `Scene` |
| domain | `Domain::stepped(0.0, 1.0, 1.0, 0).with_labels(&["Screen Center", "Screen Edge"])` |
| libretro | `[opt(0, "Screen Center"), opt(1, "Screen Edge")]` (AMD-043 §6: "option, 2 valeurs") |
| Lua | **no** `Setting` member; reachable only as the style parameter `spl.arc_center` (REQ-LUA-114) |
| settings file | `json::to_json` of an `Int` is `Value::from(i32)`: the text `0` or `1` |
| read by | `scene_script_bridge::bend_of`: `model.get(SceneArcCenter).as_int() != 0` |
| used by | `SpaceBend::new(.., arc_at_edge: bool, ..)` → `pivot = if arc_at_edge { EDGE_PIVOT_OFFSET * scale } else { 0.0 }` |
| reaches the GPU as | the single uniform `uBendPivot` (`renderer.rs::bind_bend`), read by `bend.glsl` |

### Step 1 — part 2: the decisions the map forced

**1. The setting becomes `SettingKind::Float`. There is no other way, and the settings file does
not change for 0 and 1.** `Domain::coerce` dispatches on the written value's *variant*, and
`SettingValue::Int` is rebuilt with `num::truncate_to_i32`, so an `Int`-kinded setting cannot hold
0.25 at all. Three things make the change safe where it matters:

- `SettingsModel::write` calls `value.coerce_to(descriptor.kind())` **before** the domain, so every
  write — Lua, the window, a game's options, the command line, the file — is normalised to the
  declared kind first. A write of `Int(1)` becomes `Float(1.0)`; every existing caller keeps working.
- `json::to_json` of a `Float` is `float_json`, "the shortest text that reads back as the same
  float". `0.0` writes `0` and `1.0` writes `1` — **byte for byte the text an `Int` wrote**. So a
  settings file, a `games.json` and a profile written by an older Zaxnes read back unchanged, and a
  file this lot writes is unchanged too unless the user picks an intermediate value.
- `json::from_json`'s `Float` branch reads any JSON number, so the `0` and `1` of an old file load.

Consequence to record: SPEC-09 §4.1 gives the key the type "entier", and
`tests/common/catalogue_table.rs` is an independent transcription of that table, so its row moves
from `Whole` to `Real`. That is the divergence, and it is Q-107 §8's subject.
`SettingKeys.reference.txt` holds **keys only** and does not change — and neither does the file
format of the key for the two values that existed.

**2. The domain is a `Domain::grid`, not a `Domain::stepped`.** The brief asks for "the nearest
0.25, **ties away from zero**". `Domain::stepped` snaps with `round_ties_even`, so 0.125 would go
to 0.0 and 0.375 to 0.5 — ties to the even rung. `Domain::grid` snaps to the nearest published
rung, ties to the one of **greater magnitude**, which is away from zero and is letter for letter
the rule RO-2 part 3 gave the three angles. So the two parts of the branch stay one rule:

```rust
Domain::grid(&[0.0, 0.25, 0.5, 0.75, 1.0], 2).with_labels(&[…])
```

`Domain::grid` answers `step() == None`, which costs nothing here: `ui::options::bounds` takes the
step from the **libretro list** when there is one, and there is — 0.25, which is what the console
page and the desktop slider move by.

**3. `SpaceBend::new` takes the fraction, not a flag.** `arc_at_edge: bool` becomes
`arc_center: f32`, and the pivot is `EDGE_PIVOT_OFFSET * scale * fraction` with the fraction
clamped to `[0, 1]`. Two reasons for one constructor rather than a second one:

- the pivot is **already** a single value the renderer uploads once per change (`uBendPivot`), read
  by `bend.glsl` for the scene pass, the shadow pass and the 2D-mode rectangle alike. So the
  fraction reaches all three with **no shader change and no new per-frame work** — which is what
  the brief's "the pivot height is a uniform computed when the setting changes" asks for;
- at 0.0 and 1.0 the arithmetic is bit-exact (`x * 1.0 == x`, `x * 0.0 == 0.0` for a finite `x`),
  so the two ends are **pixel-identical** to `72c3f33` by construction, not by measurement.

The price is that every `SpaceBend::new(…, false, …)` in the tests becomes `0.0` and `true` becomes
`1.0`: 30-odd call sites, all mechanical, none of them a behaviour change.

**4. The window and the console draw it as a named list, which needs one explicit marker.**
`zax_ui/options.rs::draw_row` decides on a drop-down with
`domain().options().is_some() && matches!(kind, Enum | Int)`, and `advanced::value_text` shows a
label for `Enum | Int` and a formatted number otherwise. Adding `Float` to either predicate would
turn `audio.volume`, `camera.fov` and `light.intensity` — stepped float domains whose rungs are
labelled with their own numbers — into 21-entry drop-downs. So a new explicit list, in the idiom
of `options::SWITCHES`:

```rust
/// The settings whose value is a fraction the interface shows as a **named** list.
pub const LABELLED_FRACTIONS: [SettingId; 1] = [SettingId::SceneArcCenter];
```

read by the window's `enumerated` predicate and by `advanced::value_text`, with
`draw_enum_labelled` gaining a `Float` branch for the value it writes back. Nothing else about
either interface changes: the console steps the row by its 0.25 step through the branch that was
already there, and `game_options::walk` finds five texts where it found two (its `LIMIT` is 48).

**5. The Lua surface keeps its type, because it was never an integer.**
`ScriptSettings::visual_style_parameter` returns `f64` and `GetVisualStyleParameter` hands that to
Lua, so a read of `spl.arc_center` has always been a Lua number — 0.0 and 1.0 before, 0.0 and 1.0
now, 0.5 when the user picks the middle. What has to change is **where the f64 comes from**:
`f64::from(self.int(key))` rounds a 0.5 to 1. It becomes the double read, which for the other
fifteen parameters is the same number bit for bit (`as_double()` of an `Int` is `f64::from(i32)`).
`written_style_parameters` has the same line and the same fix.

`set_visual_style_parameter` writes `write_int(key, to_i32(snap_write(…)))`. `to_i32` **truncates**,
so it keeps a `Float` branch out of its way: the function now writes a `Float` when the
descriptor's kind is `Float` and takes the old path otherwise, so the fifteen others are untouched
to the bit.

`goldens/lua-api.txt` is therefore unchanged, and no member's declared type moves — so the brief's
"stop and ask in a Q ticket instead" does not arise.

**6. What a Lua write of a value off the grid gives, and what a read gives back.**

| Written by a script | Before | Since RO-5 |
|---|---|---|
| 0 | 0 | 0 |
| 1 | 1 | 1 |
| 0.25 / 0.5 / 0.75 | 0 / 1 / 1 | 0.25 / 0.5 / 0.75 |
| 0.1 | 0 | 0 |
| 0.4 | 0 | **0.5** |
| 0.6 | 1 | **0.5** |
| 0.125 (a tie) | 0 | **0.25** (away from zero) |
| 0.375 (a tie) | 0 | **0.5** |
| 7 / −7 | 1 / 0 | 1 / 0 |

A read gives back exactly what the setting holds: `0` for 0, `1` for 1 — the same number and the
same Lua type as before — and `0.5` when the user chose the middle.

**7. The rung indices of `Setting.VisualStyleParameters` move for `spl.arc_center`.** The published
list goes from 2 rungs to 5, so rung 1 was 1.0 and is now 0.25. This is the same consequence RO-1
recorded for the three angles (Q-107 §2, consequence 2) and the same reasoning applies: the
requirement says "the rung each one holds now", so reporting a rung of the old 2-entry list would
report a value the setting does not hold. The round trip REQ-LUA-116 promises still holds exactly,
`written_style_parameters` and `restore_visual_style_parameters` reading the same list. Recorded in
Q-107 §8.

**8. The libretro list grows from 2 values to 5, and no persisted choice is lost.** `try_apply`
matches a RetroArch value by **label**, so an `.opt` file holding `Screen Center` or `Screen Edge`
still applies; the three new labels are new strings nobody has persisted. The list is also what
`ui::options::bounds` takes the console's step from, so the two have to agree — which is why it
grows rather than staying at two. AMD-043 §6 writes "2 valeurs": the divergence is Q-107 §8's.

**9. The harness `--bend` centre takes any fraction.** `parse_bend` accepted the literal strings
`"0"` and `"1"` and nothing else. It now parses a number and refuses anything outside `[0, 1]`,
which is what the monotone-pivot test and any capture campaign between the two heights need. The
angles already took anything, on or off the grid, deliberately.

### Step 1 — the test plan

| Part | Test | File |
|---|---|---|
| 1 | the three levels read "None / MSAA 4× / MSAA 8×" in English and "Aucun / MSAA 4× / MSAA 8×" in French, through `label_of_option` | `anti_alias_labels_tests.rs` (new) |
| 1 | the **domain** labels are still `None` / `FFAA` / `SMAA`, so the settings file is untouched | `anti_alias_labels_tests.rs` |
| 1 | a settings file holding `"FFAA"` loads as 1, and writing 1 writes `"FFAA"` again | `anti_alias_labels_tests.rs` |
| 1 | the libretro value strings are still `None` / `FFAA` / `SMAA` | `core_options.rs` (existing, kept) |
| 1 | the 3dSen import of field 32 is unchanged | existing import tests |
| 1 | `Video.AntiAlias` and `Setting.Msaa` read and write the same numbers; `goldens/lua-api.txt` byte for byte | existing (`two_zero_api_tests`, `lua_api_tests`) |
| 1 | the tooltip and the help say MSAA 4×/8× and name FFAA/SMAA as 3dSen's names | `anti_alias_labels_tests.rs` |
| 2 | the domain is five rungs 0, 0.25, 0.5, 0.75, 1 with the five labels, in both languages | `bend_options_tests.rs` |
| 2 | the pivot at 0 and at 1 is bit-identical to `72c3f33` (0 and `−120 · s`), and the exported scene at both is unchanged | `space_bend_tests.rs`, `bend_tests.rs` |
| 2 | the pivot is **monotone** and linear in the setting: 0.25 → −30, 0.5 → −60, 0.75 → −90 | `space_bend_tests.rs` |
| 2 | an intermediate value really bends differently: the exported positions at 0.5 are between those at 0 and at 1 and equal to neither | `bend_tests.rs` |
| 2 | a Lua write of 0 or 1 behaves exactly as before; the whole table of decision 6 | `zax-scripting::settings::tests`, `two_zero_settings.rs` |
| 2 | a Lua read of 0 and of 1 gives the same number as before, and 0.5 gives 0.5 | `zax-scripting::settings::tests` |
| 2 | the `Setting.VisualStyleParameters` round trip still puts an intermediate value back exactly | `two_zero_api_tests.rs` |
| 2 | a settings file written by an older Zaxnes (`0`, `1`) loads unchanged, and 0.5 survives a restart | `bend_settings_tests.rs` |
| 2 | a per-game 0.75 survives a restart through `games.json` | `game_config.rs` |
| 2 | the console page steps 0 → 0.25 → … → 1 and stops, showing the five labels | `bend_options_tests.rs` |
| 2 | the libretro option publishes the five values and still accepts `Screen Edge` | `core_options.rs` |
| 2 | the RO-3 option tree golden: the new values only, no row moved | `handheld_tree_tests.rs`, `goldens/handheld-option-tree.txt` |
| 2 | `--bend 0,0,60,0.5` parses and gives the interpolated pivot; `1.5` is refused | `render_report_tests.rs` |
| both | `goldens/desktop-options-shown.txt`, `SettingKeys.reference.txt`, `goldens/lua-api.txt` unchanged | their own tests |

### Step 1 — what may not change

- `goldens/lua-api.txt`: no member added, removed or retyped. Both parts leave the Lua surface's
  shape alone.
- `goldens/desktop-options-shown.txt` and `EXCLUDED`: no setting is added or removed, and neither
  part moves a row to another tab.
- `crates/zax-settings/tests/data/SettingKeys.reference.txt`: no new key, and no change to the file
  format of `scene.arc_center` for the values `0` and `1` (decision 1).
- the settings file's `video.anti_alias`: still the label `None` / `FFAA` / `SMAA` (part 1 changes
  no domain label).
- the libretro value strings of `video.anti_alias` (the decision above).
- the handheld defaults (`ui/handheld/defaults.rs`): no setting is new, and `scene.arc_center` keeps
  its own default of 0 on both platforms, which costs nothing (the pivot is read only while the arc
  is active).
- the per-frame paths: part 1 touches string tables only; part 2's pivot is computed in
  `SpaceBend::new`, which `scene_script_bridge::bend_of` calls when a curvature setting changes and
  never per frame, per tile or per pixel, and reaches the GPU as the uniform that was already there.

### Step 4 — the closing, and the state the lot is left in

**Q-107 gains two sections**, 7 and 8, in the shared ticket
(`spec/changes/Q-107.md`) and in its copy (`impl/docs/spec/changes/Q-107.md`), the two byte for
byte identical:

- **§7**, the anti-aliasing labels: what AMD-032 §2 names, what it decided the levels *do*, the
  owner's point, the three labels in both languages, the rewritten tooltip, the six things kept
  (the values, the settings file, field 32, the Lua API and its golden, the row's place), the fact
  that the change is four string-table entries and no code, and the reservation on libretro — one
  `values` vector that is both persisted and displayed, so the two acronyms stay there and a
  display label of libretro's own is a lot of its own. R1 is asked whether it wants one.
- **§8**, the arc centre: the five rungs and their labels, the pivot formula, the uniform the
  fraction travels through, the three consequences (the kind becoming real with the settings file
  unchanged for 0 and 1, the grid rather than the step so that ties go away from zero as §3 does
  for the angles, the libretro list growing from two values to five without losing a persisted
  choice), the 3dSen compatibility table case by case, the bit-exactness of the two ends, and the
  reservation on `LABELLED_FRACTIONS` — an explicit list rather than a declarative rule, R1 asked
  whether it would prefer a marker on the descriptor.

The ticket's header records RO-5 and adds AMD-032 §2, REQ-LUA-114 and SPEC-09 §4.1 to the
requirements it concerns.

**One correction in the shared ticket, and why it is not a deletion.** §1 carried a paragraph
announcing that RO-4 would add `emu.patches_always_hooked` and one line to each of the two goldens.
The owner withdrew the lot, so the sentence asserted something false about the current state. It is
rewritten to say that the exception no longer exists and that the two goldens are RO-3's — rather
than removed, because **Q-108** describes the withdrawn lot and would otherwise read as being in
force. Q-108 itself is untouched: it is R0's to retire.

**`rust/docs/OPTIONS-LAYOUT.md` gains §6 and §7**, the same two divergences written for the
implementation side (the mapping, what is kept, the consequences for R1, the harness change). Its
header records RO-5 and states plainly that RO-4 was withdrawn and appears in no section, so that
nobody reading the note looks for a seventh divergence that was never merged. §2's line
"`scene.arc_center` is not concerned" now points at §7 — it was true of the angles' grid and would
have read as "the arc centre has no grid".

**The code references the ticket** where the ticket says it does: `// Q-107 §7` in the two string
tables (`english.rs` had it, `french.rs` gained it in this step), `// Q-107 §8` in
`zax-settings/src/table.rs`, `zax-app/src/ui/options.rs`, `space_bend.rs` (added in this step, the
interpolation being there) and in the two test files that record the type change.

**The spec copy.** RO-5 implements no amendment — it is two owner requests that *diverge* from two
already-implemented ones — so there was nothing new to copy.
`docs/spec/changes/AMD-032-anticrenelage-echelle-resolution.md` and `AMD-043-courbure-espace.md`
are both present and byte-identical to the spec folder's; Q-107 is the one file this lot syncs.

#### The final run

Foreground, one suite at a time, `CARGO_TARGET_DIR=rust/target-ro`, `--features zax-app/bundled`:

| | |
|---|---|
| `cargo fmt --check` | clean |
| `cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` | clean |
| `cargo test --release --workspace --features zax-app/bundled` | **2482 passed, 0 failed, 84 ignored** |
| `cargo build --release --features zax-app/bundled` | `rust/target-ro/release/zaxnes.exe` |

The 84 ignored are the ones that need a real 3dSen installation (`ZAXNES_3DSEN_INSTALL`) plus the
GPU-bound ones, as on every lot of this branch; R0 runs them against real 1.1.2 and 2.0
installations. No GPU stress campaign was run.

The two scratch probes and the root `tests/` tree are still untracked and still on disk, as RO-1 to
RO-3 left them; every commit of this lot staged files **by name**, so neither probe ever entered
the index.

#### What R0 should measure, and what the lot claims

Part 1 changed no code at all, so no profile can move. Part 2 adds, to a function that runs when a
curvature setting changes, a finiteness test, a clamp and a branch; the uniform it produces was
already uploaded once per change, and **no shader source changed**, so a callgrind profile that
uses nothing new should be identical within noise. The claim worth checking on real hardware is the
pixel-identity of the two ends: a curvature capture of RU-4a at `--bend 0,0,60,0` and
`--bend 0,0,60,1` must match the pre-lot capture byte for byte, which is what the bit-comparison of
`arc_pivot()` and the vertex-for-vertex export test assert in software.
