# RU-1 — Profiles, import and library of 3dSen 2.0 (AMD-039, AMD-053, AMD-056)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-1, V2-15 and
V2-27 of `dotnetupdated` are read as a reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, amendments copied | done — `f2a05fe` |
| 2 | AMD-039: UnityFS header flag 0x200, synthetic format-8 bundle tests | done — `837918c` |
| 3 | Profiles layer: `Ver` in the header, sanitised version text, `obsolete`, inspect/apply split | done — `5030db6` |
| 4 | AMD-053 favourites, search, "Favorites first" order (`zax-profiles`, `library.json` v2) | done — `bc46821` |
| 5 | AMD-053 resume point (`resume` store, exit write, start-up offer) | done — `032638a` |
| 6 | AMD-053 command line: `-slot=<n>`, `-noresume`, `-emustats` and `--` aliases | done — `b789322` |
| 7 | AMD-056 import dialogs, desktop frontend, both languages | done |
| 8 | UI wiring: commands, chooser, console/handheld screens, options | todo |
| 9 | Closing: fmt, clippy, release tests, Lua API dump, journal | todo |

## Requirement map

### AMD-039 — importing the 3dSen 2.0 bundle

| Requirement | Rust crate / file |
|---|---|
| AMD-030 §4.1 bit 0x200: data blocks aligned on 16 after the *compressed* information block | `zax-profiles/src/import/bundle.rs` (`DATA_ALIGNED_FLAG`, the `INFO_AT_END_FLAG == 0` arm) |
| formats 6 to 8, platform and serialized-file version 22 ignored | `zax-profiles/src/import/bundle.rs`, `serialized_file.rs` — already there, nothing to change |
| path pattern `StandaloneWindows64` | `zax-profiles/src/import/installation.rs` — unchanged in 2.0, nothing to change |
| §4 acceptance criteria (three synthetic cases) | `zax-profiles/tests/profiles/common/synthetic_bundle.rs`, `unity_bundle_tests.rs` |

The .NET fix (V2-1, `bac8554`) is 18 lines in `UnityBundle.cs`: one flag constant and one
`reader.Align(16)` before `dataStart` is taken. The Rust reader has the same shape.

### AMD-056 — import texts and dialogs

| Requirement | Rust crate / file |
|---|---|
| REQ-UI-084 text 2: version **sanitised** (REQ-PROF-021) | `zax-profiles/src/catalogue/user_folder.rs` (`import_messages::unsupported_version`), which calls `profile::version_rules::sanitize` |
| REQ-UI-084 text 3: space after the path, `obsolete` | same module (`import_messages::obsolete_format`), plus `ui/strings/english.rs` and `french.rs` (`notify.obsolete_profile_format`) |
| REQ-UI-214 dialogs (refusal / built-in / replace / nothing) | `zax-app/src/ui/zax_ui/mod.rs` (`import_profile`), `ui/zax_ui/dialogs.rs`, new `ui/profile_import.rs`; the dialog mechanism already exists (`emulation/host/games.rs` `ConfirmationRequest`, `inform`) |
| non-writing inspection before the dialog | `zax-profiles/src/catalogue/user_folder.rs`: `inspect` → `ProfileImportPlan` → `apply`, `import` keeping its current meaning for everything with no dialog |
| dialog titles and buttons in the texts table, both languages | `zax-app/src/ui/keys.rs`, `ui/strings/english.rs`, `ui/strings/french.rs` |
| a successful import shows nothing and selects the entry | `zax-app/src/ui/zax_ui/mod.rs`, `ui/profile_chooser.rs` |

### AMD-053 — resume point, favourites, search, import cause, command line

