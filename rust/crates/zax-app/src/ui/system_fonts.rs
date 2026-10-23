//! Finds a font of the system for the interface (REQ-PUB-006): nothing is bundled, so the
//! interface uses the face of the platform and falls back to the built-in one of `ImGui`.

use std::path::{Path, PathBuf};

/// A regular face and, when there is one, the bold face of the same family.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FontFiles {
    /// The regular face.
    pub regular: Option<PathBuf>,
    /// The bold face of the same family.
    pub bold: Option<PathBuf>,
}

const WINDOWS_CANDIDATES: [(&str, &str); 4] = [
    ("segoeui.ttf", "segoeuib.ttf"),
    ("arial.ttf", "arialbd.ttf"),
    ("tahoma.ttf", "tahomabd.ttf"),
    ("verdana.ttf", "verdanab.ttf"),
];

const UNIX_CANDIDATES: [(&str, &str); 3] = [
    (
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ),
    (
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Bold.ttf",
    ),
    (
        "/System/Library/Fonts/Supplemental/Arial.ttf",
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
    ),
];

/// The regular and bold files to use: the `hint` (a path, or a file name in the fonts folder)
/// when it exists, else the first family of the platform's list that is installed.
#[must_use]
pub fn locate(hint: Option<&str>) -> FontFiles {
    if let Some(hint) = hint.filter(|hint| !hint.trim().is_empty()) {
        if Path::new(hint).is_file() {
            return FontFiles {
                regular: Some(std::path::absolute(hint).unwrap_or_else(|_| PathBuf::from(hint))),
                bold: None,
            };
        }
        if let Some(named) = resolve(hint) {
            return FontFiles {
                regular: Some(named),
                bold: None,
            };
        }
    }
    let candidates: &[(&str, &str)] = if cfg!(windows) {
        &WINDOWS_CANDIDATES
    } else {
        &UNIX_CANDIDATES
    };
    for (regular, bold) in candidates {
        if let Some(regular) = resolve(regular) {
            return FontFiles {
                regular: Some(regular),
                bold: resolve(bold),
            };
        }
    }
    FontFiles::default()
}

/// The fonts folder of Windows: `%WINDIR%\Fonts`, the documented location.
fn folder() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let windows = std::env::var_os("WINDIR").filter(|value| !value.is_empty())?;
    let folder = PathBuf::from(windows).join("Fonts");
    folder.is_dir().then_some(folder)
}

fn resolve(name: &str) -> Option<PathBuf> {
    let path = Path::new(name);
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    let candidate = folder()?.join(name);
    candidate.is_file().then_some(candidate)
}
