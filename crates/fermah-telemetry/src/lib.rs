pub mod config;

#[cfg(feature = "stdout")]
pub mod stdout;

#[cfg(feature = "tracing")]
pub mod tonic;

use std::str::FromStr;

use fermah_common::cli::spinner::SpinnerLayer;
use tracing_subscriber::{filter::Directive, fmt, EnvFilter, Registry};

use crate::config::Config;

pub const DEFAULT_FILTER: &str = "info";

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

    fn default_directive(config: &Config) -> Directive {
        let filter = config.filter.clone().unwrap_or(DEFAULT_FILTER.to_string());
        Directive::from_str(filter.as_str())
            .inspect_err(|e| eprintln!("{e}"))
            .unwrap()
    }

    fn with_directive(self, directive: Directive) -> Self;
    fn with_filter(self, filter: EnvFilter) -> Self;
    fn with_spinner_layer(self, layer: SpinnerLayer<Registry>) -> Self;
    fn with_logs(self) -> Self;
    fn with_tracer(self) -> Self;
    fn with_metrics(self) -> Self;
    fn init(self);
}
