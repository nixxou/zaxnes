# `zax-neshawk` — BizHawk's NesHawk core, ported to Rust

What it is, what it was verified against, and what is not there yet. Written at the end of lot
**NESHAWK-REWRITE**; the working is `docs/progress/NESHAWK-REWRITE.md`.

## 1. What this is for

The NESHAWK-PROFILE lots spent thirteen rounds making Zaxnes's **own** core behave like NesHawk by
adding a timing profile to it, measuring one quirk at a time against BizHawk. That path reached
tree frame 40 of the *Kirby* `.bk2` and stopped there: the remaining differences were not single
quirks but the accumulated shape of a different machine.

This lot takes the other path. `zax-neshawk` is not Zaxnes configured to resemble NesHawk — it is
**NesHawk**, translated from BizHawk 2.11.1's C# into Rust, structure and statement order and
field names intact, and then held against BizHawk until the two agreed.

They agree. The whole 156 312-frame Kirby movie, the whole of both Faxanadu dumps, and 34 026
cycle-dated bus events.

It is also, now, a **second engine of the application**: `emu.engine = "neshawk"` beside `zaxnes`
and `fceux`, with a serialised savestate, the 3D tile capture the renderer reads, thirteen board
classes and a marked fallback on Zaxnes's own board for any mapper it has not got — so no
cartridge is refused for want of a board. §7 is the honest list of what is still missing.

## 2. The result

| what | reference | compared | result |
|---|---|---|---|
| *Hoshi no Kirby* `.bk2`, **all 156 312 frames** | `kirby-jp-step1{,b,c,d,e,f,g}.txt` | 156 318 frame samples of the 2 KiB of work RAM, **no gap between the windows** | **every byte of every frame identical** |
| *Faxanadu* (USA), all 83 218 frames | `fax-usa.txt` | 832 checksum lines + the lag counter | all agree, to frame 83 200 |
| *Faxanadu* (USA Rev 1), all 83 218 frames | `fax-rev1.txt` | 832 checksum lines + the lag counter | all agree, to frame 83 200 |
| **the cycle ledger**, Kirby frames 2-1 200 | `kirby-jp-cyc0.txt` | **34 026 event lines** and every frame's absolute CPU-cycle count | **every line identical** |
| `nestest.nes` | blargg's reference log | 8 991 instructions: `PC`, `A`, `X`, `Y`, `SP`, cycle count | exact (`P` bit 4 is NesHawk's own convention; §6) |
| `instr_test-v5`, all 16 | the ROMs' own `$6000` verdict | 16 | **16 Passed**, the verdict NesHawk gives them |
| **AccuracyCoin**, same instrument on both machines | `bizhawk-runs/nhr3-coin-neshawk.txt`, taken this lot | **147 tests** | **all 147 verdicts identical**; 130/147 pass on both, the port in 4 166 frames and NesHawk in 4 161 |
| the five DMC/DMA ROMs the port "fails" | `bizhawk-runs/nhr1-*.txt`, `nhr2-*.txt`, taken this lot | 5 | **NesHawk fails all five identically** — same message, same code, same on-screen CRC |

The cycle ledger is the strongest of these and is worth saying plainly: every write to
`$4000-$401F`, every read of `$2002` and `$4015`, every NMI and IRQ entry and every watched PC
lands on the **same CPU cycle of the same frame** with the **same `A`/`X`/`Y`/`SP`**, for 1 199
consecutive frames. The instrument is `movie-sync --engine neshawk --neshawk-event-log`, which
writes the ledger in the exact shape `ext-tools/scripts/bk2cyc*.lua` writes it on BizHawk, so the
check is a file diff rather than a judgement.

Speed, release build, after 60 frames of warm-up (a figure, not a budget). Both ways round,
because the application's engine runs with the 3D tile capture **on** and the harness's tools run
with it off — one figure would describe half the uses:

| ROM | frames/s, capture off | frames/s, capture on | ms a frame (on) |
|---|---|---|---|
| Super Mario Bros. 3 | 582 | 571 | 1.75 |
| Mega Man 2 | 547 | 539 | 1.86 |
| Super Mario Bros. | 602 | 601 | 1.66 |

