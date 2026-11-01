//! The program's entry: the command line of `Program.cs`, then the mode it asks for. A normal
//! launch opens the application; `--help`, `--list-profiles` and `--import-3dsen` print and exit
//! without a window.

use std::io;
use std::path::Path;

use crate::app_options::{self, AppOptions, RUST_ONLY_USAGE, USAGE};
use crate::application::ZaxApplication;
use crate::handheld_mode::{self, DisplayEnvironment};
use crate::single_instance::{SingleInstance, ALREADY_RUNNING_MESSAGE};
use crate::three_d_sen_command;
use crate::{neutral_assets, profile_resolver};
use zax_profiles::catalogue::CatalogueEntry;
use zax_profiles::format;

/// Exit code of a run that worked.
const EXIT_SUCCESS: i32 = 0;

/// Exit code of a run that failed.
const EXIT_FAILURE: i32 = 1;

/// Exit code of a command line that cannot be read.
const EXIT_USAGE: i32 = 2;

/// Reads the command line `arguments` (the program's name left out) and runs the mode it asks
/// for; returns the exit code. The `zaxnes` executable passes its own; the Android library its
/// built one.
#[must_use]
pub fn run(arguments: &[String]) -> i32 {
    // Before anything prints: `--help`, `--list-profiles`, import output and usage errors.
    zax_platform::console::attach_to_parent_console();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let options = match AppOptions::parse(&arguments) {
        Ok(mut options) => {
            options.system_locale = sys_locale::get_locale();
            options.handheld =
                handheld_mode::resolve(options.handheld, &DisplayEnvironment::of_process());
            if options.catch_up.is_none() {
                if let Ok(value) = std::env::var("ZAXNES_CATCH_UP") {
                    match app_options::parse_catch_up(&value) {
                        Ok(catch_up) => options.catch_up = Some(catch_up),
                        Err(error) => eprintln!("ZAXNES_CATCH_UP ignored: {error}"),
                    }
                }
            }
            options
        }
        Err(error) => {
            eprintln!("{error}");
            eprint!("{USAGE}{RUST_ONLY_USAGE}");
            return EXIT_USAGE;
        }
    };

    if options.show_help {
        print!("{USAGE}{RUST_ONLY_USAGE}");
        return EXIT_SUCCESS;
    }
    if options.list_profiles {
        return list_profiles(&options);
    }
    if options.import_three_d_sen_path.is_some() {
        return import_three_d_sen(&options);
    }

    // REQ-LOOP-054: a second launch on the same user root closes rather than opening a second
    // window over the same settings, library and slot files. The guard is released when this
    // function returns, abnormal exits included.
    let instance = SingleInstance::acquire(options.user_data_root.as_deref().map(Path::new));
    if !instance.is_owner() {
        eprintln!("{ALREADY_RUNNING_MESSAGE}");
        return EXIT_SUCCESS;
    }
    ZaxApplication::new(options).run()
}

/// `--import-3dsen`: imports without a window, then exits.
fn import_three_d_sen(options: &AppOptions) -> i32 {
    let outcome =
        three_d_sen_command::run(options, &mut io::stdout().lock(), &mut io::stderr().lock());
    outcome.unwrap_or_else(|error| {
        eprintln!("{error}");
        EXIT_FAILURE
    })
}

/// `--list-profiles`: prints the catalogue, marking the profiles with no Lua script, which render
/// fully today.
fn list_profiles(options: &AppOptions) -> i32 {
    let root = neutral_assets::locate(options.asset_root.as_deref().map(Path::new));
    let entries = match profile_resolver::list(root.as_deref()) {
        Ok(entries) => entries,
        Err(message) => {
            eprintln!("{message}");
            return EXIT_FAILURE;
        }
    };

    // AMD-024 REQ-PUB-001: with no asset folder the catalogue is the built-in one, the two
    // generic entries written in code (Q-065). Nothing is an error here any more.
    match &root {
        None => println!(
            "Zaxnes: built-in entries (no asset folder; pass --assets <dir> for your own files)"
        ),
        Some(root) => println!("Zaxnes: built-in profiles of {}", root.display()),
    }
    println!("  '*' marks a profile with no Lua script, which renders fully today.");
    println!();
    for entry in &entries {
        let scripts = script_count(root.as_deref(), entry);
        let mark = if scripts == Some(0) { '*' } else { ' ' };
        let count = scripts.map_or_else(|| "-1".to_owned(), |count| count.to_string());
        println!("  {mark} {}  [{count} script(s)]", entry.title);
    }
    EXIT_SUCCESS
}

/// The number of Lua scripts of the profile file of `entry`: 0 for a built-in entry, and `None`
/// when the file cannot be read or is not a profile.
fn script_count(root: Option<&Path>, entry: &CatalogueEntry) -> Option<usize> {
    let Some(root) = root.filter(|_| !entry.file.is_empty()) else {
        return Some(0);
    };
    let path = root.join(&entry.file);
    let bytes = std::fs::read(&path).ok()?;
    let document = format::load(&bytes, None, path.to_str()).ok()?;
    Some(document.profile.scripts.len())
}
