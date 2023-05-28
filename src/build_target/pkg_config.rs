use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::build_target::include_directories::{IncludeDirectories, IncludeDirectory};
use crate::build_target::{
    LibraryType, PrintableLibrary, SHARED_LIBRARY_FILE_EXTENSION, STATIC_LIBRARY_FILE_EXTENSION,
};
use crate::flags::CXXFlags;
use crate::{find_program, EnvironmentVariable, FindProgramOptions, ModifyMode};

#[derive(Debug, Error)]
pub enum PkgConfigError {
    #[error("Could not find pkg-config executable")]
    CouldNotFindPkgConfig,
    #[error("Failed to run pkg-config")]
    FailedToRunPkgConfig(#[source] std::io::Error),
    #[error("pkg-config failed with the following error:\n{0}")]
    PkgConfigFailedWithError(String),
}

#[derive(PartialEq, Eq, Debug)]
pub struct PkgConfig {
    path: PathBuf,
    search_path_env: EnvironmentVariable,
}

impl PkgConfig {
    pub fn new() -> Result<Self, PkgConfigError> {
        let mut search_options = FindProgramOptions::new();
        search_options.with_path_env();

        if let Some(pkg_config) = find_program(&Path::new("pkg-config"), search_options) {
            Ok(Self {
                path: pkg_config,
                search_path_env: EnvironmentVariable::new("PKG_CONFIG_PATH"),
            })
        } else {
            Err(PkgConfigError::CouldNotFindPkgConfig)
        }
    }

    pub fn from_path(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            search_path_env: EnvironmentVariable::new("PKG_CONFIG_PATH"),
        }
    }

    pub fn add_search_path(&mut self, path: &Path) {
        self.search_path_env
            .set(&path.as_os_str(), ModifyMode::Append);
    }

    pub fn find_target(&self, target: &str) -> Option<PkgConfigTarget> {
        let cxx_flags = {
            let cflags = self.run(&[target, "--cflags-only-other"]).ok()?;
            let cflags = cflags.split_whitespace().collect::<Vec<&str>>();
            CXXFlags::new(&cflags)
        };
        let include_directories = {
            let args = [target, "--cflags-only-I"];
            let include_directories_str = self.run(&args).ok()?;
            let include_directories_str = include_directories_str
                .split_whitespace()
                .collect::<Vec<&str>>();
            let mut include_directories = IncludeDirectories::new();
            for dir_str in include_directories_str {
                let include_directory = IncludeDirectory::from_str(&dir_str);
                if let Some(include_directory) = include_directory {
                    include_directories.add(include_directory);
                }
            }
            include_directories
        };

        let library_names = {
            let libs_only_l = self.run(&[target, "--libs-only-l"]).ok()?;
            log::debug!("Output from 'pkg-config --libs-only-l': {}", libs_only_l);
            let split = libs_only_l.split(" ").collect::<Vec<&str>>();
            split
                .iter()
                .map(|s| s.replace("-l", ""))
                .collect::<Vec<String>>()
        };

        let search_paths = {
            let libs_only_capital_l = self.run(&[target, "--libs-only-L"]).ok()?;
            log::debug!(
                "Output from 'pkg-config --libs-only-L': {}",
                libs_only_capital_l
            );
            let split = libs_only_capital_l
                .split_whitespace()
                .collect::<Vec<&str>>();
            split
                .iter()
                .map(|s| PathBuf::from(s.replace("-L", "")))
                .collect::<Vec<PathBuf>>()
        };

        let mut library_paths = vec![];
        for lib_name in library_names {
            for search_path in &search_paths {
                if let Some(lib) = Library::find(&lib_name, &search_path) {
                    log::info!("Found library {} with pkg-config", lib.path().display());
                    library_paths.push(lib);
                } else {
                    log::error!(
                        "Failed to find library {} in {}",
                        lib_name,
                        search_path.display()
                    );
                }
            }
        }

        Some(PkgConfigTarget {
            target: target.to_string(),
            include_directories,
            cxx_flags,
            library_paths,
        })
    }

    fn run(&self, args: &[&str]) -> Result<String, PkgConfigError> {
        let output = Command::new(&self.path)
            .args(args)
            .output()
            .map_err(PkgConfigError::FailedToRunPkgConfig)?;
        let exit_status = output.status;
        if exit_status.success() {
            let stdout = output.stdout;
            let stdout = String::from_utf8(stdout).unwrap();
            Ok(stdout)
        } else {
            let stderr = output.stderr;
            let stderr = String::from_utf8(stderr).unwrap();
            Err(PkgConfigError::PkgConfigFailedWithError(stderr))
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PkgConfigTarget {
    pub target: String,
    pub include_directories: IncludeDirectories,
    pub cxx_flags: CXXFlags,
    pub library_paths: Vec<Library>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Library {
    printable: PrintableLibrary,
    dir: PathBuf,
}

impl Library {
    pub fn path(&self) -> PathBuf {
        self.dir.join(self.printable.name.clone())
    }

    pub fn find(library: &str, dir: &Path) -> Option<Self> {
        let possible_lib_names = PrintableLibrary::possible_lib_names(library);
        let mut search_options = FindProgramOptions::new();
        search_options.search_directory(dir);
        search_options.look_in_subdirectories(true);
        for lib_name in &possible_lib_names {
            match find_program(&Path::new(lib_name), search_options) {
                Some(found_lib) => {
                    let ty = match found_lib.extension().and_then(|e| e.to_str()) {
                        Some(STATIC_LIBRARY_FILE_EXTENSION) => LibraryType::Static,
                        Some(SHARED_LIBRARY_FILE_EXTENSION) => LibraryType::Dynamic,
                        _ => LibraryType::Static,
                    };
                    return Some(Self {
                        printable: PrintableLibrary {
                            name: lib_name.to_owned(),
                            ty,
                        },
                        dir: found_lib.parent().unwrap().to_path_buf(),
                    });
                }
                None => return None,
            };
        }
        None
    }
}
