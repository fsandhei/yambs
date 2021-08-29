use super::*;
use dependency::Dependency;
use mmk_parser::Keyword;
use pretty_assertions::assert_eq;
use std::cell::RefCell;
use std::fs;
use std::rc::Rc;
use tempdir::TempDir;

#[allow(dead_code)]
fn expected_library_name(path: &std::path::Path) -> String {
    let mut library_name = String::from("lib");
    library_name.push_str(utility::get_head_directory(path).to_str().unwrap());
    library_name.push_str(".a");
    library_name
}

fn construct_generator(path: PathBuf) -> MakefileGenerator {
    let toolchain = Toolchain::new().set_sample_config();
    MakefileGenerator::new(path, &toolchain)
}

#[test]
fn generate_makefile_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let source_dir = dir.path().join("source");
    utility::create_dir(&source_dir).unwrap();
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    dependency
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert(String::from("MMK_EXECUTABLE"), vec![Keyword::from("main")]);
    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    assert!(Generator::generate_makefile(&mut gen).is_ok());
    Ok(())
}

#[test]
fn print_debug_test() -> std::io::Result<()> {
    let path = std::path::PathBuf::from("some_path");
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&path.join("run.mmk"))));
    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    gen.debug();
    assert_eq!(
        format!(
            "{directory}/make_include/debug.mk",
            directory = output_dir.path().to_str().unwrap()
        ),
        gen.print_debug()
    );
    Ok(())
}

