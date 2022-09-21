#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct AssociatedFiles(std::vec::Vec<SourceFile>);

impl AssociatedFiles {
    pub fn new() -> Self {
        Self { 0: Vec::new() }
    }

    pub fn from_paths(sources: &[std::path::PathBuf]) -> Result<Self, AssociatedFileError> {
        Ok(Self(
            sources
                .iter()
                .map(|source| {
                    log::debug!("Found source file {}", source.display());
                    SourceFile::new(&source)
                })
                .collect::<Result<Vec<SourceFile>, AssociatedFileError>>()?,
        ))
    }

    pub fn push(&mut self, file: SourceFile) {
        self.0.push(file)
    }

    pub fn iter(&self) -> std::slice::Iter<'_, SourceFile> {
        self.0.iter()
    }
}

impl std::iter::IntoIterator for AssociatedFiles {
    type Item = <std::vec::Vec<SourceFile> as IntoIterator>::Item;
    type IntoIter = <std::vec::Vec<SourceFile> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> std::iter::IntoIterator for &'a AssociatedFiles {
    type Item = <&'a std::vec::Vec<SourceFile> as IntoIterator>::Item;
    type IntoIter = <&'a std::vec::Vec<SourceFile> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AssociatedFileError {
    #[error("Could not specify file type")]
    CouldNotSpecifyFileType,
    #[error("Source file {0:?} does not exist")]
    FileNotExisting(std::path::PathBuf),
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceFile {
    file_type: FileType,
    file: std::path::PathBuf,
}

impl SourceFile {
    pub fn new(file: &std::path::Path) -> Result<Self, AssociatedFileError> {
        if !file.exists() {
            return Err(AssociatedFileError::FileNotExisting(file.to_path_buf()));
        }
        let file_type = match file.extension().and_then(|extension| extension.to_str()) {
            Some("cpp") | Some("cc") => FileType::Source,
            Some("h") | Some("hpp") => FileType::Header,
            Some(_) | None => return Err(AssociatedFileError::CouldNotSpecifyFileType),
        };

        Ok(Self {
            file_type,
            file: file.to_path_buf(),
        })
    }

    pub fn file(&self) -> std::path::PathBuf {
        self.file.clone()
    }

    pub fn is_source(&self) -> bool {
        self.file_type == FileType::Source
    }

    pub fn is_header(&self) -> bool {
        self.file_type == FileType::Header
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum FileType {
    Source,
    Header,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_file_is_source_file_type() {
        let file = "file.cpp";
        let expected = SourceFile {
            file_type: FileType::Source,
            file: std::path::PathBuf::from(file),
        };
        let actual = SourceFile::new(&std::path::PathBuf::from(file)).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn header_file_is_header_file_type() {
        let file = "file.h";
        let expected = SourceFile {
            file_type: FileType::Header,
            file: std::path::PathBuf::from(file),
        };
        let actual = SourceFile::new(&std::path::PathBuf::from(file)).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn fails_to_recognize_file_type() {
        let file = "file.py";
        let actual = SourceFile::new(&std::path::PathBuf::from(file));
        assert!(matches!(
            actual.unwrap_err(),
            AssociatedFileError::CouldNotSpecifyFileType
        ));
    }
}
