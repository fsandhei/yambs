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

    * Overall: * Endre alle Error - meldinger som er relevant til å ta MyMakeError for Result.
    *          * Ordne bedre feilhåndtering for mmk_parser. Feilhåndteringen der baserer seg
    *            foreløpig på utviklerens feil og ikke brukerens feil. Feil skal oppdages fra
    *            brukeren sin side.
    *         * Dekke case der tre dependencies eksisterer: A avhenger av B, og C avhenger av B. Får alle samme B?
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
    let myfile = mmk_parser::validate_file_path(matches.value_of("mmk_file").unwrap_or_terminate()).unwrap_or_terminate();
    let mut builder = Builder::new();

    builder.read_mmk_files_from_path(&myfile).unwrap_or_terminate();

    if let Some(matches) = matches.subcommand_matches("extern")
    {
        if matches.is_present("dot")
        {
            let last = &builder.top_dependency;
            match external::dottie(&last, false, &mut String::new())
            {
                Ok(()) => {
                            println!("MyMake: Dependency graph made: dependency.gv");
                            std::process::exit(1);
                          },
                Err(_) => println!("MyMake: Could not make dependency graph."),
            };
        }
    }

    print!("MyMake: Generating makefiles");
    Builder::generate_makefiles(&mut builder.top_dependency).unwrap_or_terminate();
    println!();
    builder.build_project(false).unwrap_or_terminate();
    Ok(())
}
