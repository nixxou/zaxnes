//! Maps an iNES mapper number to its board (port of `MapperRegistry`, REQ-CORE-100,
//! REQ-CORE-102).
//!
//! The registry only advertises the boards that are implemented, so that a ROM is never
//! accepted and then found unplayable. The table is the one of REQ-CORE-100; several numbers
//! share a board when they are the same silicon differently wired (9 and 10, 21/22/23/25/27,
//! 87 and 101). 156 is outside that table: it runs Metal Force (Korea), a widescreen target.

use super::mappers::Nrom;
use super::mappers::{
    Action52, Action53, AxRom, BandaiFcg, Caltron6In1, CnRom, Codemasters, ColorDreams, CpRom,
    Daou306, GxRom, IremG101, JalecoJ87, JalecoSs88006, Mapper015, Mapper034, Mmc1, Mmc2, Mmc3,
    Mmc5, Multicart225, Rambo1, UnRom512, UxRom, Vrc2Vrc4,
};
use super::{Board, CartridgeMemory};
use crate::rom::InesHeader;

/// Mapper numbers this build can run, in ascending order (REQ-CORE-102). 116 is optional in
/// REQ-CORE-101 and not implemented.
pub const SUPPORTED_MAPPERS: &[i32] = &[
    0, 1, 2, 3, 4, 5, 7, 9, 10, 11, 13, 15, 16, 18, 21, 22, 23, 25, 27, 28, 30, 32, 34, 41, 64, 66,
    71, 87, 101, 156, 225, 228,
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

/// The volatile-memory sizes of `mapper`'s board for `header`.
#[must_use]
pub(crate) fn board_memory_sizes(mapper: i32, header: &InesHeader) -> BoardMemorySizes {
    match mapper {
        // MR-4-6, nesdev ("MMC6"): the chip "contains 1 KB of internal PRG RAM", at "CPU
        // $7000-$7FFF: 1 KB PRG RAM, mirrored". The header of a StarTropics dump describes
        // the 8 KiB every other mapper-4 board has.
        4 if header.is_nes20 && header.submapper == 1 => BoardMemorySizes {
            prg_ram: Some(Mmc3::MMC6_PRG_RAM_SIZE),
            ..BoardMemorySizes::default()
        },
        5 => BoardMemorySizes {
            min_prg_ram: Mmc5::MAX_PRG_RAM_SIZE,
            ..BoardMemorySizes::default()
        },
        _ => BoardMemorySizes::default(),
    }
}

/// Builds the board of `mapper` for `memory`, or `None` when it is not implemented.
#[must_use]
pub(crate) fn create_board(mapper: i32, memory: &CartridgeMemory) -> Option<Board> {
    Some(match mapper {
        0 => Board::Nrom(Nrom),
        1 => Board::Mmc1(Mmc1::new(memory)),
        2 => Board::UxRom(UxRom::new(memory)),
        3 => Board::CnRom(CnRom::new(memory)),
        4 => Board::Mmc3(Mmc3::new(memory)),
        5 => Board::Mmc5(Mmc5::new(memory)),
        7 => Board::AxRom(AxRom::new(memory)),
        9 | 10 => Board::Mmc2(Mmc2::new(memory, mapper)),
        11 => Board::ColorDreams(ColorDreams::new(memory)),
        13 => Board::CpRom(CpRom::new(memory)),
        15 => Board::Mapper015(Mapper015::new(memory)),
        16 => Board::BandaiFcg(BandaiFcg::new(memory)),
        18 => Board::JalecoSs88006(JalecoSs88006::new(memory)),
        21 | 22 | 23 | 25 | 27 => Board::Vrc2Vrc4(Vrc2Vrc4::new(memory, mapper)),
        28 => Board::Action53(Action53::new(memory)),
        30 => Board::UnRom512(UnRom512::new(memory)),
        32 => Board::IremG101(IremG101::new(memory)),
        34 => Board::Mapper034(Mapper034::new(memory)),
        41 => Board::Caltron6In1(Caltron6In1::new(memory)),
        64 => Board::Rambo1(Rambo1::new(memory)),
        66 => Board::GxRom(GxRom::new(memory)),
        71 => Board::Codemasters(Codemasters::new(memory)),
        87 | 101 => Board::JalecoJ87(JalecoJ87::new(memory, mapper)),
        156 => Board::Daou306(Daou306::new(memory)),
        225 => Board::Multicart225(Multicart225::new(memory)),
        228 => Board::Action52(Action52::new(memory)),
        _ => return None,
    })
}
