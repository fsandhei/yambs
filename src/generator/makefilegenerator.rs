use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use indoc;

use crate::build_target::{
    associated_files::SourceFile, include_directories::IncludeType, TargetNode,
};
use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::compiler::{Compiler, Type};
use crate::errors::FsError;
use crate::generator::{Generator, GeneratorError, Sanitizer, UtilityGenerator};
use crate::utility;

fn directory_from_build_configurations(
    build_configurations: &BuildConfigurations,
) -> std::path::PathBuf {
    for build_configuration in build_configurations {
        if *build_configuration == Configuration::Debug {
            return std::path::PathBuf::from("debug");
        } else if *build_configuration == Configuration::Release {
            return std::path::PathBuf::from("release");
        }
    }
    std::path::PathBuf::from("debug")
}

pub struct MakefileGenerator {
    pub compiler: Compiler,
    pub build_configurations: BuildConfigurations,
    pub build_directory: BuildDirectory,
    pub output_directory: std::path::PathBuf,
}

impl MakefileGenerator {
    pub fn new(
        build_configurations: &BuildConfigurations,
        build_directory: &BuildDirectory,
        compiler: Compiler,
    ) -> Result<Self, GeneratorError> {
        utility::create_dir(&build_directory.as_path())?;
        Ok(Self {
            compiler,
            build_configurations: build_configurations.to_owned(),
            build_directory: build_directory.clone(),
            output_directory: build_directory.as_path().to_path_buf(),
        })
    }

    fn build_configurations_file(&self) -> &str {
        if self.build_configurations.is_debug_build() {
            "debug.mk"
        } else {
            "release.mk"
        }
    }

    fn push_and_create_directory(&mut self, dir: &std::path::Path) -> Result<(), GeneratorError> {
        self.output_directory.push(dir);
        self.create_subdir(dir)?;
        Ok(())
    }

    fn create_subdir(&self, dir: &std::path::Path) -> Result<(), GeneratorError> {
        utility::create_dir(&self.output_directory.join(dir)).map_err(GeneratorError::Fs)
    }

    fn generate_default_all_target(&self, generate: &mut Generate, targets: &[TargetNode]) {
        let targets_as_string = {
            let mut targets_as_string = String::new();
            for target in targets {
                targets_as_string.push_str("\\\n");
                targets_as_string.push_str(&format!("{}", target.borrow().name()))
            }
            targets_as_string
        };
        let text = indoc::formatdoc!(
            "\
            # Default all target to build all targets.
            all : {}\n",
            targets_as_string
        );
        generate.data.push_str(&text);
    }

    fn generate_phony(&self, generate: &mut Generate, target: &TargetNode) -> () {
        let data = indoc::formatdoc!(
            "\
            # Phony for target \"{target_name}\"
            .PHONY: {target_name}
        ",
            target_name = target.borrow().name()
        );
        generate.data.push_str(&data);
    }

    fn generate_header(
        &self,
        generate: &mut Generate,
        targets: &[TargetNode],
    ) -> Result<(), GeneratorError> {
        let data = format!(
            "\
  # ----- INCLUDES -----\n\
  include {build_directory}/make_include/strict.mk\n\
  include {build_directory}/make_include/default_make.mk\n\
  include {build_directory}/make_include/{build_configuration_file}\n\
  \n\
  # ----- DEFAULT PHONIES -----\n\
  \n\
  .SUFFIXES:         # We do not use suffixes on makefiles.\n\
  .PHONY: all\n\
  .PHONY: package\n\
  .PHONY: install\n\
  .PHONY: uninstall\n\
  .PHONY: clean\n",
            build_configuration_file = self.build_configurations_file(),
            build_directory = self.build_directory.as_path().display()
        );

        generate.data.push_str(&data);
        self.generate_default_all_target(generate, targets);
        Ok(())
    }

    fn generate_include_files(&self) -> Result<(), GeneratorError> {
        let include_output_directory = self.output_directory.join("make_include");
        let mut include_file_generator =
            IncludeFileGenerator::new(&include_output_directory, self.compiler.clone());

        for build_configuration in &self.build_configurations {
            match build_configuration {
                Configuration::Sanitizer(sanitizer) => {
                    include_file_generator.set_sanitizer(sanitizer)
                }
                Configuration::CppVersion(version) => {
                    include_file_generator.add_cpp_version(version);
                }
                _ => (),
            };
        }

        include_file_generator.generate_makefiles()
    }

