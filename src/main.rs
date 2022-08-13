use colored::Colorize;
use structopt::StructOpt;

use yambs::build_state_machine::*;
use yambs::cache::{Cache, Cacher};
use yambs::cli::command_line::CommandLine;
use yambs::compiler;
use yambs::dependency::{DependencyNode, DependencyState};
use yambs::errors::MyMakeError;
use yambs::external;
use yambs::generator::MakefileGenerator;
use yambs::logger;
use yambs::output::Output;
use yambs::unwrap_or_terminate::MyMakeUnwrap;
use yambs::utility;

fn try_main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let logger = logger::Logger::init(
        command_line.build_directory.as_path(),
        log::LevelFilter::Trace,
    )?;
    let output = Output::new();
    let cache = Cache::new(&command_line.build_directory)?;
    let compiler = compiler::Compiler::new()?;

    evaluate_compiler(&compiler, &command_line, &cache, &output)?;

    let mut generator = MakefileGenerator::new(&command_line.build_directory, compiler);
    let mut builder = BuildManager::new(&mut generator);

    builder
        .configure(&command_line)
        .map_err(MyMakeError::ConfigurationTime)?;

    parse_and_register_dependencies(&mut builder, &command_line.input_file, &output)?;

    if command_line.create_dottie_graph {
        return create_dottie_graph(&builder, &output);
    }

    generate_makefiles(&mut builder, &output, &command_line)?;

    build_project(&mut builder, &output, &command_line, &logger)?;
    Ok(())
}

fn main() {
    try_main().unwrap_or_terminate();
}

fn evaluate_compiler(
    compiler: &compiler::Compiler,
    command_line: &CommandLine,
    cache: &Cache,
    output: &Output,
) -> Result<(), MyMakeError> {
    if !compiler.is_changed(cache) {
        let test_dir = command_line.build_directory.as_path().join("sample");
        output.status("Evaluating compiler by doing a sample build...");
        compiler.evaluate(&test_dir)?;
        compiler.cache(cache)?;
        output.status("Evaluating compiler by doing a sample build... done");
    }
    Ok(())
}

fn generate_makefiles(
    builder: &mut BuildManager,
    output: &Output,
    command_line: &CommandLine,
) -> Result<(), MyMakeError> {
    builder.generate_makefiles()?;
    output.status(&format!(
        "Build files generated in {}",
        command_line.build_directory.as_path().display()
    ));
    Ok(())
}

fn parse_and_register_dependencies(
    builder: &mut BuildManager,
    top_path: &std::path::Path,
    output: &Output,
) -> Result<(), MyMakeError> {
    builder.parse_and_register_dependencies(top_path)?;
    if builder.top_dependency().is_some() {
        let number_of_mmk_files = builder.number_of_dependencies();
        output.status(&format!("Read {} Yambs files", number_of_mmk_files));
    }
    Ok(())
}

fn create_dottie_graph(builder: &BuildManager, output: &Output) -> Result<(), MyMakeError> {
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
    command_line: &CommandLine,
    logger: &logger::Logger,
) -> Result<(), MyMakeError> {
    if let Some(top_dependency) = &builder.top_dependency() {
        let process_output = build_dependency(
            builder,
            top_dependency,
            command_line.build_directory.as_path(),
            output,
            command_line,
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
    command_line: &CommandLine,
) -> Result<std::process::Output, MyMakeError> {
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
                builder.resolve_build_directory(command_line.build_directory.as_path());
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
        let dep_output = build_dependency(
            builder,
            required_dependency,
            build_path_dep,
            output,
            command_line,
        )?;
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
    if command_line.verbose {
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
