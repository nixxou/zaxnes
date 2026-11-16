# The BizHawk Lua of lot NESHAWK-PROFILE-8

Four scripts, all in `G:/3DSen/ext-tools/scripts/`, all run as
`EmuHawk.exe --movie="<bk2>" --lua="<script>" "<rom>"` on BizHawk **2.11.1** with the `NesHawk`
core. The texts are kept here because the scripts live outside the repository.

## The alignment they depend on, which this lot re-pinned

* a block `## frame f` holds **this tree's frame f + 1**, as lots 6 and 7 measured;
* both instruments date an **execute** at the instruction's own start cycle, so the four marks of
  `q183.lua` compare against `frame-trace --event-log`'s with **no offset applied**. This lot's
  frames 3 and 4 read a residual of exactly **-1** on all four marks and frames 7 to 11 exactly
  **+2** on all four, which no wrong convention could do;
* `q183c.lua`'s slip table needs **no** cross-machine calibration at all: the stall is a difference
  of two BizHawk cycles (an instruction's duration minus the shortest duration that PC is ever seen
  to take), and the writes that passed through the DMA are the cycles carrying a `w` **and** an `r`.

## `q183.lua` — the four-mark ledger of frames 1 to 56

```lua
-- WHERE THE CPU PHASE IS BORN: the four once-a-frame cycle marks of frames 1 to 56
-- (lot NESHAWK-PROFILE-8 step 1).
--
-- NESHAWK-PROFILE-7 found the movie's first difference at tree frame 55, where a DMC halt lands on
-- the game's `LDA $4016,X` read on NesHawk and not here, and said the frame's NMI entry is already
-- FIVE cycles apart -- so the collision is not born in frame 55, it is born in whichever earlier
-- frame first charges the two machines a different amount of work. This script is that ledger.
--
-- Kirby's frame structure, read off `frame-trace --frame 55 --out` on the Zaxnes side:
--
--   $C341  the NMI handler's first instruction (the vector's own target)
--   $C3A1  its `RTI`
--   $C067  `JSR $C067` from the main thread's $E5BC: PHA, `LDA $35`, `BPL $C076`, `LDA #$00`,
--   $C06E    `STA $39` -- the LAST INSTRUCTION BEFORE THE SPIN LOOP, once a frame
--   $C070  `LDA $39` / $C072 `BEQ $C070` -- the 6-cycle spin loop the next NMI breaks
--   $C074  `PLA` -- the spin loop's exit, once a frame
--
-- $C06E and $C074 are each executed EXACTLY ONCE a frame, which is what makes them cheap: four
-- `event.onmemoryexecute` registrations on four named addresses, no `onmemoryexecuteany`.
--
-- What each mark measures:
--   * $C06E - $C341  the work the NMI handler plus the frame's main thread cost;
--   * $C341 of frame f+1 - $C074 of frame f  how long the spin loop span;
--   * $C341 itself     the phase the previous frame's total work left the loop in.
--
-- Block numbering, as q182d/q182e measured and q182h used: `## frame f` holds THIS TREE'S FRAME
-- f + 1, so this window is BizHawk framecount 0 to 55.
local LAST = 55            -- BizHawk framecount; tree frame 56

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q183.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

local function mark(tag)
  return function()
    out:write(string.format("%d %s\n", emu.totalexecutedcycles(), tag))
  end
end

