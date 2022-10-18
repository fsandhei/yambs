use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use indoc;

use crate::build_target::{
    associated_files::SourceFile, include_directories::IncludeType,
    target_registry::TargetRegistry, LibraryType, TargetNode,
};
use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::compiler::{Compiler, Type};
use crate::errors::FsError;
use crate::generator::{Generator, GeneratorError, Sanitizer, UtilityGenerator};
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
        match target.borrow().library_type() {
            LibraryType::Static => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking static library {target_name}\"\n\
                    \t@$(strip $(AR) $(ARFLAGS) $@ $?)",
                target_name = target.borrow().name(),
                prerequisites = generate_prerequisites(target, output_directory)
            ),
            LibraryType::Dynamic => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t@echo \"Linking shared library {target_name}\"\n\
                    \t@$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ -o $@)",
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
    let sources = borrowed_target
        .source_files
        .iter()
        .filter(|file| file.is_source());
    let dependency_root_path = &borrowed_target.manifest.directory;

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
    for dependency in &borrowed_target.dependencies {
        formatted_string.push_str("\\\n");
        formatted_string.push_str(&format!("   {}", dependency.name));
    }
    formatted_string
}

fn generate_search_directories(target: &TargetNode) -> String {
    let borrowed_target = target.borrow();
    let mut formatted_string = String::new();
    formatted_string.push_str(&search_directory_from_target(target));
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
    if !borrowed_target.dependencies.is_empty() {
        formatted_string.push_str("-L.");
    }
    formatted_string.trim_end().to_string()
}

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

fn search_directory_from_target(target: &TargetNode) -> String {
    let borrowed_target = target.borrow();
    let project_base = &borrowed_target.manifest.directory;
    let include_line = format!("-I{} ", project_base.join("include").display().to_string());
    include_line
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

    fn generate_makefile(
        &mut self,
        generate: &mut Generate,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        self.generate_header(generate, &registry.registry)?;

        for target in &registry.registry {
            log::debug!(
                "Generating makefiles for target {:?} (manifest path: {})",
                target.borrow().name(),
                target.borrow().manifest.directory.display()
            );
            self.generate_rule_declaration_for_target(generate, target);
            if !target.borrow().dependencies.is_empty() {
                self.push_and_create_directory(&std::path::Path::new("lib"))?;
                let dependencies = &target.borrow().dependencies;
                for dependency in dependencies {
                    if dependency.manifest_dir_path != target.borrow().manifest.directory {
                        log::debug!("Generating build rule for dependency \"{}\" (manifest path = {}) to target \"{}\" (manifest path {})",
                            dependency.name,
                            dependency.manifest_dir_path.display(),
                            target.borrow().name(),
                            target.borrow().manifest.directory.display());
                        let dependency_target = dependency.to_build_target(registry).unwrap();
                        let rule = LibraryTargetFactory::create_rule(
                            &dependency_target,
                            &self.output_directory,
                        );
                        generate.data.push_str(&rule);
                    }
                }
                self.output_directory.pop();
            }
        }
        self.generate_object_rules(generate, &registry.registry);
        self.generate_depends_rules(generate);
        Ok(())
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

    fn generate_phony(&self, generate: &mut Generate, target: &TargetNode) -> () {
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
            .source_files
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = &borrowed_target.manifest.directory;

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
                    dependencies = generate_search_directories(target)
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
        self.push_and_create_directory(&directory_from_build_configurations(
            &self.build_configurations,
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

    fn construct_generator<'generator>(
        path: &std::path::PathBuf,
    ) -> IncludeFileGenerator<'generator> {
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
