use super::*;
use mmk_parser::{Keyword, Mmk};
use pretty_assertions::assert_eq;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use tempdir::TempDir;
use utility;

#[allow(dead_code)]
fn expected_library_name(path: &std::path::Path) -> String {
    let mut library_name = String::from("lib");
    library_name.push_str(utility::get_head_directory(path).to_str().unwrap());
    library_name.push_str(".a");
    library_name
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
    let (_dir, lib_file_path, _file, _expected) = make_mmk_file("example");
    let mut dep_registry = DependencyRegistry::new();
    Dependency::create_dependency_from_path(&lib_file_path, &mut dep_registry).unwrap()
}

#[test]
fn test_get_pretty_name_executable() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);
    let actual = dependency.borrow().get_pretty_name();
    let expected = "x";
    assert_eq!(expected, actual);
}

#[test]
fn test_get_pretty_name_library_label() {
    let dependency = fixture_simple_dependency();
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("MYLIB")],
    );
    dependency.borrow_mut().add_library_name();
    let actual = dependency.borrow().get_pretty_name();
    let expected = "MYLIB";
    assert_eq!(expected, actual);
}

#[test]
fn test_is_in_process_true() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::InProcess);
    assert!(dependency.borrow().is_in_process());
}

#[test]
fn test_is_in_process_false() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::Registered);
    assert!(!dependency.borrow().is_in_process());
}

#[test]
fn test_is_makefile_made_true() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::MakefileMade);
    assert!(dependency.borrow().is_makefile_made());
}

#[test]
fn test_is_makefile_made_false() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::NotInProcess);
    assert!(!dependency.borrow().is_makefile_made());
}

#[test]
fn test_is_building_true() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::Building);
    assert!(dependency.borrow().is_building());
}

#[test]
fn test_is_building_false() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::BuildComplete);
    assert!(!dependency.borrow().is_building());
}

#[test]
fn test_is_build_completed_true() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::BuildComplete);
    assert!(dependency.borrow().is_build_completed());
}

#[test]
fn test_is_build_completed_false() {
    let dependency = fixture_simple_dependency();
    dependency
        .borrow_mut()
        .change_state(DependencyState::NotInProcess);
    assert!(!dependency.borrow().is_build_completed());
}

#[test]
fn read_mmk_files_one_file() -> std::io::Result<()> {
    let (_dir, lib_file_path, mut file, mut expected) = make_mmk_file("example");

    write!(
        file,
        "MMK_EXECUTABLE:
                x"
    )?;
    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&lib_file_path, &mut dep_registry).unwrap();
    expected
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);
    assert_eq!(top_dependency.borrow().mmk_data(), &expected);
    Ok(())
}

#[test]
fn read_mmk_files_two_files() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut expected_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, _file_dep, expected_2) = make_mmk_file("example_dep");

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

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

    expected_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_dep_path.parent().unwrap().to_str().unwrap(),
        )],
    );
    expected_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    let expected_lib_name = expected_library_name(&dir.path());
    let expected_lib_name_dep = expected_library_name(&dir_dep.path());
    assert_eq!(
        top_dependency,
        Rc::new(RefCell::new(Dependency {
            path: test_file_path,
            mmk_data: expected_1,
            requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                path: test_file_dep_path,
                mmk_data: expected_2,
                requires: RefCell::new(Vec::new()),
                library_name: expected_lib_name_dep,
                state: DependencyState::Registered,
            }))]),
            library_name: expected_lib_name,
            state: DependencyState::Registered,
        }))
    );
    Ok(())
}

#[test]
fn add_library_name_test() {
    let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
    dependency.add_library_name();
    assert_eq!(dependency.library_name(), String::from("libdirectory.a"));
}

#[test]
fn add_library_name_from_label_test() {
    let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
    dependency.mmk_data_mut().data_mut().insert(
        String::from("MMK_LIBRARY_LABEL"),
        vec![Keyword::from("mylibrary")],
    );
    dependency.add_library_name();
    assert_eq!(dependency.library_name(), String::from("mylibrary"));
}

#[test]
fn library_file_name_test() {
    let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
    dependency.add_library_name();
    assert_eq!(
        dependency.library_file_name(),
        String::from("libdirectory.a")
    );
}

#[test]
fn library_file_name_from_label_test() {
    let mut dependency = Dependency::from(&std::path::PathBuf::from("/some/directory/src/lib.mmk"));
    dependency.mmk_data_mut().data_mut().insert(
        String::from("MMK_LIBRARY_LABEL"),
        vec![Keyword::from("mylibrary")],
    );
    dependency.add_library_name();
    assert_eq!(
        dependency.library_file_name(),
        String::from("libmylibrary.a")
    );
}