| Requirement | Rust crate / file |
|---|---|
| REQ-LOOP-076 resume point, write at exit | `zax-app/src/emulation/host/resume.rs` (new), called from `EmulationHost::shutdown` |
| the `resume` store itself | `zax-app/src/persistence/resume_point.rs` (new) |
| start-up offer, `resume.last` command | `zax-app/src/emulation/commands.rs`, `emulation/host/commands.rs`, `ui/zax_ui/menu.rs`, `ui/handheld/*` |
| REQ-UI-210 favourites: `favorite` field, `favorite.toggle`, `library.filter_favorites` | `zax-profiles/src/library/game_library.rs`, `library/store.rs` (schema 2 + `.v1.bak`), `zax-app/src/emulation/commands.rs`, `emulation/host/*` |
| REQ-UI-211 search, accents and case ignored | `zax-profiles/src/library/search.rs` (new, the Latin table of Q-090), used by `GameLibrary::enumerate` and `ui/profile_chooser.rs` |
| REQ-PERS-033 "Favorites first" | `zax-profiles/src/library/game_library.rs`, `zax-settings/src/table.rs` (domain value 2), `zax-app/src/persistence/app_settings.rs` |
| REQ-UI-212 cause of a refused import | `zax-profiles/src/catalogue/user_folder.rs` (needs field 2 `Ver` in the header: `format.rs`, `wire/decoder.rs`) |
| REQ-UI-213 command line | `zax-app/src/app_options.rs`, `zax-app/src/application.rs` (`pending_state_load`), `zax-app/src/app_log.rs` |

### Divergences from the .NET lots, and why

- **`resume` is not a slot of `saves/index.json`.** The Rust application has no shared slot index:
  a ROM's states are its own files (`<rom>.ssN`), read back from the files, and `index.json` only
  survives as the legacy store that is emptied once (`persistence/save_slots.rs`). AMD-053 §3
  point 12 (38 entries in `index.json`) therefore has nothing to amend here. The resume point gets
  its own single-file store (`saves/resume.state`, `saves/resume.png`) whose metadata — game and
  instant — are read from the save-state header the Rust loader already carries. Recorded as
  Q-102.
- **No `-id=` option.** REQ-UI-007's `-id=` was never implemented in the Rust application; the
  game is named by `--rom` / `--profile`. `-slot=<n>` therefore reads "the slot of the game this
  run launches", and the "slot of another game" rule falls back on the library lookup that the
  quick slot already does.
- **`--load-state <n>` already exists** (Rust-only). `-slot=<n>` is routed through the same
  `pending_state_load` field; nothing is duplicated.
- **`-emustats` reuses the frame profiler's clock**, not a new measurement in the frame loop.

### Performance

PERFORMANCE-HISTORY §10 names, for this lot, only the `Zax.Settings` row: *handheld defaults
(`ui/handheld/defaults.rs`), handheld option rows, desktop `EXCLUDED`, settings catalogue test*.
Nothing in this lot runs per frame, per tile or per pixel:

- the resume point is written once, at exit, after the emulation loop has stopped;
- the search and the favourites filter run when the list is rebuilt, never per frame;
- `-emustats` is the one addition inside the frame loop and it is off unless the option is given;
- no new setting is read in a hot path, and `ui.sort_option` only gains a domain value.

Handheld defaults: the resume point is on (it costs nothing during play), favourites are offered
(the list shows the marker without text input), the search is not offered (no keyboard).

## Notes

### Building `zax-app` in this working copy (read this first if the build fails)

Two things stood in the way of compiling `zax-app`, neither of them caused by this lot. Both are
worked around locally, outside the repository:

1. **The two demo-game binaries are not in this checkout.** `src/demo_game.rs` does
   `include_bytes!("../demo/FlappyZaxNes.nes")` and `include_bytes!("../demo/flappyzaxnes.3dn")`,
   and `.gitignore` excludes `*.nes` and `*.3dn`, so they are never committed: a fresh checkout
   cannot build the crate. Stand-ins were made here so the lot could be compiled and tested:
   the `.3dn` by `zax-harness profile-build` from `demo/source/profile/flappyzaxnes.json` with its
   `"template": "empty.3dn"` line removed (that template is not in the tree either), and the
   `.nes` by hand (a minimal iNES image). **Neither is the shipped FlappyZaxNes**, both are
   untracked and ignored, and no test reads their content — `asset_free_startup_tests` only
   counts files by extension. Rebuilding the real ROM needs cc65 (`ca65`/`ld65`), which is not
   installed here.
2. **`SDL2.lib` is not on the linker path.** `cargo test -p zax-app` fails with `LNK1181` unless
   SDL2 is linked statically. `--features bundled` does that, but it needs CMake, which is not on
   `PATH` — it does ship with Visual Studio. Every `zax-app` and workspace command of this lot was
   therefore run as:

   ```sh
   export PATH="/c/Program Files/Microsoft Visual Studio/2022/Community/Common7/IDE/CommonExtensions/Microsoft/CMake/CMake/bin:$PATH"
   cargo test --release -p zax-app --features bundled
   ```

