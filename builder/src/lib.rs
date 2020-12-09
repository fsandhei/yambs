use error::MyMakeError;
use mmk_parser;


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dependency {
    pub path: std::path::PathBuf,
    pub requires: Vec<Box<Dependency>>,
}

impl Dependency {
    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            requires: Vec::new(),
        }
    }
    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.push(Box::new(dependency));
    }

    pub fn update(self: &mut Self) -> Result<(), MyMakeError> {
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", &self.path, err))),
        };
        let mut top = mmk_parser::Mmk::new();
        top.parse_file(&file_content);

        for path in top.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");

            &self.add_dependency(Dependency::from(&dep_path));
        }
        Ok(())
    }
}
#[derive(Clone)]
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

    pub fn dependency_from_path(self: Self, path: &std::path::PathBuf) -> Option<Box<Dependency>> {
        for dependency in self.mmk_dependencies {
            for dep_requirement in dependency.requires {
                if dep_requirement.path.to_str() == path.to_str() 
                {
                    return Some(dep_requirement)
                }
            }
        }
        None
    }

    pub fn read_mmk_files(self: &mut Self, top_path: &std::path::PathBuf) -> Result<(), MyMakeError> {
        let mut top_dependency: Dependency;
        let mut required_dependency: Vec<Dependency> = vec![];
        if let Some(dependency) = self.clone().dependency_from_path(top_path)
        {
            top_dependency = *dependency;
        }
        else 
        {
            top_dependency = Dependency::from(top_path);
        }
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

            required_dependency.push(Dependency::from(&dep_path));
        }
        
        for mut dep in required_dependency {
            self.read_mmk_files(&dep.path)?;
            dep.update()?;
            top_dependency.add_dependency(dep);
        }
        self.mmk_data.push(top);
        self.mmk_dependencies.push(top_dependency.clone());
        print!(".");
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
        builder.read_mmk_files(&test_file_path).unwrap();
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(builder.mmk_data[0], expected);
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

        builder.read_mmk_files(&test_file_path).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);


        assert_eq!(builder.mmk_data[1], expected_1);
        assert_eq!(builder.mmk_data[0], expected_2);
        assert_eq!(
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                requires: vec![Box::new(Dependency {
                    path: test_file_dep_path,
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

        builder.read_mmk_files(&test_file_path).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string(),
                 second_dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert_eq!(builder.mmk_data[2], expected_1);
        assert_eq!(builder.mmk_data[1], expected_2);
        assert_eq!(builder.mmk_data[0], expected_3);
        assert_eq!(
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                requires: vec![Box::new(Dependency {
                    path: test_file_dep_path,
                    requires: Vec::new()
                }),
                Box::new(Dependency {
                    path: test_file_second_dep_path,
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

        builder.read_mmk_files(&test_file_path).unwrap();

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

        assert_eq!(builder.mmk_data[2], expected_1);
        assert_eq!(builder.mmk_data[1], expected_2);
        assert_eq!(builder.mmk_data[0], expected_3);
        assert_eq!(
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                requires: vec![Box::new(Dependency {
                    path: test_file_dep_path,
                    requires: vec![
                        Box::new(Dependency {
                            path: test_file_second_dep_path,
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

        builder.read_mmk_files(&test_file_path).unwrap();

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

        assert_eq!(builder.mmk_data[3], expected_1);    
        assert_eq!(builder.mmk_data[2], expected_2);
        assert_eq!(builder.mmk_data[1], expected_3);
        assert_eq!(builder.mmk_data[0], expected_4);
        assert_eq!(
            builder.mmk_dependencies.last(),
            Some(&Dependency {
                path: test_file_path,
                requires: vec![Box::new(Dependency {
                    path: test_file_third_dep_path,
                    requires: vec![]
                }),
                Box::new(Dependency {
                    path: test_file_dep_path,
                    requires: vec![
                        Box::new(Dependency {
                            path: test_file_second_dep_path,
                            requires: vec![]
                        })]
                })]
            })
        );
        Ok(())
    }
}