#[test]
fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut expected_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, _file_dep, expected_2) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) =
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
        test_file_dep_path.parent().unwrap().to_str().unwrap(),
        test_file_second_dep_path
            .parent()
            .unwrap()
            .to_str()
            .unwrap()
    )?;

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

    expected_1.data_mut().insert(
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
    expected_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    let expected_lib_name = expected_library_name(&dir.path());
    let expected_lib_name_dep = expected_library_name(&dir_dep.path());
    let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());

    assert_eq!(
        top_dependency,
        Rc::new(RefCell::new(Dependency {
            path: test_file_path,
            mmk_data: expected_1,
            requires: RefCell::new(vec![
                Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(Vec::new()),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(Vec::new()),
                    library_name: expected_lib_name_second_dep,
                    state: DependencyState::Registered,
                }))
            ]),
            library_name: expected_lib_name,
            state: DependencyState::Registered,
        }))
    );
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut expected_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut expected_2) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) =
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

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

    expected_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_dep_path.parent().unwrap().to_str().unwrap(),
        )],
    );
    expected_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    expected_2.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_second_dep_path
                .parent()
                .unwrap()
                .to_str()
                .unwrap(),
        )],
    );

    let expected_lib_name = expected_library_name(&dir.path());
    let expected_lib_name_dep = expected_library_name(&dir_dep.path());
    let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());

    assert_eq!(
        top_dependency,
        Rc::new(RefCell::new(Dependency {
            path: test_file_path,
            mmk_data: expected_1,
            requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                path: test_file_dep_path,
                mmk_data: expected_2,
                requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                    path: test_file_second_dep_path,
                    mmk_data: expected_3,
                    requires: RefCell::new(vec![]),
                    library_name: expected_lib_name_second_dep,
                    state: DependencyState::Registered,
                }))]),
                library_name: expected_lib_name_dep,
                state: DependencyState::Registered,
            }))]),
            library_name: expected_lib_name,
            state: DependencyState::Registered,
        }))
    );
    Ok(())
}

#[test]
fn read_mmk_files_three_files_one_common_dependency() -> std::io::Result<()> {
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, _file_second_file_dep, _) =
        make_mmk_file("example_dep_second");

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
            .to_string()
    )?;

    let mut dep_registry = DependencyRegistry::new();
    let result = Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

    assert!(result.is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
    let (dir, test_file_path, mut file, mut expected_1) = make_mmk_file("example");
    let (dir_dep, test_file_dep_path, mut file_dep, mut expected_2) = make_mmk_file("example_dep");
    let (second_dir_dep, test_file_second_dep_path, _file_second_file_dep, expected_3) =
        make_mmk_file("example_dep_second");
    let (third_dir_dep, test_file_third_dep_path, _file_third_file_dep, expected_4) =
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

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry).unwrap();

    expected_1.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![
            Keyword::from(test_file_third_dep_path.parent().unwrap().to_str().unwrap()),
            Keyword::from(test_file_dep_path.parent().unwrap().to_str().unwrap()),
        ],
    );
    expected_1
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);

    expected_2.data_mut().insert(
        String::from("MMK_REQUIRE"),
        vec![Keyword::from(
            test_file_second_dep_path
                .parent()
                .unwrap()
                .to_str()
                .unwrap(),
        )],
    );

    let expected_lib_name = expected_library_name(&dir.path());
    let expected_lib_name_dep = expected_library_name(&dir_dep.path());
    let expected_lib_name_second_dep = expected_library_name(&second_dir_dep.path());
    let expected_lib_name_third_dep = expected_library_name(&third_dir_dep.path());

    assert_eq!(
        top_dependency,
        Rc::new(RefCell::new(Dependency {
            path: test_file_path,
            mmk_data: expected_1,
            requires: RefCell::new(vec![
                Rc::new(RefCell::new(Dependency {
                    path: test_file_third_dep_path,
                    mmk_data: expected_4,
                    requires: RefCell::new(vec![]),
                    library_name: expected_lib_name_third_dep,
                    state: DependencyState::Registered,
                })),
                Rc::new(RefCell::new(Dependency {
                    path: test_file_dep_path,
                    mmk_data: expected_2,
                    requires: RefCell::new(vec![Rc::new(RefCell::new(Dependency {
                        path: test_file_second_dep_path,
                        mmk_data: expected_3,
                        requires: RefCell::new(vec![]),
                        library_name: expected_lib_name_second_dep,
                        state: DependencyState::Registered,
                    }))]),
                    library_name: expected_lib_name_dep,
                    state: DependencyState::Registered,
                }))
            ]),
            library_name: expected_lib_name,
            state: DependencyState::Registered,
        }))
    );
    Ok(())
}

#[test]
fn read_mmk_files_two_files_circulation() -> Result<(), crate::errors::DependencyError> {
    let (_dir, test_file_path, mut file, _expected_1) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _expected_2) = make_mmk_file("example_dep");

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

    let mut dep_registry = DependencyRegistry::new();
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);

    assert!(top_dependency.is_err());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_one_dependency_serial_and_one_circular_serial() -> std::io::Result<()>
{
    let (_dir, test_file_path, mut file, _expected_1) = make_mmk_file("example");
    let (_dir_dep, test_file_dep_path, mut file_dep, _expected_2) = make_mmk_file("example_dep");
    let (_second_dir_dep, test_file_second_dep_path, mut file_second_file_dep, _expected_3) =
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
    let top_dependency =
        Dependency::create_dependency_from_path(&test_file_path, &mut dep_registry);
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
    let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
    let mut dependency = Dependency::from(&project_path);
    dependency
        .mmk_data_mut()
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);
    assert!(dependency.is_executable());
}

#[test]
fn is_executable_false_test() {
    let project_path = std::path::PathBuf::from("/some/path/name/for/MyProject/test/run.mmk");
    let dependency = Dependency::from(&project_path);
    assert!(!dependency.is_executable());
}
