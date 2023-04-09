use crate::build_target::{target_registry::TargetRegistry, TargetError};
use crate::errors::FsError;

#[cfg(target_os = "linux")]
pub mod makefile;

#[cfg(target_os = "linux")]
pub use makefile::MakefileGenerator;

#[cfg(target_os = "linux")]
pub(crate) const STATIC_LIBRARY_FILE_EXTENSION: &str = "a";
#[cfg(target_os = "linux")]
pub(crate) const SHARED_LIBRARY_FILE_EXTENSION: &str = "so";

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Dependency(#[from] TargetError),
    #[error("Error occured creating rule")]
    CreateRule,
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

pub trait Sanitizer {
    fn set_sanitizer(&mut self, sanitizer: &str);
}

pub trait UtilityGenerator<'config> {
    fn generate_build_files(&'config mut self) -> Result<(), GeneratorError>;
    fn add_cpp_version(&mut self, version: &'config str);
    fn print_cpp_version(&'config self) -> &'config str;
    fn generate_flags_sanitizer(&self) -> String;
}

pub mod targets {
    use crate::build_target::include_directories::IncludeDirectories;
    use crate::build_target::{DependencySource, TargetNode, TargetSource};

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
            let target_dependencies = match target_node.borrow().target_source {
                TargetSource::FromSource(ref s) => {
                    s.dependencies
                        .iter()
                        .filter_map(|d| match d.source {
                            DependencySource::FromSource(ref ds) => Some(ds),
                            DependencySource::FromPrebuilt(_)
                            | DependencySource::FromHeaderOnly(_) => None,
                        })
                        .map(|ds| ds.name.to_owned())
                        .collect::<Vec<String>>()
                }
                TargetSource::FromPrebuilt(_) => Vec::new(),
            };

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
            let source_data = borrowed_target.target_source.from_source().unwrap();
            let sources = source_data
                .source_files
                .iter()
                .filter(|file| file.is_source());
            let dependency_root_path = &source_data.manifest.directory;
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
                let object_target = ObjectTarget {
                    target: target_name.clone(),
                    object,
                    source: source_file,
                    include_directories: borrowed_target.include_directories.clone(),
                };

                object_targets.push(object_target);
            }
            object_targets
        }
    }
}
