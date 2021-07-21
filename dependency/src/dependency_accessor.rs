use crate::DependencyNode;

use error::MyMakeError;

pub trait DependencyAccessor {
    // Dependency accessors
    fn set_dependency(&mut self, dependency: &DependencyNode);
    fn get_dependency(&self) -> Result<&DependencyNode, MyMakeError>;
}