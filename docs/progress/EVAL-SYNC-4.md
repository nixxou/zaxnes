# EVAL-SYNC-4 — frames 400-1100 of *Kirby*, the isolated differences, and frame 926

Branch `eval/bk2`, worktree `cleanroom/wt-bk2`, base `301aa3d5` (lot EVAL-SYNC-3).

EVAL-SYNC-3 closed the first divergence — `movie-sync` had been playing the game's 3D widescreen
hack — and left one window open: frames **401 to 1 000**, where a single object on screen parts.
It named the measurement that would close it, and the owner made it:
`G:/3DSen/test-movies/bizhawk-runs/kirby-jp-step1b.txt`, frames **400 to 1 100** in
`kirby-jp-step1.txt`'s own format, with screenshots every 50 frames.

## Steps

- [x] 1. Reproduce the comparison, and the per-frame table.
- [x] 2. The isolated differences of 645-875.
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

---

## Step 2 — the nine isolated differences, and where `AA` comes from

Said first, because it is the step's result: **eight of the nine are the reading-instant artefact,
and this step turns that from a class into a number — the reference reads the 2 KiB between 5 and
17 CPU cycles before Zaxnes's frame boundary.** The ninth, frame **733**, is the same mechanism at
**170** cycles, which no fixed reading point explains, and it is the one entry of the nine that is
**not** settled. And `AA` is **a value the program writes**: not open bus, not DMA, not an
unmapped or mirrored read, and the reference itself proves it.

### One routine accounts for all nine

`frame-trace --watch <addr>` names the writing instruction, and it is the **same instruction** in
all nine frames: `STA $0400,X` at **`$C223`**, inside

```
    $C221  B1 16     LDA ($16),Y      ; the job's source, a pointer in $16/$17
    $C223  9D 00 04  STA $0400,X      ; into the page-$04 buffer
    $C226  E8        INX
    $C227  C8        INY
    $C228  C4 01     CPY $01          ; $01 is the job's length
    $C22A  90 F5     BCC $C221
    $C22C  86 40     STX $40          ; the buffer index, saved for the next job
    $C22E  60        RTS
  . $C12A  C6 04     DEC $04          ; one job fewer
    $C12C  F0 46     BEQ $C174        ; the queue is empty
```

a **job-queue drain**: `$0004` counts the jobs left, each job's descriptor (`$0001` length,
`$0003` destination, `$0016`/`$0017` source) is read from a table, and the body copies bytes into
the 256-byte buffer at page `$04` — the buffer the NMI handler flushes to the PPU. It runs on the
last visible scanlines and the post-render line, scanline ~213 to 240, and in frame 733 it ran
**five** separate jobs (`STX $40` at instructions 7977, 8123, 8401, 8638, 8919).

So the frame boundary falls **inside a copy**, and whichever machine reads its 2 KiB later has
copied one byte more. That is the whole of it.

### The measurement, in cycles into the frame

`--converge` compares our own 2 KiB against the reference's bytes for the same frame at every
instruction. `cyc` below is CPU cycles from the start of our frame:

| frame | the live addresses | last instant with **0 live** differences | `cyc` there | `cyc` at our boundary | **the reference reads this many cycles early** |
|---|---|---|---|---|---|
| **645** | `$04BE` | instruction 9043, SL 240 dot 315 | 29 770 | 29 776 | **6** |
| **680** | `$04E0` | 8941, SL 240 dot 314 | 29 770 | 29 775 | **5** |
| **714** | `$0405` | 8901, SL 240 dot 287 | 29 760 | 29 777 | **17** |
| **740** | `$04AE` | 8996, SL 240 dot 320 | 29 772 | 29 777 | **5** |
| **796** | `$0490` | 8966, SL 240 dot 326 | 29 775 | 29 780 | **5** |
| **821** | `$044F` | 9012, SL 240 dot 306 | 29 766 | 29 776 | **10** |
| **837** | `$04FE` | 9066, SL 240 dot 309 | 29 769 | 29 779 | **10** |
| **875** | `$042C` | 9023, SL 240 dot 324 | 29 772 | 29 777 | **5** |
| **733** | `$0040`, `$0048`, `$0493`-`$0498` | 8883, **SL 239 dot 164** | **29 606** | 29 776 | **170** |

Our own boundary sits at cycle **29 775-29 780** of the frame in all nine, which is the control:
it says the spread in the last column is the *reference's* reading point moving, not ours.

Every run of this step printed `IDENTICAL, so tracing changed nothing`, and every frame reported
the 3D extension **locked** with **0** instructions costing no cycle.

### Eight of nine: the reading instant, now with a figure on it

Five to seventeen CPU cycles — **less than one 6502 instruction**, 15 to 51 PPU dots. The
reference's `ram=` for frame *f* is taken a handful of cycles **before** Zaxnes's vblank-start
boundary, which is why it is always the machine holding the *older* byte. That answers one of the
things EVAL-SYNC-3 §5 listed as out of reach from this side — that `--converge` can prove a frame
is a reading artefact rather than a divergence but cannot say by how many dots the two reading
points are apart — and the answer is **fewer than twenty cycles**, measured eight times
independently on eight different frames.

