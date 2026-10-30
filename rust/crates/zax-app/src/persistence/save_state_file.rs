//! The save state file (SPEC-09 REQ-PERS-050 to 058): a signature, a version and five sections,
//! each prefixed by its length. The core, the scripts' persistent data, the camera and the
//! learned conversion state are opaque blocks here; the header is JSON.
//!
//! Rust-only additions (owner request): a sixth, optional section holds a JPEG thumbnail of the
//! 2D picture, and the header says whether the built-in hack ran, which version of it, the
//! application version and the local date and time. Files without them still read. A seventh,
//! optional section holds the state of the RetroAchievements under way (hit counts, measured
//! values); the thumbnail section is then written even when empty.

use serde::Deserialize;
use sha1::{Digest, Sha1};
use time::format_description::FormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};
use zax_engine3d::state_codec;
use zax_render::camera::{CameraMode, CameraModeState, CameraStateData};
use zax_render::numerics::Vector3;
use zax_scripting::persistent;

/// The signature that opens the file.
pub const SIGNATURE: &[u8; 8] = b"ZAXNESAV";
/// The signature of the previous product, still accepted.
pub const LEGACY_SIGNATURE: &[u8; 8] = b"SEN3DSAV";
/// Version of the container.
pub const FORMAT_VERSION: i32 = 1;
/// Notification text of an unreadable state (REQ-LOOP-046).
pub const CORRUPTED_MESSAGE: &str = "Corrupted save state.";

/// The message shown when a state was made with the built-in ROM patches set the other way.
pub const OTHER_PATCH_MESSAGE: &str =
    "This save state was made with the built-in ROM patches set the other way.";
/// The message shown when a state was made with another version of the built-in hack.
pub const OTHER_HACK_VERSION_MESSAGE: &str =
    "This save state was made with another version of the built-in hack: it cannot be loaded.";
/// The question asked before a state made with the built-in hack loads while it is off.
pub const TURN_HACK_ON_QUESTION: &str = "To play this save state, the built-in hack will be turned on: the game reloads, then the save state loads. Continue?";
/// The question asked before a state made without the built-in hack loads while it is on.
pub const OTHER_COMPONENTS_QUESTION: &str = "This save state was made with other parts of the built-in hack (the widescreen, the aids) than those set now. Load it with the parts set now, or set them as the save state had them (the game reloads)?";
/// The button of [`OTHER_COMPONENTS_QUESTION`] that loads the state with the parts set now.
pub const LOAD_AS_SET_LABEL: &str = "Parts set now";
/// The button of [`OTHER_COMPONENTS_QUESTION`] that sets the parts as the state had them.
pub const LOAD_ALIGNED_LABEL: &str = "Parts of the save";
/// The question asked to turn the built-in hack off for a state made without it.
pub const TURN_HACK_OFF_QUESTION: &str = "To play this save state, the built-in hack will be turned off: the game reloads, then the save state loads. Continue?";
/// Notification text of a state whose game is gone (REQ-LOOP-044).
pub const GAME_MISSING_MESSAGE: &str = "The game profile doesn't exist anymore.";
/// Largest header the reader accepts.
pub const MAX_HEADER_BYTES: usize = 4096;
/// Size of the camera section: the mode, two setpoints of seven floats and the transition.
pub const CAMERA_SECTION_BYTES: usize = 4 + (2 * 7 * 4) + 4;

const TIMESTAMP: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day] [hour]:[minute]");

/// The header of a state file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveStateHeader {
    /// The core that wrote the state.
    pub core_id: String,
    /// The version of the core's state format.
    pub core_state_version: i32,
    /// The game the state belongs to.
    pub game_id: String,
    /// The fingerprint of the ROM the state was taken with.
    pub rom_sha1: String,
    /// Unix time of the save.
    pub saved_at: i64,
    /// The slot it was saved to.
    pub slot_id: String,
    /// Whether the game ran with a built-in patch (`None` in files older than the field).
    #[serde(default)]
    pub is_hack: Option<bool>,
    /// The built-in patch's name, when one ran.
    #[serde(default)]
    pub hack_name: Option<String>,
    /// The built-in patch's version (`BuiltinPatch::version`, 8 hex digits), when one ran.
    #[serde(default)]
    pub hack_version: Option<String>,
    /// The components of the built-in hack pack applied (`widescreen`, the keys of the aids on;
    /// `None` in files older than the field).
    #[serde(default)]
    pub hack_components: Option<Vec<String>>,
    /// Whether the state was taken at the game's wait for the next frame, no code of the hack
    /// running (`None` in files older than the field).
    #[serde(default)]
    pub at_idle: Option<bool>,
    /// The version of the application that wrote the file.
    #[serde(default)]
    pub app_version: Option<String>,
    /// The local date and time of the save, as the player read it.
    #[serde(default)]
    pub saved_at_local: Option<String>,
}

