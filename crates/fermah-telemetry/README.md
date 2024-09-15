# Overview

This crate provides a simple way to collect and report telemetry data from your application. It is designed to be ready
to use by the other crates, and flexible enough to be exported to any other telemetry system, due to the usage of
OpenTelemetry Protocol (OTLP).

Telemetry crate covers the following:

- Logs
- Metrics (Counters, Gauges, Histograms)
- Tracing (Function instrumentation)

# Usage

To use this crate, add the following to the `Cargo.toml`:

```toml
[dependencies]
fermah-telemetry = { workspace = true }
```

To use the crate in your code, add the following:

```rust
use fermah_telemetry::tonic::TonicTelemetry;
use fermah_telemetry::config::{Config, OtlpConfig};

use opentelemetry::global::meter;
use opentelemetry::metrics::Counter;
use opentelemetry::KeyValue;

use tracing::{span, info};

/// Telemetry currently only runs in Tokio runtime
#[tokio::main]
async fn main() {
    TonicTelemetry::from_config(Config {
        export: Some(OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            timeout_secs: 3,
        }),
    })
    .with_logs()
    .with_metrics()
    .with_tracer()
    .init();

    // Logging
    info!("info log");

    // Metrics
    let counter = meter("test_meter")
        .u64_counter("test_counter")
        .init();
    counter.add(1, &[KeyValue::new("test", "true")]); // You may add labels to meters
    
    // Tracing wtih span and instrumentation
    #[tracing::instrument]
    fn test_fn() {
        let span = span!(tracing::Level::INFO, "test_fn_span");
        let _enter = span.enter();
    }
    
    test_fn();
}
```

# Configuration

The crate should self-configure labels using an OTLP concept called Resource. The Resource is a set of key-value pairs
that
describe the entity that produced the telemetry data. The Resource is used to configure the telemetry system, and a few
more values can be added in the Collector layer, like OS or Arch.

# Exporter

The crate uses the OpenTelemetry Protocol (OTLP) to export the telemetry data. A default setup using Grafana Alloy is
available as a docker compose file in the root of the crate, alongside a `config.alloy` file that describes the
exporter.
The exporter can be configured using the following environment variables:

- `SERVICE_NAME` - the name of the service to be used in the telemetry data apart from app's own data
- `ENVIRONMENT` - the environment name to be used in the telemetry data, e.g. `main`, `test`, `local`
- `GRAFANA_CLOUD_API_KEY` - the API key for Grafana Cloud with write
  scopes: `logs:write`, `metrics:write`, `traces:write`

You may run the exporter using the following command:

```shell
GRAFANA_CLOUD_API_KEY=<key> SERVICE_NAME=<name> ENVIRONMENT=<local|dev|test|main> docker-compose up -d
```

Grafana Alloy GUI can be accessed on `localhost:12345`.