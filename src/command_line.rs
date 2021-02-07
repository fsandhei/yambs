use builder::Builder;
use external::*;
use error::MyMakeError;
use clap::{Arg, App, SubCommand, ArgMatches};

struct CommandLine<'a> {
    matches: ArgMatches<'a>,
}


impl CommandLine {
    fn new() -> Self {
        Self {
            matches: App::new("MyMake Makefile Build System")
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
            .arg(Arg::with_name("runtime configurations")
                        .short("v")
                        .value_delimiter(",")
                        .default_value("release")
                        .help("Set runtime configurations.")
                        .min_values(1))
            .arg(Arg::with_name("Make job parallelization")
                        .short("J"))
                        .default_value("0")
                        .help("Make job parallelization")
            .subcommand(SubCommand::with_name("extern")
                        .about("Run external programs from MyMake.")
                        .arg(Arg::with_name("dot")
                            .long("dot-dependency")
                            .help("Produce a dot graph visualization of the project dependency.")))
            .get_matches()
        }
    }


    fn parse_runtime_configuration(&self, builder: &mut Builder) {
        if self.matches.is_present("runtime configurations") {
            let build_configs: Vec<_> = matches.values_of("runtime configurations").unwrap().collect();
            for config in build_configs {
                if config == "debug" {
                    builder.debug();
                }
                // else if config == "release" {

                // }
            }
        }
    }


    fn parse_extern(&self) {
        if let Some(ref matches) = matches.subcommand_matches("extern") {
            if matches.is_present("dot") {
                let last = &builder.top_dependency();
                match external::dottie(&last, false, &mut String::new()) {
                    Ok(()) => {
                                println!("MyMake: Dependency graph made: dependency.gv");
                                std::process::exit(0);
                            },
                    Err(_) => MyMakeError::new("Could not make dependency graph."),
                };
            }
        }
    }


    pub fn parse_command_line(&self, builder: &mut Builder) {
        self.parse_runtime_configuration(builder);

        if self.matches.is_present("clean") {
            builder.clean().unwrap_or_terminate();
        }
    }
}