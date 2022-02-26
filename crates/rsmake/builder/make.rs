use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use std::vec::Vec;

use crate::builder::filter;
use crate::errors::{FsError, MakeError};

#[allow(dead_code)]
pub struct Make {
    configs: Vec<String>,
    log_file: Option<File>,
}

impl Make {
    pub fn new() -> Self {
        Self {
            configs: Vec::new(),
            log_file: None,
        }
    }

    pub fn with_flag(&mut self, flag: &str, value: &str) -> &mut Make {
        self.configs.push(flag.to_string());
        self.configs.push(value.to_string());
        self
    }

    pub fn add_logger(&mut self, log_file_name: &PathBuf) -> Result<(), MakeError> {
        let file = std::fs::File::create(&log_file_name);

        self.log_file = match file {
            Ok(file) => Some(file),
            Err(err) => {
                return Err(FsError::CreateFile(log_file_name.into(), err)).map_err(MakeError::Fs)
            }
        };
        Ok(())
    }

    fn log(&self, output: &Output) -> Result<(), MakeError> {
        let stderr = String::from_utf8(output.stderr.clone()).unwrap();
        let stdout = String::from_utf8(output.stdout.clone()).unwrap();

        let stderr_filtered = filter::filter_string(&stderr);
        if stderr_filtered != String::from("") {
            filter::println_colored(&stderr_filtered);
        }

        self.log_file
            .as_ref()
            .unwrap()
            .write(stdout.as_bytes())
            .map_err(FsError::WriteToFile)
            .map_err(MakeError::Fs)?;
        self.log_file
            .as_ref()
            .unwrap()
            .write(b"\n\n")
            .map_err(FsError::WriteToFile)
            .map_err(MakeError::Fs)?;
        self.log_file
            .as_ref()
            .unwrap()
            .write(stderr.as_bytes())
            .map_err(FsError::WriteToFile)
            .map_err(MakeError::Fs)?;
        self.log_file
            .as_ref()
            .unwrap()
            .write(b"\n\n")
            .map_err(FsError::WriteToFile)
            .map_err(MakeError::Fs)?;
        Ok(())
    }

    pub fn log_text(&self, text: String) -> Result<(), MakeError> {
        self.log_file
            .as_ref()
            .unwrap()
            .write(text.as_bytes())
            .map_err(FsError::WriteToFile)
            .map_err(MakeError::Fs)?;
        Ok(())
    }

    pub fn spawn(&self) -> Result<Output, MakeError> {
        let spawn = Command::new("/usr/bin/make")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .args(&self.configs)
            .spawn()
            .map_err(|_| FsError::Spawn(Command::new("/usr/bin/make")))
            .map_err(MakeError::Fs)?;
        let output = spawn.wait_with_output().unwrap();
        self.log(&output)?;
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn with_flag_test() {
        let mut make = Make::new();
        make.with_flag("-j", "10");
        make.with_flag("-r", "debug");
        assert_eq!(make.configs, ["-j", "10", "-r", "debug"]);
    }
}
