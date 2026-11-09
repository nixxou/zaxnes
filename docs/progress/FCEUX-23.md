# FCEUX-23 — the engine abstraction (FCEUX-2) and the per-game choice (FCEUX-3)

Branch `feat/fceux-engine`, worktree `cleanroom/wt-fceux2`, base `dev` `591836cc`.

Design note: `docs/review/FCEUX-ENGINE.md` §2 (the trait), §3 (savestates), §4 (the per-game
choice), §6 (the 3D state). Previous lots: `docs/progress/FCEUX-1.md`, `docs/progress/FCEUX-6.md`.

## Steps

- [x] 1. Bring the FCEUX work onto this branch by cherry-pick; the baseline.
- [ ] 2. `zax-core-api`: the types both engines share, a pure move.
- [ ] 3. The `Engine` trait, and both implementations.
- [ ] 4. The application drives the trait.
- [ ] 5. The per-game choice (FCEUX-3).
- [ ] 6. The app with FCEUX, end to end.
- [ ] 7. Docs.
- [ ] 8. Final pass.

## The environment this lot ran in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| the fork | `cleanroom/fceux-zax` branch `zax`, its existing MSVC build linked, **not rebuilt** |
| `rust/test-roms` | a directory junction to `wt-fceux`'s copy (gitignored; the set lot FCEUX-6 used) |
| SDL2 | `--features zax-app/bundled`, since no `SDL2.lib` is on the machine's link path |

---

## Step 1 — the cherry-pick, and the baseline

`git cherry-pick c0a97023..feat/fceux-core`, 19 commits in order, no merge. They became
`ff8bd82f..7aaa9755`.

### The one conflict, and how it was resolved

**`rust/crates/zax-harness/src/main.rs`**, at commit `ed74ea0e` *FCEUX-1 (4/8): the Rust side*.
Both sides had grown the same four places since `c0a97023`:

| Place | `dev`'s side | the branch's side | kept |
|---|---|---|---|
| `USAGE` | a `classify` line | four FCEUX lines | **both**, `classify` then the FCEUX four |
| `run`'s match arms | a `classify` arm | `fceux-run`/`-frame`/`-tiles`/`-bench`/`-compare` arms, each `#[cfg(feature = "fceux-core")]` | **both**, `classify` first then the five |
| the help function | `print_help()`, prints, takes `classify::USAGE` as a ninth paragraph; its doc comment records the LUA-MERGE reason it exists | `help() -> String`, returns, takes eight and appends `{fceux_usage}` | **`dev`'s shape** (`print_help`, prints) **with the branch's `fceux_usage`**: the branch's reason for the function (clippy's line budget on `run`) and `dev`'s are the same reason, and `dev`'s doc comment states it, so the one that survives is the one with the history in it |
| the `fceux_usage` block | — | `#[cfg(feature = "fceux-core")]` paragraph list | **kept verbatim**, and the later commits of the branch (`0ed86250`, `0fc216d3`) grew it to four paragraphs on top of this resolution without a second conflict |

Checked against the branch's own file afterwards: `git show feat/fceux-core:…/main.rs` differs from
the resolution by exactly the four `classify` additions and the `print_help` shape, and by nothing
else. Carried forward as the branch had it, deliberately: the FCEUX lines of the `USAGE` **const**
are unconditional on the branch, so a build without `fceux-core` lists five commands it does not
have. That is the branch's behaviour, not a conflict, and not this step's to change.

The remaining 18 commits applied with no conflict; `lib.rs` and `main.rs` auto-merged twice more.

### What the cherry-pick changed, and what it did not

`git diff 591836cc..HEAD --stat`: 22 files, +8 078/−6. **`zax-nes` is not among them.** The only
file of an existing crate that changed apart from the harness's `main.rs`, `lib.rs` and
`Cargo.toml` is `golden_core.rs`, and its whole diff is three `fn` → `pub(crate) fn`
(`collect_roms`, `relative_name`, `hex`) so that `fceux_run` can reuse them. No expression changed,
so golden-core's output cannot have.

### Builds and tests

| | |
|---|---|
| `cargo build --workspace --release --features zax-app/bundled` | ok |
| `cargo build --workspace --release --features zax-app/bundled,zax-harness/fceux-core` | ok |
| `cargo test --release -p zax-fceux -p zax-harness` (feature **off**) | all green, 65 tests, 1 ignored |
| the same `--features zax-fceux/fceux-core,zax-harness/fceux-core` | all green, 89 tests, 4 ignored |

