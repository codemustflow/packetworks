use confique::Config;
use logging::LogLevel;

#[derive(Debug, Config)]
pub struct Environment {
    #[config(env = "LOG_LEVEL", default = "info")]
    pub log_level: LogLevel,

    #[config(env = "LOG_FOR_HUMANS", default = false)]
    pub log_for_humans: bool,

    #[config(env = "BIND_ADDRESS", default = "0.0.0.0")]
    pub bind_address: String,

    #[config(env = "BIND_PORT", default = 9000)]
    pub bind_port: u16,
}

pub fn load_environment() -> Result<Environment, confique::Error> {
    let _ = dotenvy::dotenv();
    Environment::builder().env().load()
}
