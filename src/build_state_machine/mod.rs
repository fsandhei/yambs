use crate::build_target::{target_registry::TargetRegistry, BuildTarget, TargetError};
use crate::cli::command_line::BuildOpts;
use crate::cli::command_line::ConfigurationOpts;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::errors::{CacheError, FsError};
use crate::generator::{Generator, GeneratorError};
use crate::manifest;
use crate::parser;
use crate::YAMBS_MANIFEST_NAME;

mod filter;
mod make;
use make::Make;

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum BuildManagerError {
    #[error(transparent)]
    Target(#[from] TargetError),
    #[error("Cannot find cache of manifest or target registry.")]
    CannotFindCachedManifestOrRegistry,
    #[error("Failed to cache manifest.")]
    FailedToCacheManifest(#[source] CacheError),
    #[error(transparent)]
    Generator(#[from] GeneratorError),
    #[error("Failed to parse recipe file")]
    FailedToParse(#[source] parser::ParseTomlError),
    #[error("{0}: called in an unexpected way.")]
    UnexpectedCall(String),
    #[error(transparent)]
    Fs(#[from] FsError),
}

pub struct BuildManager<'a> {
    generator: &'a mut dyn Generator,
    configuration: configurations::BuildType,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'gen> BuildManager<'gen> {
    pub fn new(generator: &'gen mut dyn Generator) -> BuildManager {
        BuildManager {
            generator,
            configuration: configurations::BuildType::Debug,
            make: Make::default(),
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn configure(&mut self, opts: &BuildOpts) -> Result<(), BuildManagerError> {
        self.add_make_flag("-j", &opts.jobs.to_string());
        self.top_build_directory = opts.build_directory.to_owned();

        self.use_configuration(&opts.configuration)?;

        Ok(())
    }

    pub fn make(&self) -> &Make {
        &self.make
    }

    pub fn make_mut(&mut self) -> &mut Make {
        &mut self.make
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

    pub fn generate_makefiles(
        &mut self,
        registry: &TargetRegistry,
    ) -> Result<(), BuildManagerError> {
        self.generator.generate(registry)?;
        Ok(())
    }

    pub fn resolve_build_directory(&self, path: &std::path::Path) -> std::path::PathBuf {
        match self.configuration {
            configurations::BuildType::Debug => path.join("debug"),
            configurations::BuildType::Release => path.join("release"),
        }
    }

    fn add_make_flag(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
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
