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
- [x] 2/9 — Mid-frame scroll: the capture follows the scroll as `zax-nes/src/ppu/tiles.rs` samples
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

---

# Step 2 — The row's scroll, read at the end of the line

Fork commit `b1141a9d`; worktree commit below.

## What was wrong

Lot FCEUX-1's prototype read `RefreshAddr` at the **start** of the line. That is the row's address
*before* any write made during the line, and AMD-021 R2-005 says the row of line L is the
left-edge shadow of the internal address at the **end** of line L: a `$2006` write made during
line L reaches the row of line L itself, while the vertical increment and the horizontal reload at
the end of the line only reach line L + 1. Every game with a mid-frame scroll change — a status
bar, a parallax split — therefore had the rows below the split read with the scroll of the line
before.

`RefreshAddr` on its own cannot answer the question, because the fetch pipeline's coarse-X
increments, `Fixit1`'s vertical increment and `Fixit2`'s horizontal reload all land in it. So the
fork now keeps **`zax_row_start` / `zax_row_next`** beside it — `zax-nes`'s own two fields under
another name — written from five one-line hooks in `src/ppu.cpp`:

| Where | Writes | Why |
|---|---|---|
| `B2006` second write | both | a CPU write reaches the row of the line it is made in |
| `B2007` | both | FCEUX takes the plain PPUCTRL step here on every line, fetching or not |
| `A2007`, non-fetching branch | both | likewise |
| `A2007`, fetching branch | `zax_row_next` only | the *pipeline's own* vertical increment that a `$2007` access borrows while rendering; R2-005 and Q-137 hold it out of this line's row |
| `Fixit2` | `zax_row_next` only | the horizontal reload belongs to the next line |
| `Fixit1`, `pputile.inc`'s coarse-X | neither | the end-of-line vertical increment, and the pipeline's walk along the line |
| end of `DoLine` | `zax_row_start = zax_row_next` | the line is over |

and the background capture moved from the **first** thing `DoLine` does to the **last**, after the
sixteen dots FCEUX runs once it has already incremented `scanline`. That is the same place
`zax-nes` captures (`end_capture_line` at dot 340, then `v_row_start = v_row_next`), and it keeps
REQ-CORE-044's order: the sprites of a band are still captured at the start of line `8b+7` and so
still come before the background row of that same line.

FCEUX's own emulation is untouched — the pair is two stores at each of six places and is read by
nothing but the capture. `zaxselftest` on `nestest.nes` over 60 frames reports the digests it
reported before the change, to the bit: pixels `e455081e6a3eefe9`, emphasis `d395e456e4e72325`,
audio `7fc10a9ce4ed8fcd` over 47 917 samples.

## What it is worth

*Super Mario Bros.* frame 200 — the brief's target — is now **969 records on both sides, all 969
identical**, and so are frames 60 and 300. (Lot FCEUX-1: 863 of 969.)

| | originals, 60/300/1200/3000 | public ROMs, 60/300 |
|---|---|---|
| ROMs identical at every frame | 7 → **41 / 98** | 102 → **162 / 289** |
| frames `identical` | 85 → **253 / 392** | 234 → **351 / 578** |
| frames `capture` | 176 → **30 / 392** | 122 → **18 / 578** |
| frames `emulation` | 131 → 109 | 222 → 209 |

The 78 originals with a profile go from 0 to **33 of 78** identical at every frame, and from 141
capture-class frames to **12**.

Per profile:

| Profile | frames | identical | capture | emulation |
|---|---|---|---|---|
| Adventure Island | 16 | 1 | 8 | 7 |
| Blaster Master | 24 | **22** | 0 | 2 |
| Castlevania | 36 | 4 | 0 | 32 |
| Excitebike | 24 | 12 | 0 | 12 |
| Final Fantasy | 28 | **28** | 0 | 0 |
| Mega Man 1 | 24 | **24** | 0 | 0 |
| Mega Man 2 | 32 | **26** | 0 | 6 |
| Mega Man 3 | 36 | **36** | 0 | 0 |
| Metroid | 16 | 12 | 0 | 4 |
| Ninja Gaiden III | 16 | **16** | 0 | 0 |
| Super Mario Bros. | 16 | 10 | 4 | 2 |
| Super Mario Bros. 3 | 32 | 0 | 0 | 32 |
| Zelda II | 12 | 0 | 0 | 12 |
| (no profile) | 80 | 62 | 18 | 0 |

Four whole games — *Final Fantasy*, *Mega Man 1*, *Mega Man 3*, *Ninja Gaiden III* — are now
record-identical on every dump at every compared frame, *Ninja Gaiden III* included, which is the
game whose status-bar split changes the nametable mirroring mid-frame.

## The one finding of this step, and it is not a capture bug

*Super Mario Bros. 3* and *Zelda II* are the two games whose every compared frame is still
emulation-class, and the reason is **one gated rule in `zax-nes`**, not anything in the capture.

The measurement, on *Super Mario Bros. 3* frame 60, is the row list the tool prints with
`--detail`:

    zaxnes rows: ... 176x32 184x32 194x32 202x32 210x32 218x32 226x32 234x32
    fceux  rows: ... 176x32 184x32 193x32 201x32 209x32 217x32 225x32 233x32

The banner's rows are one line apart on the two engines, and nothing else differs. SMB3's MMC3
split writes `$2006` at dot 278 of line 193 and then touches `$2007` at dot 290, in the horizontal
blank with rendering still on. FCEUX's `A2007` takes the **fetch pipeline's vertical increment**
there (`ppu.cpp:840-852`), which is the hardware rule; `zax-nes`'s default build takes the plain
PPUCTRL step, because the hardware rule lives behind the feature **`ppu-2007-render-read`** and
that feature is out of `default` (Q-139) — waiting on the owner's eye, precisely because it moves
the picture in *Super Mario Bros. 3* and *Zelda II*.

Built with the feature on, the two engines agree:

| *Super Mario Bros. 3* `[USA][!!]`, frame | `zax-nes` default | `zax-nes` + `ppu-2007-render-read` |
|---|---|---|
| 60 | 960 / 992 records, 768 identical, pictures differ | **960 / 960, 944 identical, pictures identical** |
| 300 | 976 / 1008, 784 | **976 / 976, 960** |
| 1200 | 972 / 1004, 707 | **972 / 972, 956** |
| 3000 | 962 / 994, 701 | **962 / 962, 946** |

The row lists become identical and the frames become capture-class, with 16 records of one row
(`y` = 193, the split row) still differing on `plane0+plane1` — a CHR bank, which is step 3's
business.

**This is worth the owner's attention**, because it is the one place where the two engines cannot
agree without a decision: `zax-nes` has the rule, has measured it at +0.000 % to +0.005 % in cost,
and keeps it out of the default build because of what it does to the picture of these two games.
FCEUX applies it unconditionally. So *3D on FCEUX for Super Mario Bros. 3 and Zelda II needs
`ppu-2007-render-read` turned on in `zax-nes`* — or the difference accepted. Nothing in this lot
can decide that, and **no change was made to `zax-nes`** (the step-2 target was the capture, and
parity here is not the capture's to win).

The count difference of 32 that the default build also showed (one spurious row at `y` = 192) was
ours and **is fixed**: it came from letting the borrowed pipeline increment reach `zax_row_start`,
which the `A2007` fetching-branch rule above now prevents.
