use dependency::{Dependency, DependencyRegistry};
use error::MyMakeError;
use generator::MmkGenerator;
use std::io::{self, Write};
use colored::Colorize;
use std::process::Output;

mod filter;
mod clean;
mod make;
use make::Make;

pub struct Builder {
    top_dependency: Dependency,
    dep_registry: DependencyRegistry,
    // log_file: Option<std::fs::File>,
    generator: Option<MmkGenerator>,
    debug: bool,
    verbose: bool,
    make: Make,
}


impl Builder {
    pub fn new() -> Builder {
        Builder {
            top_dependency: Dependency::new(),
            dep_registry: DependencyRegistry::new(),
            // log_file: None,
            generator: None,
            debug: false,
            verbose: false,
            make: Make::new(),
        }
    }


    pub fn add_generator(&mut self) {
        self.generator = Some(MmkGenerator::new(&self.top_dependency, 
                &std::path::PathBuf::from(".build"))
                            .unwrap())
    }


    pub fn add_make(&mut self, flag: &str, value: &str) {
        self.make = Make::new().with_flag(flag, value);
    }

    
    pub fn debug(&mut self) {
        self.debug = true;
        self.generator.as_mut().unwrap().debug();
    }


    pub fn verbose(&mut self) {
        self.verbose = true;
    }


    pub fn top_dependency(&self) -> &Dependency {
        &self.top_dependency
    }


    pub fn create_log_file(&mut self) -> Result<(), MyMakeError> {
        if self.top_dependency.is_makefile_made() {
            let log_file_name = self.top_dependency.get_build_directory().join("mymake_log.txt");
            self.make.add_logger(&log_file_name)
        }
        else
        {
            return Err(MyMakeError::from(format!("Error: Can't create log file because top dependency does not have a makefile!")));
        }
    }


    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        print!("MyMake: Reading MyMake files");
        io::stdout().flush().unwrap();
        let top_dependency = Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = top_dependency;
        println!();
        Ok(())
    }


    pub fn generate_makefiles(&mut self) -> Result<(), MyMakeError> {
        self.generator.as_mut().unwrap().generate_makefiles(&mut self.top_dependency)
    }


    pub fn build_project(&mut self) -> Result<(), MyMakeError> {
        println!("MyMake: Building...");
        self.create_log_file()?;
        
        let output = self.build_dependency(&self.top_dependency, self.verbose);
        if output.is_ok() && output.unwrap().status.success() {
            println!("MyMake: {}", "Build SUCCESS".green());
        }
        else {
            println!("MyMake: {}", "Build FAILED".red());
        }
        let log_path = self.top_dependency.get_build_directory().join("mymake_log.txt");
        println!("Build log available at {:?}", log_path);
        Ok(())
    }


    pub fn build_dependency(&self, dependency: &Dependency, 
                            verbosity: bool) -> Result<Output, MyMakeError> {
        for required_dependency in dependency.requires().borrow().iter() {
            let dep_output = self.build_dependency(&required_dependency.borrow(), 
                                                         verbosity)?;
            if !dep_output.status.success() {
                return Ok(dep_output);
            }
        }

        let build_directory = dependency.get_build_directory();
        if self.debug {
            self.change_directory(build_directory.join("debug"), verbosity);
        }
        else {
            self.change_directory(build_directory, verbosity);
        }
        Builder::construct_build_message(dependency);
        
        let output = self.make.spawn()?;

        // let output = child.wait_with_output()?;
        // let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        // let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        
        // let stderr_filtered = filter::filter_string(&stderr);
        // if stderr_filtered != String::from("") {
        //     filter::println_colored(&stderr_filtered);
        // }
        
        // self.log_file.as_ref().unwrap().write(stdout.as_bytes())?;
        // self.log_file.as_ref().unwrap().write(stderr.as_bytes())?;
    
        Ok(output)
    }


    pub fn construct_build_message(dependency: &Dependency) {
        let dep_type: &str;
        let dep_type_name: String;
        
        if dependency.is_executable() {
            dep_type = "executable";
            dep_type_name = dependency.mmk_data().data["MMK_EXECUTABLE"][0].clone();
        }
        else {
            dep_type = "library";
            dep_type_name = dependency.library_name();
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
        clean::clean(&self.top_dependency)?;
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
        let test_file_path = dir.path().join("mymakeinfo.mmk");
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
        assert_eq!(builder.top_dependency.mmk_data(), &expected);
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
