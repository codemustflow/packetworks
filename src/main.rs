mod env;

use crate::env::Environment;

fn main() -> Result<(), confique::Error> {
    let environment = Environment::load()?;
    let _ = environment.log_level;
    Ok(())
}
