//! The 3D of the core drawn into an offscreen OpenGL context, the way the frontend's framebuffer
//! receives it: `FlappyZaxNes` (the demo kept in `zax-app/demo`, out of git) and, from
//! `ZAXNES_LIBRETRO_ROMS`, Faxanadu with its bundled profile. A missing ROM, profile or OpenGL
//! skips the test with a line saying so; `ZAXNES_LIBRETRO_SHOTS` names a folder for the pictures.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use zax_libretro::core::{Core, Options};
use zax_libretro::profiles;
use zax_render::offscreen::{ContextApi, OffscreenContext};
use zax_render::SceneRenderer;

const SIZE: (i32, i32) = (640, 360);

fn rom(name: &str) -> Option<Vec<u8>> {
    let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zax-app/demo");
    let mut folders = vec![demo];
    if let Some(list) = std::env::var_os("ZAXNES_LIBRETRO_ROMS") {
        folders.extend(std::env::split_paths(&list));
    }
    folders
        .iter()
        .find_map(|folder| std::fs::read(folder.join(name)).ok())
}

/// A game loaded in 3D, or none with a line saying why.
fn three_d(name: &str) -> Option<Core> {
    three_d_with(name, Options::default())
}

fn three_d_with(name: &str, options: Options) -> Option<Core> {
    let Some(image) = rom(name) else {
        eprintln!("skipped: no {name}");
        return None;
    };
    let mut core = Core::load(&image, name, options).ok()?;
    // FlappyZaxNes's profile is not in the core (owner decision): the tests take the demo's.
    let demo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../zax-app/demo/flappyzaxnes.3dn");
    let own = (name == "FlappyZaxNes.nes")
        .then(|| std::fs::read(&demo).ok())
        .flatten()
        .and_then(|bytes| zax_profiles::format::load(&bytes, None, None).ok());
    let Some(document) = own.or_else(|| profiles::find(name, &image, None)) else {
        eprintln!("skipped: no profile for {name}");
        return None;
    };
    let tiles = document.hash_state().map(|state| state.tiles.clone());
    core.start_three_d(&document.profile, tiles, |line| eprintln!("{line}"))
        .ok()?;
    Some(core)
}

struct Gpu {
    context: OffscreenContext,
    renderer: SceneRenderer,
}

fn gpu() -> Option<Gpu> {
    let context = OffscreenContext::new(SIZE.0, SIZE.1, ContextApi::DesktopCore)
        .map_err(|error| eprintln!("skipped: no OpenGL ({error})"))
        .ok()?;
    let renderer = SceneRenderer::new(Rc::clone(&context.gl()), None).ok()?;
    Some(Gpu { context, renderer })
}

/// Draws the game's 3D and gives the picture as RGBA rows, the top one first.
fn draw(gpu: &mut Gpu, core: &mut Core) -> Vec<u8> {
    let target = gpu.context.target();
    let Some(three_d) = core.three_d_mut() else {
        return Vec::new();
    };
    let (scene, frame) = three_d.scene_and_frame();
    if gpu.renderer.render(&target, scene, &frame).is_err() {
        return Vec::new();
    }
    gpu.context.finish();
    gpu.context.read_pixels()
}

fn shot(rgba: &[u8], name: &str) {
    let Some(folder) = std::env::var_os("ZAXNES_LIBRETRO_SHOTS") else {
        return;
    };
    let path = Path::new(&folder).join(format!("{name}.png"));
    let Ok(file) = std::fs::File::create(&path) else {
        return;
    };
    let width = u32::try_from(SIZE.0).unwrap_or(1);
    let height = u32::try_from(SIZE.1).unwrap_or(1);
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    if let Ok(mut writer) = encoder.write_header() {
        let _ = writer.write_image_data(rgba);
    }
}

/// Runs `frames` frames with `input(frame)`, the camera following at 60 frames a second.
fn play(core: &mut Core, frames: u32, input: impl Fn(u32) -> u16) {
    for frame in 0..frames {
        core.run(input(frame));
        if let Some(three_d) = core.three_d_mut() {
            three_d.advance(1.0 / 60.0, (0.0, 0.0), 0.0);
        }
    }
}

fn varied(rgba: &[u8]) -> bool {
    rgba.get(..4)
        .is_some_and(|first| rgba.chunks_exact(4).any(|pixel| pixel != first))
}

#[test]
fn the_demo_draws_in_3d() {
    let Some(mut core) = three_d("FlappyZaxNes.nes") else {
        return;
    };
    let Some(mut gpu) = gpu() else {
        return;
    };
    play(&mut core, 240, |_| 0);
    let picture = draw(&mut gpu, &mut core);
    assert!(varied(&picture));
    shot(&picture, "3d-flappy");
    // The camera turned by the right stick for half a second.
    if let Some(three_d) = core.three_d_mut() {
        for _ in 0..30 {
            three_d.advance(1.0 / 60.0, (1.0, 0.0), 0.0);
        }
    }
    let turned = draw(&mut gpu, &mut core);
    assert_ne!(picture, turned, "the stick turns the camera");
    shot(&turned, "3d-flappy-turned");
}

