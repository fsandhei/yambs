extern crate mmk_parser;
extern crate generator;
mod unwrap_or_terminate;

use clap::{Arg, App};
use unwrap_or_terminate::MyMakeUnwrap;
use std::io;
use std::io::Write;

/*
TODO: 
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en /makefile.
*/
struct Builder
{
    mmk_data: std::vec::Vec<mmk_parser::Mmk>,
    num_mmk_data: u32,
    file_data: String,
}

impl Builder
{
    fn from(top: mmk_parser::Mmk, file_content: String) -> Builder
    {
        Builder 
        {
            mmk_data: vec![top],
            num_mmk_data: 1,
            file_data: file_content, 
        }    
    }

    fn read_mmk_files(self: &mut Self)
    {
        let top_mut = self.mmk_data.last_mut().unwrap();
        print!("MyMake: Reading mmk file: {}\r", self.num_mmk_data);
        top_mut.parse_file(&self.file_data);

        for path in top_mut.data["MMK_DEPEND"].clone()
        {
            if path == ""
            {
                break;
            }
            self.num_mmk_data += 1;
            let mut mmk_path = path.clone();
            mmk_path.push_str("/mymakeinfo.mmk");
            let dep_path = std::path::Path::new(&mmk_path);
            let mut dep_mmk = mmk_parser::Mmk::new();

            self.file_data = mmk_parser::read_file(dep_path).unwrap_or_terminate();            
            dep_mmk.parse_file(&self.file_data);
            self.mmk_data.push(dep_mmk);
            self.read_mmk_files();
        }
        println!();
    }
}


fn main() -> Result<(), std::io::Error> {
    let matches = App::new("MyMake")
        .version("0.1.0")
        .author("Fredrik Sandhei <fredrik.sandhei@gmail.com>")
        .about("GNU Make overlay for C / C++ projects.")
        .arg(Arg::with_name("mmk_file")
                    .short("f")
                    .long("file")
                    .takes_value(true)
                    .help("Input file for MMK"))
        .get_matches();
        
    let myfile = std::path::Path::new(matches.value_of("mmk_file").unwrap_or_terminate());
    // let root = std::path::Path::new(myfile.parent().unwrap());
    let file_content =  mmk_parser::read_file(myfile).unwrap_or_terminate();
    let top = mmk_parser::Mmk::new();

    let mut builder = Builder::from(top, file_content);
    builder.read_mmk_files();
    println!("{:?}", builder.mmk_data);
    
    // let mut generator: generator::MmkGenerator = generator::Generator::new(root, parsed_mmk);
    // generator::Generator::generate_makefile(&mut generator)?;
    Ok(())
}
