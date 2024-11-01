use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::flags::CompilerFlags;

#[derive(Debug, Error)]
pub enum ParseStandardError {
    #[error("C standard \"{0}\" used is not allowed.")]
    InvalidCStandard(String),
    #[error("C++ standard \"{0}\" used is not allowed.")]
    InvalidCXXStandard(String),
    #[error("Could not recognize the given standard: \"{0}\"")]
    UnrecognizedStandard(String),
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub std: Option<Standard>,
    pub language: Option<Language>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Language {
    #[serde(rename = "C++")]
    CXX,
    C,
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CXX => write!(f, "C++"),
            Self::C => write!(f, "C"),
        }
    }
}

impl FromStr for Language {
    type Err = ParseLanguageError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "C++" => Ok(Self::CXX),
            "C" => Ok(Self::C),
            _ => Err(Self::Err::InvalidLanguage(s.to_string())),
        }
    }
}

#[derive(Debug, Error)]
pub enum ParseLanguageError {
    #[error(
        "Language input is not valid: {0}.
    Either pick C or C++"
    )]
    InvalidLanguage(String),
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Standard {
    CXX(CXXStandard),
    C(CStandard),
}

impl Standard {
    pub fn new(standard: &str, language: &Language) -> Result<Self, ParseStandardError> {
        match language {
            Language::CXX => Ok(Self::CXX(CXXStandard::parse(standard)?)),
            Language::C => Ok(Self::C(CStandard::parse(standard)?)),
        }
    }

    pub fn verify_from_language(&self, language: &Language) -> Result<(), ParseStandardError> {
        let allowed_language = match self {
            Self::C(_) => Language::C,
            Self::CXX(_) => Language::CXX,
        };
        if language == &allowed_language {
            Ok(())
        } else {
            match language {
                Language::CXX => Err(ParseStandardError::InvalidCXXStandard(self.to_string())),
                Language::C => Err(ParseStandardError::InvalidCStandard(self.to_string())),
            }
        }
    }

    pub fn parse(standard: &str) -> Result<Self, ParseStandardError> {
        match CXXStandard::parse(standard) {
            Ok(cxx) => return Ok(Self::CXX(cxx)),
            Err(_) => {}
        };
        match CStandard::parse(standard) {
            Ok(c) => return Ok(Self::C(c)),
            Err(_) => {}
        }
        Err(ParseStandardError::UnrecognizedStandard(
            standard.to_string(),
        ))
    }
}

impl std::string::ToString for Standard {
    fn to_string(&self) -> String {
        match self {
            Self::CXX(cxx) => cxx.to_string(),
            Self::C(c) => c.to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum CStandard {
    C89,
    C90,
    C11,
    C17,
}

impl CStandard {
    pub fn parse(standard: &str) -> Result<Self, ParseStandardError> {
        let converted_standard = match standard.to_lowercase().as_str() {
            "c89" => Ok(CStandard::C89),
            "c90" => Ok(CStandard::C90),
            "c11" => Ok(CStandard::C11),
            "c17" => Ok(CStandard::C17),
            _ => Err(ParseStandardError::InvalidCStandard(standard.to_string())),
        };
        converted_standard
    }
}

impl std::string::ToString for CStandard {
    fn to_string(&self) -> String {
        match self {
            Self::C89 => "c89".to_string(),
            Self::C90 => "c90".to_string(),
            Self::C11 => "c11".to_string(),
            Self::C17 => "c17".to_string(),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum CXXStandard {
    CXX98,
    CXX03,
    CXX11,
    CXX14,
    CXX17,
    CXX20,
    CXX23,
}

impl CXXStandard {
    pub fn parse(standard: &str) -> Result<Self, ParseStandardError> {
        let converted_standard = match standard.to_lowercase().as_str() {
            "c++98" => Ok(CXXStandard::CXX98),
            "c++03" => Ok(CXXStandard::CXX03),
            "c++11" => Ok(CXXStandard::CXX11),
            "c++14" => Ok(CXXStandard::CXX14),
            "c++17" => Ok(CXXStandard::CXX17),
            "c++20" => Ok(CXXStandard::CXX20),
            "c++23" => Ok(CXXStandard::CXX23),
            _ => Err(ParseStandardError::InvalidCXXStandard(standard.to_string())),
        };
        converted_standard
    }
}

impl std::default::Default for CXXStandard {
    fn default() -> Self {
        CXXStandard::CXX17
    }
}

impl std::string::ToString for CXXStandard {
    fn to_string(&self) -> String {
        match self {
            CXXStandard::CXX98 => "c++98".to_string(),
            CXXStandard::CXX03 => "c++03".to_string(),
            CXXStandard::CXX11 => "c++11".to_string(),
            CXXStandard::CXX14 => "c++14".to_string(),
            CXXStandard::CXX17 => "c++17".to_string(),
            CXXStandard::CXX20 => "c++20".to_string(),
            CXXStandard::CXX23 => "c++23".to_string(),
        }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RawManifestData {
    pub project_config: Option<ProjectConfig>,
    #[serde(rename = "executable")]
    pub executables: Option<std::collections::BTreeMap<String, RawExecutableData>>,
    #[serde(rename = "library")]
    pub libraries: Option<std::collections::BTreeMap<String, RawLibraryData>>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RawExecutableData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LibraryType {
    #[default]
    Static,
    #[serde(rename = "shared")]
    Dynamic,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawLibraryData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
    #[serde(default, rename = "type")]
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawCommonData {
    pub sources: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub dependencies: std::collections::BTreeMap<String, DependencyData>,
    #[serde(flatten)]
    pub compiler_flags: CompilerFlags,
    #[serde(default)]
    pub defines: Vec<Define>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Define {
    #[serde(rename = "macro")]
    pub macro_: String,
    #[serde(rename = "value")]
    pub value: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ParseDefineError {
    #[error("Incorrect syntax. Must be <key>=<value>")]
    IncorrectSyntax,
}

impl Define {
    pub fn from_cli(s: &str) -> Result<Self, ParseDefineError> {
        let (macro_, value) = s.split_once('=').ok_or(ParseDefineError::IncorrectSyntax)?;
        Ok(Self {
            macro_: macro_.to_string(),
            value: Some(value.to_string()),
        })
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct SourceData {
    pub path: std::path::PathBuf,
    #[serde(default)]
    pub origin: IncludeSearchType,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct HeaderOnlyData {
    pub include_directory: std::path::PathBuf,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct PkgConfigData {
    #[serde(rename = "pkg_config_search_dir")]
    pub search_dir: PathBuf,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DependencyData {
    Source(SourceData),
    HeaderOnly(HeaderOnlyData),
    PkgConfig(PkgConfigData),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, Default)]
pub enum IncludeSearchType {
    System,
    #[default]
    Include,
}

#[cfg(test)]
mod tests {
    use super::*;

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
