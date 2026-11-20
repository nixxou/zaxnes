//! Where Steam installed 3dSen (owner request, 2026-11-20, Windows only): Steam's folder from the
//! registry (`HKCU\Software\Valve\Steam`, `SteamPath`, read with `reg.exe`: no foreign call),
//! else its usual places; then every library its `steamapps/libraryfolders.vdf` lists, and in
//! each one the `steamapps/common` folders whose name holds "3dSen" (3dSen, 3dSenVR...).

use std::path::{Path, PathBuf};

/// The 3dSen installations Steam knows, best first: the plain game before the VR one.
#[must_use]
pub fn installations() -> Vec<PathBuf> {
    let mut found = Vec::new();
    for steam in steam_folders() {
        for library in libraries(&steam) {
            let common = library.join("steamapps").join("common");
            let Ok(listing) = std::fs::read_dir(&common) else {
                continue;
            };
            let mut games: Vec<PathBuf> = listing
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| {
                    path.is_dir()
                        && path.file_name().is_some_and(|name| {
                            name.to_string_lossy().to_lowercase().contains("3dsen")
                        })
                })
                .collect();
            // "3dSen" before "3dSenVR": the shorter name first.
            games.sort_by_key(|path| path.as_os_str().len());
            for game in games {
                if !contains(&found, &game) {
                    found.push(game);
                }
            }
        }
    }
    found
}

/// Whether `paths` holds `path`, letter case aside (Windows paths: the registry writes Steam's
/// in lower case, its library list in the case it was made with).
fn contains(paths: &[PathBuf], path: &Path) -> bool {
    let key = |path: &Path| path.to_string_lossy().to_lowercase().replace('/', "\\");
    paths.iter().any(|known| key(known) == key(path))
}

/// Steam's folders: the registry's, then the usual ones that exist.
fn steam_folders() -> Vec<PathBuf> {
    let mut folders = Vec::new();
    if let Some(folder) = registry_steam_path() {
        folders.push(folder);
    }
    for usual in ["C:\\Program Files (x86)\\Steam", "C:\\Program Files\\Steam"] {
        let usual = PathBuf::from(usual);
        if usual.is_dir() && !contains(&folders, &usual) {
            folders.push(usual);
        }
    }
    folders
}

/// `SteamPath` of the registry, on Windows.
fn registry_steam_path() -> Option<PathBuf> {
    if !cfg!(windows) {
        return None;
    }
    let mut command = std::process::Command::new("reg");
    command.args(["query", "HKCU\\Software\\Valve\\Steam", "/v", "SteamPath"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // No console window flashing up.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    let output = command.output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_reg_value(&text).map(|path| PathBuf::from(path.replace('/', "\\")))
}

/// The value of the `reg query` output `text`: `    SteamPath    REG_SZ    c:/program files (x86)/steam`.
fn parse_reg_value(text: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let (_, value) = line.split_once("REG_SZ")?;
        let value = value.trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}

/// The libraries of the Steam at `steam`: itself, then those of its `libraryfolders.vdf`.
fn libraries(steam: &Path) -> Vec<PathBuf> {
    let mut libraries = vec![steam.to_path_buf()];
    let vdf = steam.join("steamapps").join("libraryfolders.vdf");
    if let Ok(text) = std::fs::read_to_string(vdf) {
        for path in parse_library_paths(&text) {
            if !contains(&libraries, &path) {
                libraries.push(path);
            }
        }
    }
    libraries
}

/// The `"path"` values of a `libraryfolders.vdf`, their `\\` made single.
fn parse_library_paths(text: &str) -> Vec<PathBuf> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            let rest = line.strip_prefix("\"path\"")?.trim();
            let value = rest.strip_prefix('"')?.strip_suffix('"')?;
            Some(PathBuf::from(value.replace("\\\\", "\\")))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_registry_and_the_library_list_are_read() {
        let reg = "\r\nHKEY_CURRENT_USER\\Software\\Valve\\Steam\r\n    SteamPath    REG_SZ    c:/program files (x86)/steam\r\n\r\n";
        assert_eq!(
            Some("c:/program files (x86)/steam".to_owned()),
            parse_reg_value(reg)
        );
        let vdf = "\"libraryfolders\"\n{\n\t\"0\"\n\t{\n\t\t\"path\"\t\t\"C:\\\\Program Files (x86)\\\\Steam\"\n\t}\n\t\"1\"\n\t{\n\t\t\"path\"\t\t\"D:\\\\SteamLibrary\"\n\t\t\"label\"\t\t\"\"\n\t}\n}\n";
        assert_eq!(
            vec![
                PathBuf::from("C:\\Program Files (x86)\\Steam"),
                PathBuf::from("D:\\SteamLibrary")
            ],
            parse_library_paths(vdf)
        );
    }
}
