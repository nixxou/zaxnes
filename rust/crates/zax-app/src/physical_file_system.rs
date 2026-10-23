//! The profile library's view of the disk: the physical file system.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use zax_profiles::catalogue::ProfileFileSystem;

/// [`ProfileFileSystem`] over `std::fs`.
#[derive(Debug, Clone, Copy, Default)]
pub struct PhysicalProfileFileSystem;

impl ProfileFileSystem for PhysicalProfileFileSystem {
    fn file_exists(&self, path: &Path) -> bool {
        path.is_file()
    }

    fn directory_exists(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn create_directory(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn list_files(&self, directory: &Path, extension: &str) -> io::Result<Vec<PathBuf>> {
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let wanted = extension.to_lowercase();
        let mut files = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.to_string_lossy().to_lowercase().ends_with(&wanted) {
                files.push(path);
            }
        }
        Ok(files)
    }

    fn list_directories(&self, directory: &Path) -> io::Result<Vec<PathBuf>> {
        if !directory.is_dir() {
            return Ok(Vec::new());
        }
        let mut folders = Vec::new();
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                folders.push(entry.path());
            }
        }
        Ok(folders)
    }

    fn read_all_bytes(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn read_all_text(&self, path: &Path) -> io::Result<String> {
        fs::read_to_string(path)
    }

    fn write_all_bytes(&self, path: &Path, contents: &[u8]) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn write_all_text(&self, path: &Path, contents: &str) -> io::Result<()> {
        fs::write(path, contents)
    }

    fn delete_file(&self, path: &Path) -> io::Result<()> {
        if path.is_file() {
            fs::remove_file(path)
        } else {
            Ok(())
        }
    }

    fn copy_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        refuse_existing(destination, overwrite)?;
        fs::copy(source, destination).map(|_| ())
    }

    fn move_file(&self, source: &Path, destination: &Path, overwrite: bool) -> io::Result<()> {
        refuse_existing(destination, overwrite)?;
        fs::rename(source, destination)
    }
}

fn refuse_existing(destination: &Path, overwrite: bool) -> io::Result<()> {
    if !overwrite && destination.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("the file '{}' already exists.", destination.display()),
        ));
    }
    Ok(())
}
