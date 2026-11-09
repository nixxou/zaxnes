//! Lot KIRBY-MERGE: what can be held about the seven Kirby's Adventure hack packs **without the
//! game's ROM**, which is not on this machine (`docs/progress/KIRBY-MERGE.md` §3.4). Every pack
//! is a `.zxpk` compiled into the binary, so its structure, its options and its wiring to the
//! margin decoder are all readable from the build alone; only running the game is not.
//!
//! The branch's own figures — 37 of its 41 ROMs take a whole pack, 1 all but the invincibility —
//! are about a ROM pool this side has not got, and are left to whoever has it.

#![allow(clippy::panic, clippy::expect_used, clippy::unwrap_used)] // test code

use zax_nes::builtin_patches::{self, BuiltinPatch};
use zax_nes::hack_pack::{HackPack, WIDESCREEN};

/// The seven builds, in the order `PATCHES` lists them.
const KIRBY_PATCHES: [&str; 7] = [
    builtin_patches::KIRBY_USA_REV1,
    builtin_patches::KIRBY_USA,
    builtin_patches::KIRBY_CANADA,
    builtin_patches::KIRBY_JAPAN,
    builtin_patches::KIRBY_EUROPE,
    builtin_patches::KIRBY_EUROPE_NTSC,
    builtin_patches::KIRBY_GERMANY,
];

fn patch(name: &str) -> &'static BuiltinPatch {
    builtin_patches::by_name(name).unwrap_or_else(|| panic!("{name} is not a built-in patch"))
}

/// A pack per build, each registered, each naming the same game, and the ROM name of each one
/// its own (the two European entries share a CRC and differ by their header's region).
#[test]
fn a_pack_per_build_is_registered() {
    for name in KIRBY_PATCHES {
        let patch = patch(name);
        assert_eq!("Kirby's Adventure", patch.game, "{name}");
        assert!(
            !patch.advanced,
            "{name}: the plain widescreen, as the other decoders"
        );
        assert!(
            patch.boot_zone,
            "{name}: the pack's code runs from power-on"
        );
        assert!(
            patch.idle_loop.is_some(),
            "{name}: the overclock needs the game's wait"
        );
    }
    // Six distinct source CRCs for seven packs: Europe and its NTSC twin are the same data, told
    // apart by the signatures of the pack (`build_pack.py --ntsc`), so the PAL entry takes the
    // CRC and the NTSC one is reached by signature.
    let mut crcs: Vec<u32> = KIRBY_PATCHES.iter().map(|n| patch(n).source_crc).collect();
    crcs.sort_unstable();
    crcs.dedup();
    assert_eq!(
        6,
        crcs.len(),
        "one CRC per build, the two European entries sharing theirs"
    );
    assert_eq!(
        patch(builtin_patches::KIRBY_EUROPE).source_crc,
        patch(builtin_patches::KIRBY_EUROPE_NTSC).source_crc
    );
}

/// Every pack parses, carries its widescreen component and one component per option that names
/// one, and every group of every component has at least one hook or one check.
#[test]
fn every_pack_parses_and_carries_its_components() {
    for name in KIRBY_PATCHES {
        let patch = patch(name);
        assert!(
            zax_nes::hack_pack::is_pack(patch.bps),
            "{name}: the blob is a hack pack"
        );
        let pack = HackPack::read(patch.bps).unwrap_or_else(|| panic!("{name} does not parse"));
        assert!(
            pack.component(WIDESCREEN).is_some(),
            "{name}: no widescreen component"
        );
        for option in patch.options {
            let Some(component) = pack.component(option.key) else {
                continue;
            };
            assert!(
                !component.groups.is_empty(),
                "{name}/{}: a component with no group",
                option.key
            );
            for group in &component.groups {
                assert!(
                    !group.hooks.is_empty() || !group.checks.is_empty(),
                    "{name}/{}/{}: a group that neither hooks nor checks",
                    option.key,
                    group.name
                );
            }
        }
    }
}

