use std::fs;

use pretty_assertions::assert_eq;
use tempdir::TempDir;

use super::*;
use crate::cli::build_configurations::BuildDirectory;
use crate::compiler::Compiler;
use crate::dependency::{Dependency, DependencyRegistry};
use crate::mmk_parser::{Keyword, Mmk};

struct MmkStub {
    temp_dir: TempDir,
    output_dir: TempDir,
    mmk_file: Mmk,
    _mmk_fh: std::fs::File,
}

impl MmkStub {
    pub fn new(mmk_file_name: &str) -> Self {
        let temp_dir = TempDir::new("example").unwrap();
        let output_dir = TempDir::new("build").unwrap();
        let mmk_file = Mmk::new(&temp_dir.path().join(mmk_file_name));
        let _mmk_fh = std::fs::File::create(&mmk_file.file()).unwrap();

        Self {
            temp_dir,
            output_dir,
            mmk_file,
            _mmk_fh,
        }
    }
}

fn construct_generator(path: PathBuf) -> MakefileGenerator {
    std::env::set_var("CXX", "g++");
    MakefileGenerator::new(&BuildDirectory::from(path), Compiler::new().unwrap())
}

#[test]
fn generate_makefile_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let source_dir = mmk_stub.temp_dir.path().join("source");
    utility::create_dir(&source_dir).unwrap();
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    mmk_stub
        .mmk_file
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("main")]);

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    assert!(Generator::generate_makefile(&mut gen).is_ok());
    Ok(())
}

#[test]
fn print_debug_test() -> std::io::Result<()> {
    let mmk_stub = MmkStub::new("run.mmk");
    let dependency = DependencyNode::new(Dependency::from(&mmk_stub.mmk_file.file()));
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.debug();
    assert_eq!(
        format!(
            "{directory}/make_include/debug.mk",
            directory = mmk_stub.output_dir.path().to_str().unwrap()
        ),
        gen.print_debug()
    );
    Ok(())
}

