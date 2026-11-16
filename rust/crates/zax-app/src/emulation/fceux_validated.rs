//! The games validated for **3D on the FCEUX engine**, and the marker for every other game
//! (lot FCEUX-48 step 2, which is the vetting half of §7 of `docs/review/FCEUX-ENGINE.md`).
//!
//! §7 recommended "the fix plus a vetted list". Step 1 of the same lot took the fix — FCEUX's
//! CHR-RAM heap overflow — so an arbitrary ROM is no longer a **memory-safety** question, and what
//! is left is a question of **quality**: does the 3D the renderer builds from FCEUX's tile records
//! look like the one it builds from Zaxnes's? Lot FCEUX-6 answered that per game and lot FCEUX-23
//! added *Punch-Out!!*; this table is those verdicts, and nothing else.
//!
//! So it is a **marker, not a gate**: the per-game engine option stays available for every game,
//! which is the owner's choice (§4: the choice is per game precisely because it is the user's).
//! The engine row, the notification at load and the performance overlay say "not validated on
//! FCEUX" for a game that is not here, and that is the whole of the enforcement.
//!
//! ## How a game is identified
//!
//! By the **CRC-32 of the ROM data** — the PRG and CHR of the image with the iNES header, a
//! trainer and any overdump left out, which is the CRC the No-Intro set gives
//! (`zax_nes::builtin_patches::data_crc`). That is the identifier the application already uses to
//! recognise a particular ROM file: the built-in patches and the widescreen hack certificates are
//! keyed by it and `RomCrcCache` already keeps it per file, so nothing new has to be computed or
//! stored. **Never by file name**: a name is the user's to change and says nothing about the bytes.
//!
//! One consequence, stated rather than hidden: the table lists **dumps**, not titles. Each verdict
//! was measured on the dumps lot FCEUX-6 had, so another dump of the same game — a different
//! revision, a translation, a ROM hack — is *not* validated and the marker will say so. That is the
//! honest answer; a looser key would claim a measurement nobody made.

/// What lot FCEUX-6's sweep found for one ROM dump.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// The tile records **and** the palette were identical to Zaxnes's at every compared frame —
    /// 60, 300, 1 200 and 3 000 — and the 2D pictures with them.
    Identical,
    /// §6 of the design note marks the game ship-ready and names a deviation. The text is that
    /// deviation, and every one of them is an **emulation** difference between the two cores
    /// rather than a capture one: no work on the tile capture can remove it.
    WithBlemish(&'static str),
}

impl Verdict {
    /// The deviation, when there is one; `None` for a dump that matched at every frame.
    #[must_use]
    pub const fn blemish(self) -> Option<&'static str> {
        match self {
            Self::Identical => None,
            Self::WithBlemish(why) => Some(why),
        }
    }
}

/// One validated dump: the CRC-32 of its data, the game it is a dump of (a label for the
/// interface and the logs — the CRC is the identity), and what was measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidatedRom {
    /// `zax_nes::builtin_patches::data_crc` of the image.
    pub crc: u32,
    /// The game, for a human to read. Not an identifier.
    pub game: &'static str,
    /// What lot FCEUX-6 measured on this dump.
    pub verdict: Verdict,
}

/// The three deviations §6 names, written once because several dumps share each of them.
mod why {
    /// *Metroid*, *Punch-Out!!* and the two *Rockman 2* dumps: the records are identical at
    /// frames 60, 300 and 1 200 and the two machines part at 3 000, which is FCEUX's emulation
    /// differing from Zaxnes's and out of reach of the capture.
    pub const PARTS_AT_3000: &str =
        "identical at frames 60, 300 and 1 200; the two cores part at 3 000";
    /// *Final Fantasy*: all 960 records are identical on every dump at every frame, and palette
    /// entry `$3F0B` is 32 on Zaxnes and 0 on FCEUX at frames 60 and 300 — 0.69 % of the 3D
    /// picture. Frame 1 200 is 100.000 %.
    pub const PALETTE_ENTRY: &str =
        "every record identical; palette entry $3F0B differs early, 0.69 % of the 3D picture";
    /// *Adventure Island*: the `$4014` transfer begins at a different `$2003`, so every sprite
    /// matches after rotating FCEUX's OAM number by a constant. REQ-CORE-043 numbers a record
    /// after its OAM entry, so a capture can neither cause nor cure it; 99.999 % of the picture.
    pub const OAM_ROTATION: &str =
        "the OAM is rotated by a constant, which renumbers sprites without moving them; 99.999 %";
}

