//! Builds rcheevos (vendored in `rcheevos/`, MIT) and the thin C layer over its client.
//!
//! Only what a NES needs is compiled: the client, the runtime, the server API and the ROM hash.
//! The disc, encrypted and zip hashes are left out.

use std::path::Path;

const SOURCES: [&str; 25] = [
    "rcheevos/src/rc_client.c",
    "rcheevos/src/rc_compat.c",
    "rcheevos/src/rc_util.c",
    "rcheevos/src/rc_version.c",
    "rcheevos/src/rapi/rc_api_common.c",
    "rcheevos/src/rapi/rc_api_editor.c",
    "rcheevos/src/rapi/rc_api_info.c",
    "rcheevos/src/rapi/rc_api_runtime.c",
    "rcheevos/src/rapi/rc_api_user.c",
    "rcheevos/src/rcheevos/alloc.c",
    "rcheevos/src/rcheevos/condition.c",
    "rcheevos/src/rcheevos/condset.c",
    "rcheevos/src/rcheevos/consoleinfo.c",
    "rcheevos/src/rcheevos/format.c",
    "rcheevos/src/rcheevos/lboard.c",
    "rcheevos/src/rcheevos/memref.c",
    "rcheevos/src/rcheevos/operand.c",
    "rcheevos/src/rcheevos/rc_validate.c",
    "rcheevos/src/rcheevos/richpresence.c",
    "rcheevos/src/rcheevos/runtime.c",
    "rcheevos/src/rcheevos/runtime_progress.c",
    "rcheevos/src/rcheevos/trigger.c",
    "rcheevos/src/rcheevos/value.c",
    "rcheevos/src/rhash/hash.c",
    "rcheevos/src/rhash/hash_rom.c",
];

fn main() {
    let mut build = cc::Build::new();
    build
        .include("rcheevos/include")
        .include("rcheevos/src")
        .define("RC_CLIENT_SUPPORTS_HASH", None)
        .define("RC_HASH_NO_DISC", None)
        .define("RC_HASH_NO_ENCRYPTED", None)
        .define("RC_HASH_NO_ZIP", None)
        .define("RC_STATIC", None)
        .warnings(false)
        .file("rcheevos/src/rhash/md5.c")
        .file("csrc/zax_ra.c");
    for source in SOURCES {
        build.file(source);
    }
    if build.get_compiler().is_like_msvc() {
        build.define("_CRT_SECURE_NO_WARNINGS", None);
    }
    build.compile("rcheevos");
    for directory in ["csrc", "rcheevos"] {
        println!("cargo:rerun-if-changed={}", Path::new(directory).display());
    }
}
