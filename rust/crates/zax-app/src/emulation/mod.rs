//! The emulation loop (SPEC-07): states, commands, games, save states, rollback.

pub mod emulation_state;
pub mod fast_forward_state;
pub mod frame_pacer;
pub mod hotkey_tracker;
pub mod notification_center;
pub mod rollback_history;
pub mod speed_setting;
pub mod game_catalog;
