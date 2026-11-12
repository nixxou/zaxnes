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
        let a = u16::from(frame % 40 < 3);
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
    assert!(
        pixels.iter().any(|pixel| *pixel != pixels[0]),
        "a picture, not one colour"
    );
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
fn the_overscan_crops_and_the_extra_margin_widens() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut core = Core::load(&rom, "FlappyZaxNes.nes", widescreen(false)).expect("the demo loads");
    let mut options = core.options().clone();
    options.overscan = [8, 8, 8, 8];
    assert!(core.set_options(options).geometry);
    assert_eq!(core.picture_size(), (240, 224));
    play(&mut core, 30);
    let (pixels, width, height) = core.picture();
    assert_eq!((width, height, pixels.len()), (240, 224, 240 * 224));
    shot(&core, "flappy-overscan");

    // Widened, the left and right sides stay: the margin goes on past them.
    let mut options = core.options().clone();
    options.widescreen = true;
    options.overscan = [8, 0, 8, 8];
    core.set_options(options);
    play(&mut core, 30);
    assert_eq!(core.picture().1, 416);
    assert_eq!(core.picture().2, 232);
    // 25 % more: the margin asked for grows, the demo drawing 80 pixels at most.
    let mut options = core.options().clone();
    options.overscan = [0; 4];
    options.extra_percent = 25;
    assert!(core.set_options(options).geometry);
    play(&mut core, 30);
    assert_eq!(core.picture().1, 416);
}

/// How far left, in output pixels, the playfield of `after` lies from `before`'s: the shift that
/// best matches row 120 (the pipes and the sky), out of 1024.
fn offset(before: &[u32], after: &[u32]) -> i32 {
    let row = |picture: &[u32]| picture[120 * 1024..121 * 1024].to_vec();
    let (before, after) = (row(before), row(after));
    (-24..=24)
        .min_by_key(|shift: &i32| {
            (64..960)
                .filter(|x: &i32| {
                    let source = usize::try_from(x + shift).unwrap_or(0);
                    after[usize::try_from(*x).unwrap_or(0)] != before[source]
                })
                .count()
        })
        .unwrap_or(0)
}

/// At 120 Hz each frame takes two calls; the second shows the playfield half way to the next
/// frame's, the frame's sound split between the two.
#[test]
fn smooth_scrolling_draws_a_picture_between_two_frames() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut core = Core::load(&rom, "FlappyZaxNes.nes", widescreen(false)).expect("the demo loads");
    let normal = core.frames_per_second();
    let mut options = core.options().clone();
    options.smooth_120 = true;
    let changed = core.set_options(options);
    assert!(changed.timing && changed.geometry);
    assert!((core.frames_per_second() - 2.0 * normal).abs() < 1e-9);
    assert_eq!(core.picture_size(), (1024, 240));
    assert!((core.aspect_ratio() - 256.0 / 240.0).abs() < 1e-6);
    // Past the title, flying: the playfield scrolls.
    let mut between = 0;
    let mut sound = 0;
    let mut last_first: Option<Vec<u32>> = None;
    let mut halves: Vec<i32> = Vec::new();
    let mut checked = 0;
    for frame in 0..600_u32 {
        let start = if (300..306).contains(&frame) { 1 << 3 } else { 0 };
        let flap = u16::from(frame % 30 < 2);
        core.run(start | flap);
        let first = core.picture().0.to_vec();
        sound += core.sound().len();
        // The picture between the frame before and this one: half its offset.
        if let (Some(before), Some(middle)) = (&last_first, halves.last().copied()) {
            if (310..400).contains(&frame) {
                let whole = offset(before, &first);
                if whole != 0 {
                    checked += 1;
                    assert!(
                        (middle * 2 - whole).abs() <= 1,
                        "frame {frame}: between {middle}, whole {whole}"
                    );
                }
            }
        }
        core.run(start | flap);
        sound += core.sound().len();
        assert_eq!(core.picture().1, 1024);
        halves.push(offset(&first, core.picture().0));
        if (310..400).contains(&frame) && core.picture().0 != first.as_slice() {
            between += 1;
        }
        last_first = Some(first);
        if frame == 500 {
            shot(&core, "flappy-120-between");
        }
    }
    // Flying from frame 306: the playfield scrolls, a pixel every frame or two.
    eprintln!("{between} pictures between of 90, {checked} half way checked");
    assert!(checked > 40);
    assert!(between > 40, "the picture between moves: {between}");
    // About 800 stereo frames a game frame, split in two.
    let frames = sound / 2 / 600;
    assert!((700..=900).contains(&frames), "{frames}");
}

