use builder::Builder;
use clap::{Arg, App, SubCommand, ArgMatches};
use error::MyMakeError;
use std::path::PathBuf;
use crate::MyMakeUnwrap;


pub struct CommandLine<'a> {
    matches: ArgMatches<'a>,
}


impl<'a> CommandLine<'a> {
    pub fn new() -> Self {
        Self {
            matches: App::new("MyMake Makefile Build System")
            .version("0.1.0")
            .about("\
            GNU Make build system overlay for C++ projects. MyMake generates makefiles and builds the project with the \n\
            specifications written in the respective MyMake files.")
            .author("Written and maintained by Fredrik Sandhei <fredrik.sandhei@gmail.com>")        
            .arg(Arg::with_name("mmk_file")
                        .short("g")
                        .required(true)
                        .takes_value(true)
                        .help("Input file for MyMake."))
            .arg(Arg::with_name("clean")
                        .long("clean")
                        .help("Removes .build directories, cleaning the project."))
            .arg(Arg::with_name("runtime configurations")
                        .short("r")
                        .value_delimiter(",")
                        .default_value("release")
                        .help("Set runtime configurations."))
            .arg(Arg::with_name("verbosity")
                        .short("v")
                        .multiple(true)
                        .help("Toggles verbosity"))
            .arg(Arg::with_name("jobs")
                        .short("j")
                        .default_value("1")
                        .help("Make job parallelization"))
            .subcommand(SubCommand::with_name("extern")
                        .about("Run external programs from MyMake.")
                        .arg(Arg::with_name("dot")
                            .long("dot-dependency")
                            .help("Produce a dot graph visualization of the project dependency.")))
            .get_matches()
        }
    }


    fn parse_runtime_configuration(&self, builder: &mut Builder) -> Result<(), MyMakeError>{
        if self.matches.is_present("runtime configurations") {
            let build_configs: Vec<_> = self.matches.values_of("runtime configurations").unwrap().collect();

            if build_configs.contains(&"release") && build_configs.contains(&"debug") {
                return Err(MyMakeError::from("release and debug can't be used together. Only use one build configuration.".to_string()))
            }

            for config in build_configs {
                if config == "debug" {
                    builder.debug();
                }
                if config == "release" {
                    builder.release();
                }
            }
        }

        if self.matches.is_present("verbosity") {
            builder.verbose();
        }

        if self.matches.is_present("jobs") {
            let value = self.matches.value_of("jobs").unwrap();
            builder.add_make("-j", value);
        }

        Ok(())
    }


    fn parse_extern(&self, builder: &mut Builder) -> Result<(), MyMakeError>{
        if let Some(ref matches) = self.matches.subcommand_matches("extern") {
            if matches.is_present("dot") {
                let last = &builder.top_dependency();
                match external::dottie(&last, false, &mut String::new()) {
                    Ok(()) => {
                                println!("MyMake: Dependency graph made: dependency.gv");
                                std::process::exit(0);
                            },
                    Err(_) => return Err(MyMakeError::from("Could not make dependency graph.".to_string())),
                };
            }
        }
        Ok(())
    }


    pub fn parse_command_line(&self, builder: &mut Builder) -> Result<(), MyMakeError> {
        if self.matches.is_present("clean") {
            builder.clean().unwrap_or_terminate();
            std::process::exit(0);
        }
        

        self.parse_runtime_configuration(builder)?;
        self.parse_extern(builder)?;
        Ok(())
    }


    pub fn validate_file_path(&self) -> PathBuf {
        mmk_parser::validate_file_path(self.matches.value_of("mmk_file")
                                        .unwrap_or_terminate())
                                        .unwrap_or_terminate()
    }
}