So about nine to ten times real time, and the capture costs **about 2 %**. Mega Man **3** is not in
`ZAXNES_ROMS`; Mega Man 2 (MMC1) stands in for it and SMB (NROM) is the third point.

## 3. What is ported, and from where

Every file carries a header naming its C# source. `LICENSE-BizHawk` carries BizHawk's MIT notice.

| Rust | C# |
|---|---|
| `src/cpu/mod.rs` | `CPUs/MOS 6502X/MOS6502X.cs`, `IMOS6502XLink.cs` |
| `src/cpu/uop.rs` | `Execute.cs`'s `enum Uop`, the `VOP_*` constants and the 264-row `Microcode` table |
| `src/cpu/execute.rs` | `Execute.cs`'s 252 micro-operation bodies and the dispatch |
| `src/nes/mod.rs` | `NES.Core.cs` — `RunCpuOne`, the OAM and DMC DMA blocks, `FrameAdvance`, `HardReset`, the memory map, `ReadReg`/`WriteReg`, the joypad ports — and `NES.CpuLink.cs` |
| `src/nes/load.rs` | `NES.cs`'s `Init` and `DetectRegion`, plus SHA-1 and MD5 (this crate has no dependencies) |
| `src/nes/apu.rs` | `APU.cs`'s `DMCUnit.Run`, `DMCUnit.Fetch`, `RunOneFirst`, `RunOneLast` |
| `src/apu.rs` | the rest of `APU.cs`: the five channels, `set_lenctr_en`, the frame sequencer, the mixer |
| `src/ppu/mod.rs` | `PPU.cs`'s state, `PPUREGS`, `Reg_2000`, `Reg_2001`, `PpuOpenBusDecay`, `pipeline` |
| `src/ppu/run.rs` | `PPU.run.cs` and `PPU.cs`'s `runppu` |
| `src/ppu/regs.rs` | `PPU.regs.cs`'s `$2000`-`$2007` |
| `src/boards/` | `NesBoardBase.cs`, `NROM`, `UxROM`, `CNROM`, `AxROM`, `SxROM` (MMC1, with `SuROM`/`SXROM`/`SoROM`), `MMC3.cs`, `TxROM` — then the second board set: `GxROM.cs` (66), `IC_74x377.cs` (11 and the Color Dreams/AGCI boards), `PxROM_FxROM.cs` (MMC2 = 9, MMC4 = 10), `Camerica.cs` (71 and 232), `Mapper069.cs`'s `Sunsoft_FME7` (69), `VRC2_4.cs` (21, 22, 23, 25, 27, 116) |
| `src/boards/zaxnes.rs` | **nothing** — the fallback of §4b, which is Zaxnes's own board behind NesHawk's `INesBoard` |
| `src/cart.rs` | `CartInfo.cs`, `NES.iNES.cs` |
| `src/gamedb.rs` | `BootGodDB.cs` (the `NesCarts.xml` state machine), `IdentifyFromGameDB`, `Database.cs`'s record format |
| `src/controllers.rs` | `NESControllers.cs`'s `StrobeInfo`, `NesDeck`, `ControllerNES`, `UnpluggedNES` |
| `src/state.rs` | the `SyncState(Serializer)` members of `NES`, `PPU`, `APU`, `MOS6502X` and every board — see §4c |
| `src/engine.rs` | **nothing**: `zax_core_api::Engine` over the port, which is Zaxnes's own interface |
| `src/tiles.rs` | **nothing**: the 3D tile capture, which is Zaxnes's own (SPEC-02 §4.1, REQ-CORE-030 to 046) |

### Why the game databases are part of the port

Board choice is not cosmetic for a timing port. *Faxanadu* is `NES-SGROM` with **no** WRAM in
NesCartDB where its iNES header implies the generic 8 KiB, and an MMC3 cart names its revision
chip there (`MMC3A`/`MMC3C`…), which sets `oldIrqType` and so changes the IRQ edge. A load without
the databases is a different machine. `movie-sync --engine neshawk --neshawk-gamedb <dir>` (or
`ZAXNES_BIZHAWK_GAMEDB`) points at BizHawk's own `gamedb`; with neither, detection falls back to
the iNES header and the load report says so.

## 4. The shape of the port, and the three places it had to differ

