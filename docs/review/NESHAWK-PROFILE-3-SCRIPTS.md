# The instruments of lot NESHAWK-PROFILE-3, in full

Lot NESHAWK-PROFILE-3 found its item by **measuring the same events on both machines and aligning
them**, which took four scripts that live outside this tree: three Lua scripts BizHawk runs (in
`G:/3DSen/ext-tools/scripts/`, never in the worktree) and two Python analysers. Their text is here
so that the lot's figures can be reproduced without them having to be rewritten, and because a
measurement whose instrument is not written down is not a measurement.

What is **not** here: their outputs (references, ledgers, traces and screenshots, which are
movies and pictures of a commercial game), and the screenshot comparison, which reads those
pictures — `docs/review/MOVIE-SYNC.md` §4's commands block describes the colour-bijection rule it
implements, as it has since EVAL-SYNC-3.

| | |
|---|---|
| `cyccheck.lua` | **validation**, and it was run first: the memory-domain list, every register name, the two interrupt vectors, and all three callback kinds over four frames. Nothing in the lot rests on a guessed API name |
| `bk2cyc1.lua` | the **cycle ledger**, in the format `zax-harness frame-trace --event-log` writes |
| `bk2trace1d.lua` | the **second per-frame reference**, frames 10 000 to 30 000, in `kirby-jp-step1c.txt`'s own format |
| `phaserom.lua` | reads the **interrupt-phase ROM**'s two tables back out of work RAM |
| `romresult.lua` | a blargg test ROM's own verdict, for asking NesHawk what it does with `cpu_interrupts_v2` and `ppu_vbl_nmi` |
| `align.py`, `drift.py` | the two analysers: intervals between events, and each event's residual against its frame's own anchor |

**How NesHawk is selected.** BizHawk 2.11.1's `config.ini` names `quickerNES` as the preferred NES
core, and a `.bk2` whose `Core` header is `NesHawk` overrides it — so every movie run is NesHawk
without anything having to be done. A **plain ROM** is not: `5-branch_delays_irq` booted on
QuickerNES until the core was forced, which is why the test-ROM runs pass
`--config=<a copy of config.ini with `"NES": "NesHawk"`>`. The copy lives beside the scripts.

---

## `cyccheck.lua`

```lua
-- Validation only (lot NESHAWK-PROFILE-3 step 1): checks the names and the behaviour of every
-- BizHawk Lua call the cycle ledger is about to be built on, over four frames, and then exits.
-- Nothing here is a measurement; it exists so that the measurement is not built on a guess.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"
local out = io.open(dir .. "cyccheck.txt", "w")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)

out:write("romhash ", gameinfo.getromhash(), "\n")

out:write("-- memory domains\n")
for i, d in ipairs(memory.getmemorydomainlist()) do out:write("   ", i, " ", d, "\n") end

out:write("-- registers\n")
for name, value in pairs(emu.getregisters()) do
  out:write("   ", tostring(name), " = ", tostring(value), "\n")
end

out:write("-- vectors\n")
local nmi = memory.read_u16_le(0xFFFA, "System Bus")
local irq = memory.read_u16_le(0xFFFE, "System Bus")
out:write(string.format("   nmi=%04X irq=%04X\n", nmi, irq))

local base, cur, armed = 0, 0, false
local function ev(tag, detail)
  if not armed then return end
  out:write(string.format("  %d %6d %-8s %s\n", cur, emu.totalexecutedcycles() - base,
    tag, detail or ""))
end

-- The three hook kinds, each on something Kirby is known to do every frame.
local ids = {}
ids[#ids + 1] = event.onmemoryexecute(function() ev("nmi") end, nmi, "nmi", "System Bus")
ids[#ids + 1] = event.onmemoryexecute(function() ev("pc$C37C") end, 0xC37C, "c37c", "System Bus")
ids[#ids + 1] = event.onmemorywrite(function(a, v) ev(string.format("w$%04X", a),
  string.format("=%02X", v)) end, 0x4017, "w4017", "System Bus")
ids[#ids + 1] = event.onmemorywrite(function(a, v) ev(string.format("w$%04X", a),
  string.format("=%02X", v)) end, 0x4016, "w4016", "System Bus")
ids[#ids + 1] = event.onmemoryread(function(a, v) ev("r$4015",
  string.format("=%02X", v)) end, 0x4015, "r4015", "System Bus")

while true do
  local f = emu.framecount()
  if f >= 100 then
    armed = true
    cur = f + 1
    base = emu.totalexecutedcycles()
    out:write(string.format("F %d start=%d\n", cur, base))
  end
  if f >= 104 then
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```

