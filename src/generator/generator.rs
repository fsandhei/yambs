use crate::dependency::target::TargetNode;
use crate::generator::GeneratorError;

pub trait Generator: Sanitizer + RuntimeSettings {
    fn generate_makefile(&mut self) -> Result<(), GeneratorError>;
    fn generate_rule_executable(&mut self) -> Result<(), GeneratorError>;
    fn generate_rule_package(&mut self) -> Result<(), GeneratorError>;
    fn generate_appending_flags(&mut self) -> Result<(), GeneratorError>;
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
    fn generate_makefiles(&mut self, dependency: &TargetNode) -> Result<(), GeneratorError>;
}

pub trait UtilityGenerator<'config> {
    fn generate_makefiles(&'config mut self) -> Result<(), GeneratorError>;
    fn add_cpp_version(&mut self, version: &'config str);
    fn print_cpp_version(&'config self) -> &'config str;
    fn generate_flags_sanitizer(&self) -> String;
}
