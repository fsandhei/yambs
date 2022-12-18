use std::process::Command;
use std::vec::Vec;

use crate::build_state_machine::filter;
use crate::errors::FsError;
use crate::output;

lazy_static::lazy_static! {
    static ref PROGRAM_ROOT_PATHS: Vec<std::path::PathBuf> = {
        vec![
            std::path::PathBuf::from("/usr/bin"),
            std::path::PathBuf::from("/usr/.local/bin")
        ]
    };
}

fn find_program(program: &str) -> Option<std::path::PathBuf> {
    for path in &*PROGRAM_ROOT_PATHS {
        let executable_path = path.join(program);
        log::debug!("Looking for {} in {}", program, path.display());
        if executable_path.is_file() {
            log::debug!("Found {} as {}", program, executable_path.display());
            return Some(executable_path);
        }
    }
    None
}

#[derive(Debug)]
pub struct Make {
    configs: Vec<String>,
    executable: std::path::PathBuf,
    process: Option<std::process::Child>,
}

impl Make {
    pub fn new() -> Result<Self, FsError> {
        let jobs = Jobs::default();
        let configs = vec!["-j".to_string(), jobs.0.to_string()];
        let executable =
            find_program("make").ok_or_else(|| FsError::CouldNotFindProgram("make".to_string()))?;

        Ok(Self {
            configs,
            executable,
            process: None,
        })
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
        log::debug!(
            "Running \"{} {}\" in directory {}",
            self.executable.display(),
            self.configs.join(" "),
            makefile_directory.display()
        );
        let child = Command::new(&self.executable)
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

#[derive(Debug)]
struct Jobs(usize);

impl Jobs {
    fn calculate_heuristic() -> usize {
        const HEURISTIC_MULTIPLIER: usize = 2;
        HEURISTIC_MULTIPLIER * num_cpus::get()
    }
}

impl std::default::Default for Jobs {
    fn default() -> Self {
        Self {
            0: Jobs::calculate_heuristic(),
        }
    }
}
