use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct MyMakeError {
    details: String
}

impl MyMakeError {
    #[cfg(maybe_unused)]
    pub fn new(msg: &str) -> MyMakeError {
        MyMakeError{details: msg.to_string()}
    }
    pub fn from(msg: String) -> MyMakeError {
        MyMakeError{details: msg}
    }
}

impl fmt::Display for MyMakeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl Error for MyMakeError {
    fn description(&self) -> &str {
        &self.details 
    }
}

impl std::convert::From<std::io::Error> for MyMakeError {
    fn from(error: std::io::Error) -> Self {
        MyMakeError::from(format!("{}", error))
    }
}

