#![cfg(feature = "vs-system")]
// A failed setup panics, which is how a test says the fixture is wrong.
#![allow(clippy::expect_used, clippy::panic)]

//! `zax-harness vs`: the command that drives a VS. System cabinet (lot MF-VS step 6).

use std::path::PathBuf;

use zax_harness::vs;

/// A NES 2.0 VS image of `mapper` with `prg_8k` 8 KiB PRG banks and `chr_8k` CHR banks, byte 13
/// and byte 15 as given. Console type 1, so the image is a VS. System however it is read.
fn vs_image(mapper: u8, prg_8k: usize, chr_8k: usize, byte13: u8, byte15: u8) -> Vec<u8> {
    let prg = prg_8k * 8 * 1024;
    let chr = chr_8k * 8 * 1024;
    let mut image = vec![0u8; 16 + prg + chr];
    image[..4].copy_from_slice(b"NES\x1A");
    image[4] = u8::try_from(prg / (16 * 1024)).expect("a small fixture");
    image[5] = u8::try_from(chr / (8 * 1024)).expect("a small fixture");
    image[6] = (mapper & 0x0F) << 4;
    image[7] = (mapper & 0xF0) | 0x09;
    image[13] = byte13;
    image[15] = byte15;

    image[16..19].copy_from_slice(&[0x4C, 0x00, 0x80]);
    let vectors = 16 + prg - 6;
    for slot in 0..3 {
        image[vectors + 2 * slot] = 0x00;
        image[vectors + 2 * slot + 1] = 0x80;
    }
    image
}

/// Writes `image` into the temporary folder under `name` and returns its path.
///
/// **Each test uses a name of its own**: the folder is shared and the tests run in parallel, so
/// two fixtures under one name would race and one test would read the other's header.
fn write(name: &str, image: &[u8]) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("zax-harness-vs-{}", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the scratch folder");
    let path = directory.join(name);
    std::fs::write(&path, image).expect("the fixture");
    path
}

fn options(arguments: &[&str]) -> vs::Options {
    let owned: Vec<String> = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect();
    vs::Options::parse(&owned).expect("the arguments parse")
}

fn parse_error(arguments: &[&str]) -> String {
    let owned: Vec<String> = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect();
    match vs::Options::parse(&owned) {
        Ok(_) => panic!("{arguments:?} should have been refused"),
        Err(error) => error.to_string(),
    }
}

/// The report names the PPU, the hardware, the DIP switches and both port reads, and its last
/// line is the one-line summary `--quiet` keeps.
#[test]
fn the_report_names_the_machine_and_the_cabinet() {
    // Byte 13 `$03` is the RP2C04-0002 and byte 15 `$05` is "1P via $4017".
    let path = write("report.nes", &vs_image(99, 4, 2, 0x03, 0x05));
    let report = vs::run(&options(&[
        &path.to_string_lossy(),
        "--dip",
        "0xA5",
        "--coin",
        "1",
        "--service",
    ]))
    .expect("the fixture is a VS image");

    for expected in [
        "RP2C04-0002",
        "Vs. Unisystem",
        "PlayerOneOnPort1",
        "mapper       99",
        "remapped into the ordered palette",
        // $A5 is switches 1, 3, 6 and 8; $4016 then carries switch 1 in bit 3, the service
        // button in bit 2 and coin slot 1 in bit 5, and $4017 switches 3 to 8 as a mask.
        "dip          0xA5 (on: 1,3,6,8)",
        "service      true",
        "coins        1:true 2:false",
        "$4016        0x2C",
        "$4017        0xA4",
        "vs: report.nes, RP2C04-0002, Vs. Unisystem, loaded",
    ] {
        assert!(
            report.contains(expected),
            "{expected:?} missing from\n{report}"
        );
    }
}

/// `--quiet` keeps the summary and drops the detail; `--frames` runs frames and reports a digest.
#[test]
fn quiet_keeps_the_summary_and_frames_reports_a_digest() {
    let path = write("digest.nes", &vs_image(99, 4, 2, 0x03, 0x04));
    let quiet = vs::run(&options(&[
        &path.to_string_lossy(),
        "--quiet",
        "--frames",
        "2",
    ]))
    .expect("the fixture is a VS image");
    assert_eq!(quiet.lines().count(), 1, "{quiet}");
    assert!(quiet.contains("2 frames, palette digest"), "{quiet}");

    // The digest is of the frames, so a different number of them gives a different one, and the
    // same number gives the same one.
    let again = vs::run(&options(&[
        &path.to_string_lossy(),
        "--quiet",
        "--frames",
        "2",
    ]))
    .expect("the fixture is a VS image");
    assert_eq!(quiet, again, "the same run must give the same digest");

    let longer = vs::run(&options(&[
        &path.to_string_lossy(),
        "--quiet",
        "--frames",
        "3",
    ]))
    .expect("the fixture is a VS image");
    assert_ne!(quiet, longer);
}

