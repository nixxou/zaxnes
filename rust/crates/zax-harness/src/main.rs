#![forbid(unsafe_code)]

//! zax-harness: headless tools of the port.

use std::path::Path;
use std::process::ExitCode;

#[cfg(feature = "vs-system")]
use zax_harness::vs;
use zax_harness::{
    bench_core, classify, compare, golden_core, lua_api, meshes, packs, profile_tool, render,
    test_roms, HarnessError,
};
#[cfg(feature = "fceux-core")]
use zax_harness::{fceux_bench, fceux_compare, fceux_run, fceux_tiles};

const USAGE: &str = "usage: zax-harness golden-core <roms dir> <out.jsonl> [frames]
       zax-harness test-roms <roms dir> <out.json> [--filter <substring>] [options]
       zax-harness meshes --profile <file|name> [options] (zax-harness help for the options)
       zax-harness lua-api [--out <file>]
       zax-harness render --out <dir> [--frames synthetic[:<scene>]] [options]
       zax-harness bench-core <rom.nes> [options] (ms per frame of each stage)
       zax-harness compare <reference> <capture> [--tolerance <levels>] [--min <percent>]
       zax-harness packs <roms dir> (how the built-in hack packs apply to each ROM)
       zax-harness profile-dump <file.3dn> [--tiles] (the patterns and their 3D looks)
       zax-harness profile-build <description.json> <out.3dn> (a profile from JSON)
       zax-harness classify <rules.json> --out <classified.json> [options] (tiles of dumps to patterns)
       zax-harness vs <rom.nes> [--dip <byte>] [--coin <slot>] [--frames <n>] (a VS. System)
       zax-harness fceux-run <roms dir|rom.nes> <out.jsonl> [frames] (golden-core on FCEUX)
       zax-harness fceux-frame <rom.nes> <out.json> [frames] (FCEUX's tile records)
       zax-harness fceux-compare <zaxnes.jsonl> <fceux.jsonl> [options] (the two engines)
       zax-harness fceux-tiles <roms dir|rom.nes> <out.jsonl> [options] (the records, field by field)
       zax-harness fceux-bench <rom.nes> --engine fceux|zaxnes [options] (what the capture costs)";

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
        [command, rest @ ..] if command == "test-roms" => {
            let options = test_roms::Options::parse(rest)?;
            let summary = test_roms::run(&options)?;
            println!(
                "test-roms: {} ROMs, {} pass, {} fail, {} timeout, {} unsupported, {} manual",
                summary.total,
                summary.pass,
                summary.fail,
                summary.timeout,
                summary.unsupported,
                summary.manual
            );
            Ok(())
        }
        [command, dir] if command == "packs" => {
            print!("{}", packs::run(Path::new(dir))?);
            Ok(())
        }
        [command, file, rest @ ..] if command == "profile-dump" => {
            let tiles = rest.iter().any(|flag| flag == "--tiles");
            print!("{}", profile_tool::dump(Path::new(file), tiles)?);
            Ok(())
        }
        [command, description, out] if command == "profile-build" => {
            println!(
                "{}",
                profile_tool::build(Path::new(description), Path::new(out))?
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
        [command, rest @ ..] if command == "bench-core" => {
            println!("{}", bench_core::run(&bench_core::Options::parse(rest)?)?);
            Ok(())
        }
        [command, rest @ ..] if command == "classify" => {
            println!("{}", classify::run(&classify::Options::parse(rest)?)?);
            Ok(())
        }
        #[cfg(feature = "vs-system")]
        [command, rest @ ..] if command == "vs" => {
            let options = vs::Options::parse(rest)?;
            print!("{}", vs::run(&options)?);
            Ok(())
        }
        [command, rest @ ..] if command == "compare" => {
            let options = compare::Options::parse(rest)?;
            println!("{}", compare::run(&options)?);
            Ok(())
        }
        [command] if matches!(command.as_str(), "help" | "--help" | "-h") => {
            print_help();
            Ok(())
        }
        // The FCEUX engine's five, which only exist in a build with it; their own function,
        // tried last, because the command names above are all distinct from theirs.
        _ => fceux_command(args).unwrap_or_else(|| Err(HarnessError::Usage(USAGE.to_owned()))),
    }
}

/// The FCEUX engine's own subcommands, or `None` when `args` names none of them.
///
/// Its own function for the reason [`print_help`] is one: `run` is two lines inside clippy's
/// hundred with the five here, and five past it with them inlined — and the budget counts the
/// source lines, so `#[cfg]`-ing them out of the build does not help.
#[cfg(feature = "fceux-core")]
fn fceux_command(args: &[String]) -> Option<Result<(), HarnessError>> {
    match args {
        [command, roms, out_path, rest @ ..] if command == "fceux-run" => {
            Some(fceux_run::command(roms, out_path, rest))
        }
        [command, rom, out_path, rest @ ..] if command == "fceux-frame" => {
            Some(fceux_run::frame_command(rom, out_path, rest))
        }
        [command, rest @ ..] if command == "fceux-tiles" => Some(fceux_tiles::command(rest)),
        [command, rest @ ..] if command == "fceux-bench" => Some(fceux_bench::command(rest)),
        [command, rest @ ..] if command == "fceux-compare" => Some(fceux_compare::command(rest)),
        _ => None,
    }
}

/// Without the engine there are none, so every one of its names falls through to the usage.
#[cfg(not(feature = "fceux-core"))]
fn fceux_command(_args: &[String]) -> Option<Result<(), HarnessError>> {
    None
}

/// The whole help: the one-line usage of every subcommand, then each one's own paragraph.
///
/// Apart from `run` (LUA-MERGE: both sides of the merge added subcommands to it — `test-roms`
/// and `vs` on one side, `classify` on the other — and together they took it past the hundred
/// lines clippy allows). Each command's paragraph joins the list only when it exists in this
/// build.
fn print_help() {
    // The `vs` usage joins the list only when the command exists.
    #[cfg(feature = "vs-system")]
    let vs_usage = format!("\n\n{}", vs::USAGE);
    #[cfg(not(feature = "vs-system"))]
    let vs_usage = String::new();
    // And the two FCEUX commands only in a build with the engine.
    #[cfg(feature = "fceux-core")]
    let fceux_usage = format!(
        "\n\n{}\n\n{}\n\n{}\n\n{}",
        fceux_run::USAGE,
        fceux_compare::USAGE,
        fceux_tiles::USAGE,
        fceux_bench::USAGE
    );
    #[cfg(not(feature = "fceux-core"))]
    let fceux_usage = String::new();
    println!(
        "{USAGE}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}{vs_usage}{fceux_usage}",
        test_roms::USAGE,
        meshes::USAGE,
        lua_api::USAGE,
        render::USAGE,
        compare::USAGE,
        bench_core::USAGE,
        profile_tool::USAGE,
        classify::USAGE
    );
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
