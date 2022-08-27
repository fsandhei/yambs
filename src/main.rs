use std::io::BufRead;

use anyhow::Context;
use colored::Colorize;
use regex::Regex;
use structopt::StructOpt;

use yambs::build_state_machine::*;
use yambs::cache::Cache;
use yambs::cli::command_line::{BuildOpts, CommandLine, RemakeOpts, Subcommand};
use yambs::compiler;
use yambs::dependency::{DependencyNode, DependencyRegistry, DependencyState};
use yambs::external;
use yambs::generator::MakefileGenerator;
use yambs::logger;
use yambs::output::Output;
use yambs::unwrap_or_terminate::MyMakeUnwrap;
use yambs::utility;

fn try_main() -> anyhow::Result<()> {
    let command_line = CommandLine::from_args();
    let output = Output::new();

    if let Some(subcommand) = command_line.subcommand {
        match subcommand {
            Subcommand::Build(ref build_opts) => do_build(build_opts, &output)?,
            Subcommand::Remake(ref remake_opts) => do_remake(remake_opts)?,
        }
    }
    Ok(())
}

fn main() {
    try_main().unwrap_or_terminate();
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
    output: &Output,
) -> anyhow::Result<()> {
    if !cache.detect_change(compiler) {
        let test_dir = opts.build_directory.as_path().join("sample");
        output.status("Evaluating compiler by doing a sample build...");
        compiler.evaluate(&test_dir)?;
        cache.cache(compiler)?;
        output.status("Evaluating compiler by doing a sample build... done");
    }
    Ok(())
}

fn do_build(opts: &BuildOpts, output: &Output) -> anyhow::Result<()> {
    let logger = logger::Logger::init(opts.build_directory.as_path(), log::LevelFilter::Trace)?;
    log_invoked_command();
    let cache = Cache::new(&opts.build_directory)?;
    let compiler = compiler::Compiler::new()?;
    let mut dependency_registry = DependencyRegistry::new();

    evaluate_compiler(&compiler, &opts, &cache, &output)?;

    let mut generator = MakefileGenerator::new(&opts.build_directory, compiler);
    let mut builder = BuildManager::new(&mut generator);

    builder
        .configure(&opts)
        .context("An error occured when configuring the project.")?;

    parse_and_register_dependencies(
        &mut builder,
        &opts.input_file,
        &output,
        &mut dependency_registry,
    )?;

    if opts.create_dottie_graph {
        return create_dottie_graph(&builder, &output);
    }

    generate_makefiles(&mut builder, &output, opts)?;

    build_project(&mut builder, &output, opts, &logger)?;
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
    builder: &mut BuildManager,
    output: &Output,
    opts: &BuildOpts,
) -> anyhow::Result<()> {
    builder.generate_makefiles()?;
    output.status(&format!(
        "Build files generated in {}",
        opts.build_directory.as_path().display()
    ));
    Ok(())
}

fn parse_and_register_dependencies(
    builder: &mut BuildManager,
    top_path: &std::path::Path,
    output: &Output,
    dep_registry: &mut DependencyRegistry,
) -> anyhow::Result<()> {
    builder.parse_and_register_dependencies(dep_registry, top_path)?;
    if builder.top_dependency().is_some() {
        let number_of_mmk_files = dep_registry.number_of_dependencies();
        output.status(&format!("Read {} Yambs files", number_of_mmk_files));
    }
    Ok(())
}

fn create_dottie_graph(builder: &BuildManager, output: &Output) -> anyhow::Result<()> {
    let mut dottie_buffer = String::new();
    if let Some(dependency) = builder.top_dependency() {
        if external::dottie(dependency, false, &mut dottie_buffer).is_ok() {
            output.status("Created dottie file dependency.gv");
        }
    }
    Ok(())
}

fn build_project(
    builder: &mut BuildManager,
    output: &Output,
    opts: &BuildOpts,
    logger: &logger::Logger,
) -> anyhow::Result<()> {
    if let Some(top_dependency) = &builder.top_dependency() {
        let process_output = build_dependency(
            builder,
            top_dependency,
            opts.build_directory.as_path(),
            output,
            opts,
        );
        let build_status_message = {
            if process_output.is_ok() && process_output.unwrap().status.success() {
                format!("{}", "Build SUCCESS".green())
            } else {
                format!("{}", "Build FAILED".red())
            }
        };
        output.status(&build_status_message);
        let log_path = logger.path();
        output.status(&format!("Build log available at {:?}", log_path.display()));
    }
    Ok(())
}

fn change_directory(directory: std::path::PathBuf) {
    std::env::set_current_dir(directory).unwrap()
}

pub fn build_dependency(
    builder: &BuildManager,
    dependency: &DependencyNode,
    build_path: &std::path::Path,
    output: &Output,
    opts: &BuildOpts,
) -> anyhow::Result<std::process::Output> {
    let build_directory = builder.resolve_build_directory(build_path);

    for required_dependency in dependency.dependency().ref_dep.requires() {
        let build_path_dep = &build_directory
            .join("libs")
            .join(required_dependency.dependency().ref_dep.get_project_name());

        if required_dependency
            .dependency()
            .ref_dep
            .is_build_completed()
        {
            let top_build_directory_resolved =
                builder.resolve_build_directory(opts.build_directory.as_path());
            let directory_to_link = top_build_directory_resolved
                .join("libs")
                .join(required_dependency.dependency().ref_dep.get_project_name());

            if !build_path_dep.is_dir() {
                utility::create_symlink(directory_to_link, build_path_dep)?;
            }

            // Se eventuelt etter annen løsning.
            continue;
        }

        required_dependency
            .dependency_mut()
            .ref_dep
            .change_state(DependencyState::Building);
        let dep_output =
            build_dependency(builder, required_dependency, build_path_dep, output, opts)?;
        if !dep_output.status.success() {
            return Ok(dep_output);
        }
        required_dependency
            .dependency_mut()
            .ref_dep
            .change_state(DependencyState::BuildComplete);
    }

    dependency
        .dependency_mut()
        .ref_dep
        .change_state(DependencyState::Building);

    let change_directory_message = format!("Entering directory {}\n", build_directory.display());
    if opts.verbose {
        output.status(&change_directory_message);
    }
    change_directory(build_directory);
    output.status(&construct_build_message(dependency));

    let output = builder.make().spawn(output)?;
    dependency
        .dependency_mut()
        .ref_dep
        .change_state(DependencyState::BuildComplete);

    Ok(output)
}

fn construct_build_message(dependency: &DependencyNode) -> String {
    let dep_type = if dependency.dependency().ref_dep.is_executable() {
        "executable"
    } else {
        "library"
    };
    let dep_type_name = dependency.dependency().ref_dep.get_name().unwrap();

    let green_building = format!("{}", "Building".green());
    let target = format!("{} {}", dep_type, dep_type_name);
    format!("{} {}", green_building, target)
}
