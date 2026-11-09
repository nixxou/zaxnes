# FCEUX-6 — the tile capture finished: records identical to Zaxnes's

Lot FCEUX-6, role R3 (clean side). Branch `feat/fceux-core` in the worktree `wt-fceux`, on top of
`0ee93af0` (the end of lot FCEUX-1); fork of FCEUX in `cleanroom/fceux-zax`, branch `zax`, on top of
`3f022b8c`.

The goal: **tile records identical field by field to `zax-nes`'s**, so that one 3dn profile gives
the same 3D picture whichever engine produced the records. `zax-nes` is the reference and is read,
never edited.

## Steps

- [x] 1/9 — The measuring stick: a harness subcommand that runs both engines over a ROM list and
  frame list and reports, per frame, the record counts, the records identical on all fields, the
  differing fields with the record's row and column, and the **capture / emulation** class. The
  baseline table.
- [ ] 2/9 — Mid-frame scroll: the capture follows the scroll as `zax-nes/src/ppu/tiles.rs` samples
  it.
- [ ] 3/9 — CHR banking at fetch time.
- [ ] 4/9 — MMC2/MMC4: the `$FD`/`$FE` latch walked per column and put back (REQ-CORE-039), and
  the sprite ordering of REQ-CORE-044.
- [ ] 5/9 — MMC5: extended attributes and the vertical split.
- [ ] 6/9 — The rest of the classification.
- [ ] 7/9 — Cost of the capture.
- [ ] 8/9 — The 3D picture, per original with a profile.
- [ ] 9/9 — Final pass.

## Environment of the session

