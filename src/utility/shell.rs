use crate::errors::FsError;

pub fn execute_get_stdout<I, S>(exe: &std::path::Path, args: I) -> Result<String, FsError>
where
    I: std::iter::IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let output = spawn_and_run(exe, args)?;
    String::from_utf8(output.stdout).map_err(FsError::FailedToCreateStringFromUtf8)
}

pub fn execute<I, S>(exe: &std::path::Path, args: I) -> Result<(), FsError>
where
    I: std::iter::IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    spawn_and_run(exe, args)?;
    Ok(())
}

fn spawn_and_run<I, S>(exe: &std::path::Path, args: I) -> Result<std::process::Output, FsError>
where
    I: std::iter::IntoIterator<Item = S>,
    S: AsRef<std::ffi::OsStr>,
{
    let child = std::process::Command::new(exe)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(FsError::SpawnChild)?;
    child.wait_with_output().map_err(FsError::FailedToExecute)
}
