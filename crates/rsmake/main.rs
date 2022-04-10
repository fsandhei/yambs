use std::io::Write;
use structopt::StructOpt;

mod builder;
mod cache;
mod cli;
mod compiler;
mod dependency;
mod errors;
mod external;
mod generator;
mod mmk_parser;
mod unwrap_or_terminate;
mod utility;

use crate::cache::{Cache, Cacher};
use builder::*;
use cli::command_line::CommandLine;
use errors::MyMakeError;
use generator::MakefileGenerator;
use unwrap_or_terminate::MyMakeUnwrap;

fn try_main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let myfile = &command_line.input_file;
    let cache = Cache::new(&command_line.build_directory)?;

    let compiler = compiler::Compiler::new()?;
    evaluate_compiler(&compiler, &command_line, &cache)?;

    let mut generator = MakefileGenerator::new(&command_line.build_directory, compiler);
    let mut builder = Builder::new(&mut generator);

    builder
        .configure(&command_line)
        .map_err(MyMakeError::ConfigurationTime)?;

    read_mmk_files_from_path(&mut builder, &myfile)?;
    generate_makefiles(&mut builder)?;

    if command_line.create_dottie_graph {
        return create_dottie_graph(&builder);
    }

    builder.build_project()?;
    Ok(())
}

fn main() {
    try_main().unwrap_or_terminate();
}

fn evaluate_compiler(
    compiler: &compiler::Compiler,
    command_line: &CommandLine,
    cache: &Cache,
) -> Result<(), MyMakeError> {
    if !compiler.is_changed(cache) {
        let test_dir = command_line.build_directory.as_path().join("sample");
        println!("rsmake: Evaluating compiler by doing a sample build...");
        compiler.evaluate(&test_dir)?;
        println!("rsmake: Evaluating compiler by doing a sample build... done");
        compiler.cache(cache)?;
    }
    Ok(())
}

fn generate_makefiles(builder: &mut Builder) -> Result<(), MyMakeError> {
    print!("rsmake: Generating makefiles");
    builder.generate_makefiles()?;
    println!();
    Ok(())
}

fn read_mmk_files_from_path(
    builder: &mut Builder,
    top_path: &std::path::Path,
) -> Result<(), MyMakeError> {
    print!("rsmake: Reading RsMake files");
    std::io::stdout().flush().unwrap();
    builder.read_mmk_files_from_path(&top_path)?;
    println!();
    Ok(())
}

fn create_dottie_graph(builder: &Builder) -> Result<(), MyMakeError> {
    let mut dottie_buffer = String::new();
    if let Some(dependency) = builder.top_dependency() {
        if external::dottie(dependency, false, &mut dottie_buffer).is_ok() {
            println!("rsmake: Created dottie file dependency.gv");
        }
    }
    Ok(())
}
