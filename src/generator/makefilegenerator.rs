use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use indoc;

use crate::build_target;
use crate::build_target::include_directories;
use crate::build_target::{
    include_directories::IncludeType, target_registry::TargetRegistry, Dependency, LibraryType,
    TargetNode, TargetState,
};

use crate::cli::command_line;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::compiler::{Compiler, Type};
use crate::errors::FsError;
use crate::generator::{
    targets::ObjectTarget, Generator, GeneratorError, Sanitizer, UtilityGenerator,
};
use crate::utility;

struct ExecutableTargetFactory;

impl ExecutableTargetFactory {
    pub fn create_rule(target: &TargetNode, output_directory: &std::path::Path) -> String {
        format!("\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking executable {target_name}\"\n\
                    \t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)",
                    target_name = target.borrow().name(),
                    prerequisites = generate_prerequisites(target, output_directory),
                    dependencies = generate_search_directories(target),
            )
    }
}

struct LibraryTargetFactory;

impl LibraryTargetFactory {
    pub fn create_rule(target: &TargetNode, output_directory: &std::path::Path) -> String {
        match target.borrow().library_type().unwrap() {
            LibraryType::Static => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking static library {target_name}\"\n\
                    \t@$(strip $(AR) $(ARFLAGS) $@ $?)\n\n",
                target_name = target.borrow().name(),
                prerequisites = generate_prerequisites(target, output_directory)
            ),
            LibraryType::Dynamic => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking shared library {target_name}\"\n\
                    \t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ -o $@)\n",
                    target_name = target.borrow().name(),
                    prerequisites = generate_prerequisites(target, output_directory),
                    dependencies = generate_search_directories(target),
            )
        }
    }
}

struct TargetRuleFactory;

impl TargetRuleFactory {
    pub fn create_rule(target: &TargetNode, output_dir: &std::path::Path) -> String {
        if target.borrow().is_executable() {
            ExecutableTargetFactory::create_rule(target, output_dir)
        } else {
            LibraryTargetFactory::create_rule(target, output_dir)
        }
    }
}

fn generate_prerequisites(target: &TargetNode, output_directory: &std::path::Path) -> String {
    let mut formatted_string = String::new();
    let borrowed_target = target.borrow();
    match borrowed_target.target_source {
        build_target::TargetSource::FromSource(ref source_data) => {
            let sources = source_data
                .source_files
                .iter()
                .filter(|file| file.is_source());
            let dependency_root_path = &source_data.manifest.directory;

            for source in sources {
                let source_file = source.file();
                let source_dir = source_file
                    .parent()
                    .and_then(|p| p.strip_prefix(dependency_root_path).ok())
                    .unwrap();
                let object = output_directory
                    .join(source_dir)
                    .join(source_file.file_name().unwrap())
                    .with_extension("o");
                formatted_string.push_str("\\\n");
                formatted_string.push_str(&format!("   {}", object.display().to_string()));
            }
            for dependency in &source_data.dependencies {
                formatted_string.push_str("\\\n");
                match dependency.source {
                    build_target::DependencySource::FromSource(ref s) => {
                        formatted_string.push_str(&format!("   {}", s.name));
                    }
                }
            }
        }
    }
    formatted_string
}

fn generate_search_directories(target: &TargetNode) -> String {
    let borrowed_target = target.borrow();
    let mut formatted_string = String::new();
    formatted_string.push_str(&generate_include_directories(
        &borrowed_target.include_directories,
    ));
    match borrowed_target.target_source {
        build_target::TargetSource::FromSource(ref source_data) => {
            if !source_data.dependencies.is_empty() {
                formatted_string.push_str("-L.");
            }
        }
    };
    formatted_string.trim_end().to_string()
}

fn generate_include_directories(
    include_directories: &include_directories::IncludeDirectories,
) -> String {
    let mut formatted_string = String::new();
    for include in include_directories {
        if include.include_type == IncludeType::System {
            formatted_string.push_str(&format!("-isystem {}", include.path.display().to_string()))
        } else {
            formatted_string.push_str(&format!("-I{}", include.path.display().to_string()))
        }
        formatted_string.push(' ');
    }
    formatted_string.trim_end().to_string()
}

