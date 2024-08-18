use std::time::Duration;

use fermah_common::cli::spinner::SpinnerLayer;
use opentelemetry::{global, KeyValue};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{ExportConfig, Protocol, WithExportConfig};
use opentelemetry_resource_detectors::{HostResourceDetector, OsResourceDetector};
use opentelemetry_sdk::{
    logs::LoggerProvider,
    metrics::{
        reader::{DefaultAggregationSelector, DefaultTemporalitySelector},
        SdkMeterProvider,
    },
    resource::{ResourceDetector, SdkProvidedResourceDetector, TelemetryResourceDetector},
    trace::{RandomIdGenerator, Sampler, Tracer, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::resource as otel_resource;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{
    filter::Directive,
    fmt::Layer,
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
    Registry,
};
use uuid::Uuid;

use crate::{config::Config, Telemetry};

/// Telemetry layer with OTLP exporter and tracing subscriber.
///
/// # Usage
///
/// ## Meters
/// ```ignore
/// use fermah_telemetry::tonic::TonicTelemetry;
/// use fermah_telemetry::config::Config;
///
/// use fermah_config::ConfigFile;
///
/// use opentelemetry::KeyValue;
/// use opentelemetry::global::meter;
///
/// #[tokio::main]
/// async fn main() {
///     TonicTelemetry::default().init();
///
///     let counter = meter("test_meter")
///                 .u64_counter("test_counter")
///                 .init();
///         counter.add(1, &[KeyValue::new("test", "true")]);
/// }
/// ```
///
/// ## Function instrumentation
/// ```
///     #[tracing::instrument(
///          fields(
///              span_field = "value",
///          )
///     )]
///     fn test_fn() {}
/// ```
///
/// ## Logging
/// ```
///    tracing::info!("info log");
/// ```
///
/// can be controlled with RUST_LOG env var.
///
pub struct TonicTelemetry {
    resource: Resource,
    config: Config,
    filter: EnvFilter,
    spinner_layer: Option<SpinnerLayer<Registry>>,
    stdout_logs: Layer<Registry>,
    logs: Option<LoggerProvider>,
    tracer: Option<Tracer>,
    tracer_provider: Option<TracerProvider>,
    metrics: Option<SdkMeterProvider>,
}

impl Telemetry for TonicTelemetry {
    fn from_config(config: Config) -> Self {
        let resource = Self::create_resource();

        Self {
            filter: EnvFilter::builder()
                .with_default_directive(Self::default_directive(&config))
                .from_env_lossy(),
            resource,
            config,
            spinner_layer: None,
            stdout_logs: TonicTelemetry::default_fmt_layer(),
            logs: None,
            tracer: None,
            tracer_provider: None,
            metrics: None,
        }
    }

    fn with_directive(mut self, directive: Directive) -> Self {
        self.filter = EnvFilter::builder()
            .with_default_directive(directive)
            .from_env_lossy();
        self
    }

    fn with_filter(mut self, filter: EnvFilter) -> Self {
        self.filter = filter;
        self
    }

    fn with_spinner_layer(mut self, layer: SpinnerLayer<Registry>) -> Self {
        self.spinner_layer = Some(layer);
        self
    }

    /// For logs with no export, stdout fmt layer is used.
    fn with_logs(mut self) -> Self {
        if self.config.export.is_none() {
            return self;
        }

        let logs = opentelemetry_otlp::new_pipeline()
            .logging()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_export_config(self.get_export_config()),
            )
            .with_log_config(opentelemetry_sdk::logs::config().with_resource(self.resource.clone()))
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("should build opentelemetry logs layer");

        self.logs = Some(logs);
        self
    }

    fn with_tracer(mut self) -> Self {
        if self.config.export.is_none() {
            return self;
        }

        // Provider for stdout of traces
        // let provider = TracerProvider::builder()
        //     .with_simple_exporter(opentelemetry_stdout::SpanExporter::default())
        //     .with_config(self.get_trace_config())
        //     .build();
        // self.tracer_provider = Some(provider.clone());
        // let tracer = provider.tracer("global");

        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_export_config(self.get_export_config()),
            )
            .with_trace_config(self.get_trace_config())
            .install_batch(opentelemetry_sdk::runtime::Tokio)
            .expect("should build opentelemetry tracer layer");

        self.tracer_provider = Some(tracer.provider().unwrap().clone());
        self.tracer = Some(tracer);
        self
    }

    fn with_metrics(mut self) -> Self {
        if self.config.export.is_none() {
            return self;
        }

        let metrics = opentelemetry_otlp::new_pipeline()
            .metrics(opentelemetry_sdk::runtime::Tokio)
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_export_config(self.get_export_config()),
            )
            .with_resource(self.resource.clone())
            .with_period(Duration::from_secs(3))
            .with_timeout(Duration::from_secs(3))
            .with_aggregation_selector(DefaultAggregationSelector::new())
            .with_temporality_selector(DefaultTemporalitySelector::new())
            .build()
            .expect("should build opentelemetry metrics layer");

        self.metrics = Some(metrics);
        self
    }

    fn init(self) {
        if let Some(metrics) = &self.metrics {
            global::set_meter_provider(metrics.clone());
        }

        if let Some(provider) = &self.tracer_provider {
            global::set_tracer_provider(provider.clone());
        }

        let logs = self.logs.as_ref().map(OpenTelemetryTracingBridge::new);

        let tracer = self
            .tracer
            .as_ref()
            .map(|tracer| tracing_opentelemetry::layer().with_tracer(tracer.clone()));

        Registry::default()
            .with(self.stdout_logs)
            .with(logs)
            .with(tracer)
            .with(self.filter)
            .init();
    }
}

