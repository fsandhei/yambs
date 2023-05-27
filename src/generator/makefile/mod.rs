use std::io::Write;
use std::rc::Rc;

use indoc;

mod include_file_generator;
pub mod make;

use crate::build_target;
use crate::build_target::include_directories;
use crate::build_target::{
    include_directories::IncludeType, target_registry::TargetRegistry, Dependency, LibraryType,
    TargetNode, TargetState, TargetType,
};
use crate::cli::command_line;
use crate::cli::configurations;
use crate::cli::BuildDirectory;
use crate::errors::FsError;
use crate::generator;
use crate::generator::{
    targets::ObjectTarget, targets::ProgressDocument, targets::ProgressTrackingTarget, Generator,
    GeneratorError, UtilityGenerator, SHARED_LIBRARY_FILE_EXTENSION, STATIC_LIBRARY_FILE_EXTENSION,
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
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)",
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
                    \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) -rdynamic -shared {dependencies} $^ -o $@)\n\n",
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
                formatted_string.push_str(&format!("   {}", object.display()));
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
                formatted_string.push_str(" -L.");
            }
        }
    };
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
    pub toolchain: Rc<NormalizedToolchain>,
    pub configurations: command_line::ConfigurationOpts,
    pub build_directory: BuildDirectory,
    pub output_directory: std::path::PathBuf,
    pub progress_document: ProgressDocument,
}

impl MakefileGenerator {
    pub fn new(
        configurations: &command_line::ConfigurationOpts,
        build_directory: &BuildDirectory,
        toolchain: Rc<NormalizedToolchain>,
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
                match borrowed_target.target_source {
                    build_target::TargetSource::FromSource(ref s) => {
                        log::debug!(
                            "Generating makefiles for target {:?} (manifest path: {})",
                            target.borrow().name(),
                            s.manifest.directory.display()
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
                            s,
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
                    }
                }
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
                        let dep_dir = format!("{}.dir", &s.name);
                        self.push_and_create_directory(std::path::Path::new(&dep_dir))?;
                        self.generate_rule_for_dependency(writers, dependency, registry);
                        self.output_directory.pop();
                    }
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
        let mut include_file_generator =
            IncludeFileGenerator::new(&include_output_directory, &self.toolchain);

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
            generate_defines(&defines)
        } else {
            match borrowed_target.target_source {
                build_target::TargetSource::FromSource(ref s) => generate_defines(&s.defines),
            }
        };

        makefile_writer.data.push_str(&defines);

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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use tempdir::TempDir;

    use crate::flags;
    use crate::manifest;
    use crate::tests::EnvLock;

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
        compiler: CXXCompiler,
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

    fn create_compiler() -> CXXCompiler {
        let _lock = EnvLock::lock("CXX", "gcc");
        CXXCompiler::new().unwrap()
    }

    fn create_configuration_opts() -> command_line::ConfigurationOpts {
        let build_type = configurations::BuildType::Debug;
        let cxx_standard = configurations::CXXStandard::CXX17;
        let sanitizer = None;
        let generator_type = generator::GeneratorType::GNUMakefiles;

        command_line::ConfigurationOpts {
            build_type,
            cxx_standard,
            sanitizer,
            generator_type,
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
            source_files,
            defines: Vec::new(),
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
        }
    }

    // FIXME: Add test for generation of dependencies.

