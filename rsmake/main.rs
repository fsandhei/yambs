use std::io::Write;

use structopt::StructOpt;

use builder::*;
use cli::command_line::CommandLine;
use error::{FsError, MyMakeError};
use generator::MakefileGenerator;
use unwrap_or_terminate::MyMakeUnwrap;

mod unwrap_or_terminate;

fn main() -> Result<(), MyMakeError> {
    let command_line = CommandLine::from_args();
    let myfile = &command_line.input_file;
    let toolchain_file = &mmk_parser::find_toolchain_file(myfile.parent().unwrap_or(&myfile))
        .or_else(|_| {
            mmk_parser::find_toolchain_file(
                &std::env::current_dir().map_err(FsError::AccessDirectory)?,
            )
        })
        .unwrap_or_terminate();

    let toolchain = mmk_parser::read_toolchain(&toolchain_file).unwrap_or_terminate();

    let mut generator = MakefileGenerator::new(std::env::current_dir().unwrap(), &toolchain);
    let mut builder = Builder::new(&mut generator);

    builder
        .configure(&command_line)
        .map_err(MyMakeError::ConfigurationTime)?;

    println!(
        "rsmake: Using toolchain from {}",
        toolchain_file.to_str().unwrap()
    );
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
