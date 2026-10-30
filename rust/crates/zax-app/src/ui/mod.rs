//! The user interface (SPEC-10, AMD-010, AMD-035, AMD-036): the models, the two tables of texts
//! and the Dear `ImGui` screens. The models draw nothing, so the tests check them without a
//! window.

pub mod achievements_view;
pub mod commands;
pub mod confirm_dialog;
pub mod debug_overlay;
pub mod file_browser;
pub mod file_picker;
pub mod fonts;
pub mod frontend;
pub mod handheld;
pub mod key_config;
pub mod keys;
pub mod menu_bar;
pub mod menu_shortcuts;
#[cfg(feature = "native-dialog")]
pub mod native_picker;
pub mod notifications;
pub mod options;
pub mod platform;
pub mod profile_chooser;
pub mod shots;
pub mod state;
pub mod strings;
pub mod system_fonts;
pub mod text;
pub mod zax_ui;
