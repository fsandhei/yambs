use crate::errors::CompilerError;
use regex::Regex;

#[derive(Debug, Clone)]
pub struct Compiler {
    compiler_exe: std::path::PathBuf,
    compiler_type: Type,
}

impl Compiler {
    pub fn new() -> Result<Self, CompilerError> {
        let compiler_exe = std::env::var_os("CXX")
            .map(std::path::PathBuf::from)
            .ok_or_else(|| CompilerError::CXXEnvNotSet)?;
        let compiler_type = Compiler::evaluate_compiler_type(&compiler_exe)?;
        Ok(Self {
            compiler_exe,
            compiler_type,
        })
    }

    pub fn compiler_type(&self) -> &Type {
        &self.compiler_type
    }

    fn evaluate_compiler_type(compiler_exe: &std::path::Path) -> Result<Type, CompilerError> {
        if let Some(exe) = compiler_exe.file_name() {
            let gcc_pattern =
                Regex::new(r"g\+\+.*|gcc.*").expect("Could not compile regular expression");
            let clang_pattern =
                Regex::new(r"clang.*").expect("Could not compile regular expression");
            return exe
                .to_str()
                .and_then(|exe_str| {
                    if gcc_pattern.is_match(&exe_str) {
                        Some(Type::Gcc)
                    } else if clang_pattern.is_match(&exe_str) {
                        Some(Type::Clang)
                    } else {
                        None
                    }
                })
                .ok_or_else(|| CompilerError::InvalidCompiler);
        }
        Err(CompilerError::CXXEnvNotSet)
    }
    // fn evaluate(compiler_exe: &std::path::Path) -> Result<(), CompilerError> {

    // }
}

#[derive(Debug, Clone)]
#[allow(non_camel_case_types)]
pub enum Type {
    Gcc,
    Clang,
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
        let result = Compiler::new();
        assert_eq!(
            result.unwrap_err().to_string(),
            "Environment variable CXX was not set. Please set it to a valid C++ compiler."
        );
    }

    #[test]
    fn evaluate_compiler_type_gcc() {
        let mut lock = EnvLock::new();
        {
            lock.lock("gcc-9");
            let compiler = Compiler::new().unwrap();
            assert!(matches!(compiler.compiler_type(), &Type::Gcc));
        }
        {
            lock.lock("gcc-11");
            let compiler = Compiler::new().unwrap();
            assert!(matches!(compiler.compiler_type(), &Type::Gcc));
        }
    }

    #[test]
    fn evaluate_compiler_type_clang() {
        let mut lock = EnvLock::new();
        {
            lock.lock("clang-9");
            let compiler = Compiler::new().unwrap();
            assert!(matches!(compiler.compiler_type(), &Type::Clang));
        }
        {
            lock.lock("clang-11");
            let compiler = Compiler::new().unwrap();
            assert!(matches!(compiler.compiler_type(), &Type::Clang));
        }
    }
}