## `bk2cyc1.lua`

```lua
-- The CYCLE LEDGER of a window of frames, in the format `zax-harness frame-trace --event-log`
-- writes (lot NESHAWK-PROFILE-3 step 1), so that the two machines can be aligned EVENT FOR EVENT
-- instead of only frame for frame.
--
--   F <frame> start=<emu.totalexecutedcycles() at the frame's start>
--     <frame> <cycles since that start> <tag> <detail>
--
-- `start` is absolute and the two machines' counters have no reason to agree; what is comparable
-- is (a) the difference between consecutive frames' NMI entries, which is the frame's length as
-- the PROGRAM sees it, and (b) every event's cycle measured from the frame's own NMI entry.
-- Measuring from the NMI and not from `start` is deliberate: BizHawk ends a frame on a PPU
-- condition and Zaxnes's tracer notices the boundary only after the instruction that crossed it,
-- so the two `start`s are a few cycles apart by construction while the NMI entry is an
-- instruction boundary on both.
--
-- Every call used here was validated first by `cyccheck.lua`: the domain is "System Bus", the
-- register names are A/X/Y/S (there is no combined P), and a callback's
-- `emu.totalexecutedcycles()` is the cycle of the access that fired it.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"

-- The window and the output file. These four lines are the whole configuration.
local first = 1100
local last = 10000
local name = "kirby-jp-cyc1.txt"
-- Kirby's frame-sync and music-driver points. Replace with whatever a bisection needs: every
-- PC here costs one `event.onmemoryexecute`, and a PC inside a spin loop costs a line a visit.
local watched_pc = { 0xC37C, 0xC06E, 0x80FC, 0x804C }

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

-- The two interrupt vectors, which is how a handler entry is recognised on both machines: the
-- first instruction executed at the vector's own target.
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

-- Every register of the 2A03's own page: $4014 is the OAM request, $4016 the controller strobe,
-- the rest the APU. A write callback per address rather than one for all writes, which would
-- fire on every store the game makes.
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

## `bk2trace1d.lua`

```lua
-- Per-frame reference for frames 10 000 to 30 000 of the loaded movie, in exactly the format of
-- `kirby-jp-step1c.txt` (lot NESHAWK-PROFILE-3 step 3): the checksum, the lag count and the raw
-- 2 KiB of work RAM at the start of every frame, plus a screenshot every 2 000 frames.
--
-- `bk2trace1c.lua` with the window moved and the screenshot interval widened; the checksum is the
-- same `s = s * 31 + b` over $0000-$07FF the earlier references use, so `movie-sync --ram-ref`
-- and `--trace-ref` read this file without being told anything new. About 4.2 KiB a line.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"
local first, last = 10000, 30000
local out = io.open(dir .. "kirby-jp-step1d.txt", "w")
out:write("romhash ", gameinfo.getromhash(), "\n")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)
local lagcount = 0
while true do
  local f = emu.framecount()
  if f > 0 and emu.islagged() then lagcount = lagcount + 1 end
  if f >= first then
    local s, hex = 0, {}
    for i = 0, 0x7FF do
      local b = memory.read_u8(i, "RAM")
      s = (s * 31 + b) % 4294967296
      hex[#hex + 1] = string.format("%02x", b)
    end
    out:write(f, " ", string.format("%08x", s), " lag=", lagcount, " ram=", table.concat(hex), "\n")
    if f % 2000 == 0 then client.screenshot(dir .. "kirby-jp-d" .. f .. ".png") end
  end
  if f >= last then
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```

## `phaserom.lua`

```lua
-- Reads the interrupt-phase ROM's two tables (lot NESHAWK-PROFILE-3 step 2): it waits for the
-- ROM's own finished flag at $02FF and dumps $0300-$033F (the taken branch does NOT cross a page)
-- and $0400-$043F (it does). Each byte is the low byte of the PC the vblank NMI displaced, so the
-- table says at which instruction boundary of a `LDA zp` / `BEQ` spin loop each machine takes it.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"
local out = io.open(dir .. "phaserom.txt", "w")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)

