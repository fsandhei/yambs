use std::path::PathBuf;
use thiserror;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MyMakeError {
    #[error("Error occured during compilation: {description}")]
    CompileTime { description: String },
    #[error("Error occured during configure time: {0}")]
    ConfigurationTime(String),
    #[error("{description}")]
    Generic { description: String },
    #[error("Error occured during parsing")]
    Parse(#[source] ParseError),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error(transparent)]
    Dependency(#[from] DependencyError),
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    #[error("{0}: called in an unexpected way.")]
    UnexpectedCall(String),
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Make(#[from] MakeError),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum DependencyError {
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error("Dependency circulation! {0:?} depends on {1:?}, which depends on itself")]
    Circulation(PathBuf, PathBuf),
    #[error("Call on get_dependency when dependency is not set. Call on set_dependency must be done prior!")]
    NotSet,
    #[error("Dependency does not have a makefile")]
    NoMakefile,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error("Error occured in creating directory {0:?}")]
    CreateDirectory(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured in removing directory {0:?}")]
    RemoveDirectory(std::path::PathBuf, #[source] std::io::Error),
    #[error("Failed to create symlink between {dest:?} and {src:?}")]
    CreateSymlink {
        dest: std::path::PathBuf,
        src: std::path::PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("Error occured in removing file {0:?}")]
    RemoveFile(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured in creating file {0:?}")]
    CreateFile(std::path::PathBuf, #[source] std::io::Error),
    #[error("Error occured reading from file {0:?}")]
    ReadFromFile(std::path::PathBuf, #[source] std::io::Error),
    #[error("The path {0:?} does not exist")]
    FileDoesNotExist(std::path::PathBuf),
    #[error("Failed to canonicalize path")]
    Canonicalize(#[source] std::io::Error),
    #[error("Failed to pop from path")]
    PopError,
    #[error("Failed to write to file")]
    WriteToFile(#[source] std::io::Error),
    #[error("Failed to spawn process {0:?}")]
    Spawn(std::process::Command),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum GeneratorError {
    #[error("C++ version \"{0}\" used is not allowed.")]
    InvalidCppVersion(String),
    #[error(transparent)]
    Fs(#[from] FsError),
    #[error(transparent)]
    Toolchain(#[from] ToolchainError),
    #[error("No settings exist for compiler {0:?}")]
    NoCompiler(PathBuf),
    #[error(transparent)]
    Dependency(#[from] DependencyError),
    #[error("Error occured creating rule")]
    CreateRule,
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("{file}: {keyword} is not a valid MMK keyword!")]
    InvalidKeyword {
        file: std::path::PathBuf,
        keyword: String,
    },
    #[error(
        "{file}: Invalid spacing of arguments! Keep at least one line between each RsMake keyword."
    )]
    InvalidSpacing { file: std::path::PathBuf },
    #[error(transparent)]
    FileSystem(#[from] FsError),
    #[error("{0:?} is not a valid RsMake filename! File must be named lib.mmk or run.mmk.")]
    InvalidFilename(String),
    #[error(transparent)]
    Toolchain(#[from] ToolchainError),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum MakeError {
    #[error("The following error occured from the file system: {0})")]
    Fs(#[source] FsError),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum ToolchainError {
    #[error("Key \"{0}\" could not not be found")]
    KeyNotFound(String),
    #[error("\"{0}\" is not allowed as keyword for toolchain.")]
    InvalidKeyword(String),
    #[error("{0} is not a valid name for toolchain file. It must be named toolchain.mmk")]
    InvalidName(String),
    #[error(transparent)]
    FileSystem(#[from] FsError),
}
