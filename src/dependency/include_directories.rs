use crate::mmk_parser;

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct IncludeDirectories(Vec<IncludeDirectory>);

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct IncludeDirectory {
    pub include_type: IncludeType,
    pub path: std::path::PathBuf,
}

impl IncludeDirectory {
    fn find(path: &std::path::Path) -> Option<std::path::PathBuf> {
        let include_path = path.join("include");
        if include_path.is_dir() {
            return Some(include_path);
        }
        while let Some(parent) = path.parent() {
            return Self::find(parent);
        }
        None
    }
}

impl IncludeDirectories {
    pub fn from_mmk(mmk: &mmk_parser::Mmk) -> Option<Self> {
        if !mmk.has_dependencies() && !mmk.has_system_include() {
            return None;
        }
        let mut include_directories = vec![];
        if let Some(requires) = mmk.get_args("MMK_REQUIRE") {
            for keyword in requires {
                let path_as_str = keyword.argument();
                let path = IncludeDirectory::find(&std::path::Path::new(path_as_str))?;
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
        }
        if let Some(sys_includes) = mmk.get_args("MMK_SYS_INCLUDE") {
            for keyword in sys_includes {
                let path_as_str = keyword.argument();
                let path = std::path::PathBuf::from(path_as_str);
                include_directories.push(IncludeDirectory {
                    include_type: IncludeType::System,
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

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum IncludeType {
    Include,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempdir::TempDir;

    use mmk_parser::Keyword;

    #[test]
    fn from_mmk_registers_include_directories_within_mmk_directory() {
        let dep_dir = TempDir::new("base_one").unwrap();
        let dep_include_path = dep_dir.path().join("include");
        std::fs::create_dir(&dep_include_path).unwrap();

        let sec_dep_dir = TempDir::new("base_two").unwrap();
        let sec_dep_include_path = sec_dep_dir.path().join("include");
        std::fs::create_dir(&sec_dep_include_path).unwrap();

        let third_dep_dir = TempDir::new("base_three").unwrap();
        let third_dep_include_path = third_dep_dir.path().join("include");
        std::fs::create_dir(&third_dep_include_path).unwrap();

        let mut mmk_file = mmk_parser::Mmk::new(&std::path::Path::new("some/path/to/lib.mmk"));
        mmk_file.data_mut().insert(
            "MMK_REQUIRE".to_string(),
            vec![
                Keyword::from(&dep_dir.path().display().to_string()),
                Keyword::from(&sec_dep_dir.path().display().to_string()),
                Keyword::from(&third_dep_dir.path().display().to_string()),
            ],
        );

        let expected = IncludeDirectories(vec![
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: sec_dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: third_dep_include_path,
            },
        ]);
        let actual = IncludeDirectories::from_mmk(&mmk_file).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn from_mmk_registers_include_directories_in_parent_of_mmk_directory() {
        let dep_dir = TempDir::new("base_one").unwrap();
        let dep_src_dir = dep_dir.path().join("src");
        std::fs::create_dir(&dep_src_dir).unwrap();
        let dep_include_path = dep_src_dir.join("include");
        std::fs::create_dir(&dep_include_path).unwrap();

        let sec_dep_dir = TempDir::new("base_two").unwrap();
        let sec_dep_src_dir = sec_dep_dir.path().join("src");
        std::fs::create_dir(&sec_dep_src_dir).unwrap();
        let sec_dep_include_path = sec_dep_src_dir.join("include");
        std::fs::create_dir(&sec_dep_include_path).unwrap();

        let third_dep_dir = TempDir::new("base_three").unwrap();
        let third_dep_src_dir = third_dep_dir.path().join("src");
        std::fs::create_dir(&third_dep_src_dir).unwrap();
        let third_dep_include_path = third_dep_src_dir.join("include");
        std::fs::create_dir(&third_dep_include_path).unwrap();

        let mut mmk_file = mmk_parser::Mmk::new(&std::path::Path::new("some/path/to/lib.mmk"));
        mmk_file.data_mut().insert(
            "MMK_REQUIRE".to_string(),
            vec![
                Keyword::from(&dep_src_dir.display().to_string()),
                Keyword::from(&sec_dep_src_dir.display().to_string()),
                Keyword::from(&third_dep_src_dir.display().to_string()),
            ],
        );

        let expected = IncludeDirectories(vec![
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: sec_dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: third_dep_include_path,
            },
        ]);
        let actual = IncludeDirectories::from_mmk(&mmk_file).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn from_mmk_registers_sys_includes() {
        let mut mmk_file = mmk_parser::Mmk::new(&std::path::Path::new("some/path/to/lib.mmk"));
        mmk_file.data_mut().insert(
            "MMK_SYS_INCLUDE".to_string(),
            vec![Keyword::from("/some/dependency/include")],
        );
        let expected = IncludeDirectories(vec![IncludeDirectory {
            include_type: IncludeType::System,
            path: std::path::PathBuf::from("/some/dependency/include"),
        }]);
        let actual = IncludeDirectories::from_mmk(&mmk_file).unwrap();
        assert_eq!(actual, expected);
    }
}
