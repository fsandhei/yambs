use builder::Dependency;
use std::io::Write;
use std::fs::File;
use std::fs::OpenOptions;
use std::env;

pub fn dottie(top: &Dependency, recursive: bool) -> std::io::Result<()>{
        
    let mut dottie_file = create_dottie_file(recursive)?;
    let top_path = &top.path;
    
    if recursive == false
    {
        dottie_file.write(b"\
        digraph G{\n\
        ")?;
    }
    
    for requirement in &top.requires
    {
        let data = format!("\
        {:?} -> {:?}\n\
        ", requirement.path
            , top_path);

        dottie_file.write(data.as_bytes())?;
        dottie(requirement, true)?;
    }

    if recursive == false
    {
        dottie_file.write(b"}")?;
    }    
    
    Ok(())
}

fn create_dottie_file(first_run: bool) -> std::io::Result<File>
{
    let current_dir = env::current_dir()?;
    let dot_file_path = current_dir.join("dependency.gv");

    if dottie_file_exists()
    {
        if first_run == false
        {
            File::create(dot_file_path)
        }
        else
        {
            OpenOptions::new()
            .write(true)
            .append(true)
            .open(dot_file_path)
        }
    }
    else
    {
        File::create(dot_file_path)
    }

    // File::create(dot_file_path)
}

fn dottie_file_exists() -> bool
{
    let current_dir = env::current_dir().expect("MyMake: Current path does not exist!");
    let dot_file_path = current_dir.join("dependency.gv");
    dot_file_path.exists()
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
