//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

use std::collections::HashMap;
use std::vec::Vec;
use std::fs;
use std::io;
use std::path::Path;
use error::MyMakeError;
use regex::Regex;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Mmk
{
    pub data: HashMap<String, Vec<String>>,
}

impl Mmk
{
    pub fn new() -> Mmk
    {
        Mmk { data: HashMap::new() }
    }

    pub fn parse_file(self: &mut Self, data: &String) -> Result<&mut Mmk, MyMakeError>
    {
        let no_comment_data = remove_comments(data);
        parse_mmk(self, &no_comment_data, "MMK_SOURCES")?;
        parse_mmk(self, &no_comment_data, "MMK_HEADERS")?;
        parse_mmk(self, &no_comment_data, "MMK_EXECUTABLE")?;
        parse_mmk(self, &no_comment_data, "MMK_DEPEND")
    }

    pub fn to_string(self: &Self, key: &str) -> String
    {
        let mut formatted_string = String::new();
        if self.data.contains_key(key)
        {
            for item in &self.data[key]
            {
                if *item == String::from("")
                {
                    break;
                }
                if key == "MMK_DEPEND"
                {
                    formatted_string.push_str("-I");
                }
                formatted_string.push_str(item);
                formatted_string.push_str(" ");
            }
        }
        formatted_string.trim_end().to_string()
    }

    pub fn valid_keyword(self: &Self, keyword: & str) -> Result<(), MyMakeError>
    {
        if keyword == "MMK_DEPEND"
        || keyword == "MMK_SOURCES" 
        || keyword == "MMK_HEADERS"
        || keyword == "MMK_EXECUTABLE" {
            Ok(())
        }
        else {
            Err(MyMakeError::from(format!("Invalid keyword {}", keyword)))
        }
    }

    pub fn sources_to_objects(self: &Self) -> String {
        let sources = &self.to_string("MMK_SOURCES");
        let objects = sources.replace(".cpp", ".o");
        objects
    }


    fn parse_line(&mut self, line: &str) -> Result<(), MyMakeError> {
        if line != "" {
            let mmk_rule = Regex::new(r"(MMK_\w+)\s*=\s*(.*)$").unwrap();
            if let Some(captured) = mmk_rule.captures(line) {
                let mmk_keyword = captured.get(1).unwrap().as_str();
                let args = captured.get(2).unwrap().as_str();
                let args_vec: Vec<String> = args.split_whitespace().map(|s| s.to_string()).collect();
                self.valid_keyword(mmk_keyword)?;
                self.data.insert(String::from(mmk_keyword), args_vec);    
            }
        }
        Ok(())
    }


    pub fn parse(&mut self, data: &String) -> Result<(), MyMakeError> {
        let no_comment_data = remove_comments(data);
        let mut lines = no_comment_data.lines();
        let mut current_line = lines.next();
        while current_line != None {
            self.parse_line(current_line.unwrap())?;
            current_line = lines.next();
        }
        Ok(())
    }
}

pub fn validate_file_path(file_path_as_str: &str) -> Result<std::path::PathBuf, MyMakeError> {
    let file_path = std::path::PathBuf::from(file_path_as_str);
    if !file_path.is_file() {
        return Err(MyMakeError::from(format!("Error: {:?} is not a valid path!", &file_path)));
    }
    Ok(file_path)
}

pub fn read_file(file_path: &Path) -> Result<String, io::Error>
{
    fs::read_to_string(&file_path)
}

fn clip_string(data: &String, keyword:&str) -> String
{
    let keyword_index: usize = match data.find(&keyword)
    {
        Some(match_index) => match_index,
        None => return String::from(""),
    };
    data[keyword_index..].to_string()
}

pub fn remove_comments(data: &String) -> String {
    let mut lines = data.lines();
    let mut current_line = lines.next();
    let comment_expression = Regex::new(r"#.*").unwrap();
    let mut non_comment_data: String = data.clone();
    
    while current_line != None {
        non_comment_data = comment_expression.replace(&non_comment_data, "").to_string();
        current_line = lines.next();
    }
    non_comment_data
}

pub fn parse_mmk<'a>(mmk_container: &'a mut Mmk, data: &String, keyword: &str) -> Result<&'a mut Mmk, MyMakeError>
{
    mmk_container.valid_keyword(keyword)?;
    {
        let filtered_data: String = clip_string(&data, &keyword).replace(" ", "")
                                                .to_string();

        if filtered_data == "" {
            mmk_container.data.insert(keyword.to_string(), vec![filtered_data]);
            return Ok(mmk_container);
        }
        let split_data: Vec<&str> = filtered_data.trim_start()
                                                    .split_terminator("=")
                                                    .collect();

        let mut mmk_right_side: Vec<String> = split_data[1].split_terminator("\\").map(|s| {
                s.trim_end_matches("MMK_DEPEND")
                .trim_end_matches("MMK_SOURCES")
                .trim_end_matches("MMK_HEADERS")
                .trim_end_matches("MMK_EXECUTABLE")
                .trim_matches(&['\n', '\r'][..])
                .to_string()
            }
        ).collect();
        mmk_right_side.retain(|x| x.is_empty() == false);
        mmk_container.data.insert(keyword.to_string(), mmk_right_side);
    }
    Ok(mmk_container)
}


#[cfg(test)]
pub mod tests
{
    use super::*;
    use pretty_assertions::assert_eq;
    #[test]
    fn test_mmk_file_reader()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path);        
        assert_eq!(content.unwrap(),("\
            #This is a comment.\n\
            MMK_DEPEND = /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/\n\
            \n\
            MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n\
            \n\
            #This is a second comment.\n\
            MMK_EXECUTABLE = x\n\
            "));
    }

    #[test]
    fn test_remove_comments()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path).unwrap();     
        assert_eq!(remove_comments(&content),"\
            \n\
            MMK_DEPEND = /home/fredrik/Documents/Tests/AStarPathFinder/PlanGenerator/test/\n\
            \n\
            MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n\
            \n\
            \n\
            MMK_EXECUTABLE = x\n\
            ");
    }
    
    #[test]
    fn test_parse_mmk_sources() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \
                                                          otherfilename.cpp\n");

        mmk_content.parse( &content)?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        Ok(())
    }

    #[test]
    fn test_parse_mmk_source() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\");
        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES")?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
        Ok(())
    }


    #[test]
    fn test_parse_mmk_source_newline_after_end() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
        ");
        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES")?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
        Ok(())
    }

    #[test]
    fn test_parse_mmk_dependencies() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND")?;
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
        Ok(())
    }

    #[test]
    fn test_multiple_keywords() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n
                                            
                                            MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/\n
                                                         
                                            MMK_EXECUTABLE = main");

        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES")?;
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND")?;
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/"]);
        parse_mmk(&mut mmk_content, &content, "MMK_EXECUTABLE")?;
        assert_eq!(mmk_content.data["MMK_EXECUTABLE"], ["main"]);
        Ok(())
    }
    #[test]
    fn test_parse_mmk_no_valid_keyword() -> Result<(), MyMakeError>
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        let result = parse_mmk(&mut mmk_content, &content, "MMK_DEP");
        assert!(result.is_err());
        Ok(())
    }
}

