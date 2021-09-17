use colored::Colorize;
pub trait MyMakeUnwrap<T> {
    fn unwrap_or_terminate(self) -> T;
}

impl<T, E> MyMakeUnwrap<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn unwrap_or_terminate(self) -> T {
        match self {
            Ok(t) => t,
            Err(err) => {
                eprintln!("\r{}", format!("DMake: {}", err).red());
                std::process::exit(1);
            }
        }
    }
}

impl<T> MyMakeUnwrap<T> for Option<T> {
    fn unwrap_or_terminate(self) -> T {
        match self {
            Some(t) => t,
            None => {
                eprintln!("{}", "DMake: Invalid input or no input given!".red());
                std::process::exit(1);
            }
        }
    }
}
