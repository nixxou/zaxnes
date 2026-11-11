# The instruments of lot NESHAWK-PROFILE-5, in full

Lot NESHAWK-PROFILE-5 measured **both machines from the power-on** and then measured the DMC's
sample fetches one byte at a time. That took two Lua scripts, both of them small edits of lot
NESHAWK-PROFILE-3's `bk2cyc1.lua`, and **no new Python**: the two analysers `align.py` and
`drift.py` are `docs/review/NESHAWK-PROFILE-3-SCRIPTS.md`'s, regenerated from that document into
the session's scratchpad and run unchanged.

The scripts live in `G:/3DSen/ext-tools/scripts/` and never in the worktree; their outputs
(ledgers of a commercial game's execution) live in `G:/3DSen/test-movies/bizhawk-runs/` and are
never committed. Their text is here so the lot's figures can be reproduced without them being
rewritten.

| | |
|---|---|
| `bk2cyc2.lua` | the **cycle ledger of the boot**: frames 1 to 1 100, the boot's and the frame-sync loop's PCs, and `$FFFC` watched as a read so the reset sequence measures itself |
| `bk2dmc1.lua` | the **fetch ledger**: frames 10 to 14 with one `event.onmemoryread` per byte of the first sample's first page, so that each firing *is* one DMC fetch with its own cycle |

**The limit that shaped both, said plainly: BizHawk's Lua cannot see frame 1.** `--lua` is loaded
after the first frame has run, so the ledger's first `F` line is frame 2 and the power-on dead
frame's own events are not in it. That is why the 6502's reset sequence was measured at the movie's
**soft** reset (frame 5), which is inside the window, and why the power-on is read from the source
and checked against the one number frame 1 does leave behind — its length.

**What neither script can answer, and it is this lot's open question.** Both report a DMA read at
whatever cycle `event.onmemoryread` fires on, and **what cycle that is for a read performed by a
DMA unit rather than by the CPU is not published**. Lot NESHAWK-PROFILE-3's calibration — an
execute at 0, a read or a write at 1 — was measured on **CPU** accesses only. `Q-182` records what
turns on it and names the one further run that would settle it: for a single known reload fetch,
both its own callback cycle **and** the cycle the CPU's stalled read executes on.

**How NesHawk is selected** is `docs/review/NESHAWK-PROFILE-3-SCRIPTS.md`'s note, unchanged: a
`.bk2` whose `Core` header says `NesHawk` overrides `config.ini`'s preferred core, so every movie
run here is NesHawk without anything having to be done.

---

## `bk2cyc2.lua`

```lua
-- The CYCLE LEDGER OF THE BOOT: frames 1 to 1100 of the loaded movie, in the format
-- `zax-harness frame-trace --event-log` writes (lot NESHAWK-PROFILE-5 step 1).
--
-- `bk2cyc1.lua` with three changes, and they are the whole of this instrument:
--   * the window starts at frame **1**, so the power-on dead frame is in it. `armed` is set on the
--     first pass of the loop, before any `emu.frameadvance()`, which is why frame 1's own events
--     are recorded;
--   * the watched PCs are the boot's and the frame-sync loop's rather than the music driver's:
--     `$FFF0` is the reset vector's own target (`SEI / LDA #$00 / STA $8000 / JMP $C000`, read out
--     of the ROM's `$FFFC`), `$C000` the game's entry, `$C06E` the `STA $39` that ends the main
--     loop's frame, `$C074` the `PLA` the frame-sync spin loop falls through to, and `$C3A1` the
--     NMI handler's `RTI`. The loop's interior (`$C070`, `$C072`) is deliberately NOT watched: it
--     is a 6-cycle loop spun for thousands of cycles a frame, so it would cost millions of lines,
--     and `$C074`'s cycle against the frame's NMI entry says which instruction the `RTI` returned
--     to just as exactly;
--   * `$FFFC` is watched as a READ, which is the reset sequence's own vector fetch: its cycle
--     against the first instruction at `$FFF0` measures the length of the tail of that sequence on
--     this machine without anything having to be assumed about the microcode.
--
-- Every call used here was validated by `cyccheck.lua` in lot NESHAWK-PROFILE-3: the domain is
-- "System Bus", and a callback's `emu.totalexecutedcycles()` is the cycle of the access.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"

local first = 1
local last = 1100
local name = "kirby-jp-cyc7.txt"
local watched_pc = { 0xFFF0, 0xC000, 0xC06E, 0xC074, 0xC3A1 }

local out = io.open(dir .. name, "w")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)
out:write("romhash ", gameinfo.getromhash(), "\n")

local base, cur, armed = 0, 0, false
local lagcount = 0

local function ev(tag, detail)
  if not armed then return end
  out:write(string.format("  %d %6d %-8s %s\n", cur, emu.totalexecutedcycles() - base,
    tag, detail or ""))
end

local function regs()
  return string.format("A:%02X X:%02X Y:%02X SP:%02X", emu.getregister("A"),
    emu.getregister("X"), emu.getregister("Y"), emu.getregister("S"))