The C# has `NES`, `PPU`, `APU` and the board all holding references to each other. The rule here
is: **a member that reaches the machine is `impl Nes`** (or `impl Bus`), in the module of the
component it belongs to, with the C# method's name and body. So `runppu`, every `TickPPU_*`, every
`$2000`-`$2007` handler, `DMCUnit.Run`/`Fetch` and `RunOneLast` live beside the machine rather
than beside their component's state; the components hold state and the members that need nothing
else. No statement moved across a cycle boundary to make that work.

Three deviations exist and each is documented at its site:

1. **`Nes` is `{ cpu: Mos6502x, b: Bus }`**, and `cpu.execute_one(&mut self.b)` borrows the two
   halves at once by destructuring. Four registers the C# keeps on the CPU but the machine also
   *drives* live on `Bus` and the CPU reaches them through `Mos6502xLink`: `address_bus`, `RDY`,
   `rdy_freeze` and `TotalExecutedCycles`. `Nes` derefs to `Bus`, so the ported code still reads
   `self.ppu`, `self.apu`, `self.board` exactly as the C# does.
2. **`BoardEnv`**: a board reaches `NES.CIRAM` and `NES.DB` and nothing else, so those two travel
   in a struct built from disjoint fields of `Bus` at each call.
3. **`on_exec_fetch` carries `A`/`X`/`Y`/`S`**, because BizHawk's own `event.onmemoryexecute`
   callbacks read them off the CPU object at that moment and the cycle ledger prints them.

All three are consequences of one fact, and it is the lesson of the lot: **a C# field written from
both sides needs an explicit crossing in Rust**, and a port that quietly drops one of those
writes will pass every test that does not exercise it. §5 is what that cost.

## 4b. The fallback: a mapper with no ported board runs on Zaxnes's

The owner's decision of 2026-11-12. Porting every NesHawk board was never the plan — the brief
asks for the boards covering about 80-95 % of the commercial library — and the question is what
happens to the rest. The answer is **not a load error**: `boards::zaxnes::ZaxnesBoard` puts a whole
`zax-nes` cartridge behind NesHawk's `INesBoard`, and the cartridge runs with the port's CPU, PPU
and APU and the other engine's board. One component differs, and **everything that can tell a user
or a tool says so**: the load report, `RomLoadResult::note`, `NesHawkEngine::board_is_from_zaxnes`,
`movie-sync`'s printed note, and every `test-roms` row (`neshawk_board`, `board_from_zaxnes`, and
the words "board from Zaxnes, not NesHawk-exact" in the message).

Three things it loses, listed because "not NesHawk-exact" otherwise means nothing:

1. **The IRQ counter's clock.** A NesHawk board counts PPU A12 rises through `AddressPPU`; a
   Zaxnes board of the same mapper may count them through its own `A12Watcher` with its own
   filter, or count scanlines. On an MMC3-like mapper that is a cycle or two on the IRQ — exactly
   what a `.bk2` notices.
2. **The fetch kind on a PPU read.** NesHawk's `ReadPPU` does not carry one, so the adapter says
   `ChrFetch::Background` — the pessimistic choice for a board that distinguishes them.
3. **Four-screen nametables.** NesHawk's CIRAM is on the console and a board supplies a mirroring
   map; Zaxnes's cartridge may serve nametable bytes itself. The adapter honours that through
   `nametable_page`, but the console's 2 KiB and the cartridge's view of them are two memories
   here where in Zaxnes they are one.

One implementation note worth keeping, because it is load-bearing rather than clever: the
cartridge is behind a **`RefCell`**. NesHawk's `INesBoard` read hooks are `&self` in this port
(every ported board leaves no trace on a read) while `Cartridge::try_cpu_read` is `&mut` — the
MMC5's `$5204` clears its IRQ flag when read. Without the cell those side effects would silently
not happen, which is the worst kind of defect: a game that mostly works.

## 4c. The savestate, and the one guarantee it has that the C# has not

`src/state.rs` is the port of `SyncState`. BizHawk's `Serializer` is one visitor that both reads
and writes, so a class lists its fields once; this port cannot do that in a single method, because
`Engine::save_state_to` takes `&self` and `try_load_state` a `&mut self`. So `sync_struct!`
generates **both halves from one field list** — the same token sequence, so the write order and the
read order cannot drift apart.

