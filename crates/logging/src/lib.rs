use serde::Deserialize;
use strum::Display;
use tracing::span::EnteredSpan;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt;
use tracing_subscriber::layer::Layer;
use tracing_subscriber::prelude::*;
use tracing_subscriber::util::SubscriberInitExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Display)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

pub struct LoggingGuard {
    _binary_scope: Option<EnteredSpan>,
}

pub fn init_logging(
    binary_name: &'static str,
    log_level: LogLevel,
    log_for_humans: bool,
) -> Result<LoggingGuard, tracing_subscriber::util::TryInitError> {
    let level = level_filter(log_level);

    match log_for_humans {
        false => tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .json()
                    .flatten_event(true)
                    .with_current_span(true)
                    .with_span_list(false)
                    .with_filter(level),
            )
            .try_init()?,
        true => tracing_subscriber::registry()
            .with(
                fmt::layer()
                    .pretty()
                    .with_file(false)
                    .with_line_number(false)
                    .with_target(false)
                    .with_filter(level),
            )
            .try_init()?,
    }

    Ok(LoggingGuard {
        _binary_scope: (!log_for_humans)
            .then(|| tracing::info_span!("app", binary = binary_name).entered()),
    })
}

const fn level_filter(level: LogLevel) -> LevelFilter {
    match level {
        LogLevel::Trace => LevelFilter::TRACE,
        LogLevel::Debug => LevelFilter::DEBUG,
        LogLevel::Info => LevelFilter::INFO,
        LogLevel::Warn => LevelFilter::WARN,
        LogLevel::Error => LevelFilter::ERROR,
    }
}