### Other notes

- `tests/` in the working tree at the start of the lot is untracked .NET build output
  (`bin/`, `obj/`, `TestResults/`, 0 `.cs` files) left over from the `src/`+`tests/` layout that
  commit `d3680e4` removed. Nothing of it is kept and nothing of it is committed.
- Reference reading: `git show dotnetupdated:<path>`, `git diff origin/archive-dotnetversion
  dotnetupdated -- <path>`; the .NET lots are V2-1 (`bac8554`, AMD-039), V2-15 (`db4909b`,
  AMD-053) and V2-27 (`9465acb`, AMD-056). Their Q tickets: Q-088 (`-noresume` suspends the
  resume point whole), Q-089 and Q-090 (answered; AMD-056 §2 supersedes Q-089 on the form of
  `<v>`).
- Q-090's problem — no NFD decomposition — exists in Rust too, for a different reason: the
  standard library has neither Unicode normalisation nor general categories, and the workspace
  must stay offline, so no crate can be added. The Latin table R1 confirmed for the .NET build is
  the whole mechanism here; combining marks are dropped by their Unicode blocks.

### Step 2 — AMD-039

The whole fix is one flag and one alignment in `import/bundle.rs`: with bit 0x200 set and the
information block first, the reader aligns on 16 after the block's *compressed* bytes before
taking `data_start`. With bit 0x80 the bit is ignored, the data already following the aligned
header. Everything AMD-030 asks for is unchanged: formats 6 to 8, LZ4 and LZ4HC, a refusal with a
message rather than a panic for anything that does not read.

Tests, all three criteria of AMD-039 §4 plus the import level:

- `SyntheticBundleOptions::three_d_sen_2()` — format 8, LZ4HC, 128 KiB blocks, serialized file 22,
  flags 0x243 — and `build_with_layout`, which reports `info_end` and `data_start` so a test can
  state the offsets §2 measures instead of recomputing them;
- `align_data_flag: Some(false)` forges the second criterion: the same bytes, padding included,
  with bit 0x200 out of the header;
- the two 2.0 layouts also joined the `every_layout_yields_the_same_text_assets` table;
- measured on the synthetic bundle: the compressed information block ends at a position that is
  not a multiple of 16, so the padding is really what the reader skips (the test asserts that
  before reading).

The optional real-install tests are now version-aware: the bundle's format version (7 or 8, read
from twelve bytes of its header) tells 1.1.2 and 2.0 apart, and the serialized-file version
follows (21 or 22). The inventory fingerprint comparison moved into a test of its own that
reports and returns on a 2.0, the inventory describing the 1.1.2 (AMD-039 §4, §5). The AMD-037
"no game twice" test now states what a 2.0 gives — the four profiles of §5 kept as user profiles,
the other 97 recognised identical, one library entry per identifier — which is Q-076, copied
alongside.

### Step 3 — the profiles layer of AMD-053 REQ-UI-212 and AMD-056

- `ProfileHeader` gained field 2 `Ver` and, with it, `format_version()` and
  `has_unsupported_version()`. SPEC-03 §4.1 calls the header the subset {1, 7, 8, 9, 10, 11};
  REQ-UI-212 needs one field more, because the cause it must name is read from the version. The
  decoder's `read_header` reads it; nothing else changed in the wire format.
- `import_messages::unsupported_version` sanitises through `version_rules::sanitize`, so `"v1"`
  reads « 3dn format version 1 » (AMD-056 §2, superseding Q-089 on the form of `<v>`). The two
  spaces before "is" stay — they are the original's. The sanitising sits in the message rather
  than at the call sites, so the import of REQ-UI-212 and the load of REQ-PERS-040 spell a version
  the same way.
- `import_messages::obsolete_format` gained the space after the path and spells `obsolete`; the
  English string table followed (`notify.obsolete_profile_format`). The French entry already had
  the space and the right word, so it is unchanged, and `notifications::translate` still matches
  the pattern.
