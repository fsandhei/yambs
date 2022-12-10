use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use indoc;

use crate::build_target;
use crate::build_target::include_directories;
use crate::build_target::{
    include_directories::IncludeType, target_registry::TargetRegistry, Dependency, LibraryType,
    TargetNode, TargetState, TargetType,
};
use crate::cli::command_line;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::compiler::{Compiler, Type};
use crate::errors::FsError;
use crate::generator;
use crate::generator::{
    targets::ObjectTarget, Generator, GeneratorError, Sanitizer, UtilityGenerator,
};
use crate::progress;
use crate::utility;

const STATIC_LIBRARY_FILE_EXTENSION: &str = "a";
const SHARED_LIBRARY_FILE_EXTENSION: &str = "so";

struct ExecutableTargetFactory;

impl ExecutableTargetFactory {
    pub fn create_rule(
        target: &TargetNode,
        output_directory: &std::path::Path,
        build_type: &configurations::BuildType,
    ) -> String {
        let target_name = target.borrow().name();
        format!("\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)",
                    target_name = target_name,
                    target_name_capitalized = target_name.to_uppercase(),
                    prerequisites = generate_prerequisites(target, output_directory, build_type),
                    dependencies = generate_search_directories(target),
            )
    }
}

struct LibraryTargetFactory;

impl LibraryTargetFactory {
    pub fn create_rule(
        target: &TargetNode,
        output_directory: &std::path::Path,
        build_type: &configurations::BuildType,
    ) -> String {
        let mut formatted_string = String::new();
        let library_name = library_name_from_target_type(&target.borrow().target_type);
        let target_rule = match target.borrow().library_type().unwrap() {
            LibraryType::Static => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(AR) $(ARFLAGS) $@ $?)\n\n",
                target_name = library_name,
                prerequisites = generate_prerequisites(target, output_directory, build_type)
            ),
            LibraryType::Dynamic => format!(
                "\
                {target_name} : \
                    {prerequisites}\n\
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ -o $@)\n\n",
                    target_name = library_name,
                    target_name_capitalized = target.borrow().name().to_uppercase(),
                    prerequisites = generate_prerequisites(target, output_directory, build_type),
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
    pub fn create_rule(
        target: &TargetNode,
        output_dir: &std::path::Path,
        build_type: &configurations::BuildType,
    ) -> String {
        if target.borrow().is_executable() {
            ExecutableTargetFactory::create_rule(target, output_dir, build_type)
        } else {
            LibraryTargetFactory::create_rule(target, output_dir, build_type)
        }
    }
}

fn library_name_from_dependency_source_data(
    dependency_source_data: &build_target::DependencySourceData,
) -> String {
    match dependency_source_data.library_type {
        build_target::LibraryType::Dynamic => format!(
            "lib{}.{}",
            dependency_source_data.name, SHARED_LIBRARY_FILE_EXTENSION
        ),
        build_target::LibraryType::Static => format!(
            "lib{}.{}",
            dependency_source_data.name, STATIC_LIBRARY_FILE_EXTENSION
        ),
    }
}

fn library_name_from_target_type(target_type: &TargetType) -> String {
    match target_type {
        TargetType::Executable(_) => panic!("Not a library"),
        TargetType::Library(lib_type, name) => match lib_type {
            build_target::LibraryType::Dynamic => {
                format!("lib{}.{}", name, SHARED_LIBRARY_FILE_EXTENSION)
            }
            build_target::LibraryType::Static => {
                format!("lib{}.{}", name, STATIC_LIBRARY_FILE_EXTENSION)
            }
        },
    }
}

fn generate_prerequisites(
    target: &TargetNode,
    output_directory: &std::path::Path,
    build_type: &configurations::BuildType,
) -> String {
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
                        formatted_string.push_str(&format!(
                            "   {}",
                            library_name_from_dependency_source_data(s)
                        ));
                    }
                    build_target::DependencySource::FromPrebuilt(ref b) => match build_type {
                        configurations::BuildType::Debug => {
                            formatted_string
                                .push_str(&format!("   {}", b.debug_binary_path.display()));
                        }
                        configurations::BuildType::Release => {
                            formatted_string
                                .push_str(&format!("   {}", b.release_binary_path.display()));
                        }
                    },
                }
            }
        }
        _ => {}
    }
    formatted_string
}