    fn generate_object_rules(&self, generate: &mut Generate, targets: &[TargetNode]) {
        for target in targets {
            self.generate_object_rules_for_target(generate, target);
        }
        generate.data.push('\n');
    }

    fn generate_object_rules_for_target(&self, generate: &mut Generate, target: &TargetNode) {
        let mut formatted_string = String::new();
        let borrowed_target = target.borrow();
        let sources = borrowed_target
            .associated_files
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = &borrowed_target.manifest_dir_path;

        for source in sources {
            if !generate.source_files_generated_cache.contains(&source) {
                let source_file = source.file();
                let source_dir = source_file
                    .parent()
                    .and_then(|p| p.strip_prefix(dependency_root_path).ok());

                if let Some(dir) = source_dir {
                    self.create_subdir(dir).unwrap();
                }
                let object = {
                    if let Some(dir) = source_dir {
                        self.output_directory
                            .join(dir)
                            .join(source_file.file_name().unwrap())
                    } else {
                        self.output_directory.join(source_file.file_name().unwrap())
                    }
                }
                .with_extension("o");
                formatted_string.push_str(&format!("# Build rule for {}\n", object.display()));
                formatted_string.push_str(&object.display().to_string());
                formatted_string.push_str(": \\\n");
                formatted_string.push('\t');
                formatted_string.push_str(&source_file.display().to_string());
                formatted_string.push('\n');
                formatted_string.push_str(&format!(
                    "\t@echo \"Building CXX object {}\"\n",
                    object.display()
                ));
                formatted_string.push_str(&format!(
                    "\t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) \
         $(WARNINGS) {dependencies} $< -c -o $@)\n\n",
                    dependencies = self.generate_search_directories(target)
                ));
                if !generate.object_files_cached.contains(&object) {
                    generate.object_files_cached.insert(object);
                }
                generate.source_files_generated_cache.insert(source.clone());
            }
        }

        generate.data.push_str(formatted_string.trim_end());
    }

    fn generate_depends_rules(&self, generate: &mut Generate) {
        let depend_files = generate
            .object_files_cached
            .iter()
            .map(|object| {
                let mut object_clone = object.clone();
                object_clone.set_extension("d");
                object_clone
            })
            .collect::<Vec<std::path::PathBuf>>();

        generate.data.push('\n');
        for depend_file in depend_files {
            generate
                .data
                .push_str(&format!("# Silently include {}\n", depend_file.display()));
            generate
                .data
                .push_str(&format!("sinclude {}\n", depend_file.display()));
        }
    }

    fn search_directory_from_target(&self, target: &TargetNode) -> String {
        let borrowed_target = target.borrow();
        let project_base = &borrowed_target.manifest_dir_path;
        let include_line = format!("-I{} ", project_base.join("include").display().to_string());
        include_line
    }

    fn generate_search_directories(&self, target: &TargetNode) -> String {
        let borrowed_target = target.borrow();
        let mut formatted_string = String::new();
        formatted_string.push_str(&self.search_directory_from_target(target));
        if let Some(include_directories) = &borrowed_target.include_directories {
            for include in include_directories {
                if include.include_type == IncludeType::System {
                    formatted_string
                        .push_str(&format!("-isystem {}", include.path.display().to_string()))
                } else {
                    formatted_string.push_str(&format!("-I{}", include.path.display().to_string()))
                }
                formatted_string.push(' ');
            }
        }
        formatted_string.trim_end().to_string()
    }

    fn generate_rule_declaration_for_target(&self, generate: &mut Generate, target: &TargetNode) {
        self.generate_phony(generate, target);
        let target_rule_declaration = self.determine_rule_for_target(target);
        generate.data.push('\n');
        generate
            .data
            .push_str(&format!("# Rule for target {}\n", target.borrow().name()));
        generate.data.push_str(&target_rule_declaration);
        generate.data.push('\n');
        generate.data.push('\n');
    }

