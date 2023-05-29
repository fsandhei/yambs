use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ConfigurationError {
    #[error("Build configuration \"{0}\" used is not valid.")]
    InvalidBuildType(String),
    #[error("Invalid sanitizer option set: {0}")]
    InvalidSanitizerOption(String),
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize, Default)]
pub enum BuildType {
    #[default]
    Debug,
    Release,
}

impl std::str::FromStr for BuildType {
    type Err = ConfigurationError;
    fn from_str(config: &str) -> Result<Self, Self::Err> {
        match config {
            "release" => Ok(BuildType::Release),
            "debug" => Ok(BuildType::Debug),
            _ => Err(Self::Err::InvalidBuildType(config.to_string())),
        }
    }
}

impl std::string::ToString for BuildType {
    fn to_string(&self) -> String {
        match self {
            BuildType::Release => "release".to_string(),
            BuildType::Debug => "debug".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Sanitizer {
    Address,
    Thread,
    Memory,
    Leak,
}

impl std::str::FromStr for Sanitizer {
    type Err = ConfigurationError;

    fn from_str(sanitizer: &str) -> Result<Self, Self::Err> {
        match sanitizer.to_lowercase().as_str() {
            "address" => Ok(Sanitizer::Address),
            "thread" => Ok(Sanitizer::Thread),
            "memory" => Ok(Sanitizer::Memory),
            "leak" => Ok(Sanitizer::Leak),
            _ => Err(Self::Err::InvalidSanitizerOption(sanitizer.to_string())),
        }
    }
}

impl std::string::ToString for Sanitizer {
    fn to_string(&self) -> String {
        match self {
            Sanitizer::Address => "address".to_string(),
            Sanitizer::Thread => "thread".to_string(),
            Sanitizer::Memory => "memory".to_string(),
            Sanitizer::Leak => "leak".to_string(),
        }
    }
}

// fn parse_sanitizer_options(sanitizer_options: &[&Configuration]) -> Result<(), CommandLineError> {
//     if sanitizer_options.contains(&&Configuration::Sanitizer("address".to_string()))
//         && sanitizer_options.contains(&&Configuration::Sanitizer("thread".to_string()))
//     {
//         return Err(CommandLineError::IllegalSanitizerCombination);
//     }
//     Ok(())
// }

#[cfg(test)]
mod tests {
    use super::*;

    use std::str::FromStr;

    #[test]
    fn build_configuration_is_debug_from_str() {
        let build_configuration = BuildType::from_str("debug").unwrap();
        assert_eq!(build_configuration, BuildType::Debug);
    }

    #[test]
    fn build_configuration_is_release_from_str() {
        let build_configuration = BuildType::from_str("release").unwrap();
        assert_eq!(build_configuration, BuildType::Release);
    }

    #[test]
    fn build_configuration_is_debug_by_default() {
        let build_configuration = BuildType::from_str("relwithdebinfo");
        assert_eq!(
            build_configuration.unwrap_err(),
            ConfigurationError::InvalidBuildType("relwithdebinfo".to_string())
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
