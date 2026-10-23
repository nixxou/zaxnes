//! The command line (`AppOptions.cs`): the parsing of the arguments, and the usage text, which
//! must be the text of the C# program.
#![allow(clippy::expect_used, clippy::unwrap_used)] // test code

use zax_app::app_options::{AppOptions, OptionsError, RUST_ONLY_USAGE, USAGE};

/// The C# program's options, read to compare the usage text with it.
const CSHARP_OPTIONS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../src/Zax.App/AppOptions.cs"
);

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

/// The usage text is the text of the C# program, byte for byte. The comparison needs the C#
/// source of the repository; without it the test says so and passes.
#[test]
fn the_usage_text_is_the_text_of_the_csharp_program() {
    let Ok(source) = std::fs::read_to_string(CSHARP_OPTIONS) else {
        eprintln!("skipped: {CSHARP_OPTIONS} is not there");
        return;
    };
    let expected =
        csharp_usage_literal(&source).expect("the C# source holds the raw string literal of Usage");
    assert_eq!(USAGE, expected);
}

/// The text of the raw string literal `Usage` of `AppOptions.cs`: its lines without the
/// indentation of the literal, and without the line break before the closing delimiter.
fn csharp_usage_literal(source: &str) -> Option<String> {
    const INDENT: &str = "        ";
    let mut body: Vec<&str> = source
        .lines()
        .skip_while(|line| !line.contains("public static string Usage =>"))
        .skip(2)
        .take_while(|line| !line.trim_start().starts_with("\"\"\""))
        .collect();
    body.pop()?;
    let mut text = String::new();
    for line in body {
        text.push_str(line.strip_prefix(INDENT).unwrap_or(line));
        text.push('\n');
    }
    Some(text)
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