local nmi = memory.read_u16_le(0xFFFA, "System Bus")
out:write(string.format("## nmivector %04X\n", nmi))
event.onmemoryexecute(mark("nmi"), nmi, "q183_nmi", "System Bus")
event.onmemoryexecute(mark("rti"), 0xC3A1, "q183_rti", "System Bus")
event.onmemoryexecute(mark("c06e"), 0xC06E, "q183_c06e", "System Bus")
event.onmemoryexecute(mark("c074"), 0xC074, "q183_c074", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

## `q183b.lua` — the whole bus stream of tree frames 11 and 12

```lua
-- THE WHOLE BUS STREAM OF TREE FRAMES 11 AND 12 (lot NESHAWK-PROFILE-8 step 1).
--
-- `q183.lua`'s four-mark ledger of frames 1 to 56 says the CPU phase residual (this core's
-- absolute cycle minus NesHawk's, on an execute, which both instruments date at the instruction's
-- start) is a CONSTANT +2 from the movie's frame-4 reset through tree frame 11's `$C074`, and
-- leaves it at tree frame 12's NMI entry, which is +1. One cycle, and frame 11 is the first frame
-- of the movie with any DMC sample fetch (58) or OAM transfer (1) at all -- frames 1 to 10 have
-- neither.
--
-- So the whole stream of frames 11 and 12, aligned execute to execute against
-- `frame-trace --frame 11 --frames 2 --out`, names the instruction the cycle is lost on.
--
-- Registered late, as q182h does, so the pre-roll costs no Lua call; the window is small enough
-- that `onmemoryexecuteany` plus every read and write is affordable (about 1.7 MB a frame).
local FIRST = 10           -- BizHawk framecount; tree frame 11
local LAST = 11            -- tree frame 12

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q183b.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

while emu.framecount() < FIRST do emu.frameadvance() end

local function rd(addr, val) out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function wr(addr, val) out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function ex(addr)      out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end
event.onmemoryread(rd, nil, "q183b_r", "System Bus")
event.onmemorywrite(wr, nil, "q183b_w", "System Bus")
event.onmemoryexecuteany(ex, "q183b_x", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

## `q183c.lua` — the same over tree frames 11 to 46, for the 705-fetch slip table

```lua
-- THE WHOLE BUS STREAM OF TREE FRAMES 11 TO 46 (lot NESHAWK-PROFILE-8 step 1).
--
-- `q183b.lua`'s two-frame window named the cycle the CPU phase is born on: a DMC sample fetch
-- whose HALT lands on a cycle the CPU is WRITING on costs NesHawk three cycles where this core
-- charges four, and four of frame 11's 69 fetches do it. Six instances are not a rule, so this is
-- the same dump over 36 frames -- about 2 500 fetches -- so that the cost can be tabulated against
-- the halt's position inside the instruction from BIZHAWK'S OWN STREAM ALONE: the instruction's
-- nominal length is the shortest duration that PC is ever seen to take in the window, so the stall
-- is a difference of two BizHawk numbers and needs no cross-machine calibration at all.
--
-- About 0.7 MB a frame. The file is an intermediate: extract the table and delete it.
local FIRST = 10           -- BizHawk framecount; tree frame 11
local LAST = 45            -- tree frame 46

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q183c.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

while emu.framecount() < FIRST do emu.frameadvance() end

local function rd(addr, val) out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function wr(addr, val) out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function ex(addr)      out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end
event.onmemoryread(rd, nil, "q183c_r", "System Bus")
event.onmemorywrite(wr, nil, "q183c_w", "System Bus")
event.onmemoryexecuteany(ex, "q183c_x", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

## `q183d.lua` — the same across the movie's reset, tree frames 3 to 8

```lua
-- THE WHOLE BUS STREAM ACROSS THE MOVIE'S RESET: tree frames 3 to 8 (lot NESHAWK-PROFILE-8 step 1).
--
-- `q183.lua`'s four-mark ledger says the execute residual (this core's absolute cycle minus
-- NesHawk's) is **-1** on every mark of tree frames 3 and 4 and **+2** on every mark of frames 7 to
-- 11 -- so the movie's reset, which the `.bk2`'s Reset column puts after frame 4, costs this core
-- THREE CPU cycles more than it costs NesHawk, and that is the whole CPU-against-DMC phase the
-- frame-55 collision needs (the DMC fetch train's own residual is -1, measured over 1 524 fetches).
--
-- Frames 1 to 10 make no DMC fetch and no OAM transfer at all, so nothing else can move the
-- residual in this window: whatever the stream shows is the reset.
local FIRST = 2            -- BizHawk framecount; tree frame 3
local LAST = 7             -- tree frame 8

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q183d.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

while emu.framecount() < FIRST do emu.frameadvance() end

local function rd(addr, val) out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function wr(addr, val) out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function ex(addr)      out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end
event.onmemoryread(rd, nil, "q183d_r", "System Bus")
event.onmemorywrite(wr, nil, "q183d_w", "System Bus")
event.onmemoryexecuteany(ex, "q183d_x", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

