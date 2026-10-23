//! `render --rom`, `--press`, `--dump-frame` and `--frames <file.json>` (port of the ROM mode of
//! `RenderExport.cs`, `InputScript.cs` and `FrameJson.cs`).
//!
//! The ROM is a public test ROM of `rust/test-roms/` (mapper 0); the tests that need it say so
//! and return when it is absent. No commercial game is involved: the profile is built here.

// A test: a failed render or a malformed capture should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::Value;
use zax_harness::render::frame_json;
use zax_harness::render::input_script::InputScript;
use zax_harness::render::{self, FrameSource, Options};
use zax_nes::TileRecord;
use zax_profiles::enums::{TileFlip, VariantGeometry, VariantRenderStyle};
use zax_profiles::format;
use zax_profiles::layout::FieldPresence;
use zax_profiles::pattern::ProfilePattern;
use zax_profiles::profile::GameProfile;
use zax_profiles::script::ProfileScript;
use zax_profiles::tile::{PatternGrid, ProfileTile};
use zax_profiles::values::Vec2I;
use zax_profiles::variant::PatternVariant;
use zax_render::assets::png_image;

const A: u16 = 1 << 0;
const START: u16 = 1 << 3;
const RIGHT: u16 = 1 << 7;

fn test_rom() -> Option<PathBuf> {
    let rom = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms/nestest.nes");
    if rom.is_file() {
        Some(rom)
    } else {
        eprintln!("skipped: {} is absent", rom.display());
        None
    }
}

fn directory(name: &str) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    root
}

fn arguments(values: &[&str]) -> Vec<String> {
    values.iter().map(|&value| value.to_owned()).collect()
}

fn path_text(path: &Path) -> &str {
    path.to_str().unwrap()
}

/// Renders with `values` and returns the parsed `render.json` of `out`.
fn render(out: &Path, values: &[&str]) -> Vec<Value> {
    let mut all = vec!["--out", path_text(out)];
    all.extend_from_slice(values);
    let options = Options::parse(&arguments(&all)).unwrap();
    render::run(&options).unwrap();
    serde_json::from_str(&fs::read_to_string(out.join("render.json")).unwrap()).unwrap()
}

/// A profile with one pattern for the tile of `glyph`, so that the scene is 3D.
fn profile_for(glyph: &TileRecord) -> GameProfile {
    let tile = Arc::new(ProfileTile::new(glyph.plane0, glyph.plane1, TileFlip::NONE));
    let mut profile = GameProfile::create_empty();
    profile.title = Some("Round trip".to_owned());
    profile.permanent_patterns = vec![Arc::new(ProfilePattern {
        grid: Some(PatternGrid::new(1, 1, vec![Some(tile)])),
        tile_size: Some(Vec2I::new(1, 1)),
        permanent: true,
        variants: vec![Arc::new(PatternVariant {
            depth: 4,
            geometry: VariantGeometry::Default,
            alpha: 1.0,
            render_style: VariantRenderStyle::Normal,
            ..PatternVariant::default()
        })],
        ..ProfilePattern::default()
    })];
    profile
}

/// The first background glyph the menu of `rom` draws.
fn menu_glyph(rom: &Path, root: &Path) -> TileRecord {
    let probe = root.join("probe.json");
    render(
        &root.join("probe"),
        &["--rom", path_text(rom), "--dump-frame", path_text(&probe)],
    );
    let frame = frame_json::read(&fs::read_to_string(&probe).unwrap()).unwrap();
    *frame
        .records
        .iter()
        .find(|record| record.background && (record.plane0 | record.plane1) != 0)
        .expect("the menu draws text")
}

#[test]
fn press_holds_the_buttons_of_the_frames_counted_from_the_reset() {
    let script = InputScript::parse("200:start, 260:start,400-700:right").unwrap();
    assert_eq!(script.at(0), 0);
    assert_eq!(script.at(199), 0);
    assert_eq!(script.at(200), START);
    assert_eq!(script.at(201), 0);
    assert_eq!(script.at(260), START);
    assert_eq!(script.at(399), 0);
    assert_eq!(script.at(400), RIGHT);
    assert_eq!(script.at(700), RIGHT);
    assert_eq!(script.at(701), 0);
}

#[test]
fn press_joins_buttons_and_takes_the_union_of_overlapping_ranges() {
    let script = InputScript::parse("0-10:a+Start,5-20:RIGHT").unwrap();
    assert_eq!(script.at(0), A | START);
    assert_eq!(script.at(5), A | START | RIGHT);
    assert_eq!(script.at(11), RIGHT);
    assert_eq!(script.at(21), 0);
}

#[test]
fn press_all_eight_buttons_have_their_bit() {
    for (bit, name) in ["a", "b", "select", "start", "up", "down", "left", "right"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            InputScript::parse(&format!("3:{name}")).unwrap().at(3),
            1 << bit
        );
    }
}

