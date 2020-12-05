extern crate mmk_parser;
extern crate generator;

mod unwrap_or_terminate;

use clap::{Arg, App, SubCommand};
use builder::*;
use external;
use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Builder: *Generere dependency graph. Finne ut hva som skal bygges i riktig rekkefølge
              *Refakturere testene i Builder.
    * Første inkrement: Ha kun én dependency som trengs for å vise konsept.
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en /makefile.

    * Overall: Endre alle Error - meldinger som er relevant til å ta MyMakeError for Result.
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
                    .help("Input file for MMK."))
        .subcommand(SubCommand::with_name("extern")
                    .about("Run external programs from MyMake.")
                    .arg(Arg::with_name("dot")
                        .long("dot-dependency")
                        .help("Produce a dot graph visualization of the project dependency.")))
        .get_matches();
    let myfilepath = std::path::Path::new(matches.value_of("mmk_file").unwrap_or_terminate());
    let myfile = std::path::PathBuf::from(myfilepath);
    let mut builder = Builder::new();

    print!("MyMake: Reading mmk files");
    builder.read_mmk_files(&myfile).unwrap_or_terminate();
    println!();

    if let Some(matches) = matches.subcommand_matches("extern")
    {
        if matches.is_present("dot")
        {
            if let Some(last) = builder.mmk_dependencies.last()
            {
                match external::dottie(last, false, &mut String::new())
                {
                    Ok(()) => println!("MyMake: Dependency graph made: dependency.gv"),
                    Err(_) => println!("MyMake: Could not make dependency graph."),
                };
            }
        }
    }
    
    // let mut generator: generator::MmkGenerator = generator::Generator::new(root, parsed_mmk);
    // generator::Generator::generate_makefile(&mut generator)?;
    Ok(())
}
