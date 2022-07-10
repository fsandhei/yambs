use std::path::PathBuf;

use crate::cache;
use crate::dependency::DependencyNode;
use crate::errors::CompilerError;

// LEGG TIL TESTER

const CACHE_FILE_NAME: &str = "dependencies";

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
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

    pub fn add_dependency(&mut self, dependency: DependencyNode) {
        self.registry.push(dependency);
    }

    pub fn dependency_from_path(&self, path: &PathBuf) -> Option<DependencyNode> {
        for dependency in &self.registry {
            let borrowed_dep = dependency.try_borrow();
            if borrowed_dep.is_ok() {
                if borrowed_dep.unwrap().path == *path {
                    return Some(dependency.clone());
                }
            } else {
                return None;
            }
        }
        None
    }
}

impl cache::Cacher for DependencyRegistry {
    type Err = CompilerError;

    fn cache(&self, cache: &cache::Cache) -> Result<(), Self::Err> {
        cache
            .cache(&self, CACHE_FILE_NAME)
            .map_err(CompilerError::FailedToCache)
    }

    fn is_changed(&self, cache: &cache::Cache) -> bool {
        cache.detect_change(self, CACHE_FILE_NAME)
    }
}