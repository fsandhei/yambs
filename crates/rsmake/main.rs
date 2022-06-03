use colored::Colorize;
use structopt::StructOpt;

use rsmake::builder::*;
use rsmake::cache::{Cache, Cacher};
use rsmake::cli::command_line::CommandLine;
use rsmake::compiler;
use rsmake::dependency::DependencyNode;
use rsmake::errors::MyMakeError;
use rsmake::external;
use rsmake::generator::MakefileGenerator;
use rsmake::output::Output;
use rsmake::unwrap_or_terminate::MyMakeUnwrap;
use rsmake::utility;

fn try_main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let output = Output::new();
    let cache = Cache::new(&command_line.build_directory)?;

    let compiler = compiler::Compiler::new()?;
    evaluate_compiler(&compiler, &command_line, &cache, &output)?;

    let mut generator = MakefileGenerator::new(&command_line.build_directory, compiler);
    let mut builder = Builder::new(&mut generator);

    builder
        .configure(&command_line)
        .map_err(MyMakeError::ConfigurationTime)?;

    read_mmk_files_from_path(&mut builder, &command_line.input_file, &output)?;

    if command_line.create_dottie_graph {
        return create_dottie_graph(&builder, &output);
    }

    generate_makefiles(&mut builder, &output, &command_line)?;

    build_project(&mut builder, &output, &command_line)?;
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
    builder: &mut Builder,
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

fn read_mmk_files_from_path(
    builder: &mut Builder,
    top_path: &std::path::Path,
    output: &Output,
) -> Result<(), MyMakeError> {
    builder.read_mmk_files_from_path(&top_path)?;
    if builder.top_dependency().is_some() {
        let number_of_mmk_files = builder.number_of_dependencies();
        output.status(&format!("Read {} RsMake files", number_of_mmk_files));
    }
    Ok(())
}

fn create_dottie_graph(builder: &Builder, output: &Output) -> Result<(), MyMakeError> {
    let mut dottie_buffer = String::new();
    if let Some(dependency) = builder.top_dependency() {
        if external::dottie(dependency, false, &mut dottie_buffer).is_ok() {
            output.status("Created dottie file dependency.gv");
        }
    }
    Ok(())
}

fn build_project(
    builder: &mut Builder,
    output: &Output,
    command_line: &CommandLine,
) -> Result<(), MyMakeError> {
    builder.create_log_file()?;
    if let Some(top_dependency) = &builder.top_dependency() {
        let process_output = build_dependency(
            &builder,
            &top_dependency,
            &command_line.build_directory.as_path(),
            &output,
            command_line,
        );
        let build_status_message: String;
        if process_output.is_ok() && process_output.unwrap().status.success() {
            build_status_message = format!("{}", "Build SUCCESS".green());
        } else {
            build_status_message = format!("{}", "Build FAILED".red());
        }
        output.status(&format!("{}", build_status_message));
        builder.make().log_text(build_status_message)?;
        let log_path = command_line
            .build_directory
            .as_path()
            .join("rsmake_log.txt");
        output.status(&format!("Build log available at {:?}", log_path));
    }
    Ok(())
}

fn change_directory(directory: std::path::PathBuf) {
    std::env::set_current_dir(directory).unwrap()
}

pub fn build_dependency(
    builder: &Builder,
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
                builder.resolve_build_directory(&command_line.build_directory.as_path());
            let directory_to_link = top_build_directory_resolved
                .join("libs")
                .join(required_dependency.dependency().ref_dep.get_project_name());

            if !build_path_dep.is_dir() {
                utility::create_symlink(directory_to_link, build_path_dep)?;
            }

            // Se eventuelt etter annen løsning.
            continue;
        }

        required_dependency.dependency_mut().ref_dep.building();
        let dep_output = build_dependency(
            &builder,
            &required_dependency,
            &build_path_dep,
            &output,
            &command_line,
        )?;
        if !dep_output.status.success() {
            return Ok(dep_output);
        }
        required_dependency
            .dependency_mut()
            .ref_dep
            .build_complete();
    }

    dependency.dependency_mut().ref_dep.building();

    let change_directory_message = format!("Entering directory {}\n", build_directory.display());
    if command_line.verbose {
        output.status(&change_directory_message);
    }
    builder.make().log_text(change_directory_message).unwrap();
    change_directory(build_directory);
    output.status(&format!("{}", construct_build_message(dependency)));

    let output = builder.make().spawn()?;
    dependency.dependency_mut().ref_dep.build_complete();

    Ok(output)
}

fn construct_build_message(dependency: &DependencyNode) -> String {
    let dep_type = if dependency.dependency().ref_dep.is_executable() {
        "executable"
    } else {
        "library"
    };
    let dep_type_name = dependency.dependency().ref_dep.get_pretty_name();

    let green_building = format!("{}", "Building".green());
    let target = format!("{} {:?}", dep_type, dep_type_name);
    format!("{} {}", green_building, target)
}
