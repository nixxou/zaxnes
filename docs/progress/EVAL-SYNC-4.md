# EVAL-SYNC-4 — frames 400-1100 of *Kirby*, the isolated differences, and frame 926

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `301aa3d5` (lot EVAL-SYNC-3).

EVAL-SYNC-3 closed the first divergence — `movie-sync` had been playing the game's 3D widescreen
hack — and left one window open: frames **401 to 1 000**, where a single object on screen parts.
It named the measurement that would close it, and the owner made it:
`G:/3DSen/test-movies/bizhawk-runs/kirby-jp-step1b.txt`, frames **400 to 1 100** in
`kirby-jp-step1.txt`'s own format, with screenshots every 50 frames.

## Steps

- [x] 1. Reproduce the comparison, and the per-frame table.
- [ ] 2. The isolated differences of 645-875.
- [ ] 3. Frame 926.
- [ ] 4. What the hardware does there.
- [ ] 5. The report.
- [ ] 6. The final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| `ZAXNES_ROMS` | `G:/3dsen/demo/roms` |
| `ZAXNES_TEST_ROMS` | `G:/3dsen/cleanroom/assets-free/test-roms` |
| `ZAXNES_PROFILES` | `G:/3dsen/cleanroom/assets-neutral` |
| disk on `G:` at the start | 9.8 GB free |

---

## Step 1 — the comparison reproduced, and the per-frame table

### The reference takes no new reading

`kirby-jp-step1b.txt` is `kirby-jp-step1.txt`'s format exactly — `romhash` and then 701 lines
`<f> <checksum> lag=<n> ram=<4096 hex digits>` for frames **400 to 1 100** — so `--ram-ref` took
it unchanged. The two files **overlap by one line** and that line is byte-identical
(`400 05450e3e lag=6 …`), which is the cheapest possible check that the owner's second run is the
same run as the first and not a fresh one that happened to be labelled the same way.

The one instrument this step had to add is **`--ram-ref-table <file>`**: the printed report lists
the first twelve frames with a live difference (`FRAMES_LISTED`), and this window holds nine
isolated ones and then one hundred and seventy-five in a row. The option writes **every** compared
frame's own line — `<frame> total= live= stack= temp=` and the live addresses — so the table is a
file rather than a paragraph. `RamCompare::keep_table` is off by default, a movie being 156 312
frames of entries. One new unit test
(`ram_trace::tests::the_table_keeps_every_compared_frame_and_the_report_still_stops`) asserts both
halves: the table keeps all twenty frames of a synthetic run where the report keeps twelve, and
the two agree about the measurement itself (`plain.report() == tabled.report()`).

### What the run says

`movie-sync --engine zaxnes --lag --frames 1100 --trace-step 1 --trace-ref <R> --ram-ref <R>
--ram-ref-table <table>`, the EVAL-SYNC-3 defaults (resets after the flagged frame, built-in
patches **off**):

| | |
|---|---|
| frames compared, byte for byte | **701 of 701** |
| identical to the byte | **0** |
| with **no live difference** | **517 of 701** |
| the last frame with no live difference | **925** |
| the first live difference | frame **645** |
| frames with a live difference | **184** — nine isolated, then 926 to 1 100 **without a gap** |
| checksum lines agreeing | **0 of 701** |
| **lag counts** | **every one of the 701 agrees** |

### The nine isolated frames, 400-925

Exactly the owner's list, address for address and value for value (ours/theirs):

| frame | total | live | stack | below `$0020` | the live addresses |
|---|---|---|---|---|---|
| **645** | 3 | 1 | 2 | 0 | `$04BE` = FF/0F |
| **680** | 3 | 1 | 2 | 0 | `$04E0` = 00/10 |
| **714** | 5 | 1 | 4 | 0 | `$0405` = 90/AA |
| **733** | 14 | **8** | 5 | 1 | `$0040` = 99/76, `$0048` = 30/12, `$0493`-`$0498` = AA AA AA AA AA AA / C0 55 15 FF FF FF |
| **740** | 11 | 1 | 10 | 0 | `$04AE` = 0F/4F |
| **796** | 7 | 1 | 6 | 0 | `$0490` = 00/D5 |
| **821** | 2 | 1 | 1 | 0 | `$044F` = AA/00 |
| **837** | 4 | 1 | 3 | 0 | `$04FE` = AA/F5 |
| **875** | 3 | 1 | 2 | 0 | `$042C` = 72/15 |

