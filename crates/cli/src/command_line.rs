use std::path::PathBuf;
use std::str::FromStr;

use structopt::StructOpt;

use crate::build_configurations::BuildConfigurations;
use error::CommandLineError;

// TODO: Need to add tests for C++ validation and sanitizer validation
// TODO: Add default values that correctly correspond for 'configuration' when not all options are
// specified.
// TODO: Perhaps, BuilderConfigurations should be defaulted to have a predefined set of configurations
// TODO: and remove those which are replaced by command line opted input.
// TODO: At a later stage, should jobs be added to build configurations or should it be abstracted
// TODO: to its own struct?

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
    /// Toggles verbose output.
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,
    #[structopt(
        short = "c",
        long = "configuration",
        default_value,
        parse(try_from_str = BuildConfigurations::from_str),
    )]
    /// "Set runtime configurations (build configurations, C++ standard, sanitizers, etc)"
    pub configuration: BuildConfigurations,
    #[structopt(short = "j", long = "jobs", default_value = "10")]
    ///"Set parallelization of builds for Make."
    pub jobs: u8,
}

fn validate_file_path(path: &str) -> Result<PathBuf, CommandLineError> {
    let file_name = mmk_parser::validate_file_path(path)?;
    mmk_parser::validate_file_name(&file_name)?;
    Ok(file_name)
}

// TODO: Add tests for cli usage:
// TODO: Example:
// TODO:    configuration is given partially, resulting in defaults and user provided values.
// TODO:    configuration is not given, which defaults to the default values.
