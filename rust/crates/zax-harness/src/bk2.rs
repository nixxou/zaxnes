//! `.bk2` movies (`BizHawk`), read from the **public description of the format** on `TASVideos`
//! and from the file's own self-description — no `BizHawk` source of any kind was consulted, and
//! none may be.
//!
//! # What the format is
//!
//! A `.bk2` is a zip archive of text files:
//!
//! | member | what it is |
//! |---|---|
//! | `Header.txt` | `key value` lines: `MovieVersion`, `Author`, `emuVersion`, `Platform`, `GameName`, `SHA1`, `BoardName`, `Core`, `rerecordCount`, `StartsFromSavestate` |
//! | `Input Log.txt` | `[Input]`, a `LogKey:` line, one line per frame, `[/Input]` |
//! | `SyncSettings.json` | the core settings the run needs to sync |
//! | `Comments.txt`, `Subtitles.txt` | prose |
//! | `CoreState` | **optional**; when it is there the movie starts from a savestate |
//!
//! # The one thing worth knowing about the input log
//!
//! **It describes itself.** The `LogKey:` line names every button in the order the frame lines
//! write them, with `#` starting a new pipe-separated field. A real NES movie's key is
//!
//! ```text
//! LogKey:#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|P1 B|P1 A|#P2 Up|…|P2 A|
//! ```
//!
//! which makes the frame line `|..|........|........|`: two console buttons, then eight for each
//! pad. So this reader **does not hard-code a layout** — it reads the key and builds the mapping
//! from it, and refuses by name any button it does not know. That matters: the order is **not**
//! `.fm2`'s `RLDUTSBA`, and a reader that assumed either order would silently replay the wrong
//! buttons. A button is pressed when its character is anything but `.` or a space, which is the
//! rule the format description gives.
//!
//! # What is refused, and why each one has to be
//!
//! A movie that cannot be honoured exactly is worse than no movie at all, so every one of these
//! is a refusal with the thing named rather than a warning:
//!
//! * a **`CoreState`** member — the movie is anchored to a savestate of another emulator's core,
//!   which nothing here can load. The format description is explicit that the *existence* of the
//!   member is what decides this, whatever the header says, so both are checked;
//! * `Platform` other than `NES`;
//! * a **port device** that is not a standard pad or an empty socket, the **Famicom** layout, or
//!   anything on the **Famicom expansion port** — the frame lines would be replayed against the
//!   wrong device;
//! * a **`RegionOverride`** other than 0. The public description does not say what the non-zero
//!   values mean, and a guess that got the region wrong would desync the movie by construction;
//! * non-empty **`BoardProperties`**, which are per-board overrides this reader cannot apply;
//! * a `LogKey` naming a button, an analog axis or a controller this reader has no word for.
//!
//! # When a `Reset` takes effect, which the description does not say and a measurement does
//!
//! The `LogKey` names `Reset` as a console button and the frame lines carry one character per
//! frame for it, so a reset is a **per-frame boolean**. What the public description does not say
//! is *where inside the frame* `BizHawk` taps the line — and the answer turns out to be neither
//! "inside" nor the obvious one: **the frame whose bit is set is emulated normally and the reset
//! happens after it.**
//!
//! That is a measurement, on the owner's per-frame run of the *Kirby* `.bk2`
//! (`bizhawk-runs/kirby-jp-step1.txt`: the whole 2 KiB and the cumulative lag count at the start
//! of each of the first 400 frames). The movie's first Reset bit is on frame 3, and the two
//! readings are told apart by what frame 3 does:
//!
//! | | resetting **before** frame 3 | resetting **after** it |
//! |---|---|---|
//! | the work RAM changes during frame 3 | no — nothing moves until frame 6 | **yes, as `BizHawk`'s does** |
//! | frame 3 is a lag frame | yes | **no, as in `BizHawk`'s counter** |
//! | the lag column, frames 1-400 | parts at once | **matches frame for frame** |
//! | the 2 KiB | — | **matches to frame 246**, bar the stack page and the temporaries below `$0020` |
//!
//! So this reader places the command on the frame **after** the one the bit is on, and
//! `movie-sync`'s `--resets early` is what restores the other reading. Lot EVAL-SYNC-2 ran with
//! the other one and reported `--resets late` as the one that matched; `late` is now one frame
//! further on again, the base having moved.
//!
//! # What is *not* refused
//!
//! A stated `SHA1` that does not match the ROM. The format description says the header carries
//! "SHA1 (also CRC32 or MD5)" and does **not** say what the hash is taken over, so this reader
//! tries the whole file and the data after the iNES header and reports *unverified* when neither
//! matches, rather than claiming the movie was recorded on another ROM. The one real `.bk2` this
//! lot has matches neither, on either dump of its game.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use sha1::{Digest, Sha1};

