// use crate::parser;
use crate::targets;

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
}

impl IncludeDirectories {
    pub fn from_dependencies(dependencies: &[targets::Dependency]) -> Self {
        let include_directories = dependencies
            .iter()
            .filter_map(|dependency| {
                let (path, origin) = dependency.data.source()?;
                let include_path = IncludeDirectory::find(&path)?;

                Some(match origin {
                    targets::IncludeSearchType::Include => IncludeDirectory {
                        include_type: IncludeType::Include,
                        path: include_path,
                    },
                    targets::IncludeSearchType::System => IncludeDirectory {
                        include_type: IncludeType::System,
                        path: include_path,
                    },
                })
            })
            .collect::<Vec<IncludeDirectory>>();
        Self(include_directories)
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
        _tempdir: TempDir,
        dep_include_path: std::path::PathBuf,
        dependency: targets::Dependency,
    }

    impl DependencyStub {
        fn create_include_dir(tempdir_name: &str, dependency_name: &str) -> Self {
            let dep_dir = TempDir::new(tempdir_name).unwrap();
            let dep_include_path = dep_dir.path().join("include");
            std::fs::create_dir(&dep_include_path).unwrap();

            let dependency = targets::Dependency {
                name: dependency_name.to_string(),
                data: targets::DependencyData::Source {
                    path: dep_dir.path().to_path_buf(),
                    origin: targets::IncludeSearchType::Include,
                },
            };
            Self {
                _tempdir: dep_dir,
                dep_include_path,
                dependency,
            }
        }
    }

    #[test]
    fn from_mmk_registers_include_directories_within_mmk_directory() {
        let stub_one = DependencyStub::create_include_dir("base_one", "DependencyOne");
        let stub_two = DependencyStub::create_include_dir("base_two", "DependencyTwo");
        let stub_three = DependencyStub::create_include_dir("base_three", "DependencyThree");

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
        let stub_one = DependencyStub::create_include_dir("base_one", "DependencyOne");
        let mut stub_two = DependencyStub::create_include_dir("base_two", "DependencyTwo");
        stub_two.dependency.data = targets::DependencyData::Source {
            path: stub_two.dependency.data.source().unwrap().0,
            origin: targets::IncludeSearchType::System,
        };
        let stub_three = DependencyStub::create_include_dir("base_three", "DependencyThree");
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
}
