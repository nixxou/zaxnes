# The Lua scripts of lot NESHAWK-PROFILE-9, in full

Both run as `EmuHawk.exe --movie=<bk2> --lua=<script> <rom>` on **BizHawk 2.11.1**, NesHawk, and
both write into `G:/3DSen/test-movies/bizhawk-runs/`. They live in `G:/3DSen/ext-tools/scripts/`,
which is outside this tree, so they are recorded here as well — a measurement nobody can repeat is
not a measurement.

Two calibrations they rest on, both measured and neither assumed:

* the two instruments' counters differ by **one**: `emu.totalexecutedcycles()` is a 1-based count of
  the cycles that have **run**, where `frame-trace`'s `CYC` is the 0-based index of the cycle about
  to run. So a steady residual of **−1** on an execute is the two machines **in step**, and the
  *parity* of a given physical cycle is opposite in the two counts. `q185.lua` was written to settle
  this at the power-on and could not — a `--lua` script is loaded after frame 0 — so what settles it
  is NesHawk's own OAM rule: `cpu_deadcounter = 2` when `apu.dmc.timer % 2 == 0`, a timer that
  powers on at 1 020 and reloads to even values only, which makes its parity the parity of the
  cycles elapsed;
* `onmemoryexecuteany` does **not** fire for an instruction an interrupt displaced (NesHawk
  re-fetches the opcode and runs `VOP_RESET`/`VOP_NMI` instead), where `frame-trace` prints the line
  and charges it no cycles. An alignment of the two execute streams has to skip those, one per
  frame.

## `q184.lua` — the whole bus stream of tree frames 11 to 17

The round-1 window: with items 12 and 1c on, the four marks hold a constant −1 to BizHawk frame 12
and the first visible symptom is tree frame 16's twelve controller strobes against NesHawk's eight.
About 0.7 MB a frame; the file is an intermediate (4.8 MB), read the window out of it and delete it.

```lua
-- THE WHOLE BUS STREAM OF TREE FRAMES 11 TO 17 (lot NESHAWK-PROFILE-9 step 3, round 1).
--
-- With item 12 (the reset lands on the frame's own PPU dot) and item 1c (an odd slip costs the
-- fetch one cycle less) both on, the four once-a-frame marks of the Kirby .bk2 hold a constant
-- residual of -1 from the power-on through the movie's reset to BIZHAWK FRAME 12, where the RTI
-- mark leaves it at +1; the first VISIBLE symptom is tree frame 16, where this core takes 12
-- controller strobes against NesHawk's 8 -- an extra pass of the input routine this core has and
-- NesHawk does not.
--
-- So this is q183c.lua's dump over the frames that straddle it: BizHawk frames 10 to 16, which are
-- tree frames 11 to 17. About 0.7 MB a frame. The file is an intermediate: read the window out of
-- it and delete it.
local FIRST = 10           -- BizHawk framecount; tree frame 11
local LAST = 16            -- tree frame 17

local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q184.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

while emu.framecount() < FIRST do emu.frameadvance() end

local function rd(addr, val) out:write(string.format("%d r %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function wr(addr, val) out:write(string.format("%d w %04X %02X\n", emu.totalexecutedcycles(), addr, val or 0)) end
local function ex(addr)      out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr)) end
event.onmemoryread(rd, nil, "q184_r", "System Bus")
event.onmemorywrite(wr, nil, "q184_w", "System Bus")
event.onmemoryexecuteany(ex, "q184_x", "System Bus")

while true do
  local f = emu.framecount()
  out:write(string.format("## frame %d start %d\n", f, emu.totalexecutedcycles()))
  emu.frameadvance()
  if f >= LAST then break end
end
out:write("## done\n")
out:close(); client.exit()
```

### What was read out of it, and how

Three readings, each a difference of two **BizHawk** numbers:

1. **the execute-to-execute alignment** against `frame-trace --out`'s own stream: the residual is a
   constant −1 for 18 instructions and leaves it at tree frame 11's `$C360 STA $4014`;
2. **every OAM transfer's cost**, as `(the next execute's cycle) − (the $4014 write's cycle) − 1`:
   514 at an even write cycle, 513 at an odd one, and +2 for a DMC fetch inside;
3. **the cycle-by-cycle shape** of one transfer, printing `IDLE` for a cycle carrying no bus access
   at all: the leading dead cycle, the 512 accesses with their reads on **even** cycles, and the one
   trailing dead cycle that is item 13's second half.

## `q185.lua` — where BizHawk's cycle counter starts (and why it could not answer)

Kept because the question it asks is the one the whole round turns on, and because the next lot
should not write it again: **a `--lua` script is loaded after frame 0 has already run**, so its
callbacks never see the power-on reset. Its output begins at 27 393, which is the start-up frame's
own end.

```lua
-- WHERE BIZHAWK'S CYCLE COUNTER STARTS (lot NESHAWK-PROFILE-9 step 3, round 1).
--
-- The whole of round 1 turns on one question: `emu.totalexecutedcycles()` inside a bus callback --
-- is it the number of cycles that have FINISHED (so the cycle being reported is number N+1, and a
-- steady residual of -1 against this core's own "cycles elapsed before this instruction" means the
-- two machines are physically IN STEP), or is it the index of the cycle being run (so -1 means
-- this core's program really is one cycle early)?
--
-- The answer is at the power-on: the 6502's reset sequence is seven cycles and reads the vector at
-- $FFFC and $FFFD on its sixth and seventh. So the FIRST thirty bus accesses of frame 0, with the
-- counter beside each, say where the counter's zero is -- and nothing has to be assumed about
-- Kirby, the mapper or the PPU to read it.
local out = io.open("G:/3DSen/test-movies/bizhawk-runs/q185.txt", "w")
client.SetSoundOn(false); client.speedmode(6400); emu.limitframerate(false)

local n = 0
local function note(kind)
  return function(addr, val)
    if n < 40 then
      n = n + 1
      out:write(string.format("%d %s %04X %02X\n", emu.totalexecutedcycles(), kind, addr, val or 0))
    end
  end
end
event.onmemoryread(note("r"), nil, "q185_r", "System Bus")
event.onmemorywrite(note("w"), nil, "q185_w", "System Bus")
event.onmemoryexecuteany(function(addr)
  if n < 40 then
    n = n + 1
    out:write(string.format("%d x %04X 00\n", emu.totalexecutedcycles(), addr))
  end
end, "q185_x", "System Bus")

out:write(string.format("## before the first frame, the counter reads %d\n", emu.totalexecutedcycles()))
emu.frameadvance()
out:write(string.format("## after frame %d the counter reads %d\n", emu.framecount(), emu.totalexecutedcycles()))
out:write("## done\n")
out:close(); client.exit()
```

## The references this lot measured against, and did not reproduce

Nothing new was taken from BizHawk beyond `q184.txt` and `q185.txt`: lot 8's `q183.txt` (the
four-mark ledger of frames 1-56) and lot 7's `q182g.txt` (the controller-strobe ledger of frames
1-1 100) answered everything else, which is `docs/review/NESHAWK-PROFILE.md`'s own rule — *a ledger
already taken is worth reading before a new one is taken*.
