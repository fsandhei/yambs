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

pub struct BuildManager<'a> {
    top_dependency: Option<DependencyNode>,
    dep_registry: DependencyRegistry,
    generator: Box<&'a mut dyn GeneratorExecutor>,
    debug: bool,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'a> BuildManager<'a> {
    pub fn new(generator: &mut dyn GeneratorExecutor) -> BuildManager {
        BuildManager {
            top_dependency: None,
            dep_registry: DependencyRegistry::new(),
            generator: Box::new(generator),
            debug: false,
            make: Make::new(),
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

    pub fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }

    pub fn use_std(&mut self, version: &str) -> Result<(), BuildManagerError> {
        Ok(self.generator.as_mut().use_std(version)?)
    }

    pub fn debug(&mut self) {
        self.debug = true;
        self.generator.as_mut().debug();
    }

    pub fn release(&mut self) {
        self.generator.as_mut().release();
    }

    pub fn make(&self) -> &Make {
        &self.make
    }

    pub fn parse_and_register_dependencies(
        self: &mut Self,
        top_path: &std::path::Path,
    ) -> Result<(), BuildManagerError> {
        let file_content = utility::read_file(&top_path)?;
        let mut mmk_data = mmk_parser::Mmk::new(&top_path);
        mmk_data
            .parse(&file_content)
            .map_err(BuildManagerError::FailedToParse)?;

        let top_dependency = Dependency::from_path(&top_path, &mut self.dep_registry, &mmk_data)?;
        self.top_dependency = Some(top_dependency.clone());
        Ok(())
    }

    fn add_dependency_to_generator(&mut self, dependency: &DependencyNode) {
        self.generator.as_mut().set_dependency(dependency);
    }

    pub fn generate_makefiles(&mut self) -> Result<(), BuildManagerError> {
        if let Some(top_dependency) = self.top_dependency.clone() {
            self.add_dependency_to_generator(&top_dependency);
            return Ok(self
                .generator
                .as_mut()
                .generate_makefiles(&top_dependency)?);
        } else {
            return Err(BuildManagerError::UnexpectedCall(String::from(
                "builder.generate_builder()",
            )));
        }
    }

    pub fn number_of_dependencies(&self) -> usize {
        self.dep_registry.number_of_dependencies()
    }

    pub fn resolve_build_directory(&self, path: &std::path::Path) -> std::path::PathBuf {
        if self.debug {
            return path.join("debug");
        } else {
            return path.join("release");
        }
    }

    fn use_configuration(
        &mut self,
        configurations: &BuildConfigurations,
    ) -> Result<(), BuildManagerError> {
        for configuration in configurations {
            match configuration {
                Configuration::Debug => Ok(self.debug()),
                Configuration::Release => Ok(self.release()),
                Configuration::Sanitizer(sanitizer) => Ok(self.set_sanitizer(&sanitizer)),
                Configuration::CppVersion(version) => self.use_std(&version),
            }?;
        }
        Ok(())
    }

    fn set_sanitizer(&mut self, sanitizers: &str) {
        self.generator.as_mut().set_sanitizer(sanitizers);
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