fn generate_compiler_flags_for_target(target: &TargetNode, makefile_writer: &mut Writer) {
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
    makefile_writer.data.push('\n');
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
                formatted_string.push_str(" -L.");
            }
        }
        _ => {}
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
        "\t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target}_CXXFLAGS) $({target}_CPPFLAGS) \
         $(WARNINGS) {dependencies} $< -c -o $@)\n\n",
        dependencies = generate_include_directories(&object_target.include_directories),
        target = object_target.target.to_uppercase(),
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

    /// Create JSON progress document with similar hierarchical layout as the makefile targets:
    /// There exists an all target which has dependencies to all of the other targets
    /// Each target contains a list of object files tied to itself and a reference to an
    /// existing target. This should be useful for when yambs is being called to build
    /// a specific target.
    fn populate_progress_document(
        &self,
        registry: &TargetRegistry,
    ) -> Result<generator::targets::ProgressDocument, GeneratorError> {
        let mut progress_document = generator::targets::ProgressDocument {
            targets: Vec::new(),
        };

        let mut target_all = generator::targets::ProgressTrackingTarget {
            target: "all".to_string(),
            object_files: Vec::new(),
            dependencies: Vec::new(),
        };

        for target_node in &registry.registry {
            let target_object_targets = self
                .create_object_targets(target_node)
                .iter()
                .map(|o| o.object.to_path_buf())
                .collect::<Vec<std::path::PathBuf>>();
            let target_name = target_node.borrow().name();
            let target_dependencies = match target_node.borrow().target_source {
                build_target::TargetSource::FromSource(ref s) => s
                    .dependencies
                    .iter()
                    .filter_map(|d| match d.source {
                        build_target::DependencySource::FromSource(ref ds) => Some(ds),
                        build_target::DependencySource::FromPrebuilt(_) => None,
                    })
                    .map(|ds| ds.name.to_owned())
                    .collect::<Vec<String>>(),
                build_target::TargetSource::FromPrebuilt(_) => Vec::new(),
            };

            let target = generator::targets::ProgressTrackingTarget {
                target: target_name.clone(),
                object_files: target_object_targets.clone(),
                dependencies: target_dependencies.clone(),
            };
            progress_document.targets.push(target);

            target_all.dependencies.push(target_name);
        }
        progress_document.targets.push(target_all);
        Ok(progress_document)
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
        let target_name = borrowed_target.name();

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
                target: target_name.clone(),
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
        writers: &mut Writers,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        self.generate_header(&mut writers.makefile_writer, &registry.registry)?;

        for target in &registry.registry {
            if target.borrow().state != TargetState::BuildFileMade {
                let borrowed_target = target.borrow();
                match borrowed_target.target_source {
                    build_target::TargetSource::FromPrebuilt(_) => {}
                    build_target::TargetSource::FromSource(ref s) => {
                        log::debug!(
                            "Generating makefiles for target {:?} (manifest path: {})",
                            target.borrow().name(),
                            s.manifest.directory.display()
                        );
                        self.generate_rule_declaration_for_target(writers, target);
                        self.generate_rule_for_dependencies_from_source_data(
                            writers,
                            &borrowed_target.name(),
                            s,
                            registry,
                        )?;

                        self.create_object_targets(target)
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
                    }
                }
            }
            target.borrow_mut().state = TargetState::BuildFileMade;
        }
        self.generate_object_rules(writers);
        self.generate_depends_rules(&mut writers.makefile_writer);
        Ok(())
    }

    fn generate_rule_for_dependencies_from_source_data(
        &mut self,
        writers: &mut Writers,
        target_name: &str,
        source_data: &build_target::SourceBuildData,
        registry: &TargetRegistry,
    ) -> Result<(), GeneratorError> {
        if !source_data.dependencies.is_empty() {
            let dependencies = &source_data.dependencies;
            for dependency in dependencies {
                match dependency.source {
                    build_target::DependencySource::FromPrebuilt(_) => {}
                    build_target::DependencySource::FromSource(ref s) => {
                        log::debug!("Generating build rule for dependency \"{}\" (manifest path = {}) to target \"{}\" (manifest path {})",
                            s.name,
                            s.manifest.directory.display(),
                            target_name,
                            source_data.manifest.directory.display());
                        if s.manifest.directory != source_data.manifest.directory {
                            let dep_dir = format!("{}.d", &s.name);
                            self.push_and_create_directory(&std::path::Path::new(&dep_dir))?;
                            self.generate_rule_for_dependency(writers, dependency, registry);
                            self.output_directory.pop();
                        } else {
                            self.generate_rule_for_dependency(writers, dependency, registry);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn generate_rule_for_dependency(
        &self,
        writers: &mut Writers,
        dependency: &Dependency,
        registry: &TargetRegistry,
    ) {
        let dependency_target = dependency.to_build_target(registry).unwrap();

        if dependency_target.borrow().state != TargetState::BuildFileMade {
            generate_compiler_flags_for_target(&dependency_target, &mut writers.makefile_writer);
            writers.makefile_writer.data.push('\n');
            let rule = LibraryTargetFactory::create_rule(
                &dependency_target,
                &self.output_directory,
                &self.configurations.build_type,
            );
            self.create_object_targets(&dependency_target)
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
        utility::create_dir(&self.output_directory.join(dir)).map_err(GeneratorError::Fs)
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

        writer.data.push_str(&data);
        self.generate_default_all_target(writer, targets);
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

    fn generate_object_rules(&self, writers: &mut Writers) {
        for object_target in &writers.makefile_writer.object_targets {
            writers
                .makefile_writer
                .data
                .push_str(&generate_object_target(object_target))
        }
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
        generate_compiler_flags_for_target(target, &mut writers.makefile_writer);
        let target_rule_declaration = TargetRuleFactory::create_rule(
            target,
            &self.output_directory,
            &self.configurations.build_type,
        );
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
}

impl Generator for MakefileGenerator {
    fn generate(&mut self, registry: &TargetRegistry) -> Result<(), GeneratorError> {
        self.generate_include_files()?;
        self.push_and_create_directory(&directory_from_build_configuration(
            &self.configurations.build_type,
        ))?;
        let mut writers = Writers {
            makefile_writer: Writer::new(&self.output_directory.join("Makefile"))?,
            progress_writer: ProgressWriter::new(&self.output_directory)?,
        };
        self.generate_makefile(&mut writers, registry)?;
        let progress_document = self.populate_progress_document(registry)?;
        writers.progress_writer.write_document(&progress_document);
        writers.makefile_writer.write()?;
        Ok(())
    }
}

struct Writers {
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

    use crate::flags;
    use crate::manifest;
    use crate::tests::EnvLock;
    use pretty_assertions::assert_eq;
    use tempdir::TempDir;

    use super::*;

    struct TargetNodeStub;

    impl TargetNodeStub {
        pub fn builder() -> TargetNodeBuilder {
            TargetNodeBuilder {
                target_source: None,
                target_type: None,
                include_directories: None,
                compiler_flags: None,
            }
        }
    }

    struct TargetNodeBuilder {
        target_source: Option<build_target::TargetSource>,
        target_type: Option<build_target::TargetType>,
        include_directories: Option<build_target::include_directories::IncludeDirectories>,
        compiler_flags: Option<flags::CompilerFlags>,
    }

    impl TargetNodeBuilder {
        pub fn create(self) -> build_target::TargetNode {
            let build_target = build_target::BuildTarget {
                target_source: self.target_source.unwrap(),
                state: build_target::TargetState::Registered,
                target_type: self
                    .target_type
                    .unwrap_or(build_target::TargetType::Executable("x".to_string())),
                include_directories: self
                    .include_directories
                    .unwrap_or(build_target::include_directories::IncludeDirectories::new()),
                compiler_flags: self.compiler_flags.unwrap_or(flags::CompilerFlags::new()),
            };

            build_target::TargetNode::new(build_target)
        }

        pub fn with_target_source(mut self, target_source: build_target::TargetSource) -> Self {
            self.target_source = Some(target_source);
            self
        }

        pub fn with_target_type(mut self, target_type: build_target::TargetType) -> Self {
            self.target_type = Some(target_type);
            self
        }

        pub fn with_include_directories(
            mut self,
            include_directories: build_target::include_directories::IncludeDirectories,
        ) -> Self {
            self.include_directories = Some(include_directories);
            self
        }
    }

    struct ManifestStub {
        pub manifest: manifest::Manifest,
    }

    impl ManifestStub {
        pub fn new(path: &std::path::Path) -> Self {
            let manifest_path = path.join(crate::YAMBS_MANIFEST_NAME);
            std::fs::File::create(manifest_path).unwrap();
            Self {
                manifest: manifest::Manifest::new(path),
            }
        }
    }

    struct ProjectTestFixture {
        dir: TempDir,
        target_registry: build_target::target_registry::TargetRegistry,
    }

    impl ProjectTestFixture {
        pub fn new() -> Self {
            let dir = TempDir::new("project").unwrap();
            let target_registry = build_target::target_registry::TargetRegistry::new();

            Self {
                dir,
                target_registry,
            }
        }
    }

    struct MakefileGeneratorTestFixture {
        build_dir: TempDir,
        configuration_opts: command_line::ConfigurationOpts,
        compiler: Compiler,
        writers: Writers,
    }

    impl MakefileGeneratorTestFixture {
        pub fn new() -> Self {
            let build_dir = TempDir::new("build").unwrap();
            let build_dir_path = build_dir.path().to_path_buf();
            let configuration_opts = create_configuration_opts();
            let compiler = create_compiler();
            let writers = Writers {
                makefile_writer: Writer::new(&build_dir_path.join("Makefile")).unwrap(),
                progress_writer: ProgressWriter::new(&build_dir_path).unwrap(),
            };

            Self {
                build_dir,
                configuration_opts,
                compiler,
                writers,
            }
        }

        pub fn build_directory(&self) -> BuildDirectory {
            BuildDirectory::from(self.build_dir.path())
        }
    }

    fn create_compiler() -> Compiler {
        let _lock = EnvLock::lock("CXX", "gcc");
        Compiler::new().unwrap()
    }

    fn create_configuration_opts() -> command_line::ConfigurationOpts {
        let build_type = configurations::BuildType::Debug;
        let cxx_standard = configurations::CXXStandard::CXX17;
        let sanitizer = None;

        command_line::ConfigurationOpts {
            build_type,
            cxx_standard,
            sanitizer,
        }
    }

    fn create_include_directory(
        path: &std::path::Path,
    ) -> build_target::include_directories::IncludeDirectory {
        build_target::include_directories::IncludeDirectory {
            include_type: build_target::include_directories::IncludeType::Include,
            path: path.to_path_buf(),
        }
    }

    fn create_source_file(path: &std::path::Path) -> build_target::associated_files::SourceFile {
        std::fs::File::create(path).unwrap();
        build_target::associated_files::SourceFile::new(path).unwrap()
    }

    fn produce_include_path(base_dir: TempDir) -> std::path::PathBuf {
        let build_dir = std::path::PathBuf::from(".build");
        let output_directory = base_dir.path().join(build_dir).join("make_include");
        output_directory
    }

    fn construct_generator<'generator>(path: &std::path::Path) -> IncludeFileGenerator<'generator> {
        IncludeFileGenerator::new(path, crate::compiler::Compiler::new().unwrap())
    }

    fn create_files(files: &[std::path::PathBuf]) {
        for file in files {
            std::fs::File::create(file).unwrap();
        }
    }

    fn create_sample_source_target(manifest_dir: &std::path::Path) -> build_target::TargetSource {
        let manifest = ManifestStub::new(manifest_dir);
        let source_files_paths = [
            manifest.manifest.directory.join("a.cpp"),
            manifest.manifest.directory.join("b.cpp"),
            manifest.manifest.directory.join("c.cpp"),
        ];
        create_files(&source_files_paths);
        let source_files =
            build_target::associated_files::SourceFiles::from_paths(&source_files_paths).unwrap();
        let dependencies = Vec::<build_target::Dependency>::new();
        build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        })
    }

    fn object_files_from_sample_source(
        target_source: &build_target::TargetSource,
        build_dir: &std::path::Path,
    ) -> Vec<std::path::PathBuf> {
        match target_source {
            build_target::TargetSource::FromSource(s) => s
                .source_files
                .iter()
                .map(|s| {
                    let p = s.file();
                    let file_name = p.file_name().unwrap();
                    build_dir.join(file_name).with_extension("o")
                })
                .collect::<Vec<std::path::PathBuf>>(),
            build_target::TargetSource::FromPrebuilt(_) => {
                panic!("target_source was not a source type!");
            }
        }
    }

    // FIXME: Add test for generation of dependencies.

    #[test]
    fn populate_progress_document_generates_document_with_all_object_files_to_targets() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();

        let generator = MakefileGenerator::new(
            &fixture.configuration_opts,
            &fixture.build_directory(),
            fixture.compiler.clone(),
        )
        .unwrap();

        let target_source = create_sample_source_target(project_path);
        let object_files_paths =
            object_files_from_sample_source(&target_source, fixture.build_directory().as_path());

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Library(
                build_target::LibraryType::Static,
                "myLib".to_string(),
            ))
            .with_include_directories(include_directories)
            .create();
        project_fixture.target_registry.add_target(target_node);

        let actual = generator
            .populate_progress_document(&project_fixture.target_registry)
            .unwrap();
        let expected = generator::targets::ProgressDocument {
            targets: vec![
                generator::targets::ProgressTrackingTarget {
                    target: "myLib".to_string(),
                    object_files: object_files_paths,
                    dependencies: Vec::new(),
                },
                generator::targets::ProgressTrackingTarget {
                    target: "all".to_string(),
                    object_files: Vec::new(),
                    dependencies: vec!["myLib".to_string()],
                },
            ],
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn progress_document_is_generated_into_json() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();

        let generator = MakefileGenerator::new(
            &fixture.configuration_opts,
            &fixture.build_directory(),
            fixture.compiler.clone(),
        )
        .unwrap();

        let target_source = create_sample_source_target(project_path);
        let object_files_paths =
            object_files_from_sample_source(&target_source, fixture.build_directory().as_path());
        let object_files_str_formatted = {
            let mut formatted_string = String::new();
            for object_file in object_files_paths {
                formatted_string.push_str(&format!("{:?},\n", &object_file.display()));
                formatted_string.push_str("        ");
            }
            formatted_string
                .trim_end()
                .trim_end_matches(',')
                .to_string()
        };

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Library(
                build_target::LibraryType::Static,
                "myLib".to_string(),
            ))
            .with_include_directories(include_directories)
            .create();
        project_fixture.target_registry.add_target(target_node);

        let actual = serde_json::to_string_pretty(
            &generator
                .populate_progress_document(&project_fixture.target_registry)
                .unwrap(),
        )
        .unwrap();
        let expected = indoc::formatdoc!(
            r#"
            {{
              "targets": [
                {{
                  "target": "myLib",
                  "object_files": [
                    {object_files}
                  ]
                }},
                {{
                  "target": "all",
                  "dependencies": [
                    "myLib"
                  ]
                }}
              ]
            }}"#,
            object_files = object_files_str_formatted
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn generate_include_directories_generate_include_statements() {
        let project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let inner_project_path = project_fixture.dir.path().join("myProject");
        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        let project_include_path = project_path.join("include");
        let inner_project_include_path = inner_project_path.join("include");
        include_directories.add(create_include_directory(&project_include_path));
        include_directories.add(create_include_directory(&inner_project_include_path));

        let actual = generate_include_directories(&include_directories);
        let expected = format!(
            "-I{} -I{}",
            project_include_path.display(),
            inner_project_include_path.display()
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn factory_creates_static_library_target_declaration() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();
        let build_path = fixture.build_dir.path();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Library(
                build_target::LibraryType::Static,
                "myLib".to_string(),
            ))
            .with_include_directories(include_directories)
            .create();

        let target_name = library_name_from_target_type(&target_node.borrow().target_type);

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        let build_type = configurations::BuildType::Debug;

        let actual = TargetRuleFactory::create_rule(&target_node, &build_path, &build_type);
        let expected = indoc::formatdoc!(
            "
        {target_name} : \
            {object_files_string}\n\
            \t$(strip $(AR) $(ARFLAGS) $@ $?)

        # Convenience rule for \"{target}\"
        {target}: {target_name}


",
            target_name = target_name,
            target = target_node.borrow().name(),
            object_files_string = generate_prerequisites(&target_node, &build_path, &build_type),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn factory_creates_dynamic_library_target_declaration() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();
        let build_path = fixture.build_dir.path();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Library(
                build_target::LibraryType::Dynamic,
                "myLib".to_string(),
            ))
            .with_include_directories(include_directories)
            .create();

        let target_name = library_name_from_target_type(&target_node.borrow().target_type);

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        let build_type = configurations::BuildType::Debug;

        let actual = TargetRuleFactory::create_rule(&target_node, &build_path, &build_type);
        let expected = indoc::formatdoc!(
                    "\
                {target_name} : \
                    {object_files_string}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ -o $@)

        # Convenience rule for \"{target}\"
        {target}: {target_name}


",
                    target_name = target_name,
                    target_name_capitalized = target_node.borrow().name().to_uppercase(),
                    target = target_node.borrow().name(),
                    object_files_string = generate_prerequisites(&target_node, &build_path, &build_type),
                    dependencies = generate_search_directories(&target_node),
                );
        assert_eq!(actual, expected);
    }

    #[test]
    fn factory_creates_executable_target_declaration() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();
        let build_path = fixture.build_dir.path();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Executable("x".to_string()))
            .with_include_directories(include_directories)
            .create();

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        let build_type = configurations::BuildType::Debug;
        let target_name = target_node.borrow().name();

        let actual = TargetRuleFactory::create_rule(&target_node, &build_path, &build_type);
        let expected = indoc::formatdoc!(
            "
        {target_name} : \
            {object_files_string}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)
",
            target_name = target_name,
            target_name_capitalized = target_name.to_uppercase(),
            object_files_string = generate_prerequisites(&target_node, &build_path, &build_type),
            dependencies = generate_search_directories(&target_node),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn generator_produces_target_rule_for_single_target() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let mut fixture = MakefileGeneratorTestFixture::new();
        let build_path = fixture.build_dir.path();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();
        let generator = MakefileGenerator::new(
            &fixture.configuration_opts,
            &fixture.build_directory(),
            fixture.compiler,
        )
        .unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Executable("x".to_string()))
            .with_include_directories(include_directories)
            .create();

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        generator.generate_rule_declaration_for_target(&mut fixture.writers, &target_node);
        let build_type = configurations::BuildType::Debug;
        let actual = fixture.writers.makefile_writer.data;
        let target_name = target_node.borrow().name();
        let expected = indoc::formatdoc!(
            "


        # Phony for target \"{target_name}\"
        .PHONY: {target_name}

        # CXXFLAGS for target \"{target_name}\"
        {target_name_capitalized}_CXXFLAGS +=

        # CPPFLAGS for target \"{target_name}\"
        {target_name_capitalized}_CPPFLAGS +=

        # Rule for target \"{target_name}\"
        {target_name} : \
            {object_files_string}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)


",
            target_name = target_name,
            target_name_capitalized = target_name.to_uppercase(),
            object_files_string = generate_prerequisites(&target_node, &build_path, &build_type),
            dependencies = generate_search_directories(&target_node),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn generator_produces_object_target_rule_for_single_target() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let mut fixture = MakefileGeneratorTestFixture::new();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();
        let generator = MakefileGenerator::new(
            &fixture.configuration_opts,
            &fixture.build_directory(),
            fixture.compiler,
        )
        .unwrap();

        let manifest = ManifestStub::new(project_path);
        let mut source_files = build_target::associated_files::SourceFiles::new();
        source_files.push(create_source_file(&source_directory.join("a.cpp")));
        source_files.push(create_source_file(&source_directory.join("b.cpp")));
        source_files.push(create_source_file(&source_directory.join("c.cpp")));
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Executable("x".to_string()))
            .with_include_directories(include_directories)
            .create();

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        let object_targets = generator.create_object_targets(&target_node);
        fixture
            .writers
            .makefile_writer
            .object_targets
            .extend_from_slice(&object_targets);
        generator.generate_object_rules(&mut fixture.writers);
        let actual = fixture.writers.makefile_writer.data;

        let expected = {
            let mut formatted_string = String::new();
            let object_targets = generator.create_object_targets(&target_node);
            for object_target in object_targets {
                let object = object_target.object;
                formatted_string.push_str(&indoc::formatdoc!(
                    "
            # Build rule for {object}
            {object}: \\
            \t{source}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target}_CXXFLAGS) $({target}_CPPFLAGS) $(WARNINGS) {dependencies} $< -c -o $@)


",
                    object = object.display(),
                    source = object_target.source.display(),
                    target = object_target.target.to_uppercase(),
                    dependencies = generate_search_directories(&target_node),
                ));
            }
            formatted_string
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn generator_produces_makefile_for_single_target() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let mut fixture = MakefileGeneratorTestFixture::new();
        let build_path = fixture.build_dir.path();
        let source_directory = project_path.join("src");
        std::fs::create_dir(&source_directory).unwrap();
        let mut generator = MakefileGenerator::new(
            &fixture.configuration_opts,
            &fixture.build_directory(),
            fixture.compiler,
        )
        .unwrap();

        let manifest = ManifestStub::new(project_path);
        let mut source_files = build_target::associated_files::SourceFiles::new();
        source_files.push(create_source_file(&source_directory.join("a.cpp")));
        source_files.push(create_source_file(&source_directory.join("b.cpp")));
        source_files.push(create_source_file(&source_directory.join("c.cpp")));
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files.clone(),
        });

        let mut include_directories = build_target::include_directories::IncludeDirectories::new();
        include_directories.add(create_include_directory(&project_path.join("include")));

        let target_node = TargetNodeStub::builder()
            .with_target_source(target_source)
            .with_target_type(build_target::TargetType::Executable("x".to_string()))
            .with_include_directories(include_directories)
            .create();

        project_fixture
            .target_registry
            .add_target(target_node.clone());

        generator
            .generate_makefile(&mut fixture.writers, &project_fixture.target_registry)
            .unwrap();
        let actual = fixture.writers.makefile_writer.data;
        let object_targets = generator.create_object_targets(&target_node);
        let object_target_rules = {
            let mut formatted_string = String::new();

            for object_target in &object_targets {
                formatted_string.push_str(&generate_object_target(&object_target));
            }
            formatted_string
        };

        let depends_rules = {
            let mut formatted_string = String::new();
            for object_target in &object_targets {
                let object = object_target.object.as_path();
                let depend_file = object.with_extension("d");
                formatted_string
                    .push_str(&format!("# Silently include {}\n", depend_file.display()));
                formatted_string.push_str(&format!("sinclude {}\n", depend_file.display()));
            }
            formatted_string
        };

        let build_type = configurations::BuildType::Debug;
        let target_name = target_node.borrow().name();

        let expected = indoc::formatdoc!(
            "\
        # ----- INCLUDES -----\n\
        include {build_directory}/make_include/strict.mk\n\
        include {build_directory}/make_include/default_make.mk\n\
        include {build_directory}/make_include/debug.mk\n\
        \n\
        # ----- DEFAULT PHONIES -----\n\
        \n\
        .SUFFIXES:         # We do not use suffixes on makefiles.\n\
        .PHONY: all\n\
        .PHONY: package\n\
        .PHONY: install\n\
        .PHONY: uninstall\n\
        .PHONY: clean
        # Default all target to build all targets.\n\
        all : \\
           {target_name}\n

        # Phony for target \"{target_name}\"
        .PHONY: {target_name}

        # CXXFLAGS for target \"{target_name}\"
        {target_name_capitalized}_CXXFLAGS +=

        # CPPFLAGS for target \"{target_name}\"
        {target_name_capitalized}_CPPFLAGS +=

        # Rule for target \"{target_name}\"
        {target_name} : \
            {object_files_string}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) \
            $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)

        {object_target_rules}
        {depends_rules}
