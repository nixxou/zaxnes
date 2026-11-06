//! Maps an iNES mapper number to its board (port of `MapperRegistry`, REQ-CORE-100,
//! REQ-CORE-102).
//!
//! The registry only advertises the boards that are implemented, so that a ROM is never
//! accepted and then found unplayable. The table is the one of REQ-CORE-100; several numbers
//! share a board when they are the same silicon differently wired (9 and 10, 21/22/23/25/27,
//! 87 and 101). 156 is outside that table: it runs Metal Force (Korea), a widescreen target.

use super::mappers::Nrom;
use super::mappers::{
    Action52, Action53, AxRom, Bandai74161, BandaiFcg, Bf9096, Caltron6In1, CnRom, CnRomProtect,
    Codemasters, ColorDreams, CpRom, Daou306, Fme7, GxRom, IremG101, IremTamS1, JalecoJ87,
    JalecoJf11, JalecoSs88006, Mapper015, Mapper034, Mapper078, McAcc, Mmc1, Mmc2, Mmc3, Mmc5,
    Multicart225, Namco163, NamcoVariant, Namcot108, Rambo1, SmallBoard, Sunsoft1, Sunsoft2,
    TaitoTc0190, UnRom512, UxRom, Vrc1, Vrc2Vrc4, Vrc6, Vrc7,
};
use super::{BankWindows, Board, CartridgeMemory};
use crate::rom::InesHeader;

/// Mapper numbers this build can run, in ascending order (REQ-CORE-102). 116 is optional in
/// REQ-CORE-101 and not implemented.
pub const SUPPORTED_MAPPERS: &[i32] = &[
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 19, 21, 22, 23, 24, 25, 26, 27, 28, 30, 32, 33,
    34, 41, 64, 66, 69, 70, 71, 75, 76, 78, 85, 87, 88, 89, 93, 94, 95, 97, 101, 118, 140, 152,
    154, 155, 156, 180, 184, 185, 206, 210, 225, 228, 232,
];

/// Whether `mapper` is implemented.
#[must_use]
pub fn is_supported(mapper: i32) -> bool {
    SUPPORTED_MAPPERS.contains(&mapper)
}

/// The sizes the *board* dictates for the volatile memories, settled once at load time
/// (REQ-CORE-100).
///
/// `prg_ram` and `chr_ram` are exact: the board carries that much RAM and the header cannot
/// say otherwise. `min_prg_ram` is the floor for boards that bank *more* than the header's
/// 8 KiB without fixing a size (MMC1 SOROM and SXROM, MMC5).
// The three fields name the three memories; the `_ram` they share is the point.
#[allow(clippy::struct_field_names)]
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct BoardMemorySizes {
    pub prg_ram: Option<usize>,
    pub chr_ram: Option<usize>,
    pub min_prg_ram: usize,
}

/// Which of the two mapper-34 boards a dump is (nesdev, "INES Mapper 034"): "Submapper 1
/// denotes the NINA-001/NINA-002 circuit board", "Submapper 2 denotes the I-IM/BNROM circuit
/// board", and "In the absence of valid submapper information, mapper 34 .NES files should be
/// considered BNROM when the CHR-ROM size is 0-8 KiB, and NINA-001/NINA-002 when the CHR-ROM
/// size is above 8 KiB". The submapper takes precedence over the size (MR-34-1).
#[must_use]
pub(crate) fn mapper_34_is_nina(header: &InesHeader) -> bool {
    if header.is_nes20 && (header.submapper == 1 || header.submapper == 2) {
        return header.submapper == 1;
    }

    header.chr_rom_size > 8 * 1024
}

