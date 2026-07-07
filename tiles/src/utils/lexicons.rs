//! Module for structs to represent lexicons in Atproto

use atrium_api::types::string::Datetime;
use serde::{Deserialize, Serialize};

use crate::repl::PiMsgEvent;

#[derive(Serialize, Deserialize, Debug)]
pub struct SessionSnapshotRecord {
    #[serde(rename = "$type")]
    pub r#type: String,
    pub name: String,
    #[serde(rename = "sessionId")]
    session_id: String,
    #[serde(rename = "createdAt")]
    created_at: String,
    // One turn a user prompt + the assistant response
    pub turns: Vec<Turn>,
}

impl SessionSnapshotRecord {
    pub fn new(name: &str, session_id: &str) -> Self {
        SessionSnapshotRecord {
            r#type: String::from("run.tiles.chat.sessionSnapshot"),
            name: name.to_owned(),
            session_id: session_id.to_owned(),
            created_at: Datetime::now().as_str().to_string(),
            turns: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Turn {
    pub api: Option<String>,
    pub provider: Option<String>,
    pub model: String,
    pub messages: Vec<PiMsgEvent>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct ModelUsage {
    input: i32,
    output: i32,
    #[serde(rename = "cacheRead")]
    cache_read: Option<i32>,
    #[serde(rename = "cacheWrite")]
    cache_write: Option<i32>,
    #[serde(rename = "totalTokens")]
    total_tokens: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ChatMessage {
    role: String,
    content: Vec<ContentItem>,
    timestamp: i64,
    #[serde(rename = "toolName")]
    tool_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ContentItem {
    r#type: String,
    text: Option<String>,
    thinking: Option<String>,
    name: Option<String>,
    arguments: Option<String>,
}