It also explains why only nine of 526 frames show anything: the drain has to still be running in
those last twenty cycles. In most frames the queue is empty well before vblank and both machines
hold the finished buffer.

### Where `AA` comes from, and it is the program

The brief asks whether `AA` is an open-bus read, a DMA, a value the program writes, or a read of
an unmapped or mirrored address. It is **the program's own data**, and three independent things say
so:

1. **The instruction that loads it is named.** Frame 821, instruction 9012:
   `LDA ($16),Y = $670D @ $671A = AA` — a job source in the **cartridge's PRG-RAM**, the MMC3's
   8 KiB at `$6000`-`$7FFF`. Frame 837: `$672D @ $674A = AA`. Frame 733: `$672D @ $6747`-`$674C`,
   six `AA` in a row.
2. **The reference holds the same `AA`.** `$044B`-`$044E` are `AA AA AA AA` *in the reference* at
   frame 821, `$04FA`-`$04FD` are at 837, `$0491`-`$0492` are at 733. `BizHawk` copied the same
   bytes from the same table and stopped one byte earlier. A value conjured by open bus on one
   machine could not appear in the other's RAM.
3. **It cannot be a power-on fill.** `AA` is in none of the four — the `.bk2`'s
   `00 00 00 00 FF FF FF FF`, all-`$FF`, all-`$00`, `xoroshiro128+` — and `--prg-ram` was not
   given, so the cartridge RAM is `zax-nes`'s own power-on.

Frame **714** is the mirror image and worth stating because it is the one where `AA` is on
`BizHawk`'s side: `$0405` is `90` here and `AA` there. `90` is what our `STA` at instruction 8902
had just written (`LDA ($16),Y = $011E @ $0120 = 90`, this job's source being in **console** RAM);
`AA` is the byte the **previous** frame's copy had left there. So in both directions `AA` is a
copied byte and the disagreement is only about whether the copy had reached that index.

No `$2002` read occurs in any of the nine frames (`$4014` once per frame in the NMI handler,
`$4011` once per frame, no `$4010`/`$4012`/`$4013`/`$4015` write at all), so sprite 0, sprite
overflow and the vblank flag are not in play, and neither is a DPCM sample fetch.

### Frame 733, which is not settled

Everything above holds for it — same routine, same instruction, same table, and **an instant of
our own frame holds the reference's live memory exactly** (0 of about 1 790 live bytes differing at
instruction 8883, with `$0001`, `$0003`, `$0016`, `$0017` — the job descriptor — identical in the
reference too). What does not hold is the figure: **170 cycles**, an order of magnitude past the
five to seventeen the other eight agree on. A reading point five to seventeen cycles before our
boundary cannot also be 170 cycles before it.

What the 170 cycles are, exactly: our machine finished the fifth job at instruction 8914
(`$0498`), ran `DEC $04` to `00` at 8921 and left the drain; the reference's state is the drain
still at `$0493` with `$0004` = `01`, i.e. **six and a half iterations plus the exit behind us**.
The one byte below `$0020` that differs at this frame is `$0004` itself (`00` here, `01` there),
which is the queue counter and says the same thing from the other side.

So at the end of frame 733 the two machines' CPU was about **1.5 scanlines apart in program
position**, and it did not propagate: frame 734 and the 191 frames to 925 have **no live
difference at all**, frames 740 and 750 are among them, and the screenshot at 750 matches.

What it is **not**, each measured rather than assumed:

| | |
|---|---|
| the 3D extension | **locked**, 0 instructions costing no cycle |
| `$2002`, sprite 0, sprite overflow | **not read once** in the frame |
| a DPCM fetch stealing cycles | no `$4010`/`$4012`/`$4013`/`$4015` write in the frame; `$4011` is written once, which is the level register and fetches nothing |
| the OAM DMA | one, at SL 241 dot 168, in the NMI handler — the same dot as in frames 645 and 821 |
| the MMC3 IRQ | one, entering its handler at **SL 174 dot 291** — again the same dot as 645 and 821 |
| an open-bus or unmapped read | the differing values are all in the reference's own RAM too (above) |

Step 4 says what further measurement would settle it, and it is one line added to the owner's
existing script.

### The commands

```sh
R=$M/bizhawk-runs/kirby-jp-step1b.txt
for fa in 645:04BE 680:04E0 714:0405 740:04AE 796:0490 821:044F 837:04FE 875:042C; do
  f=${fa%%:*}; a=${fa##*:}
  $H frame-trace "$K" "$KR" --frame $f --watch $a --converge "$R" --extra 1500 --out f$f.txt
done
$H frame-trace "$K" "$KR" --frame 733 --watch 0040 --watch 0048 --watch 0493 --watch 0498 \
    --converge "$R" --extra 1500 --context 6 --out f733.txt
$H frame-trace "$K" "$KR" --frame 733 --watch 0004 --converge "$R" --extra 10 --context 8

# our own bytes, so that the two artefact classes can be read beside the live ones
$H movie-sync "$K" "$KR" --engine zaxnes --no-lag --frames 740 --dump dump \
    --dump-frames 645,714,733
```
