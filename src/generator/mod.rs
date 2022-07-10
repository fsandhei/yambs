use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

mod generator;
mod include_file_generator;

use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory, Configuration};
use crate::compiler::Compiler;
use crate::dependency::{DependencyAccessor, DependencyNode, DependencyState, IncludeType};
use crate::errors::{DependencyError, FsError, GeneratorError};
use crate::utility;
pub use generator::{Generator, GeneratorExecutor, RuntimeSettings, Sanitizer, UtilityGenerator};
use include_file_generator::IncludeFileGenerator;

#[derive(PartialEq, Eq)]
enum GeneratorState {
    IncludeGenerated,
    IncludeNotGenerated,
}

pub struct MakefileGenerator {
    filename: Option<File>,
    dependency: Option<DependencyNode>,
    build_directory: PathBuf,
    output_directory: PathBuf,
    build_configurations: BuildConfigurations,
    state: GeneratorState,
    compiler: Compiler,
}

impl MakefileGenerator {
    pub fn new(build_directory: &BuildDirectory, compiler: Compiler) -> MakefileGenerator {
        let output_directory = build_directory.as_path();
        utility::create_dir(&output_directory).unwrap();

        MakefileGenerator {
            filename: None,
            dependency: None,
            build_directory: output_directory.to_path_buf(),
            output_directory: output_directory.to_path_buf(),
            build_configurations: BuildConfigurations::new(),
            state: GeneratorState::IncludeNotGenerated,
            compiler,
        }
    }

    fn generate_include_files(&mut self) -> Result<(), GeneratorError> {
        let include_output_directory = self.build_directory.join("make_include");
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

    pub fn replace_generator(
        &mut self,
        dependency: &DependencyNode,
        build_directory: std::path::PathBuf,
    ) {
        utility::create_dir(&build_directory).unwrap();
        self.set_dependency(dependency);
        self.output_directory = build_directory;
        self.create_makefile();
    }

    pub fn create_makefile(&mut self) {
        let filename = utility::create_file(&self.output_directory.join("makefile")).unwrap();
        self.filename = Some(filename);
    }

    fn use_subdir(&mut self, dir: std::path::PathBuf) -> Result<(), GeneratorError> {
        let new_output_dir = self.output_directory.join(dir);
        utility::create_dir(&new_output_dir)?;
        self.output_directory = new_output_dir;
        Ok(())
    }

    fn create_subdir(&self, dir: &std::path::Path) -> Result<(), GeneratorError> {
        utility::create_dir(&self.output_directory.join(dir)).map_err(GeneratorError::Fs)
    }

    fn get_required_project_lib_dir(&self) -> PathBuf {
        self.output_directory.join("libs")
    }

    fn is_debug_build(&self) -> bool {
        self.build_configurations.is_debug_build()
    }

    fn make_object_rule(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();

        let borrowed_dependency = self.get_dependency()?.dependency();
        let sources = borrowed_dependency
            .ref_dep
            .associated_files()
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = borrowed_dependency.ref_dep.get_parent_directory();

        for source in sources {
            let source_file = source.file();
            let source_dir = source_file
                .parent()
                .and_then(|p| p.strip_prefix(dependency_root_path).ok());

            if let Some(dir) = source_dir {
                self.create_subdir(&dir).unwrap();
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

            formatted_string.push_str(&object.display().to_string());
            formatted_string.push_str(": \\\n");
            formatted_string.push_str("\t");
            formatted_string.push_str(&source_file.display().to_string());
            formatted_string.push_str("\n");
            formatted_string.push_str(&format!("\t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) \
                                                        $(WARNINGS) {dependencies} $< -c -o $@)\n\n"
            , dependencies = self.print_dependencies()?));
        }

        Ok(formatted_string.trim_end().to_string())
    }

    fn print_header_includes(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.get_dependency()?.dependency();
        let sources = borrowed_dependency
            .ref_dep
            .associated_files()
            .iter()
            .filter(|file| file.is_source());

        for source in sources {
            let source_name = source.file();
            let include_file = self
                .output_directory
                .join(source_name.file_name().unwrap())
                .with_extension("d");

            formatted_string.push_str("sinclude ");
            formatted_string.push_str(&include_file.display().to_string());
            formatted_string.push_str("\n");
        }
        Ok(formatted_string)
    }

    fn print_required_dependencies_libraries(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        for dependency in self.get_dependency()?.dependency().ref_dep.requires() {
            if dependency.dependency().ref_dep.get_name().is_some() {
                let required_dep = dependency;
                let mut output_directory = self
                    .get_required_project_lib_dir()
                    .join(required_dep.dependency().ref_dep.get_project_name());
                if self.is_debug_build() {
                    output_directory = output_directory.join("debug");
                } else {
                    output_directory = output_directory.join("release");
                }
                let library_name = format!(
                    "lib{}.a",
                    &required_dep.dependency().ref_dep.library_file_name()
                );
                formatted_string.push_str("\t");
                formatted_string.push_str(&format!(
                    "{} \\\n",
                    output_directory.join(library_name).display()
                ));
            }
        }
        Ok(formatted_string)
    }

    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }

    fn library_path(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        let library_name = format!(
            "lib{}.a",
            &self
                .get_dependency()?
                .dependency()
                .ref_dep
                .library_file_name()
        );
        utility::print_full_path(
            &mut formatted_string,
            self.output_directory.to_str().unwrap(),
            &library_name,
            true,
        );

        Ok(formatted_string)
    }

    fn print_prerequisites(self: &Self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.get_dependency()?.dependency();
        let sources = borrowed_dependency
            .ref_dep
            .associated_files()
            .iter()
            .filter(|file| file.is_source());
        let dependency_root_path = borrowed_dependency.ref_dep.get_parent_directory();

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
            formatted_string.push_str("\\\n\t");
            formatted_string.push_str(&format!("{} ", object.display().to_string()));
        }
        formatted_string.push_str("\\\n");
        formatted_string.push_str(&self.print_required_dependencies_libraries()?);
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        Ok(formatted_string)
    }

