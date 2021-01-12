use error::MyMakeError;
use mmk_parser;
use std::cell::RefCell;
use std::rc::Rc;

pub struct DependencyRegistry {
    pub registry: Vec<Dependency>,
}

impl DependencyRegistry {

    pub fn new() -> DependencyRegistry {
        DependencyRegistry {
            registry: Vec::new(),
        }
    }

    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        // let rc_dependency = Rc::new(dependency);
        self.registry.push(dependency);
    }

    pub fn dependency_in_registry(self: &Self, dependency: Dependency) -> bool {
        self.registry.contains(&Rc::new(dependency))
    }

    pub fn dependency_from_path(self: &Self, path: &std::path::PathBuf) -> Option<&Dependency> {
        for dependency in &self.registry {
            if &dependency.path == path {
                return Some(dependency)
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Dependency {
    pub path: std::path::PathBuf,
    pub mmk_data: mmk_parser::Mmk,
    pub requires: RefCell<Vec<RefCell<Dependency>>>,
    pub makefile_made: bool,
    pub library_name: String,
    pub in_process: bool
}

impl Dependency {
    pub fn new() -> Dependency {
        Dependency {
            path: std::path::PathBuf::new(),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            makefile_made: false,
            library_name: String::new(),
            in_process: false
        }
    }


    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            mmk_data: mmk_parser::Mmk::new(),
            requires: RefCell::new(Vec::new()),
            makefile_made: false,
            library_name: String::new(),
            in_process: false,
        }
    }


    pub fn create_dependency_from_path(path: &std::path::PathBuf,
                                       dep_registry: &mut DependencyRegistry) -> Result<Dependency, MyMakeError>{                      
        let mut dependency = Dependency::from(path);        
        dependency.in_process = true;
        dep_registry.add_dependency(dependency.to_owned());
        dependency.read_and_add_mmk_data()?;
        dependency.add_library_name();
        dependency.detect_and_add_dependencies(dep_registry)?;
        dependency.print_ok();
        dependency.in_process = false;
        Ok(dependency)
    }


    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.borrow_mut().push(RefCell::new(dependency));
    }


    pub fn makefile_made(self: &mut Self)
    {
        self.makefile_made = true;
    }


    pub fn get_build_directory(self: &Self) -> std::path::PathBuf {
        let parent = self.path.parent().unwrap();
        let build_directory_name = std::path::PathBuf::from(".build");
        parent.join(build_directory_name)
    }


    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, MyMakeError>{
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", self.path, err))),
        };
        let mut mmk_data = mmk_parser::Mmk::new();
        mmk_data.parse_file(&file_content)?;
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }


    pub fn add_library_name(self: &mut Self) {
        self.library_name = self.mmk_data.to_string("MMK_LIBRARY_LABEL");
    }


    pub fn detect_and_add_dependencies(self: &mut Self, dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError>{
        for path in self.mmk_data.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");

            self.detect_cycle_dependency_from_path(&dep_path, dep_registry)?;
            let dependency = Dependency::create_dependency_from_path(&dep_path, dep_registry)?;       
            self.add_dependency(dependency);
        }
        Ok(())
    }

    
    pub fn detect_cycle_dependency_from_path(self: &Self, path: &std::path::PathBuf, 
                                             dep_registry: &mut DependencyRegistry) -> Result<(), MyMakeError> {
        if let Some(dependency) = dep_registry.dependency_from_path(path) {
            if dependency.in_process == true {
                return Err(MyMakeError::from(format!("Error: dependency circulation!\n{:?} depends on\n{:?}, which depends on itself", 
                                             path, self.path)));
                }
            }
        Ok(())
        }

    pub fn print_ok(self: &Self) {
        print!(".");
    }
}