/// Everything a state file holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SaveStateContent {
    /// The header.
    pub header: SaveStateHeader,
    /// The core's state block.
    pub core: Vec<u8>,
    /// The scripts' persistent data.
    pub lua: Vec<u8>,
    /// The camera section.
    pub camera: Vec<u8>,
    /// The learned conversion state.
    pub conversion: Vec<u8>,
    /// A JPEG thumbnail of the 2D picture, or empty.
    pub thumbnail: Vec<u8>,
    /// The state of the RetroAchievements under way, or empty.
    pub achievements: Vec<u8>,
}

/// Serialises `content` into a state file.
#[must_use]
pub fn write(content: &SaveStateContent) -> Vec<u8> {
    let header = header_json(&content.header);
    let mut file = Vec::with_capacity(
        SIGNATURE.len()
            + 8
            + header.len()
            + 16
            + content.core.len()
            + content.lua.len()
            + content.camera.len()
            + content.conversion.len()
            + 4
            + content.thumbnail.len()
            + 4
            + content.achievements.len(),
    );
    file.extend_from_slice(SIGNATURE);
    file.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    for section in [
        header.as_slice(),
        &content.core,
        &content.lua,
        &content.camera,
        &content.conversion,
    ] {
        file.extend_from_slice(&section_length(section).to_le_bytes());
        file.extend_from_slice(section);
    }
    if !content.thumbnail.is_empty() || !content.achievements.is_empty() {
        file.extend_from_slice(&section_length(&content.thumbnail).to_le_bytes());
        file.extend_from_slice(&content.thumbnail);
    }
    if !content.achievements.is_empty() {
        file.extend_from_slice(&section_length(&content.achievements).to_le_bytes());
        file.extend_from_slice(&content.achievements);
    }
    file
}

fn header_json(header: &SaveStateHeader) -> Vec<u8> {
    let mut json = serde_json::json!({
        "coreId": header.core_id,
        "coreStateVersion": header.core_state_version,
        "gameId": header.game_id,
        "romSha1": header.rom_sha1,
        "savedAt": header.saved_at,
        "slotId": header.slot_id,
    });
    let optional = [
        ("isHack", header.is_hack.map(serde_json::Value::Bool)),
        (
            "hackName",
            header.hack_name.clone().map(serde_json::Value::String),
        ),
        (
            "hackVersion",
            header.hack_version.clone().map(serde_json::Value::String),
        ),
        (
            "hackComponents",
            header.hack_components.as_ref().map(|components| {
                serde_json::Value::Array(
                    components
                        .iter()
                        .cloned()
                        .map(serde_json::Value::String)
                        .collect(),
                )
            }),
        ),
        ("atIdle", header.at_idle.map(serde_json::Value::Bool)),
        (
            "appVersion",
            header.app_version.clone().map(serde_json::Value::String),
        ),
        (
            "savedAtLocal",
            header.saved_at_local.clone().map(serde_json::Value::String),
        ),
    ];
    if let Some(object) = json.as_object_mut() {
        for (key, value) in optional {
            if let Some(value) = value {
                object.insert(key.to_owned(), value);
            }
        }
    }
    json.to_string().into_bytes()
}

#[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)] // sections are far below 2 GiB, as the C# int
const fn section_length(section: &[u8]) -> i32 {
    section.len() as i32
}