use crate::movie_file::{FrameCommand, FrameInput, MovieFile, RamPattern};

/// The members this reader looks for, in the spelling `BizHawk` writes them. Matched without
/// regard to case, because the format description fixes the contents and not the spelling.
const HEADER: &str = "header.txt";
const INPUT_LOG: &str = "input log.txt";
const SYNC_SETTINGS: &str = "syncsettings.json";
const CORE_STATE: &str = "corestate";

/// The SHA-1 of `data`, which is the fingerprint a `.bk2` header states for its ROM.
#[must_use]
pub fn sha1(data: &[u8]) -> [u8; 20] {
    Sha1::digest(data).into()
}

/// Reads a `.bk2`.
///
/// # Errors
/// The archive is not a `.bk2` this instrument can replay; the message names the one thing.
pub fn read(bytes: &[u8]) -> Result<MovieFile, String> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes))
        .map_err(|error| format!("the file is not a readable zip archive: {error}"))?;
    let names: Vec<String> = archive.file_names().map(ToOwned::to_owned).collect();
    let member = |wanted: &str| {
        names
            .iter()
            .find(|name| name.to_ascii_lowercase() == wanted)
            .cloned()
    };

    if member(CORE_STATE).is_some() {
        return Err(
            "the movie is anchored to a savestate (it carries a `CoreState`), which is a state \
             of another emulator's core and cannot be loaded here"
                .to_owned(),
        );
    }

    let mut notes = Vec::new();
    let header = parse_header(&read_member(&mut archive, member(HEADER), "Header.txt")?);
    check_header(&header)?;
    let settings = read_member(&mut archive, member(SYNC_SETTINGS), "SyncSettings.json")?;
    check_sync_settings(&settings, &mut notes)?;
    let frames = parse_input_log(&read_member(
        &mut archive,
        member(INPUT_LOG),
        "Input Log.txt",
    )?)?;

    if let Some(core) = header.get("Core") {
        notes.push(format!(
            "the movie was recorded on BizHawk's `{core}` core; a movie only replays exactly on \
             the machine that recorded it"
        ));
    }
    notes.push(
        "the format carries no RAM-fill option the way an `.fm2`'s `RAMInitOption` does, and what \
         `NesHawk` powers the work RAM on with is not published, so it was measured: of the three \
         deterministic fills only `00 00 00 00 FF FF FF FF` keeps the two Faxanadu traces in step \
         with BizHawk's own (207 and 220 of 832 checksum lines exactly equal, against 0 of 832 \
         for all-$00 and for all-$FF), so that is what a `.bk2` is read as asking for. `--ram` \
         overrides it"
            .to_owned(),
    );
    notes.push(
        "the format states no region in the header and `RegionOverride` is 0, so the region is \
         taken from the ROM's own iNES header"
            .to_owned(),
    );
    notes.push(
        "the `Reset` column is read as *the console is reset AFTER the frame whose bit is set*, \
         which is measured rather than assumed: against BizHawk's per-frame run of the Kirby \
         .bk2 (`kirby-jp-step1.txt`, 400 frames of work RAM and of the lag counter) that reading \
         matches the lag count frame for frame and the 2 KiB to frame 246, while resetting \
         *before* the flagged frame leaves the RAM untouched until frame 6 and counts the flagged \
         frame as a lag frame where BizHawk does not. `--resets early` restores the other \
         reading, `--resets late` moves one frame further on again"
            .to_owned(),
    );

    Ok(MovieFile {
        kind: "bk2",
        pal: false,
        pal_stated: false,
        // Measured, not assumed, and the note above carries the measurement: `NesHawk`'s
        // power-on work RAM is the `00 00 00 00 FF FF FF FF` pattern, the same one FCEUX's
        // `RAMInitOption 0` writes. Lot EVAL-SYNC read it as all-$00 (the machine's own, which is
        // what `zax-nes` powers on with, REQ-CORE-004) because it had nothing to check it
        // against; the reference traces are that check.
        ram: RamPattern::FceuxDefault,
        rom_md5: None,
        rom_sha1: header.get("SHA1").map(String::as_str).and_then(hex20),
        frames,
        // The frame whose `Reset` bit is set runs **first**, and the console is reset after it.
        // Measured, and the note above carries the measurement; `MovieFile::reset_after_frame`
        // carries the reasoning for both formats.
        reset_after_frame: true,
        header,
        notes,
    })
}

