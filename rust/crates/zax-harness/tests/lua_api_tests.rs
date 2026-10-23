//! `lua-api` reproduces `goldens/lua-api.txt` byte for byte (SPEC-05 §4, REQ-LUA-080).

// A test: unreadable goldens or a failed write should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;

use zax_goldens::goldens_dir;
use zax_harness::lua_api::{self, Options};

#[test]
fn the_api_dump_is_identical_to_the_golden() {
    let scratch = std::env::temp_dir().join(format!("zax-lua-api-{}", std::process::id()));
    fs::create_dir_all(&scratch).unwrap();
    let output = scratch.join("lua-api.txt");

    let message = lua_api::run(&Options {
        output: Some(output.clone()),
    })
    .unwrap();
    let dump = fs::read(&output).unwrap();
    let golden = fs::read(goldens_dir().join("lua-api.txt")).unwrap();
    fs::remove_dir_all(&scratch).unwrap();

    assert_eq!(
        message
            .as_deref()
            .map(|text| text.starts_with("wrote 495 lines")),
        Some(true)
    );
    assert_eq!(
        String::from_utf8(dump).unwrap(),
        String::from_utf8(golden).unwrap()
    );
}

#[test]
fn an_unknown_argument_is_a_usage_error() {
    let error = Options::parse(&["--nope".to_owned()]).unwrap_err();
    assert!(error.to_string().contains("unknown argument: --nope"));
}