#[test]
fn generate_header_release_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
            directory = output_dir.path().to_str().unwrap()
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_header_debug_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
            directory = output_dir.path().to_str().unwrap()
        ),
        fs::read_to_string(test_file.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_package_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_new_dep")?;
    let output_dir = TempDir::new("build")?;
    let include_dir = dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    utility::create_dir(dir_first_dep.path().join("include")).unwrap();
    utility::create_dir(dir_second_dep.path().join("include")).unwrap();
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    dependency.borrow_mut().add_library_name();
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );

    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
    \t-lstdc++\n\
    \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
    \n\
    {directory}/filename.o: \\\n\
    \t{dep_directory}/filename.cpp\n\
    \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    {directory}/ofilename.o: \\\n\
    \t{dep_directory}/ofilename.cpp\n\
    \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    sinclude {directory}/filename.d\n\
    sinclude {directory}/ofilename.d\n\
    \n", 
    directory = output_dir.path().to_str().unwrap(),
    dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap(),
    dir_dep_str = dir_first_dep.path().to_str().unwrap().to_string(),
    dir_second_dep_str = dir_second_dep.path().to_str().unwrap().to_string()),
    fs::read_to_string(test_file.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn generate_executable_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_new_dep")?;
    let output_dir = TempDir::new("build")?;
    let include_dir = dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    utility::create_dir(dir_first_dep.path().join("include")).unwrap();
    utility::create_dir(dir_second_dep.path().join("include")).unwrap();

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    dependency
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert("MMK_EXECUTABLE".to_string(), vec![Keyword::from("x")]);
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );
    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
    \t-lstdc++\n\
    \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $^ -o $@)\n\
    \n\
    {directory}/filename.o: \\\n\
    \t{dep_directory}/filename.cpp\n\
    \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    {directory}/ofilename.o: \\\n\
    \t{dep_directory}/ofilename.cpp\n\
    \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) -I{dep_directory} -I{dir_dep_str}/include -I{dir_second_dep_str}/include $< -c -o $@)\n\
    \n\
    sinclude {directory}/filename.d\n\
    sinclude {directory}/ofilename.d\n\
    \n",
    directory = output_dir.path().to_str().unwrap(),
    dep_directory = dependency.borrow().get_parent_directory().to_str().unwrap(),
    dir_dep_str = dir_first_dep.path().to_str().unwrap().to_string(),
    dir_second_dep_str = dir_second_dep.path().to_str().unwrap().to_string()),
    fs::read_to_string(test_file.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn generate_appending_flags_test_cxxflags() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_CXXFLAGS_APPEND".to_string(),
        vec![Keyword::from("-pthread")],
    );
    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_CPPFLAGS_APPEND".to_string(),
        vec![Keyword::from("-somesetting")],
    );
    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_CXXFLAGS_APPEND".to_string(),
        vec![Keyword::from("-pthread")],
    );
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_CPPFLAGS_APPEND".to_string(),
        vec![Keyword::from("-somesetting")],
    );

    let mut gen = construct_generator(output_dir.path().to_path_buf());
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
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SOURCES".to_string(),
        vec![
            Keyword::from("filename.cpp"),
            Keyword::from("ofilename.cpp"),
        ],
    );
    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let actual = gen.print_header_includes();
    let expected = format!(
        "sinclude {directory}/filename.d\n\
                                    sinclude {directory}/ofilename.d\n",
        directory = output_dir.path().to_str().unwrap()
    );
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_dependencies_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_second_dep")?;

    let dep_include_dir = dir_first_dep.path().join("include");
    let second_dep_include_dir = dir_second_dep.path().join("include");
    utility::create_dir(&dep_include_dir).unwrap();
    utility::create_dir(&second_dep_include_dir).unwrap();

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![
            Keyword::from(dir_first_dep.path().to_str().unwrap()),
            Keyword::from(dir_second_dep.path().to_str().unwrap()),
        ],
    );

    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{} -I{} -I{}",
        dir.path().to_str().unwrap(),
        dep_include_dir.to_str().unwrap(),
        second_dep_include_dir.to_str().unwrap()
    );
    let actual = gen.print_dependencies();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_dependencies_with_sys_include_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dir_first_dep = TempDir::new("example_dep")?;
    let dir_second_dep = TempDir::new("example_second_dep")?;

    let dep_include_dir = dir_first_dep.path().join("include");
    let second_dep_include_dir = dir_second_dep.path().join("include");
    utility::create_dir(&dep_include_dir).unwrap();
    utility::create_dir(&second_dep_include_dir).unwrap();

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_REQUIRE".to_string(),
        vec![Keyword::from(dir_first_dep.path().to_str().unwrap())],
    );
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SYS_INCLUDE".to_string(),
        vec![Keyword::from(dir_second_dep.path().to_str().unwrap())],
    );

    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{} -I{} -isystem {}",
        dir.path().to_str().unwrap(),
        dep_include_dir.to_str().unwrap(),
        dir_second_dep.path().to_str().unwrap()
    );
    let actual = gen.print_dependencies();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_dependencies_with_only_sys_include_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dir_dep = TempDir::new("example_dep")?;

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_SYS_INCLUDE".to_string(),
        vec![Keyword::from(dir_dep.path().to_str().unwrap())],
    );

    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "-I{}  -isystem {}",
        dir.path().to_str().unwrap(),
        dir_dep.path().to_str().unwrap()
    );
    let actual = gen.print_dependencies();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_required_dependencies_libraries_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dir_dep = TempDir::new("example_dep")?;

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    let dependency_dep = Rc::new(RefCell::new(Dependency::from(
        &dir_dep.path().join("lib.mmk"),
    )));
    dependency_dep
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert(
            "MMK_LIBRARY_LABEL".to_string(),
            vec![Keyword::from("myDependency")],
        );
    dependency_dep.borrow_mut().add_library_name();
    dependency
        .borrow_mut()
        .add_dependency(Rc::clone(&dependency_dep));

    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!(
        "\t{directory}/libs/{dep_directory}/release/{library_name} \\\n",
        directory = output_dir.path().to_str().unwrap(),
        dep_directory = dependency_dep.borrow().get_project_name().to_str().unwrap(),
        library_name = dependency_dep.borrow().library_file_name()
    );

    let actual = gen.print_required_dependencies_libraries();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_required_dependencies_libraries_multiple_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dir_dep = TempDir::new("example_dep")?;

    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("run.mmk"))));
    let dependency_dep = Rc::new(RefCell::new(Dependency::from(
        &dir_dep.path().join("lib.mmk"),
    )));
    let second_dependency_dep = Rc::new(RefCell::new(Dependency::from(
        &dir_dep.path().join("lib.mmk"),
    )));
    dependency_dep
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert(
            "MMK_LIBRARY_LABEL".to_string(),
            vec![Keyword::from("myDependency")],
        );
    dependency_dep.borrow_mut().add_library_name();
    second_dependency_dep
        .borrow_mut()
        .mmk_data_mut()
        .data_mut()
        .insert(
            "MMK_LIBRARY_LABEL".to_string(),
            vec![Keyword::from("mySecondDependency")],
        );
    second_dependency_dep.borrow_mut().add_library_name();
    dependency
        .borrow_mut()
        .add_dependency(Rc::clone(&dependency_dep));
    dependency
        .borrow_mut()
        .add_dependency(Rc::clone(&second_dependency_dep));

    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);
    let expected = format!("\t{directory}/libs/{dep_directory}/release/{library_name} \\\n\
                                    \t{directory}/libs/{second_dep_directory}/release/{second_library_name} \\\n",
                                    directory = output_dir.path().to_str().unwrap(),
                                    dep_directory = dependency_dep.borrow().get_project_name().to_str().unwrap(),
                                    second_dep_directory = second_dependency_dep.borrow().get_project_name().to_str().unwrap(),
                                    library_name = dependency_dep.borrow().library_file_name(),
                                    second_library_name = second_dependency_dep.borrow().library_file_name());

    let actual = gen.print_required_dependencies_libraries();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_library_name_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);

    let expected = format!(
        "{directory}/{library_file_name}",
        directory = output_dir.path().to_str().unwrap(),
        library_file_name = dependency.borrow().library_file_name()
    );
    let actual = gen.print_library_name();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn print_library_name_with_label_test() -> std::io::Result<()> {
    let dir = TempDir::new("example")?;
    let output_dir = TempDir::new("build")?;
    let dependency = Rc::new(RefCell::new(Dependency::from(&dir.path().join("lib.mmk"))));
    dependency.borrow_mut().mmk_data_mut().data_mut().insert(
        "MMK_LIBRARY_LABEL".to_string(),
        vec![Keyword::from("myDependency")],
    );
    dependency.borrow_mut().add_library_name();
    let mut gen = construct_generator(output_dir.path().to_path_buf());
    gen.set_dependency(&dependency);

    let expected = format!(
        "{directory}/libmyDependency.a",
        directory = output_dir.path().to_str().unwrap()
    );
    let actual = gen.print_library_name();
    assert!(actual.is_ok());
    assert_eq!(expected, actual.unwrap());
    Ok(())
}

#[test]
fn replace_generator_test() -> std::io::Result<()> {
    let dir_dep = TempDir::new("example_dep")?;
    let output_dir = TempDir::new("build")?;
    let replacement_output_dir = dir_dep.path().join("build");

    let dependency = Rc::new(RefCell::new(Dependency::from(
        &dir_dep.path().join("lib.mmk"),
    )));
    let mut gen = construct_generator(output_dir.path().to_path_buf());

    gen.replace_generator(&dependency, replacement_output_dir.clone());
    assert_eq!(dependency, gen.dependency.unwrap());
    assert_eq!(replacement_output_dir, gen.output_directory);

    Ok(())
}
