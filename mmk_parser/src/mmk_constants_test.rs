use super::*;

fn fixture_construct_constants() -> Constants {
    let path_to_mmk_dir = PathBuf::from("/some/project/top/path/");
    let src_dir = PathBuf::from("/some/project/top/path/source");
    Constants::new(&path_to_mmk_dir, &src_dir)
}
#[test]
fn get_constant_test() {
    let constants = fixture_construct_constants();
    let expected = "project_top";
    let input = String::from("${project_top}");
    let actual = constants.get_constant(&input).unwrap();
    assert_eq!(actual, expected);
}

#[test]
fn get_constant_invalid_test() {
    let constants = fixture_construct_constants();
    let input = String::from("${project_top_wrong}");
    let actual = constants.get_constant(&input);
    assert!(actual.is_none());
}

#[test]
fn is_constant_test() {
    let constants = fixture_construct_constants();
    let input = String::from("project_top");
    let actual = constants.is_constant(&input);
    assert_eq!(actual, true);
}

#[test]
fn is_constant_invalid_test() {
    let constants = fixture_construct_constants();
    let input = String::from("project_top_wrong");
    let actual = constants.is_constant(&input);
    assert_eq!(actual, false);
}
