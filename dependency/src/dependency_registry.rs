use std::path::PathBuf;
use crate::Dependency;

#[allow(dead_code)]
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
        self.registry.push(dependency);
    }


    pub fn dependency_from_path(self: &Self, path: &PathBuf) -> Option<&Dependency> {
        for dependency in &self.registry {
            if &dependency.path == path {
                return Some(dependency)
            }
        }
        None
    }
}