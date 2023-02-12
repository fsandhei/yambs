pub mod build_target;
pub mod cache;
pub mod cli;
pub mod compiler;
pub mod errors;
pub mod external;
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

pub const YAMBS_MANIFEST_NAME: &str = "yambs.toml";
pub static YAMBS_BUILD_DIR_VAR: OnceCell<BuildDirectory> = OnceCell::new();
pub static YAMBS_MANIFEST_DIR: OnceCell<ManifestDirectory> = OnceCell::new();
pub static YAMBS_BUILD_TYPE: OnceCell<BuildType> = OnceCell::new();

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
