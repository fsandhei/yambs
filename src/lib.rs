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
) -> std::path::PathBuf {
    if path == std::path::Path::new(".") {
        base_dir.to_path_buf()
    } else {
        base_dir.join(path).canonicalize().unwrap()
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
    pub struct EnvLock {
        mutex: std::sync::Mutex<()>,
        env_var: Option<String>,
        old_env_value: Option<String>,
    }

    impl EnvLock {
        pub fn new() -> Self {
            Self {
                mutex: std::sync::Mutex::new(()),
                env_var: None,
                old_env_value: None,
            }
        }
        pub fn lock(&mut self, env_var: &str, new_value: &str) {
            let _lock = self.mutex.lock().unwrap();
            self.old_env_value = std::env::var(env_var).ok();
            self.env_var = Some(env_var.to_string());
            std::env::set_var(&env_var, new_value);
        }
    }

    impl Drop for EnvLock {
        fn drop(&mut self) {
            if let Some(ref env_var) = self.env_var {
                if let Some(ref old_env_value) = self.old_env_value {
                    std::env::set_var(env_var, old_env_value);
                }
            }
        }
    }
}
