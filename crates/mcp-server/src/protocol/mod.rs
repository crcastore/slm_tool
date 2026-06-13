// Protocol-level utilities.
//
// The MCP protocol serialization and deserialization is handled by the `rmcp`
// crate.  This module provides project-specific helpers on top of the protocol
// types.

use rmcp::model::Content;
use serde::Serialize;

/// Serialise `value` as pretty-printed JSON and wrap it in an MCP text
/// `Content` object.
pub fn json_content(value: impl Serialize) -> Content {
    let text = serde_json::to_string_pretty(&value)
        .unwrap_or_else(|e| format!("{{\"error\": \"serialization failed: {e}\"}}"));
    Content::text(text)
}
