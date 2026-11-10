# IMP66 — AMD-066: importing a single profile file that carries no identifier

Lot of three steps. Normative source: `spec/changes/AMD-066-import-sans-identifiant.md`
(REQ-PROF-088 to REQ-PROF-090), amending AMD-037 §2 and §4 and completing REQ-PROF-063 and
AMD-053/AMD-056 REQ-UI-212 and REQ-UI-214.

- [ ] 1/3 — Plan: where the built-in bytes must reach `inspect`, and which tests assert the
  refusal AMD-066 removes.
- [x] 2/3 — Implementation and tests: REQ-PROF-088 to 090 and AMD-066 §3's automatic criteria.
- [ ] 3/3 — Final pass: fmt, clippy `-D warnings`, `cargo test --release` of the crates
  touched, performance note.

## Step 1 — the plan

### What the three requirements ask

- **REQ-PROF-088** — a chosen file whose **bytes** are those of a built-in profile of the
  catalogue in use gives the « already built-in » outcome, `Id` or no `Id`. Nothing is copied.
- **REQ-PROF-089** — a readable file of an accepted signature and a supported version whose
  `Id` is absent or empty and which REQ-PROF-088 does not cover **is accepted**: copied byte for
  byte, keyed by AMD-037's derived identifier — the very one the next folder scan will give it —
  which is what the replace detection uses and what the outcome shows. The file is never
  rewritten.
- **REQ-PROF-090** — the absence of an `Id` is no longer a cause of refusal. The « obsolete or
  wrong profile format » text stays for a missing, unreadable or malformed file and for the 3dh
  causes of REQ-PROF-072; the signature and version texts keep their own.

### Why the `Id` branch cannot do the work of REQ-PROF-088

`UserProfileFolder::inspect` recognises a built-in profile by looking its `Id` up in
`built_in_ids`, and the app fills that list with the **catalogue** identifiers of the library
entries whose source is `BuiltIn` — `"1"` to `"101"` (AMD-037 §3). The `Id` a built-in profile
*file* carries is a 12-character one (AMD-065 REQ-PROF-086 point 1), which is never in that
list. So neither of the two halves of REQ-PROF-088 — with or without `Id` — can be reached
through `built_in_ids`: the comparison has to be on the **bytes**, as AMD-037 already does for
the folder scan (`UserProfileFolder::is_built_in_copy`).

That decides the order inside `inspect`: file read → signature → version → **bytes against the
built-in contents** → identifier (carried, else derived) → built-in identifier → replaced/added.

### Where the built-in bytes must reach `inspect`

They are already read once at start-up, for AMD-037 and AMD-065:

- `ProfileCatalogue::read_built_in_profiles` (`zax-profiles/src/catalogue/profile_catalogue.rs`)
  returns `BuiltInProfiles { contents, headers }`;
- `LibraryGameCatalog::open` (`zax-app/src/library_game_catalog.rs`) calls it, hands
  `built_in.contents()` to the folder scan, and keeps `built_in.into_contents()` in the field
  `built_in_contents` for `add_imported_profiles`.

So the single production call site, `LibraryGameCatalog::inspect_profile_import`, has the bytes
in hand: it only has to pass `&self.built_in_contents` on. `UserProfileFolder::inspect` and
`UserProfileFolder::import` gain a `built_in_contents: &[Vec<u8>]` parameter next to
`built_in_ids`, rather than a second `_excluding` method: every caller must decide which
catalogue it compares against, and a silent `&[]` default would hide REQ-PROF-088.

Call sites to update (14): `zax-app/src/library_game_catalog.rs:203`;
`zax-profiles/tests/profiles/catalogue_and_library_tests.rs` (10),
`hash_profile_library_tests.rs` (2), `program_profiles_folder_tests.rs` (1).

`LibraryGameCatalog::from_library`, which the interface harness uses, leaves
`built_in_contents` empty — the harness catalogue has no profile files at all — so the
interface criterion of AMD-066 §3 needs a harness built over a real asset folder
(`LibraryGameCatalog::open`), which `UiHarness` does not offer yet.

### The tests that assert the refusal AMD-066 removes

- `zax-profiles/tests/profiles/catalogue_and_library_tests.rs`,
  `req_prof_063_importing_a_profile_without_an_identifier_is_refused` — the only test of the
  old rule. It is **replaced** by the new one: the same file, with no built-in content to
  compare against, is now accepted under its derived identifier (AMD-066 REQ-PROF-089).
- `user_folder.rs`, row 4 of the table of causes in the doc comment of `inspect` and the
  `!header.has_id()` branch: both go (REQ-PROF-090). The doc of `import_messages::obsolete_format`
  says « unreadable or has no identifier » and must lose the second half.
