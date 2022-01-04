use std::io::Write;

use structopt::StructOpt;

use builder::*;
use cli::command_line::CommandLine;
use error::MyMakeError;
use generator::MakefileGenerator;
use unwrap_or_terminate::MyMakeUnwrap;

mod unwrap_or_terminate;

fn main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let myfile = &command_line.input_file;

    let mut generator = MakefileGenerator::new(std::env::current_dir().unwrap());
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
