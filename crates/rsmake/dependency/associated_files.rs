use crate::errors::AssociatedFileError;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AssociatedFiles(std::vec::Vec<SourceFile>);

impl AssociatedFiles {
    pub fn new() -> Self {
        Self { 0: Vec::new() }
    }
    pub fn push(&mut self, file: SourceFile) {
        self.0.push(file)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SourceFile {
    file_type: FileType,
    file: std::path::PathBuf,
}

impl SourceFile {
    pub fn new(file: &std::path::Path) -> Result<Self, AssociatedFileError> {
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
}

#[derive(Debug, PartialEq, Eq, Clone)]
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
}
