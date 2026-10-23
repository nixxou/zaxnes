#![forbid(unsafe_code)]
//! Typed readers for the C# reference goldens in `rust/goldens/` (test-only helper).

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Error reading or parsing a golden file.
#[derive(Debug)]
pub enum GoldenError {
    Io(std::io::Error),
    Json(serde_json::Error),
}

impl std::fmt::Display for GoldenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "golden I/O error: {e}"),
            Self::Json(e) => write!(f, "golden JSON error: {e}"),
        }
    }
}

impl std::error::Error for GoldenError {}

impl From<std::io::Error> for GoldenError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for GoldenError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

/// The `rust/goldens` directory of this repository.
#[must_use]
pub fn goldens_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../goldens")
}

/// One row of `core-goldens.jsonl` (one public test ROM). Maps are keyed by checkpoint frame.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CoreGolden {
    pub rom: String,
    pub rom_sha256: String,
    pub mapper: Option<u32>,
    pub load_ok: bool,
    #[serde(default)]
    pub load_failure: Option<String>,
    #[serde(default)]
    pub load_error: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub sample_rate: Option<u32>,
    #[serde(default)]
    pub frames: Option<u32>,
    #[serde(default)]
    pub pixel_sha256: BTreeMap<u32, String>,
    /// `"<count>:<sha256>"`.
    #[serde(default)]
    pub tiles_sha256: BTreeMap<u32, String>,
    #[serde(default)]
    pub cycles: BTreeMap<u32, u64>,
    /// `"<sha256>:<samples>"`, running hash since reset.
    #[serde(default)]
    pub audio_i16_sha256_samples: BTreeMap<u32, String>,
    #[serde(default)]
    pub blargg_status: Option<u32>,
    #[serde(default)]
    pub blargg_text: Option<String>,
    #[serde(default)]
    pub exception: Option<String>,
}

/// Parses `core-goldens.jsonl` content (blank lines ignored).
///
/// # Errors
/// Returns an error if a line is not a valid row.
pub fn parse_core_goldens(text: &str) -> Result<Vec<CoreGolden>, GoldenError> {
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).map_err(GoldenError::from))
        .collect()
}

/// Reads `core-goldens.jsonl` from `path`.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn read_core_goldens(path: &Path) -> Result<Vec<CoreGolden>, GoldenError> {
    parse_core_goldens(&std::fs::read_to_string(path)?)
}

/// One scene of `render-synthetic/render.json`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderScene {
    pub scene: String,
    pub description: String,
    pub width: u32,
    pub height: u32,
    pub opaque_tiles: u32,
    pub transparent_tiles: u32,
    pub hud_tiles: u32,
    pub shadow_casters: u32,
    pub draw_calls: u32,
    pub triangles: u32,
    pub resident_meshes: u32,
    pub anti_alias_samples: u32,
    pub shadow_map_size: u32,
    pub requested_shadow_map_size: u32,
    pub distinct_colors: u32,
    pub background_pixels: u32,
    pub mean_luma: f64,
}

/// Parses `render.json`.
///
/// # Errors
/// Returns an error if the JSON does not match.
pub fn parse_render(text: &str) -> Result<Vec<RenderScene>, GoldenError> {
    Ok(serde_json::from_str(text)?)
}

/// Reads `render.json` from `path`.
///
/// # Errors
/// Returns an error if the file cannot be read or parsed.
pub fn read_render(path: &Path) -> Result<Vec<RenderScene>, GoldenError> {
    parse_render(&std::fs::read_to_string(path)?)
}

/// One non-comment line of `lua-api.txt`: `kind name rest...`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaApiLine {
    /// `global`, `const` or `member`.
    pub kind: String,
    pub name: String,
    pub rest: Vec<String>,
}

/// The parsed `lua-api.txt`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LuaApi {
    /// Lines starting with `#`, verbatim.
    pub comments: Vec<String>,
    pub lines: Vec<LuaApiLine>,
}

/// Parses `lua-api.txt` content.
#[must_use]
pub fn parse_lua_api(text: &str) -> LuaApi {
    let mut comments = Vec::new();
    let mut lines = Vec::new();
    for l in text.lines().filter(|l| !l.trim().is_empty()) {
        if l.starts_with('#') {
            comments.push(l.to_string());
            continue;
        }
        let mut it = l.split_whitespace().map(str::to_string);
        let kind = it.next().unwrap_or_default();
        let name = it.next().unwrap_or_default();
        lines.push(LuaApiLine {
            kind,
            name,
            rest: it.collect(),
        });
    }
    LuaApi { comments, lines }
}

/// Reads `lua-api.txt` from `path`.
///
/// # Errors
/// Returns an error if the file cannot be read.
pub fn read_lua_api(path: &Path) -> Result<LuaApi, GoldenError> {
    Ok(parse_lua_api(&std::fs::read_to_string(path)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_core_goldens() {
        let rows = read_core_goldens(&goldens_dir().join("core-goldens.jsonl")).unwrap();
        assert_eq!(rows.len(), 291);
        assert!(rows
            .iter()
            .filter(|r| r.load_ok)
            .all(|r| r.pixel_sha256.contains_key(&1200)));
    }

    #[test]
    fn loads_render() {
        let scenes = read_render(&goldens_dir().join("render-synthetic/render.json")).unwrap();
        assert!(scenes.iter().any(|s| s.scene == "framing"));
        assert!(scenes.iter().all(|s| s.width == 640 && s.height == 480));
    }

    #[test]
    fn loads_lua_api() {
        let api = read_lua_api(&goldens_dir().join("lua-api.txt")).unwrap();
        assert_eq!(api.lines.len(), 494);
        assert_eq!(api.lines.iter().filter(|l| l.kind == "global").count(), 35);
    }
}
