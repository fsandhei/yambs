use dependency::{Dependency, DependencyRegistry, DependencyNode};
use error::MyMakeError;
use generator::MmkGenerator;
use std::io::{self, Write};
use colored::Colorize;
use std::process::Output;
use std::rc::Rc;

mod filter;
mod clean;
mod make;
use make::Make;


pub struct Builder {
    top_dependency: Option<DependencyNode>,
    dep_registry: DependencyRegistry,
    generator: Option<MmkGenerator>,
    debug: bool,
    verbose: bool,
    make: Make,
}


impl Builder {
    pub fn new() -> Builder {
        Builder {
            top_dependency: None,
            dep_registry: DependencyRegistry::new(),
            generator: None,
            debug: false,
            verbose: false,
            make: Make::new(),
        }
    }


    pub fn add_generator(&mut self) {
        if let Some(top_dependency) = &self.top_dependency {
            self.generator = Some(MmkGenerator::new(&top_dependency, 
                &std::path::PathBuf::from(".build"))
                            .unwrap())
        }
    }


    pub fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }


    pub fn use_std(&mut self, version: &str) -> Result<(), MyMakeError> {
        self.generator.as_mut().unwrap().use_std(version)
    }

    
    pub fn debug(&mut self) {
        self.debug = true;
        self.generator.as_mut().unwrap().debug();
    }


    pub fn release(&mut self) {
        self.generator.as_mut().unwrap().release();
    }


    pub fn verbose(&mut self) {
        self.verbose = true;
    }


    pub fn top_dependency(&self) -> &Option<DependencyNode> {
        &self.top_dependency
    }


    pub fn create_log_file(&mut self) -> Result<(), MyMakeError> {
        if let Some(top_dependency) = &self.top_dependency {
            if top_dependency.borrow().is_makefile_made() {
                let log_file_name = top_dependency.borrow().get_build_directory().join("mymake_log.txt");
                return self.make.add_logger(&log_file_name);
            }
            else {
                return Err(MyMakeError::from(format!("Error: Can't create log file because top dependency does not have a makefile!")));
            }
        }
        Ok(())
    }


    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        print!("MyMake: Reading MyMake files");
        io::stdout().flush().unwrap();
        let top_dependency = Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = Some(Rc::clone(&top_dependency));
        println!();
        Ok(())
    }


    pub fn generate_makefiles(&mut self) -> Result<(), MyMakeError> {
        if let Some(top_dependency) = &self.top_dependency {
            return self.generator.as_mut().unwrap().generate_makefiles(&top_dependency);
        }
        else {
            return Err(MyMakeError::from(String::from("builder.generate_builder(): Called in unexpected way.")));
        }
        
    }


    pub fn build_project(&mut self) -> Result<(), MyMakeError> {
        println!("MyMake: Building...");
        self.create_log_file()?;
        
        if let Some(top_dependency) = &self.top_dependency {
            let output = self.build_dependency(&top_dependency, self.verbose);
            if output.is_ok() && output.unwrap().status.success() {
                println!("MyMake: {}", "Build SUCCESS".green());
            }
            else {
                println!("MyMake: {}", "Build FAILED".red());
            }
            let log_path = top_dependency.borrow().get_build_directory().join("mymake_log.txt");
            println!("MyMake: Build log available at {:?}", log_path);
        }
        Ok(())
    }


    pub fn build_dependency(&self, dependency: &DependencyNode, 
                            verbosity: bool) -> Result<Output, MyMakeError> {
        for required_dependency in dependency.borrow().requires().borrow().iter() {
            if required_dependency.borrow().is_build_completed() {
                continue;
            }
            required_dependency.borrow_mut().building();
            let dep_output = self.build_dependency(&required_dependency, 
                                                         verbosity)?;
            if !dep_output.status.success() {
                return Ok(dep_output);
            }
            required_dependency.borrow_mut().build_complete();
        }

        dependency.borrow_mut().building();
        let build_directory = dependency.borrow().get_build_directory();
        if self.debug {
            self.change_directory(build_directory.join("debug"), verbosity);
        }
        else {
            self.change_directory(build_directory.join("release"), verbosity);
        }
        Builder::construct_build_message(dependency);
        
        let output = self.make.spawn()?;
        dependency.borrow_mut().build_complete();
    
        Ok(output)
    }


    pub fn construct_build_message(dependency: &DependencyNode) {
        let dep_type: &str;
        let dep_type_name: String;
        
        if dependency.borrow().is_executable() {
            dep_type = "executable";
            dep_type_name = dependency.borrow().mmk_data().data["MMK_EXECUTABLE"][0].clone();
        }
        else {
            dep_type = "library";
            dep_type_name = dependency.borrow().library_name();
        }
        let green_building = format!("{}", "Building".green());
        let target = format!("{} {:?}", dep_type, dep_type_name);
        println!("{} {}", green_building, target);
    }


    pub fn change_directory(&self, directory: std::path::PathBuf, verbose: bool) {
        let message = format!("Entering directory {:?}\n", directory);
        if verbose {
            print!("{}", message);
        }
        self.make.log_text(message).unwrap();
        std::env::set_current_dir(directory).unwrap()
    }


    pub fn clean(&self) -> Result<(), MyMakeError> {
        if let Some(top_dependency) = &self.top_dependency {
            clean::clean(top_dependency)?;
        }
        else {
            return Err(MyMakeError::from(String::from("builder.clean(): Unexpected call of function.")));
        }
        Ok(())
    }
}

