# NESHAWK-REWRITE — a complete Rust port of BizHawk's NesHawk core

Lot brief: build `zax-neshawk`, a **faithful port** of BizHawk 2.11.1's NesHawk core, using
NesHawk's own source as the reference — its structure, its per-cycle loop, its state variable
names, its quirks. Not a clean-room design. BizHawk is MIT, which is compatible; 3dSen's sources
and decompilation remain strictly off limits and were not consulted.

Owner's priority: **speed**; port in large faithful chunks; test against BizHawk early and often.

## The reference, and how it is read

BizHawk at tag **2.11.1**, `github.com/TASEmulators/BizHawk`, MIT.

* `src/BizHawk.Emulation.Cores/CPUs/MOS 6502X/` — the CPU.
* `src/BizHawk.Emulation.Cores/Consoles/Nintendo/NES/` — the machine, PPU, APU, boards,
  controllers.

**Deviation from the brief's letter, with its reason.** The brief says to read the source through
`WebFetch`. `WebFetch` answers a prompt against the page using a small relay model, and on these
files it **truncates and fabricates**: asked for lines 1-200 of `Execute.cs` verbatim it returned
~95 lines, invented a `};` to close the `Microcode` array early, and reported the file as
"approximately 2,847 lines" when it is 3,325. For a line-by-line port of a 107 KB microcode table
that is not a usable instrument — a fabricated table entry is a silent wrong-opcode bug.

