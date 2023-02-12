// YAMBS_DEFINED_VARIABLES: Variables that are defined by yambs at configure time. All are prefixed
// with "yambs_"
// Environment variables: Allow environment variables from the calling shell be detected in yambs
// User defined variables?

use std::env;
use std::ffi::OsString;

use regex::Regex;

use crate::YAMBS_BUILD_DIR_VAR;
use crate::YAMBS_BUILD_TYPE;
use crate::YAMBS_MANIFEST_DIR;

lazy_static::lazy_static! {
    static ref ENV_VAR_REGEX: Regex = Regex::new(r"\$\{env:(?P<env>.*)\}").unwrap();
    static ref VAR_REGEX: Regex = Regex::new(r"\$\{(?P<var>.*)\}").unwrap();
}

#[derive(Debug, thiserror::Error)]
pub enum PreprocessorError {
    #[error("Failed to parse environment variable")]
    EnvVar(#[source] ParseEnvError),
    #[error("No such preset variable exists: {0}")]
    NoSuchPreset(String),
}

pub struct Preprocessor {
    pub manifest_content: String,
    pub registered_env_vars: Vec<EnvironmentVariable>,
    pub yambs_variables: [Variable; 3],
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

        if let Some(var_captures) = VAR_REGEX.captures(manifest_content) {
            let var = var_captures.name("var").unwrap().as_str();
            let preset_var = preprocessor
                .yambs_variables
                .iter()
                .find(|pvar| pvar.key == var)
                .ok_or_else(|| PreprocessorError::NoSuchPreset(var.to_string()))?;

            let total_capture = var_captures.get(0).unwrap().as_str();
            *manifest_content = manifest_content.replace(total_capture, &preset_var.value);
        }

        Ok(preprocessor)
    }

    // TODO: Need to figure out if it is possible to globally initialize the preset yambs variables
    // so it is easily passable down to the other dependencies.
    // I don't want to always pass the build opts, but only once to the main manifest.
    // OnceCell can globally initialize values once, but where should it be done?
    // If done in main, then the preprocessor just needs to reference to the values of those static
    // variables.
    fn new(manifest_content: &str) -> Self {
        unsafe {
            Self {
                manifest_content: manifest_content.to_string(),
                registered_env_vars: Vec::new(),
                yambs_variables: [
                    Variable {
                        key: "YAMBS_BUILD_DIR".to_string(),
                        value: YAMBS_BUILD_DIR_VAR
                            .get_unchecked()
                            .as_path()
                            .display()
                            .to_string(),
                    },
                    Variable {
                        key: "YAMBS_MANIFEST_DIR".to_string(),
                        value: YAMBS_MANIFEST_DIR
                            .get_unchecked()
                            .as_path()
                            .display()
                            .to_string(),
                    },
                    Variable {
                        key: "YAMBS_BUILD_TYPE".to_string(),
                        value: YAMBS_BUILD_TYPE.get_unchecked().to_string(),
                    },
                ],
            }
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

pub struct Variable {
    pub key: String,
    pub value: String,
}
