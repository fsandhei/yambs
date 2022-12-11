use crate::flags::CompilerFlags;
use crate::parser::types;

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Executable {
    pub name: String,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub compiler_flags: CompilerFlags,
    pub defines: Vec<types::Define>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Library {
    pub name: String,
    pub sources: Vec<std::path::PathBuf>,
    pub dependencies: Vec<Dependency>,
    pub compiler_flags: CompilerFlags,
    pub lib_type: types::LibraryType,
    pub defines: Vec<types::Define>,
}

#[derive(thiserror::Error, Debug)]
pub enum DependencyError {
    #[error("Failed to canonicalize path \"{0}\"")]
    FailedToCanonicalizePath(std::path::PathBuf, #[source] std::io::Error),
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
pub struct Dependency {
    pub name: String,
    #[serde(flatten)]
    pub data: types::DependencyData,
}

impl Dependency {
    pub fn new(
        name: &str,
        data: &types::DependencyData,
        manifest_dir: &std::path::Path,
    ) -> Result<Self, DependencyError> {
        let dependency: Result<Self, DependencyError>;
        match data {
            types::DependencyData::Source(ref source_data) => {
                log::debug!(
                    "Found dependency {} in path {} with origin {:?}",
                    name,
                    source_data.path.display(),
                    source_data.origin
                );
                dependency = Dependency::from_source(name, source_data, manifest_dir);
            }
            types::DependencyData::Binary(ref binary_data) => {
                log::debug!(
                    "Found prebuilt dependency {}, with search type {:?}
                    release configuration: {:?}
                    debug configuration: {:?}
                    ",
                    name,
                    binary_data.search_type,
                    binary_data.release_path_information,
                    binary_data.debug_path_information,
                );
                dependency = Dependency::from_binary(name, binary_data, manifest_dir);
            }
        }
        dependency
    }

    pub fn from_source(
        name: &str,
        source_data: &types::SourceData,
        manifest_dir: &std::path::Path,
    ) -> Result<Self, DependencyError> {
        let canonicalized_path = crate::canonicalize_source(manifest_dir, &source_data.path)
            .map_err(|err| {
                DependencyError::FailedToCanonicalizePath(source_data.path.clone(), err)
            })?;
        let canonicalized_data = types::DependencyData::Source(types::SourceData {
            path: canonicalized_path,
            origin: source_data.origin.clone(),
        });
        Ok(Self {
            name: name.to_string(),
            data: canonicalized_data,
        })
    }

    pub fn from_binary(
        name: &str,
        binary_data: &types::BinaryData,
        manifest_dir: &std::path::Path,
    ) -> Result<Self, DependencyError> {
        let canonicalized_release_information = types::BinaryPath {
            path: crate::canonicalize_source(
                manifest_dir,
                &binary_data.release_path_information.path,
            )
            .map_err(|err| {
                DependencyError::FailedToCanonicalizePath(
                    binary_data.release_path_information.path.clone(),
                    err,
                )
            })?,
        };
        let include_directory = crate::canonicalize_source(
            manifest_dir,
            &binary_data.include_directory,
        )
        .map_err(|err| {
            DependencyError::FailedToCanonicalizePath(binary_data.include_directory.clone(), err)
        })?;

        let canonicalized_debug_information = types::BinaryPath {
            path: crate::canonicalize_source(
                manifest_dir,
                &binary_data.debug_path_information.path,
            )
            .map_err(|err| {
                DependencyError::FailedToCanonicalizePath(
                    binary_data.debug_path_information.path.clone(),
                    err,
                )
            })?,
        };

        let canonicalized_data = types::DependencyData::Binary(types::BinaryData {
            release_path_information: canonicalized_release_information,
            debug_path_information: canonicalized_debug_information,
            include_directory,
            search_type: binary_data.search_type.clone(),
        });
        Ok(Self {
            name: name.to_string(),
            data: canonicalized_data,
        })
    }
}