//TODO: Skriv om testene for Builder slik at det stemmer med funksjonalitet.
#[cfg(test)]
mod tests {
    use super::*;
    use mmk_parser::Mmk;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;

    fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
        let dir: TempDir = TempDir::new(&dir_name).unwrap();
        let test_file_path = dir.path().join("lib.mmk");
        let mut file = File::create(&test_file_path)
                                .expect("make_mmk_file(): Something went wrong writing to file.");
        write!(file, 
        "MMK_SOURCES:
            some_file.cpp
            some_other_file.cpp
        
        MMK_HEADERS:
            some_file.h
            some_other_file.h
        
            ").expect("make_mmk_file(): Something went wrong writing to file.");

        let mut mmk_data = Mmk::new();
        mmk_data.data.insert(String::from("MMK_SOURCES"), 
                             vec![String::from("some_file.cpp"), 
                                  String::from("some_other_file.cpp")]);
        
        mmk_data.data.insert(String::from("MMK_HEADERS"), 
                             vec![String::from("some_file.h"), 
                                  String::from("some_other_file.h")]);

        (dir, test_file_path, file, mmk_data)
    }
    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected) = make_mmk_file("example");
        
        write!(
            file,
            "MMK_EXECUTABLE:
                x
            ")?;
        assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(builder.top_dependency.unwrap().borrow().mmk_data(), &expected);
        Ok(())
    }

    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1)     = make_mmk_file("example");
        let (dir_dep, _, _file_dep, _) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_DEPEND:
                {}
        \n
        
        MMK_EXECUTABLE:
            x
        ",
            &dir_dep.path().to_str().unwrap().to_string()
        )?;

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
            = make_mmk_file("example");
        let (dir_dep, _, _file_dep, _) 
            = make_mmk_file("example_dep");
        let (second_dir_dep, _, _file_second_file_dep, _) 
            = make_mmk_file("example_dep");

        write!(
            file,
            "\
        MMK_DEPEND:
            {}
            {}
        
        \n
        MMK_EXECUTABLE:
            x",
            &dir_dep.path().to_str().unwrap().to_string(),
            &second_dir_dep.path().to_str().unwrap().to_string()
        )?;

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string(),
                 second_dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert!((builder.read_mmk_files_from_path(&test_file_path)).is_ok());
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
        = make_mmk_file("example");
    let (dir_dep, _, mut file_dep, mut expected_2) 
        = make_mmk_file("example_dep");
    let (second_dir_dep, _, _file_second_file_dep, _) 
        = make_mmk_file("example_dep_second");

        write!(
            file,
            "\
        MMK_DEPEND:
            {}
        \n
        MMK_EXECUTABLE:
            x",
            &dir_dep.path().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_DEPEND:
            {}
        \n
        ",
        &second_dir_dep.path().to_str().unwrap().to_string())?;

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_2
            .data
            .insert(String::from("MMK_DEPEND"), vec![second_dir_dep.path().to_str().unwrap().to_string()]);

        assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
        Ok(())
    }

    #[test]
    fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
        = make_mmk_file("example");
    let (dir_dep, _, mut file_dep, mut expected_2) 
        = make_mmk_file("example_dep");
    let (second_dir_dep, _, _file_second_file_dep, _) 
        = make_mmk_file("example_dep_second");
    let (third_dir_dep, _, _file_third_file_dep, _) 
        = make_mmk_file("example_dep_third");

        write!(
            file,
            "\
        MMK_DEPEND:
            {}
            {}
        \n
        MMK_EXECUTABLE:
            x",            
            &third_dir_dep.path().to_str().unwrap().to_string(),
            &dir_dep.path().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_DEPEND:
            {}
        \n
        ",
        &second_dir_dep.path().to_str().unwrap().to_string())?;

        

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![third_dir_dep.path().to_str().unwrap().to_string(),
                 dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_2
            .data
            .insert(String::from("MMK_DEPEND"), vec![second_dir_dep.path().to_str().unwrap().to_string()]);
        
        assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
        Ok(())
    }
    #[test]
    fn read_mmk_files_two_files_circulation() -> Result<(), MyMakeError> {
        let mut builder = Builder::new();
        let (dir, test_file_path, mut file, _expected_1)              = make_mmk_file("example");
        let (dir_dep, _test_file_dep_path, mut file_dep, _expected_2) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_DEPEND:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
            &dir_dep.path().to_str().unwrap().to_string()
        ).unwrap();

        write!(
            file_dep,
            "\
            MMK_DEPEND:
                {}
        \n", &dir.path().to_str().unwrap().to_string()
        ).unwrap();

        let result = builder.read_mmk_files_from_path(&test_file_path);

        assert!(result.is_err());
        Ok(())
    }
}