/// Parses a state file, or `None` when it does not match the format (REQ-PERS-052).
#[must_use]
pub fn try_read(file: &[u8]) -> Option<SaveStateContent> {
    let head = file.get(..SIGNATURE.len())?;
    if !has_known_signature(head) {
        return None;
    }
    let mut cursor = &file[SIGNATURE.len()..];
    if read_i32(&mut cursor)? != FORMAT_VERSION {
        return None;
    }
    let header_bytes = read_section(&mut cursor)?;
    if header_bytes.len() > MAX_HEADER_BYTES {
        return None;
    }
    let core = read_section(&mut cursor)?;
    let lua = read_section(&mut cursor)?;
    let camera = read_section(&mut cursor)?;
    let conversion = read_section(&mut cursor)?;
    let thumbnail = if cursor.is_empty() {
        &[][..]
    } else {
        read_section(&mut cursor)?
    };
    let achievements = if cursor.is_empty() {
        &[][..]
    } else {
        read_section(&mut cursor)?
    };
    // Trailing bytes mean the file does not match the format it claims.
    if !cursor.is_empty() {
        return None;
    }
    if core.is_empty()
        || lua.len() != persistent::BLOCK_SIZE
        || camera.len() != CAMERA_SECTION_BYTES
    {
        return None;
    }
    let header: SaveStateHeader = serde_json::from_slice(header_bytes).ok()?;
    Some(SaveStateContent {
        header,
        core: core.to_vec(),
        lua: lua.to_vec(),
        camera: camera.to_vec(),
        conversion: conversion.to_vec(),
        thumbnail: thumbnail.to_vec(),
        achievements: achievements.to_vec(),
    })
}

/// The largest file this build writes for a core whose state is at most `core_max_state_size`
/// bytes (REQ-ARCH-013).
#[must_use]
pub fn max_file_size(core_max_state_size: usize) -> usize {
    SIGNATURE.len()
        + 4
        + 4
        + MAX_HEADER_BYTES
        + 4
        + core_max_state_size
        + 4
        + persistent::BLOCK_SIZE
        + 4
        + CAMERA_SECTION_BYTES
        // The learned conversion state has a reserved capacity precisely so that this bound
        // exists (REQ-ARCH-013, last sentence).
        + 4
        + state_codec::RESERVED_CAPACITY
        + 4
        + MAX_THUMBNAIL_BYTES
        + 4
        + MAX_ACHIEVEMENTS_BYTES
}

/// Largest thumbnail section: a 320 x 240 JPEG stays far below it.
pub const MAX_THUMBNAIL_BYTES: usize = 256 * 1024;

/// Largest RetroAchievements section written: a NES set's state is a few kilobytes.
pub const MAX_ACHIEVEMENTS_BYTES: usize = 64 * 1024;

/// The JPEG thumbnail of a 2D picture of NES colour indices, 240 rows of `picture.len() / 240`
/// pixels: wider than 4:3, the centre 320 columns only. `None` for a picture of no such size.
#[must_use]
pub fn thumbnail_jpeg(picture: &[u8]) -> Option<Vec<u8>> {
    const HEIGHT: usize = 240;
    const WIDTH_4_3: usize = 320;
    if picture.is_empty() || picture.len() % HEIGHT != 0 {
        return None;
    }
    let width = picture.len() / HEIGHT;
    let kept = width.min(WIDTH_4_3);
    let left = (width - kept) / 2;
    let mut rgb = Vec::with_capacity(kept * HEIGHT * 3);
    for row in picture.chunks_exact(width) {
        for &index in &row[left..left + kept] {
            let color = zax_engine3d::palette::nes_color(usize::from(index));
            rgb.extend_from_slice(&[color.r, color.g, color.b]);
        }
    }
    let mut jpeg = Vec::new();
    jpeg_encoder::Encoder::new(&mut jpeg, 85)
        .encode(
            &rgb,
            u16::try_from(kept).ok()?,
            u16::try_from(HEIGHT).ok()?,
            jpeg_encoder::ColorType::Rgb,
        )
        .ok()?;
    Some(jpeg)
}

