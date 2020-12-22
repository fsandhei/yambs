use dependency::Dependency;
use error::MyMakeError;

pub struct Builder {
    pub top_dependency: Dependency,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            top_dependency: Dependency::new(),
        }
    }

    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        print!("MyMake: Reading mmk files");
        let top_dependency = Dependency::create_dependency_from_path(&top_path)?;
        self.top_dependency = top_dependency;
        println!();
        Ok(())
    }
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
                })]),
                makefile_made: false,
                library_name: String::new(),
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
                }),
                RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::new(),
                })]),
                makefile_made: false,
                library_name: String::new(),
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
                        })]),
                    makefile_made: false,
                    library_name: String::new(),
                })]),
                makefile_made: false,
                library_name: String::new(),
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
                        })]),
                    makefile_made: false,
                    library_name: String::new(),
                })]),
                makefile_made: false,
                library_name: String::new(),
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
