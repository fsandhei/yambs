use std::path::PathBuf;

use crate::parser::types;
use crate::targets;
use crate::YAMBS_MANIFEST_NAME;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Manifest {
    pub directory: std::path::PathBuf,
    pub modification_time: std::time::SystemTime,
}

impl Manifest {
    pub fn new(directory: &std::path::Path) -> Self {
        let metadata = std::fs::metadata(directory.join(YAMBS_MANIFEST_NAME))
            .unwrap_or_else(|_| panic!("Could not fetch metadata from {}", YAMBS_MANIFEST_NAME));
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

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct ManifestData {
    pub project_configuration: Option<types::ProjectConfiguration>,
    pub targets: Vec<targets::Target>,
}

#[derive(thiserror::Error, Debug)]
pub enum ParseManifestError {
    #[error("Failed to parse dependency")]
    FailedToParseDependency(#[source] targets::DependencyError),
    #[error("Failed to canonicalize {1:?}")]
    FailedToCanonicalizePath(#[source] std::io::Error, PathBuf),
}

impl ManifestData {
    pub fn from_raw(
        contents: types::RawManifestData,
        manifest_dir: &std::path::Path,
    ) -> Result<Self, ParseManifestError> {
        let mut targets = Vec::<targets::Target>::new();
        let mut executables = {
            let mut target_executables = Vec::new();
            if let Some(executables) = contents.executables {
                for executable in executables {
                    let name = executable.0;
                    let data = executable.1;

                    let dependencies = data.common_raw.dependencies;
                    let mut parsed_dependencies = Vec::new();
                    for dependency in dependencies {
                        let dep_name = dependency.0;
                        let dep_data = dependency.1;
                        let parsed_dependency =
                            targets::Dependency::new(&dep_name, &dep_data, manifest_dir)
                                .map_err(ParseManifestError::FailedToParseDependency)?;
                        parsed_dependencies.push(parsed_dependency);
                    }
                    let canonicalized_sources = {
                        let mut canonicalized_sources = Vec::new();
                        let sources = data.common_raw.sources;
                        for source in sources {
                            let canonicalized_source =
                                crate::canonicalize_source(manifest_dir, &source).map_err(|e| {
                                    ParseManifestError::FailedToCanonicalizePath(e, source)
                                })?;
                            canonicalized_sources.push(canonicalized_source);
                        }
                        Ok(canonicalized_sources)
                    }?;
                    let target_executable = targets::Target::Executable(targets::Executable {
                        name,
                        sources: canonicalized_sources,
                        dependencies: parsed_dependencies,
                        compiler_flags: data.common_raw.compiler_flags,
                        defines: data.common_raw.defines,
                        additional_include_directories: data.common_raw.include_directories,
                        additional_system_include_directories: data
                            .common_raw
                            .system_include_directories,
                    });
                    target_executables.push(target_executable);
                }
            }
            Ok(target_executables)
        }?;
        let mut libraries = {
            let mut target_libraries = Vec::new();
            if let Some(libraries) = contents.libraries {
                for library in libraries {
                    let name = library.0;
                    let data = library.1;

                    let dependencies = data.common_raw.dependencies;
                    let mut parsed_dependencies = Vec::new();
                    for dependency in dependencies {
                        let dep_name = dependency.0;
                        let dep_data = dependency.1;
                        let parsed_dependency =
                            targets::Dependency::new(&dep_name, &dep_data, manifest_dir)
                                .map_err(ParseManifestError::FailedToParseDependency)?;
                        parsed_dependencies.push(parsed_dependency);
                    }
                    let canonicalized_sources = {
                        let mut canonicalized_sources = Vec::new();
                        let sources = data.common_raw.sources;
                        for source in sources {
                            let canonicalized_source =
                                crate::canonicalize_source(manifest_dir, &source).map_err(|e| {
                                    ParseManifestError::FailedToCanonicalizePath(e, source)
                                })?;
                            canonicalized_sources.push(canonicalized_source);
                        }
                        Ok(canonicalized_sources)
                    }?;
                    let target_library = targets::Target::Library(targets::Library {
                        name,
                        sources: canonicalized_sources,
                        dependencies: parsed_dependencies,
                        compiler_flags: data.common_raw.compiler_flags,
                        lib_type: data.lib_type,
                        defines: data.common_raw.defines,
                        additional_include_directories: data.common_raw.include_directories,
                        additional_system_include_directories: data
                            .common_raw
                            .system_include_directories,
                    });
                    target_libraries.push(target_library);
                }
            }
            Ok(target_libraries)
        }?;
        targets.append(&mut executables);
        targets.append(&mut libraries);
        let project_configuration = contents.project_configuration;
        Ok(Self {
            project_configuration,
            targets,
        })
    }
}
