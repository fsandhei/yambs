//!
//#![warn(missing_debug_implementations, rust_2018_idioms_, missing_docs)]

pub mod mmk_file_reader
{
    pub struct Mmk
    {
        pub left_side: String,
        pub right_side: Vec<String>,
    }

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

    pub fn parse_mmk(data: &String, keyword: &str) -> Mmk
    {
        let filtered_data: String = clip_string(&data, &keyword).replace(" ", "")
                                                // .replace("\n", "")
                                                .to_string();

        let split_data: Vec<&str> = filtered_data.trim_start()
                                                 .split_terminator("=")
                                                 .collect();

        let mmk_right_side: Vec<String> = split_data[1].split_terminator("\\").map(|s| 
            {
                let new_str = s.trim_matches(&['\n', '\r'][..]);

                new_str.to_string()
            }
        ).collect();

        let mmk: Mmk = Mmk
        {
            left_side: split_data[0].to_string(),
            right_side: mmk_right_side,
        };
        mmk
    }
}

#[cfg(test)]
mod tests
{
    use textwrap::dedent;
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
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n");
        let mmk_data = super::mmk_file_reader::parse_mmk(&content, "MMK_SOURCES");
        assert_eq!(mmk_data.left_side, "MMK_SOURCES");
        assert_eq!(mmk_data.right_side, ["filename.cpp", "otherfilename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_source()
    {
        let content: String = String::from("MMK_SOURCES = filename.cpp \\");

        let mmk_data = super::mmk_file_reader::parse_mmk(&content, "MMK_SOURCES");
        assert_eq!(mmk_data.left_side, "MMK_SOURCES");
        assert_eq!(mmk_data.right_side, ["filename.cpp"]);
    }

    #[test]
    fn test_parse_mmk_dependencies()
    {
        let content: String = String::from("MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/to/depend/on\n");
        let mmk_data = super::mmk_file_reader::parse_mmk(&content, "MMK_DEPEND");
        assert_eq!(mmk_data.left_side, "MMK_DEPEND");
        assert_eq!(mmk_data.right_side, ["/some/path/to/depend/on", "/another/path/to/depend/on"]);
    }

    #[test]
    fn test_multiple_keywords()
    {
        let content: String = String::from("MMK_SOURCES = filename.cpp \\
                                                          otherfilename.cpp\n
                                            
                                            MMK_DEPEND = /some/path/to/depend/on \\
                                                         /another/path/\n");

        let mmk_data = super::mmk_file_reader::parse_mmk(&content, "MMK_SOURCES");
        assert_eq!(mmk_data.left_side, "MMK_SOURCES");
        assert_eq!(mmk_data.right_side, ["filename.cpp", "otherfilename.cpp"]);
    }
}

