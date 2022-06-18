use crate::mmk_parser;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IncludeDirectories(Vec<IncludeDirectory>);

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct IncludeDirectory {
    pub include_type: IncludeType,
    pub path: std::path::PathBuf,
}

impl IncludeDirectories {
    pub fn from_mmk(mmk: &mmk_parser::Mmk) -> Option<Self> {
        if !mmk.has_dependencies() {
            return None;
        }
        let mut include_directories = vec![];
        for keyword in &mmk.data()["MMK_REQUIRE"] {
            let path_as_str = keyword.argument();
            let path = std::path::PathBuf::from(path_as_str);
            if keyword.option() == "SYSTEM" {
                include_directories.push(IncludeDirectory {
                    include_type: IncludeType::System,
                    path,
                });
            } else {
                include_directories.push(IncludeDirectory {
                    include_type: IncludeType::Include,
                    path,
                });
            }
        }
        Some(Self(include_directories))
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

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum IncludeType {
    Include,
    System,
}