out:write("romhash ", gameinfo.getromhash(), "\n")
out:write("domains:")
for _, d in ipairs(memory.getmemorydomainlist()) do out:write(" ", d) end
out:write("\n")

local function dump(name, base)
  local row = {}
  for i = 0, 0x3F do
    row[#row + 1] = string.format("%02X", memory.read_u8(base + i, "System Bus"))
  end
  out:write(name, ": ", table.concat(row, " "), "\n")
end

while true do
  if memory.read_u8(0x02FF, "System Bus") == 0x5A then
    out:write("finished at frame ", emu.framecount(), "\n")
    dump("A (no page cross)", 0x0300)
    dump("B (page cross)   ", 0x0400)
    out:close()
    client.exit()
  end
  if emu.framecount() >= 400 then
    out:write("DID NOT FINISH at frame ", emu.framecount(), "\n")
    dump("A (no page cross)", 0x0300)
    dump("B (page cross)   ", 0x0400)
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```

## `romresult.lua`

```lua
-- Runs the loaded ROM for a while and writes out a blargg test ROM's own verdict: the status
-- byte at $6000, the three-byte signature at $6001-$6003 and the zero-terminated text at $6004.
-- Lot NESHAWK-PROFILE-3 step 2 uses it to ask NesHawk what it does with
-- `cpu_interrupts_v2/5-branch_delays_irq`, whose source states the hardware rule in words.
local dir = "G:/3DSen/test-movies/bizhawk-runs/"
local out = io.open(dir .. "romresult.txt", "w")
client.SetSoundOn(false)
client.speedmode(6400)
emu.limitframerate(false)

out:write("romhash ", gameinfo.getromhash(), "\n")

local function report()
  local status = memory.read_u8(0x6000, "System Bus")
  local sig = string.format("%02X %02X %02X", memory.read_u8(0x6001, "System Bus"),
    memory.read_u8(0x6002, "System Bus"), memory.read_u8(0x6003, "System Bus"))
  local text = {}
  for i = 0x6004, 0x60FF do
    local b = memory.read_u8(i, "System Bus")
    if b == 0 then break end
    text[#text + 1] = string.char(b)
  end
  out:write(string.format("frame %d status=%02X sig=%s\n", emu.framecount(), status, sig))
  out:write(table.concat(text), "\n")
end

while true do
  local f = emu.framecount()
  -- $6000 holds $80 while the test runs and its result once it has finished.
  if f > 60 and memory.read_u8(0x6000, "System Bus") ~= 0x80 then
    report()
    out:close()
    client.exit()
  end
  if f >= 1200 then
    out:write("DID NOT FINISH\n")
    report()
    out:close()
    client.exit()
  end
  emu.frameadvance()
end
```

## `align.py`

```python
"""Align two cycle ledgers (BizHawk's Lua and zax-harness frame-trace --event-log) event for
event, and name the first INTERVAL whose length disagrees.

What is comparable between two machines is not an event's offset -- the frame's start is quantised
differently on the two instruments and the NMI entry is quantised by the instruction that was in
flight -- but the CPU CYCLES BETWEEN TWO PROGRAM POINTS.  So every consecutive pair of events in a
frame is one measurement, and a mismatch names the stretch the cycles were lost in.

The one calibration: the two instruments report a WRITE at a different point of its cycle than
they report an EXECUTE.  Measured, not assumed (see --bias and the `same kind` rows, which must
come out exactly 0).
"""
import sys
from collections import defaultdict


def read(path):
    frames = {}
    cur = None
    for line in open(path, encoding="utf-8", errors="replace"):
        if line.startswith("F "):
            parts = line.split()
            cur = {"start": int(parts[2].split("=")[1]), "ev": [],
                   "hdr": dict(p.split("=", 1) for p in parts[2:] if "=" in p)}
            frames[int(parts[1])] = cur
        elif line.startswith("  ") and cur is not None:
            parts = line.split()
            if len(parts) < 3:
                continue
            try:
                cur["ev"].append((int(parts[1]), parts[2]))
            except ValueError:
                pass
    return frames


def kind(tag):
    """How the two instruments report this event's cycle, relative to an EXECUTE.

    Measured, not assumed: with these three values every `w->r$4015` interval of frames
    1100-1200 comes out at exactly 0 on 75 of 75 occurrences for `r$4015->w$4011` and 56 of 57
    for `w$400C->r$4015`, which no wrong calibration could do.
    """
    if tag.startswith("r$") or tag.startswith("w$") or tag == "oam":
        return 1
    return 0


def main(bizhawk, zaxnes, first, last, bias):
    bh, zx = read(bizhawk), read(zaxnes)
    common = sorted(f for f in bh if f in zx and first <= f <= last)
    print(f"frames in both ledgers, {first}-{last}: {len(common)}")

    # The accumulated frame length, which telescopes so that the two instruments' different
    # quantisation of a frame boundary only touches the two ends of the span.
    a, b = common[0], common[-1]
    print(f"cycles from the start of frame {a} to the start of frame {b}: "
          f"bizhawk {bh[b]['start'] - bh[a]['start']}, "
          f"zaxnes {zx[b]['start'] - zx[a]['start']}")

    per_pair = defaultdict(lambda: defaultdict(int))
    bad = []
    seq = []
    for f in common:
        ta = [t for _, t in bh[f]["ev"]]
        tb = [t for _, t in zx[f]["ev"]]
        if ta != tb:
            seq.append((f, ta, tb))
            continue
        rows = []
        for i in range(len(ta) - 1):
            (ca, _), (cb, _) = bh[f]["ev"][i], bh[f]["ev"][i + 1]
            (da, _), (db, _) = zx[f]["ev"][i], zx[f]["ev"][i + 1]
            shift = bias * (kind(ta[i + 1]) - kind(ta[i]))
            delta = (db - da - shift) - (cb - ca)
            per_pair[f"{ta[i]}->{ta[i + 1]}"][delta] += 1
            if delta:
                rows.append((f"{ta[i]}->{ta[i + 1]}", cb - ca, db - da, delta))
        if rows:
            bad.append((f, rows))
    print(f"frames whose event SEQUENCE differs: {len(seq)}")
    for f, ta, tb in seq[:6]:
        print(f"   frame {f}:\n      bizhawk {ta}\n      zaxnes  {tb}")
    print(f"frames with at least one interval differing: {len(bad)} of {len(common)}")
    if bad:
        print(f"the first: {bad[0][0]}")

    print("\nper interval, the distribution of (zaxnes - bizhawk):")
    for pair in sorted(per_pair, key=lambda p: -sum(per_pair[p].values())):
        spread = sorted(per_pair[pair].items(), key=lambda kv: -kv[1])
        total = sum(per_pair[pair].values())
        clean = per_pair[pair].get(0, 0)
        shown = " ".join(f"{d:+}x{n}" for d, n in spread[:7])
        print(f"   {pair:<22} {total:>7} of which {clean:>7} agree   {shown}")

    print("\nthe first twelve frames with a differing interval:")
    for f, rows in bad[:12]:
        print(f"   frame {f}: " + "; ".join(
            f"{p} bh={x} zx={y} ({d:+})" for p, x, y, d in rows))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]),
         int(sys.argv[5]) if len(sys.argv) > 5 else 1)
