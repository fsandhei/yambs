use crate::manifest;

mod constants;

// FIXME: Write tests!
// FIXME: Vurdere variabel for filstier som settes av yambs for å hjelpe forkortelse av paths.
// Bruke relativ path, kanskje?
pub fn parse(manifest_path: &std::path::Path) -> Result<manifest::ParsedManifest, ParseTomlError> {
    let toml_content =
        String::from_utf8(std::fs::read(&manifest_path).map_err(ParseTomlError::FailedToRead)?)
            .map_err(ParseTomlError::FailedToConvertUtf8)?;
    let metadata =
        std::fs::metadata(&manifest_path).expect("Could not fetch metadata from yambs.json");
    let manifest_directory = manifest_path.parent().unwrap();
    Ok(manifest::ParsedManifest {
        manifest: manifest::Manifest {
            directory: manifest_directory.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time of manifest"),
        },
        data: parse_toml(&toml_content, manifest_directory)?,
    })
}

fn parse_toml(
    toml: &str,
    manifest_dir: &std::path::Path,
) -> Result<manifest::ManifestData, ParseTomlError> {
    let manifest_contents =
        toml::from_str::<manifest::RawManifestData>(toml).map_err(ParseTomlError::FailedToParse)?;
    Ok(manifest::ManifestData::from_raw(
        manifest_contents,
        manifest_dir,
    ))
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
    use crate::manifest::ManifestData;
    use crate::targets::{
        Dependency, DependencyData, Executable, IncludeSearchType, Library, LibraryType, Target,
    };

    struct TestFixture {
        pub tempdir: tempdir::TempDir,
    }

    impl TestFixture {
        pub fn new() -> Self {
            Self {
                tempdir: tempdir::TempDir::new("parse").unwrap(),
            }
        }

        pub fn create_dummy_file(&self, path_postfix: &std::path::Path) -> std::path::PathBuf {
            let path = self.tempdir.path().join(path_postfix);
            std::fs::File::create(&path).unwrap();
            path
        }
    }

    #[test]
    #[ignore]
    fn parse_produces_manifest_with_executables() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("main.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        const TOML_RECIPE: &str = r#"
    [executable.x]
    main = "main.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let manifest = parse_toml(TOML_RECIPE, &manifest_dir).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                main: manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![Target::Executable(executable)],
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
            let manifest = parse_toml(TOML_WITH_REQUIRE_RECIPE, &manifest_dir).unwrap();
            let executable = Executable {
                name: "x".to_string(),
                main: manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                ],
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData::Source {
                            path: std::path::PathBuf::from("/some/path/SomeProject"),
                            origin: IncludeSearchType::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::Source {
                            path: std::path::PathBuf::from("/some/path/SomeSecondProject"),
                            origin: IncludeSearchType::Include,
                        },
                    },
                ],
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![Target::Executable(executable)],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    #[ignore]
    fn parse_produces_manifest_with_multiple_executables() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("main.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

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
            let manifest = parse_toml(input, &manifest_dir).unwrap();
            let executable_x = Executable {
                name: "x".to_string(),
                main: manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
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
                        data: DependencyData::Source {
                            path: std::path::PathBuf::from("/some/path/to/SomeProject"),
                            origin: IncludeSearchType::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::Source {
                            path: std::path::PathBuf::from("/some/path/to/SomeSecondProject"),
                            origin: IncludeSearchType::Include,
                        },
                    },
                ],
                compiler_flags: None,
            };
            let expected = ManifestData {
                targets: vec![
                    Target::Executable(executable_x),
                    Target::Executable(executable_y),
                ],
            };
            assert_eq!(manifest, expected);
        }
    }

    #[test]
    fn parse_produces_manifest_with_libraries() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.tempdir.path().to_path_buf();

        fixture.create_dummy_file(&std::path::PathBuf::from("generator.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("x.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("y.cpp"));
        fixture.create_dummy_file(&std::path::PathBuf::from("z.cpp"));

        const TOML_RECIPE: &str = r#"
    [library.MyLibraryData]
    main = "generator.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let manifest = parse_toml(TOML_RECIPE, &manifest_dir).unwrap();
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: manifest_dir.join(std::path::PathBuf::from("generator.cpp")),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
                lib_type: LibraryType::default(),
            };
            let expected = ManifestData {
                targets: vec![Target::Library(library)],
            };
            assert_eq!(manifest, expected);
        }
        let dep_project_path = fixture.create_dummy_file(&std::path::PathBuf::from("SomeProject"));
        let second_dep_project_path =
            fixture.create_dummy_file(&std::path::PathBuf::from("SomeSecondProject"));
        let toml_with_require_recipe = format!(
            r#"
    [library.MyLibraryData]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "generator.cpp"

    [library.MyLibraryData.dependencies]
    SomeProject = {{ path = "{}" }}
    SomeSecondProject = {{ path = "{}" }}
    "#,
            dep_project_path.display(),
            second_dep_project_path.display()
        );
        {
            let manifest = parse_toml(&toml_with_require_recipe, &manifest_dir).unwrap();
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: manifest_dir.join(std::path::PathBuf::from("generator.cpp")),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                ],
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData::Source {
                            path: dep_project_path,
                            origin: IncludeSearchType::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData::Source {
                            path: second_dep_project_path,
                            origin: IncludeSearchType::Include,
                        },
                    },
                ],
                compiler_flags: None,
                lib_type: LibraryType::default(),
            };
            let expected = ManifestData {
                targets: vec![Target::Library(library)],
            };
            assert_eq!(manifest, expected);
        }
    }
}
