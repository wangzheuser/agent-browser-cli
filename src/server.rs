use crate::html;
use crate::protocol::{DriverState, ExecResult, Session, TabInfo, WsIncoming};
use anyhow::{anyhow, Result};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime};
use tokio::sync::{mpsc, oneshot, Mutex, Notify};
use tower_http::cors::CorsLayer;
use uuid::Uuid;

const HOST: &str = "127.0.0.1";
const WS_PORT: u16 = 18765;
const API_PORT: u16 = 18767;

#[derive(Clone)]
pub struct AppState {
    driver: Arc<Mutex<DriverState>>,
    started_at: SystemTime,
    last_activity: Arc<Mutex<Instant>>,
    shutdown: mpsc::UnboundedSender<()>,
    sessions_ready: Arc<Notify>,
}

#[derive(Debug, Deserialize)]
struct ScanRequest {
    #[serde(default)]
    tabs_only: bool,
    switch_tab_id: Option<String>,
    #[serde(default)]
    text_only: bool,
}

#[derive(Debug, Deserialize)]
struct ExecRequest {
    #[serde(default)]
    script: String,
    switch_tab_id: Option<String>,
    #[serde(default)]
    no_monitor: bool,
    wait_js: Option<String>,
    #[serde(default = "default_wait_timeout")]
    wait_timeout: f64,
    #[serde(default = "default_wait_interval")]
    wait_interval: f64,
}

#[derive(Debug, Deserialize)]
struct OpenRequest {
    url: String,
    #[serde(default = "default_active")]
    active: bool,
    switch_tab_id: Option<String>,
}

fn default_active() -> bool {
    true
}

fn default_wait_timeout() -> f64 {
    3.0
}

fn default_wait_interval() -> f64 {
    0.1
}

pub async fn run_daemon() -> Result<()> {
    let (shutdown_tx, mut shutdown_rx) = mpsc::unbounded_channel::<()>();
    let state = AppState {
        driver: Arc::new(Mutex::new(DriverState::default())),
        started_at: SystemTime::now(),
        last_activity: Arc::new(Mutex::new(Instant::now())),
        shutdown: shutdown_tx,
        sessions_ready: Arc::new(Notify::new()),
    };

    let ws_state = state.clone();
    tokio::spawn(async move {
        if let Err(err) = run_ws_server(ws_state).await {
            eprintln!("WebSocket 服务异常: {err:?}");
        }
    });

    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/tabs", get(tabs))
        .route("/scan", post(scan))
        .route("/exec", post(exec))
        .route("/open", post(open_tab))
        .route("/shutdown", post(shutdown))
        .with_state(state.clone())
        .layer(CorsLayer::permissive());

    let addr: SocketAddr = format!("{HOST}:{API_PORT}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("agent-browser-cli rust server listening on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
        })
        .await?;
    Ok(())
}

async fn run_ws_server(state: AppState) -> Result<()> {
    let app = Router::new().route("/", get(ws_handler)).with_state(state);
    let addr: SocketAddr = format!("{HOST}:{WS_PORT}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("WebSocket server running on ws://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();
    let mut registered_ids: Vec<String> = Vec::new();
    loop {
        tokio::select! {
            Some(outgoing) = rx.recv() => {
                if socket.send(Message::Text(outgoing)).await.is_err() {
                    break;
                }
            }
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break; };
                let Message::Text(text) = msg else { continue; };
                if text.trim() == r#"{"type":"ping"}"# {
                    continue;
                }
                match serde_json::from_str::<WsIncoming>(&text) {
                    Ok(incoming) => handle_ws_message(incoming, &state, tx.clone(), &mut registered_ids).await,
                    Err(err) => eprintln!("WebSocket 消息解析失败: {err}: {text}"),
                }
            }
        }
    }
    let mut driver = state.driver.lock().await;
    for id in registered_ids {
        if let Some(session) = driver.sessions.get_mut(&id) {
            session.disconnected_at = Some(Instant::now());
        }
    }
}

