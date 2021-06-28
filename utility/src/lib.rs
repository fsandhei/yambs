use error::MyMakeError;
use std::fs::File;
use std::path::{PathBuf, Path};

// TODO: Vurder om denne skal returnere Result<PathBuf, MyMakeError>
pub fn get_source_directory_from_path<P: AsRef<Path>>(path: P) -> PathBuf {
    if path.as_ref().join("source").is_dir() {
        return path.as_ref().join("source");
    }
    else if path.as_ref().join("src").is_dir() {
        return path.as_ref().join("src");
    }
    else {
        return path.as_ref().to_path_buf();
    }
}


pub fn get_include_directory_from_path<P: AsRef<Path>>(path: P) -> Result<PathBuf, MyMakeError> {
    if path.as_ref().join("include").is_dir() {
        return Ok(path.as_ref().join("include"));
    }
    else {
        let parent = path.as_ref().parent().unwrap();
        if parent.join("include").is_dir() {
            return Ok(parent.join("include"));
        }
        else {
            return Err(MyMakeError::from(format!("Error: Could not find include directory from {:?}", parent)));
        }        
    }
}


pub fn get_mmk_library_file_from_path(path: &PathBuf) -> Result<PathBuf, MyMakeError> {
    if path.join("lib.mmk").is_file() {
        return Ok(path.join("lib.mmk"));
    }
    else {
        return Err(MyMakeError::from(format!("{:?} does not contain a lib.mmk file!", path)));
    }
}


pub fn is_source_directory<P: AsRef<Path>>(path: P) -> bool {
    (path.as_ref().ends_with("source") || path.as_ref().ends_with("src"))
    && path.as_ref().is_dir()
}


pub fn is_test_directory<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().ends_with("test")
}


pub fn get_head_directory(path: &Path) -> &Path {
    let part_to_strip = path.parent().unwrap();
    return path.strip_prefix(part_to_strip).unwrap()
}


// Add test for this function.
pub fn get_project_top_directory(path: &Path) -> &Path {
    let parent = path.parent().unwrap();
    if is_source_directory(parent) || is_test_directory(parent) {
        return parent.parent().unwrap();
    }
    else {
        return parent;
    }
}


pub fn directory_exists(path: &Path) -> bool {
    path.exists()
}


pub fn create_dir<D: AsRef<Path>>(dir: D) -> Result<(), MyMakeError> {
    if !dir.as_ref().is_dir() {
        std::fs::create_dir_all(dir.as_ref())?;
    }
    Ok(())
}


pub fn remove_dir(dir: &std::path::PathBuf) -> Result<(), MyMakeError> {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
}


pub fn create_symlink<D, S>(destination: D, source: S) -> Result<(), MyMakeError> 
    where D: AsRef<Path>,
          S: AsRef<Path> {
    match std::os::unix::fs::symlink(destination.as_ref(), source.as_ref()) {
        Ok(()) => Ok(()),
        Err(err) => Err(MyMakeError::from(format!("Error: Could not create symlink between {:?} and {:?}: {}", destination.as_ref(), source.as_ref(), err))),
    }
}


pub fn create_file(dir: &PathBuf, filename: &str) -> Result<File, MyMakeError> {
    let file = dir.join(filename);
    if file.is_file() {
        match std::fs::remove_file(&file) {
            Ok(()) => (),
            Err(err) => return Err(MyMakeError::from(format!("Error removing {:?}: {}", file, err))),
        };
    }
    let filename = File::create(&file)?;
    Ok(filename)
}

#[cfg(test)]
#[path = "./lib_test.rs"]
mod lib_test;