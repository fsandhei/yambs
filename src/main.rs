extern crate mmk_parser;
extern crate generator;

mod unwrap_or_terminate;


use clap::{Arg, App};
use builder::*;
use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Builder: Generere dependency graph. Finne ut hva som skal bygges i riktig rekkefølge
    * Første inkrement: Ha kun én dependency som trengs for å vise konsept.
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en /makefile.
*/

fn main() -> Result<(), std::io::Error> {
    let matches = App::new("MyMake")
        .version("0.1.0")
        .author("Fredrik Sandhei <fredrik.sandhei@gmail.com>")
        .about("GNU Make overlay for C / C++ projects.")
        .arg(Arg::with_name("mmk_file")
                    .short("g")
                    .long("generator")
                    .takes_value(true)
                    .help("Input file for MMK"))
        .get_matches();
        
    let myfile = std::path::Path::new(matches.value_of("mmk_file").unwrap_or_terminate());
    let mut builder = Builder::new();

    print!("MyMake: Reading mmk files");
    builder.read_mmk_files(myfile).unwrap_or_terminate();
    println!();
    println!("{:?}", builder.mmk_data);
    
    // let mut generator: generator::MmkGenerator = generator::Generator::new(root, parsed_mmk);
    // generator::Generator::generate_makefile(&mut generator)?;
    Ok(())
}
