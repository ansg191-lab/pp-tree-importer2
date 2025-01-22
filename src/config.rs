use std::sync::Arc;

use crate::error::Error;

const CPU_MULTIPLIER: usize = 3;

#[derive(Debug, Clone)]
pub struct Config {
    pub gdrive_folder_id: String,
    pub bucket_name: String,
    pub concurrency: usize,
}

impl Config {
    pub fn from_env() -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self {
            gdrive_folder_id: std::env::var("PP_GDRIVE_FOLDER")?,
            bucket_name: std::env::var("PP_BUCKET")?,
            concurrency: std::env::var("PP_CONCURRENCY")
                .map(|x| x.parse())
                .unwrap_or_else(|_| Ok(num_cpus::get() * CPU_MULTIPLIER))?,
        }))
    }
}
