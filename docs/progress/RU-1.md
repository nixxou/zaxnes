# RU-1 — Profiles, import and library of 3dSen 2.0 (AMD-039, AMD-053, AMD-056)

Branch `rustupdated`. The application is the Rust workspace `rust/`; the .NET lots V2-1, V2-15 and
V2-27 of `dotnetupdated` are read as a reference only.

## Checklist

| # | Step | Status |
|---|---|---|
| 1 | Setup: sources read, requirement map, amendments copied | done — `f2a05fe` |
| 2 | AMD-039: UnityFS header flag 0x200, synthetic format-8 bundle tests | done |
| 3 | Profiles layer: `Ver` in the header, sanitised version text, `obsolete`, inspect/apply split | todo |
| 4 | AMD-053 favourites, search, "Favorites first" order (`zax-profiles`, `library.json` v2) | todo |
| 5 | AMD-053 resume point (`resume` store, exit write, start-up offer) | todo |
| 6 | AMD-053 command line: `-slot=<n>`, `-noresume`, `-emustats` and `--` aliases | todo |
| 7 | AMD-056 import dialogs, desktop frontend, both languages | todo |
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

### Next step

Step 3: the profiles layer of AMD-053 and AMD-056 — field 2 `Ver` in the header, the sanitised
version text, `obsolete`, and the inspect/apply split the dialogs need.