- `UserProfileFolder::import` split into `inspect` → `ProfileImportPlan` → `apply`. `inspect`
  writes nothing and tells the four rows of the table of causes apart in the order they can be
  told apart (unreadable or malformed, wrong signature, unsupported version, missing identifier);
  it also carries the title an « Already in Zaxnes » dialogue names and the user profile a
  replacement would take over. `apply` is what writes, and what logs — including the two outcomes
  that write nothing, so that an inspection made only to choose a dialogue does not log a refusal
  twice. `import` is `inspect` then `apply`, unchanged in meaning for every caller with nowhere to
  show a dialogue.

Tests: the version cause with its sanitised text, the inspect/apply split (nothing on disk until
`apply`), a replacement plan naming the profile it takes over, a built-in-identifier plan carrying
the file's title, and the two language tests for the corrected texts.

### Step 4 — favourites, search, "Favorites first"

- `GameLibraryEntry.favorite`, `GameLibrary::set_favorite` / `toggle_favorite`, and
  `LibrarySortOption::FavoritesFirst` with `compare_by_favorite` (favourites first, each group by
  title — the criterion of A-Z).
- `GameLibrary::enumerate(option, search, favorites_only)` is the one place the two filters meet:
  an entry is kept when it satisfies **both** (REQ-UI-211). Nothing in it runs per frame; the
  interface calls it when the list is rebuilt.
- `library/search.rs` holds the normalisation. Rust's standard library has neither Unicode
  normalisation nor general categories, and the workspace adds no crate, so the Latin table R1
  confirmed for the .NET build (Q-090) is the whole mechanism here, with combining marks dropped
  by their Unicode blocks. A letter NFD does not decompose — ß, œ, Ø, Đ — is left alone, which is
  what NFD would do; a unit test asserts the two tables line up.
- `library.json` is at `schemaVersion` 2 with the `favorite` field. A version-1 file reads with
  `favorite` false everywhere (serde default), and what the requirement adds is the
  `library.json.v1.bak` copy kept before the migrated file is written again
  (`GameLibraryStore::migrated_from_version`, a `Cell` because `load` takes `&self`).
- `ui.sort_option` gained the domain value 2 ("Favorites first") in the settings table, and
  `AppSettings` gained `library_sort_order`, `slot_sort_order` (which brings 2 back to A-Z for the
  slot lists, REQ-PERS-057) and `profile_sort_option` (which writes 0 in place of 2 into profile
  field 36, AMD-053 §3 point 10). `set_sort_option` clamps to 0–2.
- `audio_input_interface` in the settings table was over the 100-line clippy limit once the third
  option went in, so the interface settings of §4.1.9 moved into their own `interface` function.
  No descriptor changed in that move.

Handheld: nothing to add to `ui/handheld/defaults.rs`. The favourites filter and the search text
are interface state, not settings, and `ui.sort_option` keeps its default of 0 everywhere.

### Step 5 — the resume point

- `persistence/resume_point.rs`: the store. Two files, `saves/resume.state` and `saves/resume.png`
  (`UserDataPaths::resume_state_file` / `resume_thumbnail_file`), whose game and instant come from
  the savestate header. `peek` validates the file whole, so a state truncated in its body is
  caught at start-up and not when the user activates the entry. Q-102 holds the reasoning for not
  being an entry of `saves/index.json`.
- `emulation/host/resume.rs`: the three erasure rules, the start-up check, the title and the
  thumbnail the entry carries, and `resume_last_game`. `prepare_resume_point` runs once, from
  `EmulationHost::new`, after the language event: it reads one file and no more.
- `jobs::load_resume_point` builds the same `LoadKind::LoadSlot` job a slot load builds, so the
  stages, the hack-version refusal and the REQ-LOOP-046 failure are the slot ones. `job.slot_id`
  is `"resume"`, which only a log line reads.
- `shutdown` now writes the resume point **before** stopping the game, which is the order
  REQ-LOOP-076 prescribes: the `Save` hook of each script and the capture (both inside
  `write_or_clear_resume_point`, through `capture_save_state`, which runs the hook), then the end
  hook and the stop, then the settings and the slot metadata. No emulated frame runs from there,
  so REQ-LOOP-041's "between two emulated frames" holds by construction.
- `-noresume` suspends the point whole — neither read, nor written, nor erased (Q-088, confirmed
  by R1). It is an option of the host (`EmulationHostOptions::suspend_resume`), not a setting.