/// The local date and time of `unix_seconds` with seconds, for the header.
#[must_use]
pub fn local_date_time(unix_seconds: i64) -> String {
    const WITH_SECONDS: &[FormatItem<'static>] =
        format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    OffsetDateTime::from_unix_timestamp(unix_seconds)
        .map(|time| time.to_offset(offset))
        .ok()
        .and_then(|time| time.format(WITH_SECONDS).ok())
        .unwrap_or_default()
}

/// Whether `head` is one of the two signatures.
#[must_use]
pub fn has_known_signature(head: &[u8]) -> bool {
    head == SIGNATURE || head == LEGACY_SIGNATURE
}

/// Serialises the camera section.
#[must_use]
pub fn write_camera(state: &CameraStateData) -> Vec<u8> {
    let mut block = Vec::with_capacity(CAMERA_SECTION_BYTES);
    block.extend_from_slice(&(state.mode as i32).to_le_bytes());
    write_mode(&mut block, &state.tpv);
    write_mode(&mut block, &state.fpv);
    block.extend_from_slice(&state.transition_remaining.to_le_bytes());
    block
}

/// Reads the camera section, or `None` when it has the wrong size.
#[must_use]
pub fn read_camera(block: &[u8]) -> Option<CameraStateData> {
    if block.len() != CAMERA_SECTION_BYTES {
        return None;
    }
    let mut cursor = block;
    let mode = read_i32(&mut cursor)?;
    let tpv = read_mode(&mut cursor)?;
    let fpv = read_mode(&mut cursor)?;
    let transition_remaining = read_f32(&mut cursor)?;
    Some(CameraStateData {
        mode: if mode == 1 {
            CameraMode::Fpv
        } else {
            CameraMode::Tpv
        },
        tpv,
        fpv,
        transition_remaining,
    })
}

const HEX: &[u8; 16] = b"0123456789abcdef";

/// The fingerprint of a ROM: the SHA-1 of its PRG then CHR data, in lower-case hexadecimal.
#[must_use]
pub fn rom_fingerprint(prg_rom: &[u8], chr_rom: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(prg_rom);
    hasher.update(chr_rom);
    let mut text = String::with_capacity(40);
    for byte in hasher.finalize() {
        text.push(char::from(HEX[usize::from(byte >> 4)]));
        text.push(char::from(HEX[usize::from(byte & 15)]));
    }
    text
}

/// The text shown for the instant `unix_seconds`, in local time, or `"empty"` for an empty slot.
#[must_use]
pub fn format_timestamp(unix_seconds: i64) -> String {
    if unix_seconds <= 0 {
        return "empty".to_owned();
    }
    let Ok(instant) = OffsetDateTime::from_unix_timestamp(unix_seconds) else {
        return "empty".to_owned();
    };
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    instant
        .to_offset(offset)
        .format(TIMESTAMP)
        .unwrap_or_else(|_| "empty".to_owned())
}

fn write_mode(block: &mut Vec<u8>, mode: &CameraModeState) {
    block.extend_from_slice(&mode.distance.to_le_bytes());
    write_vector(block, mode.angles);
    write_vector(block, mode.target);
}

fn write_vector(block: &mut Vec<u8>, value: Vector3) {
    for component in [value.x, value.y, value.z] {
        block.extend_from_slice(&component.to_le_bytes());
    }
}

fn read_mode(cursor: &mut &[u8]) -> Option<CameraModeState> {
    let distance = read_f32(cursor)?;
    let angles = read_vector(cursor)?;
    let target = read_vector(cursor)?;
    Some(CameraModeState {
        distance,
        angles,
        target,
    })
}

fn read_vector(cursor: &mut &[u8]) -> Option<Vector3> {
    Some(Vector3 {
        x: read_f32(cursor)?,
        y: read_f32(cursor)?,
        z: read_f32(cursor)?,
    })
}

fn read_i32(cursor: &mut &[u8]) -> Option<i32> {
    let (bytes, rest) = cursor.split_first_chunk::<4>()?;
    *cursor = rest;
    Some(i32::from_le_bytes(*bytes))
}

fn read_f32(cursor: &mut &[u8]) -> Option<f32> {
    let (bytes, rest) = cursor.split_first_chunk::<4>()?;
    *cursor = rest;
    Some(f32::from_le_bytes(*bytes))
}

fn read_section<'a>(cursor: &mut &'a [u8]) -> Option<&'a [u8]> {
    let length = usize::try_from(read_i32(cursor)?).ok()?;
    if length > cursor.len() {
        return None;
    }
    let (section, rest) = cursor.split_at(length);
    *cursor = rest;
    Some(section)
}
