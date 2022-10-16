use crate::build_target::{BuildTarget, TargetNode, TargetType};
use crate::cache;
use crate::utility;
// LEGG TIL TESTER

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct TargetRegistry {
    pub registry: Vec<TargetNode>,
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

    pub fn get_target(
        &self,
        path: &std::path::Path,
        target_type: TargetType,
    ) -> Option<TargetNode> {
        for target in &self.registry {
            let borrowed_target = target.try_borrow();
            if let Ok(borrowed_target) = borrowed_target {
                if borrowed_target.manifest.directory == *path
                    && borrowed_target.target_type == target_type
                {
                    return Some(target.clone());
                }
            }
        }
        None
    }

    pub fn get_target_from_predicate<P>(&self, predicate: P) -> Option<TargetNode>
    where
        P: Fn(&BuildTarget) -> bool,
    {
        for target in &self.registry {
            let borrowed_target = target.try_borrow();
            if let Ok(borrowed_target) = borrowed_target {
                if predicate(&*borrowed_target) {
                    return Some(target.clone());
                }
            }
        }
        None
    }
}

impl cache::Cacher for TargetRegistry {
    const CACHE_FILE_NAME: &'static str = "targets";
}
