//! Plays a ROM headless with a pad script and writes the frame drawn with an HD pack, to check a
//! pack outside the application: `hd_shot <rom.nes> <pack folder> <out.png> <frames> [start
//! frames...]` (Start is pressed for 4 frames at each listed frame).

use std::path::Path;

use zax_core_api::button;
use zax_hdpacks::compose::ChrIndex;
use zax_hdpacks::image::Rgba;
use zax_hdpacks::{compose, FolderFiles, FrameInput};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: hd_shot <rom> <pack folder> <out.png> <frames> [start frames...]");
        return;
    }
    let Ok(image) = std::fs::read(&args[1]) else {
        eprintln!("cannot read {}", args[1]);
        return;
    };
    let pack = match zax_hdpacks::pack::load(&FolderFiles(Path::new(&args[2]).to_path_buf())) {
        Ok(pack) => pack,
        Err(error) => {
            eprintln!("{error}");
            return;
        }
    };
    let frames: u64 = args[4].parse().unwrap_or(600);
    let starts: Vec<u64> = args[5..].iter().filter_map(|a| a.parse().ok()).collect();
    let mut core = zax_nes::NesCore::new();
    let loaded = core.load_rom(&image, "game.nes");
    if !loaded.success {
        eprintln!("ROM not loaded");
        return;
    }
    for frame in 0..frames {
        let pressed = starts.iter().any(|&s| (s..s + 4).contains(&frame));
        core.run_frame(if pressed { u16::from(button::START) } else { 0 });
    }
    let mut picture = Vec::new();
    core.pixel_frame_indices_into(&mut picture);
    let batch = core.frame_batch();
    let mut rgb = [[0u8; 3]; 64];
    for (index, colour) in rgb.iter_mut().enumerate() {
        let c = zax_engine3d::palette::nes_color(index);
        *colour = [c.r, c.g, c.b];
    }
    let memory = |address: u16| core.read_memory(address);
    let input = FrameInput {
        picture: &picture,
        width: 256,
        height: picture.len() / 256,
        margin: 0,
        margin_y: 0,
        records: batch.records,
        palette: batch.palette,
        frame: frames,
        rgb: &rgb,
        memory: &memory,
    };
    let mut out = Vec::new();
    compose(&pack, &ChrIndex::default(), &input, &mut out);
    let scale = pack.scale;
    #[allow(clippy::cast_possible_truncation)]
    let shot = Rgba {
        width: 256 * scale,
        height: (input.height as u32) * scale,
        data: out
            .iter()
            .enumerate()
            .map(|(at, &v)| if at % 4 == 3 { 255 } else { v })
            .collect(),
    };
    let _ = std::fs::write(&args[3], shot.to_png());
    println!("{} records, written {}", batch.records.len(), args[3]);
}
