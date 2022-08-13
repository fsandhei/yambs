use crate::cache;
use crate::dependency::DependencyNode;
use crate::utility;
// LEGG TIL TESTER

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

    pub fn from_cache(cache: &cache::Cache) -> Option<Self> {
        let cache_file = cache
            .cache_directory
            .join(<Self as cache::Cacher>::CACHE_FILE_NAME);
        if cache_file.is_file() {
            let cached_data = utility::read_file(&cache_file).expect("Failed to read from cache");
            return serde_json::from_str(&cached_data).ok();
        }
        None
    }

    pub fn number_of_dependencies(&self) -> usize {
        self.registry.len()
    }

    pub fn add_dependency(&mut self, dependency: DependencyNode) {
        self.registry.push(dependency);
    }

    pub fn dependency_from_path(&self, path: &std::path::Path) -> Option<DependencyNode> {
        for dependency in &self.registry {
            let borrowed_dep = dependency.try_borrow();
            if let Ok(dep) = borrowed_dep {
                if dep.path == *path {
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
    const CACHE_FILE_NAME: &'static str = "dependencies";
}
