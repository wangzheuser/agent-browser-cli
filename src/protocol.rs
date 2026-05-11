use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
    #[serde(rename = "type", default = "default_tab_type")]
    pub tab_type: String,
    #[serde(default)]
    pub connected_at: Option<f64>,
}

fn default_tab_type() -> String {
    "ext_ws".to_string()
}

#[derive(Debug, Clone)]
pub struct Session {
    pub info: TabInfo,
    pub sender: mpsc::UnboundedSender<String>,
    pub disconnected_at: Option<Instant>,
}

impl Session {
    pub fn is_active(&self) -> bool {
        self.disconnected_at.is_none()
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExecResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub closed: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "newTabs")]
    pub new_tabs: Option<Value>,
}

#[derive(Debug)]
pub struct PendingExec {
    pub delivered_at: Option<Instant>,
    pub tx: oneshot::Sender<anyhow::Result<ExecResult>>,
}

#[derive(Default)]
pub struct DriverState {
    pub sessions: HashMap<String, Session>,
    pub pending: HashMap<String, PendingExec>,
    pub default_session_id: Option<String>,
    pub latest_session_id: Option<String>,
    pub active_exec_sessions: HashMap<String, String>,
    pub acked: HashSet<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum WsIncoming {
    #[serde(rename = "ext_ready")]
    ExtReady { tabs: Vec<ExtTab> },
    #[serde(rename = "tabs_update")]
    TabsUpdate { tabs: Vec<ExtTab> },
    #[serde(rename = "ack")]
    Ack { id: String },
    #[serde(rename = "result")]
    Result {
        id: String,
        result: Value,
        #[serde(rename = "newTabs")]
        new_tabs: Option<Value>,
    },
    #[serde(rename = "error")]
    Error {
        id: String,
        error: Value,
        #[serde(rename = "newTabs")]
        new_tabs: Option<Value>,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
pub struct ExtTab {
    pub id: Value,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: String,
}

impl ExtTab {
    pub fn into_tab_info(self) -> TabInfo {
        TabInfo {
            id: match self.id {
                Value::String(s) => s,
                other => other.to_string(),
            },
            url: self.url,
            title: self.title,
            tab_type: "ext_ws".to_string(),
            connected_at: None,
        }
    }
}
