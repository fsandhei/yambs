use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::cli::command_line::CommandLine;
use crate::dependency::{Dependency, DependencyNode, DependencyRegistry};
use crate::errors::BuildManagerError;
use crate::generator::GeneratorExecutor;
use crate::mmk_parser;
use crate::utility;

mod filter;
mod make;
use make::Make;

enum BuildConfiguration {
    Debug,
    Release,
}

pub struct BuildManager<'a> {
    top_dependency: Option<DependencyNode>,
    generator: &'a mut dyn GeneratorExecutor,
    configuration: BuildConfiguration,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'gen> BuildManager<'gen> {
    pub fn new(generator: &'gen mut dyn GeneratorExecutor) -> BuildManager {
        BuildManager {
            top_dependency: None,
            generator,
            configuration: BuildConfiguration::Release,
            make: Make::default(),
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn top_dependency(&self) -> Option<&DependencyNode> {
        self.top_dependency.as_ref()
    }

    pub fn configure(&mut self, command_line: &CommandLine) -> Result<(), BuildManagerError> {
        self.add_make("-j", &command_line.jobs.to_string());
        self.top_build_directory = command_line.build_directory.to_owned();

        self.use_configuration(&command_line.configuration)?;

        Ok(())
    }

    pub fn make(&self) -> &Make {
        &self.make
    }

    pub fn parse_and_register_dependencies(
        &mut self,
        dep_registry: &mut DependencyRegistry,
        top_path: &std::path::Path,
    ) -> Result<(), BuildManagerError> {
        let file_content = utility::read_file(top_path)?;
        let mut mmk_data = mmk_parser::Mmk::new(top_path);
        mmk_data
            .parse(&file_content)
            .map_err(BuildManagerError::FailedToParse)?;

        let top_dependency = Dependency::from_path(top_path, dep_registry, &mmk_data)?;
        self.top_dependency = Some(top_dependency);
        Ok(())
    }

    fn add_dependency_to_generator(&mut self, dependency: &DependencyNode) {
        self.generator.set_dependency(dependency);
    }

    pub fn generate_makefiles(&mut self) -> Result<(), BuildManagerError> {
        if let Some(top_dependency) = self.top_dependency.clone() {
            self.add_dependency_to_generator(&top_dependency);
            Ok(self.generator.generate_makefiles(&top_dependency)?)
        } else {
            Err(BuildManagerError::UnexpectedCall(String::from(
                "builder.generate_builder()",
            )))
        }
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
