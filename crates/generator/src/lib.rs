mod generator;
pub mod generator_mock;
mod include_file_generator;

pub use crate::generator::{
    Generator, GeneratorExecutor, RuntimeSettings, Sanitizer, UtilityGenerator,
};

use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::rc::Rc;

use cli::build_configurations::{BuildConfigurations, Configuration};
use dependency::{DependencyAccessor, DependencyNode};
use error::GeneratorError;
use include_file_generator::IncludeFileGenerator;
use mmk_parser::Toolchain;
use utility;

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
    toolchain: Toolchain,
}

impl MakefileGenerator {
    pub fn new(build_directory: std::path::PathBuf, toolchain: &Toolchain) -> MakefileGenerator {
        let output_directory = build_directory;
        utility::create_dir(&output_directory).unwrap();

        MakefileGenerator {
            filename: None,
            dependency: None,
            build_directory: output_directory.clone(),
            output_directory,
            build_configurations: BuildConfigurations::new(),
            state: GeneratorState::IncludeNotGenerated,
            toolchain: toolchain.clone(),
        }
    }

    fn generate_include_files(&mut self) -> Result<(), error::GeneratorError> {
        let include_output_directory = self.build_directory.join("make_include");
        let mut include_file_generator =
            IncludeFileGenerator::new(&include_output_directory, &self.toolchain);

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
        let filename = utility::create_file(&self.output_directory, "makefile").unwrap();
        self.filename = Some(filename);
    }

    fn use_subdir(&mut self, dir: std::path::PathBuf) -> Result<(), GeneratorError> {
        let new_output_dir = self.output_directory.join(dir);
        utility::create_dir(&new_output_dir)?;
        self.output_directory = new_output_dir;
        Ok(())
    }

    fn create_subdir(&self, dir: std::path::PathBuf) -> Result<(), GeneratorError> {
        utility::create_dir(&self.output_directory.join(dir)).map_err(GeneratorError::Fs)
    }

    fn get_required_project_lib_dir(&self) -> PathBuf {
        self.output_directory.join("libs")
    }

    fn is_debug_build(&self) -> bool {
        self.build_configurations.is_debug_build()
    }

    fn make_object_rule(&self, mmk_data: &mmk_parser::Mmk) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();

        let borrowed_dependency = self.get_dependency()?.borrow();

        if mmk_data.data().contains_key("MMK_SOURCES") {
            let mut object = String::new();
            for source in &mmk_data.data()["MMK_SOURCES"] {
                let source_name = source.argument();
                if let Some(source_path) = mmk_data.source_file_path(source_name) {
                    self.create_subdir(source_path).unwrap();
                }

                if source_name.ends_with(".cpp") {
                    object = source_name.replace(".cpp", ".o");
                }
                if source_name.ends_with(".cc") {
                    object = source_name.replace(".cc", ".o");
                }

                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&object);
                formatted_string.push_str(": \\\n");
                formatted_string.push_str("\t");
                formatted_string
                    .push_str(borrowed_dependency.get_parent_directory().to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(source_name);
                formatted_string.push_str("\n");
                formatted_string.push_str(&format!("\t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) \
                                                          $(WARNINGS) {dependencies} $< -c -o $@)\n\n"
                , dependencies = self.print_dependencies()?));
            }
        }
        Ok(formatted_string.trim_end().to_string())
    }

    fn print_header_includes(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        let borrowed_dependency = self.get_dependency()?.borrow();
        let mmk_data = borrowed_dependency.mmk_data();
        let mut include_file = String::new();
        if mmk_data.data().contains_key("MMK_SOURCES") {
            for source in &mmk_data.data()["MMK_SOURCES"] {
                let source_name = source.argument();
                if source_name.ends_with(".cpp") {
                    include_file = source_name.replace(".cpp", ".d");
                }
                if source_name.ends_with(".cc") {
                    include_file = source_name.replace(".cc", ".d");
                }

                formatted_string.push_str("sinclude ");
                formatted_string.push_str(self.output_directory.to_str().unwrap());
                formatted_string.push_str("/");
                formatted_string.push_str(&include_file);
                formatted_string.push_str("\n");
            }
        }
        Ok(formatted_string)
    }

    fn print_required_dependencies_libraries(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        for dependency in self.get_dependency()?.borrow().requires().borrow().iter() {
            if dependency.borrow().library_name() != "" {
                let required_dep = dependency.borrow();
                let mut output_directory = self
                    .get_required_project_lib_dir()
                    .join(required_dep.get_project_name());
                if self.is_debug_build() {
                    output_directory = output_directory.join("debug");
                } else {
                    output_directory = output_directory.join("release");
                }
                formatted_string.push_str("\t");
                utility::print_full_path(
                    &mut formatted_string,
                    output_directory.to_str().unwrap(),
                    &required_dep.library_file_name(),
                    false,
                );
            }
        }
        Ok(formatted_string)
    }

    pub fn print_mandatory_libraries(self: &Self) -> String {
        let mut formatted_string = String::new();
        formatted_string.push_str("-lstdc++");
        formatted_string
    }

    fn print_library_name(&self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        utility::print_full_path(
            &mut formatted_string,
            self.output_directory.to_str().unwrap(),
            &self.get_dependency()?.borrow().library_file_name(),
            true,
        );

        Ok(formatted_string)
    }

    fn print_prerequisites(self: &Self) -> Result<String, GeneratorError> {
        let mut formatted_string = String::new();
        let mut object = String::new();
        let borrowed_dependency = self.get_dependency()?.borrow();
        if borrowed_dependency
            .mmk_data()
            .data()
            .contains_key("MMK_SOURCES")
        {
            formatted_string.push_str("\\\n");
            for source in &borrowed_dependency.mmk_data().data()["MMK_SOURCES"] {
                let source_name = source.argument();
                if source_name.ends_with(".cpp") {
                    object = source_name.replace(".cpp", ".o");
                }
                if source_name.ends_with(".cc") {
                    object = source_name.replace(".cc", ".o");
                }
                formatted_string.push_str("\t");
                utility::print_full_path(
                    &mut formatted_string,
                    self.output_directory.to_str().unwrap(),
                    &object,
                    false,
                );
            }
        }
        formatted_string.push_str(&self.print_required_dependencies_libraries()?);
        formatted_string.push_str("\t");
        formatted_string.push_str(&self.print_mandatory_libraries());
        Ok(formatted_string)
    }

    fn print_dependencies(&self) -> Result<String, GeneratorError> {
        let borrowed_dependency = self.get_dependency()?.borrow();
        let mut formatted_string = self.print_include_dependency_top()?;
        formatted_string.push_str(
            &borrowed_dependency
                .mmk_data()
                .get_include_directories()
                .unwrap(),
        );
        if borrowed_dependency.mmk_data().has_system_include() {
            formatted_string.push_str(" ");
            formatted_string.push_str(&borrowed_dependency.mmk_data().to_string("MMK_SYS_INCLUDE"));
        }

        Ok(formatted_string)
    }

    fn print_include_dependency_top(&self) -> Result<String, GeneratorError> {
        let include_line = format!(
            "-I{} ",
            utility::get_project_top_directory(self.get_dependency()?.borrow().path())
                .to_str()
                .unwrap()
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
            .map_err(|e| error::FsError::CreateFile(PathBuf::from("header.mk"), e))?;
        Ok(())
    }
}

