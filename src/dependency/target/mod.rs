use std::cell::RefCell;
use std::rc::Rc;

use crate::errors;
use crate::flags::CompilerFlags;
use crate::parser;
use crate::targets;
use crate::utility;
use crate::YAMBS_FILE_NAME;

pub mod associated_files;
pub mod include_directories;
pub mod target_registry;
use associated_files::AssociatedFiles;
use include_directories::IncludeDirectories;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
    pub recipe_dir_path: std::path::PathBuf,
    pub main: std::path::PathBuf,
    pub modification_time: std::time::SystemTime,
    pub dependencies: Vec<TargetNode>,
    pub state: TargetState,
    pub associated_files: AssociatedFiles,
    pub target_type: TargetType,
    pub include_directories: Option<IncludeDirectories>,
    pub compiler_flags: CompilerFlags,
}

impl Target {
    pub fn create(
        recipe_dir_path: &std::path::Path,
        target: &targets::Target,
        registry: &mut target_registry::TargetRegistry,
    ) -> Result<TargetNode, TargetError> {
        let target_node = match target {
            targets::Target::Executable(executable) => {
                TargetNode::new(Target::executable(recipe_dir_path, &executable)?)
            }
            targets::Target::Library(library) => {
                TargetNode::new(Target::library(recipe_dir_path, &library)?)
            }
        };
        target_node.borrow_mut().state = TargetState::InProcess;
        let target_vec = target_node.borrow().detect_target(registry, target)?;

        for target in target_vec {
            target_node.borrow_mut().add_target(target);
        }
        target_node.borrow_mut().state = TargetState::Registered;
        Ok(target_node)
    }

    pub fn is_executable(&self) -> bool {
        match self.target_type {
            TargetType::Executable(_) => true,
            TargetType::Library(_) => false,
        }
    }

    pub fn is_library(&self) -> bool {
        !self.is_executable()
    }

    pub fn library_file_name(&self) -> String {
        match &self.target_type {
            TargetType::Library(library_name) => library_name.to_owned(),
            _ => panic!("Dependency is not a library"),
        }
    }

    pub fn project_name(&self) -> &std::path::Path {
        utility::get_head_directory(&self.recipe_dir_path)
    }

    pub fn name(&self) -> Option<String> {
        match self.target_type {
            TargetType::Executable(ref executable) => Some(executable.to_owned()),
            TargetType::Library(ref library) => Some(library.to_owned()),
        }
    }

    fn executable(
        recipe_dir_path: &std::path::Path,
        executable: &targets::Executable,
    ) -> Result<Self, TargetError> {
        let metadata =
            std::fs::metadata(recipe_dir_path).expect("Could not fetch metadata from yambs.json");
        let mut source_files = executable.sources.clone();
        source_files.push(executable.main.clone());

        Ok(Self {
            recipe_dir_path: recipe_dir_path.parent().unwrap().to_path_buf(),
            main: executable.main.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time."),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            associated_files: AssociatedFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            target_type: TargetType::Executable(executable.name.to_string()),
            include_directories: IncludeDirectories::from_dependencies(&executable.dependencies),
            compiler_flags: executable
                .compiler_flags
                .clone()
                .unwrap_or(CompilerFlags::new()),
        })
    }

    fn library(
        recipe_dir_path: &std::path::Path,
        library: &targets::Library,
    ) -> Result<Self, TargetError> {
        let metadata =
            std::fs::metadata(recipe_dir_path).expect("Could not fetch metadata from yambs.json");

        let mut source_files = library.sources.clone();
        source_files.push(library.main.clone());

        Ok(Self {
            recipe_dir_path: recipe_dir_path.parent().unwrap().to_path_buf(),
            main: library.main.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time."),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            associated_files: AssociatedFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            target_type: TargetType::Library(library.name.to_string()),
            include_directories: IncludeDirectories::from_dependencies(&library.dependencies),
            compiler_flags: library
                .compiler_flags
                .clone()
                .unwrap_or(CompilerFlags::new()),
        })
    }

    fn detect_target(
        &self,
        registry: &mut target_registry::TargetRegistry,
        target: &targets::Target,
    ) -> Result<Vec<TargetNode>, TargetError> {
        let mut target_vec = Vec::new();
        for dependency in target.dependencies() {
            if let Some(registered_dep) = registry.get_target(&dependency.data.path) {
                self.detect_cycle_from_target(&registered_dep)?;
                target_vec.push(registered_dep);
            } else {
                let recipe_path = dependency.data.path.join(YAMBS_FILE_NAME);
                let recipe = parser::parse(&recipe_path).map_err(TargetError::Parse)?;
                let dep_target = recipe
                    .recipe
                    .targets
                    .iter()
                    .find(|dep| {
                        if let Some(lib) = dep.library() {
                            return lib.name == dependency.name;
                        } else {
                            false
                        }
                    })
                    .unwrap();
                let target = Target::create(&dependency.data.path, dep_target, registry)?;
                target_vec.push(target)
            }
        }

        Ok(target_vec)
    }

    fn detect_cycle_from_target(&self, target_node: &TargetNode) -> Result<(), TargetError> {
        if target_node.borrow().state == TargetState::InProcess {
            return Err(TargetError::Circulation(
                target_node.borrow().recipe_dir_path.to_path_buf(),
                self.recipe_dir_path.to_path_buf(),
            ));
        }
        Ok(())
    }

    fn add_target(&mut self, target: TargetNode) {
        self.dependencies.push(target);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TargetNode(Rc<RefCell<Target>>);

impl TargetNode {
    pub fn new(target: Target) -> Self {
        Self {
            0: Rc::new(RefCell::new(target)),
        }
    }
    pub fn try_borrow(&self) -> Result<std::cell::Ref<'_, Target>, std::cell::BorrowError> {
        self.0.try_borrow()
    }
}

impl std::ops::Deref for TargetNode {
    type Target = Rc<RefCell<Target>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TargetType {
    Executable(String),
    Library(String),
}

#[derive(Debug, PartialEq, Eq, Clone, serde::Serialize, serde::Deserialize)]
pub enum TargetState {
    NotInProcess,
    InProcess,
    Registered,
    MakefileMade,
    Building,
    BuildComplete,
}

impl TargetState {
    pub fn new() -> Self {
        TargetState::NotInProcess
    }
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TargetError {
    #[error(transparent)]
    Fs(#[from] errors::FsError),
    #[error("Failed to parse TOML recipe.")]
    Parse(#[source] parser::ParseTomlError),
    #[error("Failed to create cache of dependencies")]
    FailedToCache(#[source] errors::CacheError),
    #[error("Dependency circulation! {0:?} depends on {1:?}, which depends on itself")]
    Circulation(std::path::PathBuf, std::path::PathBuf),
    #[error("Call on get_dependency when dependency is not set. Call on set_dependency must be done prior!")]
    NotSet,
    #[error("Error occured classifying associated file")]
    AssociatedFile(#[source] associated_files::AssociatedFileError),
}
