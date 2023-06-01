use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::cli::configurations::BuildType;
use crate::errors;
use crate::flags::CompilerFlags;
use crate::manifest;
use crate::parser;
use crate::parser::types;
use crate::targets;
use crate::toolchain::NormalizedToolchain;
use crate::YAMBS_MANIFEST_NAME;

pub mod associated_files;
pub mod include_directories;
pub mod pkg_config;
pub mod target_registry;
use associated_files::SourceFiles;
use include_directories::IncludeDirectory;
use include_directories::IncludeType;
use pkg_config::{PkgConfigError, PkgConfigTarget};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DependencySourceData {
    pub manifest: manifest::Manifest,
    pub library: PrintableLibrary,
    pub include_directory: IncludeDirectory,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct HeaderOnlyData {
    pub name: String,
    pub include_directory: IncludeDirectory,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DependencySource {
    FromSource(DependencySourceData),
    FromHeaderOnly(HeaderOnlyData),
    FromPkgConfig(PkgConfigTarget),
}

impl DependencySource {
    pub fn from_source(&self) -> Option<&DependencySourceData> {
        match self {
            Self::FromSource(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Dependency {
    pub source: DependencySource,
}

impl Dependency {
    pub fn to_build_target(
        &self,
        registry: &target_registry::TargetRegistry,
    ) -> Option<TargetNode> {
        registry.get_target_from_predicate(|build_target| match build_target.target_source {
            TargetSource::FromSource(ref source_data) => {
                let dependency_source_data = self.source.from_source().unwrap();
                source_data.manifest.directory == dependency_source_data.manifest.directory
                    && build_target.library_type()
                        == Some(dependency_source_data.library.ty.clone())
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceBuildData {
    pub manifest: manifest::Manifest,
    pub dependencies: Vec<Dependency>,
    pub source_files: SourceFiles,
    pub defines: Vec<types::Define>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
// FIXME: Can this enum be removed? I can't remember why this was made at all; it may be
// completely redundant and a wasteful abstraction.
pub enum TargetSource {
    FromSource(SourceBuildData),
}

impl TargetSource {
    pub fn from_source(&self) -> Option<&SourceBuildData> {
        match self {
            Self::FromSource(s) => Some(s),
        }
    }

    pub fn from_source_mut(&mut self) -> Option<&mut SourceBuildData> {
        match self {
            Self::FromSource(s) => Some(s),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuildTarget {
    pub target_source: TargetSource,
    pub state: TargetState,
    pub target_type: TargetType,
    pub include_directory: IncludeDirectory,
    pub compiler_flags: CompilerFlags,
}

impl BuildTarget {
    pub fn target_node_from_source(
        manifest_dir_path: &std::path::Path,
        target: &targets::Target,
        registry: &mut target_registry::TargetRegistry,
        toolchain: &Rc<RefCell<NormalizedToolchain>>,
        build_type: &BuildType,
    ) -> Result<TargetNode, TargetError> {
        let target_type = TargetType::new(target);

        if let Some(existing_node) =
            registry.get_target_from_predicate(|build_target| match build_target.target_source {
                TargetSource::FromSource(ref source_data) => {
                    source_data.manifest.directory == manifest_dir_path
                        && build_target.target_type == target_type
                }
            })
        {
            return Ok(existing_node);
        }

        let target_node = match target {
            targets::Target::Executable(executable) => TargetNode::new(
                BuildTarget::executable_from_source(manifest_dir_path, executable)?,
            ),
            targets::Target::Library(library) => TargetNode::new(BuildTarget::library_from_source(
                manifest_dir_path,
                library,
            )?),
        };

        log::debug!(
            "Creating build target \"{}\"...",
            target_node.borrow().name()
        );
        registry.add_target(target_node.clone());
        target_node.borrow_mut().state = TargetState::InProcess;
        let target_vec = target_node
            .borrow()
            .detect_target(registry, target, toolchain, build_type)?;

        for target in target_vec {
            match target.source {
                DependencySource::FromSource(ref s) => {
                    log::debug!(
                        "Registering target \"{}\" (manifest directory {})",
                        s.library.name,
                        s.manifest.directory.display()
                    );
                }
                DependencySource::FromHeaderOnly(ref h) => {
                    log::debug!("Registering header only target \"{}\"", h.name);
                }
                _ => {}
            }
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
            TargetType::Library(lib) => lib.name.to_owned(),
            _ => panic!("Dependency is not a library"),
        }
    }

    pub fn library_type(&self) -> Option<LibraryType> {
        match &self.target_type {
            TargetType::Library(lib) => Some(lib.ty.to_owned()),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        match self.target_type {
            TargetType::Executable(ref exe) => exe.0.to_owned(),
            TargetType::Library(ref lib) => lib.name.to_owned(),
        }
    }

    fn executable_from_source(
        manifest_dir_path: &std::path::Path,
        executable: &targets::Executable,
    ) -> Result<Self, TargetError> {
        let source_files = executable.sources.clone();

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            defines: executable.defines.clone(),
        });

        Ok(Self {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Executable(PrintableExecutable(executable.name.to_string())),
            include_directory: include_directories::IncludeDirectory {
                include_type: include_directories::IncludeType::Include,
                path: manifest_dir_path.to_path_buf().join("include"),
            },
            compiler_flags: executable.compiler_flags.clone(),
        })
    }

    fn library_from_source(
        manifest_dir_path: &std::path::Path,
        library: &targets::Library,
    ) -> Result<Self, TargetError> {
        let source_files = library.sources.clone();

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            defines: library.defines.clone(),
        });

        Ok(Self {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Library(PrintableLibrary::from(library)),
            include_directory: include_directories::IncludeDirectory {
                include_type: include_directories::IncludeType::Include,
                path: manifest_dir_path.to_path_buf().join("include"),
            },
            compiler_flags: library.compiler_flags.clone(),
        })
    }

    fn detect_target(
        &self,
        registry: &mut target_registry::TargetRegistry,
        target: &targets::Target,
        toolchain: &Rc<RefCell<NormalizedToolchain>>,
        build_type: &BuildType,
    ) -> Result<Vec<Dependency>, TargetError> {
        log::debug!(
            "Checking if target \"{}\" has registered dependencies",
            self.name()
        );
        let mut target_vec = Vec::new();
        for dependency in target.dependencies() {
            match dependency.data {
                types::DependencyData::Source(ref dependency_source_data) => {
                    if let Some(registered_dep) =
                        registry.get_target_from_predicate(|build_target| {
                            match build_target.target_source {
                                TargetSource::FromSource(ref source_data) => {
                                    source_data.manifest.directory == dependency_source_data.path
                                        && build_target.name() == dependency.name
                                }
                            }
                        })
                    {
                        log::debug!(
                            "Found registered dependency. Checking for cyclic dependencies"
                        );
                        self.detect_cycle_from_target(&registered_dep)?;
                        let borrowed_dep = registered_dep.borrow();
                        let source_data = borrowed_dep.target_source.from_source().unwrap();
                        let dependency_source =
                            DependencySource::FromSource(DependencySourceData {
                                library: PrintableLibrary {
                                    name: registered_dep.borrow().name(),
                                    ty: registered_dep.borrow().library_type().ok_or_else(
                                        || {
                                            TargetError::DependencyNotALibrary(
                                                registered_dep.borrow().name(),
                                            )
                                        },
                                    )?,
                                },
                                manifest: source_data.manifest.clone(),
                                include_directory: registered_dep
                                    .borrow()
                                    .include_directory
                                    .clone(),
                            });
                        let dependency = Dependency {
                            source: dependency_source,
                        };
                        target_vec.push(dependency);
                    } else {
                        log::debug!(
                            "No registered dependency found. Creating dependency build target."
                        );
                        let manifest_path = dependency_source_data.path.join(YAMBS_MANIFEST_NAME);
                        let manifest = parser::parse(&manifest_path).map_err(TargetError::Parse)?;
                        let dep_target = manifest
                            .data
                            .targets
                            .iter()
                            .find_map(|dep| {
                                if let Some(lib) = dep.library() {
                                    if lib.name == dependency.name {
                                        return Some(dep);
                                    }
                                }
                                None
                            })
                            .ok_or_else(|| {
                                TargetError::NoLibraryWithName(dependency.name.clone())
                            })?;
                        let target = BuildTarget::target_node_from_source(
                            &dependency_source_data.path,
                            dep_target,
                            registry,
                            toolchain,
                            build_type,
                        )?;
                        let borrowed_target = target.borrow();
                        let source_data = borrowed_target.target_source.from_source().unwrap();
                        let dependency_source =
                            DependencySource::FromSource(DependencySourceData {
                                library: PrintableLibrary {
                                    name: target.borrow().name(),
                                    ty: target.borrow().library_type().ok_or_else(|| {
                                        TargetError::DependencyNotALibrary(target.borrow().name())
                                    })?,
                                },
                                manifest: source_data.manifest.clone(),
                                include_directory: target.borrow().include_directory.clone(),
                            });
                        target_vec.push(Dependency {
                            source: dependency_source,
                        });
                    }
                }
                types::DependencyData::HeaderOnly(ref header_only_data) => {
                    let header_only = HeaderOnlyData {
                        name: dependency.name.to_string(),
                        include_directory: IncludeDirectory {
                            path: header_only_data.include_directory.clone(),
                            include_type: IncludeType::Include,
                        },
                    };
                    let header_only = DependencySource::FromHeaderOnly(header_only);
                    target_vec.push(Dependency {
                        source: header_only,
                    });
                }
                types::DependencyData::PkgConfig(ref pkg_config_data) => {
                    let mut toolchain_lock = toolchain.borrow_mut();
                    if let Some(ref mut pkg_config) = toolchain_lock.pkg_config {
                        match build_type {
                            BuildType::Debug => {
                                pkg_config.add_search_path(&pkg_config_data.debug.search_dir);
                            }
                            BuildType::Release => {
                                pkg_config.add_search_path(&pkg_config_data.release.search_dir);
                            }
                        }
                        match pkg_config.find_target(&dependency.name) {
                            Ok(pkg_config_target) => {
                                let pkg_config_dep =
                                    DependencySource::FromPkgConfig(pkg_config_target);
                                target_vec.push(Dependency {
                                    source: pkg_config_dep,
                                });
                            }
                            Err(e) => {
                                return Err(TargetError::CouldNotFindPkgConfigPackage(
                                    dependency.name.clone(),
                                    e,
                                ))
                            }
                        }
                    } else {
                        return Err(TargetError::NoPkgConfigInstance);
                    }
                }
            }
        }

        Ok(target_vec)
    }

    fn detect_cycle_from_target(&self, target_node: &TargetNode) -> Result<(), TargetError> {
        if target_node.borrow().state == TargetState::InProcess
            && target_node.borrow().name() == self.name()
        {
            let borrowed_target_node = target_node.borrow();
            let target_node_source_data = borrowed_target_node.target_source.from_source().unwrap();
            let source_data = self.target_source.from_source().unwrap();
            return Err(TargetError::Circulation(
                target_node_source_data.manifest.directory.to_path_buf(),
                source_data.manifest.directory.to_path_buf(),
            ));
        }
        Ok(())
    }

    fn add_target(&mut self, dependency: Dependency) {
        if let Some(source_data) = self.target_source.from_source_mut() {
            source_data.dependencies.push(dependency);
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TargetNode(Rc<RefCell<BuildTarget>>);

impl TargetNode {
    pub fn new(target: BuildTarget) -> Self {
        Self(Rc::new(RefCell::new(target)))
    }
}

impl std::ops::Deref for TargetNode {
    type Target = Rc<RefCell<BuildTarget>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PrintableExecutable(String);

impl fmt::Display for PrintableExecutable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(target_os = "linux")]
pub const STATIC_LIBRARY_FILE_EXTENSION: &str = "a";
#[cfg(target_os = "linux")]
pub const SHARED_LIBRARY_FILE_EXTENSION: &str = "so";

//  FIXME: This should be concretized to a Library type instead.
// It does not really make sense that a Library only has a name and type; it is lacking.
// For example, there is no directory property so there is no practical way to fully establish the
// location of the library.
//
// It has been difficult to create a Library type with a directory because of lack of information
// when creating it. Could it be a choice of compressing some of the types in DependencySourceData
// in favor of them in Library? Then this class would make more sense.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Eq)]
pub struct PrintableLibrary {
    pub name: String,
    pub ty: LibraryType,
}

impl PrintableLibrary {
    pub fn possible_lib_names(name: &str) -> [String; 2] {
        #[cfg(target_family = "unix")]
        {
            [
                format!("lib{}.{}", name, STATIC_LIBRARY_FILE_EXTENSION),
                format!("lib{}.{}", name, SHARED_LIBRARY_FILE_EXTENSION),
            ]
        }
    }
}

impl From<targets::Library> for PrintableLibrary {
    fn from(lib: targets::Library) -> Self {
        Self {
            name: lib.name,
            ty: LibraryType::from(&lib.lib_type),
        }
    }
}

impl From<&targets::Library> for PrintableLibrary {
    fn from(lib: &targets::Library) -> Self {
        Self {
            name: lib.name.clone(),
            ty: LibraryType::from(&lib.lib_type),
        }
    }
}

impl fmt::Display for PrintableLibrary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.ty {
            LibraryType::Static => {
                #[cfg(target_family = "unix")]
                write!(
                    f,
                    "{}",
                    format!("lib{}.{}", self.name, STATIC_LIBRARY_FILE_EXTENSION)
                )
            }
            LibraryType::Dynamic => {
                #[cfg(target_family = "unix")]
                write!(
                    f,
                    "{}",
                    format!("lib{}.{}", self.name, SHARED_LIBRARY_FILE_EXTENSION)
                )
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum TargetType {
    Executable(PrintableExecutable),
    Library(PrintableLibrary),
}

impl TargetType {
    pub fn new(target: &targets::Target) -> Self {
        match target {
            targets::Target::Executable(executable) => {
                Self::Executable(PrintableExecutable(executable.name.clone()))
            }
            targets::Target::Library(lib) => Self::Library(PrintableLibrary {
                name: lib.name.clone(),
                ty: LibraryType::from(&lib.lib_type),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, Eq)]
pub enum LibraryType {
    Static,
    Dynamic,
}

impl LibraryType {
    pub fn from(lib_type: &types::LibraryType) -> Self {
        match lib_type {
            &types::LibraryType::Dynamic => LibraryType::Dynamic,
            &types::LibraryType::Static => LibraryType::Static,
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
    BuildFileMade,
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
    #[error("Could not find any library with name {0}")]
    NoLibraryWithName(String),
    #[error("Dependency \"{0}\" parsed is not a library, but an executable")]
    DependencyNotALibrary(String),
    #[error("Error occured when parsing include directories")]
    IncludeDirectories(#[source] include_directories::IncludeDirectoriesError),
    #[error("Could not find any instance of pkg-config! Unable to find pkg-config dependencies.")]
    NoPkgConfigInstance,
    #[error("Could not find any pkg-config package with name {0}")]
    CouldNotFindPkgConfigPackage(String, #[source] PkgConfigError),
}
