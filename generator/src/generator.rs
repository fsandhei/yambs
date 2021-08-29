use error::MyMakeError;
use dependency::{DependencyNode, DependencyAccessor};

pub trait Generator
    : DependencyAccessor
    + Sanitizer
    + RuntimeSettings {
    fn generate_makefile(&mut self)        -> Result<(), MyMakeError>;
    fn generate_rule_executable(&mut self) -> Result<(), MyMakeError>;
    fn generate_rule_package(&mut self)    -> Result<(), MyMakeError>;
    fn generate_appending_flags(&mut self) -> Result<(), MyMakeError>;

    // Finish status: Just for display purposes. Will be removed.
    fn print_ok(&self);
}

pub trait Sanitizer {
    fn set_sanitizers(&mut self, sanitizers: &[String]);
}


pub trait RuntimeSettings {
    fn debug(&mut self);
    fn release(&mut self);
    fn use_std(&mut self, version: &str) -> Result<(), MyMakeError>;
}


pub trait GeneratorExecutor : Generator {
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), MyMakeError>;
}


pub trait UtilityGenerator<'config> {
    fn generate_makefiles(&'config mut self) -> Result<(), MyMakeError>;
    fn add_cpp_version(&mut self, version: &'config str) -> Result<(), MyMakeError>;
    fn print_cpp_version(&'config self) -> &'config str;
    fn generate_flags_sanitizer(&self) -> String;
}