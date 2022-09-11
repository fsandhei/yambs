mod compiler_flags;
mod constants;
pub mod targets;

use either::Either;

pub struct ParsedRecipe {
    pub path: std::path::PathBuf,
    pub recipe: Recipe,
}

// FIXME: Write tests!
pub fn parse(toml_path: &std::path::Path) -> Result<ParsedRecipe, ParseTomlError> {
    let toml_content =
        String::from_utf8(std::fs::read(toml_path).map_err(ParseTomlError::FailedToRead)?)
            .map_err(ParseTomlError::FailedToConvertUtf8)?;
    Ok(ParsedRecipe {
        path: toml_path.to_path_buf(),
        recipe: parse_toml(&toml_content)?,
    })
}

fn parse_toml(toml: &str) -> Result<Recipe, ParseTomlError> {
    let recipe_contents =
        toml::from_str::<RawRecipe>(toml).map_err(ParseTomlError::FailedToParse)?;
    Ok(Recipe::from(recipe_contents))
}

#[derive(Debug, PartialEq, Eq)]
pub struct Recipe {
    pub targets: Vec<targets::Target>,
}

impl std::convert::From<RawRecipe> for Recipe {
    fn from(contents: RawRecipe) -> Self {
        let mut targets = Vec::<targets::Target>::new();
        let mut executables = {
            if let Some(executables) = contents.executables {
                executables
                    .into_iter()
                    .map(|(name, data)| ExecutableData::from_raw(name, data))
                    .map(|data| targets::Target(Either::Left(targets::Executable { data })))
                    .collect::<Vec<targets::Target>>()
            } else {
                Vec::new()
            }
        };
        let mut libraries = {
            if let Some(libraries) = contents.libraries {
                libraries
                    .into_iter()
                    .map(|(name, data)| LibraryData::from_raw(name, data))
                    .map(|data| targets::Target(either::Right(targets::Library { data })))
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
pub struct RawRecipe {
    #[serde(rename = "executable")]
    pub executables: Option<std::collections::HashMap<String, targets::RawExecutableData>>,
    #[serde(rename = "library")]
    pub libraries: Option<std::collections::HashMap<String, targets::RawLibraryData>>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct ExecutableData {
    pub name: String,
    #[serde(flatten)]
    pub data: targets::RawExecutableData,
}

impl ExecutableData {
    pub fn from_raw(name: String, data: targets::RawExecutableData) -> Self {
        Self { name, data }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct LibraryData {
    pub name: String,
    #[serde(flatten)]
    pub data: targets::RawLibraryData,
}

impl LibraryData {
    pub fn from_raw(name: String, data: targets::RawLibraryData) -> Self {
        Self { name, data }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RequiredProject {
    #[serde(flatten, with = "either::serde_untagged")]
    project: Either<std::path::PathBuf, String>,
    #[serde(default)]
    origin: ProjectOrigin,
}

impl RequiredProject {
    pub fn try_path(self) -> Option<std::path::PathBuf> {
        self.project.left()
    }

    pub fn try_name(self) -> Option<String> {
        self.project.right()
    }

    pub fn origin(&self) -> &ProjectOrigin {
        &self.origin
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub enum ProjectOrigin {
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
    use crate::parser::targets::RawLibraryData;

    use super::targets::{Executable, Library, RawCommonData, RawExecutableData, Target};
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
            let executable_data = ExecutableData {
                name: "x".to_string(),
                data: RawExecutableData {
                    common_raw: RawCommonData {
                        main: std::path::PathBuf::from("main.cpp"),
                        sources: vec![
                            std::path::PathBuf::from("x.cpp"),
                            std::path::PathBuf::from("y.cpp"),
                            std::path::PathBuf::from("z.cpp"),
                        ],
                        requires: None,
                        compiler_flags: None,
                    },
                },
            };
            let executable = Executable {
                data: executable_data,
            };
            let expected = Recipe {
                targets: vec![Target(Either::Left(executable))],
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
            let executable_data = ExecutableData {
                name: "x".to_string(),
                data: RawExecutableData {
                    common_raw: RawCommonData {
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
                                project: Either::Left(std::path::PathBuf::from(
                                    "SomeSecondProject",
                                )),
                                origin: ProjectOrigin::Include,
                            },
                        ]),
                        compiler_flags: None,
                    },
                },
            };
            let executable = Executable {
                data: executable_data,
            };
            let expected = Recipe {
                targets: vec![Target(Either::Left(executable))],
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
            let executable_data_x = ExecutableData {
                name: "x".to_string(),
                data: RawExecutableData {
                    common_raw: RawCommonData {
                        main: std::path::PathBuf::from("main.cpp"),
                        sources: vec![
                            std::path::PathBuf::from("x.cpp"),
                            std::path::PathBuf::from("y.cpp"),
                            std::path::PathBuf::from("z.cpp"),
                        ],
                        requires: None,
                        compiler_flags: None,
                    },
                },
            };
            let executable_data_y = ExecutableData {
                name: "y".to_string(),
                data: RawExecutableData {
                    common_raw: RawCommonData {
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
                                project: Either::Left(std::path::PathBuf::from(
                                    "SomeSecondProject",
                                )),
                                origin: ProjectOrigin::Include,
                            },
                        ]),
                        compiler_flags: None,
                    },
                },
            };
            let executable_x = Executable {
                data: executable_data_x,
            };
            let executable_y = Executable {
                data: executable_data_y,
            };
            let expected = Recipe {
                targets: vec![
                    Target(Either::Left(executable_y)),
                    Target(Either::Left(executable_x)),
                ],
            };
            assert_eq!(recipe, expected);
        }
    }

    #[test]
    fn parse_produces_recipe_with_libraries() {
        const TOML_RECIPE: &str = r#"
    [library.MyLibraryData]
    main = "generator.cpp"
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    "#;
        {
            let recipe = parse_toml(TOML_RECIPE).unwrap();
            let library_data = LibraryData {
                name: "MyLibraryData".to_string(),
                data: RawLibraryData {
                    common_raw: RawCommonData {
                        main: std::path::PathBuf::from("generator.cpp"),
                        sources: vec![
                            std::path::PathBuf::from("x.cpp"),
                            std::path::PathBuf::from("y.cpp"),
                            std::path::PathBuf::from("z.cpp"),
                        ],
                        requires: None,
                        compiler_flags: None,
                    },
                    lib_type: targets::LibraryType::default(),
                },
            };
            let library = Library { data: library_data };
            let expected = Recipe {
                targets: vec![Target(Either::Right(library))],
            };
            assert_eq!(recipe, expected);
        }
        const TOML_WITH_REQUIRE_RECIPE: &str = r#"
    [library.MyLibraryData]
    requires = ["SomeProject", "SomeSecondProject"]
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "generator.cpp"
    "#;
        {
            let recipe = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
            let library_data = LibraryData {
                name: "MyLibraryData".to_string(),
                data: RawLibraryData {
                    common_raw: RawCommonData {
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
                                project: Either::Left(std::path::PathBuf::from(
                                    "SomeSecondProject",
                                )),
                                origin: ProjectOrigin::Include,
                            },
                        ]),
                        compiler_flags: None,
                    },
                    lib_type: targets::LibraryType::default(),
                },
            };
            let library = Library { data: library_data };
            let expected = Recipe {
                targets: vec![Target(Either::Right(library))],
            };
            assert_eq!(recipe, expected);
        }
    }
}
