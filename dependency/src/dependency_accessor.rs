use crate::DependencyNode;

use error::DependencyError;

pub trait DependencyAccessor {
    // Dependency accessors
    fn set_dependency(&mut self, dependency: &DependencyNode);
    fn get_dependency(&self) -> Result<&DependencyNode, DependencyError>;
}
