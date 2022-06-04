use std::rc::Rc;
use std::{cell::RefCell, path};

use crate::errors::DependencyError;
use crate::mmk_parser;
use crate::utility;

mod associated_files;
mod dependency_accessor;
mod dependency_registry;
mod dependency_state;

pub use associated_files::{AssociatedFiles, SourceFile};
pub use dependency_accessor::DependencyAccessor;
pub use dependency_registry::DependencyRegistry;
pub use dependency_state::DependencyState;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dependency {
    path: std::path::PathBuf,
    mmk_data: mmk_parser::Mmk,
    requires: Vec<DependencyNode>,
    library_name: String,
    state: DependencyState,
    associated_files: AssociatedFiles,
}

impl Dependency {
    pub fn from(path: &std::path::Path) -> Dependency {
        let source_path: std::path::PathBuf;
        if path.ends_with("run.mmk") || path.ends_with("lib.mmk") {
            source_path = path.to_owned();
        } else {
            source_path = utility::get_mmk_library_file_from_path(path).unwrap();
        }

        Dependency {
            path: std::path::PathBuf::from(source_path),
            mmk_data: mmk_parser::Mmk::new(&path),
            requires: Vec::new(),
            library_name: String::new(),
            state: DependencyState::new(),
            associated_files: AssociatedFiles::new(),
        }
    }

    pub fn change_state(&mut self, to_state: DependencyState) {
        self.state = to_state;
    }

    pub fn create_dependency_from_path(
        path: &std::path::Path,
        dep_registry: &mut DependencyRegistry,
    ) -> Result<DependencyNode, DependencyError> {
        let dependency_node = DependencyNode::new(Dependency::from(path));
        dep_registry.add_dependency(dependency_node.clone());
        dependency_node
            .dependency_mut()
            .ref_dep
            .change_state(DependencyState::InProcess);
        dependency_node
            .dependency_mut()
            .ref_dep
            .read_and_add_mmk_data()?;
        dependency_node.dependency_mut().ref_dep.add_library_name();
        dependency_node
            .dependency_mut()
            .ref_dep
            .populate_associated_files()?;

        let dep_vec = dependency_node
            .dependency()
            .ref_dep
            .detect_dependency(dep_registry)?;

        for dep in dep_vec {
            dependency_node.dependency_mut().ref_dep.add_dependency(dep);
        }

        dependency_node
            .dependency_mut()
            .ref_dep
            .change_state(DependencyState::Registered);
        Ok(dependency_node)
    }

    pub fn add_dependency(&mut self, dependency: DependencyNode) {
        self.requires.push(dependency);
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

    pub fn get_project_name(&self) -> &std::path::Path {
        let parent = self.path.parent().unwrap();
        if utility::is_source_directory(parent) || utility::is_test_directory(parent) {
            return utility::get_head_directory(parent.parent().unwrap());
        } else {
            return utility::get_head_directory(parent);
        }
    }

    pub fn get_parent_directory(&self) -> &std::path::Path {
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

    pub fn requires(&self) -> &Vec<DependencyNode> {
        &self.requires
    }

    pub fn path(&self) -> &path::PathBuf {
        &self.path
    }

    fn detect_cycle_from_dependency(
        &self,
        dependency: &DependencyNode,
    ) -> Result<(), DependencyError> {
        if dependency.dependency().ref_dep.is_in_process() {
            return Err(DependencyError::Circulation(
                dependency.dependency().ref_dep.path().into(),
                self.path.to_path_buf(),
            ));
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

    fn populate_associated_files(&mut self) -> Result<(), DependencyError> {
        if self.mmk_data.data().contains_key("MMK_SOURCES") {
            self.populate_associated_files_by_keyword("MMK_SOURCES")?;
        }
        if self.mmk_data.data().contains_key("MMK_HEADERS") {
            self.populate_associated_files_by_keyword("MMK_HEADERS")?;
        }
        Ok(())
    }

    fn populate_associated_files_by_keyword(
        &mut self,
        mmk_keyword: &str,
    ) -> Result<(), DependencyError> {
        for keyword in &self.mmk_data.data()[mmk_keyword] {
            if keyword.argument() == "" {
                break;
            }
            let root = self.path.parent().unwrap();
            let source_file = root.join(keyword.argument());
            self.associated_files
                .push(SourceFile::new(&source_file).map_err(DependencyError::AssociatedFile)?);
        }
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DependencyNode(Rc<RefCell<Dependency>>);

impl DependencyNode {
    pub fn new(dependency: Dependency) -> Self {
        Self {
            0: Rc::new(RefCell::new(dependency)),
        }
    }

    pub fn dependency(&self) -> RefDependencyWrapper<'_> {
        RefDependencyWrapper {
            ref_dep: self.0.borrow(),
        }
    }

    pub fn dependency_mut(&self) -> MutRefDependencyWrapper<'_> {
        MutRefDependencyWrapper {
            ref_dep: self.0.borrow_mut(),
        }
    }

    pub fn try_borrow(&self) -> Result<std::cell::Ref<'_, Dependency>, std::cell::BorrowError> {
        self.0.try_borrow()
    }
}

pub struct RefDependencyWrapper<'a> {
    pub ref_dep: std::cell::Ref<'a, Dependency>,
}

pub struct MutRefDependencyWrapper<'a> {
    pub ref_dep: std::cell::RefMut<'a, Dependency>,
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
