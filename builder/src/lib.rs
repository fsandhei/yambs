mod filter;
mod clean;
mod make;

use dependency::{Dependency, DependencyRegistry, DependencyNode};
use error::MyMakeError;
use generator::GeneratorExecutor;
use std::env;
use colored::Colorize;
use std::process::Output;
use std::rc::Rc;
use std::path::PathBuf;

use make::Make;

pub struct Builder<'a> {
    top_dependency: Option<DependencyNode>,
    dep_registry: DependencyRegistry,
    generator: Box<&'a mut dyn GeneratorExecutor>,
    debug: bool,
    verbose: bool,
    make: Make,
    top_build_directory: Option<PathBuf>
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
            top_build_directory: Some(env::current_dir().unwrap()) //At the moment hardcoded as current working directory.
        }
    }


    pub fn add_generator(&mut self, generator: &'a mut dyn GeneratorExecutor) {
        if let Some(_) = &self.top_dependency {
            self.generator = Box::new(generator);
        }
    }


    pub fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }


    pub fn use_std(&mut self, version: &str) -> Result<(), MyMakeError> {
            return self.generator.as_mut().use_std(version);
    }


    pub fn set_sanitizers(&mut self, sanitizers: &[String]) {
        self.generator.as_mut().set_sanitizers(sanitizers);
    }

    
    pub fn debug(&mut self) {
        self.debug = true;
        self.generator.as_mut().debug();
    }


    pub fn release(&mut self) {        
        self.generator.as_mut().release();
    }


    pub fn is_verbose(&self) -> bool {
        self.verbose
    }

    pub fn set_verbose(&mut self, value: bool) {
        self.verbose = value;
    }


    pub fn top_dependency(&self) -> &Option<DependencyNode> {
        &self.top_dependency
    }


    pub fn create_log_file(&mut self) -> Result<(), MyMakeError> {
        if let Some(top_dependency) = &self.top_dependency {
            if top_dependency.borrow().is_makefile_made() {
                let log_file_name = env::current_dir().unwrap().join("mymake_log.txt");
                return self.make.add_logger(&log_file_name);
            }
            else {
                return Err(MyMakeError::from(format!("Error: Can't create log file because top dependency does not have a makefile!")));
            }
        }
        Ok(())
    }


    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        let top_dependency = Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = Some(Rc::clone(&top_dependency));
        Ok(())
    }


    fn add_dependency_to_generator(&mut self, dependency: &DependencyNode) {
        self.generator.as_mut().set_dependency(dependency);
    }


    pub fn generate_makefiles(&mut self) -> Result<(), MyMakeError> {
        if let Some(top_dependency) = self.top_dependency.clone() {
            self.add_dependency_to_generator(&top_dependency);
            return self.generator.as_mut().generate_makefiles(&top_dependency);
        }
        else {
            return Err(MyMakeError::from(String::from("builder.generate_builder(): Called in unexpected way.")));
        }        
    }


    pub fn build_project(&mut self) -> Result<(), MyMakeError> {
        self.create_log_file()?;
        let build_directory = std::env::current_dir().unwrap();
        
        if let Some(top_dependency) = &self.top_dependency {
            let output = self.build_dependency(&top_dependency,
                                                                        &build_directory, 
                                                                        self.verbose);
            let build_status_message : String;
            if output.is_ok() && output.unwrap().status.success() {
                build_status_message = format!("MyMake: {}", "Build SUCCESS".green());
            }
            else {
                build_status_message = format!("MyMake: {}", "Build FAILED".red());
            }
            println!("{}", build_status_message);
            self.make.log_text(build_status_message)?;
            let log_path = self.top_build_directory.as_ref().unwrap().join("mymake_log.txt");
            println!("MyMake: Build log available at {:?}", log_path);
        }
        Ok(())
    }

    
    pub fn build_dependency(&self, dependency: &DependencyNode,
                            build_path: &PathBuf, 
                            verbosity: bool) -> Result<Output, MyMakeError> {
        
        let build_directory = self.resolve_build_directory(build_path);

        for required_dependency in dependency.borrow().requires().borrow().iter() {
            let build_path_dep = &build_directory.join("libs")
                                            .join(required_dependency.borrow().get_project_name());

            if required_dependency.borrow().is_build_completed() {
                let top_build_directory_resolved = self.resolve_build_directory(self.top_build_directory.as_ref().unwrap());
                let directory_to_link = top_build_directory_resolved.join("libs").join(required_dependency.borrow().get_project_name());

                if !build_path_dep.is_dir() {
                    utility::create_symlink(directory_to_link,build_path_dep)?;
                }
                
                // Se eventuelt etter annen løsning.
                continue;
            }
            
            required_dependency.borrow_mut().building();
            let dep_output = self.build_dependency(&required_dependency,
                                                          &build_path_dep,
                                                          verbosity)?;
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


    fn resolve_build_directory(&self, path: &PathBuf) -> PathBuf {
        if self.debug {
            return path.join("debug");
        }
        else {
            return path.join("release");
        }
    }


    fn construct_build_message(dependency: &DependencyNode) -> String {
        let dep_type: &str;
        let dep_type_name: String;
        
        if dependency.borrow().is_executable() {
            dep_type = "executable";
            dep_type_name = dependency.borrow().mmk_data().data()["MMK_EXECUTABLE"][0].argument().clone();
        }
        else {
            dep_type = "library";
            dep_type_name = dependency.borrow().library_name();
        }
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
}

#[cfg(test)]
#[path = "./lib_test.rs"]
mod lib_test;