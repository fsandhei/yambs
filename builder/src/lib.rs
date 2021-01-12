use dependency::{Dependency, DependencyRegistry};
use error::MyMakeError;
use std::io::{self, Write};
use std::process::Command;
use colored::Colorize;

pub struct Builder {
    pub top_dependency: Dependency,
    pub dep_registry: DependencyRegistry,
}


impl Builder {
    pub fn new() -> Builder {
        Builder {
            top_dependency: Dependency::new(),
            dep_registry: DependencyRegistry::new(),
        }
    }


    pub fn create_log_file(&self) -> Result<std::fs::File, MyMakeError> {
        if self.top_dependency.makefile_made {
            let log_file_name = self.top_dependency.get_build_directory().join("mymake_log.txt");
            match std::fs::File::create(&log_file_name) {
                Ok(file) => file,
                Err(err) => return Err(MyMakeError::from(format!("Error creating {:?}: {}", log_file_name, err))),
            };
        }
        return Err(MyMakeError::from(format!("Error: Can't create log file because top dependency does not have a makefile!")));
    }


    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        print!("MyMake: Reading mmk files");
        io::stdout().flush().unwrap();
        let top_dependency = Dependency::create_dependency_from_path(&top_path, &mut self.dep_registry)?;
        self.top_dependency = top_dependency;
        println!();
        Ok(())
    }

    // TBD: Flytte funksjon til generator?
    pub fn generate_makefiles(dependency: &mut Dependency) -> Result<(), MyMakeError> {

        let mut generator: generator::MmkGenerator;
        if !&dependency.makefile_made
        {
            generator = generator::MmkGenerator::new(&dependency,
                                       std::path::PathBuf::from(".build"))?;
            &dependency.makefile_made();
            generator::Generator::generate_makefile(&mut generator)?;
        }
        for required_dependency in dependency.requires.borrow().iter()
        {
            if !required_dependency.borrow().makefile_made
            {                   
                required_dependency.borrow_mut().makefile_made();
                generator = generator::MmkGenerator::new(&required_dependency.borrow(),
                     std::path::PathBuf::from(".build"))?;
                generator::Generator::generate_makefile(&mut generator)?;
            }
            Builder::generate_makefiles(&mut required_dependency.borrow_mut())?;
        }
        Ok(())
    }


    pub fn build_project(self: &Self, verbosity: bool) -> Result<(), MyMakeError> {
        println!("MyMake: Building...");
        let stdout = self.create_log_file()?;
        let stderr = stdout.try_clone().unwrap();
        let output = self.build_dependency(&self.top_dependency, verbosity, &stdout, &stderr)?;
        if output.status.success() {
            println!("{}", "Build SUCCESS".green());
        }
        else {
            println!("{}", "Build FAILED".red());
        }
        Ok(())
    }


    pub fn build_dependency(&self, dependency: &Dependency, 
                            verbosity: bool, 
                            stdout: &std::fs::File,
                            stderr: &std::fs::File) -> Result<std::process::Output, MyMakeError> {
        for required_dependency in dependency.requires.borrow().iter() {
            let dep_output = self.build_dependency(&required_dependency.borrow(), 
                                                         verbosity,
                                                         stdout,
                                                         stderr)?;
            if !dep_output.status.success() {
                return Ok(dep_output);
            }
        }
        let build_directory = dependency.get_build_directory();
        self.change_directory(build_directory, verbosity);
        Builder::construct_build_message(dependency);
        let output = Command::new("/usr/bin/make").output().expect("Failed...");
        
        Ok(output)
    }


    pub fn construct_build_message(dependency: &Dependency) {
        let dep_type: &str;
        let dep_type_name: &String;
        
        if dependency.library_name == "" {
            dep_type = "executable";
            dep_type_name = &dependency.mmk_data.data["MMK_EXECUTABLE"][0];
        }
        else {
            dep_type = "library";
            dep_type_name = &dependency.mmk_data.data["MMK_LIBRARY_LABEL"][0];
        }
        println!("Building {} {:?}", dep_type, dep_type_name);
    }


    pub fn change_directory(&self, directory: std::path::PathBuf, verbose: bool) {
        let message = format!("Entering directory {:?}", directory);
        if verbose {
            println!("{}", message);
        }

        std::env::set_current_dir(directory).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mmk_parser::Mmk;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;
    use std::cell::RefCell;

    fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
        let dir: TempDir = TempDir::new(&dir_name).unwrap();
        let test_file_path = dir.path().join("mymakeinfo.mmk");
        let mut file = File::create(&test_file_path)
                                .expect("make_mmk_file(): Something went wrong writing to file.");
        write!(file, 
        "\
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n").expect("make_mmk_file(): Something went wrong writing to file.");

        let mut mmk_data = Mmk::new();
        mmk_data.data.insert(String::from("MMK_SOURCES"), 
                             vec![String::from("some_file.cpp"), 
                                  String::from("some_other_file.cpp")]);
        
        mmk_data.data.insert(String::from("MMK_HEADERS"), 
                             vec![String::from("some_file.h"), 
                                  String::from("some_other_file.h")]);
        
        mmk_data.data.insert(String::from("MMK_DEPEND"), 
                             vec![String::new()]);
        mmk_data.data.insert(String::from("MMK_EXECUTABLE"), 
                             vec![String::new()]);
        mmk_data.data.insert(String::from("MMK_LIBRARY_LABEL"), 
                             vec![String::new()]);

        (dir, test_file_path, file, mmk_data)
    }
    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected) = make_mmk_file("example");
        
        write!(
            file,
            "\
        MMK_EXECUTABLE = x"
        )?;
        builder.read_mmk_files_from_path(&test_file_path).unwrap();
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(builder.top_dependency.mmk_data, expected);
        Ok(())
    }

    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1)     = make_mmk_file("example");
        let (dir_dep, test_file_dep_path, _file_dep, expected_2) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_DEPEND = {} \\
        \n
        
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string()
        )?;

        builder.read_mmk_files_from_path(&test_file_path).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert_eq!(
            builder.top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::new(),
                in_process: false,
            }
        );
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
            = make_mmk_file("example");
        let (dir_dep, test_file_dep_path, _file_dep, expected_2) 
            = make_mmk_file("example_dep");
        let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) 
            = make_mmk_file("example_dep");

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
                     {} \\
        
        \n
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string(),
            &second_dir_dep.path().to_str().unwrap().to_string()
        )?;

        builder.read_mmk_files_from_path(&test_file_path).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string(),
                 second_dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert_eq!(
            builder.top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                }),
                RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::new(),
                in_process: false,
            }
        );
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
        = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut expected_2) 
        = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) 
        = make_mmk_file("example_dep_second");

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
        \n
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_DEPEND = {} \\
        \n
        ",
        &second_dir_dep.path().to_str().unwrap().to_string())?;

        builder.read_mmk_files_from_path(&test_file_path).unwrap();

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

        assert_eq!(
            builder.top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![
                        RefCell::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_3,
                            requires: RefCell::new(vec![]),
                            makefile_made: false,
                            library_name: String::new(),
                            in_process: false,
                        })]),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::new(),
                in_process: false,
            }
        );
        Ok(())
    }

    #[test]
    fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let (_dir, test_file_path, mut file, mut expected_1) 
        = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut expected_2) 
        = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) 
        = make_mmk_file("example_dep_second");
    let (third_dir_dep, test_file_third_dep_path, _file_third_file_dep, expected_4) 
        = make_mmk_file("example_dep_third");

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
                     {} \\
        \n
        MMK_EXECUTABLE = x",            
            &third_dir_dep.path().to_str().unwrap().to_string(),
            &dir_dep.path().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_DEPEND = {} \\
        \n
        ",
        &second_dir_dep.path().to_str().unwrap().to_string())?;

        builder.read_mmk_files_from_path(&test_file_path).unwrap();

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
        
        assert_eq!(
            builder.top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(vec![]),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                }),
                RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![
                        RefCell::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_4,
                            requires: RefCell::new(vec![]),
                            makefile_made: false,
                            library_name: String::new(),
                            in_process: false,
                        })]),
                    makefile_made: false,
                    library_name: String::new(),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::new(),
                in_process: false,
            }
        );
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
            MMK_DEPEND = {} \\
        \n
        
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string()
        ).unwrap();

        write!(
            file_dep,
            "\
            MMK_DEPEND = {} \\
        \n", &dir.path().to_str().unwrap().to_string()
        ).unwrap();

        let result = builder.read_mmk_files_from_path(&test_file_path);

        assert!(result.is_err());
        Ok(())
    }
}
