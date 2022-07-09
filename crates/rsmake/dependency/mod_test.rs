use super::*;
use mmk_parser::{Keyword, Mmk};
use pretty_assertions::assert_eq;
use std::fs::File;
use std::io::Write;
use tempdir::TempDir;
use utility;

use crate::dependency::include_directories::IncludeDirectories;

fn expected_associated_files(root_path: &std::path::Path) -> AssociatedFiles {
    let mut associated_files = AssociatedFiles::new();
    associated_files.push(SourceFile::new(&root_path.join("some_file.cpp")).unwrap());
    associated_files.push(SourceFile::new(&root_path.join("some_other_file.cpp")).unwrap());
    associated_files.push(SourceFile::new(&root_path.join("some_file.h")).unwrap());
    associated_files.push(SourceFile::new(&root_path.join("some_other_file.h")).unwrap());
    associated_files
}

fn make_mmk_file(dir_name: &str) -> (TempDir, std::path::PathBuf, File, Mmk) {
    let dir: TempDir = TempDir::new(&dir_name).unwrap();
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
        \n
        MMK_HEADERS:
            some_file.h
            some_other_file.h
        
        \n"
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

fn fixture_simple_dependency() -> DependencyNode {
    let (_dir, lib_file_path, _file, mmk_file) = make_mmk_file("example");
    let mut dep_registry = DependencyRegistry::new();
    Dependency::from_path(&lib_file_path, &mut dep_registry, &mmk_file).unwrap()
}

#[test]
fn test_get_name_executable() {
    let (_dir, lib_file_path, _file, mut mmk_file) = make_mmk_file("example");
    mmk_file
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);
    let mut dep_registry = DependencyRegistry::new();
    let dependency = Dependency::from_path(&lib_file_path, &mut dep_registry, &mmk_file).unwrap();
    let actual = dependency.dependency().ref_dep.get_name().unwrap();
    let expected = "x";
    assert_eq!(expected, actual);
}

#[test]
fn test_get_name_library_label() {
    let (_dir, lib_file_path, _file, mut mmk_file) = make_mmk_file("example");
    let mut dep_registry = DependencyRegistry::new();
    mmk_file.data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("MYLIB")],
    );
    let dependency = Dependency::from_path(&lib_file_path, &mut dep_registry, &mmk_file).unwrap();
    dependency.dependency().ref_dep.add_library_name(&mmk_file);
    let actual = dependency.dependency().ref_dep.get_name().unwrap();
    let expected = "MYLIB";
    assert_eq!(expected, actual);
}

#[test]
fn test_is_in_process_true() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::InProcess);
    assert!(ref_dep.is_in_process());
}

#[test]
fn test_is_in_process_false() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::Registered);
    assert!(!ref_dep.is_in_process());
}

#[test]
fn test_is_makefile_made_true() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::MakefileMade);
    assert!(ref_dep.is_makefile_made());
}

#[test]
fn test_is_makefile_made_false() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::NotInProcess);
    assert!(!ref_dep.is_makefile_made());
}

#[test]
fn test_is_building_true() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::Building);
    assert!(ref_dep.is_building());
}

#[test]
fn test_is_building_false() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::BuildComplete);
    assert!(!ref_dep.is_building());
}

#[test]
fn test_is_build_completed_true() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::BuildComplete);
    assert!(ref_dep.is_build_completed());
}

#[test]
fn test_is_build_completed_false() {
    let dependency = fixture_simple_dependency();
    let mut ref_dep = dependency.dependency_mut().ref_dep;
    ref_dep.change_state(DependencyState::NotInProcess);
    assert!(!ref_dep.is_build_completed());
}

#[test]
fn read_mmk_files_one_file() -> std::io::Result<()> {
    let (_dir, lib_file_path, mut file, mut mmk_file) = make_mmk_file("example");

    write!(
        file,
        "MMK_EXECUTABLE:
                x"
    )?;

    mmk_file
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);
    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::from_path(&lib_file_path, &mut dep_registry, &mmk_file).unwrap();
    let ref_dep = top_dependency.dependency().ref_dep;
    assert!(ref_dep.is_executable());
    assert!(ref_dep.requires().is_empty());
    Ok(())
}

