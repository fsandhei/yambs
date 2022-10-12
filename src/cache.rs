// TODO: Get caching up and running. Start with compiler caching
// TODO: Use serde maybe?

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::errors::CacheError;
use crate::utility;

pub struct Cache {
    pub cache_directory: std::path::PathBuf,
}

impl Cache {
    pub fn new(base_directory: &std::path::Path) -> std::io::Result<Self> {
        let cache_directory = base_directory.join("cache");
        std::fs::create_dir_all(&cache_directory)?;
        Ok(Self { cache_directory })
    }

    pub fn cache<T>(&self, object: &T) -> Result<(), CacheError>
    where
        T: Serialize + Cacher,
    {
        let cache_file = self.cache_directory.join(T::CACHE_FILE_NAME);
        let file_handler = std::fs::File::create(cache_file).map_err(CacheError::FailedToCache)?;
        serde_json::to_writer_pretty(file_handler, object).map_err(CacheError::FailedToWrite)
    }

    pub fn detect_change<T>(&self, cached: &T) -> bool
    where
        T: DeserializeOwned + PartialEq + Cacher,
    {
        let cache_file = self.cache_directory.join(T::CACHE_FILE_NAME);
        if cache_file.is_file() {
            let cached_data = utility::read_file(&cache_file).expect("Failed to read from cache");
            let existing_cache: T =
                serde_json::from_str(&cached_data).expect("Failed to deserialize cache data");
            return existing_cache == *cached;
        }
        false
    }

    pub fn from_cache<T>(&self) -> Option<T>
    where
        T: DeserializeOwned + Cacher,
        T: for<'de> Deserialize<'de>,
    {
        let file = std::fs::File::open(self.cache_directory.join(T::CACHE_FILE_NAME)).ok()?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).ok()
    }
}

pub trait Cacher {
    const CACHE_FILE_NAME: &'static str;
}