### `fceux-tiles` on *Super Mario Bros.* frame 200

    zax-harness fceux-tiles "…/Super Mario Bros. [World][!!].nes" … --frames 200
    1 of 1 ROMs identical at every compared frame

and the row itself: `"zaxnes":969,"fceux":969,"same_in_order":969,"same_as_multiset":969,`
`"pixels_identical":true,"class":"identical"`. **969 / 969**, as FCEUX-6 step 2 left it.

### The baseline of this lot

**golden-core over the 98 originals** (`G:/3DSen/test-originals`, 1 200 frames, feature off):

| build | sha256 of the `.jsonl` |
|---|---|
| this branch at `7aaa9755` | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` |
| a clean build of `dev` `591836cc` (`git archive` into a fresh directory, its own target dir) | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` |

**Byte-identical**, which is what the reasoning above predicted and what the brief asked to be
verified rather than assumed. That digest is this lot's golden-core baseline; every later step is
measured against it.

**`render` baseline** (`--assets assets-neutral`, 640x480, defaults otherwise), the captures step 4
has to reproduce at tolerance 0:

| capture | sha256 |
|---|---|
| `smb-300/Super Mario Bros. [World][!!].png` | `1a04889cc411cc800b98f7af56753af1e6f6b2b8d6ab6f42512e0d67838d7303` |
| `smb-300/… -2d.png` | `e85b4826381fb7e052402dfc1bc760bb98fc0d38289ba1e518774ae1a248bc97` |
| `smb-1200/Super Mario Bros. [World][!!].png` | `ad5bb22c0a408af1b6c39a0e7eb0ea5e33b51eae1712812373aecce31b9d35d7` |
| `smb-1200/… -2d.png` | `14ce068b681fec9c0a4869aa0d24e1e8dd5e392a7b8d2432da1554149ab63ea2` |
| `smb3-300/Super Mario Bros. 3 [USA][!!].png` | `fe4b017918e007f7bc719e1db5183749bdfda4f7196fc42158d30bded14d3dcd` |
| `smb3-300/… -2d.png` | `a4188e6294e22f5a393b5919777110a2231202dda5c8beb291b50a79d11f925e` |
| `smb3-1200/Super Mario Bros. 3 [USA][!!].png` | `31a40a16bca19f5d3f1d18faf1cd8e508782706aae2bba6b095e50a0feab77b9` |
| `smb3-1200/… -2d.png` | `40bdafe9fc9b2a0af1795843bf1007bddb488f2b5dd392457c3c9e6b3ccc2946` |
| `mm3-300/Mega Man 3 [USA][!!].png` | `5fcbfb0a878f5060ebdb8a58d27fefe7df64f8948a90314e595431ff8b84e462` |
| `mm3-300/… -2d.png` | `59252059ecdd5ca6e9dd4a0d26a04cbf5d7bdd28f7667299c1f90b0dd39af392` |
| `mm3-1200/Mega Man 3 [USA][!!].png` | `7eb64ba997213fdde9a548c7564a0f3b661e37617648967940887da716331ef4` |
| `mm3-1200/… -2d.png` | `ee59e944668493ec0692a64708052a82f629feebc93b5a4fdc22047f2a8d872e` |

**`bench-core` baseline**, each game with its own profile, three runs each, every run alone
(i9-13900K):

| ROM | `core run_frame` ms | `total` ms | frames/s |
|---|---|---|---|
| *Super Mario Bros. 3* | 1.1787 / 1.1722 / 1.1699 | 1.4075 / 1.3977 / 1.3917 | 710 / 715 / 719 |
| *Mega Man 3* | (1.2492) / 0.9214 / 0.9364 | (1.3566) / 1.0232 / 1.0420 | (737) / 977 / 960 |

*Mega Man 3*'s first run is in brackets because it is a cold one — 1 135 of its 1 200 frames over
1 ms against 199 and 225 on the two after it, which is the machine and not the code. The figures
step 4's +1 % is measured against are the **medians of the warm runs**: SMB3 **1.3977 ms** total
and **1.1722 ms** of core, *Mega Man 3* **1.0326 ms** total and **0.9289 ms** of core.
