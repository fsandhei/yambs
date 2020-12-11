use error::MyMakeError;
use mmk_parser;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
pub struct Dependency {
    pub path: std::path::PathBuf,
    pub mmk_data: mmk_parser::Mmk,
    pub requires: Vec<Rc<Dependency>>,
}

impl Dependency {
    pub fn new() -> Dependency {
        Dependency {
            path: std::path::PathBuf::new(),
            mmk_data: mmk_parser::Mmk::new(),
            requires: Vec::new(),
        }
    }
    pub fn from(path: &std::path::Path) -> Dependency {
        Dependency {
            path: std::path::PathBuf::from(path),
            mmk_data: mmk_parser::Mmk::new(),
            requires: Vec::new(),
        }
    }

    pub fn create_dependency_from_path(path: &std::path::PathBuf) -> Result<Dependency, MyMakeError>{
        let mut dependency = Dependency::from(path);
        dependency.read_and_add_mmk_data()?;
        dependency.detect_and_add_dependencies()?;
        dependency.print_ok();
        Ok(dependency)
    }

    pub fn add_dependency(self: &mut Self, dependency: Dependency) {
        self.requires.push(Rc::new(dependency));
    }

    pub fn read_and_add_mmk_data(self: &mut Self) -> Result<mmk_parser::Mmk, MyMakeError>{
        let file_content = match mmk_parser::read_file(&self.path)
        {
            Ok(data) => data,
            Err(err) => return Err(MyMakeError::from(format!("Error parsing {:?}: {}", self.path, err))),
        };
        let mut mmk_data = mmk_parser::Mmk::new();
        mmk_data.parse_file(&file_content);
        self.mmk_data = mmk_data.clone();
        Ok(mmk_data)
    }

    pub fn detect_and_add_dependencies(self: &mut Self) -> Result<(), MyMakeError>{
        for path in self.mmk_data.data["MMK_DEPEND"].clone() {
            if path == "" {
                break;
            }
            let mmk_path = path.clone();
            let dep_path = std::path::Path::new(&mmk_path).join("mymakeinfo.mmk");
            let dependency = Dependency::create_dependency_from_path(&dep_path)?;
            self.add_dependency(dependency);
        }
        Ok(())
    }

    pub fn print_ok(self: &Self) {
        print!(".");
    }
}