async fn handle_ws_message(
    incoming: WsIncoming,
    state: &AppState,
    sender: mpsc::UnboundedSender<String>,
    registered_ids: &mut Vec<String>,
) {
    let mut driver = state.driver.lock().await;
    match incoming {
        WsIncoming::ExtReady { tabs } | WsIncoming::TabsUpdate { tabs } => {
            let current: std::collections::HashSet<String> = tabs
                .iter()
                .map(|t| t.id.to_string().trim_matches('"').to_string())
                .collect();
            for session in driver.sessions.values_mut() {
                if session.info.tab_type == "ext_ws" && !current.contains(&session.info.id) {
                    session.disconnected_at = Some(Instant::now());
                }
            }
            for tab in tabs {
                let info = tab.into_tab_info();
                if !registered_ids.contains(&info.id) {
                    registered_ids.push(info.id.clone());
                }
                driver.latest_session_id = Some(info.id.clone());
                if driver.default_session_id.is_none() {
                    driver.default_session_id = Some(info.id.clone());
                }
                driver.sessions.insert(
                    info.id.clone(),
                    Session {
                        info,
                        sender: sender.clone(),
                        disconnected_at: None,
                    },
                );
            }
            state.sessions_ready.notify_waiters();
        }
        WsIncoming::Ack { id } => {
            driver.acked.insert(id.clone());
            if let Some(pending) = driver.pending.get_mut(&id) {
                pending.delivered_at = Some(Instant::now());
            }
        }
        WsIncoming::Result {
            id,
            result,
            new_tabs,
        } => {
            if let Some(pending) = driver.pending.remove(&id) {
                let _ = pending.tx.send(Ok(ExecResult {
                    data: Some(result),
                    result: None,
                    closed: None,
                    new_tabs,
                }));
            }
            driver.acked.remove(&id);
            driver.active_exec_sessions.remove(&id);
        }
        WsIncoming::Error {
            id,
            error,
            new_tabs,
        } => {
            if let Some(pending) = driver.pending.remove(&id) {
                let mut value = json!({ "error": error });
                if let Some(tabs) = new_tabs {
                    value["newTabs"] = tabs;
                }
                let _ = pending.tx.send(Err(anyhow!(value.to_string())));
            }
            driver.acked.remove(&id);
            driver.active_exec_sessions.remove(&id);
        }
        WsIncoming::Other => {}
    }
}

async fn root() -> &'static str {
    "agent-browser-cli"
}

async fn health(State(state): State<AppState>) -> Json<Value> {
    touch(&state).await;
    let ready = !active_tabs(&state, false).await.is_empty();
    let uptime = state
        .started_at
        .elapsed()
        .map(|d| d.as_secs_f64())
        .unwrap_or_default();
    Json(json!({ "ok": ready, "running": true, "ready": ready, "uptime": uptime, "ttl": 300 }))
}

async fn tabs(State(state): State<AppState>) -> Json<Value> {
    touch(&state).await;
    let tabs = active_tabs(&state, true).await;
    Json(
        json!({ "ok": true, "result": { "status": "success", "metadata": { "tabs_count": tabs.len(), "tabs": tabs } } }),
    )
}

async fn scan(State(state): State<AppState>, Json(req): Json<ScanRequest>) -> Json<Value> {
    touch(&state).await;
    let result = scan_page(&state, req.tabs_only, req.switch_tab_id, req.text_only).await;
    Json(match result {
        Ok(value) => json!({ "ok": true, "result": value }),
        Err(err) => json!({ "ok": false, "error": err.to_string() }),
    })
}

async fn exec(State(state): State<AppState>, Json(req): Json<ExecRequest>) -> Json<Value> {
    touch(&state).await;
    let script = if req.wait_js.is_some() && !is_extension_json(&req.script) {
        wrap_script_with_wait(
            &req.script,
            req.wait_js.as_deref().unwrap_or_default(),
            req.wait_timeout,
            req.wait_interval,
        )
    } else {
        req.script.clone()
    };
    let result = execute_page_js(&state, &script, req.switch_tab_id, req.no_monitor).await;
    Json(match result {
        Ok(value) => {
            json!({ "ok": true, "result": value, "combined_wait": req.wait_js.is_some() && !is_extension_json(&req.script) })
        }
        Err(err) => json!({ "ok": false, "error": err.to_string() }),
    })
}

async fn open_tab(State(state): State<AppState>, Json(req): Json<OpenRequest>) -> Json<Value> {
    touch(&state).await;
    let payload = json!({ "cmd": "openTab", "url": normalize_url(&req.url), "active": req.active })
        .to_string();
    let result = execute_page_js(&state, &payload, req.switch_tab_id, true).await;
    Json(match result {
        Ok(value) => json!({ "ok": true, "result": value }),
        Err(err) => json!({ "ok": false, "error": err.to_string() }),
    })
}

async fn shutdown(State(state): State<AppState>) -> Json<Value> {
    touch(&state).await;
    let _ = state.shutdown.send(());
    Json(json!({ "ok": true, "status": "shutdown_requested" }))
}

async fn active_tabs(state: &AppState, wait_ready: bool) -> Vec<TabInfo> {
    if wait_ready {
        wait_for_sessions(state, Duration::from_secs(5)).await;
    }
    let driver = state.driver.lock().await;
    driver
        .sessions
        .values()
        .filter(|s| s.is_active())
        .map(|s| {
            let mut info = s.info.clone();
            if info.url.len() > 50 {
                info.url = format!("{}...", info.url.chars().take(50).collect::<String>());
            }
            info
        })
        .collect()
}

