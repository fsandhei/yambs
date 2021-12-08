use std::path::PathBuf;
use std::str::FromStr;

use error::CommandLineError;

use lazy_static::lazy_static;
use regex::Regex;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(
    author = "Fredrik Sandhei <fredrik.sandhei@gmail.com>",
    version = "0.1.0",
    name = "RsMake",
    about = "\
             GNU Make build system overlay for C++ projects. RsMake generates makefiles and builds the project with the \n\
             specifications written in the respective RsMake files."
)]
pub struct CommandLine {
    /// Input file for RsMake.
    #[structopt(short = "g", parse(try_from_str = validate_file_path))]
    pub input_file: PathBuf,
    /// Toggles verbosity
    #[structopt(short = "v", long = "verbose", help = "Toggle verbose output")]
    pub verbose: bool,
    #[structopt(
        short = "c",
        long = "configuration",
        default_value = "release",
        parse(try_from_str = BuildConfigurations::from_str),
        help = "Set runtime configurations (build configurations, C++ standard, sanitizers, etc)"
    )]
    pub configuration: BuildConfigurations,
    #[structopt(
        short = "j",
        long = "jobs",
        default_value = "10",
        help = "Set parallelization of builds for Make."
    )]
    pub jobs: u8,
}

#[derive(PartialEq, Eq, Debug)]
pub enum Configuration {
    Debug,
    Release,
    Sanitizer(String),
    CppVersion(String),
}

impl std::str::FromStr for Configuration {
    type Err = CommandLineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        lazy_static! {
            static ref CPP: Regex = Regex::new(r"c\+\+[0-9][0-9]").expect("Regex failed");
            static ref BUILD_CONFIG: Regex = Regex::new(r"release|debug").expect("Regex failed");
            static ref SANITIZER: Regex =
                Regex::new(r"^thread|address|undefined|leak").expect("Regex failed");
        }
        let config = s.to_lowercase();
        if CPP.is_match(&config) {
            return parse_cpp_version(&config);
        } else if BUILD_CONFIG.is_match(&config) {
            if config == "release" {
                return Ok(Configuration::Release);
            } else {
                return Ok(Configuration::Debug);
            }
        } else if SANITIZER.is_match(&config) {
            return Ok(Configuration::Sanitizer(config));
        } else {
            return Err(CommandLineError::InvalidConfiguration);
        }
    }
}

#[derive(StructOpt, Debug)]
pub struct BuildConfigurations {
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

impl std::str::FromStr for BuildConfigurations {
    type Err = CommandLineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut build_configurations = Self::new();
        let cli_configurations = s.split(",");
        for cli_config in cli_configurations {
            let configuration = Configuration::from_str(cli_config)?;
            build_configurations.add_configuration(configuration);
        }

        let sanitizers = {
            let mut sanitizers = Vec::<&Configuration>::new();
            for config in &build_configurations {
                if matches!(config, Configuration::Sanitizer(_)) {
                    sanitizers.push(config);
                }
            }
            sanitizers
        };
        parse_sanitizer_options(&sanitizers)?;
        Ok(build_configurations)
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

fn parse_cpp_version(version: &str) -> Result<Configuration, CommandLineError> {
    match version {
        "c++98" | "c++11" | "c++14" | "c++17" | "c++20" => {
            Ok(Configuration::CppVersion(version.to_string()))
        }
        _ => Err(CommandLineError::InvalidCppVersion(version.to_string())),
    }
}

fn parse_sanitizer_options(sanitizer_options: &[&Configuration]) -> Result<(), CommandLineError> {
    if sanitizer_options.contains(&&Configuration::Sanitizer("address".to_string()))
        && sanitizer_options.contains(&&Configuration::Sanitizer("thread".to_string()))
    {
        return Err(CommandLineError::IllegalSanitizerCombination);
    }
    Ok(())
}

fn validate_file_path(path: &str) -> Result<PathBuf, CommandLineError> {
    let file_name = mmk_parser::validate_file_path(path)?;
    mmk_parser::validate_file_name(&file_name)?;
    Ok(file_name)
}
