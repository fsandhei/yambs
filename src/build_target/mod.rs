use std::cell::RefCell;
use std::rc::Rc;

use crate::errors;
use crate::flags::CompilerFlags;
use crate::manifest;
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
    pub manifest: manifest::Manifest,
    pub library_type: LibraryType,
}

impl Dependency {
    pub fn to_build_target(
        &self,
        registry: &target_registry::TargetRegistry,
    ) -> Option<TargetNode> {
        registry.get_target_from_predicate(|build_target| {
            build_target.manifest.directory == self.manifest.directory
                && build_target.library_type() == self.library_type
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuildTarget {
    pub manifest: manifest::Manifest,
    pub dependencies: Vec<Dependency>,
    pub state: TargetState,
    pub source_files: SourceFiles,
    pub target_type: TargetType,
    pub include_directories: IncludeDirectories,
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
                TargetNode::new(BuildTarget::executable(manifest_dir_path, executable)?)
            }
            targets::Target::Library(library) => {
                TargetNode::new(BuildTarget::library(manifest_dir_path, library)?)
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
                target.manifest.directory.display()
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
        let source_files = executable.sources.clone();

        let mut include_directories =
            IncludeDirectories::from_dependencies(&executable.dependencies);
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest_dir_path.to_path_buf().join("include"),
        });

        Ok(Self {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            target_type: TargetType::Executable(executable.name.to_string()),
            include_directories,
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
        let source_files = library.sources.clone();

        let mut include_directories = IncludeDirectories::from_dependencies(&library.dependencies);
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest_dir_path.to_path_buf().join("include"),
        });

        Ok(Self {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
            target_type: TargetType::from_library(library),
            include_directories,
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
                if let Some(registered_dep) = registry.get_target_from_predicate(|build_target| {
                    build_target.manifest.directory == path
                        && build_target.name() == dependency.name
                }) {
                    log::debug!("Found registered dependency. Checking for cyclic dependencies");
                    self.detect_cycle_from_target(&registered_dep)?;
                    target_vec.push(Dependency {
                        name: registered_dep.borrow().name(),
                        manifest: registered_dep.borrow().manifest.clone(),
                        library_type: registered_dep.borrow().library_type(),
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
                                lib.name == dependency.name
                            } else {
                                false
                            }
                        })
                        .ok_or_else(|| TargetError::NoLibraryWithName(dependency.name.clone()))?;
                    let target = BuildTarget::create(&path, dep_target, registry)?;
                    target_vec.push(Dependency {
                        name: target.borrow().name(),
                        manifest: target.borrow().manifest.clone(),
                        library_type: target.borrow().library_type(),
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
        TargetType::Library(LibraryType::from(lib_type), library_name)
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
    BuildFileMade,
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
    #[error("Could not find any library with name {0}")]
    NoLibraryWithName(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    struct StubManifest {
        manifest: manifest::Manifest,
    }

    impl StubManifest {
        fn new(manifest_dir: &std::path::Path) -> Self {
            std::fs::File::create(manifest_dir.join(YAMBS_MANIFEST_NAME)).unwrap();

            Self {
                manifest: manifest::Manifest::new(manifest_dir),
            }
        }
    }

    struct StubTarget {
        pub target: targets::Target,
    }

    impl StubTarget {
        fn executable(executable: targets::Executable) -> Self {
            for source in &executable.sources {
                if !source.exists() {
                    std::fs::File::create(source).unwrap();
                }
            }
            Self {
                target: targets::Target::Executable(executable),
            }
        }

        fn library(library: targets::Library) -> Self {
            for source in &library.sources {
                if !source.exists() {
                    std::fs::File::create(source).unwrap();
                }
            }
            Self {
                target: targets::Target::Library(library),
            }
        }
    }

    struct TestFixture {
        pub _dir: tempdir::TempDir,
        // pub stub_registry: target_registry::TargetRegistry,
        pub stub_manifest: StubManifest,
    }

    impl TestFixture {
        fn new() -> Self {
            let dir = tempdir::TempDir::new("build_target").unwrap();
            // let stub_registry = target_registry::TargetRegistry::new();
            let stub_manifest = StubManifest::new(dir.path());
            Self {
                _dir: dir,
                // stub_registry,
                stub_manifest,
            }
        }
    }

    #[test]
    fn can_create_build_target_from_executable() {
        let fixture = TestFixture::new();
        let stub_executable = StubTarget::executable(targets::Executable {
            name: "x".to_string(),
            sources: vec![
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("x.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("y.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("z.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("main.cpp")),
            ],
            dependencies: Vec::new(),
            compiler_flags: None,
        });

        let executable = stub_executable.target.executable().unwrap();

        let manifest = fixture.stub_manifest.manifest.clone();
        let mut include_directories =
            IncludeDirectories::from_dependencies(&executable.dependencies);
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.to_path_buf().join("include"),
        });

        let expected = BuildTarget {
            manifest,
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&executable.sources.clone()).unwrap(),
            target_type: TargetType::Executable("x".to_string()),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual =
            BuildTarget::executable(&fixture.stub_manifest.manifest.directory, &executable)
                .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_static_library() {
        let fixture = TestFixture::new();

        let stub_library = StubTarget::library(targets::Library {
            name: "MyLibraryData".to_string(),
            sources: vec![
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("x.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("y.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("z.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("main.cpp")),
            ],
            dependencies: Vec::new(),
            compiler_flags: None,
            lib_type: targets::LibraryType::Static,
        });
        let library = &stub_library.target.library().unwrap();

        let manifest = fixture.stub_manifest.manifest.clone();
        let mut include_directories = IncludeDirectories::from_dependencies(&library.dependencies);
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.to_path_buf().join("include"),
        });

        let expected = BuildTarget {
            manifest,
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&library.sources.clone()).unwrap(),
            target_type: TargetType::Library(LibraryType::Static, "libMyLibraryData.a".to_string()),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual =
            BuildTarget::library(&fixture.stub_manifest.manifest.directory, &library).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_shared_library() {
        let fixture = TestFixture::new();
        let stub_library = StubTarget::library(targets::Library {
            name: "MyLibraryData".to_string(),
            sources: vec![
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("x.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("y.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("z.cpp")),
                fixture
                    .stub_manifest
                    .manifest
                    .directory
                    .join(std::path::PathBuf::from("main.cpp")),
            ],
            dependencies: Vec::new(),
            compiler_flags: None,
            lib_type: targets::LibraryType::Dynamic,
        });
        let library = &stub_library.target.library().unwrap();

        let manifest = fixture.stub_manifest.manifest.clone();
        let mut include_directories = IncludeDirectories::from_dependencies(&library.dependencies);
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.to_path_buf().join("include"),
        });

        let expected = BuildTarget {
            manifest,
            dependencies: Vec::new(),
            state: TargetState::NotInProcess,
            source_files: SourceFiles::from_paths(&library.sources.clone()).unwrap(),
            target_type: TargetType::Library(
                LibraryType::Dynamic,
                "libMyLibraryData.so".to_string(),
            ),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual =
            BuildTarget::library(&fixture.stub_manifest.manifest.directory, library).unwrap();
        assert_eq!(actual, expected);
    }
}