    fn print_dependencies(&self) -> Result<String, GeneratorError> {
        let borrowed_dependency = self.get_dependency()?;
        let mut formatted_string = self.print_include_dependency_top()?;
        if let Some(include_directories) = borrowed_dependency
            .dependency()
            .ref_dep
            .include_directories()
        {
            for include in include_directories {
                if include.include_type == IncludeType::System {
                    formatted_string
                        .push_str(&format!("-isystem {}", include.path.display().to_string()))
                } else {
                    formatted_string.push_str(&format!("-I{}", include.path.display().to_string()))
                }
                formatted_string.push_str(" ");
            }
        }
        Ok(formatted_string.trim_end().to_string())
    }

    fn print_include_dependency_top(&self) -> Result<String, GeneratorError> {
        let dep = self.get_dependency()?.dependency();
        let project_base = utility::get_project_top_directory(&dep.ref_dep.path());
        let include_line = format!(
            "-I{} -I{} ",
            project_base.display(),
            project_base.join("include").display().to_string()
        );
        Ok(include_line)
    }

    fn print_release(&self) -> String {
        let release_include = format!(
            "{build_path}/make_include/release.mk",
            build_path = self.build_directory.to_str().unwrap()
        );
        release_include
    }

    fn print_debug(&self) -> String {
        if self.is_debug_build() {
            let debug_include = format!(
                "{build_path}/make_include/debug.mk",
                build_path = self.build_directory.to_str().unwrap()
            );
            debug_include
        } else {
            self.print_release()
        }
    }

    fn generate_header(self: &mut Self) -> Result<(), GeneratorError> {
        let data = format!(
            "\
        # Generated by MmkGenerator.generate_header(). DO NOT EDIT THIS FILE.\n\
        \n\
        # ----- INCLUDES -----\n\
        include {build_path}/make_include/strict.mk\n\
        include {build_path}/make_include/default_make.mk\n\
        include {debug}\n\
        \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean\n",
            debug = self.print_debug(),
            build_path = self.build_directory.to_str().unwrap()
        );

        self.filename
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|e| FsError::CreateFile(PathBuf::from("header.mk"), e))?;
        Ok(())
    }
}

impl GeneratorExecutor for MakefileGenerator {
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), GeneratorError> {
        if self.state == GeneratorState::IncludeNotGenerated {
            self.generate_include_files()?;
            self.state = GeneratorState::IncludeGenerated;
        }

        if !&dependency.dependency().ref_dep.is_makefile_made() {
            dependency
                .dependency_mut()
                .ref_dep
                .change_state(DependencyState::MakefileMade);
            self.generate_makefile()?;
        }

        let dependency_output_library_head = self.get_required_project_lib_dir();

        if !utility::directory_exists(&dependency_output_library_head) {
            utility::create_dir(&dependency_output_library_head)?;
        }

        for required_dependency in dependency.dependency().ref_dep.requires() {
            if !required_dependency.dependency().ref_dep.is_makefile_made() {
                required_dependency
                    .dependency_mut()
                    .ref_dep
                    .change_state(DependencyState::MakefileMade);
                let mut build_directory = dependency_output_library_head
                    .join(required_dependency.dependency().ref_dep.get_project_name());
                if self.is_debug_build() {
                    build_directory.push("debug");
                } else {
                    build_directory.push("release");
                }
                self.replace_generator(&required_dependency.clone(), build_directory);
                self.generate_makefile()?;
            }
            self.generate_makefiles(&required_dependency)?;
        }
        Ok(())
    }
}

