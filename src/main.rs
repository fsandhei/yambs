use std::io::BufRead;
use std::path::Path;

use anyhow::Context;
use clap::CommandFactory;
use clap::Parser;
use colored::Colorize;
use regex::Regex;

use yambs::build_target::{target_registry::TargetRegistry, BuildTarget};
use yambs::cache::Cache;
use yambs::cli::command_line::{BuildOpts, CommandLine, ManifestDirectory, RemakeOpts, Subcommand};
use yambs::compiler;
use yambs::external;
use yambs::generator::{
    makefile::Make, Generator, GeneratorInfo, GeneratorType, MakefileGenerator,
};
use yambs::logger;
use yambs::manifest;
use yambs::output;
use yambs::output::Output;
use yambs::parser;
use yambs::progress;
use yambs::toolchain::{Toolchain, TOOLCHAIN_FILE_NAME};
use yambs::YAMBS_MANIFEST_NAME;

fn main() -> anyhow::Result<()> {
    let command_line = CommandLine::parse();
    let output = Output::new();

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

fn evaluate_compiler(
    compiler: &compiler::CXXCompiler,
    opts: &BuildOpts,
    cache: &Cache,
) -> anyhow::Result<()> {
    log::trace!("evaluate_compiler");
    if !cache.detect_change(compiler) {
        let test_dir = opts.build_directory.as_path().join("sample");
        log::debug!("Evaluating compiler by doing a sample build...");
        compiler.evaluate(&test_dir)?;
        cache.cache(compiler)?;
        log::debug!("Evaluating compiler by doing a sample build... done");
    }
    Ok(())
}

fn detect_toolchain_file(toolchain_file: &Path, cache: &Cache) -> anyhow::Result<Toolchain> {
    if !toolchain_file.exists() {
        log::debug!("Toolchain file does not exist. Using default setup found on system.");
        if let Some(toolchain) = Toolchain::from_cache(cache) {
            return Ok(toolchain);
        } else {
            return Toolchain::new().with_context(|| "Error occured when parsing toolchain");
        }
    }
    log::debug!(
        "Using toolchain file located at {}",
        toolchain_file.display()
    );
    let toolchain = Toolchain::from_file(toolchain_file)
        .with_context(|| "Error occured when parsing toolchain file")?;
    if let Some(cached_toolchain) = cache.from_cache::<Toolchain>() {
        log::debug!("Found cached toolchain. Checking for equality.");
        if toolchain != cached_toolchain {
            anyhow::bail!("Cached toolchain is not the same as the current toolchain, invalidating the next build. Clean the build directory and run yambs again.");
        }
        Ok(cached_toolchain)
    } else {
        cache.cache(&toolchain)?;
        Ok(toolchain)
    }
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

pub fn generator_from_build_opts(
    opts: &BuildOpts,
    cache: &Cache,
) -> anyhow::Result<Box<dyn Generator>> {
    let toolchain = detect_toolchain_file(
        &opts
            .manifest_dir
            .as_path()
            .join(".yambs")
            .join(TOOLCHAIN_FILE_NAME),
        cache,
    ).with_context(|| "
    Failed to get information about toolchain.
    A toolchain has to be provided to yambs in order to work.
    It is recommended to specify it through a file located in .yambs/toolchain.toml.

    At the very minimum you can set CXX, and yambs will attempt to find minimum other settings required.")?;
    evaluate_compiler(&toolchain.cxx_compiler, opts, cache)?;

    let generator_type = &opts.configuration.generator_type;
    log::info!("Using {:?} as generator.", generator_type);
    match generator_type {
        GeneratorType::GNUMakefiles => Ok(Box::new(MakefileGenerator::new(
            &opts.configuration,
            &opts.build_directory,
            toolchain,
        )?) as Box<dyn Generator>),
    }
}

fn try_cached_manifest(
    cache: &Cache,
    dep_registry: &mut TargetRegistry,
    manifest: &manifest::ParsedManifest,
) -> Option<manifest::ParsedManifest> {
    log::trace!("try_cached_manifest");
    log::debug!("Checking for cache of manifest.");
    if let Some(cached_manifest) = cache.from_cache::<manifest::ParsedManifest>() {
        log::debug!("Found cached manifest. Checking if it is up to date.");
        if manifest.manifest.modification_time <= cached_manifest.manifest.modification_time {
            check_dependencies_for_up_to_date(cache)?;
            log::debug!("Cached manifest is up to date! Using it for this build.");
            let cached_registry = TargetRegistry::from_cache(cache)?;
            *dep_registry = cached_registry;
            return Some(cached_manifest);
        }
        log::debug!("Cached manifest is older than latest manifest. Discarding cached.");
    }
    None
}

fn check_dependencies_for_up_to_date(cache: &Cache) -> Option<()> {
    let test_target_registry = TargetRegistry::from_cache(cache)?;
    for target in test_target_registry.registry {
        if let Some(source_data) = target.borrow().target_source.from_source() {
            let manifest_compare = manifest::Manifest::new(&source_data.manifest.directory);
            if source_data.manifest.modification_time < manifest_compare.modification_time {
                return None;
            }
            log::debug!("{} is up to date", target.borrow().name());
        }
    }
    Some(())
}

fn do_build(opts: &mut BuildOpts, output: &Output) -> anyhow::Result<()> {
    let logger = logger::Logger::init(opts.build_directory.as_path(), log::LevelFilter::Trace)?;
    log_invoked_command();
    log::trace!("do_build");

    let cache = Cache::new(opts.build_directory.as_path())?;

    let mut dependency_registry = TargetRegistry::new();
    let manifest_path = locate_manifest(&opts.manifest_dir)?;
    let manifest = parser::parse(&manifest_path).with_context(|| "Failed to parse manifest")?;

    // override the command line settings if there are configurations set in the manifest
    if let Some(cxx_standard) = manifest
        .data
        .project_configuration
        .as_ref()
        .and_then(|pc| pc.cxx_std.clone())
    {
        opts.configuration.cxx_standard = cxx_standard;
    }

    let mut generator = generator_from_build_opts(&opts, &cache)?;

    let buildfile_directory =
        if try_cached_manifest(&cache, &mut dependency_registry, &manifest).is_none() {
            log::debug!("Did not find a cached manifest that suited. Making a new one.");
            parse_and_register_dependencies(&cache, &manifest, output, &mut dependency_registry)
                .with_context(|| "An error occured when registering project dependencies")?;

            let buildfile_directory =
                generate_build_files(&mut generator, &dependency_registry, &opts)?;

            cache.cache(&GeneratorInfo {
                type_: GeneratorType::GNUMakefiles,
                buildfile_directory: buildfile_directory.clone(),
            })?;
            buildfile_directory
        } else {
            let cached_generator_info = cache
                .from_cache::<GeneratorInfo>()
                .ok_or_else(|| anyhow::anyhow!("Could not retrieve generator cache"))?;
            cached_generator_info.buildfile_directory
        };

    // FIXME: This most likely does not work anymore...
    if opts.create_dottie_graph {
        return create_dottie_graph(&dependency_registry, output);
    }

    build_project(&buildfile_directory, output, &opts, &logger)?;
    cache.cache(&dependency_registry)?;
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
    cache: &Cache,
    manifest: &manifest::ParsedManifest,
    output: &Output,
    dep_registry: &mut TargetRegistry,
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
        )?;
    }
    cache
        .cache(manifest)
        .with_context(|| "Failed to cache manifest file")?;
    let number_of_targets = dep_registry.number_of_targets();
    output.status(&format!("Registered {} build targets", number_of_targets));
    Ok(())
}

fn create_dottie_graph(registry: &TargetRegistry, output: &Output) -> anyhow::Result<()> {
    let mut dottie_buffer = String::new();
    for target in &registry.registry {
        if external::dottie(target, registry, false, &mut dottie_buffer).is_ok() {
            output.status(&format!(
                "Created dottie file dependency-{}.gv",
                target.borrow().name()
            ));
        }
    }
    Ok(())
}

fn run_make(
    args: &[String],
    makefile_directory: &std::path::Path,
    output: &Output,
) -> anyhow::Result<std::process::Output> {
    std::env::set_current_dir(makefile_directory).with_context(|| {
        format!(
            "Could not access directory {}",
            makefile_directory.display()
        )
    })?;
    let make = Make::new(args)?;

    log::debug!("Running make in directory {}", makefile_directory.display());
    let build_process = make.run()?;
    Ok(build_process.wait_with_output(output))
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
        let process_output = run_make(&make_args, &owned_buildfile_directory, &output_clone)?;
        Ok::<std::process::Output, anyhow::Error>(process_output)
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

    let process_output = make_thread.join().unwrap()?;
    let process_code = process_output.status.code();
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
