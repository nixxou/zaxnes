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
- [x] 3. Frame 926.
- [x] 4. What the hardware does there.
- [x] 5. The report.
- [x] 6. The final pass.

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

---

## Step 3 — frame 926, and the branch is `$8241` on a read of `$4015`

The result first: **two of the three differing bytes are step 2's artefact again and the third is
the divergence.** `$0039` is the game's own *frame-sync semaphore*; it reads `01` here and `00`
there because **Zaxnes's frame-926 work ran about sixty CPU cycles behind `NesHawk`'s and the NMI
caught it seventeen instructions short of the point that clears it.** Nothing read anything to
decide that. What then turns sixty cycles into two different games is a read, and it is
**`LDA $4015` at `$823C`** — the APU status register, whose bit 4 is *"the DMC still has bytes to
fetch"* — feeding `AND #$10 / BNE $8248` at **`$8241`**. And the one mechanism in the frame of the
right size to move the CPU sixty cycles is the **DMC's own sample-fetch DMA**: 56 fetches in frame
926, 3 or 4 halt cycles each, **168 to 224 cycles**.

### The three bytes, and which of them is the divergence

`frame-trace --frame 926 --watch 0038 --watch 0039 --watch 0049 --converge <R>`:

| | written by | at instruction | scanline/dot | ours | theirs |
|---|---|---|---|---|---|
| `$0038` | `STA $38` at **`$F049`** | 8899 | SL 238 dot 225 | `86` | `87` |
| `$0049` | `STA $49` at **`$F075`** | 8884 | SL 238 dot 57 | `13` | `00` |
| **`$0039`** | **`INC $39` at `$C37C`** | **133** | **SL 249 dot 120** | **`01`** | **`00`** |

`$0038` and `$0049` are the MMC3 bank-select and bank-data shadows the game keeps beside every
`$8000`/`$8001` write; both are rewritten dozens of times a frame (22 and 9 changes in frame 926
alone) and both change in the last hundred instructions, so they are the step-2 class exactly. The
new `--converge-window` shows it directly, and this is what the option was added for — a count
says how far apart two machines are and never in which direction:

```
      < 8973 (-4): 9 byte(s), 4 live $0038 $0039 $0048 $0049
      < 8976 (-1): 8 byte(s), 3 live $0038 $0039 $0049
      = 8977 (+0): 8 byte(s), 3 live $0038 $0039 $0049
      > 8994 (+17): 14 byte(s), 3 live $0038 $0039 $0049
```

`$0048` drops out one instruction before the boundary — one more artefact that resolved itself —
and `$0039` is in every line.

### What `$0039` is, measured over twelve frames

`frame-trace --frame 918 --frames 12 --watch 0039 --watch 01A5` names both ends of it:

```
    $C37C  E6 39     INC $39       ; set, in the NMI path, scanline 249
  …
    $C067  48        PHA
    $C068  A5 35     LDA $35
    $C06A  10 0A     BPL $C076     ; the wait is enabled when bit 7 of $35 is set
    $C06C  A9 00     LDA #$00
    $C06E  85 39     STA $39       ; cleared: this frame's work is done
    $C070  A5 39     LDA $39
    $C072  F0 FC     BEQ $C070     ; and spin until the next frame's NMI sets it
```

So `$0039` is the **frame semaphore**: non-zero means "a frame has begun and the main loop has not
finished its work", and the main loop clears it and then spins at `$C070` until the next NMI. That
spin is why nothing before frame 926 ever showed this: *however* far apart the two machines'
program positions drift inside a frame, the spin absorbs it and both leave at the same NMI.

| frame | `$0039` set at | cleared at (`$C06E`) | and then |
|---|---|---|---|
| 918 | 133 | 6 548, SL 171 | spins to the frame's end |
| 919 | 132 | **8 802**, SL 235 | spins |
| 920-925 | 132-604 | 6 015 - 7 068, SL 146-187 | spins |
| **926** | **133** | **never reached** | the frame ends mid-work |
| 927 | 603 (`01` → **`02`**) | 1 621 | — |

Frame 926 is the one frame of the twelve in which the main loop did not reach `$C06E`. Our frame
926 begins at instruction 0 **inside the spin** (`$C070 LDA $39` at SL 241 dot 1), so frame 925's
work had finished normally; the NMI arrives at instruction 2, SL 241 dot 31; `INC $39` at 133; and
the frame then ends at instruction 8977 with the semaphore still set.

