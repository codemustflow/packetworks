use confique::Config;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Config)]
pub struct Environment {
    #[config(env = "PACKETWORKS_LOG_LEVEL", default = "info")]
    pub log_level: LogLevel,
}

impl Environment {
    pub fn load() -> Result<Self, confique::Error> {
        let _ = dotenvy::dotenv();
        Self::builder().env().load()
    }
}
