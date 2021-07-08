
use super::*;
use pretty_assertions::assert_eq;
use std::fs;
use tempdir::TempDir;

fn produce_include_path(base_dir: TempDir) -> PathBuf {
    let build_dir = PathBuf::from(".build");
    let output_directory = base_dir.path().join(build_dir).join("make_include");
    output_directory
}

#[test]
fn add_cpp_version_cpp98_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("c++98")?;
    assert_eq!(gen.args["C++"], "-std=c++98");
    Ok(())
}

#[test]
fn add_cpp_version_cpp11_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("c++11")?;
    assert_eq!(gen.args["C++"], "-std=c++11");
    Ok(())
}

#[test]
fn add_cpp_version_cpp14_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("c++14")?;
    assert_eq!(gen.args["C++"], "-std=c++14");
    Ok(())
}

#[test]
fn add_cpp_version_cpp17_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("c++17")?;
    assert_eq!(gen.args["C++"], "-std=c++17");
    Ok(())
}

#[test]
fn add_cpp_version_cpp17_uppercase_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("C++17")?;
    assert_eq!(gen.args["C++"], "-std=c++17");
    Ok(())
}

#[test]
fn add_cpp_version_cpp20_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.add_cpp_version("c++20")?;
    assert_eq!(gen.args["C++"], "-std=c++20");
    Ok(())
}

#[test]
fn add_cpp_version_invalid_test() -> Result<(), MyMakeError> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let result = gen.add_cpp_version("python");
    assert!(result.is_err());
    assert_eq!(
        &String::from("python is not a valid C++ version."),
        result.unwrap_err().to_string()
    );
    Ok(())
}

#[test]
fn generate_strict_mk_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("strict.mk");
    gen.generate_strict_mk().unwrap();
    assert_eq!("\
        #Generated by IncludeFileGenerator.generate_strict_mk. DO NOT EDIT.\n\

        CXXFLAGS += -Wall \\
                    -Wextra \\
                    -Wmisleading-indentation \\
                    -Wduplicated-cond \\
                    -Wduplicated-branches \\
                    -Wshadow \\
                    -Wnon-virtual-dtor \\
                    -Wold-style-cast \\
                    -Wcast-align \\
                    -Wunused \\
                    -Woverloaded-virtual \\
                    -Wpedantic \\
                    -Wconversion \\
                    -Wsign-conversion \\
                    -Wnull-dereference \\
                    -Wdouble-promotion \\
                    -Wformat=2\n\
        \n\
        CXXFLAGS += -std=c++20\n\
        \n\
        \n\

        #-Wall                     # Reasonable and standard\n\
        #-Wextra                   # Warn if indentation implies blocks where blocks do not exist.\n\
        #-Wmisleading-indentation  # Warn if if / else chain has duplicated conditions\n\
        #-Wduplicated-cond         # Warn if if / else branches has duplicated conditions\n\
        #-Wduplicated-branches     # warn the user if a variable declaration shadows one from a parent context\n\
        #-Wshadow                  # warn the user if a class with virtual functions has a non-virtual destructor. This helps\n\
        #-Wnon-virtual-dtor        # catch hard to track down memory errors\n\
        #-Wold-style-cast          # warn for C-style casts\n\
        #-Wcast-align              # warn for potential performance problem casts\n\
        #-Wunused                  # warn on anything being unused\n\
        #-Woverloaded-virtual      # warn if you overload (not override) a virtual function\n\
        #-Wpedantic                # warn if non-standard C++ is used\n\
        #-Wconversion              # warn on type conversions that may lose data\n\
        #-Wsign-conversion         # warn on sign conversions\n\
        #-Wnull-dereference        # warn if a null dereference is detected\n\
        #-Wdouble-promotion        # warn if float is implicit promoted to double\n\
        #-Wformat=2                # warn on security issues around functions that format output (ie printf)\n\
        ", fs::read_to_string(file_name.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn generate_debug_mk_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("debug.mk");
    gen.generate_debug_mk().unwrap();
    assert_eq!(
        "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.\n\
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        \n\


        # When building with sanitizer options, certain linker options must be added.\n\
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.\n\
        # This is done to support address space layout randomization (ASLR).\n\
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization\n\
        ",
        fs::read_to_string(file_name.to_str().unwrap()).unwrap()
    );
    Ok(())
}