### How far short, and in which direction

Our frame 926's last instruction is **8977, `$F061 STA $8001`, scanline 240 dot 338**. From there
to `$C06E` is **seventeen instructions and 61 CPU cycles**:

```
  $F064 RTS · $E8F3 JMP $F07B · $F07B LDA #$87 · $F07D STA $38 · $F07F STA $8000
  $F082 LDA $0576 · $F085 STA $49 · $F087 STA $8001 · $F08A RTS · $E4BF LDX $056A
  $E4C2 BEQ $E4A6 · $E4A6 JSR $C067 · $C067 PHA · $C068 LDA $35 · $C06A BPL
  $C06C LDA #$00 · $C06E STA $39
```

And those seventeen instructions write **exactly the reference's three values**: `$F07D STA $38`
with `#$87` → `87`; `$F085 STA $49` from `$0576` → `00`; `$C06E STA $39` → `00`. (Frame 925's
instructions 6001-6015 are the same seventeen, with the same three values, which is the control.)
So the reference's frame-926 state is **Zaxnes's own state seventeen instructions later**, and
`NesHawk` was therefore **ahead** of us — by at least 61 cycles less the 5-to-17-cycle reading
offset step 2 measured, so **about 45 to 55 CPU cycles at minimum**. There is no upper bound from
this side: `NesHawk` may have reached `$C06E` and spun, which leaves no trace in the 2 KiB.

That is the opposite sign from frame 733, where the reference was **170 cycles behind**. The two
machines' program position inside a frame therefore moves both ways, and is re-synchronised every
frame by the spin.

### `--converge`: the last instant Zaxnes held `BizHawk`'s live memory

| frame | |
|---|---|
| **925** | **no live byte differs at 2 466 instants**, from instruction 6 741 (−2 405 from the boundary) to 9 206 (+60) — the whole of the spin, the boundary included |
| **926** | **at no instant of the frame does the live memory agree.** The closest is **1 live byte** — `$0039` alone — at instruction 8 881 |
| 927 | **56** live bytes differ at every instant, `$0048`, `$006A`, `$006C`, `$006E`, `$0071`, `$007F`, `$0083`, `$0091` among them, and `$0039` goes `01` → **`02`** at `$C37C` because the semaphore was never cleared |

So the last agreement is **inside frame 925**, and frame 926 is the divergence. The `02` at frame
927 is the mechanism by which it stops being a one-byte difference: the semaphore is now a count
of frames owed, the main loop's phase is a frame behind, and from 927 the two machines run
different code.

### What decided it: the read, and what it reads about

`$C06A BPL $C076` is the branch the semaphore's clear hangs off, and it is **not** the branch that
decided anything here — our machine never reached it. Nothing in frame 926 reads anything to
decide whether the work fits in a frame; what decides it is **when the NMI arrives relative to the
program**, and the NMI arrives at a fixed dot.

The branch that *does* turn this into two different games is the one the DMC feeds, and it is in
the frame on both machines:

```
    $823C  AD 15 40  LDA $4015     ; APU status; bit 4 = the DMC has bytes left to fetch
    $823F  29 10     AND #$10
    $8241  D0 05     BNE $8248     ; taken: a sample is still playing, do not disturb it
    $8243  A9 7F     LDA #$7F
    $8245  8D 11 40  STA $4011     ; not taken: drive the level directly
```

| frame | the value our `$4015` read returns | the branch at `$8241` | where |
|---|---|---|---|
| 925 | **`$1F`** — bit 4 **set** | taken, `RTS` | SL 20 dot 62 |
| **926** | **`$1F`** — bit 4 **set** | taken, `RTS` | SL 23 dot 124 |
| 927 | **`$0F`** — bit 4 **clear** | **not taken**, `STA $4011` | SL 26 dot 152 |
| 928 | `$0F` | not taken | SL 19 dot 192 |

So bit 4 of `$4015` is read once a frame and it is a **function of the DMC's remaining byte
count** — that is, of DMC timing to the fetch. In frame 926 our sample was still playing at
scanline 23 and ran out later in the frame; a machine whose DMC had exhausted the sample a little
earlier takes the other arm at `$8241`.

### The mechanism of the right size, and its number

`frame-trace` now reports `NesBus`'s own two DMA counters (`dmc_dma_count`, `oam_dma_count` — both
already public, for the DMA timing tests; no `zax-nes` change). Over all 701 frames of the window:

