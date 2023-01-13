use std::io::Write;

use regex::Regex;
use serde::{Deserialize, Serialize};
use textwrap::indent;

use crate::cache::Cacher;
use crate::errors;
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
    #[error("Failed to cache of compiler data")]
    FailedToCache(#[source] errors::CacheError),
    #[error(
        "Failed to retrieve compiler version from\n\
        \n\
        \t{0} --version"
    )]
    FailedToGetVersion(std::path::PathBuf, #[source] errors::FsError),
    #[error("Failed to find version pattern")]
    FailedToFindVersionPattern,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompilerInfo {
    pub compiler_type: Type,
    pub compiler_version: String,
}

impl CompilerInfo {
    pub fn new(compiler_exe: &std::path::PathBuf) -> Result<Self, CompilerError> {
        let compiler_type = Type::new(compiler_exe)?;
        let compiler_version = parse_version(compiler_exe)?;

        Ok(Self {
            compiler_type,
            compiler_version,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Compiler {
    pub compiler_exe: std::path::PathBuf,
    pub compiler_info: CompilerInfo,
}

impl Compiler {
    pub fn new() -> Result<Self, CompilerError> {
        let compiler_exe = std::env::var_os("CXX")
            .map(std::path::PathBuf::from)
            .ok_or(CompilerError::CXXEnvNotSet)?;
        let compiler_info = CompilerInfo::new(&compiler_exe)?;
        Ok(Self {
            compiler_exe,
            compiler_info,
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
    utility::shell::execute_get_stdout(compiler_exe, ["--version"])
        .map_err(|e| CompilerError::FailedToGetVersion(compiler_exe.to_path_buf(), e))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(non_camel_case_types)]
pub enum Type {
    Gcc,
    Clang,
}

impl Type {
    pub fn new(compiler_exe: &std::path::Path) -> Result<Self, CompilerError> {
        let version_output_raw = compiler_version_raw(compiler_exe)?;
        let gcc_pattern = Regex::new(r"GCC|gcc|g\+\+").expect("Could not compile regular expression");
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

impl Cacher for Compiler {
    const CACHE_FILE_NAME: &'static str = "compiler";
}

impl std::string::ToString for Compiler {
    fn to_string(&self) -> String {
        self.compiler_exe.display().to_string()
    }
}

