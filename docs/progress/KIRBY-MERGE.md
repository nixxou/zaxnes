# KIRBY-MERGE — bring `feat/widescreen-kirby` onto `dev`

Lot: merge the branch `feat/widescreen-kirby` (36 commits, diverged from `dev` at `514570fb`,
2026-11-06) into `dev` (`19c5065d`, 506 commits since the same point). Despite its name the
branch carries five deep mechanisms besides Kirby's pack: the widescreen extension's per-page
sprite high bytes, the wide 2D picture's margins at a split, run-ahead up to 10, the experimental
smooth scrolling at 120 Hz, and Kirby's margin decoder with its hack-pack aids.

Steps: 1 plan · 2 merge · 3 behaviour · 4 measurements · 5 final pass.

- [x] **Step 1/5 — plan** (this note)
- [x] **Step 2/5 — merge and resolve** (§2)
- [x] **Step 3/5 — behaviour** (§3)
- [x] **Step 4/5 — measurements** (§4)
- [x] **Step 5/5 — final pass** (§5)

---

## 1. Step 1: the plan

### 1.1 What each side is

`git log --oneline 514570fb..feat/widescreen-kirby` — 36 commits, in six groups:

| Group | Commits | What it touches |
|---|---|---|
| Kirby's margin decoder | `aa4c546a` … `2e76ec0f`, `6209885e`, `1478b267` | new `zax-nes/src/wide_decode/kirby.rs` (269 lines), `wide_decode/mod.rs` (three new trait methods, two registry arms) |
| The wide 2D picture | `672b8586`, `9c55f3a1`, `05a0386b`, `8db4bbb3` | `ppu/tiles.rs`, `ppu/mod.rs`: `shown_palette`/`shown_mask`, `rows_to_decode`, `row_ends`/`record_end`, `minimum_rows`, split strips |
| Smooth scrolling | `3e162037`, `650235e5`, `1711174b`, `88fbb9ac`, `d9f0552f`, `10f8d843`, `5ecababf` | `nes_core.rs` (+316), `wide.rs` (`CameraReport`), `ppu/tiles.rs` (`keep_line_scroll`, `playfield_scroll`, `split_steady_sprites`), `zax-render` (`picture_shift`, `picture_overlay`, `picture_bands`, `Residency::overlay`), `zax-app` (`phase_at`, `display_rate`, `tint_smooth_band`), `zax-settings` (`emu.smooth_scroll_ahead`, `emu.smooth_scroll_view`) |
| Run-ahead to 10 | `3eeb0165`, `4b768584` | `NesCore::MAX_RUN_AHEAD` 3 → 10, the `emu.run_ahead` domain 0..3 → 0..10 with eleven labels, a game's own run-ahead row in `game_options.rs`, `advanced::step_by_domain` |
| The widescreen extension | `b4c463e3`, `5e6eb7a2` | `$5007` bit 5 + `$500C`: sprite high bytes and flags **per sprite page**; option bits `u8` → `u16` (`aids_on`, `host_options`, `hack_option_bits`, the harness's `--hack-options`) |
| Kirby's pack | `0ce72d98`, `c35ce6f1`, `2124776f`, `b1983df5`, `e9a08b99`, `17d1a6d2`, `5a693b73`, `da13ef77`, `5b1ecafc`, `1a99706a`, `aa569117`, `5636604e` | seven `.zxpk` packs (one per build), seven `PATCHES` entries (`PATCHES` 57 → 64), `KIRBY_OPTIONS` (eight options: `input_priority`, `invincible`, `level_select` bit **8**, `no_slowdown`, `margin_enemies`, `margin_freeze`, `margin_freeze_tint`, `kirby_attacks_4_3`, `smooth_scroll`) |

`dev`'s 506 commits since `514570fb` are the mapper lots (MF-\*, MR-1, PR-1), the render lots
(AMD-043 bend, AMD-049 backdrop, AMD-055 styles/MSVO/SSR, AMD-057, AMD-058, AMD-060 SMAA), the
options reorganisation RO-1/RO-2/RO-3/RO-5, AUDIO-1/AUDIO-2, LUA-MERGE (the editor mode) and the
goldens regeneration `ab68ee48`.

### 1.2 The 27 files both sides changed

`comm -12` of the two `--name-only` lists. **Eight** of them conflict textually (confirmed by
`git merge-tree --write-tree dev feat/widescreen-kirby`); the other nineteen auto-merge, which
does not mean they are safe — the semantic conflicts of §1.4 live there.

| File | `dev` did | the branch did | conflict |
|---|---|---|---|
| `zax-nes/src/nes_bus.rs` | rewrote the whole DMA path into a state machine (`DMA_HALT`/`DMA_DUMMY`/`DMA_DMC`/`DMA_OAM`, REQ-CORE-113) with a **fast OAM loop**: `latch_sprites()` is now called from **two** sites | changed the one call site to `latch_sprites(page)` | **textual** |
| `zax-nes/src/ppu/mod.rs` | +848: `OPAQUE_PIXEL`, the read-buffer window, `run_visible_span`, the MMC5 hooks, the sprite-limit modes | +6 fields (`shown_palette`, `shown_mask`, `playfield_end`, `wants_playfield_end`, `line_scroll`, `tracks_line_scroll`, `tinted_sprites`) and the vblank snapshot | **textual** |
| `zax-nes/src/ppu/tiles.rs` | MF-LEG-2 split `capture_background_row` into `capture_background_row_as::<SUBSTITUTED>` + an `#[inline(never)]` cold arm; the `colour` closure now ORs `OPAQUE_PIXEL` (AMD-050 REQ-CORE-049) | +408: `rows_to_decode`, `row_ends`, `record_end`, `find_playfield_end`, `keep_line_scroll`, `playfield_scroll`, `split_steady_sprites`, `composed_entry`, `entry_colour`, `asleep_tint`/`TINT_ENTRIES`, and `palette` → `shown_palette` everywhere in the wide picture | **textual + semantic** |
| `zax-render/src/renderer.rs` | +1442: the bend variants (`picture_quad_bent`, `wide_picture_cells`, `wide_quad_for_subdivided_cached`), the style/MSVO/SSR/SMAA passes, `ScenePrograms` | `overlay: NesPictureTexture`, `picture_bands`, `GlResidency::overlay` | **textual** |
| `zax-render/src/wgpu_backend/renderer.rs` | +1772, the same lots | the same three additions | **textual** |
| `zax-app/src/ui/handheld/advanced.rs` | RO-3 turned the flat page into `ROW_SETTINGS: [SettingId; 57]` + `is_named_list`/`rungs_of`/`step_by_rung` | `step_by_domain` for a setting with no row, `value_text` labels for `Int`, an `Int` increment floor of 1 | **textual + semantic** |
| `zax-app/src/ui/handheld/defaults.rs` | RO-3/AUDIO-2 moved the handheld defaults around | `defaults()` 10 → 11 entries, `EmuSmoothScrollAhead` = false | **textual** |
| `zax-app/src/ui/handheld/game_options.rs` | RO-3 reshaped `GAME_ROWS` | `GAME_ROWS` 16 → 18 with `Setting(EmuRunAhead)` and `Setting(EmuSmoothScrollAhead)`, two `row_settings` arms, a test module | **textual** |
| `zax-nes/src/nes_core.rs` | +170 (the MMC5 variant switch, the pack zone, the state blocks) | +316: the smooth scrolling, `MAX_RUN_AHEAD` 10, `u16` options, `hack_option_on`, the pack's `idle_loop` fallback | auto |
| `zax-nes/src/wide.rs` | the `#[inline(always)]` note on `zone_offset` (MF-5b-fix: **+16.6 M** on Zelda II out of line) | `CameraReport`, `take_camera_report`, `latch_sprites(page)`, `page_high`/`page_flags`/`built_page`, `pending_flags`/`active_flags`, `sprite_tinted`, `clear_overclock` | auto |
| `zax-nes/src/builtin_patches.rs` | reworked the Zelda II test to use `pack_for` | seven Kirby patches, `PATCHES` 57 → 64, `KIRBY_OPTIONS` | auto |
| `zax-settings/src/table.rs` | +222 (the curvature grid, the style parameters, SMAA level 3, `finite_real` clipping, `audio.latency_ms`, `audio.classic_output`, `render.backdrop_lighting`) | the run-ahead domain 0..10 and `video_smooth_scroll()` | auto |
| `zax-settings/src/catalogue.rs`, `tests/catalogue.rs` | three new ids + the additions list | two new ids + the additions list | auto |
| `zax-render/src/frame_plan.rs` | +213 (`DrawLists`, `Residency::texture` → `Option`, the bend) | `Residency::overlay`, `FixedMeshes::picture_bands`, the band path in `collect_picture_quad`, `OVERLAY_NUDGE` | auto |
| `zax-render/src/interop.rs` | +232 (the subdivided quads for the bend) | `picture_bands_for`, `picture_band`, `PictureBands` | auto |
| `zax-render/src/scene.rs`, `wgpu_backend/resources.rs` | the style/bend fields | `picture_shift`, `picture_overlay`; the overlay texture slot | auto |
| `zax-app/src/application.rs` | +many (the style wiring, the editor mode, the profiler) | `display_rate`, `log_rates`, `frame_time`, `picture_between`, `tint_smooth_band`, the band/overlay plumbing in `update_scene` | auto |
| `zax-app/src/game_session.rs` | the editor mode, the checkpoints | `picture_shift`/`picture_motion`/`overlay` in `FramePipeline`, `copy_nes_picture_layers`, three accessors | auto |
| `zax-app/src/emulation/host/mod.rs`, `host/hack_options.rs` | AUDIO-2, the editor | `display_rate`, `hack_option_bits` → `u16` | auto |
| `zax-app/tests/run_ahead_tests.rs` | — | the 0..10 bounds and the eleven labels | auto |
| `zax-harness/src/bench_core.rs`, `render/mod.rs`, `render/rom.rs` | the LUA-MERGE help reflow | `u16` hack options, `--smooth-ahead`, `ZAXNES_TRACE_WRAM`, `smooth_scrolling()` | auto |

### 1.3 The eight textual conflicts and how they will be resolved

The rule for the three core files is `dev`'s: **keep `dev`'s measured shapes and fit the branch's
additions into them**, never the other way round.

1. **`nes_bus.rs`** — `dev` has two `latch_sprites()` sites (the fast OAM loop at ~731 and the
   per-cycle put at ~770), the branch has one taking `page`. Keep `dev`'s two sites and pass
   `self.dma_oam_page` at both. The branch's `page` local does not exist in `dev`'s state machine;
   `dma_oam_page` is the same byte.
2. **`ppu/mod.rs`** — keep `dev`'s field order verbatim and append the branch's six fields in the
   **cold tail** of the struct, beside `hides_overscan`/`margin_palette`, rather than in the
   middle next to `palette` where the branch put `shown_palette`/`shown_mask`: inserting 33 bytes
   before `decay_stamp`/`timing`/`dot` moves every hot field's offset, and MF-2 step 0 is the
   note on what that costs in the largest function of the crate. The per-frame copy
   (`shown_palette = palette` in the vblank branch) is the same work wherever the field sits.
3. **`ppu/tiles.rs`** — two things at once:
   * keep `dev`'s `capture_background_row` / `capture_background_row_as::<SUBSTITUTED>` /
     `capture_background_row_substituted` split untouched (MF-LEG-2: +8.9 M and +7.5 M over 300
     frames on `duelito` for the two shapes that were rejected);
   * the `colour` closure of `wide_picture_into` is changed by **both** sides — `dev` ORs
     `OPAQUE_PIXEL` on every non-zero entry (AMD-050 REQ-CORE-049), the branch adds the
     `TINT_ENTRIES` arm and reads `shown_palette`. Both are needed. The branch also added
     `entry_colour`, which is the same mapping **without** the opacity bit, used by
     `split_steady_sprites` to write into the very picture `wide_picture_into` produced. Keeping
     both sides literally would leave the overlay's pixels with bit 6 clear, which
     `gl/picture.rs` reads as transparent. Resolution: make `entry_colour` the single
     implementation (tint arm + `shown_palette` + the opacity bit) and have the `colour` closure
     call it. See §1.4 (S1).
4. **`zax-render/src/renderer.rs`** and 5. **`wgpu_backend/renderer.rs`** — the branch's three
   additions (`overlay` texture, `picture_bands`, `FixedMeshes::picture_bands`) drop into `dev`'s
   much larger `SceneRenderer`/`WgpuSceneRenderer` and `prepare` bodies. `dev`'s
   `wide_quad_for_subdivided_cached` + `wide_picture_cells` block stays; the band block is added
   after it.
6. **`advanced.rs`** — `dev`'s `ROW_SETTINGS`/`step_by_rung` already does what the branch's
   `value_text` and `step` edits were for (a whole number with labelled rungs reads and steps as
   a list). What is **not** covered is a setting the console shows as a game-option row but which
   is not in `ROW_SETTINGS`: `row_of` returns `None` and the row does not move. Keep `dev`'s
   shape and keep the branch's fallback, widened to flip a `Bool` (see §1.4 (S2)).
7. **`defaults.rs`** — take `dev`'s ten entries and the branch's eleventh; the array length
   becomes 11 and the value becomes the one §1.4 (S4) settles.
8. **`game_options.rs`** — take `dev`'s `GAME_ROWS` and insert the branch's two rows at the
   branch's position (after `CatchUpLast3d`), length 16 → 18, plus the two `row_settings` arms
   and the branch's test module.

### 1.4 The semantic conflicts the markers will not show

Found by reading, before the merge. AUDIO-2 found two of these and LUA-MERGE five; the shape is
always the same — a list, a golden, a tooltip table or a function both sides grew.

* **(S1) `entry_colour` vs `OPAQUE_PIXEL`.** Described in §1.3 point 3. A defect if left: the
  sprites the smooth scrolling draws apart would be invisible, and the pixels it writes back
  under them would be too.
* **(S2) The console's `ROW_SETTINGS` and the branch's two game rows.** `emu.smooth_scroll_ahead`
  is not in `dev`'s `ROW_SETTINGS`, so `advanced::step`/`value_text`/`label` have no row for it
  and the game-option row the branch adds would draw and step nothing. Either list it in
  `ROW_SETTINGS` (which changes `rows()` and may move `goldens/handheld-option-tree.txt`) or
  teach the branch's `step_by_domain` fallback to flip a `Bool`. The second keeps `dev`'s
  catalogue, its golden and the RO-3 tree untouched, and is what the branch's own fallback was
  for; that is the plan, with a test.
* **(S3) The options window's layout table, golden and tooltips.** RO-1 point 2 pins the options
  window to **exactly** the keys of `rust/goldens/desktop-options-shown.txt`
  (`desktop_options_golden_tests.rs`). The branch's two new settings are in
  `SettingTheme::Emulation`, so they will be shown and the golden will fail. They need a
  `LayoutEntry::Row` in `ui/options.rs`, a line in the golden, and — the LUA-MERGE lesson with
  `ui.editor_mode` — a tooltip and a mention in the layout note if those tables are exhaustive.
* **(S4) `emu.smooth_scroll_ahead` is `true` by default on the branch.** With the default
  run-ahead of 1 and a display above 61 Hz it shifts the 2D playfield of **every game, with no
  pack at all**, and turns on `set_tracks_line_scroll`, which is a test per visible line in
  `end_capture_line`. That is exactly the case step 3 of the work order names: a default that
  changes a picture for a game without a pack goes behind an option **off by default**. The
  branch's own handheld default already sets it to `false`. Plan: the catalogue default becomes
  `false`, the reason recorded here and in the setting's comment, and the handheld entry kept so
  that the two agree. The owner's "on by default" is noted as what it was and why it cannot
  stand as the merge's default.
* **(S5) `set_hack_options(u8)` → `u16` reaches six callers on `dev`.** `application.rs`,
  `host/hack_options.rs`, `host/movies.rs` (twice: the comparison `u64::from(...)` and the JSON
  field), `bench_core.rs`, `render/mod.rs`, `render/rom.rs` and
  `zax-harness/tests/widescreen_zone_tests.rs`. The branch changed its own tree's copies; the
  movie path (`movies.rs`) is `dev`-side only and must widen too, or a movie recorded with an aid
  above bit 7 compares wrongly.
* **(S6) `PATCHES: [BuiltinPatch; 57]` → 64 and the catalogue tests.** `dev` did not change the
  count, so there is no textual conflict, but tests that count patches, or that walk `PATCHES`
  and assert on names, groups or option bits, will see seven more. Also `for_rom(crc)` now
  answers Kirby's decoder for an **unpatched** Kirby ROM: `hack_compat.json`'s fingerprint is the
  CRC-32 of the patches' names, ROM CRCs and bytes, so it changes and every ROM is studied again
  — intended, and the mechanism is built for it.
* **(S7) The bend and the two bands.** `dev`'s AMD-046 `scene.bend_2d_mode` (default **on**)
  subdivides the 2D quad so that it curves; the branch's `picture_bands_for` builds two flat
  quads of two triangles each. With the smooth scrolling on and the bend on, the bands would not
  bend. The feature is experimental and, after (S4), off by default; the interaction is recorded
  here and the band meshes are left flat unless step 3 shows it is visible, in which case they
  take the subdivision `wide_picture_cells` already carries.
* **(S8) `Residency::texture` lost its `Result` on `dev`.** The branch's new `overlay` method is
  written as `Result<Option<_>, _>`, like `picture`, which is what `dev` still has for `picture`
  — so this one is only a reminder to copy `picture`'s signature and not `texture`'s.
* **(S9) Functions both sides pushed past clippy's hundred lines.** The branch already adds
  `#[allow(clippy::too_many_lines)]` to `decode_margins`, `Running::frame`, `update_scene` and
  `render::rom::capture`. On `dev` these four are longer still. Expect clippy to ask for more
  allows or a tail extracted, as LUA-MERGE did for `EmulationHost::new`.

### 1.5 The hot paths

What runs for **every** game on every frame or every fetch, and what only a pack pays. The
tolerance is +0.02 % per ROM on totals and cores.

**Every game, every visible line** (240/frame — the one real risk):

* `end_capture_line` gains `if self.tracks_line_scroll { self.keep_line_scroll(line); }`. One
  load and one test per line, and `end_capture_line` is inlined into `Ppu2C02::run_dots`, the
  largest function of the crate and the one MF-2 step 0 and MF-LEG-2 both warn about: the cost to
  watch is not the test itself (240 × 300 ≈ 72 k instructions, 0.001 % of a 7-billion run) but the
  register pressure it may add to `run_dots`. If it shows, the fix is the MF-LEG-2 one — the call
  behind a `const` parameter or an `#[inline(never)]` arm, never a second copy of the body.
  After (S4) `tracks_line_scroll` is false for every game with no pack and no option, so the
  branch never allocates `line_scroll` either.

**Every game, every frame** (60/s, ~300 per measurement — all negligible but listed):

* `NesCore::end_frame` gains `smooth_scroll()` and `smooth_scroll_ahead()`. Both start with a
  guard: `hack_option_on("smooth_scroll")` is `None`-checked on `builtin_patch` and `pack` before
  any string compare, and `smooth_scroll_ahead` returns on `!self.smooth_ahead`. What is **not**
  guarded is `take_camera_report()`, which reads the extension's scratch RAM (allocated whether
  or not the window is unlocked) and `set_wants_playfield_end`/`set_tracks_line_scroll`: ~20
  instructions a frame. Worth folding behind the same guard if step 4 sees it.