So the reference files are fetched with `curl` as single source files over HTTPS from
`raw.githubusercontent.com`, which is still *reading source files through the web* and is **not**
an archive download (the brief's actual prohibition). Integrity is checked rather than assumed:
every file's byte count is compared against the size `api.github.com/repos/.../contents/...`
reports for it. All 21 files matched exactly on the first fetch.

The mirror lives in the session scratchpad, **outside the repository**, and is never staged.

**Owner decision, 2026-11-12: a full local clone now exists and is THE reference from here on.**
`G:/3DSen/ext-tools/bizhawk-src` — BizHawk tag 2.11.1 (commit `bdddf4a`), sparse checkout of the
NES core, `CPUs/MOS 6502X`, `BizHawk.Emulation.Common`, `BizHawk.Client.Common/movie` and
`Assets/gamedb` (incl. `NesCarts.xml`, which NesHawk uses to pick *Kirby*'s `HVC-TKROM` board).
Read and grep it directly; no more curl or WebFetch. A reference map with file/line pointers to
every function the port needs is at `G:/3DSen/ext-tools/BIZHAWK-SRC-INDEX.md` — read it before
step 2. If a file you ported was fetched before the clone, it is the same tag; no re-port needed.

## Porting precision rules (owner, 2026-11-12 — "a single cycle divergence costs hours later")

* **Translate, do not redesign.** Keep NesHawk's structure, the **order of every statement inside
  a cycle** (`RunCpuOne`, `runppu`, `RunOneFirst`/`RunOneLast`, the microcode steps), its state
  variables and their names. No batching, no "equivalent" shortcut, no reordering for speed —
  speed comes later, and only behind a measured equivalence.
* **Keep C# semantics exactly**: integer widths and wrapping (`byte`/`ushort`/`int` casts,
  `(byte)PC + (sbyte)op` style arithmetic), signedness, pre/post increments, `&&`/`||`
  short-circuits that skip a side effect, integer division and `%` on negatives, the default
  values fields get at construction, `switch` fall-through order. When a line is not obviously
  equivalent in Rust, transcribe it literally and comment the C# line next to it.
* **Diff against BizHawk component by component, as soon as each one runs**, before building on
  it: the 6502 per instruction **and per cycle** (bus address, read/write, value) on `nestest`
  and a few `cpu_*` ROMs; then the whole machine on the *Kirby* `.bk2` frames 1-60 with a
  per-instruction bus stream (`q186/q187.lua` style, `event.onmemoryexecuteany` +
  `onmemoryread/write`, `emu.totalexecutedcycles()`) against the same stream from
  `zax-neshawk`, cycle for cycle, **zero divergence** before porting the next layer. The cycle
  ledger `bizhawk-runs/kirby-jp-cyc0.txt` (frames 2-1 201) is the second check.
* **Never fix a divergence with a tuning constant.** A divergence means a line was mistranslated:
  find the line.
* Record every divergence found and the line that caused it in this file (one line each), so a
  pattern of translation mistakes is visible.

## Licence obligations

* `rust/crates/zax-neshawk/LICENSE-BizHawk` — BizHawk's MIT copyright notice and licence text.
* The crate README states the provenance.
* Every ported file carries a header comment naming the C# file it comes from.

## Steps

- [x] 1. Crate `zax-neshawk` (new workspace member, safe Rust only) + the **6502** ported from
      `MOS 6502X` — microcode tables, the `Execute.cs` state machine, `RDY`/`rdy_freeze`,
      `branch_irq_hack`, interrupts, the unofficial opcodes, `NESSoftReset`. Passes `nestest`
      (log compare) and the `cpu_*`/`instr_*` ROMs NesHawk itself passes.
- [x] 2. The machine: `NES.Core.cs` (`RunCpuOne`, the OAM/DMC DMA blocks, `dmc_realign`,
      `cpu_deadcounter`, `FrameAdvance`, `HardReset`/`SoftReset`, power-on RAM fill, frame-done),
      `PPU*.cs` (per-dot `runppu`, `TickPPU_VBL`/`preVBL`, `NewDeadPPU`, `NMI_offset`,
      `idleSynch`, sprite evaluation, registers, open bus), `APU.cs` (all channels, `DMCUnit`
      with `RunOneFirst`/`RunOneLast`, `dmc_dma_countdown`, `set_lenctr_en`, the frame sequencer,
      `NESSoftReset`), controllers (standard pad, `UnpluggedNES`). Boards: NROM, MMC1, MMC3,
      UxROM, CNROM, AxROM first, then the other common ones.
      **Owner decision, 2026-11-12 (read before porting boards):** do NOT port every NesHawk
      board. Port faithfully only the boards that cover **about 80-95 % of the commercial
      library**: iNES 0 (NROM), 1 (MMC1/SxROM), 2 (UxROM), 3 (CNROM), 4 (MMC3/TxROM, incl. its
      IRQ exactly as NesHawk), 7 (AxROM), then 9/10 (MMC2/MMC4), 11 (Color Dreams), 66 (GxROM),
      71 (Camerica), 5 (MMC5), 69 (FME-7), 23/25 (VRC2/4) as time allows; exotic ones such as
      Namco 163 are out of scope. For any other mapper, **fall back on Zaxnes's own boards** from
      `zax-nes` through an adapter, accepting the loss of NesHawk timing fidelity; mark that case
      explicitly (capabilities/log/harness output: "board from Zaxnes, not NesHawk-exact").
- [x] 3. The `Engine` implementation (`EngineKind::NesHawk`, savestates tagged and refusing other
      engines), wired into `zax-harness` (`movie-sync`/`frame-trace`/`test-roms --engine
      neshawk`), the application's `emu.engine = "neshawk"`, and `.bk2` playback.
- [x] 4. Verification against BizHawk: the *Kirby* `.bk2` to the last frame (156 312) against the
      per-frame references and the cycle ledger; *Faxanadu*; AccuracyCoin and the blargg suites
      with the **same verdicts as NesHawk** (including the `$8B ANE` hang).
- [x] 5. 3D: the tile capture the renderer needs, as `zax-fceux` does; capture parity measured on
      the profiled games.
- [x] 6. Speed and final pass: bench, fmt, clippy `-D warnings`, `cargo test --release`,
      golden-core unchanged for the Zaxnes engine, and `docs/review/NESHAWK-REWRITE.md`.

## Log

### Step 1 — done

Environment: `rust/target-nhr` (added to `.gitignore` — it was not there), `CARGO_INCREMENTAL=0`,
`TMP`/`TEMP` under it. Disk at start: G: 27 GB free, C: 8 GB free.

**Result: `nestest.nes` matches its reference log on all 8 991 instructions** — `PC`, `A`, `X`,
`Y`, `SP` and the **cycle count** exactly, `P` with one bit excluded for the reason below. The
whole workspace builds, `clippy -D warnings` and `fmt` are clean.

Files: `src/cpu/mod.rs` (`MOS6502X.cs` + `IMOS6502XLink.cs`), `src/cpu/uop.rs` (the `Uop` enum,
the `VOP_*` constants, `MICROCODE`), `src/cpu/execute.rs` (the 252 micro-operation bodies and the
dispatch), `tests/nestest.rs`.

#### Two things the mechanical transliteration got wrong, and what caught them

The enum, the 264-row table and the dispatch were transliterated from the C# by script rather
than typed, which is both faster and safer — but the script had to be corrected twice, and both
corrections are worth recording because they are the failure mode of this approach:

1. **A commented-out micro-operation was read as a live one.** `VOP_RESET`'s row is
   `Uop.FetchDummy, /*Uop.FetchDummy,*/ Uop.FetchDummy` — the C# has one of the three *commented
   out*. A regex that treats the row as text picks up all three, giving a nine-cycle reset where
   NesHawk's is eight. Only that one row in the whole table is affected, which the fixed script
   reports. `the_reset_sequence_is_the_cs_eight_micro_operations` is a test for exactly this.
2. **`Uop`'s variant order is behaviour, not style.** `AtStart` and `AtInstructionStart` both test
   `Microcode[opcode][mi] >= Uop.End`, an *ordering* comparison on the enum discriminants. Sorting
   or regrouping the variants would silently change which micro-operations count as an instruction
   boundary — and the seven appended under the C#'s "More unofficial micro-ops" comment sit above
   `End`, so they count too, which is a quirk of where they were added. Twenty further variants
   are named by no row at all and are kept only to hold their discriminants.

A third check was mechanical rather than assumed: every micro-operation a table row names has a
dispatch arm (252 of them), so the C#'s `default: throw new InvalidOperationException()` is
unreachable — which matters because this crate may not panic.

#### The one column of nestest that is masked, and why it is not a cover-up

**Bit 4 of `P` — the B flag — is excluded from the comparison.** On a real 6502 bit 4 is not a
register bit at all; it exists only in the byte a `PHP` or an interrupt *pushes*. NesHawk models
it as real state, and `Execute.cs` is unambiguous: all four of its `FlagB` assignments are inside
push micro-operations (`PushP` and `PushP_BRK` set it, `PushP_IRQ` and `PushP_NMI` clear it) and
none put it back. So after a `PHP` NesHawk's `P` keeps bit 4 where silicon clears it. nestest's
**line 72** is the first place the two part, and the port keeps NesHawk's behaviour because that
is what this lot is for.

What makes this a convention difference rather than a hidden defect: the pushed bytes are still
checked exactly. nestest pulls the pushed `P` into `A` and operates on it, and `A` is compared
**unmasked** on every one of the 8 991 lines — the `A:7F` on the line after that `PHP` is the
pushed `0x7F` being verified byte for byte. `PC`, `X`, `Y`, `SP` and every cycle count are
compared exactly as well. A microcode or timing error cannot survive 8 991 instructions of that;
a one-bit flag convention can, and does.

#### Carried to step 2, not skipped

The brief also asks step 1 for the `cpu_*`/`instr_*` ROM verdicts. **Those cannot run yet**: every
blargg suite reports through PRG-RAM at `$6000` and waits on the PPU's warm-up before it starts,
so each needs a board, a PPU and an APU — step 2's work. They are run as soon as the machine
exists, against NesHawk's own verdicts, not against "all pass".

`zax-app` does not build in this worktree and did not before this lot: `demo_game.rs`
`include_bytes!`s `crates/zax-app/demo/FlappyZaxNes.nes` and `flappyzaxnes.3dn`, which are
untracked binary assets absent here. The rest of the workspace builds; step 3's wiring into the
application is written against the same tree and will need those assets to be compiled.

`Execute.cs` anatomy, which the Rust mirrors one-for-one:

| `Execute.cs` | what it is | Rust |
|---|---|---|
| 32-318 | `Microcode`, 264 entries (256 opcodes + 8 VOPs), each a `Uop[]` | `MICROCODE: [&[Uop]; 264]` |
| 364-480 | `enum Uop` | `enum Uop` |
| 510-518 | `VOP_Fetch1` = 256 … `VOP_NUM` = 264 | the same consts |
| 520-3035 | the uop bodies | one method each, same names |
| 3037+ | `ExecuteOneRetry`'s dispatch, `ExecuteOne` | `match` on `Uop` |

### Step 2 — the machine (part 1 of 2: everything but the second board set)

`zax-neshawk` is now a whole NES. 16 of 16 `instr_test-v5` ROMs report **Passed** (`$6000 = 0`,
the same verdict NesHawk gives them), and `nestest` still matches its reference log on all 8 991
instructions after the one structural change below.

Files, each naming its C# source in its header:

| Rust | C# |
|---|---|
| `src/nes/mod.rs` | `NES.Core.cs` (`RunCpuOne`, the DMA blocks, `FrameAdvance`, `HardReset`, the memory map, `ReadReg`/`WriteReg`, the joypad ports), `NES.CpuLink.cs` |
| `src/nes/load.rs` | `NES.cs`'s `Init` and `DetectRegion`, plus SHA-1 and MD5 (no dependency is allowed here) |
| `src/nes/apu.rs` | `APU.cs`'s `DMCUnit.Run`, `DMCUnit.Fetch`, `RunOneFirst`, `RunOneLast` — the members that reach the machine |
| `src/apu.rs` | the rest of `APU.cs`: the five channels, `set_lenctr_en`, the frame sequencer, the mixer |
| `src/ppu/mod.rs` | `PPU.cs` state, `PPUREGS`, `Reg_2000`, `Reg_2001`, `PpuOpenBusDecay`, `pipeline` |
| `src/ppu/run.rs` | `PPU.run.cs` + `PPU.cs`'s `runppu` |
| `src/ppu/regs.rs` | `PPU.regs.cs`'s `$2000-$2007` |
| `src/boards/*` | `NesBoardBase.cs`, `NROM`, `UxROM`, `CNROM`, `AxROM`, `SxROM` (MMC1, with SuROM/SXROM/SoROM), `MMC3.cs`, `TxROM` |
| `src/cart.rs` | `CartInfo.cs`, `NES.iNES.cs` |
| `src/gamedb.rs` | `BootGodDB.cs` (`NesCarts.xml`), `IdentifyFromGameDB`, `Database.cs`'s record format |
| `src/controllers.rs` | `NESControllers.cs`'s `StrobeInfo`, `NesDeck`, `ControllerNES`, `UnpluggedNES` |

#### How the C#'s mutual references are kept, and the three places the shape had to change

The C# has `NES`, `PPU`, `APU` and the board all pointing at each other. The rule here is: a
member that reaches the machine is `impl Nes`, in the module of the component it belongs to, with
the C# method's name and body. So `runppu`, every `TickPPU_*`, every `$2000-$2007` handler,
`DMCUnit.Run`/`Fetch` and `RunOneLast` are `impl Nes`; the components hold state and the members
that need nothing else. No statement moved across a cycle boundary to make that work.

Three deviations, each documented at its site:

1. **`address_bus` moved from the CPU to the machine**, reached by the seven micro-operations that
   drive it through a new `Mos6502xLink::set_address_bus`. It has to be one register: the CPU
   writes it, `NES.ReadMemory` overwrites it with the address being read, and the DMC DMA's halt
   cycles read it back. Rust cannot lend the machine to the CPU the machine owns, so the register
   lives on the machine side. Same register, same write order. `nestest` re-run after the change
   and still exact.
2. **`BoardEnv`**: a board reaches `NES.CIRAM` and `NES.DB` and nothing else, so those two travel
   in a struct built from disjoint fields of `Nes` at each call.
3. **The CPU is lifted out of `Nes` for the duration of one `execute_one`** (`mem::take`, put back
   after), which is what lets the link hold the rest of the machine. It costs three copies of a
   ~100-byte struct per CPU cycle; if step 6's bench shows that mattering the fix is
   `Option<Box<Mos6502x>>`, and it is a pure speed question with no behaviour in it.

#### Three C# details that a tidy-up would have silently changed

* `MMC3Board_Base.WritePpu` returns **before** `base.WritePpu` when the board has no VRAM, so on a
  CHR-ROM board (Kirby's) a PPU write below `$2000` does nothing at all; and when there *is* VRAM
  it reassigns `addr` to the mapped address, so the base writes the same mapped byte a second
  time. Both transcribed.
* `DMCUnit.Fetch` tests `(sample_address & 0x1F) == 0x4017`, which after the mask can never hold.
  It is a defect of the reference (it means `0x17`); kept, with `#[allow(clippy::bad_bit_mask)]`
  and a note, because "the same bug" is the requirement.
* `NoiseUnit.clock_env` tests `env_divider != 0` and then `env_divider == 0` in two **separate**
  ifs, so a divider that reaches zero is reloaded in the same call — unlike `PulseUnit.clock_env`,
  which uses `else if`. The two channels really do differ.

#### Why the game databases had to be ported too

Board choice is not a cosmetic matter for a timing port. *Faxanadu* is `NES-SGROM` with **no**
WRAM in NesCartDB where its iNES header implies the generic 8 KiB, and an MMC3 cart names its
revision chip there (`MMC3A`/`MMC3C`…), which changes `oldIrqType` and so the IRQ edge. So
`gamedb.rs` ports both `BootGodDB.cs`'s `NesCarts.xml` state machine and the BizHawk game
database's record format, read at load time from a `gamedb` directory
(`ZAXNES_BIZHAWK_GAMEDB` in the tests). Checked against the clone: Kirby (JP) is chosen as
`HVC-TKROM` with chip `MMC3C`, exactly as the index says, and Faxanadu as `NES-SGROM`.

Not ported, stated rather than faked: VS. System, FDS, NSF, UNIF, the light gun, the cheat
engine, the code-data logger, the memory domains, `Palettes.cs`.

### Step 3 — the port behind the harness, and the first verification against BizHawk

`movie-sync --engine neshawk` plays a `.bk2` on the port of the core that recorded it, with
`--ram-ref` holding the run against BizHawk's own per-frame work RAM byte for byte, and
`--neshawk-event-log` writing the **cycle ledger** in the very shape `scripts/bk2cyc*.lua` writes
on BizHawk, so the two can be diffed event for event. `--neshawk-gamedb` (or
`ZAXNES_BIZHAWK_GAMEDB`) points at BizHawk's `gamedb` directory, without which board detection
falls back to the iNES header.

#### Results

| what | against | result |
|---|---|---|
| *Hoshi no Kirby* `.bk2`, frames 1-400 | `kirby-jp-step1.txt` | **400 of 400 frames byte-identical** (2 KiB of work RAM) |
| the same, frames 1-10 000 | `kirby-jp-step1c.txt` | **8 901 of 8 901 compared frames byte-identical** |
| *Faxanadu* (USA) `.bk2`, all 83 218 frames | `fax-usa.txt` | **832 of 832 reference lines agree, to frame 83 200**, and every lag count agrees |

#### Divergence 1, and the mistranslated line

The first Kirby run parted from BizHawk at **frame 11** — the frame the game first writes `$4014`
and first enables the DMC. The RAM diff said what the cause was before any bisection: our stack
held the same byte sequence as BizHawk's **two addresses higher**, so a 2-byte push was missing.

The line was not in the DMA code at all. `NES.WriteReg($4014)` does `cpu.RDY = false`, and
`WriteReg` is reached from inside `cpu.ExecuteOne()` — but this port lifted the CPU **out** of the
machine for the duration of `execute_one` (`mem::take`, put back after) so that the link could
hold the rest of it. The write therefore landed on the *placeholder* CPU and was thrown away:
`RDY` stayed high, the 6502 went on executing straight through the 513-cycle sprite DMA, and the
frame was that much shorter. The same hole swallowed every bus-side read of
`cpu.TotalExecutedCycles` — `double_controller_read` and `double_2007_read`, both of which read 0
for ever — and made the cycle ledger's `A`/`X`/`Y`/`SP` columns garbage, which is what exposed it.

**The fix is structural, not a constant.** `Nes` is now `{ cpu: Mos6502x, b: Bus }`, and
`cpu.execute_one(&mut self.b)` borrows the two halves at once by destructuring, so the CPU is
**live** throughout the instruction and a `$4014` write reaches the real `RDY`. The four
registers the C# keeps on the CPU but the *machine* also drives — `address_bus`, `RDY`,
`rdy_freeze` and `TotalExecutedCycles` — live on `Bus` and the CPU reaches them through
`Mos6502xLink` (`rdy`, `set_rdy_freeze`, `rdy_freeze`, `add_executed_cycle`, `set_address_bus`).
`on_exec_fetch` gained the four register values for the same reason. Same registers, same write
order, same points; only their home differs. `Nes` derefs to `Bus`, so every `self.ppu`,
`self.apu`, `self.board` of the ported code still reads as the C# does.

`nestest` was re-run after the refactor and is still exact on all 8 991 instructions, and the 16
`instr_test-v5` ROMs still all report Passed.

**The lesson, recorded because it will recur:** a port that cannot keep a component *live* while
another component calls into it will silently drop the writes that cross back. The three
`&mut`-splitting devices this port uses — the `Bus` split, `BoardEnv`, and the link's four pin
accessors — are each there because a C# field is written from both sides, and each one had to be
found by a diff rather than by reading.

### Step 4 — verification against BizHawk: the result

**The port and BizHawk are the same machine on every reference this lot has.**

| what | reference | frames compared | result |
|---|---|---|---|
| *Hoshi no Kirby* `.bk2`, **the whole movie** | `kirby-jp-step1g.txt` | 56 313 (frames 100 000 - 156 312) | **all identical to the byte** |
| the same, frames 60 000 - 100 000 | `kirby-jp-step1f.txt` | 40 001 | all identical to the byte |
| the same, frames 30 000 - 60 000 | `kirby-jp-step1e.txt` | 30 001 | all identical to the byte |
| the same, frames 10 000 - 30 000 | `kirby-jp-step1d.txt` | 20 001 | all identical to the byte |
| the same, frames 1 100 - 10 000 | `kirby-jp-step1c.txt` | 8 901 | all identical to the byte |
| the same, frames 400 - 1 100 | `kirby-jp-step1b.txt` | 701 | all identical to the byte |
| the same, frames 1 - 400 | `kirby-jp-step1.txt` | 400 | all identical to the byte |
| *Faxanadu* (USA), all 83 218 frames | `fax-usa.txt` | 832 checksum lines | all agree, to frame 83 200; every lag count agrees |
| *Faxanadu* (USA Rev 1), all 83 218 frames | `fax-rev1.txt` | 832 checksum lines | all agree, to frame 83 200; every lag count agrees |
| **the cycle ledger**, Kirby frames 2 - 1 200 | `kirby-jp-cyc0.txt` | **34 026 event lines** | **every line identical, and every frame's `start=` CPU cycle identical** |

So the 2 KiB of work RAM agrees byte for byte on **every single frame of the 156 312-frame Kirby
movie, with no gap between the reference windows**, and on both Faxanadu revisions' whole movies.

The cycle ledger is the stronger statement: every `$4000-$401F` write, every `$2002` and `$4015`
read, every NMI and IRQ entry and every traced PC lands on the **same CPU cycle of the same
frame** with the **same `A`/`X`/`Y`/`SP`**, over 1 199 frames and 34 026 events. Two details of
the comparison worth stating so that nobody re-finds them:

* The `lag=` column differs by one everywhere, and is **not** a divergence: the Lua counts lag
  *before* running frame `f` and the port reports it *after*, so `ours(f) == bizhawk(f+1)`
  everywhere — checked on all 1 200 frames. The harness's own offset-aware lag comparison
  (`--trace-ref`) agrees on every line of both Faxanadu runs.
* Frame 1 201 is where `bk2cyc0.lua` stopped recording (0 events), not a difference.

Kirby's vectors, for whoever reads a ledger next: NMI `$C341`, RESET `$FFF0`, IRQ `$0029` — the
IRQ vector points into **work RAM**, a trampoline the game writes, which is why the ledgers tag
`$0029` as `irq`.

### Step 6 — the final pass (steps 3 and 5 are partly open; see the gap table)

| | result |
|---|---|
| speed, release, 600 frames after 60 of warm-up | SMB3 **594 f/s** (1.68 ms), Mega Man 2 **556 f/s** (1.80 ms), SMB **622 f/s** (1.61 ms). Mega Man **3** is not in `ZAXNES_ROMS`; Mega Man 2 (MMC1) stands in and SMB (NROM) is the third point |
| `golden-core`, 98 originals x 1 200 frames, Zaxnes engine | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical**. `zax-nes` was not touched by this lot |
| `cargo test --release --workspace` | green, less three **pre-existing** environment gaps (below) |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |

**The three environment gaps, each shown to be pre-existing rather than caused here.**
`zax-platform` and `zax-app` fail to *link* (`LNK1181: SDL2.lib`), which no change of this lot can
reach — neither crate is a dependency of anything this lot touched. `zax-harness`'s
`bench_core_tests` read `rust/test-roms/nestest.nes`, which `git ls-files` shows is **tracked by
nothing** and `.gitignore:102` ignores, so it is a per-worktree asset like `zax-app`'s two demo
files; it was copied in from `assets-free` (untracked, never staged) and the four tests then pass.

### What is left, and why — the honest list

The gap table of `docs/review/NESHAWK-REWRITE.md` §7 is the full statement. In short: step 3's
`Engine` implementation and application wiring are **not** done (the harness is, which is what
the verification needed), step 4's AccuracyCoin and screen-read suites are **not** scored, and
step 5 (the 3D tile capture) is **not** started. Each entry names what it would take; the two
that are cheapest and unblock the rest are a `SyncState` port (for savestates, which the `Engine`
trait needs) and making `test_roms`'s four readers generic over a five-method trait (which gets
AccuracyCoin and every screen-reading suite at once).

What this lot did deliver is the thing the lot exists for: **the port and BizHawk are the same
machine**, on 156 312 frames of work RAM, two whole Faxanadu movies, and 34 026 cycle-dated bus
events.

### Step 4, closed — `test-roms --engine neshawk`, AccuracyCoin, and a correction to the brief

`test_roms`'s four readers (`blargg`, `screen`, `coin`, `mapperel`) are now generic over a
five-method `TestMachine` (`src/test_roms/machine.rs`): `read_memory`, `run_frame`, `soft_reset`,
`power`, PPUCTRL's low two bits and a side-effect-free nametable-byte peek. Same readers on both
machines is the only way a verdict comparison means anything. `--engine zaxnes` is still
**212 pass / 7 fail**, its documented figure, so the refactor changed nothing for Zaxnes.

**`--engine neshawk`: 183 pass / 18 fail / 31 unsupported / 99 manual.** The 31 unsupported are
the unported boards. Of the 18 failures three also fail on Zaxnes, and the other 15 were each run
down — **none is a port defect**: ten are `holy-mapperel` MMC1 ROMs whose `$E000` bit 4 NesHawk
does not honour either (only `_is_snrom` does, and `SxROM.cs` says so), and five are DMC/DMA ROMs
that **NesHawk fails identically**, verified on BizHawk this lot down to the on-screen CRC.

#### AccuracyCoin: 147 of 147 identical — and `coin-neshawk.txt` is not a NesHawk run

The brief asks the port to reproduce NesHawk's AccuracyCoin result from
`bizhawk-runs/coin-neshawk.txt` (*"67/147, it hangs at `$8B ANE`"*). It does not, and it should
not: **that file was not taken on NesHawk.** `EmuHawk.exe` with a ROM and a Lua script and no
`--config` loads the NES on BizHawk's *default* core, which this installation reports as
`[QuickerNES] Cart Mapper Info` — the trap was hit in this session on the first attempt at
`7-dmc_basics` and is what sent this step looking. With
`--config=ext-tools/scripts/config-neshawk.ini` (`PreferredCores` → `"NES": "NesHawk"`), the same
`coin_bizhawk.lua` protocol and a larger frame cap:

| | pass | frames | `$8B ANE` |
|---|---|---|---|
| **NesHawk** (`nhr3-coin-neshawk.txt`, this lot) | **130/147** | 4 161, run-all completes | **PASS** |
| **the port** (`--coin-run-all`) | **130/147** | 4 166, run-all completes | PASS |
| the old `coin-neshawk.txt` | 67/147 | 100 001, never completed | NOT RUN |

**All 147 verdicts are identical** between the port and NesHawk. The old file disagrees with
NesHawk's own run on **80 of 147**. Anything else measured against it should be re-read.

Two instruments, and the difference matters: `coin::run` walks the menu and runs each test from a
freshly powered machine (that is what `test-roms` reports, 130/147 here too), while
`--coin-run-all` presses Start once and lets the ROM run itself, which is what the Lua does. A
figure from one is not comparable with a figure from the other.

New BizHawk references written this lot (new files only; nothing was deleted):
`nhr1-7-dmc-basics.txt`, `nhr1-apu-test.txt`, `nhr1-dma-4016-read.txt`,
`nhr1-sprdma-and-dmc-dma.txt`, `nhr1-sprdma-and-dmc-dma-512.txt`, `nhr2-dma-4016-read.txt`,
`nhr3-coin-neshawk.txt`, with their scripts `ext-tools/scripts/nhr1.lua`, `nhr2.lua`, `nhr3.lua`.
**Every future BizHawk run must pass `--config=scripts/config-neshawk.ini`.**

#### One instrument bug of this step's own, found and fixed

`run_one_neshawk` first skipped the harness's `region_of` rule, so the seven `pal_apu_tests` ROMs
ran as NTSC and failed. The harness's rule is that a ROM made for one console must be *run* on
that console; `_display_type` is read by `HardReset`, so the port now sets it and powers on again.
183 pass, up from 176.

### Resumption 2, step A — the `Engine` implementation, a real savestate, and the 3D capture

Three of the five open items in one pass, because the engine needs the other two to exist.

#### The savestate: `SyncState`, and one guarantee the C# has not got

`src/state.rs` is the port of BizHawk's `SyncState` members. The C#'s `Serializer` is a single
visitor that both reads and writes, so a class lists its fields once; this port cannot do that in
one method, because `Engine::save_state_to` takes `&self` and `try_load_state` a `&mut self`. So
`sync_struct!` generates **both halves from one field list** — the same token sequence, so the
write order and the read order cannot drift apart.

What the macro adds over the reference is worth recording: each generated body begins with an
**exhaustive** destructuring (`let Self { a, b, c } = self`, no `..`), so a field added to a struct
later and named in neither the saved list nor the `skip:` list is a **compile error**. A savestate
that silently forgets a new field is the classic emulator defect — a game comes back from a state
with the wrong banks mapped — and here it cannot happen. The `skip:` groups are the `&'static`
lookup tables, the ROM image, the load-time metadata and the tracing instruments, each with its
reason at the site. `Board::write_state`/`read_state` are **required** trait methods for the same
reason: a new board does not compile until it has answered the question.

The block carries the engine's magic (`ZXNH`), the version, the **board class** and the host's
region, and refuses a block of another engine with REQ-CORE-120's own message, a block of another
board class, and a payload whose length the machine does not read exactly.

**The frame buffer is in the block only when the host asks for a self-contained state.** A
rollback slot leaves it out, which is what REQ-CORE-121's own in-memory snapshot does ("leaves out
the half of the PPU state that *is* the frame just run") and is 120 KiB a slot.

#### The engine

`src/engine.rs`, `EngineKind::NesHawk = 2`, `Capabilities::NESHAWK`. Two deviations, both stated
at their site rather than papered over:

1. **`set_region` powers the machine on again** when the region really changes. `_display_type` is
   read by `HardReset` (it picks the APU's tables, the PPU's region and `cpu_sequence`), so
   NesHawk cannot change region without a reboot — in BizHawk it needs a core reboot too. The
   host's own sequence is `load_rom` then `set_region`, where there is no state to lose.
2. **Audio is resampled here, not by `blip_buf`.** `BlipBuffer.cs` is a `DllImport` shim over
   Blargg's native `blip_buf`, so **there is no C# to port**: the band-limited synthesis is in a C
   library this lot may not fetch. The deltas' *timing* is NesHawk's to the cycle, which is what
   the port is verified on; the reconstruction filter is a box filter over each output sample
   window, which has a zero at every multiple of the output rate. The waveform is therefore this
   engine's own and not bit-identical to BizHawk's.

One defect of the first draft, found by the test rather than by reading: the resampler only ran
when a frame had deltas, so a **silent** game produced no samples at all and would starve the
host's audio device. It now advances to the end of every frame whether or not the APU moved. The
step is `clock.wrapping_sub(last)` rather than a difference of absolute positions, because
`APU.sampleclock` is a `uint` the port never resets and so wraps after about forty minutes of
play — with absolute positions that moment would be a negative span and four billion samples of
silence.

#### The 3D tile capture (step 5's first half)

`src/tiles.rs`. **Not a port of anything**: NesHawk has no tile capture, so what is ported is the
machine the capture reads and the rule is the spec's (SPEC-02 §4.1, REQ-CORE-030 to 046), followed
as `zax_nes::ppu::tiles` implements it so that the two engines agree. Sprites at the first dot of
a visible line when `line & 7 == 7`; the background row at the **last** dot, when the fine Y of
the row-start shadow is 1 (AMD-021 R2-005). The shadow is `v` shorn of the two end-of-line
reloads, which meant mirroring six sites of NesHawk's dot loop: the `$2006` install, the `$2007`
auto-increment and the pre-render vertical reload reach both halves; the dot-256 vertical increment
and the dot-257 horizontal reload reach only `v_row_next`; a coarse-X increment reaches neither.

Every read goes through `Board::peek_ppu`, so a frame runs identically whether records are built or
not — asserted rather than assumed: `the_tile_capture_produces_records_and_changes_nothing`
compares 120 frames of work RAM, the frame buffer and the cycle count with the capture on and off.

#### The machine is still the same machine

| | result |
|---|---|
| `nestest`, after the capture hooks were cut into the PPU's dot loop | still exact on all 8 991 instructions |
| the 16 `instr_test-v5` ROMs | still all Passed |
| *Kirby* `.bk2` frames 1-400 against `kirby-jp-step1.txt` | **400 of 400 byte-identical** |
| `cargo test -p zax-neshawk --release` | 21 pass, 1 ignored (needs `ZAXNES_ROMS`) |
| `fmt`, `clippy -D warnings` on `zax-neshawk` and `zax-core-api` | clean |

### Resumption 2, step B — the Zaxnes-board fallback

The owner's decision of 2026-11-12, implemented: a mapper with no ported NesHawk board now runs
on **Zaxnes's** board through `boards::zaxnes::ZaxnesBoard`, marked "board from Zaxnes, not
NesHawk-exact" everywhere a user or a tool can see it, and **never a load error**. Only a mapper
*neither* emulator has is still refused (and there is exactly one file in the collection like
that, a headerless image with no database entry).

| | before | after |
|---|---|---|
| `test-roms --engine neshawk`, 331 ROMs | 183 pass / 18 fail / **31 unsupported** / 99 manual | **199 pass / 20 fail / 1 unsupported / 111 manual** |

The 31 unsupported were the unported boards. They are now 1, and **every one of the 20 failures is
accounted for without a port defect**:

* **12 `holy-mapperel` MMC1.** Ten report "$E000 bit 4 does not disable WRAM", which is NesHawk's
  own documented behaviour (`SxROM.cs` honours `wram_disable` only when `_is_snrom`). The other
  two are SNROM and **fail on Zaxnes as well** — checked this step, both engines run them and both
  refuse them, with different detail digits.
* **AccuracyCoin 130/147** — identical to NesHawk's own run, which is the result of record.
* **5 DMC/DMA ROMs** that NesHawk fails identically, verified on BizHawk in the earlier step.
* **2 mapper-4 variants** (`mmc3_test/6-MMC6`, `mmc3_test_2/6-MMC3_alt`) which were *unsupported*
  before: NesHawk routes them to board classes this port has not got, and Zaxnes's mapper-4 board
  does not implement MMC6 either. They are the fallback's honest limit, not a regression.

#### Three design points worth recording

1. **The cartridge is in a `RefCell`**, and that is load-bearing rather than clever. NesHawk's
   `INesBoard` read hooks are `&self` in this port (every ported board leaves no trace on a read)
   while `Cartridge::try_cpu_read` is `&mut` — the MMC5's `$5204` clears its IRQ flag when read.
   Without the cell those side effects would silently not happen, which is the worst kind of
   defect: a game that mostly works. Safe Rust, one borrow per call, never held across a call out.
2. **`Board::save_ram` became `save_ram_into(&self, &mut Vec<u8>)`.** A `Ref` cannot lend a slice
   out of a `&self`, and both callers copied the bytes immediately anyway.
3. **`BoardSystemHardReset` cannot rebuild this board from its class** — it holds a whole
   `zax-nes` cartridge built from the ROM image, which that method has not got. A new
   `Board::power_on_in_place` hook, which only this board overrides, resets it through
   `Cartridge::reset` instead.

The three things the fallback loses, stated in the module header rather than left to be found: the
IRQ counter's clock (a Zaxnes board may count A12 through its own `A12Watcher` or count
scanlines), the fetch kind on a PPU read (NesHawk's `ReadPPU` does not carry one, so the adapter
says `Background`), and four-screen nametables (the console's 2 KiB and the cartridge's view of
them are two memories here where in Zaxnes they are one).