```

## `drift.py`

```python
"""The one measurement lot NESHAWK-PROFILE-3 step 1 is after: the first EVENT whose cycle differs.

The two ledgers' absolute cycle counters differ by a constant as long as neither machine has
gained or lost a cycle, so the whole comparison is one number per event:

    D(event) = zaxnes_absolute - bizhawk_absolute - bias(kind)

`bias` is the instruments' own calibration (an execute, a write and a read are reported at
different points of their cycle: 0, 1 and 2).  The ANCHOR is each frame's NMI handler entry, which
is an instruction boundary on both machines and is not quantised by anything; a frame's own events
are then held against that frame's anchor, so a frame that starts in step and ends out of step
says so, and the drift of the anchor itself says whether a whole frame was gained or lost.
"""
import sys
from collections import Counter, defaultdict

import align


def bias(tag):
    return align.kind(tag)


def absolutes(frames):
    """Each frame's events as (absolute cycle, tag), in CORRECTED cycle order.

    The correction matters for the order and not only for the arithmetic: an execute and a write
    on adjacent cycles are reported one cycle apart by one instrument and the other way round by
    the other, so an uncorrected sort makes two identical runs look like two different event
    sequences.
    """
    out = {}
    for f, fr in frames.items():
        rows = [(fr["start"] + off, tag) for off, tag in fr["ev"]]
        rows.sort(key=lambda row: row[0] - bias(row[1]))
        out[f] = rows
    return out


