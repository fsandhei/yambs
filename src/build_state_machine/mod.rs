use crate::build_target::{target_registry::TargetRegistry, BuildTarget, TargetError};
use crate::cli::command_line::BuildOpts;
use crate::cli::command_line::ConfigurationOpts;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::errors::{CacheError, FsError};
use crate::generator;
use crate::manifest;
use crate::parser;
use crate::YAMBS_MANIFEST_NAME;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BuildManagerError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error("Cannot find cache of manifest or target registry.")]
    CannotFindCachedManifestOrRegistry,
    #[error("Failed to cache manifest.")]
    FailedToCacheManifest(#[source] CacheError),
    #[error("Failed to parse recipe file")]
    FailedToParse(#[source] parser::ParseTomlError),
    #[error("{0}: called in an unexpected way.")]
    UnexpectedCall(String),
    #[error(transparent)]
    Fs(#[from] FsError),
}

pub struct BuildManager {
    configuration: configurations::BuildType,
    top_build_directory: BuildDirectory,
}

impl BuildManager {
    pub fn new() -> BuildManager {
        BuildManager {
            configuration: configurations::BuildType::Debug,
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn build(
        &self,
        args: Vec<String>,
    ) -> Result<generator::makefile::make::Make, BuildManagerError> {
        let mut make = generator::makefile::make::Make::new()?;
        let makefile_directory = self.resolve_build_directory(self.top_build_directory.as_path());
        make.spawn_with_args(&makefile_directory, args)?;

        Ok(make)
    }

    pub fn configure(&mut self, opts: &BuildOpts) -> Result<(), BuildManagerError> {
        self.top_build_directory = opts.build_directory.to_owned();

        self.use_configuration(&opts.configuration)?;

        Ok(())
    }

    pub fn parse_and_register_dependencies(
        &mut self,
        dep_registry: &mut TargetRegistry,
        manifest: &manifest::ParsedManifest,
    ) -> Result<(), BuildManagerError> {
        let manifest_path = manifest.manifest.directory.join(YAMBS_MANIFEST_NAME);
        for build_target in &manifest.data.targets {
            if let Some(lib) = build_target.library() {
                log::debug!(
                    "Creating build target for library {} in manifest {}",
                    lib.name,
                    manifest_path.display()
                );
            }
            if let Some(exe) = build_target.executable() {
                log::debug!(
                    "Creating build target for executable {} in manifest {}",
                    exe.name,
                    manifest_path.display()
                );
            }
            BuildTarget::target_node_from_source(
                &manifest.manifest.directory,
                build_target,
                dep_registry,
            )
            .map_err(BuildManagerError::Target)?;
        }
        Ok(())
    }

    pub fn resolve_build_directory(&self, path: &std::path::Path) -> std::path::PathBuf {
        match self.configuration {
            configurations::BuildType::Debug => path.join("debug"),
            configurations::BuildType::Release => path.join("release"),
        }
    }

    fn use_configuration(
        &mut self,
        configurations: &ConfigurationOpts,
    ) -> Result<(), BuildManagerError> {
        self.configuration = configurations.build_type.clone();
        Ok(())
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
