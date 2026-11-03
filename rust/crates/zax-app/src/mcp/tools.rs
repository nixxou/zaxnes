//! The tools of the MCP server and their execution on the main thread, once per displayed frame
//! after it is drawn (so a screenshot is of what the window shows).
//!
//! Every action goes through the host's commands, the same ones the window's menus and the movie
//! bar send: what Claude does shows in the window (a seek moves the bar). A call that starts a
//! seek (seek, step, an exact reload) is answered once the seek has arrived and a frame has been
//! drawn there; a screenshot asked during a seek waits for it too.

use std::fmt::Write as _;

use serde_json::{json, Value};

use super::server::{Content, McpServer, ToolCall, ToolResult};
use crate::application::AppHost;
use crate::emulation::commands::{Command, CommandArgs};

/// What the server tells the client about itself.
pub const INSTRUCTIONS: &str = "Zaxnes, a NES emulator that renders games in 3D through Lua \
profiles. These tools drive the running window: place the game on a frame of a movie (seek, \
step), look at it (screenshot, 3d or 2d), read its work RAM, reload the profile's scripts after \
editing them (fast: the scripts restart on the frame on screen; exact: the movie is replayed from \
its start with them), and read the log (Lua output and errors). Frames are counted from the \
movie's start.";

/// The tools, as `tools/list` gives them.
#[must_use]
pub fn definitions() -> Vec<Value> {
    let empty = json!({ "type": "object", "properties": {} });
    vec![
        json!({ "name": "status", "description": "The game, the movie (frame on screen, length, \
            seek under way, share explored), pause state and whether the scripts drive the core.",
            "inputSchema": empty }),
        json!({ "name": "open_movie", "description": "Plays a movie from its start: an FCEUX .fm2 \
            (from power on) or a Zaxnes movie (.mv<n>). A game must be running.",
            "inputSchema": { "type": "object", "properties": {
                "path": { "type": "string", "description": "The movie file" } },
                "required": ["path"] } }),
        json!({ "name": "seek", "description": "Goes to a frame of the movie (the scripts run the \
            last second before it). Answers once there.",
            "inputSchema": { "type": "object", "properties": {
                "frame": { "type": "integer", "minimum": 0 } }, "required": ["frame"] } }),
        json!({ "name": "step", "description": "Pauses and moves by `count` frames (negative: \
            back).",
            "inputSchema": { "type": "object", "properties": {
                "count": { "type": "integer", "default": 1 } } } }),
        json!({ "name": "play", "description": "Resumes the game.", "inputSchema": empty }),
        json!({ "name": "pause", "description": "Pauses the game.", "inputSchema": empty }),
        json!({ "name": "reload_script", "description": "Reads the profile file again and \
            restarts its scripts where the game is. fast: the scripts start on the frame on \
            screen; exact: the movie is replayed from its start with them (forced when the scripts \
            change the core). Answers once done.",
            "inputSchema": { "type": "object", "properties": {
                "mode": { "type": "string", "enum": ["fast", "exact"], "default": "fast" } } } }),
        json!({ "name": "screenshot", "description": "The picture on screen as a PNG: the 3D view \
            (scaled to width x height, default 960 x 720) or the NES picture (2d, 256 x 240, \
            wider with widescreen margins).",
            "inputSchema": { "type": "object", "properties": {
                "view": { "type": "string", "enum": ["3d", "2d"], "default": "3d" },
                "width": { "type": "integer", "minimum": 16, "maximum": 3840 },
                "height": { "type": "integer", "minimum": 16, "maximum": 2160 } } } }),
        json!({ "name": "read_ram", "description": "Reads CPU memory without side effects \
            ($0000-$07FF is the work RAM), as hexadecimal lines of 16 bytes.",
            "inputSchema": { "type": "object", "properties": {
                "address": { "type": "integer", "minimum": 0, "maximum": 65535 },
                "length": { "type": "integer", "minimum": 1, "maximum": 4096, "default": 16 } },
                "required": ["address"] } }),
        json!({ "name": "dump_frame", "description": "Writes the tile records of the frame on             screen to a JSON file, as zax-harness render --dump-frame does (palette, then low and             high planes, x, y, palette, mirrors, hidden, sprite of each tile).",
            "inputSchema": { "type": "object", "properties": {
                "path": { "type": "string", "description": "The JSON file to write" } },
                "required": ["path"] } }),
        json!({ "name": "get_log", "description": "The last lines of the log: Lua print, warnings \
            and errors, and the program's own lines.",
            "inputSchema": { "type": "object", "properties": {
                "lines": { "type": "integer", "minimum": 1, "maximum": 1000, "default": 50 } } } }),
        json!({ "name": "run_command", "description": "Runs a command of the menus by its name \
            (e.g. emulation.pause_resume, video.toggle_2d, emulation.reset), with an optional \
            number and text.",
            "inputSchema": { "type": "object", "properties": {
                "name": { "type": "string" },
                "number": { "type": "integer" },
                "text": { "type": "string" } }, "required": ["name"] } }),
    ]
}