/// Reads one member as text.
fn read_member(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    name: Option<String>,
    wanted: &str,
) -> Result<String, String> {
    let name = name.ok_or_else(|| format!("the archive has no `{wanted}`, so it is not a .bk2"))?;
    let mut file = archive
        .by_name(&name)
        .map_err(|error| format!("`{wanted}` cannot be read: {error}"))?;
    let mut text = String::new();
    file.read_to_string(&mut text)
        .map_err(|error| format!("`{wanted}` is not text: {error}"))?;
    Ok(text)
}

/// `key value` lines, the key being the first word.
fn parse_header(text: &str) -> BTreeMap<String, String> {
    let mut header = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim_end_matches('\r');
        if let Some((key, value)) = line.split_once(' ') {
            header
                .entry(key.to_owned())
                .or_insert_with(|| value.trim().to_owned());
        }
    }
    header
}

fn check_header(header: &BTreeMap<String, String>) -> Result<(), String> {
    match header.get("Platform").map(String::as_str) {
        Some("NES") => {}
        Some(other) => {
            return Err(format!(
                "the movie is of the `{other}` system, not of the NES"
            ))
        }
        None => return Err("the header states no `Platform`, so it is not a .bk2".to_owned()),
    }
    for (key, what) in [
        ("StartsFromSavestate", "a savestate"),
        ("StartsFromSaveRam", "a battery save"),
    ] {
        if header
            .get(key)
            .is_some_and(|value| !matches!(value.to_ascii_lowercase().as_str(), "" | "false" | "0"))
        {
            return Err(format!(
                "the movie starts from {what}, which cannot be used here"
            ));
        }
    }
    Ok(())
}

/// The sync settings, of which only the ones that decide what machine the movie needs are read.
///
/// `BizHawk` wraps the object in `{"o": …}`; both shapes are accepted because the public
/// description does not fix the wrapper.
fn check_sync_settings(text: &str, notes: &mut Vec<String>) -> Result<(), String> {
    let root: serde_json::Value = serde_json::from_str(text)
        .map_err(|error| format!("`SyncSettings.json` does not parse: {error}"))?;
    let settings = root.get("o").unwrap_or(&root);

    match settings
        .get("RegionOverride")
        .and_then(serde_json::Value::as_i64)
    {
        None | Some(0) => {}
        Some(other) => {
            return Err(format!(
                "the sync settings override the region with `RegionOverride {other}`, and the \
                 public description of the format does not say which region that is"
            ))
        }
    }
    // The one field that *does* speak about the power-on work RAM. `null` (or absent) is the
    // core's own pattern, which the caller's note records as measured; anything else names a
    // pattern whose shape the public description of the format does not fix, so it is refused by
    // name rather than guessed at — a wrong 2 KiB cannot be recovered from by any emulation.
    match settings.get("InitialWRamStatePattern") {
        None | Some(serde_json::Value::Null) => {}
        Some(other) => {
            return Err(format!(
                "the sync settings set `InitialWRamStatePattern` to {other}, and the public \
                 description of the format does not say how that names a 2 KiB fill"
            ))
        }
    }
    if let Some(properties) = settings
        .get("BoardProperties")
        .and_then(serde_json::Value::as_object)
    {
        if !properties.is_empty() {
            let named: Vec<&str> = properties.keys().map(String::as_str).collect();
            return Err(format!(
                "the sync settings set board properties this reader cannot apply: {}",
                named.join(", ")
            ));
        }
    }

    let Some(controls) = settings.get("Controls") else {
        notes.push("the sync settings name no controls; two standard pads are assumed".to_owned());
        return Ok(());
    };
    if controls
        .get("Famicom")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        return Err(
            "the movie was recorded on a Famicom, whose controllers are wired to the console and \
             whose expansion port this reader cannot honour"
                .to_owned(),
        );
    }
    for port in ["NesLeftPort", "NesRightPort"] {
        match controls.get(port).and_then(serde_json::Value::as_str) {
            None | Some("ControllerNES" | "UnpluggedNES") => {}
            Some(other) => {
                return Err(format!(
                    "`{port}` holds `{other}`, which is not a standard pad or an empty socket"
                ))
            }
        }
    }
    match controls
        .get("FamicomExpPort")
        .and_then(serde_json::Value::as_str)
    {
        None | Some("UnpluggedFam") => Ok(()),
        Some(other) => Err(format!(
            "the Famicom expansion port holds `{other}`, which this reader cannot honour"
        )),
    }
}

