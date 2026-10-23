//! An in-memory file system: nothing is read from or written to a real disk, and no path of any
//! machine appears in the tests (lot PUB-1).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::{Path, PathBuf};

use zax_profiles::catalogue::ProfileFileSystem;

#[derive(Default)]
pub struct FakeProfileFileSystem {
    files: RefCell<BTreeMap<String, (PathBuf, Vec<u8>)>>,
    directories: RefCell<BTreeMap<String, PathBuf>>,
    reads: RefCell<Vec<PathBuf>>,
    unreadable: RefCell<BTreeSet<String>>,
}

fn key(path: &Path) -> String {
    path.to_string_lossy()
        .trim_end_matches(['/', '\\'])
        .to_lowercase()
}

impl FakeProfileFileSystem {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_file(&self, path: &Path, contents: &[u8]) {
        let mut files = self.files.borrow_mut();
        match files.get_mut(&key(path)) {
            Some((_, existing)) => contents.clone_into(existing),
            None => {
                files.insert(key(path), (path.to_path_buf(), contents.to_vec()));
            }
        }
        drop(files);
        self.add_parent_directories(path);
    }

    pub fn add_text(&self, path: &Path, contents: &str) {
        self.add_file(path, contents.as_bytes());
    }

    /// Every file path, as first written.
    pub fn paths(&self) -> Vec<PathBuf> {
        self.files
            .borrow()
            .values()
            .map(|(path, _)| path.clone())
            .collect()
    }

    /// The paths read so far, in order.
    pub fn reads(&self) -> Vec<PathBuf> {
        self.reads.borrow().clone()
    }

    /// Makes listing `directory` fail with an access error.
    pub fn make_unreadable(&self, directory: &Path) {
        self.unreadable.borrow_mut().insert(key(directory));
    }

    fn add_parent_directories(&self, path: &Path) {
        let mut directories = self.directories.borrow_mut();
        for ancestor in path.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() {
                directories
                    .entry(key(ancestor))
                    .or_insert_with(|| ancestor.to_path_buf());
            }
        }
    }

    fn not_found(path: &Path) -> io::Error {
        io::Error::new(io::ErrorKind::NotFound, path.display().to_string())
    }
}

impl ProfileFileSystem for FakeProfileFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        self.files.borrow().contains_key(&key(path))
    }

    fn directory_exists(&self, path: &Path) -> bool {
        self.directories.borrow().contains_key(&key(path))
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        self.directories
            .borrow_mut()
            .entry(key(path))
            .or_insert_with(|| path.to_path_buf());
        Ok(())
    }

    fn list_files(&self, directory: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
        let normalised = key(directory);
        if self.unreadable.borrow().contains(&normalised) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("access to {} is denied", directory.display()),
            ));
        }

        let extension = extension.to_lowercase();
        let mut matches: Vec<PathBuf> = self
            .files
            .borrow()
            .values()
            .filter(|(path, _)| {
                path.parent()
                    .is_some_and(|parent| key(parent) == normalised)
                    && path.to_string_lossy().to_lowercase().ends_with(&extension)
            })
            .map(|(path, _)| path.clone())
            .collect();

        // Deliberately not in name order: the scan is what must impose an order.
        matches.reverse();
        Ok(matches)
    }

    fn list_directories(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        let normalised = key(directory);
        Ok(self
            .directories
            .borrow()
            .values()
            .filter(|path| {
                path.parent()
                    .is_some_and(|parent| key(parent) == normalised)
            })
            .cloned()
            .collect())
    }

    fn read_all_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.reads.borrow_mut().push(path.to_path_buf());
        self.files
            .borrow()
            .get(&key(path))
            .map(|(_, contents)| contents.clone())
            .ok_or_else(|| Self::not_found(path))
    }

    fn read_all_text(&self, path: &Path) -> io::Result<String> {
        String::from_utf8(self.read_all_bytes(path)?)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
    }

    fn write_all_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        self.add_file(path, contents);
        Ok(())
    }

    fn write_all_text(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.add_text(path, contents);
        Ok(())
    }

    fn delete_file(&self, path: &Path) -> io::Result<()> {
        self.files.borrow_mut().remove(&key(path));
        Ok(())
    }

    fn copy_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        if !overwrite && self.file_exists(destination) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} already exists", destination.display()),
            ));
        }
        let contents = self.read_all_bytes(source)?;
        self.add_file(destination, &contents);
        Ok(())
    }

    fn move_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        self.copy_file(source, destination, overwrite)?;
        self.files.borrow_mut().remove(&key(source));
        Ok(())
    }
}
