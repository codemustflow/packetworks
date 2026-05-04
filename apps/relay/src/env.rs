use confique::Config;
use logging::LogLevel;

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
