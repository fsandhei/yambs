use crate::errors;

pub trait Executor {
    fn spawn<I>(
        &self,
        directory: &std::path::Path,
        args: I,
    ) -> Result<std::process::Child, errors::FsError>
    where
        I: std::iter::IntoIterator<Item = String>;
}