*Kirby* frames 1-400 re-run after the load path changed: still **400 of 400 byte-identical**.

### Resumption 2, step C — the application, and the `.bk2` default engine

`emu.engine` is now a choice of **three** (`Zaxnes`, `FCEUX`, `NesHawk`), with the row shown in
**every** build: it was gated on the `fceux-core` feature because without it the row was a choice
of one, and `zax-neshawk` is safe Rust behind no feature at all, so that is no longer true.
`EngineHandle::of_kind` gained a `NesHawk` arm with no fallback (the crate keeps no
process-global state and so cannot refuse to be created the way FCEUX can) and the tile capture
**on**, since the records are what the 3D renderer reads. A load with no copy of BizHawk's
`gamedb` logs a line saying the board was chosen from the iNES header alone.

**The greying test is now "is the engine Zaxnes", not "is the engine FCEUX".** The widescreen
rows, the hack packs, the sprite limit and the run-ahead are greyed on NesHawk for exactly the
reason they are greyed on FCEUX — all four are patches to *Zaxnes's* bus or PPU, and
`Capabilities::NESHAWK` and `Capabilities::FCEUX` agree on every one of them. The four hint texts
were rewritten engine-neutrally in both languages ("this engine has no widescreen…"), their keys
kept so an older translation still resolves, and `detail.emu.engine` now describes all three
engines and says what NesHawk is *for*. A new `status.engine_neshawk` marker joins
`status.engine_fceux`, in English and French.

