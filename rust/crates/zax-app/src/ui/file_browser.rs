//! The file browser as data: the folder shown, its entries, the filter, the search text, the
//! path trail and the chosen result (AMD-010 REQ-UI-101). It draws nothing.

use std::path::{Component, Path, PathBuf};

use super::keys;
use super::strings;
use super::text;

/// What an entry of the list is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileEntryKind {
    /// A drive or the root of the file system, listed above any folder.
    Drive,
    /// A folder.
    Folder,
    /// A file that passes the filter.
    File,
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileEntry {
    /// The name shown.
    pub name: String,
    /// The full path.
    pub path: String,
    /// What it is.
    pub kind: FileEntryKind,
}

/// One entry of the file type drop-down: a label and the extensions it admits (none admits
/// every file).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileFilter {
    /// The label shown.
    pub label: String,
    /// The extensions without the dot.
    pub extensions: Vec<&'static str>,
}

fn filter(label_key: &str, extensions: &[&'static str]) -> FileFilter {
    FileFilter {
        label: strings::get(label_key),
        extensions: extensions.to_vec(),
    }
}

/// The filters of the ROM dialog (REQ-UI-201).
#[must_use]
pub fn rom_filters() -> Vec<FileFilter> {
    vec![
        filter(keys::BROWSER_FILTER_NES_ROMS, &["nes", "zip"]),
        filter(keys::BROWSER_FILTER_NES_FILES, &["nes"]),
        filter(keys::BROWSER_FILTER_ZIP, &["zip"]),
        filter(keys::BROWSER_FILTER_ALL, &[]),
    ]
}

/// The filters of the profile import dialog.
#[must_use]
pub fn profile_filters() -> Vec<FileFilter> {
    vec![
        filter(keys::BROWSER_FILTER_PROFILES, &["3dn"]),
        filter(keys::BROWSER_FILTER_ALL, &[]),
    ]
}

/// The filters of the 3dSen installation picker (AMD-030 §2).
#[must_use]
pub fn three_d_sen_filters() -> Vec<FileFilter> {
    vec![
        filter(keys::BROWSER_FILTER_BUNDLE, &["bundle"]),
        filter(keys::BROWSER_FILTER_ALL, &[]),
    ]
}

/// The filters of the cover import dialog (Q-050).
#[must_use]
pub fn cover_filters() -> Vec<FileFilter> {
    vec![
        filter(keys::BROWSER_FILTER_PNG, &["png"]),
        filter(keys::BROWSER_FILTER_ALL, &[]),
    ]
}

/// The label of the root of the trail, "Computer".
#[must_use]
pub fn computer_label() -> String {
    strings::get(keys::BROWSER_COMPUTER)
}

/// The state of the file browser.
#[derive(Debug, Clone)]
pub struct FileBrowserModel {
    title: String,
    filters: Vec<FileFilter>,
    filter_index: usize,
    directory: Option<String>,
    entries: Vec<FileEntry>,
    trail: Vec<(String, String)>,
    search: String,
    /// The entry the user selected.
    pub selected: Option<FileEntry>,
    allow_folders: bool,
    hint: String,
    result: Option<String>,
}

impl Default for FileBrowserModel {
    fn default() -> Self {
        Self {
            title: text::open_rom_title(),
            filters: Vec::new(),
            filter_index: 0,
            directory: None,
            entries: Vec::new(),
            trail: Vec::new(),
            search: String::new(),
            selected: None,
            allow_folders: false,
            hint: String::new(),
            result: None,
        }
    }
}

impl FileBrowserModel {
    /// A closed browser.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The title of the dialog.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The extensions in force; empty admits every file.
    #[must_use]
    pub fn extensions(&self) -> &[&'static str] {
        self.filters
            .get(self.filter_index)
            .map_or(&[], |filter| filter.extensions.as_slice())
    }

    /// The index of the filter in force.
    #[must_use]
    pub const fn filter_index(&self) -> usize {
        self.filter_index
    }

    /// The filters of the drop-down.
    #[must_use]
    pub fn filters(&self) -> &[FileFilter] {
        &self.filters
    }

    /// The folder shown; none lists the drives.
    #[must_use]
    pub fn directory(&self) -> Option<&str> {
        self.directory.as_deref()
    }

    /// The lines of the list: drives or folders first, then the files.
    #[must_use]
    pub fn entries(&self) -> &[FileEntry] {
        &self.entries
    }

    /// The path trail: one button per component (REQ-UI-101); each is a label and a target.
    #[must_use]
    pub fn trail(&self) -> &[(String, String)] {
        &self.trail
    }

    /// Whether the dialog answers with a folder as well as a file (AMD-030 §2).
    #[must_use]
    pub const fn allow_folders(&self) -> bool {
        self.allow_folders
    }

    /// What the dialog asks for, when it says so.
    #[must_use]
    pub fn hint(&self) -> &str {
        &self.hint
    }

    /// The chosen path, once the dialog confirmed one.
    #[must_use]
    pub fn result(&self) -> Option<&str> {
        self.result.as_deref()
    }

    /// The text of the file name field: the selected file, or the folder when folders count.
    #[must_use]
    pub fn file_name(&self) -> String {
        match &self.selected {
            Some(entry) if entry.kind == FileEntryKind::File => entry.name.clone(),
            Some(entry) if self.allow_folders => entry.name.clone(),
            _ if self.allow_folders => self.directory.clone().unwrap_or_default(),
            _ => String::new(),
        }
    }

    /// Whether the Open button works.
    #[must_use]
    pub fn can_confirm(&self) -> bool {
        matches!(&self.selected, Some(entry) if entry.kind == FileEntryKind::File)
            || (self.allow_folders && (self.selected.is_some() || self.directory.is_some()))
    }

    /// The search text.
    #[must_use]
    pub fn search(&self) -> &str {
        &self.search
    }

    /// Sets the search text and lists the folder again.
    pub fn set_search(&mut self, value: &str) {
        if self.search != value {
            value.clone_into(&mut self.search);
            self.reload();
        }
    }

    /// Opens the browser: a title, the filters, the folder to start in (the drives when it does
    /// not exist), and whether a folder is an answer.
    pub fn open(
        &mut self,
        title: &str,
        filters: Vec<FileFilter>,
        start_directory: Option<&str>,
        allow_folders: bool,
        hint: Option<&str>,
    ) {
        title.clone_into(&mut self.title);
        self.filters = filters;
        self.filter_index = 0;
        self.result = None;
        self.selected = None;
        self.allow_folders = allow_folders;
        hint.unwrap_or_default().clone_into(&mut self.hint);
        self.search.clear();
        self.go_to(start_directory);
    }

    /// Picks the filter at `index` and lists the folder again.
    pub fn choose_filter(&mut self, index: usize) {
        if self.filters.is_empty() {
            return;
        }
        self.filter_index = index.min(self.filters.len() - 1);
        self.selected = None;
        self.reload();
    }

    /// Shows `directory`, or the drives when it is none or does not exist.
    pub fn go_to(&mut self, directory: Option<&str>) {
        self.directory = directory
            .filter(|directory| exists(directory))
            .map(|directory| full_path(Path::new(directory)));
        self.selected = None;
        self.reload();
        self.rebuild_trail();
    }

    /// Goes to the parent of the folder shown; from a root, to the drives.
    pub fn go_up(&mut self) {
        let Some(directory) = self.directory.as_deref() else {
            return;
        };
        let parent = Path::new(directory)
            .parent()
            .map(|parent| parent.to_string_lossy().into_owned())
            .filter(|parent| !parent.is_empty());
        self.go_to(parent.as_deref());
    }

    /// Double click or Enter on an entry: a file is the answer, a folder is entered. Returns
    /// whether the dialog answered.
    pub fn activate(&mut self, entry: &FileEntry) -> bool {
        if entry.kind == FileEntryKind::File {
            self.result = Some(entry.path.clone());
            return true;
        }
        self.go_to(Some(&entry.path));
        false
    }

    /// The Open button. Returns whether the dialog answered.
    pub fn confirm(&mut self) -> bool {
        if let Some(file) = self
            .selected
            .as_ref()
            .filter(|entry| entry.kind == FileEntryKind::File)
        {
            self.result = Some(file.path.clone());
            return true;
        }
        if !self.allow_folders {
            return false;
        }
        if let Some(folder) = &self.selected {
            self.result = Some(folder.path.clone());
            return true;
        }
        match &self.directory {
            Some(directory) => {
                self.result = Some(directory.clone());
                true
            }
            None => false,
        }
    }

    /// Whether the filter in force admits the file `name`.
    #[must_use]
    pub fn accepts(&self, name: &str) -> bool {
        let extensions = self.extensions();
        if extensions.is_empty() {
            return true;
        }
        Path::new(name)
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                extensions
                    .iter()
                    .any(|allowed| allowed.eq_ignore_ascii_case(extension))
            })
    }

    fn reload(&mut self) {
        self.entries = self.list();
    }

    fn list(&self) -> Vec<FileEntry> {
        let Some(directory) = &self.directory else {
            return drives()
                .into_iter()
                .map(|drive| FileEntry {
                    name: drive.clone(),
                    path: drive,
                    kind: FileEntryKind::Drive,
                })
                .collect();
        };
        let needle = self.search.trim().to_lowercase();
        let mut folders = Vec::new();
        let mut files = Vec::new();
        // An unreadable folder lists nothing, as the C# does (IOException, access denied).
        if let Ok(listing) = std::fs::read_dir(directory) {
            for item in listing.flatten() {
                let name = item.file_name().to_string_lossy().into_owned();
                let path = item.path().to_string_lossy().into_owned();
                if item.file_type().is_ok_and(|kind| kind.is_dir()) {
                    folders.push(FileEntry {
                        name,
                        path,
                        kind: FileEntryKind::Folder,
                    });
                } else if self.accepts(&name)
                    && (needle.is_empty() || name.to_lowercase().contains(&needle))
                {
                    files.push(FileEntry {
                        name,
                        path,
                        kind: FileEntryKind::File,
                    });
                }
            }
        }
        folders.sort_by_cached_key(|entry| entry.name.to_uppercase());
        files.sort_by_cached_key(|entry| entry.name.to_uppercase());
        folders.extend(files);
        folders
    }

    fn rebuild_trail(&mut self) {
        self.trail.clear();
        self.trail.push((computer_label(), String::new()));
        let Some(directory) = &self.directory else {
            return;
        };
        // One button per component, each ending with the separator (REQ-UI-101).
        let mut walked = PathBuf::new();
        for component in Path::new(directory).components() {
            match component {
                Component::Prefix(_) => walked.push(component),
                Component::RootDir => {
                    walked.push(component);
                    let root = walked.to_string_lossy().into_owned();
                    self.trail.push((root.clone(), root));
                }
                Component::Normal(part) => {
                    walked.push(part);
                    self.trail.push((
                        format!("{}{}", part.to_string_lossy(), std::path::MAIN_SEPARATOR),
                        walked.to_string_lossy().into_owned(),
                    ));
                }
                Component::CurDir | Component::ParentDir => {}
            }
        }
    }
}

fn exists(directory: &str) -> bool {
    !directory.trim().is_empty() && Path::new(directory).is_dir()
}

fn full_path(path: &Path) -> String {
    std::path::absolute(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned()
}

#[cfg(windows)]
fn drives() -> Vec<String> {
    ('A'..='Z')
        .map(|letter| format!("{letter}:\\"))
        .filter(|drive| Path::new(drive).is_dir())
        .collect()
}

#[cfg(not(windows))]
fn drives() -> Vec<String> {
    vec!["/".to_owned()]
}
