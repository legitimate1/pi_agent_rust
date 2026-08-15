use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ToolConfig {
    pub shell_path: Option<String>,
    pub shell_command_prefix: Option<String>,
    pub image_auto_resize: bool,
    pub block_images: bool,
    pub tool_descriptions: HashMap<String, String>,
    /// Per-tool JSON Schema overrides for tool parameters (from tools.toml).
    pub tool_parameters: HashMap<String, serde_json::Value>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            shell_path: None,
            shell_command_prefix: None,
            image_auto_resize: true,
            block_images: false,
            tool_descriptions: HashMap::new(),
            tool_parameters: HashMap::new(),
        }
    }
}
