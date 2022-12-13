use crate::flags::CompilerFlags;

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RawManifestData {
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
pub enum LibraryType {
    Static,
    #[serde(rename = "shared")]
    Dynamic,
}

impl Default for LibraryType {
    fn default() -> Self {
        LibraryType::Static
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawLibraryData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
    #[serde(default, rename = "type")]
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
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
        let (macro_, value) = s
            .split_once("=")
            .ok_or_else(|| ParseDefineError::IncorrectSyntax)?;
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
pub struct BinaryPath {
    #[serde(rename = "binary_path")]
    pub path: std::path::PathBuf,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct BinaryData {
    #[serde(rename = "debug")]
    pub debug_path_information: BinaryPath,
    #[serde(rename = "release")]
    pub release_path_information: BinaryPath,
    pub include_directory: std::path::PathBuf,
    #[serde(default = "IncludeSearchType::system")]
    pub search_type: IncludeSearchType,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DependencyData {
    Source(SourceData),
    Binary(BinaryData),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub enum IncludeSearchType {
    System,
    Include,
}

impl IncludeSearchType {
    fn system() -> IncludeSearchType {
        IncludeSearchType::System
    }
}

impl Default for IncludeSearchType {
    fn default() -> Self {
        IncludeSearchType::Include
    }
}
