use error::MyMakeError;

pub trait Generator
{
    fn generate_makefile(self: &mut Self)        -> Result<(), MyMakeError>;
    fn generate_header(self: &mut Self)          -> Result<(), MyMakeError>;
    fn generate_rule_executable(self: &mut Self) -> Result<(), MyMakeError>;
    fn generate_rule_package(self: &mut Self)    -> Result<(), MyMakeError>;
    fn generate_appending_flags(&mut self)       -> Result<(), MyMakeError>;
    fn print_ok(self: &Self);
}