| Item | Value |
|---|---|
| Worktree | `cleanroom/wt-fceux`, branch `feat/fceux-core`, base commit `0ee93af0` |
| Fork | `cleanroom/fceux-zax`, branch `zax`, base commit `3f022b8c` |
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-fx` |
| `TMP` / `TEMP` | `<worktree>/rust/target-fx/tmp` |
| C/C++ build trees | `cleanroom/fceux-zax/build-*` |
| Free space on `G:` at the start | 14 GB (floor: 5 GB) |
| ROM sets | `G:/3DSen/test-originals` (98 originals), `rust/test-roms` (291 public ROMs) |

Nothing was downloaded. The toolchains are those lot FCEUX-1 recorded (MSVC 14.43 and CMake/Ninja
from VS 2022, g++ 13.3 and CMake 3.28.3 in WSL Ubuntu 24.04).

---

# Step 1 — The measuring stick, and the baseline

Worktree commit below. Nothing in the fork changed at this step.

## What was built

`zax-harness fceux-tiles <roms dir|rom.nes> <out.jsonl> [--frames a,b,c] [--profiles-only]
[--in-process] [--detail]` (`crates/zax-harness/src/fceux_tiles.rs`). Both engines run side by
side in one process, driven exactly as `golden-core` and `fceux-run` drive them — the file stem as
Zaxnes's ROM name and the basename as FCEUX's, FCEUX's deterministic default RAM fill, a reset
after loading, no button pressed — so a difference it reports is the engines' and not the
harness's. A directory sweep gives each ROM its own child process with a deadline, for the reason
lot FCEUX-1 §7 found: `other/oam3.nes` corrupts FCEUX's heap and then **hangs** rather than dying.

Per ROM and per frame it reports:

| Column | Meaning |
|---|---|
| `zaxnes`, `fceux` | records on each side |
| `same_in_order` | records equal on **all ten fields** at the same position in both lists — the strictest reading, and the one the renderer sees (`FrameBatch::records` is ordered) |
| `same_as_multiset` | the same, order ignored |
| `fields` | the differing records grouped by *which* fields differ (`plane0+plane1:106`) |
| `rows` | the distinct `y` of the differing records |
| `pixels_identical` | whether the two engines' 256×240 index pictures are identical for that frame |
| `class` | `identical`, `capture` or `emulation` |
| `first_pixel_diff` | the first frame of the whole run whose pictures differ (for step 6's list) |

**The class is the column that makes the rest readable.** A frame whose pictures are identical but
whose records differ is a *capture* bug and ours; a frame whose pictures already differ is FCEUX's
emulation against Zaxnes's, which lot FCEUX-1 step 5 measured and which this lot cannot and must
not try to fix. Mixing the two is why "89 %" was not actionable.

Pairing: positional while the two lists are the same length, which is the normal case. When the
lengths differ the records are grouped by `(background, index)` and zipped inside each group, the
leftovers counted as `unmatched_*` — a deterministic alignment that still names the differing
fields instead of degenerating to a bare count.

## Which originals have a 3dn profile

The application has no automatic ROM → profile mapping: a profile is a per-game choice the user
makes (`zax-app`'s `choose_profile_for_rom`). So the comparison carries its own table
(`fceux_tiles::PROFILES`, matched on a prefix of the dump's file name, with a unit test on the
traps). The profiles are those of **`assets-neutral/profiles.json`, edition `full`**, whose files
are in `assets-neutral/profiles/` (101 of them; `assets-neutral/profiles-2.1/` holds the 2.1
versions and is not used here).

**78 of the 98 originals have a profile, covering 13 distinct profiles:**

| Profile | Dumps in `test-originals` |
|---|---|
| Adventure Island | `Adventure Island`, `Adventure Island Classic`, `Takahashi Meijin no Bouken-j` (the Japanese title), `Kiddy Sun in Fantasia` (an Asian hack of it: CNROM, 2 PRG, 4 CHR, the same shape) |
| Blaster Master | `Blaster Master` ×5, `Chou-WS-M` (*Chou Wakusei Senki: MetaFight*, the Japanese title — MMC1, 8 PRG, 16 CHR, exactly the western dumps') |
| Castlevania | `Castlevania` ×7, `Akumajou Dracula` ×2 |
| Excitebike | `Excitebike` ×6 |
| Final Fantasy | `Final Fantasy` ×7 |
| Mega Man 1 | `Mega Man` ×4, `Rockman` ×2 |
| Mega Man 2 | `Mega Man 2` ×6, `Rockman 2-DWnN` ×2 |
| Mega Man 3 | `Mega Man 3` ×7, `Rockman 3-DWnS` ×2 |
| Metroid | `Metroid` ×4 |
| Ninja Gaiden III | `Ninja Gaiden III-TASoD` ×3, `Ninja Ryuuken Den III-YnH` |
| Super Mario Bros. | `Super Mario Bros.` ×2, `Super Mario Bros.-2A` ×2 |
| Super Mario Bros. 3 | `Super Mario Bros. 3` ×8 |
| Zelda II: The Adventure of Link | `Zelda II - The Adventure of Link` ×2, `Zelda II-TAoL` |

**The 20 originals with no profile** are the *Mega Man 4*, *Mega Man 5* and *Mega Man 6* dumps
(and the `Rockman 4-AY`, `Rockman 5-BnW`, `Rockman 6-SSnT` dumps of the same three games) and
`Metal Force [Korea][Unl]`: the catalogue has no entry for any of them. They are still compared,
for field identity.

## The baseline

`zax-harness fceux-tiles "G:/3DSen/test-originals" base-originals.jsonl --frames 60,300,1200,3000`
(5 min 42 s) and `zax-harness fceux-tiles test-roms base-testroms.jsonl --frames 60,300`
(1 min 51 s).

| | 98 originals, frames 60/300/1200/3000 | 291 public ROMs, frames 60/300 |
|---|---|---|
| ROMs compared | 98 | 289 (1 took the engine down, 1 refused by both) |
| ROMs **identical at every compared frame** | **7 / 98** | **102 / 289** |
| frames `identical` | 85 / 392 | 234 / 578 |
| frames `capture` (ours to fix) | **176 / 392** | **122 / 578** |
| frames `emulation` (FCEUX vs Zaxnes) | 131 / 392 | 222 / 578 |

Of the 78 originals with a profile: **0 of 78** are identical at every frame; 40 of their 312
frames are identical, 141 are capture-class and 131 emulation-class.

Per profile, before any fix:

| Profile | frames compared | identical | capture | emulation |
|---|---|---|---|---|
| Adventure Island | 16 | 1 | 8 | 7 |
| Blaster Master | 24 | 0 | 21 | 3 |
| Castlevania | 36 | 0 | 4 | 32 |
| Excitebike | 24 | 0 | 12 | 12 |
| Final Fantasy | 28 | 0 | 17 | 11 |
| Mega Man 1 | 24 | 0 | 24 | 0 |
| Mega Man 2 | 32 | 10 | 14 | 8 |
| Mega Man 3 | 36 | 15 | 21 | 0 |
| Metroid | 16 | 4 | 0 | 12 |
| Ninja Gaiden III | 16 | 10 | 6 | 0 |
| Super Mario Bros. | 16 | 0 | 14 | 2 |
| Super Mario Bros. 3 | 32 | 0 | 0 | 32 |
| Zelda II | 12 | 0 | 0 | 12 |
| (no profile) | 80 | 45 | 35 | 0 |

## What the fields say the work is

Differing records of the **capture-class** frames only, over both ROM sets, grouped by which
fields differ:

| Fields | Records |
|---|---|
| `plane0+plane1` | 11 858 |
| `plane0` | 2 414 |
| `plane0+plane1+palette` | 1 728 |
| `plane1` | 1 234 |
| `palette` | 1 176 |
| `y` | 583 |
| `index` | 175 |
| `y+index` | 160 |
| everything else | under 100 each |

So the capture-class work is almost entirely **the wrong tile fetched**, not the wrong geometry:
`x` barely appears, and `plane0`/`plane1`/`palette` are what a tile taken from the wrong nametable
column or the wrong CHR bank gets wrong. `y`/`index` differences are rows emitted on the wrong
line, which is the same cause seen from the other side.

The cross-check against lot FCEUX-1: on *Super Mario Bros.* frame 200 this tool reports 969
records on both sides and **863 identical**, which is FCEUX-1 step 6's figure to the record, and
the rows it names — `y` = 16, 24, 88, 160, 232 — are the five that note named. The instrument
agrees with the one it replaces.
