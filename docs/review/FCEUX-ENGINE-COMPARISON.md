# FCEUX against Zaxnes's own core, ROM by ROM

Produced by lot **FCEUX-1 step 5**: `zax-harness golden-core` on Zaxnes's core and
`zax-harness fceux-run` on FCEUX, then `zax-harness fceux-compare`. Both engines are driven
identically — 1200 frames, no button pressed, a reset after loading, 48 kHz audio, and
FCEUX's deterministic default RAM fill — so a difference is the engines' and not the
harness's.

**This is information, not a pass/fail gate.** Two independent emulators differing on a
pixel is the expected case. The table exists so that a game one engine emulates badly can be
given the other, and so that the two can go on checking each other.

The pictures compared are the **same quantity on both sides**: the SHA-256 of the 256x240
image of NES colour indices, 0 to 63 a pixel. The audio digests are deliberately *not*
compared — both are hashed identically, but the two APUs filter and resample differently by
design, so they differ on every ROM and the column would carry no information. Sample
*counts* are compared by `fceux-compare` instead.

## The 291 public test ROMs

`rust/test-roms` — the set `rust/goldens/core-goldens.jsonl` was taken over.

| | |
|---|---|
| ROMs | 291 |
| loaded by both | 289 |
| loaded by Zaxnes only | 1 — `other/oam3.nes` |
| loaded by FCEUX only | 0 |
| refused by both | 1 — `other/LINUSMUS.NES` |
| took the FCEUX engine down | 1 — `other/oam3.nes` |

### Identical pictures, by frame

| Frame | Identical | Of | Share |
|---|---|---|---|
| 1 | 287 | 289 | 99% |
| 60 | 183 | 289 | 63% |
| 300 | 161 | 289 | 56% |
| 600 | 165 | 289 | 57% |
| 1200 | 162 | 289 | 56% |

**Identical at 60, 300 *and* 1200: 157 of 289.** 132 differ at one or more.

The ones that differ, by mapper: **0** x92, **4** x20, **1** x7, **3** x6, **11** x3, **5** x3, **2** x1.

### The test ROMs' own verdicts

113 ROMs run blargg's $6000 protocol and report on both engines. This outranks every
digest in the table above: it is the ROM saying whether the engine is right, not us
saying the two agree.

| | Zaxnes | FCEUX |
|---|---|---|
| passed | 95 | 58 |

Same status code on both: 70. Passed by Zaxnes but not FCEUX: **37**. Passed by FCEUX but not Zaxnes: **0**.

The 43 they disagree on:

| ROM | Zaxnes | FCEUX |
|---|---|---|
| apu_reset/4017_timing.nes | Failed (129) — Delay after effective $4017 write: 10 | Failed (2) — Delay after effective $4017 write: 14 |
| apu_reset/4017_written.nes | Failed (129) — Press RESET | Failed (2) — At power, $4017 should be written with $00 |
| apu_test/apu_test.nes | Passed — 8-dmc_rates | Failed (1) — Writing $00 or $80 to $4017 shouldn't affect flag |
| apu_test/rom_singles/3-irq_flag.nes | Passed — 3-irq_flag | Failed (6) — Writing $00 or $80 to $4017 shouldn't affect flag |
| apu_test/rom_singles/4-jitter.nes | Passed — 4-jitter | Failed (2) — Frame irq is set too soon |
| apu_test/rom_singles/5-len_timing.nes | Passed — 5-len_timing | Failed (2) — Channel: 0 |
| apu_test/rom_singles/6-irq_flag_timing.nes | Passed — 6-irq_flag_timing | Failed (2) — Flag first set too soon |
| cpu_dummy_writes/cpu_dummy_writes_oam.nes | Passed — TEST: cpu_dummy_writes_oam | Failed (2) — TEST: cpu_dummy_writes_oam |
| cpu_exec_space/test_cpu_exec_space_ppuio.nes | Passed — TEST:test_cpu_exec_space_ppuio | Failed (5) — TEST:test_cpu_exec_space_ppuio |
| cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes | Passed — NMI BRK 00 | Failed (1) — NMI BRK 00 |
| cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes | Passed — NMI BRK | Failed (1) — NMI BRK |
| cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes | Passed — 0 +0 | Failed (1) — 0 +0 |
| cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes | Passed — test_jmp | Failed (1) — test_jmp |
| instr_misc/instr_misc.nes | Passed — 04-dummy_reads_apu | Failed (1) — LDA abs,x |
| instr_misc/rom_singles/03-dummy_reads.nes | Passed — 03-dummy_reads | Failed (3) — LDA abs,x |
| instr_timing/instr_timing.nes | still running —  | Failed (1) — Instruction timing test |
| instr_timing/rom_singles/1-instr_timing.nes | Passed — Instruction timing test | Failed (3) — Instruction timing test |
| mmc3_test/1-clocking.nes | Passed — 1-clocking | Failed (3) — Should decrement when A12 is toggled via PPUADDR |
| mmc3_test/2-details.nes | Passed — 2-details | Failed (2) — Counter isn't working when reloaded with 255 |
| mmc3_test/3-A12_clocking.nes | Passed — 3-A12_clocking | Failed (4) — Should be clocked when A12 changes to 1 via PPUADDR write |
| mmc3_test/4-scanline_timing.nes | Passed — 4-scanline_timing | Failed (2) — Scanline 0 IRQ should occur later when $2000=$08 |
| mmc3_test/5-MMC3.nes | Passed — 5-MMC3 | Failed (2) — Should reload and set IRQ every clock when reload is 0 |
| mmc3_test/6-MMC6.nes | Failed (3) — IRQ shouldn't occur when reloading after counter normally  | Failed (2) — IRQ should be set when reloading to 0 after clear |
| mmc3_test_2/rom_singles/1-clocking.nes | Passed — 1-clocking | Failed (3) — Should decrement when A12 is toggled via PPUADDR |
| mmc3_test_2/rom_singles/2-details.nes | Passed — 2-details | Failed (2) — Counter isn't working when reloaded with 255 |
| mmc3_test_2/rom_singles/3-A12_clocking.nes | Passed — 3-A12_clocking | Failed (4) — Should be clocked when A12 changes to 1 via PPUADDR write |
| mmc3_test_2/rom_singles/4-scanline_timing.nes | Passed — 4-scanline_timing | Failed (2) — Scanline 0 IRQ should occur later when $2000=$08 |
| mmc3_test_2/rom_singles/5-MMC3.nes | Passed — 5-MMC3 | Failed (2) — Should reload and set IRQ every clock when reload is 0 |
| oam_read/oam_read.nes | Passed — ---------------- | Failed (1) — **************** |
| oam_stress/oam_stress.nes | still running —  | Failed (1) — **************** |
| ppu_open_bus/ppu_open_bus.nes | Passed — ppu_open_bus | Failed (3) — Decay value should become zero by one second |
| ppu_vbl_nmi/ppu_vbl_nmi.nes | still running — 03 | Failed (1) — VBL period is too short with BG off |
| ppu_vbl_nmi/rom_singles/01-vbl_basics.nes | Passed — 01-vbl_basics | Failed (7) — VBL period is too short with BG off |
| ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes | Passed — T+ 1 2 | Failed (1) — T+ 1 2 |
| ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes | Passed — 00 V | Failed (1) — 00 V |
| ppu_vbl_nmi/rom_singles/05-nmi_timing.nes | Passed — 00 4 | Failed (1) — 00 3 |
| ppu_vbl_nmi/rom_singles/06-suppression.nes | Passed — 00 - N | Failed (1) — 00 - N |
| ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes | Passed — 00 N | Failed (1) — 00 N |
| ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes | Passed — 03 - | Failed (1) — 03 - |
| ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes | Passed — 00 01 01 02 | Failed (2) — 04 |
| ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes | Passed — 08 08 09 07 | Failed (2) — 02 |
| sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes | Passed — T+ Clocks (decimal) | Failed (1) — T+ Clocks (decimal) |
| sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes | Passed — T+ Clocks (decimal) | Failed (1) — T+ Clocks (decimal) |

