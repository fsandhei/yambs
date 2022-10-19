//TODO: Skriv om testene for BuildManager slik at det stemmer med funksjonalitet.
use std::fs::File;
use std::io::Write;

use indoc;
use tempdir::TempDir;

use super::*;
use crate::build_target::{target_registry::TargetRegistry, TargetNode};
use crate::cli::configurations;
use crate::generator::{Generator, GeneratorError, Sanitizer};
use crate::parser;
use crate::{YAMBS_MANIFEST_DIR_ENV, YAMBS_MANIFEST_NAME};

pub struct GeneratorMock {
    _dep: Option<TargetNode>,
}

impl GeneratorMock {
    pub fn new() -> Self {
        Self { _dep: None }
    }
}

impl Sanitizer for GeneratorMock {
    fn set_sanitizer(&mut self, _: &str) {}
}

impl Generator for GeneratorMock {
    fn generate(&mut self, _registry: &TargetRegistry) -> Result<(), GeneratorError> {
        Ok(())
    }
}
struct EnvLock {
    mutex: std::sync::Mutex<()>,
    env_var: Option<String>,
    old_env_value: Option<String>,
}

impl EnvLock {
    fn new() -> Self {
        Self {
            mutex: std::sync::Mutex::new(()),
            env_var: None,
            old_env_value: None,
        }
    }
    fn lock(&mut self, env_var: &str, new_value: &str) {
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

fn dummy_manifest(dir_name: &str) -> (TempDir, std::path::PathBuf) {
    let dir: TempDir = TempDir::new(dir_name).unwrap();
    let test_file_path = dir.path().join(YAMBS_MANIFEST_NAME);
    let mut file = File::create(&test_file_path)
        .expect("dummy_manifest(): Something went wrong writing to file.");
    write!(
        file,
        indoc::indoc!(
            "\
         [executable.x]
            sources = [\"some_file.cpp\", \"some_other_file.cpp\"]
            main = \"main.cpp\""
        )
    )
    .expect("dummy_manifest(): Something went wrong writing to file.");

    std::fs::File::create(dir.path().join("some_file.cpp")).unwrap();
    std::fs::File::create(dir.path().join("some_other_file.cpp")).unwrap();
    std::fs::File::create(dir.path().join("main.cpp")).unwrap();
    (dir, test_file_path)
}

#[test]
fn parse_and_register_one_target() {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let mut dep_registry = TargetRegistry::new();
    let (dir, test_file_path) = dummy_manifest("example");
    let mut lock = EnvLock::new();
    let manifest = parser::parse(&test_file_path).unwrap();
    lock.lock(YAMBS_MANIFEST_DIR_ENV, &dir.path().display().to_string());

    builder
        .parse_and_register_dependencies(&mut dep_registry, &manifest)
        .unwrap();
}

// FIXME: Put tests back in when working on dependency support.

// #[test]
// fn read_mmk_files_two_files() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let mut dep_registry = TargetRegistry::new();
//     let (_dir, test_file_path) = dummy_manifest("example");
//     let (_dir_dep, test_file_dep_path) = dummy_manifest("example_dep");
//
//     assert!(builder
//         .parse_and_register_dependencies(&mut dep_registry, &test_file_path)
//         .is_ok());
//     Ok(())
// }
//
// #[test]
// fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let mut dep_registry = TargetRegistry::new();
//     let (_dir, test_file_path) = dummy_manifest("example");
//     let (_dir_dep, test_file_dep_path) = dummy_manifest("example_dep");
//     let (_second_dir_dep, test_file_second_dep_path) = dummy_manifest("example_dep");
//
//     assert!((builder.parse_and_register_dependencies(&mut dep_registry, &test_file_path)).is_ok());
//     Ok(())
// }
//
// #[test]
// fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let mut dep_registry = TargetRegistry::new();
//     let (_dir, test_file_path) = dummy_manifest("example");
//     let (_dir_dep, test_file_dep_path) = dummy_manifest("example_dep");
//     let (_second_dir_dep, test_file_second_dep_path) = dummy_manifest("example_dep_second");
//
//     assert!(builder
//         .parse_and_register_dependencies(&mut dep_registry, &test_file_path)
//         .is_ok());
//     Ok(())
// }
//
// #[test]
// fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let mut dep_registry = TargetRegistry::new();
//     let (_dir, test_file_path) = dummy_manifest("example");
//     let (_dir_dep, test_file_dep_path) = dummy_manifest("example_dep");
//     let (_second_dir_dep, test_file_second_dep_path) = dummy_manifest("example_dep_second");
//     let (_third_dir_dep, test_file_third_dep_path) = dummy_manifest("example_dep_third");
//
//     assert!(builder
//         .parse_and_register_dependencies(&mut dep_registry, &test_file_path)
//         .is_ok());
//     Ok(())
// }
//
// #[test]
// fn read_mmk_files_two_files_circulation() -> Result<(), BuildManagerError> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let mut dep_registry = TargetRegistry::new();
//     let (_dir, test_file_path) = dummy_manifest("example");
//     let (_dir_dep, test_file_dep_path) = dummy_manifest("example_dep");
//
//     let result = builder.parse_and_register_dependencies(&mut dep_registry, &test_file_path);
//
//     assert!(result.is_err());
//     Ok(())
// }

#[test]
fn resolve_build_directory_debug() {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    builder.configuration = configurations::BuildType::Debug;
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("debug");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

#[test]
fn resolve_build_directory_release() {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    builder.configuration = configurations::BuildType::Release;
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("release");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}
