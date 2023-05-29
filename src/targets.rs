use std::path::Path;

use crate::flags::CompilerFlags;
use crate::parser::types;
use crate::parser::types::{PkgConfigData, PkgConfigSearchDir};

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
            types::DependencyData::HeaderOnly(ref header_only_data) => {
                log::debug!(
                    "Found header only dependency {} with include directory \"{}\"",
                    name,
                    header_only_data.include_directory.display()
                );
                dependency = Dependency::from_header_only(name, header_only_data, manifest_dir);
            }
            types::DependencyData::PkgConfig(ref pkgconfig_data) => {
                log::debug!("Found pkgconfig dependency {}", name);
                dependency = Dependency::from_pkgconfig_data(name, pkgconfig_data, manifest_dir);
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

    fn from_header_only(
        name: &str,
        header_only_data: &types::HeaderOnlyData,
        manifest_dir: &std::path::Path,
    ) -> Result<Self, DependencyError> {
        let include_directory =
            crate::canonicalize_source(manifest_dir, &header_only_data.include_directory).map_err(
                |err| {
                    DependencyError::FailedToCanonicalizePath(
                        header_only_data.include_directory.clone(),
                        err,
                    )
                },
            )?;
        let canonicalized_data =
            types::DependencyData::HeaderOnly(types::HeaderOnlyData { include_directory });
        Ok(Self {
            name: name.to_string(),
            data: canonicalized_data,
        })
    }

    fn from_pkgconfig_data(
        name: &str,
        pkgconfig_data: &types::PkgConfigData,
        manifest_dir: &Path,
    ) -> Result<Self, DependencyError> {
        let debug_search_dir = crate::canonicalize_source(
            manifest_dir,
            &pkgconfig_data.debug.search_dir,
        )
        .map_err(|err| {
            DependencyError::FailedToCanonicalizePath(pkgconfig_data.debug.search_dir.clone(), err)
        })?;
        let release_search_dir =
            crate::canonicalize_source(manifest_dir, &pkgconfig_data.release.search_dir).map_err(
                |err| {
                    DependencyError::FailedToCanonicalizePath(
                        pkgconfig_data.release.search_dir.clone(),
                        err,
                    )
                },
            )?;

        Ok(Self {
            name: name.to_string(),
            data: types::DependencyData::PkgConfig(PkgConfigData {
                debug: PkgConfigSearchDir {
                    search_dir: debug_search_dir,
                },
                release: PkgConfigSearchDir {
                    search_dir: release_search_dir,
                },
            }),
        })
    }
}