/// A call answered later: when no seek runs and enough frames were drawn.
struct Deferred {
    call: ToolCall,
    /// Frames still to draw before it may be answered (the command has to be applied first).
    frames: u32,
}

/// The server and the calls waiting on the main thread.
pub struct McpBridge {
    server: McpServer,
    deferred: Vec<Deferred>,
}

impl McpBridge {
    /// Starts the server on `127.0.0.1:port`, and the log's memory for `get_log`.
    ///
    /// # Errors
    /// The port could not be bound.
    pub fn start(port: u16) -> std::io::Result<Self> {
        crate::app_log::keep_recent();
        Ok(Self {
            server: McpServer::start(port)?,
            deferred: Vec::new(),
        })
    }

    /// The port listened on.
    #[must_use]
    pub const fn port(&self) -> u16 {
        self.server.port()
    }

    /// Once a displayed frame, after it is drawn: answers what can be answered.
    pub fn serve(&mut self, host: &mut AppHost) {
        let seeking = host.movie_seek_target().is_some();
        let mut waiting = Vec::new();
        for mut deferred in std::mem::take(&mut self.deferred) {
            deferred.frames = deferred.frames.saturating_sub(1);
            if deferred.frames > 0 || seeking {
                waiting.push(deferred);
            } else {
                let result = finish(&deferred.call, host);
                let _ = deferred.call.reply.send(result);
            }
        }
        self.deferred = waiting;
        for call in self.server.take_calls() {
            match start(&call, host, seeking) {
                Started::Answer(result) => {
                    let _ = call.reply.send(result);
                }
                Started::Later(frames) => self.deferred.push(Deferred { call, frames }),
            }
        }
    }
}

/// What starting a call gives.
enum Started {
    Answer(ToolResult),
    /// Answered by [`finish`] after this many frames, no seek running.
    Later(u32),
}

/// Starts `call`: commands are sent, quick reads answered at once.
fn start(call: &ToolCall, host: &mut AppHost, seeking: bool) -> Started {
    let arguments = &call.arguments;
    let number = |name: &str| arguments.get(name).and_then(Value::as_i64);
    match call.name.as_str() {
        "status" | "get_log" | "read_ram" | "dump_frame" => Started::Answer(finish(call, host)),
        "screenshot" => {
            if seeking {
                Started::Later(1)
            } else {
                Started::Answer(finish(call, host))
            }
        }
        "open_movie" => {
            let Some(path) = arguments.get("path").and_then(Value::as_str) else {
                return Started::Answer(ToolResult::error("open_movie needs a path."));
            };
            host.enqueue(Command::OpenMovie, CommandArgs::of_text(path));
            Started::Later(3)
        }
        "seek" => {
            let Some(frame) = number("frame") else {
                return Started::Answer(ToolResult::error("seek needs a frame."));
            };
            if host.movie_position().is_none() {
                return Started::Answer(ToolResult::error("No movie is playing."));
            }
            host.enqueue(Command::Pause, CommandArgs::default());
            host.enqueue(
                Command::SeekMovie,
                CommandArgs::of_number(i32::try_from(frame.max(0)).unwrap_or(i32::MAX)),
            );
            Started::Later(3)
        }
        "step" => {
            let Some((position, _)) = host.movie_position() else {
                return Started::Answer(ToolResult::error("No movie is playing."));
            };
            let target = i64::from(position) + number("count").unwrap_or(1);
            host.enqueue(Command::Pause, CommandArgs::default());
            host.enqueue(
                Command::SeekMovie,
                CommandArgs::of_number(i32::try_from(target.max(0)).unwrap_or(i32::MAX)),
            );
            Started::Later(3)
        }
        "play" => {
            host.enqueue(Command::Resume, CommandArgs::default());
            Started::Later(2)
        }
        "pause" => {
            host.enqueue(Command::Pause, CommandArgs::default());
            Started::Later(2)
        }
        "reload_script" => {
            let exact = arguments.get("mode").and_then(Value::as_str) == Some("exact");
            host.enqueue(
                Command::ReloadScript,
                CommandArgs::of_number(i32::from(exact)),
            );
            Started::Later(3)
        }
        "run_command" => {
            let name = arguments.get("name").and_then(Value::as_str).unwrap_or("");
            let Some(command) = Command::from_name(name) else {
                return Started::Answer(ToolResult::error(format!("No command '{name}'.")));
            };
            let args = CommandArgs {
                number: number("number")
                    .and_then(|value| i32::try_from(value).ok())
                    .unwrap_or(0),
                text: arguments
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::to_owned),
                number2: 0,
            };
            if !host.can_execute(command, &args) {
                return Started::Answer(ToolResult::error(format!(
                    "The command '{name}' cannot run now."
                )));
            }
            host.enqueue(command, args);
            Started::Later(2)
        }
        other => Started::Answer(ToolResult::error(format!("No tool '{other}'."))),
    }
}