async fn wait_for_sessions(state: &AppState, timeout: Duration) -> bool {
    if has_active_sessions(state).await {
        return true;
    }
    let deadline = Instant::now() + timeout;
    loop {
        let now = Instant::now();
        if now >= deadline {
            return has_active_sessions(state).await;
        }
        let remaining = deadline.saturating_duration_since(now);
        tokio::select! {
            _ = state.sessions_ready.notified() => {
                if has_active_sessions(state).await {
                    return true;
                }
            }
            _ = tokio::time::sleep(remaining.min(Duration::from_millis(200))) => {
                if has_active_sessions(state).await {
                    return true;
                }
            }
        }
    }
}

async fn has_active_sessions(state: &AppState) -> bool {
    let driver = state.driver.lock().await;
    driver.sessions.values().any(|s| s.is_active())
}

async fn scan_page(
    state: &AppState,
    tabs_only: bool,
    switch_tab_id: Option<String>,
    text_only: bool,
) -> Result<Value> {
    if let Some(tab_id) = switch_tab_id {
        state.driver.lock().await.default_session_id = Some(tab_id);
    }
    let tabs = active_tabs(state, true).await;
    if tabs.is_empty() {
        return Ok(
            json!({ "status": "error", "msg": "没有可用的浏览器标签页，查L3记忆分析原因。" }),
        );
    }
    let default_session_id = state.driver.lock().await.default_session_id.clone();
    let mut result = json!({
        "status": "success",
        "metadata": {
            "tabs_count": tabs.len(),
            "tabs": tabs,
            "active_tab": default_session_id,
        }
    });
    if !tabs_only {
        let content = get_html(state, true, 35000, text_only).await?;
        result["content"] = Value::String(content);
    }
    Ok(result)
}

async fn execute_page_js(
    state: &AppState,
    script: &str,
    switch_tab_id: Option<String>,
    no_monitor: bool,
) -> Result<Value> {
    if let Some(tab_id) = switch_tab_id {
        state.driver.lock().await.default_session_id = Some(tab_id);
    }
    let before = if no_monitor {
        None
    } else {
        get_html(state, false, 9_999_999, false).await.ok()
    };
    let before_tabs: std::collections::HashSet<String> = active_tabs(state, true)
        .await
        .into_iter()
        .map(|t| t.id)
        .collect();
    let response = execute_raw_js(state, script, Duration::from_secs(15)).await?;
    let mut result = json!({
        "status": "success",
        "js_return": response.data.or(response.result).unwrap_or(Value::Null),
        "tab_id": state.driver.lock().await.default_session_id,
    });
    if let Some(tabs) = response.new_tabs {
        result["newTabs"] = tabs;
    } else {
        let after_tabs = active_tabs(state, false).await;
        let new_tabs: Vec<_> = after_tabs
            .into_iter()
            .filter(|t| !before_tabs.contains(&t.id))
            .map(|t| json!({ "id": t.id, "url": t.url }))
            .collect();
        if !new_tabs.is_empty() {
            result["newTabs"] = json!(new_tabs);
        }
    }
    if !no_monitor {
        if let Some(before_html) = before {
            if let Ok(current_html) = get_html(state, false, 9_999_999, false).await {
                result["change"] = html::changed_elements(&before_html, &current_html);
            }
        }
    }
    Ok(result)
}

async fn execute_raw_js(state: &AppState, code: &str, timeout: Duration) -> Result<ExecResult> {
    let (session_id, sender) = {
        wait_for_sessions(state, Duration::from_secs(5)).await;
        let driver = state.driver.lock().await;
        let session_id = driver
            .default_session_id
            .clone()
            .or_else(|| driver.latest_session_id.clone())
            .ok_or_else(|| anyhow!("没有可用的浏览器标签页，查L3记忆分析原因。"))?;
        let session = driver
            .sessions
            .get(&session_id)
            .filter(|s| s.is_active())
            .ok_or_else(|| anyhow!("会话ID {session_id} 未连接"))?;
        (session_id, session.sender.clone())
    };
    let exec_id = Uuid::new_v4().to_string();
    let payload = json!({ "id": exec_id, "code": code, "tabId": session_id.parse::<i64>().unwrap_or_default() }).to_string();
    let (tx, rx) = oneshot::channel();
    {
        let mut driver = state.driver.lock().await;
        driver.pending.insert(
            exec_id.clone(),
            crate::protocol::PendingExec {
                delivered_at: None,
                tx,
            },
        );
        driver
            .active_exec_sessions
            .insert(exec_id.clone(), session_id.clone());
    }
    sender
        .send(payload)
        .map_err(|_| anyhow!("浏览器扩展连接已断开"))?;
    match tokio::time::timeout(timeout, rx).await {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => Err(anyhow!("执行结果通道已关闭")),
        Err(_) => {
            let mut driver = state.driver.lock().await;
            let acked = driver.acked.remove(&exec_id);
            driver.pending.remove(&exec_id);
            driver.active_exec_sessions.remove(&exec_id);
            if acked {
                Ok(ExecResult {
                    data: None,
                    result: Some(json!(format!(
                        "No response data in {}s (ACK received, script may still be running)",
                        timeout.as_secs()
                    ))),
                    closed: None,
                    new_tabs: None,
                })
            } else {
                Ok(ExecResult {
                    data: None,
                    result: Some(json!(format!(
                        "No response data in {}s (no ACK, script may not have been delivered)",
                        timeout.as_secs()
                    ))),
                    closed: None,
                    new_tabs: None,
                })
            }
        }
    }
}

