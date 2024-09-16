pub mod config;

#[cfg(feature = "stdout")]
pub mod stdout;

#[cfg(feature = "tracing")]
pub mod tonic;

use std::env;

use fermah_common::cli::spinner::SpinnerLayer;
use tracing_subscriber::{fmt, EnvFilter, Registry};

use crate::config::Config;

pub const DEFAULT_FILTER: &str = "info,ethers=debug";

pub trait Telemetry: Default {
    fn from_config(config: Config) -> Self;

    fn default_fmt_layer() -> fmt::Layer<Registry> {
        fmt::layer()
            .with_ansi(cfg!(debug_assertions))
            .with_file(true)
            .with_line_number(true)
            .with_target(false)
            .with_thread_names(true)
    }

    fn filter_from_config(config: Option<&Config>) -> EnvFilter {
        env::var("RUST_LOG").map_or_else(
            |_| {
                let filter = match config {
                    None => DEFAULT_FILTER.to_string(),
                    Some(cfg) => cfg.filter.clone().unwrap_or(DEFAULT_FILTER.to_string()),
                };
                EnvFilter::from(filter)
            },
            |_| EnvFilter::builder().from_env_lossy(),
        )
    }

    fn with_filter(self, filter: EnvFilter) -> Self;
    fn with_spinner_layer(self, layer: SpinnerLayer<Registry>) -> Self;
    fn with_logs(self) -> Self;
    fn with_tracer(self) -> Self;
    fn with_metrics(self) -> Self;
    fn with_service_name(self, name: String) -> Self;
    fn with_env(self, env: String) -> Self;
    fn init(self);
}
