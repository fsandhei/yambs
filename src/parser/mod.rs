use either::Either;

use crate::targets;

mod constants;

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
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| targets::Dependency::new(&name, data))
                            .collect::<Vec<targets::Dependency>>();
                        Either::Left(targets::Executable {
                            name,
                            main: data.common_raw.main,
                            sources: data.common_raw.sources,
                            dependencies,
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
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| targets::Dependency::new(&name, data))
                            .collect::<Vec<targets::Dependency>>();
                        either::Right(targets::Library {
                            name,
                            main: data.common_raw.main,
                            sources: data.common_raw.sources,
                            dependencies,
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

    use super::targets::{Dependency, DependencyData, DependencySource, Executable, Library};
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
                dependencies: Vec::new(),
                compiler_flags: None,
            };
            let expected = Recipe {
                targets: vec![Either::Left(executable)],
            };
            assert_eq!(recipe, expected);
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
            let recipe = parse_toml(TOML_WITH_REQUIRE_RECIPE).unwrap();
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
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
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
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "main.cpp"

    [executable.y.dependencies]
    SomeProject = { path = "/some/path/to/SomeProject" }
    SomeSecondProject = { path = "/some/path/to/SomeSecondProject" }
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
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/to/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/to/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
                    },
                ],
                compiler_flags: None,
            };
            let expected = Recipe {
                targets: vec![Either::Left(executable_x), Either::Left(executable_y)],
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
                dependencies: Vec::new(),
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
    sources = ['x.cpp', 'y.cpp', 'z.cpp']
    main = "generator.cpp"

    [library.MyLibraryData.dependencies]
    SomeProject = { path = "/some/path/to/SomeProject" }
    SomeSecondProject = { path = "/some/path/to/SomeSecondProject" }
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
                dependencies: vec![
                    Dependency {
                        name: "SomeProject".to_string(),
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/to/SomeProject"),
                            origin: DependencySource::Include,
                        },
                    },
                    Dependency {
                        name: "SomeSecondProject".to_string(),
                        data: DependencyData {
                            path: std::path::PathBuf::from("/some/path/to/SomeSecondProject"),
                            origin: DependencySource::Include,
                        },
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
