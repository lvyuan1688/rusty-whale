//! prompt-template: templated system prompts for rusty-whale.
//!
//! A `Template` is a string with `{{var}}` placeholders. `render` fills
//! them from a `serde_json::Value`. Unknown placeholders are left as-is
//! (escaped back to the literal `{{var}}`), which lets templates carry
//! optional context that may not always be present.
//!
//! Use cases:
//!   - Per-role system prompts (coder / reviewer / planner).
//!   - Injecting project metadata (lang, framework, git branch).
//!   - Building tool-use prompts from a stable scaffolding.

use anyhow::{anyhow, Result};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// A compiled prompt template.
#[derive(Debug, Clone)]
pub struct Template {
    raw: String,
    /// Names of `{{var}}` placeholders found in `raw`, in first-occurrence order.
    pub placeholders: Vec<String>,
}

impl Template {
    /// Parse a template string and discover its placeholders.
    pub fn new(raw: impl Into<String>) -> Self {
        let raw = raw.into();
        let placeholders = find_placeholders(&raw);
        Self { raw, placeholders }
    }

    /// Render the template using `value` as the source of variable bindings.
    /// Nested lookups use dot path: `{{project.lang}}` → value["project"]["lang"].
    pub fn render(&self, value: &Value) -> Result<String> {
        let mut out = String::with_capacity(self.raw.len());
        let bytes = self.raw.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                // find closing }}
                if let Some(end) = find_close(&self.raw, i + 2) {
                    let key = self.raw[i + 2..end].trim();
                    match lookup(value, key) {
                        Some(s) => out.push_str(&s),
                        None => {
                            // leave literal
                            out.push_str("{{");
                            out.push_str(key);
                            out.push_str("}}");
                        }
                    }
                    i = end + 2;
                    continue;
                }
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        Ok(out)
    }

    /// Render with a serializable struct (convenience over `render(&Value)`).
    pub fn render_with<T: Serialize>(&self, ctx: &T) -> Result<String> {
        let v = serde_json::to_value(ctx)?;
        self.render(&v)
    }

    /// Render with no bindings — keeps unknown placeholders literal.
    pub fn render_static(&self) -> Result<String> {
        self.render(&Value::Null)
    }
}

/// Built-in per-role templates.
pub fn role_template(role: &str) -> Result<Template> {
    let raw = match role {
        "coder" => "You are a senior software engineer. \
                     Project: {{project.name}} ({{project.lang}}). \
                     Prefer minimal, readable changes.",
        "reviewer" => "You are reviewing a PR. \
                        Focus on correctness, then security, then style. \
                        Repo: {{project.name}}.",
        "planner" => "You break work into small, verifiable steps. \
                       Current goal: {{goal}}. Constraints: {{constraints}}.",
        _ => return Err(anyhow!("unknown role: {role}")),
    };
    Ok(Template::new(raw))
}

// ---- internal helpers ----------------------------------------------------

fn find_placeholders(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'{' && i + 1 < bytes.len() && bytes[i + 1] == b'{' {
            if let Some(end) = find_close(s, i + 2) {
                let key = s[i + 2..end].trim().to_string();
                if !key.is_empty() && !out.contains(&key) {
                    out.push(key);
                }
                i = end + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn find_close(s: &str, start: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = start;
    while i + 1 < bytes.len() {
        if bytes[i] == b'}' && bytes[i + 1] == b'}' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn lookup(value: &Value, key: &str) -> Option<String> {
    let mut cur = value;
    for part in key.split('.') {
        cur = cur.get(part)?;
    }
    match cur {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Null => None,
        other => Some(other.to_string()),
    }
}

// ---- tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_simple_placeholder() {
        let t = Template::new("Hello {{name}}!");
        assert_eq!(t.render(&json!({"name": "world"})).unwrap(), "Hello world!");
    }

    #[test]
    fn handles_nested_dot_path() {
        let t = Template::new("lang={{project.lang}}");
        let v = json!({"project": {"lang": "rust"}});
        assert_eq!(t.render(&v).unwrap(), "lang=rust");
    }

    #[test]
    fn leaves_unknown_placeholder_literal() {
        let t = Template::new("Hi {{missing}}");
        assert_eq!(t.render(&json!({})).unwrap(), "Hi {{missing}}");
    }

    #[test]
    fn placeholders_discovered_in_order() {
        let t = Template::new("{{a}} {{b}} {{a}}");
        assert_eq!(t.placeholders, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn role_template_coder() {
        let t = role_template("coder").unwrap();
        let v = json!({"project": {"name": "rusty-whale", "lang": "rust"}});
        let s = t.render(&v).unwrap();
        assert!(s.contains("senior software engineer"));
        assert!(s.contains("rusty-whale"));
    }

    #[test]
    fn unknown_role_errors() {
        assert!(role_template("dj").is_err());
    }

    #[test]
    fn render_with_serializable() {
        #[derive(Serialize)]
        struct Ctx { name: String }
        let t = Template::new("Hi {{name}}");
        let s = t.render_with(&Ctx { name: "x".into() }).unwrap();
        assert_eq!(s, "Hi x");
    }
}
