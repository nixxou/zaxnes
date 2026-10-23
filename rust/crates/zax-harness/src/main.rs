#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port.

use std::path::Path;
use std::process::ExitCode;

use zax_harness::{golden_core, meshes, HarnessError};

const USAGE: &str = "usage: zax-harness golden-core <roms dir> <out.jsonl> [frames]
       zax-harness meshes --profile <file|name> [options] (zax-harness help for the options)";

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
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => {
            println!("{USAGE}\n\n{}", meshes::USAGE);
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
