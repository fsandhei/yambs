extern crate mmk_parser;
extern crate generator;

mod unwrap_or_terminate;

use clap::{Arg, App, SubCommand};
use builder::*;
use external;
use error::MyMakeError;

use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Builder: *Generere dependency graph. Finne ut hva som skal bygges i riktig rekkefølge
              *Refakturere testene i Builder.
    * Første inkrement: Ha kun én dependency som trengs for å vise konsept.
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en makefile.
    *           Toolchain: Utrede hvordan MyMake skal finne informasjon om toolchain til sluttbruker.
    *                      En liste med predefinerte pather blir søkt gjennom av MyMake til å finne de ulike nødvendige programmene
    *                      (gcc, clang, AR...).

    * Overall: * Endre alle Error - meldinger som er relevant til å ta MyMakeError for Result.
    *          * Ordne bedre feilhåndtering for mmk_parser. Feilhåndteringen der baserer seg
    *            foreløpig på utviklerens feil og ikke brukerens feil. Feil skal oppdages fra
    *            brukeren sin side.
    *         * Dekke case der tre dependencies eksisterer: A avhenger av B, og C avhenger av B. Får alle samme B?

    "Lag struct CommandLine som håndterer argumentene inn til MyMake. Bruk den til å passere ting videre til de riktige structene."
*/

fn main() -> Result<(), MyMakeError> {
    let matches = App::new("MyMake Makefile Build System")
        .version("0.1.0")
        .about("\
        GNU Make build system overlay for C++ projects. MyMake generates makefiles and builds the project with the \n\
        specifications written in the respective MyMake files.")
        .author("Written and maintained by Fredrik Sandhei <fredrik.sandhei@gmail.com>")        
        .arg(Arg::with_name("mmk_file")
                    .short("g")
                    .long("generator")
                    .takes_value(true)
                    .help("Input file for MyMake."))
        .arg(Arg::with_name("clean")
                    .long("clean")
                    .help("Removes .build directories, cleaning the project."))
        .subcommand(SubCommand::with_name("extern")
                    .about("Run external programs from MyMake.")
                    .arg(Arg::with_name("dot")
                        .long("dot-dependency")
                        .help("Produce a dot graph visualization of the project dependency.")))
        .get_matches();
    let myfile = mmk_parser::validate_file_path(matches.value_of("mmk_file").unwrap_or_terminate()).unwrap_or_terminate();
    let mut builder = Builder::new();

    builder.read_mmk_files_from_path(&myfile).unwrap_or_terminate();

    if let Some(ref matches) = matches.subcommand_matches("extern")
    {
        if matches.is_present("dot")
        {
            let last = &builder.top_dependency;
            match external::dottie(&last, false, &mut String::new())
            {
                Ok(()) => {
                            println!("MyMake: Dependency graph made: dependency.gv");
                            std::process::exit(0);
                          },
                Err(_) => println!("MyMake: Could not make dependency graph."),
            };
        }
    }
    
    if matches.is_present("clean") {
        builder.clean().unwrap_or_terminate();
    }

    else {
        print!("MyMake: Generating makefiles");
        Builder::generate_makefiles(&mut builder.top_dependency).unwrap_or_terminate();
        println!();
        builder.build_project(false).unwrap_or_terminate();
    }
    
    Ok(())
}
