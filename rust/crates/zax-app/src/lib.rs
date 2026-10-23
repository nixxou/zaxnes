#![forbid(unsafe_code)]

//! zax-app: the application loop of Zaxnes (port of `Zax.App`, without the user interface).

mod flags;
pub mod audio;
pub mod emulation;
pub mod frame_rate_counter;
pub mod input;
pub mod library_game_catalog;
pub mod neutral_assets;
pub mod physical_file_system;
pub mod persistence;
pub mod render_scaling;
pub mod render_skip;
pub mod user_data_paths;
