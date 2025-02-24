use std::{env::VarError, sync::Arc};

use tracing::Subscriber;
use tracing_subscriber::{Layer, registry::LookupSpan};
use valuable::{Valuable, Value, Visit};

use crate::error::Error;

const CPU_MULTIPLIER: usize = 3;

#[derive(Debug, Clone, Valuable)]
pub struct Config {
    pub log_format: LogFormat,
    pub gdrive_folder_id: String,
    pub bucket_name: String,
    pub concurrency: usize,
}

impl Config {
    pub fn from_env() -> Result<Arc<Self>, Error> {
        Ok(Arc::new(Self {
            log_format: LogFormat::from_env()?,
            gdrive_folder_id: std::env::var("PP_GDRIVE_FOLDER")?,
            bucket_name: std::env::var("PP_BUCKET")?,
            concurrency: std::env::var("PP_CONCURRENCY")
                .map(|x| x.parse())
                .unwrap_or_else(|_| Ok(num_cpus::get() * CPU_MULTIPLIER))?,
        }))
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub enum LogFormat {
    #[default]
    Text,
    Json,
}

impl LogFormat {
    fn from_env() -> Result<Self, Error> {
        let res = std::env::var("PP_LOG_FORMAT");
        match res.as_deref() {
            Ok("text") => Ok(Self::Text),
            Ok("json") => Ok(Self::Json),
            Ok(s) => Err(Error::UnknownLogType(s.to_string())),
            Err(VarError::NotPresent) => Ok(Self::Text),
            Err(e) => Err(Error::EnvVar(e.clone())),
        }
    }

    pub fn into_layer<S>(self) -> Box<dyn Layer<S> + Send + Sync>
    where
        S: Subscriber + for<'a> LookupSpan<'a>,
    {
        match self {
            Self::Text => Layer::boxed(tracing_subscriber::fmt::layer()),
            Self::Json => Layer::boxed(tracing_subscriber::fmt::layer().json()),
        }
    }
}

impl Valuable for LogFormat {
    fn as_value(&self) -> Value<'_> {
        match self {
            Self::Text => "text".into(),
            Self::Json => "json".into(),
        }
    }
    fn visit(&self, visit: &mut dyn Visit) {
        visit.visit_value(self.as_value())
    }
}
