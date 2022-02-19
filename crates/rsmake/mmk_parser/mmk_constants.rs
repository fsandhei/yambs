/*
 * Konstanter som man burde ha:
 * project_top: I praksis directory opp for .mmk - fila. Burde være konstant for et prosjekt. Er det mulig å gjøre?
 * src_dir: src/source - dir: Trengs denne?
 * os : operativsystem.
*/
use regex::Regex;
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::string::String;

use crate::utility;

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
pub struct Constant {
    keyword: String,
}

impl Constant {
    pub fn new(keyword: &str) -> Self {
        Self {
            keyword: String::from(keyword),
        }
    }
}

impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.keyword)
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Constants {
    list: HashMap<Constant, String>,
}

impl Constants {
    pub fn new(path_to_mmk_dir: &Path, src_path: &Path) -> Self {
        let mut collection = HashMap::<Constant, String>::new();
        let path = utility::get_project_top_directory(path_to_mmk_dir);

        collection.insert(
            Constant::new("project_top"),
            path.to_str().unwrap().to_string(),
        );
        collection.insert(
            Constant::new("src_dir"),
            src_path.to_str().unwrap().to_string(),
        );
        Self { list: collection }
    }

    pub fn get_constant(&self, data: &String) -> Option<String> {
        let constant_pattern = Regex::new(r"\$\{(.*)\}").unwrap();
        if let Some(captured) = constant_pattern.captures(data) {
            let capture_s = captured.get(1).unwrap().as_str();
            if self.is_constant(&String::from(capture_s)) {
                return Some(capture_s.to_string());
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
        if data == "project_top" || data == "src_dir" {
            return true;
        }
        false
    }
}

#[cfg(test)]
#[path = "./mmk_constants_test.rs"]
mod mmk_contants_test;
