extern crate mmk_parser;
extern crate generator;
mod unwrap_or_terminate;

use clap::{Arg, App};
use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en /makefile.
*/

fn main() {
    let matches = App::new("MyMake")
        .version("0.1.0")
        .author("Fredrik Sandhei <fredrik.sandhei@gmail.com>")
        .about("GNU Make overlay for C / C++ projects.")
        .arg(Arg::with_name("mmk_file")
                    .short("f")
                    .long("file")
                    .takes_value(true)
                    .help("Input file for MMK"))
        .get_matches();
    let myfile = std::path::Path::new(matches.value_of("mmk_file").unwrap_or_terminate());
    let root = std::path::Path::new(myfile.parent().unwrap());
    let file_content =  mmk_parser::read_file(myfile).unwrap_or_terminate();

    let mut parsed_mmk = mmk_parser::Mmk::new();
    mmk_parser::parse_mmk(&mut parsed_mmk, &file_content, "MMK_SOURCES");
    
    //let generator: generator::MmkGenerator = generator::Generator::new(root.to_str().unwrap(), parsed_mmk);
}
