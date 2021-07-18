use error::MyMakeError;
use dependency::{DependencyNode, DependencyAccessor};

pub trait Generator
    : DependencyAccessor
    + Sanitizer
    + RuntimeSettings {
    // Generate functions
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), MyMakeError>;
    fn generate_makefile(&mut self)                               -> Result<(), MyMakeError>;
    fn generate_header(&mut self)                                 -> Result<(), MyMakeError>;
    fn generate_rule_executable(&mut self)                        -> Result<(), MyMakeError>;
    fn generate_rule_package(&mut self)                           -> Result<(), MyMakeError>;
    fn generate_appending_flags(&mut self)                        -> Result<(), MyMakeError>;

    // Finish status: Just for display purposes. Will be removed.
    fn print_ok(&self);
}

pub trait Sanitizer {
    fn set_sanitizers(&mut self, sanitizers: Vec<&str>);
}


pub trait RuntimeSettings {
    // Runtime settings
    fn debug(&mut self);
    fn release(&mut self);
    fn use_std(&mut self, version: &str) -> Result<(), MyMakeError>;
}