use std::path::PathBuf;

use crate::cli::configurations::CXXStandard;
use crate::flags::CompilerFlags;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Language {
    #[serde(rename = "C++")]
    CXX,
    C,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectConfig {
    pub cxx_std: Option<CXXStandard>,
    pub language: Option<Language>,
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
