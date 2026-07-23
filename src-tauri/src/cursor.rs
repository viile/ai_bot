use crate::models::CursorStatus;
use std::env;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::time::timeout;

#[derive(Clone)]
pub struct CursorConfig {
    pub binary: PathBuf,
    pub model: Option<String>,
    pub timeout_ms: u64,
    pub api_key: Option<String>,
}

pub fn load_config() -> Option<CursorConfig> {
    let binary = env::var("CURSOR_AGENT_BIN")
        .ok()
        .map(PathBuf::from)
        .or_else(|| which::which("cursor-agent").ok())
        .or_else(|| {
            dirs::home_dir().map(|h| h.join(".local/bin/cursor-agent"))
                .filter(|p| p.exists())
        })?;

    if !binary.exists() {
        return None;
    }

    let model = env::var("CURSOR_MODEL").ok().filter(|s| !s.trim().is_empty());
    let timeout_ms = env::var("CURSOR_TIMEOUT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120_000);
    let api_key = env::var("CURSOR_API_KEY")
        .ok()
        .filter(|s| !s.trim().is_empty());

    Some(CursorConfig {
        binary,
        model,
        timeout_ms,
        api_key,
    })
}

pub async fn get_status() -> CursorStatus {
    let Some(config) = load_config() else {
        return CursorStatus {
            available: false,
            logged_in: false,
            binary: None,
            model: None,
            message: "未找到 cursor-agent，请安装或设置 CURSOR_AGENT_BIN".into(),
        };
    };

    if config.api_key.is_some() {
        return CursorStatus {
            available: true,
            logged_in: true,
            binary: Some(config.binary.display().to_string()),
            model: config.model,
            message: "已通过 CURSOR_API_KEY 鉴权".into(),
        };
    }

    match timeout(
        Duration::from_secs(10),
        Command::new(&config.binary).arg("status").output(),
    )
    .await
    {
        Ok(Ok(out)) => {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            let logged_in = out.status.success() && !combined.contains("Not logged in");
            CursorStatus {
                available: true,
                logged_in,
                binary: Some(config.binary.display().to_string()),
                model: config.model,
                message: if logged_in {
                    "cursor-agent 已登录".into()
                } else {
                    "cursor-agent 未登录，请运行 cursor-agent login".into()
                },
            }
        }
        _ => CursorStatus {
            available: false,
            logged_in: false,
            binary: Some(config.binary.display().to_string()),
            model: config.model,
            message: "无法检查 cursor-agent 状态".into(),
        },
    }
}

fn strip_fences(text: &str) -> String {
    let mut cleaned = text.trim().to_string();
    if cleaned.starts_with("```") {
        if let Some(pos) = cleaned.find('\n') {
            cleaned = cleaned[pos + 1..].to_string();
        }
    }
    if cleaned.ends_with("```") {
        cleaned = cleaned.trim_end_matches("```").to_string();
    }
    cleaned.trim().to_string()
}

fn extract_assistant_text(obj: &serde_json::Value) -> String {
    let Some(content) = obj.pointer("/message/content") else {
        return String::new();
    };
    if let Some(s) = content.as_str() {
        return s.to_string();
    }
    let Some(arr) = content.as_array() else {
        return String::new();
    };
    arr.iter()
        .map(|p| {
            if let Some(s) = p.as_str() {
                s.to_string()
            } else if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                p.get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                p.get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string()
            }
        })
        .collect()
}

pub struct AgentResult {
    pub text: String,
    pub chat_id: Option<String>,
}

