//! The save state file (SPEC-09 REQ-PERS-050 to 058): a signature, a version and five sections,
//! each prefixed by its length. The core, the scripts' persistent data, the camera and the
//! learned conversion state are opaque blocks here; the header is JSON.

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
/// Notification text of a state whose game is gone (REQ-LOOP-044).
pub const GAME_MISSING_MESSAGE: &str = "The game profile doesn't exist anymore.";
/// Largest header the reader accepts.
pub const MAX_HEADER_BYTES: usize = 4096;
/// Size of the camera section: the mode, two setpoints of seven floats and the transition.
pub const CAMERA_SECTION_BYTES: usize = 4 + (2 * 7 * 4) + 4;

const TIMESTAMP: &[FormatItem<'static>] = format_description!("[year]-[month]-[day] [hour]:[minute]");

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
            + content.conversion.len(),
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
    file
}

fn header_json(header: &SaveStateHeader) -> Vec<u8> {
    serde_json::json!({
        "coreId": header.core_id,
        "coreStateVersion": header.core_state_version,
        "gameId": header.game_id,
        "romSha1": header.rom_sha1,
        "savedAt": header.saved_at,
        "slotId": header.slot_id,
    })
    .to_string()
    .into_bytes()
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
    // Trailing bytes mean the file does not match the format it claims.
    if !cursor.is_empty() {
        return None;
    }
    if core.is_empty() || lua.len() != persistent::BLOCK_SIZE || camera.len() != CAMERA_SECTION_BYTES {
        return None;
    }
    let header: SaveStateHeader = serde_json::from_slice(header_bytes).ok()?;
    Some(SaveStateContent {
        header,
        core: core.to_vec(),
        lua: lua.to_vec(),
        camera: camera.to_vec(),
        conversion: conversion.to_vec(),
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
        mode: if mode == 1 { CameraMode::Fpv } else { CameraMode::Tpv },
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
