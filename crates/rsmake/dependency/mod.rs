use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::{cell::RefCell, path};

use crate::errors::DependencyError;
use crate::mmk_parser;
use crate::utility;

mod dependency_accessor;
mod dependency_registry;
mod dependency_state;

pub use dependency_accessor::DependencyAccessor;
pub use dependency_registry::DependencyRegistry;
pub use dependency_state::DependencyState;

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
    pub fn from(path: &std::path::Path) -> Dependency {
        let source_path: PathBuf;
        if path.ends_with("run.mmk") || path.ends_with("lib.mmk") {
            source_path = path.to_owned();
        } else {
            source_path = utility::get_mmk_library_file_from_path(path).unwrap();
        }

        Dependency {
            path: std::path::PathBuf::from(source_path),
            mmk_data: mmk_parser::Mmk::new(&path),
            requires: RefCell::new(Vec::new()),
            library_name: String::new(),
            state: DependencyState::new(),
        }
    }

    fn change_state(&mut self, to_state: DependencyState) {
        self.state = to_state;
    }

    pub fn create_dependency_from_path(
        path: &std::path::Path,
        dep_registry: &mut DependencyRegistry,
    ) -> Result<DependencyNode, DependencyError> {
        let dependency = Rc::new(RefCell::new(Dependency::from(path)));
        dep_registry.add_dependency(Rc::clone(&dependency));
        dependency
            .borrow_mut()
            .change_state(DependencyState::InProcess);
        dependency.borrow_mut().read_and_add_mmk_data()?;
        dependency.borrow_mut().add_library_name();

        let dep_vec = dependency.borrow().detect_dependency(dep_registry)?;

        for dep in dep_vec {
            dependency.borrow_mut().add_dependency(dep);
        }

        dependency
            .borrow_mut()
            .change_state(DependencyState::Registered);
        Ok(dependency)
    }

    pub fn num_of_dependencies(&self) -> usize {
        let underlying_dependencies_sum = {
            let mut underlying_dependencies_sum: usize = 0;
            for dep in self.requires.borrow().iter() {
                underlying_dependencies_sum += dep.borrow().num_of_dependencies();
            }
            underlying_dependencies_sum
        };
        self.requires.borrow().len() + underlying_dependencies_sum
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

    pub fn makefile_made(self: &mut Self) {
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
        if utility::is_source_directory(parent) || utility::is_test_directory(parent) {
            return utility::get_head_directory(parent.parent().unwrap());
        } else {
            return utility::get_head_directory(parent);
        }
    }

    pub fn get_parent_directory(&self) -> &Path {
        self.path.parent().unwrap()
    }

    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, DependencyError> {
        let file_content = utility::read_file(&self.path)?;
        let mut mmk_data = mmk_parser::Mmk::new(&self.path);
        mmk_data.parse(&file_content)?;
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }

    pub fn library_file_name(&self) -> String {
        if self.mmk_data.has_library_label() {
            return format!("lib{}.a", self.library_name());
        } else {
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
            .to_str()
            .unwrap()
            .to_string();
        self.library_name.push_str("lib");
        self.library_name.push_str(&library_name);
        self.library_name.push_str(".a");
    }

    pub fn mmk_data(&self) -> &mmk_parser::Mmk {
        &self.mmk_data
    }

    #[allow(unused)]
    pub fn mmk_data_mut(&mut self) -> &mut mmk_parser::Mmk {
        &mut self.mmk_data
    }

    pub fn library_name(&self) -> String {
        self.library_name.clone()
    }

    fn print_library_name(&self) -> String {
        if self.mmk_data().has_library_label() {
            return self.mmk_data().to_string("MMK_LIBRARY_LABEL");
        } else {
            return self.library_name();
        }
    }

    pub fn get_pretty_name(&self) -> String {
        if self.is_executable() {
            return self.mmk_data().to_string("MMK_EXECUTABLE");
        } else {
            return self.print_library_name();
        }
    }

    pub fn requires(&self) -> &RefCell<Vec<DependencyNode>> {
        &self.requires
    }

    pub fn path(&self) -> &path::PathBuf {
        &self.path
    }

    fn detect_cycle_from_dependency(
        &self,
        dependency: &DependencyNode,
    ) -> Result<(), DependencyError> {
        if dependency.borrow().is_in_process() {
            if dependency.borrow().is_in_process() {
                return Err(DependencyError::Circulation(
                    dependency.borrow().path().into(),
                    self.path.to_path_buf(),
                ));
            }
        }
        Ok(())
    }

    fn detect_dependency(
        &self,
        dep_registry: &mut DependencyRegistry,
    ) -> Result<Vec<DependencyNode>, DependencyError> {
        let mut dep_vec: Vec<DependencyNode> = Vec::new();
        if self.mmk_data().has_dependencies() {
            for keyword in self.mmk_data().data()["MMK_REQUIRE"].clone() {
                if keyword.argument() == "" {
                    break;
                }

                let mmk_path = std::path::PathBuf::from(keyword.argument())
                    .canonicalize()
                    .unwrap();
                let dep_path = &mmk_path.join("lib.mmk");

                if let Some(dependency) = dep_registry.dependency_from_path(&dep_path) {
                    self.detect_cycle_from_dependency(&dependency)?;
                    dep_vec.push(dependency);
                } else {
                    let dependency =
                        Dependency::create_dependency_from_path(&mmk_path, dep_registry)?;
                    dep_vec.push(dependency);
                }
            }
        }
        Ok(dep_vec)
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
