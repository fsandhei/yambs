/*
 * Konstanter som man burde ha:
 * project_top: I praksis directory opp for .mmk - fila. Burde være konstant for et prosjekt. Er det mulig å gjøre?
 * src_dir: src/source - dir: Trengs denne?
 * os : operativsystem.
*/
use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::path::Path;
use std::string::String;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Eq, PartialEq, Hash, Clone, Serialize, Deserialize)]
pub struct Constant(String);

impl Constant {
    pub fn new(keyword: &str) -> Self {
        Self(String::from(keyword))
    }
}

impl std::fmt::Display for Constant {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct Constants {
    list: HashMap<Constant, String>,
}

impl Constants {
    #[allow(dead_code)]
    pub fn new(toml_manifest_dir: &Path, src_path: &Path) -> Self {
        let mut collection = HashMap::<Constant, String>::new();

        collection.insert(
            Constant::new("project_top"),
            toml_manifest_dir.display().to_string(),
        );
        collection.insert(Constant::new("src_dir"), src_path.display().to_string());
        Self { list: collection }
    }

    #[allow(dead_code)]
    pub fn get_constant(&self, data: &String) -> Option<String> {
        let constant_pattern = Regex::new(r"\$\{(.*)\}").unwrap();
        if let Some(captured) = constant_pattern.captures(data) {
            let capture_s = captured.get(1).unwrap().as_str();
            if self.is_constant(capture_s) {
                return Some(capture_s.to_string());
            }
            return None;
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_item(&self, key: Constant) -> Option<String> {
        if let Some(item) = self.list.get(&key) {
            return Some(item.clone());
        }
        None
    }

    #[allow(dead_code)]
    fn is_constant(&self, data: &str) -> bool {
        if data == "project_top" || data == "src_dir" {
            return true;
        }
        false
    }
}

#[cfg(test)]
#[path = "./constants_test.rs"]
mod mmk_contants_test;