- Nothing else asserts it: no test reads the `apply` log line of an `AlreadyBuiltIn`, and the
  dialogue tests of `zax-app/tests/profile_import_dialog_tests.rs` all use files that carry an
  `Id`.

### What AMD-066 §3 asks for, as tests

1. (`zax-profiles`, needs `ZAXNES_PROFILES`) `assets-neutral/profiles/Contra.bytes` chosen as
   `Contra.3dn` against the catalogue of `assets-neutral/`: outcome `AlreadyBuiltIn`, nothing
   written in the user folder.
2. (`zax-profiles`) a synthetic 3dn with no `Id`, unlike every built-in profile: `Added`, copied
   byte for byte, `plan.id` equal to `derived_id` of its bytes and to the identifier the next
   `scan` gives it; the folder holds one profile more.
3. (`zax-profiles`) the same file imported again: `Replaced`, same identifier, never a second
   entry.
4. (`zax-profiles`) a truncated file and a malformed protobuf stay refused with the « obsolete »
   text; a foreign signature and a `Ver` of `"1"` keep theirs.
5. (`zax-app`, needs `ZAXNES_PROFILES`) the import dialogue shows, for that `Contra.3dn`,
   « Already in Zaxnes » in English and « Déjà dans Zaxnes » in French.

### Two side effects of the rule

- The `apply` log line of an `AlreadyBuiltIn` says « the identifier "X" belongs to a built-in
  profile », which is untrue of a file recognised by its bytes under a derived identifier. It
  becomes a sentence true of both halves of REQ-PROF-088.
- `plan.id` of an `AlreadyBuiltIn` is the carried `Id` when there is one, else the derived
  identifier, so that the field is never empty outside a refusal, as its doc says.

### Performance

Import-time only: one `Vec<u8>` comparison per built-in profile, on bytes already in memory,
and one SHA-256 of the chosen file when it carries no `Id`. No per-frame cost.

## Step 2 — what was written

### `zax-profiles/src/catalogue/user_folder.rs`

- `inspect` and `import` take a fourth argument, `built_in_contents: &[Vec<u8>]`, between
  `built_in_ids` and `user_profiles`. The bytes the chosen file is read with are kept rather
  than dropped after `load_header`, since both new rules need them.
- Order of the decisions, after the three causes of refusal that remain: **bytes against
  `built_in_contents`** (REQ-PROF-088) → identifier, carried or `derived_id` of the bytes
  (REQ-PROF-089) → `built_in_ids` → replaced/added. The byte test comes first because the `Id`
  a built-in file carries is never a catalogue identifier.
- The `!header.has_id()` refusal is gone (REQ-PROF-090), and with it row 4 of the table of
  causes in the doc comment and the second half of the doc of `import_messages::obsolete_format`.
- New private `already_built_in(source, id, header)`, the plan the two ways of recognising a
  built-in profile share.
- The `apply` log line of an `AlreadyBuiltIn` no longer claims that the identifier belongs to a
  built-in profile — untrue of a file recognised by its bytes under a derived identifier. It
  names the title instead.

### `zax-app`

- `LibraryGameCatalog::inspect_profile_import` passes `&self.built_in_contents`, the bytes
  `LibraryGameCatalog::open` already keeps from the start-up reading.
- `UiHarness::with_asset_root` (test support): a harness whose library comes from
  `LibraryGameCatalog::open` over a real asset folder, so the built-in **bytes** are there.
  `UiHarness::new` and it share a new private `over`.

### Tests, one per criterion of AMD-066 §3

| Criterion | Test |
|---|---|
| 1 — `Contra.bytes` as `Contra.3dn`, catalogue of `assets-neutral/` | `catalogue_and_library_tests::req_prof_088_the_bytes_of_the_built_in_contra_are_already_in_the_catalogue` (ignored, needs `ZAXNES_PROFILES`) |
| 2 — a synthetic 3dn with no `Id` | `req_prof_089_importing_a_profile_without_an_identifier_uses_its_derived_one` |
| 3 — the same file imported again | `req_prof_089_reimporting_the_same_identifier_less_file_replaces_its_entry` |
| 4 — truncated and malformed files | `req_prof_090_an_unreadable_file_keeps_the_obsolete_format_message` |
| 5 — the dialogue in English and French | `zax-app` `profile_import_dialog_tests::req_prof_088_the_built_in_contra_is_already_in_zaxnes_in_both_languages` (ignored, needs `ZAXNES_PROFILES`) |

`req_prof_088_importing_the_bytes_of_a_built_in_profile_imports_nothing` covers the two halves
of REQ-PROF-088 — with and without an `Id` — on synthetic bytes.

The one test of the old rule,
`req_prof_063_importing_a_profile_without_an_identifier_is_refused`, is replaced by the second
of these; the thirteen other call sites of `inspect`/`import` gained a `&[]`.
