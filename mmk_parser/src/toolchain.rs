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


    fn parse_line(&mut self, captured: Captures) {
        let tool = captured.get(1).unwrap().as_str();
        let tool_constant = Constant::new(tool);
        let tool_path_str = captured.get(2).unwrap().as_str();
        let tool_path = PathBuf::from(tool_path_str);
        self.config.insert(tool_constant, tool_path);
    }


    pub fn parse(&mut self, path: &PathBuf) -> Result<(), MyMakeError> {
        let content = utility::read_file(path)?;
        let assign_rule = Regex::new(r"([a-zA-Z]+)\s*=\s*([a-zA-Z]+)").unwrap();
        let mut lines = content.lines();
        let current_line = lines.next();
        while current_line != None {
            if let Some(captured) = assign_rule.captures(current_line.unwrap()) {
                self.parse_line(captured);
            }
        }
        Ok(())
    }
}