#### `.bk2` playback: what was asked, and what the application can actually do

The brief asks for "`.bk2` playback in the application defaulting to `neshawk` for movies whose
`Core` is `NesHawk`". **The application has no `.bk2` reader at all** — it opens `.fm2`, `.fcm`
and Zaxnes's own `.mv<n>`/`.zmv`, and the `.bk2` reader is the harness's. Adding one is a feature
lot, not a wiring change, so what this step delivered is the *rule* and its one live use:

* `EngineKind::for_bizhawk_core(core)` in `zax-core-api` — the single place the mapping lives, so
  that every reader agrees. `NesHawk` → the port; `QuickNes` (BizHawk's *default* core, which is
  why `coin-neshawk.txt` turned out not to be a NesHawk run) → `None`.
* `MovieFile::recorded_engine()`, the sibling of `recorded_profile()`: that one picks a timing
  chip for Zaxnes's core, this one picks the core.
* **`movie-sync`'s `--engine` now defaults to `neshawk` for a `.bk2` whose `Core` is NesHawk**,
  and to `both` for everything else. `--engine` always wins, and the choice is printed, so nobody
  reads a `neshawk` run as a `both` run. Verified on the Kirby `.bk2`: with no `--engine` at all
  it now prints the note and runs `neshawk`.

