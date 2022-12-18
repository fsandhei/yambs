//TODO: Skriv om testene for BuildManager slik at det stemmer med funksjonalitet.
use std::fs::File;
use std::io::Write;

use indoc;
use tempdir::TempDir;

use super::*;
use crate::build_target::target_registry::TargetRegistry;
use crate::cli::configurations;
use crate::parser;
use crate::YAMBS_MANIFEST_NAME;

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
            sources = [\"some_file.cpp\", \"some_other_file.cpp\", \"main.cpp\"]"
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
    let mut builder = BuildManager::new();
    let mut dep_registry = TargetRegistry::new();
    let (_dir, test_file_path) = dummy_manifest("example");
    let manifest = parser::parse(&test_file_path).unwrap();

    builder
        .parse_and_register_dependencies(&mut dep_registry, &manifest)
        .unwrap();
}

#[test]
fn resolve_build_directory_debug() {
    let mut builder = BuildManager::new();
    builder.configuration = configurations::BuildType::Debug;
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("debug");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

#[test]
fn resolve_build_directory_release() {
    let mut builder = BuildManager::new();
    builder.configuration = configurations::BuildType::Release;
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("release");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}
