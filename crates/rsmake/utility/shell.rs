use crate::errors::FsError;

pub fn execute_get_stdout(exe: &std::path::Path, args: &[&str]) -> Result<String, FsError> {
    let output = std::process::Command::new(exe)
        .args(args.into_iter())
        .output()
        .unwrap();
    if !output.status.success() {
        let stderr =
            String::from_utf8(output.stderr).expect("Failed to create string from u8 array.");
        return Err(FsError::FailedToExecute {
            exe: exe.to_path_buf(),
            args: args.join(" "),
            stderr,
        });
    }
    String::from_utf8(output.stdout).map_err(FsError::FailedToCreateStringFromUtf8)
}
