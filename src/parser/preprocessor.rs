// YAMBS_DEFINED_VARIABLES: Variables that are defined by yambs at configure time. All are prefixed
// with "yambs_"
// Environment variables: Allow environment variables from the calling shell be detected in yambs
// User defined variables?
//

use std::env;
use std::ffi::OsString;

struct Preprocessor {
    manifest_content: String,
}

impl Preprocessor {
    pub fn new(manifest_content: &str) -> Self {
        Self {
            manifest_content: manifest_content.to_string(),
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum ParseEnvError {
    #[error("Environment variable is empty: {0}")]
    EnvIsEmpty(String),
}

struct EnvironmentVariable {
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