* `Ppu2C02::start_frame` gains `self.tinted_sprites = 0;`.
* `Running::frame` gains `self.host.display_rate = self.display_rate()`, which calls
  `window().display_mode()` — **an SDL query per displayed frame**. This is the one app-side cost
  worth a look in step 4's frame-time measurement; it should be read once and on a display change,
  not per frame.
* `Running::frame` gains `log_rates`, which writes a line to the log **every five seconds for
  every game**. New output on every run; to be gated or dropped.

**Only a widescreen frame** (the margin > 0, so SMB3 and Zelda II with their packs pay it, and
they are in step 4's campaign):

* `decode_margins` now walks `rows_to_decode()`, a `Vec` of up to 32 `CapturedRow` built per
  frame, where it walked `self.captured_rows` in place. A per-frame allocation on the widescreen
  path; if it shows on SMB3 or Zelda II it becomes a fixed-size array on the stack.
* `row_ends()` (31 entries) per `decode_margins` and per `wide_picture_into`, and `record_end`
  per painted record.
* `self.frame_margin > 0 && self.wide.sprite_tinted(n)` in the sprite capture loop — behind a
  test the loop already makes.

**Only a pack that uses them**: Kirby's decoder, the aids' components, `$500C`/`$5007` bit 5 (the
paged high bytes are a `match` arm reached only by a ROM that writes `$5007` with bit 5),
`split_steady_sprites`, `composed_entry`, the band meshes and the overlay texture.

**Nothing at all for the 98 originals**: no commercial original unlocks the extension, none has a
margin decoder, and golden-core must stay byte-identical to `19c5065d`'s.

### 1.6 The risks, ranked

1. **The wide 2D picture changes for every widescreen game, on purpose.** `shown_palette`/
   `shown_mask` (the frame's last palette and PPUMASK instead of the current ones) and
   `row_ends`/`record_end` (a record stops where the next kept row begins) are general fixes, not
   Kirby-only. SMB3 and Zelda II with their packs will move if they have a split or a palette
   write after the last row. Step 3's A/B every 5 frames is what decides whether each difference
   is the fix it claims to be or a defect; the owner's rule is that nothing may move for those two
   unless it is behind an option off by default, so a difference that is not plainly a fix goes
   behind one.
2. **`emu.smooth_scroll_ahead` on by default** — (S4). Settled by flipping the default.
3. **The options golden and the console tree** — (S2), (S3). Mechanical once found.
4. **`run_dots`'s register pressure** from the per-line test — §1.5. Measured in step 4.
5. **The core goldens.** They were regenerated at `ab68ee48`; the merge must not move a row. The
   branch touches `nes_bus.rs` (one argument), `ppu/mod.rs` (new cold fields and a per-frame
   copy) and `ppu/tiles.rs` (the capture's tail), none of which changes a 2D pixel or a timing
   for a ROM that never unlocks the extension — but `latch_sprites(page)` now reads a page byte
   that the fast OAM loop must supply correctly, and `$5100`-`$51FF`'s new `address & 3 == 1` arm
   changes what a write to `$5101 + 4n` does for an **unlocked** ROM: the four existing widescreen
   packs write high bytes at all four addresses of a sprite (`WIDESCREEN-EXTENSION.md`: "any of
   its four addresses"), so a pack that used `$5101` for the high byte would now write a flag
   instead. This is the sharpest behaviour risk of the merge and step 3's pack A/B is what finds
   it.
6. **The bend and the bands** — (S7), cosmetic and behind an off-by-default option.
7. **`u16` options and the movie path** — (S5); a `hackOptions` field recorded before the merge
   still reads back the same for bits 0-7.

### 1.7 Plan of the remaining steps

* **Step 2** — `git merge --no-ff feat/widescreen-kirby`, resolve the eight, then hunt (S1)-(S9);
  build the whole workspace and run the tests of `zax-nes`, `zax-render`, `zax-app`,
  `zax-settings` and `zax-harness` before committing.
* **Step 3** — golden-core over the 98 originals against `19c5065d`; the widescreen packs A/B
  (`render --dump-frame --dump-every 5 --advanced --widescreen 128`) on both commits; Kirby's
  seven packs loaded and their eight options exercised; the defaults of the smooth scrolling and
  run-ahead settled.
* **Step 4** — callgrind in WSL on the five level-2 games and the thirteen PR-1 ROMs, plus SMB3
  and Zelda II with their packs at `--advanced --widescreen 128`, each commit from `git archive`
  in a fresh directory; the app's 2D and 3D main-thread frame time before and after. Scripted
  totals repeated four times (LUA-MERGE step 4: Lua's per-process string hashing).
* **Step 5** — fmt; clippy `-D warnings` on every crate touched; `cargo test --release` over the
  workspace `--include-ignored --no-fail-fast` with both ROM roots; golden-core byte-identical;
  the Android and PortMaster feature sets `cargo check`.

---

## 2. Step 2: the merge

Commit `33b5d29b`, which is the merge commit **and** step 2's commit: everything below is
resolved in it, so there is no separate `KIRBY-MERGE (2/5)` tree change — only this note's step
marker. `git merge --no-ff feat/widescreen-kirby`, message *Merge feat/widescreen-kirby: Kirby's
widescreen and its engine work*. Eight textual conflicts, exactly the eight the trial merge
predicted, and four semantic ones the markers did not show — two of them found only by the
compiler and the tests, which is why they are numbered on past the plan's list.

### 2.1 The eight textual conflicts, per file

**`zax-nes/src/nes_bus.rs`.** `dev` rewrote the whole DMA path into a state machine
(REQ-CORE-113) in which `latch_sprites()` is called from **two** sites — the fast OAM loop and
the per-cycle put — where the branch's base had one. Kept `dev`'s two sites, each passing
`self.dma_oam_page`, which is the same byte the branch's `page` was. The marker itself landed
inside `after_apu_write`, where `dev`'s rewrite had moved the surrounding text and where the call
has no business being: the branch's line was dropped there and applied where the transfer
actually ends.

**`zax-nes/src/ppu/mod.rs`.** `dev` has `palette_marked` and four VS. System fields right after
`palette`; the branch put `shown_palette`/`shown_mask` in the same place. Both kept, with the
branch's two **moved to the cold tail** of the struct beside `margin_palette` and renamed
`frame_palette`/`frame_mask`. The reason is on the field itself: 33 bytes inserted before
`decay_stamp`, `timing` and `dot` moves every hot field's offset, which is `docs/progress/MF-2.md`
step 0's trap in `Ppu2C02::run_dots`, and the once-per-frame copy costs the same wherever the
fields sit. The rename also stops them being read as a plural of `dev`'s `shown_palette_entry`,
which is the VS remap of one written byte and an unrelated thing. The `new()` conflict is the
same two initialisers in the same new order.

**`zax-nes/src/ppu/tiles.rs`.** Both sides changed the `colour` closure of `wide_picture_into`:
`dev` ORs `OPAQUE_PIXEL` on every non-zero entry (AMD-050 REQ-CORE-049), the branch reads the
frame's last palette and answers the `TINT_ENTRIES` arm. The branch had also added `entry_colour`,
the same mapping **without** the opacity bit, for `split_steady_sprites`. Resolved into one
function: `entry_colour` now holds the tint arm, the frame palette *and* the opacity bit, and the
closure is `|entry| self.entry_colour(entry)`. See (S1). `dev`'s MF-LEG-2 split of
`capture_background_row` into `capture_background_row_as::<SUBSTITUTED>` plus an
`#[inline(never)]` cold arm is untouched.

**`zax-render/src/renderer.rs`** and **`wgpu_backend/renderer.rs`.** `dev` added
`wide_picture_cells` (the bend's subdivision cache) where the branch added `picture_bands`. Both
fields, both initialisers; `dev`'s `wide_quad_for_subdivided_cached` block stays and the band
block follows it (that part auto-merged).

**`zax-app/src/ui/handheld/advanced.rs`.** `dev`'s RO-3 rewrite — `ROW_SETTINGS` plus
`is_named_list`, `rungs_of` and `step_by_rung` — already shows and steps a labelled whole number
as a list in its `Enum | Int` arm. The branch's second `Int` arm did the same by index where
`dev`'s matches by value, and its `step_by_domain` fallback became dead once (S2) was resolved
`dev`'s way; both dropped, so `value_text` and `step` are `dev`'s byte for byte. What was added
is one line of `ROW_SETTINGS`.

**`zax-app/src/ui/handheld/defaults.rs`.** `dev` has 13 handheld entries, the branch 11 with
`EmuSmoothScrollAhead` = false as its eleventh. Result: 14 — `dev`'s 13 and the branch's one,
kept even though the global default is now false as well, so that the handheld still says no if
the global default ever goes back on.

**`zax-app/src/ui/handheld/game_options.rs`.** The branch wrote a flat `GAME_ROWS` of 18 with two
new rows; RO-3 has since replaced that list by the `GAME_MAIN` / `GAME_*_ROWS` tree, in which the
branch's `Setting(EmuRunAhead)` row **already existed** as `GAME_EMULATION_ROWS`. So
`Setting(EmuSmoothScrollAhead)` joined it there (1 → 2 rows) and `one_setting` gained its arm;
`dev`'s tree is otherwise untouched.

### 2.2 The semantic conflicts, resolved

* **(S1) `entry_colour` vs `OPAQUE_PIXEL`** — as predicted, and a real defect if left: the
  sprites `split_steady_sprites` moves into the overlay, and the pixels it writes back under
  them, would have carried bit 6 clear and read as transparent in `gl/picture.rs`. One function
  now, with the reason on it.
* **(S2) the console catalogue** — resolved the **opposite** way to the plan, because `dev`'s own
  tests state the rule the plan only guessed at:
  `ro_3_every_setting_a_game_can_set_has_a_global_value_to_point_at` and
  `ro_3_every_setting_row_of_the_tree_is_a_known_console_setting` hold *every* `Setting` row of a
  game's options to being in `advanced::ROW_SETTINGS` **and** offered by the global tree — RO-3's
  "one tree, two interfaces", so that "Use Global" has a global value to point at. So
  `emu.smooth_scroll_ahead` is a console row: `ROW_SETTINGS` 57 → 58,
  `options::EMULATION_ROWS` 5 → 6 (under the global run-ahead), `GAME_EMULATION_ROWS` 1 → 2, and
  `goldens/handheld-option-tree.txt` regenerated — **two lines, the two expected ones**, nothing
  else moved. `step_by_domain` is then unreachable and gone.
  `emu.smooth_scroll_view` stays desktop-only, as `audio.latency_ms` does: it is a debugging view
  and no game sets it.
* **(S3) the options window** — both settings added to `options::EMULATION` under the run-ahead;
  `goldens/desktop-options-shown.txt` gained its two keys and nothing else (the diff is two
  lines); `detail.emu.smooth_scroll_ahead` and `detail.emu.smooth_scroll_view` written in English
  and in French, which `ro_1_every_shown_setting_has_a_detailed_tooltip_in_both_languages`
  requires and `ro_1_every_detail_text_belongs_to_a_shown_setting` keeps honest;
  `rust/docs/OPTIONS-LAYOUT.md` took its two rows.
* **(S4) `emu.smooth_scroll_ahead` was on by default** — now **off**, and this is the one place
  the lot overrode the branch's own choice. The branch shipped it on (owner request) with the
  handheld's defaults turning it back off. On, with the default run-ahead of 1 and any display
  above 61 Hz, it shifts the 2D playfield of **every game with no pack at all** and turns on the
  PPU's per-line scroll record. The owner's rule for this lot is that nothing may move for a game
  without a pack unless it is behind an option off by default, and the work order's step 3 names
  this case word for word. Nothing else about the feature changed: on, it behaves exactly as the
  branch built it. The reasoning sits in the descriptor's own comment so that it is not undone by
  accident.
* **(S5) `u16` option bits** — no change needed on `dev`'s movie path after all: `movies.rs`
  compares through `u64::from(self.hack_option_bits())`, which widens a `u16` as happily as a
  `u8`, and writes the same JSON number. A `hackOptions` field recorded before the merge reads
  back identically for bits 0-7.
* **(S6) `PATCHES` 57 → 64** — no counted assertion broke. `for_rom(crc)` now answers Kirby's
  decoder for an **unpatched** Kirby ROM, and the built-in-patch fingerprint changes, so
  `hack_compat.json` is dropped and every ROM is studied again: intended, and the mechanism was
  built for it.
* **(S7) the bend and the bands** — recorded, not changed: `picture_bands_for` builds two flat
  quads, so with `scene.bend_2d_mode` on (its default) and the smooth scrolling on, the two bands
  do not curve. The feature is experimental and now off by default; step 3 looks at whether this
  is visible enough to be worth giving the bands the subdivision `wide_picture_cells` already
  carries.
* **(S8)** — a non-issue: the new `overlay` copies `picture`'s signature, not `texture`'s.
* **(S9) clippy's line limit** — not yet; step 5 settles it.
* **(S10) `zax-render/src/scene_export.rs`** (new, found by the compiler) — `dev`'s AMD-055
  geometry export implements `Residency` and builds a `FixedMeshes`, both of which the branch
  grew. `MeshTable::overlay` answers as its `picture` does, and the export passes
  `picture_bands: None`: a 2D export is the one whole quad whether or not the playfield is being
  shifted on screen, since the export answers the floor and extent criteria of AMD-055
  REQ-SCN-084 and a fraction of a pixel of scrolling does not enter them.
* **(S11) the harness's run-ahead bound** (found by the tests) — the branch raised
  `MAX_RUN_AHEAD` to 10 and updated `zax-app/tests/run_ahead_tests.rs`, but two `dev`-side
  harness tests still asserted that `--run-ahead 4` is refused (`bench_core_tests`'s
  `req_arch_012_the_bench_runs_without_run_ahead_unless_asked` and `render_rom_tests`'s
  `req_loop_064_render_refuses_a_run_ahead_outside_zero_to_three`), and four help strings and
  error messages in `bench_core.rs` and `render/mod.rs` still read "0 to 3" while the code they
  document already filtered on `MAX_RUN_AHEAD`. Both tests now pin the **new** edge — 10
  accepted, 11 refused, and the render one checks 3, 4 and 10 all land — and the four strings
  read "0 to 10". `zax-harness/tests/widescreen_zone_tests.rs` needed the new `smooth_ahead`
  field.

### 2.3 What was not merged, and what was overridden

No **feature** of the branch was dropped. Three pieces of its *code* were, each because `dev`
already carried the same intent in a better shape — `advanced::value_text`'s duplicate `Int` arm,
`advanced::step_by_domain`, and the flat `GAME_ROWS` — and one of its *choices* was overridden:
the default of `emu.smooth_scroll_ahead`, for the reason in (S4).

### 2.4 Found and deliberately left alone

`Ppu2C02::paint_extra_sprites` writes its colours **without** the opacity bit of AMD-050
REQ-CORE-049, so the extra sprites of a wide 2D picture — the extension's second sprite page, and
sprites whose high byte puts them left of x = 0 — come out with alpha 0 in the GL 2D path. That is
`dev`'s behaviour at `19c5065d` and not something this merge introduces; correcting it would move
the picture of the packs that use the second page, which the owner's rule forbids in this lot. It
is kept to the byte, with a comment on the line, and is a candidate for a lot of its own.

### 2.5 Verification of the merge commit

* `cargo build --workspace --all-targets` clean in **debug and release**, and `cargo check -p
  zax-app --features bundled,wgpu --all-targets` clean.
* `cargo test --release` over the five crates both sides touched — `zax-nes`, `zax-settings`,
  `zax-render`, `zax-app`, `zax-harness` — with `ZAXNES_ROMS` and `ZAXNES_TEST_ROMS` set:
  **2589 passed, 0 failed, 14 ignored**.
* Goldens moved: `desktop-options-shown.txt` +2 lines and `handheld-option-tree.txt` +2 lines,
  each exactly the new rows. `rust/goldens/core-goldens.jsonl` **untouched** — step 3 is what
  checks the core against `19c5065d`.

---

## 3. Step 3: behaviour

### 3.1 golden-core over the 98 originals

`zax-harness golden-core G:/3DSen/test-originals <out> ` (1 200 frames each, the default), run on
the merge:

| | |
|---|---|
| ROMs | 98 |
| rows | 98, 145 334 bytes |
| md5 | **`f79e7df842c3ee43e7d9b67861a927b5`** |

That is byte for byte the digest LUA-MERGE step 5 recorded at `19c5065d` (itself identical to
`ab68ee48`'s). **The emulation core did not move for a single commercial original.** Nothing of
the branch can reach them: none unlocks the widescreen extension, none has a margin decoder, and
the smooth scrolling is off.

`rust/goldens/core-goldens.jsonl` is untouched by the merge — no row moved, so there was nothing
to explain and nothing to regenerate.

### 3.2 The widescreen packs, A/B against `19c5065d`

`19c5065d` was rebuilt from `git archive` into a fresh directory and its `zax-harness` built
there; both binaries were then run over the same ROMs with the same flags, and **every file each
run wrote** was compared, not only the frame dumps: the periodic frame JSONs, the final frame
JSON, the 640 × 480 render PNG and `--png-nes`, the composed wide 2D picture, which is where the
branch's changes to `wide_picture_into` would show.

The pack list is the harness's own: `zax-harness packs G:/3DSen/test-originals` reports 97 of the
98 originals taking a whole pack, which is **45 distinct packs**; one ROM per pack was taken,
preferring the one the pack matches by CRC.

| Regime | Packs | Files compared | Differing |
|---|---|---|---|
| `--run 300 --advanced --widescreen 128 --dump-every 5` (the owner's A/B) | 45 | 2 925 | **0** |
| `--run 1500` with an input script (start mashed, then right held with jumps) | 45 | 13 725 | **0** |
| `--run 1500` + input + **`--run-ahead 1`**, SMB3 / Zelda II / SMB / Castlevania | 4 | 1 220 | **0** |
| `--run 1500` + input + **`--run-ahead 3`**, the same four | 4 | 1 220 | **0** |
| **total** | | **19 090** | **0** |

The second regime is not idling: all 45 packs end on a different picture at 1 500 frames than at
300, and 36 of the 45 produce 50 or more *distinct* frame dumps out of the 301 taken — Castlevania
289, Super Mario Bros. 3 219, Zelda II 122, Metroid 206. Super Mario Bros. reaches level 1-1 and
scrolls with scenery drawn in both margins (checked by eye on its `--png-nes`). Super Mario Bros.
3 reaches its world map, and its title screen already has the decoder filling the margins. Zelda
II stops at the name-registration screen — its menus did not yield to the generic script — so its
122 distinct frames are menus and the attract sequence rather than the overworld.

The run-ahead regimes are there for one reason: the branch's change from the PPU's *current*
palette to the palette the **frame ended with** (`frame_palette`) can only differ when the host
composes the picture later than the frame, which is exactly what run-ahead does. It differs on
neither of the owner's two games at N = 1 or N = 3.

**So: nothing moves for SMB3, Zelda II or any of the 45 widescreen packs.** There was therefore
no difference to explain and nothing to put behind an option on that count.

Two reservations, stated plainly:

* Zelda II never reached gameplay under the generic input script, so its share of the evidence is
  weaker than the rest. What covers it instead is that its 1 220 run-ahead files and its 610
  ordinary ones are identical, and that no other pack moved either.
* The three changes the branch made to the wide picture on purpose — the frame's last palette and
  PPUMASK, a record stopping where the next kept row begins (`row_ends`/`record_end`), and
  `rows_to_decode`'s strips — are **reachable** by these runs and did not fire differently on any
  of them. They are written to bite on a split or a mid-frame palette write that these 45 games
  do not make at the moments captured; a game that does will show them, and that is the branch's
  intent, not a regression.

### 3.3 The core's own goldens and the test suites

Covered by step 2's run and re-checked here: 2 589 tests over the five crates both sides touched,
0 failed. Nothing of `rust/goldens/core-goldens.jsonl` moved.

### 3.4 Kirby's packs — what could be checked, and what could not

**No Kirby's Adventure ROM exists on this machine.** `G:/3DSen/test-originals` (98 commercial
originals) has none, `G:/3dsen/demo/roms` has none, and a search of the drive finds none. The
branch's own figures — 37 of its 41 ROMs take a whole pack, 1 all but the invincibility — are
about a ROM pool this side has not got. **The seven packs were therefore not run**, and
"No slowdown", "Invincible", "Jump and attack first", "Level select" and the asleep enemies were
not seen behaving. That is a gap this lot cannot close; whoever has the ROMs should close it.

Everything about the seven packs that does **not** need the game is now held by a new test file,
`zax-nes/tests/kirby_pack_tests.rs` (6 tests, all passing):

* **a pack per build is registered** — the seven `PATCHES` entries, all naming *Kirby's
  Adventure*, all plain (not advanced) widescreen, all booting their execution zone from
  power-on, all with the game's idle loop so the overclock can end; **six** distinct source CRCs
  for seven packs, Europe and its NTSC twin sharing one by design and told apart by signatures;
* **every pack parses and carries its components** — each `.zxpk` is a hack pack, each has its
  `widescreen` component, and every group of every option's component has at least one hook or
  one check;
* **the options keep their bits and their defaults** — the eight the owner chose, verbatim:
  `input_priority` 0 off, `invincible` 1 off, `no_slowdown` 2 **on**, `smooth_scroll` 3 off,
  `margin_enemies` 4 **on**, `margin_freeze` 5 off, `margin_freeze_tint` 6 off,
  `kirby_attacks_4_3` 7 off, and `level_select` at **bit 8**, off;
* **the asleep enemies and their grey tint are off by default** — a test of its own, because that
  was an owner's decision taken late on the branch (`da13ef77`);
* **every pack and every build reaches the margin decoder** — through `for_patch` for the seven
  names, and through `for_rom` for the seven unpatched builds, which is the plain widescreen of a
  game whose margins a decoder draws;
* **why the option word had to widen** — Kirby fills the ROM's `$500A` byte exactly (bits 0 to 7,
  once each), and the ninth option does not fit in a `u8` at all, so a build that narrowed the
  word back would drop the level select silently.

### 3.5 The smooth scrolling and the run-ahead: what the defaults do

| Setting | Branch | Merge | What the default does to a game with no pack |
|---|---|---|---|
| `emu.smooth_scroll_ahead` | **on** | **off** | nothing: no shift, no motion, and the PPU keeps no per-line scroll |
| `emu.smooth_scroll_view` | off | off | nothing (a debugging tint of the band) |
| `smooth_scroll` (a pack aid, Kirby only) | off | off | nothing; it needs the pack's code to report a camera |
| `emu.run_ahead` | 1, domain 0..10 | unchanged | unchanged: the default is still 1 frame, only the ceiling moved from 3 to 10 |

The flip of `emu.smooth_scroll_ahead` is argued in §2.2 (S4). What it protects against is
concrete: on, with the default run-ahead of 1 and any display above 61 Hz, the core turns on
`Ppu2C02::set_tracks_line_scroll`, which puts a test and a record on **every visible line of
every frame of every game**, and the host then draws the 2D playfield shifted between frames.
Neither is something a game without a pack asked for.

Four new tests in `zax-nes/tests/smooth_scroll_default_tests.rs` hold both halves of that on a
real scrolling game (Super Mario Bros. from the demo root; they pass silently without it, as
`ppu_golden_tests` does):

* **off by default, a game with no pack is left alone** — 400 frames, and on every one of them
  `picture_shift()` and `picture_motion()` are `None`; afterwards the PPU answers no
  `playfield_scroll()` at all, so `end_capture_line` did the work it did before the merge;
* **turned on, the same game gets a shifted playfield** — the band that scrolls is found on more
  than half the frames and runs over at least 16 lines. This is what says the switch is a gate
  and not a dead option: the feature works, it is simply not on;
* **turning it off again stops everything** — the switch is not one-way;
* **the run-ahead bound is ten, and the real frames still match a plain run** — REQ-LOOP-065's
  invariant checked at the new edge: 120 frames at N = 10, restoring the real frame each time,
  leave the work RAM exactly where a plain run leaves it.

### 3.6 Verdict on the owner's rule

Nothing moves for SMB3, Zelda II or the widescreen packs: 19 090 compared output files, zero
differences, across four regimes. The core is byte-identical over the 98 originals. The one
default that would have moved a picture for games with no pack is off, and a test says what it
does on either side of the switch. The one thing not done is running Kirby's seven packs, for
want of the game.

---

## 4. Step 4: measurements

Callgrind in WSL2 Ubuntu, `rust/tools/perf/`. Every compared commit built by `build-clean.sh`
from a `git archive` (or `git ls-files`) tar into a brand-new directory with its own
`CARGO_TARGET_DIR`, as `PERFORMANCE-HISTORY.md` §2 requires. Base: `19c5065d`.

### 4.1 Where it started: +0.248 % on the set, +0.584 % on the worst ROM

The first measurement of the merge as committed, PR-1's thirteen ROMs at 300 frames:

| | duelito | m0 | m1 | m4 | metroid | mm2 | mmc5 | oamstr | readbuf | smb1 | smb3 | vblnmi | z2 | **set** |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| merge as committed | +0.159 | +0.154 | +0.168 | +0.149 | **+0.584** | +0.375 | +0.085 | +0.200 | +0.156 | +0.434 | +0.290 | +0.229 | +0.385 | **+0.248 %** |
| after this step | +0.045 | +0.048 | +0.041 | +0.037 | +0.050 | +0.045 | +0.029 | +0.024 | +0.043 | +0.040 | +0.041 | +0.028 | +0.042 | **+0.040 %** |

Every ROM was over the +0.02 % tolerance and the five commercial games were five to twelve times
over it. What follows is what the causes were, and what each fix bought.

### 4.2 The causes, in the order they were found

**(1) A test on every CPU instruction, armed by one `or_else`. The whole of the regression on the
five commercial games.**

`end_frame` ends the overclock where the game waits for the next frame:

```rust
if let Some((first, last)) = idle_loop {
    if (first..=last).contains(&self.cpu.pc) && self.bus.ppu().wide().stalling() { … }
}
```

That `contains` runs **once per CPU instruction** — about 10 500 a frame — at roughly 5 Ir each.
`dev` answered `None` here for a game whose patch is not applied, and the whole test vanished
from the loop. The branch added `.or_else(|| self.pack.as_ref().map(…))` to the lookup so that a
pack's aid could ask for the overclock — and a hack pack is found **by signature** on every one
of the five commercial games whether or not its patch is applied. So the test came back on for
all five.

It was invisible in the per-symbol view (everything is inlined into `run_frame`) and only a
line-level profile named it: `core/src/ops/range.rs` charged **3 102 540 Ir to `run_frame` over
60 frames** on `metroid` — +15.5 M over 300, with `cmp.rs` adding 0.6 M more.

Fixed by keeping the branch's intent and asking the question once a frame instead:

```rust
let idle_loop = self.idle_loop.filter(|_| self.bus.ppu().wide().unlocked());
```

A ROM that has not unlocked the register window cannot have written `$5006`, so it cannot be
stalling, so the test has nothing to find. An aid that *does* ask for the overclock still gets
its wait ended, because a pack's code unlocks the window. **metroid +0.524 % → +0.050 %, smb1
+0.386 % → +0.040 %, z2 +0.316 % → +0.042 %, the set +0.161 % → +0.040 %.**

**(2) Three lookups a frame that scan a `const` array or a sixty-arm string match.**

`end_frame` worked out, every frame: the margin decoder (`wide_decode::for_patch`, a match on
one string literal per decoder, to which the branch added seven arms for Kirby's builds), the
patch's idle loop (`builtin_patches::by_name`, which scans `PATCHES` — a `const` array, so every
call materialises all **64** entries, 57 before the branch), and whether the `smooth_scroll` aid
is on (the same scan again). None of the three can change inside a frame. All three are now
worked out by `NesCore::refresh_patch_cache`, called at the four points an answer can move: a
ROM loaded or closed, a pack installed, the options set, `emu.builtin_patches` turned. The frame
path reads three fields. **The set +0.248 % → +0.161 %**, and it is what takes the ROMs with no
patch from +0.15 % to +0.045 %.

**(3) `keep_line_scroll` inlined into `run_dots`.** The per-line scroll record of the smooth
scrolling is called from `end_capture_line`, which `Ppu2C02::run_dots` inlines — the largest
function of the crate, and the one MF-2 step 0 and MF-LEG-2 both warn about. Letting its body in
beside the capture, for a test that is false on every line of every game that asked for no
smooth scrolling, cost `duelito` **+5.6 M over 300 frames** in `run_dots` alone and every other
ROM between +1.2 M and +2.4 M. `#[inline(never)]`, and only the test remains.

**(4) `decode_margins` and `find_playfield_end` inlined into `run_frame`.** The same remedy, and
it is worth **−3.3 M on `metroid`**. The branch's `decode_margins` is much larger than `dev`'s;
behind a call it costs 23 040 Ir over 600 level-2 frames — 38 a frame — and the frame path keeps
its registers.

**(5) A kilobyte of Kirby's state in the PPU.** `$5007` bit 5 keeps sprite high bytes and flags
**per page**: two `[[u8; 64]; 8]` tables, 1 KiB, inside `WideExtension`, which lives inside
`Ppu2C02`. They are now a `Box` allocated on the first `$500C` write, so only a ROM that asks for
paged sprites has them. This did **not** move the instruction count — callgrind counts
instructions, not cache lines, and the fields sit after the hot ones — and it is kept on its
merits: a kilobyte that only Kirby's pack ever touches has no business in the struct the hottest
loop of the program works on.

### 4.3 Two shapes measured and rejected

Recorded so that nobody builds them again, in the manner of MF-5b-fix step 3.

* **The frame-palette snapshot behind a call.** `process_dot` runs once per PPU dot — about
  89 000 times a frame — and the branch writes a 33-byte snapshot inside it, guarded by a test it
  already makes. Putting that body behind an `#[inline(never)]` call, which is the MF-LEG-2
  remedy for cold code in a hot function, made **every ROM of the set worse**: the set total
  +0.040 % → **+0.088 %**, `metroid` +0.049 % → +0.125 %, `m0` +0.048 % → +0.103 %. The copy
  stays inline, with a comment saying why.
* **Hoisting the wide picture's new per-record work.** `record_end` divides the record index by
  the row width once per painted record (about 2 000 a frame at margin 128), and `paint_side_tile`
  gained a term in its per-pixel test. Answering "does this frame have a split at all?" once a
  frame and hoisting the per-pixel term out to once a row made Super Mario Bros. 3 with its
  widescreen pack go from **+0.156 % to +0.36 %**. Reverted.

### 4.4 The figures as they stand

**PR-1's thirteen, 300 frames, no pack** — the table in §4.1: **+0.024 % (`oamstr`) to +0.050 %
(`metroid`), set total +0.040 %**.

**Level 2: the five commercial games with their 3D packs, 600 frames, inputs pressed.**

| game | core | ppu | apu | draw_list | total |
|---|---|---|---|---|---|
| metroid | +0.056 % | +0.089 % | **+0** | +0.000 % | +0.050 % |
| mm2 | +0.052 % | +0.090 % | **+0** | +0.000 % | +0.035 % |
| smb1 | +0.052 % | +0.091 % | **+0** | **+0** | +0.053 % |
| smb3 | +0.041 % | +0.065 % | **+0** | −0.010 % | **+0.018 %** |
| zelda2 | +0.049 % | +0.086 % | **+0** | −0.010 % | +0.039 % |

The `apu` stage is **+0 to the instruction** on all five, and `draw_list` is +0 or below. The
`ppu` stage moves by an almost identical **absolute** figure on every game — 5 980 010, 6 197 080,
6 225 995, 6 249 262, 6 252 523 — which is the signature of a fixed per-frame cost, not of work
that scales with the game.

**Super Mario Bros. 3 and Zelda II with their widescreen packs** (`bench-core --advanced
--widescreen 128`, 300 frames):

| game | `19c5065d` | merge | delta | |
|---|---|---|---|---|
| smb3 | 8 801 335 494 | 8 815 078 693 | +13 743 199 | **+0.156 %** |
| zelda2 | 6 945 577 537 | 6 954 070 774 | +8 493 237 | **+0.122 %** |

**The app's frame time.** `render --rom "Super Mario Bros. 3" --profile … --run 300 --bench 120`,
three runs a side, on the Windows release binaries:

| | run 1 | run 2 | run 3 | Lua hooks |
|---|---|---|---|---|
| `19c5065d` | 0.37 ms median, 0.58 p99 | 0.37 / 0.63 | 0.43 / 0.63 | 0.186–0.196 ms mean |
| merge | 0.37 ms median, 0.52 p99 | 0.37 / 0.60 | 0.37 / 0.59 | 0.186–0.192 ms mean |

**The same median to the hundredth of a millisecond**, with the only outlier on the base's side.
The 2D path has no bench of its own in `render`; what stands for it is step 3's A/B, where the
composed 2D picture of all 45 packs came out byte-identical, and the widescreen figures above,
which are the 2D composition's own cost.

### 4.5 What is left, and what the owner has to decide

**Still above +0.02 %**: every PR-1 ROM except `oamstr` (+0.024 %) and `vblnmi` (+0.028 %), by
0.004 to 0.03 points; all five level-2 totals except `smb3` (+0.018 %); and the two widescreen
runs, by a wider margin. Two different things are left, and they want different answers.

**(a) On games with no pack: ~+0.04 %, and it is not executed work.** Of `m0`'s +2.67 M, about
450 K is **one-off at load** — `pack_for` now tests 64 packs instead of 57 (`HackPack::read`
+179 K, `resolve` +94 K, the allocator +98 K, UTF-8 validation +51 K) — which at 300 frames reads
as 0.008 points and at any real session length as nothing. The remaining ~1.85 M is a **uniform
+0.11 % on `run_dots` alone**, on every ROM, and it is not attributable to a line: deleting the
per-line smooth-scrolling call outright gives back only 150 K of it (0.002 %). It is the register
allocation of the crate's largest function shifting under a handful of new statements and a
bigger `Ppu2C02` — the effect MF-2 step 0 names.

For scale, from `docs/review/BOARD-FEATURES.md`: a **control** on this very level-2 instrument — a
`Board` variant that is never constructed, a provably behaviour-free change — moves `core` by
**+0.096 %** and `lua_hooks` by +0.36 %. This lot's `core` figures are +0.041 % to +0.056 %,
**half the drift of a change that does nothing**. No option can gate codegen drift; there is
nothing here to put behind a switch, and the features that caused it are already off by default.

**(b) On a game running a widescreen pack: +0.12 % to +0.16 %, and it is real work.** This is the
branch's new wide picture — a record stopping where the next kept row begins (`row_ends`,
`record_end`'s division per record, the per-pixel clip in `paint_side_tile`), `rows_to_decode`'s
per-frame `Vec`, and the `minimum_rows` count. It is paid **only** by a game whose widescreen is
on, which is where the owner's rule says such work belongs — but Super Mario Bros. 3 and Zelda II
with their packs are named in the tolerance, so the rule is not satisfied by that alone. The one
attempt at taking it back made it worse (§4.3) and was reverted; a proper fix is a
monomorphised `paint_side_tile` on the MF-LEG-2 pattern, which is a lot of its own rather than a
step of this one.

**Nothing was removed and nothing was parked.** Every feature of the branch works as it was
built; the two that could cost a game that wants neither — the smooth scrolling and the per-page
sprites — are off by default and allocate nothing until asked. What is over the tolerance is
(a) drift below the instrument's own floor for a no-op, and (b) the cost of the wide picture for
a game that has asked for a wide picture. Both are recorded here for R0 and the owner, because
parking a feature is their decision and not this lot's — the precedent is MF-LEG-3, which left
the same call to them.

---

## 5. Step 5: the final pass

The owner's decision of 2026-11-09 settles step 4's residue: what is left above +0.02 % — the
`run_dots` codegen drift of about +0.04 % and the wide picture's +0.12 % to +0.16 % on SMB3 and
Zelda II with their packs — is **accepted**, the project's budget now being under +1 % between
two releases. No measurement was redone in this step and nothing was reshaped for performance.

| check | result |
| --- | --- |
| `cargo fmt --all -- --check` | clean |
| `cargo clippy --workspace --all-targets --features zax-app/bundled -- -D warnings` | clean (one `#[allow]`, §5.2) |
| `cargo clippy -p zax-render --features wgpu,offscreen --all-targets -- -D warnings` | clean |
| `cargo clippy -p zax-app --features bundled,wgpu --all-targets -- -D warnings` | clean |
| `cargo test --release --workspace --features zax-app/bundled --no-fail-fast -- --include-ignored` | **3 313 passed, 0 failed, 0 ignored**, 7 filtered out, over 180 test binaries and 11 doc-test runs — **after the one fix of §5.1** |
| golden-core over the 98 originals, 1 200 frames | `f79e7df842c3ee43e7d9b67861a927b5`, 98 rows, 145 334 bytes — **byte-identical** (`fc /b`, not only the digest) to step 3's file and to the figure recorded at `19c5065d` and `ab68ee48` |
| `cargo check -p zax-app --no-default-features --features android` | clean |
| `cargo check -p zax-app --no-default-features --features firmware-sdl` (PortMaster) | clean |
| `cargo check -p zax-android --target aarch64-linux-android` | **still not checkable here**, as AUDIO-1 and LUA-MERGE recorded: the C build of `mlua-sys` wants the NDK's `clang`, which this machine has not got. The target is installed; the failure is in the C build, not in our code |

The three asset roots LUA-MERGE's step 5 identified are all needed, and were all set:
`ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS` on `assets-free/test-roms`, and
`ZAXNES_PROFILES` on `assets-neutral` (without the third, `--include-ignored` over the *whole*
workspace wakes about ninety asset-corpus tests of `zax-profiles`, `zax-scripting` and
`zax-engine3d`). `rust/test-roms` is present in the tree tested — 71 entries — which is what
`ppu_golden_tests` and `golden_core_tests` read **directly**, ignoring `ZAXNES_TEST_ROMS`; all
three ran and passed, so none of them passed vacuously.

### 5.1 (S12) A twelfth semantic conflict, and the only failure of the step

`zax-scripting`'s `two_zero_api_tests::req_lua_114_run_ahead_and_visual_style_are_brought_into_their_domain`
failed: `left: 3.0, right: 7.0`.

| | |
|---|---|
| `dev`'s side | the test pins AMD-041 REQ-LOOP-064's acceptance criterion word for word — «`Setting.RunAheadFrames = 7` → 3» — and AMD-040 REQ-LUA-114's line «écriture ramenée dans [0, 3]» |
| the branch's side | `NesCore::MAX_RUN_AHEAD` 3 → 10 (owner request), the catalogue domain `stepped(0, 10, 1)` with eleven labels |
| kept | the branch's domain, because this lot may not remove a feature the owner asked for; the test now pins the **new** edge |

It is the same shape as (S11), and it was missed for the same reason (S11) was caught: step 2 ran
the tests of the **five crates both sides touched**, and `zax-scripting` is not one of them — the
branch never touched it, and `dev` touched it only in LUA-MERGE. The Lua binding itself
(`api/settings.rs`: `whole(definition, "RunAheadFrames", SettingId::EmuRunAhead)`) reads the
descriptor's domain, so nothing of the code was wrong; what was wrong was a test, and three
comments, still stating the old bound.

What this step did, and deliberately did **not** do:

* **`spec/changes/Q-181.md`** (new) — the spec is not ambiguous here, it is *superseded*: the
  domain of REQ-LOOP-064 and the `RunAheadFrames` line of REQ-LUA-114 both say [0, 3], and the
  criterion names the number 7. The ticket records the owner's [0, 10], the provisional choice
  (keep it), what is unaffected (default 1, a negative write → 0, persistence, and the clamp
  **rule** itself — only the upper bound moved), and what R0 has to amend if the owner confirms.
  The code points at it with `// Q-181`, as CLAUDE.md requires.
* **the test** now checks the rule at the bound that exists — `= 11` → 10, `= 7` → 7, `= -2` → 0,
  default 1 — with the criterion's own figure explained in the doc comment rather than deleted.
* **three stale comments** corrected, all of them prose the branch's auto-merge left behind: the
  `emu.run_ahead` descriptor (`zax-settings/src/table.rs`, which still said "0 to 3" and "a write
  of 7 reads 3" **directly above a domain of 0..10**), `GameSession::run_ahead` and its setter,
  and `AppSettings::run_ahead`. Plus the feature's line in `rust/docs/PERFORMANCE-HISTORY.md`.
* **not** done: lowering `MAX_RUN_AHEAD` back to 3 to satisfy the criterion literally. That is
  removing a feature, which this lot's rules forbid; it is in Q-181 as the alternative, for the
  owner.

### 5.2 (S9) clippy's line limit: one allow, and the proof it is needed

Step 2 left (S9) to this step. Only **one** function needed anything: `Ppu2C02::new`, which both
sides' new fields took to **101 lines of the 100 allowed** — shown by removing the attribute and
re-running, which gives `warning: this function has too many lines (101/100)`. LUA-MERGE's remedy,
giving the tail to another function as `EmulationHost::new` and the harness's `run` did, does not
apply: the body is a **single struct literal**, one line per field, and the only way to shorten it
is to break `Ppu2C02` into sub-structs, which moves the field offsets `docs/progress/MF-2.md`
step 0 measured in `run_dots`. So an `#[allow(clippy::too_many_lines)]` with its reason on the
line, in the idiom `cpu::opcode_table` and `cartridges::registry` already use for a flat list.
Six of those 101 lines are the `#[cfg(feature = …)]` attributes of the VS System and
exact-sprite-eval fields, which the lint counts whether or not the features are on.

The four allows the branch already carried (`decode_margins`, `Running::frame`, `update_scene`,
`render::rom::capture`) were enough on `dev`'s longer bodies; nothing else was added and nothing
was extracted. Besides that, the step's only other changes were `cargo fmt`'s reflow of three
lines written by hand in step 4 (`nes_core.rs`, `wide.rs`) and of four assertion blocks in the two
test files step 3 added.

### 5.3 The 7 filtered out, each re-verified rather than taken on trust

The same two groups LUA-MERGE recorded, checked again here because this lot regenerated a golden.

**Six: `three_d_sen_real_install_tests`** (`zax-profiles`). Run on purpose: 0 passed, 6 failed,
and every one of the six prints exactly `ZAXNES_3DSEN_INSTALL is not set: it names a 3dSen
installation (or a stand-in) folder` and nothing else. The clean side has no 3dSen installation
and must not have one, so these six cannot run here at all.

**One: `ro_3_write_the_option_tree_golden`**, the workspace's only golden **regenerator**,
`#[ignore]`d precisely so that it does not run — and `--include-ignored` runs it, racing the
reader in the same binary. Two things settle it, and the second matters more for this lot than it
did for LUA-MERGE, because step 2 **did** move that golden by two lines: the reader passes alone
(`--test handheld_tree_tests`: 5 passed, 1 ignored), and running the regenerator on purpose
leaves `git status -- rust/goldens` **empty**. So the console option tree the code draws is
byte-for-byte the committed golden, the two new `emu.smooth_scroll_ahead` lines included.

### 5.4 The owner's rules, re-checked structurally

`git diff 19c5065d HEAD` is **empty** over `zax-engine3d`, `zax-platform`, `zax-profiles`,
`zax-goldens`, `zax-achievements` and `zax-android` — the 3D engine, the platform layer and the
profile decoder are byte-identical to the base — and over `zax-scripting` it is the one test of
§5.1. `rust/goldens/core-goldens.jsonl` is untouched: no row moved, so there was nothing to
explain and nothing was regenerated. Version still 0.5.0, the 62 `board-*` Cargo features intact,
`PATCHES` at 64 (`dev`'s 57 and Kirby's seven). The two goldens that did move are step 2's, two
lines each and exactly the new rows.

Nothing moves for SMB3, Zelda II or the 45 widescreen packs (step 3: 19 090 compared output
files, zero differences, four regimes); the core is byte-identical over the 98 commercial
originals; `emu.smooth_scroll_ahead`, the one default that would have moved a picture for a game
with no pack, is off, with four tests saying what the switch does on either side of it.

G: was between 18.4 and 12.8 GB free throughout, above the 5 GB floor. Step 4's clean-build tars
in `rust/target-km/` were deleted once this note had their figures; `target-km/base19` (the
`19c5065d` clean build) is kept until the lot is closed, and `target-km` is untracked and was
never staged.

### 5.5 What is left for someone else

* **Kirby's seven packs were never run** — there is no Kirby's Adventure ROM on this machine
  (§3.4). Ten tests hold everything about them that does not need the game; "No slowdown",
  "Invincible", "Jump and attack first", "Level select" and the asleep enemies have not been seen
  behaving. Whoever has the ROM should close that gap.
* **Q-181** — for R0 and the owner: amend AMD-041 REQ-LOOP-064 (its value list, its interval, the
  number in its criterion, the eleven labels of its §3) and AMD-040 REQ-LUA-114's
  `RunAheadFrames` line, or decide to go back to [0, 3].
* **Step 4's two residues**, accepted by the owner on 2026-11-09 and recorded in §4.5: the
  `run_dots` codegen drift, and the wide picture's cost for a game whose widescreen is on. The
  second has a known remedy — a monomorphised `paint_side_tile` on the MF-LEG-2 pattern — which is
  a lot of its own.
* **§2.4** — `Ppu2C02::paint_extra_sprites` writes its colours without AMD-050 REQ-CORE-049's
  opacity bit. That is `dev`'s behaviour at `19c5065d`, kept to the byte because correcting it
  would move the picture of the packs that use the extension's second sprite page. A lot of its
  own.

## Summary

`dev` carries `feat/widescreen-kirby`: Kirby's margin decoder and its seven packs with their nine
options, the widescreen extension's per-page sprite high bytes (`$5007` bit 5, `$500C`) and its
16-bit option word, the wide 2D picture's margins ending at a split with the frame's last palette
and PPUMASK, run-ahead up to 10 frames with a run-ahead per game, and the experimental smooth
scrolling at 120 Hz — the last **off by default**, which is the one choice of the branch this lot
overrode and the only way the owner's "nothing moves for a game with no pack" could be kept.

`dev`'s own work is intact: the ~140 mappers and their Cargo features, the MMC5 two-variant
switch, the AUDIO-1/AUDIO-2 outputs, the RO-1/RO-3 options layout, the editor mode, and the
measured shapes of the three core files — the DMA state machine's two latch sites, `Ppu2C02`'s
field order, and MF-LEG-2's split of `capture_background_row`.