An application `.bk2` importer therefore has the rule waiting for it; the gap table says so.

#### One pre-existing break this step found and fixed

`cargo clippy -p zax-app --all-features` did not compile, and not because of this lot:
`ControllerPortType::Empty` was added by lot NESHAWK-PROFILE-13 and `zax-fceux`'s
`set_port_type` was never given its arm, so the **`fceux-core` configuration had not compiled
since**. Fixed by refusing `Empty` as it refuses `Zapper`, which is the right answer and not a
convenience: `Empty` is BizHawk's `UnpluggedNES`, whose read answers 0 for ever where a pad
answers 1 from the ninth read, and accepting it would hand back a pad while reporting a socket.

### Resumption 2, step D — the second board set

Six board classes ported from the C#, ~1 320 lines of reference: `GxROM` (66), `IC_74x377`
(11 and the Color Dreams/AGCI boards, incl. 144), `PxROM_FxROM` (MMC2 = 9 and MMC4 = 10),
`Camerica_Mapper071` and `Camerica_Mapper232`, `Sunsoft_FME7` (69), and `VRC2_4` (21, 22, 23, 25,
27, 116).

#### Verified against NesHawk itself, not against "it looks right"

Every ROM in the collection that reaches one of the new boards was put to **NesHawk running in
BizHawk** (`scripts/nhr4.lua`, `--config=config-neshawk.ini`) and the two screens compared through
**one** decoder, so that a verdict difference could not be a decoder difference. `nhr4.lua` dumps
all four nametable pages as raw bytes and the Python side applies the harness's own `Font::Ascii6`
rule and its own most-text page choice.

