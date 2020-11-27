
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
        let test_file = dir.path().join("makefile");
        let mut file = File::create(&test_file)?;
        write!(file, "")?;
        builder.read_mmk_files(&test_file).unwrap();
        // assert_eq!(builder.mmk_data[0]);
        Ok(())
    }
}
