use regex::Regex;
use structopt::StructOpt;

use error::CommandLineError;

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
        let cpp_pattern: Regex = Regex::new(r"c\+\+[0-9][0-9]").expect("Regex failed");
        let build_config: Regex = Regex::new(r"release|debug").expect("Regex failed");
        let sanitizer: Regex = Regex::new(r"^thread|address|undefined|leak").expect("Regex failed");
        let config = s.to_lowercase();
        if cpp_pattern.is_match(&config) {
            return parse_cpp_version(&config);
        } else if build_config.is_match(&config) {
            if config == "release" {
                return Ok(Configuration::Release);
            } else {
                return Ok(Configuration::Debug);
            }
        } else if sanitizer.is_match(&config) {
            return Ok(Configuration::Sanitizer(config));
        } else {
            return Err(CommandLineError::InvalidConfiguration);
        }
    }
}

impl std::string::ToString for Configuration {
    fn to_string(&self) -> String {
        match self {
            Self::Debug => "debug".to_string(),
            Self::Release => "release".to_string(),
            Self::Sanitizer(sanitizer) => sanitizer.to_owned(),
            Self::CppVersion(cpp_version) => cpp_version.to_owned(),
        }
    }
}

#[derive(StructOpt, Debug, PartialEq, Eq)]
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

    pub fn remove_configuration(&mut self, configuration: &Configuration) {
        let pos = self
            .configurations
            .as_slice()
            .into_iter()
            .position(|config| config == configuration);
        if let Some(found_pos) = pos {
            self.configurations.remove(found_pos);
        }
    }

    pub fn has_configuration(&self, configuration: &Configuration) -> bool {
        self.configurations.contains(configuration)
    }

    pub fn is_debug_build(&self) -> bool {
        self.configurations.contains(&Configuration::Debug)
    }
}

impl Default for BuildConfigurations {
    fn default() -> Self {
        let mut build_configurations = Self::new();
        build_configurations.add_configuration(Configuration::Release);
        build_configurations.add_configuration(Configuration::CppVersion("C++17".to_string()));
        build_configurations
    }
}

impl std::str::FromStr for BuildConfigurations {
    type Err = CommandLineError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut build_configurations = Self::default();
        if s.is_empty() {
            return Ok(build_configurations);
        }
        let cli_configurations = s.split(",").filter(|s| !s.is_empty());
        for cli_config in cli_configurations {
            if cli_config == "release" || cli_config == "c++17" {
                continue;
            }
            let configuration = Configuration::from_str(cli_config)?;
            build_configurations.add_configuration(configuration);
        }

