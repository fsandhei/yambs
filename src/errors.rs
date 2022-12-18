use log4rs::config::runtime::ConfigErrors;
use thiserror;

#[derive(thiserror::Error, Debug)]
pub enum AssociatedFileError {
    #[error("Could not specify file type")]
    CouldNotSpecifyFileType,
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Error occured when creating cache")]
    FailedToCache(std::io::Error),
    #[error("Error occured when writing to cache")]
    FailedToWrite(serde_json::Error),
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum CommandLineError {
    #[error("Input cannot be non-UTF-8")]
    NonUtf8Input,
    #[error("C++ version \"{0}\" used is not allowed.")]
    InvalidCppVersion(String),
    #[error("Configuration input was not valid.")]
    InvalidConfiguration,
    #[error("release and debug can't be used together. Only use one build configuration.")]
    InvalidBuildConfiguration,
    #[error("Invalid argument used for sanitizer. Valid arguments are address, undefined, leak and thread.")]
    InvalidSanitizerArgument,
    #[error("address cannot be used together with thread. Pick only one.")]
    IllegalSanitizerCombination,
    #[error(transparent)]
    Fs(#[from] FsError),
}

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
    #[error("Could not find program {0}")]
    CouldNotFindProgram(String),
    #[error("Failed to pop from path")]
    PopError,
    #[error("Failed to write to file")]
    WriteToFile(#[source] std::io::Error),
    #[error("Failed to spawn process {0:?}")]
    Spawn(std::process::Command),
    #[error("Failed to spawn child process: {0:?}")]
    SpawnChild(#[source] std::io::Error),
    #[error("Could not access directory")]
    AccessDirectory(#[source] std::io::Error),
    #[error("Could not find include directory from {0:?}")]
    NoIncludeDirectory(std::path::PathBuf),
    #[error("{0:?} does not contain a lib.mmk file!")]
    NoLibraryFile(std::path::PathBuf),
    #[error("Environment variable ${0} is not set.")]
    EnvVariableNotSet(String, #[source] std::env::VarError),
    #[error("Failed to convert utf8 array to string")]
    FailedToCreateStringFromUtf8(#[source] std::string::FromUtf8Error),
    #[error("Failed to execute external program")]
    FailedToExecute(#[source] std::io::Error),
    #[error(
        "{0:?} is not a YAMBS manifest file.\n\
             Hint: Recipe files are called yambs.toml"
    )]
    InvalidRecipeFilename(std::path::PathBuf),
    #[error("Failed to read JSON object from reader.")]
    FailedToReadBufReader(#[source] serde_json::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum LoggerError {
    #[error("Failed to create file appender: {0}")]
    FailedToCreateFileAppender(#[source] std::io::Error),
    #[error("Failed to create logger configuration: {0}")]
    FailedToCreateConfig(#[source] ConfigErrors),
    #[error(transparent)]
    FailedToSetLogger(#[from] log::SetLoggerError),
}
