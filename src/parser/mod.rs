mod constants;
mod keywords;

use either::Either;

use keywords::Keywords;

// FIXME: Write tests!
pub fn parse(toml_path: &std::path::Path) -> Result<Recipe, ParseTomlError> {
    // let toml_fh = std::fs::File::open(toml_path).map_err(ParseTomlError::FailedToOpen)?;
    let toml_content =
        String::from_utf8(std::fs::read(toml_path).map_err(ParseTomlError::FailedToRead)?)
            .map_err(ParseTomlError::FailedToConvertUtf8)?;
    parse_toml(&toml_content)
}

fn parse_toml(toml: &str) -> Result<Recipe, ParseTomlError> {
    toml::from_str(toml).map_err(ParseTomlError::FailedToParse)
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Recipe {
    executable: Option<std::collections::HashMap<String, Executable>>,
    library: Option<std::collections::HashMap<String, Library>>,
    #[serde(flatten)]
    keywords: Option<Keywords>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
struct Executable {
    main: std::path::PathBuf,
    sources: Vec<std::path::PathBuf>,
    requires: Option<Vec<RequiredProject>>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
struct Library {
    main: std::path::PathBuf,
    sources: Vec<std::path::PathBuf>,
    requires: Option<Vec<RequiredProject>>,
    #[serde(default)]
    lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
enum LibraryType {
    Static,
    Dynamic,
}

impl Default for LibraryType {
    fn default() -> Self {
        LibraryType::Static
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
struct RequiredProject {
    #[serde(flatten, with = "either::serde_untagged")]
    project: Either<std::path::PathBuf, String>,
    #[serde(default)]
    origin: ProjectOrigin,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
enum ProjectOrigin {
    System,
    Include,
}

impl Default for ProjectOrigin {
    fn default() -> Self {
        ProjectOrigin::Include
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ParseTomlError {
    #[error("Failed to parse TOML recipe file.")]
    FailedToParse(#[source] toml::de::Error),
    #[error("Failed to read TOML recipe file.")]
    FailedToRead(#[source] std::io::Error),
    #[error("Failed to convert UTF-8 bytes to string")]
    FailedToConvertUtf8(#[source] std::string::FromUtf8Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_produces_recipe_with_executables() {
        const TOML_RECIPE: &str = r#"
    [executable.x]
    main = "main.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let recipe = parse_toml(TOML_RECIPE).unwrap();
            let executable = Executable {
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: None,
            };
            let expected = Recipe {
                executable: Some(std::collections::HashMap::from([(
                    "x".to_string(),
                    executable,
                )])),
                library: None,
                keywords: None,
            };
            assert_eq!(recipe, expected);
        }
        const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    [executable.x]
    requires = ["SomeProject", "SomeSecondProject"]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "main.cpp"
    "#;
        {
            let recipe = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let executable = Executable {
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Some(vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ]),
            };
            let expected = Recipe {
                executable: Some(std::collections::HashMap::from([(
                    "x".to_string(),
                    executable,
                )])),
                library: None,
                keywords: None,
            };
            assert_eq!(recipe, expected);
        }
    }

    #[test]
    fn parse_produces_recipe_with_multiple_executables() {
        let input = r#"
    [executable.x]
    main = "main.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']

    [executable.y]
    requires = ["SomeProject", "SomeSecondProject"]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "main.cpp"
    "#;
        {
            let recipe = parse_toml(input).unwrap();
            let executable_x = Executable {
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: None,
            };
            let executable_y = Executable {
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Some(vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ]),
            };
            let expected = Recipe {
                executable: Some(std::collections::HashMap::from([
                    ("x".to_string(), executable_x),
                    ("y".to_string(), executable_y),
                ])),
                library: None,
                keywords: None,
            };
            assert_eq!(recipe, expected);
        }
    }

    #[test]
    fn parse_produces_recipe_with_libraries() {
        const TOML_RECIPE: &str = r#"
    [library.MyLibrary]
    main = "generator.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let recipe = parse_toml(TOML_RECIPE).unwrap();
            let library = Library {
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: None,
                lib_type: LibraryType::Static,
            };
            let expected = Recipe {
                library: Some(std::collections::HashMap::from([(
                    "MyLibrary".to_string(),
                    library,
                )])),
                executable: None,
                keywords: None,
            };
            assert_eq!(recipe, expected);
        }
        const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    [library.MyLibrary]
    requires = ["SomeProject", "SomeSecondProject"]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "generator.cpp"
    "#;
        {
            let recipe = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let library = Library {
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Some(vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ]),
                lib_type: LibraryType::Static,
            };
            let expected = Recipe {
                library: Some(std::collections::HashMap::from([(
                    "MyLibrary".to_string(),
                    library,
                )])),
                executable: None,
                keywords: None,
            };
            assert_eq!(recipe, expected);
        }
    }
}
