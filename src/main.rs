// extern crate mmk_parser;

mod command_line;
mod unwrap_or_terminate;

use builder::*;
use error::MyMakeError;
use generator::MakefileGenerator;
use mmk_parser::Constant;

use unwrap_or_terminate::MyMakeUnwrap;

use std::io::Write;

fn main() -> Result<(), MyMakeError> {
    let command_line = command_line::CommandLine::new();
    let myfile = command_line.validate_file_path();
    let toolchain_file = &mmk_parser::find_toolchain_file(&myfile.parent().unwrap().to_path_buf())
        .unwrap_or_terminate();

    let toolchain = mmk_parser::read_toolchain(&toolchain_file).unwrap_or_terminate();

    let mut generator = MakefileGenerator::new(std::env::current_dir().unwrap(), &toolchain);
    let mut builder = Builder::new(&mut generator);

    if builder.is_verbose() {
        let compiler = toolchain
            .get_item(&Constant::new("compiler"))
            .unwrap_or_terminate();
        let linker = toolchain
            .get_item(&Constant::new("linker"))
            .unwrap_or_terminate();

        println!(
            "MyMake: Using toolchain from {}",
            toolchain_file.to_str().unwrap()
        );
        println!("MyMake: Using CC = {}", compiler.to_str().unwrap());
        println!("MyMake: Using LD = {}", linker.to_str().unwrap());
    }

    print!("MyMake: Reading MyMake files");
    std::io::stdout().flush().unwrap();
    builder
        .read_mmk_files_from_path(&myfile)
        .unwrap_or_terminate();
    println!();

    command_line
    .parse_command_line(&mut builder)
    .unwrap_or_terminate();

    print!("MyMake: Generating makefiles");
    builder.generate_makefiles().unwrap_or_terminate();
    println!();
    builder.build_project().unwrap_or_terminate();
    Ok(())
}
