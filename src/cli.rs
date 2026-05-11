use crate::server;
use anyhow::{anyhow, Result};
use clap::{Args, Parser, Subcommand};
use fs2::FileExt;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::env;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

const HOST: &str = "127.0.0.1";
const PORT: u16 = 18767;

#[derive(Debug, Parser)]
#[command(name = "agent-browser-cli")]
pub struct Cli {
    #[command(subcommand)]
    command: CommandKind,
}

#[derive(Debug, Subcommand)]
enum CommandKind {
    Tabs,
    Scan(ScanArgs),
    Exec(ExecArgs),
    Open(OpenArgs),
    #[command(name = "new-tab")]
    NewTab(OpenArgs),
    Status,
    Stop,
    Restart,
    Daemon,
}

#[derive(Debug, Args)]
struct ScanArgs {
    #[arg(long)]
    tab: Option<String>,
    #[arg(long)]
    tabs_only: bool,
    #[arg(long)]
    text_only: bool,
    #[arg(long, default_value_t = 60.0)]
    timeout: f64,
}

#[derive(Debug, Args)]
struct ExecArgs {
    #[arg(default_value = "")]
    script: String,
    #[arg(long)]
    file: Option<PathBuf>,
    #[arg(long)]
    tab: Option<String>,
    #[arg(long)]
    monitor: bool,
    #[arg(long)]
    wait_js: Option<String>,
    #[arg(long, default_value_t = 3.0)]
    wait_timeout: f64,
    #[arg(long, default_value_t = 0.1)]
    wait_interval: f64,
    #[arg(long, default_value_t = 60.0)]
    timeout: f64,
}

#[derive(Debug, Args)]
struct OpenArgs {
    url: String,
    #[arg(long)]
    background: bool,
    #[arg(long)]
    tab: Option<String>,
    #[arg(long, default_value_t = 30.0)]
    timeout: f64,
}

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        CommandKind::Daemon => {
            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(server::run_daemon())
        }
        CommandKind::Tabs => {
            ensure_server()?;
            print_json(request("GET", "/tabs", None, 30.0)?);
            Ok(())
        }
        CommandKind::Scan(args) => {
            ensure_server()?;
            print_json(request(
                "POST",
                "/scan",
                Some(json!({
                    "tabs_only": args.tabs_only,
                    "text_only": args.text_only,
                    "switch_tab_id": args.tab,
                })),
                args.timeout,
            )?);
            Ok(())
        }
        CommandKind::Exec(args) => {
            ensure_server()?;
            let script = if let Some(file) = args.file {
                std::fs::read_to_string(file)?
            } else {
                args.script
            };
            print_json(request(
                "POST",
                "/exec",
                Some(json!({
                    "script": script,
                    "switch_tab_id": args.tab,
                    "no_monitor": !args.monitor,
                    "wait_js": args.wait_js,
                    "wait_timeout": args.wait_timeout,
                    "wait_interval": args.wait_interval,
                })),
                args.timeout,
            )?);
            Ok(())
        }
        CommandKind::Open(args) | CommandKind::NewTab(args) => {
            ensure_server()?;
            print_json(request(
                "POST",
                "/open",
                Some(json!({
                    "url": args.url,
                    "active": !args.background,
                    "switch_tab_id": args.tab,
                })),
                args.timeout,
            )?);
            Ok(())
        }
        CommandKind::Status => {
            match request("GET", "/health", None, 1.0) {
                Ok(value) => print_json(value),
                Err(_) => print_json(json!({ "ok": true, "running": false })),
            }
            Ok(())
        }
        CommandKind::Stop => {
            match request("POST", "/shutdown", Some(json!({})), 3.0) {
                Ok(value) => print_json(value),
                Err(_) => print_json(json!({ "ok": true, "status": "not_running" })),
            }
            Ok(())
        }
        CommandKind::Restart => {
            let _ = request("POST", "/shutdown", Some(json!({})), 3.0);
            wait_server_stopped(Duration::from_secs(5));
            ensure_server()?;
            print_json(request("GET", "/health", None, 3.0)?);
            Ok(())
        }
    }
}

fn request(method: &str, path: &str, payload: Option<Value>, timeout_secs: f64) -> Result<Value> {
    let client = Client::builder()
        .timeout(Duration::from_secs_f64(timeout_secs.max(0.1)))
        .build()?;
    let url = format!("http://{HOST}:{PORT}{path}");
    let response = match method {
        "GET" => client.get(url).send()?,
        "POST" => client
            .post(url)
            .json(&payload.unwrap_or_else(|| json!({})))
            .send()?,
        _ => return Err(anyhow!("不支持的 HTTP 方法: {method}")),
    };
    Ok(response.json()?)
}

fn is_server_alive() -> bool {
    request("GET", "/health", None, 1.0)
        .ok()
        .and_then(|v| v.get("running").and_then(Value::as_bool))
        .unwrap_or(false)
}

fn ensure_server() -> Result<()> {
    if is_server_alive() {
        return Ok(());
    }
    let lock_path = project_dir().join(".agent-browser-cli.lock");
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(lock_path)?;
    lock.lock_exclusive()?;
    let result = ensure_server_locked();
    let _ = lock.unlock();
    result
}

fn ensure_server_locked() -> Result<()> {
    if is_server_alive() {
        return Ok(());
    }
    start_server()?;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        if is_server_alive() {
            return Ok(());
        }
        sleep(Duration::from_millis(200));
    }
    Err(anyhow!(
        "agent-browser-cli server 启动超时，查看 .agent-browser-cli.log"
    ))
}

fn start_server() -> Result<()> {
    let exe = env::current_exe()?;
    let log_path = project_dir().join(".agent-browser-cli.log");
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    let log_err = log.try_clone()?;
    let mut command = Command::new(exe);
    command
        .arg("daemon")
        .current_dir(project_dir())
        .stdin(Stdio::null())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        // 后台 daemon 必须脱离当前终端会话，否则 CLI 退出后子进程会被一起回收。
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
    }
    command.spawn()?;
    Ok(())
}

fn wait_server_stopped(timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if !is_server_alive() {
            return true;
        }
        sleep(Duration::from_millis(100));
    }
    !is_server_alive()
}

fn project_dir() -> PathBuf {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(PathBuf::from))
        .or_else(|| env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn print_json(value: Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_string())
    );
}
