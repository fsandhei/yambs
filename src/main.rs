use std::cell::RefCell;
use std::rc::Rc;

use anyhow::Context;
use clap::CommandFactory;
use clap::Parser;
use colored::Colorize;
use regex::Regex;
use std::io::BufRead;
use std::path::Path;
use yambs::toolchain::ToolchainError;

use parser::types::Language;
use yambs::build_target::{target_registry::TargetRegistry, BuildTarget};
use yambs::cli::command_line::{BuildOpts, CommandLine, ManifestDirectory, RemakeOpts, Subcommand};
use yambs::cli::configurations::BuildType;
use yambs::compiler::Compiler;
use yambs::generator::{
    makefile::make::BuildProcess, makefile::Make, Generator, GeneratorType, MakefileGenerator,
};
use yambs::logger;
use yambs::manifest;
use yambs::output;
use yambs::output::Output;
use yambs::parser;
use yambs::progress;
use yambs::toolchain::{NormalizedToolchain, TOOLCHAIN_FILE_NAME};
use yambs::ProjectConfig;
use yambs::YAMBS_MANIFEST_NAME;
use yambs::{YAMBS_BUILD_DIR_VAR, YAMBS_BUILD_TYPE, YAMBS_MANIFEST_DIR};

fn main() -> anyhow::Result<()> {
    let command_line = CommandLine::parse();
    let output = Output::new();

    if command_line.show_version {
        let version = env!("CARGO_PKG_VERSION");
        println!("{}", version);
        return Ok(());
    }

    if let Some(subcommand) = command_line.subcommand {
        match subcommand {
            Subcommand::Build(mut build_opts) => do_build(&mut build_opts, &output)?,
            Subcommand::Remake(ref remake_opts) => do_remake(remake_opts)?,
        }
    } else {
        CommandLine::command().print_help()?;
        println!();
        std::process::exit(0);
    }
    Ok(())
}

fn log_invoked_command() {
    log::info!(
        "Command line: {}",
        std::env::args()
            .map(|mut s| {
                s.push(' ');
                s
            })
            .collect::<String>()
    )
}

fn initialize_preset_variables(opts: &BuildOpts) -> anyhow::Result<()> {
    YAMBS_BUILD_DIR_VAR
        .set(opts.build_directory.clone())
        .map_err(|_| anyhow::anyhow!("Error occured fetching build directory"))?;
    YAMBS_MANIFEST_DIR
        .set(opts.manifest_dir.clone())
        .map_err(|_| anyhow::anyhow!("Error occurred fetching manifest directory"))?;
    YAMBS_BUILD_TYPE
        .set(opts.configuration.build_type.clone())
        .map_err(|_| anyhow::anyhow!("Error occured fetching build type"))?;
    Ok(())
}

fn evaluate_compiler(
    toolchain: &Rc<RefCell<NormalizedToolchain>>,
    project_config: &ProjectConfig,
) -> anyhow::Result<()> {
    let toolchain = toolchain.borrow();
    let language = &project_config.language;
    let compiler: Box<dyn Compiler> = match language {
        Language::CXX => Box::new(toolchain.cxx.compiler.clone()),
        Language::C => Box::new(toolchain.cc.compiler.clone()),
    };
    log::trace!("evaluate_compiler");
    let test_dir = project_config.build_directory.as_path().join("sample");
    log::debug!("Evaluating compiler by doing a sample build...");
    compiler.evaluate(&test_dir)?;
    log::debug!("Evaluating compiler by doing a sample build... done");
    Ok(())
}

fn detect_toolchain_file(toolchain_file: &Path) -> anyhow::Result<NormalizedToolchain> {
    log::debug!(
        "Using toolchain file located at {}",
        toolchain_file.display()
    );
    let toolchain = NormalizedToolchain::from_file(toolchain_file)
        .with_context(|| "Error occured when parsing toolchain file")?;
    Ok(toolchain)
}

fn locate_manifest(manifest_dir: &ManifestDirectory) -> anyhow::Result<std::path::PathBuf> {
    let manifest_file = manifest_dir.as_path().join(YAMBS_MANIFEST_NAME);

    if !manifest_file.is_file() {
        anyhow::bail!(
            "Could not locate manifest file in {}",
            manifest_dir.as_path().display()
        );
    }
    Ok(manifest_file)
}

