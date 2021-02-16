extern crate mmk_parser;
extern crate generator;

mod unwrap_or_terminate;
mod command_line;

use builder::*;
use error::MyMakeError;

use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Builder: *Generere dependency graph. Finne ut hva som skal bygges i riktig rekkefølge
              *Refakturere testene i Builder.
    * Første inkrement: Ha kun én dependency som trengs for å vise konsept.
    *Implementere unwrap_or_terminate() for Option / Result
    *
    *Mmk_parser: Vurder å legge tilbake MMK_LIBRARY_LABEL for å kunne ha ulike library navn.
    *            Legg inn validering på filnavn og extension. Tillatte navn skal være 
                 "lib.mmk" og "build.mmk"             
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en makefile.
    *           Toolchain: Utrede hvordan MyMake skal finne informasjon om toolchain til sluttbruker.
    *                      En liste med predefinerte pather blir søkt gjennom av MyMake til å finne de ulike nødvendige programmene
    *                      (gcc, clang, AR...).
*               Include: Generatoren lager include - filene som trengs til byggene. Da slippes det å lages spesifikke mapper for dette
                         til sluttbrukeren.
                         include-filene til et prosjekt skal legges i /file/to/project/.build/include/
                         include-filene skal ligges i /file/to/project/.build/mymake_include/

    * Overall: * Endre alle Error - meldinger som er relevant til å ta MyMakeError for Result.
    *          * Ordne bedre feilhåndtering for mmk_parser. Feilhåndteringen der baserer seg
    *            foreløpig på utviklerens feil og ikke brukerens feil. Feil skal oppdages fra
    *            brukeren sin side.
    *         * Dekke case der tre dependencies eksisterer: A avhenger av B, og C avhenger av B. Får alle samme B?

    "Lag struct CommandLine som håndterer argumentene inn til MyMake. Bruk den til å passere ting videre til de riktige structene."
*/

fn main() -> Result<(), MyMakeError> {
    let command_line = command_line::CommandLine::new();
    let myfile = command_line.validate_file_path();
    let mut builder = Builder::new();    

    builder.read_mmk_files_from_path(&myfile).unwrap_or_terminate();
    builder.add_generator();
    command_line.parse_command_line(&mut builder).unwrap_or_terminate();

    print!("MyMake: Generating makefiles");
    builder.generate_makefiles().unwrap_or_terminate();
    println!();
    builder.build_project().unwrap_or_terminate();
    Ok(())
}