",
            build_directory = generator.build_directory.as_path().display(),
            target_name = target_name,
            target_name_capitalized = target_name.to_uppercase(),
            object_files_string = generate_prerequisites(&target_node, &build_path, &build_type),
            dependencies = generate_search_directories(&target_node),
            object_target_rules = object_target_rules,
            depends_rules = depends_rules
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn add_cpp_version_cpp98_test() -> Result<(), GeneratorError> {
        let _lock = EnvLock::lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++98");
        assert_eq!(gen.args["C++"], "c++98");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp11_test() -> Result<(), GeneratorError> {
        let _lock = EnvLock::lock("CXX", "gcc");
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++11");
        assert_eq!(gen.args["C++"], "c++11");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp14_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++14");
        assert_eq!(gen.args["C++"], "c++14");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_uppercase_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("C++17");
        assert_eq!(gen.args["C++"], "c++17");
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp20_test() -> Result<(), GeneratorError> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
        let mut gen = construct_generator(&output_directory);
        gen.add_cpp_version("c++20");
        assert_eq!(gen.args["C++"], "c++20");
        Ok(())
    }

    #[test]
    fn generate_strict_mk_test() -> std::io::Result<()> {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let _lock = EnvLock::lock("CXX", "gcc");
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
        let output_directory = produce_include_path(TempDir::new("example").unwrap());

        {
            let _lock = EnvLock::lock("CXX", "gcc");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }

        {
            let _lock = EnvLock::lock("CXX", "/usr/bin/gcc");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }

        {
            let _lock = EnvLock::lock("CXX", "g++");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }
        {
            let _lock = EnvLock::lock("CXX", "/usr/bin/g++");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "true");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "false");
        }
    }

    #[test]
    fn evaluate_compiler_with_clang_results_in_clang_set() {
        let output_directory = produce_include_path(TempDir::new("example").unwrap());
        {
            let _lock = EnvLock::lock("CXX", "clang");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "false");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "true");
        }
        {
            let _lock = EnvLock::lock("CXX", "/usr/bin/clang");
            let gen = construct_generator(&output_directory);
            assert_eq!(gen.compiler_constants["CXX_USES_GCC"], "false");
            assert_eq!(gen.compiler_constants["CXX_USES_CLANG"], "true");
        }
    }
}
