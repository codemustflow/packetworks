use confique::Config;
use serde::Deserialize;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Config)]
pub struct Environment {
    #[config(env = "LOG_LEVEL", default = "info")]
    pub log_level: LogLevel,

    #[config(env = "LOG_FOR_HUMANS", default = false)]
    pub log_for_humans: bool,
}

pub fn load_environment() -> Result<Environment, confique::Error> {
    let _ = dotenvy::dotenv();
    Environment::builder().env().load()
}
