use crate::config::clear_config;
use crate::error::CfdError;

pub fn execute() -> Result<(), CfdError> {
    clear_config()
}
