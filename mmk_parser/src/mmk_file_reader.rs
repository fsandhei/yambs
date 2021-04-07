use std::fs::File;
use std::path::Path;


mod mmk_file_reader
{
    fn read_file(file_path: &Path) -> String
    {
        let display = file_path.display();
        let mut file = match File::open(&path)
        {
            Err(why) => panic!("Could not open {}: {}", display, why),
            ok(file) => file,
        };
        let mut content = String::new();
        match file.read_to_string(&mut content)
        {
            Err(why) => panic!("Could not read {}: {}", display, why),
            ok(_) => content,
        };
    }
}