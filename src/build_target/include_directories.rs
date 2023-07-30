use regex::Regex;

#[derive(Debug, thiserror::Error)]
pub enum IncludeDirectoriesError {
    #[error("Could not find any include directory located at {0}")]
    CouldNotFindIncludeDirectory(std::path::PathBuf),
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IncludeDirectories(Vec<IncludeDirectory>);

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct IncludeDirectory {
    pub include_type: IncludeType,
    pub path: std::path::PathBuf,
}

impl IncludeDirectory {
    pub fn from_str(s: &str) -> Option<Self> {
        lazy_static::lazy_static! {
            static ref INCLUDE_PATH_REGEX: Regex = Regex::new("(?P<type>(-I|-isystem))\\s*?(?P<path>.*)$").unwrap();
        }
        if let Some(captures) = INCLUDE_PATH_REGEX.captures(s) {
            let include_type = if let Some(ty) = captures.name("type") {
                match ty.as_str() {
                    "-I" => IncludeType::Include,
                    "-isystem" => IncludeType::System,
                    _ => IncludeType::Include,
                }
            } else {
                log::warn!("Could not determine any include flag, so defaulting to regular -I");
                IncludeType::Include
            };
            let path = captures.name("path").unwrap().as_str();
            let path = std::path::PathBuf::from(path);

            Some(Self { include_type, path })
        } else {
            None
        }
    }

    pub fn as_include_flag(&self) -> String {
        if self.include_type == IncludeType::System {
            format!("-isystem {}", self.path.display())
        } else {
            format!("-I{}", self.path.display())
        }
    }
}

impl IncludeDirectories {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn add(&mut self, include_directory: IncludeDirectory) {
        if !self.0.contains(&include_directory) {
            self.0.push(include_directory)
        }
    }

    pub fn iter(&self) -> std::slice::Iter<'_, IncludeDirectory> {
        self.0.iter()
    }
}

impl std::iter::IntoIterator for IncludeDirectories {
    type Item = IncludeDirectory;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> std::iter::IntoIterator for &'a IncludeDirectories {
    type Item = &'a IncludeDirectory;
    type IntoIter = std::slice::Iter<'a, IncludeDirectory>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum IncludeType {
    Include,
    System,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn from_str_parses_i_include() {
        let input = "-I/some/include/path";

        assert_eq!(
            IncludeDirectory::from_str(input),
            Some(IncludeDirectory {
                include_type: IncludeType::Include,
                path: PathBuf::from("/some/include/path"),
            })
        );
    }

    #[test]
    fn from_str_parses_isystem_include() {
        let input = "-isystem/some/include/path";

        assert_eq!(
            IncludeDirectory::from_str(input),
            Some(IncludeDirectory {
                include_type: IncludeType::System,
                path: PathBuf::from("/some/include/path"),
            })
        );
    }
}
