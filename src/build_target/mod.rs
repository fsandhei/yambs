use std::cell::RefCell;
use std::rc::Rc;

use crate::errors;
use crate::flags::CompilerFlags;
use crate::manifest;
use crate::parser;
use crate::parser::types;
use crate::targets;
use crate::YAMBS_MANIFEST_NAME;

pub mod associated_files;
pub mod include_directories;
pub mod target_registry;
use associated_files::SourceFiles;
use include_directories::IncludeDirectories;

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DependencySourceData {
    pub name: String,
    pub manifest: manifest::Manifest,
    pub library_type: LibraryType,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DependencyPrebuiltData {
    pub name: String,
    pub debug_binary_path: std::path::PathBuf,
    pub release_binary_path: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum DependencySource {
    FromSource(DependencySourceData),
    FromPrebuilt(DependencyPrebuiltData),
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
                        == Some(dependency_source_data.library_type.clone())
            }
            _ => false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceBuildData {
    pub manifest: manifest::Manifest,
    pub dependencies: Vec<Dependency>,
    pub source_files: SourceFiles,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PrebuiltBuildData {
    pub debug_binary_path: std::path::PathBuf,
    pub release_binary_path: std::path::PathBuf,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum TargetSource {
    FromSource(SourceBuildData),
    FromPrebuilt(PrebuiltBuildData),
}

impl TargetSource {
    pub fn from_source(&self) -> Option<&SourceBuildData> {
        match self {
            Self::FromSource(s) => Some(s),
            _ => None,
        }
    }

    pub fn from_source_mut(&mut self) -> Option<&mut SourceBuildData> {
        match self {
            Self::FromSource(s) => Some(s),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BuildTarget {
    pub target_source: TargetSource,
    pub state: TargetState,
    pub target_type: TargetType,
    pub include_directories: IncludeDirectories,
    pub compiler_flags: CompilerFlags,
}

impl BuildTarget {
    pub fn target_node_from_binary(
        name: &str,
        binary_data: &types::BinaryData,
        registry: &mut target_registry::TargetRegistry,
    ) -> Result<TargetNode, TargetError> {
        let target_type =
            TargetType::from_prebuilt(name, &binary_data.debug_path_information.path)?;

        if let Some(existing_node) =
            registry.get_target_from_predicate(|build_target| match build_target.target_source {
                TargetSource::FromPrebuilt(ref other_binary_data) => {
                    (other_binary_data.debug_binary_path == binary_data.debug_path_information.path
                        && other_binary_data.release_binary_path
                            == binary_data.release_path_information.path)
                        && build_target.target_type == target_type
                }
                _ => false,
            })
        {
            return Ok(existing_node);
        }

        let target_node = TargetNode::new(BuildTarget::library_from_prebuilt(name, binary_data)?);
        Ok(target_node)
    }

    pub fn target_node_from_source(
        manifest_dir_path: &std::path::Path,
        target: &targets::Target,
        registry: &mut target_registry::TargetRegistry,
    ) -> Result<TargetNode, TargetError> {
        let target_type = TargetType::new(target);

        if let Some(existing_node) =
            registry.get_target_from_predicate(|build_target| match build_target.target_source {
                TargetSource::FromSource(ref source_data) => {
                    source_data.manifest.directory == manifest_dir_path
                        && build_target.target_type == target_type
                }
                _ => false,
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
        let target_vec = target_node.borrow().detect_target(registry, target)?;

        for target in target_vec {
            match target.source {
                DependencySource::FromSource(ref s) => {
                    log::debug!(
                        "Registering target \"{}\" (manifest directory {})",
                        s.name,
                        s.manifest.directory.display()
                    );
                }
                DependencySource::FromPrebuilt(ref b) => {
                    log::debug!("Registering prebuilt target \"{}\"", b.name);
                }
            }
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

    pub fn library_type(&self) -> Option<LibraryType> {
        match &self.target_type {
            TargetType::Library(library_type, _) => Some(library_type.to_owned()),
            _ => None,
        }
    }

    pub fn name(&self) -> String {
        match self.target_type {
            TargetType::Executable(ref name) => name.to_owned(),
            TargetType::Library(_, ref name) => name.to_owned(),
        }
    }

    fn executable_from_source(
        manifest_dir_path: &std::path::Path,
        executable: &targets::Executable,
    ) -> Result<Self, TargetError> {
        let source_files = executable.sources.clone();

        let mut include_directories =
            IncludeDirectories::from_dependencies(&executable.dependencies)
                .map_err(TargetError::IncludeDirectories)?;
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest_dir_path.to_path_buf().join("include"),
        });

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
        });

        Ok(Self {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Executable(executable.name.to_string()),
            include_directories,
            compiler_flags: executable
                .compiler_flags
                .clone()
                .unwrap_or(CompilerFlags::new()),
        })
    }

    fn library_from_prebuilt(
        name: &str,
        binary_data: &types::BinaryData,
    ) -> Result<Self, TargetError> {
        let mut include_directories = IncludeDirectories::new();
        let include_type = match binary_data.search_type {
            types::IncludeSearchType::Include => include_directories::IncludeType::Include,
            types::IncludeSearchType::System => include_directories::IncludeType::System,
        };

        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_type.clone(),
            path: binary_data.include_directory.clone(),
        });

        let target_source = TargetSource::FromPrebuilt(PrebuiltBuildData {
            debug_binary_path: binary_data.debug_path_information.path.clone(),
            release_binary_path: binary_data.release_path_information.path.clone(),
        });

        Ok(Self {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::from_prebuilt(
                name,
                &binary_data.release_path_information.path,
            )?,
            include_directories,
            compiler_flags: CompilerFlags::new(),
        })
    }

    fn library_from_source(
        manifest_dir_path: &std::path::Path,
        library: &targets::Library,
    ) -> Result<Self, TargetError> {
        let source_files = library.sources.clone();

        let mut include_directories = IncludeDirectories::from_dependencies(&library.dependencies)
            .map_err(TargetError::IncludeDirectories)?;
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest_dir_path.to_path_buf().join("include"),
        });

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest::Manifest::new(manifest_dir_path),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&source_files)
                .map_err(TargetError::AssociatedFile)?,
        });

        Ok(Self {
            target_source,
            state: TargetState::NotInProcess,
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
            match dependency.data {
                types::DependencyData::Binary(ref dependency_binary_data) => {
                    let dependency_target = BuildTarget::target_node_from_binary(
                        &dependency.name,
                        &dependency_binary_data,
                        registry,
                    )?;
                    let dependency_source =
                        DependencySource::FromPrebuilt(DependencyPrebuiltData {
                            name: dependency_target.borrow().name(),
                            debug_binary_path: dependency_binary_data
                                .debug_path_information
                                .path
                                .clone(),
                            release_binary_path: dependency_binary_data
                                .release_path_information
                                .path
                                .clone(),
                        });
                    let dependency = Dependency {
                        source: dependency_source,
                    };
                    target_vec.push(dependency);
                }
                types::DependencyData::Source(ref dependency_source_data) => {
                    if let Some(registered_dep) =
                        registry.get_target_from_predicate(|build_target| {
                            match build_target.target_source {
                                TargetSource::FromSource(ref source_data) => {
                                    source_data.manifest.directory == dependency_source_data.path
                                        && build_target.name() == dependency.name
                                }
                                _ => false,
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
                                name: registered_dep.borrow().name(),
                                manifest: source_data.manifest.clone(),
                                library_type: registered_dep.borrow().library_type().ok_or_else(
                                    || {
                                        TargetError::DependencyNotALibrary(
                                            registered_dep.borrow().name(),
                                        )
                                    },
                                )?,
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
                        )?;
                        let borrowed_target = target.borrow();
                        let source_data = borrowed_target.target_source.from_source().unwrap();
                        let dependency_source =
                            DependencySource::FromSource(DependencySourceData {
                                name: target.borrow().name(),
                                manifest: source_data.manifest.clone(),
                                library_type: target.borrow().library_type().ok_or_else(|| {
                                    TargetError::DependencyNotALibrary(target.borrow().name())
                                })?,
                            });
                        target_vec.push(Dependency {
                            source: dependency_source,
                        });
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
    pub fn new(target: &targets::Target) -> Self {
        if let Some(library) = target.library() {
            Self::from_library(library)
        } else {
            let executable = target.executable().unwrap();
            Self::Executable(executable.name.to_string())
        }
    }

    pub fn from_library(library: &targets::Library) -> TargetType {
        let lib_type = &library.lib_type;
        let library_name = match lib_type {
            types::LibraryType::Dynamic => format!("lib{}.so", library.name),
            types::LibraryType::Static => format!("lib{}.a", library.name),
        };
        TargetType::Library(LibraryType::from(lib_type), library_name)
    }

    pub fn from_prebuilt(name: &str, binary: &std::path::Path) -> Result<TargetType, TargetError> {
        let extension = binary.extension().and_then(std::ffi::OsStr::to_str);
        match extension {
            Some("a") => Ok(TargetType::Library(LibraryType::Static, name.to_string())),
            Some("so") => Ok(TargetType::Library(LibraryType::Dynamic, name.to_string())),
            _ => Err(TargetError::DependencyNotALibrary(name.to_string())),
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
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
}

#[cfg(test)]
mod tests {
    use super::*;

    use pretty_assertions::assert_eq;

    struct StubManifest {
        manifest: manifest::Manifest,
    }

    impl StubManifest {
        fn new(manifest_dir: &std::path::Path, data: &str) -> Self {
            use std::io::Write;

            let mut fh = std::fs::File::create(manifest_dir.join(YAMBS_MANIFEST_NAME)).unwrap();

            fh.write(data.as_bytes()).unwrap();
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

    struct StubProjectBuilder {
        pub targets: Vec<StubTarget>,
    }

    impl StubProjectBuilder {
        fn with_target(mut self, target: StubTarget) -> Self {
            self.targets.push(target);
            self
        }

        fn create(self, manifest_dir: &std::path::Path) -> StubProject {
            let mut manifest_data = String::new();
            for target in &self.targets {
                if let Some(library) = target.target.library() {
                    manifest_data.push_str(&format!(
                        r#"[library.{}]
                       sources = {:?}
                       type = "static"
                       "#,
                        library.name, library.sources
                    ));
                } else {
                    let executable = target.target.executable().unwrap();
                    manifest_data.push_str(&format!(
                        r#"
                            [executable.{}]
                            sources = {:?}    
                            "#,
                        executable.name, executable.sources,
                    ));
                }
            }
            let include_directory = manifest_dir.join("include");
            std::fs::create_dir(&include_directory).unwrap();
            StubProject {
                targets: self.targets,
                manifest: StubManifest::new(manifest_dir, &manifest_data),
                _include_directory: include_directory,
            }
        }
    }

    struct StubProject {
        pub targets: Vec<StubTarget>,
        pub manifest: StubManifest,
        pub _include_directory: std::path::PathBuf,
    }

    impl StubProject {
        fn builder() -> StubProjectBuilder {
            StubProjectBuilder {
                targets: Vec::new(),
            }
        }

        fn target_with_target_type(&self, target_type: TargetType) -> Option<targets::Target> {
            match target_type {
                TargetType::Executable(name) => self
                    .targets
                    .iter()
                    .filter_map(|target| target.target.executable())
                    .find(|target| target.name == name)
                    .map(|executable| targets::Target::Executable(executable.to_owned())),
                TargetType::Library(_, name) => self
                    .targets
                    .iter()
                    .filter_map(|target| target.target.library())
                    .find(|target| target.name == name)
                    .map(|library| targets::Target::Library(library.to_owned())),
            }
        }
    }

    struct TestFixture {
        pub dir: tempdir::TempDir,
        pub stub_registry: target_registry::TargetRegistry,
    }

    impl TestFixture {
        fn new() -> Self {
            let dir = tempdir::TempDir::new("build_target").unwrap();
            let stub_registry = target_registry::TargetRegistry::new();
            Self { dir, stub_registry }
        }
    }

    // FIXME: Refactor tests so we can use less lines!

    #[test]
    fn can_create_build_target_from_executable() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
            }))
            .create(fixture.dir.path());
        let executable_target = stub_project
            .target_with_target_type(TargetType::Executable("x".to_string()))
            .unwrap();
        let executable = executable_target.executable().unwrap();

        let stub_manifest = stub_project.manifest;
        let manifest = stub_manifest.manifest;
        let mut include_directories =
            IncludeDirectories::from_dependencies(&executable.dependencies).unwrap();
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.join("include"),
        });

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest.clone(),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&executable.sources.clone()).unwrap(),
        });

        let expected = BuildTarget {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Executable("x".to_string()),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual = BuildTarget::executable_from_source(&manifest.directory, executable).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_static_library() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "MyLibraryData".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
                lib_type: types::LibraryType::Static,
            }))
            .create(manifest_dir);
        let library_target = stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "MyLibraryData".to_string(),
            ))
            .unwrap();
        let library = library_target.library().unwrap();

        let stub_manifest = stub_project.manifest;
        let manifest = stub_manifest.manifest;

        let mut include_directories =
            IncludeDirectories::from_dependencies(&library.dependencies).unwrap();
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.join("include"),
        });

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest.clone(),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&library.sources.clone()).unwrap(),
        });

        let expected = BuildTarget {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Library(LibraryType::Static, "libMyLibraryData.a".to_string()),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual = BuildTarget::library_from_source(&manifest.directory, library).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_shared_library() {
        let fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "MyLibraryData".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                dependencies: Vec::new(),
                compiler_flags: None,
                lib_type: types::LibraryType::Dynamic,
            }))
            .create(manifest_dir);
        let library_target = stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "MyLibraryData".to_string(),
            ))
            .unwrap();
        let library = library_target.library().unwrap();

        let stub_manifest = stub_project.manifest;
        let manifest = stub_manifest.manifest;

        let mut include_directories =
            IncludeDirectories::from_dependencies(&library.dependencies).unwrap();
        include_directories.add(include_directories::IncludeDirectory {
            include_type: include_directories::IncludeType::Include,
            path: manifest.directory.join("include"),
        });

        let target_source = TargetSource::FromSource(SourceBuildData {
            manifest: manifest.clone(),
            dependencies: Vec::new(),
            source_files: SourceFiles::from_paths(&library.sources.clone()).unwrap(),
        });

        let expected = BuildTarget {
            target_source,
            state: TargetState::NotInProcess,
            target_type: TargetType::Library(
                LibraryType::Dynamic,
                "libMyLibraryData.so".to_string(),
            ),
            include_directories,
            compiler_flags: CompilerFlags::new(),
        };
        let actual = BuildTarget::library_from_source(&manifest.directory, library).unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_executable_with_dependency_detect_target() {
        let mut fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let dep_manifest_dir = tempdir::TempDir::new("dependency").unwrap();

        let dep_stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "DependencyLibrary".to_string(),
                sources: vec![
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("x.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("y.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("z.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("a.cpp")),
                ],
                compiler_flags: None,
                lib_type: types::LibraryType::Static,
                dependencies: vec![],
            }))
            .create(dep_manifest_dir.path());

        let dependency_library_target = dep_stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "DependencyLibrary".to_string(),
            ))
            .unwrap();
        let dependency_library = dependency_library_target.library().unwrap();

        let stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                compiler_flags: None,
                dependencies: vec![targets::Dependency::from_source(
                    &dependency_library.name,
                    &types::SourceData {
                        path: dep_manifest_dir.path().to_path_buf(),
                        origin: types::IncludeSearchType::Include,
                    },
                    dep_manifest_dir.path(),
                )
                .unwrap()],
            }))
            .create(manifest_dir);

        let executable_target = stub_project
            .target_with_target_type(TargetType::Executable("x".to_string()))
            .unwrap();
        let executable = executable_target.executable().unwrap();

        let dep_stub_manifest = dep_stub_project.manifest;
        let dep_manifest = dep_stub_manifest.manifest;

        let dependency_build_target =
            BuildTarget::library_from_source(&dep_manifest.directory, dependency_library).unwrap();

        let dependency_source = DependencySource::FromSource(DependencySourceData {
            name: dependency_build_target.name(),
            manifest: dep_manifest,
            library_type: LibraryType::Static,
        });

        let expected = vec![Dependency {
            source: dependency_source,
        }];

        let stub_manifest = stub_project.manifest;
        let manifest = stub_manifest.manifest;
        let build_target =
            BuildTarget::executable_from_source(&manifest.directory, executable).unwrap();
        let actual = build_target
            .detect_target(&mut fixture.stub_registry, &executable_target)
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_executable_with_two_dependencies() {
        let mut fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();

        let dep_manifest_dir = tempdir::TempDir::new("dependency").unwrap();

        let dep_stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "DependencyLibrary".to_string(),
                sources: vec![
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("x.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("y.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("z.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("a.cpp")),
                ],
                compiler_flags: None,
                lib_type: types::LibraryType::Static,
                dependencies: vec![],
            }))
            .create(dep_manifest_dir.path());

        let second_dep_manifest_dir = tempdir::TempDir::new("second_dependency").unwrap();
        let second_dep_stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "SecondDependencyLibrary".to_string(),
                sources: vec![
                    second_dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("x.cpp")),
                    second_dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("y.cpp")),
                    second_dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("z.cpp")),
                    second_dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("a.cpp")),
                ],
                compiler_flags: None,
                lib_type: types::LibraryType::Static,
                dependencies: vec![],
            }))
            .create(second_dep_manifest_dir.path());

        let dependency_library_target = dep_stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "DependencyLibrary".to_string(),
            ))
            .unwrap();
        let dependency_library = dependency_library_target.library().unwrap();

        let second_dependency_library_target = second_dep_stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "SecondDependencyLibrary".to_string(),
            ))
            .unwrap();
        let second_dependency_library = second_dependency_library_target.library().unwrap();

        let stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                compiler_flags: None,
                dependencies: vec![
                    targets::Dependency::from_source(
                        &second_dependency_library.name,
                        &types::SourceData {
                            path: second_dep_manifest_dir.path().to_path_buf(),
                            origin: types::IncludeSearchType::Include,
                        },
                        second_dep_manifest_dir.path(),
                    )
                    .unwrap(),
                    targets::Dependency::from_source(
                        &dependency_library.name,
                        &types::SourceData {
                            path: dep_manifest_dir.path().to_path_buf(),
                            origin: types::IncludeSearchType::Include,
                        },
                        dep_manifest_dir.path(),
                    )
                    .unwrap(),
                ],
            }))
            .create(manifest_dir);

        let executable_target = stub_project
            .target_with_target_type(TargetType::Executable("x".to_string()))
            .unwrap();
        let executable = executable_target.executable().unwrap();

        let dep_stub_manifest = dep_stub_project.manifest;
        let dep_manifest = dep_stub_manifest.manifest;

        let second_dep_stub_manifest = second_dep_stub_project.manifest;
        let second_dep_manifest = second_dep_stub_manifest.manifest;

        let dependency_build_target =
            BuildTarget::library_from_source(&dep_manifest.directory, dependency_library).unwrap();

        let second_dependency_build_target = BuildTarget::library_from_source(
            &second_dep_manifest.directory,
            second_dependency_library,
        )
        .unwrap();

        let second_dependency_source = DependencySource::FromSource(DependencySourceData {
            name: second_dependency_build_target.name(),
            manifest: second_dep_manifest,
            library_type: LibraryType::Static,
        });

        let dependency_source = DependencySource::FromSource(DependencySourceData {
            name: dependency_build_target.name(),
            manifest: dep_manifest,
            library_type: LibraryType::Static,
        });

        let expected = vec![
            Dependency {
                source: second_dependency_source,
            },
            Dependency {
                source: dependency_source,
            },
        ];

        let stub_manifest = stub_project.manifest;
        let manifest = stub_manifest.manifest;
        let build_target =
            BuildTarget::executable_from_source(&manifest.directory, executable).unwrap();
        let actual = build_target
            .detect_target(&mut fixture.stub_registry, &executable_target)
            .unwrap();
        assert_eq!(actual, expected);
    }

    #[test]
    fn can_create_build_target_from_executable_with_dependency() {
        let mut fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let dep_manifest_dir = tempdir::TempDir::new("dependency").unwrap();

        let dep_stub_project = StubProject::builder()
            .with_target(StubTarget::library(targets::Library {
                name: "DependencyLibrary".to_string(),
                sources: vec![
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("x.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("y.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("z.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("a.cpp")),
                ],
                compiler_flags: None,
                lib_type: types::LibraryType::Static,
                dependencies: vec![],
            }))
            .create(dep_manifest_dir.path());

        let dependency_library_target = dep_stub_project
            .target_with_target_type(TargetType::Library(
                LibraryType::Static,
                "DependencyLibrary".to_string(),
            ))
            .unwrap();
        let dependency_library = dependency_library_target.library().unwrap();

        let stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                compiler_flags: None,
                dependencies: vec![targets::Dependency::from_source(
                    &dependency_library.name,
                    &types::SourceData {
                        path: dep_manifest_dir.path().to_path_buf(),
                        origin: types::IncludeSearchType::Include,
                    },
                    dep_manifest_dir.path(),
                )
                .unwrap()],
            }))
            .create(manifest_dir);

        let executable_target = stub_project
            .target_with_target_type(TargetType::Executable("x".to_string()))
            .unwrap();

        BuildTarget::target_node_from_source(
            manifest_dir,
            &executable_target,
            &mut fixture.stub_registry,
        )
        .unwrap();
    }

    #[test]
    fn dependency_can_not_be_an_executable() {
        let mut fixture = TestFixture::new();
        let manifest_dir = fixture.dir.path();
        let dep_manifest_dir = tempdir::TempDir::new("dependency").unwrap();

        let dep_stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "DependencyLibraryButExecutable".to_string(),
                sources: vec![
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("x.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("y.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("z.cpp")),
                    dep_manifest_dir
                        .path()
                        .join(std::path::PathBuf::from("a.cpp")),
                ],
                compiler_flags: None,
                dependencies: vec![],
            }))
            .create(dep_manifest_dir.path());

        let stub_project = StubProject::builder()
            .with_target(StubTarget::executable(targets::Executable {
                name: "x".to_string(),
                sources: vec![
                    manifest_dir.join(std::path::PathBuf::from("x.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("y.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("z.cpp")),
                    manifest_dir.join(std::path::PathBuf::from("main.cpp")),
                ],
                compiler_flags: None,
                dependencies: vec![targets::Dependency::from_source(
                    "DependencyLibraryButExecutable",
                    &types::SourceData {
                        path: dep_manifest_dir.path().to_path_buf(),
                        origin: types::IncludeSearchType::Include,
                    },
                    dep_manifest_dir.path(),
                )
                .unwrap()],
            }))
            .create(manifest_dir);

        let executable_target = stub_project
            .target_with_target_type(TargetType::Executable("x".to_string()))
            .unwrap();

        let dep_executable_target = dep_stub_project
            .target_with_target_type(TargetType::Executable(
                "DependencyLibraryButExecutable".to_string(),
            ))
            .unwrap();

        BuildTarget::target_node_from_source(
            dep_manifest_dir.path(),
            &dep_executable_target,
            &mut fixture.stub_registry,
        )
        .unwrap();

        let actual = BuildTarget::target_node_from_source(
            manifest_dir,
            &executable_target,
            &mut fixture.stub_registry,
        )
        .unwrap_err();
        assert_eq!(actual.to_string(), "Dependency \"DependencyLibraryButExecutable\" parsed is not a library, but an executable");
        assert!(matches!(actual, TargetError::DependencyNotALibrary(_)));
    }

    // TODO:
    // * Test that checks if there is cyclic dependency
}