/// Every validated dump: the **nine games §6 of `docs/review/FCEUX-ENGINE.md` marks ship**, with
/// the dumps of each that `zax-harness fceux-tiles` compared at frames 60, 300, 1 200 and 3 000.
/// Sorted by CRC, so that a diff of this file reads and a lookup could become a binary search
/// without reordering anything.
///
/// Per game, which is how §6 states them: *Mega Man* 6 dumps, *Mega Man 2* 8, *Mega Man 3* 9,
/// *Ninja Gaiden III* 4, *Metroid* 4, *Blaster Master* 5 of its 6, *Final Fantasy* 7,
/// *Adventure Island* 3, *Punch-Out!!* 1 — 47 in all.
///
/// What is **not** here, and why — the absence is the information:
///
/// * *Super Mario Bros.* — §6 calls it **borderline**: frame 300 is 100.000 % and frame 1 200 is
///   **89.647 %**, from sprites held in different OAM slots. Playable, visibly not the same
///   picture;
/// * *Castlevania* and *Excitebike* — §6 says **no**: 89.785 % and 86.997 % at frame 1 200, and
///   *Castlevania* is the one game whose record *count* differs as well (1 024 against 1 028);
/// * *Super Mario Bros. 3* and *Zelda II* — §6: **blocked on a decision**. They need
///   `ppu-2007-render-read` on in `zax-nes`, which is non-default by Q-139 precisely because it
///   moves those two games' pictures. With it on, the two engines' records become identical row
///   for row — so these two are a *decision* away from this table rather than a measurement away;
/// * *Blaster Master* `[Europe]`, the one dump of the six that is not identical (§6: "5 of 6");
/// * every game with no 3dn profile. §6 judged the 13 games that have one, so there is no verdict
///   to copy for the rest — and a game with no profile has no 3D to be validated for.
static VALIDATED: &[ValidatedRom] = &[
    ValidatedRom {
        crc: 0x0013_88B3,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x0150_E2EF,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x039B_4A9C,
        game: "Blaster Master",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x07E8_D0CF,
        game: "Metroid",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0x0FCF_C04D,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x11C8_7C02,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x1248_1CC0,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x16B0_6537,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x1C47_D202,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x1F53_E946,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0x2F2E_30F7,
        game: "Ninja Gaiden III",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x3552_BE7B,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0x3F0F_D764,
        game: "Blaster Master",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x466E_FDC2,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0x4676_CED3,
        game: "Blaster Master",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x5996_29D5,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x603A_AA57,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x6150_517C,
        game: "Mega Man 2",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0x6A45_7A43,
        game: "Adventure Island",
        verdict: Verdict::WithBlemish(why::OAM_ROTATION),
    },
    ValidatedRom {
        crc: 0x6EE4_BB0A,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x7008_0810,
        game: "Metroid",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0x7751_588D,
        game: "Metroid",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0x837A_3D8A,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x8535_9519,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0x8686_7830,
        game: "Adventure Island",
        verdict: Verdict::WithBlemish(why::OAM_ROTATION),
    },
    ValidatedRom {
        crc: 0x891C_14BC,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x8A88_AE79,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x902E_3168,
        game: "Ninja Gaiden III",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0x9447_6A70,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xA663_8CBA,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xA7A8_2B60,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xAEF5_B452,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xB683_A856,
        game: "Blaster Master",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xB95E_9E7F,
        game: "Punch-Out!!",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0xBEFB_4FA8,
        game: "Ninja Gaiden III",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xC42D_6424,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0xCEBD_2A31,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0xCF5D_E2BC,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xD1E6_EAB9,
        game: "Ninja Gaiden III",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xD31D_C910,
        game: "Mega Man",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xD59A_DB3F,
        game: "Metroid",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
    ValidatedRom {
        crc: 0xD9F1_E47C,
        game: "Mega Man 3",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xE228_7991,
        game: "Mega Man 2",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xE59C_4A99,
        game: "Blaster Master",
        verdict: Verdict::Identical,
    },
    ValidatedRom {
        crc: 0xF71E_7EDD,
        game: "Final Fantasy",
        verdict: Verdict::WithBlemish(why::PALETTE_ENTRY),
    },
    ValidatedRom {
        crc: 0xF8A7_13BE,
        game: "Adventure Island",
        verdict: Verdict::WithBlemish(why::OAM_ROTATION),
    },
    ValidatedRom {
        crc: 0xFBCF_0690,
        game: "Mega Man 2",
        verdict: Verdict::WithBlemish(why::PARTS_AT_3000),
    },
];

/// What can be said about one game, which is what the interface shows. The three cases are kept
/// apart on purpose: "not validated" and "nothing is known about it" are different statements and
/// the second must not be printed as the first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// In the table, with its verdict.
    Validated(&'static ValidatedRom),
    /// Not in the table. The engine option still works; the marker says so.
    NotValidated,
    /// No game in play, or a ROM whose CRC the application has not read — so there is nothing to
    /// say either way, and the marker says nothing.
    Unknown,
}

impl Status {
    /// Whether this is a game lot FCEUX-6 measured and §6 ships.
    #[must_use]
    pub const fn is_validated(self) -> bool {
        matches!(self, Self::Validated(_))
    }
}

/// The status of the ROM data whose CRC-32 is `crc`; `None` for `crc` means the application does
/// not know the CRC, which is [`Status::Unknown`] and not a verdict.
#[must_use]
pub fn status_of(crc: Option<u32>) -> Status {
    match crc {
        None => Status::Unknown,
        Some(crc) => of_crc(crc).map_or(Status::NotValidated, Status::Validated),
    }
}

/// What the table says about the ROM data whose CRC-32 is `crc`: `None` when it is not in it,
/// which the interface reports as "not validated on FCEUX".
#[must_use]
pub fn of_crc(crc: u32) -> Option<&'static ValidatedRom> {
    VALIDATED.iter().find(|rom| rom.crc == crc)
}

