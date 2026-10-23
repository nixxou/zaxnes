//! The user interface (SPEC-10, AMD-010, AMD-035, AMD-036): the models, the two tables of texts
//! and the Dear `ImGui` screens. The models draw nothing, so the tests check them without a
//! window.

pub mod commands;
pub mod confirm_dialog;
pub mod file_browser;
pub mod fonts;
pub mod key_config;
pub mod keys;
pub mod menu_bar;
pub mod menu_shortcuts;
pub mod notifications;
pub mod options;
pub mod platform;
pub mod profile_chooser;
pub mod state;
pub mod strings;
pub mod system_fonts;
pub mod text;
pub mod zax_ui;
