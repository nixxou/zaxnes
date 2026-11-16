# The BizHawk Lua of lot NESHAWK-PROFILE-7

Five scripts, all in `G:/3DSen/ext-tools/scripts/`, all run as
`EmuHawk.exe --movie="<bk2>" --lua="<script>" "<rom>"` on BizHawk **2.11.1** with the `NesHawk`
core. Each is `q182c.lua` or `q182d.lua` with one idea changed; the texts are kept here because the
scripts live outside the repository.

## The alignment they all depend on, measured

* a block `## frame f` holds **this tree's frame f + 1** — BizHawk's `emu.framecount()` is the
  tree's frame number minus one. Pinned on the execute stream (core frame 55's NMI handler and
  block 54's are the same 9 050 instructions) and on the work RAM (`q182c.txt`'s last write to each
  address over blocks 45 083-45 089 matches `kirby-jp-step1e.txt` line 45 090 on 455 of 455);
* `emu.totalexecutedcycles()` and `frame-trace --event-log`'s cycle are the **same clock** to a few
  cycles: BizHawk's first frame is 27 392 cycles and this core's power-on plus start-up frame is
  27 394;
* a register **write** is reported one cycle apart between the two instruments, an **execute** on
  the same cycle; a DMA read's own calibration is **−1**, which the 1 524-fetch comparison measures
  rather than assumes.

## `q182d.lua` — the filtered wide window

`q182c.lua` dumps every System Bus access, which is 0.84 MB a frame. The DMC question wants two
kinds of access, so the callbacks stay registered for every address (BizHawk's
`event.onmemoryread(func, address, name, domain)` hooks **one** address, not a range) and the filter
is in the callback: every `$4000`-`$401F` access, and every **read** of `$FC00`-`$FEFF`, which for
this game is the DPCM sample region and nothing else. 66 frames cost 120 KB.

Window: blocks 45 030-45 095. Output `bizhawk-runs/q182d.txt`.

## `q182e.lua` — the DMC trigger ledger, frames 1 to 45 100

The one that made the lot possible. Five callbacks on **five named addresses** — `w $4010`,
`w $4012`, `w $4013`, `w $4015`, `r $4015` — so the 45 100-frame pre-roll costs only the accesses
wanted rather than one Lua call per bus access. That is the game's whole DPCM schedule from the
power-on, and it is what says the two machines' schedules first differ at frame **1002**. 2.8 MB,
4m 20s.

## `q182g.lua` — the controller-strobe ledger, frames 1 to 1 100

One callback on `w $4016`. Kirby's input routine re-reads the pad until two passes agree, and a DMC
fetch that falls on a `$4016` read makes a pass disagree with itself — so the **number of strobes in
a frame counts the passes**, and the number of passes counts the DMC/controller collisions.
`frame-trace --event-log --dmc-census` writes the same `w$4016` events on this side, so the two
tables are directly comparable. 7 s.

## `q182h.lua` — the whole bus stream at the first differing frame

`q182c.lua` with the window at blocks 52-59. The whole stream, `x` executes included, which is what
the execute-to-execute comparison of frame 55 needs: it is the only alignment that carries one
calibration rather than two. 5.9 MB, 10 s.

## `q182i.lua` — the fetch train from the power-on

`q182d.lua` with the window at blocks 1-60. 78 KB, and it holds every DMC sample fetch of the
movie's first 59 frames — 1 524 of them, which is the measurement that retired item 6.

## The two things to copy rather than rediscover

* **register one callback per address when you can.** `q182c`'s pre-roll to frame 45 000 is four
  billion Lua calls; `q182e`'s to the same frame is a few thousand. A window that looked impossible
  for two lots was a four-minute run.
* **the game's own re-read loop is an instrument.** Kirby strobes `$4016` eight times a frame and
  more when a pass is corrupted, so one hooked address counts every DMC/controller collision in the
  movie without any state having to be read out of either core.
