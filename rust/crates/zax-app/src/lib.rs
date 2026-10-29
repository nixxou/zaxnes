#![forbid(unsafe_code)]

//! zax-app: the application loop of Zaxnes (port of `Zax.App`, without the user interface).

pub mod app_log;
pub mod app_options;
pub mod application;
pub mod audio;
pub mod core_worker;
pub mod emulation;
mod flags;
pub mod frame_profiler;
pub mod frame_rate_counter;
pub mod game_session;
pub mod graphics;
pub mod handheld_mode;
pub mod input;
pub mod library_game_catalog;
pub mod neutral_assets;
pub mod overscan;
pub mod persistence;
pub mod physical_file_system;
pub mod pipeline;
pub mod profile_resolver;
pub mod render_scaling;
pub mod render_skip;
pub mod scene_script_bridge;
pub mod single_instance;
pub mod three_d_sen_command;
pub mod ui;
pub mod user_data_paths;
