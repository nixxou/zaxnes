# RU21B — AMD-065: a 2.1 profile replaces the 2.0-or-older profile of the same game

Lot of three steps. Normative source: `spec/changes/AMD-065-generation-remplace.md`
(REQ-PROF-085 to REQ-PROF-087), amending AMD-062 §8, AMD-037 and AMD-039 §5.

- [x] 1/3 — Plan: where "same game" is decided, and how the replaced entry's identifier
  reaches `rom.json`, the saves and the cover.
- [x] 2/3 — Implementation and tests: REQ-PROF-085 to 087, AMD-065 §3's automatic criteria,
  and the real-install test made to hold for a 2.0 **and** a 2.1 installation.
- [ ] 3/3 — Final pass: fmt, clippy `-D warnings`, `cargo test --release` of the crates
  touched, performance note.

## Step 1 — the plan

### What the three requirements ask

- **REQ-PROF-085** generation: **2.1** when the content is a 3dh (`hash::envelope::is_hash_profile`,
  envelope signature `"3dh"`), **earlier** otherwise. Read at the content, never at the extension:
  the 3dSen import writes a 3dh under the name `<name>.3dn` (AMD-062 §8).
- **REQ-PROF-086** same game, either condition:
  1. the same `Id` **read in the file** — field 7 of a 3dn, field 7 of a 3dh envelope — non-empty.
     A derived identifier (AMD-037) does **not** count, so the comparison must see the header's
     `Id`, not the library identifier.
  2. the same profile name: file name without extension (for a built-in profile, the file name
     `file` names in the catalogue), case-insensitive, `'` counting as `_`.
- **REQ-PROF-087** replacement: one entry per game as soon as one of its profiles is 2.1; the
  content is the 2.1 profile's, the earlier ones add nothing, the entry keeps the **identifier**
  of the entry that was there before, and the **title** is the 2.1 profile's.

### Where "same game" must be decided

Three places, all at load time:

1. `GameLibrary::build` (`zax-profiles/src/library/game_library.rs`) — the catalogue entries are
   added first, then the user profiles. This is where a user 3dh takes over a built-in entry and
   where a user 3dn of a game a 3dh already holds adds nothing.
2. `GameLibrary::add_user_profile` — the same decision for a single profile, which is what the
   interface import (REQ-UI-214) and the 3dSen import
   (`LibraryGameCatalog::add_imported_profiles`) go through. Without it, importing the 101
   profiles of a 2.1 installation while the application runs would add 101 entries.
3. `LibraryGameCatalog::open` (`zax-app/src/library_game_catalog.rs`) — it must hand the library
   the `Id` each built-in profile **file** carries, which only condition 1 needs and which the
   catalogue JSON does not hold.

Nothing else: the scan of the profile folders (`UserProfileFolder::scan_excluding`,
`ProfileFolders::scan_excluding`) is left alone on purpose. AMD-065 asks for one **library
entry**, not for a file to be ignored — « rien n'est supprimé ni réécrit », and the 2.1
acceptance criterion asks for « aucun fichier ignoré ». The two formats of one game carry
different identifiers, so both files pass the scan and the library is what drops one of them.

`UserProfileFolder::inspect` is left alone too: its `Replaced` outcome means the old **file** is
deleted by `apply`, which AMD-065 forbids. A 2.1 file whose game is already in the library is
therefore `Added` — the file is copied, nothing is deleted — and the library entry is taken over
by `add_user_profile` afterwards.

### How the identity reaches the code

- A user profile already carries everything: `UserProfile::header` holds the `Id` read in the file
  (none for the 45 profiles without one — the derived identifier lands in `UserProfile::id`
  alone, never in the header) and its signature says the generation; `UserProfile::file_path`
  gives the name.
- A built-in profile gives its name without any reading (`CatalogueEntry::file`), but its `Id`
  needs the file. The bytes of the 101 files are **already** read at start-up for AMD-037's
  byte-for-byte comparison (`ProfileCatalogue::built_in_contents`), so a single new reader returns
  both and nothing is read twice.

New type `GameIdentity` (`zax-profiles/src/catalogue/game_identity.rs`) with `ProfileGeneration`;
`ProfileCatalogue::read_built_in_profiles` returns the bytes **and** the header of each file.
`GameLibrary::build` keeps its four-argument form — the catalogue entry names alone already
decide condition 2 — and a five-argument `build_with_built_in_profiles` adds condition 1.

### How the replaced entry's identifier reaches `rom.json`, the saves and the cover

The entry keeps the identifier it had, so nothing downstream moves:

- `rom.json` (REQ-PROF-064) is applied by `GameLibrary::apply_rom_paths`, keyed on `record.id` →
  `find_mut(id)`: a built-in game keeps its catalogue identifier (`"1"` to `"101"`) and keeps its
  ROM.
- `library.json` (`GameLibrary::to_file`, `apply`) is keyed on the same identifier.
- the cover is keyed on the title and the identifier (`persistence::game_art`), and the saves on
  the identifier, so both stay attached.

A library entry therefore has to remember the identity of the profiles it **replaced** as well as
that of the profile it shows, so that a third profile of the same game still finds it by whichever
of the two conditions matches.

### The data, checked against `assets-neutral/`

