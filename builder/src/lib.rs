use mmk_parser;
use std::fmt;
use std::error::Error;
#[derive(Debug)]
pub struct MyMakeError {
    details: String
}

impl MyMakeError {
    #[cfg(maybe_unused)]
    fn new(msg: &str) -> MyMakeError {
        MyMakeError{details: msg.to_string()}
    }
    fn from(msg: String) -> MyMakeError {
        MyMakeError{details: msg}
    }
}

impl fmt::Display for MyMakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MyMakeError {
    fn description(&self) -> &str {
        &self.details 
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    pub path: std::path::PathBuf,
    pub requires: Vec<Dependency>,
}

impl Dependency {
    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            requires: vec![],
        }
    }
    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.push(dependency);
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

    pub fn read_mmk_files(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        let mut top_dependency = Dependency::from(top_path);
        let file_content = match mmk_parser::read_file(top_path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", top_path, err))),
        };
        let mut top = mmk_parser::Mmk::new();
        top.parse_file(&file_content);

        for path in top.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");

            let required_dependency = Dependency::from(&dep_path);
            top_dependency.add_dependency(required_dependency);
            self.read_mmk_files(&dep_path)?;
        }
        self.mmk_data.push(top);
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");
        let mut file = File::create(&test_file)?;
        write!(
            file,
            "\
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x"
        )?;
        builder.read_mmk_files(&test_file).unwrap();
        let mut expected = Mmk::new();

        expected
            .data
            .insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("some_file.cpp"),
                String::from("some_other_file.cpp"),
            ],
        );
        expected.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("some_file.h"),
                String::from("some_other_file.h"),
            ],
        );
        assert_eq!(builder.mmk_data[0], expected);
        Ok(())
    }

    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");

        let dir_dep = TempDir::new("example_dep")?;
        let test_file_dep = dir_dep.path().join("mymakeinfo.mmk");

        let mut file = File::create(&test_file)?;
        let mut file_dep = File::create(&test_file_dep)?;

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
        \n
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string()
        )?;

        write!(
            file_dep,
            "\
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x"
        )?;

        builder.read_mmk_files(&test_file).unwrap();
        let mut expected_1 = Mmk::new();
        let mut expected_2 = Mmk::new();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_1.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("some_file.cpp"),
                String::from("some_other_file.cpp"),
            ],
        );
        expected_1.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("some_file.h"),
                String::from("some_other_file.h"),
            ],
        );

        expected_2
            .data
            .insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected_2
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_2.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("/some/some_file.cpp"),
                String::from("/some/other_file.cpp"),
            ],
        );
        expected_2.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("/some/some_file.h"),
                String::from("/some/some_other_file.h"),
            ],
        );
        assert_eq!(builder.mmk_data[1], expected_1);
        assert_eq!(builder.mmk_data[0], expected_2);
        assert_eq!(
            builder.mmk_dependencies[1],
            Dependency {
                path: test_file,
                requires: vec![Dependency {
                    path: test_file_dep,
                    requires: Vec::new()
                }]
            }
        );
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");

        let dir_dep = TempDir::new("example_dep")?;
        let test_file_dep = dir_dep.path().join("mymakeinfo.mmk");

        let second_dir_dep = TempDir::new("example_dep_second")?;
        let second_test_file_dep = second_dir_dep.path().join("mymakeinfo.mmk");

        let mut file = File::create(&test_file)?;
        let mut file_dep = File::create(&test_file_dep)?;
        let mut second_file_dep = File::create(&second_test_file_dep)?;

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
                     {} \\
        \n
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string(),
            &second_dir_dep.path().to_str().unwrap().to_string()
        )?;

        write!(
            file_dep,
            "\
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x"
        )?;

        write!(
            second_file_dep,
            "\
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x"
        )?;

        builder.read_mmk_files(&test_file).unwrap();
        let mut expected_1 = Mmk::new();
        let mut expected_2 = Mmk::new();
        let mut expected_3 = Mmk::new();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string(),
                 second_dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_1.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("some_file.cpp"),
                String::from("some_other_file.cpp"),
            ],
        );
        expected_1.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("some_file.h"),
                String::from("some_other_file.h"),
            ],
        );

        expected_2
            .data
            .insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected_2
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_2.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("/some/some_file.cpp"),
                String::from("/some/other_file.cpp"),
            ],
        );
        expected_2.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("/some/some_file.h"),
                String::from("/some/some_other_file.h"),
            ],
        );

        expected_3
            .data
            .insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected_3
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_3.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("/some/some_file.cpp"),
                String::from("/some/other_file.cpp"),
            ],
        );
        expected_3.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("/some/some_file.h"),
                String::from("/some/some_other_file.h"),
            ],
        );

        assert_eq!(builder.mmk_data[2], expected_1);
        assert_eq!(builder.mmk_data[1], expected_2);
        assert_eq!(builder.mmk_data[0], expected_3);
        assert_eq!(
            builder.mmk_dependencies[2],
            Dependency {
                path: test_file,
                requires: vec![Dependency {
                    path: test_file_dep,
                    requires: Vec::new()
                },
                Dependency {
                    path: second_test_file_dep,
                    requires: Vec::new()
                }]
            }
        );
        Ok(())
    }

    #[test]
    fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");

        let dir_dep = TempDir::new("example_dep")?;
        let test_file_dep = dir_dep.path().join("mymakeinfo.mmk");

        let second_dir_dep = TempDir::new("example_dep_second")?;
        let second_test_file_dep = second_dir_dep.path().join("mymakeinfo.mmk");

        let mut file = File::create(&test_file)?;
        let mut file_dep = File::create(&test_file_dep)?;
        let mut second_file_dep = File::create(&second_test_file_dep)?;

        write!(
            file,
            "\
        MMK_DEPEND = {} \\
        \n
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x",
            &dir_dep.path().to_str().unwrap().to_string()            
        )?;

        write!(
            file_dep,
            "\
        MMK_DEPEND = {} \\
        \n
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x",
        &second_dir_dep.path().to_str().unwrap().to_string()
        )?;

        write!(
            second_file_dep,
            "\
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x"
        )?;

        builder.read_mmk_files(&test_file).unwrap();
        let mut expected_1 = Mmk::new();
        let mut expected_2 = Mmk::new();
        let mut expected_3 = Mmk::new();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_1.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("some_file.cpp"),
                String::from("some_other_file.cpp"),
            ],
        );
        expected_1.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("some_file.h"),
                String::from("some_other_file.h"),
            ],
        );

        expected_2
            .data
            .insert(String::from("MMK_DEPEND"), vec![second_dir_dep.path().to_str().unwrap().to_string()]);
        expected_2
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_2.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("/some/some_file.cpp"),
                String::from("/some/other_file.cpp"),
            ],
        );
        expected_2.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("/some/some_file.h"),
                String::from("/some/some_other_file.h"),
            ],
        );

        expected_3
            .data
            .insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected_3
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_3.data.insert(
            String::from("MMK_SOURCES"),
            vec![
                String::from("/some/some_file.cpp"),
                String::from("/some/other_file.cpp"),
            ],
        );
        expected_3.data.insert(
            String::from("MMK_HEADERS"),
            vec![
                String::from("/some/some_file.h"),
                String::from("/some/some_other_file.h"),
            ],
        );

        assert_eq!(builder.mmk_data[2], expected_1);
        assert_eq!(builder.mmk_data[1], expected_2);
        assert_eq!(builder.mmk_data[0], expected_3);
        assert_eq!(
            builder.mmk_dependencies[2],
            Dependency {
                path: test_file,
                requires: vec![Dependency {
                    path: test_file_dep,
                    requires: vec![
                        Dependency {
                            path: second_test_file_dep,
                            requires: vec![]
                        }]
                }]
            }
        );
        Ok(())
    }
}
