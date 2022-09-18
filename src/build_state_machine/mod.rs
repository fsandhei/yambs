use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::cli::command_line::BuildOpts;
use crate::dependency::target::{target_registry::TargetRegistry, Target, TargetError, TargetNode};
use crate::errors::FsError;
use crate::generator::{GeneratorError, GeneratorExecutor};
use crate::parser;

mod filter;
mod make;
use make::Make;

enum BuildConfiguration {
    Debug,
    Release,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BuildManagerError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    #[error("Failed to parse YAMBS Recipe file")]
    FailedToParse(#[source] parser::ParseTomlError),
    #[error("{0}: called in an unexpected way.")]
    UnexpectedCall(String),
    #[error(transparent)]
    Fs(#[from] FsError),
}

pub struct BuildManager<'a> {
    targets: Vec<TargetNode>,
    generator: &'a mut dyn GeneratorExecutor,
    configuration: BuildConfiguration,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'gen> BuildManager<'gen> {
    pub fn new(generator: &'gen mut dyn GeneratorExecutor) -> BuildManager {
        BuildManager {
            targets: Vec::new(),
            generator,
            configuration: BuildConfiguration::Release,
            make: Make::default(),
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn targets(&self) -> &Vec<TargetNode> {
        self.targets.as_ref()
    }

    pub fn configure(&mut self, opts: &BuildOpts) -> Result<(), BuildManagerError> {
        self.add_make("-j", &opts.jobs.to_string());
        self.top_build_directory = opts.build_directory.to_owned();

        self.use_configuration(&opts.configuration)?;

        Ok(())
    }

    pub fn make(&self) -> &Make {
        &self.make
    }

    pub fn parse_and_register_dependencies(
        &mut self,
        dep_registry: &mut TargetRegistry,
        recipe_path: &std::path::Path,
    ) -> Result<(), BuildManagerError> {
        let recipe = parser::parse(recipe_path).map_err(BuildManagerError::FailedToParse)?;

        for build_target in recipe.recipe.targets {
            let target = Target::create(recipe_path, &build_target, dep_registry)
                .map_err(BuildManagerError::Target)?;
            self.targets.push(target);
        }
        Ok(())
    }

    pub fn generate_makefiles(&mut self) -> Result<(), BuildManagerError> {
        for target in &self.targets {
            self.generator.set_target(target);
            self.generator.generate_makefiles(&target)?;
        }
        Ok(())
    }

    pub fn resolve_build_directory(&self, path: &std::path::Path) -> std::path::PathBuf {
        match self.configuration {
            BuildConfiguration::Debug => path.join("debug"),
            BuildConfiguration::Release => path.join("release"),
        }
    }

    fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }

    fn use_std(&mut self, version: &str) -> Result<(), BuildManagerError> {
        Ok(self.generator.use_std(version)?)
    }

    fn debug(&mut self) {
        self.generator.debug();
    }

    fn release(&mut self) {
        self.generator.release();
    }

    fn use_configuration(
        &mut self,
        configurations: &BuildConfigurations,
    ) -> Result<(), BuildManagerError> {
        for configuration in configurations {
            match configuration {
                Configuration::Debug => {
                    self.configuration = BuildConfiguration::Debug;
                    self.debug();
                    Ok(())
                }
                Configuration::Release => {
                    self.release();
                    Ok(())
                }
                Configuration::Sanitizer(sanitizer) => {
                    self.set_sanitizer(sanitizer);
                    Ok(())
                }
                Configuration::CppVersion(version) => self.use_std(version),
            }?;
        }
        Ok(())
    }

    fn set_sanitizer(&mut self, sanitizers: &str) {
        self.generator.set_sanitizer(sanitizers);
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
