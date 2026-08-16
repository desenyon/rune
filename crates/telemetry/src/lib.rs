//! Structured observability. Secrets are redacted before events are emitted.

mod crash;

pub use crash::{write_crash_bundle, CrashBundleRequest};
use rune_security::redact_secrets;
use std::io::{self, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Clone, Debug)]
pub struct TelemetryConfig {
    pub json: bool,
    pub filter: String,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            json: false,
            filter: "info".into(),
        }
    }
}

struct RedactingWriter<W: Write + Send + Sync + 'static> {
    inner: Arc<Mutex<W>>,
}

impl<W: Write + Send + Sync + 'static> Clone for RedactingWriter<W> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl<W: Write + Send + Sync + 'static> Write for RedactingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let (redacted, _) = redact_secrets(&text);
        self.inner.lock().expect("log writer").write_all(redacted.as_bytes())?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.lock().expect("log writer").flush()
    }
}

impl<W: Write + Send + Sync + 'static> MakeWriter<'_> for RedactingWriter<W> {
    type Writer = RedactingWriter<W>;

    fn make_writer(&self) -> Self::Writer {
        self.clone()
    }
}

pub fn init(config: TelemetryConfig) -> Result<(), String> {
    let filter = EnvFilter::try_new(&config.filter)
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive(LevelFilter::INFO.into());
    let writer = RedactingWriter {
        inner: Arc::new(Mutex::new(io::stderr())),
    };
    let subscriber = fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false);
    if config.json {
        subscriber.json().try_init().map_err(|err| err.to_string())
    } else {
        subscriber.try_init().map_err(|err| err.to_string())
    }
}

pub fn export_debug_log(destination: PathBuf, contents: &str) -> std::io::Result<()> {
    let (redacted, _) = redact_secrets(contents);
    std::fs::write(destination, redacted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exported_logs_redact_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("debug.log");
        export_debug_log(path.clone(), "Authorization: Bearer supersecrettokenvalue").unwrap();
        let text = std::fs::read_to_string(path).unwrap();
        assert!(text.contains("[REDACTED]"));
        assert!(!text.contains("supersecrettokenvalue"));
    }
}
