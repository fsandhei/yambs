use crate::cache;
use crate::targets;
use crate::YAMBS_MANIFEST_NAME;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub directory: std::path::PathBuf,
    pub modification_time: std::time::SystemTime,
}

impl Manifest {
    pub fn new(directory: &std::path::Path) -> Self {
        let metadata = std::fs::metadata(directory.join(YAMBS_MANIFEST_NAME)).expect(&format!(
            "Could not fetch metadata from {}",
            YAMBS_MANIFEST_NAME
        ));
        Self {
            directory: directory.to_path_buf(),
            modification_time: metadata
                .modified()
                .expect("Could not fetch last modified time of manifest"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ParsedManifest {
    #[serde(flatten)]
    pub manifest: Manifest,
    pub data: ManifestData,
}

impl cache::Cacher for ParsedManifest {
    const CACHE_FILE_NAME: &'static str = "manifest";
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ManifestData {
    pub targets: Vec<targets::Target>,
}

#[derive(Debug, serde::Deserialize, PartialEq, Eq)]
pub struct RawManifestData {
    #[serde(rename = "executable")]
    pub executables: Option<std::collections::HashMap<String, targets::RawExecutableData>>,
    #[serde(rename = "library")]
    pub libraries: Option<std::collections::HashMap<String, targets::RawLibraryData>>,
}

impl std::convert::From<RawManifestData> for ManifestData {
    fn from(contents: RawManifestData) -> Self {
        let mut targets = Vec::<targets::Target>::new();
        let mut executables = {
            if let Some(executables) = contents.executables {
                executables
                    .into_iter()
                    .map(|(name, data)| {
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| {
                                let dependency = targets::Dependency::new(&name, data);
                                match dependency.data {
                                    targets::DependencyData::Source {
                                        ref path,
                                        ref origin,
                                    } => {
                                        log::debug!(
                                            "Found dependency {} in path {} with origin {:?}",
                                            dependency.name,
                                            path.display(),
                                            origin
                                        );
                                    }
                                }
                                dependency
                            })
                            .collect::<Vec<targets::Dependency>>();
                        targets::Target::Executable(targets::Executable {
                            name,
                            main: crate::canonicalize_source(&data.common_raw.main),
                            sources: data
                                .common_raw
                                .sources
                                .iter()
                                .map(|source| crate::canonicalize_source(&source))
                                .collect::<Vec<std::path::PathBuf>>(),
                            dependencies,
                            compiler_flags: data.common_raw.compiler_flags,
                        })
                    })
                    .collect::<Vec<targets::Target>>()
            } else {
                Vec::new()
            }
        };
        let mut libraries = {
            if let Some(libraries) = contents.libraries {
                libraries
                    .into_iter()
                    .map(|(name, data)| {
                        let dependencies = data
                            .common_raw
                            .dependencies
                            .iter()
                            .map(|(name, data)| {
                                let dependency = targets::Dependency::new(&name, data);
                                match dependency.data {
                                    targets::DependencyData::Source {
                                        ref path,
                                        ref origin,
                                    } => {
                                        log::debug!(
                                            "Found dependency {} in path {} with origin {:?}",
                                            dependency.name,
                                            path.display(),
                                            origin
                                        );
                                    }
                                }
                                dependency
                            })
                            .collect::<Vec<targets::Dependency>>();
                        targets::Target::Library(targets::Library {
                            name,
                            main: crate::canonicalize_source(&data.common_raw.main),
                            sources: data
                                .common_raw
                                .sources
                                .iter()
                                .map(|source| crate::canonicalize_source(&source))
                                .collect::<Vec<std::path::PathBuf>>(),
                            dependencies,
                            compiler_flags: data.common_raw.compiler_flags,
                            lib_type: data.lib_type,
                        })
                    })
                    .collect::<Vec<targets::Target>>()
            } else {
                Vec::new()
            }
        };
        targets.append(&mut executables);
        targets.append(&mut libraries);
        Self { targets }
    }
}