#[test]
fn generate_header_release_test() -> std::io::Result<()> {
    let mmk_stub = MmkStub::new("run.mmk");
    let dependency = DependencyNode::new(Dependency::from(&mmk_stub.mmk_file.file()));
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(gen.generate_header().is_ok());
    assert_eq!(
        format!(
            "\
    # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
    \n\
    # ----- INCLUDES -----\n\
    include {directory}/make_include/strict.mk\n\
    include {directory}/make_include/default_make.mk\n\
    include {directory}/make_include/release.mk\n\
    \n\
    # ----- DEFAULT PHONIES -----\n\
    \n\
    .SUFFIXES:         # We do not use suffixes on makefiles.\n\
    .PHONY: all\n\
    .PHONY: package\n\
    .PHONY: install\n\
    .PHONY: uninstall\n\
    .PHONY: clean\n",
            directory = mmk_stub.output_dir.path().to_str().unwrap()
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_header_debug_test() -> std::io::Result<()> {
    let mmk_stub = MmkStub::new("run.mmk");
    let dependency = DependencyNode::new(Dependency::from(&mmk_stub.mmk_file.file()));
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.debug();
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(gen.generate_header().is_ok());
    assert_eq!(
        format!(
            "\
    # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
    \n\
    # ----- INCLUDES -----\n\
    include {directory}/make_include/strict.mk\n\
    include {directory}/make_include/default_make.mk\n\
    include {directory}/make_include/debug.mk\n\
    \n\
    # ----- DEFAULT PHONIES -----\n\
    \n\
    .SUFFIXES:         # We do not use suffixes on makefiles.\n\
    .PHONY: all\n\
    .PHONY: package\n\
    .PHONY: install\n\
    .PHONY: uninstall\n\
    .PHONY: clean\n",
            directory = mmk_stub.output_dir.path().to_str().unwrap()
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_package_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_new_dep")?;
    let include_dir = mmk_stub.temp_dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    utility::create_dir(dir_first_dep.path().join("include")).unwrap();
    utility::create_dir(dir_second_dep.path().join("include")).unwrap();

    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );

    let _ = std::fs::File::create(dir_first_dep.path().join("lib.mmk")).unwrap();
    let _ = std::fs::File::create(dir_second_dep.path().join("lib.mmk")).unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();

    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");

    assert!(Generator::generate_rule_package(&mut gen).is_ok());
    assert_eq!(format!("\n\
    #Generated by MmkGenerator.generate_rule_package(). \n\
    \n\
    {directory}/libtmp.a: \\\n\
    \t{directory}/filename.o \\\n\
    \t{directory}/ofilename.o \\\n\
    \t{build}/libs/{lib_dep} \\\n\
    \t{build}/libs/{second_lib_dep} \\\n\
    \t-lstdc++\n\
    \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
    \n\
    {directory}/filename.o: \\\n\
    \t{dep_directory}/filename.cpp\n\
    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dep_include_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    {directory}/ofilename.o: \\\n\
    \t{dep_directory}/ofilename.cpp\n\
    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dep_include_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    sinclude {directory}/filename.d\n\
    sinclude {directory}/ofilename.d\n\
    \n", 
    directory = mmk_stub.output_dir.path().display(),
    dep_directory = dependency.dependency().ref_dep.get_parent_directory().display(),
    dep_include_directory = dependency.dependency().ref_dep.get_parent_directory().join("include").display(),
    dir_dep_str = dir_first_dep.path().display(),
                       dir_second_dep_str = dir_second_dep.path().display(),
                       build = mmk_stub.output_dir.path().display(),
                       lib_dep = dir_first_dep.path().file_name().map(std::path::PathBuf::from).unwrap().join("release").join("libtmp.a").display(),
                       second_lib_dep = dir_second_dep.path().file_name().map(std::path::PathBuf::from).unwrap().join("release").join("libtmp.a").display(),
    ),
    fs::read_to_string(test_file.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn generate_executable_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_new_dep")?;
    let include_dir = mmk_stub.temp_dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    utility::create_dir(dir_first_dep.path().join("include")).unwrap();
    utility::create_dir(dir_second_dep.path().join("include")).unwrap();

    let mut dep_registry = DependencyRegistry::new();

    let _ = std::fs::File::create(dir_first_dep.path().join("lib.mmk")).unwrap();
    let _ = std::fs::File::create(dir_second_dep.path().join("lib.mmk")).unwrap();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    mmk_stub
        .mmk_file
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );
    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(Generator::generate_rule_executable(&mut gen).is_ok());
    assert_eq!(format!("\n\
    #Generated by MmkGenerator.generate_rule_executable(). \n\
    \n\
    .PHONY: x\n\
    x: \\\n\
    \t{directory}/filename.o \\\n\
    \t{directory}/ofilename.o \\\n\
    \t{build}/libs/{lib_dep} \\\n\
    \t{build}/libs/{second_lib_dep} \\\n\
    \t-lstdc++\n\
    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) -I{dep_directory} -I{dep_include_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $^ -o $@)\n\
    \n\
    {directory}/filename.o: \\\n\
    \t{dep_directory}/filename.cpp\n\
    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dep_include_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    {directory}/ofilename.o: \\\n\
    \t{dep_directory}/ofilename.cpp\n\
    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dep_include_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    sinclude {directory}/filename.d\n\
    sinclude {directory}/ofilename.d\n\
    \n",
    directory = mmk_stub.output_dir.path().to_str().unwrap(),
    dep_directory = dependency.dependency().ref_dep.get_parent_directory().to_str().unwrap(),
    dep_include_directory = dependency.dependency().ref_dep.get_parent_directory().join("include").display(),
    build = mmk_stub.output_dir.path().display(),
lib_dep = dir_first_dep.path().file_name().map(std::path::PathBuf::from).unwrap().join("release").join("libtmp.a").display(),
second_lib_dep = dir_second_dep.path().file_name().map(std::path::PathBuf::from).unwrap().join("release").join("libtmp.a").display(),
    dir_dep_str = dir_first_dep.path().to_str().unwrap().to_string(),
    dir_second_dep_str = dir_second_dep.path().to_str().unwrap().to_string()),
    fs::read_to_string(test_file.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn generate_appending_flags_test_cxxflags() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_CXXFLAGS_APPEND".to_string(),
        vec![Keyword::from("-pthread")],
    );
    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(Generator::generate_appending_flags(&mut gen).is_ok());
    assert_eq!(
        format!(
            "\
    CXXFLAGS += -pthread\n\
    "
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_appending_flags_test_cppflags() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_CPPFLAGS_APPEND".to_string(),
        vec![Keyword::from("-somesetting")],
    );
    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(Generator::generate_appending_flags(&mut gen).is_ok());
    assert_eq!(
        format!(
            "\
    CPPFLAGS += -somesetting\n\
    "
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_appending_flags_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let mut dep_registry = DependencyRegistry::new();
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_CXXFLAGS_APPEND".to_string(),
        vec![Keyword::from("-pthread")],
    );
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_CPPFLAGS_APPEND".to_string(),
        vec![Keyword::from("-somesetting")],
    );

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.create_makefile();
    let test_file = gen.output_directory.join("makefile");
    assert!(Generator::generate_appending_flags(&mut gen).is_ok());
    assert_eq!(
        format!(
            "\
    CXXFLAGS += -pthread\n\
    CPPFLAGS += -somesetting\n\
    "
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn print_header_includes_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let actual = gen.print_header_includes().unwrap();
    let expected = format!(
        "sinclude {directory}/filename.d\n\
                                    sinclude {directory}/ofilename.d\n",
        directory = mmk_stub.output_dir.path().to_str().unwrap()
    );
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn print_dependencies_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_second_dep")?;

    let dep_include_dir = dir_first_dep.path().join("include");
    let second_dep_include_dir = dir_second_dep.path().join("include");
    utility::create_dir(&dep_include_dir).unwrap();
    utility::create_dir(&second_dep_include_dir).unwrap();

    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );

    let _ = std::fs::File::create(&dir_first_dep.path().join("lib.mmk")).unwrap();
    let _ = std::fs::File::create(&dir_second_dep.path().join("lib.mmk")).unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{} -I{} -I{} -I{}",
        mmk_stub.temp_dir.path().display(),
        mmk_stub.temp_dir.path().join("include").display(),
        dep_include_dir.display(),
        second_dep_include_dir.display()
    );
    let actual = gen.print_dependencies().unwrap();
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn print_dependencies_with_sys_include_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_second_dep")?;

    let dep_include_dir = dir_first_dep.path().join("include");
    let second_dep_include_dir = dir_second_dep.path().join("include");
    utility::create_dir(&dep_include_dir).unwrap();
    utility::create_dir(&second_dep_include_dir).unwrap();

    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![Keyword::from(dir_first_dep.path().to_str().unwrap())],
    );
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SYS_INCLUDE".to_string(),
        vec![Keyword::from(dir_second_dep.path().to_str().unwrap())],
    );

    let _ = std::fs::File::create(&dir_first_dep.path().join("lib.mmk")).unwrap();
    let _ = std::fs::File::create(&dir_second_dep.path().join("lib.mmk")).unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{} -I{} -I{} -isystem {}",
        mmk_stub.temp_dir.path().display(),
        mmk_stub.temp_dir.path().join("include").display(),
        dep_include_dir.display(),
        dir_second_dep.path().display()
    );
    let actual = gen.print_dependencies().unwrap();
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn print_dependencies_with_only_sys_include_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_dep = TempDir::new("example_dep")?;
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_SYS_INCLUDE".to_string(),
        vec![Keyword::from(dir_dep.path().to_str().unwrap())],
    );

    let _ = std::fs::File::create(&dir_dep.path().join("lib.mmk")).unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{} -I{} -isystem {}",
        mmk_stub.temp_dir.path().display(),
        mmk_stub.temp_dir.path().join("include").display(),
        dir_dep.path().display()
    );
    let actual = gen.print_dependencies().unwrap();
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn print_required_dependencies_libraries_test() {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_dep = TempDir::new("example_dep").unwrap();
    let mut mmk_file_dep = Mmk::new(&dir_dep.path().join("lib.mmk"));
    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![Keyword::from(&dir_dep.path().display().to_string())],
    );

    mmk_file_dep.data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("myDependency")],
    );

    let mut dep_fh = std::fs::File::create(&dir_dep.path().join("lib.mmk")).unwrap();

    write!(
        dep_fh,
        "\
        MMK_LIBRARY_LABEL:\n\
           myDependency
"
    )
    .unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let dependency_dep = Dependency::from_path(
        &dir_dep.path().join("lib.mmk"),
        &mut dep_registry,
        &mmk_file_dep,
    )
    .unwrap();

    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "\t{directory}/libs/{dep_directory}/release/lib{library_name}.a \\\n",
        directory = mmk_stub.output_dir.path().to_str().unwrap(),
        dep_directory = dependency_dep
            .dependency()
            .ref_dep
            .get_project_name()
            .display(),
        library_name = dependency_dep.dependency().ref_dep.library_file_name()
    );

    let actual = gen.print_required_dependencies_libraries().unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn print_required_dependencies_libraries_multiple_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let dir_dep = TempDir::new("example_dep")?;
    let second_dir_dep = TempDir::new("second_example_dep")?;

    let mut mmk_file_dep = Mmk::new(&dir_dep.path().join("lib.mmk"));
    let mut mmk_file_second_dep = Mmk::new(&second_dir_dep.path().join("lib.mmk"));

    let mut dep_registry = DependencyRegistry::new();

    mmk_stub.mmk_file.data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(&dir_dep.path().display().to_string()),
            Keyword::from(&second_dir_dep.path().display().to_string()),
        ],
    );

    mmk_file_dep.data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("myDependency")],
    );

    mmk_file_second_dep.data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("mySecondDependency")],
    );

    let mut dep_fh = std::fs::File::create(&dir_dep.path().join("lib.mmk")).unwrap();
    let mut second_dep_fh = std::fs::File::create(&second_dir_dep.path().join("lib.mmk")).unwrap();

    write!(
        dep_fh,
        "\
        MMK_LIBRARY_LABEL:\n\
           myDependency
"
    )
    .unwrap();

    write!(
        second_dep_fh,
        "\
        MMK_LIBRARY_LABEL:\n\
           mySecondDependency
"
    )
    .unwrap();

    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();

    let dependency_dep = Dependency::from_path(
        &dir_dep.path().join("lib.mmk"),
        &mut dep_registry,
        &mmk_file_dep,
    )
    .unwrap();

    let second_dependency_dep = Dependency::from_path(
        &second_dir_dep.path().join("lib.mmk"),
        &mut dep_registry,
        &mmk_file_second_dep,
    )
    .unwrap();

    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!("\t{directory}/libs/{dep_directory}/release/lib{library_name}.a \\\n\
                                    \t{directory}/libs/{second_dep_directory}/release/lib{second_library_name}.a \\\n",
                                    directory = mmk_stub.output_dir.path().to_str().unwrap(),
                                    dep_directory = dependency_dep.dependency().ref_dep.get_project_name().to_str().unwrap(),
                                    second_dep_directory = second_dependency_dep.dependency().ref_dep.get_project_name().to_str().unwrap(),
                                    library_name = dependency_dep.dependency().ref_dep.library_file_name(),
                                    second_library_name = second_dependency_dep.dependency().ref_dep.library_file_name());

    let actual = gen.print_required_dependencies_libraries().unwrap();
    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn library_path_test() -> std::io::Result<()> {
    let mut mmk_stub = MmkStub::new("run.mmk");
    let mut dep_registry = DependencyRegistry::new();
    mmk_stub.mmk_file.data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("mylib")],
    );
    let dependency = Dependency::from_path(
        &mmk_stub.mmk_file.file(),
        &mut dep_registry,
        &mmk_stub.mmk_file,
    )
    .unwrap();
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);

    let expected = format!(
        "{directory}/lib{library_file_name}.a",
        directory = mmk_stub.output_dir.path().to_str().unwrap(),
        library_file_name = dependency.dependency().ref_dep.library_file_name()
    );
    let actual = gen.library_path().unwrap();
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn replace_generator_test() -> std::io::Result<()> {
    let mmk_stub = MmkStub::new("run.mmk");
    let dir_dep = TempDir::new("example_dep")?;
    let replacement_output_dir = dir_dep.path().join("build");

    let dependency = DependencyNode::new(Dependency::from(&mmk_stub.mmk_file.file()));
    let mut gen = construct_generator(mmk_stub.output_dir.path().to_path_buf());

    gen.replace_generator(&dependency, replacement_output_dir.clone());
    assert_eq!(dependency, gen.dependency.unwrap());
    assert_eq!(replacement_output_dir, gen.output_directory);

    Ok(())
}