/// The eight options of the Kirby packs, their bits and their defaults — the owner's choices,
/// which the merge must not have moved. `level_select` is the one at **bit 8**: the ROM's
/// `$500A` byte is full, so an aid that only the host acts on rides in the high byte, which is
/// what widened the option word to `u16` (`WIDESCREEN-EXTENSION.md`).
#[test]
fn the_options_keep_their_bits_and_their_defaults() {
    let expected: [(&str, u8, bool); 8] = [
        ("input_priority", 0, false),
        ("invincible", 1, false),
        ("no_slowdown", 2, true),
        ("smooth_scroll", 3, false),
        ("margin_enemies", 4, true),
        ("margin_freeze", 5, false),
        ("margin_freeze_tint", 6, false),
        ("kirby_attacks_4_3", 7, false),
    ];
    for name in KIRBY_PATCHES {
        let patch = patch(name);
        assert_eq!(
            9,
            patch.options.len(),
            "{name}: eight options and the level select"
        );
        for (key, bit, default) in expected {
            let option = patch
                .options
                .iter()
                .find(|option| option.key == key)
                .unwrap_or_else(|| panic!("{name}: no option {key}"));
            assert_eq!(bit, option.bit, "{name}/{key}: the bit");
            assert_eq!(default, option.default, "{name}/{key}: the default");
        }
        let level_select = patch
            .options
            .iter()
            .find(|option| option.key == "level_select")
            .unwrap_or_else(|| panic!("{name}: no level select"));
        assert_eq!(
            8, level_select.bit,
            "{name}: the level select is an aid of the host"
        );
        assert!(
            !level_select.default,
            "{name}: the level select is off by default"
        );
    }
}

/// The two aids the owner asked to be **off** whatever the rest does: the enemies asleep in the
/// margins and their grey tint (`da13ef77`). Their component exists in every pack, so turning
/// them on is a choice the player makes and never a default.
#[test]
fn the_asleep_enemies_and_their_tint_are_off_by_default() {
    for name in KIRBY_PATCHES {
        let patch = patch(name);
        for key in ["margin_freeze", "margin_freeze_tint"] {
            let option = patch
                .options
                .iter()
                .find(|option| option.key == key)
                .unwrap_or_else(|| panic!("{name}: no option {key}"));
            assert!(!option.default, "{name}/{key} must be off by default");
        }
    }
}

/// Every pack is wired to Kirby's margin decoder, and so is an **unpatched** Kirby ROM, by the
/// CRCs the decoder lists: the plain widescreen of a game whose margins a decoder draws.
#[test]
fn every_pack_and_every_build_reaches_the_margin_decoder() {
    for name in KIRBY_PATCHES {
        assert!(
            zax_nes::wide_decode::for_patch(name).is_some(),
            "{name}: no margin decoder"
        );
    }
    for name in KIRBY_PATCHES {
        let crc = patch(name).source_crc;
        assert!(
            zax_nes::wide_decode::for_rom(crc).is_some(),
            "{name}: the unpatched build reaches no decoder"
        );
    }
}

/// Why the option word had to become sixteen bits: Kirby fills the ROM's eight. Every option
/// the ROM's code reads through `$500A` is in bits 0-7, and the one option above them —
/// `level_select`, which only the host acts on — does not fit in a `u8` at all, so a build that
/// narrowed the word back would silently drop it.
#[test]
fn the_rom_s_eight_bits_are_full_and_the_ninth_option_needs_the_wider_word() {
    for name in KIRBY_PATCHES {
        let patch = patch(name);
        let mut rom_bits: Vec<u8> = patch
            .options
            .iter()
            .filter(|option| option.key != "level_select")
            .map(|option| option.bit)
            .collect();
        rom_bits.sort_unstable();
        assert_eq!(
            (0..8).collect::<Vec<u8>>(),
            rom_bits,
            "{name}: the ROM's $500A byte is exactly bits 0 to 7, once each"
        );
        let host_only = patch
            .options
            .iter()
            .find(|option| option.key == "level_select")
            .expect("the level select");
        assert!(
            host_only.bit > 7,
            "{name}: the ninth option is above the ROM's byte"
        );
        assert!(
            u8::try_from(1u16 << host_only.bit).is_err(),
            "{name}: its bit does not fit in the u8 the option word used to be"
        );
    }
}
