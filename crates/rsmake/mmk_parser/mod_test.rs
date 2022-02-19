use super::*;
use pretty_assertions::assert_eq;
use tempdir::TempDir;

use keyword::Keyword;
use mmk_constants::Constant;
use utility;

#[test]
fn test_mmk_file_reader() {
    let current_dir = std::env::current_dir().expect(
        "Could not retrieve current directory for
                                                                 mmk_parser::test_mmk_file_reader",
    );
    let path = current_dir.join("src/test.mmk");
    let content = utility::read_file(&path);
    assert_eq!(
        content.unwrap(),
        ("\
#This is a comment.
MMK_REQUIRE:
   /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/

MMK_SOURCES:
   filename.cpp
   otherfilename.cpp

#This is a second comment.
MMK_EXECUTABLE:
   x\n")
    );
}

#[test]
fn test_remove_comments() {
    let current_dir = std::env::current_dir().expect(
        "Could not retrieve current directory for
    mmk_parser::test_mmk_file_reader",
    );
    let path = current_dir.join("src/test.mmk");

    let content = utility::read_file(&path).unwrap();
    assert_eq!(
        remove_comments(&content),
        String::from(
            "
MMK_REQUIRE:
   /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/

MMK_SOURCES:
   filename.cpp
   otherfilename.cpp


MMK_EXECUTABLE:
   x\n"
        )
    );
}

#[test]
fn test_to_string_mmk_sources() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:\n\
                                            filename.cpp\n\
                                            otherfilename.cpp\n",
    );

    mmk_content.parse(&content)?;
    let expected = "filename.cpp otherfilename.cpp";
    let actual = mmk_content.to_string("MMK_SOURCES");
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn test_to_string_mmk_headers() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_HEADERS:\n\
                                            filename.h\n\
                                            otherfilename.h\n",
    );

    mmk_content.parse(&content)?;
    let expected = "filename.h otherfilename.h";
    let actual = mmk_content.to_string("MMK_HEADERS");
    assert_eq!(expected, actual);
    Ok(())
}

#[test]
fn test_parse_mmk_sources() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:\n\
                                            filename.cpp\n\
                                            otherfilename.cpp\n",
    );

    mmk_content.parse(&content)?;
    assert_eq!(
        mmk_content.data["MMK_SOURCES"],
        [
            Keyword::from("filename.cpp"),
            Keyword::from("otherfilename.cpp")
        ]
    );
    Ok(())
}

#[test]
fn test_parse_mmk_source() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:\n\
                                            filename.cpp",
    );
    mmk_content.parse(&content)?;
    assert_eq!(
        mmk_content.data["MMK_SOURCES"],
        [Keyword::from("filename.cpp")]
    );
    Ok(())
}

#[test]
fn test_valid_keyword_mmk_sources() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_SOURCES").is_ok());
}

#[test]
fn test_valid_keyword_mmk_headers() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_HEADERS").is_ok());
}

#[test]
fn test_valid_keyword_mmk_require() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_REQUIRE").is_ok());
}

#[test]
fn test_valid_keyword_mmk_executable() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_EXECUTABLE").is_ok());
}

#[test]
fn test_valid_keyword_mmk_sys_include() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_SYS_INCLUDE").is_ok());
}

#[test]
fn test_valid_keyword_mmk_cppflags_append() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_CPPFLAGS_APPEND").is_ok());
}

#[test]
fn test_valid_keyword_mmk_cxxflags_append() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert!(mmk_content.valid_keyword("MMK_CXXFLAGS_APPEND").is_ok());
}

#[test]
fn test_parse_dependencies() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_REQUIRE:\n\
                                            /some/path/to/depend/on \n\
                                            /another/path/to/depend/on\n",
    );
    mmk_content.parse(&content)?;
    assert_eq!(
        mmk_content.data["MMK_REQUIRE"],
        [
            Keyword::from("/some/path/to/depend/on"),
            Keyword::from("/another/path/to/depend/on")
        ]
    );
    Ok(())
}

#[test]
fn test_multiple_keywords() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:
                                            filename.cpp
                                            otherfilename.cpp
                                        
                                        MMK_REQUIRE:
                                            /some/path/to/depend/on
                                            /another/path/
                                                        
                                        MMK_EXECUTABLE:
                                            main",
    );
    mmk_content.parse(&content)?;
    assert_eq!(
        mmk_content.data["MMK_SOURCES"],
        [
            Keyword::from("filename.cpp"),
            Keyword::from("otherfilename.cpp")
        ]
    );
    assert_eq!(
        mmk_content.data["MMK_REQUIRE"],
        [
            Keyword::from("/some/path/to/depend/on"),
            Keyword::from("/another/path/")
        ]
    );
    assert_eq!(mmk_content.data["MMK_EXECUTABLE"], [Keyword::from("main")]);
    Ok(())
}

#[test]
fn test_has_library_label_true() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_LIBRARY_LABEL:\n\
                                            myLib",
    );

    mmk_content.parse(&content)?;
    assert!(mmk_content.has_library_label());
    Ok(())
}

#[test]
fn test_has_library_label_false() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:\n\
                                            my_source.cpp",
    );

    mmk_content.parse(&content)?;
    assert!(!mmk_content.has_library_label());
    Ok(())
}

