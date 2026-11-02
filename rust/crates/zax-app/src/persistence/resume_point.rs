//! The resume point of AMD-053 REQ-LOOP-076: the savestate a normal exit writes, and the entry
//! the next launch offers.
//!
//! This is an accepted divergence **(D)**: the original program writes no savestate at all when it
//! closes. REQ-LOOP-052 keeps its promise all the same — no numbered slot and no quick slot is
//! ever written behind the user's back; the resume point is a store that only this path writes and
//! only the `resume.last` command reads, so REQ-PERS-052 ("it appears in no Load or Save list")
//! holds by construction.
//!
//! **Why a store of its own and not an entry of `saves/index.json`.** AMD-053 §3 point 12 adds a
//! thirty-eighth entry to that index, which is what the .NET application did. The Rust application
//! has no such index: a game's states are the files of its ROM (`<rom>.ss<n>`), read back from the
//! files, and `index.json` survives only as the shared store of before, which is deleted once it
//! is empty (`save_slots`). The resume point is therefore two files — `resume.state` and
//! `resume.png` — whose game and instant are read from the savestate header the loader already
//! carries. Every rule of REQ-LOOP-076 applies to them word for word. Q-102.
// Q-102

use std::fs;

use crate::emulation::host::{THUMBNAIL_HEIGHT, THUMBNAIL_WIDTH};
use crate::persistence::save_state_file::{self, SaveStateContent};
use crate::user_data_paths::{remove_file_if_present, write_atomic, UserDataPaths};

/// The identifier the resume point carries in a savestate header and in the log.
pub const SLOT_ID: &str = "resume";

/// What the resume point holds, read from the header of its file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumePoint {
    /// The game the state belongs to.
    pub game_id: String,
    /// Unix time of the write.
    pub saved_at: i64,
    /// Whether the thumbnail file is there.
    pub has_thumbnail: bool,
}

/// Reads the resume point's savestate whole; none when there is none or it cannot be read.
#[must_use]
pub fn read(paths: &UserDataPaths) -> Option<SaveStateContent> {
    let bytes = fs::read(paths.resume_state_file()).ok()?;
    save_state_file::try_read(&bytes)
}

/// What the resume point holds, or none when there is no readable one. The file is validated
/// whole, so a savestate truncated in its body is caught here rather than when the user activates
/// the entry (REQ-LOOP-076, erasure rule 3).
#[must_use]
pub fn peek(paths: &UserDataPaths) -> Option<ResumePoint> {
    let content = read(paths)?;
    (!content.header.game_id.is_empty()).then(|| ResumePoint {
        game_id: content.header.game_id,
        saved_at: content.header.saved_at,
        has_thumbnail: paths.resume_thumbnail_file().is_file(),
    })
}

/// Whether a file is there at all, readable or not: what tells "nothing to discard" from "an
/// unreadable point to discard".
#[must_use]
pub fn file_exists(paths: &UserDataPaths) -> bool {
    paths.resume_state_file().is_file()
}

/// Writes the resume point, with its thumbnail when there is one (256 x 192 RGB, REQ-PERS-051).
///
/// # Errors
///
/// The state or the thumbnail could not be written.
pub fn write(
    paths: &UserDataPaths,
    content: &SaveStateContent,
    picture: Option<&[u8]>,
) -> std::io::Result<()> {
    write_atomic(&paths.resume_state_file(), &save_state_file::write(content))?;
    let thumbnail = paths.resume_thumbnail_file();
    let Some(picture) = picture else {
        // A previous thumbnail must not stay behind and be shown for a different state.
        remove_file_if_present(&thumbnail);
        return Ok(());
    };
    let png = zax_render::assets::png_image::encode_rgb(
        u32::try_from(THUMBNAIL_WIDTH).unwrap_or(0),
        u32::try_from(THUMBNAIL_HEIGHT).unwrap_or(0),
        picture,
    )
    .map_err(std::io::Error::other)?;
    // AMD-051 REQ-PERS-088: as for a slot, the thumbnail is replaced atomically with the state.
    write_atomic(&thumbnail, &png)
}

/// Deletes the resume point's files. One that cannot be removed is left: the point is gone either
/// way, since the entry is only offered for a file that reads.
pub fn clear(paths: &UserDataPaths) {
    remove_file_if_present(&paths.resume_state_file());
    remove_file_if_present(&paths.resume_thumbnail_file());
}
