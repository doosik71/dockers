use tracing_subscriber::EnvFilter;
use tracing_subscriber::fmt;

use crate::config::LoggingConfig;
use crate::error::Result;

pub fn init(config: &LoggingConfig) -> Result<()> {
    let env_filter = EnvFilter::try_new(config.level.clone())?;

    let subscriber = fmt()
        .with_env_filter(env_filter)
        .with_target(config.include_target)
        .with_writer(std::io::stderr)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    if config.write_to_file {
        tracing::warn!(
            log_dir = %config.log_dir.display(),
            "file logging is not implemented yet; stderr logging remains active"
        );
    }

    tracing::debug!(
        log_dir = %config.log_dir.display(),
        "logging initialized"
    );

    Ok(())
}
