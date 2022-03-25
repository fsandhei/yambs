use crate::dependency::{DependencyAccessor, DependencyNode};
use crate::errors::GeneratorError;

pub trait Generator: DependencyAccessor + Sanitizer + RuntimeSettings {
    fn generate_makefile(&mut self) -> Result<(), GeneratorError>;
    fn generate_rule_executable(&mut self) -> Result<(), GeneratorError>;
    fn generate_rule_package(&mut self) -> Result<(), GeneratorError>;
    fn generate_appending_flags(&mut self) -> Result<(), GeneratorError>;

    // Finish status: Just for display purposes. Will be removed.
    #[deprecated = "Figured that this function is redundant. Will be removed."]
    fn print_ok(&self);
}

pub trait Sanitizer {
    fn set_sanitizer(&mut self, sanitizer: &str);
}

pub trait RuntimeSettings {
    fn debug(&mut self);
    fn release(&mut self);
    fn use_std(&mut self, version: &str) -> Result<(), GeneratorError>;
}

pub trait GeneratorExecutor: Generator {
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), GeneratorError>;
}

pub trait UtilityGenerator<'config> {
    fn generate_makefiles(&'config mut self) -> Result<(), GeneratorError>;
    fn add_cpp_version(&mut self, version: &'config str);
    fn print_cpp_version(&'config self) -> &'config str;
    fn generate_flags_sanitizer(&self) -> String;
}