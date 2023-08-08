use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

pub mod build_target;
pub mod cli;
pub mod compiler;
pub mod errors;
pub mod flags;
pub mod generator;
pub mod logger;
pub mod manifest;
pub mod output;
pub mod parser;
pub mod progress;
pub mod targets;
pub mod toolchain;
pub mod utility;

use once_cell::sync::OnceCell;

use crate::cli::command_line::ManifestDirectory;
use crate::cli::configurations::BuildType;
use crate::cli::BuildDirectory;
use crate::generator::GeneratorType;
use crate::parser::types::{Define, Language, Standard};

pub const YAMBS_MANIFEST_NAME: &str = "yambs.toml";
pub static YAMBS_BUILD_DIR_VAR: OnceCell<BuildDirectory> = OnceCell::new();
pub static YAMBS_MANIFEST_DIR: OnceCell<ManifestDirectory> = OnceCell::new();
pub static YAMBS_BUILD_TYPE: OnceCell<BuildType> = OnceCell::new();

#[derive(Clone, Debug)]
pub struct ProjectConfig {
    pub std: Standard,
    pub language: Language,
    pub build_directory: BuildDirectory,
    pub build_type: BuildType,
    pub generator_type: GeneratorType,
    pub defines: Vec<Define>,
}

pub enum ModifyMode {
    Set,
    Append,
}

#[derive(PartialEq, Eq, Debug)]
struct EnvironmentVariable {
    captured_val: Option<OsString>,
    key: String,
    val: Option<OsString>,
}

impl EnvironmentVariable {
    pub fn new(key: &str) -> Self {
        let captured_val = env::var_os(key);

        Self {
            captured_val: captured_val.clone(),
            key: key.to_string(),
            val: captured_val,
        }
    }

    pub fn set(&mut self, value: &OsStr, mode: ModifyMode) {
        match mode {
            ModifyMode::Set => {
                self.val = Some(value.to_os_string());
                env::set_var(&self.key, value);
            }
            ModifyMode::Append => {
                if let Some(ref mut val) = self.val {
                    #[cfg(target_family = "unix")]
                    {
                        val.push(format!(":{}", value.to_string_lossy()));
                    }
                    #[cfg(target_family = "windows")]
                    {
                        val.push(format!(";{}", value.to_string_lossy()));
                    }
                } else {
                    self.val = Some(value.to_os_string());
                }
                env::set_var(&self.key, self.val.as_ref().unwrap());
            }
        }
    }
}

// FIXME: Should have check for absolute path. Perhaps better check?
pub fn canonicalize_source(
    base_dir: &std::path::Path,
    path: &std::path::Path,
) -> std::io::Result<std::path::PathBuf> {
    if path == std::path::Path::new(".") {
        Ok(base_dir.to_path_buf())
    } else {
        base_dir.join(path).canonicalize()
    }
}

lazy_static::lazy_static! {
    static ref PATH_ENV_PATHS: Vec<PathBuf> = {
        let path_env = std::env::var_os("PATH").unwrap();
        std::env::split_paths(&path_env).collect::<Vec<PathBuf>>()
    };
}

#[derive(Debug, Clone)]
pub struct FindProgramOptions {
    search_directories: Vec<PathBuf>,
    look_in_subdirectories: bool,
}

impl FindProgramOptions {
    pub fn new() -> Self {
        Self {
            search_directories: vec![],
            look_in_subdirectories: false,
        }
    }

    pub fn with_path_env(&mut self) -> &mut Self {
        self.search_directories.extend_from_slice(&PATH_ENV_PATHS);
        self
    }

    pub fn search_directory(&mut self, dir: &Path) -> &mut Self {
        self.search_directories.push(dir.to_path_buf());
        self
    }

    pub fn look_in_subdirectories(&mut self, look_in_subdirectories: bool) -> &mut Self {
        self.look_in_subdirectories = look_in_subdirectories;
        self
    }
}

pub fn find_program(
    program: &Path,
    search_options: FindProgramOptions,
) -> Option<std::path::PathBuf>
where
{
    for dir in search_options.search_directories {
        log::debug!("Looking for {} in {}", program.display(), dir.display());
        let executable_path = dir.join(program);
        if executable_path.is_file() {
            log::debug!(
                "Found {} as {}",
                program.display(),
                executable_path.display()
            );
            return Some(executable_path);
        }
        if search_options.look_in_subdirectories {
            let read_dir = std::fs::read_dir(&dir).ok()?;
            let subdirectories = read_dir
                .into_iter()
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    if path.is_dir() {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect::<Vec<PathBuf>>();

            for subdirectory in subdirectories {
                log::debug!(
                    "Looking for {} in {}",
                    program.display(),
                    subdirectory.display()
                );
                let executable_path = subdirectory.join(program);
                if executable_path.is_file() {
                    log::debug!(
                        "Found {} as {}",
                        program.display(),
                        executable_path.display()
                    );
                    return Some(executable_path);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    lazy_static::lazy_static! {
        static ref ENV_LOCK_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());
    }
    pub struct EnvLock<'env> {
        _mutex_guard: std::sync::MutexGuard<'env, ()>,
        env_var: String,
        old_env_value: Option<String>,
    }

    impl<'env> EnvLock<'env> {
        pub fn lock(env_var: &str, new_value: &str) -> Self {
            let mutex_guard = ENV_LOCK_MUTEX.lock().unwrap();
            let old_env_value = std::env::var(env_var).ok();
            std::env::set_var(env_var, new_value);
            Self {
                _mutex_guard: mutex_guard,
                env_var: env_var.to_string(),
                old_env_value,
            }
        }
    }

    impl<'env> Drop for EnvLock<'env> {
        fn drop(&mut self) {
            if let Some(ref old_env_value) = self.old_env_value {
                std::env::set_var(&self.env_var, old_env_value);
            } else {
                std::env::remove_var(&self.env_var);
            }
        }
    }
}
