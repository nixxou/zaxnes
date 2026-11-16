# NESHAWK-PROFILE-13 — tree frame 40 is the movie's own empty socket

Branch `feat/neshawk-profile`, parent `1115e023` (NESHAWK-PROFILE-12). The item list is
`docs/review/NESHAWK-PROFILE.md`; lot 12's step 2-3 and its "Round 3's window" are the brief.

## Steps

1. Tree frame 40's `$4016` collision: measure it on BizHawk, name the differing read, implement,
   re-run every window.
2. Rounds, until the movie ends or five rounds.
3. No regression and the final pass.

## The environment this lot runs in

| | |
|---|---|
| `CARGO_TARGET_DIR` | `<worktree>/rust/target-ev` |
| `TMP` / `TEMP` | `<worktree>/rust/target-ev/tmp` |
| `CARGO_INCREMENTAL` | `0` |
| disk at the start | `G:` 31 GB free (stop at 5), `C:` 8.4 GB (stop at 1) |

---

## Step 1 — the first difference of any kind is not a timing item at all

### The answer first

**The *Hoshi no Kirby* `.bk2` says `"NesRightPort": "UnpluggedNES"`, and this side read that field,
accepted it and threw it away.** `bk2.rs`'s `check_sync_settings` has checked `NesLeftPort` and
`NesRightPort` since the reader was written — `ControllerNES` and `UnpluggedNES` both pass — and
then returned `Ok(())`, so every `.bk2` was replayed with a **pad in both sockets**. Kirby reads
`$4017`, the empty one.

**A pad and an empty socket differ on exactly one read: the ninth after a strobe.** The first
eight are 0 on both (no button held). From the ninth a standard pad answers **1** — its shift
register is depleted — and an empty socket answers 0 for ever, having none. A game only ever
reaches a ninth read when **a DMA adds a clock to its own eight**, which is why twelve lots did
not see it: the work-RAM reference cannot, and nothing else in the movie asks.

**Tree frame 40 is that read, and BizHawk's own stream shows it** (`q192.lua`, the whole bus
stream of BizHawk frames 38-40). Kirby's fourth input pass of the frame, port 2:

| cycle (frame-relative) | 1452 | 1466 | 1480 | 1494 | 1498 | 1512 | 1526 | 1540 | **1554** |
|---|---|---|---|---|---|---|---|---|---|
| NesHawk | 40 | 40 | 40 | 40 ¹ | 00 ² | 40 | 40 | 40 | **40** |
| this core, before | 40 | 40 | 40 | 40 ¹ | 40 | 40 | 40 | 40 | **41** |

¹ the DMC halt's re-read of the CPU's own address — a ninth clock on both machines.
² NesHawk's bus carries a second re-read at 1495 and fetches `$FC40` at 1496; its `00` is the
same byte with the open-bus bits taken from the fetch. The **value** that matters is the last
column: `41` has bit 0 set, Kirby's `AND #$03` is then non-zero, and the routine takes **two more
passes** — six strobe pairs in the frame against NesHawk's four.

### The two candidate rules, and how each was closed

The brief's hypothesis was a DMA/controller **rule** — whether a DMA re-read clocks the port.
**Refused by measurement, and by the strongest gate the recipe has**: `dmc_dma_during_read4`
/`dma_4016_read` is a controlled sweep of exactly that question (strobe, `lda $4016` with a DMC
DMA walked across five alignments, then count the reads to a 1), and put to BizHawk itself:

| | output | CRC | verdict |
|---|---|---|---|
| NesHawk (`q189.lua`, NesHawk **forced**) | `08 08 08 08 07` | `6B48DA20` | Failed |
| this core, `--profile neshawk` | `08 08 08 08 07` | `6B48DA20` | Failed |
| this core, `--profile standard` (= hardware) | `08 08 07 08 08` | — | Passed |

Byte-identical and CRC-identical: **the two machines already agree on that rule at every one of
the five alignments.** `q193.lua` reads NesHawk's own `$4016` stream of the same ROM and shows the
mechanism in the one alignment that gives 07 — `305328`, `305329`, fetch, idle, `305332 = 00` —
which is the *same shape* as Kirby's frame 40 and clocks the port **twice** there too. So the
difference cannot be the clock count, and what is left is the **value** the ninth clock returns.
`AccuracyCoin`'s `DMA + $4016 Read` and `Controller Clocking` are both `NOT RUN` on NesHawk's own
run (`coin-neshawk.txt`), so that gate does not fire either way.

### Where it is implemented, and why it is not a `NesHawkCompat` item