pub fn construct_generator(
    project_config: &ProjectConfig,
    toolchain: &Rc<RefCell<NormalizedToolchain>>,
) -> anyhow::Result<Box<dyn Generator>> {
    let generator_type = &project_config.generator_type;
    log::info!("Using {:?} as generator.", generator_type);
    match generator_type {
        GeneratorType::GNUMakefiles => Ok(Box::new(MakefileGenerator::new(
            &project_config,
            toolchain.clone(),
        )?) as Box<dyn Generator>),
    }
}

fn do_build(opts: &BuildOpts, output: &Output) -> anyhow::Result<()> {
    let logger = logger::Logger::init(opts.build_directory.as_path(), log::LevelFilter::Trace)?;
    log_invoked_command();

    initialize_preset_variables(opts)?;
    log::trace!("do_build");

    let mut dependency_registry = TargetRegistry::new();
    let manifest_path = locate_manifest(&opts.manifest_dir)?;
    let manifest = parser::parse(&manifest_path).with_context(|| "Failed to parse manifest")?;

    // override the command line settings if there are configurations set in the manifest
    let std = if let Some(ref std) = opts.configuration.standard {
        log::info!("Using standard {} given on command line", std.to_string());
        std.clone()
    } else {
        if let Some(std) = manifest
            .data
            .project_config
            .as_ref()
            .and_then(|pc| pc.std.clone())
        {
            log::info!("Using standard {} found in manifest", std.to_string());
            std
        } else {
            anyhow::bail!(
                "No standard is set! Please set one either through command line or in manifest."
            );
        }
    };

    let language = if let Some(language) = manifest
        .data
        .project_config
        .as_ref()
        .and_then(|pc| pc.language.clone())
    {
        log::info!("Using language {} found in manifest", language.to_string());
        language
    } else {
        log::warn!("No language specified. Using C++");
        Language::CXX
    };

    let project_config = ProjectConfig {
        std,
        language,
        build_directory: opts.build_directory.clone(),
        build_type: opts.configuration.build_type.clone(),
        generator_type: opts.configuration.generator_type.clone(),
        defines: opts.configuration.defines.clone(),
    };

    // FIXME: The logic here is quirky. It is easy to mess up and understand the flow.
    // Can it be simplified?
    // There should be made an integration test for this to check if it is working as intended.
    let toolchain = {
        match detect_toolchain_file(
            &opts
                .manifest_dir
                .as_path()
                .join(".yambs")
                .join(TOOLCHAIN_FILE_NAME),
        ) {
            Ok(tc) => Ok(tc),
            Err(e) => {
                let tc_err = e.downcast::<ToolchainError>().unwrap();
                match tc_err {
                    ToolchainError::ToolchainNotFound(_) => {
                        log::warn!("Failed to find project-local toolchain.");
                        log::info!(
                    "Attempt finding toolchain from $HOME directory: $HOME/.yambs/toolchain.toml"
                );
                        let home_dir =
                            home::home_dir().context("Failed to locate user's HOME directory")?;
                        detect_toolchain_file(&home_dir.join(".yambs").join(TOOLCHAIN_FILE_NAME))
                    }
                    _ => return Err(anyhow::anyhow!(tc_err)),
                }
            }
        }
    };

    let toolchain = match toolchain {
        Ok(tc) => tc,
        Err(e) => {
            let tc_err = e.downcast_ref::<ToolchainError>().unwrap();
            match tc_err {
                ToolchainError::FailedToParseToolchainFile(_, _) => return Err(e),
                _ => {
                    println!("Warning: Did not find any toolchain file. Attempt using CXX value");
                    match NormalizedToolchain::new() {
                        Ok(tc) => tc,
                        Err(_) => {
                            anyhow::bail!(
                                "
    Failed to get information about toolchain.
    A toolchain has to be provided to yambs in order to work.
    It is recommended to specify it through a file located in .yambs/toolchain.toml.

    At the very minimum you can set CXX or CC, and yambs will attempt to find minimum other settings required."
                            )
                        }
                    }
                }
            }
        }
    };

    let toolchain = Rc::new(RefCell::new(toolchain));

    evaluate_compiler(&toolchain, &project_config)?;

    let mut generator = construct_generator(&project_config, &toolchain)?;
    parse_and_register_dependencies(
        &manifest,
        output,
        &mut dependency_registry,
        &toolchain,
        &opts.configuration.build_type,
    )
    .with_context(|| "An error occured when registering project dependencies")?;

    let buildfile_directory = generate_build_files(&mut generator, &dependency_registry, opts)?;

    build_project(&buildfile_directory, output, opts, &logger)?;
    Ok(())
}