| | |
|---|---|
| frames with a DPCM sample playing | **212 of 701**, in 42 runs of 2 to 17 frames |
| fetches in a full frame of sample | **69** |
| what one fetch costs the CPU | **3 or 4 halt cycles** (nesdev, *DMA*) |
| so what the DMC takes from a frame | **207 to 276 cycles** |
| OAM transfers | **1 in every frame**, identical throughout |
| the overclock | **locked, 0** stalled instructions in every frame |
| `$2002` reads | **0** in every frame examined — not sprite 0, not overflow, not the vblank flag |

And the correlation with step 2's table is the measurement that matters:

| frame | DPCM playing? | fetches | the program-position offset |
|---|---|---|---|
| 645, 680, 740, 821, 837, 875 | **no** | **0** | **5, 5, 5, 6, 10, 10 cycles** |
| 714 | yes | 49 | 17 |
| 796 | yes | 61 | 5 |
| **733** | yes | **69** | **170, the reference behind** |
| **926** | yes | **56** | **≥45, the reference ahead** |

**Every frame in which the DMC is idle agrees to within ten cycles; both frames that are tens or
hundreds of cycles out have a sample playing.** That is not proof — 796 has 61 fetches and is
still 5 cycles — but it is the only mechanism in the frame whose size matches, and it is the only
one whose per-fetch cost is *conditional* (three cycles or four, depending on the cycle the halt
lands on), which is exactly what produces a difference of either sign that does not accumulate.

### The commands

```sh
R=$M/bizhawk-runs/kirby-jp-step1b.txt
$H frame-trace "$K" "$KR" --frame 926 --watch 0038 --watch 0039 --watch 0049 \
    --converge "$R" --extra 2000 --context 14 --out f926.txt
$H frame-trace "$K" "$KR" --frame 926 --converge "$R" --extra 200 --converge-window 26
$H frame-trace "$K" "$KR" --frame 925 --frames 3 --converge "$R" --extra 60 --converge-window 4

# what $0039 is, and the one frame of twelve that never clears it
$H frame-trace "$K" "$KR" --frame 918 --frames 12 --watch 0039 --watch 01A5 --context 2

# the DPCM census of the whole window, 701 frames in five seconds
$H frame-trace "$K" "$KR" --frame 400 --frames 701 | grep -E '^  frame|DMC sample'

# the $4015 read and its branch
for f in 925 926 927 928; do
  $H frame-trace "$K" "$KR" --frame $f --out g$f.txt >/dev/null
  grep -A4 'LDA \$4015' g$f.txt | head -5
done
```

---

## Step 4 — what the hardware does there, and the classification

### The sample, named from the game's own writes

Frame **923** starts it, and the three writes say everything:

| the write | at | value | what it means |
|---|---|---|---|
| `STA $4012` | `$83CE` | `$F4` | sample address `$C000 + $F4 × 64` = `$FD00` |
| `STA $4013` | `$83D4` | `$10` | sample length `$10 × 16 + 1` = **257 bytes** |
| `STA $4010` | `$8278` | `$0F` | **rate index `$F`**, loop **off**, IRQ **off** |
| `STA $4015` | `$827D`, `$8282` | `$0F`, `$1F` | bit 4 set: start the DMC |

nesdev, *APU DMC*, NTSC rate table: index `$F` is a period of **54 CPU cycles**, and a byte is
eight timer periods, so the memory reader asks for one byte every **432 CPU cycles**:
29 780.5 / 432 = **68.9 a frame**. Zaxnes's own counters read **69** in a full frame — and the
four frames of this sample read **63 + 69 + 69 + 56 = 257**, which is the sample length to the
byte. So the *number* of fetches Zaxnes performs is exactly the documented one; the only thing
left to disagree about is what each one costs and where its halt lands.

### What a fetch costs, from nesdev

nesdev, *DMA*:

> *"DMC DMA normally takes 3 or 4 cycles, depending on whether alignment is needed."*
> *"After the halt, DMC DMA always performs a dummy cycle where no work is done. If the next
> cycle is not a get cycle, then a cycle will be spent on alignment."*
> *"The CPU alternates between cycles on which DMA can get (read) and cycles on which DMA can
> put (write)."*
> *"The CPU only allows this on read cycles. If the CPU is writing, it ignores the halt and the
> DMA unit waits until the next cycle to try again, repeating until successful."*
> *"When the DMA process completes, the CPU performs the read it attempted when halted."*