A socket being empty is a property of the **recorded machine**, not of the 2A03's timing: it is in
the movie file, it applies to both profiles, and `NesHawkCompat` is the wrong place for it. So:

| | |
|---|---|
| `zax-core-api` `ControllerPortType::Empty` | a third device, beside `Controller` and `Zapper` |
| `zax-nes` `InputPorts::read` | one arm: an empty socket's data bit is 0 and no shift register is clocked. The strobe is still settled, because the *other* port reloads on it |
| `zax-nes` `InputPorts::load_state` | accepts the new discriminant; the format and `STATE_VERSION` are unchanged |
| `zax-harness` `bk2.rs` | `check_sync_settings` now **returns** the two port types, with a note naming what it honoured |
| `zax-harness` `movie_file.rs` | `MovieFile::ports`; an `.fm2` states no socket, so two pads |
| `zax-harness` `movie_sync.rs`, `frame_trace.rs` | `ZaxnesMachine::plug_ports`, before the power-on |

Nothing else can reach `Empty`: a machine powers on with two pads and the application never offers
it, so every ROM of every suite runs the code it ran before.

### What it is worth

| | before | **with the item** |
|---|---|---|
| tree frames in step on **every** event (`kirby-jp-cyc0.txt`) | 3-**40** | 3-**73** |
| first difference of any kind | tree frame 40, an extra `$4016` pass | tree frame **74**, one `irq` at −3 |
| frames 2-1 100 whose `w$4016` count differs from BizHawk's (`q182g.txt`) | **7** (40, 114, 213, 393, 654, 674, 784) | **0** |
| `step1` identical to the byte | 219 / 400 | **344** |
| `step1b` identical to the byte | 342 / 701 | **646** |
| `step1c` identical to the byte | 459 / 8 901 | **472** |
| the parting frame | 9 353 | **9 353** |
| `golden-core` | `8d508070…` | **`8d508070…`** byte-identical |
| `test-roms --profile standard` / `neshawk` | 212/7 and 205/14 | **212/7 and 205/14** |
| `AccuracyCoin` standard / neshawk | 128/147 and 119/147 | **128/147 and 119/147** |
| *Faxanadu* USA / Rev 1 | 202 / 210 | **202 / 210** (both its ports are `ControllerNES`) |

The seven strobe-count frames going to **zero** is the sharpest figure here: every one of them was
this core taking passes NesHawk did not, and it is the mechanism lot NESHAWK-PROFILE-7 measured at
frame 55 and lot 8 at frame 40 and three lots chased as a DMA item. **The wall at 9 353 does not
move**, which is now the fourth item in a row of which that is true — what the lot buys is the
clock, and this one buys 33 frames of it.

### A warning about every ROM BizHawk has been asked a question about

`G:/3DSen/ext-tools/BizHawk-2.11.1/config.ini` has `"PreferredCores": { … "NES": "quickerNES" … }`.
A `.bk2` whose header says `NesHawk` forces the core, so **every movie measurement of every lot is
sound** — but a bare **ROM** run (`EmuHawk.exe --lua=q189.lua <rom>`) loads **QuickerNES**, and its
log line `[QuickerNES] Cart Mapper Info:` is the only sign. This lot runs ROMs with
`--config=config-neshawk.ini`, a copy of the owner's config with that one field changed, and the
`[QuickerNES]` line is then absent and the `BoardID` comes back in NesHawk's spelling. **Lot
NESHAWK-PROFILE-11's six-ROM gate on item 8d was taken with `q189.lua` and no such override**, so
it may have been QuickerNES's verdict and not NesHawk's; step 2 re-takes it.

### The instruments

| | |
|---|---|
| `q192.lua` | the whole bus stream of BizHawk frames 38-40 (tree 39-41), `q191.lua`'s format |
| `q193.lua` | every `$4016` access of `dma_4016_read` on NesHawk — two address-specific callbacks for 400 frames |
| `q189.lua` + `--config=config-neshawk.ini` | a ROM's verdict off the screen, on NesHawk and not QuickerNES |
| `--dmc-census`'s new `rport` event | every `$4016`/`$4017` read, the DMA's re-reads among them, with its value and cycle. Feature-gated like the other eight; **left out of the `--event-log` ledger on purpose**, because `bk2cyc1.lua` wrote the references and logs `$4016` writes alone |
| `rust/target-ev/tmp/strobes.py` | per-frame `w$4016` counts, BizHawk's against this core's. Scratch, not committed |

### [PROGRESS]

    [PROGRESS] NESHAWK-PROFILE-13 step 1/3 done: tree frame 40 is the movie's own `UnpluggedNES` port 2, and the clock is in step to frame 73

---
