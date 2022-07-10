use std::rc::Rc;
use std::{cell::RefCell, path};

use crate::errors::DependencyError;
use crate::mmk_parser;
use crate::utility;

mod associated_files;
mod dependency_accessor;
mod dependency_registry;
mod dependency_state;
mod include_directories;
pub use associated_files::{AssociatedFiles, SourceFile};
pub use dependency_accessor::DependencyAccessor;
pub use dependency_registry::DependencyRegistry;
pub use dependency_state::DependencyState;
pub use include_directories::{IncludeDirectories, IncludeType};

// Dependency class should not have a Mmk object in it.
// It should only need the path to it.
// From there on, we can fetch all the metadata from that file.
// Instead of using the mmk as an object, we should use other objects to determine
// if Dependency is an executable or library, files, etc.

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    path: std::path::PathBuf,
    modification_time: std::time::SystemTime,
    #[serde(skip)]
    requires: Vec<DependencyNode>,
    state: DependencyState,
    associated_files: AssociatedFiles,
    dependency_type: DependencyType,
    include_directories: Option<IncludeDirectories>,
    additional_flags: std::collections::HashMap<String, Vec<mmk_parser::Keyword>>,
}

impl Dependency {
    pub fn from(path: &std::path::Path) -> Dependency {
        let source_path: std::path::PathBuf;
        if path.ends_with("run.mmk") || path.ends_with("lib.mmk") {
            source_path = path.to_owned();
        } else {
            source_path = utility::get_mmk_library_file_from_path(path).unwrap();
        }
        let metadata =
            std::fs::metadata(&source_path).expect("Could not fetch metadata from mmk file.");

        Dependency {
            path: source_path,
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time."),
            requires: Vec::new(),
            state: DependencyState::new(),
            associated_files: AssociatedFiles::new(),
            dependency_type: DependencyType::None,
            include_directories: None,
            additional_flags: std::collections::HashMap::new(),
        }
    }

    pub fn from_path(
        path: &std::path::Path,
        dep_registry: &mut DependencyRegistry,
        mmk_data: &mmk_parser::Mmk,
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
            .determine_dependency_type(&mmk_data)?;
        dependency_node
            .dependency_mut()
            .ref_dep
            .populate_associated_files(&mmk_data)?;

        dependency_node.dependency_mut().ref_dep.include_directories =
            IncludeDirectories::from_mmk(&mmk_data);

        dependency_node
            .dependency_mut()
            .ref_dep
            .append_additional_flags(&mmk_data);

        let dep_vec = dependency_node
            .dependency()
            .ref_dep
            .detect_dependency(dep_registry, &mmk_data)?;

        for dep in dep_vec {
            dependency_node.dependency_mut().ref_dep.add_dependency(dep);
        }

        dependency_node
            .dependency_mut()
            .ref_dep
            .change_state(DependencyState::Registered);
        Ok(dependency_node)
    }

    fn append_additional_flags(&mut self, mmk: &mmk_parser::Mmk) {
        if let Some(cxxflags) = mmk.get_args("MMK_CXXFLAGS_APPEND") {
            self.additional_flags
                .insert("cxxflags".to_string(), cxxflags.to_owned());
        }
        if let Some(cppflags) = mmk.get_args("MMK_CPPFLAGS_APPEND") {
            self.additional_flags
                .insert("cppflags".to_string(), cppflags.to_owned());
        }
    }

    pub fn additional_keyword(&self, key: &str) -> Option<&Vec<mmk_parser::Keyword>> {
        self.additional_flags.get(key)
    }

    pub fn change_state(&mut self, to_state: DependencyState) {
        self.state = to_state;
    }

    pub fn associated_files(&self) -> &AssociatedFiles {
        &self.associated_files
    }

    pub fn include_directories(&self) -> Option<&IncludeDirectories> {
        self.include_directories.as_ref()
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
        match self.dependency_type {
            DependencyType::Executable(_) => true,
            DependencyType::Library(_) | _ => false,
        }
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

    fn determine_dependency_type(
        &mut self,
        mmk_data: &mmk_parser::Mmk,
    ) -> Result<(), DependencyError> {
        if mmk_data.has_executables() {
            self.dependency_type = DependencyType::Executable(mmk_data.to_string("MMK_EXECUTABLE"));
        } else {
            self.dependency_type = DependencyType::Library(self.add_library_name(&mmk_data));
        }
        Ok(())
    }

    pub fn library_file_name(&self) -> String {
        match &self.dependency_type {
            DependencyType::Library(library_name) => library_name.to_owned(),
            _ => panic!("Dependency is not a library"),
        }
    }

    fn add_library_name(&self, mmk_data: &mmk_parser::Mmk) -> String {
        if mmk_data.has_library_label() {
            return mmk_data
                .get_args("MMK_LIBRARY_LABEL")
                .unwrap()
                .first()
                .unwrap()
                .argument()
                .to_string();
        }
        let root_path = self.path.parent().unwrap().parent().unwrap();
        utility::get_head_directory(root_path).display().to_string()
    }

    pub fn get_name(&self) -> Option<String> {
        match self.dependency_type {
            DependencyType::Executable(ref executable) => Some(executable.to_owned()),
            DependencyType::Library(ref library) => Some(library.to_owned()),
            DependencyType::None => None,
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
        mmk_data: &mmk_parser::Mmk,
    ) -> Result<Vec<DependencyNode>, DependencyError> {
        let mut dep_vec: Vec<DependencyNode> = Vec::new();
        if mmk_data.has_dependencies() {
            for keyword in mmk_data.data()["MMK_REQUIRE"].clone() {
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
                    let file_content = utility::read_file(&dep_path)?;
                    let mut dep_mmk_data = mmk_parser::Mmk::new(&dep_path);
                    dep_mmk_data.parse(&file_content)?;
                    let dependency = Dependency::from_path(&mmk_path, dep_registry, &dep_mmk_data)?;
                    dep_vec.push(dependency);
                }
            }
        }
        Ok(dep_vec)
    }

    fn populate_associated_files(
        &mut self,
        mmk_data: &mmk_parser::Mmk,
    ) -> Result<(), DependencyError> {
        if mmk_data.data().contains_key("MMK_SOURCES") {
            self.populate_associated_files_by_keyword(&mmk_data, "MMK_SOURCES")?;
        }
        if mmk_data.data().contains_key("MMK_HEADERS") {
            self.populate_associated_files_by_keyword(&mmk_data, "MMK_HEADERS")?;
        }
        Ok(())
    }

    fn populate_associated_files_by_keyword(
        &mut self,
        mmk_data: &mmk_parser::Mmk,
        mmk_keyword: &str,
    ) -> Result<(), DependencyError> {
        for keyword in &mmk_data.data()[mmk_keyword] {
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

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
enum DependencyType {
    Executable(String),
    Library(String),
    None,
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
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