Three things follow, and they are the whole of step 3's arithmetic:

1. the cost is **conditional** — three cycles or four, on the get/put parity of the cycle the
   halt lands on — so two machines that model it differently differ by a cycle **sometimes**, in
   either direction, and the difference does not accumulate into a constant;
2. the halt **slips** when the CPU is writing, so the cost also depends on the instruction mix
   the fetch falls in;
3. per frame the budget is **69 × 3 to 69 × 4 = 207 to 276 cycles**, and over the whole 257-byte
   sample **771 to 1 028**. Step 3 measured displacements of **≥45** cycles (frame 926, after 257
   fetches) and **170** (frame 733, after 128 fetches since that sample began). Both are a
   **fraction of a cycle per fetch** — 0.2 and 1.3 — so the mechanism has the budget for them
   several times over, and nothing else in the frame does.

And what a console cannot do, which bounds it from the other side: nesdev's *Cycle reference
chart* puts NTSC at one 21.477272 MHz crystal divided by 12 for the CPU and 4 for the PPU,
**29 780.5 CPU cycles a frame**, with DMA the only thing that moves CPU time against PPU time —
and DMA *takes* cycles, never grants them. Zaxnes's frames in this window measure **29 778 to
29 785** (the spread is one whole instruction either side of the boundary test), and the overclock
is locked with **0** free instructions in every frame traced.

### The public tests that cover it, run on Zaxnes for this step

`ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`, this lot's own binary:

| suite | ROMs | pass | fail | no documented verdict |
|---|---|---|---|---|
| `dmc_dma_during_read4` | 5 | **3** | 0 | 2 (`dma_2007_read`, `double_2007_read`) |
| `sprdma_and_dmc_dma` | 2 | **2** | 0 | 0 |
| `dmc_tests` | 4 | 0 | 0 | 4 (nothing readable on screen) |
| `apu_test` (suite + 8 singles + …) | 19 | **19** | 0 | 0 |
| `blargg_apu_2005.07.30` | 11 | **11** | 0 | 0 |
| `apu_reset` | 6 | **6** | 0 | 0 |
| `cpu_interrupts_v2` | 6 | **6** | 0 | 0 |
| `instr_misc` | 5 | **5** | 0 | 0 |
| **total** | **58** | **52** | **0** | **6** |

**No failure anywhere, on any ROM that reports a verdict**, and the ones that matter print their
own measurement rather than a verdict alone:

| | what it prints on Zaxnes |
|---|---|
| `sprdma_and_dmc_dma` | `0F 526 / SPRDMA and DMC DMA / Passed` |
| `sprdma_and_dmc_dma_512` | `0F 528 / Passed` |
| `dmc_dma_during_read4/dma_4016_read` | `08 08 07 08 08 / Passed` — the `$4016` collision, counted |
| `dmc_dma_during_read4/read_write_2007` | `33 11 22 33 09 55 66 77` twice over / `Passed` |
| `cpu_interrupts_v2/4-irq_and_dma` | `9 +527 / Passed` |
| `apu_test/7-dmc_basics`, `8-dmc_rates` | `Passed` |
| `apu_reset/4017_timing` | `Delay after effective $4017 write: 10 / Passed` |
| `instr_misc/04-dummy_reads_apu` | `Passed` |

`AccuracyCoin` scores **128 of 147**, which is the tally `docs/review/MF-T-test-roms.md` records
for this tree — reproduced here character for character, so the suite is being run the same way
earlier lots ran it. Nineteen rows fail, and **three of them are on its *APU Registers and DMA
tests* page**: `DMC DMA Bus Conflicts`, `Explicit DMA Abort`, `Implicit DMA Abort`.

### The one named Zaxnes gap on this mechanism — and it is too small, and is not to be fixed

`docs/progress/MF-4a.md` §Step 3 already diagnosed all three, which is why this step does not have
to. Two of them are the same thing and it is the right kind of thing:

