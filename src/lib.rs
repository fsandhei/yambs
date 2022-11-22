pub mod build_state_machine;
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
pub mod utility;

use cli::command_line::CommandLine;
use cli::command_line::Subcommand;

pub const YAMBS_MANIFEST_NAME: &str = "yambs.toml";
pub const YAMBS_MANIFEST_DIR_ENV: &str = "YAMBS_MANIFEST_DIR";
pub const YAMBS_BUILD_SYSTEM_EXECUTABLE_ENV: &str = "YAMBS_BUILD_SYSTEM_EXECUTABLE";

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

pub struct YambsEnvironmentVariable(EnvironmentVariable);

impl YambsEnvironmentVariable {
    pub fn new(key: &str, value: &str) -> Self
where {
        std::env::set_var(key, value);
        Self {
            0: EnvironmentVariable(key.to_string()),
        }
    }
}

impl Drop for YambsEnvironmentVariable {
    fn drop(&mut self) {
        std::env::remove_var(&self.0 .0)
    }
}

pub struct EnvironmentVariable(String);

pub struct YambsEnvironmentVariables(std::vec::Vec<YambsEnvironmentVariable>);

impl YambsEnvironmentVariables {
    pub fn from_command_line(command_line: &CommandLine) -> Self {
        let mut env_vars = vec![];
        if let Some(ref subcommand) = command_line.subcommand {
            match subcommand {
                Subcommand::Build(ref build_opts) => {
                    env_vars.push(YambsEnvironmentVariable::new(
                        YAMBS_MANIFEST_DIR_ENV,
                        &build_opts.manifest_dir.as_path().display().to_string(),
                    ));
                }
                _ => (),
            }
        }
        Self(env_vars)
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
            std::env::set_var(&env_var, new_value);
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
