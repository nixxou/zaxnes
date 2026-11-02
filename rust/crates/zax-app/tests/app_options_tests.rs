//! The command line: the parsing of the arguments and the usage text.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use zax_app::app_options::{AppOptions, OptionsError, RUST_ONLY_USAGE, USAGE};

fn parse(arguments: &[&str]) -> AppOptions {
    AppOptions::parse(arguments).expect("the arguments parse")
}

#[test]
fn app_options_parses_the_options_of_the_slice() {
    let options = parse(&[
        "--rom",
        "game.nes",
        "--profile",
        "Balloon Fight",
        "--size",
        "1600x900",
        "--fov",
        "85",
        "--no-shadows",
        "--no-audio",
    ]);

    assert_eq!(options.rom_path.as_deref(), Some("game.nes"));
    assert_eq!(options.profile.as_deref(), Some("Balloon Fight"));
    assert_eq!(options.width, 1600);
    assert_eq!(options.height, 900);
    assert_eq!(options.fov, Some(85.0));
    assert_eq!(options.shadows, Some(false));
    assert!(!options.audio);
}

/// REQ-SCN-060 and REQ-SCN-061 with Q-066: the display options parse, and an option that was not
/// given stays `None`, "use the persisted setting", instead of carrying a default that would
/// overwrite it at every launch.
#[test]
fn req_scn_060_061_the_display_options_parse_and_default_to_the_setting() {
    let defaults = parse(&[]);
    assert_eq!(defaults.frame_rate, None);
    assert_eq!(defaults.skip_render, None);
    assert_eq!(defaults.skybox, None);
    assert_eq!(defaults.vsync, None);
    assert_eq!(defaults.fov, None);
    assert_eq!(defaults.shadows, None);

    // SPEC-09 gives video.vsync three values; the bare flag is still "every refresh".
    assert_eq!(parse(&["--vsync"]).vsync, Some(1));
    assert_eq!(parse(&["--vsync", "0"]).vsync, Some(0));
    assert_eq!(parse(&["--vsync", "2"]).vsync, Some(2));
    assert_eq!(parse(&["--vsync", "--no-shadows"]).vsync, Some(1));
    assert_eq!(
        AppOptions::parse(&["--vsync", "3"]),
        Err(OptionsError::VSync {
            value: "3".to_owned()
        })
    );

    let options = parse(&["--fps", "120", "--no-render-skip", "--skybox", "fantasy"]);
    assert_eq!(options.frame_rate, Some(120));
    assert_eq!(options.skip_render, Some(false));
    assert_eq!(options.skybox, Some(2));

    assert_eq!(
        AppOptions::parse(&["--skybox", "nebula"]),
        Err(OptionsError::Skybox)
    );

    // AMD-031 §2: --tv and --tv-model are gone, so they follow the general rule for an unknown
    // option.
    assert!(AppOptions::parse(&["--tv", "1"]).is_err());
    assert!(AppOptions::parse(&["--tv-model", "tv.glb"]).is_err());
    assert!(!USAGE.contains("--tv"));
}

#[test]
fn app_options_rejects_what_it_cannot_parse() {
    assert_eq!(
        AppOptions::parse(&["--nope"]),
        Err(OptionsError::UnknownArgument {
            argument: "--nope".to_owned()
        })
    );
    assert_eq!(
        AppOptions::parse(&["--rom"]),
        Err(OptionsError::MissingValue { option: "--rom" })
    );
    assert!(matches!(
        AppOptions::parse(&["--size", "big"]),
        Err(OptionsError::Size { .. })
    ));
}

/// REQ-PUB-006: the font can be named, or the built-in one forced back on the command line.
#[test]
fn req_pub_006_the_font_can_be_named_or_forced_back_to_the_built_in_one() {
    let named = parse(&["--font", "arial.ttf"]);
    assert_eq!(named.font_file.as_deref(), Some("arial.ttf"));
    assert!(!named.built_in_font);

    assert!(parse(&["--builtin-font"]).built_in_font);
}

