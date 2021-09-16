use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use crate::generator::{Sanitizer, UtilityGenerator};
use error::MyMakeError;
use mmk_parser::{Constant, Toolchain};
use utility;

#[allow(dead_code)]
pub struct IncludeFileGenerator<'generator> {
    file: Option<File>,
    output_directory: PathBuf,
    args: HashMap<&'generator str, String>,
    toolchain: &'generator Toolchain,
    compiler_constants: HashMap<&'generator str, &'generator str>,
}

#[allow(dead_code)]
impl<'generator> IncludeFileGenerator<'generator> {
    pub fn new(output_directory: &std::path::PathBuf, toolchain: &'generator Toolchain) -> Self {
        utility::create_dir(&output_directory).unwrap();

        let mut compiler_constants = HashMap::with_capacity(30);
        compiler_constants.insert("CC_USES_CLANG", "false");
        compiler_constants.insert("CC_USES_GCC", "false");

        IncludeFileGenerator {
            file: None,
            output_directory: output_directory.clone(),
            args: HashMap::new(),
            toolchain,
            compiler_constants: compiler_constants,
        }
    }

    fn create_mk_file(&mut self, filename_prefix: &str) {
        let mut filename = PathBuf::from(filename_prefix);
        filename.set_extension("mk");
        let file =
            utility::create_file(&self.output_directory, filename.to_str().unwrap()).unwrap();
        self.file = Some(file);
    }

    pub fn get_sanitizers(&self) -> String {
        let result = self.args.get("sanitizers");
        if result.is_some() {
            return format!("-fsanitize={}", result.unwrap());
        }
        String::new()
    }

    pub fn print_build_directory(&self) -> &str {
        self.output_directory.to_str().unwrap()
    }

    pub fn change_directory(&mut self, directory: std::path::PathBuf) {
        self.output_directory = directory;
        utility::create_dir(&self.output_directory).unwrap()
    }

    fn generate_strict_mk(&mut self) -> Result<(), MyMakeError> {
        self.create_mk_file("strict");
        let data = format!("\
        #Generated by IncludeFileGenerator.generate_strict_mk. DO NOT EDIT.\n\
        \n\
        include {def_directory}/defines.mk\n\
        \n\
        \n\
        GLINUX_WARNINGS := -Wall \\
                          -Wextra \\
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
        \n\
        ifeq ($(CC_USES_GCC), true)
            CXXFLAGS += $(GLINUX_WARNINGS) \\
                        -Wmisleading-indentation \\
                        -Wduplicated-cond \\
                        -Wduplicated-branches \\
                        -Wlogical-op \\
                        -Wuseless-cast\n\
       \n\
       \n\
       else ifeq ($(CC_USES_CLANG), true)
            CXXFLAGS += $(GLINUX_WARNINGS)\n\
        
       endif\n\
        CXXFLAGS += {cpp_version}\n\
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
        ", 
        cpp_version = self.print_cpp_version(),
        def_directory = self.print_build_directory());
        match self.file.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => {
                return Err(MyMakeError::from(format!(
                    "Error creating strict.mk: {}",
                    err
                )))
            }
        };
        Ok(())
    }

    fn generate_debug_mk(&mut self) -> Result<(), MyMakeError> {
        self.create_mk_file("debug");
        let data = format!(
            "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.\n\
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        \n\
        {flags_sanitizer}\

        # When building with sanitizer options, certain linker options must be added.\n\
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.\n\
        # This is done to support address space layout randomization (ASLR).\n\
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization\n\
        ",
            flags_sanitizer = self.generate_flags_sanitizer()
        );
        match self.file.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => {
                return Err(MyMakeError::from(format!(
                    "Error creating debug.mk: {}",
                    err
                )))
            }
        };
        Ok(())
    }

    fn generate_release_mk(&mut self) -> Result<(), MyMakeError> {
        self.create_mk_file("release");
        let data = format!(
            "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O3\\
            -DNDEBUG\n
        "
        );
        match self.file.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => {
                return Err(MyMakeError::from(format!(
                    "Error creating release.mk: {}",
                    err
                )))
            }
        };
        Ok(())
    }

    fn generate_default_mk(&mut self) -> Result<(), MyMakeError> {
        self.create_mk_file("default_make");
        let data = format!("\
        #Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file\n\
        #excluding system header files.\n\
        CPPFLAGS+=-MMD\\
            -MP\n
        \n\
        CXXFLAGS+= -pthread");
        match self.file.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => (),
            Err(err) => {
                return Err(MyMakeError::from(format!(
                    "Error creating release.mk: {}",
                    err
                )))
            }
        };
        Ok(())
    }

    fn generate_defines_mk(&mut self) -> Result<(), MyMakeError> {
        self.create_mk_file("defines");

        let data = format!(
            "\
        # Defines.mk\n\
        # Contains a number of defines determined from MyMake configuration time.\n\
        \n\
        CC := {compiler_path}\n\
        {compiler_conditional_flags}\n\
        CP := /usr/bin/cp\n\
        CP_FORCE := -f\n\
        \n\
        ",
            compiler_path = self
                .toolchain
                .get_item(&Constant::new("compiler"))?
                .to_str()
                .ok_or_else(|| "/usr/bin/gcc")
                .unwrap(),
            compiler_conditional_flags = self.evaluate_compiler().unwrap()
        );
        match self.file.as_ref().unwrap().write(data.as_bytes()) {
            Ok(_) => Ok(()),
            Err(err) => {
                return Err(MyMakeError::from(format!(
                    "Error creating defines.mk: {}",
                    err
                )))
            }
        }
    }

    fn evaluate_compiler(&mut self) -> Result<String, MyMakeError> {
        let compiler_path = self.toolchain.get_item(&Constant::new("compiler"))?;
        let compiler = utility::get_head_directory(compiler_path);
        let compiler_str = compiler.to_str().ok_or_else(|| {
            MyMakeError::from_str(
                "Error creating &str from OsStr in include_file_generator.evaluate_compiler()",
            )
        })?;
        match compiler_str {
            "gcc" => {
                self.compiler_constants.insert("CC_USES_GCC", "true");
            }
            "clang" => {
                self.compiler_constants.insert("CC_USES_CLANG", "true");
            }
            _ => {
                return Err(MyMakeError::from(format!(
                    "no settings exist for {}",
                    compiler_str
                )))
            }
        };

        let (gcc_key, gcc_value) = self
            .get_makefile_constant("CC_USES_GCC")
            .unwrap_or((&"CC_USES_GCC", &"true"));

        let (clang_key, clang_value) = self
            .get_makefile_constant("CC_USES_CLANG")
            .unwrap_or((&"CC_USES_CLANG", &"true"));

        Ok(format!(
            "{} := {}\n\
                    {} := {}\n",
            gcc_key, gcc_value, clang_key, clang_value
        ))
    }

    fn get_makefile_constant(&self, key: &str) -> Option<(&&str, &&str)> {
        self.compiler_constants.get_key_value(key)
    }
}

