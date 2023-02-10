// YAMBS_DEFINED_VARIABLES: Variables that are defined by yambs at configure time. All are prefixed
// with "yambs_"
// Environment variables: Allow environment variables from the calling shell be detected in yambs
// User defined variables?
//

use std::env;
use std::ffi::OsString;

use regex::Regex;

#[derive(Debug, thiserror::Error)]
pub enum PreprocessorError {
    #[error("Failed to parse environment variable")]
    EnvVar(#[source] ParseEnvError),
}

pub struct Preprocessor {
    pub manifest_content: String,
    pub registered_env_vars: Vec<EnvironmentVariable>,
}

lazy_static::lazy_static! {
    static ref ENV_VAR_REGEX: Regex = Regex::new(r"\$\{env:(?P<env>.*)\}").unwrap();
}

impl Preprocessor {
    pub fn parse(manifest_content: &str) -> Result<Self, PreprocessorError> {
        let mut preprocessor = Self::new(manifest_content);

        let manifest_content = &mut preprocessor.manifest_content;

        if let Some(env_captures) = ENV_VAR_REGEX.captures(manifest_content) {
            let env = EnvironmentVariable::parse(env_captures.name("env").unwrap().as_str())
                .map_err(PreprocessorError::EnvVar)?;

            let total_capture = env_captures.get(0).unwrap().as_str();
            *manifest_content =
                manifest_content.replace(total_capture, env.value.to_str().unwrap());

            if !preprocessor.registered_env_vars.contains(&env) {
                log::debug!("Registered environment variable {}", env.key);
                preprocessor.registered_env_vars.push(env);
            }
        }

        Ok(preprocessor)
    }

    fn new(manifest_content: &str) -> Self {
        Self {
            manifest_content: manifest_content.to_string(),
            registered_env_vars: Vec::new(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ParseEnvError {
    #[error("Environment variable is empty: {0}")]
    EnvIsEmpty(String),
}

#[derive(Debug, PartialEq, Eq)]
pub struct EnvironmentVariable {
    key: String,
    value: OsString,
}

impl EnvironmentVariable {
    fn parse(s: &str) -> Result<Self, ParseEnvError> {
        let key = s.to_string();
        let value = env::var_os(s).ok_or_else(|| ParseEnvError::EnvIsEmpty(key.clone()))?;
        Ok(Self { key, value })
    }
}
