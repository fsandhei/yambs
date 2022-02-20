use crate::errors::CompilerError;

#[derive(Debug, Clone)]
pub struct Compiler {
    compiler_exe: std::path::PathBuf,
}

impl Compiler {
    pub fn new() -> Result<Self, CompilerError> {
        let compiler_exe = std::env::var_os("CXX")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| CompilerError::CXXEnvNotSet)?;
        Ok(Self { compiler_exe })
    }

    // fn evaluate(compiler_exe: &std::path::Path) -> Result<(), CompilerError> {

    // }
}

impl std::string::ToString for Compiler {
    fn to_string(&self) -> String {
        self.compiler_exe.display().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EnvLock {
        mutex: std::sync::Mutex<()>,
        old_env_value: Option<String>,
    }

    impl EnvLock {
        fn new() -> Self {
            Self {
                mutex: std::sync::Mutex::new(()),
                old_env_value: None,
            }
        }
        fn lock(&mut self, new_value: &str) {
            let _lock = self.mutex.lock().unwrap();
            self.old_env_value = std::env::var("CXX").ok();
            std::env::set_var("CXX", new_value);
        }
    }

    impl Drop for EnvLock {
        fn drop(&mut self) {
            if let Some(ref old_env_value) = self.old_env_value {
                std::env::set_var("CXX", old_env_value);
            }
        }
    }

    #[test]
    fn evaluate_compiler_fails_when_cxx_is_not_set() {
        let mut lock = EnvLock::new();
        lock.lock("");
        std::env::remove_var("CXX");
        let result = Compiler::new();
        assert_eq!(
            result.unwrap_err().to_string(),
            "Environment variable CXX was not set. Please set it to a valid C++ compiler."
        );
    }
}
