use dependency::DependencyNode;
use error::MyMakeError;

pub use crate::generator::Generator;

pub struct GeneratorMock {
    dependency: Option<DependencyNode>,
}

impl GeneratorMock {
    pub fn new() -> Self {
        Self{ dependency: None }
    }
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
    
    fn set_dependency(&mut self, dependency: &DependencyNode) {
        self.dependency = Some(dependency.clone());
    }

    fn get_dependency(&self) -> Result<&DependencyNode, MyMakeError> {
        if let Some(dep) = &self.dependency {
            return Ok(dep);
        }
        return Err(MyMakeError::from_str("Call on get_dependency when dependency is not set. Call on set_dependency must be done prior!"));
    }
}