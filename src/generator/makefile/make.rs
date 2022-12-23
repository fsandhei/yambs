use std::process::Command;
use std::vec::Vec;

use crate::errors::FsError;
use crate::output;
use crate::output::filter;

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

pub struct BuildProcess(std::process::Child);

impl BuildProcess {
    pub fn wait_with_output(self, output: &output::Output) -> std::process::Output {
        let process_output = self.0.wait_with_output().unwrap();
        Make::log(&process_output, output).unwrap();
        process_output
    }
}

#[derive(Debug)]
struct MakeArgs(Vec<String>);

impl MakeArgs {
    fn from_slice(slice: &[String]) -> Self {
        let mut args = Self::default();
        args.0.extend_from_slice(slice);
        args
    }
}

impl std::default::Default for MakeArgs {
    fn default() -> Self {
        let jobs = Jobs::default();
        let jobs_as_args = jobs_to_args(jobs);
        Self(jobs_as_args.to_vec())
    }
}

impl std::iter::IntoIterator for MakeArgs {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> std::iter::IntoIterator for &'a MakeArgs {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

#[derive(Debug)]
pub struct Make {
    args: MakeArgs,
    executable: std::path::PathBuf,
}

impl Make {
    pub fn new(args: &[String]) -> Result<Self, FsError> {
        let args = MakeArgs::from_slice(args);
        let executable =
            find_program("make").ok_or_else(|| FsError::CouldNotFindProgram("make".to_string()))?;

        Ok(Self { args, executable })
    }

    pub fn run(&self) -> Result<BuildProcess, FsError> {
        let child = Command::new(&self.executable)
            .args(&self.args)
            .stderr(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .map_err(|_| FsError::Spawn(Command::new(self.executable.display().to_string())))?;
        Ok(BuildProcess(child))
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
}

fn jobs_to_args(jobs: Jobs) -> [String; 2] {
    ["-j".to_string(), jobs.0.to_string()]
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
        Self(Jobs::calculate_heuristic())
    }
}