impl GeneratorExecutor for MakefileGenerator {
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), GeneratorError> {
        if self.state == GeneratorState::IncludeNotGenerated {
            self.generate_include_files()?;
            self.state = GeneratorState::IncludeGenerated;
        }

        if !&dependency.borrow().is_makefile_made() {
            dependency.borrow_mut().makefile_made();
            self.generate_makefile()?;
        }

        let dependency_output_library_head = self.get_required_project_lib_dir();

        if !utility::directory_exists(&dependency_output_library_head) {
            utility::create_dir(&dependency_output_library_head)?;
        }

        for required_dependency in dependency.borrow().requires().borrow().iter() {
            if !required_dependency.borrow().is_makefile_made() {
                required_dependency.borrow_mut().makefile_made();
                let mut build_directory = dependency_output_library_head
                    .join(required_dependency.borrow().get_project_name());
                if self.is_debug_build() {
                    build_directory.push("debug");
                } else {
                    build_directory.push("release");
                }
                self.replace_generator(&Rc::clone(required_dependency), build_directory);
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
        if self.get_dependency()?.borrow().mmk_data().has_executables() {
            self.generate_rule_executable()?;
        } else {
            self.generate_rule_package()?;
        }
        self.print_ok();
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
            package = self.print_library_name()?,
            sources_to_objects =
                self.make_object_rule(&self.get_dependency()?.borrow().mmk_data())?,
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
        \t$(strip $(CC) $(CXXFLAGS) $(CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)\n\
        \n\
        {sources_to_objects}\n\
        \n\
        {include_headers}\n\
        ",
            executable = self
                .get_dependency()?
                .borrow()
                .mmk_data()
                .to_string("MMK_EXECUTABLE"),
            prerequisites = self.print_prerequisites()?,
            dependencies = self.print_dependencies()?,
            sources_to_objects =
                self.make_object_rule(&self.get_dependency()?.borrow().mmk_data())?,
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
        let borrowed_dependency = self.get_dependency()?.borrow();
        if borrowed_dependency
            .mmk_data()
            .data()
            .contains_key("MMK_CXXFLAGS_APPEND")
        {
            data.push_str(
                &format!(
                    "CXXFLAGS += {cxxflags}\n",
                    cxxflags = borrowed_dependency
                        .mmk_data()
                        .to_string("MMK_CXXFLAGS_APPEND")
                )
                .to_owned(),
            );
        }

        if borrowed_dependency
            .mmk_data()
            .data()
            .contains_key("MMK_CPPFLAGS_APPEND")
        {
            data.push_str(
                &format!(
                    "CPPFLAGS += {cppflags}\n",
                    cppflags = borrowed_dependency
                        .mmk_data()
                        .to_string("MMK_CPPFLAGS_APPEND")
                )
                .to_owned(),
            );
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

    fn print_ok(self: &Self) -> () {
        print!(".");
    }
}

impl DependencyAccessor for MakefileGenerator {
    fn set_dependency(&mut self, dependency: &DependencyNode) {
        self.dependency = Some(dependency.clone());
    }

    fn get_dependency(&self) -> Result<&DependencyNode, error::DependencyError> {
        if let Some(dep) = &self.dependency {
            return Ok(dep);
        }
        return Err(error::DependencyError::NotSet);
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
#[path = "./lib_test.rs"]
mod lib_test;
