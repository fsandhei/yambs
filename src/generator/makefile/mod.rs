use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use indoc;

mod include_file_generator;
pub mod make;

use crate::build_target;
use crate::build_target::include_directories;
use crate::build_target::{
    include_directories::{IncludeDirectory, IncludeType},
    pkg_config::ProvideMethod,
    target_registry::TargetRegistry,
    Dependency, DependencySource, LibraryType, TargetNode, TargetState, TargetType,
};
use crate::cli::command_line;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::errors::FsError;
use crate::generator;
use crate::generator::{
    targets::ObjectTarget, targets::ProgressDocument, targets::ProgressTrackingTarget, Generator,
    GeneratorError, UtilityGenerator,
};
use crate::parser::types;
use crate::progress;
use crate::toolchain::NormalizedToolchain;
use crate::utility;

use include_file_generator::IncludeFileGenerator;
pub use make::Make;

struct ExecutableTargetFactory;

impl ExecutableTargetFactory {
    pub fn create_rule(target: &TargetNode, output_directory: &std::path::Path) -> String {
        let target_name = target.borrow().name();
        format!("\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ $({target_name_capitalized}_LDFLAGS) -o $@)",
                    target_name = target_name,
                    target_name_capitalized = target_name.to_uppercase(),
                    prerequisites = generate_prerequisites(target, output_directory),
                    dependencies = generate_search_directories(target),
            )
    }
}

struct LibraryTargetFactory;

impl LibraryTargetFactory {
    pub fn create_rule(target: &TargetNode, output_directory: &std::path::Path) -> String {
        let mut formatted_string = String::new();
        let library_name = library_name_from_target_type(&target.borrow().target_type);
        let target_rule = match target.borrow().library_type().unwrap() {
            LibraryType::Static => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\n",
                target_name = library_name,
                prerequisites = generate_prerequisites(target, output_directory)
            ),
            LibraryType::Dynamic => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ $({target_name_capitalized}_LDFLAGS) -o $@)\n\n",
                    target_name = library_name,
                    target_name_capitalized = target.borrow().name().to_uppercase(),
                    prerequisites = generate_prerequisites(target, output_directory),
                    dependencies = generate_search_directories(target),
            ),
        };
        formatted_string.push_str(&target_rule);

        let convenience_rule = indoc::formatdoc!(
            "# Convenience rule for \"{target_name}\"
            {target_name}: {library_name}\n
            ",
            target_name = target.borrow().name(),
            library_name = library_name
        );
        formatted_string.push_str(&convenience_rule);
        formatted_string
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