The 101 files of `profiles-2.1/` are all 3dh and all carry an envelope `Id`; the 101 built-in
profiles are all 3dn and 56 of them carry an `Id`. Pairing them gives **56 by condition 1, 45 by
condition 2, 0 unpaired**, 101 distinct targets, and no file whose two conditions point at two
different games. `Hogan's Alley` (envelope `Id` `3348a6f46bb7`) pairs with the built-in
`Hogan_s Alley` by the apostrophe rule, as AMD-065 §3 states.

### Points settled by the reading, and worth a reviewer's eye

- **Identifier of the entry.** « quand celle-ci existait avant » is read as the build order: the
  entry already in the library when the 2.1 profile is added keeps its identifier. A 2.1 profile
  read first keeps its own, which is the « sinon » of the rule.
- **A replaced built-in entry becomes deletable.** Its source is `User` and its file is the user's
  3dh, so `can_delete()` is true and deleting it deletes that file alone — the built-in asset is
  untouched (AMD-030 §5) and the built-in entry comes back at the next start. This is the only way
  back from an imported 2.1 profile, and REQ-UI-037 protects the built-in **profile**, which the
  entry no longer uses.
- **A title the user chose still wins.** `library.json`'s title is applied after the build, as it
  already was for every user profile; REQ-PROF-087's title is the one derived from the profile.

## Step 2 — the implementation and its tests

New in `zax-profiles`:

- `catalogue/game_identity.rs` — `ProfileGeneration` (REQ-PROF-085, read at the content through
  `hash::envelope::is_hash_profile`, never at the extension) and `GameIdentity` (REQ-PROF-086: the
  `Id` read in the file, which never holds a derived identifier, and the normalised profile name —
  case folded, `'` as `_`).
- `ProfileCatalogue::read_built_in_profiles` returns `BuiltInProfiles`: the bytes AMD-037 compares
  a user file against **and** the header each built-in file carries, read once for the two uses.
  `built_in_contents` is kept and is now a thin wrapper over it.
- `GameLibraryEntry` remembers the identity of the profile it shows and of those it has replaced,
  and `take_over` applies REQ-PROF-087: the identifier stays, the title, the file, the source and
  the origin come from the 2.1 profile. `GameLibrary::build_with_built_in_profiles` is `build`
  plus condition 1 for the built-in profiles.

### A fourth place the rule had to reach

Step 1's plan named three. A failing test found the fourth: the **folder scan** yields one profile
per identifier (REQ-PROF-062, « the first read wins »), so for the 56 games whose 3dn and 3dh carry
the *same* `Id` it dropped one of the two files before the library ever saw it — and which one
depended on the file names. `UserProfileFolder::scan_excluding` and `ProfileFolders::scan_excluding`
now keep the 2.1 file when the two generations differ, and log a line saying the other file stays
on the disk. Between two profiles of the same generation they are unchanged.

`GameLibrary::add` now asks REQ-PROF-087 **before** REQ-PERS-030's duplicated-identifier rule,
since the test only fires when one of the two profiles is a 2.1.

### Tests

- `catalogue/game_identity.rs` unit tests: the name normalisation, condition 1, condition 2,
  neither, and the generation read at the content.
- `tests/profiles/generation_replacement_tests.rs`, 15 tests: both directions of REQ-PROF-085
  (a 3dh named `.3dn` replaces, a 3dn named `.3dh` does not), the two conditions of REQ-PROF-086
  with the apostrophe case, a derived identifier that is not a file `Id`, the generic entries that
  designate no game, and for REQ-PROF-087 the kept identifier through `apply_rom_paths` and
  `to_file`, the title, the single entry for a user 3dn plus a user 3dh with the 3dn still byte for
  byte on the disk, both read orders, the unchanged behaviour between two earlier generations
  (AMD-039 §5), the run-time import in both directions, and an entry still found under the name it
  was replaced under. Two `#[ignore]`d tests run AMD-065 §3's first criteria on `assets-neutral/`:
  three 2.1 profiles (one by `Id`, two by name) and then **all 101**, which leave exactly the
  catalogue's entries, each shown from its 2.1 profile, nothing derived and nothing ignored.
- `hash_profile_library_tests::amd_039_a_two_zero_and_a_two_one_profile_of_the_same_game_keep_their_identifiers`
  encoded the sentence AMD-065 §4 replaces (two entries accepted). It is now
  `amd_065_a_two_zero_and_a_two_one_profile_of_the_same_game_make_one_entry`, and checks the 101
  real pairs under the names their two installations give them: 56 paired by their `Id`, 45 by
  their name, one entry each, the 3dn untouched.
- `three_d_sen_real_install_tests::amd_037_…` now reads the generation at the content of what the
  import wrote, and expects, on a 2.1, 0 derived identifiers and a library of exactly the
  catalogue's size with every entry 2.1; on a 2.0 and a 1.x, exactly what it expected before.

### What was run

`cargo test -p zax-profiles --test profiles -- --include-ignored` with `ZAXNES_PROFILES`: 350
passed, 6 failed — the six that need `ZAXNES_3DSEN_INSTALL`, which this machine's clean side does
not have (R0 runs them). `cargo test --release -p zax-app --features bundled`: all green.
`cargo clippy -D warnings` clean on both crates, `cargo fmt --check` clean.