> **The hardware** (nesdev, *DMA*): *"When sample playback is stopped during the APU cycle before
> a reload DMA would schedule (that is, on the 2nd or 3rd CPU cycle before the halt attempt), the
> DMA starts, but is aborted after a single cycle."* Either explicitly — the enable bit cleared by
> a `$4015` write — or **implicitly, when the sample ends just before a reload would trigger**.
>
> **Zaxnes**: `NesBus::run_dma` (`rust/crates/zax-nes/src/nes_bus.rs:682`) owns every cycle from
> the halt onwards, and a `$4015` write is a write cycle, so the only abort it can see is one
> *before* the halt, which `after_apu_write` handles. The post-halt abort it cannot reach: the
> fetch either costs its full three or four cycles or none. The missing piece is a two-or-three
> cycle delay on the `$4015` **stop**, matching the one `DmcChannel::set_enabled`
> (`rust/crates/zax-nes/src/apu/dmc_channel.rs:218`) already returns for the **start**, plus an
> abort check inside the DMA loop.
>
> **Reproduction**: `AccuracyCoin.nes`, rows `Explicit DMA Abort` and `Implicit DMA Abort`, code 2
> each — `zax-harness test-roms <roms> out.json --filter AccuracyCoin`.
>
> **Size of the fix**: written once, measured, and **taken back out** in lot MF-4a step 3 — it
> moved no `AccuracyCoin` row, it changes what `$4015` bit 4 reads back for two to three cycles
> after a disable, and nothing in the public suites pins the new behaviour. REQ touched:
> **REQ-CORE-113** (DMA), with REQ-CORE-110 beside it; `Q-119` is the open ticket on this code and
> its own §3 records that the DMA cycles are 0.011 % of a run.
>
> **Not fixed here**, and the owner's rule is not the only reason: **it is the wrong size.** The
> implicit abort happens **once per sample end** — this sample does not loop (`$4010` bit 6 clear)
> — and is worth **one to three cycles**. Frame 926 needs **forty-five to fifty-five**. So even if
> `NesHawk` were right and Zaxnes wrong here, it accounts for at most a twentieth of the
> displacement.

The third row, `DMC DMA Bus Conflicts`, is about the *byte* a colliding fetch returns, not about
cycles, and MF-4a's diagnosis is that it needs a per-controller-port open-bus mask `InputPorts`
does not have — a controller-port change, not a DMA one. It cannot move the CPU's position at all.

### The classification

**Undetermined, and much more sharply than EVAL-SYNC-3 could put it.**

| | |
|---|---|
| the eight isolated frames of 645-875 | **not an error of either machine.** A reading-point artefact, and step 2 puts a number on it for the first time: **5 to 17 CPU cycles**, the reference reading earlier |
| **frames 733 and 926** | **undetermined.** A difference in the two machines' CPU position inside the frame of **170** and **≥45** cycles, re-synchronised every frame by the game's own spin at `$C070` and visible only in the two frames of 526 whose work ends within that distance of vblank |
| what it is **not** | the overclock (locked, 0 stalls, every frame); `$2002`, sprite 0, sprite overflow (**not read once**); the OAM transfer (one a frame, in every frame, `sprdma_and_dmc_dma` passing on both counts); the MMC3 IRQ (same scanline 174, dots 288 and 296); the mapper's soft reset (no reset in this window); the power-on RAM (400 frames of agreement before it); an open-bus or unmapped read (every differing value is in the reference's own RAM too) |
| what is left, by size | the **DMC sample-fetch DMA**, 207-276 cycles a frame and 771-1 028 over the sample, whose per-fetch cost is conditional in the hardware. **Every live-difference frame in which the DMC is idle agrees to within ten cycles** (six of them); both frames tens or hundreds of cycles out have a sample playing |
| Zaxnes's standing on it | **52 of 58 public ROMs pass and none fails**, the DMA-cycle ROMs printing their own counts; `AccuracyCoin` 128/147 with **two** DMA rows that are a known, already-diagnosed gap worth one to three cycles a sample |
| `NesHawk`'s standing on it | **not published.** No figure for its DMC halt cost, its get/put alignment or its abort behaviour is available from a public source, and its source may not be read |

So this is **not** classified (a): no hardware behaviour has been found that Zaxnes gets wrong by
anything like the measured size, and on every public measurement of this mechanism Zaxnes prints
the documented answer. Neither is it classified (b): to say "a `NesHawk` behaviour Zaxnes is right
not to share" would need `NesHawk`'s figure, and there is none. It is **undetermined**, with one
candidate mechanism identified by size and by correlation, and one measurement that would close
it.

### What to ask `BizHawk` for, in order of what it settles

Four additions, all to the script that already produced `kirby-jp-step1b.txt`. Function names are
from `BizHawk`'s public *Lua Functions List*; the spellings should be checked against the list of
the version in use, since it has grown between releases.

