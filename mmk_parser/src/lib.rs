//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

//TODO: Burde ha muligheten til å kunne bruke path som bruker relativ-path-direktiver (../)

use error::{FsError, MyMakeError, ParseError, ToolchainError};
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::vec::Vec;
use utility;

mod keyword;
pub use keyword::Keyword;

mod mmk_constants;
pub use mmk_constants::{Constant, Constants};

mod toolchain;
pub use toolchain::Toolchain;

pub fn read_toolchain(path: &PathBuf) -> Result<Toolchain, ToolchainError> {
    let mut toolchain = Toolchain::new();
    let content = toolchain.get_content(path)?;
    toolchain.parse(content)?;
    Ok(toolchain)
}

pub fn find_toolchain_file(path: &Path) -> Result<PathBuf, MyMakeError> {
    let toolchain_filename = "toolchain.mmk";
    let mymake_includes = path.join("mymake");

    if path.join(toolchain_filename).is_file() {
        return Ok(path.join(toolchain_filename));
    } else if mymake_includes.is_dir() {
        let toolchain_file = mymake_includes.join(toolchain_filename);
        if toolchain_file.is_file() {
            return Ok(toolchain_file);
        } else {
            return Err(MyMakeError::ConfigurationTime(format!(
                "Could not find mymake directory and toolchain file from {:?}",
                path.to_str().unwrap()
            )));
        }
    }
    return find_toolchain_file(path.parent().unwrap());
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mmk {
    data: HashMap<String, Vec<Keyword>>,
    constants: Constants,
    file: std::path::PathBuf,
}

impl Mmk {
    pub fn new(path: &PathBuf) -> Mmk {
        let source_path =
            utility::get_source_directory_from_path(utility::get_project_top_directory(path));

        Mmk {
            data: HashMap::new(),
            constants: Constants::new(path, &source_path),
            file: source_path,
        }
    }

    pub fn data(&self) -> &HashMap<String, Vec<Keyword>> {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut HashMap<String, Vec<Keyword>> {
        &mut self.data
    }

    pub fn has_executables(&self) -> bool {
        self.data.contains_key("MMK_EXECUTABLE")
    }

    pub fn has_dependencies(&self) -> bool {
        self.data.contains_key("MMK_REQUIRE")
    }

    pub fn to_string(self: &Self, key: &str) -> String {
        let mut formatted_string = String::new();
        if self.data.contains_key(key) {
            for item in &self.data[key] {
                if item.argument() == "" {
                    break;
                }

                if key == "MMK_SYS_INCLUDE" {
                    formatted_string.push_str("-isystem ");
                }
                formatted_string.push_str(&item.argument());
                formatted_string.push_str(" ");
            }
        }
        formatted_string.trim_end().to_string()
    }

    pub fn get_include_directories(&self) -> Result<String, MyMakeError> {
        if self.data.contains_key("MMK_REQUIRE") {
            let mut formatted_string = String::new();
            for keyword in &self.data["MMK_REQUIRE"] {
                if keyword.option() == "SYSTEM" {
                    formatted_string.push_str("-isystem");
                    formatted_string.push_str(" ");
                } else {
                    formatted_string.push_str("-I");
                }
                let dep_path =
                    utility::get_include_directory_from_path(&PathBuf::from(keyword.argument()))?;
                formatted_string.push_str(dep_path.to_str().unwrap());
                formatted_string.push_str(" ");
            }
            return Ok(formatted_string.trim_end().to_string());
        }
        Ok(String::from(""))
    }

    pub fn valid_keyword(&self, keyword: &str) -> Result<(), ParseError> {
        let stripped_keyword = keyword.trim_end_matches(":");
        if stripped_keyword == "MMK_REQUIRE"
            || stripped_keyword == "MMK_SOURCES"
            || stripped_keyword == "MMK_HEADERS"
            || stripped_keyword == "MMK_EXECUTABLE"
            || stripped_keyword == "MMK_SYS_INCLUDE"
            || stripped_keyword == "MMK_CXXFLAGS_APPEND"
            || stripped_keyword == "MMK_CPPFLAGS_APPEND"
            || stripped_keyword == "MMK_LIBRARY_LABEL"
        {
            Ok(())
        } else {
            Err(ParseError::InvalidKeyword {
                file: self.file.to_path_buf(),
                keyword: stripped_keyword.to_string(),
            })
        }
    }

    pub fn sources_to_objects(self: &Self) -> String {
        let sources = &self.to_string("MMK_SOURCES");
        let objects = sources.replace(".cpp", ".o");
        objects
    }

    fn parse_mmk_expression(
        &mut self,
        mmk_keyword: &str,
        data_iter: &mut std::str::Lines,
    ) -> Result<(), ParseError> {
        self.valid_keyword(mmk_keyword)?;
        let mut arg_vec: Vec<Keyword> = Vec::new();
        let mut current_line = data_iter.next();
        while current_line != None {
            let line = current_line.unwrap().trim();
            if line != "" && !self.valid_keyword(&line).is_ok() {
                let keyword = self.parse_and_create_keyword(line);

                arg_vec.push(keyword);
            } else if line == "" {
                break;
            } else {
                return Err(ParseError::InvalidSpacing {
                    file: self.file.to_path_buf(),
                });
            }
            current_line = data_iter.next();
        }
        self.data.insert(String::from(mmk_keyword), arg_vec);
        Ok(())
    }

    fn parse_and_create_keyword(&self, line: &str) -> Keyword {
        let line_split: Vec<&str> = line.split(" ").collect();
        let keyword: Keyword;
        if line_split.len() == 1 {
            let arg = line_split[0];
            keyword = Keyword::from(&self.replace_constant_with_value(&arg.to_string()))
        } else {
            let option = line_split[1];
            let arg = line_split[0];
            keyword = Keyword::from(&self.replace_constant_with_value(&arg.to_string()))
                .with_option(option);
        }
        keyword
    }

    pub fn has_library_label(&self) -> bool {
        self.data.contains_key("MMK_LIBRARY_LABEL")
    }

    pub fn has_system_include(&self) -> bool {
        self.data.contains_key("MMK_SYS_INCLUDE")
    }

    pub fn parse(&mut self, data: &String) -> Result<(), ParseError> {
        let no_comment_data = remove_comments(data);
        let mut lines = no_comment_data.lines();
        let mut current_line = lines.next();
        let mmk_rule = Regex::new(r"(MMK_\w+):[\r\n]*").unwrap();
        while current_line != None {
            if let Some(captured) = mmk_rule.captures(current_line.unwrap()) {
                let mmk_keyword = captured.get(1).unwrap().as_str();
                self.parse_mmk_expression(mmk_keyword, &mut lines)?;
                current_line = lines.next();
            } else {
                current_line = lines.next();
            }
        }
        Ok(())
    }

    fn replace_constant_with_value(&self, mmk_keyword_value: &str) -> String {
        if let Some(constant_string) = self.constants.get_constant(&mmk_keyword_value.to_string()) {
            let item = self
                .constants
                .get_item(Constant::new(&constant_string))
                .unwrap();
            let constant_reconstructed = format!("${{{}}}", constant_string);
            return mmk_keyword_value.replace(&constant_reconstructed, &item);
        } else {
            return mmk_keyword_value.to_string();
        }
    }

    pub fn source_file_path(&self, source: &String) -> Option<PathBuf> {
        let mut source_path = PathBuf::from(source);
        if source_path.pop() {
            return Some(source_path);
        }
        None
    }
}

pub fn validate_file_path(file_path_as_str: &str) -> Result<PathBuf, FsError> {
    let file_path = PathBuf::from(file_path_as_str)
        .canonicalize()
        .map_err(FsError::Canonicalize)?;

    if !file_path.is_file() {
        return Err(FsError::FileDoesNotExist(file_path));
    }
    Ok(file_path)
}

pub fn validate_file_name(path: &PathBuf) -> Result<(), ParseError> {
    let file_name = path.file_name().unwrap().to_str().unwrap();
    match file_name {
        "lib.mmk" | "run.mmk" => (),
        _ => {
            return Err(ParseError::InvalidFilename(file_name.to_string()));
            // return Err(MyMakeError::from(format!(
            //     "{:?} is illegal name! File must be named lib.mmk or run.mmk.",
            //     file_name
            // )))
        }
    };
    Ok(())
}

pub fn remove_comments(data: &String) -> String {
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

#[cfg(test)]
#[path = "./lib_test.rs"]
mod lib_test;