#[test]
fn the_rust_only_options_parse_and_are_listed_apart_from_the_csharp_usage() {
    let options = parse(&["--handheld", "--roms", "/storage/roms"]);
    assert!(options.handheld);
    assert_eq!(Some("/storage/roms"), options.roms_directory.as_deref());
    assert!(!parse(&["--rom", "a.nes"]).handheld);
    assert_eq!(
        Err(OptionsError::MissingValue { option: "--roms" }),
        AppOptions::parse(&["--roms"])
    );
    // The text compared with the C# program is untouched; the new options have their own block.
    assert!(!USAGE.contains("--handheld"));
    assert!(RUST_ONLY_USAGE.contains("--handheld") && RUST_ONLY_USAGE.contains("--roms"));
}

/// Rust-only: `--backend` names the graphics API of the run, `--load-state` a slot to load once
/// the game has started.
#[test]
fn the_graphics_api_and_the_state_to_load_parse() {
    let options = parse(&["--backend", "Vulkan", "--load-state", "3"]);
    assert_eq!(Some(1), options.backend);
    // `--load-state` and the `-slot=` of AMD-053 REQ-UI-213 fill the same field, so it implies
    // `-noresume` too.
    assert_eq!(Some("3".to_owned()), options.slot);
    assert!(options.no_resume);
    assert_eq!(Some(0), parse(&["--backend", "gl"]).backend);
    assert_eq!(Some(2), parse(&["--backend", "dx12"]).backend);
    assert!(matches!(
        AppOptions::parse(&["--backend", "metal"]),
        Err(OptionsError::Backend { .. })
    ));
    assert!(matches!(
        AppOptions::parse(&["--load-state", "first"]),
        Err(OptionsError::Index { .. })
    ));
    assert!(RUST_ONLY_USAGE.contains("--backend") && RUST_ONLY_USAGE.contains("--load-state"));
}

/// AMD-053 REQ-UI-213: the three options taken from the original program, single dash, with their
/// double-dash aliases, and `-slot` implying `-noresume`.
#[test]
fn req_ui_213_the_three_options_of_the_original_program_parse() {
    let options = parse(&["-slot=7", "-emustats"]);
    assert_eq!(Some("7".to_owned()), options.slot);
    assert!(options.emu_stats);
    // « -slot implique -noresume », whatever order the two came in.
    assert!(options.no_resume);
    assert_eq!(Vec::<String>::new(), options.notices);

    assert!(parse(&["-noresume"]).no_resume);
    assert!(parse(&["-noresume"]).slot.is_none());
    assert!(!parse(&["-noresume"]).emu_stats);

    // The double-dash forms are accepted as aliases (D).
    assert_eq!(Some("quick".to_owned()), parse(&["--slot=quick"]).slot);
    assert!(parse(&["--no-resume"]).no_resume);
    assert!(parse(&["--emu-stats"]).emu_stats);

    // The usage text names them.
    assert!(USAGE.contains("-slot=<n>"));
    assert!(USAGE.contains("-noresume"));
    assert!(USAGE.contains("-emustats"));
}

/// REQ-UI-213: the command line accepts the 36 slots REQ-PERS-050 keeps and the quick slot — the
/// exposure limit of AMD-010 REQ-UI-208 does not reach it — and ignores anything else with a
/// notice, the program starting normally.
#[test]
fn req_ui_213_slot_accepts_1_to_36_and_quick_and_ignores_the_rest() {
    for slot in ["1", "10", "36", "quick"] {
        let options = parse(&[&format!("-slot={slot}")]);
        assert_eq!(Some(slot.to_owned()), options.slot, "{slot}");
        assert_eq!(Vec::<String>::new(), options.notices, "{slot}");
    }
    for slot in ["0", "37", "99", "", "first"] {
        let options = parse(&[&format!("-slot={slot}")]);
        assert_eq!(None, options.slot, "{slot}");
        // The start-up carries on: no error, and the fact is there for the log.
        assert!(!options.no_resume, "{slot}");
        assert_eq!(1, options.notices.len(), "{slot}");
        assert!(options.notices[0].contains("is ignored"), "{slot}");
    }
}
