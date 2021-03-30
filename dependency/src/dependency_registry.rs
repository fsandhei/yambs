use std::path::PathBuf;
use crate::Dependency;
use std::cell::RefCell;
use std::rc::Rc;

#[allow(dead_code)]
pub struct DependencyRegistry{
    registry: Vec<Rc<RefCell<Dependency>>>,
}

impl DependencyRegistry {
    pub fn new() -> DependencyRegistry {
        DependencyRegistry {
            registry: Vec::new(),
        }
    }


    pub fn add_dependency(&mut self, dependency: Rc<RefCell<Dependency>>) {
        self.registry.push(dependency);
    }


    pub fn get_last_dependency(&self) -> &Rc<RefCell<Dependency>> {
        self.registry.last().unwrap()
    }


    pub fn get_last_dependency_mut(& mut self) -> &mut Rc<RefCell<Dependency>> {
        self.registry.last_mut().unwrap()
    }


    pub fn get_root_dependency(& self) -> &Rc<RefCell<Dependency>> {
        self.registry.first().unwrap()
    }


    pub fn get_root_dependency_mut(&mut self) -> &mut Rc<RefCell<Dependency>> {
        self.registry.first_mut().unwrap()
    }


    pub fn dependency_from_path(&self, path: &PathBuf) -> Option<Rc<RefCell<Dependency>>> {
        for dependency in &self.registry {
            let borrowed_dep = dependency.try_borrow();
            if borrowed_dep.is_ok() {
                if borrowed_dep.unwrap().path == *path {
                    return Some(Rc::clone(&dependency));
                }
            }
            else {
                return None;
            }
        }
        None
    }
}