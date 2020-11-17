extern crate mmk_parser;
extern crate generator;

use clap::{Arg, App};

fn main() {
    let matches = App::new("MyMake")
        .version("0.1.0")
        .author("Fredrik Sandhei <fredrik.sandhei@gmail.com>")
        .about("GNU Make overlay for C / C++ projects.")
        .arg(Arg::with_name("mmk file")
                    .short("f")
                    .long("file")
                    .takes_value(true)
                    .help("Input file for MMK"))
        .get_matches();
    
}
