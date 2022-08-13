use std::process::Command;
use std::vec::Vec;

use crate::build_state_machine::filter;
use crate::errors::FsError;
use crate::output;

#[allow(dead_code)]
#[derive(Default)]
pub struct Make {
    configs: Vec<String>,
}

impl Make {
    pub fn with_flag(&mut self, flag: &str, value: &str) -> &mut Make {
        self.configs.push(flag.to_string());
        self.configs.push(value.to_string());
        self
    }

    fn log(
        &self,
        process_output: &std::process::Output,
        output: &output::Output,
    ) -> Result<(), FsError> {
        let stderr = String::from_utf8(process_output.stderr.clone()).unwrap();
        let stdout = String::from_utf8(process_output.stdout.clone()).unwrap();

        let stderr_filtered = filter::filter_string(&stderr);
        if stderr_filtered != *"" {
            filter::println_colored(&stderr_filtered, output);
        }

        if !stdout.is_empty() {
            log::debug!("{}", stdout);
        }
        if !stderr.is_empty() {
            log::debug!("{}", stderr);
        }
        Ok(())
    }

    pub fn spawn(&self, output: &output::Output) -> Result<std::process::Output, FsError> {
        let spawn = Command::new("/usr/bin/make")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .args(&self.configs)
            .spawn()
            .map_err(|_| FsError::Spawn(Command::new("/usr/bin/make")))?;
        let process_output = spawn.wait_with_output().unwrap();
        self.log(&process_output, output)?;
        Ok(process_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn with_flag_test() {
        let mut make = Make::default();
        make.with_flag("-j", "10");
        make.with_flag("-r", "debug");
        assert_eq!(make.configs, ["-j", "10", "-r", "debug"]);
    }
}
