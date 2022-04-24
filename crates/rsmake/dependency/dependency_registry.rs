use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use crate::dependency::{Dependency, DependencyNode};

// LEGG TIL TESTER

#[allow(dead_code)]
pub struct DependencyRegistry {
    registry: Vec<DependencyNode>,
}

impl DependencyRegistry {
    pub fn new() -> DependencyRegistry {
        DependencyRegistry {
            registry: Vec::new(),
        }
    }

    pub fn number_of_dependencies(&self) -> usize {
        self.registry.len()
    }

    pub fn add_dependency(&mut self, dependency: Rc<RefCell<Dependency>>) {
        self.registry.push(dependency);
    }

    pub fn dependency_from_path(&self, path: &PathBuf) -> Option<Rc<RefCell<Dependency>>> {
        for dependency in &self.registry {
            let borrowed_dep = dependency.try_borrow();
            if borrowed_dep.is_ok() {
                if borrowed_dep.unwrap().path == *path {
                    return Some(Rc::clone(&dependency));
                }
            } else {
                return None;
            }
        }
        None
    }
}