fn library_name_from_target_type(target_type: &TargetType) -> String {
    match target_type {
        TargetType::Executable(_) => panic!("Not a library"),
        TargetType::Library(lib) => lib.to_string(),
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
        formatted_string.push_str(&format!("   {}", object.display()));
    }
    for dependency in &borrowed_target.dependencies {
        formatted_string.push_str("\\\n");
        match dependency.source {
            build_target::DependencySource::FromSource(ref s) => {
                formatted_string.push_str(&format!("   {}", s.library));
            }
            build_target::DependencySource::FromPkgConfig(ref pkg) => match pkg.method {
                ProvideMethod::Finegrained(ref libs) => {
                    for (i, lib) in libs.iter().enumerate() {
                        formatted_string.push_str(&format!("   {}", lib.path().display()));
                        if i != libs.len() - 1 {
                            formatted_string.push_str("\\\n");
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
    formatted_string
}

fn generate_search_directories(target: &TargetNode) -> String {
    let borrowed_target = target.borrow();
    let mut formatted_string = String::new();
    formatted_string.push_str(&borrowed_target.include_directory.as_include_flag());

    for dependency in &borrowed_target.dependencies {
        match dependency.source.from_source() {
            Some(sd) => {
                let include_dir = &sd.include_directory;
                formatted_string.push_str(&include_dir.as_include_flag());
            }
            None => {}
        }
    }
    formatted_string.trim_end().to_string()
}

fn generate_defines(defines: &[types::Define]) -> String {
    defines
        .iter()
        .map(|d| {
            if let Some(ref value) = d.value {
                format!(" -D{}={}", d.macro_, value)
            } else {
                format!(" -D{}", d.macro_,)
            }
        })
        .collect::<String>()
}

fn generate_include_directories(
    include_directories: &include_directories::IncludeDirectories,
) -> String {
    let mut formatted_string = String::new();
    for include in include_directories {
        if include.include_type == IncludeType::System {
            formatted_string.push_str(&format!("-isystem {}", include.path.display()))
        } else {
            formatted_string.push_str(&format!("-I{}", include.path.display()))
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
        "\t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target}_CXXFLAGS) $({target}_CPPFLAGS) \
         $(WARNINGS) {dependencies} $< -c -o $@)\n\n",
        dependencies = generate_include_directories(&object_target.include_directories),
        target = object_target.target.to_uppercase(),
    ));
    formatted_string
}

pub struct MakefileGenerator {
    pub toolchain: Rc<RefCell<NormalizedToolchain>>,
    pub configurations: command_line::ConfigurationOpts,
    pub build_directory: BuildDirectory,
    pub output_directory: std::path::PathBuf,
    pub progress_document: ProgressDocument,
}

impl MakefileGenerator {
    pub fn new(
        configurations: &command_line::ConfigurationOpts,
        build_directory: &BuildDirectory,
        toolchain: Rc<RefCell<NormalizedToolchain>>,
    ) -> Result<Self, GeneratorError> {
        utility::create_dir(build_directory.as_path())?;
        Ok(Self {
            configurations: configurations.to_owned(),
            build_directory: build_directory.clone(),
            output_directory: build_directory.as_path().to_path_buf(),
            progress_document: ProgressDocument::new(),
            toolchain,
        })
    }

    fn generate_all_target_for_progress_document(&mut self, object_targets: &[ObjectTarget]) {
        let mut target_all = ProgressTrackingTarget {
            target: "all".to_string(),
            object_files: Vec::new(),
            dependencies: Vec::new(),
        };

        for object_target in object_targets {
            let target_name = object_target.target.clone();
            if !target_all.dependencies.contains(&target_name) {
                target_all.dependencies.push(target_name);
            }
        }
        self.progress_document.targets.push(target_all);
    }

    fn generate_makefile(
        &mut self,
        writers: &mut Writers,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        self.generate_header(&mut writers.makefile_writer, &registry.registry)?;

        self.push_and_create_directory(std::path::Path::new("deps"))?;
        for target in &registry.registry {
            if target.borrow().state != TargetState::BuildFileMade {
                let borrowed_target = target.borrow();
                let dep_dir = format!("{}.dir", &borrowed_target.name());
                self.push_and_create_directory(std::path::Path::new(&dep_dir))?;
                log::debug!(
                    "Generating makefiles for target {:?} (manifest path: {})",
                    target.borrow().name(),
                    borrowed_target.manifest.directory.display()
                );

                self.generate_rule_declaration_for_target(writers, target);
                // Quick hack to allow each dependency / target to be placed in their own
                // folder, without it being a subfolder of a separate target.
                // FIXME: Need to figure out if there is a better way to solve this. It is
                // rather clunky.
                self.output_directory.pop();
                self.generate_rule_for_dependencies_from_source_data(
                    writers,
                    &borrowed_target.name(),
                    &borrowed_target,
                    registry,
                )?;
                self.push_and_create_directory(std::path::Path::new(&dep_dir))?;

                let progress_tracking_target =
                    ProgressTrackingTarget::from_target(target, &self.output_directory);
                self.progress_document
                    .add_progress_tracking_target(progress_tracking_target);
                ObjectTarget::create_object_targets(target, &self.output_directory)
                    .into_iter()
                    .for_each(|object_target| {
                        if !writers
                            .makefile_writer
                            .object_targets
                            .contains(&object_target)
                        {
                            writers.makefile_writer.object_targets.push(object_target);
                        }
                    });
                self.output_directory.pop();
            }
            target.borrow_mut().state = TargetState::BuildFileMade;
        }
        self.output_directory.pop();
        self.generate_object_rules(writers)?;
        self.generate_depends_rules(&mut writers.makefile_writer);
        Ok(())
    }

    fn generate_rule_for_dependencies_from_source_data(
        &mut self,
        writers: &mut Writers,
        target_name: &str,
        build_target: &build_target::BuildTarget,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        if !build_target.dependencies.is_empty() {
            let dependencies = &build_target.dependencies;
            for dependency in dependencies {
                match dependency.source {
                    build_target::DependencySource::FromSource(ref s) => {
                        log::debug!("Generating build rule for dependency \"{}\" (manifest path = {}) to target \"{}\" (manifest path {})",
                            s.library.name,
                            s.manifest.directory.display(),
                            target_name,
                            build_target.manifest.directory.display());
                        let dep_dir = format!("{}.dir", &s.library.name);
                        self.push_and_create_directory(std::path::Path::new(&dep_dir))?;
                        self.generate_rule_for_dependency(writers, dependency, registry);
                        self.output_directory.pop();
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    fn generate_rule_for_dependency(
        &mut self,
        writers: &mut Writers,
        dependency: &Dependency,
        registry: &TargetRegistry,
    ) {
        let dependency_target = dependency.to_build_target(registry).unwrap();
        if dependency_target.borrow().state != TargetState::BuildFileMade {
            self.generate_compiler_flags_for_target(
                &dependency_target,
                &mut writers.makefile_writer,
            );
            writers.makefile_writer.data.push('\n');
            let rule =
                LibraryTargetFactory::create_rule(&dependency_target, &self.output_directory);
            ObjectTarget::create_object_targets(&dependency_target, &self.output_directory)
                .iter()
                .for_each(|object_target| {
                    if !writers
                        .makefile_writer
                        .object_targets
                        .contains(object_target)
                    {
                        writers
                            .makefile_writer
                            .object_targets
                            .push(object_target.clone());
                    }
                });
            writers.makefile_writer.data.push_str(&rule);
            let progress_tracking_target =
                ProgressTrackingTarget::from_target(&dependency_target, &self.output_directory);
            self.progress_document
                .add_progress_tracking_target(progress_tracking_target);
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
        Ok(match std::fs::create_dir_all(&self.output_directory) {
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
        utility::create_dir(self.output_directory.join(dir)).map_err(GeneratorError::Fs)
    }

    fn generate_default_all_target(&self, writer: &mut Writer, targets: &[TargetNode]) {
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
        writer.data.push_str(&text);
    }

    fn generate_phony(&self, writer: &mut Writer, target: &TargetNode) {
        let data = indoc::formatdoc!(
            "\n
            # Phony for target \"{target_name}\"
            .PHONY: {target_name}\n
        ",
            target_name = target.borrow().name()
        );
        writer.data.push_str(&data);
    }

    fn generate_header(
        &self,
        writer: &mut Writer,
        targets: &[TargetNode],
    ) -> Result<(), GeneratorError> {
        let data = format!(
            "\
  # ----- INCLUDES -----\n\
  include {build_directory}/make_include/warnings.mk\n\
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

        writer.data.push_str(&data);
        self.generate_default_all_target(writer, targets);
        Ok(())
    }

    fn generate_include_files(&self) -> Result<(), GeneratorError> {
        let include_output_directory = self.output_directory.join("make_include");
        let toolchain = self.toolchain.borrow();
        let mut include_file_generator =
            IncludeFileGenerator::new(&include_output_directory, &toolchain);

        let cxx_standard = &self.configurations.cxx_standard.to_string();
        include_file_generator.add_cpp_version(cxx_standard);
        include_file_generator.generate_build_files()
    }

    fn generate_object_rules(&self, writers: &mut Writers) -> Result<(), GeneratorError> {
        for object_target in &writers.makefile_writer.object_targets {
            let object_dir = object_target
                .object
                .parent()
                .and_then(|p| p.strip_prefix(&self.output_directory).ok());

            if let Some(object_dir) = object_dir {
                self.create_subdir(object_dir)?;
            }
            writers
                .makefile_writer
                .data
                .push_str(&generate_object_target(object_target))
        }
        Ok(())
    }

    fn generate_depends_rules(&self, writer: &mut Writer) {
        let depend_files = writer
            .object_targets
            .iter()
            .map(|object_target| {
                let mut object_clone = object_target.object.clone();
                object_clone.set_extension("d");
                object_clone
            })
            .collect::<Vec<std::path::PathBuf>>();

        writer.data.push('\n');
        for depend_file in depend_files {
            writer
                .data
                .push_str(&format!("# Silently include {}\n", depend_file.display()));
            writer
                .data
                .push_str(&format!("sinclude {}\n", depend_file.display()));
        }
    }

    fn generate_rule_declaration_for_target(&self, writers: &mut Writers, target: &TargetNode) {
        self.generate_phony(&mut writers.makefile_writer, target);
        self.generate_compiler_flags_for_target(target, &mut writers.makefile_writer);
        let target_rule_declaration =
            TargetRuleFactory::create_rule(target, &self.output_directory);
        writers.makefile_writer.data.push('\n');
        writers.makefile_writer.data.push_str(&format!(
            "# Rule for target \"{}\"\n",
            target.borrow().name()
        ));
        writers
            .makefile_writer
            .data
            .push_str(&target_rule_declaration);
        writers.makefile_writer.data.push('\n');
        writers.makefile_writer.data.push('\n');
    }

    fn generate_compiler_flags_for_target(
        &self,
        target: &TargetNode,
        makefile_writer: &mut Writer,
    ) {
        let borrowed_target = target.borrow();
        let target_name = borrowed_target.name();
        let target_name_capitalized = target_name.to_uppercase();
        let cxx_flags = &borrowed_target.compiler_flags.cxx_flags;

        makefile_writer.data.push_str(&indoc::formatdoc!(
            "# CXXFLAGS for target \"{target_name}\"
    {target_name_capitalized}_CXXFLAGS +="
        ));

        if let Some(cxx) = cxx_flags {
            makefile_writer.data.push_str(&indoc::formatdoc!(
                "{cxx_flags}",
                cxx_flags = cxx.flags().join(" ")
            ));
        }

        for include_dir in &borrowed_target.compiler_flags.include_directories {
            let include_dir = IncludeDirectory {
                path: include_dir.to_path_buf(),
                include_type: IncludeType::Include,
            };
            makefile_writer
                .data
                .push_str(&include_dir.as_include_flag());
            makefile_writer.data.push(' ');
        }

        for include_dir in &borrowed_target.compiler_flags.system_include_directories {
            let include_dir = IncludeDirectory {
                path: include_dir.to_path_buf(),
                include_type: IncludeType::System,
            };
            makefile_writer
                .data
                .push_str(&include_dir.as_include_flag());
            makefile_writer.data.push(' ');
        }

        makefile_writer.data.push('\n');
        makefile_writer.data.push('\n');

        let cpp_flags = &borrowed_target.compiler_flags.cpp_flags;
        makefile_writer.data.push_str(&indoc::formatdoc!(
            "# CPPFLAGS for target \"{target_name}\"
    {target_name_capitalized}_CPPFLAGS +="
        ));

        if let Some(cpp) = cpp_flags {
            makefile_writer.data.push_str(&indoc::formatdoc!(
                "{cpp_flags}",
                cpp_flags = cpp.flags().join(" ")
            ));
        }

        let defines = if !self.configurations.defines.is_empty() {
            let defines = &self.configurations.defines;
            generate_defines(defines)
        } else {
            generate_defines(&borrowed_target.defines)
        };

        makefile_writer.data.push_str(&defines);

        makefile_writer.data.push('\n');
        makefile_writer.data.push('\n');
        makefile_writer.data.push_str(&indoc::formatdoc!(
            "# LDFLAGS for target \"{target_name}\"
                {target_name_capitalized}_LDFLAGS +="
        ));

        let deps = &borrowed_target.dependencies;
        for dep in deps {
            match dep.source {
                DependencySource::FromPkgConfig(ref pkg_config_target) => {
                    match pkg_config_target.method {
                        ProvideMethod::PkgConfigOutput(ref ld_flags) => {
                            let search_flags = ld_flags.link_dirs.join(" ");
                            makefile_writer.data.push_str(&search_flags);
                            let libs = ld_flags.link_libs.join(" ");
                            makefile_writer.data.push(' ');
                            makefile_writer.data.push_str(&libs);
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        makefile_writer.data.push('\n');
        makefile_writer.data.push('\n');
    }
}

impl Generator for MakefileGenerator {
    fn generate(
        &mut self,
        registry: &TargetRegistry,
    ) -> Result<std::path::PathBuf, GeneratorError> {
        self.generate_include_files()?;
        self.push_and_create_directory(&std::path::PathBuf::from(
            &self.configurations.build_type.to_string(),
        ))?;
        let mut writers = Writers {
            makefile_writer: Writer::new(&self.output_directory.join("Makefile"))?,
            progress_writer: ProgressWriter::new(&self.output_directory)?,
        };
        self.generate_makefile(&mut writers, registry)?;
        self.generate_all_target_for_progress_document(&writers.makefile_writer.object_targets);
        writers
            .progress_writer
            .write_document(&self.progress_document);
        writers.makefile_writer.write()?;
        Ok(self.output_directory.clone())
    }
}

pub(crate) struct Writers {
    makefile_writer: Writer,
    progress_writer: ProgressWriter,
}

struct ProgressWriter {
    file_handle: std::fs::File,
}

impl ProgressWriter {
    pub fn new(base_dir: &std::path::Path) -> Result<Self, GeneratorError> {
        let path = base_dir.join(progress::PROGRESS_FILE_NAME);
        let file_handle = utility::create_file(&path)?;
        Ok(Self { file_handle })
    }

    pub fn write_document(&mut self, document: &generator::targets::ProgressDocument) {
        let s = serde_json::to_string_pretty(document).unwrap();
        self.file_handle.write_all(s.as_bytes()).unwrap();
    }
}

struct Writer {
    file_handle: std::fs::File,
    data: String,
    object_targets: Vec<ObjectTarget>,
}

impl Writer {
    pub fn new(path: &std::path::Path) -> Result<Self, GeneratorError> {
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
