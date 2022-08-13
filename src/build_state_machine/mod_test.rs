//TODO: Skriv om testene for BuildManager slik at det stemmer med funksjonalitet.
use std::fs::File;
use std::io::Write;
use tempdir::TempDir;

use super::*;
use crate::dependency::{DependencyAccessor, DependencyNode};
use crate::errors::{DependencyError, GeneratorError};
use crate::generator::{Generator, GeneratorExecutor, RuntimeSettings, Sanitizer};
use crate::mmk_parser::{Keyword, Mmk};
use crate::utility;

pub struct GeneratorMock {
    dep: Option<DependencyNode>,
}

impl GeneratorMock {
    pub fn new() -> Self {
        Self { dep: None }
    }
}

impl GeneratorExecutor for GeneratorMock {
    fn generate_makefiles(&mut self, _dependency: &DependencyNode) -> Result<(), GeneratorError> {
        Ok(())
    }
}

impl Sanitizer for GeneratorMock {
    fn set_sanitizer(&mut self, _: &str) {}
}

impl Generator for GeneratorMock {
    fn generate_makefile(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_rule_executable(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_rule_package(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_appending_flags(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }
}

impl RuntimeSettings for GeneratorMock {
    fn debug(&mut self) {}

    fn release(&mut self) {}

    fn use_std(&mut self, _version: &str) -> Result<(), GeneratorError> {
        Ok(())
    }
}

impl DependencyAccessor for GeneratorMock {
    fn set_dependency(&mut self, _: &DependencyNode) {}
    fn get_dependency(&self) -> Result<&DependencyNode, DependencyError> {
        if let Some(dependency) = &self.dep {
            return Ok(dependency);
        }
        Err(DependencyError::NotSet)
    }
}

fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
    let dir: TempDir = TempDir::new(dir_name).unwrap();
    let source_dir = dir.path().join("source");
    utility::create_dir(&source_dir).unwrap();
    let test_file_path = source_dir.join("lib.mmk");
    let mut file = File::create(&test_file_path)
        .expect("make_mmk_file(): Something went wrong writing to file.");
    write!(
        file,
        "MMK_SOURCES:
            some_file.cpp
            some_other_file.cpp
        
        MMK_HEADERS:
            some_file.h
            some_other_file.h
        
            "
    )
    .expect("make_mmk_file(): Something went wrong writing to file.");

    let mut mmk_data = Mmk::new(&test_file_path);
    mmk_data.data_mut().insert(
        String::from("MMK_SOURCES"),
        vec![
            Keyword::from("some_file.cpp"),
            Keyword::from("some_other_file.cpp"),
        ],
    );

    mmk_data.data_mut().insert(
        String::from("MMK_HEADERS"),
        vec![
            Keyword::from("some_file.h"),
            Keyword::from("some_other_file.h"),
        ],
    );

    (dir, test_file_path, file, mmk_data)
}

#[test]
fn read_mmk_files_one_file() {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");

    write!(
        file,
        "MMK_EXECUTABLE:
                x
            "
    )
    .unwrap();
    assert!(builder
        .parse_and_register_dependencies(&test_file_path)
        .is_ok());
}

#[test]
fn read_mmk_files_two_files() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, _file_dep, _) = make_mmk_file("example_dep");

    write!(
        file,
        "\
            MMK_REQUIRE:
                {}
        \n
        
        MMK_EXECUTABLE:
            x
        ",
        &test_file_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    assert!(builder
        .parse_and_register_dependencies(&test_file_path)
        .is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, _file_dep, _) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep");

    write!(
        file,
        "\
        MMK_REQUIRE:
            {}
            {}
        
        \n
        MMK_EXECUTABLE:
            x",
        &test_file_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        &test_file_second_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    assert!((builder.parse_and_register_dependencies(&test_file_path)).is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");

    write!(
        file,
        "\
        MMK_REQUIRE:
            {}
        \n
        MMK_EXECUTABLE:
            x",
        &test_file_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    write!(
        file_dep,
        "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_second_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    assert!(builder
        .parse_and_register_dependencies(&test_file_path)
        .is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");
    let (_third_dir_dep, test_file_third_dep_path, _file_third_file_dep, _) =
        make_mmk_file("example_dep_third");

    write!(
        file,
        "\
        MMK_REQUIRE:
            {}
            {}
        \n
        MMK_EXECUTABLE:
            x",
        &test_file_third_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string(),
        &test_file_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    write!(
        file_dep,
        "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_second_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    assert!(builder
        .parse_and_register_dependencies(&test_file_path)
        .is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_two_files_circulation() -> Result<(), BuildManagerError> {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _) = make_mmk_file("example_dep");

    write!(
        file,
        "\
            MMK_REQUIRE:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
        &test_file_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )
    .unwrap();

    write!(
        file_dep,
        "\
            MMK_REQUIRE:
                {}
        \n",
        &test_file_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )
    .unwrap();

    let result = builder.parse_and_register_dependencies(&test_file_path);

    assert!(result.is_err());
    Ok(())
}

// #[test]
// fn add_generator() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let (_dir, test_file_path, mut file, _) = make_mmk_file("example");

//     write!(
//         file,
//         "MMK_EXECUTABLE:
//             x
//         ")?;
//     assert!(builder.parse_and_register_dependencies(&test_file_path).is_ok());

//     builder.add_generator();
//     assert!(builder.generator.is_some());
//     Ok(())
// }

#[test]
fn resolve_build_directory_debug() {
    let mut generator = GeneratorMock::new();
    let mut builder = BuildManager::new(&mut generator);
    builder.debug();
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("debug");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

#[test]
fn resolve_build_directory_release() {
    let mut generator = GeneratorMock::new();
    let builder = BuildManager::new(&mut generator);
    let path = std::path::PathBuf::from("some/path");
    let expected = path.join("release");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

// #[test]
// fn construct_build_message_executable() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = BuildManager::new(&mut generator);
//     let (_dir, test_file_path, mut file, _) = make_mmk_file("example");

//     write!(
//         file,
//         "MMK_EXECUTABLE:
//                 x"
//     )?;
//     assert!(builder.parse_and_register_dependencies(&test_file_path).is_ok());
//     let green_text = "Building".green();
//     let expected_message = format!("{} executable \"x\"", green_text);
//     let borrowed_dependency = builder.top_dependency.unwrap();
//     assert_eq!(
//         BuildManager::construct_build_message(&borrowed_dependency),
//         expected_message
//     );
//     Ok(())
// }
