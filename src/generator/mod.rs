use crate::build_target::{target_registry::TargetRegistry, TargetError};
use crate::errors::FsError;

#[cfg(target_os = "linux")]
pub mod makefile;

#[cfg(target_os = "linux")]
pub use makefile::MakefileGenerator;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Dependency(#[from] TargetError),
    #[error("Error occured creating rule")]
    CreateRule,
    #[error("Could not find any standards to use when generating build files")]
    StandardNotFound,
}

#[derive(clap::ValueEnum, Debug, Clone, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum GeneratorType {
    #[cfg(target_os = "linux")]
    /// Use GNU Makefiles
    GNUMakefiles,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GeneratorInfo {
    #[serde(rename = "type")]
    pub type_: GeneratorType,
    pub buildfile_directory: std::path::PathBuf,
}

pub trait Generator {
    /// Generate build files based on the information from the target registry.
    /// Returns the directory of the main build file.
    fn generate(&mut self, registry: &TargetRegistry)
        -> Result<std::path::PathBuf, GeneratorError>;
}

pub trait UtilityGenerator<'config> {
    fn generate_build_files(&'config mut self) -> Result<(), GeneratorError>;
    fn add_cpp_version(&mut self, version: &'config str);
    fn print_cpp_version(&'config self) -> &'config str;
}

pub mod targets {
    use crate::build_target::include_directories::IncludeDirectories;
    use crate::build_target::{DependencySource, TargetNode};

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ProgressDocument {
        pub targets: Vec<ProgressTrackingTarget>,
    }

    impl ProgressDocument {
        pub fn new() -> Self {
            Self {
                targets: Vec::new(),
            }
        }

        pub fn add_progress_tracking_target(&mut self, target: ProgressTrackingTarget) {
            self.targets.push(target)
        }
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ProgressTrackingTarget {
        pub target: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub object_files: Vec<std::path::PathBuf>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub dependencies: Vec<String>,
    }

    impl ProgressTrackingTarget {
        pub fn from_target(target_node: &TargetNode, output_directory: &std::path::Path) -> Self {
            let target_object_targets =
                ObjectTarget::create_object_targets(target_node, output_directory)
                    .iter()
                    .map(|o| o.object.to_path_buf())
                    .collect::<Vec<std::path::PathBuf>>();
            let target_name = target_node.borrow().name();
            let target_dependencies = target_node
                .borrow()
                .dependencies
                .iter()
                .filter_map(|d| match d.source {
                    DependencySource::FromSource(ref ds) => Some(ds),
                    _ => None,
                })
                .map(|ds| ds.library.name.to_owned())
                .collect::<Vec<String>>();

            Self {
                target: target_name,
                object_files: target_object_targets,
                dependencies: target_dependencies,
            }
        }
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ObjectTarget {
        pub target: String,
        pub object: std::path::PathBuf,
        pub source: std::path::PathBuf,
        pub include_directories: IncludeDirectories,
    }

    impl ObjectTarget {
        pub fn create_object_targets(
            target: &TargetNode,
            output_directory: &std::path::Path,
        ) -> Vec<ObjectTarget> {
            let mut object_targets = Vec::new();
            let borrowed_target = target.borrow();
            let sources = borrowed_target
                .source_files
                .iter()
                .filter(|file| file.is_source());
            let dependency_root_path = &borrowed_target.manifest.directory;
            let target_name = borrowed_target.name();

            for source in sources {
                let source_file = source.file();
                let source_dir = source_file
                    .parent()
                    .and_then(|p| p.strip_prefix(dependency_root_path).ok());

                let object = {
                    if let Some(dir) = source_dir {
                        output_directory
                            .join(dir)
                            .join(source_file.file_name().unwrap())
                    } else {
                        output_directory.join(source_file.file_name().unwrap())
                    }
                }
                .with_extension("o");
                let include_directories = {
                    let mut include_directories = IncludeDirectories::new();
                    include_directories.add(borrowed_target.include_directory.clone());
                    let deps = &borrowed_target.dependencies;
                    for dep in deps {
                        match dep.source {
                            DependencySource::FromSource(ref sd) => {
                                let include_dir = sd.include_directory.clone();
                                include_directories.add(include_dir);
                            }
                            DependencySource::FromHeaderOnly(ref hd) => {
                                include_directories.add(hd.include_directory.clone());
                            }
                            DependencySource::FromPkgConfig(ref pkg) => {
                                for dir in &pkg.include_directories {
                                    include_directories.add(dir.clone());
                                }
                            }
                        }
                    }
                    include_directories
                };

                let object_target = ObjectTarget {
                    target: target_name.clone(),
                    object,
                    source: source_file,
                    include_directories,
                };

                object_targets.push(object_target);
            }
            object_targets
        }
    }
}
