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
- [x] 3/9 — CHR banking at fetch time.
- [x] 4/9 — MMC2/MMC4: the `$FD`/`$FE` latch walked per column and put back (REQ-CORE-039), and
  the sprite ordering of REQ-CORE-044.
- [x] 5/9 — MMC5: extended attributes and the vertical split.
- [x] 6/9 — The rest of the classification.
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
Zaxnes's ROM name and the basename as FCEUX's, a deterministic RAM fill, a reset after loading, no
button pressed — so a difference it reports is the engines' and not the harness's. (Which fill is
the comparable one turned out to matter, and step 3 below settles it: `--ram zero`, because
`zax-nes` clears the work RAM *and* the CHR-RAM. The step-1 baseline below was taken with lot
FCEUX-1's `Default` fill; step 3 gives both figures.) A directory sweep gives each ROM its own child process with a deadline, for the reason
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

---

# Step 3 — CHR banking at fetch time

Fork commit `9c48ca28`; worktree commit below.

## The banks are right, and step 2 is why

A record's pattern planes must be the CHR bytes with **the bank mapping in force when the tile was
fetched**. After step 2 that holds by construction, and the reason is worth stating because it is
not obvious:

* the background row of line L is read at the **end of line L**, through `VPage`, which is the same
  1 KiB-granular pointer table the PPU itself fetches through. `zax-nes` reads its row at the same
  instant, through the inert `ppu_peek` path. Whatever a board switched during line L — an MMC3
  status-bar split, an MMC1 CHR bank, a `$2007` write into CHR-RAM — is in force on both sides;
* the sprites of band `b` are read at the **start of line `8b+7`**, which is where
  `begin_capture_line` reads them;
* neither side can be disturbed by the reading: `VPage` is a plain pointer table (no A12 rise, no
  MMC2/MMC4 latch, no `$2007` buffer), which is the same property `ppu_peek` has by design.

A tile crossing a bank switch **inside** a line is still read with the bank the end of the line
had, on both engines, so the two agree there as well — the approximation is the old PPU's and it is
the same approximation `zax-nes` makes.

The measurement, over both ROM sets at frames 60/300/1200/3000 and 60/300:

| | mapper | ROMs | frames | identical | capture | emulation |
|---|---|---|---|---|---|---|
| originals | 1 (MMC1) | 27 | 108 | 88 | **0** | 20 |
| originals | 2 (UxROM) | 16 | 64 | 32 | **0** | 32 |
| originals | 4 (MMC3) | 40 | 160 | 112 | 16 | 32 |
| originals | 3 (CNROM) | 4 | 16 | 1 | 8 | 7 |
| originals | 156 | 1 | 4 | 4 | **0** | 0 |
| public | 1, 2, 11, 22, 28, 34 | 30 | 60 | 54 | **0** | 6 |
| public | 4 (MMC3) | 24 | 48 | 8 | 1 | 39 |
| public | 5 (MMC5) | 4 | 8 | 0 | 2 | 6 |

**Not one** capture-class difference on any board now comes from a background tile's planes. Every
mapper-4 and mapper-3 capture-class frame left is a sprite `index`, a sprite `x`, or a frame whose
screen is blank — all three accounted for below — and the two MMC5 frames are step 5's.

The bank-switching and CHR-RAM games verify it directly. **Of the 98 originals, 47 are CHR-RAM
carts** (`chr = 0` in the header), and of those:

| CHR-RAM game | compared frames | result |
|---|---|---|
| *Final Fantasy* (7 dumps) | 28 | **all identical** |
| *Mega Man 1* (6 dumps) | 24 | **all identical** |
| *Mega Man 2* (6 dumps) | 24 | **all identical** |
| *Mega Man 6* (2 dumps) | 8 | **all identical** |
| *Metroid* (4), *Rockman 2* (2) | 24 | 18 identical, 6 emulation |
| *Mega Man 4* (6 dumps) | 24 | 12 identical, 12 capture — **all on a blank screen**, below |
| *Rockman 4* (3), *Rockman 6* (1) | 16 | 12 identical, 4 sprite `index` only |
| *Castlevania* / *Akumajou Dracula* (9) | 36 | 4 identical, 32 emulation (`first_pixel_diff` 2 to 4) |

and the bank-switching games that are not CHR-RAM: *Mega Man 3* (9 dumps, MMC3, 36 frames) and
*Ninja Gaiden III* (4 dumps, MMC3 with a mid-frame mirroring switch, 16 frames) are **identical on
every frame**, *Blaster Master* (6 dumps, MMC1) 22 of 24.

## The finding that moved the numbers: what power-on writes into memory

The comparison was driving FCEUX with its **`Default` RAM fill**, which is what lot FCEUX-1 chose
for the digest comparison: FCEUX's own `00 00 00 00 FF FF FF FF` pattern. That is the wrong
setting for a record comparison, and the reason is `FCEU_MemoryRand`
(`src/fceu.cpp:990-1022`): it fills the **work RAM and the CHR-RAM with the same pattern**
(`src/ines.cpp:78-79` calls it on the CHR-RAM of every `chr = 0` cart, with `default_zero` left
false). `zax-nes` clears both — `NesBus::ram` is `[0; RAM_SIZE]` and `CartridgeMemory`'s
`chr_ram` is `vec![0; …]` (REQ-CORE-004).

So the two machines were starting from different memory, and a CHR-RAM cart that reads a tile it
never wrote read a different tile on each engine. *Mega Man 6* showed it exactly: 64 sprite records
whose planes were `0000000000000000` on Zaxnes and `ffffffff00000000` on FCEUX — the pattern,
read straight out of unwritten CHR-RAM.

`fceux-tiles` now takes `--ram zero|default|ff` and **defaults to `zero`**, which is the setting
that compares like with like. What it is worth:

| | `--ram default` | `--ram zero` |
|---|---|---|
| originals identical at every frame | 41 / 98 | **47 / 98** |
| public ROMs identical at every frame | 162 / 289 | **177 / 289** |
| capture-class frames, originals | 30 | **28** |
| capture-class frames, public ROMs | 18 | **6** |

`fceux-run` and `docs/review/FCEUX-ENGINE-COMPARISON.md` are **not** changed: their figures are
lot FCEUX-1's and re-running them was not in this brief. But the same reasoning applies to them,
and a lot that revisits the engine comparison should expect the pixel-identity figures to improve
for the same reason. Recorded here rather than acted on.

## The capture bug this step found: no row on line 0 under vertical mirroring

`other/high-hopes.nes` at frame 300 was emitting **one row more on FCEUX than on Zaxnes**, at
`y = -1` (line 0), 32 records of a row the game does not show. `zax-nes` has an explicit exception
there — REQ-CORE-037's justification, `tiles.rs:227`: *no row is captured on line 0 under vertical
mirroring* — and the capture did not.

It is now in `zax/zaxtiles.cpp`. FCEUX keeps no mirroring *mode* a board cannot bypass
(`setmirrorw` and `SetupCartMirroring`'s four-screen arm write `vnapage` directly and never touch
`setmirror`'s switch), so the layout is read back from the four `vnapage` pointers themselves —
vertical is `vnapage[0] == vnapage[2] && vnapage[1] == vnapage[3] && vnapage[0] != vnapage[1]` —
which is true whichever way a board set them. Frame 300 of `high-hopes` became identical.

## Two discriminators the measuring stick gained, and what they settle

Both exist because the same shapes of difference kept appearing and neither is the capture's:

**1. `[OAM rotated by k]`.** Several ROMs showed pairs of sprite records carrying the *same* planes,
position, palette and flags on both engines but a different `index`, always by a constant. Both
engines walk OAM entries 0 to 63 and number the record after the entry (REQ-CORE-043), so a
capture cannot get that wrong: it means the two OAM arrays hold the same sprites at different
offsets, which is what happens when the `$4014` transfer starts at a different `$2003`. The tool
now searches for the `k` in 1..64 that turns **every** one of FCEUX's sprite records into one of
Zaxnes's and names it. It finds one on *Adventure Island* (4 frames), *Takahashi Meijin no
Bouken-jima* (2), *Kiddy Sun in Fantasia* (2) — all `k = 32`, a 128-byte rotation — and on
`spritecans-2011` (`k = 15`). Those eight frames are emulation differences wearing a capture
costume.

**2. `[blank screen]`.** When Zaxnes's picture for the frame is a **single colour**, two identical
pictures prove almost nothing: a game that blanks the screen with `$2001` and a game that draws a
screenful of blank tiles look exactly the same, so the pixel test cannot tell a capture difference
from an emulation one. The tool now says so. It marks eleven of the remaining capture-class
frames: the six *Mega Man 4* dumps at frames 60 and 3000,
`nes_instr_test/rom_singles/01-implied.nes` at 60 — where FCEUX captured **no records at all**
while Zaxnes captured thirty rows, which is exactly what one engine having `$2001` bit 3 clear for
that frame looks like — and `other/high-hopes.nes` at 60.

## What is left, after this step

28 capture-class frames on the originals and 5 on the public ROMs, and **every one of them is
accounted for**:

| Cause | Frames | Ours? |
|---|---|---|
| OAM rotated (`$2003` at the `$4014` transfer) | 8 originals + 1 public | no — emulation |
| sprite `index` only, no whole-array rotation (the game put the sprite in another OAM slot) | 8 originals | no — emulation |
| blank screen, the pixel test cannot decide | 12 originals + 2 public | undecidable |
| MMC5 sprite planes (`mmc5test`) | 2 public | **yes — step 5** |

The two `mmc5test` frames are the only remaining capture-class differences anywhere whose cause is
a mapper feature, and they are step 5's. Steps 4, 5 and 6 pick them up from here.

---

# Step 4 — MMC2 and MMC4

Fork commit `f288a60f`; worktree commit below.

## What REQ-CORE-039 and REQ-CORE-044 ask for, and what was done

On MMC2 and MMC4 the pattern a tile shows depends on the tiles fetched **before it on the line**:
fetching tile `$FD` or `$FE` from either pattern table flips that table's latch and remaps 4 KiB of
CHR. A capture that reads sixteen whole tiles out of order therefore reads some of them through
the wrong bank unless it walks the line in display order and moves the latch as it goes — and it
must then put the real latch back, because the next line's first fetches would otherwise see a
latch the capture left behind (REQ-CORE-033).

Three things now do that, mirroring `zax-nes/src/ppu/tiles.rs` call for call:

1. **The background row** is walked left to right (it already was) with
   `zax_mmc2_advance_latch(pattern_table, tile)` after each tile's planes are read, and the latch
   saved before the loop and restored after it.
2. **The sprites of a capture are emitted by increasing x** on those two boards, OAM number
   breaking ties — because "display order" for sprites is x order, not OAM order. A **stable
   insertion sort**, which is what `Ppu2C02::sort_sprites_by_x` is and for the same reason: equal
   x values must keep OAM order or the output is not reproducible (REQ-ARCH-012). The sort only
   runs when an MMC2/MMC4 cart is live.
3. **`emit_sprite`** advances the latch after each half, as Zaxnes's does.

`latch0` and `latch1` are file-static in `src/boards/mmc2and4.cpp`, so the three accessors live
there: read the pair, put it back, and move it on as one pattern fetch would (which is the board's
own `MMC2and4PPUHook`). All three answer "not this board" unless `PPU_hook` is that hook —
`ResetGameLoaded` clears `PPU_hook` before every load (`src/fceu.cpp:401`) and each board's `Init`
installs its own, so the test is exact — which means every other board pays one load and one
comparison per captured tile and nothing else.

## What could be verified, and what could not

**There is no mapper 9 or 10 game in either ROM set.** The 98 originals are mappers 0, 1, 2, 3, 4
and 156; the 291 public ROMs are 0, 1, 2, 3, 4, 5, 11, 22, 28 and 34. The MMC2/MMC4 games are
*Punch-Out!!*, *Mike Tyson's Punch-Out!!* and *Fire Emblem*, and none of them is here. The
catalogue does have a `Punch Out!!` profile, so this is a gap a later lot should close with the
ROM in hand.

What **is** here is `assets-free/test-roms/holy-mapperel/testroms/`:

| ROM | Board | frames 60 / 300 / 1200 |
|---|---|---|
| `M10_P128K_C64K_S8K.nes` | MMC4, battery WRAM | **identical on all three** |
| `M10_P128K_C64K_W8K.nes` | MMC4, plain WRAM | **identical on all three** |
| `M9_P128K_C64K.nes` | MMC2 | 946 of 960 records identical at 300 and 1200; the frames are **emulation**-class (the pictures differ) and the 14 differing records are one row of text, `y` = 48 — the ROM's own printed verdict, which the two engines disagree about. Frame 60 is a blank screen on Zaxnes and a drawn one on FCEUX |

And the latch walk was measured **both ways**, by building the fork once with the advance and the
sort removed: on these three ROMs the figures are **identical with and without them**. So the walk
is implemented to the requirement and provably breaks nothing, but *no ROM available in this
session exercises it* — these test ROMs do not put `$FD`/`$FE` tiles where the latch matters
mid-row. That is stated rather than dressed up.

## The test that does prove the restore: REQ-CORE-033

`the_tile_capture_does_not_change_what_fceux_draws`
(`crates/zax-fceux/tests/fceux_core_tests.rs`, **not** `#[ignore]`d) runs each ROM 180 frames with
the capture **off**, hashes every frame, runs it again with the capture **on**, and requires the
two digests to be equal. That is the guard for every save-and-restore the capture does — the
MMC2/MMC4 latch here, and the MMC5's per-tile state in step 5 — and without it a wrong restore
would silently drift the picture of a mapper 9, 10 or 5 game the moment a host asked for records,
with nothing else in the suite noticing.

It passes on all six ROMs it can find:

    nestest.nes:              capture on and off agree, pixels 1df943b3a46bc069
    M9_P128K_C64K.nes:        capture on and off agree, pixels 221428505e199ec5
    M10_P128K_C64K_S8K.nes:   capture on and off agree, pixels 6ed207f1e2455ad5
    M10_P128K_C64K_W8K.nes:   capture on and off agree, pixels 6ed207f1e2455ad5
    mmc5test.nes:             capture on and off agree, pixels 067f912e427b8d64
    mmc5exram.nes:            capture on and off agree, pixels 52d3f0fe1eb4a004

## An upstream FCEUX finding the new test turned up

Writing that test broke `sixty_frames_of_nestest_match_the_c_self_tests_digests` — but only when
the suite ran in parallel, and only on the **audio** digest, with the pixel and emphasis digests
and the sample count all still exact. The cause is `SexyFilter` (`src/filter.cpp:51-53`), the
output DC blocker: its two accumulators are `static int64` locals and **nothing ever resets them**
— not `FCEUSND_Power`, not `MakeFilters`, not loading another game. A host that plays game A and
then loads game B gets the tail of A's filter in B's first samples.

It costs nothing here (the record comparison does not look at audio) and it is **upstream
FCEUX's**, so nothing was patched for it. The test now runs with the sample rate at 0, where
`FlushEmulateSound` returns before the filter is reached (`src/sound.cpp:1043-1048`), so it cannot
perturb anything; `sixty_frames` is green four runs out of four. **A lot that does audio on the
FCEUX engine should fix this**, and the fix is one `memset` in `MakeFilters` or a reset call on
the C API — recorded here for whoever gets there.

---

# Step 5 — The MMC5

Fork commit `c9381dac`; worktree commit below. Only `zax/zaxtiles.cpp` changed in the fork — no
`src/` file is touched, because FCEUX already keeps the chip's whole effect on the PPU's fetches
in the `MMC5Hack*` globals of `src/ppu.cpp`.

## The one thing that was actually wrong: sprites read the background's banks

The MMC5 has **two CHR bank sets**, one for the background and one for the sprites, and
`FetchSpriteData` reads through `MMC5SPRVPage` whenever `MMC5Hack` is on (`src/ppu.cpp:1529-1532`
and `:1582`). The capture read every pattern — background and sprite alike — through `VPage`, so
on an MMC5 every sprite record carried bytes from the *background* banks.

That is step 3's last open item ("MMC5 sprite planes (`mmc5test`), 2 public frames, **yes — step
5**"), and it is now closed. Measured by building the capture both ways over the same frames:

| ROM | before step 5 | after step 5 |
|---|---|---|
| `mmc5test_v2/mmc5test.nes` | **capture**-class at 60, 300 and 1200: 963 of 976 records identical, `spr.plane0+plane1:13` | **identical on all three**, 976 / 976 |
| `exram/mmc5exram.nes` | emulation: 833 / 865 / 897 of 961 | unchanged |
| `mmc5test/mmc5test.nes` (v1) | emulation: 640 of 960, `bg.y:320` | unchanged |
| `other/firefly.nes` | emulation: 928 / 960 / 960 of 968 | unchanged |

Thirteen sprite records on three frames is a small number, and it is the whole of what any ROM
available here could show: see below.

## What else the step put in, and the two places FCEUX and `zax-nes` disagree

**Background patterns now go through `FCEUPPU_GetCHR`** (`src/ppu.cpp:419-429`), which is FCEUX's
own answer to "where does CHR come from under the hack modes": `ExRAM` mode 1's per-tile 4 KiB
bank, the MMC5's background bank set otherwise, plain `VPage` when no MMC5 is live. One call
covers every board, so there is no second copy of `pputile.inc`'s banking to keep in step — and
for a non-MMC5 cart it is `&VPage[vadr >> 10][vadr]`, the expression the capture already used,
which is why the 98 originals did not move by a single record.

**Mode 1's extended attributes and the vertical split** are `pputile.inc`'s formulas where they
cannot be called, gated exactly as `RefreshLine` gates them — and three corrections were needed
against the first draft, each of which would have put records on the screen that FCEUX does not
draw:

1. **No split outside CHR modes 0 and 1.** `RefreshLine` reaches a split branch only from
   `MMC5HackCHRMode == 0` or `== 1`; in modes 2 and 3 the plain loop draws the line and `$5200`
   bit 7 moves no pixel. `zax-nes` gates on the same `ex_ram_mode <= 1`
   (`Mmc5Ext::background_substitutes`).
2. **The split's row is taken on the band's top line,** `y = line - 1`, and not on `line`. The
   eight rows of a captured band are drawn on scanlines `y .. y+7` while FCEUX's split row changes
   at every multiple of 8, so a band generally spans two split rows and one tile has to stand for
   the whole band. `zax-nes` makes the same choice — `position_captured_background_tile` is handed
   `y` — so the two agree on which row that is. The two expressions differ only when `line` is a
   multiple of 8, which is why this is a correction and not a rewrite.
3. **The attribute override is gated on `geniestage != 1`** like the renderer's own block, not on
   `MMC5Hack` alone.

Two disagreements are left that are **not** the capture's to settle, and both are resolved in
**FCEUX's** favour, so that the records keep describing the picture FCEUX draws (REQ-CORE-033)
instead of a third picture neither engine shows:

| | FCEUX | `zax-nes` |
|---|---|---|
| in CHR mode 1 with the split armed, the split covers | the **whole line**: `RefreshLine` computes `tochange` and then never reads it, `PPUT_MMC5SP` being defined *outside* the loop (`src/ppu.cpp:1214-1225`) | the `$5200` region only (`Mmc5Ext::split_covers`) |
| `$5201`, the split's vertical scroll | whole tiles — `MMC5HackSPScroll = ($5201 >> 3) & 0x1F` (`src/boards/mmc5.cpp:569`), so the fine part is dropped | pixels, the hardware's own unit: `(line + split_scroll).rem_euclid(240)` |

A game that uses either feature is therefore an **emulation** difference between the two engines
and not a capture one — the same shape of finding as step 2's `ppu-2007-render-read`, and like it,
recorded rather than papered over. The alternative (capture the split as `zax-nes` computes it)
would make the 3D view on FCEUX disagree with FCEUX's own 2D picture, which is worse and would
break the one invariant REQ-CORE-033 exists to protect.

Nothing needs saving and restoring for the MMC5, unlike the MMC2/MMC4 latch of step 4: FCEUX keeps
the split's row and column in the capture's own locals rather than in board state, so the capture
leaves nothing behind. `the_tile_capture_does_not_change_what_fceux_draws` checks that anyway, on
`mmc5test.nes` and `mmc5exram.nes` among its six ROMs, and still passes.

## No ROM here exercises either mode — measured, not assumed

The brief asks to "verify on what MMC5 ROMs you have; if none have a profile, verify field
identity anyway". **None of the four has a profile**, and field identity is the table above. But
the more useful answer is *which code those four ROMs reach*, and it was measured with a temporary
probe in the capture (built, run, removed — not committed) that printed the first time each gate
became true over **1 200 frames** of each ROM:

| ROM | `$5104` CHR mode | `$5200` | mode-1 attributes reached | split reached |
|---|---|---|---|---|
| `exram/mmc5exram.nes` | 2 | `00` | **never** | **never** |
| `mmc5test/mmc5test.nes` (v1) | 3 | `00` | **never** | **never** |
| `mmc5test_v2/mmc5test.nes` | 0 | `00` | **never** | **never** |
| `other/firefly.nes` | 0 | `00` | **never** | **never** |

So **mode 1 and the vertical split are implemented to the requirement, measured to break nothing,
and unproven in use** — exactly the position step 4 reached on the MMC2/MMC4 latch walk, and for
the same reason: the ROMs that would exercise them are not here. `mmc5exram.nes` is named for
`ExRAM` but exercises mode **2** (`ExRAM` as a nametable), not mode 1. A later lot with
*Castlevania III*, *Just Breed*, *Laser Invasion* or *Uchuu Keibitai SDF* in hand should close
this.

It also explains why the ex-attribute and split code moved no number: `mmc5exram.nes` is identical
to the record before and after the step. **The sprite bank set is the entire measurable content of
step 5.**

## The mapper-5 differences that are left, all emulation-class

| ROM | frames | records identical | fields | first frame whose pixels differ | cause |
|---|---|---|---|---|---|
| `mmc5test/mmc5test.nes` (v1) | 60, 300, 1200 | 640 / 960 | `bg.y:320` | 4 | the ten bottom rows land one line apart (Zaxnes 159, 167, …; FCEUX 160, 168, …): the MMC5's scanline detection fires a line later on FCEUX, so the `$2006` write it triggers lands a line later. Checked **not** to be step 2's `ppu-2007-render-read` — with that feature on, the figures are unchanged |
| `exram/mmc5exram.nes` | 60, 300, 1200 | 833 / 865 / 897 of 961 | `bg.plane1`, `bg.plane0+palette`, `bg.plane0+plane1+palette` | 5 | `ExRAM` **mode 2** nametable and CHR-window emulation, neither of them the capture's: the capture's own two modes are provably never entered (above) |
| `other/firefly.nes` | 60, 300, 1200 | 928 / 960 / 960 of 968 | sprite and background `x`, `y`, `index`, planes | 3 | FCEUX against Zaxnes from frame 3 |

## The state of the two ROM sets after this step

| | 98 originals, 60/300/1200/3000 | 291 public ROMs, 60/300 |
|---|---|---|
| ROMs identical at every compared frame | 47 / 98 (step 3: 47) | **180 / 291** (step 3: 177) |
| frames `identical` | 259 / 392 (step 3: 259) | **384 / 578** |
| frames `capture` | 28 / 392 (step 3: 28) | 6 → **3 / 578** |
| frames `emulation` | 105 / 392 | 191 / 578 |

The originals are unmoved to the record, which is the check that the `read_planes` refactor cost
nothing on the 97 non-MMC5 boards among them. On the public set the two MMC5 capture-class frames
are gone and **three capture-class frames remain in the whole 291-ROM set**:

| ROM | frame | shape | step 3's verdict |
|---|---|---|---|
| `nes_instr_test/rom_singles/01-implied.nes` | 60 | FCEUX captures 0 records, Zaxnes 960 | blank screen — the pixel test cannot decide |
| `other/high-hopes.nes` | 60 | `bg.y+index:160` | blank screen — the pixel test cannot decide |
| `spritecans-2011/spritecans.nes` | 60 | `spr.index:2` | the game put the sprite in another OAM slot — emulation |

Step 6 takes those three and the 28 on the originals and closes the classification.

## Verification of this step

* the fork builds clean with the documented `zax\build-msvc.cmd`;
* `cargo test --release -p zax-fceux --features fceux-core`: **14 passed, 1 ignored**, including
  `the_tile_capture_does_not_change_what_fceux_draws` (capture on and off hash the same 180 frames
  on `nestest`, `M9`, both `M10`s, `mmc5test` and `mmc5exram`) and
  `sixty_frames_of_nestest_match_the_c_self_tests_digests`;
* no Rust source changed at this step, so there is nothing new to `fmt` or `clippy`, and the
  default build (the feature off) is untouched by construction.

---

# Step 6 — The rest of the classification

Worktree commit below. Nothing in the fork changed at this step: **no capture bug was left to
fix.** What the step produced instead is the proof of that, in the instrument rather than in prose.

## `unexplained: 0`

Steps 2 to 5 took the capture-class frames from 176 + 122 to 28 + 3. Writing "and the 31 that are
left are all fine, honestly" into a note is worth very little, because nobody can re-run a note.
So `FrameDiff::cause` now puts the reasoning in the tool: for every capture-class frame it names
the discriminator that accounts for it, or says **`unexplained`** — and `fceux-tiles` prints the
tally with the unexplained count on its own last line. That number is the lot's actual acceptance
criterion, and it is **0** on both ROM sets.

The `capture` class is decided by one question — *do the two engines' pictures agree for this
frame?* — and that question has three known blind spots. The cause names which one applies:

| Cause | What it means | Why the capture cannot be at fault |
|---|---|---|
| `oam-rotated-by-k` | every sprite record matches after turning FCEUX's OAM number `n` into `(n + k) mod 64` | the two OAM arrays are rotations of each other, so the `$4014` transfer started at a different `$2003`. Both engines number a record after its OAM entry (REQ-CORE-043), so a capture cannot produce an offset — and cannot remove one |
| `sprite-oam-slot` | every differing record is a **sprite** whose differing fields include `index`, with no single `k` rotating the whole array | the game put that one sprite in another OAM entry. Once the slots differ the positional pairing is comparing two different sprites, so the `x` and the planes differing as well is the consequence, not a second defect |
| `blank-screen` | Zaxnes's picture for the frame is a **single colour** | "the pictures agree" then carries almost no information: a screen blanked with `$2001` and a screenful of blank tiles look exactly the same. The pixel test cannot decide either way |
| `unexplained` | none of the above | **a capture bug.** Must be 0 |

and every cause carries `, diverged at N` when the two pictures had **already** differed at an
earlier frame — `first_pixel_diff`, which step 1 recorded for exactly this. That is what turns
each of the three readings from a plausible story into a decided one: if the machines parted
company at frame 8, their agreeing again at frame 3000 is a coincidence, not evidence.

`the_cause_names_why_a_capture_class_frame_is_not_the_captures_fault` pins all four verdicts,
including the one that matters: a **background** record differing on a plane with an informative
picture comes out `unexplained`, so the discriminators cannot quietly excuse a real bug. (It also
records that a frame holding a single sprite is trivially a rotation of itself, which is why the
`sprite-oam-slot` case has to clear `oam_rotation` by hand to reach that branch.)

## The 31 capture-class frames, each with its cause

**98 originals, frames 60/300/1200/3000 — 28 frames:**

| Cause | Frames | ROMs |
|---|---|---|
| `blank-screen, diverged at 8` | 12 | the six *Mega Man 4* dumps at frames 60 and 3000 |
| `oam-rotated-by-32, diverged at 7` | 8 | *Adventure Island* ×2, *Adventure Island Classic* ×2, *Takahashi Meijin no Bouken-jima* ×2, *Kiddy Sun in Fantasia* ×2 |
| `sprite-oam-slot, diverged at 10` | 4 | *Super Mario Bros.* at 1200 and 3000, two dumps each |
| `sprite-oam-slot, diverged at 8` | 3 | the three *Rockman 4-AY* dumps at 3000 (`spr.index:52`) |
| `sprite-oam-slot, diverged at 5` | 1 | *Rockman 6-SSnT* at 3000 (`spr.index:5`) |
| **`unexplained`** | **0** | |

**291 public ROMs, frames 60/300 — 3 frames:**

| ROM | frame | cause | detail |
|---|---|---|---|
| `nes_instr_test/rom_singles/01-implied.nes` | 60 | `blank-screen, diverged at 4` | Zaxnes captures 960 records and FCEUX **none at all** — which is what one engine having `$2001` bit 3 clear for that frame looks like, on a frame whose picture is one colour either way |
| `other/high-hopes.nes` | 60 | `blank-screen, diverged at 5` | the **top five** rows are one line apart (Zaxnes 7, 15, 23, 31, 39; FCEUX 6, 14, 22, 30, 38) and the other twelve are exact. A one-line disagreement about the scroll at the start of a frame, on a blank screen, 55 frames after the pictures parted |
| `spritecans-2011/spritecans.nes` | 60 | `oam-rotated-by-15, diverged at 3` | `spr.index:2` |
| **`unexplained`** | | **0** | |

## Why none of these can be fixed, as opposed to merely being excused

The three readings are not "we could not work out what this is". Each says the difference is in
**the two machines' state**, which the capture reads and does not create:

* an OAM rotation is the content of a 256-byte array at a different offset. A capture that
  "corrected" it would have to renumber records against REQ-CORE-043 and would then be wrong on
  every game that does not have the offset;
* a sprite in another OAM slot is the same, one entry at a time;
* a uniform picture is not a capture question at all. It is the *measurement* that fails there,
  not the capture, and the only honest fix is a better discriminator — which is what
  `diverged at N` is: all fourteen `blank-screen` frames diverged at frame 4, 5 or 8, long before
  the frames compared.

The one shape among them that **would** have been the capture's is `other/high-hopes.nes`'s
one-line row offset — the signature steps 2 and 3 fixed elsewhere. It is not fixable here and the
reason is specific: the five rows that differ are the only five above the screen's split, the
twelve below are already exact, the screen is blank, and the pictures diverged 55 frames earlier.
There is no residual to chase, and a change that moved those five rows would move the twelve that
are right.

## The emulation-class differences, per ROM, with the first differing frame

Not fixed — these are FCEUX's emulation against Zaxnes's, which lot FCEUX-1 step 5 measured and
which this lot must not touch. **39 of the 98 originals** have at least one emulation-class frame,
and every one of them has a profile; the 20 originals without a profile have **no**
emulation-class frame at all.

| Profile | Dumps | First frame whose pictures differ | Emulation-class frames |
|---|---|---|---|
| Adventure Island | 4 | 7 | 60 and 3000 (3000 only on *Kiddy Sun*) |
| Blaster Master | 1 of 6 (`[Europe][!]`) | 10 | 1200, 3000 |
| Castlevania | 7 *Castlevania* | 13 | all four |
| Castlevania | 2 *Akumajou Dracula* | 6 | 1200, 3000 |
| Excitebike | 6 | 8 | 1200, 3000 |
| Mega Man 2 | 2 of 8 (`Rockman 2-DWnN`) | 8 | 3000 |
| Metroid | 4 | 18 | 3000 |
| Super Mario Bros. | 2 of 4 | 10 | 1200 |
| Super Mario Bros. 3 | 8 | 12 or 13 | all four |
| Zelda II | 3 | 9 or 11 | all four |

Public ROMs: 191 of 578 compared frames are emulation-class, and `fceux-tiles` prints the first
differing frame of every ROM that is not identical, so the full list is the tool's output rather
than a table transcribed here.

**The two games whose every compared frame is emulation-class — *Super Mario Bros. 3* and *Zelda
II* — are the step-2 finding and nothing else.** Both need `ppu-2007-render-read` turned on in
`zax-nes` for the two engines to agree, and step 2 measured that with the feature on *Super Mario
Bros. 3* becomes capture-class and then record-identical. That is still the one decision in this
lot that needs the owner, and **no change was made to `zax-nes`**.

## The final per-profile table

98 originals, frames 60/300/1200/3000, after steps 2 to 5:

| Profile | Dumps | dumps identical at every frame | frames | identical | capture | emulation |
|---|---|---|---|---|---|---|
| Adventure Island | 4 | 0 | 16 | 1 | 8 | 7 |
| Blaster Master | 6 | **5** | 24 | 22 | 0 | 2 |
| Castlevania | 9 | 0 | 36 | 4 | 0 | 32 |
| Excitebike | 6 | 0 | 24 | 12 | 0 | 12 |
| Final Fantasy | 7 | **7** | 28 | **28** | 0 | 0 |
| Mega Man 1 | 6 | **6** | 24 | **24** | 0 | 0 |
| Mega Man 2 | 8 | **6** | 32 | 30 | 0 | 2 |
| Mega Man 3 | 9 | **9** | 36 | **36** | 0 | 0 |
| Metroid | 4 | 0 | 16 | 12 | 0 | 4 |
| Ninja Gaiden III | 4 | **4** | 16 | **16** | 0 | 0 |
| Super Mario Bros. | 4 | 0 | 16 | 10 | 4 | 2 |
| Super Mario Bros. 3 | 8 | 0 | 32 | 0 | 0 | 32 |
| Zelda II | 3 | 0 | 12 | 0 | 0 | 12 |
| (no profile) | 20 | **10** | 80 | 64 | 16 | 0 |
| **total** | **98** | **47** | **392** | **259** | **28** | **105** |

Four games — *Final Fantasy*, *Mega Man 1*, *Mega Man 3*, *Ninja Gaiden III* — are
record-identical on every dump at every compared frame, and *Blaster Master*, *Mega Man 2* and the
no-profile set are identical bar their emulation-class or blank-screen frames. Step 8 turns this
into the 3D answer per game.

## The new PPU, and what its hooks would take

`newppu` (`src/ppu.cpp:2066` onwards, FCEUX's cycle-accurate PPU, off by default and selected by
`FCEUX_PPU_Loop`) is out of this lot's scope and the capture does nothing there. Everything
needed to scope it was read, and it is four items, in increasing order of cost:

1. **The two capture moments exist as positions.** `FCEUX_PPU_Loop` has an explicit
   `for (int sl = 0; sl < normalscanlines; sl++)` with the 32-tile fetch loop inside it
   (`src/ppu.cpp:2252-2285`), so the sprite capture goes at the top of the `sl` body and the
   background row after the `xt` loop — the same two places `DoLine` has them. **No new call site
   problem.**
2. **The row address has to be assembled, not shadowed.** `newppu` keeps the scroll as *five
   separate counters* — `ppur.fv`, `v`, `h`, `vt`, `ht`, plus `fh` (`src/ppu.cpp:167-187`) — and
   not as a packed 15-bit `RefreshAddr`. So step 2's `zax_row_start` / `zax_row_next` pair becomes
   a packed value built from the five at the right instants. The upside is that this is probably
   *fewer* hooks than the old PPU's five, because `newppu` keeps the latch install explicit
   (`ppur.install_latches()`) instead of folding it into `Fixit1` and `Fixit2`.
3. **The pattern read transfers unchanged.** `bgdata.main[xt].pt` holds only the **two** pattern
   bytes of the line being drawn, which is `pputile.inc`'s first problem over again, so the
   capture still has to read all sixteen bytes itself by walking the nametable. That is what
   `zax/zaxtiles.cpp` already does and it is PPU-agnostic.
4. **The MMC5 part has nothing to mirror, because `newppu` does not draw it.** After
   `src/ppu.cpp:2060` the only MMC5 references in the whole new PPU are `MMC5_hb(yp)` and
   `MMC5_hb(240)` — the scanline hook for the IRQ. **`newppu` implements neither the extended
   attribute mode nor the vertical split**: it renders an MMC5 game through the ordinary path with
   the board's CHR windows. So a capture there would be right for free on CHR mode 0 and wrong in
   exactly the way `newppu`'s own picture is wrong on mode 1 and the split — which, by the rule
   step 5 settled, is the correct behaviour for a capture, and an emulation difference to record.

Steps 2's row rule and 4's MMC2/MMC4 latch walk transfer as they stand; step 3's "read through the
board's own pointer table" needs `newppu`'s equivalent named, and `FCEUPPU_GetCHR` is already it.

## Verification of this step

* `cargo fmt -p zax-harness -- --check`: clean;
* `cargo clippy --release -p zax-harness --all-targets -- -D warnings`, **with and without**
  `fceux-core`: clean;
* `cargo test --release -p zax-harness --features fceux-core`: the five `fceux_tiles` unit tests
  pass, including the new one;
* both sweeps re-run end to end with the new column: `unexplained: 0` on the 98 originals and on
  the 291 public ROMs.