impl TonicTelemetry {
    /// Returns the export configuration for the telemetry pipeline.
    /// If not set, it will default to a local OTLP exporter.
    ///
    fn get_export_config(&self) -> ExportConfig {
        let export = self.config.export.clone().unwrap_or_default();

        ExportConfig {
            endpoint: export.endpoint,
            protocol: Protocol::Grpc,
            timeout: Duration::from_secs(export.timeout_secs),
        }
    }

    fn get_trace_config(&self) -> opentelemetry_sdk::trace::Config {
        opentelemetry_sdk::trace::config()
            .with_sampler(Sampler::AlwaysOn)
            .with_id_generator(RandomIdGenerator::default())
            .with_max_events_per_span(64)
            .with_max_attributes_per_span(16)
            .with_max_events_per_span(16)
            .with_resource(self.resource.clone())
    }

    /// Creates a new OpenTelemetry resource based on context info and environment variables.
    ///
    fn create_resource() -> Resource {
        let telemetry_resource = TelemetryResourceDetector.detect(Duration::from_secs(0));
        let sdk_resource = SdkProvidedResourceDetector.detect(Duration::from_secs(0));
        let os_resource = OsResourceDetector.detect(Duration::from_secs(0));

        let hr = HostResourceDetector::default();
        let host_resource = hr.detect(Duration::from_secs(0));

        let provided = Resource::new(vec![
            KeyValue::new(
                otel_resource::SERVICE_NAME,
                env!("CARGO_PKG_NAME").to_string(),
            ),
            KeyValue::new(
                otel_resource::SERVICE_VERSION,
                env!("CARGO_PKG_VERSION").to_string(),
            ),
            KeyValue::new(
                otel_resource::SERVICE_INSTANCE_ID,
                Uuid::new_v4().to_string(),
            ),
            KeyValue::new(
                otel_resource::DEPLOYMENT_ENVIRONMENT,
                std::env::var("ENV").unwrap_or("local".to_string()),
            ),
        ]);

        sdk_resource
            .merge(&provided)
            .merge(&telemetry_resource)
            .merge(&os_resource)
            .merge(&host_resource)
    }
}

impl Default for TonicTelemetry {
    fn default() -> Self {
        Self {
            resource: Self::create_resource(),
            config: Config::default(),
            filter: EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
            spinner_layer: None,
            stdout_logs: TonicTelemetry::default_fmt_layer(),
            logs: None,
            tracer: None,
            tracer_provider: None,
            metrics: None,
        }
        .with_logs()
        .with_tracer()
        .with_metrics()
    }
}

#[cfg(test)]
mod tests {
    use opentelemetry::global::meter;
    use tracing::{debug, error, info, span, trace, warn};

    use super::*;

    /// Ignored, can be ran manually with local alloy exporter running.
    #[ignore]
    #[tokio::test]
    async fn test_telemetry() {
        TonicTelemetry::default().init();

        debug!("test debug log");
        trace!("test trace log");
        info!("test info log");
        warn!("test warn log");
        error!("test error log");

        let counter = meter("test_meter").u64_counter("test_counter").init();
        counter.add(1, &[KeyValue::new("test", "true")]);

        #[tracing::instrument]
        fn test_fn() {
            let span = span!(tracing::Level::INFO, "test_span");
            let _enter = span.enter();

            info!("test span log");
        }

        test_fn();

        // Sleep to allow the exporter to send the data
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
