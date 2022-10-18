use crate::errors::CommandLineError;

use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigurationError {
    #[error("Build configuration \"{0}\" used is not valid.")]
    InvalidBuildConfiguration(String),
    #[error("C++ standard \"{0}\" used is not allowed.")]
    InvalidCXXStandard(String),
    #[error("Invalid sanitizer option set: {0}")]
    InvalidSanitizerOption(String),
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum BuildConfiguration {
    Debug,
    Release,
}

impl std::default::Default for BuildConfiguration {
    fn default() -> Self {
        BuildConfiguration::Debug
    }
}

impl std::str::FromStr for BuildConfiguration {
    type Err = ConfigurationError;
    fn from_str(config: &str) -> Result<Self, Self::Err> {
        match config {
            "release" => Ok(BuildConfiguration::Release),
            "debug" => Ok(BuildConfiguration::Debug),
            _ => Err(Self::Err::InvalidBuildConfiguration(config.to_string())),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum CXXStandard {
    CXX98,
    CXX03,
    CXX11,
    CXX14,
    CXX17,
    CXX20,
}

impl CXXStandard {
    pub fn parse(standard: &str) -> Result<Self, ConfigurationError> {
        let converted_standard = match standard {
            "c++98" => Ok(CXXStandard::CXX98),
            "c++03" => Ok(CXXStandard::CXX03),
            "c++11" => Ok(CXXStandard::CXX11),
            "c++14" => Ok(CXXStandard::CXX14),
            "c++17" => Ok(CXXStandard::CXX17),
            "c++20" => Ok(CXXStandard::CXX20),
            _ => Err(ConfigurationError::InvalidCXXStandard(standard.to_string())),
        };
        converted_standard
    }
}

impl std::default::Default for CXXStandard {
    fn default() -> Self {
        CXXStandard::CXX17
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Sanitizer {
    Address,
    Thread,
    Memory,
    Leak,
}

impl std::str::FromStr for Sanitizer {
    type Err = ConfigurationError;

    fn from_str(sanitizer: &str) -> Result<Self, Self::Err> {
        match sanitizer {
            "address" => Ok(Sanitizer::Address),
            "thread" => Ok(Sanitizer::Thread),
            "memory" => Ok(Sanitizer::Memory),
            "leak" => Ok(Sanitizer::Leak),
            _ => Err(Self::Err::InvalidSanitizerOption(sanitizer.to_string())),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BuildDirectory(std::path::PathBuf);

impl BuildDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}

impl std::convert::From<std::path::PathBuf> for BuildDirectory {
    fn from(f: std::path::PathBuf) -> Self {
        Self { 0: f }
    }
}

impl Default for BuildDirectory {
    fn default() -> Self {
        Self {
            0: std::env::current_dir().expect("Could not locate current directory."),
        }
    }
}

impl std::string::ToString for BuildDirectory {
    fn to_string(&self) -> String {
        self.0.display().to_string()
    }
}

impl std::str::FromStr for BuildDirectory {
    type Err = CommandLineError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let canonicalized_path = canonicalize_path(&std::path::PathBuf::from(s))
            .map_err(crate::errors::FsError::Canonicalize)?;
        Ok(Self {
            0: canonicalized_path,
        })
    }
}

fn canonicalize_path(path: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    if !path.is_absolute() {
        return Ok(std::env::current_dir()?.join(path));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn build_configuration_is_debug_from_str() {
        let build_configuration = BuildConfiguration::from_str("debug").unwrap();
        assert_eq!(build_configuration, BuildConfiguration::Debug);
    }

    #[test]
    fn build_configuration_is_release_from_str() {
        let build_configuration = BuildConfiguration::from_str("release").unwrap();
        assert_eq!(build_configuration, BuildConfiguration::Release);
    }

    #[test]
    fn build_configuration_is_debug_by_default() {
        let build_configuration = BuildConfiguration::from_str("relwithdebinfo");
        assert_eq!(
            build_configuration.unwrap_err(),
            ConfigurationError::InvalidBuildConfiguration("relwithdebinfo".to_string())
        );
    }

    #[test]
    fn cxxstandard_parse_cpp98_test() {
        let cpp_version = CXXStandard::parse("c++98").unwrap();
        assert_eq!(cpp_version, CXXStandard::CXX98);
    }

    #[test]
    fn cxxstandard_parse_cpp11_test() {
        let cpp_version = CXXStandard::parse("c++11").unwrap();
        assert_eq!(cpp_version, CXXStandard::CXX11);
    }

    #[test]
    fn cxxstandard_parse_cpp14_test() {
        let cpp_version = CXXStandard::parse("c++14").unwrap();
        assert_eq!(cpp_version, CXXStandard::CXX14);
    }

    #[test]
    fn cxxstandard_parse_cpp17_test() {
        let cpp_version = CXXStandard::parse("c++17").unwrap();
        assert_eq!(cpp_version, CXXStandard::CXX17);
    }

    #[test]
    fn cxxstandard_parse_cpp20_test() {
        let cpp_version = CXXStandard::parse("c++20").unwrap();
        assert_eq!(cpp_version, CXXStandard::CXX20);
    }

    #[test]
    fn parse_fails_on_invalid_version() {
        let result = CXXStandard::parse("python");
        assert!(result.is_err());
    }
}
