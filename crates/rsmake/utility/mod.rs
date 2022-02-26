use std::fs::File;
use std::path::{Path, PathBuf};

use crate::errors::FsError;

pub fn get_include_directory_from_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, FsError> {
    if path.as_ref().join("include").is_dir() {
        return Ok(path.as_ref().join("include"));
    } else {
        let parent = path.as_ref().parent().unwrap();
        if parent.join("include").is_dir() {
            return Ok(parent.join("include"));
        } else {
            return Err(FsError::NoIncludeDirectory(parent.into()));
        }
    }
}

pub fn get_mmk_library_file_from_path(path: &Path) -> Result<PathBuf, FsError> {
    if path.join("lib.mmk").is_file() {
        return Ok(path.join("lib.mmk"));
    } else {
        return Err(FsError::NoLibraryFile(path.into()));
    }
}

pub fn is_source_directory<P: AsRef<Path>>(path: P) -> bool {
    (path.as_ref().ends_with("source") || path.as_ref().ends_with("src")) && path.as_ref().is_dir()
}

pub fn is_test_directory<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().ends_with("test")
}

pub fn get_head_directory(path: &Path) -> &Path {
    let part_to_strip = path.parent().unwrap();
    return path.strip_prefix(part_to_strip).unwrap();
}

// Add test for this function.
pub fn get_project_top_directory(path: &Path) -> &Path {
    let parent = path.parent().unwrap();
    if is_source_directory(parent) || is_test_directory(parent) {
        return parent.parent().unwrap();
    } else {
        return parent;
    }
}

pub fn directory_exists(path: &Path) -> bool {
    path.exists()
}

pub fn create_dir<D: AsRef<Path>>(dir: D) -> Result<(), FsError> {
    if !dir.as_ref().is_dir() {
        std::fs::create_dir_all(dir.as_ref())
            .map_err(|err| FsError::CreateDirectory(dir.as_ref().to_path_buf(), err))?;
    }
    Ok(())
}

pub fn create_symlink<D, S>(destination: D, source: S) -> Result<(), FsError>
where
    D: AsRef<Path>,
    S: AsRef<Path>,
{
    std::os::unix::fs::symlink(destination.as_ref(), source.as_ref()).map_err(|err| {
        FsError::CreateSymlink {
            dest: destination.as_ref().to_path_buf(),
            src: source.as_ref().to_path_buf(),
            source: err,
        }
    })
}

pub fn create_file(file: &Path) -> Result<File, FsError> {
    File::create(&file).map_err(|err| FsError::CreateFile(file.to_path_buf(), err))
}

// This should be separated into its own "Make" mod.
pub fn print_full_path(os: &mut String, dir: &str, filename: &str, no_newline: bool) {
    os.push_str(dir);
    os.push_str("/");
    os.push_str(filename);
    if !no_newline {
        os.push_str(" \\\n");
    }
}

pub fn read_file(file_path: &Path) -> Result<String, FsError> {
    std::fs::read_to_string(&file_path)
        .map_err(|err| FsError::ReadFromFile(file_path.to_path_buf(), err))
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
