use crate::mmk_constants::Constant;

use error::{FsError, ToolchainError};
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

    pub fn parse(&mut self, content: String) -> Result<(), ToolchainError> {
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

    pub fn get_content(&self, path: &std::path::Path) -> Result<String, ToolchainError> {
        let filename = path
            .file_name()
            .ok_or_else(|| FsError::PopError)
            .map_err(ToolchainError::FileSystem)?;
        self.validate_filename(filename)?;
        utility::read_file(path).map_err(ToolchainError::FileSystem)
    }

    pub fn get_item(&self, toolchain_key: &Constant) -> Result<&PathBuf, ToolchainError> {
        self.config
            .get(&toolchain_key)
            .ok_or(ToolchainError::KeyNotFound(toolchain_key.to_string()))
    }

    fn parse_line(&mut self, captured: Captures) -> Result<(), ToolchainError> {
        let tool = captured.get(1).unwrap().as_str();
        self.verify_keyword(tool)?;
        let tool_constant = Constant::new(tool);
        let tool_path_str = captured.get(2).unwrap().as_str();
        let tool_path = PathBuf::from(tool_path_str);
        self.config.insert(tool_constant, tool_path);
        Ok(())
    }

    fn validate_filename(&self, filename: &OsStr) -> Result<(), ToolchainError> {
        match filename.to_str() {
            Some("toolchain.mmk") => Ok(()),
            _ => {
                return Err(ToolchainError::InvalidName(
                    filename.to_str().unwrap().to_string(),
                ))
            }
        }
    }

    fn verify_keyword(&self, keyword: &str) -> Result<(), ToolchainError> {
        match keyword {
            "compiler" | "linker" => Ok(()),
            _ => Err(ToolchainError::InvalidKeyword(keyword.to_string())),
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
