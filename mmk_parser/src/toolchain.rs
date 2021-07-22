use error::MyMakeError;
use std::collections::HashMap;
use std::path::PathBuf;
use utility;
use regex::{Captures, Regex};

use crate::mmk_constants::Constant;

pub struct Toolchain {
    config: HashMap<Constant, PathBuf>
}

impl Toolchain {
    pub fn new() -> Self {
        Self { config: HashMap::new() }
    }


    fn parse_line(&mut self, captured: Captures) -> Result<(), MyMakeError> {
        let tool = captured.get(1).unwrap().as_str();
        self.verify_keyword(tool)?;
        let tool_constant = Constant::new(tool);
        let tool_path_str = captured.get(2).unwrap().as_str();
        let tool_path = PathBuf::from(tool_path_str);
        self.config.insert(tool_constant, tool_path);
        Ok(())
    }


    pub fn parse(&mut self, content: String) -> Result<(), MyMakeError> {
        let assign_rule = Regex::new(r"(\w+)\s*=\s*([_/a-zA-Z]+)").unwrap();
        let mut lines = content.lines();
        let mut current_line = lines.next();
        while current_line != None {
            if let Some(captured) = assign_rule.captures(current_line.unwrap()) {
                self.parse_line(captured)?;
            }
            current_line = lines.next();
        }
        Ok(())
    }


    pub fn get_content(&self, path: &PathBuf) -> Result<String, MyMakeError> {
        utility::read_file(path)
    }


    fn verify_keyword(&self, keyword: &str) -> Result<(), MyMakeError> {
        match keyword {
            "compiler" | "linker" => Ok(()),
            _ => Err(MyMakeError::from(format!("Error: {} is not allowed as keyword for toolchain.", keyword))),
        }
    }
}


#[cfg(test)]
#[path = "./toolchain_test.rs"]
mod toolchain_test;