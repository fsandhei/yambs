use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::build_target::pkg_config::PkgConfig;
use crate::compiler::{CCCompiler, CXXCompiler, CompilerError, Linker, StdLibCC, StdLibCXX};
use crate::{find_program, FindProgramOptions};

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

impl Archiver {
    pub fn new() -> Result<Self, ArchiverError> {
        let archiver_exe = {
            if let Some(archiver_from_env) = Self::try_from_environment_variable() {
                log::debug!("Found archiver in $AR. Using this.");
                Ok(archiver_from_env)
            } else {
                log::debug!("Did not find archiver in $AR. Will try to find 'ar' in common installation places.");
                let mut search_options = FindProgramOptions::new();
                search_options.with_path_env();
                if let Some(archiver) = find_program(Path::new("ar"), search_options) {
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
            compiler: CXXCompiler::from_toolchain_cxx_data(toolchain_cxx_data)
                .map_err(ToolchainError::CouldNotGetCompiler)?,
            linker,
        })
    }
}

#[derive(PartialEq, Eq, Debug)]
pub struct ToolchainCC {
    pub compiler: CCCompiler,
    pub linker: Linker,
}

impl ToolchainCC {
    pub fn new() -> Result<Self, ToolchainError> {
        Ok(Self {
            compiler: CCCompiler::new().map_err(ToolchainError::CouldNotGetCompiler)?,
            linker: Linker::new(),
        })
    }

    pub fn from_toolchain_cc_data(
        toolchain_cc_data: &ToolchainCCData,
    ) -> Result<Self, ToolchainError> {
        let linker = if let Some(ref linker) = toolchain_cc_data.linker {
            linker.clone()
        } else {
            Linker::default()
        };
        Ok(Self {
            compiler: CCCompiler::from_toolchain_cc_data(toolchain_cc_data)
                .map_err(ToolchainError::CouldNotGetCompiler)?,
            linker,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
pub struct ToolchainCCData {
    pub compiler: PathBuf,
    pub linker: Option<Linker>,
    #[serde(default)]
    pub stdlib: StdLibCC,
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
struct Toolchain {
    #[serde(rename = "CXX")]
    pub cxx: ToolchainCXXData,
    #[serde(rename = "CC")]
    pub cc: ToolchainCCData,
    pub common: CommonToolchainData,
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
            let toolchain_file_content = String::from_utf8(
                fs::read(path)
                    .map_err(|e| ToolchainError::FailedToParseTomlFile(e, path.to_path_buf()))?,
            )
            .map_err(ToolchainError::FailedToConvertUtf8)?;
            toml::from_str(&toolchain_file_content)
                .map_err(|e| ToolchainError::FailedToParseToolchainFile(e, path.to_path_buf()))
        } else {
            Err(ToolchainError::CouldNotGetFilename)
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

        let pkg_config = {
            if let Some(ref pkg_config) = self.common.pkg_config {
                log::debug!("Using pkg_config found from toolchain file");
                Some(PkgConfig::from_path(pkg_config))
            } else {
                PkgConfig::new().ok()
            }
        };

        Ok(NormalizedToolchain {
            cxx: ToolchainCXX::from_toolchain_cxx_data(&self.cxx)?,
            cc: ToolchainCC::from_toolchain_cc_data(&self.cc)?,
            archiver,
            pkg_config,
        })
    }
}

#[derive(PartialEq, Eq, Debug, Deserialize)]
struct CommonToolchainData {
    pub archiver: Option<PathBuf>,
    #[serde(rename = "pkg-config")]
    pub pkg_config: Option<PathBuf>,
}

#[derive(PartialEq, Eq, Debug)]
pub struct NormalizedToolchain {
    pub cxx: ToolchainCXX,
    pub cc: ToolchainCC,
    pub archiver: Archiver,
    pub pkg_config: Option<PkgConfig>,
}

impl NormalizedToolchain {
    pub fn new() -> Result<Self, ToolchainError> {
        Ok(Self {
            cxx: ToolchainCXX::new()?,
            cc: ToolchainCC::new()?,
            archiver: Archiver::new().map_err(ToolchainError::Archiver)?,
            pkg_config: PkgConfig::new().ok(),
        })
    }

    pub fn from_file(path: &Path) -> Result<Self, ToolchainError> {
        if path.exists() {
            log::debug!("Parsing toolchain at {}", path.display());
            let raw_toolchain = Toolchain::new(path)?;
            raw_toolchain.to_toolchain()
        } else {
            Err(ToolchainError::ToolchainNotFound(path.to_path_buf()))
        }
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
    #[error("Failed to get information about compiler")]
    CouldNotGetCompiler(#[source] CompilerError),
    #[error("Failed to retrieve file name from toolchain file")]
    CouldNotGetFilename,
    #[error("Failed to parse TOML toolchain file {1}")]
    FailedToParseTomlFile(#[source] std::io::Error, PathBuf),
    #[error("Failed to parse toolchain file {1}")]
    FailedToParseToolchainFile(#[source] toml::de::Error, PathBuf),
    #[error("Failed to convert UTF-8 bytes to string")]
    FailedToConvertUtf8(#[source] std::string::FromUtf8Error),
    #[error("Toolchain not found at {0}")]
    ToolchainNotFound(PathBuf),
}