| ROM | NesHawk on BizHawk | the port | class picked |
|---|---|---|---|
| `M9_P128K_C64K` | `PNROM (MMC2)` / `0000` | identical | `PxROM_FxROM` |
| `M10_P128K_C64K_S8K` | `F*ROM (MMC4)` / `0000` | identical | `PxROM_FxROM` |
| `M10_P128K_C64K_W8K` | `F*ROM (MMC4)` / `0000` | identical | `PxROM_FxROM` |
| `M11_P64K_C64K_V` | `COLOR DREAMS` / `0000` | identical | `IC_74x377` |
| `M66_P64K_C16K_V` | `G*ROM` / `0000` | identical | `GxROM` |
| `M69_P128K_C64K_S8K` | `J*ROM (FME-7)` / `0000` | identical | `Sunsoft_FME7` |
| `M69_P128K_C64K_W8K` | `J*ROM (FME-7)` / `0000` | identical | `Sunsoft_FME7` |
| `m22chrbankingtest/0-127` | the 128-tile CHR grid, 16 lines | **character for character identical** | `VRC2_4` |

New references, new files only: `bizhawk-runs/nhr4-*.txt` (8), script
`ext-tools/scripts/nhr4.lua`.

**And against Zaxnes**: of the 40 `holy-mapperel` ROMs, the two engines now give the **same
verdict on 30 and differ on 10** — and all ten are the documented MMC1 `$E000 bit 4` cases, which
is NesHawk's own behaviour. Not one ROM of a *new* board disagrees with Zaxnes.