/// Whether the ROM data whose CRC-32 is `crc` is validated for 3D on the FCEUX engine.
#[must_use]
pub fn is_validated(crc: u32) -> bool {
    of_crc(crc).is_some()
}

/// Every validated dump, for the tests and for anything that wants to print the list.
#[must_use]
pub fn all() -> &'static [ValidatedRom] {
    VALIDATED
}

#[cfg(test)]
mod tests {
    use super::{all, is_validated, of_crc, status_of, Status, Verdict};
    use std::collections::BTreeSet;

    /// "Not validated" and "nothing is known" are different answers, and the marker prints them
    /// differently, so the three cases are pinned here.
    #[test]
    fn the_three_statuses_are_told_apart() {
        assert_eq!(status_of(None), Status::Unknown);
        assert_eq!(status_of(Some(0)), Status::NotValidated);
        assert!(status_of(Some(0x603A_AA57)).is_validated());
        assert!(!status_of(None).is_validated());
        assert!(!status_of(Some(0xD445_F698)).is_validated());
        let Status::Validated(rom) = status_of(Some(0x603A_AA57)) else {
            panic!("Mega Man 3 [USA] is validated");
        };
        assert_eq!(rom.game, "Mega Man 3");
    }

    /// The table is a lookup key, so the same CRC twice would be a silent contradiction.
    #[test]
    fn every_crc_appears_once() {
        let unique: BTreeSet<u32> = all().iter().map(|rom| rom.crc).collect();
        assert_eq!(unique.len(), all().len(), "a CRC is in the table twice");
    }

