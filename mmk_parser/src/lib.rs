//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

pub mod mmk_file_reader
{
    use std::collections::HashMap;
    use std::vec::Vec;
    #[derive(Debug)]
    pub struct Mmk
    {
        pub data: HashMap<String, Vec<String>>,
        // pub left_side: String,
        // pub right_side: Vec<String>,
    }

    impl Mmk
    {
        pub fn new() -> Mmk
        {
            Mmk { data: HashMap::new() }
        }

        pub fn to_string(&self, key: &str) -> String
        {
            let mut formatted_string = String::new();
            for item in &self.data[key]
            {                
                formatted_string.push_str(&item[..].trim());
                formatted_string.push_str(" ");
            }
            formatted_string.trim_end().to_string()
        }
    }

    // Implementere Display for Mmk her!

    use std::fs;
    use std::io;
    use std::path::Path;

    pub fn read_file(file_path: &Path) -> Result<String, io::Error>
    {
        fs::read_to_string(&file_path)
    }

     fn clip_string(data: &String, keyword:&str) -> String
     {
        let keyword_index: usize = data.find(&keyword).expect("Word not found!");

        data[keyword_index..].to_string()
     }

    pub fn parse_mmk<'a>(mmk_container: &'a mut Mmk, data: &String, keyword: &str) -> &'a mut Mmk
    {
        let filtered_data: String = clip_string(&data, &keyword).replace(" ", "")
                                                .to_string();

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
        mmk_container
    }
}

#[cfg(test)]
mod tests
{
    use textwrap::dedent;
    use super::mmk_file_reader;
    #[test]
    fn test_mmk_file_reader()
    {
        let path = std::path::Path::new("/home/fredrik/bin/mymake/mmk_parser/src/test.mmk");
        let content = super::mmk_file_reader::read_file(&path);        
        assert_eq!(content.unwrap(), dedent(
        "MMK_SOURCES = filename.cpp \\
              otherfilename.cpp\n"));
    }
    #[test]
    fn test_parse_mmk_sources()
    {
        let mut mmk_content = mmk_file_reader::Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n");

        mmk_file_reader::parse_mmk( &mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_source()
    {
        let mut mmk_content = mmk_file_reader::Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\");
        mmk_file_reader::parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_dependencies()
    {
        let mut mmk_content = mmk_file_reader::Mmk::new();
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        mmk_file_reader::parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
    }

    #[test]
    fn test_multiple_keywords()
    {
        let mut mmk_content = mmk_file_reader::Mmk::new();
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n
                                            
                                            MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/\n");

        mmk_file_reader::parse_mmk(&mut mmk_content, &content, "MMK_SOURCES");
        assert_eq!(mmk_content.data["MMK_SOURCES"], ["filename.cpp", "otherfilename.cpp"]);
        mmk_file_reader::parse_mmk(&mut mmk_content, &content, "MMK_DEPEND");
        assert_eq!(mmk_content.data["MMK_DEPEND"], ["/some/path/to/depend/on", "/another/path/"]);
    }
}

