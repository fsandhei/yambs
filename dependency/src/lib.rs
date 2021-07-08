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

                let mmk_path = std::path::PathBuf::from(path).canonicalize().unwrap();
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


    pub fn get_pretty_name(&self) -> String {
        if self.is_executable() {
            return self.mmk_data().to_string("MMK_EXECUTABLE");
        }
        else {
            return self.print_library_name();
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
#[path = "./lib_test.rs"]
mod lib_test;
