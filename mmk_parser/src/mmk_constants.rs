/* 
   * Konstanter som man burde ha:
   * project_top: I praksis directory opp for .mmk - fila
   * src_dir: src/source - dir
*/
use std::cmp::Eq;
use std::hash::Hash;
use std::collections::HashMap;
use regex::Regex;
use std::string::String;
use std::path::PathBuf;

use utility;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Constant {
    keyword: String,
}

impl Constant {
    pub fn new(keyword: &str) -> Self {
        Self { keyword: String::from(keyword) }
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Constants {
    list: HashMap<Constant, String>
}

impl Constants {
    pub fn new(path_to_mmk_dir: &PathBuf, src_path: &PathBuf) -> Self {
        let mut collection = HashMap::<Constant, String>::new();
        let path = utility::get_project_top_directory(path_to_mmk_dir);

        collection.insert(Constant::new("project_top"), path.to_str().unwrap().to_string());
        collection.insert(Constant::new("src_dir"), src_path.to_str().unwrap().to_string());
        Self { list: collection }
    }


    pub fn get_constant(&self, data: &String) -> Option<String> {
        let constant_pattern = Regex::new(r"\$\{(.*)\}").unwrap();
        if let Some(captured) = constant_pattern.captures(data) {
            let capture_s = captured.get(1).unwrap().as_str();
            if self.is_constant(&String::from(capture_s)) {
                return Some(capture_s.to_string())
            }
            return None;
        }
        None
    }

    pub fn get_item(&self, key: Constant) -> Option<String> {
        if let Some(item) = self.list.get(&key) {
            return Some(item.clone());
        }
        None
    }


    fn is_constant(&self, data: &String) -> bool {
        if data == "project_top"
        || data == "src_dir" {
            return true;
        }
        false
    }
}


#[cfg(test)]
mod tests {
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
}