#[test]
fn test_has_system_include_true() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SYS_INCLUDE:\n\
                                            /some/third/party/software/",
    );

    mmk_content.parse(&content)?;
    assert!(mmk_content.has_system_include());
    Ok(())
}

#[test]
fn test_has_system_include_false() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SOURCES:\n\
                                            my_source.cpp",
    );

    mmk_content.parse(&content)?;
    assert!(!mmk_content.has_system_include());
    Ok(())
}

#[test]
fn test_parse_mmk_no_valid_keyword() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_REQUIRES:\n\
                                            /some/path/to/depend/on \n\
                                            /another/path/to/depend/on\n",
    );
    let result = mmk_content.parse(&content);
    assert!(result.is_err());
    assert_eq!(
        &String::from("/some/path/lib.mmk: MMK_REQUIRES is not a valid MMK keyword!"),
        &result.unwrap_err().to_string()
    );
    Ok(())
}

#[test]
fn test_parse_mmk_invalid_spacing_between_keywords() -> Result<(), ParseError> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_REQUIRE:\n\
                                            /some/path/to/depend/on\n\
                                        MMK_SOURCES:\n\
                                            some_file.cpp\n",
    );
    let result = mmk_content.parse(&content);
    assert!(result.is_err());
    assert_eq!(
        String::from(
            "/some/path/lib.mmk: Invalid spacing of arguments! Keep at least one line between each RsMake keyword."
        ),
        result.unwrap_err().to_string()
    );
    Ok(())
}

#[test]
fn get_include_directories_for_make_test() -> std::io::Result<()> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let dir = TempDir::new("example")?;
    let src_dir = dir.path().join("src");
    let include_dir = dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    let content: String = format!(
        "MMK_REQUIRE:\n\
                                        {}",
        src_dir.to_str().unwrap()
    );

    mmk_content.parse(&content).unwrap();
    let actual = mmk_content.get_include_directories();
    assert!(actual.is_ok());
    assert_eq!(
        actual.unwrap(),
        format!("-I{}", include_dir.to_str().unwrap())
    );
    Ok(())
}

#[test]
fn get_include_directories_for_make_system_option_set_test() -> std::io::Result<()> {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let dir = TempDir::new("example")?;
    let src_dir = dir.path().join("src");
    let include_dir = dir.path().join("include");
    utility::create_dir(&include_dir).unwrap();
    let content: String = format!(
        "MMK_REQUIRE:\n\
                                        {} SYSTEM",
        src_dir.to_str().unwrap()
    );

    mmk_content.parse(&content).unwrap();
    let actual = mmk_content.get_include_directories();
    assert!(actual.is_ok());
    assert_eq!(
        actual.unwrap(),
        format!("-isystem {}", include_dir.to_str().unwrap())
    );
    Ok(())
}

#[test]
fn validate_file_name_test() {
    let some_valid_file_path = PathBuf::from("lib.mmk");
    assert!(validate_file_name(&some_valid_file_path).is_ok());
}

#[test]
fn validate_file_name_invalid_file_name_test() {
    let some_invalid_file_path = PathBuf::from("mymakeinfo.mmk");
    let result = validate_file_name(&some_invalid_file_path);
    assert!(result.is_err());
    assert_eq!(
        String::from("\"mymakeinfo.mmk\" is not a valid RsMake filename! File must be named lib.mmk or run.mmk."),
        result.unwrap_err().to_string()
    );
}

#[test]
fn test_to_string_mmk_sys_include() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_SYS_INCLUDE:\n\
                                            /some/third/party/software/\n\
                                            /second/third/party/thing",
    );
    mmk_content.parse(&content).unwrap();
    let expected =
        String::from("-isystem /some/third/party/software/ -isystem /second/third/party/thing");
    let actual = mmk_content.to_string("MMK_SYS_INCLUDE");
    assert_eq!(expected, actual);
}

#[test]
fn test_constant_has_top_path_variable() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mmk_content = Mmk::new(&path);
    assert_eq!(
        mmk_content
            .constants
            .get_item(Constant::new("project_top"))
            .unwrap(),
        "/some/path"
    );
}

#[test]
fn test_constant_is_replaced_with_item() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_REQUIRE:\n\
                                            ${project_top}/depend/on \n\
                                            ${project_top}/second/depend\n",
    );
    let result = mmk_content.parse(&content);
    assert!(result.is_ok());
    let expected = [
        Keyword::from("/some/path/depend/on"),
        Keyword::from("/some/path/second/depend"),
    ];

    assert_eq!(mmk_content.data["MMK_REQUIRE"], expected);
}

#[test]
fn test_optional_arguments_to_keyword_arguments() {
    let path = PathBuf::from("/some/path/lib.mmk");
    let mut mmk_content = Mmk::new(&path);
    let content: String = String::from(
        "MMK_REQUIRE:\n\
                                            ${project_top}/depend/on SYSTEM",
    );
    let result = mmk_content.parse(&content);
    assert!(result.is_ok());
    let expected = [Keyword::from("/some/path/depend/on").with_option("SYSTEM")];
    assert_eq!(mmk_content.data["MMK_REQUIRE"], expected);
}
