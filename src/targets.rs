use crate::flags::CompilerFlags;
use crate::YAMBS_MANIFEST_DIR_ENV;

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum Target {
    Executable(Executable),
    Library(Library),
}

impl Target {
    pub fn library(&self) -> Option<&Library> {
        match self {
            Target::Library(library) => Some(library),
            _ => None,
        }
    }

    pub fn executable(&self) -> Option<&Executable> {
        match self {
            Target::Executable(exe) => Some(exe),
            _ => None,
        }
    }

    pub fn dependencies(&self) -> &Vec<Dependency> {
        match self {
            Target::Executable(exec) => &exec.dependencies,
            Target::Library(lib) => &lib.dependencies,
        }
    }
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Executable {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub compiler_flags: Option<CompilerFlags>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub main: std::path::PathBuf,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
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
    pub dependencies: std::collections::HashMap<String, DependencyData>,
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

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    #[serde(flatten)]
    pub data: DependencyData,
}

impl Dependency {
    pub fn new(name: &str, data: &DependencyData) -> Self {
        let (path, origin) = data.from_filesystem().unwrap();
        let canonicalized_data = DependencyData::FromFilesystem {
            path: canonicalize_source(&path),
            origin,
        };
        Self {
            name: name.to_string(),
            data: canonicalized_data,
        }
    }
}

fn canonicalize_source(path: &std::path::Path) -> std::path::PathBuf {
    std::fs::canonicalize(
        std::env::var_os(YAMBS_MANIFEST_DIR_ENV)
            .map(std::path::PathBuf::from)
            .unwrap()
            .join(path),
    )
    .unwrap()
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum DependencyData {
    FromFilesystem {
        path: std::path::PathBuf,
        #[serde(default)]
        origin: DependencySource,
    },
}

impl DependencyData {
    pub fn from_filesystem(&self) -> Option<(std::path::PathBuf, DependencySource)> {
        match self {
            DependencyData::FromFilesystem { path, origin } => {
                Some((path.to_owned(), origin.to_owned()))
            }
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, PartialEq, Eq)]
pub enum DependencySource {
    System,
    Include,
}

impl Default for DependencySource {
    fn default() -> Self {
        DependencySource::Include
    }
}
