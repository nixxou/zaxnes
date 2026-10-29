//! The emulation loop (SPEC-07): states, commands, games, save states, rollback.

pub mod commands;
pub mod emulation_state;
pub mod fast_forward_state;
pub mod frame_pacer;
pub mod frame_steps;
pub mod game_catalog;
pub mod host;
pub mod hotkey_tracker;
pub mod notification_center;
pub mod rollback_history;
pub mod rom_file;
pub mod speed_setting;
pub mod video_pacing;
