use std::process::Command;
use std::vec::Vec;

use crate::build_state_machine::filter;
use crate::errors::FsError;
use crate::output;

#[derive(Default)]
pub struct Make {
    configs: Vec<String>,
    process: Option<std::process::Child>,
}

impl Make {
    pub fn with_flag(&mut self, flag: &str, value: &str) -> &mut Make {
        self.configs.push(flag.to_string());
        self.configs.push(value.to_string());
        self
    }

    fn log(process_output: &std::process::Output, output: &output::Output) -> Result<(), FsError> {
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

    pub fn spawn(&mut self, makefile_directory: &std::path::Path) -> Result<(), FsError> {
        std::env::set_current_dir(makefile_directory).map_err(FsError::AccessDirectory)?;
        log::debug!("Running make in directory {}", makefile_directory.display());
        let child = Command::new("/usr/bin/make")
            .args(&self.configs)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| FsError::Spawn(Command::new("/usr/bin/make")))?;
        self.process = Some(child);
        Ok(())
    }

    pub fn wait_with_output(&mut self, output: &output::Output) -> std::process::Output {
        if let Some(process) = self.process.take() {
            let process_output = process.wait_with_output().unwrap();
            Make::log(&process_output, output).unwrap();
            process_output
        } else {
            panic!("No process to call wait on!");
        }
    }

    pub fn spawn_with_args<I>(
        &mut self,
        makefile_directory: &std::path::Path,
        args: I,
    ) -> Result<(), FsError>
    where
        I: std::iter::IntoIterator<Item = String>,
    {
        self.configs.extend(args);
        self.spawn(makefile_directory)
    }
}
