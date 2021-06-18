use error::MyMakeError;
use mmk_parser;
use utility;
use std::{cell::RefCell, path};
use std::path::{PathBuf, Path};
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
    pub fn from(path: &std::path::PathBuf) -> Dependency {
        let source_path : PathBuf;
        if path.ends_with("run.mmk") || path.ends_with("lib.mmk") {
            source_path = path.to_owned();
        }
        else {
            source_path = utility::get_mmk_library_file_from_path(path).unwrap();
        }
        
        Dependency {
            path: std::path::PathBuf::from(source_path),
            mmk_data: mmk_parser::Mmk::new(&path),
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
        if self.mmk_data().has_dependencies() {
            for path in self.mmk_data().data()["MMK_REQUIRE"].clone() {
                if path == "" {
                    break;
                }

                let mmk_path = std::path::PathBuf::from(path);
                let dep_path = &mmk_path.join("lib.mmk");
                
                if let Some(dependency) = dep_registry.dependency_from_path(&dep_path) {
                    self.detect_cycle_from_dependency(&dependency)?;
                    dep_vec.push(dependency);
                }

                else {
                    let dependency = Dependency::create_dependency_from_path(&mmk_path, dep_registry)?;
                    dep_vec.push(dependency);
                }
            }
        }
        Ok(dep_vec)
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


    pub fn is_build_completed(&self) -> bool {
        self.state == DependencyState::BuildComplete
    }


    pub fn is_executable(&self) -> bool {
        self.mmk_data().has_executables()
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


    pub fn get_project_name(&self) -> &std::path::Path {
        let parent = self.path.parent().unwrap();
        if utility::is_source_directory(parent) || 
           utility::is_test_directory(parent){
            return utility::get_head_directory(parent.parent().unwrap());
        }
        else {
            return utility::get_head_directory(parent);
        }
    }


    pub fn get_parent_directory(&self) -> &Path {
        self.path.parent().unwrap()
    }


    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, MyMakeError>{
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", self.path, err))),
        };
        let mut mmk_data = mmk_parser::Mmk::new(&self.path);
        mmk_data.parse(&file_content)?;
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }

    
    pub fn library_file_name(&self) -> String {
        if self.mmk_data.has_library_label() {
            return format!("lib{}.a", self.library_name());
        }
        else {
            return self.library_name();
        }
    }


    pub fn add_library_name(self: &mut Self) {
        let library_name: String;

        if self.mmk_data.has_library_label() {
            library_name = self.mmk_data.to_string("MMK_LIBRARY_LABEL");
            self.library_name = library_name;
            return;
        }
        let root_path = self.path.parent().unwrap().parent().unwrap();
        library_name = utility::get_head_directory(root_path)
                                                    .to_str().unwrap().to_string();
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

    
    pub fn library_name(&self) -> String {
        self.library_name.clone()
    }


    pub fn print_library_name(&self) -> String {
        if self.mmk_data().has_library_label() {
            return self.mmk_data().to_string("MMK_LIBRARY_LABEL");
        }
        else {
            return self.library_name();
        }
    }


    pub fn requires(&self) -> &RefCell<Vec<DependencyNode>> {
        &self.requires
    }

    
    pub fn path(&self) -> &path::PathBuf {
        &self.path
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


    fn print_ok(self: &Self) {
        print!(".");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use utility;
    use mmk_parser::Mmk;
    use std::fs::File;
    use std::io::Write;
    use tempdir::TempDir;
    use std::cell::RefCell;
    use pretty_assertions::assert_eq;

    #[allow(dead_code)]
    fn expected_library_name(path: &std::path::Path) -> String {
        let mut library_name = String::from("lib");
        library_name.push_str(utility::get_head_directory(path).to_str().unwrap());
        library_name.push_str(".a");
        library_name
    }

    fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
        let dir: TempDir = TempDir::new(&dir_name).unwrap();
        let source_dir = dir.path().join("source");
        utility::create_dir(&source_dir).unwrap();
        let test_file_path = source_dir.join("lib.mmk");
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

        let mut mmk_data = Mmk::new(&test_file_path);
        mmk_data.data_mut().insert(String::from("MMK_SOURCES"), 
                             vec![String::from("some_file.cpp"), 
                                  String::from("some_other_file.cpp")]);
        
        mmk_data.data_mut().insert(String::from("MMK_HEADERS"), 
                             vec![String::from("some_file.h"), 
                                  String::from("some_other_file.h")]);

        (dir, test_file_path, file, mmk_data)
    }


    fn fixture_simple_dependency() -> DependencyNode {
        let (_dir, lib_file_path, _file, _expected) = make_mmk_file("example");
        let mut dep_registry = DependencyRegistry::new();
        Dependency::create_dependency_from_path(&lib_file_path, &mut dep_registry).unwrap()
    }

    
    #[test]
    fn test_is_in_process_true() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::InProcess);
        assert!(dependency.borrow().is_in_process());
    }


    #[test]
    fn test_is_in_process_false() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::Registered);
        assert!(!dependency.borrow().is_in_process());
    }


    #[test]
    fn test_is_makefile_made_true() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::MakefileMade);
        assert!(dependency.borrow().is_makefile_made());
    }


    #[test]
    fn test_is_makefile_made_false() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::NotInProcess);
        assert!(!dependency.borrow().is_makefile_made());
    }



    #[test]
    fn test_is_building_true() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::Building);
        assert!(dependency.borrow().is_building());
    }


    #[test]
    fn test_is_building_false() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::BuildComplete);
        assert!(!dependency.borrow().is_building());
    }


    #[test]
    fn test_is_build_completed_true() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::BuildComplete);
        assert!(dependency.borrow().is_build_completed());
    }


    #[test]
    fn test_is_build_completed_false() {
        let dependency = fixture_simple_dependency();
        dependency.borrow_mut().change_state(DependencyState::NotInProcess);
        assert!(!dependency.borrow().is_build_completed());
    }


    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()> {
        let (_dir, lib_file_path, mut file, mut expected) = make_mmk_file("example");
        
        write!(
            file,
            "MMK_EXECUTABLE:
                x"
        )?;
        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&lib_file_path, &mut dep_registry).unwrap();
        expected
            .data_mut()
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert_eq!(top_dependency.borrow().mmk_data(), &expected);
        Ok(())
    }


    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()> {
        let (dir, test_file_path, mut file, mut expected_1)     = make_mmk_file("example");
        let (dir_dep, test_file_dep_path, _file_dep, expected_2) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_REQUIRE:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string()
        )?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data_mut().insert(
            String::from("MMK_REQUIRE"),
            vec![test_file_dep_path.parent().unwrap().to_str().unwrap().to_string()],
        );
        expected_1
            .data_mut()
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        let expected_lib_name = expected_library_name(&dir.path());
        let expected_lib_name_dep = expected_library_name(&dir_dep.path());
        assert_eq!(
            top_dependency,
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                }))]),
                library_name: expected_lib_name,
                state: DependencyState::Registered,
            }))
        );
        Ok(())
    }

    
    #[test]
    fn add_library_name_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
        dependency.add_library_name();
        assert_eq!(dependency.library_name(), String::from("libdirectory.a"));
    }

    #[test]
    fn add_library_name_from_label_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
        dependency.mmk_data_mut().data_mut().insert(String::from("MMK_LIBRARY_LABEL"), vec!["mylibrary".to_string()]);
        dependency.add_library_name();
        assert_eq!(dependency.library_name(), String::from("mylibrary"));
    }


    #[test]
    fn library_file_name_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
        dependency.add_library_name();
        assert_eq!(dependency.library_file_name(), String::from("libdirectory.a"));
    }


    #[test]
    fn library_file_name_from_label_test() {
        let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
        dependency.mmk_data_mut().data_mut().insert(String::from("MMK_LIBRARY_LABEL"), vec!["mylibrary".to_string()]);
        dependency.add_library_name();
        assert_eq!(dependency.library_file_name(), String::from("libmylibrary.a"));
    }


    #[test]
    fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
        let (dir, test_file_path, mut file, mut expected_1) 
            = make_mmk_file("example");
        let (dir_dep, test_file_dep_path, _file_dep, expected_2) 
            = make_mmk_file("example_dep");
        let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) 
            = make_mmk_file("example_dep");

        write!(
            file,
            "\
        MMK_REQUIRE:
            {}
            {}
        
        \n
        MMK_EXECUTABLE:
            x",
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string(),
            &test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string()
        )?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data_mut().insert(
            String::from("MMK_REQUIRE"),
            vec![test_file_dep_path.parent().unwrap().to_str().unwrap().to_string(),
                 test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string()],
        );
        expected_1
            .data_mut()
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        let expected_lib_name = expected_library_name(&dir.path());
        let expected_lib_name_dep = expected_library_name(&dir_dep.path());
        let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());
        
        assert_eq!(
            top_dependency,
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    library_name: expected_lib_name_second_dep,
                    state: DependencyState::Registered,
                }))]),
                library_name: expected_lib_name,
                state: DependencyState::Registered,
            }))
        );
        Ok(())
    }


    #[test]
    fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
        let (dir, test_file_path, mut file, mut expected_1) 
        = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut expected_2) 
        = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) 
        = make_mmk_file("example_dep_second");

        write!(
            file,
            "\
        MMK_REQUIRE:
            {}
        \n
        MMK_EXECUTABLE:
            x",
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data_mut().insert(
            String::from("MMK_REQUIRE"),
            vec![test_file_dep_path.parent().unwrap().to_str().unwrap().to_string()],
        );
        expected_1
            .data_mut()
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_2
            .data_mut()
            .insert(String::from("MMK_REQUIRE"),
            vec![test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string()]);

        
        let expected_lib_name = expected_library_name(&dir.path());
        let expected_lib_name_dep = expected_library_name(&dir_dep.path());
        let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());

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
                            library_name: expected_lib_name_second_dep,
                            state: DependencyState::Registered,
                        }))]),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                }))]),
                library_name: expected_lib_name,
                state: DependencyState::Registered,
            }))
        );
        Ok(())
    }



    #[test]
    fn read_mmk_files_three_files_one_common_dependency() -> std::io::Result<()> {
        let (_dir, test_file_path, mut file, _) 
        = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _) 
        = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) 
        = make_mmk_file("example_dep_second");

        write!(
            file,
            "\
        MMK_REQUIRE:
            {}
            {}
        \n
        MMK_EXECUTABLE:
            x",
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string(),
            &test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_REQUIRE:
            {}
        \n
        ",
        test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let result = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

        assert!(result.is_ok());
        Ok(())
    }


    #[test]
    fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
        let (dir, test_file_path, mut file, mut expected_1) 
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
        MMK_REQUIRE:
            {}
            {}
        \n
        MMK_EXECUTABLE:
            x",            
            &test_file_third_dep_path.parent().unwrap().to_str().unwrap().to_string(),
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

        expected_1.data_mut().insert(
            String::from("MMK_REQUIRE"),
            vec![test_file_third_dep_path.parent().unwrap().to_str().unwrap().to_string(),
                   test_file_dep_path.parent().unwrap().to_str().unwrap().to_string()],
        );
        expected_1
            .data_mut()
            .insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_2
            .data_mut()
            .insert(String::from("MMK_REQUIRE"),
            vec![test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string()]);
        
        
        let expected_lib_name = expected_library_name(&dir.path());
        let expected_lib_name_dep = expected_library_name(&dir_dep.path());
        let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());
        let expected_lib_name_third_dep = expected_library_name(&third_dir_dep.path());

        assert_eq!(
            top_dependency,
            Rc::new(RefCell::new(Dependency {
                path: test_file_path,
                mmk_data: expected_1,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_4,
                    requires: RefCell::new(vec![]),
                    library_name: expected_lib_name_third_dep,
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![
                        Rc::new(RefCell::new(Dependency {
                            path: test_file_second_dep_path,
                            mmk_data: expected_3,
                            requires: RefCell::new(vec![]),
                            library_name: expected_lib_name_second_dep,
                            state: DependencyState::Registered,
                        }))]),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                }))]),
                library_name: expected_lib_name,
                state: DependencyState::Registered,
            }))
        );
        Ok(())
    }


    #[test]
    fn read_mmk_files_two_files_circulation() -> Result<(), MyMakeError> {
        let (_dir, test_file_path, mut file, _expected_1)              = make_mmk_file("example");
        let (_dir_dep, test_file_dep_path, mut file_dep, _expected_2) = make_mmk_file("example_dep");

        write!(
            file,
            "\
            MMK_REQUIRE:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string()
        ).unwrap();

        write!(
            file_dep,
            "\
            MMK_REQUIRE:
                {}
        \n", &test_file_path.parent().unwrap().to_str().unwrap().to_string()
        ).unwrap();

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

        assert!(top_dependency.is_err());
        Ok(())
    }

    #[test]
    fn read_mmk_files_four_files_one_dependency_serial_and_one_circular_serial() -> std::io::Result<()> {
        let (_dir, test_file_path, mut file, _expected_1) 
            = make_mmk_file("example");
        let (_dir_dep, test_file_dep_path, mut file_dep, _expected_2) 
            = make_mmk_file("example_dep");
        let (_second_dir_dep, test_file_second_dep_path, mut file_second_file_dep, _expected_3) 
            = make_mmk_file("example_dep_second");

        write!(
            file,
            "\
        MMK_REQUIRE:
            {}
        \n
        MMK_EXECUTABLE:
            x",            
            &test_file_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        write!(
            file_dep,
            "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_second_dep_path.parent().unwrap().to_str().unwrap().to_string())?;

        write!(
            file_second_file_dep,
            "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_path.parent().unwrap().to_str().unwrap().to_string())?;

        let mut dep_registry = DependencyRegistry::new();
        let top_dependency = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);
        assert!(top_dependency.is_err());
        Ok(())
    }


    #[test]
    fn get_project_name_test() {
        let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
        let dependency = Dependency::from(&project_path);
        assert_eq!(std::path::PathBuf::from("MyProject"), dependency.get_project_name());
    }


    #[test]
    fn is_executable_test() {
        let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
        let mut dependency = Dependency::from(&project_path);
        dependency.mmk_data_mut().data_mut().insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        assert!(dependency.is_executable());
    }


    #[test]
    fn is_executable_false_test() {
        let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
        let dependency = Dependency::from(&project_path);
        assert!(!dependency.is_executable());
    }
}