#[test]
fn generate_debug_mk_with_address_sanitizer_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("debug.mk");
    gen.set_sanitizers(vec!["address"]);
    gen.generate_debug_mk().unwrap();
    assert_eq!(
        "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.\n\
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        \n\
        CXXFLAGS += -fsanitize=address \n\
        \n\
        LDFLAGS += -fsanitize=address \


        # When building with sanitizer options, certain linker options must be added.\n\
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.\n\
        # This is done to support address space layout randomization (ASLR).\n\
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization\n\
        ",
        fs::read_to_string(file_name.to_str().unwrap()).unwrap()
    );
    Ok(())
}


#[test]
fn generate_debug_mk_with_thread_sanitizer_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("debug.mk");
    gen.set_sanitizers(vec!["thread"]);
    gen.generate_debug_mk().unwrap();
    assert_eq!(
        "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.\n\
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        \n\
        CXXFLAGS += -fsanitize=thread -fPIE -pie \n\
        \n\
        LDFLAGS += -fsanitize=thread -fPIE -pie \


        # When building with sanitizer options, certain linker options must be added.\n\
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.\n\
        # This is done to support address space layout randomization (ASLR).\n\
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization\n\
        ",
        fs::read_to_string(file_name.to_str().unwrap()).unwrap()
    );
    Ok(())
}


#[test]
fn generate_release_mk_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("release.mk");
    gen.generate_release_mk().unwrap();
    assert_eq!(
        "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O2\\
        ",
        fs::read_to_string(file_name.to_str().unwrap()).unwrap()
    );
    Ok(())
}

#[test]
fn generate_default_mk_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let file_name = output_directory.join("default_make.mk");
    gen.generate_default_mk().unwrap();
    assert_eq!("\
        #Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file\n\
        #excluding system header files.\n\
        CPPFLAGS+=-MMD\\
            -MP\n
        \n\
        CXXFLAGS+= -pthread", fs::read_to_string(file_name.to_str().unwrap()).unwrap());
    Ok(())
}

#[test]
fn change_directory_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);

    assert_eq!(gen.output_directory, output_directory);

    let new_output_directory = produce_include_path(TempDir::new("example_new").unwrap());
    gen.change_directory(new_output_directory.clone());

    assert_eq!(gen.output_directory, new_output_directory);
    Ok(())
}


#[test]
fn generate_flags_sanitizer_no_sanitizers_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    let actual = gen.generate_flags_sanitizer();
    let expected = String::new();
    assert_eq!(expected, actual);
    Ok(())
}


#[test]
fn generate_flags_sanitizer_address_sanitizer_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.set_sanitizers(vec!["address"]);
    let actual = gen.generate_flags_sanitizer();
    let expected = String::from("\
                                    CXXFLAGS += -fsanitize=address \n\
                                    \n\
                                    LDFLAGS += -fsanitize=address ");
    assert_eq!(expected, actual);
    Ok(())
}


#[test]
fn generate_flags_sanitizer_thread_sanitizer_test() -> std::io::Result<()> {
    let output_directory = produce_include_path(TempDir::new("example").unwrap());
    let mut gen = IncludeFileGenerator::new(&output_directory);
    gen.set_sanitizers(vec!["thread"]);
    let actual = gen.generate_flags_sanitizer();
    let expected = String::from("\
                                    CXXFLAGS += -fsanitize=thread -fPIE -pie \n\
                                    \n\
                                    LDFLAGS += -fsanitize=thread -fPIE -pie ");
    assert_eq!(expected, actual);
    Ok(())
}