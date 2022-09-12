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
    pub targets: Vec<Either<targets::Executable, targets::Library>>,
}

impl std::convert::From<RawRecipe> for Recipe {
    fn from(contents: RawRecipe) -> Self {
        let mut targets = Vec::<Either<targets::Executable, targets::Library>>::new();
        let mut executables = {
            if let Some(executables) = contents.executables {
                executables
                    .into_iter()
                    .map(|(name, data)| {
                        Either::Left(targets::Executable {
                            name,
                            main: data.common_raw.main,
                            sources: data.common_raw.sources,
                            requires: data.common_raw.requires,
                            compiler_flags: data.common_raw.compiler_flags,
                        })
                    })
                    .collect::<Vec<Either<targets::Executable, targets::Library>>>()
            } else {
                Vec::new()
            }
        };
        let mut libraries = {
            if let Some(libraries) = contents.libraries {
                libraries
                    .into_iter()
                    .map(|(name, data)| {
                        either::Right(targets::Library {
                            name,
                            main: data.common_raw.main,
                            sources: data.common_raw.sources,
                            requires: data.common_raw.requires,
                            compiler_flags: data.common_raw.compiler_flags,
                            lib_type: data.lib_type,
                        })
                    })
                    .collect::<Vec<Either<targets::Executable, targets::Library>>>()
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

    use super::targets::{Executable, Library};
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
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Vec::new(),
                compiler_flags: None,
            };
            let expected = Recipe {
                targets: vec![Either::Left(executable)],
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
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ],
                compiler_flags: None,
            };
            let expected = Recipe {
                targets: vec![Either::Left(executable)],
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
                name: "x".to_string(),
                main: std::path::PathBuf::from("main.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Vec::new(),
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
                requires: vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ],
                compiler_flags: None,
            };
            let expected = Recipe {
                targets: vec![Either::Left(executable_y), Either::Left(executable_x)],
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
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: Vec::new(),
                compiler_flags: None,
                lib_type: targets::LibraryType::default(),
            };
            let expected = Recipe {
                targets: vec![Either::Right(library)],
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
            let library = Library {
                name: "MyLibraryData".to_string(),
                main: std::path::PathBuf::from("generator.cpp"),
                sources: vec![
                    std::path::PathBuf::from("x.cpp"),
                    std::path::PathBuf::from("y.cpp"),
                    std::path::PathBuf::from("z.cpp"),
                ],
                requires: vec![
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeProject")),
                        origin: ProjectOrigin::Include,
                    },
                    RequiredProject {
                        project: Either::Left(std::path::PathBuf::from("SomeSecondProject")),
                        origin: ProjectOrigin::Include,
                    },
                ],
                compiler_flags: None,
                lib_type: targets::LibraryType::default(),
            };
            let expected = Recipe {
                targets: vec![Either::Right(library)],
            };
            assert_eq!(recipe, expected);
        }
    }
}
