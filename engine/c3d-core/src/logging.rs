//! Logging and tracing initialization helpers.

use tracing_subscriber::EnvFilter;

/// Configuration for process-wide logging.
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    /// Default filter directive when `RUST_LOG` is unset.
    pub default_filter: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            default_filter: "info,c3d_core=debug".to_string(),
        }
    }
}

/// Initialize the global tracing subscriber once per process.
pub fn init_logging(config: &LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(config.default_filter.clone()));

    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