**1. The reading instant itself — this settles step 2 completely and gives step 3 its sign.**
On the same hook that already writes the `ram=` field, add two fields:

```lua
-- appended to each existing line, frames 400-1100
local cyc = emu.totalexecutedcycles()
local pc  = emu.getregister("PC")
-- ... write "cyc=" .. cyc .. " pc=" .. string.format("%04X", pc)
```

Zaxnes can then stop *inferring* the reading point from which bytes moved and read it off: `pc`
alone decides eight of the nine frames of step 2 (the copy loop at `$C221`-`$C22A` is six
instructions, so the PC says which iteration), and `cyc` gives the rest. **This is the single
most valuable line in the list** — it converts every figure of steps 2 and 3 from a bounded
inference into a measurement.

**2. The frame-sync point, which is frame 926's own question.** `$C06E` is the `STA $39` that
clears the semaphore and `$C37C` is the `INC $39` that sets it:

```lua
event.onmemoryexecute(function()
  -- "C06E f=<framecount> cyc=<totalexecutedcycles>"
end, 0xC06E)
event.onmemoryexecute(function()
  -- "C37C f=<framecount> cyc=<totalexecutedcycles>"
end, 0xC37C)
```

for frames **900 to 950**. Two outcomes, and they point opposite ways: if frame 926 has **no**
`C06E` line then `NesHawk` did not reach it either and the three bytes have another explanation
entirely; if it has one, `cyc(C06E) − cyc(C37C)` is the cycle count `NesHawk` needed for frame
926's work, against Zaxnes's — *which never finished it* — and the difference is the number this
lot could only bound at ≥45.

**3. The `$4015` read, which is the branch.** The value `NesHawk` returns at `$823C` decides
`BNE $8248` at `$8241`:

```lua
event.onmemoryread(function(addr, val)
  -- "4015 f=<framecount> cyc=<totalexecutedcycles> val=<val>"
end, 0x4015)
```

for frames **900 to 950** and **720 to 745**. Zaxnes reads `$1F` at frames 925 and 926 and `$0F`
at 927 and 928 (step 3's table). If `NesHawk` reads `$0F` at frame 926, the two machines took
**different arms of a branch** there and the divergence is a code-path difference rather than a
few cycles of slack — and the quantity that decided it is the DMC's bytes-remaining counter, which
is the candidate of §4 named directly.

**4. The frame length, to say whether the drift is inside a frame or between frames.**
`emu.totalexecutedcycles()` logged once a frame at `event.onframeend` (or at the same hook as 1)
over frames **400-1100**: the first differences give `NesHawk`'s cycles per frame against Zaxnes's
29 778-29 785. Equal totals put the drift **inside** the frame, which is where the DMC is; unequal
totals would mean something about the frame itself and would change the whole diagnosis.

If 1 and 4 come back and the drift is inside the frame with the reading instant pinned, the
remaining ask is narrow: **the cycle at which `NesHawk` performs each DMC fetch** for one frame.
`event.onmemoryread` on the sample's own range (`$FD00`-`$FE00` for this sample, from the
`$4012` = `$F4` and `$4013` = `$10` above) logging `emu.totalexecutedcycles()` gives 69 numbers
for a frame; Zaxnes's own 69 are already available from `frame-trace`, and the two lists either
agree or name the cycle at which they first do not.

### The commands

```sh
export ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms
for f in dmc_dma_during_read4 sprdma_and_dmc_dma dmc_tests apu_test blargg_apu_2005 \
         apu_reset cpu_interrupts_v2 instr_misc AccuracyCoin; do
  $H test-roms "$ZAXNES_TEST_ROMS" "tr-$f.json" --filter "$f" --no-shots --quiet
done

# the sample the game starts, and its rate
$H frame-trace "$K" "$KR" --frame 923 --out f923.txt
grep -E 'ST[AXY] \$40(10|11|12|13|15)' f923.txt
```

---

## Step 5 — the report

`docs/review/MOVIE-SYNC.md` now covers all four lots. What was added, and where:

| | |
|---|---|
| the preamble | the fourth lot named, with a pointer to its three sections and the statement that it changes no emulation at all |
| **The short version** | a sixth paragraph: in step to frame 925 with ten screenshots matching, the parting at 926, the artefact measured at 5-17 cycles, the semaphore, the `$4015` branch and the DMC |
| **§1d** (new) | the two instruments — `--ram-ref-table` and `--converge-window` — each with the reason it exists (a report that stops at twelve; a count that has no sign), and the two DMA counters `frame-trace` now reports |
| **§4c** | the "measurement that would decide it" block gains a **Superseded** note: the owner made it, §4e is the answer, and its *second* ask — the per-scanline log — is **not** what is needed and why |
| **§4e** (new, placed after §4d so the lettering reads c, d, e) | the whole of this lot: where it parts, the pictures, the nine isolated frames with the cycle table, where `AA` comes from, frame 926 instruction by instruction, `--converge` on 925/926/927, the `$4015` branch, the sample named from the game's own writes, what a fetch costs from nesdev, the 58 public ROMs, the one named gap and why it is too small, the classification, and the four `BizHawk` Lua additions |
| **§5** | EVAL-SYNC-3's first two unknowns struck through and answered — the divergence **located**, the reading point **measured** — and three new ones, each a real bound: the DMC sized but not proved, the post-halt abort unmeasured on a game, and frame 733 still the odd one out |
| **§6** | a new subsection, *"the window is closed, and the answer is still no"*: reason 1 stronger than it was, reasons 2-4 untouched with reason 4 in its sharper form, and EVAL-SYNC-2's argument at its third demonstration. The to-do list is renumbered again: item 0 is now **four lines of Lua rather than a lot**, item 1 is the post-halt DMA abort decision, and EVAL-SYNC-3's item 0 is struck as done |
| **§6, commands** | every command of this lot, including the two new options, the DPCM census and the nine public suites |

The strike-throughs are the document's own convention: a reader who was told something was unknown
is owed the answer in the place they were told it.

One correction of an earlier lot's figure, made in the place it was said: §4c's screenshot table
read "0 of 6, the nearest 275 pixels of 57 344". That stands for the six frames it measured
(1 000 and later), and §4e now shows that the eleven frames **before** 950 are a different story —
ten of them match exactly. Nothing in §4c was wrong; its window simply began after the agreement
ended.

---

## Step 6 — the final pass

Everything the brief's final check asks for, re-run at the end rather than trusted from the step
that produced it.

### fmt and clippy

| | |
|---|---|
| `cargo fmt --all --check` | clean |
| clippy `-D warnings --all-targets`, **whole workspace**, feature **off** | clean |
| the same, feature **on** (`zax-app/fceux-core,zax-harness/fceux-core,zax-fceux/fceux-core`) | clean |

### The tests, `--include-ignored`

With `ZAXNES_ROMS=G:/3dsen/demo/roms`, `ZAXNES_TEST_ROMS=G:/3dsen/cleanroom/assets-free/test-roms`
and `ZAXNES_PROFILES=G:/3dsen/cleanroom/assets-neutral`:

| | passed | failed | ignored |
|---|---|---|---|
| **`zax-harness`**, feature off | **126** | 0 | 0 |
| **`zax-harness`**, feature on (`--test-threads 1`) | **146** | 0 | 0 |
| **`zax-nes`** (untouched; run to show it) | **1 187** | 0 | 0 |

EVAL-SYNC-3's figures move by exactly this lot's two new unit tests: the `zax-harness` lib goes
from 50 to **52**, its feature-off total from 124 to **126** and its feature-on total from 144 to
**146**. The two are
`ram_trace::tests::the_table_keeps_every_compared_frame_and_the_report_still_stops` and
`frame_trace::tests::the_converge_window_names_the_live_addresses_and_which_side_of_the_boundary`.
`zax-nes`'s 1 187 is EVAL-SYNC-3's number to the test, which it has to be: nothing in that crate
moved.

**One trap worth recording for whoever runs this next.** `compare_tests` calls
`std::env::temp_dir()`, which on Windows reads `TMP`/`TEMP` — and the brief's `TMP` must therefore
be given as a **Windows** path (`G:\…\target-ev\tmp`), not as the MSYS `/g/…` form. With the MSYS
form all four of its tests fail at `fs::create_dir_all`, which looks like a regression of this lot
and is not one. (A first attempt at the suite also died with `LNK1104` while the host's `C:` was
full; it linked and passed on the next attempt with `TEMP` on `G:`.)

### golden-core

`zax-harness golden-core "G:/3DSen/test-originals" <out.jsonl> 1200`, feature off, 98 ROMs,
1 min 45 s:

    8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773

