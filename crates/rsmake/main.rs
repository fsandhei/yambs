use std::io::Write;

use structopt::StructOpt;

mod builder;
mod cli;
mod compiler;
mod dependency;
mod errors;
mod external;
mod generator;
mod mmk_parser;
mod unwrap_or_terminate;
mod utility;

use builder::*;
use cli::command_line::CommandLine;
use errors::MyMakeError;
use generator::MakefileGenerator;
use unwrap_or_terminate::MyMakeUnwrap;

fn main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let myfile = &command_line.input_file;

    let compiler = compiler::Compiler::new()?;

    let mut generator = MakefileGenerator::new(&command_line.build_directory);
    let mut builder = Builder::new(&mut generator);

    builder
        .configure(&command_line)
        .map_err(MyMakeError::ConfigurationTime)?;

    print!("rsmake: Reading MyMake files");
    std::io::stdout().flush().unwrap();
    builder
        .read_mmk_files_from_path(&myfile)
        .unwrap_or_terminate();
    println!();

    print!("rsmake: Generating makefiles");
    builder.generate_makefiles().unwrap_or_terminate();
    println!();
    builder.build_project().unwrap_or_terminate();
    Ok(())
}
