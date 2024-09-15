use std::time::Duration;

use indicatif::ProgressBar;
use termion::color;
use tracing::Subscriber;
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

const TICK_STRINGS: [&str; 12] = ["π", "∫", "∑", "∆", "∇", "π", "∂", "∏", "∞", "√", "𝜆", "𝛾"];

#[derive(Clone)]
pub enum SpinnerTemplate {
    Default,
    Progress,
}

#[derive(Debug, Clone)]
pub struct Spinner {
    spinner: ProgressBar,
    current_step: u8,
    total_steps: u8,
}

impl Spinner {
    const DEFAULT_TEMPLATE: &'static str =
        "{prefix} {spinner:.magenta} {elapsed:.yellow} {msg:.magenta}";
    const PROGRESS_TEMPLATE: &'static str =
        "{bar:.magenta/blue} {bytes}/{total_bytes} {eta:.yellow}";

    pub fn new(total_steps: u8, message: &str, template: SpinnerTemplate) -> Self {
        let tpl = match template {
            SpinnerTemplate::Default => Self::DEFAULT_TEMPLATE.to_string(),
            SpinnerTemplate::Progress => {
                format!("{} {}", Self::DEFAULT_TEMPLATE, Self::PROGRESS_TEMPLATE)
            }
        };

        let spinner = ProgressBar::new_spinner().with_prefix(format!("[{}/{}]", 1, total_steps));
        spinner.set_style(
            indicatif::ProgressStyle::with_template(tpl.as_str())
                .unwrap()
                .tick_strings(&TICK_STRINGS),
        );
        spinner.set_message(message.to_string());
        spinner.enable_steady_tick(Duration::from_millis(100));

        Spinner {
            spinner,
            current_step: 1,
            total_steps,
        }
    }

    pub fn update_step(&mut self, message: &str) {
        self.current_step += 1;

        self.spinner
            .set_prefix(format!("[{}/{}]", self.current_step, self.total_steps));
        self.spinner.set_message(message.to_string());
    }

    pub fn suspend(&self) {
        self.spinner.disable_steady_tick();
    }

    pub fn resume(&self) {
        self.spinner.enable_steady_tick(Duration::from_millis(100));
    }

    pub fn finish(&self, message: &str, success: bool) {
        match success {
            true => {
                self.spinner.finish_with_message(format!(
                    "{}✓ {}{}",
                    color::Fg(color::Green),
                    message,
                    color::Fg(color::Reset)
                ))
            }
            false => {
                self.spinner.finish_with_message(format!(
                    "{}✕ {}{}",
                    color::Fg(color::Red),
                    message,
                    color::Fg(color::Reset)
                ))
            }
        }
    }

    pub fn inner(&self) -> &ProgressBar {
        &self.spinner
    }
}

pub struct SpinnerLayer<S: Subscriber> {
    inner: tracing_subscriber::fmt::Layer<S>,
    spinner: Spinner,
}

impl<S: Subscriber> SpinnerLayer<S> {
    pub fn new(inner: tracing_subscriber::fmt::Layer<S>, spinner: Spinner) -> Self {
        Self { inner, spinner }
    }
}

impl<S> Layer<S> for SpinnerLayer<S>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &tracing::Event<'_>, context: Context<'_, S>) {
        self.spinner.inner().suspend(|| {
            self.inner.on_event(event, context);
        });
    }
}
