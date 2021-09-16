use crate::mmk_constants::Constant;

use error::MyMakeError;
use regex::{Captures, Regex};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use utility;

#[derive(Clone)]
pub struct Toolchain {
    config: HashMap<Constant, PathBuf>,
}

impl Toolchain {
    pub fn new() -> Self {
        Self {
            config: HashMap::new(),
        }
    }

    pub fn parse(&mut self, content: String) -> Result<(), MyMakeError> {
        let content_without_comments = self.remove_comments(&content);
        let assign_rule = Regex::new(r"(\w+)\s*=\s*([_/a-zA-Z]+)").unwrap();
        let mut lines = content_without_comments.lines();
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
        if let Some(filename) = path.file_name() {
            self.validate_filename(filename)?;
            utility::read_file(path)
        } else {
            return Err(MyMakeError::from(format!(
                "Error: {} is not a valid path to toolchain file.",
                path.to_str().unwrap()
            )));
        }
    }

    pub fn get_item(&self, toolchain_key: &Constant) -> Result<&PathBuf, MyMakeError> {
        self.config
            .get(&toolchain_key)
            .ok_or(MyMakeError::from(format!(
                "Error: {} could not be found",
                toolchain_key
            )))
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

    fn validate_filename(&self, filename: &OsStr) -> Result<(), MyMakeError> {
        match filename.to_str() {
            Some("toolchain.mmk") => Ok(()),
            _ => {
                return Err(MyMakeError::from(format!(
                "Error: {} is not a valid name for toolchain file. It must be named toolchain.mmk",
                filename.to_str().unwrap()
            )))
            }
        }
    }

    fn verify_keyword(&self, keyword: &str) -> Result<(), MyMakeError> {
        match keyword {
            "compiler" | "linker" => Ok(()),
            _ => Err(MyMakeError::from(format!(
                "Error: {} is not allowed as keyword for toolchain.",
                keyword
            ))),
        }
    }

    fn remove_comments(&self, data: &String) -> String {
        let mut lines = data.lines();
        let mut current_line = lines.next();
        let comment_expression = Regex::new(r"#.*").unwrap();
        let mut non_comment_data: String = data.clone();

        while current_line != None {
            non_comment_data = comment_expression
                .replace(&non_comment_data, "")
                .to_string();
            current_line = lines.next();
        }
        non_comment_data
    }

    // Only for testing!
    pub fn set_sample_config(mut self) -> Self {
        self.config
            .insert(Constant::new("compiler"), PathBuf::from("/usr/bin/gcc"));
        self.config
            .insert(Constant::new("linker"), PathBuf::from("/usr/bin/ld"));
        self
    }
}

#[cfg(test)]
#[path = "./toolchain_test.rs"]
mod toolchain_test;
