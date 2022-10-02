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
pub mod output;
pub mod parser;
pub mod targets;
pub mod utility;

pub const YAMBS_MANIFEST_NAME: &str = "yambs.toml";
pub const YAMBS_MANIFEST_DIR_ENV: &str = "YAMBS_MANIFEST_DIR";
pub const YAMBS_BUILD_SYSTEM_EXECUTABLE_ENV: &str = "YAMBS_BUILD_SYSTEM_EXECUTABLE";

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
