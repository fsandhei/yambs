use crate::parser::compiler_flags::CompilerFlags;
use crate::parser::RequiredProject;

#[derive(Debug, PartialEq, Eq)]
pub struct Executable {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub requires: Vec<RequiredProject>,
    pub compiler_flags: Option<CompilerFlags>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub requires: Vec<RequiredProject>,
    pub compiler_flags: Option<CompilerFlags>,
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct RawExecutableData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawLibraryData {
    #[serde(flatten)]
    pub common_raw: RawCommonData,
    #[serde(default)]
    pub lib_type: LibraryType,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawCommonData {
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    #[serde(default)]
    pub requires: Vec<RequiredProject>,
    #[serde(flatten)]
    pub compiler_flags: Option<CompilerFlags>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub enum LibraryType {
    Static,
    Dynamic,
}

impl Default for LibraryType {
    fn default() -> Self {
        LibraryType::Static
    }
}
