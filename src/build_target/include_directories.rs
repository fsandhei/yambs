use crate::parser::types;
use crate::targets;

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
    fn find(path: &std::path::Path) -> Option<std::path::PathBuf> {
        let include_path = path.join("include");
        if include_path.is_dir() {
            log::debug!("Found include directory {:?}", include_path.display());
            return Some(include_path);
        }
        if let Some(parent) = path.parent() {
            return Self::find(parent);
        }
        None
    }

    pub fn from_dependency(dependency: &targets::Dependency) -> Option<Self> {
        let include_directory = match dependency.data {
            types::DependencyData::Source(ref source_data) => {
                let include_path = IncludeDirectory::find(&source_data.path)?;

                match source_data.origin {
                    types::IncludeSearchType::Include => IncludeDirectory {
                        include_type: IncludeType::Include,
                        path: include_path,
                    },
                    types::IncludeSearchType::System => IncludeDirectory {
                        include_type: IncludeType::System,
                        path: include_path,
                    },
                }
            }
            types::DependencyData::Binary(ref binary_data) => {
                let include_path = binary_data.include_directory.clone();
                match binary_data.search_type {
                    types::IncludeSearchType::Include => IncludeDirectory {
                        include_type: IncludeType::Include,
                        path: include_path,
                    },
                    types::IncludeSearchType::System => IncludeDirectory {
                        include_type: IncludeType::System,
                        path: include_path,
                    },
                }
            }
            types::DependencyData::HeaderOnly(ref header_only_data) => IncludeDirectory {
                include_type: IncludeType::System,
                path: header_only_data.include_directory.to_path_buf(),
            },
        };
        Some(include_directory)
    }
}

impl IncludeDirectories {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_dependencies(
        dependencies: &[targets::Dependency],
    ) -> Result<Self, IncludeDirectoriesError> {
        let mut include_directories = Vec::new();
        for dependency in dependencies {
            if let Some(include_directory) = IncludeDirectory::from_dependency(dependency) {
                include_directories.push(include_directory);
            }
        }
        include_directories.dedup_by(|path_a, path_b| path_a == path_b);
        Ok(Self(include_directories))
    }

    pub fn add(&mut self, include_directory: IncludeDirectory) {
        self.0.push(include_directory)
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

    struct DependencyStub {
        dep_include_path: std::path::PathBuf,
        dependency: targets::Dependency,
    }

    impl DependencyStub {
        fn create_include_dir(base_dir: &std::path::Path, dependency_name: &str) -> Self {
            let dep_include_path = base_dir.join("include");
            if !dep_include_path.exists() {
                std::fs::create_dir(&dep_include_path).unwrap();
            }

            let dependency = targets::Dependency {
                name: dependency_name.to_string(),
                data: types::DependencyData::Source(types::SourceData {
                    path: base_dir.to_path_buf(),
                    origin: types::IncludeSearchType::Include,
                }),
            };
            Self {
                dep_include_path,
                dependency,
            }
        }
    }

    #[test]
    fn from_mmk_registers_include_directories_within_mmk_directory() {
        let tempdir_stub_one = TempDir::new("base_one").unwrap();
        let stub_one = DependencyStub::create_include_dir(tempdir_stub_one.path(), "DependencyOne");
        let tempdir_stub_two = TempDir::new("base_two").unwrap();
        let stub_two = DependencyStub::create_include_dir(tempdir_stub_two.path(), "DependencyTwo");
        let tempdir_stub_three = TempDir::new("base_three").unwrap();
        let stub_three =
            DependencyStub::create_include_dir(tempdir_stub_three.path(), "DependencyThree");

        let expected = IncludeDirectories(vec![
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: stub_one.dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: stub_two.dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: stub_three.dep_include_path,
            },
        ]);
        let actual = IncludeDirectories::from_dependencies(&[
            stub_one.dependency,
            stub_two.dependency,
            stub_three.dependency,
        ])
        .unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn from_mmk_registers_include_directories_third_party() {
        let tempdir_stub_one = TempDir::new("base_one").unwrap();
        let stub_one = DependencyStub::create_include_dir(tempdir_stub_one.path(), "DependencyOne");
        let tempdir_stub_two = TempDir::new("base_two").unwrap();
        let mut stub_two =
            DependencyStub::create_include_dir(tempdir_stub_two.path(), "DependencyTwo");
        match stub_two.dependency.data {
            types::DependencyData::Source(ref mut source_data) => {
                source_data.origin = types::IncludeSearchType::System;
            }
            _ => {}
        };
        let tempdir_stub_three = TempDir::new("base_three").unwrap();
        let stub_three =
            DependencyStub::create_include_dir(tempdir_stub_three.path(), "DependencyThree");
        let expected = IncludeDirectories(vec![
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: stub_one.dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::System,
                path: stub_two.dep_include_path,
            },
            IncludeDirectory {
                include_type: IncludeType::Include,
                path: stub_three.dep_include_path,
            },
        ]);

        let actual = IncludeDirectories::from_dependencies(&[
            stub_one.dependency,
            stub_two.dependency,
            stub_three.dependency,
        ])
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn from_mmk_registers_include_directories_with_duplicates() {
        let tempdir_stub = TempDir::new("base_one").unwrap();
        let stub_one = DependencyStub::create_include_dir(tempdir_stub.path(), "DependencyOne");
        let stub_two = DependencyStub::create_include_dir(tempdir_stub.path(), "DependencyTwo");
        let stub_three = DependencyStub::create_include_dir(tempdir_stub.path(), "DependencyThree");
        let expected = IncludeDirectories(vec![IncludeDirectory {
            include_type: IncludeType::Include,
            path: stub_one.dep_include_path,
        }]);

        let actual = IncludeDirectories::from_dependencies(&[
            stub_one.dependency,
            stub_two.dependency,
            stub_three.dependency,
        ])
        .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn no_include_directory_does_not_cause_an_error() {
        let tempdir_stub = TempDir::new("base_one").unwrap();
        let stub_one = DependencyStub::create_include_dir(tempdir_stub.path(), "DependencyOne");
        std::fs::remove_dir(stub_one.dep_include_path).unwrap();
        let expected = IncludeDirectories(vec![]);

        let actual = IncludeDirectories::from_dependencies(&[stub_one.dependency]).unwrap();
        assert_eq!(actual, expected);
    }
}
