use std::io::BufRead;

use anyhow::Context;
use colored::Colorize;
use regex::Regex;
use structopt::StructOpt;

use yambs::build_state_machine::*;
use yambs::cache::Cache;
use yambs::cli::command_line::{BuildOpts, CommandLine, RemakeOpts, Subcommand};
use yambs::compiler;
use yambs::dependency::target::{target_registry::TargetRegistry, TargetNode, TargetState};
use yambs::external;
use yambs::generator::MakefileGenerator;
use yambs::logger;
use yambs::output::Output;
use yambs::utility;

fn main() -> anyhow::Result<()> {
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
        output.status("Evaluating compiler by doing a sample build...\n");
        compiler.evaluate(&test_dir)?;
        cache.cache(compiler)?;
        output.status("Evaluating compiler by doing a sample build... done\n");
    }
    Ok(())
}

fn do_build(opts: &BuildOpts, output: &Output) -> anyhow::Result<()> {
    let logger = logger::Logger::init(opts.build_directory.as_path(), log::LevelFilter::Trace)?;
    log_invoked_command();
    let cache = Cache::new(&opts.build_directory)?;
    let compiler = compiler::Compiler::new()?;
    let mut dependency_registry = TargetRegistry::new();

    evaluate_compiler(&compiler, &opts, &cache, &output)?;

    let mut generator = MakefileGenerator::new(&opts.build_directory, compiler);
    let mut build_manager = BuildManager::new(&mut generator);

    build_manager
        .configure(&opts)
        .context("An error occured when configuring the project.")?;

    parse_and_register_dependencies(
        &mut build_manager,
        &opts.input_file,
        &output,
        &mut dependency_registry,
    )?;

    if opts.create_dottie_graph {
        return create_dottie_graph(&build_manager, &output);
    }

    generate_makefiles(&mut build_manager, &output, opts)?;

    build_project(&mut build_manager, &output, opts, &logger)?;
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
    output: &Output,
    opts: &BuildOpts,
) -> anyhow::Result<()> {
    build_manager.generate_makefiles()?;
    output.status(&format!(
        "Build files generated in {}\n",
        opts.build_directory.as_path().display()
    ));
    Ok(())
}

fn parse_and_register_dependencies(
    build_manager: &mut BuildManager,
    top_path: &std::path::Path,
    output: &Output,
    dep_registry: &mut TargetRegistry,
) -> anyhow::Result<()> {
    build_manager.parse_and_register_dependencies(dep_registry, top_path)?;
    let number_of_yambs_files = dep_registry.number_of_targets();
    output.status(&format!("Read {} Yambs files\n", number_of_yambs_files));
    Ok(())
}

fn create_dottie_graph(build_manager: &BuildManager, output: &Output) -> anyhow::Result<()> {
    let mut dottie_buffer = String::new();
    for target in build_manager.targets() {
        if external::dottie(target, false, &mut dottie_buffer).is_ok() {
            output.status(&format!(
                "Created dottie file dependency-{}.gv\n",
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
    for target in build_manager.targets() {
        let process_output = build_dependency(
            build_manager,
            target,
            opts.build_directory.as_path(),
            output,
            opts,
        );
        let build_status_message = {
            if process_output.is_ok() && process_output.unwrap().status.success() {
                format!("{}\n", "Build SUCCESS".green())
            } else {
                format!("{}\n", "Build FAILED".red())
            }
        };
        output.status(&build_status_message);
    }
    let log_path = logger.path();
    output.status(&format!(
        "Build log available at {:?}\n",
        log_path.display()
    ));
    Ok(())
}

pub fn build_dependency(
    build_manager: &BuildManager,
    dependency: &TargetNode,
    build_path: &std::path::Path,
    output: &Output,
    opts: &BuildOpts,
) -> anyhow::Result<std::process::Output> {
    let build_directory = build_manager.resolve_build_directory(build_path);
    for required_dependency in &dependency.borrow().dependencies {
        let borrowed_required_dependency = required_dependency.borrow();
        let project_name = borrowed_required_dependency.project_name();
        let build_path_dep = &build_directory.join("libs").join(project_name);

        if required_dependency.borrow().state == TargetState::BuildComplete {
            let top_build_directory_resolved =
                build_manager.resolve_build_directory(opts.build_directory.as_path());
            let directory_to_link = top_build_directory_resolved.join("libs").join(project_name);

            if !build_path_dep.is_dir() {
                utility::create_symlink(directory_to_link, build_path_dep)?;
            }

            // Se eventuelt etter annen løsning.
            continue;
        }

        required_dependency.borrow_mut().state = TargetState::Building;
        let dep_output = build_dependency(
            build_manager,
            &required_dependency,
            build_path_dep,
            output,
            opts,
        )?;
        if !dep_output.status.success() {
            return Ok(dep_output);
        }

        required_dependency.borrow_mut().state = TargetState::BuildComplete;
    }

    dependency.borrow_mut().state = TargetState::Building;

    let change_directory_message = format!("Entering directory {}\n", build_directory.display());
    if opts.verbose {
        output.status(&change_directory_message);
    }
    std::env::set_current_dir(&build_directory).with_context(|| {
        format!(
            "Failed to change directory to {}",
            build_directory.display()
        )
    })?;
    output.status(&construct_build_message(dependency));

    let process_output = build_manager.make().spawn(output)?;
    dependency.borrow_mut().state = TargetState::BuildComplete;

    Ok(process_output)
}

fn construct_build_message(dependency: &TargetNode) -> String {
    let dep_type = if dependency.borrow().is_executable() {
        "executable"
    } else {
        "library"
    };
    let dep_type_name = dependency.borrow().name();

    let target = format!("{} {}", dep_type, dep_type_name);
    format!("Building {}\n", target)
}
