mod env;

use crate::env::load_environment;
use anyhow::{Context, Result};
use tracing::info;

fn main() -> Result<()> {
    let environment = load_environment().context("failed to load environment")?;
    packetworks_logging::init_logging(environment.log_level, environment.log_for_humans)
        .context("failed to initialize logging")?;

    info!(
        binary = "participant",
        log_level = %environment.log_level,
        log_for_humans = environment.log_for_humans,
        "logging initialized"
    );

    Ok(())
}