/// One group of the `LogKey`: the buttons written into one pipe-separated field, in order.
type LogKey = Vec<Vec<String>>;

/// Splits a `LogKey:` line into its fields. `#` starts a new field.
fn parse_log_key(line: &str) -> LogKey {
    let mut groups: LogKey = Vec::new();
    for entry in line.split('|') {
        let entry = entry.trim_end_matches('\r');
        if entry.is_empty() {
            continue;
        }
        if let Some(first) = entry.strip_prefix('#') {
            groups.push(vec![first.to_owned()]);
        } else if let Some(last) = groups.last_mut() {
            last.push(entry.to_owned());
        } else {
            groups.push(vec![entry.to_owned()]);
        }
    }
    groups
}

/// What one named button does: a bit of the input word, or a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Button {
    /// A bit of `NesCore::run_frame`'s word.
    Bit(u16),
    /// A command applied before the frame.
    Command(FrameCommand),
}

/// The button a `LogKey` name stands for, or `None` when this reader has no word for it.
fn button(name: &str) -> Option<Button> {
    if let Some(pad) = name.strip_prefix("P1 ") {
        return pad_bit(pad).map(Button::Bit);
    }
    if let Some(pad) = name.strip_prefix("P2 ") {
        return pad_bit(pad).map(|bit| Button::Bit(bit << 8));
    }
    match name {
        "Reset" => Some(Button::Command(FrameCommand::SoftReset)),
        "Power" => Some(Button::Command(FrameCommand::HardReset)),
        _ => None,
    }
}

/// A standard pad's bit, in `NesCore::run_frame`'s order: A, B, Select, Start, Up, Down, Left,
/// Right from bit 0.
fn pad_bit(name: &str) -> Option<u16> {
    Some(match name {
        "A" => 0x01,
        "B" => 0x02,
        "Select" => 0x04,
        "Start" => 0x08,
        "Up" => 0x10,
        "Down" => 0x20,
        "Left" => 0x40,
        "Right" => 0x80,
        _ => return None,
    })
}

/// `[Input]`, a `LogKey`, the frames, `[/Input]`.
fn parse_input_log(text: &str) -> Result<Vec<FrameInput>, String> {
    let mut key: Option<LogKey> = None;
    let mut frames = Vec::new();
    for (number, raw) in text.lines().enumerate() {
        let line = raw.trim_end_matches('\r');
        if let Some(rest) = line.strip_prefix("LogKey:") {
            let groups = parse_log_key(rest);
            let unknown: Vec<String> = groups
                .iter()
                .flatten()
                .filter(|name| button(name).is_none())
                .cloned()
                .collect();
            if !unknown.is_empty() {
                return Err(format!(
                    "the input log names buttons this reader has no word for: {}",
                    unknown.join(", ")
                ));
            }
            key = Some(groups);
            continue;
        }
        if !line.starts_with('|') {
            continue;
        }
        let key = key
            .as_ref()
            .ok_or_else(|| "the input log has frames before its `LogKey:` line".to_owned())?;
        frames.push(parse_frame(line, key).ok_or_else(|| {
            format!("line {}: the frame does not match the `LogKey`", number + 1)
        })?);
    }
    if key.is_none() {
        return Err("the input log has no `LogKey:` line".to_owned());
    }
    if frames.is_empty() {
        return Err("the input log has no frames".to_owned());
    }
    Ok(frames)
}

/// One frame line against the key: the fields in order, each character a button of its group.
fn parse_frame(line: &str, key: &LogKey) -> Option<FrameInput> {
    let mut fields = line.strip_prefix('|')?.split('|');
    let mut frame = FrameInput::default();
    for group in key {
        let field = fields.next()?;
        let characters: Vec<char> = field.chars().collect();
        if characters.len() != group.len() {
            return None;
        }
        for (name, character) in group.iter().zip(characters) {
            if character == '.' || character == ' ' {
                continue;
            }
            match button(name)? {
                Button::Bit(bit) => frame.word |= bit,
                // A power cycle wins over a reset: it is the stronger of the two.
                Button::Command(FrameCommand::HardReset) => {
                    frame.command = FrameCommand::HardReset;
                }
                Button::Command(command) => {
                    if frame.command == FrameCommand::None {
                        frame.command = command;
                    }
                }
            }
        }
    }
    // Everything after the last named field must be empty: a line with more fields than the key
    // describes is a line this reader is misreading.
    fields.all(str::is_empty).then_some(frame)
}