The campaign totals are unchanged at **199 pass / 20 fail / 1 unsupported / 111 manual**, which is
the point: these ROMs already passed on the Zaxnes fallback, and they pass on the ported boards
too. What changed is *which* board runs them — 9 cartridges still fall back, down from 18.

#### Three scope lines drawn on purpose

1. **MMC5 (mapper 5) is not ported**, and runs on the Zaxnes fallback. `ExROM.cs` is 799 lines and
   needs two things this port's `Board` trait cannot express: a **mutable** PPU-register read
   (`ReadReg2xxx`) and nametable bytes the *cartridge* serves (`ExRAM`, fill mode). The fallback
   already runs all five MMC5 ROMs in the collection and is marked, so the honest trade was the
   six boards that fit over the one that does not. It is the gap table's entry, not a silence.
2. **`SUNSOFT-5B` is refused by `Sunsoft_FME7`** and left to the fallback. The C#'s `Sunsoft_5` is
   this board plus the 5B's own sound chip; this port has no expansion audio at all, and a board
   that silently drops a third of Gimmick!'s music is worse than one marked "not NesHawk-exact".
3. **Mapper 71 (Camerica) is ported but unexercised**: the collection has no mapper-71 ROM and no
   mapper-71 game. Stated rather than claimed.

#### Four C# details a tidy-up would have changed

* **`IC_74x377`'s `bus_conflict` is `true` at construction**, not at `Configure`
  (`private bool bus_conflict = true;`), and `Configure` only ever *clears* it. A
  `derive(Default)` would start it `false` and give every Color Dreams cartridge the wrong write
  path — so the `Default` impl is written out, with the reason.
* **`PxROM_FxROM` does not serialise `prg_banks_8k`**: `SyncState` ends `if (ser.IsReader)
  SyncPRG();` and recomputes them from `prg_reg`. Transcribed, so a block of this board is the
  shape the reference's is.
* **`VRC2_4`'s `$9002` on a VRC2 falls through** (`goto case 0x1000`) and then runs `SyncPRG()`
  anyway, so a VRC2 write there sets the mirroring **and** re-syncs PRG. Both, in that order.
* **`latch6k_value` is in the block only when `latch6k_exists`**, which makes a VRC2 block's
  length depend on the board. Kept.
* Where the C# **throws** — an unknown PCB for a `KONAMI-VRC-2`/`VRC-4` — this port *refuses the
  board* instead, because this crate may not panic. The cartridge then falls through to the Zaxnes
  board with its marker, which is a better outcome than aborting the load and is visible.

One instrument bug of this step's own: `test-roms`'s new `board` detail key **collided** with
`mapperel.rs`'s, which writes the board name the ROM prints on its own screen. Renamed
`neshawk_board`.

*Kirby* frames 1-400 re-run after the registry changed: still **400 of 400 byte-identical**.

### Resumption 2, step E — `neshawk-tiles`, and the capture parity measured

