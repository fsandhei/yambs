use crate::build_target::{target_registry::TargetRegistry, TargetError};
use crate::cache;
use crate::errors::FsError;

pub mod makefile;

pub use makefile::MakefileGenerator;

pub(crate) const STATIC_LIBRARY_FILE_EXTENSION: &str = "a";
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum GeneratorType {
    GNUMakefile,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GeneratorInfo {
    #[serde(rename = "type")]
    pub type_: GeneratorType,
    pub buildfile_directory: std::path::PathBuf,
}

impl cache::Cacher for GeneratorInfo {
    const CACHE_FILE_NAME: &'static str = "generator_info";
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
    fn generate_makefiles(&'config mut self) -> Result<(), GeneratorError>;
    fn add_cpp_version(&mut self, version: &'config str);
    fn print_cpp_version(&'config self) -> &'config str;
    fn generate_flags_sanitizer(&self) -> String;
}

pub mod targets {
    use crate::build_target::include_directories::IncludeDirectories;

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ProgressDocument {
        pub targets: Vec<ProgressTrackingTarget>,
    }

    #[derive(Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ProgressTrackingTarget {
        pub target: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub object_files: Vec<std::path::PathBuf>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        pub dependencies: Vec<String>,
    }

    #[derive(Clone, Debug, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
    pub struct ObjectTarget {
        pub target: String,
        pub object: std::path::PathBuf,
        pub source: std::path::PathBuf,
        pub include_directories: IncludeDirectories,
    }
}
