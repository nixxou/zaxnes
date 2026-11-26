//! Walks right through a game headless with an HD pack and writes a shot every 180 frames:
//! `hd_walk <rom.nes> <pack folder> <out dir> <shots>`. Start is pressed at frame 120; Right is
//! held once $18 reads 5 (Castlevania's play state), the same script as the Mesen comparison.

use std::path::Path;

use zax_core_api::button;
use zax_hdpacks::compose::ChrIndex;
use zax_hdpacks::image::Rgba;
use zax_hdpacks::{compose, FolderFiles, FrameInput};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 5 {
        eprintln!("usage: hd_walk <rom> <pack folder> <out dir> <shots>");
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
    let shots: u32 = args[4].parse().unwrap_or(4);
    let mut core = zax_nes::NesCore::new();
    if !core.load_rom(&image, "game.nes").success {
        eprintln!("ROM not loaded");
        return;
    }
    core.set_tracks_playfield_scroll(true);
    let mut rgb = [[0u8; 3]; 64];
    for (index, colour) in rgb.iter_mut().enumerate() {
        let c = zax_engine3d::palette::nes_color(index);
        *colour = [c.r, c.g, c.b];
    }
    let (mut playing, mut taken, mut frame) = (None::<u64>, 0, 0u64);
    while taken < shots && frame < 20_000 {
        let pad = if (120..124).contains(&frame) {
            u16::from(button::START)
        } else if playing.is_some() {
            u16::from(button::RIGHT)
        } else {
            0
        };
        core.run_frame(pad);
        frame += 1;
        if playing.is_none() && core.read_memory(0x18) == 5 {
            playing = Some(frame);
        }
        let Some(start) = playing else { continue };
        if (frame - start) % 180 != 0 {
            continue;
        }
        let mut picture = Vec::new();
        core.pixel_frame_indices_into(&mut picture);
        let batch = core.frame_batch();
        let memory = |address: u16| core.read_memory(address);
        let input = FrameInput {
            picture: &picture,
            width: 256,
            height: picture.len() / 256,
            margin: 0,
            margin_y: 0,
            records: batch.records,
            palette: batch.palette,
            frame,
            rgb: &rgb,
            memory: &memory,
            scroll: core.playfield_origin(),
        };
        let mut out = Vec::new();
        compose(&pack, &ChrIndex::default(), &input, &mut out);
        #[allow(clippy::cast_possible_truncation)]
        let shot = Rgba {
            width: 256 * pack.scale,
            height: (input.height as u32) * pack.scale,
            data: out
                .iter()
                .enumerate()
                .map(|(at, &v)| if at % 4 == 3 { 255 } else { v })
                .collect(),
        };
        let path = format!("{}/shot{taken:02}_f{frame:05}.png", args[3]);
        let _ = std::fs::write(&path, shot.to_png());
        let ram: Vec<u8> = (0..0x800u16).map(|a| core.read_memory(a)).collect();
        let _ = std::fs::write(path.replace(".png", ".ram"), ram);
        println!("{path}");
        taken += 1;
    }
}