impl Generator for MakefileGenerator {
    fn generate_makefile(self: &mut Self) -> Result<(), GeneratorError> {
        self.create_makefile();
        self.generate_header()?;
        self.generate_appending_flags()?;
        if self.get_dependency()?.dependency().ref_dep.is_executable() {
            self.generate_rule_executable()?;
        } else {
            self.generate_rule_package()?;
        }
        Ok(())
    }

    fn generate_rule_package(self: &mut Self) -> Result<(), GeneratorError> {
        let data = format!(
            "\n\
        #Generated by MmkGenerator.generate_rule_package(). \n\
        \n\
        {package}: {prerequisites}\n\
        \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ",
            prerequisites = self.print_prerequisites()?,
            package = self.library_path()?,
            sources_to_objects = self.make_object_rule()?,
            include_headers = self.print_header_includes()?
        );

        self.filename
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|_| GeneratorError::CreateRule)?;
        Ok(())
    }

    fn generate_rule_executable(self: &mut Self) -> Result<(), GeneratorError> {
        let data = format!(
            "\n\
        #Generated by MmkGenerator.generate_rule_executable(). \n\
        \n\
        .PHONY: {executable}\n\
        {executable}: {prerequisites}\n\
        \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ",
            executable = self
                .get_dependency()?
                .dependency()
                .ref_dep
                .get_name()
                .unwrap(),
            prerequisites = self.print_prerequisites()?,
            dependencies = self.print_dependencies()?,
            sources_to_objects = self.make_object_rule()?,
            include_headers = self.print_header_includes()?
        );

        self.filename
            .as_ref()
            .unwrap()
            .write(data.as_bytes())
            .map_err(|_| GeneratorError::CreateRule)?;
        Ok(())
    }

    fn generate_appending_flags(&mut self) -> Result<(), GeneratorError> {
        let mut data = String::new();
        let borrowed_dependency = self.get_dependency()?;
        if let Some(cxxflags) = borrowed_dependency
            .dependency()
            .ref_dep
            .additional_keyword("cxxflags")
        {
            data.push_str(&format!(
                "CXXFLAGS += {cxxflags_str}",
                cxxflags_str = cxxflags
                    .iter()
                    .map(|cxxflag| format!("{}\n", cxxflag.argument().to_string()))
                    .collect::<String>()
            ));
        }

        if let Some(cppflags) = borrowed_dependency
            .dependency()
            .ref_dep
            .additional_keyword("cppflags")
        {
            data.push_str(&format!(
                "CPPFLAGS += {cppflags_str}",
                cppflags_str = cppflags
                    .iter()
                    .map(|cppflag| format!("{}\n", cppflag.argument().to_string()))
                    .collect::<String>()
            ));
        }

        if !data.is_empty() {
            self.filename
                .as_ref()
                .unwrap()
                .write(data.as_bytes())
                .map_err(|_| GeneratorError::CreateRule)?;
        }
        Ok(())
    }
}

impl DependencyAccessor for MakefileGenerator {
    fn set_dependency(&mut self, dependency: &DependencyNode) {
        self.dependency = Some(dependency.clone());
    }

    fn get_dependency(&self) -> Result<&DependencyNode, DependencyError> {
        if let Some(dep) = &self.dependency {
            return Ok(dep);
        }
        return Err(DependencyError::NotSet);
    }
}

impl Sanitizer for MakefileGenerator {
    fn set_sanitizer(&mut self, sanitizer: &str) {
        let sanitizer_configuration = Configuration::Sanitizer(sanitizer.into());
        self.build_configurations
            .add_configuration(sanitizer_configuration);
    }
}

impl RuntimeSettings for MakefileGenerator {
    fn use_std(&mut self, version: &str) -> Result<(), GeneratorError> {
        self.build_configurations
            .add_configuration(Configuration::CppVersion(version.to_string()));
        Ok(())
    }

    fn debug(&mut self) {
        self.build_configurations
            .add_configuration(Configuration::Debug);
        self.use_subdir(std::path::PathBuf::from("debug")).unwrap();
    }

    fn release(&mut self) {
        self.build_configurations
            .add_configuration(Configuration::Release);
        if !self.is_debug_build() {
            self.use_subdir(std::path::PathBuf::from("release"))
                .unwrap();
        }
    }
}

#[cfg(test)]
#[path = "./mod_test.rs"]
mod lib_test;