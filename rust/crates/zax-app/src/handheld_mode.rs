//! Whether this run is on a handheld console (Rust-only): the gamepad-only interface replaces the
//! desktop one. Three things ask for it, in this order: `--handheld`, the variable
//! `ZAXNES_HANDHELD` (`1` forces it, `0` forbids the automatic choice), and an aarch64 Linux
//! machine that has neither an X11 nor a Wayland display, which is how the handheld firmwares
//! start their applications.

/// The variable that forces the handheld mode (`1`) or forbids the automatic choice (`0`).
pub const ENVIRONMENT_VARIABLE: &str = "ZAXNES_HANDHELD";

/// What the process knows about the machine it runs on.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DisplayEnvironment {
    /// The value of `ZAXNES_HANDHELD`, when set.
    pub handheld_variable: Option<String>,
    /// Whether an X11 display is set (`DISPLAY`).
    pub x11_display: bool,
    /// Whether a Wayland display is set (`WAYLAND_DISPLAY`).
    pub wayland_display: bool,
    /// Whether this is an aarch64 Linux build.
    pub aarch64_linux: bool,
}

impl DisplayEnvironment {
    /// The environment of the running process.
    #[must_use]
    pub fn of_process() -> Self {
        let present = |name: &str| std::env::var_os(name).is_some_and(|value| !value.is_empty());
        Self {
            handheld_variable: std::env::var(ENVIRONMENT_VARIABLE).ok(),
            x11_display: present("DISPLAY"),
            wayland_display: present("WAYLAND_DISPLAY"),
            aarch64_linux: cfg!(all(target_arch = "aarch64", target_os = "linux")),
        }
    }
}

/// Whether the handheld interface is used: `requested` is `--handheld`.
#[must_use]
pub fn resolve(requested: bool, environment: &DisplayEnvironment) -> bool {
    if requested {
        return true;
    }
    match environment.handheld_variable.as_deref().map(str::trim) {
        Some("1") => true,
        Some("0") => false,
        _ => environment.aarch64_linux && !environment.x11_display && !environment.wayland_display,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device() -> DisplayEnvironment {
        DisplayEnvironment {
            aarch64_linux: true,
            ..DisplayEnvironment::default()
        }
    }

    #[test]
    fn the_flag_always_wins() {
        let desktop = DisplayEnvironment {
            handheld_variable: Some("0".to_owned()),
            x11_display: true,
            ..DisplayEnvironment::default()
        };
        assert!(resolve(true, &desktop));
        assert!(!resolve(false, &desktop));
    }

    #[test]
    fn the_variable_forces_or_forbids() {
        let forced = DisplayEnvironment {
            handheld_variable: Some("1".to_owned()),
            x11_display: true,
            ..DisplayEnvironment::default()
        };
        assert!(resolve(false, &forced));
        let forbidden = DisplayEnvironment {
            handheld_variable: Some("0".to_owned()),
            ..device()
        };
        assert!(!resolve(false, &forbidden));
    }

    #[test]
    fn an_aarch64_linux_without_a_display_server_is_a_handheld() {
        assert!(resolve(false, &device()));
        for with_display in [
            DisplayEnvironment {
                x11_display: true,
                ..device()
            },
            DisplayEnvironment {
                wayland_display: true,
                ..device()
            },
        ] {
            assert!(!resolve(false, &with_display));
        }
    }

    #[test]
    fn another_platform_without_the_flag_is_a_desktop() {
        assert!(!resolve(false, &DisplayEnvironment::default()));
    }
}
