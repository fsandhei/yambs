use crate::targets;
use crate::YAMBS_MANIFEST_DIR_ENV;

mod constants;

pub struct ParsedManifest {
    pub path: std::path::PathBuf,
    pub data: ManifestData,
}

// FIXME: Write tests!
// FIXME: Vurdere variabel for filstier som settes av yambs for å hjelpe forkortelse av paths.
// Bruke relativ path, kanskje?
pub fn parse(toml_path: &std::path::Path) -> Result<ParsedManifest, ParseTomlError> {
    let toml_content =
        String::from_utf8(std::fs::read(toml_path).map_err(ParseTomlError::FailedToRead)?)
            .map_err(ParseTomlError::FailedToConvertUtf8)?;
    Ok(ParsedManifest {
        path: toml_path.to_path_buf(),
        data: parse_toml(&toml_content)?,
    })
}

fn parse_toml(toml: &str) -> Result<ManifestData, ParseTomlError> {
    let manifest_contents =
        toml::from_str::<RawManifestData>(toml).map_err(ParseTomlError::FailedToParse)?;
    Ok(ManifestData::from(manifest_contents))
}

#[derive(Debug, PartialEq, Eq)]
pub struct ManifestData {
    pub targets: Vec<targets::Target>,
}

fn canonicalize_source(path: &std::path::Path) -> std::path::PathBuf {
    std::env::var_os(YAMBS_MANIFEST_DIR_ENV)
        .map(std::path::PathBuf::from)
        .unwrap()
        .join(path)
}

impl std::convert::From<RawManifestData> for ManifestData {
    fn from(contents: RawManifestData) -> Self {
        let mut targets = Vec::<targets::Target>::new();
        let mut executables = {
            if let Some(executables) = contents.executables {
                executables
                    .into_iter()
                    .map(|(name, data)| {
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| {
                                let dependency = targets::Dependency::new(&name, data);
                                match dependency.data {
                                    targets::DependencyData::FromFilesystem {
                                        ref path,
                                        ref origin,
                                    } => {
                                        log::debug!(
                                            "Found dependency {} in path {} with origin {:?}",
                                            dependency.name,
                                            path.display(),
                                            origin
                                        );
                                    }
                                }
                                dependency
                            })
                            .collect::<Vec<targets::Dependency>>();
                        targets::Target::Executable(targets::Executable {
                            name,
                            main: canonicalize_source(&data.common_raw.main),
                            sources: data
                                .common_raw
                                .sources
                                .iter()
                                .map(|source| canonicalize_source(&source))
                                .collect::<Vec<std::path::PathBuf>>(),
                            dependencies,
                            compiler_flags: data.common_raw.compiler_flags,
                        })
                    })
                    .collect::<Vec<targets::Target>>()
            } else {
                Vec::new()
            }
        };
        let mut libraries = {
            if let Some(libraries) = contents.libraries {
                libraries
                    .into_iter()
                    .map(|(name, data)| {
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| {
                                let dependency = targets::Dependency::new(&name, data);
                                match dependency.data {
                                    targets::DependencyData::FromFilesystem {
                                        ref path,
                                        ref origin,
                                    } => {
                                        log::debug!(
                                            "Found dependency {} in path {} with origin {:?}",
                                            dependency.name,
                                            path.display(),
                                            origin
                                        );
                                    }
                                }
                                dependency
                            })
                            .collect::<Vec<targets::Dependency>>();
                        targets::Target::Library(targets::Library {
                            name,
                            main: canonicalize_source(&data.common_raw.main),
                            sources: data
                                .common_raw
                                .sources
                                .iter()
                                .map(|source| canonicalize_source(&source))
                                .collect::<Vec<std::path::PathBuf>>(),
                            dependencies,
                            compiler_flags: data.common_raw.compiler_flags,
                            lib_type: data.lib_type,
                        })
                    })
                    .collect::<Vec<targets::Target>>()
            } else {
                Vec::new()
            }
        };
        targets.append(&mut executables);
        targets.append(&mut libraries);
        Self { targets }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawManifestData {
    #[serde(rename = "executable")]
    pub executables: Option<std::collections::HashMap<String, targets::RawExecutableData>>,
    #[serde(rename = "library")]
    pub libraries: Option<std::collections::HashMap<String, targets::RawLibraryData>>,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseTomlError {
    #[error("Failed to parse TOML manifest file.")]
    FailedToParse(#[source] toml::de::Error),
    #[error("Failed to read TOML manifest file.")]
    FailedToRead(#[source] std::io::Error),
    #[error("Failed to convert UTF-8 bytes to string")]
    FailedToConvertUtf8(#[source] std::string::FromUtf8Error),
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::targets::{Dependency, DependencyData, DependencySource, Executable, Library};
    use crate::tests::EnvLock;

    #[test]
    fn parse_produces_manifest_with_executables() {
        let mut lock = EnvLock::new();
        lock.lock(YAMBS_MANIFEST_DIR_ENV, "");
        const TOML_RECIPE: &str = r#"
    [executable.x]
    main = "main.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let manifest = parse_toml(TOML_RECIPE).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![targets::Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
        const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    [executable.x]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "main.cpp"
    [executable.x.dependencies]
    SomeProject = { path = "/some/path/SomeProject" }
    SomeSecondProject = { path = "/some/path/SomeSecondProject" }
    "#;
        {
            let manifest = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
                    },
                ],
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![targets::Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_multiple_executables() {
        let mut lock = EnvLock::new();
        lock.lock(YAMBS_MANIFEST_DIR_ENV, "");
        let input = r#"
    [executable.x]
    main = "main.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']

    [executable.y]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "main.cpp"

    [executable.y.dependencies]
    SomeProject = { path = "/some/path/to/SomeProject" }
    SomeSecondProject = { path = "/some/path/to/SomeSecondProject" }
    "#;
        {
            let manifest = parse_toml(input).unwrap();
            let executable_x = Executable {
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
            };
            let executable_y = Executable {
                name: "y".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/to/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/to/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
                    },
                ],
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![
                    targets::Target::Executable(executable_x),
                    targets::Target::Executable(executable_y),
                ],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_libraries() {
        let mut lock = EnvLock::new();
        lock.lock(YAMBS_MANIFEST_DIR_ENV, "");
        const TOML_RECIPE: &str = r#"
    [library.MyLibraryData]
    main = "generator.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let manifest = parse_toml(TOML_RECIPE).unwrap();
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
                lib_type: targets::LibraryType::default(),
            };
            let expected = ManifestData {
                targets: vec![targets::Target::Library(library)],
            };
            assert_eq!(manifest, expected);
        }
        const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    [library.MyLibraryData]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "generator.cpp"

    [library.MyLibraryData.dependencies]
    SomeProject = { path = "/some/path/to/SomeProject" }
    SomeSecondProject = { path = "/some/path/to/SomeSecondProject" }
    "#;
        {
            let manifest = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/to/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::FromFilesystem {
                            path: std::path::PathBuf::from("/some/path/to/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
                    },
                ],
                compiler_flags: None,
                lib_type: targets::LibraryType::default(),
            };
            let expected = ManifestData {
                targets: vec![targets::Target::Library(library)],
            };
            assert_eq!(manifest, expected);
        }
    }
}
