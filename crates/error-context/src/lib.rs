//! error-context — structured error context propagation for rusty-whale.
//!
//! Instead of stringified error chains, `ErrorContext` carries a typed
//! breadcrumb trail: component + operation + optional payload. Contexts
//! nest via `with_cause`, so the root error can be walked back to its
//! origin. `Report` renders the chain for logs or TUI display.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

/// Severity of an error context entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Severity {
    /// Informational context (e.g. "loading config").
    Info,
    /// Recoverable warning.
    Warn,
    /// Hard failure.
    Error,
    /// Fatal — process cannot continue.
    Fatal,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Info => f.write_str("INFO"),
            Severity::Warn => f.write_str("WARN"),
            Severity::Error => f.write_str("ERROR"),
            Severity::Fatal => f.write_str("FATAL"),
        }
    }
}

/// A single breadcrumb in an error chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Breadcrumb {
    /// Owning component, e.g. "provider::openai".
    pub component: String,
    /// Operation that was attempted, e.g. "complete".
    pub operation: String,
    /// Optional structured payload (request id, status code, ...).
    pub payload: Option<serde_json::Value>,
    /// Severity at this hop.
    pub severity: Severity,
}

impl Breadcrumb {
    pub fn new(component: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            operation: operation.into(),
            payload: None,
            severity: Severity::Error,
        }
    }

    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = Some(payload);
        self
    }

    pub fn with_severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    pub fn info(mut self) -> Self {
        self.severity = Severity::Info;
        self
    }

    pub fn warn(mut self) -> Self {
        self.severity = Severity::Warn;
        self
    }

    pub fn fatal(mut self) -> Self {
        self.severity = Severity::Fatal;
        self
    }
}

/// A structured error with a breadcrumb chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorContext {
    /// Human-readable root message.
    pub message: String,
    /// Breadcrumbs ordered root-first.
    pub chain: VecDeque<Breadcrumb>,
    /// Optional error code for programmatic matching.
    pub code: Option<String>,
    /// Optional retry hint (seconds). `None` = not retryable.
    pub retry_after_secs: Option<u32>,
}

impl ErrorContext {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            chain: VecDeque::new(),
            code: None,
            retry_after_secs: None,
        }
    }

    /// Push a breadcrumb onto the chain (becomes the new root cause context).
    pub fn with(mut self, breadcrumb: Breadcrumb) -> Self {
        self.chain.push_back(breadcrumb);
        self
    }

    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    pub fn retryable_in(mut self, secs: u32) -> Self {
        self.retry_after_secs = Some(secs);
        self
    }

    pub fn is_retryable(&self) -> bool {
        self.retry_after_secs.is_some()
    }

    /// Walk the chain and collect component names (root → leaf).
    pub fn components(&self) -> Vec<&str> {
        self.chain.iter().map(|b| b.component.as_str()).collect()
    }

    /// Render as a single multi-line string for logs.
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("[{}] {}", self.severity_root(), self.message));
        if let Some(code) = &self.code {
            out.push_str(&format!(" (code={})", code));
        }
        if let Some(retry) = self.retry_after_secs {
            out.push_str(&format!(" (retry_after={}s)", retry));
        }
        out.push('\n');
        for (i, b) in self.chain.iter().enumerate() {
            out.push_str(&format!(
                "  └─ #{} [{}] {}::{}",
                i + 1,
                b.severity,
                b.component,
                b.operation
            ));
            if let Some(p) = &b.payload {
                out.push_str(&format!(" payload={}", p));
            }
            out.push('\n');
        }
        out
    }

    fn severity_root(&self) -> Severity {
        self.chain
            .back()
            .map(|b| b.severity)
            .unwrap_or(Severity::Error)
    }
}

impl fmt::Display for ErrorContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.render())
    }
}

impl std::error::Error for ErrorContext {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

/// Convenience constructor — like `anyhow::anyhow` but for ErrorContext.
pub fn ctx(message: impl Into<String>) -> ErrorContext {
    ErrorContext::new(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn breadcrumb_builder_chains() {
        let b = Breadcrumb::new("provider", "complete")
            .with_payload(json!({"status": 429}))
            .warn();
        assert_eq!(b.component, "provider");
        assert_eq!(b.operation, "complete");
        assert_eq!(b.severity, Severity::Warn);
        assert!(b.payload.is_some());
    }

    #[test]
    fn error_context_nests_breadcrumbs() {
        let e = ctx("rate limited by provider")
            .with(Breadcrumb::new("provider::openai", "complete"))
            .with(Breadcrumb::new("retry", "backoff").info())
            .with_code("E_RATE_LIMIT")
            .retryable_in(5);
        assert_eq!(e.chain.len(), 2);
        assert_eq!(e.code.as_deref(), Some("E_RATE_LIMIT"));
        assert_eq!(e.retry_after_secs, Some(5));
        assert!(e.is_retryable());
    }

    #[test]
    fn components_walk_root_to_leaf() {
        let e = ctx("fail")
            .with(Breadcrumb::new("a", "x"))
            .with(Breadcrumb::new("b", "y"));
        assert_eq!(e.components(), vec!["a", "b"]);
    }

    #[test]
    fn render_includes_message_chain_and_code() {
        let e = ctx("rate limited")
            .with(Breadcrumb::new("openai", "complete").warn())
            .with_code("E_429")
            .retryable_in(2);
        let rendered = e.render();
        assert!(rendered.contains("rate limited"));
        assert!(rendered.contains("code=E_429"));
        assert!(rendered.contains("retry_after=2s"));
        assert!(rendered.contains("openai::complete"));
    }

    #[test]
    fn severity_root_falls_back_to_error() {
        let e = ctx("no breadcrumbs");
        assert_eq!(e.severity_root(), Severity::Error);
    }

    #[test]
    fn severity_root_uses_last_breadcrumb() {
        let e = ctx("x")
            .with(Breadcrumb::new("a", "x").info())
            .with(Breadcrumb::new("b", "y").fatal());
        assert_eq!(e.severity_root(), Severity::Fatal);
    }

    #[test]
    fn error_context_implements_std_error() {
        let e = ctx("boom");
        let _: &dyn std::error::Error = &e;
    }

    #[test]
    fn serde_roundtrip_preserves_chain() {
        let e = ctx("rate limited")
            .with(Breadcrumb::new("openai", "complete").with_payload(json!({"s": 429})))
            .with_code("E_429")
            .retryable_in(3);
        let s = serde_json::to_string(&e).unwrap();
        let back: ErrorContext = serde_json::from_str(&s).unwrap();
        assert_eq!(back.chain.len(), 1);
        assert_eq!(back.code.as_deref(), Some("E_429"));
        assert_eq!(back.retry_after_secs, Some(3));
    }
}
