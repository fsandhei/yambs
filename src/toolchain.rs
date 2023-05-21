use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::compiler::{CXXCompiler, CompilerError, Linker, StdLibCXX};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const TOOLCHAIN_FILE_NAME: &str = "toolchain.toml";

#[derive(PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct Archiver {
    pub path: PathBuf,
}

#[derive(Debug, Error)]
pub enum ArchiverError {
    #[error("No archiver found")]
    NoArchiverFound,
    #[error("Archiver does not exist")]
    ArchiverDoesNotExist,
}

lazy_static::lazy_static! {
    static ref PATH_ENV_PATHS: Vec<PathBuf> = {
        let path_env = std::env::var_os("PATH").unwrap();
        std::env::split_paths(&path_env).collect::<Vec<PathBuf>>()
    };
}

fn find_program(program: &Path) -> Option<std::path::PathBuf>
where
{
    for path in &*PATH_ENV_PATHS {
        log::debug!("Looking for {} in {}", program.display(), path.display());
        let executable_path = path.join(&program);
        if executable_path.is_file() {
            log::debug!(
                "Found {} as {}",
                program.display(),
                executable_path.display()
            );
            return Some(executable_path);
        }
    }
    None
}

impl Archiver {
    pub fn new() -> Result<Self, ArchiverError> {
        let archiver_exe = {
            if let Some(archiver_from_env) = Self::try_from_environment_variable() {
                log::debug!("Found archiver in $AR. Using this.");
                Ok(archiver_from_env)
            } else {
                log::debug!("Did not find archiver in $AR. Will try to find 'ar' in common installation places.");
                if let Some(archiver) = find_program(&Path::new("ar")) {
                    Ok(archiver)
                } else {
                    return Err(ArchiverError::NoArchiverFound);
                }
            }
        }?;
        Ok(Self { path: archiver_exe })
    }

    pub fn from_path(path: &Path) -> Result<Self, ArchiverError> {
        Ok(Self {
            path: path.to_path_buf(),
        })
    }

    fn try_from_environment_variable() -> Option<PathBuf> {
        env::var_os("AR").map(PathBuf::from)
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct ToolchainCXXData {
    pub compiler: PathBuf,
    pub linker: Option<Linker>,
    #[serde(default)]
    pub stdlib: StdLibCXX,
}

#[derive(PartialEq, Eq, Debug)]
pub struct ToolchainCXX {
    pub compiler: CXXCompiler,
    pub linker: Linker,
}

impl ToolchainCXX {
    pub fn new() -> Result<Self, ToolchainError> {
        Ok(Self {
            compiler: CXXCompiler::new().map_err(ToolchainError::CouldNotGetCompiler)?,
            linker: Linker::new(),
        })
    }

    pub fn from_toolchain_cxx_data(
        toolchain_cxx_data: &ToolchainCXXData,
    ) -> Result<Self, ToolchainError> {
        let linker = if let Some(ref linker) = toolchain_cxx_data.linker {
            linker.clone()
        } else {
            Linker::default()
        };
        Ok(Self {
            compiler: CXXCompiler::from_toolchain_cxx_data(&toolchain_cxx_data)
                .map_err(ToolchainError::CouldNotGetCompiler)?,
            linker,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
struct Toolchain {
    #[serde(rename = "CXX")]
    pub cxx: ToolchainCXXData,
    #[serde(flatten)]
    pub common: CommonToolchainData,
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
struct CommonToolchainData {
    pub archiver: Option<PathBuf>,
}

impl Toolchain {
    fn new(path: &Path) -> Result<Self, ToolchainError> {
        if !path.is_file() {
            return Err(ToolchainError::NotAFile);
        }
        if let Some(file_name) = path.file_name() {
            if file_name != TOOLCHAIN_FILE_NAME {
                return Err(ToolchainError::IncorrectFilename);
            }
            let toolchain_file_content =
                String::from_utf8(fs::read(path).map_err(ToolchainError::FailedToParseTomlFile)?)
                    .map_err(ToolchainError::FailedToConvertUtf8)?;
            toml::from_str(&toolchain_file_content)
                .map_err(ToolchainError::FailedToParseToolchainFile)
        } else {
            return Err(ToolchainError::CouldNotGetFilename);
        }
    }

    fn to_toolchain(&self) -> Result<NormalizedToolchain, ToolchainError> {
        let archiver = {
            if let Some(ref archiver) = self.common.archiver {
                log::debug!("Using archiver found from toolchain file");
                Archiver::from_path(archiver)
            } else {
                Archiver::new()
            }
        }
        .map_err(ToolchainError::Archiver)?;

        Ok(NormalizedToolchain {
            cxx: ToolchainCXX::from_toolchain_cxx_data(&self.cxx)?,
            archiver,
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct NormalizedToolchain {
    pub cxx: ToolchainCXX,
    pub archiver: Archiver,
}

impl NormalizedToolchain {
    pub fn new() -> Result<Self, ToolchainError> {
        Ok(Self {
            cxx: ToolchainCXX::new()?,
            archiver: Archiver::new().map_err(ToolchainError::Archiver)?,
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, ToolchainError> {
        log::debug!("Parsing toolchain at {}", path.display());
        let raw_toolchain = Toolchain::new(path)?;
        raw_toolchain.to_toolchain()
    }
}

#[derive(Debug, Error)]
pub enum ToolchainError {
    #[error("Error occured with locating archiver")]
    Archiver(#[source] ArchiverError),
    #[error("Path to toolchain file is not a file")]
    NotAFile,
    #[error(
        "File name for toolchain file is incorrect. Toolchain file shall be named {}",
        TOOLCHAIN_FILE_NAME
    )]
    IncorrectFilename,
    #[error("Failed to get information about compiler specified in environment variable CXX")]
    CouldNotGetCompiler(#[source] CompilerError),
    #[error("Failed to retrieve file name from toolchain file")]
    CouldNotGetFilename,
    #[error("Failed to parse TOML toolchain file")]
    FailedToParseTomlFile(#[source] std::io::Error),
    #[error("Failed to parse toolchain file")]
    FailedToParseToolchainFile(#[source] toml::de::Error),
    #[error("Failed to convert UTF-8 bytes to string")]
    FailedToConvertUtf8(#[source] std::string::FromUtf8Error),
}