#[test]
fn faxanadu_plays_in_3d() {
    let Some(mut core) = three_d("Faxanadu [USA][!!].nes") else {
        return;
    };
    let Some(mut gpu) = gpu() else {
        return;
    };
    // The title, Start, then walk right through Eolis.
    play(&mut core, 1800, |frame| {
        let start = matches!(frame % 300, 250..=255);
        let right = frame >= 1000;
        u16::from(start) << 3 | u16::from(right) << 7
    });
    let picture = draw(&mut gpu, &mut core);
    assert!(varied(&picture));
    shot(&picture, "3d-faxanadu");
}

#[test]
fn the_render_options_reach_the_scene_at_once() {
    let Some(mut core) = three_d("FlappyZaxNes.nes") else {
        return;
    };
    play(&mut core, 60, |_| 0);
    let mut options = core.options().clone();
    options.render.anti_alias = 3;
    options.render.enhanced = true;
    options.render.reflections = true;
    options.render.skybox = 2;
    core.set_options(options);
    let scene = core.three_d_mut().expect("in 3D").scene();
    assert_eq!(scene.settings.anti_alias(), 3);
    assert!(scene.settings.style.enhanced && scene.settings.style.reflections);
    assert_eq!(scene.settings.skybox, zax_render::scene::SkyboxKind::Fantasy);
    // Drawn so, without an error.
    let Some(mut gpu) = gpu() else {
        return;
    };
    play(&mut core, 60, |_| 0);
    let picture = draw(&mut gpu, &mut core);
    assert!(varied(&picture));
    shot(&picture, "3d-flappy-enhanced");
}

#[test]
fn y_and_l2_work_as_in_the_application() {
    use zax_libretro::three_d::CameraNews;
    let Some(mut core) = three_d("FlappyZaxNes.nes") else {
        return;
    };
    play(&mut core, 30, |_| 0);
    let frame = 1.0 / 60.0;
    let three_d = core.three_d_mut().expect("in 3D");
    // A short press of Y: 2D, another: 3D.
    for held in [true, true, false] {
        three_d.buttons(frame, held, false);
    }
    assert!(three_d.flat);
    for held in [true, false] {
        three_d.buttons(frame, held, false);
    }
    assert!(!three_d.flat);
    // Y held: 2D head on, staying so once let go.
    for _ in 0..40 {
        three_d.buttons(frame, true, false);
    }
    three_d.buttons(frame, false, false);
    assert!(three_d.flat && three_d.scene().picture_head_on);
    three_d.flat = false;
    // L2 held keeps the camera; turned, a short press puts it back.
    let news: Vec<_> = (0..40)
        .filter_map(|_| three_d.buttons(frame, false, true))
        .collect();
    assert_eq!(news, [CameraNews::Kept]);
    assert_eq!(three_d.buttons(frame, false, false), None);
    let kept = three_d.scene().camera.current().angles;
    for _ in 0..30 {
        three_d.advance(frame, (1.0, 0.0), 0.0);
    }
    assert_ne!(three_d.scene().camera.current().angles, kept);
    three_d.buttons(frame, false, true);
    assert_eq!(
        three_d.buttons(frame, false, false),
        Some(CameraNews::Restored)
    );
    assert_eq!(three_d.scene().camera.current().angles, kept);
}

#[test]
fn the_optimised_lua_plays_the_same() {
    let options = Options {
        optimized_lua: true,
        ..Options::default()
    };
    let (Some(mut fast), Some(mut plain)) = (
        three_d_with("FlappyZaxNes.nes", options),
        three_d("FlappyZaxNes.nes"),
    ) else {
        return;
    };
    let input = |frame: u32| u16::from(frame % 40 < 3) | u16::from(frame == 100) << 3;
    play(&mut fast, 400, input);
    play(&mut plain, 400, input);
    assert_eq!(fast.system_ram(), plain.system_ram());
}

/// `RetroArch`'s run-ahead of one frame: each frame runs, is saved, the next one runs ahead
/// with the same buttons and is shown, and the state comes back.
#[test]
fn the_frontends_run_ahead_keeps_the_game_on_its_course() {
    let name = "Faxanadu [USA][!!].nes";
    let (Some(mut ahead), Some(mut plain)) = (three_d(name), three_d(name)) else {
        return;
    };
    let input = |frame: u32| {
        let start = matches!(frame % 300, 250..=255);
        let right = frame >= 1000;
        u16::from(start) << 3 | u16::from(right) << 7
    };
    let mut state = vec![0; ahead.state_size()];
    for frame in 0..1500 {
        plain.run(input(frame));
        ahead.run(input(frame));
        assert!(ahead.save_state(&mut state));
        ahead.run(input(frame));
        assert!(ahead.load_state(&state));
    }
    assert_eq!(ahead.system_ram(), plain.system_ram());
    let Some(mut gpu) = gpu() else {
        return;
    };
    let picture = draw(&mut gpu, &mut ahead);
    assert!(varied(&picture));
    shot(&picture, "3d-faxanadu-run-ahead");
}

#[test]
fn a_3d_state_comes_back() {
    let Some(mut core) = three_d("FlappyZaxNes.nes") else {
        return;
    };
    play(&mut core, 120, |_| 0);
    let mut state = vec![0; core.state_size()];
    assert!(core.save_state(&mut state));
    play(&mut core, 60, |frame| u16::from(frame % 20 < 2));
    assert!(core.load_state(&state));
    let mut again = vec![0; core.state_size()];
    assert!(core.save_state(&mut again));
    assert_eq!(
        state, again,
        "the machine and the scripts' PData as they were"
    );
}
