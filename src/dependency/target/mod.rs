use std::cell::RefCell;
use std::rc::Rc;

use crate::errors;
use crate::flags::CompilerFlags;
use crate::parser;
use crate::targets;
use crate::YAMBS_FILE_NAME;

mod associated_files;
mod include_directories;
mod target_registry;
use associated_files::AssociatedFiles;
use include_directories::IncludeDirectories;
use target_registry::TargetRegistry;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Target {
    recipe_dir_path: std::path::PathBuf,
    main: std::path::PathBuf,
    modification_time: std::time::SystemTime,
    dependencies: Vec<TargetNode>,
    state: TargetState,
    associated_files: AssociatedFiles,
    target_type: TargetType,
    include_directories: Option<IncludeDirectories>,
    compiler_flags: CompilerFlags,
}

impl Target {
    pub fn create(
        recipe_dir_path: &std::path::Path,
        target: &targets::Target,
        registry: &mut TargetRegistry,
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

    fn executable(
        recipe_dir_path: &std::path::Path,
        executable: &targets::Executable,
    ) -> Result<Self, TargetError> {
        let metadata = std::fs::metadata(recipe_dir_path.join(YAMBS_FILE_NAME))
            .expect("Could not fetch metadata from yambs.json");

        Ok(Self {
            recipe_dir_path: recipe_dir_path.to_path_buf(),
            main: executable.main.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time."),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            associated_files: AssociatedFiles::from_paths(&executable.sources)
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
        let metadata = std::fs::metadata(recipe_dir_path.join(YAMBS_FILE_NAME))
            .expect("Could not fetch metadata from yambs.json");

        Ok(Self {
            recipe_dir_path: recipe_dir_path.to_path_buf(),
            main: library.main.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time."),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            associated_files: AssociatedFiles::from_paths(&library.sources)
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
        registry: &mut TargetRegistry,
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
    None,
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
