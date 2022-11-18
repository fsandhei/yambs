use std::io::BufRead;

use anyhow::Context;
use clap::CommandFactory;
use clap::Parser;
use colored::Colorize;
use regex::Regex;

use yambs::build_state_machine::BuildManager;
use yambs::build_target::target_registry::TargetRegistry;
use yambs::cache::Cache;
use yambs::cli::command_line::{BuildOpts, CommandLine, ManifestDirectory, RemakeOpts, Subcommand};
use yambs::compiler;
use yambs::external;
use yambs::generator::MakefileGenerator;
use yambs::logger;
use yambs::manifest;
use yambs::output::Output;
use yambs::parser;
use yambs::{YambsEnvironmentVariables, YAMBS_MANIFEST_NAME};

fn main() -> anyhow::Result<()> {
    let command_line = CommandLine::parse();
    let output = Output::new();
    let _environment_variables = YambsEnvironmentVariables::from_command_line(&command_line);

    if let Some(subcommand) = command_line.subcommand {
        match subcommand {
            Subcommand::Build(ref build_opts) => do_build(build_opts, &output)?,
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
    compiler: &compiler::Compiler,
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

fn do_build(opts: &BuildOpts, output: &Output) -> anyhow::Result<()> {
    let logger = logger::Logger::init(opts.build_directory.as_path(), log::LevelFilter::Trace)?;
    log_invoked_command();
    log::trace!("do_build");

    let cache = Cache::new(opts.build_directory.as_path())?;
    let compiler = compiler::Compiler::new()?;

    let mut dependency_registry = TargetRegistry::new();
    let manifest_path = locate_manifest(&opts.manifest_dir)?;
    let manifest = parser::parse(&manifest_path).with_context(|| "Failed to parse manifest")?;

    evaluate_compiler(&compiler, opts, &cache)?;

    let mut generator =
        MakefileGenerator::new(&opts.configuration, &opts.build_directory, compiler)?;
    let mut build_manager = BuildManager::new(&mut generator);

    build_manager
        .configure(opts)
        .context("An error occured when configuring the project.")?;

    if try_cached_manifest(&cache, &mut dependency_registry, &manifest).is_none() {
        log::debug!("Did not find a cached manifest that suited. Making a new one.");
        parse_and_register_dependencies(
            &mut build_manager,
            &cache,
            &manifest,
            output,
            &mut dependency_registry,
        )
        .with_context(|| "An error occured when registering project dependencies")?;

        generate_makefiles(&mut build_manager, &dependency_registry, opts)?;
    }

    // FIXME: This most likely does not work anymore...
    if opts.create_dottie_graph {
        return create_dottie_graph(&dependency_registry, output);
    }

    build_project(&mut build_manager, output, opts, &logger)?;
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

fn generate_makefiles(
    build_manager: &mut BuildManager,
    registry: &TargetRegistry,
    opts: &BuildOpts,
) -> anyhow::Result<()> {
    log::trace!("generate_makefiles");
    build_manager.generate_makefiles(registry)?;
    log::debug!(
        "Build files generated in {}",
        opts.build_directory.as_path().display()
    );
    Ok(())
}

fn parse_and_register_dependencies(
    build_manager: &mut BuildManager,
    cache: &Cache,
    manifest: &manifest::ParsedManifest,
    output: &Output,
    dep_registry: &mut TargetRegistry,
) -> anyhow::Result<()> {
    log::trace!("parse_and_register_dependencies");
    build_manager.parse_and_register_dependencies(dep_registry, manifest)?;
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

fn build_project(
    build_manager: &mut BuildManager,
    output: &Output,
    opts: &BuildOpts,
    logger: &logger::Logger,
) -> anyhow::Result<()> {
    log::trace!("build_project");
    let build_directory = build_manager.resolve_build_directory(opts.build_directory.as_path());
    let make_process = build_manager.make_mut().spawn_with_args(
        &build_directory,
        output,
        opts.make_args.clone(),
    )?;
    let process_code: Option<i32> = make_process.status.code();
    if process_code != Some(0) {
        output.status(&format!("{}", "Build FAILED".red()));
    }

    if process_code == Some(0) {
        output.status(&format!("{}", "Build SUCCESS".green()));
    }
    let log_path = logger.path();
    output.status(&format!("Build log available at {:?}", log_path.display()));
    Ok(())
}