#[test]
fn press_nothing_holds_nothing() {
    for text in ["", "  ", ",,"] {
        let script = InputScript::parse(text).unwrap();
        assert_eq!(script, InputScript::default(), "'{text}'");
        assert_eq!(script.at(0), 0);
    }
}

#[test]
fn press_refuses_what_the_csharp_script_refuses() {
    for text in [
        "start",      // no colon
        ":a",         // no frames
        "x:a",        // frames are not numbers
        "5-3:a",      // a range runs forwards
        "-1:a",       // frames start at zero
        "1-2-3:a",    // one dash at most
        "10:fire",    // not a button
        "10:a+turbo", // one unknown button spoils the entry
    ] {
        let error = InputScript::parse(text).unwrap_err().to_string();
        assert!(error.starts_with("--press:"), "'{text}' gave '{error}'");
    }
}

#[test]
fn the_command_line_reports_a_bad_press_and_keeps_the_rom_options() {
    assert!(Options::parse(&arguments(&["--out", "x", "--press", "9:fire"])).is_err());

    let options = Options::parse(&arguments(&[
        "--out",
        "x",
        "--rom",
        "g.nes",
        "--run",
        "5",
        "--press",
        "1-2:a",
        "--profile",
        "Game",
        "--sav",
        "g.sav",
        "--dump-frame",
        "f.json",
    ]))
    .unwrap();
    assert_eq!(options.rom, Some(PathBuf::from("g.nes")));
    assert_eq!(options.run_frames, 5);
    assert_eq!(options.press.at(2), A);
    assert_eq!(options.profile.as_deref(), Some("Game"));
    assert_eq!(options.battery, Some(PathBuf::from("g.sav")));
    assert_eq!(options.dump_frame, Some(PathBuf::from("f.json")));

    let defaults = Options::parse(&arguments(&["--out", "x"])).unwrap();
    assert_eq!(defaults.run_frames, 120);
    assert_eq!(defaults.frames, FrameSource::Synthetic(None));
    assert_eq!(
        Options::parse(&arguments(&["--out", "x", "--frames", "synthetic:alpha"]))
            .unwrap()
            .frames,
        FrameSource::Synthetic(Some("alpha".to_owned()))
    );
    assert_eq!(
        Options::parse(&arguments(&["--out", "x", "--frames", "dump.json"]))
            .unwrap()
            .frames,
        FrameSource::Json(PathBuf::from("dump.json"))
    );
}

#[test]
fn the_frame_dump_has_the_layout_of_frame_json() {
    let mut palette = [0; 32];
    palette[0] = 15;
    palette[31] = 48;
    let records = [
        TileRecord::new(u64::MAX, 1, 8, -1, 0, false, false, false, 2, true),
        TileRecord::new(3, 4, 255, 239, 0, true, true, true, 1, false),
    ];
    let text = frame_json::write(&records, &palette).unwrap();
    let palette_text = "15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, \
                        0, 0, 0, 0, 0, 0, 0, 48";
    assert_eq!(
        text,
        format!(
            "{{\n  \"palette\": [{palette_text}],\n  \"tiles\": [\n    \
             {{ \"low\": 18446744073709551615, \"high\": 1, \"x\": 8, \"y\": -1, \"palette\": 2, \
             \"mirrorH\": false, \"mirrorV\": false, \"hidden\": false, \"sprite\": false }},\n    \
             {{ \"low\": 3, \"high\": 4, \"x\": 255, \"y\": 239, \"palette\": 1, \
             \"mirrorH\": true, \"mirrorV\": true, \"hidden\": true, \"sprite\": true }}\n  ]\n}}\n"
        )
    );

    // Read back, the sprite takes the sprite order and the background its position.
    let frame = frame_json::read(&text).unwrap();
    assert_eq!(frame.palette, palette);
    assert_eq!(frame.records, records);
}