fn generate_object_target(object_target: &ObjectTarget) -> String {
    let mut formatted_string = String::new();
    formatted_string.push_str(&format!(
        "# Build rule for {}\n",
        object_target.object.display()
    ));
    formatted_string.push_str(&object_target.object.display().to_string());
    formatted_string.push_str(": \\\n");
    formatted_string.push('\t');
    formatted_string.push_str(&object_target.source.display().to_string());
    formatted_string.push('\n');
    formatted_string.push_str(&format!(
        "\t@echo \"Building CXX object {}\"\n",
        object_target.object.display()
    ));
    formatted_string.push_str(&format!(
        "\t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) \
         $(WARNINGS) {dependencies} $< -c -o $@)\n\n",
        dependencies = generate_include_directories(&object_target.include_directories),
    ));
    formatted_string
}

fn directory_from_build_configuration(
    build_type: &configurations::BuildType,
) -> std::path::PathBuf {
    std::path::PathBuf::from(build_type.to_string())
}

pub struct MakefileGenerator {
    pub compiler: Compiler,
    pub configurations: command_line::ConfigurationOpts,
    pub build_directory: BuildDirectory,
    pub output_directory: std::path::PathBuf,
}

impl MakefileGenerator {
    pub fn new(
        configurations: &command_line::ConfigurationOpts,
        build_directory: &BuildDirectory,
        compiler: Compiler,
    ) -> Result<Self, GeneratorError> {
        utility::create_dir(&build_directory.as_path())?;
        Ok(Self {
            compiler,
            configurations: configurations.to_owned(),
            build_directory: build_directory.clone(),
            output_directory: build_directory.as_path().to_path_buf(),
        })
    }

    fn create_object_targets(&self, target: &TargetNode) -> Vec<ObjectTarget> {
        let mut object_targets = Vec::new();
        let borrowed_target = target.borrow();
        let source_data = borrowed_target.target_source.from_source().unwrap();
        let sources = source_data
            .source_files
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = &source_data.manifest.directory;

        for source in sources {
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
            let object_target = ObjectTarget {
                object,
                source: source_file,
                include_directories: borrowed_target.include_directories.clone(),
            };

            object_targets.push(object_target);
        }
        object_targets
    }

    fn generate_makefile(
        &mut self,
        generate: &mut Generate,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        self.generate_header(generate, &registry.registry)?;

        for target in &registry.registry {
            if target.borrow().state != TargetState::BuildFileMade {
                let borrowed_target = target.borrow();
                match borrowed_target.target_source {
                    build_target::TargetSource::FromSource(ref s) => {
                        log::debug!(
                            "Generating makefiles for target {:?} (manifest path: {})",
                            target.borrow().name(),
                            s.manifest.directory.display()
                        );
                        self.generate_rule_declaration_for_target(generate, target);
                        self.generate_rule_for_dependencies_from_source_data(
                            generate,
                            &borrowed_target.name(),
                            s,
                            registry,
                        )?;

                        self.create_object_targets(target)
                            .iter()
                            .for_each(|object_target| {
                                if !generate.object_targets.contains(object_target) {
                                    generate.object_targets.push(object_target.clone());
                                }
                            });
                    }
                }
            }
            target.borrow_mut().state = TargetState::BuildFileMade;
        }
        self.generate_object_rules(generate);
        self.generate_depends_rules(generate);
        Ok(())
    }

