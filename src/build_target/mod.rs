use std::cell::RefCell;
use std::rc::Rc;

use crate::errors;
use crate::flags::CompilerFlags;
use crate::parser;
use crate::targets;
use crate::utility;
use crate::YAMBS_MANIFEST_NAME;

pub mod associated_files;
pub mod include_directories;
pub mod target_registry;
use associated_files::SourceFiles;
use include_directories::IncludeDirectories;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    pub name: String,
    pub manifest_dir_path: std::path::PathBuf,
}

impl Dependency {
    pub fn to_build_target(
        &self,
        registry: &target_registry::TargetRegistry,
    ) -> Option<TargetNode> {
        registry.get_target(
            &self.manifest_dir_path,
            TargetType::Library(LibraryType::Static, self.name.clone()),
        )
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub directory: std::path::PathBuf,
    pub modification_time: std::time::SystemTime,
}

impl Manifest {
    pub fn new(directory: &std::path::Path) -> Self {
        let metadata = std::fs::metadata(directory.join(YAMBS_MANIFEST_NAME)).expect(&format!(
            "Could not fetch metadata from {}",
            YAMBS_MANIFEST_NAME
        ));
        Self {
            directory: directory.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time of manifest"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuildTarget {
    pub manifest: Manifest,
    pub main: std::path::PathBuf,
    pub dependencies: Vec<Dependency>,
    pub state: TargetState,
    pub source_files: SourceFiles,
    pub target_type: TargetType,
    pub include_directories: Option<IncludeDirectories>,
    pub compiler_flags: CompilerFlags,
}

impl BuildTarget {
    pub fn create(
        manifest_dir_path: &std::path::Path,
        target: &targets::Target,
        registry: &mut target_registry::TargetRegistry,
    ) -> Result<TargetNode, TargetError> {
        let target_node = match target {
            targets::Target::Executable(executable) => {
                TargetNode::new(BuildTarget::executable(manifest_dir_path, &executable)?)
            }
            targets::Target::Library(library) => {
                TargetNode::new(BuildTarget::library(manifest_dir_path, &library)?)
            }
        };
        if let Some(existing_node) = registry.get_target(
            &target_node.borrow().manifest.directory,
            target_node.borrow().target_type.clone(),
        ) {
            return Ok(existing_node);
        }

        log::debug!(
            "Creating build target \"{}\"...",
            target_node.borrow().name()
        );
        registry.add_target(target_node.clone());
        target_node.borrow_mut().state = TargetState::InProcess;
        let target_vec = target_node.borrow().detect_target(registry, target)?;

        for target in target_vec {
            log::debug!(
                "Registering target \"{}\" (manifest directory {})",
                target.name,
                target.manifest_dir_path.display()
            );
            target_node.borrow_mut().add_target(target);
        }
        target_node.borrow_mut().state = TargetState::Registered;
        Ok(target_node)
    }

    pub fn is_executable(&self) -> bool {
        match self.target_type {
            TargetType::Executable(_) => true,
            TargetType::Library(_, _) => false,
        }
    }

    pub fn is_library(&self) -> bool {
        !self.is_executable()
    }

    pub fn library_file_name(&self) -> String {
        match &self.target_type {
            TargetType::Library(_, library_name) => library_name.to_owned(),
            _ => panic!("Dependency is not a library"),
        }
    }

    pub fn library_type(&self) -> LibraryType {
        match &self.target_type {
            TargetType::Library(library_type, _) => library_type.to_owned(),
            _ => panic!("Dependency is not a library"),
        }
    }

    pub fn project_name(&self) -> &std::path::Path {
        utility::get_head_directory(&self.manifest.directory)
    }

    pub fn name(&self) -> String {
        match self.target_type {
            TargetType::Executable(ref name) => name.to_owned(),
            TargetType::Library(_, ref name) => name.to_owned(),
        }
    }

    fn executable(
        manifest_dir_path: &std::path::Path,
        executable: &targets::Executable,
    ) -> Result<Self, TargetError> {
        let mut source_files = executable.sources.clone();
        source_files.push(executable.main.clone());

        Ok(Self {
            manifest: Manifest::new(&manifest_dir_path),
            main: executable.main.to_path_buf(),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&source_files)
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
        manifest_dir_path: &std::path::Path,
        library: &targets::Library,
    ) -> Result<Self, TargetError> {
        let mut source_files = library.sources.clone();
        source_files.push(library.main.clone());

        Ok(Self {
            manifest: Manifest::new(&manifest_dir_path),
            main: library.main.to_path_buf(),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            target_type: TargetType::from_library(library),
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
    ) -> Result<Vec<Dependency>, TargetError> {
        log::debug!(
            "Checking if target \"{}\" has registered dependencies",
            self.name()
        );
        let mut target_vec = Vec::new();
        for dependency in target.dependencies() {
            if let Some((path, _)) = dependency.data.source() {
                if let Some(registered_dep) = registry.get_target(
                    &path,
                    TargetType::Library(LibraryType::Static, dependency.name.clone()),
                ) {
                    log::debug!("Found registered dependency. Checking for cyclic dependencies");
                    self.detect_cycle_from_target(&registered_dep)?;
                    target_vec.push(Dependency {
                        name: registered_dep.borrow().name(),
                        manifest_dir_path: registered_dep.borrow().manifest.directory.clone(),
                    });
                } else {
                    log::debug!(
                        "No registered dependency found. Creating dependency build target."
                    );
                    let manifest_path = path.join(YAMBS_MANIFEST_NAME);
                    let manifest = parser::parse(&manifest_path).map_err(TargetError::Parse)?;
                    let dep_target = manifest
                        .data
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
                    let target = BuildTarget::create(&path, dep_target, registry)?;
                    target_vec.push(Dependency {
                        name: target.borrow().name(),
                        manifest_dir_path: target.borrow().manifest.directory.clone(),
                    });
                }
            }
        }

        Ok(target_vec)
    }

    fn detect_cycle_from_target(&self, target_node: &TargetNode) -> Result<(), TargetError> {
        if target_node.borrow().state == TargetState::InProcess
            && target_node.borrow().name() == self.name()
        {
            return Err(TargetError::Circulation(
                target_node.borrow().manifest.directory.to_path_buf(),
                self.manifest.directory.to_path_buf(),
            ));
        }
        Ok(())
    }

    fn add_target(&mut self, dependency: Dependency) {
        self.dependencies.push(dependency);
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TargetNode(Rc<RefCell<BuildTarget>>);

impl TargetNode {
    pub fn new(target: BuildTarget) -> Self {
        Self {
            0: Rc::new(RefCell::new(target)),
        }
    }
}

impl std::ops::Deref for TargetNode {
    type Target = Rc<RefCell<BuildTarget>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TargetType {
    Executable(String),
    Library(LibraryType, String),
}

impl TargetType {
    pub fn from_library(library: &targets::Library) -> TargetType {
        let lib_type = &library.lib_type;
        let library_name = match lib_type {
            targets::LibraryType::Dynamic => format!("lib{}.so", library.name),
            targets::LibraryType::Static => format!("lib{}.a", library.name),
        };
        TargetType::Library(LibraryType::from(&lib_type), library_name)
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum LibraryType {
    Static,
    Dynamic,
}

impl LibraryType {
    pub fn from(lib_type: &targets::LibraryType) -> Self {
        match lib_type {
            &targets::LibraryType::Dynamic => LibraryType::Dynamic,
            &targets::LibraryType::Static => LibraryType::Static,
        }
    }
}

impl Default for LibraryType {
    fn default() -> Self {
        LibraryType::Static
    }
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
    #[error("Failed to parse TOML manifest.")]
    Parse(#[source] parser::ParseTomlError),
    #[error("Failed to create cache of dependencies")]
    FailedToCache(#[source] errors::CacheError),
    #[error("Dependency circulation! {0:?} depends on {1:?}, which depends on itself")]
    Circulation(std::path::PathBuf, std::path::PathBuf),
    #[error("Error occured classifying associated file")]
    AssociatedFile(#[source] associated_files::AssociatedFileError),
}
