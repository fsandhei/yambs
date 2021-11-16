use crate::MyMakeUnwrap;
use builder::Builder;
use clap::{App, Arg, ArgMatches, SubCommand};
use error::CommandLineError;
use std::path::PathBuf;

#[derive(PartialEq, Eq)]
enum Configuration {
    Debug,
    Release,
    Sanitizer(Vec<String>),
    CppVersion(String),
}

struct BuildConfigurations {
    configurations: Vec<Configuration>,
}

impl BuildConfigurations {
    pub fn new() -> Self {
        Self {
            configurations: Vec::new(),
        }
    }

    pub fn add_configuration(&mut self, configuration: Configuration) {
        self.configurations.push(configuration);
    }

    pub fn is_debug_build(&self) -> bool {
        self.configurations.contains(&Configuration::Debug)
    }
}

impl IntoIterator for BuildConfigurations {
    type Item = Configuration;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.configurations.into_iter()
    }
}

impl<'a> IntoIterator for &'a BuildConfigurations {
    type Item = &'a Configuration;
    type IntoIter = std::slice::Iter<'a, Configuration>;

    fn into_iter(self) -> Self::IntoIter {
        self.configurations.iter()
    }
}

pub struct CommandLine<'a> {
    matches: ArgMatches<'a>,
    configuration: String,
    verbose: bool,
    jobs: String,
    cpp_version: String,
    sanitizers: Vec<String>,
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
            .get_matches(),
            configuration: String::from("release"),
            verbose: false,
            jobs: String::from("10"),
            cpp_version: String::from("-std=c++17"),
            sanitizers: Vec::<String>::new(),
        }
    }

    fn parse_runtime_configuration(&mut self) -> Result<(), CommandLineError> {
        if self.matches.is_present("runtime configurations") {
            let build_configs: Vec<_> = self
                .matches
                .values_of("runtime configurations")
                .unwrap()
                .collect();

            if build_configs.contains(&"release") && build_configs.contains(&"debug") {
                return Err(CommandLineError::InvalidConfiguration);
            }

            for config in build_configs {
                if config == "debug" {
                    self.configuration = config.to_string();
                    continue;
                }
                if config == "release" {
                    self.configuration = config.to_string();
                    continue;
                }

                let config_str = config.to_lowercase();
                let cpp_version = match config_str.as_str() {
                    "c++98" => "-std=c++98",
                    "c++11" => "-std=c++11",
                    "c++14" => "-std=c++14",
                    "c++17" => "-std=c++17",
                    "c++20" => "-std=c++20",
                    _ => return Err(CommandLineError::InvalidCppVersion(config_str)),
                };
                self.cpp_version = cpp_version.to_string();
            }
        }

        if self.matches.is_present("verbosity") {
            self.verbose = true;
        }

        if self.matches.is_present("jobs") {
            self.jobs = self
                .matches
                .value_of("jobs")
                .unwrap_or_else(|| "10")
                .to_string();
        }

        Ok(())
    }

    // fn parse_extern(&self) -> Result<(), MyMakeError> {
    //     if let Some(ref matches) = self.matches.subcommand_matches("extern") {
    //         if matches.is_present("dot") {
    //             let last = &builder.top_dependency();
    //             if let Some(top_dep) = last {
    //                 match external::dottie(top_dep, false, &mut String::new()) {
    //                     Ok(()) => {
    //                         println!("rsmake: Dependency graph made: dependency.gv");
    //                         std::process::exit(0);
    //                     }
    //                     Err(_) => {
    //                         return Err(MyMakeError::from_str("Could not make dependency graph."))
    //                     }
    //                 };
    //             }
    //         }
    //     }
    //     Ok(())
    // }

    fn parse_sanitizer_options(&mut self) -> Result<(), CommandLineError> {
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
                    return Err(CommandLineError::InvalidSanitizerArgument);
                }
            }
            if sanitizer_options.contains(&String::from("address"))
                && sanitizer_options.contains(&String::from("thread"))
            {
                return Err(CommandLineError::IllegalSanitizerCombination);
            }
            self.sanitizers = sanitizer_options;
        }
        Ok(())
    }

    pub fn parse_command_line(&mut self, builder: &mut Builder) -> Result<(), CommandLineError> {
        if self.matches.is_present("clean") {
            std::process::exit(0);
        }

        self.parse_runtime_configuration()?;
        self.parse_sanitizer_options()?;
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
