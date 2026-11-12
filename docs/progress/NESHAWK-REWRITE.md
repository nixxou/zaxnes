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
- [~] 2. The machine: `NES.Core.cs` (`RunCpuOne`, the OAM/DMC DMA blocks, `dmc_realign`,
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
- [ ] 3. The `Engine` implementation (`EngineKind::NesHawk`, savestates tagged and refusing other
      engines), wired into `zax-harness` (`movie-sync`/`frame-trace`/`test-roms --engine
      neshawk`), the application's `emu.engine = "neshawk"`, and `.bk2` playback.
- [ ] 4. Verification against BizHawk: the *Kirby* `.bk2` to the last frame (156 312) against the
      per-frame references and the cycle ledger; *Faxanadu*; AccuracyCoin and the blargg suites
      with the **same verdicts as NesHawk** (including the `$8B ANE` hang).
- [ ] 5. 3D: the tile capture the renderer needs, as `zax-fceux` does; capture parity measured on
      the profiled games.
- [ ] 6. Speed and final pass: bench, fmt, clippy `-D warnings`, `cargo test --release`,
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