/// 40 hex characters into 20 bytes.
fn hex20(text: &str) -> Option<[u8; 20]> {
    let text = text.trim();
    if text.len() != 40 {
        return None;
    }
    let mut out = [0u8; 20];
    for (byte, pair) in out.iter_mut().zip(text.as_bytes().chunks_exact(2)) {
        let digit = |c: u8| match c {
            b'0'..=b'9' => Some(c - b'0'),
            b'a'..=b'f' => Some(c - b'a' + 10),
            b'A'..=b'F' => Some(c - b'A' + 10),
            _ => None,
        };
        *byte = (digit(pair[0])? << 4) | digit(pair[1])?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;
    use std::io::Write as _;

    /// The `LogKey` a real NES `.bk2` carries, read off `invariel-faxanadu.bk2`.
    const NES_KEY: &str = "#Power|Reset|#P1 Up|P1 Down|P1 Left|P1 Right|P1 Start|P1 Select|\
                           P1 B|P1 A|#P2 Up|P2 Down|P2 Left|P2 Right|P2 Start|P2 Select|P2 B|P2 A|";

    const SYNC: &str = r#"{"o":{"BoardProperties":{},"RegionOverride":0,"Controls":{
        "Famicom":false,"NesLeftPort":"ControllerNES","NesRightPort":"ControllerNES",
        "FamicomExpPort":"UnpluggedFam"}}}"#;

    /// Builds a `.bk2` the way the format describes one, which is what the tests replay.
    fn archive(members: &[(&str, &str)]) -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut writer = zip::ZipWriter::new(Cursor::new(&mut out));
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            for (name, body) in members {
                let _ = writer.start_file(*name, options);
                let _ = writer.write_all(body.as_bytes());
            }
            let _ = writer.finish();
        }
        out
    }

    /// A whole movie: the header, the settings and a log of `lines`.
    fn movie(header_extra: &str, sync: &str, lines: &[&str]) -> Vec<u8> {
        let mut log = format!("[Input]\r\nLogKey:{NES_KEY}\r\n");
        for line in lines {
            log.push_str(line);
            log.push_str("\r\n");
        }
        log.push_str("[/Input]\r\n");
        let header = format!(
            "MovieVersion BizHawk v2.0 Tasproj v1.0\r\nPlatform NES\r\nGameName Test\r\n\
             Core NesHawk\r\nrerecordCount 7\r\n{header_extra}"
        );
        archive(&[
            ("Header.txt", header.as_str()),
            ("SyncSettings.json", sync),
            ("Input Log.txt", log.as_str()),
            ("Comments.txt", ""),
        ])
    }

    #[test]
    fn a_frame_line_is_read_through_the_log_key_and_not_through_a_guess() {
        // `.bk2`'s pad order is Up Down Left Right Start Select B A -- NOT `.fm2`'s RLDUTSBA.
        // A reader that assumed either order would replay the wrong buttons, so this test names
        // the bits one at a time.
        let lines = [
            "|..|........|........|",
            "|..|.......A|........|",
            "|..|U.......|........|",
            "|..|......B.|........|",
            "|..|...R....|........|",
            "|..|........|.......A|",
            "|..|UDLRSsBA|........|",
        ];
        let bytes = movie("", SYNC, &lines);
        let file = read(&bytes).unwrap_or_default();
        let words: Vec<u16> = file.frames.iter().map(|frame| frame.word).collect();
        assert_eq!(words, [0x0000, 0x01, 0x10, 0x02, 0x80, 0x0100, 0xFF]);
        assert_eq!(file.kind, "bk2");
    }

    #[test]
    fn the_console_buttons_become_commands() {
        let lines = [
            "|..|........|........|",
            "|.r|........|........|",
            "|P.|........|........|",
            "|Pr|........|........|",
        ];
        let file = read(&movie("", SYNC, &lines)).unwrap_or_default();
        assert_eq!(
            file.commands(),
            [
                (1, FrameCommand::SoftReset),
                (2, FrameCommand::HardReset),
                // Both at once is a power cycle: it is the stronger of the two.
                (3, FrameCommand::HardReset),
            ]
        );
        // The frame indices above are the frames the **bits** are on, which is what the file
        // says. Where the reset is *applied* is one frame later, and the reader says so twice:
        // through the flag `movie-sync` reads and in a note that carries the measurement.
        assert!(file.reset_after_frame);
        assert!(
            file.notes
                .iter()
                .any(|note| note.contains("AFTER the frame") && note.contains("frame 246")),
            "{:?}",
            file.notes
        );
    }

    #[test]
    fn a_savestate_anchored_movie_is_refused_by_name() {
        let mut members: Vec<(&str, &str)> = vec![
            ("Header.txt", "Platform NES\r\nCore NesHawk\r\n"),
            ("SyncSettings.json", SYNC),
            ("Input Log.txt", "[Input]\r\n[/Input]\r\n"),
            ("CoreState", "not really a state"),
        ];
        let error = read(&archive(&members)).err().unwrap_or_default();
        assert!(error.contains("CoreState"), "{error}");
        assert!(error.contains("savestate"), "{error}");
        // And the header's own flag is checked too, because the two can disagree.
        members.pop();
        members[0].1 = "Platform NES\r\nStartsFromSavestate True\r\n";
        let error = read(&archive(&members)).err().unwrap_or_default();
        assert!(error.contains("starts from a savestate"), "{error}");
    }

    #[test]
    fn the_things_that_cannot_be_honoured_are_refused_each_by_its_name() {
        let line = "|..|........|........|";
        let cases: [(&str, &str, &str); 6] = [
            (
                "another system",
                r#"{"o":{"RegionOverride":0}}"#,
                "the `SNES` system",
            ),
            ("", r#"{"o":{"RegionOverride":2}}"#, "RegionOverride 2"),
            (
                "",
                r#"{"o":{"BoardProperties":{"PPUType":"x"},"RegionOverride":0}}"#,
                "PPUType",
            ),
            (
                "",
                r#"{"o":{"RegionOverride":0,"Controls":{"Famicom":true}}}"#,
                "Famicom",
            ),
            (
                "",
                r#"{"o":{"RegionOverride":0,"Controls":{"NesLeftPort":"Zapper"}}}"#,
                "Zapper",
            ),
            (
                "",
                r#"{"o":{"RegionOverride":0,"Controls":{"FamicomExpPort":"ArkanoidFam"}}}"#,
                "ArkanoidFam",
            ),
        ];
        for (index, (platform, sync, expected)) in cases.iter().enumerate() {
            let bytes = if index == 0 {
                let log = format!("[Input]\r\nLogKey:{NES_KEY}\r\n{line}\r\n[/Input]\r\n");
                archive(&[
                    ("Header.txt", "Platform SNES\r\n"),
                    ("SyncSettings.json", sync),
                    ("Input Log.txt", log.as_str()),
                ])
            } else {
                let _ = platform;
                movie("", sync, &[line])
            };
            let error = read(&bytes).err().unwrap_or_default();
            assert!(
                error.contains(expected),
                "case {index} should name {expected:?}, said {error:?}"
            );
        }
    }

    /// The power-on work RAM a `.bk2` asks for, and the one field that can override it.
    ///
    /// The pattern is not a reading of any source — the format does not carry one and `NesHawk`'s
    /// own is not published — it is lot EVAL-SYNC-2's measurement against the owner's `BizHawk`
    /// runs of *Faxanadu*, where of the three deterministic fills only this one puts any line of
    /// the trace in step at all. The note has to carry that, because a reader of the report
    /// otherwise has no way to tell a measurement from a guess.
    #[test]
    fn the_power_on_work_ram_is_the_measured_pattern_and_the_note_says_how_it_is_known() {
        let file = read(&movie("", SYNC, &["|..|........|........|"])).unwrap_or_default();
        assert_eq!(file.ram, RamPattern::FceuxDefault);
        let mut ram = [0x55u8; 16];
        file.ram.fill(&mut ram);
        assert_eq!(
            ram,
            [0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF, 0, 0, 0, 0, 0xFF, 0xFF, 0xFF, 0xFF]
        );
        let note = file
            .notes
            .iter()
            .find(|note| note.contains("00 00 00 00 FF FF FF FF"))
            .cloned()
            .unwrap_or_default();
        assert!(note.contains("measured"), "{note}");
        assert!(note.contains("207"), "the figure has to be in it: {note}");
    }

    /// `InitialWRamStatePattern` is the one sync setting that speaks about the 2 KiB. `null` is
    /// the core's own pattern; anything else names a fill whose shape the public description does
    /// not fix, and a wrong 2 KiB is not something emulation can recover from, so it is refused.
    #[test]
    fn an_explicit_initial_work_ram_pattern_is_refused_by_name() {
        let line = "|..|........|........|";
        for null in [
            r#"{"o":{"RegionOverride":0,"InitialWRamStatePattern":null}}"#,
            r#"{"o":{"RegionOverride":0}}"#,
        ] {
            let file = read(&movie("", null, &[line])).unwrap_or_default();
            assert_eq!(file.ram, RamPattern::FceuxDefault, "{null}");
        }
        let error = read(&movie(
            "",
            r#"{"o":{"RegionOverride":0,"InitialWRamStatePattern":[0,255,0,255]}}"#,
            &[line],
        ))
        .err()
        .unwrap_or_default();
        assert!(error.contains("InitialWRamStatePattern"), "{error}");
        assert!(error.contains("2 KiB fill"), "{error}");
    }

    #[test]
    fn a_log_key_naming_something_unknown_is_refused_rather_than_misread() {
        let log = "[Input]\r\nLogKey:#P1 Up|P1 Zapper X|\r\n|..|\r\n[/Input]\r\n";
        let bytes = archive(&[
            ("Header.txt", "Platform NES\r\n"),
            ("SyncSettings.json", SYNC),
            ("Input Log.txt", log),
        ]);
        let error = read(&bytes).err().unwrap_or_default();
        assert!(error.contains("P1 Zapper X"), "{error}");
    }

    #[test]
    fn a_frame_line_that_does_not_match_the_key_is_refused() {
        for bad in [
            "|..|.......|........|",
            "|..|........|........|.|",
            "|..|........|",
        ] {
            let error = read(&movie("", SYNC, &[bad])).err().unwrap_or_default();
            assert!(
                error.contains("does not match"),
                "{bad:?} should be refused, said {error:?}"
            );
        }
    }

    #[test]
    fn rubbish_and_missing_members_are_refused() {
        assert!(read(b"not a zip at all").is_err());
        let error = read(&archive(&[("Header.txt", "Platform NES\r\n")]))
            .err()
            .unwrap_or_default();
        assert!(error.contains("SyncSettings.json"), "{error}");
        let log = "[Input]\r\n|..|........|........|\r\n[/Input]\r\n";
        let error = read(&archive(&[
            ("Header.txt", "Platform NES\r\n"),
            ("SyncSettings.json", SYNC),
            ("Input Log.txt", log),
        ]))
        .err()
        .unwrap_or_default();
        assert!(error.contains("LogKey"), "{error}");
    }

    #[test]
    fn the_header_is_kept_and_the_sha1_read() {
        let file = read(&movie(
            "SHA1 9A8BF1084D90C3DB1A22CB295081295EED5D3402\r\nAuthor Someone\r\n",
            SYNC,
            &["|..|........|........|"],
        ))
        .unwrap_or_default();
        assert_eq!(
            file.header.get("Author").map(String::as_str),
            Some("Someone")
        );
        assert_eq!(file.header.get("Core").map(String::as_str), Some("NesHawk"));
        assert_eq!(
            file.rom_sha1.map(|digest| digest[0..3].to_vec()),
            Some(vec![0x9A, 0x8B, 0xF1])
        );
        assert!(!file.pal_stated, "the format states no region");
        assert!(file.notes.iter().any(|note| note.contains("NesHawk")));
        // A SHA1 that is not 40 hex characters is no SHA1 rather than a wrong one.
        let odd = read(&movie(
            "SHA1 nonsense\r\n",
            SYNC,
            &["|..|........|........|"],
        ))
        .unwrap_or_default();
        assert_eq!(odd.rom_sha1, None);
    }

    #[test]
    fn sha1_matches_the_published_examples() {
        let hex = |digest: [u8; 20]| {
            digest.iter().fold(String::new(), |mut text, byte| {
                let _ = write!(text, "{byte:02x}");
                text
            })
        };
        assert_eq!(hex(sha1(b"")), "da39a3ee5e6b4b0d3255bfef95601890afd80709");
        assert_eq!(
            hex(sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
    }
}