    fn determine_rule_for_target(&self, target: &TargetNode) -> String {
        let borrowed_target = target.borrow();
        if borrowed_target.is_executable() {
            return format!("\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking executable {target_name}\"\n\
                    \t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)",
                    target_name = borrowed_target.name(),
                    prerequisites = self.generate_prerequisites(target),
                    dependencies = self.generate_search_directories(target),
            );
        } else {
            return format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking static library {target_name}\"\n\
                    \t@$(strip $(AR) $(ARFLAGS) $@ $?)",
                target_name = borrowed_target.name(),
                prerequisites = self.generate_prerequisites(target)
            );
        }
    }

    fn generate_prerequisites(&self, target: &TargetNode) -> String {
        let mut formatted_string = String::new();
        let borrowed_target = target.borrow();
        let sources = borrowed_target
            .associated_files
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = &borrowed_target.manifest_dir_path;

        for source in sources {
            let source_file = source.file();
            let source_dir = source_file
                .parent()
                .and_then(|p| p.strip_prefix(dependency_root_path).ok())
                .unwrap();
            let object = self
                .output_directory
                .join(source_dir)
                .join(source_file.file_name().unwrap())
                .with_extension("o");
            formatted_string.push_str("\\\n");
            formatted_string.push_str(&format!("   {}", object.display().to_string()));
        }
        // formatted_string.push_str(&self.print_required_dependencies_libraries()?); // TODO: Need to think this one out.
        formatted_string
    }
}

impl Generator for MakefileGenerator {
    fn generate(&mut self, targets: &[TargetNode]) -> Result<(), GeneratorError> {
        self.generate_include_files()?;
        self.push_and_create_directory(&directory_from_build_configurations(
            &self.build_configurations,
        ))?;
        let mut generate = Generate::new(&self.output_directory.join("Makefile"))?;
        self.generate_header(&mut generate, targets)?;

        for target in targets {
            log::debug!(
                "Generating makefiles for target {:?} (manifest location: {})",
                target.borrow().name(),
                target.borrow().manifest_dir_path.display()
            );
            self.generate_rule_declaration_for_target(&mut generate, target);
        }
        self.generate_object_rules(&mut generate, targets);
        self.generate_depends_rules(&mut generate);
        generate.write()?;
        Ok(())
    }
}

struct Generate {
    file_handle: std::fs::File,
    data: String,
    source_files_generated_cache: std::collections::HashSet<SourceFile>,
    object_files_cached: std::collections::HashSet<std::path::PathBuf>,
}

impl Generate {
    pub fn new(path: &std::path::PathBuf) -> Result<Self, GeneratorError> {
        let file_handle = utility::create_file(path)?;
        Ok(Self {
            file_handle,
            data: String::new(),
            source_files_generated_cache: std::collections::HashSet::new(),
            object_files_cached: std::collections::HashSet::new(),
        })
    }

    pub fn write(&mut self) -> Result<(), FsError> {
        self.file_handle
            .write(self.data.as_bytes())
            .map_err(FsError::WriteToFile)?;
        Ok(())
    }
}

fn evaluate_compiler(
    compiler_constants: &mut std::collections::HashMap<&str, &str>,
    compiler: &Compiler,
) {
    match *compiler.compiler_type() {
        Type::Gcc => compiler_constants.insert("CXX_USES_GCC", "true"),
        Type::Clang => compiler_constants.insert("CXX_USES_CLANG", "true"),
    };
}

#[allow(dead_code)]
pub struct IncludeFileGenerator<'generator> {
    file: Option<File>,
    output_directory: std::path::PathBuf,
    args: HashMap<&'generator str, String>,
    compiler_constants: HashMap<&'generator str, &'generator str>,
}

#[allow(dead_code)]
impl<'generator> IncludeFileGenerator<'generator> {
    pub fn new(output_directory: &std::path::Path, compiler: Compiler) -> Self {
        utility::create_dir(&output_directory).unwrap();

        let mut compiler_constants = HashMap::new();
        compiler_constants.insert("CXX_USES_CLANG", "false");
        compiler_constants.insert("CXX_USES_GCC", "false");
        evaluate_compiler(&mut compiler_constants, &compiler);

        IncludeFileGenerator {
            file: None,
            output_directory: output_directory.to_path_buf(),
            args: HashMap::new(),
            compiler_constants,
        }
    }

