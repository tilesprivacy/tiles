//! Types we use for Agent communication

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tilekit::modelfile::Role;

//TODO: Change this into harness agnostic types in the names

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum PiResponse {
    #[serde(rename = "response")]
    Response(PiResponseMessage),
    #[serde(rename = "agent_start")]
    AgentStart,
    #[serde(rename = "message_update")]
    MessageUpdate(PiMessageUpdate),
    #[serde(rename = "agent_end")]
    AgentEnd(PiAgentEndEvent),
    #[serde(rename = "turn_end")]
    TurnEnd(PiTurnEndEvent),
    #[serde(rename = "agent_settled")]
    AgentSettled,
    #[serde(rename = "turn_start")]
    TurnStart,
    #[serde(rename = "message_start")]
    MessageStart,
    #[serde(rename = "message_end")]
    MessageEnd,
    #[serde(rename = "bash_execution_update")]
    BashExecutionUpdate,
    #[serde(rename = "tool_execution_start")]
    ToolExecutionStart,
    #[serde(rename = "tool_execution_end")]
    ToolExecutionEnd,
    #[serde(rename = "tool_execution_update")]
    ToolExecutionUpdate,
    #[serde(rename = "queue_update")]
    QueueUpdate,
    #[serde(rename = "compaction_start")]
    CompactionStart,
    #[serde(rename = "compaction_end")]
    CompactionEnd,
    #[serde(rename = "auto_retry_start")]
    AutoRetryStart,
    #[serde(rename = "auto_retry_end")]
    AutoRetryEnd,
    #[serde(rename = "summarization_retry_scheduled")]
    SummarizationRetryScheduled,
    #[serde(rename = "summarization_retry_attempt_start")]
    SummarizationRetryStart,
    #[serde(rename = "summarization_retry_finished")]
    SummarizationRetryFinished,
    #[serde(rename = "extension_error")]
    ExtensionError,
    #[serde[other]]
    Unknown,
}

impl PiResponse {
    pub fn get_type(&self) -> &'static str {
        match self {
            PiResponse::Response(_) => "response",
            PiResponse::AgentStart => "agent_start",
            PiResponse::MessageUpdate(_) => "message_update",
            PiResponse::AgentEnd(_) => "agent_end",
            PiResponse::TurnEnd(_) => "turn_end",
            PiResponse::AgentSettled => "agent_settled",
            PiResponse::Unknown => "unknown",
            PiResponse::TurnStart => "turn_start",
            PiResponse::MessageStart => "message_start",
            PiResponse::MessageEnd => "message_end",
            PiResponse::BashExecutionUpdate => "bash_execution_update",
            PiResponse::ToolExecutionStart => "tool_execution_start",
            PiResponse::ToolExecutionEnd => "tool_execution_end",
            PiResponse::ToolExecutionUpdate => "tool_execution_update",
            PiResponse::QueueUpdate => "queue_update",
            PiResponse::CompactionStart => "compaction_start",
            PiResponse::CompactionEnd => "compaction_end",
            PiResponse::AutoRetryStart => "auto_retry_start",
            PiResponse::AutoRetryEnd => "auto_retry_end",
            PiResponse::SummarizationRetryScheduled => "summarization_retry_scheduled",
            PiResponse::SummarizationRetryStart => "summarization_retry_attempt_start",
            PiResponse::SummarizationRetryFinished => "summarization_retry_finished",
            PiResponse::ExtensionError => "extension_error",
        }
    }
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PiAgentEndEvent {
    pub messages: Vec<PiMsgEvent>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct GetStateData {
    pub model: PiModelInfo,
    #[serde(rename = "thinkingLevel")]
    pub thinking_level: String,
    #[serde(rename = "isStreaming")]
    pub is_streaming: bool,
    #[serde(rename = "sessionId")]
    pub session_id: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PiModelInfo {
    pub id: String,
    pub name: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PiSettings {
    pub compaction: Option<CompactionSettings>,
    #[serde(rename = "defaultThinkingLevel")]
    pub default_thinking_level: Option<ReasoningEffort>,
}

impl Default for PiSettings {
    fn default() -> Self {
        PiSettings {
            compaction: Some(CompactionSettings { enabled: false }),
            default_thinking_level: Some(ReasoningEffort::Medium),
        }
    }
}
#[derive(Serialize, Deserialize, Debug, PartialEq, PartialOrd)]
pub struct CompactionSettings {
    pub enabled: bool,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PiMessageUpdate {
    #[serde(rename = "assistantMessageEvent")]
    pub assistant_message_event: PiAsstTextMsg,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PiAsstTextMsg {
    pub r#type: AsstMsgEventType,
    pub delta: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum AsstMsgEventType {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "text_start")]
    TextStart,
    #[serde(rename = "text_delta")]
    TextDelta,
    #[serde(rename = "text_end")]
    TextEnd,
    #[serde(rename = "thinking_start")]
    ThinkingStart,
    #[serde(rename = "thinking_delta")]
    ThinkingDelta,
    #[serde(rename = "thinking_end")]
    ThinkingEnd,
    #[serde(rename = "toolcall_start")]
    ToolcallStart,
    #[serde(rename = "toolcall_delta")]
    ToolcallDelta,
    #[serde(rename = "toolcall_end")]
    ToolcallEnd,
    #[serde(rename = "done")]
    Done,
    #[serde(rename = "error")]
    Error,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct PiResponseMessage {
    pub command: CommandType,
    pub success: bool,
    pub data: Option<Value>,
}
#[derive(Deserialize, Serialize, Debug)]
pub enum CommandType {
    #[serde(rename = "status")]
    Status,
    #[serde(rename = "share")]
    Share,
    #[serde(rename = "sessions")]
    Sessions,
    #[serde(rename = "resume")]
    Resume,
    #[serde(rename = "reasoning")]
    Reasoning,
    #[serde(rename = "set_thinking_level")]
    SetThinkingLevel,
    #[serde(rename = "abort")]
    Abort,
    #[serde(rename = "skills")]
    Skills,
    #[serde(rename = "get_commands")]
    GetCommands,
    #[serde(other)]
    Unknown,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct Commands {
    pub name: String,
    pub description: String,
    pub source: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PiTurnEndEvent {
    message: PiTurnEndEventMsg,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PiTurnEndEventMsg {
    role: String,
    content: Vec<PiMsgContent>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PiMsgEvent {
    pub role: Role,
    pub content: Vec<PiMsgContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "stopReason")]
    pub stop_reason: Option<String>,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "toolName")]
    pub tool_name: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PiMsgContent {
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default, deserialize_with = "map_to_option_string")]
    pub arguments: Option<String>,
    // Tool name
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

fn map_to_option_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<Value>::deserialize(deserializer)?;

    match opt {
        Some(Value::String(s)) => Ok(Some(s)),
        Some(Value::Object(map)) => serde_json::to_string(&map)
            .map(Some)
            .map_err(serde::de::Error::custom),
        Some(other) => Ok(Some(other.to_string())),
        None => Ok(None),
    }
}
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum ReasoningEffort {
    #[serde(rename = "high")]
    High,
    #[serde(rename = "medium")]
    Medium,
    #[serde(rename = "low")]
    Low,
}
