// extern crate mmk_parser;

mod unwrap_or_terminate;
mod command_line;

use builder::*;
use generator::MakefileGenerator;
use error::MyMakeError;

use unwrap_or_terminate::MyMakeUnwrap;

use std::io::Write;

fn main() -> Result<(), MyMakeError> {
    let command_line = command_line::CommandLine::new();
    let myfile = command_line.validate_file_path();
    let mut generator = MakefileGenerator::new(std::env::current_dir().unwrap());
    let mut builder = Builder::new(&mut generator);

    print!("MyMake: Reading MyMake files");
    std::io::stdout().flush().unwrap();
    builder.read_mmk_files_from_path(&myfile).unwrap_or_terminate();
    println!();
    command_line.parse_command_line(&mut builder).unwrap_or_terminate();

    print!("MyMake: Generating makefiles");
    builder.generate_makefiles().unwrap_or_terminate();
    println!();
    builder.build_project().unwrap_or_terminate();
    Ok(())
}