impl<'generator> UtilityGenerator<'generator> for IncludeFileGenerator<'generator> {
    fn generate_makefiles(&'generator mut self) -> Result<(), MyMakeError> {
        self.generate_strict_mk()?;
        self.generate_debug_mk()?;
        self.generate_default_mk()?;
        self.generate_defines_mk()?;
        self.generate_release_mk()
    }

    fn add_cpp_version(&mut self, version: &str) -> Result<(), MyMakeError> {
        let cpp_version_string = match version.to_lowercase().as_str() {
            "c++98" => "-std=c++98",
            "c++11" => "-std=c++11",
            "c++14" => "-std=c++14",
            "c++17" => "-std=c++17",
            "c++20" => "-std=c++20",
            _ => {
                return Err(MyMakeError::from(format!(
                    "{} is not a valid C++ version.",
                    version
                )))
            }
        };
        self.args.insert("C++", cpp_version_string.to_string());
        Ok(())
    }

    fn print_cpp_version(&'generator self) -> &str {
        if self.args.contains_key("C++") {
            self.args.get("C++").unwrap()
        } else {
            "-std=c++20"
        }
    }

    fn generate_flags_sanitizer(&self) -> String {
        if self.args.contains_key("sanitizers") {
            return format!(
                "\
            CXXFLAGS += {sanitizers}\n\
            \n\
            LDFLAGS += {sanitizers}",
                sanitizers = self.get_sanitizers()
            );
        }
        String::new()
    }
}

impl<'generator> Sanitizer for IncludeFileGenerator<'generator> {
    fn set_sanitizers(&mut self, sanitizers: &[String]) {
        let mut sanitizer_str = String::new();
        for option in sanitizers {
            match option.as_str() {
                "address" => sanitizer_str.push_str("address "), // sanitizer_str.push_str("address kernel-adress hwaddress pointer-compare pointer-subtract"),
                "thread" => sanitizer_str.push_str("thread -fPIE -pie "),
                "leak" => sanitizer_str.push_str("leak "),
                "undefined" => sanitizer_str.push_str("undefined "),
                _ => (),
            }
        }
        self.args.insert("sanitizers", sanitizer_str);
    }
}

#[cfg(test)]
#[path = "./include_file_generator_test.rs"]
mod include_file_generator_test;
