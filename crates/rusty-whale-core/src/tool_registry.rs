//! Tool registry — dynamic tool registration and lookup.

use crate::lib::{Tool, ToolDef, ToolError};
use std::collections::HashMap;
use std::sync::Arc;

/// Registry of tools available to the agent.
///
/// Tools are registered at startup and looked up by name when the
/// LLM requests a tool call.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tools: HashMap::new() }
    }

    /// Register a tool. Overwrites if a tool with the same name exists.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Look up a tool by name.
    pub fn get(&self, name: &str) -> Option<&Arc<dyn Tool>> {
        self.tools.get(name)
    }

    /// List all registered tool definitions (for sending to the LLM).
    pub fn definitions(&self) -> Vec<ToolDef> {
        self.tools
            .values()
            .map(|t| ToolDef {
                name: t.name().to_string(),
                description: t.description().to_string(),
                parameters: t.parameters_schema(),
            })
            .collect()
    }

    /// Number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Whether the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Execute a tool by name with the given arguments.
    pub async fn execute(
        &self,
        name: &str,
        args: &HashMap<String, serde_json::Value>,
    ) -> Result<String, ToolError> {
        match self.tools.get(name) {
            Some(tool) => tool.execute(args).await,
            None => Err(ToolError::InvalidArgs(format!("unknown tool: {}", name))),
        }
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str { "echo" }
        fn description(&self) -> &str { "Echo back the input" }
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({
                "type": "object",
                "properties": {
                    "text": { "type": "string", "description": "Text to echo" }
                },
                "required": ["text"]
            })
        }
        async fn execute(
            &self,
            args: &HashMap<String, serde_json::Value>,
        ) -> Result<String, ToolError> {
            args.get("text")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .ok_or_else(|| ToolError::InvalidArgs("missing 'text'".into()))
        }
    }

    #[tokio::test]
    async fn register_and_execute() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        assert_eq!(reg.len(), 1);

        let mut args = HashMap::new();
        args.insert("text".to_string(), serde_json::json!("hello"));
        let result = reg.execute("echo", &args).await.unwrap();
        assert_eq!(result, "hello");
    }

    #[tokio::test]
    async fn unknown_tool_errors() {
        let reg = ToolRegistry::new();
        let result = reg.execute("nonexistent", &HashMap::new()).await;
        assert!(matches!(result, Err(ToolError::InvalidArgs(_))));
    }
}