async fn get_html(
    state: &AppState,
    cutlist: bool,
    maxchars: usize,
    text_only: bool,
) -> Result<String> {
    let opt = html::js_opt_html();
    let page_script = format!(
        "{opt}\nreturn optHTML({});",
        if text_only { "true" } else { "false" }
    );
    let response = execute_raw_js(state, &page_script, Duration::from_secs(30)).await?;
    let mut page = response
        .data
        .unwrap_or(Value::Null)
        .as_str()
        .unwrap_or_default()
        .to_string();
    if text_only {
        return Ok(clean_text(&page));
    }
    page = html::optimize_html_for_tokens(&page);
    if cutlist {
        let list_script = format!(
            "{}\nreturn findMainList(document.body);",
            html::js_find_main_list()
        );
        let _ = execute_raw_js(state, &list_script, Duration::from_secs(10)).await;
    }
    if page.len() > maxchars {
        page = html::smart_truncate(page, maxchars);
    }
    Ok(page)
}

fn clean_text(input: &str) -> String {
    input
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_extension_json(script: &str) -> bool {
    let trimmed = script.trim();
    trimmed.starts_with('{')
        && serde_json::from_str::<Value>(trimmed)
            .ok()
            .and_then(|v| v.get("cmd").cloned())
            .is_some()
}

fn wrap_script_with_wait(script: &str, wait_js: &str, timeout: f64, interval: f64) -> String {
    format!(
        r#"
const __agentBrowserMain = {script_json};
const __agentBrowserWait = {wait_json};
const __agentBrowserTimeoutMs = {timeout_ms};
const __agentBrowserIntervalMs = {interval_ms};
const AsyncFunction = Object.getPrototypeOf(async function(){{}}).constructor;
const __runUser = async (code) => {{
  const trimmed = String(code || '').trim();
  if (!trimmed) return undefined;
  if (/^return\b/.test(trimmed)) return await (new AsyncFunction(trimmed))();
  try {{
    const value = eval(trimmed);
    return value instanceof Promise ? await value : value;
  }} catch (e) {{
    if (e instanceof SyntaxError && (/return/i.test(e.message) || /await/i.test(e.message))) {{
      return await (new AsyncFunction(trimmed))();
    }}
    throw e;
  }}
}};
const __mainResult = await __runUser(__agentBrowserMain);
let __matched = false;
let __waitValue = undefined;
let __waitError = null;
const __deadline = Date.now() + __agentBrowserTimeoutMs;
while (true) {{
  try {{
    __waitValue = await __runUser(__agentBrowserWait);
    __waitError = null;
    if (__waitValue) {{ __matched = true; break; }}
  }} catch (e) {{
    __waitError = e.message || String(e);
  }}
  if (Date.now() >= __deadline) break;
  await new Promise(resolve => setTimeout(resolve, __agentBrowserIntervalMs));
}}
return {{ result: __mainResult, wait: {{ ok: __matched, matched: __matched, value: __waitValue, error: __waitError }} }};
"#,
        script_json = serde_json::to_string(script).unwrap_or_else(|_| "\"\"".to_string()),
        wait_json = serde_json::to_string(wait_js).unwrap_or_else(|_| "\"\"".to_string()),
        timeout_ms = (timeout.max(0.0) * 1000.0) as u64,
        interval_ms = ((interval.max(0.02)) * 1000.0) as u64,
    )
}

fn normalize_url(url: &str) -> String {
    if url.starts_with("http://") || url.starts_with("https://") {
        url.to_string()
    } else {
        format!("https://{url}")
    }
}

async fn touch(state: &AppState) {
    *state.last_activity.lock().await = Instant::now();
}

#[allow(dead_code)]
async fn not_found() -> (StatusCode, Json<Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(json!({ "ok": false, "error": "not found" })),
    )
}
