use std::path::{Path, PathBuf};

use thiserror::Error;

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

    // pub fn find_target(&self, target: &str) -> Option<PkgConfigTarget> {
    //     let cxx_flags = {
    //         let cflags = self.run(&[target, "--cflags-only-other"]).ok()?;
    //         let cflags = cflags.split(" ").collect::<Vec<&str>>();
    //         CXXFlags::new(&cflags)
    //     };
    //     let include_directories = {
    //         let args = [target, "--cflags-only-I"];
    //         let include_directories_str = self.run(&args).ok()?;
    //         let include_directories_str = include_directories_str.split(" ").collect::<Vec<&str>>();
    //         let mut include_directories = IncludeDirectories::new();
    //         for dir_str in include_directories_str {
    //             let include_directory = IncludeDirectory::from_str(&dir_str);
    //             if let Some(include_directory) = include_directory {
    //                 include_directories.add(include_directory);
    //             }
    //         }
    //         include_directories
    //     };
    //     let ld_flags = {
    //         let ldflags = self.run(&[target, "--libs-only-other"]).ok()?;
    //         let ldflags = ldflags.split(" ").collect::<Vec<&str>>();
    //         LDFlags::new(&ldflags)
    //     };

    //     let library_names_without_extension = {
    //         let libs_only_l = self.run(&[target, "--libs-only-l"]).ok()?;
    //         let split = libs_only_l.split(" ").collect::<Vec<&str>>();
    //         split
    //             .iter()
    //             .map(|s| s.replace("-l", ""))
    //             .collect::<Vec<String>>()
    //     };

    //     let libraries = library_names_without_extension
    //         .iter()
    //         .map(|l| PathBuf::from(l).with_extension("a"))
    //         .collect::<Vec<PathBuf>>();

    //     let search_paths = {
    //         let libs_only_capital_l = self.run(&[target, "--libs-only-L"]).ok()?;
    //         let split = libs_only_capital_l.split(" ").collect::<Vec<&str>>();
    //         split
    //             .iter()
    //             .map(|s| PathBuf::from(s.replace("-L", "")))
    //             .collect::<Vec<PathBuf>>()
    //     };

    //     let mut library_paths = vec![];
    //     for library in libraries {
    //         for search_path in &search_paths {
    //             if let Some(lib) = Library::find(&library, &search_path) {
    //                 library_paths.push(lib);
    //             }
    //         }
    //     }

    //     Some(PkgConfigTarget {
    //         target: target.to_string(),
    //         include_directories,
    //         cxx_flags,
    //         ld_flags,
    //         library_paths,
    //     })
    // }

    // fn run(&self, args: &[&str]) -> Result<String, PkgConfigError> {
    //     let output = Command::new(&self.path)
    //         .args(args)
    //         .output()
    //         .map_err(PkgConfigError::FailedToRunPkgConfig)?;
    //     let exit_status = output.status;
    //     if exit_status.success() {
    //         let stdout = output.stdout;
    //         let stdout = String::from_utf8(stdout).unwrap();
    //         Ok(stdout)
    //     } else {
    //         let stderr = output.stderr;
    //         let stderr = String::from_utf8(stderr).unwrap();
    //         Err(PkgConfigError::PkgConfigFailedWithError(stderr))
    //     }
    // }
}

// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// pub struct PkgConfigTarget {
//     pub target: String,
//     pub include_directories: IncludeDirectories,
//     pub cxx_flags: CXXFlags,
//     pub ld_flags: LDFlags,
//     pub library_paths: Vec<Library>,
// }
//
// #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// pub struct Library {
//     pub ty: LibraryType,
//     pub path: PathBuf,
// }

// impl Library {
//     pub fn find(library: &Path, dir: &Path) -> Option<Self> {
//         let mut search_options = FindProgramOptions::new();
//         search_options.search_directory(dir);
//         if let Some(lib) = find_program(library, search_options) {
//             Some(Self {
//                 path: lib,
//                 ty: LibraryType::Static,
//             })
//         } else {
//             None
//         }
//     }
// }