- The four commands of AMD-053 §3 point 17 joined the model: `resume.last`, `favorite.toggle`,
  `library.filter_favorites` (a tickable command) and `library.search`, with their labels in both
  languages from AMD-053 §4. The favourites filter and the search text are host state, not
  settings: nothing of them is persisted, and the `GameCatalog` trait gained `is_favorite` and
  `toggle_favorite` (defaulted, so a catalogue with no library behind it is unaffected).
- Test harness: `LoopHarness::restart_with` gives the next launch over the same user data folder,
  with the library put back **before** the host is built — the order of the real application. The
  harness also records what the scripts `print`, which is how the hook order is checked.

Nothing of this runs per frame: the write happens once, after the loop has stopped, and the
start-up check reads one file.

### Step 6 — the command line

- `parse_original_option` takes the three options before the general `match`, so their `=` form
  and their double-dash aliases are read in one place. `-slot` implies `-noresume`, whatever order
  they came in.
- `--load-state <n>` and `-slot=<n>` fill the **same** field, now `slot: Option<String>` — nothing
  is duplicated, and `-slot` adds the quick slot and the 36 slots REQ-PERS-050 keeps (the exposure
  limit of REQ-UI-208 does not reach the command line). An out-of-domain value is ignored with a
  notice, never an error, and the notices are logged at start-up.
- `load_the_asked_state` routes `quick` to `Command::QuickLoad` and a number to
  `Command::LoadSlot`, after step 7 of REQ-LOOP-021 (the state having gone to Running). An empty
  slot is ignored, the game stays launched, and the fact is logged.
- **The "slot of another game" clause of REQ-UI-213 has no referent here**, and Q-102 gained a
  section for it: it presumes a common slot index to look the game up in, which the Rust
  application no longer keeps, and `-id=` was never implemented. `-slot=<n>` is the slot of the
  game this run starts. Nothing was invented to scan `saves/` for a game.
- `-emustats` writes its line in `count_emulated_frame`, which every path runs, so the count is
  exact (the acceptance criterion is 600 ± 1 lines for 10 s of NTSC). Off, it is one
  `Option::is_some` per emulated frame and nothing more; the sink is absent rather than a flag, so
  there is nothing to format and nothing to write. The line goes through `app_log::file_line`,
  which writes to the log file only and never to the parent console, as the requirement asks. With
  the emulation on a separate core the frame leaves for the thread, so the line says so instead of
  reporting a span the host did not spend.

### Step 7 — the import dialogues of AMD-056 REQ-UI-214

- `ui/profile_import.rs` decides which dialogue a plan calls for, and with what words. It is a
  pure function of the plan, so the four rows of the table are checked with no window open; the
  **form** stays free (AMD-056 §3, AMD-010 §4), and only the texts and the buttons are fixed.
- `LibraryGameCatalog` gained `inspect_profile_import` and `apply_profile_import`;
  `import_profile` is the two in a row, which is what every non-interactive caller still uses. A
  write that fails after the confirmation carries the message of its own error, not the third text
  of REQ-UI-084, which would say nothing true there.
- The desktop flow: an unknown identifier goes straight through, showing nothing, and the imported
  entry becomes the chooser's selected one; a refusal and an « Already in Zaxnes » apply the plan
  at once (that is what logs the fact, REQ-PERS-041) and then report it; a replacement waits for
  `Replace` and writes nothing before. `Cancel`, or the closing of the dialogue, changes nothing.
- The dialogue is drawn where the confirmation box of REQ-UI-110 is, last and over everything, and
  shares `state.dialog`; it is the interface's own, so `sync_confirmation` keeps the flag up for
  it. The destructive first button is red.
- Six keys, with their entries in both tables: `button.ok`, `button.replace` and the three
  title/body pairs. « Replace » names the action, so the French entry is « Remplacer », not the
  « Oui » of REQ-PERS-042.
- Tests: the five criteria of REQ-UI-214 one by one, plus the three dialogues in French.

Nothing of this is in a per-frame path: the plan is read when a file is chosen, and the dialogue
is a value the drawing reads.

### Next step

Step 8: the UI wiring of AMD-053 — the resume entry, the favourites marker and filter, the search
field, in the desktop chooser and the console/handheld screens.
