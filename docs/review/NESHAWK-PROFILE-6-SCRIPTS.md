# The instruments of lot NESHAWK-PROFILE-6, in full

Three BizHawk Lua scripts and one census column. The scripts live in
`G:/3DSen/ext-tools/scripts/` on the owner's machine and are **not** in this repository (nothing
outside it is), so their text is here: a measurement nobody can reproduce is not a measurement.
They are run as

    EmuHawk.exe --movie="<movie.bk2>" --lua="<script.lua>" "<rom.nes>"

and every call they use was validated by `cyccheck.lua` in lot NESHAWK-PROFILE-3: the domain is
`"System Bus"`, and a callback's `emu.totalexecutedcycles()` is the cycle of the access.

## Why there are two shapes, and when to use which

| | |
|---|---|
| `q182b.lua` | callbacks registered **once, before the loop**, gated on a boolean. One short Lua call per access for every frame before the window, which is free enough to reach frame **309** (4m17s) and hopeless past it |
| `q182c.lua` | callbacks registered **late**, after advancing to the window with nothing registered. The only shape that can reach frame **45 000** (3m13s); use this one unless the window is in the first few hundred frames |
| `bk2cyc3.lua` | not a bus stream but the **cycle ledger** — `bk2cyc2.lua` with the window moved. Its callbacks are on named addresses only (the two vectors, five PCs, `$4000`-`$4017`), so registering them once costs about fifty Lua calls a frame and reaching frame 45 000 takes 5m17s |

**Q-182 was answered by `q182b.lua` and nothing else.** The question was which cycle a *DMA* read is
reported on, and the answer was to stop needing it: with the whole stream in hand, the disputed
fetch can be bracketed by two **CPU** accesses and the stall read off as a difference of two
execute reports, which share one calibration whatever it is.

## `q182b.lua` — the complete bus stream of chosen frames

```lua
-- Q-182, generalised (lot NESHAWK-PROFILE-6 step 1).
--
-- q182.lua logged every System Bus access for 14 cycles after ONE $4007 write, which settled one
-- fetch. This logs the COMPLETE access stream (reads, writes and execute-flagged fetches, each
-- with the cycle BizHawk reports it on) for a whole list of frames, so that every DMC fetch of
-- those frames can be read off it: the halt run, the sample read, and -- the quantity that needs
-- no calibration -- the opcode-to-opcode gap of the instruction the fetch stalled.
--
-- The frames below are chosen from Zaxnes's own census of the same movie (`frame-trace
-- --dmc-census --profile neshawk`, item 11 on): frame 11 carries reload fetches at a slip of 0, 1
-- and 2, and frame 310 the only clean slip of 3 in frames 10-2 000. BizHawk's framecount is this
-- tree's frame number minus one (start-of-frame numbering), so the list is in BizHawk's.
--
-- Callbacks are registered once and gated on a boolean rather than registered per frame: a
-- not-armed callback is one Lua call, and the run only has to reach frame 309.
local WANT = { [10] = true, [11] = true, [309] = true }
local LAST = 309

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q182b.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

local armed = false
local function rd(addr, val) if armed then out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end end
local function wr(addr, val) if armed then out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end end
local function ex(addr)      if armed then out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end end

event.onmemoryread(rd, nil, "q182b_r", "System Bus")
event.onmemorywrite(wr, nil, "q182b_w", "System Bus")
event.onmemoryexecuteany(ex, "q182b_x", "System Bus")

while true do
  local f = emu.framecount()
  armed = WANT[f] == true
  if armed then out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles())) end
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

Output `bizhawk-runs/q182b.txt`, one line per access (`<cycle> <r|w|x> <addr> <value>`) with a
`## frame <n> start <cycle>` marker, 2.0 MB for three frames. The frame list is chosen from this
core's own census and not guessed — `frame-trace --dmc-census --profile neshawk` over frames
10-2 000 gives the reload slips as 0 (34 275), 1 (2 114), 2 (416) and 3 (2), and frame 11 alone
carries 0, 1 and 2.

