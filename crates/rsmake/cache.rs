// TODO: Get caching up and running. Start with compiler caching
// TODO: Use serde maybe?

use serde::{Deserialize, Serialize};

use crate::cli::build_configurations::BuildDirectory;
use crate::errors::CacheError;

#[derive(Deserialize, Serialize)]
pub struct Cache {
    cache_directory: std::path::PathBuf,
}

impl Cache {
    pub fn new(build_directory: &BuildDirectory) -> std::io::Result<Self> {
        std::fs::create_dir_all(&build_directory.as_path().join("cache"))?;
        Ok(Self {
            cache_directory: build_directory.as_path().to_path_buf(),
        })
    }

    pub fn cache<T>(&self, object: &T, filename: &str) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let cache_file = self.cache_directory.join(filename);
        let file_handler = std::fs::File::create(cache_file).map_err(CacheError::FailedToCache)?;
        serde_json::to_writer(file_handler, object).map_err(CacheError::FailedToWrite)
    }
}

pub trait Cacher {
    type Err;
    fn cache(&self, cache: &Cache) -> Result<(), Self::Err>;

    // fn is_changed(&self, cache: &Cache) -> bool {

    // }
}