end

-- The two interrupt vectors, read once at the start; the reset vector's target is a watched PC
-- rather than a vector read because the ROM's `$FFFC` never changes bank.
local nmi = memory.read_u16_le(0xFFFA, "System Bus")
local irq = memory.read_u16_le(0xFFFE, "System Bus")
event.onmemoryexecute(function() ev("nmi", regs()) end, nmi, "nmi", "System Bus")
if irq ~= nmi then
  event.onmemoryexecute(function() ev("irq", regs()) end, irq, "irq", "System Bus")
end
for _, pc in ipairs(watched_pc) do
  event.onmemoryexecute(function() ev(string.format("pc$%04X", pc), regs()) end,
    pc, string.format("pc%04X", pc), "System Bus")
end
event.onmemoryread(function() ev("r$FFFC", regs()) end, 0xFFFC, "rfffc", "System Bus")

-- The 2A03's own page, as `bk2cyc1.lua` has it: $4014 is the OAM request, $4016 the strobe.
for a = 0x4000, 0x4017 do
  event.onmemorywrite(function(addr, value)
    if a == 0x4014 then
      ev("oam", string.format("=%02X", value))
    else
      ev(string.format("w$%04X", a), string.format("=%02X", value))
    end
  end, a, string.format("w%04X", a), "System Bus")
end
event.onmemoryread(function(_, value) ev("r$4015", string.format("=%02X", value)) end,
  0x4015, "r4015", "System Bus")
event.onmemoryread(function(_, value) ev("r$2002", string.format("=%02X", value)) end,
  0x2002, "r2002", "System Bus")

while true do
  local f = emu.framecount()
  if f > 0 and emu.islagged() then lagcount = lagcount + 1 end
  if f >= first - 1 then
    armed = true
    cur = f + 1
    base = emu.totalexecutedcycles()
    out:write(string.format("F %d start=%d lag=%d\n", cur, base, lagcount))
  end
  if f >= last then
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```

---

## `bk2dmc1.lua`

```lua
-- The DMC SAMPLE FETCHES of frames 10 to 14, as cycles (lot NESHAWK-PROFILE-5 step 1).
--
-- `bk2cyc2.lua` cut down to the one question frame 11 asks: the *Kirby* `.bk2`'s first DPCM sample
-- starts in that frame (`$4012=F3`, `$4013=1C`, `$4010=0F`, then `$4015=1F`), and the cycle ledger
-- says the two machines part by six cycles between that `$4015` write and the next APU write, with
-- seven of this core's DMC fetches in between. A fetch is a DMA READ of the sample byte, so the
-- read callback on the sample's own addresses is the fetch's own cycle on this machine, with
-- nothing assumed about the cost model.
--
-- `$4012=F3` puts the sample at `$C000 + 0xF3 * 64 = $FCC0`, so the first 64 bytes are
-- `$FCC0`-`$FCFF` and one callback per address covers more fetches than frame 11 has. Those
-- addresses are sample data in the fixed bank: nothing executes or reads them but the DMC.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"

local first = 10
local last = 14
local out = io.open(dir .. "kirby-jp-dmc1.txt", "w")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)
out:write("romhash ", gameinfo.getromhash(), "\n")

local base, cur, armed = 0, 0, false

local function ev(tag, detail)
  if not armed then return end
  out:write(string.format("  %d %6d %-8s %s\n", cur, emu.totalexecutedcycles() - base,
    tag, detail or ""))
end

local nmi = memory.read_u16_le(0xFFFA, "System Bus")
event.onmemoryexecute(function() ev("nmi") end, nmi, "nmi", "System Bus")
for _, pc in ipairs({ 0xC06E, 0xC074, 0xC3A1 }) do
  event.onmemoryexecute(function() ev(string.format("pc$%04X", pc)) end,
    pc, string.format("pc%04X", pc), "System Bus")
end

-- The APU page, so that the stretch the ledger named can be bracketed on this machine too.
for a = 0x4000, 0x4017 do
  event.onmemorywrite(function(_, value)
    ev(a == 0x4014 and "oam" or string.format("w$%04X", a), string.format("=%02X", value))
  end, a, string.format("w%04X", a), "System Bus")
end
event.onmemoryread(function(_, value) ev("r$4015", string.format("=%02X", value)) end,
  0x4015, "r4015", "System Bus")

-- The sample's own bytes: one callback each, and each firing is one DMC fetch.
for a = 0xFCC0, 0xFCFF do
  event.onmemoryread(function(_, value)
    ev("dmc", string.format("$%04X=%02X", a, value))
  end, a, string.format("d%04X", a), "System Bus")
end

while true do
  local f = emu.framecount()
  if f >= first - 1 then
    armed = true
    cur = f + 1
    base = emu.totalexecutedcycles()
    out:write(string.format("F %d start=%d\n", cur, base))
  end
  if f >= last then
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```
