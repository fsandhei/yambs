use std::path::PathBuf;
use std::process::Output;
use std::rc::Rc;

use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::cli::command_line::CommandLine;
use crate::dependency::{Dependency, DependencyNode, DependencyRegistry};
use crate::errors::BuilderError;
use crate::generator::GeneratorExecutor;
use colored::Colorize;

mod filter;
mod make;
use make::Make;

pub struct Builder<'a> {
    top_dependency: Option<DependencyNode>,
    dep_registry: DependencyRegistry,
    generator: Box<&'a mut dyn GeneratorExecutor>,
    debug: bool,
    verbose: bool,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'a> Builder<'a> {
    pub fn new(generator: &mut dyn GeneratorExecutor) -> Builder {
        Builder {
            top_dependency: None,
            dep_registry: DependencyRegistry::new(),
            generator: Box::new(generator),
            debug: false,
            verbose: false,
            make: Make::new(),
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn configure(&mut self, command_line: &CommandLine) -> Result<(), BuilderError> {
        if command_line.verbose {
            self.set_verbose(true);
        }
        self.add_make("-j", &command_line.jobs.to_string());
        self.top_build_directory = command_line.build_directory.to_owned();

        self.use_configuration(&command_line.configuration)?;

        Ok(())
    }

    pub fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }

    pub fn use_std(&mut self, version: &str) -> Result<(), BuilderError> {
        Ok(self.generator.as_mut().use_std(version)?)
    }

    pub fn debug(&mut self) {
        self.debug = true;
        self.generator.as_mut().debug();
    }

    pub fn release(&mut self) {
        self.generator.as_mut().release();
    }

    pub fn set_verbose(&mut self, value: bool) {
        self.verbose = value;
    }

    pub fn create_log_file(&mut self) -> Result<(), BuilderError> {
        if let Some(top_dependency) = &self.top_dependency {
            if top_dependency.borrow().is_makefile_made() {
                let log_file_name = self.top_build_directory.as_path().join("rsmake_log.txt");
                self.make.add_logger(&log_file_name)?;
            }
        }
        Ok(())
    }

    pub fn read_mmk_files_from_path(
        self: &mut Self,
        top_path: &std::path::PathBuf,
    ) -> Result<(), BuilderError> {
        let top_dependency =
            Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = Some(Rc::clone(&top_dependency));
        Ok(())
    }

    fn add_dependency_to_generator(&mut self, dependency: &DependencyNode) {
        self.generator.as_mut().set_dependency(dependency);
    }

    pub fn generate_makefiles(&mut self) -> Result<(), BuilderError> {
        if let Some(top_dependency) = self.top_dependency.clone() {
            self.add_dependency_to_generator(&top_dependency);
            return Ok(self
                .generator
                .as_mut()
                .generate_makefiles(&top_dependency)?);
        } else {
            return Err(BuilderError::UnexpectedCall(String::from(
                "builder.generate_builder()",
            )));
        }
    }

    pub fn build_project(&mut self) -> Result<(), BuilderError> {
        self.create_log_file()?;
        if let Some(top_dependency) = &self.top_dependency {
            let output = self.build_dependency(
                &top_dependency,
                &self.top_build_directory.as_path(),
                self.verbose,
            );
            let build_status_message: String;
            if output.is_ok() && output.unwrap().status.success() {
                build_status_message = format!("rsmake: {}", "Build SUCCESS".green());
            } else {
                build_status_message = format!("rsmake: {}", "Build FAILED".red());
            }
            println!("{}", build_status_message);
            self.make.log_text(build_status_message)?;
            let log_path = self.top_build_directory.as_path().join("rsmake_log.txt");
            println!("rsmake: Build log available at {:?}", log_path);
        }
        Ok(())
    }

    pub fn build_dependency(
        &self,
        dependency: &DependencyNode,
        build_path: &PathBuf,
        verbosity: bool,
    ) -> Result<Output, BuilderError> {
        let build_directory = self.resolve_build_directory(build_path);

        for required_dependency in dependency.borrow().requires().borrow().iter() {
            let build_path_dep = &build_directory
                .join("libs")
                .join(required_dependency.borrow().get_project_name());

            if required_dependency.borrow().is_build_completed() {
                let top_build_directory_resolved =
                    self.resolve_build_directory(&self.top_build_directory.as_path());
                let directory_to_link = top_build_directory_resolved
                    .join("libs")
                    .join(required_dependency.borrow().get_project_name());

                if !build_path_dep.is_dir() {
                    crate::utility::create_symlink(directory_to_link, build_path_dep)?;
                }

                // Se eventuelt etter annen løsning.
                continue;
            }

            required_dependency.borrow_mut().building();
            let dep_output =
                self.build_dependency(&required_dependency, &build_path_dep, verbosity)?;
            if !dep_output.status.success() {
                return Ok(dep_output);
            }
            required_dependency.borrow_mut().build_complete();
        }

        dependency.borrow_mut().building();

        self.change_directory(build_directory, verbosity);
        println!("{}", Builder::construct_build_message(dependency));

        let output = self.make.spawn()?;
        dependency.borrow_mut().build_complete();

        Ok(output)
    }

    fn construct_build_message(dependency: &DependencyNode) -> String {
        let dep_type = if dependency.borrow().is_executable() {
            "executable"
        } else {
            "library"
        };
        let dep_type_name = dependency.borrow().get_pretty_name();

        let green_building = format!("{}", "Building".green());
        let target = format!("{} {:?}", dep_type, dep_type_name);
        format!("{} {}", green_building, target)
    }

    pub fn change_directory(&self, directory: std::path::PathBuf, verbose: bool) {
        let message = format!("Entering directory {:?}\n", directory);
        if verbose {
            print!("{}", message);
        }
        self.make.log_text(message).unwrap();
        std::env::set_current_dir(directory).unwrap()
    }

    fn resolve_build_directory(&self, path: &PathBuf) -> PathBuf {
        if self.debug {
            return path.join("debug");
        } else {
            return path.join("release");
        }
    }

    fn use_configuration(
        &mut self,
        configurations: &BuildConfigurations,
    ) -> Result<(), BuilderError> {
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