/// `--press` reaches the cabinet's wiring: with "1P via `$4017`" the two sticks swap and the
/// start buttons do not, which the printed word shows.
#[test]
fn press_goes_through_the_cabinet_wiring() {
    let swapped = write("vs-swapped.nes", &vs_image(99, 4, 2, 0x03, 0x05));
    let report = vs::run(&options(&[
        &swapped.to_string_lossy(),
        "--press",
        "start,right",
        "--frames",
        "1",
    ]))
    .expect("the fixture is a VS image");
    // Player one's Right lands on port 1 ($4017) and its start button stays on port 0 ($4016),
    // in the slot a NES controller puts Select in.
    assert!(report.contains("word         0x8004"), "{report}");

    let plain = write("vs-plain.nes", &vs_image(99, 4, 2, 0x03, 0x04));
    let report = vs::run(&options(&[
        &plain.to_string_lossy(),
        "--press",
        "start,right",
        "--frames",
        "1",
    ]))
    .expect("the fixture is a VS image");
    assert!(report.contains("word         0x0084"), "{report}");
}

/// `--switch n[=on|off]` sets one switch at a time and composes with `--dip`.
#[test]
fn switches_can_be_set_one_at_a_time() {
    let path = write("switches.nes", &vs_image(99, 4, 2, 0x00, 0x04));
    let report = vs::run(&options(&[
        &path.to_string_lossy(),
        "--dip",
        "255",
        "--switch",
        "1=off",
        "--switch",
        "8=off",
    ]))
    .expect("the fixture is a VS image");
    assert!(
        report.contains("dip          0x7E (on: 2,3,4,5,6,7)"),
        "{report}"
    );

    let report = vs::run(&options(&[&path.to_string_lossy(), "--switch", "4"]))
        .expect("the fixture is a VS image");
    assert!(report.contains("dip          0x08 (on: 4)"), "{report}");
}

/// An image that is not a VS. System is refused with a message saying why, and so is a file that
/// is not a ROM.
#[test]
fn a_non_vs_image_is_refused_with_the_reason() {
    let mut image = vs_image(0, 2, 1, 0x00, 0x00);
    image[7] = 0x00; // No console type, no VS bit.
    let path = write("plain.nes", &image);
    let error = vs::run(&options(&[&path.to_string_lossy()]))
        .expect_err("a NES image is not a VS. System")
        .to_string();
    assert!(error.contains("is not a VS. System image"), "{error}");
    assert!(error.contains("the mapper is 0"), "{error}");

    let path = write("not-a-rom.bin", b"this is not a ROM");
    let error = vs::run(&options(&[&path.to_string_lossy()]))
        .expect_err("the file is not a ROM")
        .to_string();
    assert!(error.contains("Corrupt header"), "{error}");
}

/// Mapper 151 reaches the command too, the number alone making the image a VS. System.
#[test]
fn mapper_151_is_accepted() {
    let mut image = vs_image(151, 8, 8, 0x00, 0x00);
    image[7] &= 0xF0; // Drop the NES 2.0 identifier and the console type.
    let path = write("vs151.nes", &image);
    let report = vs::run(&options(&[&path.to_string_lossy()])).expect("mapper 151 is a VS image");
    assert!(report.contains("mapper       151"), "{report}");
    // No header field and no row in the table, so the brief's default and its message.
    assert!(report.contains("unknown VS image"), "{report}");
    assert!(report.contains("plain NES palette"), "{report}");
}

/// Every option the usage names is refused clearly when it is wrong, rather than taken silently.
#[test]
fn bad_arguments_say_what_is_wrong() {
    let path = write("arguments.nes", &vs_image(99, 4, 2, 0x03, 0x04));
    let rom = path.to_string_lossy().into_owned();

    assert!(parse_error(&[]).contains("zax-harness vs"));
    assert!(parse_error(&[&rom, "--switch", "9"]).contains("is not 1 to 8"));
    assert!(parse_error(&[&rom, "--switch", "0"]).contains("is not 1 to 8"));
    assert!(parse_error(&[&rom, "--switch", "3=maybe"]).contains("is not 1 to 8"));
    assert!(parse_error(&[&rom, "--coin", "0"]).contains("is not slot 1 or 2"));
    assert!(parse_error(&[&rom, "--coin", "3"]).contains("is not slot 1 or 2"));
    assert!(parse_error(&[&rom, "--dip", "256"]).contains("is not a byte"));
    assert!(parse_error(&[&rom, "--dip", "0xZZ"]).contains("is not a byte"));
    assert!(parse_error(&[&rom, "--press", "jump"]).contains("is not a button"));
    assert!(parse_error(&[&rom, "--frames", "soon"]).contains("is not a count"));
    // An unknown option and a missing value both print the usage.
    assert!(parse_error(&[&rom, "--nope", "1"]).contains("zax-harness vs"));
    assert!(parse_error(&[&rom, "--dip"]).contains("zax-harness vs"));

    // And `--dip` takes both forms.
    assert!(vs::run(&options(&[&rom, "--dip", "0x0F"]))
        .expect("the fixture is a VS image")
        .contains("dip          0x0F"));
    assert!(vs::run(&options(&[&rom, "--dip", "15"]))
        .expect("the fixture is a VS image")
        .contains("dip          0x0F"));
}
