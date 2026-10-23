//! `golden-core` reproduces rows of `core-goldens.jsonl`, compared as parsed JSON (key order
//! and spacing do not matter). A handful of ROMs, covering a rejected file, a blargg ROM
//! and several mappers, is copied into a scratch tree so that the directory walk is
//! exercised as well; the full set is checked by running the command on `rust/test-roms`.

// A test: unreadable goldens or a failed copy should fail it outright.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use zax_goldens::goldens_dir;
use zax_harness::golden_core;

fn golden_rows() -> Vec<Value> {
    let text = fs::read_to_string(goldens_dir().join("core-goldens.jsonl")).unwrap();
    text.lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn field<'a>(row: &'a Value, key: &str) -> &'a Value {
    row.get(key).unwrap_or(&Value::Null)
}

/// Picks the rows to replay: the first rejected file, the first blargg ROM and the first
/// ROM of each of a few mappers.
fn sample(rows: &[Value], rom_dir: &Path) -> Vec<Value> {
    let mut picked: Vec<Value> = Vec::new();
    let mut take = |matches: &dyn Fn(&Value) -> bool| {
        let found = rows.iter().find(|row| {
            matches(row)
                && rom_dir.join(field(row, "rom").as_str().unwrap()).is_file()
                && !picked.contains(row)
        });
        if let Some(row) = found {
            picked.push(row.clone());
        }
    };

    take(&|row| field(row, "load_ok") == &Value::Bool(false));
    take(&|row| row.get("blargg_status").is_some());
    for mapper in [0, 1, 4] {
        take(&|row| field(row, "mapper").as_i64() == Some(mapper));
    }

    picked
}

#[test]
fn golden_core_reproduces_the_golden_rows() {
    let rom_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../test-roms");
    if !rom_dir.is_dir() {
        eprintln!("skipped: {} not found", rom_dir.display());
        return;
    }

    let expected = sample(&golden_rows(), &rom_dir);
    assert!(
        !expected.is_empty(),
        "no golden ROM is present in test-roms"
    );

    let scratch: PathBuf = Path::new(env!("CARGO_TARGET_TMPDIR")).join("golden-core-roms");
    match fs::remove_dir_all(&scratch) {
        Ok(()) => {}
        // No leftover of an earlier run.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("cannot clear {}: {error}", scratch.display()),
    }
    for row in &expected {
        let rel = field(row, "rom").as_str().unwrap();
        let target = scratch.join(rel);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::copy(rom_dir.join(rel), target).unwrap();
    }

    let out = scratch.join("out.jsonl");
    let summary = golden_core::run(&scratch, &out, golden_core::DEFAULT_FRAMES).unwrap();
    assert_eq!(expected.len(), summary.files);

    let actual: Vec<Value> = fs::read_to_string(&out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();

    // The command writes the rows in ordinal path order.
    let mut expected = expected;
    expected.sort_by(|a, b| field(a, "rom").as_str().cmp(&field(b, "rom").as_str()));
    assert_eq!(expected.len(), actual.len());
    for (golden, row) in expected.iter().zip(&actual) {
        assert_eq!(golden, row, "{}", field(golden, "rom"));
    }
}