/// The volatile-memory sizes of `mapper`'s board for `header`.
#[must_use]
pub(crate) fn board_memory_sizes(mapper: i32, header: &InesHeader) -> BoardMemorySizes {
    match mapper {
        // MR-34-2, nesdev ("INES Mapper 034"): the NINA boards have "8 KiB of unbanked
        // PRG-RAM" at $6000-$7FFF and the BNROM has none, so that window is open bus there.
        34 if !mapper_34_is_nina(header) => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // MR-4-6, nesdev ("MMC6"): the chip "contains 1 KB of internal PRG RAM", at "CPU
        // $7000-$7FFF: 1 KB PRG RAM, mirrored". The header of a StarTropics dump describes
        // the 8 KiB every other mapper-4 board has.
        // Mapper 118 shares the MMC3's board but never its submapper 1: a TxSROM is not an
        // MMC6, so this arm is on mapper 4 alone.
        4 if header.is_nes20 && header.submapper == 1 => BoardMemorySizes {
            prg_ram: Some(Mmc3::MMC6_PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        5 => BoardMemorySizes {
            min_prg_ram: Mmc5::MAX_PRG_RAM_SIZE,
            ..BoardMemorySizes::default()
        },
        // MR-13-1, nesdev ("CPROM") and REQ-CORE-100's mapper table: the board carries "16 KiB
        // RAM (in two 8KiB RAMs)", not the header's 32 KiB default.
        13 => BoardMemorySizes {
            chr_ram: Some(CpRom::CHR_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // The Namco 163 carries the four 2 KiB windows of $6000-$7FFF whatever the header
        // says, plus the chip's own 128 bytes behind them -- that RAM lives at the end of
        // PRG-RAM so that REQ-CORE-095's single battery slice carries it, which is also
        // Mesen's `.sav` layout. See `Namco163`'s own note.
        19 => BoardMemorySizes {
            prg_ram: Some(Namco163::PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 210"): the Namcot 175 has "0-8 KiB" of PRG-RAM and the 340
        // "contains no WRAM". Neither has the chip's internal RAM.
        210 => BoardMemorySizes {
            prg_ram: Some(
                match Namco163::variant_of(210, if header.is_nes20 { header.submapper } else { 0 })
                {
                    NamcoVariant::Namco340 => 0,
                    _ => 4 * 0x800,
                },
            ),
            ..BoardMemorySizes::default()
        },
        // The three boards that carry no PRG-RAM at all, each on its page's own words:
        //
        // * 30, MR-30-2, nesdev ("UNROM 512"): "PRG RAM capacity: None";
        // * 140 ("INES Mapper 140"): "Regs lie at $6000-7FFF, so there's no SRAM";
        // * 184 ("INES Mapper 184"): "This mapper precludes iNES-default RAM because the
        //   register is mapped into $6000-7FFF".
        //
        // The last two answer that window themselves through `Mapper::intercept_cpu_write`, so
        // the RAM must not exist behind it. **Mapper 97 is deliberately absent**, although §4.6
        // of the inventory grouped it with those two: its page documents no PRG-RAM but does not
        // say the board has none, and taking RAM away from an image that uses it is the worse of
        // the two errors.
        30 | 140 | 184 => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        // nesdev ("INES Mapper 206"), the Namco 108 family: `PRG RAM: None`, the page adding
        // "with the rare exception" of the *Magical Puzzle Popils* prototype, which carries
        // "8 KiB battery-backed". So the window is open bus on these five numbers unless the
        // image declares a battery, which is the one board the page excepts.
        76 | 88 | 95 | 154 | 206 if !header.has_battery => BoardMemorySizes {
            prg_ram: Some(0),
            ..BoardMemorySizes::default()
        },
        _ => BoardMemorySizes::default(),
    }
}

/// Where in PRG-RAM the battery-backed region begins on `mapper`'s board, once the board has
/// `prg_ram_size` bytes of it. Everything from there to the end is what a host persists.
///
/// MR-1-5, nesdev ("MMC1"): "Even if the SOROM and SZROM boards utilizes a battery, it is
/// connected to only one PRG-RAM chip. The first RAM chip will not retain its data, but the
/// second one will." SOROM and SZROM are the 16 KiB boards, so the chip that retains is the
/// second 8 KiB.
///
/// **AMD-061 §8 closes Q-113 on the reading MF-1 chose**, and in these words: SOROM and SZROM
/// put "seule la seconde puce, $2000-$3FFF de la PRG-RAM" on the battery; **SXROM** carries
/// "une seule puce de 32 Kio, entièrement sur pile", so `battery_ram` exposes all 32 KiB and
/// *Final Fantasy I·II* keeps its saves; and "une image dont la taille de RAM ne permet pas de
/// distinguer la carte garde toute sa PRG-RAM sur pile". The function below is exactly that:
/// a 16 KiB mapper-1 RAM retains its second half, every other size retains all of itself. No
/// code changed in MF-2 — the amendment confirmed what was already here — and
/// `mr_1_5_only_the_second_sorom_ram_chip_is_battery_backed` and
/// `mr_1_5_an_sxrom_keeps_its_whole_ram_battery_backed` pin both halves.
#[must_use]
pub(crate) const fn battery_offset(mapper: i32, prg_ram_size: usize) -> usize {
    if mapper == 1 && prg_ram_size == 2 * BankWindows::PRG_WINDOW_SIZE {
        BankWindows::PRG_WINDOW_SIZE
    } else {
        0
    }
}

/// Whether `mapper`'s board lets the ROM drive the data bus during a write to $8000-$FFFF, so
/// that the value the register latches is "the bitwise AND of the written value and the value
/// in ROM" (MR-BUS-1, MR-66-1, the bus-conflict half of MR-34-2). Decided once at load.
///
/// Sources, all nesdev: *NES 2.0 submappers* gives mappers 2, 3 and 7 the AND on **submapper
/// 2** and says "Bus conflicts do not occur" on submapper 1; *INES Mapper 011* says "All
/// commercially-released games are subject to bus conflicts (submapper 2)", which with the
/// *Color Dreams* board page (only the *Free Fall* prototype has conflict-prevention logic)
/// makes it unconditional there; *UNROM 512* makes submapper 2 "Bus conflicts present" and for
/// submapper 0 says "The Battery bit must be specified if the game relies on bus conflicts not
/// being present", so a submapper-0 dump without battery has them; *INES Mapper 034* gives
/// BNROM's register "the value being written bitwise-AND'd with the PRG-ROM content", and only
/// BNROM — the NINA boards have conflict-free latches; and the `GxROM` board table gives mapper
/// 66 conflicts (MR-66-1), which Mesen does not implement.
///
/// **A submapper-0 dump of mappers 2, 3 and 7 has none**, which is what
/// `mr_bus_1_uxrom_without_a_submapper_has_no_conflict` pins and what MR-1b asked be kept: the
/// submapper is the gate, and most dumps of those boards carry no submapper at all.
#[must_use]
pub(crate) fn has_bus_conflicts(mapper: i32, header: &InesHeader) -> bool {
    let submapper = if header.is_nes20 { header.submapper } else { 0 };
    match mapper {
        2 | 3 | 7 => submapper == 2,
        // *INES Mapper 180* gives the board "Bus conflicts: Yes" with no submapper clause: the
        // AND logic that makes its fixed bank the first one is what puts the ROM on the bus
        // during a write.
        // *INES Mapper 078* says the same of its board, "a CNROM and a UNROM" in discrete
        // logic with nothing to stop the ROM driving the bus.
        // *INES Mapper 089* and *INES Mapper 093* both carry the page's own "BUS CONFLICTS"
        // line against the register at $8000-$FFFF (MF-6a step 2).
        // *INES Mapper 070* and *INES Mapper 152* file the Bandai 74161/7432 board under
        // "Mappers with bus conflicts" too (MF-6a step 3).
        // Mesen 2 gives mapper 185 the CNROM board's conflicts (`CnromProtect.h:21`), which
        // the *CNROM* board page describes for the family (MF-6a step 4).
        11 | 66 | 70 | 78 | 89 | 93 | 152 | 180 | 185 => true,
        30 => submapper == 2 || (submapper == 0 && !header.has_battery),
        34 => !mapper_34_is_nina(header),
        _ => false,
    }
}

/// Builds the board of `mapper` for `memory`, or `None` when it is not implemented.
#[must_use]
pub(crate) fn create_board(mapper: i32, memory: &CartridgeMemory) -> Option<Board> {
    Some(match mapper {
        0 => Board::Nrom(Nrom),
        // MF-6a step 4: mapper 155 is the MMC1 on a board with no PRG-RAM disable wire.
        1 | 155 => Board::Mmc1(Mmc1::new(memory)),
        // MF-6a step 4: mapper 94 is the third wiring of the UxROM board, its bank field
        // shifted two bits (nesdev, "INES Mapper 094").
        2 | 94 | 180 => Board::UxRom(UxRom::new(memory, mapper)),
        3 => Board::CnRom(CnRom::new(memory)),
        // MR-4-3: mapper 4 submapper 3 is Acclaim's MC-ACC, the same board with a ÷8 prescaler
        // on A12's falling edges. It is the same code, monomorphised with the difference as a
        // `const` parameter, so the MMC3 proper pays nothing (see `Mmc3Board`).
        4 if memory.rom().header().is_nes20 && memory.rom().header().submapper == 3 => {
            Board::McAcc(McAcc::new(memory, mapper))
        }
        4 | 118 => Board::Mmc3(Mmc3::new(memory, mapper)),
        5 => Board::Mmc5(Mmc5::new(memory)),
        7 => Board::AxRom(AxRom::new(memory)),
        9 | 10 => Board::Mmc2(Mmc2::new(memory, mapper)),
        11 => Board::ColorDreams(ColorDreams::new(memory)),
        13 => Board::CpRom(CpRom::new(memory)),
        15 => Board::Mapper015(Mapper015::new(memory)),
        16 => Board::BandaiFcg(BandaiFcg::new(memory)),
        18 => Board::JalecoSs88006(JalecoSs88006::new(memory)),
        21 | 22 | 23 | 25 | 27 => Board::Vrc2Vrc4(Vrc2Vrc4::new(memory, mapper)),
        // Mapper 26 is the VRC6b, the same chip with A0 and A1 swapped (nesdev, "VRC6").
        19 | 210 => Board::Namco163(Namco163::new(memory, mapper)),
        24 | 26 => Board::Vrc6(Vrc6::new(memory, mapper)),
        28 => Board::Action53(Action53::new(memory)),
        30 => Board::UnRom512(UnRom512::new(memory)),
        32 => Board::IremG101(IremG101::new(memory)),
        34 => Board::Mapper034(Mapper034::new(memory)),
        41 => Board::Caltron6In1(Caltron6In1::new(memory)),
        64 => Board::Rambo1(Rambo1::new(memory)),
        66 => Board::GxRom(GxRom::new(memory)),
        69 => Board::Fme7(Fme7::new(memory)),
        // Mapper 85 is the VRC7; its submapper picks the register-select address line.
        85 => Board::Vrc7(Vrc7::new(memory)),
        78 => Board::Mapper078(Mapper078::new(memory)),
        71 => Board::Codemasters(Codemasters::new(memory)),
        87 | 101 => Board::JalecoJ87(JalecoJ87::new(memory, mapper)),
        156 => Board::Daou306(Daou306::new(memory)),
        225 => Board::Multicart225(Multicart225::new(memory)),
        228 => Board::Action52(Action52::new(memory)),
        // MF-6a step 1: the Namco 108 and its four parameters. One board, five numbers; see
        // `Namcot108`'s own note on why the differences are fields and not `const` parameters.
        76 | 88 | 95 | 154 | 206 => Board::Namcot108(Namcot108::new(memory, mapper)),
        // MF-6a step 2: five boards that are a register decode and nothing else, sharing one
        // `Board` discriminant. `SmallBoard`'s note has the measurement that put them there.
        33 => Board::SmallBoard(SmallBoard::TaitoTc0190(TaitoTc0190::new(memory))),
        97 => Board::SmallBoard(SmallBoard::IremTamS1(IremTamS1::new(memory))),
        140 => Board::SmallBoard(SmallBoard::JalecoJf11(JalecoJf11::new(memory))),
        184 => Board::SmallBoard(SmallBoard::Sunsoft1(Sunsoft1::new(memory))),
        // Mapper 89 is the Sunsoft-3 board and 93 the Sunsoft-3R; the chip is the same.
        89 | 93 => Board::SmallBoard(SmallBoard::Sunsoft2(Sunsoft2::new(memory, mapper))),
        // MF-6a step 3: four more numbers in the same variant, so no new discriminant.
        // Mapper 152 is the same board as 70 with its mirroring wire connected.
        70 | 152 => Board::SmallBoard(SmallBoard::Bandai74161(Bandai74161::new(memory, mapper))),
        75 => Board::SmallBoard(SmallBoard::Vrc1(Vrc1::new(memory))),
        232 => Board::SmallBoard(SmallBoard::Bf9096(Bf9096::new(memory))),
        // MF-6a step 4: CNROM with the security diodes. `CnRomProtect`'s note says why it
        // is here and not a parameter of `CnRom`.
        185 => Board::SmallBoard(SmallBoard::CnRomProtect(CnRomProtect::new(memory))),
        _ => return None,
    })
}
