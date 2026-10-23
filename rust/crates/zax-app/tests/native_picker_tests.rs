//! When the native dialog is wanted: the opt-out and opt-in variable, the handhelds and the
//! sessions with no desktop.
#![cfg(feature = "native-dialog")]

use rstest::rstest;
use zax_app::ui::native_picker::{is_enabled, Desktop};

const DESKTOP: Desktop = Desktop {
    handheld: false,
    session: true,
};
const HANDHELD: Desktop = Desktop {
    handheld: true,
    session: true,
};
const NO_SESSION: Desktop = Desktop {
    handheld: false,
    session: false,
};

#[rstest]
#[case(None, DESKTOP, true)]
#[case(Some("0"), DESKTOP, false)]
#[case(Some(" 0 "), DESKTOP, false)]
#[case(Some("1"), DESKTOP, true)]
#[case(Some(""), DESKTOP, true)]
#[case(None, HANDHELD, false)]
#[case(Some("1"), HANDHELD, true)]
#[case(Some("0"), HANDHELD, false)]
#[case(None, NO_SESSION, false)]
#[case(Some("1"), NO_SESSION, true)]
fn the_native_dialog_follows_the_variable_then_the_system(
    #[case] variable: Option<&str>,
    #[case] desktop: Desktop,
    #[case] expected: bool,
) {
    assert_eq!(expected, is_enabled(variable, desktop));
}
