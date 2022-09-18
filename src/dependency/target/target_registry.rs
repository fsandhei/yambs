use crate::cache;
use crate::dependency::target::TargetNode;
use crate::utility;
// LEGG TIL TESTER

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TargetRegistry {
    registry: Vec<TargetNode>,
}

impl TargetRegistry {
    pub fn new() -> TargetRegistry {
        TargetRegistry {
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

    pub fn number_of_targets(&self) -> usize {
        self.registry.len()
    }

    pub fn add_target(&mut self, target: TargetNode) {
        self.registry.push(target);
    }

    pub fn target_from_path(&self, path: &std::path::Path) -> Option<TargetNode> {
        for target in &self.registry {
            let borrowed_dep = target.try_borrow();
            if let Ok(dep) = borrowed_dep {
                if dep.path == *path {
                    return Some(target.clone());
                }
            } else {
                return None;
            }
        }
        None
    }
}

impl cache::Cacher for TargetRegistry {
    const CACHE_FILE_NAME: &'static str = "targets";
}