//TODO: Skriv om testene for Builder slik at det stemmer med funksjonalitet.
use super::*;
use generator::generator_mock::GeneratorMock;
use mmk_parser::{Keyword, Mmk};
use std::fs::File;
use std::io::Write;
use tempdir::TempDir;
use utility;

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
fn read_mmk_files_one_file() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
    let (_dir, test_file_path, mut file, mut expected) = make_mmk_file("example");

    write!(
        file,
        "MMK_EXECUTABLE:
                x
            "
    )?;
    assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
    expected
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("x")]);
    assert_eq!(
        builder.top_dependency.unwrap().borrow().mmk_data(),
        &expected
    );
    Ok(())
}

// #[test]
// fn read_mmk_files_one_file_generate_makefile() -> std::io::Result<()> {
//     let mut builder = Builder::new();
//     let (dir, test_file_path, mut file, _) = make_mmk_file("example");

//     write!(
//         file,
//         "MMK_EXECUTABLE:
//             x
//         ")?;

//     assert!(builder.read_mmk_files_~/Documents/Tests/AStarPathFinderfrom_path(&test_file_path).is_ok());
//     builder.add_generator();

//     assert!(builder.generate_makefiles().is_ok());
//     Ok(())
// }

#[test]
fn read_mmk_files_two_files() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
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

    assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
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

    assert!((builder.read_mmk_files_from_path(&test_file_path)).is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_three_files_two_dependencies_serial() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
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

    assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
    Ok(())
}

#[test]
fn read_mmk_files_four_files_two_dependencies_serial_and_one_dependency() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
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

    assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
    Ok(())
}


#[test]
fn read_mmk_files_two_files_circulation() -> Result<(), MyMakeError> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
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

    let result = builder.read_mmk_files_from_path(&test_file_path);

    assert!(result.is_err());
    Ok(())
}

// #[test]
// fn add_generator() -> std::io::Result<()> {
//     let mut generator = GeneratorMock::new();
//     let mut builder = Builder::new(&mut generator);
//     let (_dir, test_file_path, mut file, _) = make_mmk_file("example");

//     write!(
//         file,
//         "MMK_EXECUTABLE:
//             x
//         ")?;
//     assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());

//     builder.add_generator();
//     assert!(builder.generator.is_some());
//     Ok(())
// }

#[test]
fn resolve_build_directory_debug() {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
    builder.debug();
    let path = PathBuf::from("some/path");
    let expected = path.join("debug");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

#[test]
fn resolve_build_directory_release() {
    let mut generator = GeneratorMock::new();
    let builder = Builder::new(&mut generator);
    let path = PathBuf::from("some/path");
    let expected = path.join("release");
    assert_eq!(builder.resolve_build_directory(&path), expected);
}

#[test]
fn construct_build_message_executable() -> std::io::Result<()> {
    let mut generator = GeneratorMock::new();
    let mut builder = Builder::new(&mut generator);
    let (_dir, test_file_path, mut file, _) = make_mmk_file("example");

    write!(
        file,
        "MMK_EXECUTABLE:
                x"
    )?;
    assert!(builder.read_mmk_files_from_path(&test_file_path).is_ok());
    let green_text = "Building".green();
    let expected_message = format!("{} executable \"x\"", green_text);
    let borrowed_dependency = builder.top_dependency.unwrap();
    assert_eq!(
        Builder::construct_build_message(&borrowed_dependency),
        expected_message
    );
    Ok(())
}