
use mmk_parser;
use std::io;

pub struct Builder
{
    pub mmk_data: std::vec::Vec<mmk_parser::Mmk>,
}

impl Builder
{
    pub fn new() -> Builder
    {
        Builder 
        {
            mmk_data: Vec::new(),
        }    
    }

    pub fn read_mmk_files(self: &mut Self, top_path: &std::path::Path) -> io::Result<()>
    {
         let file_content =  mmk_parser::read_file(top_path)?;
         let mut top = mmk_parser::Mmk::new();
         top.parse_file(&file_content);

        for path in top.data["MMK_DEPEND"].clone()
        {
            if path == ""
            {
                break;
            }
            let mut mmk_path = path.clone();            
            mmk_path.push_str("/mymakeinfo.mmk");            
            let dep_path = std::path::Path::new(&mmk_path);
            self.read_mmk_files(dep_path)?;
        }
        self.mmk_data.push(top);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mmk_parser::Mmk;
    use tempdir::TempDir;
    use std::fs::File;
    use std::io::Write;
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn read_mmk_files_one_file() -> std::io::Result<()>
    {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");
        let mut file = File::create(&test_file)?;
        write!(file, "\
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x")?;
        builder.read_mmk_files(&test_file).unwrap();
        let mut expected = Mmk::new();

        expected.data.insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected.data.insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected.data.insert(String::from("MMK_SOURCES"), vec![String::from("some_file.cpp"), 
                                                                 String::from("some_other_file.cpp")]);
        expected.data.insert(String::from("MMK_HEADERS"), vec![String::from("some_file.h"), 
                                                                 String::from("some_other_file.h")]);
        assert_eq!(builder.mmk_data[0], expected);
        Ok(())
    }

    #[test]
    fn read_mmk_files_two_files() -> std::io::Result<()>
    {
        let mut builder = Builder::new();
        let dir = TempDir::new("example")?;
        let test_file = dir.path().join("mymakeinfo.mmk");

        let dir_dep = TempDir::new("example_dep")?;
        let test_file_dep = dir_dep.path().join("mymakeinfo.mmk");

        let mut file = File::create(&test_file)?;
        let mut file_dep = File::create(&test_file_dep)?;

        write!(file, "\
        MMK_DEPEND = {} \\
        \n
        MMK_SOURCES = some_file.cpp \\
                      some_other_file.cpp \\
        \n
        MMK_HEADERS = some_file.h \\
                      some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x", &dir_dep.path().to_str().unwrap().to_string())?;

        write!(file_dep, "\
        MMK_SOURCES = /some/some_file.cpp \\
                      /some/other_file.cpp \\
        \n
        MMK_HEADERS = /some/some_file.h \\
                      /some/some_other_file.h \\
        
        \n
        
        MMK_EXECUTABLE = x")?;

        builder.read_mmk_files(&test_file).unwrap();
        let mut expected_1 = Mmk::new();
        let mut expected_2 = Mmk::new();

        expected_1.data.insert(String::from("MMK_DEPEND"), vec![dir_dep.path().to_str().unwrap().to_string()]);
        expected_1.data.insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);

        expected_1.data.insert(String::from("MMK_SOURCES"), vec![String::from("some_file.cpp"), 
                                                                 String::from("some_other_file.cpp")]);
        expected_1.data.insert(String::from("MMK_HEADERS"), vec![String::from("some_file.h"), 
                                                                 String::from("some_other_file.h")]);

        expected_2.data.insert(String::from("MMK_DEPEND"), vec![String::new()]);
        expected_2.data.insert(String::from("MMK_EXECUTABLE"), vec![String::from("x")]);
        expected_2.data.insert(String::from("MMK_SOURCES"), vec![String::from("/some/some_file.cpp"), 
                                                                 String::from("/some/other_file.cpp")]);
        expected_2.data.insert(String::from("MMK_HEADERS"), vec![String::from("/some/some_file.h"), 
                                                                 String::from("/some/some_other_file.h")]);
        assert_eq!(builder.mmk_data[1], expected_1);
        assert_eq!(builder.mmk_data[0], expected_2);
        Ok(())
    }
}
