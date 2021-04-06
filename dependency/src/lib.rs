use error::MyMakeError;
use mmk_parser;
use std::{cell::RefCell, path};
use std::rc::Rc;

mod dependency_registry;
mod dependency_state;
pub use crate::dependency_registry::DependencyRegistry;
pub use crate::dependency_state::DependencyState;


#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dependency {
     path: std::path::PathBuf,
     mmk_data: mmk_parser::Mmk,
     requires: RefCell<Vec<DependencyNode>>,
     library_name: String,
     state: DependencyState,
}

pub type DependencyNode = Rc<RefCell<Dependency>>;

impl Dependency {    
    pub fn new() -> Dependency {
        Dependency {
            path: std::path::PathBuf::new(),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            library_name: String::new(),
            state: DependencyState::new()
        }
    }


    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            library_name: String::new(),
            state: DependencyState::new()
        }
    }


    fn change_state(&mut self, to_state: DependencyState) {
        self.state = to_state;
    }


    pub fn create_dependency_from_path(path: &std::path::PathBuf,
                                       dep_registry: &mut DependencyRegistry) -> Result<DependencyNode, MyMakeError>{
        let dependency = Rc::new(RefCell::new(Dependency::from(path)));
        dep_registry.add_dependency(Rc::clone(&dependency));
        dependency.borrow_mut().change_state(DependencyState::InProcess);
        dependency.borrow_mut().read_and_add_mmk_data()?;
        dependency.borrow_mut().add_library_name();

        let dep_vec = dependency.borrow().detect_dependency(dep_registry)?;
        
        for dep in dep_vec {
            dependency.borrow_mut().add_dependency(dep);
        }

        dependency.borrow().print_ok();
        dependency.borrow_mut().change_state(DependencyState::Registered);
        Ok(dependency)
    }

    
    fn detect_dependency(&self, dep_registry: &mut DependencyRegistry) -> Result<Vec<DependencyNode>, MyMakeError> {
        let mut dep_vec : Vec<DependencyNode> = Vec::new();
        if self.mmk_data().data.contains_key("MMK_DEPEND") {
            for path in self.mmk_data().data["MMK_DEPEND"].clone() {
                if path == "" {
                    break;
                }

                let mmk_path = path;
                let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");
                
                if let Some(dependency) = dep_registry.dependency_from_path(&dep_path) {
                    self.detect_cycle_from_dependency(&dependency)?;
                    dep_vec.push(dependency);
                }

                else {
                    let dependency = Dependency::create_dependency_from_path(&dep_path, dep_registry)?;
                    dep_vec.push(dependency);
                }
            }
        }
        Ok(dep_vec)
    }


    #[allow(dead_code)]
    fn process(&mut self, dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError> {
        self.change_state(DependencyState::InProcess);
        self.read_and_add_mmk_data()?;
        self.add_library_name();

        let dep_vec = self.detect_dependency(dep_registry)?;
        
        for dep in dep_vec {
            self.add_dependency(dep);
        }

        self.print_ok();
        self.change_state(DependencyState::Registered);
        Ok(())
    }


    pub fn add_dependency(self: &mut Self, dependency: DependencyNode) {
        self.requires.borrow_mut().push(dependency);
    }


    pub fn is_makefile_made(&self) -> bool {
        self.state == DependencyState::MakefileMade
    }


    pub fn is_in_process(&self) -> bool {
        self.state == DependencyState::InProcess
    }


    pub fn is_building(&self) -> bool {
        self.state == DependencyState::Building
    }


    pub fn build_completed(&self) -> bool {
        self.state == DependencyState::BuildComplete
    }


    pub fn is_executable(&self) -> bool {
        self.mmk_data().data.contains_key("MMK_EXECUTABLE")
    }


    pub fn makefile_made(self: &mut Self)
    {
        self.change_state(DependencyState::MakefileMade);
    }


    pub fn building(&mut self) {
        self.change_state(DependencyState::Building);
    }


    pub fn build_complete(&mut self) {
        self.change_state(DependencyState::BuildComplete);
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
        mmk_data.parse(&file_content)?;
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }


    pub fn add_library_name(self: &mut Self) {
        let root_path = self.path.parent().unwrap().parent().unwrap();
        let prefix = root_path.parent().unwrap();
        let library_name: String;

        if self.mmk_data.has_library_label() {
            library_name = self.mmk_data.to_string("MMK_LIBRARY_LABEL");
        }
        else {
            library_name = root_path.strip_prefix(prefix).unwrap().to_str().unwrap().to_string();
        }
        self.library_name.push_str("lib");
        self.library_name.push_str(&library_name);
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

    pub fn requires(&self) -> &RefCell<Vec<DependencyNode>> {
        &self.requires
    }

    pub fn path(&self) -> &path::PathBuf {
        &self.path
    }
 

    pub fn detect_and_add_dependencies(&mut self, dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError>{
        if self.mmk_data.data.contains_key("MMK_DEPEND") {
            for path in self.mmk_data.data["MMK_DEPEND"].clone() {
                if path == "" {
                    break;
                }
                let mmk_path = path.clone();
                let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");

                if let Some(dependency) = dep_registry.dependency_from_path(&dep_path) {
                    self.add_dependency(dependency)
                }

                else {
                    self.detect_cycle_dependency_from_path(&dep_path, dep_registry)?;
                    let dependency = Dependency::create_dependency_from_path(&dep_path, dep_registry)?;
                    self.add_dependency(dependency);
                }                
            }
        }
        Ok(())
    }
    

    fn detect_cycle_dependency_from_path(&self, path: &std::path::PathBuf, 
                                             dep_registry: &DependencyRegistry) -> Result<(), MyMakeError> {
        if let Some(dependency) = dep_registry.dependency_from_path(path) {
            if dependency.borrow().is_in_process() {
                return Err(MyMakeError::from(format!("Error: dependency circulation!\n{:?} depends on\n{:?}, which depends on itself", 
                                             path, self.path)));
                }
            }
        Ok(())
    }


    fn detect_cycle_from_dependency(&self, dependency: &DependencyNode) -> Result<(), MyMakeError>{
        if dependency.borrow().is_in_process() {
            if dependency.borrow().is_in_process() {
                return Err(MyMakeError::from(format!("Error: dependency circulation!\n{:?} depends on\n{:?}, which depends on itself", 
                                             dependency.borrow().path(), self.path)));
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
    use pretty_assertions::assert_eq;

    fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
        let dir: TempDir = TempDir::new(&dir_name).unwrap();
        let test_file_path = dir.path().join("mymakeinfo.mmk");
        let mut file = File::create(&test_file_path)
                                .expect("make_mmk_file(): Something went wrong writing to file.");
        write!(file, 
        "MMK_SOURCES:
            some_file.cpp
            some_other_file.cpp
        \n
        MMK_HEADERS:
            some_file.h
            some_other_file.h
        
        \n").expect("make_mmk_file(): Something went wrong writing to file.");

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
    fn test_is_in_process_true() {
        let mut dependency = Dependency::new();
        dependency.change_state(DependencyState::InProcess);
        assert!(dependency.is_in_process());
    }


    #[test]
    fn test_is_in_process_false() {
        let mut dependency = Dependency::new();
        dependency.change_state(DependencyState::Registered);
        assert!(!dependency.is_in_process());
    }


    #[test]
    fn test_is_makefile_made_true() {
        let mut dependency = Dependency::new();
        dependency.change_state(DependencyState::MakefileMade);
        assert!(dependency.is_makefile_made());
    }


    #[test]
    fn test_is_makefile_made_false() {
        let mut dependency = Dependency::new();
        dependency.change_state(DependencyState::NotInProcess);
        assert!(!dependency.is_makefile_made());
    }


    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let (_dir, test_file_path, mut file, mut expected) = make_mmk_file("example");
        
        write!(
            file,
            "MMK_EXECUTABLE:
                x"
        )?;
        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();
        expected
            .data
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(top_dependency.borrow().mmk_data(), &expected);
        Ok(())
    }


    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
        let (_dir, test_file_path, mut file, mut expected_1)     = make_mmk_file("example");
        let (dir_dep, test_file_dep_path, _file_dep, expected_2) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_DEPEND:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
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
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                }))]),
                library_name: String::from("libtmp.a"),
                state: DependencyState::Registered,
            }))
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
    fn add_library_name_from_label_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/test/mymakeinfo.mmk"));
        dependency.mmk_data_mut().data.insert(String::from("MMK_LIBRARY_LABEL"), vec!["mylibrary".to_string()]);
        dependency.add_library_name();
        assert_eq!(dependency.library_name(), String::from("libmylibrary.a"));
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
        MMK_DEPEND:
            {}
            {}
        
        \n
        MMK_EXECUTABLE:
            x",
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
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                }))]),
                library_name: String::from("libtmp.a"),
                state: DependencyState::Registered,
            }))
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
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![
                        Rc::new(RefCell::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_3,
                            requires: RefCell::new(vec![]),
                            library_name: String::from("libtmp.a"),
                            state: DependencyState::Registered,
                        }))]),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                }))]),
                library_name: String::from("libtmp.a"),
                state: DependencyState::Registered,
            }))
        );
        Ok(())
    }



    #[test]
    fn read_mmk_files_three_files_one_common_dependency() -> std::io::Result<()> {
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
            {}
        \n
        MMK_EXECUTABLE:
            x",
            &dir_dep.path().to_str().unwrap().to_string(),
            &second_dir_dep.path().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_DEPEND:
            {}
        \n
        ",
        &second_dir_dep.path().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let result = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

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

        assert!(result.is_ok());
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
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(vec![]),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![
                        Rc::new(RefCell::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_4,
                            requires: RefCell::new(vec![]),
                            library_name: String::from("libtmp.a"),
                            state: DependencyState::Registered,
                        }))]),
                    library_name: String::from("libtmp.a"),
                    state: DependencyState::Registered,
                }))]),
                library_name: String::from("libtmp.a"),
                state: DependencyState::Registered,
            }))
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

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

        assert!(top_dependency.is_err());
        Ok(())
    }

    #[test]
    fn read_mmk_files_four_files_one_dependency_serial_and_one_circular_serial() -> std::io::Result<()> {
        let (dir, test_file_path, mut file, _expected_1) 
            = make_mmk_file("example");
        let (dir_dep, _test_file_dep_path, mut file_dep, _expected_2) 
            = make_mmk_file("example_dep");
        let (second_dir_dep, _test_file_second_dep_path, mut file_second_file_dep, _expected_3) 
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

        write!(
            file_second_file_dep,
            "\
        MMK_DEPEND:
            {}
        \n
        ",
        &dir.path().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);
        assert!(top_dependency.is_err());
        Ok(())
    }
}