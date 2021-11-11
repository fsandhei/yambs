use dependency::{DependencyAccessor, DependencyNode};
use error::{DependencyError, GeneratorError};

pub use crate::generator::{Generator, GeneratorExecutor, RuntimeSettings, Sanitizer};

pub struct GeneratorMock {
    dep: Option<DependencyNode>,
}

impl GeneratorMock {
    pub fn new() -> Self {
        Self { dep: None }
    }
}

impl GeneratorExecutor for GeneratorMock {
    fn generate_makefiles(&mut self, _dependency: &DependencyNode) -> Result<(), GeneratorError> {
        Ok(())
    }
}

impl Generator for GeneratorMock {
    fn generate_makefile(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_rule_executable(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_rule_package(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn generate_appending_flags(&mut self) -> Result<(), GeneratorError> {
        Ok(())
    }

    fn print_ok(&self) {}
}

impl Sanitizer for GeneratorMock {
    fn set_sanitizers(&mut self, _: &[String]) {}
}

impl RuntimeSettings for GeneratorMock {
    fn debug(&mut self) {}

    fn release(&mut self) {}

    fn use_std(&mut self, _version: &str) -> Result<(), GeneratorError> {
        Ok(())
    }
}

impl DependencyAccessor for GeneratorMock {
    fn set_dependency(&mut self, _: &DependencyNode) {}
    fn get_dependency(&self) -> Result<&DependencyNode, DependencyError> {
        if let Some(dependency) = &self.dep {
            return Ok(dependency);
        }
        return Err(DependencyError::NotSet);
    }
}
