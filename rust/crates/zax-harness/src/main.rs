#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port.

use std::path::Path;
use std::process::ExitCode;

use zax_harness::{compare, golden_core, lua_api, meshes, render, HarnessError};

const USAGE: &str = "usage: zax-harness golden-core <roms dir> <out.jsonl> [frames]
       zax-harness meshes --profile <file|name> [options] (zax-harness help for the options)
       zax-harness lua-api [--out <file>]
       zax-harness render --out <dir> [--frames synthetic[:<scene>]] [options]
       zax-harness compare <reference> <capture> [--tolerance <levels>] [--min <percent>]";

fn run(args: &[String]) -> Result<(), HarnessError> {
    match args {
        [command, roms_dir, out_path, rest @ ..] if command == "golden-core" => {
            let frames = match rest {
                [] => golden_core::DEFAULT_FRAMES,
                [count] => count.parse().map_err(|_| {
                    HarnessError::Usage(format!("{count:?} is not a frame count\n{USAGE}"))
                })?,
                _ => return Err(HarnessError::Usage(USAGE.to_owned())),
            };

            let summary = golden_core::run(Path::new(roms_dir), Path::new(out_path), frames)?;
            println!(
                "golden-core: {} ROMs run, {} files, {frames} frames each -> {out_path}",
                summary.run, summary.files
            );
            Ok(())
        }
        [command, rest @ ..] if command == "meshes" => {
            let options = meshes::Options::parse(rest)?;
            println!("{}", meshes::run(&options)?);
            Ok(())
        }
        [command, rest @ ..] if command == "lua-api" => {
            let options = lua_api::Options::parse(rest)?;
            if let Some(message) = lua_api::run(&options)? {
                println!("{message}");
            }
            Ok(())
        }
        [command, rest @ ..] if command == "render" => {
            let options = render::Options::parse(rest)?;
            println!("{}", render::run(&options)?);
            Ok(())
        }
        [command, rest @ ..] if command == "compare" => {
            let options = compare::Options::parse(rest)?;
            println!("{}", compare::run(&options)?);
            Ok(())
        }
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => {
            println!(
                "{USAGE}\n\n{}\n\n{}\n\n{}\n\n{}",
                meshes::USAGE,
                lua_api::USAGE,
                render::USAGE,
                compare::USAGE
            );
            Ok(())
        }
        _ => Err(HarnessError::Usage(USAGE.to_owned())),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("zax-harness: {error}");
            ExitCode::FAILURE
        }
    }
}