#[test]
fn the_frame_reader_refuses_a_broken_description() {
    for text in [
        "not json",
        r#"{ "tiles": [ { "low": 1, "high": 0, "x": 0 } ] }"#,
        r#"{ "tiles": [ { "low": -1, "high": 0, "x": 0, "y": 0 } ] }"#,
        r#"{ "tiles": [ { "low": 1, "high": 0, "x": 99999, "y": 0 } ] }"#,
        r#"{ "palette": [ 300 ] }"#,
    ] {
        assert!(frame_json::read(text).is_err(), "'{text}'");
    }

    // The palette keeps its first 32 bytes and `background` replaces the universal colour.
    let frame = frame_json::read(r#"{ "palette": [1, 2], "background": 9, "tiles": [] }"#).unwrap();
    assert_eq!(frame.palette[..3], [9, 2, 0]);
}

#[test]
fn a_rom_run_without_profile_captures_the_flat_picture() {
    let Some(rom) = test_rom() else { return };
    let out = directory("render-rom-flat");
    let report = render(
        &out,
        &[
            "--rom",
            path_text(&rom),
            "--run",
            "120",
            "--size",
            "320x240",
        ],
    );

    assert_eq!(report.len(), 1);
    assert_eq!(report[0]["scene"], "nestest");
    assert_eq!(report[0]["width"], 320);
    assert_eq!(
        report[0]["description"],
        "nestest.nes after 120 emulated frames, profile '2D Mode'"
    );
    assert!(
        report[0]["distinctColors"].as_u64().unwrap() >= 2,
        "the menu is drawn"
    );

    // The flat picture: 256 x 240, with text on it.
    let flat = png_image::decode(&fs::read(out.join("nestest-2d.png")).unwrap()).unwrap();
    assert_eq!((flat.width, flat.height), (256, 240));
    let lit = flat
        .pixels
        .chunks_exact(4)
        .filter(|pixel| pixel[0] > 128)
        .count();
    assert!(lit > 500, "{lit} lit pixels");
    assert!(out.join("nestest.png").is_file());
}

#[test]
fn a_dumped_frame_replays_to_the_same_capture() {
    let Some(rom) = test_rom() else { return };
    let root = directory("render-rom-roundtrip");

    let glyph = menu_glyph(&rom, &root);
    let profile = profile_for(&glyph);
    let profile_file = root.join("round-trip.3dn");
    fs::write(&profile_file, format::save(&profile, FieldPresence::All)).unwrap();

    let dump = root.join("frame.json");
    let size = ["--size", "320x240", "--angles", "0.4,0.5"];
    let mut live = vec![
        "--rom",
        path_text(&rom),
        "--profile",
        path_text(&profile_file),
        "--dump-frame",
        path_text(&dump),
    ];
    live.extend_from_slice(&size);
    let live_report = render(&root.join("live"), &live);

    let mut replay = vec![
        "--frames",
        path_text(&dump),
        "--profile",
        path_text(&profile_file),
    ];
    replay.extend_from_slice(&size);
    let replay_report = render(&root.join("replay"), &replay);

    assert_eq!(
        live_report[0]["description"],
        "nestest.nes after 120 emulated frames, profile 'Round trip'"
    );
    assert_eq!(replay_report[0]["scene"], "frame");
    let counts = |entry: &Value| {
        [
            "opaqueTiles",
            "transparentTiles",
            "hudTiles",
            "shadowCasters",
            "drawCalls",
            "triangles",
            "residentMeshes",
            "distinctColors",
            "backgroundPixels",
        ]
        .map(|key| entry[key].as_u64().unwrap())
    };
    let live_counts = counts(&live_report[0]);
    assert!(
        live_counts[0] > 0,
        "the glyph is drawn in 3D: {live_counts:?}"
    );
    assert_eq!(live_counts, counts(&replay_report[0]));

    let live_png = fs::read(root.join("live/nestest.png")).unwrap();
    let replay_png = fs::read(root.join("replay/frame.png")).unwrap();
    assert_eq!(
        live_png, replay_png,
        "the replayed capture is the same image"
    );
    assert!(!root.join("replay/frame-2d.png").exists());
    assert!(root.join("live/nestest-2d.png").is_file());
}

#[test]
fn pressing_a_button_changes_the_run() {
    let Some(rom) = test_rom() else { return };
    let root = directory("render-rom-press");
    let flat = |name: &str, press: &str| {
        let out = root.join(name);
        render(
            &out,
            &[
                "--rom",
                path_text(&rom),
                "--run",
                "30",
                "--press",
                press,
                "--size",
                "64x48",
            ],
        );
        fs::read(out.join("nestest-2d.png")).unwrap()
    };

    // nestest moves its cursor on Down, which the idle run does not.
    assert_ne!(flat("idle", ""), flat("down", "10:down"));
}

#[test]
fn a_profile_script_acts_on_the_scene_of_the_capture() {
    let Some(rom) = test_rom() else { return };
    let root = directory("render-rom-script");
    let mut profile = profile_for(&menu_glyph(&rom, &root));
    let plain = root.join("plain.3dn");
    fs::write(&plain, format::save(&profile, FieldPresence::All)).unwrap();

    // The Lua runtime of the profile gets the camera of the scene (REQ-LUA-088).
    profile.scripts.push(ProfileScript::new(
        Some("far"),
        true,
        Some("function Start() Camera.Distance = 400 end"),
        Some(""),
    ));
    let scripted = root.join("scripted.3dn");
    fs::write(&scripted, format::save(&profile, FieldPresence::All)).unwrap();

    let capture = |name: &str, file: &Path| {
        render(
            &root.join(name),
            &[
                "--rom",
                path_text(&rom),
                "--profile",
                path_text(file),
                "--size",
                "320x240",
            ],
        )
    };
    let near = &capture("near", &plain)[0];
    let far = &capture("far", &scripted)[0];
    assert!(
        far["backgroundPixels"].as_u64().unwrap() > near["backgroundPixels"].as_u64().unwrap(),
        "the scene is seen from further away"
    );
}
