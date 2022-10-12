use crate::build_target::{target_registry::TargetRegistry, BuildTarget, TargetError, TargetNode};
use crate::cache;
use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::cli::command_line::BuildOpts;
use crate::errors::{CacheError, FsError};
use crate::generator::{Generator, GeneratorError};
use crate::parser;
use crate::parser::ParsedManifest;

mod filter;
mod make;
use make::Make;

enum BuildConfiguration {
    Debug,
    Release,
}

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
    targets: Vec<TargetNode>,
    generator: &'a mut dyn Generator,
    configuration: BuildConfiguration,
    make: Make,
    top_build_directory: BuildDirectory,
}

impl<'gen> BuildManager<'gen> {
    pub fn new(generator: &'gen mut dyn Generator) -> BuildManager {
        BuildManager {
            targets: Vec::new(),
            generator,
            configuration: BuildConfiguration::Release,
            make: Make::default(),
            top_build_directory: BuildDirectory::default(),
        }
    }

    pub fn targets(&self) -> &Vec<TargetNode> {
        self.targets.as_ref()
    }

    pub fn configure(&mut self, opts: &BuildOpts) -> Result<(), BuildManagerError> {
        self.add_make("-j", &opts.jobs.to_string());
        self.top_build_directory = opts.build_directory.to_owned();

        self.use_configuration(&opts.configuration)?;

        Ok(())
    }

    pub fn make(&self) -> &Make {
        &self.make
    }

    pub fn parse_and_register_dependencies(
        &mut self,
        cache: &cache::Cache,
        dep_registry: &mut TargetRegistry,
        manifest_path: &std::path::Path,
    ) -> Result<(), BuildManagerError> {
        let manifest = parser::parse(manifest_path).map_err(BuildManagerError::FailedToParse)?;

        log::debug!("Checking for cache of manifest.");
        if self
            .try_cached_manifest(cache, dep_registry, &manifest)
            .is_ok()
        {
            log::debug!("Cached manifest is up to date! Using it for this build.");
        } else {
            log::debug!("No cache found. Parsing manifest file.");
            let manifest =
                parser::parse(manifest_path).map_err(BuildManagerError::FailedToParse)?;

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
                let target = BuildTarget::create(
                    manifest_path.parent().unwrap(),
                    &build_target,
                    dep_registry,
                )
                .map_err(BuildManagerError::Target)?;
                self.targets.push(target);
            }
            cache
                .cache(&manifest)
                .map_err(BuildManagerError::FailedToCacheManifest)?;
        }
        Ok(())
    }

    pub fn generate_makefiles(&mut self) -> Result<(), BuildManagerError> {
        self.generator.generate(&self.targets)?;
        Ok(())
    }

    pub fn resolve_build_directory(&self, path: &std::path::Path) -> std::path::PathBuf {
        match self.configuration {
            BuildConfiguration::Debug => path.join("debug"),
            BuildConfiguration::Release => path.join("release"),
        }
    }

    fn try_cached_manifest(
        &mut self,
        cache: &cache::Cache,
        dep_registry: &mut TargetRegistry,
        manifest: &ParsedManifest,
    ) -> Result<(), BuildManagerError> {
        if let Some(cached_manifest) = cache.from_cache::<ParsedManifest>() {
            log::debug!("Found cached manifest. Checking if it is up to date.");
            if manifest.modification_time >= cached_manifest.modification_time {
                let cached_registry = TargetRegistry::from_cache(cache)
                    .ok_or_else(|| BuildManagerError::CannotFindCachedManifestOrRegistry)?;
                *dep_registry = cached_registry;
                dep_registry
                    .registry
                    .iter()
                    .for_each(|target| self.targets.push(target.clone()));
                return Ok(());
            }
            log::debug!("Cached manifest is older than latest manifest. Discarding cached.");
        }
        Err(BuildManagerError::CannotFindCachedManifestOrRegistry)
    }

    fn add_make(&mut self, flag: &str, value: &str) {
        self.make.with_flag(flag, value);
    }

    fn use_configuration(
        &mut self,
        configurations: &BuildConfigurations,
    ) -> Result<(), BuildManagerError> {
        for configuration in configurations {
            match configuration {
                Configuration::Debug => {
                    self.configuration = BuildConfiguration::Debug;
                    Ok::<(), BuildManagerError>(())
                }
                Configuration::Release => {
                    self.configuration = BuildConfiguration::Release;
                    Ok(())
                }
                Configuration::Sanitizer(_) => Ok(()),
                Configuration::CppVersion(_) => Ok(()),
            }?;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;