    /// Kept sorted by CRC, as the doc comment says.
    #[test]
    fn the_table_is_sorted_by_crc() {
        let listed: Vec<u32> = all().iter().map(|rom| rom.crc).collect();
        let mut sorted = listed.clone();
        sorted.sort_unstable();
        assert_eq!(listed, sorted, "the table is not sorted by CRC");
    }

    /// The nine games §6 of the design note marks **ship**, and only those, with the number of
    /// dumps §6 itself gives for each.
    #[test]
    fn the_games_and_their_dumps_are_the_ones_the_design_note_ships() {
        let counted: Vec<(&str, usize)> = all()
            .iter()
            .map(|rom| rom.game)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(|game| (game, all().iter().filter(|rom| rom.game == game).count()))
            .collect();
        assert_eq!(
            counted,
            vec![
                ("Adventure Island", 3),
                ("Blaster Master", 5),
                ("Final Fantasy", 7),
                ("Mega Man", 6),
                ("Mega Man 2", 8),
                ("Mega Man 3", 9),
                ("Metroid", 4),
                ("Ninja Gaiden III", 4),
                ("Punch-Out!!", 1),
            ],
        );
        assert_eq!(all().len(), 47);
    }

    /// The games §6 does **not** ship must not be in the table, and these are the CRCs of the
    /// dumps of them this work measured: *Super Mario Bros.* (borderline), *Castlevania* and
    /// *Excitebike* (no), *Super Mario Bros. 3* and *Zelda II* (blocked on the
    /// `ppu-2007-render-read` decision), and the one *Blaster Master* dump of the six that is not
    /// identical.
    #[test]
    fn the_games_the_sweep_did_not_ship_are_not_validated() {
        for (crc, what) in [
            (0xD445_F698_u32, "Super Mario Bros. [World]"),
            (0x9A2D_B086, "Super Mario Bros. [Europe]"),
            (0x0AC1_AA8F, "Castlevania [USA]"),
            (0x49AE_B3A6, "Excitebike [Japan][USA]"),
            (0xA0B0_B742, "Super Mario Bros. 3 [USA]"),
            (0xBA32_2865, "Zelda II [USA]"),
            (0x8106_E694, "Blaster Master [Europe]"),
        ] {
            assert!(!is_validated(crc), "{what} must not be validated");
        }
    }

    /// A lookup answers with the game and the deviation, and `None` is the answer for everything
    /// else — which is the whole of the marker.
    #[test]
    fn a_lookup_gives_the_game_and_the_blemish() {
        let punch_out = of_crc(0xB95E_9E7F).expect("Punch-Out!! is validated");
        assert_eq!(punch_out.game, "Punch-Out!!");
        assert_eq!(
            punch_out.verdict.blemish(),
            Some("identical at frames 60, 300 and 1 200; the two cores part at 3 000"),
        );

        let mega_man_3 = of_crc(0x603A_AA57).expect("Mega Man 3 [USA] is validated");
        assert_eq!(mega_man_3.game, "Mega Man 3");
        assert_eq!(mega_man_3.verdict, Verdict::Identical);
        assert_eq!(mega_man_3.verdict.blemish(), None);

        assert!(of_crc(0).is_none(), "a CRC of 0 is not a validated dump");
    }

    /// Every deviation is one of the three §6 names, so a new one cannot be added without saying
    /// what it is.
    #[test]
    fn the_blemishes_are_the_three_the_design_note_names() {
        let reasons: BTreeSet<&str> = all()
            .iter()
            .filter_map(|rom| rom.verdict.blemish())
            .collect();
        assert_eq!(reasons.len(), 3, "{reasons:?}");
    }
}
