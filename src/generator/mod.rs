use crate::build_target::{target_registry::TargetRegistry, TargetError};
use crate::errors::FsError;

mod makefilegenerator;

pub use makefilegenerator::MakefileGenerator;

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

pub trait Generator {
    fn generate(&mut self, registry: &TargetRegistry) -> Result<(), GeneratorError>;
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

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub struct ObjectTarget {
        pub object: std::path::PathBuf,
        pub source: std::path::PathBuf,
        pub include_directories: IncludeDirectories,
    }
}
