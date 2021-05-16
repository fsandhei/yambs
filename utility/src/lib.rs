use error::MyMakeError;
use std::fs::File;
use std::path::{PathBuf, Path};

// LEGG TIL TESTER FOR UTILITY.

// TODO: Vurder om denne skal returnere Result<PathBuf, MyMakeError>
pub fn get_source_directory_from_path(path: &PathBuf) -> PathBuf {
    if path.join("source").is_dir() {
        return path.join("source");
    }
    else if path.join("src").is_dir() {
        return path.join("src");
    }
    else {
        return path.to_path_buf();
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


pub fn is_source_directory(path: &Path) -> bool {
    (path.ends_with("source") || path.ends_with("src"))
    && path.is_dir()
}


pub fn is_test_directory(path: &Path) -> bool {
    path.ends_with("test")
}


pub fn get_head_directory(path: &Path) -> &Path {
    let part_to_strip = path.parent().unwrap();
    return path.strip_prefix(part_to_strip).unwrap()
}


pub fn directory_exists(path: &Path) -> bool {
    path.exists()
}


pub fn create_dir(dir: &PathBuf) -> Result<(), MyMakeError> {
    if !dir.is_dir() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(())
}


pub fn remove_dir(dir: &std::path::PathBuf) -> Result<(), MyMakeError> {
    if dir.is_dir() {
        std::fs::remove_dir_all(dir)?;
    }
    Ok(())
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
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
