# KIRBY-MERGE — bring `feat/widescreen-kirby` onto `dev`

Lot: merge the branch `feat/widescreen-kirby` (36 commits, diverged from `dev` at `514570fb`,
2026-11-06) into `dev` (`19c5065d`, 506 commits since the same point). Despite its name the
branch carries five deep mechanisms besides Kirby's pack: the widescreen extension's per-page
sprite high bytes, the wide 2D picture's margins at a split, run-ahead up to 10, the experimental
smooth scrolling at 120 Hz, and Kirby's margin decoder with its hack-pack aids.

Steps: 1 plan · 2 merge · 3 behaviour · 4 measurements · 5 final pass.

- [x] **Step 1/5 — plan** (this note)
- [x] **Step 2/5 — merge and resolve** (§2)
- [ ] Step 3/5 — behaviour
- [ ] Step 4/5 — measurements
- [ ] Step 5/5 — final pass

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
