use fermah_common::cli::spinner::SpinnerLayer;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

use crate::{config::Config, Telemetry};

pub struct StdoutTelemetry {
    logs: fmt::Layer<Registry>,
    spinner_layer: Option<SpinnerLayer<Registry>>,
    filter: EnvFilter,
}

impl Telemetry for StdoutTelemetry {
    fn from_config(config: Config) -> Self {
        Self {
            logs: Self::default_fmt_layer(),
            spinner_layer: None,
            filter: Self::filter_from_config(Some(&config)),
        }
    }

    fn with_filter(mut self, filter: EnvFilter) -> Self {
        self.filter = filter;
        self
    }

    fn with_spinner_layer(mut self, layer: SpinnerLayer<Registry>) -> Self {
        self.spinner_layer = Some(layer);
        self
    }

    fn with_logs(mut self) -> Self {
        self.logs = Self::default_fmt_layer();
        self
    }

    fn with_tracer(self) -> Self {
        self
    }

    fn with_metrics(self) -> Self {
        self
    }

    fn with_service_name(self, _name: String) -> Self {
        self
    }

    fn with_env(self, _env: String) -> Self {
        self
    }

    fn init(self) {
        if let Some(sl) = self.spinner_layer {
            Registry::default().with(sl).with(self.filter).init();
        } else {
            Registry::default().with(self.logs).with(self.filter).init();
        }
    }
}

impl Default for StdoutTelemetry {
    fn default() -> Self {
        Self {
            logs: Self::default_fmt_layer(),
            spinner_layer: None,
            filter: Self::filter_from_config(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use tracing::info;

    use super::*;

    #[test]
    fn test_default() {
        StdoutTelemetry::default().init();
        info!("Hello, world!")
    }
}