### Every ROM

`=` identical picture, `x` different, `.` no digest at that frame.

| ROM | Mapper | 60 | 300 | 1200 | Zaxnes | FCEUX |
|---|---|---|---|---|---|---|
| 240pee/240pee-bnrom.nes | 34 | = | = | = | loaded | loaded |
| 240pee/240pee.nes | 2 | = | = | = | loaded | loaded |
| MMC1_A12/mmc1_a12.nes | 1 | = | = | = | loaded | loaded |
| PaddleTest3/PaddleTest.nes | 0 | = | = | = | loaded | loaded |
| apu_mixer/dmc.nes | 0 | = | = | = | Passed | Passed |
| apu_mixer/noise.nes | 0 | = | = | = | Passed | Passed |
| apu_mixer/square.nes | 0 | = | = | = | Passed | Passed |
| apu_mixer/triangle.nes | 0 | = | = | = | Passed | Passed |
| apu_reset/4015_cleared.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| apu_reset/4017_timing.nes | 0 | x | x | x | Failed (129) | Failed (2) |
| apu_reset/4017_written.nes | 0 | x | x | x | Failed (129) | Failed (2) |
| apu_reset/irq_flag_cleared.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| apu_reset/len_ctrs_enabled.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| apu_reset/works_immediately.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| apu_test/apu_test.nes | 1 | = | x | x | Passed | Failed (1) |
| apu_test/rom_singles/1-len_ctr.nes | 0 | = | = | = | Passed | Passed |
| apu_test/rom_singles/2-len_table.nes | 0 | = | = | = | Passed | Passed |
| apu_test/rom_singles/3-irq_flag.nes | 0 | x | x | x | Passed | Failed (6) |
| apu_test/rom_singles/4-jitter.nes | 0 | x | x | x | Passed | Failed (2) |
| apu_test/rom_singles/5-len_timing.nes | 0 | x | x | x | Passed | Failed (2) |
| apu_test/rom_singles/6-irq_flag_timing.nes | 0 | x | x | x | Passed | Failed (2) |
| apu_test/rom_singles/7-dmc_basics.nes | 0 | = | = | = | Passed | Passed |
| apu_test/rom_singles/8-dmc_rates.nes | 0 | = | = | = | Passed | Passed |
| blargg_apu_2005.07.30/01.len_ctr.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/02.len_table.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/03.irq_flag.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/04.clock_jitter.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/05.len_timing_mode0.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/06.len_timing_mode1.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/07.irq_flag_timing.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/08.irq_timing.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/09.reset_timing.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/10.len_halt_timing.nes | 0 | x | x | x | loaded | loaded |
| blargg_apu_2005.07.30/11.len_reload_timing.nes | 0 | x | x | x | loaded | loaded |
| blargg_litewall/blargg_litewall-10c.nes | 1 | = | = | = | loaded | loaded |
| blargg_litewall/blargg_litewall-9.nes | 1 | = | = | = | loaded | loaded |
| blargg_litewall/litewall2.nes | 0 | = | = | = | loaded | loaded |
| blargg_litewall/litewall3.nes | 0 | = | = | = | loaded | loaded |
| blargg_litewall/litewall5.nes | 0 | x | x | x | loaded | loaded |
| blargg_nes_cpu_test5/cpu.nes | 1 | = | = | = | loaded | loaded |
| blargg_nes_cpu_test5/official.nes | 1 | = | = | = | loaded | loaded |
| blargg_ppu_tests_2005.09.15b/palette_ram.nes | 0 | = | = | = | loaded | loaded |
| blargg_ppu_tests_2005.09.15b/power_up_palette.nes | 0 | = | = | = | loaded | loaded |
| blargg_ppu_tests_2005.09.15b/sprite_ram.nes | 0 | x | x | x | loaded | loaded |
| blargg_ppu_tests_2005.09.15b/vbl_clear_time.nes | 0 | = | = | = | loaded | loaded |
| blargg_ppu_tests_2005.09.15b/vram_access.nes | 0 | = | = | = | loaded | loaded |
| branch_timing_tests/1.Branch_Basics.nes | 0 | = | = | = | loaded | loaded |
| branch_timing_tests/2.Backward_Branch.nes | 0 | = | = | = | loaded | loaded |
| branch_timing_tests/3.Forward_Branch.nes | 0 | = | = | = | loaded | loaded |
| cpu_dummy_reads/cpu_dummy_reads.nes | 3 | x | x | x | loaded | loaded |
| cpu_dummy_writes/cpu_dummy_writes_oam.nes | 0 | = | x | x | Passed | Failed (2) |
| cpu_dummy_writes/cpu_dummy_writes_ppumem.nes | 0 | = | = | = | Passed | Passed |
| cpu_exec_space/test_cpu_exec_space_apu.nes | 0 | x | = | = | Passed | Passed |
| cpu_exec_space/test_cpu_exec_space_ppuio.nes | 0 | x | x | x | Passed | Failed (5) |
| cpu_interrupts_v2/cpu_interrupts.nes | 1 | x | x | x | Passed | loaded |
| cpu_interrupts_v2/rom_singles/1-cli_latency.nes | 0 | = | = | = | Passed | Passed |
| cpu_interrupts_v2/rom_singles/2-nmi_and_brk.nes | 0 | = | x | x | Passed | Failed (1) |
| cpu_interrupts_v2/rom_singles/3-nmi_and_irq.nes | 0 | = | x | x | Passed | Failed (1) |
| cpu_interrupts_v2/rom_singles/4-irq_and_dma.nes | 0 | x | x | x | Passed | Failed (1) |
| cpu_interrupts_v2/rom_singles/5-branch_delays_irq.nes | 0 | x | x | x | Passed | Failed (1) |
| cpu_reset/ram_after_reset.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| cpu_reset/registers.nes | 0 | = | = | = | Failed (129) | Failed (129) |
| cpu_timing_test6/cpu_timing_test.nes | 0 | = | = | = | loaded | loaded |
| dmc_dma_during_read4/dma_2007_read.nes | 0 | x | x | x | loaded | loaded |
| dmc_dma_during_read4/dma_2007_write.nes | 0 | = | = | = | loaded | loaded |
| dmc_dma_during_read4/dma_4016_read.nes | 0 | x | x | x | loaded | loaded |
| dmc_dma_during_read4/double_2007_read.nes | 0 | x | x | x | loaded | loaded |
| dmc_dma_during_read4/read_write_2007.nes | 0 | = | = | = | loaded | loaded |
| dmc_tests/buffer_retained.nes | 0 | = | = | = | loaded | loaded |
| dmc_tests/latency.nes | 0 | = | = | = | loaded | loaded |
| dmc_tests/status.nes | 0 | = | = | = | loaded | loaded |
| dmc_tests/status_irq.nes | 0 | = | = | = | loaded | loaded |
| dpcmletterbox/dpcmletterbox.nes | 0 | = | = | = | loaded | loaded |
| exram/mmc5exram.nes | 5 | x | x | x | loaded | loaded |
| full_palette/flowing_palette.nes | 0 | x | x | x | loaded | loaded |
| full_palette/full_palette.nes | 0 | x | x | x | loaded | loaded |
| full_palette/full_palette_smooth.nes | 0 | x | x | x | loaded | loaded |
| instr_misc/instr_misc.nes | 1 | x | x | x | Passed | Failed (1) |
| instr_misc/rom_singles/01-abs_x_wrap.nes | 0 | = | = | = | Passed | Passed |
| instr_misc/rom_singles/02-branch_wrap.nes | 0 | = | = | = | Passed | Passed |
| instr_misc/rom_singles/03-dummy_reads.nes | 0 | x | x | x | Passed | Failed (3) |
| instr_misc/rom_singles/04-dummy_reads_apu.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/all_instrs.nes | 1 | = | = | = | still running | still running |
| instr_test-v3/official_only.nes | 1 | = | = | = | still running | still running |
| instr_test-v3/rom_singles/01-implied.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/02-immediate.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/03-zero_page.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/04-zp_xy.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/05-absolute.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/06-abs_xy.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/07-ind_x.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/08-ind_y.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/09-branches.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/10-stack.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/11-jmp_jsr.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/12-rts.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/13-rti.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/14-brk.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v3/rom_singles/15-special.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/all_instrs.nes | 1 | = | = | = | still running | still running |
| instr_test-v5/official_only.nes | 1 | = | = | = | still running | still running |
| instr_test-v5/rom_singles/01-basics.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/02-implied.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/03-immediate.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/04-zero_page.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/05-zp_xy.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/06-absolute.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/07-abs_xy.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/08-ind_x.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/09-ind_y.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/10-branches.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/11-stack.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/12-jmp_jsr.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/13-rts.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/14-rti.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/15-brk.nes | 0 | = | = | = | Passed | Passed |
| instr_test-v5/rom_singles/16-special.nes | 0 | = | = | = | Passed | Passed |
| instr_timing/instr_timing.nes | 1 | = | = | x | still running | Failed (1) |
| instr_timing/rom_singles/1-instr_timing.nes | 0 | = | = | x | Passed | Failed (3) |
| instr_timing/rom_singles/2-branch_timing.nes | 0 | = | = | = | Passed | Passed |
| m22chrbankingtest/0-127.nes | 22 | = | = | = | loaded | loaded |
| mmc3_irq_tests/1.Clocking.nes | 4 | x | x | x | loaded | loaded |
| mmc3_irq_tests/2.Details.nes | 4 | x | x | x | loaded | loaded |
| mmc3_irq_tests/3.A12_clocking.nes | 4 | x | x | x | loaded | loaded |
| mmc3_irq_tests/4.Scanline_timing.nes | 4 | x | x | x | loaded | loaded |
| mmc3_irq_tests/5.MMC3_rev_A.nes | 4 | x | x | x | loaded | loaded |
| mmc3_irq_tests/6.MMC3_rev_B.nes | 4 | x | x | x | loaded | loaded |
| mmc3_test/1-clocking.nes | 4 | x | x | x | Passed | Failed (3) |
| mmc3_test/2-details.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test/3-A12_clocking.nes | 4 | x | x | x | Passed | Failed (4) |
| mmc3_test/4-scanline_timing.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test/5-MMC3.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test/6-MMC6.nes | 4 | x | x | x | Failed (3) | Failed (2) |
| mmc3_test_2/rom_singles/1-clocking.nes | 4 | x | x | x | Passed | Failed (3) |
| mmc3_test_2/rom_singles/2-details.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test_2/rom_singles/3-A12_clocking.nes | 4 | x | x | x | Passed | Failed (4) |
| mmc3_test_2/rom_singles/4-scanline_timing.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test_2/rom_singles/5-MMC3.nes | 4 | x | x | x | Passed | Failed (2) |
| mmc3_test_2/rom_singles/6-MMC3_alt.nes | 4 | = | = | = | Failed (2) | Failed (2) |
| mmc5test/mmc5test.nes | 5 | x | x | x | loaded | loaded |
| mmc5test_v2/mmc5test.nes | 5 | = | = | = | loaded | loaded |
| nes15-1.0.0/nes15-NTSC.nes | 0 | = | = | = | loaded | loaded |
| nes15-1.0.0/nes15-PAL.nes | 0 | = | = | = | loaded | loaded |
| nes_instr_test/rom_singles/01-implied.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/02-immediate.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/03-zero_page.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/04-zp_xy.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/05-absolute.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/06-abs_xy.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/07-ind_x.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/08-ind_y.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/09-branches.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/10-stack.nes | 0 | = | = | = | Passed | Passed |
| nes_instr_test/rom_singles/11-special.nes | 0 | = | = | = | Passed | Passed |
| nestest.nes | 0 | = | = | = | loaded | loaded |
| nmi_sync/demo_ntsc.nes | 0 | x | x | x | loaded | loaded |
| nmi_sync/demo_pal.nes | 0 | x | x | x | loaded | loaded |
| nrom368/fail368.nes | 0 | = | = | = | loaded | loaded |
| nrom368/test1.nes | 0 | = | = | = | loaded | loaded |
| ny2011/ny2011.nes | 0 | = | = | = | loaded | loaded |
| oam_read/oam_read.nes | 0 | x | x | x | Passed | Failed (1) |
| oam_stress/oam_stress.nes | 0 | x | x | x | still running | Failed (1) |
| other/2003-test.nes | 0 | x | x | x | loaded | loaded |
| other/8bitpeoples_-_deadline_console_invitro.nes | 1 | x | x | x | loaded | loaded |
| other/BLOCKS.NES | 0 | = | x | x | loaded | loaded |
| other/BladeBuster.nes | 4 | = | = | = | loaded | loaded |
| other/CMC80s.NES | 3 | = | = | = | loaded | loaded |
| other/DROPOFF7.NES | 4 | = | = | = | loaded | loaded |
| other/Duelito.nes | 0 | x | x | x | loaded | loaded |
| other/FLAME.NES | 0 | = | = | = | loaded | loaded |
| other/GENIE.NES | 0 | x | x | x | loaded | loaded |
| other/GREYS.NES | 0 | = | = | = | loaded | loaded |
| other/LINUSMUS.NES | None | . | . | . | refused | refused |
| other/MOTION.NES | 0 | = | = | = | loaded | loaded |
| other/PCM.demo.wgraphics.nes | 2 | x | x | = | loaded | loaded |
| other/RasterChromaLuma.NES | 0 | x | x | x | loaded | loaded |
| other/RasterDemo.NES | 0 | x | x | x | loaded | loaded |
| other/RasterTest1.NES | 0 | x | x | x | loaded | loaded |
| other/RasterTest2.NES | 0 | x | x | x | loaded | loaded |
| other/RasterTest3.NES | 0 | = | = | = | loaded | loaded |
| other/RasterTest3a.NES | 0 | = | = | = | loaded | loaded |
| other/RasterTest3b.NES | 0 | = | = | = | loaded | loaded |
| other/RasterTest3c.NES | 0 | = | = | = | loaded | loaded |
| other/RasterTest3d.NES | 0 | = | = | = | loaded | loaded |
| other/RasterTest3e.NES | 0 | = | = | = | loaded | loaded |
| other/Retrocoders - Years behind.NES | 1 | = | = | = | loaded | loaded |
| other/S0.NES | 0 | = | = | = | loaded | loaded |
| other/SPRITE.NES | 0 | x | x | x | loaded | loaded |
| other/Sayoonara!.NES | 0 | x | x | x | loaded | loaded |
| other/SimpleParallaxDemo.nes | 0 | = | = | = | loaded | loaded |
| other/Streemerz_bundle.nes | 28 | = | = | = | loaded | loaded |
| other/TANESPOT.NES | 3 | = | x | = | loaded | loaded |
| other/TEST.NES | 0 | x | x | x | loaded | loaded |
| other/apocalypse.nes | 0 | x | x | x | loaded | loaded |
| other/blargg_litewall-2.nes | 0 | = | x | x | loaded | loaded |
| other/blargg_litewall-9.nes | 1 | = | = | = | loaded | loaded |
| other/demo jitter.nes | 0 | = | = | = | loaded | loaded |
| other/demo.nes | 0 | = | = | = | loaded | loaded |
| other/fceuxd.nes | 11 | x | x | x | loaded | loaded |
| other/firefly.nes | 5 | x | x | x | loaded | loaded |
| other/high-hopes.nes | 4 | = | = | = | loaded | loaded |
| other/logo (E).nes | 0 | x | x | x | loaded | loaded |
| other/manhole.nes | 0 | = | = | = | loaded | loaded |
| other/max-300.nes | 4 | = | x | = | loaded | loaded |
| other/midscanline.nes | 1 | = | = | = | loaded | loaded |
| other/minipack.nes | 0 | = | = | = | loaded | loaded |
| other/nescafe.nes | 0 | x | = | = | loaded | loaded |
| other/nestest.nes | 0 | = | = | = | loaded | loaded |
| other/nestopia.nes | 11 | x | x | x | loaded | loaded |
| other/new-game.nes | 0 | = | = | = | loaded | loaded |
| other/nintendulator.nes | 11 | x | x | x | loaded | loaded |
| other/oam3.nes | 7 | . | . | . | loaded | crashed |
| other/oc.nes | 0 | x | x | x | loaded | loaded |
| other/physics.0.1.nes | 0 | x | x | x | loaded | loaded |
| other/pulsar.nes | 1 | x | x | x | loaded | loaded |
| other/quantum_disco_brothers_by_wAMMA.nes | 3 | x | x | x | loaded | loaded |
| other/rastesam4.nes | 4 | x | x | x | loaded | loaded |
| other/read2004.nes | 0 | x | x | x | loaded | loaded |
| other/snow.nes | 0 | x | x | x | loaded | loaded |
| other/test001.nes | 11 | = | = | = | loaded | loaded |
| other/test28.nes | 28 | = | = | = | loaded | loaded |
| other/window2_ntsc.nes | 0 | x | x | x | loaded | loaded |
| other/window2_pal.nes | 0 | x | x | x | loaded | loaded |
| other/window_old_ntsc.nes | 0 | x | x | x | loaded | loaded |
| other/window_old_pal.nes | 0 | x | x | x | loaded | loaded |
| pal_apu_tests/01.len_ctr.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/02.len_table.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/03.irq_flag.nes | 0 | x | x | x | loaded | loaded |
| pal_apu_tests/04.clock_jitter.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/05.len_timing_mode0.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/06.len_timing_mode1.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/07.irq_flag_timing.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/08.irq_timing.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/10.len_halt_timing.nes | 0 | = | = | = | loaded | loaded |
| pal_apu_tests/11.len_reload_timing.nes | 0 | = | = | = | loaded | loaded |
| ppu_open_bus/ppu_open_bus.nes | 0 | = | x | x | Passed | Failed (3) |
| ppu_read_buffer/test_ppu_read_buffer.nes | 3 | = | x | x | still running | still running |
| ppu_vbl_nmi/ppu_vbl_nmi.nes | 1 | = | x | x | still running | Failed (1) |
| ppu_vbl_nmi/rom_singles/01-vbl_basics.nes | 0 | = | x | x | Passed | Failed (7) |
| ppu_vbl_nmi/rom_singles/02-vbl_set_time.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/03-vbl_clear_time.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/04-nmi_control.nes | 0 | = | = | = | Passed | Passed |
| ppu_vbl_nmi/rom_singles/05-nmi_timing.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/06-suppression.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/07-nmi_on_timing.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/08-nmi_off_timing.nes | 0 | = | x | x | Passed | Failed (1) |
| ppu_vbl_nmi/rom_singles/09-even_odd_frames.nes | 0 | x | x | x | Passed | Failed (2) |
| ppu_vbl_nmi/rom_singles/10-even_odd_timing.nes | 0 | x | x | x | Passed | Failed (2) |
| read_joy3/count_errors.nes | 3 | x | x | x | loaded | loaded |
| read_joy3/count_errors_fast.nes | 3 | x | x | x | loaded | loaded |
| read_joy3/test_buttons.nes | 3 | = | = | = | loaded | loaded |
| read_joy3/thorough_test.nes | 3 | = | = | = | loaded | loaded |
| scanline-a1/scanline.nes | 0 | x | x | x | loaded | loaded |
| scanline/scanline.nes | 0 | x | x | x | loaded | loaded |
| scrolltest/scroll.nes | 1 | = | = | = | loaded | loaded |
| soundtest/SNDTEST.NES | 0 | = | = | = | loaded | loaded |
| sprdma_and_dmc_dma/sprdma_and_dmc_dma.nes | 0 | = | x | x | Passed | Failed (1) |
| sprdma_and_dmc_dma/sprdma_and_dmc_dma_512.nes | 0 | = | x | x | Passed | Failed (1) |
| sprite_hit_tests_2005.10.05/01.basics.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/02.alignment.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/03.corners.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/04.flip.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/05.left_clip.nes | 0 | x | x | x | loaded | loaded |
| sprite_hit_tests_2005.10.05/06.right_edge.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/07.screen_bottom.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/08.double_height.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/09.timing_basics.nes | 0 | x | x | x | loaded | loaded |
| sprite_hit_tests_2005.10.05/10.timing_order.nes | 0 | = | = | = | loaded | loaded |
| sprite_hit_tests_2005.10.05/11.edge_timing.nes | 0 | = | = | = | loaded | loaded |
| sprite_overflow_tests/1.Basics.nes | 0 | = | = | = | loaded | loaded |
| sprite_overflow_tests/2.Details.nes | 0 | = | = | = | loaded | loaded |
| sprite_overflow_tests/3.Timing.nes | 0 | x | x | x | loaded | loaded |
| sprite_overflow_tests/4.Obscure.nes | 0 | x | x | x | loaded | loaded |
| sprite_overflow_tests/5.Emulator.nes | 0 | = | = | = | loaded | loaded |
| spritecans-2011/spritecans.nes | 0 | = | x | x | loaded | loaded |
| stars_se/StarsSE.NES | 0 | = | = | = | loaded | loaded |
| stomper/smwstomp.nes | 4 | x | x | x | loaded | loaded |
| stress/NEStress.NES | 0 | x | x | x | loaded | loaded |
| tutor/tutor.nes | 0 | x | x | x | loaded | loaded |
| tvpassfail/tv.nes | 0 | = | = | = | loaded | loaded |
| vaus-test/vaus-test.nes | 0 | = | = | = | loaded | loaded |
| vbl_nmi_timing/1.frame_basics.nes | 0 | = | x | x | loaded | loaded |
| vbl_nmi_timing/2.vbl_timing.nes | 0 | = | x | x | loaded | loaded |
| vbl_nmi_timing/3.even_odd_frames.nes | 0 | x | x | x | loaded | loaded |
| vbl_nmi_timing/4.vbl_clear_timing.nes | 0 | = | x | x | loaded | loaded |
| vbl_nmi_timing/5.nmi_suppression.nes | 0 | x | x | x | loaded | loaded |
| vbl_nmi_timing/6.nmi_disable.nes | 0 | x | x | x | loaded | loaded |
| vbl_nmi_timing/7.nmi_timing.nes | 0 | x | x | x | loaded | loaded |
| volume_tests/volumes.nes | 0 | = | = | = | loaded | loaded |
| window5/colorwin_ntsc.nes | 0 | x | x | x | loaded | loaded |
| window5/colorwin_pal.nes | 0 | x | x | x | loaded | loaded |

