use error::MyMakeError;
use mmk_parser;
use std::{cell::RefCell, path};
use std::rc::Rc;

pub struct DependencyRegistry {
    pub registry: Vec<Dependency>,
}

impl DependencyRegistry {

    pub fn new() -> DependencyRegistry {
        DependencyRegistry {
            registry: Vec::new(),
        }
    }

    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        // let rc_dependency = Rc::new(dependency);
        self.registry.push(dependency);
    }

    pub fn dependency_in_registry(self: &Self, dependency: Dependency) -> bool {
        self.registry.contains(&Rc::new(dependency))
    }

    pub fn dependency_from_path(self: &Self, path: &std::path::PathBuf) -> Option<&Dependency> {
        for dependency in &self.registry {
            if &dependency.path == path {
                return Some(dependency)
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dependency {
     path: std::path::PathBuf,
     mmk_data: mmk_parser::Mmk,
     requires: RefCell<Vec<RefCell<Dependency>>>,
     makefile_made: bool,
     library_name: String,
     in_process: bool
}

impl Dependency {
    pub fn new() -> Dependency {
        Dependency {
            path: std::path::PathBuf::new(),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            makefile_made: false,
            library_name: String::new(),
            in_process: false
        }
    }


    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            makefile_made: false,
            library_name: String::new(),
            in_process: false,
        }
    }


    pub fn create_dependency_from_path(path: &std::path::PathBuf,
                                       dep_registry: &mut DependencyRegistry) -> Result<Dependency, MyMakeError>{                      
        let mut dependency = Dependency::from(path);        
        dependency.in_process = true;
        dep_registry.add_dependency(dependency.to_owned());
        dependency.read_and_add_mmk_data()?;
        dependency.add_library_name();
        dependency.detect_and_add_dependencies(dep_registry)?;
        dependency.print_ok();
        dependency.in_process = false;
        Ok(dependency)
    }


    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.borrow_mut().push(RefCell::new(dependency));
    }


    pub fn is_makefile_made(&self) -> bool {
        self.makefile_made
    }


    pub fn is_executable(&self) -> bool {
        &self.mmk_data().data["MMK_EXECUTABLE"][0] != ""
    }


    pub fn makefile_made(self: &mut Self)
    {
        self.makefile_made = true;
    }


    pub fn get_build_directory(self: &Self) -> std::path::PathBuf {
        let parent = self.path.parent().unwrap();
        let build_directory_name = std::path::PathBuf::from(".build");
        parent.join(build_directory_name)
    }


    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, MyMakeError>{
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", self.path, err))),
        };
        let mut mmk_data = mmk_parser::Mmk::new();
        mmk_data.parse_file(&file_content)?;
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }


    pub fn add_library_name(self: &mut Self) {
        let root_path = self.path.parent().unwrap().parent().unwrap();
        let prefix = root_path.parent().unwrap();
        let library_name = root_path.strip_prefix(prefix).unwrap().to_str().unwrap();
        self.library_name.push_str("lib");
        self.library_name.push_str(library_name);
        self.library_name.push_str(".a");
    }

    pub fn mmk_data(&self) -> &mmk_parser::Mmk {
        &self.mmk_data
    }

    pub fn mmk_data_mut(&mut self) -> &mut mmk_parser::Mmk {
        &mut self.mmk_data
    }

    pub fn library_name(&self) -> String{
        self.library_name.clone()
    }

    pub fn requires(&self) -> &RefCell<Vec<RefCell<Dependency>>> {
        &self.requires
    }

    pub fn path(&self) -> &path::PathBuf {
        &self.path
    }
 

    pub fn detect_and_add_dependencies(self: &mut Self, dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError>{
        for path in self.mmk_data.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");

            self.detect_cycle_dependency_from_path(&dep_path, dep_registry)?;
            let dependency = Dependency::create_dependency_from_path(&dep_path, dep_registry)?;       
            self.add_dependency(dependency);
        }
        Ok(())
    }

    
    fn detect_cycle_dependency_from_path(self: &Self, path: &std::path::PathBuf, 
                                             dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError> {
        if let Some(dependency) = dep_registry.dependency_from_path(path) {
            if dependency.in_process == true {
                return Err(MyMakeError::from(format!("Error: dependency circulation!\n{:?} depends on\n{:?}, which depends on itself", 
                                             path, self.path)));
                }
            }
        Ok(())
        }


    pub fn print_ok(self: &Self) {
        print!(".");
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

        (dir, test_file_path, file, mmk_data)
    }
    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let (_dir, test_file_path, mut file, mut expected) = make_mmk_file("example");
        
        write!(
            file,
            "\
        MMK_EXECUTABLE = x"
        )?;
        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(top_dependency.mmk_data(), &expected);
        Ok(())
    }


    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert_eq!(
            top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::from("libtmp.a"),
                in_process: false,
            }
        );
        Ok(())
    }

    #[test]
    fn add_library_name_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/test/mymakeinfo.mmk"));
        dependency.add_library_name();
        assert_eq!(dependency.library_name(), String::from("libdirectory.a"));
    }


    #[test]
    fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data.insert(
            String::from("MMK_DEPEND"),
            vec![dir_dep.path().to_str().unwrap().to_string(),
                 second_dir_dep.path().to_str().unwrap().to_string()],
        );
        expected_1
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        assert_eq!(
            top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
                    in_process: false,
                }),
                RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::from("libtmp.a"),
                in_process: false,
            }
        );
        Ok(())
    }


    #[test]
    fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

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
            top_dependency,
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
                            library_name: String::from("libtmp.a"),
                            in_process: false,
                        })]),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::from("libtmp.a"),
                in_process: false,
            }
        );
        Ok(())
    }


    #[test]
    fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

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
            top_dependency,
            Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![RefCell::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(vec![]),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
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
                            library_name: String::from("libtmp.a"),
                            in_process: false,
                        })]),
                    makefile_made: false,
                    library_name: String::from("libtmp.a"),
                    in_process: false,
                })]),
                makefile_made: false,
                library_name: String::from("libtmp.a"),
                in_process: false,
            }
        );
        Ok(())
    }


    #[test]
    fn read_mmk_files_two_files_circulation() -> Result<(), MyMakeError> {
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

        assert!(top_dependency.is_err());
        Ok(())
    }
}