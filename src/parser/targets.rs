use crate::parser::compiler_flags::CompilerFlags;
use crate::parser::{ExecutableData, LibraryData, RequiredProject};

#[derive(Debug, PartialEq, Eq)]
pub struct Target(pub either::Either<Executable, Library>);

#[derive(Debug, PartialEq, Eq)]
pub struct Executable {
    pub data: ExecutableData,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Library {
    pub data: LibraryData,
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
    pub requires: Option<Vec<RequiredProject>>,
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