/// The answer of `call` once its command took effect: what the tool reads, or the status.
fn finish(call: &ToolCall, host: &mut AppHost) -> ToolResult {
    let arguments = &call.arguments;
    let number = |name: &str| arguments.get(name).and_then(Value::as_i64);
    match call.name.as_str() {
        "screenshot" => screenshot(host, arguments),
        "read_ram" => {
            let Some(session) = host.session() else {
                return ToolResult::error("No game is running.");
            };
            let address = number("address").unwrap_or(0).clamp(0, 0xFFFF);
            let length = number("length").unwrap_or(16).clamp(1, 4096);
            let mut text = String::new();
            for row in (address..(address + length).min(0x1_0000)).step_by(16) {
                let _ = write!(text, "{row:04X}:");
                for address in row..(row + 16).min(address + length).min(0x1_0000) {
                    let value = session
                        .core()
                        .read_memory(u16::try_from(address).unwrap_or(0));
                    let _ = write!(text, " {value:02X}");
                }
                text.push('\n');
            }
            ToolResult::text(text)
        }
        "dump_frame" => dump_frame(host, arguments),
        "get_log" => {
            let count = usize::try_from(number("lines").unwrap_or(50).clamp(1, 1000)).unwrap_or(50);
            ToolResult::text(crate::app_log::recent(count).join("\n"))
        }
        _ => ToolResult::text(status(host).to_string()),
    }
}

/// `dump_frame`: the tile records and palette of the frame on screen, in the harness's
/// `--dump-frame` JSON.
fn dump_frame(host: &AppHost, arguments: &Value) -> ToolResult {
    let Some(path) = arguments.get("path").and_then(Value::as_str) else {
        return ToolResult::error("dump_frame needs a path.");
    };
    let Some(session) = host.session() else {
        return ToolResult::error("No game is running.");
    };
    let batch = session.core().frame_batch();
    let mut text = String::from(
        "{
  \"palette\": [",
    );
    for (entry, value) in batch.palette.iter().enumerate() {
        let _ = write!(text, "{}{value}", if entry > 0 { ", " } else { "" });
    }
    text.push_str(
        "],
  \"tiles\": [
",
    );
    for (n, record) in batch.records.iter().enumerate() {
        let _ = write!(
            text,
            "    {{ \"low\": {}, \"high\": {}, \"x\": {}, \"y\": {}, \"palette\": {},              \"mirrorH\": {}, \"mirrorV\": {}, \"hidden\": {}, \"sprite\": {} }}{}",
            record.plane0,
            record.plane1,
            record.x,
            record.y,
            record.palette,
            record.mirror_h,
            record.mirror_v,
            record.hidden,
            !record.background,
            if n + 1 < batch.records.len() { ",
" } else { "
" }
        );
    }
    text.push_str(
        "  ]
}
",
    );
    match std::fs::write(path, text) {
        Ok(()) => ToolResult::text(format!("{} tiles written to {path}", batch.records.len())),
        Err(error) => ToolResult::error(format!("{path}: {error}")),
    }
}

/// The state the `status` tool gives, and what every action answers with.
fn status(host: &AppHost) -> Value {
    let position = host.movie_position();
    json!({
        "game": host.current_game_title(),
        "state": format!("{:?}", host.state()),
        "movie": position.map(|(frame, length)| json!({
            "frame": frame,
            "length": length,
            "seeking_to": host.movie_seek_target(),
            "explored": host.movie_explored(),
            "explorer_trusted": host.movie_explorer_trusted(),
        })),
        "core_frame_count": host.session().map(|session| session.core().frame_count()),
        "scripts_drive_the_core": host.scripts_drive_the_core(),
        "two_dimensional": host.showing_two_dimensional(),
    })
}

/// The picture on screen as a PNG.
fn screenshot(host: &mut AppHost, arguments: &Value) -> ToolResult {
    let two_dimensional = arguments.get("view").and_then(Value::as_str) == Some("2d");
    let png = if two_dimensional {
        let Some(session) = host.session() else {
            return ToolResult::error("No game is running.");
        };
        let mut picture = Vec::new();
        let rows = session.copy_nes_picture(&mut picture);
        if rows == 0 || picture.len() % rows != 0 {
            return ToolResult::error("No picture yet.");
        }
        let width = picture.len() / rows;
        let rgb: Vec<u8> = picture
            .iter()
            .flat_map(|&index| {
                let color = zax_engine3d::palette::nes_color(usize::from(index));
                [color.r, color.g, color.b]
            })
            .collect();
        zax_render::assets::png_image::encode_rgb(
            u32::try_from(width).unwrap_or(0),
            u32::try_from(rows).unwrap_or(0),
            &rgb,
        )
    } else {
        let size = |name: &str, default: u32| {
            arguments
                .get(name)
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
                .unwrap_or(default)
                .clamp(16, 3840)
        };
        let (width, height) = (size("width", 960), size("height", 720));
        let Some(rgb) = host
            .view_mut()
            .and_then(|view| view.capture_rgb(width, height))
        else {
            return ToolResult::error("The 3D view could not be read.");
        };
        zax_render::assets::png_image::encode_rgb(width, height, &rgb)
    };
    match png {
        Ok(png) => ToolResult {
            content: vec![Content::Png(png), Content::Text(status(host).to_string())],
            is_error: false,
        },
        Err(error) => ToolResult::error(format!("PNG: {error}")),
    }
}
