#![forbid(unsafe_code)]

//! zax-app: the application loop of Zaxnes (port of `Zax.App`, without the user interface).

pub mod app_options;
pub mod audio;
pub mod emulation;
mod flags;
pub mod frame_rate_counter;
pub mod game_session;
pub mod input;
pub mod library_game_catalog;
pub mod neutral_assets;
pub mod persistence;
pub mod physical_file_system;
pub mod render_scaling;
pub mod render_skip;
pub mod scene_script_bridge;
pub mod user_data_paths;