What the macro adds over the reference: each generated body begins with an **exhaustive**
destructuring of the struct (`let Self { a, b, c } = self`, no `..`), so a field added later and
named in neither the saved list nor the `skip:` list is a **compile error**. A savestate that
silently forgets a new field is the classic emulator defect — a game returns from a state with the
wrong banks mapped — and here it cannot happen. `Board::write_state`/`read_state` are **required**
trait methods for the same reason: a new board does not compile until it has answered the question.

The block carries the engine's magic (`ZXNH`), the version, the **board class** and the host's
region, and refuses a block of another engine with REQ-CORE-120's own message, a block of another
board class, and a payload whose length the machine does not read exactly. The frame buffer is in
the block only for a self-contained state; a rollback slot leaves it out, which is what
REQ-CORE-121's own in-memory snapshot does and is 120 KiB a slot.

## 5. The divergence that was found, and the line that caused it

The port's first Kirby run parted from BizHawk at **frame 11** — the frame the game first writes
`$4014` and first enables the DMC. The byte diff named the cause before any bisection: our stack
held the same byte sequence as BizHawk's **two addresses higher**, so a 2-byte push was missing.

The mistranslated line was not in the DMA code. `NES.WriteReg($4014)` does `cpu.RDY = false`, and
`WriteReg` is reached **from inside `cpu.ExecuteOne()`**. The first version of this port lifted the
CPU *out* of the machine for the duration of `execute_one` (`mem::take`, put back after) so that
the link could hold the rest of it — so the write landed on the placeholder CPU and was thrown
away. `RDY` stayed high, the 6502 ran straight through the 513-cycle sprite DMA, and the frame was
that much shorter. The same hole swallowed every bus-side read of `cpu.TotalExecutedCycles` —
`double_controller_read` and `double_2007_read` both read 0 for ever — and made the cycle ledger's
register columns garbage, which is what exposed it.

The fix is the structural change of §4.1, not a constant. After it, frames 1-400 went from "first
live difference at frame 304" to **400 of 400 identical to the byte**, and the whole movie
followed.

Three further details of the reference that a tidy-up would have silently changed, each
transcribed as it stands:

