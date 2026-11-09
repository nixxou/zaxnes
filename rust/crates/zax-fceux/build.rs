//! Links the static library the FCEUX fork builds — and does nothing at all without the
//! `fceux-core` feature.
//!
//! The library is **not** built from here. FCEUX's sources live only in the fork
//! (`cleanroom/fceux-zax`), never in this repository, and the fork's own `CMake` project is the
//! one build system for them: duplicating its ~240-file source list in a Rust build script would
//! be a second place to keep right, and driving `CMake` from inside `cargo build` would need it
//! on `PATH` (on Windows it ships inside Visual Studio and is not there). So the fork is built
//! once with its documented command and this script only finds the result and tells `rustc` to
//! link it.
//!
//! Where it looks, in order:
//!
//! 1. `ZAX_FCEUX_LIB` — a directory holding the built library. Checked first so a CI job or a
//!    cross build can point anywhere.
//! 2. `ZAX_FCEUX_DIR` — the fork's root; its `build-msvc/` and `build-linux/` are tried.
//! 3. a directory named `fceux-zax` beside any ancestor of this crate — the fork is a sibling of
//!    the worktree in a normal checkout (`cleanroom/fceux-zax` next to `cleanroom/wt-fceux`), and
//!    walking up finds it wherever the worktree happens to sit.
//!
//! Nothing is ever downloaded. If the library is not found the script fails with the exact
//! command to run.

use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=ZAX_FCEUX_LIB");
    println!("cargo:rerun-if-env-changed=ZAX_FCEUX_DIR");

    // Cargo runs a build script whatever the features are, so this is where "off by default"
    // becomes "costs nothing": with the feature off we return before emitting any link directive.
    if std::env::var_os("CARGO_FEATURE_FCEUX_CORE").is_none() {
        return;
    }

    let msvc = std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let library = if msvc {
        "zaxfceux.lib"
    } else {
        "libzaxfceux.a"
    };

    let Some(directory) = find(library) else {
        let command = if msvc {
            "zax\\build-msvc.cmd"
        } else {
            "sh zax/build-linux.sh"
        };
        // `eprintln!` and exit rather than `panic!`: the workspace denies `clippy::panic`, and a
        // build script's failure is better read as a message than as a backtrace anyway.
        eprintln!(
            "zax-fceux: `{library}` was not found.\n\
             The `fceux-core` feature links the FCEUX fork, which has to be built first:\n\
             \n    cd <cleanroom>/fceux-zax && {command}\n\n\
             Then either put the fork beside this worktree as `fceux-zax/`, or set\n\
             ZAX_FCEUX_DIR to its root, or ZAX_FCEUX_LIB to the directory holding {library}.\n\
             Nothing is downloaded and nothing is built from here; see zax-fceux/build.rs."
        );
        std::process::exit(1);
    };

    println!("cargo:rustc-link-search=native={}", directory.display());
    println!("cargo:rustc-link-lib=static=zaxfceux");

    // zlib: the fork builds its own target from the copy FCEUX vendors when CMake finds no system
    // zlib (Windows), and links the system one otherwise (Linux).
    if directory
        .join(if msvc {
            "zaxfceux_zlib.lib"
        } else {
            "libzaxfceux_zlib.a"
        })
        .exists()
    {
        println!("cargo:rustc-link-lib=static=zaxfceux_zlib");
    } else {
        println!("cargo:rustc-link-lib=dylib=z");
    }

    // The C++ runtime. rustc links the MSVC CRT itself, but not libstdc++.
    if !msvc {
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }

    // Rebuild when the library is replaced.
    println!(
        "cargo:rerun-if-changed={}",
        directory.join(library).display()
    );
}

/// The first directory, among the three places above, that holds `library`.
fn find(library: &str) -> Option<PathBuf> {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Some(path) = std::env::var_os("ZAX_FCEUX_LIB") {
        candidates.push(PathBuf::from(path));
    }

    // Every ancestor of this crate, each with a `fceux-zax` sibling tried beside it.
    let nearby = Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .map(|ancestor| ancestor.join("fceux-zax"))
        .filter(|candidate| candidate.is_dir());

    let roots = std::env::var_os("ZAX_FCEUX_DIR")
        .map(PathBuf::from)
        .into_iter()
        .chain(nearby);
    for root in roots {
        for build in ["build-msvc", "build-linux", "build", "build-release"] {
            candidates.push(root.join(build));
        }
    }

    candidates
        .into_iter()
        .find(|directory| directory.join(library).is_file())
}
