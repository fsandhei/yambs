use dependency::DependencyNode;
use error::MyMakeError;

pub use crate::generator::Generator;

pub struct GeneratorMock {
}

impl GeneratorMock {
}

impl Generator for GeneratorMock {
    fn generate_makefiles(&mut self, _dependency: &DependencyNode) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn generate_makefile(&mut self) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn generate_header(&mut self) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn generate_rule_executable(&mut self) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn generate_rule_package(&mut self) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn generate_appending_flags(&mut self) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn debug(&mut self) {}

    fn release(&mut self) {}

    fn use_std(&mut self, _version: &str) -> Result<(), MyMakeError> {
        Ok(())
    }

    fn print_ok(&self) {}
}