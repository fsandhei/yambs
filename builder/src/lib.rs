use error::MyMakeError;
use mmk_parser;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    pub path: std::path::PathBuf,
    pub mmk_data: mmk_parser::Mmk,
    pub requires: Vec<Rc<Dependency>>,
}

impl Dependency {
    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            mmk_data: mmk_parser::Mmk::new(),
            requires: Vec::new(),
        }
    }

    pub fn create_dependency_from_path(path: &std::path::PathBuf) -> Result<Dependency, MyMakeError>{
        let mut dependency = Dependency::from(path);
        dependency.read_and_add_mmk_data()?;
        dependency.detect_and_add_dependencies()?;
        dependency.print_ok();
        Ok(dependency)
    }

    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.push(Rc::new(dependency));
    }

    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, MyMakeError>{
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", self.path, err))),
        };
        let mut mmk_data = mmk_parser::Mmk::new();
        mmk_data.parse_file(&file_content);
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }

    pub fn detect_and_add_dependencies(self: &mut Self) -> Result<(), MyMakeError>{
        for path in self.mmk_data.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");
            let dependency = Dependency::create_dependency_from_path(&dep_path)?;
            self.add_dependency(dependency);
        }
        Ok(())
    }

    pub fn print_ok(self: &Self) {
        print!(".");
    }
}
pub struct Builder {
    pub mmk_data: std::vec::Vec<mmk_parser::Mmk>,
    pub mmk_dependencies: std::vec::Vec<Dependency>,
}

impl Builder {
    pub fn new() -> Builder {
        Builder {
            mmk_data: Vec::new(),
            mmk_dependencies: Vec::new(),
        }
    }
    pub fn from(dep_path: &std::path::PathBuf) -> Builder {
        let mut builder = Builder::new();
        let dependency = Dependency::from(dep_path);
        builder.mmk_dependencies.push(dependency);
        builder
    }

    pub fn read_mmk_files_from_path(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        let top_dependency = Dependency::create_dependency_from_path(&top_path)?;
        self.mmk_dependencies.push(top_dependency);        
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
        assert_eq!(builder.mmk_dependencies.last().unwrap().mmk_data, expected);
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
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: vec![Rc::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: Vec::new()
                })]
            })
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
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: vec![Rc::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: Vec::new()
                }),
                Rc::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: Vec::new()
                })]
            })
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
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: vec![Rc::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: vec![
                        Rc::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_3,
                            requires: vec![]
                        })]
                })]
            })
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
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: vec![Rc::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_3,
                    requires: vec![]
                }),
                Rc::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: vec![
                        Rc::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_4,
                            requires: vec![]
                        })]
                })]
            })
        );
        Ok(())
    }
}