**Byte-identical** to the value the brief names, which is lot FCEUX-23's baseline and EVAL-SYNC's,
EVAL-SYNC-2's and EVAL-SYNC-3's. It is the expected answer rather than a result this time, and the
reason is in the next section: `zax-nes` did not move.

### The two movies, re-run with the final binary

| | |
|---|---|
| *Faxanadu* `[USA][!!]` against `fax-usa.txt` | **207** of 832 checksum lines, **every** lag count agrees, last agreeing line **83 200**, last picture `03d7e6a1…` |
| *Faxanadu* `[Rev 1][!!]` against `fax-rev1.txt` | **220** of 832, every lag count, last agreeing line **83 200**, last picture `13920624…` |
| *Kirby* against `kirby-jp.txt`, the whole movie | **156 312** frames run, **every** one of the 1 563 lag counts agrees, **24** input-insensitive frames — `BizHawk`'s own figure — 2 of 1 563 checksum lines, the last at frame 200 |
| *Kirby* against `kirby-jp-step1b.txt`, frames 400-1 100 | **701 of 701** compared, **517** with no live difference, the last at **925**, **701 of 701** lag counts |

Every one of those figures is EVAL-SYNC-3's, to the digit and to the digest. That is what it should
be: this lot's only code change is a new option on each of two subcommands, and both are off unless
asked for.

### What moved

`git diff --stat 301aa3d5..HEAD` over `zax-nes`, `zax-app`, `zax-fceux`, `zax-render`,
`zax-engine3d`, `zax-core-api` **and `Cargo.lock`** is **empty**. No new dependency, and every
cargo command of this pass was run `--offline`.

| | lines |
|---|---|
| `docs/progress/EVAL-SYNC-4.md` | +762 |
| `docs/review/MOVIE-SYNC.md` | +448 |
| `rust/crates/zax-harness/src/frame_trace.rs` | +168 |
| `rust/crates/zax-harness/src/ram_trace.rs` | +122 |
| `rust/crates/zax-harness/src/movie_sync.rs` | +23 |
| **`rust/crates/zax-nes`** | **0 — the owner's rule, and it holds to the byte** |

Both new options use only public API that already existed: `NesBus::dmc_dma_count` and
`oam_dma_count` are `pub const fn` there for the DMA timing tests, and nothing else was needed.

### The disk

The pass began with **9.8 GB** free on `G:` and ended with **9.8 GB**, well above the brief's 5 GB
stop; the release tree under `rust/target-ev` is 1.8 GB. No debug tree was built — every cargo
command was `--release` — and no GPU campaign was run (the four `*_wgpu*` test targets have no
tests enabled in this configuration).

**The host's `C:` is full**, 0 bytes free, with 2.9 GB of it in
`AppData\Local\Temp\claude` from earlier sessions. It cost this pass one spurious `LNK1104` and
several commands that could not write their output. Nothing was deleted; it is the owner's disk.

### What is left for the owner

1. **Close frame 926, and it is four lines of Lua rather than a lot.** §4e of the report names
   them; step 4 above gives the code. The first alone — `emu.totalexecutedcycles()` and
   `emu.getregister("PC")` beside each existing `ram=` line — settles the whole artefact class and
   gives frame 926 its sign. **Nothing is needed from this side first, and in particular no change
   to `zax-nes`.**
2. **Decide whether the post-halt DMA abort is worth a lot of its own.** A named gap with a written
   diagnosis (MF-4a §Step 3), two failing `AccuracyCoin` rows, a fix built and withdrawn, and an
   open ticket (`Q-119`). It is **not** frame 926's cause — twenty times too small — but it is the
   only DMA behaviour this build is known to get wrong, and it has never been measured on a game.
3. **Adopt or amend REQ-CORE-122**, unchanged from EVAL-SYNC-3:
   `docs/review/AMD-REQ-CORE-122-mapper-reset.md` has the exact wording.
4. **`.bk2` in the application**, unchanged, with the reader's three additions (the
   `00 00 00 00 FF FF FF FF` power-on pattern, the refusal of a non-`null`
   `InitialWRamStatePattern`, and a `.bk2`'s `Reset` happening **after** the flagged frame). The
   caveat softens again: an MMC3 `.bk2` now holds for **925** frames rather than 400.
5. **The warning for the next harness author**, unchanged and now cheap to state: `NesCore` powers
   on with the 3D built-in patches **enabled**.
6. The three `.fm2` header fields `zax-app`'s Zaxnes path ignores, unchanged from EVAL-SYNC.
