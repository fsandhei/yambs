// TODO: Get caching up and running. Start with compiler caching
// TODO: Use serde maybe?

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::cli::build_configurations::BuildDirectory;
use crate::errors::CacheError;
use crate::utility;

pub struct Cache {
    pub cache_directory: std::path::PathBuf,
}

impl Cache {
    pub fn new(build_directory: &BuildDirectory) -> std::io::Result<Self> {
        let cache_directory = build_directory.as_path().join("cache");
        std::fs::create_dir_all(&cache_directory)?;
        Ok(Self { cache_directory })
    }

    pub fn cache<T>(&self, object: &T, filename: &str) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let cache_file = self.cache_directory.join(filename);
        let file_handler = std::fs::File::create(cache_file).map_err(CacheError::FailedToCache)?;
        serde_json::to_writer(file_handler, object).map_err(CacheError::FailedToWrite)
    }

    pub fn detect_change<T>(&self, cached: &T, filename: &str) -> bool
    where
        T: DeserializeOwned + PartialEq,
    {
        let cache_file = self.cache_directory.join(filename);
        if cache_file.is_file() {
            let cached_data = utility::read_file(&cache_file).expect("Failed to read from cache");
            let existing_cache: T =
                serde_json::from_str(&cached_data).expect("Failed to deserialize cache data");
            return existing_cache == *cached;
        }
        false
    }
}

pub trait Cacher {
    type Err;
    fn cache(&self, cache: &Cache) -> Result<(), Self::Err>;

    fn is_changed(&self, cache: &Cache) -> bool;
}
