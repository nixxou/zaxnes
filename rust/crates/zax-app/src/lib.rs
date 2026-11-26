#![forbid(unsafe_code)]

//! zax-app: the application loop of Zaxnes (port of `Zax.App`, without the user interface).

pub mod achievements;
pub mod app_log;
pub mod app_options;
pub mod application;
pub mod audio;
pub mod bundled_profiles;
pub mod cheat_database;
pub mod core_worker;
pub mod demo_game;
pub mod editor_mode;
pub mod emulation;
mod flags;
pub mod frame_profiler;
pub mod frame_rate_counter;
pub mod game_session;
pub mod gamedb;
pub mod graphics;
pub mod handheld_mode;
pub mod hd_pack;
pub mod head_on_pull;
pub mod import_folder;
pub mod input;
pub mod launch_request;
pub mod library_game_catalog;
pub mod mcp;
pub mod neutral_assets;
pub mod overscan;
pub mod persistence;
pub mod physical_file_system;
pub mod pipeline;
pub mod profile_resolver;
pub mod program;
pub mod render_scaling;
pub mod render_skip;
pub mod scene_script_bridge;
pub mod shader_pack;
pub mod single_instance;
pub mod steam_three_d_sen;
pub mod three_d_sen_command;
pub mod ui;
pub mod user_data_paths;
pub mod web_bridge;