`zax-harness neshawk-tiles` is lot FCEUX-6's `fceux-tiles` pointed at the NesHawk port, and
deliberately the **same measuring stick**: the engine-neutral half of that tool — the ten-field
mask, the multiset count, the OAM-rotation check, the row histograms, the readable record pairs —
moved into `src/tile_diff.rs`, which carries no feature gate, and `fceux_tiles` now uses it.
Nothing in it was redesigned in the move; lot FCEUX-6's arguments stay at their sites.

`compare_frame_full` was split so that two lists of the *shared* `TileRecord` go through the same
body: `compare_records` for this tool, `compare_frame_full` for FCEUX's own `Tile`, both two
`map`s and one `compare_keys`.

#### The class column, and why it is the one to read

`identical` (nothing differs) / `capture` (the two 256×240 pictures agree, so only the capture
disagrees — **ours to answer for**) / `emulation` (the pictures already differ, so the cores are
not showing the same frame) / `uniform` (Zaxnes's picture is one colour, so identical pictures
prove little).

#### The profiled games — what step 5 asks for

| | |
|---|---|
| 18 originals x frames 60, 300, 1200 | **44 of 54 frames identical on all ten fields** |
| `capture`-class frames | **0** |
| `emulation`-class frames | 10 — SMB3 (3), both Zelda IIs (3 each), Excitebike at 1200 (1) |

So **on every sampled frame where the two engines show the same picture, their records are
identical on all ten fields**. There is no frame in the set where the machines agree and the
capture does not.

The power-on fill was ruled out rather than assumed: NesHawk's `HardReset` fills work RAM with
FCEUX's `00 00 00 00 FF FF FF FF` where `zax-nes` fills **zeros**, and lot FCEUX-6 found that
difference alone worth six originals. So `NesHawkEngine::set_initial_wram_pattern` and
`--ram neshawk|zero` exist, the default is `zero` (matching `zax-nes`), and the measurement is
**the same with either** — these ten frames are not the power-on.

What they are: on SMB3 frame 60 the only differing field is **`y`**, on 192 background records of
the bottom seven rows, with `plane0`, `plane1`, `index` and `palette` matching exactly and NesHawk
emitting 32 records more — the mid-screen split landing a line or two apart because the MMC3 IRQ
fires at a different dot on the two cores. That is the two machines differing, which is the point
of the lot; the port's agreement with **NesHawk** is what Kirby, Faxanadu and the cycle ledger
establish.

#### The whole 330-ROM collection, for breadth

| class | frames |
|---|---|
| `identical` | **566** |
| `emulation` | 88 |
| `uniform` | 4 |
| **`capture`** | **2** |
| error | 1 (`other/LINUSMUS.NES`, which **Zaxnes** refuses — a headerless image) |

**Of the capture-class records, 2 048 of 2 050 are identical in order (99.90 %)**, and both frames
are the same ROM: `other/window_old_ntsc.nes`, where the port's capture emits **one extra
background record** on `y = 233`, the last row of the frame, with **no field differing on any
record** (`by_fields` is empty; `unmatched_neshawk` is 1).

The mechanism, named rather than patched: a background row carries 33 records instead of 32 only
when fine X is not 0 (REQ-CORE-038), and `window_old_ntsc` rewrites the scroll every scanline, so
the fine X the two cores hold **at the capture instant** differs by a cycle's worth of dot
alignment on the frame's last line. Making it agree would mean moving when the capture samples
fine X to suit one torture-test ROM, which is the tuning constant this lot's rules forbid. One
edge record in 2 050 on 1 ROM of 330 is reported instead.

One more fact the sweep turned up and is worth recording: `first_pixel_diff` is **2 for every one
of the 18 originals**, including the fourteen whose sampled frames are all identical. The two
engines part on frame 2 and come back together — the PPU warm-up (`ppudead`) differs — which is an
engine difference of long standing and not this lot's to fix.

### Resumption 2, step F — the final pass

| | result |
|---|---|
| `golden-core`, 98 originals x 1 200 frames, **Zaxnes** engine | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical**, and `git diff` over `crates/zax-nes` is empty |
| `cargo test --release --workspace` (less the two SDL2 crates) | **2 527 passed, 0 failed, 93 ignored** |
| the same with `fceux-core` on | **205 passed, 0 failed, 17 ignored** |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| `cargo check -p zax-app --all-targets`, and clippy `--lib --all-features` | clean |
| `cargo test -p zax-neshawk --release -- --include-ignored` | 26 passed, 0 failed |

**One trap worth recording so nobody re-finds it.** `golden-core` was first run as
`golden-core <originals> <out.jsonl>` and gave `8d5f0bd3…`, which looks like a regression and is
not one: the documented invocation is `golden-core "G:/3DSen/test-originals" <out.jsonl> **1200**`,
and the trailing frame count changes which frames are *sampled* into the digest. With the
documented arguments the digest is byte-identical. `zax-nes` and `golden_core.rs` were both
untouched by the lot, which is what said the first number had to be the instrument.

Speed, release, 600 frames after 60 of warm-up, measured **both ways round** because the
application's engine runs the 3D capture and the harness's tools do not:

| ROM | capture off | capture on |
|---|---|---|
| Super Mario Bros. 3 | 582 f/s | 571 f/s |
| Mega Man 2 | 547 f/s | 539 f/s |
| Super Mario Bros. | 602 f/s | 601 f/s |

About nine to ten times real time, and **the capture costs about 2 %**.

`docs/review/NESHAWK-REWRITE.md` rewritten: §3's file table covers the second board set, the
savestate, the engine and the capture; §4b is the fallback and the three things it loses; §4c is
the savestate and the exhaustive-destructuring guarantee; §7's gap table is replaced; §7b carries
the new suite figures; §8 lists the new tools; §9 is the final table.

### What is left after this session — the honest list

Everything the resumption asked for is done. What the gap table still names:

* **MMC5** is on the fallback, not ported (`ExROM.cs` needs a mutable PPU-register read and
  cartridge-served nametables, which `Board` cannot express today).
* **`SUNSOFT-5B`** is deliberately refused so Gimmick! keeps its expansion audio on Zaxnes's board.
* **Mapper 71** is ported but no ROM in the collection exercises it.
* **Audio** is this engine's own waveform: the delta *timing* is NesHawk's to the cycle, but
  `BlipBuffer.cs` is a shim over native `blip_buf`, so there is no C# to port.
* **A region change is a power cycle**, which is NesHawk's own constraint.
* **The application cannot open a `.bk2`** — the rule is delivered and used by `movie-sync`, but an
  importer in `zax-app` is a feature lot.
* **MMC2/MMC4's capture** reads the CHR latch as the frame left it rather than walking it per
  column.
* One capture-class difference in 330 ROMs: `window_old_ntsc.nes`, one extra edge record on the
  frame's last row, mechanism named in the review and not patched with a constant.