* `MMC3Board_Base.WritePpu` returns **before** `base.WritePpu` when the board has no VRAM, so on a
  CHR-ROM board (Kirby's) a PPU write below `$2000` does nothing at all; and when there *is* VRAM
  it reassigns `addr` to the mapped address, so the base writes the same mapped byte a second time.
* `DMCUnit.Fetch` tests `(sample_address & 0x1F) == 0x4017`, which after the mask can never hold.
  It is a defect of the reference (it means `0x17`); kept, because "the same bug" is the
  requirement.
* `NoiseUnit.clock_env` tests `env_divider != 0` and then `env_divider == 0` in two **separate**
  ifs, so a divider that reaches zero is reloaded in the same call — unlike `PulseUnit.clock_env`,
  which uses `else if`. The two channels really do differ.

## 6. `nestest`'s one masked column

**Bit 4 of `P` — the B flag — is excluded from the `nestest` comparison.** On a real 6502 bit 4 is
not a register bit at all; it exists only in the byte a `PHP` or an interrupt *pushes*. NesHawk
models it as real state and never clears it after a push, so after a `PHP` its `P` keeps bit 4
where silicon clears it. The port keeps NesHawk's behaviour, because that is what this crate is
for. What makes it a convention difference rather than a hidden defect: the *pushed bytes* are
compared exactly — nestest pulls the pushed `P` into `A` and `A` is checked unmasked on every one
of the 8 991 lines — as are `PC`, `X`, `Y`, `SP` and every cycle count.

## 7. Known gaps

Stated rather than papered over. Nothing below is a measured difference from NesHawk on a path
this port claims; each is work this lot did not reach, or a scope line drawn on purpose.

| gap | what it means | what it would take |
|---|---|---|
| **MMC5 (mapper 5) is not ported** | it runs on Zaxnes's board through the adapter of §4b, marked "board from Zaxnes, not NesHawk-exact". `ExROM.cs` is 799 lines and wants two things this port's `Board` trait cannot express: a **mutable** PPU-register read (`ReadReg2xxx`) and nametable bytes the *cartridge* serves (`ExRAM`, fill mode) | widening `Board` for both, then the port. The five MMC5 ROMs in the collection already run on the fallback |
| **`SUNSOFT-5B` is refused by `Sunsoft_FME7`** | so Gimmick! (J) takes the fallback. The C#'s `Sunsoft_5` is the FME-7 plus the 5B's own sound chip, and this port has no expansion audio; a board that silently drops a third of that game's music would be worse than one marked | a `Sunsoft5BAudio` port, which needs expansion audio first |
| **Mapper 71 is ported but unexercised** | the collection has no mapper-71 ROM and no mapper-71 game, so `Camerica_Mapper071` is a faithful transcription that nothing has run | a cartridge |
| **Audio is this engine's own waveform** | the *timing* of every `blip.AddDelta` is NesHawk's to the cycle, and that is what the port is verified on. The reconstruction is not: `BlipBuffer.cs` is a `DllImport` shim over Blargg's native `blip_buf`, so **there is no C# to port**, and `NesHawkEngine` integrates the deltas and box-filters them to the output rate instead. A `.wav` from this engine is not bit-identical to BizHawk's | a `blip_buf` port, which means fetching a C library this lot may not fetch |
| **A region change is a power cycle** | `set_region` rebuilds the machine when the timing really changes, because `_display_type` is read by `HardReset` — in BizHawk changing a NES core's region needs a core reboot too. REQ-CORE-016 asks for no reset; `zax-nes` can do that and NesHawk cannot | nothing: it is NesHawk's own constraint, and the host's own order (`load_rom` then `set_region`) loses no state |
| **No run-ahead, no widescreen, no hack packs, no sprite-limit control, no machine internals** | `Capabilities::NESHAWK` says so and the application greys those rows with the reason. The four are patches to *Zaxnes's* bus or PPU, not features of an emulator | the first would be save/run/take/load per frame, as on FCEUX; the rest would mean reimplementing Zaxnes's bus |
| **The application cannot open a `.bk2`** | it opens `.fm2`, `.fcm` and Zaxnes's own. So "a `.bk2` defaults to the NesHawk engine" is delivered as the *rule* plus its one live use — `EngineKind::for_bizhawk_core`, `MovieFile::recorded_engine`, and `movie-sync`'s default engine — and not as an application feature | a `.bk2` importer in `zax-app`, which has the rule waiting for it |
| **MMC2/MMC4 capture reads the latch as the frame left it** | `zax-nes`'s capture walks a line moving the `$FD`/`$FE` latch forward tile by tile and puts it back (REQ-CORE-039); this port's capture reads through `Board::peek_ppu`, which by the C#'s own design does not touch the latch. For a mapper-9/10 game a captured tile can come from the other 4 KiB bank | the per-column latch walk, in `crate::tiles` |
| **VS. System, FDS, NSF, UNIF, the light gun, the cheat engine, the code-data logger, the memory domains, `Palettes.cs`** | not ported; such an image is refused by name rather than guessed at | — |

### The one capture difference found, and why it is not patched

Over the 330-ROM collection at frames 60 and 300, `neshawk-tiles` puts **2 frames** in the
`capture` class — the class where the two engines' pictures are identical and only the records
differ, which is the only class this lot answers for. Both are `other/window_old_ntsc.nes`, and in
both the port emits **one extra background record** on `y = 233`, the frame's last row, with **no
field differing on any record**: 2 048 of 2 050 capture-class records are identical in order,
99.90 %.

The mechanism, named rather than tuned away: a background row carries 33 records instead of 32
only when fine X is not 0 (REQ-CORE-038), and that ROM rewrites the scroll every scanline, so the
fine X the two cores hold **at the capture instant** differs by a cycle's worth of dot alignment on
the last line. Making it agree would mean moving when the capture samples fine X to suit one
torture-test ROM, which is exactly the tuning constant this lot's rules forbid.

## 7b. The ROM suites, and the correction they forced

`test-roms --engine neshawk` runs the whole 331-ROM collection on the port through the **same four
readers** that score Zaxnes (`test_roms/machine.rs` names the five operations they need). Same
readers on both machines is the only way a verdict comparison means anything.

| | pass | fail | unsupported | manual |
|---|---|---|---|---|
| `--engine neshawk` | **199** | 20 | **1** | 111 |
| `--engine zaxnes` (unchanged by this lot) | 212 | 7 | 1 | 111 |

**The one unsupported ROM is `other/LINUSMUS.NES`**, a headerless image with no database entry,
which *Zaxnes* refuses too. Before the fallback and the second board set there were **31**, all of
them unported boards.

Of the 20 failures, every one is accounted for and **none is a port defect**:

* **2 mapper-4 variants** (`mmc3_test/6-MMC6`, `mmc3_test_2/6-MMC3_alt`) which were *unsupported*
  before: NesHawk routes them to board classes this port has not got, and Zaxnes's mapper-4 board
  does not implement MMC6 either. The fallback's honest limit.
* **AccuracyCoin at 130/147**, which is NesHawk's own figure — all 147 verdicts identical.
* **12 `holy-mapperel` MMC1 ROMs**: ten are the `$E000` bit 4 case below, and **two are SNROM
  boards that fail on Zaxnes as well**, checked by running both engines over them.
* **Five DMC/DMA ROMs** that NesHawk fails identically, below.

And against the other engine, on the 40 `holy-mapperel` ROMs: the two engines give the **same
verdict on 30 and differ on 10**, and all ten are the MMC1 case. **Not one ROM of a board this lot
added disagrees with Zaxnes.**

The two detailed findings:

* **Ten `holy-mapperel` MMC1 ROMs**, all reporting "WRAM 1: `$E000` bit 4 does not disable WRAM".
  `SxROM.cs` honours `mmc1.wram_disable` **only when `_is_snrom`**, which only `NES-SNROM` sets,
  and says so: *"different MMC1 revisions handle wram_disable differently; on some it doesn't work
  at all… we only emulate the first case"*. These boards are SKROM/SJROM/SUROM/SXROM, so NesHawk
  does not disable WRAM either. The port is **faithful**, and Zaxnes passing them is Zaxnes being
  more accurate than NesHawk.
* **Five DMC/DMA ROMs** (`apu_test/7-dmc_basics` and its combined ROM,
  `dmc_dma_during_read4/dma_4016_read`, `sprdma_and_dmc_dma` and its `_512`). Each was put to
  NesHawk itself on BizHawk this lot. **NesHawk fails every one of them identically** — the same
  `Failed #19` and the same "There should be a one-byte buffer that's filled immediately if empty"
  for `7-dmc_basics`, and for `dma_4016_read` the same on-screen `08 08 08 08 07` and the same
  ROM-computed CRC `6B48DA20`, character for character.

### The correction: `bizhawk-runs/coin-neshawk.txt` is not a NesHawk run

The lot's brief asks the port to reproduce NesHawk's AccuracyCoin result — *"67/147, it hangs at
`$8B ANE`"* — from `bizhawk-runs/coin-neshawk.txt`. It does not, and it should not: **that file
was not taken on NesHawk.**

The evidence is direct. Launching `EmuHawk.exe` with a ROM and a Lua script and no `--config`
loads the NES with BizHawk's *default* core, which this installation reports as
`[QuickerNES] Cart Mapper Info` — the trap was hit in this very session, on the first attempt at
`7-dmc_basics`. Re-run with `--config=ext-tools/scripts/config-neshawk.ini` (whose
`PreferredCores` names `"NES": "NesHawk"`), the same `coin_bizhawk.lua` protocol and a larger
frame cap, NesHawk scores **130/147 in 4 161 frames** — its run-all completes, nothing hangs at
`$8B ANE`, and `$8B ANE` **passes**. The port scores 130/147 in 4 166 frames, and **all 147
verdicts are identical**. The old file disagrees with NesHawk's own run on **80 of 147** tests.

The new reference is `bizhawk-runs/nhr3-coin-neshawk.txt` (nothing was deleted). Anything else
measured against `coin-neshawk.txt` should be re-read in this light.

## 8. Running it

```
# the whole Kirby movie against BizHawk's own per-frame work RAM
ZAXNES_BIZHAWK_GAMEDB=.../BizHawk-2.11.1/gamedb \
zax-harness movie-sync <kirby.bk2> <kirby.nes> --engine neshawk --no-lag \
  --ram-ref bizhawk-runs/kirby-jp-step1g.txt

# the cycle ledger, in bk2cyc*.lua's own shape, for a file diff against BizHawk's
zax-harness movie-sync <kirby.bk2> <kirby.nes> --engine neshawk --no-lag --frames 1201 \
  --neshawk-event-log ours-cyc0.txt --neshawk-watch-pc C37C,C06E,80FC,804C
```

```
# the whole 331-ROM collection on the port, scored by the readers that score Zaxnes
zax-harness test-roms <test-roms dir> out.json --engine neshawk

# where the port's 3D tile records differ from Zaxnes's, field by field, with the class column
zax-harness neshawk-tiles <roms dir|rom.nes> out.jsonl --frames 60,300,1200 --detail --records 4

# and a .bk2 needs no --engine at all: one recorded on NesHawk defaults to the port of that core
zax-harness movie-sync <movie.bk2> <rom.nes> --no-lag
```

In the application the engine is the per-game setting `emu.engine` (`Zaxnes` / `FCEUX` /
`NesHawk`), which takes effect at the next load. Choosing NesHawk greys the widescreen rows, the
hack packs, the sprite limit and the run-ahead, each with the reason beside it.

The crate's own tests: `cargo test -p zax-neshawk --release -- --include-ignored` with
`ZAXNES_TEST_ROMS`, `ZAXNES_BIZHAWK_GAMEDB` and `ZAXNES_ROMS` set — `nestest` against its
reference log, the 16 `instr_test-v5` ROMs, a power-on smoke test, the speed figures, the
savestate round trip, the foreign-engine refusal, and the 3D capture proved to change nothing
(120 frames of work RAM, picture and cycle count with it on and off).

## 9. Suites, at the end of the lot

| | result |
|---|---|
| `golden-core`, 98 originals x 1 200 frames, **Zaxnes** engine | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical**; `zax-nes` was not touched by this lot (`git diff` over the crate is empty) |
| *Hoshi no Kirby* `.bk2`, frames 1-400, re-run after every step | **400 of 400 byte-identical** at each of the five |
| `nestest`, re-run after the capture hooks were cut into the dot loop | still exact on all 8 991 instructions |
| `test-roms --engine neshawk` | **199 pass / 20 fail / 1 unsupported / 111 manual** (was 183 / 18 / 31 / 99) |
| `test-roms --engine zaxnes` | **212 pass / 7 fail**, the documented figure, unchanged |
| `neshawk-tiles`, 18 originals x 3 frames | **44 of 54 frames identical on all ten fields, 0 capture-class** |
| `neshawk-tiles`, 330 test ROMs x 2 frames | 566 identical, 88 emulation, 4 uniform, **2 capture** — 2 048 of 2 050 capture-class records identical in order, **99.90 %** |
| `cargo test --release --workspace` | **2 527 passed, 0 failed, 93 ignored** (excluding the two SDL2 crates below) |
| the same with `fceux-core` on | **205 passed, 0 failed, 17 ignored** |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean |

**The two environment gaps, both pre-existing and neither reachable from this lot.**
`zax-platform` and `zax-app` fail to *link* their tests (`LNK1181: SDL2.lib`); `cargo check -p
zax-app --all-targets` and `cargo clippy -p zax-app --lib --all-features` are both clean, so the
code compiles and only the link is missing. `zax-harness`'s `bench_core_tests` want
`rust/test-roms/nestest.nes`, which nothing tracks and `.gitignore` ignores, so it is a
per-worktree asset like `zax-app`'s two demo files; copied in from `assets-free`, the four pass.

**One pre-existing break this lot found and fixed**: `ControllerPortType::Empty` was added by lot
NESHAWK-PROFILE-13 and `zax-fceux`'s `set_port_type` never got its arm, so the **`fceux-core`
configuration had not compiled since**. `Empty` is now refused as `Zapper` is, which is the right
answer rather than a convenience: `Empty` is BizHawk's `UnpluggedNES`, whose read answers 0 for
ever where a pad answers 1 from the ninth read, and accepting it would hand back a pad while
reporting an unplugged socket.
