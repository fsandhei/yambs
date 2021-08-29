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


#[test]
fn verify_keyword_compiler_test() {
    let toolchain = Toolchain::new();
    assert!(toolchain.verify_keyword("compiler").is_ok());
}


#[test]
fn verify_keyword_linker_test() {
    let toolchain = Toolchain::new();
    assert!(toolchain.verify_keyword("linker").is_ok());
}


#[test]
fn verify_keyword_incorrect_word_test() {
    let toolchain = Toolchain::new();
    let result = toolchain.verify_keyword("derp");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(), 
               &String::from("Error: derp is not allowed as keyword for toolchain."));
}


#[test]
fn verify_filename_correct_name_test() {
    let filename = std::ffi::OsStr::new("toolchain.mmk");
    let toolchain = Toolchain::new();
    let result = toolchain.validate_filename(&filename);
    assert!(result.is_ok());
}



#[test]
fn verify_filename_incorrect_name_test() {
    let filename = std::ffi::OsStr::new("not-a-toolchain.mmk");
    let toolchain = Toolchain::new();
    let result = toolchain.validate_filename(&filename);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().to_string(),
               &String::from("Error: not-a-toolchain.mmk is not a valid name for toolchain file. It must be named toolchain.mmk"))
}