/// The frontend's run-ahead at 120 Hz: a state saved between the two calls of a frame keeps the
/// picture between owed, so the game keeps its pace.
#[test]
fn smooth_scrolling_keeps_its_pace_under_the_frontends_run_ahead() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let smooth = Options {
        widescreen: false,
        smooth_120: true,
        ..Options::default()
    };
    let mut plain = Core::load(&rom, "FlappyZaxNes.nes", smooth.clone()).expect("the demo loads");
    let mut ahead = Core::load(&rom, "FlappyZaxNes.nes", smooth).expect("the demo loads");
    let mut state = vec![0; ahead.state_size()];
    for call in 0..800_u32 {
        let input = if (600..612).contains(&call) { 1 << 3 } else { 0 };
        plain.run(input);
        ahead.run(input);
        assert!(ahead.save_state(&mut state));
        ahead.run(input);
        assert!(ahead.load_state(&state));
    }
    assert_eq!(ahead.system_ram(), plain.system_ram());
}

/// Kirby's hack reports its camera: the picture between follows it, the run-ahead off.
#[test]
fn kirby_scrolls_smoothly_from_its_camera() {
    let Some(kirby) = owner_rom("Kirby's Adventure [USA][!!].nes") else {
        eprintln!("skipped: ZAXNES_LIBRETRO_ROMS does not hold Kirby's Adventure");
        return;
    };
    let options = Options {
        smooth_120: true,
        smooth_ahead: false,
        hacks: vec![("smooth_scroll".to_owned(), true)],
        ..Options::default()
    };
    let mut core = Core::load(&kirby, "Kirby.nes", options).expect("Kirby loads");
    let mut between = 0;
    for frame in 0..2400_u32 {
        // Start on the title and on the file screen, then right through the level.
        let start = u16::from(matches!(frame, 300..=303 | 500..=503 | 700..=703)) << 3;
        let right = u16::from(frame >= 1200) << 7;
        core.run(start | right);
        let first = core.picture().0.to_vec();
        core.run(start | right);
        if frame >= 1200 && core.picture().0 != first.as_slice() {
            between += 1;
        }
        if frame == 2000 {
            shot(&core, "kirby-120-between");
        }
    }
    eprintln!("Kirby: {between} pictures between moved of 1200");
    assert!(between > 100);
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

#[test]
fn the_demo_makes_sound() {
    let Some(rom) = demo() else {
        eprintln!("skipped: no FlappyZaxNes.nes in zax-app/demo");
        return;
    };
    let mut core = Core::load(&rom, "FlappyZaxNes.nes", widescreen(true)).expect("the demo loads");
    let mut loudest = 0_i32;
    let mut frames = 0_usize;
    // The title is silent: the game starts at Start.
    for frame in 0..600_u32 {
        core.run(if (300..306).contains(&frame) {
            1 << 3
        } else {
            0
        });
        frames += core.sound().len() / 2;
        loudest = core
            .sound()
            .iter()
            .fold(loudest, |most, sample| most.max(i32::from(*sample).abs()));
    }
    eprintln!("600 frames: {frames} sound frames, loudest {loudest}");
    // 10 s at 48 kHz, give or take what the APU keeps in its ring.
    assert!((470_000..=490_000).contains(&frames));
    assert!(loudest > 1000, "the game makes sound");
}