fn do_remake(opts: &RemakeOpts) -> anyhow::Result<()> {
    let log_file = &opts.build_directory.as_path().join(logger::YAMBS_LOG_FILE);
    let log_fh = std::fs::File::open(log_file).context("Failed to find log file")?;
    let mut reader = std::io::BufReader::new(log_fh);
    let mut line = String::new();
    let line_length = reader
        .read_line(&mut line)
        .context("Failed to read line from log file")?;
    if line_length == 0 {
        anyhow::bail!("Could not find first line of log file");
    }

    let command_line_regex = Regex::new(r"Command line:\s(?P<cmd>.*)").unwrap();
    let caps = command_line_regex.captures(&line).unwrap();
    let invoked_command = caps.name("cmd").unwrap().as_str();
    println!("{}", invoked_command);
    Ok(())
}

fn generate_build_files(
    generator: &mut Box<dyn Generator>,
    registry: &TargetRegistry,
    opts: &BuildOpts,
) -> anyhow::Result<std::path::PathBuf> {
    log::trace!("generate_build_files");
    let buildfile_directory = generator.generate(registry)?;
    log::debug!(
        "Build files generated in {}",
        opts.build_directory.as_path().display()
    );
    Ok(buildfile_directory)
}

fn parse_and_register_dependencies(
    manifest: &manifest::ParsedManifest,
    output: &Output,
    dep_registry: &mut TargetRegistry,
    toolchain: &Rc<RefCell<NormalizedToolchain>>,
    build_type: &BuildType,
) -> anyhow::Result<()> {
    log::trace!("parse_and_register_dependencies");
    let manifest_path = manifest.manifest.directory.join(YAMBS_MANIFEST_NAME);
    for build_target in &manifest.data.targets {
        if let Some(lib) = build_target.library() {
            log::debug!(
                "Creating build target for library {} in manifest {}",
                lib.name,
                manifest_path.display()
            );
        }
        if let Some(exe) = build_target.executable() {
            log::debug!(
                "Creating build target for executable {} in manifest {}",
                exe.name,
                manifest_path.display()
            );
        }
        BuildTarget::target_node_from_source(
            &manifest.manifest.directory,
            build_target,
            dep_registry,
            toolchain,
            build_type,
        )?;
    }
    let number_of_targets = dep_registry.number_of_targets();
    output.status(&format!("Registered {} build targets", number_of_targets));
    Ok(())
}

fn run_make(args: &[String], makefile_directory: &std::path::Path) -> anyhow::Result<BuildProcess> {
    std::env::set_current_dir(makefile_directory).with_context(|| {
        format!(
            "Could not access directory {}",
            makefile_directory.display()
        )
    })?;
    let make = Make::new(args)?;

    log::debug!("Running make in directory {}", makefile_directory.display());
    let build_process = make.run()?;
    Ok(build_process)
}

fn build_project(
    buildfile_directory: &std::path::Path,
    output: &Output,
    opts: &BuildOpts,
    logger: &logger::Logger,
) -> anyhow::Result<()> {
    log::trace!("build_project");
    let output_clone = output.clone();
    let progress_path = buildfile_directory.to_path_buf();
    let owned_buildfile_directory = buildfile_directory.to_path_buf();
    let mut make_args = opts.make_args.clone();
    if let Some(ref target) = opts.target {
        log::debug!("Found specified target. \"{}\" will be built.", target);
        make_args.push(target.clone());
    }
    let target = opts.target.clone();

    let make_thread = std::thread::spawn(move || {
        let mut build_process = run_make(&make_args, &owned_buildfile_directory).unwrap();

        build_process.wait_and_log(&output_clone)
    });

    let mut progress = progress::Progress::new(&progress_path, target)?;

    let pb = output::ProgressBar::new(progress.total);

    let mut joinable = make_thread.is_finished();
    while !joinable {
        let msg = format!("[{}/{}] Building...", progress.current, progress.total);
        pb.bar.set_message(msg);
        pb.bar.set_position(progress.current);
        progress.update()?;
        joinable = make_thread.is_finished();
    }

    let exit_status = make_thread.join().unwrap().unwrap();
    let process_code = exit_status.code();
    match process_code {
        Some(0) => {
            let msg = format!("{}", "Build SUCCESS".green());
            pb.finish_with_message(msg);
        }
        _ => {
            let msg = format!("{}", "Build FAILED".red());
            pb.fail_with_message(msg);
        }
    }
    let log_path = logger.path();
    output.status(&format!("Build log available at {:?}", log_path.display()));
    Ok(())
}