    fn create_mk_file(&mut self, filename_prefix: &str) {
        let mut filename = std::path::PathBuf::from(filename_prefix);
        filename.set_extension("mk");
        let file =
            utility::create_file(&self.output_directory.join(filename.to_str().unwrap())).unwrap();
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

    fn generate_strict_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("strict");
        let data = indoc::formatdoc!("\
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
        ifeq ($(CXX_USES_GCC), true)
            CXXFLAGS += $(GLINUX_WARNINGS) \\
                        -Wmisleading-indentation \\
                        -Wduplicated-cond \\
                        -Wduplicated-branches \\
                        -Wlogical-op \\
                        -Wuseless-cast\n\
       \n\
       \n\
       else ifeq ($(CXX_USES_CLANG), true)
            CXXFLAGS += $(GLINUX_WARNINGS)\n\
       endif\n\
       \n\
       CXXFLAGS += {cpp_version}\n\
       \n\
       \n

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
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("strict.mk"), e))?;
        Ok(())
    }

    fn generate_debug_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("debug");
        let data = indoc::formatdoc!(
            "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.\n\
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        \n\
        {flags_sanitizer}

        # When building with sanitizer options, certain linker options must be added.\n\
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.\n\
        # This is done to support address space layout randomization (ASLR).\n\
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization\n\
        ",
            flags_sanitizer = self.generate_flags_sanitizer()
        );
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("debug.mk"), e))?;
        Ok(())
    }

    fn generate_release_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("release");
        let data = indoc::indoc!(
            "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O3\\
            -DNDEBUG\n
        "
        )
        .to_string();
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("release.mk"), e))?;
        Ok(())
    }

    fn generate_default_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("default_make");
        let data = indoc::indoc!("\
        #Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file\n\
        #excluding system header files.\n\
        CPPFLAGS +=-MMD\\
            -MP\n
        \n\
        CXXFLAGS += -pthread\n\
        \n\
        ARFLAGS = rs
        ").to_string();
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("default.mk"), e))?;
        Ok(())
    }

    fn generate_defines_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("defines");

        let data = indoc::formatdoc!(
            "\
        # Defines.mk\n\
        # Contains a number of defines determined from MyMake configuration time.\n\
        \n\
        {compiler_conditional_flags}\n\
        CP := /usr/bin/cp\n\
        CP_FORCE := -f\n\
        \n\
        ",
            compiler_conditional_flags = self.compiler_conditional_flags()
        );
        self.file
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(std::path::PathBuf::from("defines.mk"), e))?;
        Ok(())
    }

    fn compiler_conditional_flags(&mut self) -> String {
        let (gcc_key, gcc_value) = self
            .get_makefile_constant("CXX_USES_GCC")
            .unwrap_or((&"CXX_USES_GCC", &"true"));

        let (clang_key, clang_value) = self
            .get_makefile_constant("CXX_USES_CLANG")
            .unwrap_or((&"CXX_USES_CLANG", &"true"));

        format!(
            "{} := {}\n\
             {} := {}\n",
            gcc_key, gcc_value, clang_key, clang_value
        )
    }

    fn get_makefile_constant(&self, key: &str) -> Option<(&&str, &&str)> {
        self.compiler_constants.get_key_value(key)
    }
}

impl<'generator> UtilityGenerator<'generator> for IncludeFileGenerator<'generator> {
    fn generate_makefiles(&'generator mut self) -> Result<(), GeneratorError> {
        self.generate_strict_mk()?;
        self.generate_debug_mk()?;
        self.generate_default_mk()?;
        self.generate_defines_mk()?;
        self.generate_release_mk()
    }

    fn add_cpp_version(&mut self, version: &str) {
        self.args.insert("C++", version.to_string().to_lowercase());
    }

    fn print_cpp_version(&'generator self) -> &str {
        if self.args.contains_key("C++") {
            match self.args.get("C++").unwrap().as_str() {
                "c++98" => "-std=c++98",
                "c++03" => "-std=c++03",
                "c++11" => "-std=c++11",
                "c++14" => "-std=c++14",
                "c++17" => "-std=c++17",
                "c++20" => "-std=c++20",
                _ => "-std=c++20",
            }
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
    fn set_sanitizer(&mut self, sanitizer: &str) {
        let mut sanitizer_str = String::new();
        match sanitizer {
            "address" => sanitizer_str.push_str("address "), // sanitizer_str.push_str("address kernel-adress hwaddress pointer-compare pointer-subtract"),
            "thread" => sanitizer_str.push_str("thread -fPIE -pie "),
            "leak" => sanitizer_str.push_str("leak "),
            "undefined" => sanitizer_str.push_str("undefined "),
            _ => (),
        }
        self.args.insert("sanitizers", sanitizer_str);
    }
}

#[cfg(test)]
#[path = "./include_file_generator_test.rs"]
mod include_file_generator_test;
