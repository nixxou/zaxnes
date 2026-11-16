# FDS — the Famicom Disk System, with the player's BIOS

Branch `feat/fds` (from `feat/mappers-p` 414fbbc2), worktree `G:/3DSen/wt-mappers`.
Owner's request (2026-11-16): "on pourrait intégrer le support famicom (en laissant l'utilisateur
fournir le bios)" — then "go" on the plan.

## Resume note (read first after a crash)

* Same rules as MAPPERS-P: one feature (`board-fds`), sources Mesen 2 (port), nesdev; never 3dSen;
  safe Rust; clippy `-D warnings`; commit often; never push or merge.
* The BIOS is never shipped: the player puts `disksys.rom` (8 KiB) beside the game, beside the
  program, or in its `firmware` folder.

## Design

* Core (`zax-nes`): `cartridges/mappers/fds.rs` (port of Mesen 2 `Fds`, `FdsAudio`, `ModChannel`,
  `BaseFdsChannel`, `FdsLoader`, `FdsInputButtons`), an `IrqP` kind `M20` holding it; the image
  is packaged as NES 2.0 mapper 20 (`rom::package_fds`: the BIOS twice as PRG, 32 KiB work RAM,
  8 KiB CHR-RAM, the disk as the misc ROM, submapper 1 for a `.qd`). `registry::is_study_box`
  accepts 20 only with a disk image in its misc ROM, so a bare iNES 20 stays refused.
* Disk changes: `NesCore::{disk_sides, disk_side, insert_disk, eject_disk, switch_disk_side,
  insert_next_disk, set_disk_auto_insert}`; the automatic insertion resolves at the end of the
  frame in which the BIOS read `$E445` (`NesCore::fds_end_frame`, cold).
* Saves: what the drive writes goes to `<rom>.flash` (`ZXFD` + changed 256-byte blocks) through
  the flash-save path of MAPPERS-P; savestates carry the written disk.
* App: `.fds`/`.qd` in the file filters and the handheld list, `rom_file::read_disk` with the BIOS
  lookup shared with the Study Box, and three commands in the Emulation menu (switch side, next
  disk, eject), greyed out without a drive.

## Steps

| step | state |
| --- | --- |
| core port, packaging, registry | done (9e15b723) |
| core tests (`tests/fds_tests.rs`: load, RAM/BIOS, timer IRQ, drive bytes, audio, saves, auto-insert) | done |
| app: loading with `disksys.rom`, filters, commands, tests | done (c8e7de75) |
| docs: FIDELITY-LEADS J38, BOARD-FEATURES, MF-6 inventory | done |
| perf against `e4596884` (clean builds, 9202e28d) | done: core mm2 +0.003 %, zelda2 +0.002 %, smb3 +0.002 %; totals within ±0.002 % |
| real BIOS + `fdsirqtests` | waits for the owner's `disksys.rom` |
