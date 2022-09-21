use std::path::PathBuf;
use std::str::FromStr;

use structopt::StructOpt;

use crate::cli::build_configurations::{BuildConfigurations, BuildDirectory};
use crate::errors::{CommandLineError, FsError};
use crate::YAMBS_FILE_NAME;

// TODO: Need to add tests for C++ validation and sanitizer validation
// TODO: Add default values that correctly correspond for 'configuration' when not all options are
// specified.
// TODO: Perhaps, BuildManagerConfigurations should be defaulted to have a predefined set of configurations
// TODO: and remove those which are replaced by command line opted input.
// TODO: At a later stage, should jobs be added to build configurations or should it be abstracted
// TODO: to its own struct?

#[derive(StructOpt, Debug)]
#[structopt(
    author = "Fredrik Sandhei <fredrik.sandhei@gmail.com>",
    version = "0.1.0",
    name = "YAMBS",
    about = "\
             GNU Make build system overlay for C++ projects. Yambs generates makefiles and builds the project with the \n\
             specifications written in the respective YAMBS files."
)]
pub struct CommandLine {
    #[structopt(subcommand)]
    pub subcommand: Option<Subcommand>,
}

fn validate_file_path(path_as_str: &str) -> Result<std::path::PathBuf, CommandLineError> {
    let file_path = std::path::PathBuf::from(path_as_str)
        .canonicalize()
        .map_err(FsError::Canonicalize)?;

    if !file_path.is_file() {
        return Err(FsError::FileDoesNotExist(file_path)).map_err(CommandLineError::Fs);
    }

    let filename = file_path.file_name();
    if filename.unwrap().to_str().unwrap() != YAMBS_FILE_NAME {
        return Err(FsError::InvalidRecipeFilename(file_path)).map_err(CommandLineError::Fs);
    }

    Ok(file_path)
}

#[derive(StructOpt, Debug)]
pub enum Subcommand {
    /// Build project specified by manifest YAMBS file.
    Build(BuildOpts),
    /// Print previous invocation line used and exit.
    Remake(RemakeOpts),
}

#[derive(StructOpt, Debug)]
pub struct BuildOpts {
    /// Input manifest file for YAMBS.
    #[structopt(parse(try_from_str = validate_file_path))]
    pub input_file: PathBuf,
    /// Set runtime configurations (build configurations, C++ standard, sanitizers, etc)
    #[structopt(
        short = "c",
        long = "configuration",
        default_value,
        parse(try_from_str = BuildConfigurations::from_str),
    )]
    pub configuration: BuildConfigurations,
    /// Set parallelization of builds for Make.
    #[structopt(short = "j", long = "jobs", default_value = "10")]
    pub jobs: u8,
    /// Set build directory. Generated output by Yambs will be put here. Defaults to current working directory.
    #[structopt(
        long,
        short = "b",
        default_value,
        hide_default_value(true),
        parse(try_from_str)
    )]
    pub build_directory: BuildDirectory,
    /// Create dottie graph of build tree and exit.
    #[structopt(long = "dottie-graph")]
    pub create_dottie_graph: bool,
    /// Toggles verbose output.
    #[structopt(short = "v", long = "verbose")]
    pub verbose: bool,
}

#[derive(StructOpt, Debug)]
pub struct RemakeOpts {
    /// Build directory to read invocation from.
    #[structopt(parse(try_from_str))]
    pub build_directory: BuildDirectory,
}

// TODO: Add tests for cli usage:
// TODO: Example:
// TODO:    configuration is given partially, resulting in defaults and user provided values.
// TODO:    configuration is not given, which defaults to the default values.
