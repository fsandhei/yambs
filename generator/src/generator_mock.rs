use dependency::{DependencyNode, DependencyAccessor};
use error::MyMakeError;

pub use crate::generator::{Generator, GeneratorExecutor, Sanitizer, RuntimeSettings};

pub struct GeneratorMock {
    dep: Option<DependencyNode>
}

impl GeneratorMock {
    pub fn new() -> Self {
        Self { dep: None }
    }
}


impl GeneratorExecutor for GeneratorMock {
    fn generate_makefiles(&mut self, _dependency: &DependencyNode) -> Result<(), MyMakeError> {
        Ok(())
    }
}


impl Generator for GeneratorMock {
    fn generate_makefile(&mut self) -> Result<(), MyMakeError> {
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
    
    fn print_ok(&self) {}
}


impl Sanitizer for GeneratorMock {
    fn set_sanitizers(&mut self, _: Vec<&str>) {}
}


impl RuntimeSettings for GeneratorMock {
    fn debug(&mut self) {}

    fn release(&mut self) {}

    fn use_std(&mut self, _version: &str) -> Result<(), MyMakeError> {
        Ok(())
    }
}


impl DependencyAccessor for GeneratorMock {
    fn set_dependency(&mut self, _: &DependencyNode) {}   
    fn get_dependency(&self) -> Result<&DependencyNode, MyMakeError> {
        if let Some(dependency) = &self.dep {
            return Ok(dependency)
        }
        return Err(MyMakeError::from_str("Call on get_dependency when dependency is not set. Call on set_dependency must be done prior!"));
    }
}