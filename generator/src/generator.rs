use error::MyMakeError;
use dependency::DependencyNode;

pub trait Generator {
    // Generate functions
    fn generate_makefiles(&mut self, dependency: &DependencyNode) -> Result<(), MyMakeError>;
    fn generate_makefile(&mut self)                               -> Result<(), MyMakeError>;
    fn generate_header(&mut self)                                 -> Result<(), MyMakeError>;
    fn generate_rule_executable(&mut self)                        -> Result<(), MyMakeError>;
    fn generate_rule_package(&mut self)                           -> Result<(), MyMakeError>;
    fn generate_appending_flags(&mut self)                        -> Result<(), MyMakeError>;

    // Runtime settings
    fn debug(&mut self);
    fn release(&mut self);
    fn use_std(&mut self, version: &str) -> Result<(), MyMakeError>;

    // Dependency accessors
    fn set_dependency(&mut self, dependency: &DependencyNode);
    fn get_dependency(&self) -> Result<&DependencyNode, MyMakeError>;

    // Finish status: Just for display purposes. Will be removed.
    fn print_ok(&self);
}