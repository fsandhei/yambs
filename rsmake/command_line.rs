use crate::MyMakeUnwrap;
use builder::Builder;
use clap::{App, Arg, ArgMatches, SubCommand};
use error::MyMakeError;
use std::path::PathBuf;

pub struct CommandLine<'a> {
    matches: ArgMatches<'a>,
}

impl<'a> CommandLine<'a> {
    pub fn new() -> Self {
        Self {
            matches: App::new("RsMake Build System")
            .version("0.1.0")
            .about("\
            GNU Make build system overlay for C++ projects. RsMake generates makefiles and builds the project with the \n\
            specifications written in the respective RsMake files.")
            .author("Written and maintained by Fredrik Sandhei <fredrik.sandhei@gmail.com>")        
            .arg(Arg::with_name("RsMake file")
                        .short("g")
                        .required(true)
                        .takes_value(true)
                        .help("Input file for RsMake."))
            .arg(Arg::with_name("clean")
                        .long("clean")
                        .help("Removes .build directories, cleaning the project. WARNING: Deprecated. Delete build directory manually instead."))
            .arg(Arg::with_name("runtime configurations")
                        .short("r")
                        .value_delimiter(",")
                        .default_value("release")
                        .help("Set runtime configurations."))
            .arg(Arg::with_name("verbosity")
                        .short("v")
                        .long("verbose")
                        .multiple(true)
                        .help("Toggles verbosity"))
            .arg(Arg::with_name("jobs")
                        .short("j")
                        .default_value("10")
                        .help("Make job parallelization"))
            .arg(Arg::with_name("sanitizer")
                        .value_delimiter(",")
                        .long("sanitizer")
                        .multiple(true)
                        .help("Sets sanitizers to be used for debug build (address, undefined, leak, thread)."))
            .subcommand(SubCommand::with_name("extern")
                        .about("Run external programs from RsMake.")
                        .arg(Arg::with_name("dot")
                            .long("dot-dependency")
                            .help("Produce a dot graph visualization of the project dependency.")))
            .get_matches()
        }
    }

    fn parse_runtime_configuration(&self, builder: &mut Builder) -> Result<(), MyMakeError> {
        if self.matches.is_present("runtime configurations") {
            let build_configs: Vec<_> = self
                .matches
                .values_of("runtime configurations")
                .unwrap()
                .collect();

            if build_configs.contains(&"release") && build_configs.contains(&"debug") {
                return Err(MyMakeError::from(
                    "release and debug can't be used together. Only use one build configuration."
                        .to_string(),
                ));
            }

            if !build_configs.contains(&"release") && !build_configs.contains(&"debug") {
                builder.release();
            }

            for config in build_configs {
                if config == "debug" {
                    builder.debug();
                    continue;
                }
                if config == "release" {
                    builder.release();
                    continue;
                }

                builder.use_std(config)?;
            }
        }

        if self.matches.is_present("verbosity") {
            builder.set_verbose(true);
        }

        if self.matches.is_present("jobs") {
            let value = self.matches.value_of("jobs").unwrap();
            builder.add_make("-j", value);
        }

        Ok(())
    }

    fn parse_extern(&self, builder: &mut Builder) -> Result<(), MyMakeError> {
        if let Some(ref matches) = self.matches.subcommand_matches("extern") {
            if matches.is_present("dot") {
                let last = &builder.top_dependency();
                if let Some(top_dep) = last {
                    match external::dottie(top_dep, false, &mut String::new()) {
                        Ok(()) => {
                            println!("rsmake: Dependency graph made: dependency.gv");
                            std::process::exit(0);
                        }
                        Err(_) => {
                            return Err(MyMakeError::from_str("Could not make dependency graph."))
                        }
                    };
                }
            }
        }
        Ok(())
    }

    fn parse_sanitizer_options(&self, builder: &mut Builder) -> Result<(), MyMakeError> {
        if self.matches.is_present("sanitizer") {
            let valid_options = vec!["address", "undefined", "leak", "thread"];
            let sanitizer_options: Vec<String> = self
                .matches
                .values_of("sanitizer")
                .unwrap()
                .map(|s| s.to_string())
                .collect();
            for option in &sanitizer_options {
                if !valid_options.contains(&option.as_str()) {
                    return Err(MyMakeError::from_str("Invalid argument used for sanitizer.\n\
                                                         Valid arguments are address, undefined, leak and thread."));
                }
            }
            if sanitizer_options.contains(&String::from("address"))
                && sanitizer_options.contains(&String::from("thread"))
            {
                return Err(MyMakeError::from_str(
                    "address cannot be used together with thread. Pick only one.",
                ));
            }
            builder.set_sanitizers(sanitizer_options.as_slice());
        }
        Ok(())
    }

    pub fn parse_command_line(&self, builder: &mut Builder) -> Result<(), MyMakeError> {
        if self.matches.is_present("clean") {
            // builder.clean().unwrap_or_terminate();
            std::process::exit(0);
        }

        self.parse_runtime_configuration(builder)?;
        self.parse_sanitizer_options(builder)?;
        self.parse_extern(builder)?;
        Ok(())
    }

    pub fn validate_file_path(&self) -> PathBuf {
        // Fix so the error message is explainable.
        let file_name = mmk_parser::validate_file_path(
            self.matches.value_of("RsMake file").unwrap_or_terminate(),
        )
        .unwrap_or_terminate();
        mmk_parser::validate_file_name(&file_name).unwrap_or_terminate();
        file_name
    }
}
