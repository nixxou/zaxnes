//! The program binary cache on a real driver: a renderer started over a folder of binaries draws
//! what a renderer started from source draws, whatever the folder holds. No C# counterpart (the
//! C# renderer compiled its programs at every start).

#![allow(clippy::expect_used, clippy::panic)] // test helpers fail loudly on a broken fixture

mod common;

use std::path::{Path, PathBuf};

use common::{solid, Capture, GlFixture, SceneFixture};
use zax_profiles::enums::ProfileLayer;

fn folder(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("zax-render-cache-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn files(directory: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(directory)
        .map(|listing| listing.filter_map(Result::ok).map(|e| e.path()).collect())
        .unwrap_or_default()
}

fn draw(gl: &GlFixture, directory: Option<&Path>) -> Capture {
    let mut renderer = directory.map_or_else(|| gl.renderer(), |path| gl.renderer_cached(path));
    let built = SceneFixture::new()
        .background(0x0F)
        .palette(1, 0x30)
        .pattern(solid(1, 0), 1, ProfileLayer::L2)
        .rect(solid(1, 0), 8, 10, 16, 10)
        .build();
    gl.render_built(&mut renderer, &built)
}

#[test]
fn a_renderer_over_cached_binaries_draws_what_a_renderer_from_source_draws() {
    let gl = GlFixture::create(160, 120);
    let directory = folder("warm");
    let source = draw(&gl, None);
    let cold = draw(&gl, Some(&directory));
    let warm = draw(&gl, Some(&directory));
    assert_eq!(source.pixels, cold.pixels);
    assert_eq!(source.pixels, warm.pixels);
    // A driver that gives program binaries leaves one file per program; one that does not (some
    // Mesa drivers) leaves none, and everything above still holds.
    eprintln!(
        "program binaries cached by this driver: {}",
        files(&directory).len()
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn a_damaged_or_foreign_binary_is_compiled_from_source_instead() {
    let gl = GlFixture::create(160, 120);
    let directory = folder("damaged");
    let source = draw(&gl, None);
    let _ = draw(&gl, Some(&directory));
    for file in files(&directory) {
        let mut bytes = std::fs::read(&file).expect("a cached binary");
        // Half a file: the checksum no longer matches.
        bytes.truncate(bytes.len() / 2);
        std::fs::write(&file, &bytes).expect("rewrite");
    }
    assert_eq!(source.pixels, draw(&gl, Some(&directory)).pixels);
    for file in files(&directory) {
        // Not one of ours at all.
        std::fs::write(&file, b"garbage").expect("rewrite");
    }
    assert_eq!(source.pixels, draw(&gl, Some(&directory)).pixels);
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn an_unwritable_cache_folder_does_not_stop_the_renderer() {
    let gl = GlFixture::create(160, 120);
    let blocked = folder("blocked");
    std::fs::write(&blocked, b"a file where the folder should be").expect("a file");
    let source = draw(&gl, None);
    assert_eq!(source.pixels, draw(&gl, Some(&blocked)).pixels);
    let _ = std::fs::remove_file(&blocked);
}
