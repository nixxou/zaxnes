//! The core on real games: `FlappyZaxNes` (the demo kept in `zax-app/demo`, out of git) and,
//! with `ZAXNES_LIBRETRO_ROMS` naming a folder of the owner's ROMs, Kirby's Adventure and The
//! Legend of Zelda. A missing ROM skips its test with a line saying so. With
//! `ZAXNES_LIBRETRO_SHOTS` naming a folder, the pictures are written there as PNG.

use std::path::{Path, PathBuf};

use zax_libretro::core::{Core, Options};

fn demo() -> Option<Vec<u8>> {
    std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zax-app/demo/FlappyZaxNes.nes"),
    )
    .ok()
}

fn owner_rom(name: &str) -> Option<Vec<u8>> {
    let folders = std::env::var_os("ZAXNES_LIBRETRO_ROMS")?;
    std::env::split_paths(&folders).find_map(|folder| std::fs::read(folder.join(name)).ok())
}

fn shot(core: &Core, name: &str) {
    let Some(folder) = std::env::var_os("ZAXNES_LIBRETRO_SHOTS") else {
        return;
    };
    let (pixels, width, height) = core.picture();
    let mut rgb = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        rgb.extend_from_slice(&pixel.to_be_bytes()[1..]);
    }
    let path = Path::new(&folder).join(format!("{name}.png"));
    let Ok(file) = std::fs::File::create(&path) else {
        return;
    };
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut writer) = encoder.write_header() {
        let _ = writer.write_image_data(&rgb);
    }
}

/// Runs `frames` frames, pressing Start on and off every so often to get past the title.
fn play(core: &mut Core, frames: u32) {
    for frame in 0..frames {
        let start = if frame % 60 < 5 { 1 << 3 } else { 0 };
        let a = if frame % 40 < 3 { 1 } else { 0 };
        core.run(start | a);
    }
}

fn widescreen(on: bool) -> Options {
    Options {
        widescreen: on,
        ..Options::default()
    }
}

#[test]
fn the_demo_plays_wide_with_sound_and_switches_while_playing() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut core = Core::load(&rom, "FlappyZaxNes.nes", widescreen(true)).expect("the demo loads");
    play(&mut core, 300);
    let (pixels, width, height) = core.picture();
    // 16:9 asks 88 pixels on each side; the demo's hack draws 80 at most.
    assert_eq!((width, height), (416, 240));
    assert_eq!(pixels.len(), 416 * 240);
    assert_eq!(core.picture_size(), (416, 240));
    assert!(pixels.iter().any(|pixel| *pixel != pixels[0]), "a picture, not one colour");
    // 48 kHz at about 60.1 frames a second: about 799 stereo frames each.
    let frames = core.sound().len() / 2;
    assert!((700..=900).contains(&frames), "{frames} sound frames");
    shot(&core, "flappy-wide");

    // Off while playing: the pack carries the state, so the original ROM runs at once.
    let mut options = core.options().clone();
    options.widescreen = false;
    let changed = core.set_options(options);
    assert!(changed.geometry && changed.hack_options);
    play(&mut core, 30);
    assert_eq!(core.picture().1, 256);
    shot(&core, "flappy-off");
    let mut options = core.options().clone();
    options.widescreen = true;
    assert!(core.set_options(options).hack_options);
    play(&mut core, 30);
    assert_eq!(core.picture().1, 416);
}

#[test]
fn a_state_comes_back_to_the_same_picture() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut core = Core::load(&rom, "FlappyZaxNes.nes", widescreen(true)).expect("the demo loads");
    play(&mut core, 200);
    let mut state = vec![0; core.state_size()];
    assert!(core.save_state(&mut state));
    play(&mut core, 50);
    let first = core.picture().0.to_vec();
    assert!(core.load_state(&state));
    play(&mut core, 50);
    assert_eq!(core.picture().0, first.as_slice());
    assert_eq!(core.system_ram().len(), 2048);
}

#[test]
fn kirby_widens_and_a_game_without_a_hack_does_not() {
    let (Some(kirby), Some(zelda)) = (
        owner_rom("Kirby's Adventure [USA][!!].nes"),
        owner_rom("Legend of Zelda, The (USA).nes"),
    ) else {
        eprintln!("skipped: ZAXNES_LIBRETRO_ROMS does not hold Kirby's Adventure and Zelda");
        return;
    };
    let mut core = Core::load(&kirby, "Kirby.nes", widescreen(true)).expect("Kirby loads");
    assert!(core.patch_name().is_some());
    for step in 1..=4 {
        play(&mut core, 600);
        assert_eq!(core.picture().1, 432);
        shot(&core, &format!("kirby-wide-{step}"));
    }
    let mut core = Core::load(&zelda, "Zelda.nes", widescreen(true)).expect("Zelda loads");
    play(&mut core, 300);
    assert_eq!(core.picture().1, 256);
    assert!(core.save_ram().len() >= 8192, "Zelda has a battery");
    shot(&core, "zelda");
}
