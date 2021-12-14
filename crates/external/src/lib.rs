use dependency::DependencyNode;
use std::io::Write;
use std::fs::File;
use std::fs::OpenOptions;
use std::env;

pub fn dottie(top: &DependencyNode, recursive: bool, data: &mut String) -> std::io::Result<()>{
    let mut dottie_file = create_dottie_file(recursive)?;
    let borrowed_top = top.borrow();
    let top_pretty_name = &borrowed_top.get_pretty_name();
    
    if recursive == false
    {
        data.push_str("\
        digraph G {\n\
        ");
        dottie(top, true, data)?;
        data.push_str("}");
        dottie_file.write_all(data.as_bytes())?;        
    }
    
    for requirement in borrowed_top.requires().borrow().iter()
    {
        data.push_str(&format!("\
        {:?} -> {:?}\n\
        ", requirement.borrow().get_pretty_name()
            , top_pretty_name));
        dottie(&requirement, true, data)?;
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
}

fn dottie_file_exists() -> bool
{
    let current_dir = env::current_dir().expect("rsmake: Current path does not exist!");
    let dot_file_path = current_dir.join("dependency.gv");
    dot_file_path.exists()
}

