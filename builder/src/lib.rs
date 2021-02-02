use dependency::{Dependency, DependencyRegistry};
use error::MyMakeError;
use std::io::{self, Write};
use std::process::Command;
use colored::Colorize;

mod filter;
mod clean;

pub struct Builder {
    pub top_dependency: Dependency,
    pub dep_registry: DependencyRegistry,
    pub log_file: Option<std::fs::File>,
}


impl Builder {
    pub fn new() -> Builder {
        Builder {
            top_dependency: Dependency::new(),
            dep_registry: DependencyRegistry::new(),
            log_file: None,
        }
    }


    pub fn create_log_file(&self) -> Result<Option<std::fs::File>, MyMakeError> {
        if self.top_dependency.is_makefile_made() {
            let log_file_name = self.top_dependency.get_build_directory().join("mymake_log.txt");
            match std::fs::File::create(&log_file_name) {
                Ok(file) =>  return Ok(Some(file)),
                Err(err) => return Err(MyMakeError::from(format!("Error creating {:?}: {}", log_file_name, err))),
            };
        }
        return Err(MyMakeError::from(format!("Error: Can't create log file because top dependency does not have a makefile!")));
    }


    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        print!("MyMake: Reading MyMake files");
        io::stdout().flush().unwrap();
        let top_dependency = Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = top_dependency;
        println!();
        Ok(())
    }

    // TBD: Flytte funksjon til generator?
    pub fn generate_makefiles(dependency: &mut Dependency) -> Result<(), MyMakeError> {

        let mut generator: generator::MmkGenerator;
        let build_directory = std::path::PathBuf::from(".build");
        if !&dependency.is_makefile_made()
        {
            generator = generator::MmkGenerator::new(&dependency,
                                                     &build_directory)?;
            &dependency.makefile_made();
            generator::Generator::generate_makefile(&mut generator)?;
        }
        for required_dependency in dependency.requires().borrow().iter()
        {
            if !required_dependency.borrow().is_makefile_made()
            {
                required_dependency.borrow_mut().makefile_made();
                generator = generator::MmkGenerator::new(&required_dependency.borrow(),
                                                         &build_directory)?;
                generator::Generator::generate_makefile(&mut generator)?;
            }
            Builder::generate_makefiles(&mut required_dependency.borrow_mut())?;
        }
        Ok(())
    }


    pub fn build_project(&mut self, verbosity: bool) -> Result<(), MyMakeError> {
        println!("MyMake: Building...");
        self.log_file = self.create_log_file()?;
        
        let output = self.build_dependency(&self.top_dependency, verbosity);
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
                            verbosity: bool) -> Result<std::process::Output, MyMakeError> {
        for required_dependency in dependency.requires().borrow().iter() {
            let dep_output = self.build_dependency(&required_dependency.borrow(), 
                                                         verbosity)?;
            if !dep_output.status.success() {
                return Ok(dep_output);
            }
        }

        let build_directory = dependency.get_build_directory();
        self.change_directory(build_directory, verbosity);
        Builder::construct_build_message(dependency);
        let child = Command::new("/usr/bin/make")
                                                            .stdout(std::process::Stdio::piped())
                                                            .stderr(std::process::Stdio::piped())
                                                            .spawn()?;

        let output = child.wait_with_output()?;
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();
        
        let stderr_filtered = filter::filter_string(&stderr);
        if stderr_filtered != String::from("") {
            filter::println_colored(&stderr_filtered);
        }
        
        self.log_file.as_ref().unwrap().write(stdout.as_bytes())?;
        self.log_file.as_ref().unwrap().write(stderr.as_bytes())?;
    
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
            println!("{}", message);
        }
        self.log_file.as_ref().unwrap().write(message.as_bytes()).unwrap();
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