Eight of the nine name an address in page `$04`; the ninth (733) names two in the zero page above
`$001F` **and** six consecutive bytes of page `$04`. Step 2 takes each one.

### And frame 926 onwards, which is the divergence

| frame | 926 | 927 | 928 | 929 | 930 | 931 | … | 935 | … |
|---|---|---|---|---|---|---|---|---|---|
| bytes differing | 8 | 82 | 38 | 32 | 52 | 51 | | 208 | |
| of them live | **3** | 56 | 30 | 25 | 46 | 46 | | 168 | |

Frame 926's three are `$0038` = 86/87, `$0039` = 01/00 and `$0049` = 13/00, and **no frame from
926 to 1 100 has no live difference**. Step 3 takes it.

### The pictures, which say the same thing from the other side

The owner's screenshots every fifty frames, under the colour-bijection rule of EVAL-SYNC §4
(this run's rows 8-231 against the reference's 224, every colour of one mapping to exactly one
colour of the other). The pairing was **measured rather than assumed**, both readings of
"`s<f>.png` is the start of frame *f*" being tried:

| | our frame *f* against `s<f>.png` | our frame *f-1* against it |
|---|---|---|
| MATCH, of the 15 shots | **10** | 1 |

So our frame *f* answers for `s<f>.png`, which is the convention EVAL-SYNC-3 used for
`kirby-jp-f1000.png` and is now a measurement. On that pairing:

| frame | 400 | 450 | 500 | 550 | 600 | 650 | 700 | 750 | 800 | 850 | 900 | 950 | 1 000 | 1 050 | 1 100 |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| verdict | no | **M** | **M** | **M** | **M** | **M** | **M** | **M** | **M** | **M** | **M** | no | no | no | no |
| pixels breaking the bijection | **1** | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 38 195 | 335 | 45 088 | 52 156 |

**Ten of eleven shots in the agreeing stretch match exactly**, which is the first time any *Kirby*
screenshot has matched at all — EVAL-SYNC-3 scored 0 of 6, the nearest being 275 pixels. The
picture therefore agrees with the work RAM: the two machines are running the same game to frame
~900 and have parted by 950.

Two entries are worth their own line:

* **frame 400 breaks the bijection by exactly one pixel**, at (209, 167) of the cropped picture —
  `(252,160,72)` here against `(87,29,0)` there — on a frame whose work RAM has **no live
  difference** at all. One pixel is not a desynchronised game; it is the same class as the stack
  page, one object's rendering caught at a slightly different moment, and it is the only such
  pixel in eleven shots.
* **frame 1 000's 335 pixels are inside x 147-171, y 97-113**, the same 25x17 box EVAL-SYNC-3
  measured at 275 differing pixels. The two counts are of different things (bijection breaks here,
  differing pixels there) and the box is the same, so this is EVAL-SYNC-3's "one object" seen
  again from downstream of the frame-926 divergence.

At **1 050** `BizHawk`'s picture is **one colour** (the file is 297 bytes) and ours is twelve: the
reference is on a blank transition and we are not, which is two machines playing different games
rather than one object apart.

### The commands

```sh
H=./target-ev/release/zax-harness.exe
M=G:/3DSen/test-movies
K="$M/bk2/keyword-kirbysadventure-noabilities.bk2"
KR="$M/roms/Hoshi no Kirby - Yume no Izumi no Monogatari [Japan][!!].nes"
R=$M/bizhawk-runs/kirby-jp-step1b.txt

$H movie-sync "$K" "$KR" --engine zaxnes --lag --frames 1100 --trace-step 1 \
    --trace-ref "$R" --ram-ref "$R" --ram-ref-table table.txt
awk '!/^#/ && $3 != "live=0"' table.txt          # the 184 frames, nine of them isolated

# the pictures, at the owner's own fifty-frame grid and one frame either side of it
$H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 1100 --png shots \
    --png-frames 399,400,449,450,…,1099,1100
```

The screenshot comparison is the same short script as EVAL-SYNC §4 and is **not committed** (it
reads pictures of a commercial game). It was re-validated here by the pairing table above: a
script that said MATCH indiscriminately could not score 10 on one pairing and 1 on the other.
