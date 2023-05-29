use serde::{Deserialize, Serialize};

use crate::cli;
use crate::cli::configurations;
use crate::errors::{CommandLineError, FsError};
use crate::generator::GeneratorType;
use crate::parser::types::{CXXStandard, Define};

// TODO: Need to add tests for C++ validation
// TODO: Add default values that correctly correspond for 'configuration' when not all options are
// specified.
// TODO: Perhaps, BuildManagerConfigurations should be defaulted to have a predefined set of configurations
// TODO: and remove those which are replaced by command line opted input.
// TODO: At a later stage, should jobs be added to build configurations or should it be abstracted
// TODO: to its own struct?

#[derive(clap::Parser, Debug)]
/// Meta build system overlay for C++ projects. Yambs generates makefiles and builds the project with the
/// specifications written in the respective YAMBS files.
pub struct CommandLine {
    #[command(subcommand)]
    pub subcommand: Option<Subcommand>,
    /// Display version and exit
    #[arg(long = "version")]
    pub show_version: bool,
}

#[derive(Debug, Clone)]
pub struct ManifestDirectory(std::path::PathBuf);

impl ManifestDirectory {
    pub fn as_path(&self) -> &std::path::Path {
        self.0.as_path()
    }
}

impl std::default::Default for ManifestDirectory {
    fn default() -> Self {
        Self(std::env::current_dir().unwrap())
    }
}

impl std::string::ToString for ManifestDirectory {
    fn to_string(&self) -> String {
        self.0.display().to_string()
    }
}

impl std::str::FromStr for ManifestDirectory {
    type Err = CommandLineError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let canonicalized_path =
            cli::canonicalize_path(&std::path::PathBuf::from(s)).map_err(FsError::Canonicalize)?;
        Ok(Self(canonicalized_path))
    }
}

#[derive(clap::Subcommand, Debug)]
pub enum Subcommand {
    /// Build project specified by manifest YAMBS file.
    Build(BuildOpts),
    /// Print previous invocation line used and exit.
    Remake(RemakeOpts),
}

#[derive(clap::Args, Debug)]
#[command(dont_delimit_trailing_values = true)]
pub struct BuildOpts {
    /// Input manifest file for YAMBS. By default, Yambs searches for yambs.toml manifest in current directory.
    #[arg(default_value_t, hide_default_value(true), long = "manifest-directory")]
    pub manifest_dir: ManifestDirectory,
    /// Set runtime configurations (build configurations, C++ standard, etc)
    #[command(flatten)]
    pub configuration: ConfigurationOpts,
    /// Set build directory. Generated output by Yambs will be put here. Defaults to current working directory.
    #[arg(
        long,
        short = 'b',
        default_value_t,
        hide_default_value(true),
        value_parser
    )]
    pub build_directory: cli::BuildDirectory,
    /// Toggles verbose output.
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
    /// Specific target to build
    #[arg(long)]
    pub target: Option<String>,
    #[arg(hide = true)]
    pub make_args: Vec<String>,
}

#[derive(clap::Args, Debug, Eq, PartialEq, Clone, Serialize, Deserialize)]
pub struct ConfigurationOpts {
    /// Build configuration to use
    #[arg(default_value_t, long = "build-type")]
    pub build_type: configurations::BuildType,
    /// C++ standard to be passed to compiler
    #[arg(default_value_t,
          long = "std",
          value_parser = clap::builder::ValueParser::new(CXXStandard::parse))]
    pub cxx_standard: CXXStandard,
    #[arg(default_value_t = GeneratorType::GNUMakefiles, short = 'g', value_enum)]
    pub generator_type: GeneratorType,
    #[arg(short = 'D', value_parser = Define::from_cli)]
    pub defines: Vec<Define>,
}

#[derive(clap::Args, Debug)]
pub struct RemakeOpts {
    /// Build directory to read invocation from.
    #[arg(value_parser)]
    pub build_directory: cli::BuildDirectory,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn arguments_passed_after_double_hyphen_are_parsed_raw() {
        let build_args = ["build", "--build-type", "debug", "--", "-j", "10", "x"];
        let command_line = CommandLine::parse_from(std::iter::once("build").chain(build_args));
        let build_opts = match command_line.subcommand {
            Some(Subcommand::Build(b)) => b,
            _ => panic!("Not build opts"),
        };
        assert_eq!(build_opts.make_args, vec!["-j", "10", "x"]);
    }

    #[test]
    fn test_cli() {
        use clap::CommandFactory;
        CommandLine::command().debug_assert()
    }
}