## The 98 original games

`3DSen/test-originals` — commercial games, never redistributed with Zaxnes.

| | |
|---|---|
| ROMs | 98 |
| loaded by both | 98 |
| loaded by Zaxnes only | 0 |
| loaded by FCEUX only | 0 |
| refused by both | 0 |
| took the FCEUX engine down | 0 |

### Identical pictures, by frame

| Frame | Identical | Of | Share |
|---|---|---|---|
| 1 | 98 | 98 | 100% |
| 60 | 65 | 98 | 66% |
| 300 | 61 | 98 | 62% |
| 600 | 64 | 98 | 65% |
| 1200 | 65 | 98 | 66% |

**Identical at 60, 300 *and* 1200: 44 of 98.** 54 differ at one or more.

The ones that differ, by mapper: **1** x22, **4** x11, **2** x10, **0** x8, **3** x3.

### Every ROM

`=` identical picture, `x` different, `.` no digest at that frame.

| ROM | Mapper | 60 | 300 | 1200 | Zaxnes | FCEUX |
|---|---|---|---|---|---|---|
| Adventure Island Classic [Europe][!].nes | 3 | x | = | = | loaded | loaded |
| Adventure Island [USA][!!].nes | 3 | x | = | = | loaded | loaded |
| Akumajou Dracula [Japan][!!].nes | 2 | = | = | x | loaded | loaded |
| Akumajou Dracula [World][Ja][!!].nes | 2 | = | = | x | loaded | loaded |
| Blaster Master [Europe][!].nes | 1 | x | = | x | loaded | loaded |
| Blaster Master [Japan][Virtual Console][!!].nes | 1 | = | = | = | loaded | loaded |
| Blaster Master [USA][!!].nes | 1 | = | = | = | loaded | loaded |
| Blaster Master [USA][Beta][1988-05-20][!!].nes | 1 | = | = | = | loaded | loaded |
| Blaster Master [USA][Europe][Virtual Console][!!].nes | 1 | = | = | = | loaded | loaded |
| Castlevania [Europe][!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [Europe][Virtual Console][!!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [USA][!!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [USA][Rev 1][!!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [USA][Rev 1][Virtual Console][!!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [World][!!].nes | 2 | x | x | x | loaded | loaded |
| Castlevania [World][Konami Collector's Series][Unl][!!].nes | 2 | x | x | x | loaded | loaded |
| Chou-WS-M [Japan][!!].nes | 1 | = | = | = | loaded | loaded |
| Excitebike [Europe][!].nes | 0 | = | = | x | loaded | loaded |
| Excitebike [Europe][Virtual Console][!!].nes | 0 | = | = | x | loaded | loaded |
| Excitebike [Japan][USA][!!].nes | 0 | = | = | x | loaded | loaded |
| Excitebike [Japan][USA][GameCube Edition][!!].nes | 0 | = | = | x | loaded | loaded |
| Excitebike [Japan][USA][Virtual Console][!!].nes | 0 | = | = | x | loaded | loaded |
| Excitebike [World][Switch Online][!!].nes | 0 | = | = | x | loaded | loaded |
| Final Fantasy [Japan][!!].nes | 1 | x | x | = | loaded | loaded |
| Final Fantasy [Japan][Rev 1][!!].nes | 1 | x | x | = | loaded | loaded |
| Final Fantasy [Japan][Rev 1][3DS Virtual Console][!!].nes | 1 | x | x | = | loaded | loaded |
| Final Fantasy [Japan][Rev 1][Wii and Wii U Virtual Console][!!].nes | 1 | x | x | = | loaded | loaded |
| Final Fantasy [USA][!].nes | 1 | = | x | = | loaded | loaded |
| Final Fantasy [USA][Beta][!!].nes | 1 | = | x | = | loaded | loaded |
| Final Fantasy [USA][Virtual Console][!!].nes | 1 | = | x | = | loaded | loaded |
| Kiddy Sun in Fantasia [Asia][!!].nes | 3 | = | = | = | loaded | loaded |
| Mega Man 2 [Europe][!].nes | 1 | = | x | = | loaded | loaded |
| Mega Man 2 [Europe][Virtual Console][!!].nes | 1 | = | x | = | loaded | loaded |
| Mega Man 2 [USA][!!].nes | 1 | = | x | = | loaded | loaded |
| Mega Man 2 [USA][Mega Man iam8bit 30th Anniversary][Aftermarket][Unl][!!].nes | 2 | = | x | = | loaded | loaded |
| Mega Man 2 [USA][Virtual Console][!!].nes | 1 | = | x | = | loaded | loaded |
| Mega Man 2 [World][!!].nes | 1 | = | x | = | loaded | loaded |
| Mega Man 3 [Europe][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [Europe][Rev 1][!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [Europe][Rev 1][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [USA][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [USA][Beta][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [USA][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 3 [World][Unl][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [Europe][!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [Europe][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [USA][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [USA][Rev 1][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [USA][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 4 [World][Unl][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 5 [Europe][!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 5 [Europe][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 5 [USA][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 5 [USA][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 5 [World][Unl][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 6 [USA][!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man 6 [World][Unl][!!].nes | 4 | = | = | = | loaded | loaded |
| Mega Man [Europe][!].nes | 2 | = | = | = | loaded | loaded |
| Mega Man [USA][!!].nes | 2 | = | = | = | loaded | loaded |
| Mega Man [USA][Virtual Console][!!].nes | 2 | = | = | = | loaded | loaded |
| Mega Man [World][Unl][!!].nes | 2 | = | = | = | loaded | loaded |
| Metal Force [Korea][Unl][!].nes | 156 | = | = | = | loaded | loaded |
| Metroid [Europe][!].nes | 1 | x | x | x | loaded | loaded |
| Metroid [Europe][Virtual Console][!!].nes | 1 | x | x | x | loaded | loaded |
| Metroid [USA][!!].nes | 1 | x | x | x | loaded | loaded |
| Metroid [USA][Virtual Console][!!].nes | 1 | x | x | x | loaded | loaded |
| Ninja Gaiden III-TASoD [USA][!].nes | 4 | = | = | = | loaded | loaded |
| Ninja Gaiden III-TASoD [USA][Wii U Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Ninja Gaiden III-TASoD [USA][Wii Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Ninja Ryuuken Den III-YnH [Japan][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman 2-DWnN [Japan][!!].nes | 1 | = | x | = | loaded | loaded |
| Rockman 2-DWnN [Japan][Virtual Console][!!].nes | 1 | = | x | = | loaded | loaded |
| Rockman 3-DWnS [Japan][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman 3-DWnS [Japan][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman 4-AY [Japan][!!].nes | 4 | x | = | = | loaded | loaded |
| Rockman 4-AY [Japan][Rev 1][!!].nes | 4 | x | = | = | loaded | loaded |
| Rockman 4-AY [Japan][Virtual Console][!!].nes | 4 | x | = | = | loaded | loaded |
| Rockman 5-BnW [Japan][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman 5-BnW [Japan][Virtual Console][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman 6-SSnT [Japan][!!].nes | 4 | = | = | = | loaded | loaded |
| Rockman [Japan][En][!!].nes | 2 | = | = | = | loaded | loaded |
| Rockman [Japan][Virtual Console][!!].nes | 2 | = | = | = | loaded | loaded |
| Super Mario Bros. 3 [Europe][!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [Europe][Virtual Console][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [Japan][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [Japan][Rev 1][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [Japan][Rev 1][Virtual Console][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [USA][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [USA][Rev 1][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. 3 [USA][Rev 1][Virtual Console][!!].nes | 4 | x | x | x | loaded | loaded |
| Super Mario Bros. [Europe][!].nes | 0 | = | = | = | loaded | loaded |
| Super Mario Bros. [World][!!].nes | 0 | = | = | x | loaded | loaded |
| Super Mario Bros.-2A [Australia][Virtual Console][!!].nes | 0 | = | = | = | loaded | loaded |
| Super Mario Bros.-2A [Japan][En][Virtual Console][!!].nes | 0 | = | = | x | loaded | loaded |
| Takahashi Meijin no Bouken-j [Japan][!!].nes | 3 | x | = | = | loaded | loaded |
| Zelda II - The Adventure of Link (Europe).nes | 1 | x | x | x | loaded | loaded |
| Zelda II - The Adventure of Link (USA).nes | 1 | x | x | x | loaded | loaded |
| Zelda II-TAoL [Europe][Rev 2][!].nes | 1 | x | x | x | loaded | loaded |


## FCEUX-6 tile parity

Everything above compares the two engines' **2D pictures**. This section compares what the **3D**
path actually consumes: the per-frame tile records and the palette that travel with them
(REQ-CORE-033/034/035/045). It is the deliverable of lot **FCEUX-6 step 8**, and the question it
answers is the owner's: *does one 3dn profile give the same 3D picture whichever engine produced
the records?*

Produced by `zax-harness fceux-tiles` (both engines side by side, field by field) and, for the
pictures, by `zax-harness render` fed each engine's records through the same stateless `--frames`
path, scored with `zax-harness compare --tolerance 0`. The driving is `fceux-tiles`'s: 1200 or 3000
frames, no button pressed, a reset after loading, and **the zero RAM fill** — not FCEUX's
`Default` pattern, because `FCEU_MemoryRand` fills the work RAM *and* a CHR-RAM cart's CHR with
`00 00 00 00 FF FF FF FF` while `zax-nes` clears both (REQ-CORE-004), and the two machines are
otherwise not comparable. The figures in the sections above were taken with the `Default` fill and
a lot that revisits them should expect them to improve for the same reason.

### Every frame classified

Each compared frame is `identical` (records and palette equal), `capture` (the two 2D pictures are
identical, so the machines agree about what is on screen and only the capture disagrees — **ours**)
or `emulation` (the pictures already differ, so the two cores are not showing the same frame —
FCEUX against Zaxnes, out of this lot's scope).

| | 98 originals, frames 60/300/1200/3000 | 291 public ROMs, frames 60/300 |
|---|---|---|
| ROMs identical at every compared frame, **before** FCEUX-6 | 7 / 98 | 102 / 289 |
| ROMs identical at every compared frame, **after** | **37 / 98** | **176 / 291** |
| frames `identical`, before → after | 85 → **238** / 392 | 234 → **376** / 578 |
| frames `capture`, before → after | 176 → **34** / 392 | 122 → **3** / 578 |
| frames `emulation` | 131 → 120 | 222 → 199 |

The `capture` column is the one the lot owned, and it fell from 298 frames to 37. **None of the 37
is a capture defect:** `fceux-tiles` names a cause for each and reports the count it cannot
explain, which is **0** on both sets.

| Cause | Originals | Public | Whose |
|---|---|---|---|
| `blank-screen` — Zaxnes's picture is one colour, so two identical pictures prove almost nothing | 12 | 2 | undecidable by the pixel test |
| `oam-rotated-by-k` — every sprite matches after rotating FCEUX's OAM number by a constant | 8 | 1 | emulation: the `$4014` transfer began at a different `$2003`. REQ-CORE-043 numbers a record after its OAM entry, so a capture can neither cause nor cure it |
| `sprite-oam-slot` — every differing record is a sprite differing on its `index`, with no one rotation | 8 | 0 | emulation: the game put that sprite in another OAM entry |
| `palette-only` — no record differs; the palettes do | 6 | 1 | emulation |
| **`unexplained`** | **0** | **0** | |

Every one of the 37 also carries the frame at which the two engines' pictures *first* differed —
3, 4, 5, 7, 8 or 10 in every case — so their agreeing again at frame 300, 1200 or 3000 is a
coincidence rather than evidence.

### The palette is part of the answer

A frame batch is the records **and** the 32 palette bytes, and the 3D renderer reads both: it
colours a mesh from the palette entry a record's sub-palette selects. Comparing the records alone
is therefore not enough, and *Final Fantasy* frame 300 is the case that proves it: **all 960
records equal on all ten fields**, `$3F0B` **32 on Zaxnes and 0 on FCEUX**, the two 2D pictures
**identical** (no pixel on screen uses that entry), and **0.69 % of the 3D picture different**.

| | 98 originals | 291 public ROMs |
|---|---|---|
| frames the records call identical and the palette does not | **21** | **8** |
| ROMs with at least one such frame | 14 | 6 |
| the entry most often involved | `$3F0B`, 14 frames (every *Final Fantasy* dump at 60 and 300) | spread over most of the 32 |

### The 3D picture, per game

One dump per profile, frames 300 and 1200, both engines' records through the same renderer with the
same profile. `mesh` is `residentMeshes` and `tri` is `triangles` from `render.json`; `pixels` is
the share identical at **tolerance 0**; `batch=` is whether the records *and* palette handed to the
renderer were byte-identical.

| Profile | frame | batch= | mesh z/f | triangles z/f | pixels |
|---|---|---|---|---|---|
| Blaster Master | 300 | **yes** | 218/218 | 25 376/25 376 | **100.000 %** |
| Blaster Master | 1200 | **yes** | 22/22 | 20 464/20 464 | **100.000 %** |
| Excitebike | 300 | **yes** | 52/52 | 5 916/5 916 | **100.000 %** |
| Excitebike | 1200 | no | 89/91 | 39 862/39 966 | 86.997 % |
| Final Fantasy | 300 | no (palette only) | 69/69 | 31 976/31 976 | 99.306 % |
| Final Fantasy | 1200 | **yes** | 69/69 | 31 976/31 976 | **100.000 %** |
| Mega Man 1 | 300 | **yes** | 234/234 | 19 292/19 292 | **100.000 %** |
| Mega Man 1 | 1200 | **yes** | 234/234 | 19 292/19 292 | **100.000 %** |
| Mega Man 2 | 300 | **yes** | 23/23 | 7 232/7 232 | **100.000 %** |
| Mega Man 2 | 1200 | **yes** | 116/116 | 43 404/43 404 | **100.000 %** |
| Mega Man 3 | 300 | **yes** | 0/0 | 0/0 | **100.000 %** (blank on both) |
| Mega Man 3 | 1200 | **yes** | 147/147 | 11 512/11 512 | **100.000 %** |
| Metroid | 300 | **yes** | 194/194 | 882 048/882 048 | **100.000 %** |
| Metroid | 1200 | **yes** | 72/72 | 878 680/878 680 | **100.000 %** |
| Ninja Gaiden III | 300 | **yes** | 74/74 | 12 252/12 252 | **100.000 %** |
| Ninja Gaiden III | 1200 | **yes** | 33/33 | 13 984/13 984 | **100.000 %** |
| Super Mario Bros. | 300 | **yes** | 102/102 | 28 822/28 822 | **100.000 %** |
| Super Mario Bros. | 1200 | no | 52/52 | 25 836/25 836 | 89.647 % |
| Adventure Island | 300 | no | 119/119 | 14 460/14 460 | 99.999 % |
| Adventure Island | 1200 | no | 119/119 | 14 460/14 460 | 99.999 % |
| Super Mario Bros. 3 | 300 | no | 21/21 | 13 216/13 216 | 98.551 % |
| Super Mario Bros. 3 | 1200 | no | 305/305 | 34 780/34 780 | 98.532 % |
| Castlevania | 300 | no | 253/254 | 51 112/51 096 | 99.563 % |
| Castlevania | 1200 | no | 93/93 | 29 188/28 840 | 89.785 % |
| Zelda II | 300 | no | 29/29 | 9 560/9 560 | 89.938 % |
| Zelda II | 1200 | no | 120/120 | 17 928/17 928 | 89.935 % |

Three readings:

* **every cell whose batch is byte-identical renders 100.000 % identical pixels at tolerance 0**,
  with identical mesh counts, triangles, opaque tiles and draw calls — 14 of the 26. The renderer
  is a pure function of (records, palette, profile), so the 3D answer for a game follows from the
  record answer with nothing left over;
* **mesh counts are identical in 23 of the 26 cells**, the exceptions being *Castlevania* f300
  (253/254, where the record counts themselves are 1024/1028) and *Excitebike* f1200 (89/91). The
  same 3dn profile therefore builds the same scene on both engines even on several games whose
  pictures differ;
* where the records differ the 3D loses 1 % to 13 %, and the loss tracks the cause: one palette
  entry 0.7 %, an OAM rotation 0.001 %, the `$2007`-while-rendering rule 1.5 % (*Super Mario Bros.
  3*) to 10 % (*Zelda II*), sprites in different places 10 % to 13 %.

**Not measured:** the other 65 dumps. `CLAUDE.md` forbids long GPU campaigns, so this is one dump
per game — 26 cells, 78 renders — and the per-dump answer is derivable from the sweep plus the
first reading above. Frames 300 and 1200 with no button pressed are a title screen or a transition
on several of these games (*Mega Man 3* frame 300 is blank on both engines), so this table says
whether the two engines agree, not how a game looks in play.

**One decision is still the owner's.** *Super Mario Bros. 3* and *Zelda II* cannot agree without
it: FCEUX applies the hardware rule for a `$2007` access made while rendering unconditionally,
while `zax-nes` keeps it behind the non-default feature `ppu-2007-render-read` (Q-139) precisely
because it moves those two games' pictures. With the feature on, the two engines' records for
*Super Mario Bros. 3* become identical row for row. No change was made to `zax-nes`.