## `q182c.lua` — the same, for a window the pre-roll cannot be paid for

```lua
-- `q182b.lua` with the callbacks registered LATE (lot NESHAWK-PROFILE-6 step 2 round 1).
--
-- q182b's callbacks are registered before the loop, which is free enough to reach frame 309 and
-- hopeless to reach frame 45 000: every memory access of every frame before the window costs a Lua
-- call. This advances to the window with nothing registered, registers there, dumps, and exits.
--
-- The window is the movie's first live difference against `kirby-jp-step1e.txt`, frame 45 096, and
-- the frames before it in which the DPCM sample of frame 45 085 starts -- BizHawk's framecount is
-- this tree's frame number minus one.
local FIRST = 45083          -- BizHawk framecount to start dumping at
local LAST = 45096

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q182c.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

while emu.framecount() < FIRST do emu.frameadvance() end

local function rd(addr, val) out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function wr(addr, val) out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function ex(addr)      out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end
event.onmemoryread(rd, nil, "q182c_r", "System Bus")
event.onmemorywrite(wr, nil, "q182c_w", "System Bus")
event.onmemoryexecuteany(ex, "q182c_x", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

## `bk2cyc3.lua` — the cycle ledger, window moved

`bk2cyc2.lua` (printed in full in `NESHAWK-PROFILE-5-SCRIPTS.md`) with three lines changed:

```lua
local first = 45080
local last = 45110
local name = "kirby-jp-cyc8.txt"
```

and the note at its head saying why the window is there. Everything else — the watched PCs, the
`$40xx` writes, the `$4015`/`$2002` reads, the `F <frame> start= lag=` header — is unchanged, which
is the point: the format is a contract with `frame-trace --event-log` and
`frame_trace.rs`'s `the_cycle_ledger_writes_the_format_the_other_machine_is_read_in` pins it.

## The census column

`frame-trace --dmc-census` (feature `dma-census`) prints one extra group on a **fetch** line:

    17545     fetch     5001 133:117  $FCDD 73       -    3  (50,8,420)  pc$C564 gap=8

`pc$XXXX` is the PC of the instruction the fetch happened inside and `gap` its opcode-to-opcode
length, cost included. `gap - cost` is the instruction's undisturbed length, which both machines
must agree on, so

    stall_neshawk = gap_bizhawk - (gap - cost)

is what NesHawk charged the same fetch — with no opcode cycle table and no calibration of the cycle
a DMA read is reported on. It is a fetch line alone because only a fetch carries the `cost` that
comparison subtracts.

## Reading the stream: what a NesHawk reload looks like

    301501  w 4007 26      the CPU's write; this core puts the DMC request on this cycle
    301502  x/r 81FB 28    PLP opcode fetch
    301503  r 81FC E8      PLP cycle 2
    301504  r 81FC E8      the DMA re-reads the address the CPU had on the bus
    301505  r 81FC E8
    301506  r FCC8 73      the sample fetch
    301507  (nothing)      one more cycle with the CPU stopped
    301508  r 01E3 85      PLP cycle 3
    301510  x/r 81FC E8    the next opcode

Two re-reads, the fetch, and a fourth cycle on which **nothing is reported at all** — so a reload
costs four, and `PLP` takes eight cycles instead of four. Three things in that window are worth
knowing before reading another one:

* an opcode fetch is reported **twice**, as `x` and as `r`, on the same cycle;
* a DMA re-read of the address the CPU was about to fetch from is reported as `r` **only** — the
  `x` flag tells a real opcode fetch from the halt's re-read of the same address;
* a write and the DMA's re-read **share a cycle** when the DMA lets a write microstep through
  (`310144 w 01F2 C5` and `310144 r 01F2 C5` on a `JSR`'s first push), which is what `RunCpuOne`'s
  `+2` is paying for.