    fn generate_rule_for_dependencies_from_source_data(
        &mut self,
        generate: &mut Generate,
        target_name: &str,
        source_data: &build_target::SourceBuildData,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        if !source_data.dependencies.is_empty() {
            let dependencies = &source_data.dependencies;
            for dependency in dependencies {
                match dependency.source {
                    build_target::DependencySource::FromSource(ref s) => {
                        log::debug!("Generating build rule for dependency \"{}\" (manifest path = {}) to target \"{}\" (manifest path {})",
                            s.name,
                            s.manifest.directory.display(),
                            target_name,
                            source_data.manifest.directory.display());
                        if s.manifest.directory != source_data.manifest.directory {
                            self.push_and_create_directory(std::path::Path::new("lib"))?;
                            self.generate_rule_for_dependency(generate, dependency, registry);
                            self.output_directory.pop();
                        } else {
                            self.generate_rule_for_dependency(generate, dependency, registry);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_rule_for_dependency(
        &self,
        generate: &mut Generate,
        dependency: &Dependency,
        registry: &TargetRegistry,
    ) {
        let dependency_target = dependency.to_build_target(registry).unwrap();

        if dependency_target.borrow().state != TargetState::BuildFileMade {
            let rule =
                LibraryTargetFactory::create_rule(&dependency_target, &self.output_directory);
            self.create_object_targets(&dependency_target)
                .iter()
                .for_each(|object_target| {
                    if !generate.object_targets.contains(object_target) {
                        generate.object_targets.push(object_target.clone());
                    }
                });
            generate.data.push_str(&rule);
            dependency_target.borrow_mut().state = TargetState::BuildFileMade;
        }
    }

    fn build_configurations_file(&self) -> &str {
        if self.configurations.build_type == configurations::BuildType::Debug {
            "debug.mk"
        } else {
            "release.mk"
        }
    }

    fn push_and_create_directory(&mut self, dir: &std::path::Path) -> Result<(), GeneratorError> {
        self.output_directory.push(dir);
        Ok(match std::fs::create_dir(&self.output_directory) {
            s @ Ok(()) => s,
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
        .map_err(|err| FsError::CreateDirectory(self.output_directory.clone(), err))?)
    }

    fn create_subdir(&self, dir: &std::path::Path) -> Result<(), GeneratorError> {
        utility::create_dir(&self.output_directory.join(dir)).map_err(GeneratorError::Fs)
    }

    fn generate_default_all_target(&self, generate: &mut Generate, targets: &[TargetNode]) {
        let targets_as_string = {
            let mut targets_as_string = String::new();
            for target in targets {
                targets_as_string.push_str("\\\n");
                targets_as_string.push_str(&format!("   {}", target.borrow().name()))
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

    fn generate_phony(&self, generate: &mut Generate, target: &TargetNode) {
        let data = indoc::formatdoc!(
            "\n
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

        let cxx_standard = &self.configurations.cxx_standard.to_string();
        include_file_generator.add_cpp_version(cxx_standard);
        if let Some(sanitizer) = &self.configurations.sanitizer {
            include_file_generator.set_sanitizer(&sanitizer.to_string());
        }
        include_file_generator.generate_makefiles()
    }

    fn generate_object_rules(&self, generate: &mut Generate) {
        for object_target in &generate.object_targets {
            generate
                .data
                .push_str(&generate_object_target(object_target))
        }
    }

    fn generate_depends_rules(&self, generate: &mut Generate) {
        let depend_files = generate
            .object_targets
            .iter()
            .map(|object_target| {
                let mut object_clone = object_target.object.clone();
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

    fn generate_rule_declaration_for_target(&self, generate: &mut Generate, target: &TargetNode) {
        self.generate_phony(generate, target);
        let target_rule_declaration =
            TargetRuleFactory::create_rule(target, &self.output_directory);
        generate.data.push('\n');
        generate.data.push_str(&format!(
            "# Rule for target \"{}\"\n",
            target.borrow().name()
        ));
        generate.data.push_str(&target_rule_declaration);
        generate.data.push('\n');
        generate.data.push('\n');
    }
}

impl Generator for MakefileGenerator {
    fn generate(&mut self, registry: &TargetRegistry) -> Result<(), GeneratorError> {
        self.generate_include_files()?;
        self.push_and_create_directory(&directory_from_build_configuration(
            &self.configurations.build_type,
        ))?;
        let mut generate = Generate::new(&self.output_directory.join("Makefile"))?;
        self.generate_makefile(&mut generate, registry)?;
        generate.write()?;
        Ok(())
    }
}

struct Generate {
    file_handle: std::fs::File,
    data: String,
    object_targets: Vec<ObjectTarget>,
}

impl Generate {
    pub fn new(path: &std::path::PathBuf) -> Result<Self, GeneratorError> {
        let file_handle = utility::create_file(path)?;
        Ok(Self {
            file_handle,
            data: String::new(),
            object_targets: Vec::new(),
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

pub struct IncludeFileGenerator<'generator> {
    file: Option<File>,
    output_directory: std::path::PathBuf,
    args: HashMap<&'generator str, String>,
    compiler_constants: HashMap<&'generator str, &'generator str>,
}

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

    fn generate_strict_mk(&mut self) -> Result<(), GeneratorError> {
        self.create_mk_file("strict");
        let data = indoc::formatdoc!("\
        #Generated by IncludeFileGenerator.generate_strict_mk. DO NOT EDIT.

        include {def_directory}/defines.mk


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
                          -Wformat=2


        ifeq ($(CXX_USES_GCC), true)
            CXXFLAGS += $(GLINUX_WARNINGS) \\
                        -Wmisleading-indentation \\
                        -Wduplicated-cond \\
                        -Wduplicated-branches \\
                        -Wlogical-op \\
                        -Wuseless-cast


       else ifeq ($(CXX_USES_CLANG), true)
            CXXFLAGS += $(GLINUX_WARNINGS)
       endif

       CXXFLAGS += {cpp_version}

        #-Wall                     # Reasonable and standard
        #-Wextra                   # Warn if indentation implies blocks where blocks do not exist.
        #-Wmisleading-indentation  # Warn if if / else chain has duplicated conditions
        #-Wduplicated-cond         # Warn if if / else branches has duplicated conditions
        #-Wduplicated-branches     # warn the user if a variable declaration shadows one from a parent context
        #-Wshadow                  # warn the user if a class with virtual functions has a non-virtual destructor. This helps
        #-Wnon-virtual-dtor        # catch hard to track down memory errors
        #-Wold-style-cast          # warn for C-style casts
        #-Wcast-align              # warn for potential performance problem casts
        #-Wunused                  # warn on anything being unused
        #-Woverloaded-virtual      # warn if you overload (not override) a virtual function
        #-Wpedantic                # warn if non-standard C++ is used
        #-Wconversion              # warn on type conversions that may lose data
        #-Wsign-conversion         # warn on sign conversions
        #-Wnull-dereference        # warn if a null dereference is detected
        #-Wdouble-promotion        # warn if float is implicit promoted to double
        #-Wformat=2                # warn on security issues around functions that format output (ie printf)
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
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        {flags_sanitizer}

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
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
                    -DNDEBUG
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
        let data = indoc::indoc!(
            "\
        # Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file
        # excluding system header files.
        CPPFLAGS +=-MMD\\
                   -MP

        # Additional CXX flags to be passed to the compiler
        CXXFLAGS += -pthread\\
                    -fPIC # Generate Position Independent code suitable for use in a shared library.

        # Additional AR flags being passed to the static library linker
        ARFLAGS = rs
        "
        )
        .to_string();
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
mod tests {
    use std::fs;

    use crate::tests::EnvLock;
    use pretty_assertions::assert_eq;
    use tempdir::TempDir;

    use super::*;

    fn produce_include_path(base_dir: TempDir) -> std::path::PathBuf {
        let build_dir = std::path::PathBuf::from(".build");
        let output_directory = base_dir.path().join(build_dir).join("make_include");
        output_directory
    }

    fn construct_generator<'generator>(path: &std::path::Path) -> IncludeFileGenerator<'generator> {
        IncludeFileGenerator::new(path, crate::compiler::Compiler::new().unwrap())
    }

    #[test]
    fn add_cpp_version_cpp98_test() -> Result<(), GeneratorError> {
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++98");
        assert_eq!(gen.args["C++"], "c++98");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp11_test() -> Result<(), GeneratorError> {
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++11");
        assert_eq!(gen.args["C++"], "c++11");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp14_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++14");
        assert_eq!(gen.args["C++"], "c++14");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_uppercase_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("C++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp20_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++20");
        assert_eq!(gen.args["C++"], "c++20");
        Ok(())
    }

    #[test]
    fn generate_strict_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("strict.mk");
        gen.generate_strict_mk().unwrap();
        assert_eq!(format!(indoc::indoc!("\
        #Generated by IncludeFileGenerator.generate_strict_mk. DO NOT EDIT.

        include {def_directory}/defines.mk


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
                          -Wformat=2


        ifeq ($(CXX_USES_GCC), true)
            CXXFLAGS += $(GLINUX_WARNINGS) \\
                        -Wmisleading-indentation \\
                        -Wduplicated-cond \\
                        -Wduplicated-branches \\
                        -Wlogical-op \\
                        -Wuseless-cast


       else ifeq ($(CXX_USES_CLANG), true)
            CXXFLAGS += $(GLINUX_WARNINGS)
       endif

       CXXFLAGS += -std=c++20

        #-Wall                     # Reasonable and standard
        #-Wextra                   # Warn if indentation implies blocks where blocks do not exist.
        #-Wmisleading-indentation  # Warn if if / else chain has duplicated conditions
        #-Wduplicated-cond         # Warn if if / else branches has duplicated conditions
        #-Wduplicated-branches     # warn the user if a variable declaration shadows one from a parent context
        #-Wshadow                  # warn the user if a class with virtual functions has a non-virtual destructor. This helps
        #-Wnon-virtual-dtor        # catch hard to track down memory errors
        #-Wold-style-cast          # warn for C-style casts
        #-Wcast-align              # warn for potential performance problem casts
        #-Wunused                  # warn on anything being unused
        #-Woverloaded-virtual      # warn if you overload (not override) a virtual function
        #-Wpedantic                # warn if non-standard C++ is used
        #-Wconversion              # warn on type conversions that may lose data
        #-Wsign-conversion         # warn on sign conversions
        #-Wnull-dereference        # warn if a null dereference is detected
        #-Wdouble-promotion        # warn if float is implicit promoted to double
        #-Wformat=2                # warn on security issues around functions that format output (ie printf)
        "),
        def_directory = gen.print_build_directory()), fs::read_to_string(file_name.to_str().unwrap()).unwrap());
        Ok(())
    }

    #[test]
    fn generate_debug_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf
        
        \n
        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_debug_mk_with_address_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.set_sanitizer("address");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        CXXFLAGS += -fsanitize=address 

        LDFLAGS += -fsanitize=address 

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_debug_mk_with_thread_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");

        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("debug.mk");
        gen.set_sanitizer("thread");
        gen.generate_debug_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_debug_mk. DO NOT EDIT.
        CXXFLAGS += -g \\
                    -O0 \\
                    -gdwarf

        CXXFLAGS += -fsanitize=thread -fPIE -pie 

        LDFLAGS += -fsanitize=thread -fPIE -pie 

        # When building with sanitizer options, certain linker options must be added.
        # For thread sanitizers, -fPIE and -pie will be added to linker and C++ flag options.
        # This is done to support address space layout randomization (ASLR).
        # PIE enables C++ code to be compiled and linked as position-independent code.
        # https://en.wikipedia.org/wiki/Address_space_layout_randomization
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_release_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");

        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("release.mk");
        gen.generate_release_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        #Generated by IncludeFileGenerator.generate_release_mk. DO NOT EDIT.\n\
        CXXFLAGS += -O3\\
                    -DNDEBUG
        "
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_default_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("default_make.mk");
        gen.generate_default_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
        # Automatic dependency generation: Makes GCC generate the dependencies needed for a cpp file
        # excluding system header files.
        CPPFLAGS +=-MMD\\
                   -MP
       
        # Additional CXX flags to be passed to the compiler
        CXXFLAGS += -pthread\\
                    -fPIC # Generate Position Independent code suitable for use in a shared library.

        # Additional AR flags being passed to the static library linker
        ARFLAGS = rs\n"
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_no_sanitizers_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let gen = construct_generator(&output_directory);
        let actual = gen.generate_flags_sanitizer();
        let expected = String::new();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_address_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());

        let mut gen = construct_generator(&output_directory);
        gen.set_sanitizer("address");
        let actual = gen.generate_flags_sanitizer();
        let expected = indoc::indoc!(
            "\
            CXXFLAGS += -fsanitize=address 

            LDFLAGS += -fsanitize=address ",
        );
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_flags_sanitizer_thread_sanitizer_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.set_sanitizer("thread");
        let actual = gen.generate_flags_sanitizer();
        let expected = indoc::indoc!(
            "\
            CXXFLAGS += -fsanitize=thread -fPIE -pie 

            LDFLAGS += -fsanitize=thread -fPIE -pie ",
        );
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn generate_defines_mk_test() -> std::io::Result<()> {
        let mut lock = EnvLock::new();
        lock.lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        let file_name = output_directory.join("defines.mk");
        gen.generate_defines_mk().unwrap();
        assert_eq!(
            indoc::indoc!(
                "\
    # Defines.mk\n\
    # Contains a number of defines determined from MyMake configuration time.\n\
    \n\
    CXX_USES_GCC := true\n\
    CXX_USES_CLANG := false\n\
    \n\
    CP := /usr/bin/cp\n\
    CP_FORCE := -f\n\
    \n"
            ),
            fs::read_to_string(file_name.to_str().unwrap()).unwrap()
        );
        Ok(())
    }

    #[test]
    fn evaluate_compiler_with_gcc_results_in_gcc_set() {
        let mut lock = EnvLock::new();
        let output_directory = produce_include_path(TempDir::new("example").unwrap());

        {
            lock.lock("CXX", "gcc");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }

        {
            lock.lock("CXX", "/usr/bin/gcc");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }

        {
            lock.lock("CXX", "g++");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }
        {
            lock.lock("CXX", "/usr/bin/g++");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }
    }

    #[test]
    fn evaluate_compiler_with_clang_results_in_clang_set() {
        let mut lock = EnvLock::new();
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        {
            lock.lock("CXX", "clang");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "false");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "true");
        }
        {
            lock.lock("CXX", "/usr/bin/clang");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "false");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "true");
        }
    }
}
