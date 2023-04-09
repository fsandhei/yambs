use std::default::Default;
use std::io::Write;

use regex::Regex;
use serde::{Deserialize, Serialize};
use textwrap::indent;

use crate::errors;
use crate::toolchain::ToolchainCXXData;
use crate::utility;

#[derive(Debug, thiserror::Error)]
pub enum CompilerError {
    #[error("Environment variable CXX was not set. Please set it to a valid C++ compiler.")]
    CXXEnvNotSet,
    #[error("The compiler requested is an invalid compiler.")]
    InvalidCompiler,
    #[error(
        "\
        Error occured when doing a sample compilation."
    )]
    FailedToCompileSample(#[source] errors::FsError),
    #[error("Failed to create sample main.cpp for compiler assertion")]
    FailedToCreateSample(#[source] std::io::Error),
    #[error(
        "Failed to retrieve compiler version from\n\
        \n\
        \t{0} --version"
    )]
    FailedToGetVersion(std::path::PathBuf, #[source] errors::FsError),
    #[error("Failed to find version pattern")]
    FailedToFindVersionPattern,
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Linker {
    Ld,
    Gold,
    LLD,
    Inferred,
}

impl Linker {
    pub fn new() -> Self {
        Linker::Inferred
    }
}

impl Default for Linker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct CXXCompilerInfo {
    pub compiler_type: Type,
    pub compiler_version: String,
}

impl CXXCompilerInfo {
    pub fn new(compiler_exe: &std::path::PathBuf) -> Result<Self, CompilerError> {
        let compiler_type = Type::new(compiler_exe)?;
        let compiler_version = parse_version(compiler_exe)?;

        Ok(Self {
            compiler_type,
            compiler_version,
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub enum StdLibCXX {
    #[serde(rename = "libstdc++")]
    LibStdCXX,
    #[serde(rename = "libc++")]
    LibCXX,
}

impl Default for StdLibCXX {
    fn default() -> Self {
        Self::LibStdCXX
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct CXXCompiler {
    pub compiler_exe: std::path::PathBuf,
    pub compiler_info: CXXCompilerInfo,
    pub linker: Linker,
    #[serde(default)]
    pub stdlib: StdLibCXX,
}

impl CXXCompiler {
    pub fn new() -> Result<Self, CompilerError> {
        let compiler_exe = std::env::var_os("CXX")
            .map(std::path::PathBuf::from)
            .ok_or(CompilerError::CXXEnvNotSet)?;
        let compiler_info = CXXCompilerInfo::new(&compiler_exe)?;
        let linker = Linker::new();
        let stdlib = StdLibCXX::default();

        log::debug!("Registered CXX = {}", compiler_exe.display());
        Ok(Self {
            compiler_exe,
            compiler_info,
            linker,
            stdlib,
        })
    }

    pub fn from_toolchain_cxx_data(data: &ToolchainCXXData) -> Result<Self, CompilerError> {
        let compiler_exe = data.compiler.clone();
        let compiler_info = CXXCompilerInfo::new(&compiler_exe)?;
        let linker = data.linker.clone().unwrap_or_else(|| {
            log::debug!(
                "Linker not specified. Linker is inferred by the settings the compiler has."
            );
            Linker::Inferred
        });
        let stdlib = data.stdlib.clone();

        Ok(Self {
            compiler_exe,
            compiler_info,
            linker,
            stdlib,
        })
    }

    pub fn evaluate(&self, test_dir: &std::path::Path) -> Result<(), CompilerError> {
        let main_cpp =
            create_sample_cpp_main(test_dir).map_err(CompilerError::FailedToCreateSample)?;
        log::debug!("Running sample build with compiler specified in CXX");
        self.sample_compile(&main_cpp, test_dir)?;
        log::debug!("Running sample build with compiler specified in CXX... OK");
        Ok(())
    }

    fn create_sample_compile_args(&self, destination_dir: &std::path::Path) -> Vec<String> {
        match self.compiler_info.compiler_type {
            Type::Gcc | Type::Clang => vec![
                format!("-I{}", destination_dir.display()),
                "-o".to_string(),
                destination_dir.join("a.out").display().to_string(),
            ],
        }
    }

    fn sample_compile(
        &self,
        input_file: &std::path::Path,
        test_dir: &std::path::Path,
    ) -> Result<(), CompilerError> {
        let compiler_args = self.create_sample_compile_args(test_dir);
        let args =
            std::iter::once(input_file.display().to_string()).chain(compiler_args.into_iter());
        utility::shell::execute(&self.compiler_exe, args)
            .map_err(CompilerError::FailedToCompileSample)
    }
}

fn create_sample_cpp_main(test_dir: &std::path::Path) -> std::io::Result<std::path::PathBuf> {
    if !test_dir.is_dir() {
        std::fs::create_dir_all(test_dir)?;
    }
    let main_cpp_path = test_dir.join("main.cpp");
    let mut main_cpp = std::fs::File::create(&main_cpp_path)?;

    writeln!(&mut main_cpp, "int main()")?;
    writeln!(&mut main_cpp, "{{")?;
    writeln!(&mut main_cpp, "{}", indent("return 0;", "    "))?;
    writeln!(&mut main_cpp, "}}")?;
    Ok(main_cpp_path)
}

fn try_get_version(compiler_exe: &std::path::Path) -> Result<semver::Version, CompilerError> {
    let version_regex = Regex::new(r"[0-9]+\.[0-9]+\.[0-9]+").unwrap();

    let raw_version = compiler_version_raw(compiler_exe)?;

    if version_regex.is_match(&raw_version) {
        return Ok(version_regex
            .captures(&raw_version)
            .and_then(|captures| captures.get(0))
            .map(|captured_version| captured_version.as_str())
            .and_then(|version| semver::Version::parse(version).ok())
            .unwrap());
    }
    Err(CompilerError::FailedToFindVersionPattern)
}

fn parse_version(compiler_exe: &std::path::Path) -> Result<String, CompilerError> {
    try_get_version(compiler_exe).map(|version| version.to_string())
}

fn compiler_version_raw(compiler_exe: &std::path::Path) -> Result<String, CompilerError> {
    log::debug!(
        "Fetching compiler version with '{} --version'",
        compiler_exe.display()
    );
    utility::shell::execute_get_stdout(compiler_exe, ["--version"])
        .map_err(|e| CompilerError::FailedToGetVersion(compiler_exe.to_path_buf(), e))
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
#[allow(non_camel_case_types)]
pub enum Type {
    Gcc,
    Clang,
}

impl Type {
    pub fn new(compiler_exe: &std::path::Path) -> Result<Self, CompilerError> {
        let version_output_raw = compiler_version_raw(compiler_exe)?;
        let gcc_pattern =
            Regex::new(r"GCC|gcc|g\+\+").expect("Could not compile regular expression");
        let clang_pattern = Regex::new(r"clang").expect("Could not compile regular expression");
        if gcc_pattern.is_match(&version_output_raw) {
            return Ok(Type::Gcc);
        } else if clang_pattern.is_match(&version_output_raw) {
            return Ok(Type::Clang);
        } else {
            return Err(CompilerError::InvalidCompiler);
        }
    }
}

impl ToString for Type {
    fn to_string(&self) -> String {
        match self {
            Self::Gcc => "gcc".to_string(),
            Self::Clang => "clang".to_string(),
        }
    }
}

impl std::string::ToString for CXXCompiler {
    fn to_string(&self) -> String {
        self.compiler_exe.display().to_string()
    }
}
