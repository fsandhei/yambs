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

struct Builder<'a>
{
    mmk_data: std::vec::Vec<&'a mmk_parser::Mmk>,
    num_mmk_data: u32,
    file_data: String,
}

impl<'a> Builder<'a>
{
    fn new(top: &'a mmk_parser::Mmk, file_data_: String) -> Builder<'a>
    {
        Builder 
        {
            mmk_data: vec![top],
            num_mmk_data: 0,
            file_data: file_data_, 
        }    
    }

    fn read_mmk_files(self: &mut Self, mmk: &'a mmk_parser::Mmk) -> &'a mut std::vec::Vec<&mmk_parser::Mmk>
    {
        self.num_mmk_data += 1;
        print!("MyMake: Reading mmk file: {}\r", self.num_mmk_data);
        self.mmk_data.push(mmk);        
        
        &mut self.mmk_data
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

    let mut parsed_mmk = mmk_parser::Mmk::new();
    //parsed_mmk.parse_file(&file_content);

    let mut builder = Builder::new(&parsed_mmk, file_content);
    builder.read_mmk_files(&builder.mmk_data[0]);
    
    // let mut generator: generator::MmkGenerator = generator::Generator::new(root, parsed_mmk);
    // generator::Generator::generate_makefile(&mut generator)?;
    Ok(())
}
