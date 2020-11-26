extern crate mmk_parser;
extern crate generator;
mod unwrap_or_terminate;

use clap::{Arg, App};
use unwrap_or_terminate::MyMakeUnwrap;

/*
TODO: 
    *Implementere unwrap_or_terminate() for Option / Result
    *Generator::new tar inn path i stedet for filnavn. Automatisk skal output bli en /makefile.
*/
struct Builder
{
    mmk_data: std::vec::Vec<mmk_parser::Mmk>,
}

impl Builder
{
    fn new() -> Builder
    {
        Builder 
        {
            mmk_data: Vec::new(),
        }    
    }

    fn read_mmk_files(self: &mut Self, top_path: &std::path::Path)
    {
         let file_content =  mmk_parser::read_file(top_path).unwrap_or_terminate();
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
            self.read_mmk_files(dep_path);
        }
        self.mmk_data.push(top);
    }
}


fn main() -> Result<(), std::io::Error> {
    let matches = App::new("MyMake")
        .version("0.1.0")
        .author("Fredrik Sandhei <fredrik.sandhei@gmail.com>")
        .about("GNU Make overlay for C / C++ projects.")
        .arg(Arg::with_name("mmk_file")
                    .short("g")
                    .long("generator")
                    .takes_value(true)
                    .help("Input file for MMK"))
        .get_matches();
        
    let myfile = std::path::Path::new(matches.value_of("mmk_file").unwrap_or_terminate());
    // let root = std::path::Path::new(myfile.parent().unwrap());
    // let file_content =  mmk_parser::read_file(myfile).unwrap_or_terminate();
    // let top = mmk_parser::Mmk::new();

     let mut builder = Builder::new();

    print!("MyMake: Reading mmk files");
    builder.read_mmk_files(myfile);
    println!();
    println!("{:?}", builder.mmk_data);
    
    // let mut generator: generator::MmkGenerator = generator::Generator::new(root, parsed_mmk);
    // generator::Generator::generate_makefile(&mut generator)?;
    Ok(())
}