pub async fn run_cursor_agent_stream<F, G>(
    prompt: &str,
    model: Option<&str>,
    resume_chat_id: Option<&str>,
    mut on_delta: F,
    mut on_meta: G,
) -> Result<AgentResult, String>
where
    F: FnMut(&str),
    G: FnMut(&str),
{
    let config = load_config().ok_or_else(|| "未找到 cursor-agent".to_string())?;
    let mut args = vec![
        "--print".into(),
        "--mode".into(),
        "ask".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--stream-partial-output".into(),
        "--trust".into(),
    ];

    let effective_model = model
        .map(|s| s.to_string())
        .or_else(|| config.model.clone());
    if let Some(m) = &effective_model {
        args.push("--model".into());
        args.push(m.clone());
    }
    if let Some(id) = resume_chat_id {
        args.push("--resume".into());
        args.push(id.to_string());
    }
    if let Some(key) = &config.api_key {
        args.push("--api-key".into());
        args.push(key.clone());
    }
    args.push(prompt.to_string());

    let mut child = Command::new(&config.binary)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("启动 cursor-agent 失败: {e}"))?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取 cursor-agent stdout".to_string())?;
    let mut reader = BufReader::new(stdout).lines();

    let mut full_text = String::new();
    let mut chat_id = resume_chat_id.map(|s| s.to_string());

    let run = async {
        while let Some(line) = reader
            .next_line()
            .await
            .map_err(|e| format!("读取输出失败: {e}"))?
        {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                continue;
            };

            if let Some(sid) = obj.get("session_id").and_then(|v| v.as_str()) {
                if chat_id.as_deref() != Some(sid) {
                    chat_id = Some(sid.to_string());
                    on_meta(sid);
                }
            }

            let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if typ == "user" || typ == "thinking" {
                continue;
            }

            if typ == "assistant" {
                let text = extract_assistant_text(&obj);
                if text.is_empty() {
                    continue;
                }
                if !full_text.is_empty() && text.starts_with(&full_text) {
                    let next = text[full_text.len()..].to_string();
                    if !next.is_empty() {
                        full_text = text;
                        on_delta(&next);
                    }
                } else if !full_text.ends_with(&text) {
                    full_text.push_str(&text);
                    on_delta(&text);
                }
                continue;
            }

            if typ == "result" {
                if let Some(sid) = obj.get("session_id").and_then(|v| v.as_str()) {
                    if chat_id.as_deref() != Some(sid) {
                        chat_id = Some(sid.to_string());
                        on_meta(sid);
                    }
                }
                if let Some(result) = obj.get("result").and_then(|v| v.as_str()) {
                    let result = result.to_string();
                    if full_text.is_empty() {
                        full_text = result.clone();
                        on_delta(&result);
                    } else if result.starts_with(&full_text) {
                        let next = result[full_text.len()..].to_string();
                        if !next.is_empty() {
                            full_text = result;
                            on_delta(&next);
                        }
                    } else {
                        full_text = result;
                    }
                }
            }
        }

        let status = child
            .wait()
            .await
            .map_err(|e| format!("等待 cursor-agent 失败: {e}"))?;
        if !status.success() && full_text.is_empty() {
            return Err(format!(
                "cursor-agent 调用失败 (exit {:?})",
                status.code()
            ));
        }
        Ok(())
    };

    timeout(Duration::from_millis(config.timeout_ms), run)
        .await
        .map_err(|_| format!("cursor-agent 超时 (>{}ms)", config.timeout_ms))?
        .map_err(|e: String| e)?;

    let text = strip_fences(&full_text);
    if text.is_empty() {
        return Err("cursor-agent 没有返回内容".into());
    }
    Ok(AgentResult { text, chat_id })
}

pub async fn run_cursor_agent_text(
    prompt: &str,
    model: Option<&str>,
    resume_chat_id: Option<&str>,
) -> Result<AgentResult, String> {
    let config = load_config().ok_or_else(|| "未找到 cursor-agent".to_string())?;
    let mut args = vec![
        "--print".into(),
        "--mode".into(),
        "ask".into(),
        "--output-format".into(),
        "text".into(),
        "--trust".into(),
    ];
    let effective_model = model
        .map(|s| s.to_string())
        .or_else(|| config.model.clone());
    if let Some(m) = &effective_model {
        args.push("--model".into());
        args.push(m.clone());
    }
    if let Some(id) = resume_chat_id {
        args.push("--resume".into());
        args.push(id.to_string());
    }
    if let Some(key) = &config.api_key {
        args.push("--api-key".into());
        args.push(key.clone());
    }
    args.push(prompt.to_string());

    let out = timeout(
        Duration::from_millis(config.timeout_ms),
        Command::new(&config.binary).args(&args).output(),
    )
    .await
    .map_err(|_| format!("cursor-agent 超时 (>{}ms)", config.timeout_ms))?
    .map_err(|e| format!("启动 cursor-agent 失败: {e}"))?;

    if !out.status.success() {
        return Err(format!(
            "cursor-agent 调用失败 (exit {:?}): {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }

    let text = strip_fences(&String::from_utf8_lossy(&out.stdout));
    if text.is_empty() {
        return Err("cursor-agent 没有返回任何内容".into());
    }
    Ok(AgentResult {
        text,
        chat_id: resume_chat_id.map(|s| s.to_string()),
    })
}