#[test]
fn read_mmk_files_two_files() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut mmk_file_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, _file_dep, _) = make_mmk_file("example_dep");

    write!(
        file,
        "\
            MMK_REQUIRE:
                {}
        \n
        
        MMK_EXECUTABLE:
            x",
        &test_file_dep_path.parent().unwrap().display()
    )?;

    mmk_file_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            &test_file_dep_path.parent().unwrap().display().to_string(),
        )],
    );
    mmk_file_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1).unwrap();

    let expected_lib_name_dep = utility::get_head_directory(&dir_dep.path())
        .display()
        .to_string();
    assert_eq!(
        top_dependency,
        DependencyNode::new(Dependency {
            path: test_file_path,
            requires: vec![DependencyNode::new(Dependency {
                path: test_file_dep_path,
                requires: Vec::new(),
                state: DependencyState::Registered,
                associated_files: expected_associated_files(&dir_dep.path().join("source")),
                dependency_type: DependencyType::Library(expected_lib_name_dep),
                include_directories: None,
                additional_flags: std::collections::HashMap::new(),
            })],
            state: DependencyState::Registered,
            associated_files: expected_associated_files(&dir.path().join("source")),
            dependency_type: DependencyType::Executable(String::from("x")),
            include_directories: IncludeDirectories::from_mmk(&mmk_file_1),
            additional_flags: std::collections::HashMap::new(),
        })
    );
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut mmk_file_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, _file_dep, _) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
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
        test_file_dep_path.parent().unwrap().display(),
        test_file_second_dep_path.parent().unwrap().display()
    )?;

    mmk_file_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![
            Keyword::from(test_file_dep_path.parent().unwrap().to_str().unwrap()),
            Keyword::from(
                test_file_second_dep_path
                    .parent()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            ),
        ],
    );
    mmk_file_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1).unwrap();

    let expected_lib_name_dep = utility::get_head_directory(&dir_dep.path())
        .display()
        .to_string();
    let expected_lib_name_second_dep = utility::get_head_directory(&second_dir_dep.path())
        .display()
        .to_string();

    assert_eq!(
        top_dependency,
        DependencyNode::new(Dependency {
            path: test_file_path,
            requires: vec![
                DependencyNode::new(Dependency {
                    path: test_file_dep_path,
                    requires: Vec::new(),
                    state: DependencyState::Registered,
                    associated_files: expected_associated_files(&dir_dep.path().join("source")),
                    dependency_type: DependencyType::Library(expected_lib_name_dep),
                    include_directories: None,
                    additional_flags: std::collections::HashMap::new(),
                }),
                DependencyNode::new(Dependency {
                    path: test_file_second_dep_path,
                    requires: Vec::new(),
                    state: DependencyState::Registered,
                    associated_files: expected_associated_files(
                        &second_dir_dep.path().join("source")
                    ),
                    dependency_type: DependencyType::Library(expected_lib_name_second_dep),
                    include_directories: None,
                    additional_flags: std::collections::HashMap::new(),
                })
            ],
            state: DependencyState::Registered,
            associated_files: expected_associated_files(&dir.path().join("source")),
            include_directories: IncludeDirectories::from_mmk(&mmk_file_1),
            dependency_type: DependencyType::Executable("x".to_string()),
            additional_flags: std::collections::HashMap::new(),
        })
    );
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut mmk_file_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut mmk_file_2) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");

    write!(
        file,
        "\
        MMK_REQUIRE:
            {}
        \n
        MMK_EXECUTABLE:
            x",
        test_file_dep_path.parent().unwrap().to_str().unwrap()
    )?;

    write!(
        file_dep,
        "\
        MMK_REQUIRE:
            {}
        \n
        ",
        test_file_second_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
    )?;

    mmk_file_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_dep_path.parent().unwrap().to_str().unwrap(),
        )],
    );
    mmk_file_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    mmk_file_2.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_second_dep_path
                .parent()
                .unwrap()
                .to_str()
                .unwrap(),
        )],
    );

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1).unwrap();

    let expected_lib_name_dep = utility::get_head_directory(&dir_dep.path())
        .display()
        .to_string();
    let expected_lib_name_second_dep = utility::get_head_directory(&second_dir_dep.path())
        .display()
        .to_string();

    assert_eq!(
        top_dependency,
        DependencyNode::new(Dependency {
            path: test_file_path,
            requires: vec![DependencyNode::new(Dependency {
                path: test_file_dep_path,
                requires: vec![DependencyNode::new(Dependency {
                    path: test_file_second_dep_path,
                    requires: vec![],
                    state: DependencyState::Registered,
                    associated_files: expected_associated_files(
                        &second_dir_dep.path().join("source")
                    ),
                    dependency_type: DependencyType::Library(expected_lib_name_second_dep),
                    include_directories: None,
                    additional_flags: std::collections::HashMap::new(),
                })],
                state: DependencyState::Registered,
                associated_files: expected_associated_files(&dir_dep.path().join("source")),
                dependency_type: DependencyType::Library(expected_lib_name_dep),
                include_directories: IncludeDirectories::from_mmk(&mmk_file_2),
                additional_flags: std::collections::HashMap::new(),
            })],
            state: DependencyState::Registered,
            associated_files: expected_associated_files(&dir.path().join("source")),
            dependency_type: DependencyType::Executable("x".to_string()),
            include_directories: IncludeDirectories::from_mmk(&mmk_file_1),
            additional_flags: std::collections::HashMap::new(),
        })
    );
    Ok(())
}

