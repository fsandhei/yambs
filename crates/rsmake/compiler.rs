use crate::errors::CompilerError;

pub(crate) struct Compiler {
    compiler_exe: std::path::PathBuf,
}

impl Compiler {
    pub(crate) fn new() -> Result<Self, CompilerError> {
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
