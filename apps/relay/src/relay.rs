mod env;

use crate::env::load_environment;
use anyhow::{Context, Result};
use tracing::info;

fn main() -> Result<()> {
    let environment = load_environment().context("failed to load environment")?;
    let _logging =
        logging::init_logging("relay", environment.log_level, environment.log_for_humans)
            .context("failed to initialize logging")?;

    info!(
        log_level = %environment.log_level,
        log_for_humans = environment.log_for_humans,
        "logging initialized"
    );

    Ok(())
}