#[test]
fn read_mmk_files_three_files_one_common_dependency() -> std::io::Result<()> {
    let (_dir, test_file_path, _, mut mmk_file_1) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, _, _) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");

    mmk_file_1.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(&test_file_dep_path.parent().unwrap().display().to_string()),
            Keyword::from(
                &test_file_second_dep_path
                    .parent()
                    .unwrap()
                    .display()
                    .to_string(),
            ),
        ],
    );
    mmk_file_1
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);

    let mut dep_registry = DependencyRegistry::new();
    let result = Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() {
    let (dir, test_file_path, _, mut mmk_file_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut mmk_file_2) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");
    let (third_dir_dep, test_file_third_dep_path, _file_third_file_dep, _) =
        make_mmk_file("example_dep_third");

    mmk_file_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![
            Keyword::from(test_file_third_dep_path.parent().unwrap().to_str().unwrap()),
            Keyword::from(test_file_dep_path.parent().unwrap().to_str().unwrap()),
        ],
    );
    mmk_file_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    write!(
        file_dep,
        "\
        MMK_REQUIRE:
            {}
        \n
        ",
        test_file_second_dep_path.parent().unwrap().display()
    )
    .unwrap();

    mmk_file_2.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            &test_file_second_dep_path
                .parent()
                .unwrap()
                .display()
                .to_string(),
        )],
    );

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1).unwrap();

    let expected_lib_name_dep = utility::get_head_directory(&dir_dep.path())
        .display()
        .to_string();
    let expected_lib_name_second_dep = utility::get_head_directory(&second_dir_dep.path())
        .display()
        .to_string();
    let expected_lib_name_third_dep = utility::get_head_directory(&third_dir_dep.path())
        .display()
        .to_string();

    assert_eq!(
        top_dependency,
        DependencyNode::new(Dependency {
            path: test_file_path,
            requires: vec![
                DependencyNode::new(Dependency {
                    path: test_file_third_dep_path,
                    requires: vec![],
                    state: DependencyState::Registered,
                    associated_files: expected_associated_files(
                        &third_dir_dep.path().join("source")
                    ),
                    dependency_type: DependencyType::Library(expected_lib_name_third_dep),
                    include_directories: None,
                    additional_flags: std::collections::HashMap::new(),
                }),
                DependencyNode::new(Dependency {
                    path: test_file_dep_path,
                    requires: vec![DependencyNode::new(Dependency {
                        path: test_file_second_dep_path,
                        requires: vec![],
                        state: DependencyState::Registered,
                        associated_files: expected_associated_files(
                            &second_dir_dep.path().join("source")
                        ),
                        dependency_type: DependencyType::Library(expected_lib_name_second_dep),
                        include_directories: None,
                        additional_flags: std::collections::HashMap::new(),
                    })],
                    state: DependencyState::Registered,
                    associated_files: expected_associated_files(&dir_dep.path().join("source")),
                    dependency_type: DependencyType::Library(expected_lib_name_dep),
                    include_directories: IncludeDirectories::from_mmk(&mmk_file_2),
                    additional_flags: std::collections::HashMap::new(),
                })
            ],
            state: DependencyState::Registered,
            associated_files: expected_associated_files(&dir.path().join("source")),
            dependency_type: DependencyType::Executable("x".to_string()),
            include_directories: IncludeDirectories::from_mmk(&mmk_file_1),
            additional_flags: std::collections::HashMap::new(),
        })
    );
}

#[test]
fn read_mmk_files_two_files_circulation() -> Result<(), crate::errors::DependencyError> {
    let (_dir, test_file_path, _, mut mmk_file_1) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _mmk_file_2) = make_mmk_file("example_dep");

    mmk_file_1.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![Keyword::from(
            &test_file_dep_path.parent().unwrap().display().to_string(),
        )],
    );
    mmk_file_1
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);

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

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency = Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1);

    assert!(top_dependency.is_err());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_one_dependency_serial_and_one_circular_serial() -> std::io::Result<()>
{
    let (_dir, test_file_path, _, mut mmk_file_1) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _mmk_file_2) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, mut file_second_file_dep, _mmk_file_3) =
        make_mmk_file("example_dep_second");

    mmk_file_1.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![Keyword::from(
            &test_file_dep_path.parent().unwrap().display().to_string(),
        )],
    );
    mmk_file_1
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);

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

    write!(
        file_second_file_dep,
        "\
        MMK_REQUIRE:
            {}
        \n
        ",
        &test_file_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string()
    )?;

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency = Dependency::from_path(&test_file_path, &mut dep_registry, &mmk_file_1);
    assert!(top_dependency.is_err());
    Ok(())
}

#[test]
fn get_project_name_test() {
    let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
    let dependency = Dependency::from(&project_path);
    assert_eq!(
        std::path::PathBuf::from("MyProject"),
        dependency.get_project_name()
    );
}

#[test]
fn is_executable_test() {
    let (_dir, test_file_path, _, mut mmk_file_1) = make_mmk_file("example");
    let mut dependency = Dependency::from(&test_file_path);
    mmk_file_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);
    dependency.determine_dependency_type(&mmk_file_1).unwrap();
    assert!(dependency.is_executable());
}

#[test]
fn is_executable_false_test() {
    let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
    let dependency = Dependency::from(&project_path);
    assert!(!dependency.is_executable());
}
