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
mod tests {
    use tempdir::TempDir;
    use super::*;

    #[test]
    fn get_source_directory_from_path_test() {
        let dir = TempDir::new("example").unwrap();
        let source_dir = dir.path().join("src");
        create_dir(&source_dir).unwrap();
        assert_eq!(get_source_directory_from_path(dir.path()), source_dir);
    }


    #[test]
    fn get_source_directory_from_path_no_source_directory_defaults_to_original_path_test() {
        let dir = TempDir::new("example").unwrap();
        assert_eq!(get_source_directory_from_path(dir.path()), dir.path());
    }


    #[test]
    fn get_include_directory_from_path_test() {
        let dir = TempDir::new("example").unwrap();
        let include_dir = dir.path().join("include");
        create_dir(&include_dir).unwrap();
        let actual = get_include_directory_from_path(dir.path());
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), include_dir);
    }


    #[test]
    fn get_include_directory_from_path_search_one_directory_up_test() {
        let dir = TempDir::new("example").unwrap();
        let include_dir = dir.path().join("include");
        create_dir(&include_dir).unwrap();
        let actual = get_include_directory_from_path(dir.path().join("src"));
        assert!(actual.is_ok());
        assert_eq!(actual.unwrap(), include_dir);
    }


    #[test]
    fn get_include_directory_from_path_fails_test() {
        let dir = TempDir::new("example").unwrap();
        let result = get_include_directory_from_path(dir.path());
        assert!(result.is_err());
    }


    #[test]
    fn is_source_directory_src_test() {
        let dir = TempDir::new("example").unwrap();
        let source_dir = dir.path().join("src");
        create_dir(&source_dir).unwrap();
        assert_eq!(is_source_directory(source_dir), true);
    }


    #[test]
    fn is_source_directory_source_test() {
        let dir = TempDir::new("example").unwrap();
        let source_dir = dir.path().join("source");
        create_dir(&source_dir).unwrap();
        assert_eq!(is_source_directory(source_dir), true);
    }


    #[test]
    fn is_source_directory_false_test() {
        let source_dir = PathBuf::from("/some/path/without/source/directory");
        assert_eq!(is_source_directory(source_dir), false);
    }


    #[test]
    fn is_test_directory_true_test() {
        let dir = TempDir::new("example").unwrap();
        let test_dir = dir.path().join("test");
        create_dir(&test_dir).unwrap();
        assert_eq!(is_test_directory(test_dir), true);
    }


    #[test]
    fn is_test_directory_false_test() {
        let test_dir = PathBuf::from("/some/path/without/test/directory");
        assert_eq!(is_test_directory(test_dir), false);
    }


    #[test]
    fn get_head_directory_gets_head_test() {
        let dir = PathBuf::from("some/path/to/strip/head");
        let expected = PathBuf::from("head");
        assert_eq!(get_head_directory(&dir), &expected);
    }
}
