// YAMBS_DEFINED_VARIABLES: Variables that are defined by yambs at configure time. All are prefixed
// with "yambs_"
// Environment variables: Allow environment variables from the calling shell be detected in yambs
// User defined variables?

use std::env;
use std::ffi::OsString;

use regex::Regex;

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
    pub registered_env_vars: Vec<EnvironmentVariable>,
    pub yambs_variables: Vec<Variable>,
}

impl Preprocessor {
    // TODO: Need to figure out if it is possible to globally initialize the preset yambs variables
    // so it is easily passable down to the other dependencies.
    // I don't want to always pass the build opts, but only once to the main manifest.
    // OnceCell can globally initialize values once, but where should it be done?
    // If done in main, then the preprocessor just needs to reference to the values of those static
    // variables.
    pub fn new() -> Self {
        Self {
            registered_env_vars: vec![],
            yambs_variables: vec![],
        }
    }

    pub fn with_env_var(mut self, var: EnvironmentVariable) -> Self {
        self.registered_env_vars.push(var);
        self
    }

    pub fn with_var(mut self, var: Variable) -> Self {
        self.yambs_variables.push(var);
        self
    }

    pub fn parse(&mut self, manifest_content: &str) -> Result<String, PreprocessorError> {
        let mut manifest_content = manifest_content.to_string();

        if let Some(env_captures) = ENV_VAR_REGEX.captures(&manifest_content) {
            let env = EnvironmentVariable::parse(env_captures.name("env").unwrap().as_str())
                .map_err(PreprocessorError::EnvVar)?;

            let total_capture = env_captures.get(0).unwrap().as_str();
            manifest_content = manifest_content.replace(total_capture, env.value.to_str().unwrap());

            if !self.registered_env_vars.contains(&env) {
                log::debug!("Registered environment variable {}", env.key);
                self.registered_env_vars.push(env);
            }
        }

        if let Some(var_captures) = VAR_REGEX.captures(&manifest_content) {
            let var = var_captures.name("var").unwrap().as_str();
            let preset_var = self
                .yambs_variables
                .iter()
                .find(|pvar| pvar.key == var)
                .ok_or_else(|| PreprocessorError::NoSuchPreset(var.to_string()))?;

            let total_capture = var_captures.get(0).unwrap().as_str();
            manifest_content = manifest_content.replace(total_capture, &preset_var.value);
        }

        Ok(manifest_content)
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

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;

    use crate::tests::EnvLock;

    struct Fixture<'env> {
        preprocessor: Preprocessor,
        _lock: EnvLock<'env>,
    }

    impl Fixture<'_> {
        pub fn new() -> Self {
            let vcpkg_root = EnvironmentVariable {
                key: "VCPKG_ROOT".to_string(),
                value: std::ffi::OsString::from("vcpkg-root"),
            };
            let _lock = EnvLock::lock(&vcpkg_root.key, &vcpkg_root.value.to_str().unwrap());
            Self {
                preprocessor: Preprocessor::new()
                    .with_var(Variable {
                        key: "YAMBS_MANIFEST_DIR".to_string(),
                        value: "manifest-dir".to_string(),
                    })
                    .with_env_var(vcpkg_root),
                _lock,
            }
        }
    }

    #[test]
    fn preprocessor_replaces_vars() {
        let mut fixture = Fixture::new();
        let input = "\
        [executable.factory]
        sources = [
           \"${YAMBS_MANIFEST_DIR}/src/main.cpp\",
           \"${YAMBS_MANIFEST_DIR}/src/boat.cpp\",
           \"${YAMBS_MANIFEST_DIR}/src/truck.cpp\"
        ]";

        let expected = "\
        [executable.factory]
        sources = [
           \"manifest-dir/src/main.cpp\",
           \"manifest-dir/src/boat.cpp\",
           \"manifest-dir/src/truck.cpp\"
        ]";

        let actual = fixture.preprocessor.parse(input).unwrap();
        assert_eq!(actual.as_str(), expected);
    }

    #[test]
    fn preprocessor_replaces_env_vars() {
        let mut fixture = Fixture::new();
        let input = "\
        [executable.factory]
        sources = [
           \"${YAMBS_MANIFEST_DIR}/src/main.cpp\",
           \"${YAMBS_MANIFEST_DIR}/src/boat.cpp\",
           \"${YAMBS_MANIFEST_DIR}/src/truck.cpp\"
        ]
        append_system_include_directories = [
           \"${env:VCPKG_ROOT}/third-party/include/impl\",
           \"${env:VCPKG_ROOT}/second-third-party/include/impl\",
        ]
        append_include_directories = [
           \"${YAMBS_MANIFEST_DIR}/include/impl\",
        ]";

        let expected = "\
        [executable.factory]
        sources = [
           \"manifest-dir/src/main.cpp\",
           \"manifest-dir/src/boat.cpp\",
           \"manifest-dir/src/truck.cpp\"
        ]
        append_system_include_directories = [
           \"vcpkg-root/third-party/include/impl\",
           \"vcpkg-root/second-third-party/include/impl\",
        ]
        append_include_directories = [
           \"manifest-dir/include/impl\",
        ]";

        let actual = fixture.preprocessor.parse(input).unwrap();
        assert_eq!(actual.as_str(), expected);
    }
}
