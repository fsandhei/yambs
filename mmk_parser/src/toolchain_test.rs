
use super::*;

#[test]
fn parse_compiler_test() {
    let mut toolchain = Toolchain::new();
    let input = String::from("compiler = /some/path/to/gcc");
    let result = toolchain.parse(input);
    assert!(result.is_ok());
    assert_eq!(toolchain.config[&Constant::new("compiler")], PathBuf::from("/some/path/to/gcc"));
}


#[test]
fn parse_linker_test() {
    let mut toolchain = Toolchain::new();
    let input = String::from("linker = /some/path/to/ld_gold");
    let result = toolchain.parse(input);
    assert!(result.is_ok());
    assert_eq!(toolchain.config[&Constant::new("linker")], PathBuf::from("/some/path/to/ld_gold"));
}


#[test]
fn parse_compiler_and_linker_test() {
    let mut toolchain = Toolchain::new();
    let input = String::from("linker = /some/path/to/ld_gold\n\
                                     compiler = /some/path/to/gcc\n");
    let result = toolchain.parse(input);
    assert!(result.is_ok());
    assert_eq!(toolchain.config[&Constant::new("linker")], PathBuf::from("/some/path/to/ld_gold"));
    assert_eq!(toolchain.config[&Constant::new("compiler")], PathBuf::from("/some/path/to/gcc"));
}