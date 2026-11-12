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

## 2. The result

| what | reference | compared | result |
|---|---|---|---|
| *Hoshi no Kirby* `.bk2`, **all 156 312 frames** | `kirby-jp-step1{,b,c,d,e,f,g}.txt` | 156 318 frame samples of the 2 KiB of work RAM, **no gap between the windows** | **every byte of every frame identical** |
| *Faxanadu* (USA), all 83 218 frames | `fax-usa.txt` | 832 checksum lines + the lag counter | all agree, to frame 83 200 |
| *Faxanadu* (USA Rev 1), all 83 218 frames | `fax-rev1.txt` | 832 checksum lines + the lag counter | all agree, to frame 83 200 |
| **the cycle ledger**, Kirby frames 2-1 200 | `kirby-jp-cyc0.txt` | **34 026 event lines** and every frame's absolute CPU-cycle count | **every line identical** |
| `nestest.nes` | blargg's reference log | 8 991 instructions: `PC`, `A`, `X`, `Y`, `SP`, cycle count | exact (`P` bit 4 is NesHawk's own convention; §6) |
| `instr_test-v5`, all 16 | the ROMs' own `$6000` verdict | 16 | **16 Passed**, the verdict NesHawk gives them |

The cycle ledger is the strongest of these and is worth saying plainly: every write to
`$4000-$401F`, every read of `$2002` and `$4015`, every NMI and IRQ entry and every watched PC
lands on the **same CPU cycle of the same frame** with the **same `A`/`X`/`Y`/`SP`**, for 1 199
consecutive frames. The instrument is `movie-sync --engine neshawk --neshawk-event-log`, which
writes the ledger in the exact shape `ext-tools/scripts/bk2cyc*.lua` writes it on BizHawk, so the
check is a file diff rather than a judgement.

Speed, release build, after 60 frames of warm-up (a figure, not a budget):

| ROM | frames/s | ms a frame |
|---|---|---|
| Super Mario Bros. 3 | 594 | 1.68 |
| Mega Man 2 | 556 | 1.80 |
| Super Mario Bros. | 622 | 1.61 |

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
| `src/boards/` | `NesBoardBase.cs`, `NROM`, `UxROM`, `CNROM`, `AxROM`, `SxROM` (MMC1, with `SuROM`/`SXROM`/`SoROM`), `MMC3.cs`, `TxROM` |
| `src/cart.rs` | `CartInfo.cs`, `NES.iNES.cs` |
| `src/gamedb.rs` | `BootGodDB.cs` (the `NesCarts.xml` state machine), `IdentifyFromGameDB`, `Database.cs`'s record format |
| `src/controllers.rs` | `NESControllers.cs`'s `StrobeInfo`, `NesDeck`, `ControllerNES`, `UnpluggedNES` |

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

Stated rather than papered over. Nothing below is a measured difference from NesHawk; each is work
this lot did not reach.

| gap | what it means | what it would take |
|---|---|---|
| **The `Engine` trait is not implemented** | `zax-neshawk` is reachable from `movie-sync --engine neshawk` through the harness's own small `Machine` trait, but not from the application: there is no `EngineKind::NesHawk`, no `emu.engine = "neshawk"`, and no `.bk2` default-engine choice | a `NesHawkEngine` over the 30-method trait of `zax-core-api`; the only real work in it is a **savestate**, since the port has no `SyncState` yet and currently snapshots by cloning the whole machine |
| **No savestate format** | rollback, run-ahead and cross-engine refusal all need a serialised block. `Nes: Clone` is exact and is what the harness's input-sensitivity probe uses, but it is not a format | port `SyncState` from each C# class, in the C#'s field order |
| **AccuracyCoin and the screen-reading suites are not scored** | the brief asks for NesHawk's own verdicts including the `$8B ANE` hang (`bizhawk-runs/coin-neshawk.txt`, 67/147). The `$6000` protocol cannot read them: for those ROMs the board has no WRAM, so `$6000` is open bus on BizHawk *and* here, and the verdict is only on the screen | make `test_roms`'s four readers (`blargg`, `screen`, `coin`, `mapperel`) generic over a five-method trait — they use only `read_memory`, `run_frame`, `soft_reset`, PPUCTRL's low two bits and a nametable-byte peek — then `test-roms --engine neshawk` |
| **No tile capture, so no 3D** | the renderer needs the per-frame tile records `zax-nes` and `zax-fceux` produce | the capture of `docs/review/FCEUX-ENGINE.md` §2 and lot FCEUX-6's `fceux-tiles` parity tool |
| **The second board set is not ported** | MMC2/MMC4 (9/10), Color Dreams (11), GxROM (66), Camerica (71), MMC5 (5), FME-7 (69), VRC2/4 (23/25). The six ported boards cover NROM, MMC1, UxROM, CNROM, MMC3 and AxROM | each is a short file; MMC5 is the one that would widen the `Board` trait, because its `ReadReg2xxx` is a *mutable* read |
| **No fallback to Zaxnes's boards** | the owner's decision of 2026-11-12 is that any unported mapper falls back on `zax-nes` through an adapter, marked "board from Zaxnes, not NesHawk-exact". Today an unported mapper is a load error naming the board type, which is honest but is not the decision | the adapter, plus the marker in capabilities, the log and the harness output |
| **VS. System, FDS, NSF, UNIF, the light gun, the cheat engine, the code-data logger, the memory domains, `Palettes.cs`** | not ported; a UNIF/FDS/NSF image is refused by name rather than guessed at | — |
| **Audio is deltas, not samples** | `RunCpuOne` records every `blip.AddDelta(sampleclock, delta)` into `Nes::audio_deltas`; nothing resamples them yet | a `BlipBuffer` port or the host's own resampler |

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

The crate's own tests: `cargo test -p zax-neshawk --release -- --ignored` with
`ZAXNES_TEST_ROMS`, `ZAXNES_BIZHAWK_GAMEDB` and `ZAXNES_ROMS` set — `nestest` against its
reference log, the 16 `instr_test-v5` ROMs, a power-on smoke test and the speed figure.

## 9. Suites, at the end of the lot

| | result |
|---|---|
| `golden-core`, 98 originals x 1 200 frames, **Zaxnes** engine | `8d508070850b24228285b6f04df6f98c310dbdab65e7577921c91ad4d0689773` — **byte-identical**; `zax-nes` was not touched |
| `cargo test --release --workspace` | all green, excluding three **pre-existing environment gaps**: `zax-platform` and `zax-app` need `SDL2.lib`, which this worktree has not got, and `zax-harness`'s `bench_core_tests` need the untracked `rust/test-roms/nestest.nes` (supplied from `assets-free` to run them) |
| `cargo fmt --all --check` | clean |
| `cargo clippy --workspace --all-targets -- -D warnings` | clean (same two crates excluded, for the same link reason) |
