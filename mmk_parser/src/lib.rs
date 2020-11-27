//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

use std::collections::HashMap;
use std::vec::Vec;
use std::fs;
use std::io;
use std::path::Path;
#[derive(Debug, PartialEq, Eq)]
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

    pub fn parse_file(self: &mut Self, data: &String) -> &mut Mmk
    {
        print!(".");
        parse_mmk(self, &data, "MMK_SOURCES");
        parse_mmk(self, &data, "MMK_HEADERS");
        parse_mmk(self, &data, "MMK_EXECUTABLE");
        parse_mmk(self, &data, "MMK_DEPEND")        
    }

    pub fn to_string(self: &Self, key: &str) -> String
    {
        let mut formatted_string = String::new();
        if self.data.contains_key(key)
        {
            for item in &self.data[key]
            {                
                formatted_string.push_str(&item[..].trim());
                formatted_string.push_str(" ");
            }            
        }
        formatted_string.trim_end().to_string()
    }

    pub fn valid_keyword(self: &Self, keyword: &str) -> bool
    {
        keyword    == "MMK_DEPEND"
        || keyword == "MMK_SOURCES" 
        || keyword == "MMK_HEADERS"
        || keyword == "MMK_EXECUTABLE"
    }
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

pub fn parse_mmk<'a>(mmk_container: &'a mut Mmk, data: &String, keyword: &str) -> &'a mut Mmk
{
    if mmk_container.valid_keyword(keyword)
    {
        let filtered_data: String = clip_string(&data, &keyword).replace(" ", "")
                                                .to_string();

        if filtered_data == ""
        {
            mmk_container.data.insert(keyword.to_string(), vec![filtered_data]);
            return mmk_container;
        }
        let split_data: Vec<&str> = filtered_data.trim_start()
                                                    .split_terminator("=")
                                                    .collect();

        let mmk_right_side: Vec<String> = split_data[1].split_terminator("\\").map(|s| 
            {
                s.trim_end_matches("MMK_DEPEND")
                .trim_end_matches("MMK_SOURCES")
                .trim_end_matches("MMK_HEADERS")
                .trim_end_matches("MMK_EXECUTABLE")
                .trim_matches(&['\n', '\r'][..])
                .to_string()
            }
        ).collect();
        mmk_container.data.insert(split_data[0].to_string(), mmk_right_side);
    }
    mmk_container
}

#[cfg(test)]
mod tests
{
    use textwrap::dedent;
    use super::*;
    #[test]
    fn test_mmk_file_reader()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = read_file(&path);        
        assert_eq!(content.unwrap(), dedent(
        "MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n"));
    }
    #[test]
    fn test_parse_mmk_sources()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n");

        parse_mmk( &mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_source()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\");
        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_dependencies()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
    }

    #[test]
    fn test_multiple_keywords()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n
                                            
                                            MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/\n
                                                         
                                            MMK_EXECUTABLE = main");

        parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/"]);
        parse_mmk(&mut mmk_content, &content, "MMK_EXECUTABLE");
        assert_eq!(mmk_content.data["MMK_EXECUTABLE"], ["main"]);
    }
    #[test]
    fn test_parse_mmk_no_keyword()
    {
        let mut mmk_content = Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        parse_mmk(&mut mmk_content, &content, "MMK_DEP");
        assert!(mmk_content.data.is_empty() == true);
    }
}