        if build_configurations.has_configuration(&Configuration::Debug) {
            build_configurations.remove_configuration(&Configuration::Release);
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

impl std::string::ToString for BuildConfigurations {
    fn to_string(&self) -> String {
        let mut configuration_as_string = String::new();
        for configuration in &self.configurations {
            configuration_as_string.push_str(&configuration.to_string());
            configuration_as_string.push_str(",");
        }
        configuration_as_string
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn add_cpp_version_cpp98_test() -> Result<(), CommandLineError> {
        let cpp_version = parse_cpp_version("c++98")?;
        assert_eq!(cpp_version, Configuration::CppVersion("c++98".into()));
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp11_test() -> Result<(), CommandLineError> {
        let cpp_version = parse_cpp_version("c++11")?;
        assert_eq!(cpp_version, Configuration::CppVersion("c++11".into()));
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp14_test() -> Result<(), CommandLineError> {
        let cpp_version = parse_cpp_version("c++14")?;
        assert_eq!(cpp_version, Configuration::CppVersion("c++14".into()));
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp17_test() -> Result<(), CommandLineError> {
        let cpp_version = parse_cpp_version("c++17")?;
        assert_eq!(cpp_version, Configuration::CppVersion("c++17".into()));
        Ok(())
    }

    #[test]
    fn add_cpp_version_cpp20_test() -> Result<(), CommandLineError> {
        let cpp_version = parse_cpp_version("c++20")?;
        assert_eq!(cpp_version, Configuration::CppVersion("c++20".into()));
        Ok(())
    }

    #[test]
    fn parse_cpp_version_fails_on_invalid_version() -> Result<(), CommandLineError> {
        let result = parse_cpp_version("python");
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn parse_sanitizer_options_allows_combinatio_of_address_and_leak(
    ) -> Result<(), CommandLineError> {
        let address = Configuration::Sanitizer("address".into());
        let thread = Configuration::Sanitizer("leak".into());
        let sanitizers = vec![&address, &thread];
        parse_sanitizer_options(sanitizers.as_slice())
    }

    #[test]
    fn parse_sanitizer_options_allows_combinatio_of_address_and_undefined(
    ) -> Result<(), CommandLineError> {
        let address = Configuration::Sanitizer("address".into());
        let undefined = Configuration::Sanitizer("undefined".into());
        let sanitizers = vec![&address, &undefined];
        parse_sanitizer_options(sanitizers.as_slice())
    }

    #[test]
    fn parse_sanitizer_options_does_not_allow_combination_of_address_and_thread(
    ) -> Result<(), CommandLineError> {
        let address = Configuration::Sanitizer("address".into());
        let thread = Configuration::Sanitizer("thread".into());
        let sanitizers = vec![&address, &thread];
        assert!(parse_sanitizer_options(sanitizers.as_slice()).is_err());
        Ok(())
    }

    #[test]
    fn to_string_parses_configurations_to_a_comma_separated_string() {
        let build_configurations = BuildConfigurations::default();
        let expected = "release,C++17,";
        let actual = build_configurations.to_string();
        assert_eq!(actual, expected);
    }

    #[test]
    fn from_str_produces_build_configuration() -> Result<(), CommandLineError> {
        let input = "release,c++17";
        let mut expected = BuildConfigurations::new();
        expected.add_configuration(Configuration::Release);
        expected.add_configuration(Configuration::CppVersion("C++17".to_string()));
        let actual = BuildConfigurations::from_str(input).unwrap();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn from_str_produces_build_configuration_with_trailing_comma() -> Result<(), CommandLineError> {
        let input = "release,c++17,";
        let mut expected = BuildConfigurations::new();
        expected.add_configuration(Configuration::Release);
        expected.add_configuration(Configuration::CppVersion("C++17".to_string()));
        let actual = BuildConfigurations::from_str(input).unwrap();
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn from_str_produces_build_configuration_with_multiple_sanitizers(
    ) -> Result<(), CommandLineError> {
        let input = "release,c++17,address,leak";
        let mut expected = BuildConfigurations::new();
        expected.add_configuration(Configuration::Release);
        expected.add_configuration(Configuration::CppVersion("C++17".to_string()));
        expected.add_configuration(Configuration::Sanitizer("address".to_string()));
        expected.add_configuration(Configuration::Sanitizer("leak".to_string()));
        let actual = BuildConfigurations::from_str(input)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn from_str_produces_build_configuration_when_not_exhaustive_configurations_are_given(
    ) -> Result<(), CommandLineError> {
        let input = "c++17";
        let mut expected = BuildConfigurations::new();
        expected.add_configuration(Configuration::Release);
        expected.add_configuration(Configuration::CppVersion("C++17".to_string()));
        let actual = BuildConfigurations::from_str(input)?;
        assert_eq!(actual, expected);
        Ok(())
    }

    #[test]
    fn from_str_produces_build_configuration_when_only_debug_is_given_and_defaults_to_cpp17(
    ) -> Result<(), CommandLineError> {
        let input = "debug";
        let mut expected = BuildConfigurations::new();
        expected.add_configuration(Configuration::CppVersion("C++17".to_string()));
        expected.add_configuration(Configuration::Debug);
        let actual = BuildConfigurations::from_str(input)?;
        assert_eq!(actual, expected);
        Ok(())
    }
}