def main(bizhawk, zaxnes, first, last):
    bh, zx = absolutes(align.read(bizhawk)), absolutes(align.read(zaxnes))
    common = sorted(f for f in bh if f in zx and first <= f <= last)

    def anchor(rows):
        for c, tag in rows:
            if tag == "nmi":
                return c
        return None

    # 1. The anchor's own drift: a frame whose NMI entry moves is a frame one machine ran longer.
    drift = {}
    for f in common:
        a, b = anchor(bh[f]), anchor(zx[f])
        if a is not None and b is not None:
            drift[f] = b - a
    counts = Counter(drift.values())
    print(f"frames compared: {len(common)}; frames with an NMI entry on both: {len(drift)}")
    print("the drift of the NMI entry (zaxnes - bizhawk), which is constant while neither "
          "machine gains a cycle:")
    for value, n in counts.most_common(10):
        print(f"   {value}  on {n} frame(s)")
    base = counts.most_common(1)[0][0]
    moved = [f for f in sorted(drift) if drift[f] != base]
    print(f"frames whose NMI entry is NOT at the common drift of {base}: {len(moved)}")
    if moved:
        print("   the first twenty: " + " ".join(
            f"{f}({drift[f] - base:+})" for f in moved[:20]))

    # 2. Inside each frame, against that frame's own anchor.
    per_tag = defaultdict(Counter)
    bad = []
    for f in common:
        if f not in drift:
            continue
        ta = [t for _, t in bh[f]]
        tb = [t for _, t in zx[f]]
        if ta != tb:
            bad.append((f, "the event SEQUENCES differ", len([t for t in ta if t != "r$2002"]),
                        len([t for t in tb if t != "r$2002"])))
            continue
        rows = []
        for (ca, tag), (cb, _) in zip(bh[f], zx[f]):
            d = cb - ca - bias(tag) - drift[f]
            per_tag[tag][d] += 1
            if d:
                rows.append((tag, ca, cb, d))
        if rows:
            bad.append((f, rows, None, None))
    print(f"\nframes with at least one event off its frame's anchor: {len(bad)} of {len(common)}")
    if bad:
        print(f"the first: frame {bad[0][0]}")
    print("\nper tag, the residual (0 = the same cycle on both machines):")
    for tag in sorted(per_tag, key=lambda t: -sum(per_tag[t].values())):
        total = sum(per_tag[tag].values())
        clean = per_tag[tag][0]
        shown = " ".join(f"{d:+}x{n}" for d, n in per_tag[tag].most_common(7))
        print(f"   {tag:<9} {total:>7} of which {clean:>7} at 0   {shown}")
    print("\nthe first fifteen frames with a residual:")
    for row in bad[:15]:
        f, what = row[0], row[1]
        if isinstance(what, str):
            print(f"   frame {f}: {what} ({row[2]} vs {row[3]} non-$2002 events)")
        else:
            print(f"   frame {f}: " + "; ".join(
                f"{t}@{a} ({d:+})" for t, a, _, d in what[:8]))


if __name__ == "__main__":
    main(sys.argv[1], sys.argv[2], int(sys.argv[3]), int(sys.argv[4]))
```

