use dependency::Dependency;
use std::path::PathBuf;
use error::MyMakeError;
use std::fs;

pub fn clean(dependency: &Dependency) -> Result<(), MyMakeError> {
    for required_dependency in dependency.requires.borrow().iter() {
        let dep_build_directory = required_dependency.borrow().get_build_directory();
        match remove_dir(dep_build_directory) {
            Ok(_) => (),
            Err(e) => { 
                                    eprintln!("{}", e);
                                    continue;
                                 }
        };
    }
    let top_build_directory = dependency.get_build_directory();
    remove_dir(top_build_directory)?;
    Ok(())
}

fn remove_dir(directory: PathBuf) -> Result<(), MyMakeError> {
    if directory.is_dir() {
        println!("Removing directory {:?}", directory);
        fs::remove_dir_all(directory)?;
    }
    else {
        return Err(MyMakeError::from(format!("Can not delete {:?}: No such directory.", directory)));
    }
    Ok(())
}