    #[test]
    fn populate_progress_document_generates_document_with_all_object_files_to_targets() {
        let mut project_fixture = ProjectTestFixture::new();
        let project_path = project_fixture.dir.path();
        let fixture = MakefileGeneratorTestFixture::new();

        let mut generator = MakefileGenerator::new(
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

        let progress_tracking_target =
            ProgressTrackingTarget::from_target(&target_node, &generator.output_directory);
        generator
            .progress_document
            .add_progress_tracking_target(progress_tracking_target);
        let progress_tracking_target_all = ProgressTrackingTarget {
            target: "all".to_string(),
            object_files: Vec::new(),
            dependencies: vec!["myLib".to_string()],
        };
        generator
            .progress_document
            .add_progress_tracking_target(progress_tracking_target_all);
        project_fixture.target_registry.add_target(target_node);

        let actual = generator.progress_document;
        let expected = generator::targets::ProgressDocument {
            targets: vec![
                ProgressTrackingTarget {
                    target: "myLib".to_string(),
                    object_files: object_files_paths,
                    dependencies: Vec::new(),
                },
                ProgressTrackingTarget {
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

        let mut generator = MakefileGenerator::new(
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

        let progress_tracking_target =
            ProgressTrackingTarget::from_target(&target_node, &generator.output_directory);
        generator
            .progress_document
            .add_progress_tracking_target(progress_tracking_target);
        let progress_tracking_target_all = ProgressTrackingTarget {
            target: "all".to_string(),
            object_files: Vec::new(),
            dependencies: vec!["myLib".to_string()],
        };
        generator
            .progress_document
            .add_progress_tracking_target(progress_tracking_target_all);
        project_fixture.target_registry.add_target(target_node);

        let actual = serde_json::to_string_pretty(&generator.progress_document).unwrap();
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
        std::fs::create_dir(source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files,
            defines: Vec::new(),
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

        let actual = TargetRuleFactory::create_rule(&target_node, build_path, &build_type);
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
            object_files_string = generate_prerequisites(&target_node, build_path, &build_type),
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
        std::fs::create_dir(source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files,
            defines: Vec::new(),
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

        let actual = TargetRuleFactory::create_rule(&target_node, build_path, &build_type);
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
                    object_files_string = generate_prerequisites(&target_node, build_path, &build_type),
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
        std::fs::create_dir(source_directory).unwrap();

        let manifest = ManifestStub::new(project_path);
        let source_files = build_target::associated_files::SourceFiles::new();
        let dependencies = Vec::<build_target::Dependency>::new();
        let target_source = build_target::TargetSource::FromSource(build_target::SourceBuildData {
            manifest: manifest.manifest,
            dependencies,
            source_files: source_files,
            defines: Vec::new(),
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

        let actual = TargetRuleFactory::create_rule(&target_node, build_path, &build_type);
        let expected = indoc::formatdoc!(
            "
        {target_name} : \
            {object_files_string}\n\
            \t$(strip $(CXX) $(CXXFLAGS) $(CPPFLAGS) $({target_name_capitalized}_CXXFLAGS) $({target_name_capitalized}_CPPFLAGS) $(WARNINGS) $(LDFLAGS) {dependencies} $^ -o $@)
",
            target_name = target_name,
            target_name_capitalized = target_name.to_uppercase(),
            object_files_string = generate_prerequisites(&target_node, build_path, &build_type),
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
        std::fs::create_dir(source_directory).unwrap();
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
            source_files,
            defines: Vec::new(),
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
            object_files_string = generate_prerequisites(&target_node, build_path, &build_type),
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
            defines: Vec::new(),
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

        let object_targets =
            ObjectTarget::create_object_targets(&target_node, &generator.output_directory);
        fixture
            .writers
            .makefile_writer
            .object_targets
            .extend_from_slice(&object_targets);
        generator
            .generate_object_rules(&mut fixture.writers)
            .unwrap();
        let actual = fixture.writers.makefile_writer.data;

        let expected = {
            let mut formatted_string = String::new();
            let object_targets =
                ObjectTarget::create_object_targets(&target_node, &generator.output_directory);
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
        let build_path = fixture.build_dir.path().join("deps/x.dir");
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
            defines: Vec::new(),
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
        let object_targets = ObjectTarget::create_object_targets(
            &target_node,
            &generator.output_directory.join("deps/x.dir"),
        );
        let object_target_rules = {
            let mut formatted_string = String::new();

            for object_target in &object_targets {
                formatted_string.push_str(&generate_object_target(object_target));
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
        include {build_directory}/make_include/warnings.mk\n\
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
}
