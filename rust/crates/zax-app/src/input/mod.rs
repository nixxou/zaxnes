//! Input actions, bindings and the sampling of the devices (SPEC-08).

pub mod binding_set;
pub mod binding_target;
pub mod cursor_auto_hide;
pub mod default_bindings;
pub mod desktop_input_source;
pub mod gamepad_assignment;
pub mod input_action;
pub mod input_bindings;
pub mod input_snapshot;
pub mod libretro_input_map;
pub mod menu_navigation;
pub mod press_tracker;
pub mod rebinding_session;
pub mod routing;
pub mod zapper;

pub use binding_set::BindingSet;
pub use binding_target::{BindingDeviceKind, BindingTarget};
pub use desktop_input_source::{DesktopInputSource, DeviceRefs};
pub use input_action::InputAction;
pub use input_bindings::InputBindings;
pub use input_snapshot::InputSnapshot;
