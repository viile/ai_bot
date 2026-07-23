use crate::cursor::{run_cursor_agent_stream, run_cursor_agent_text};
use crate::models::{Bot, ChatEvent, Message};
use crate::store::SharedStore;
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const HISTORY_LIMIT: usize = 30;
/// Prefer at most this many speakers per user turn.
const MAX_SPEAKERS: usize = 3;
/// Typing indicator duration after reply is ready (ms).
const TYPING_DELAY_MIN_MS: u64 = 1000;
const TYPING_DELAY_MAX_MS: u64 = 2000;

#[derive(Debug, Deserialize)]
struct RouteDecision {
    #[serde(default)]
    ids: Vec<String>,
    #[serde(default)]
    speakers: Vec<RouteSpeaker>,
}

#[derive(Debug, Deserialize)]
struct RouteSpeaker {
    #[serde(default)]
    id: String,
    #[serde(default)]
    nickname: String,
    #[serde(default)]
    hint: String,
}

#[derive(Clone)]
struct SelectedBot {
    bot: Bot,
    /// Optional angle so parallel speakers stay differentiated.
    hint: Option<String>,
}

fn emit_event(app: &AppHandle, event: ChatEvent) {
    let _ = app.emit("chat-event", event);
}

fn format_history(messages: &[Message]) -> String {
    let start = messages.len().saturating_sub(HISTORY_LIMIT);
    messages[start..]
        .iter()
        .filter(|m| m.status != "error" || !m.content.is_empty())
        .map(|m| {
            let name = if m.sender_type == "user" {
                "用户"
            } else {
                m.nickname.as_str()
            };
            format!("[{name}]: {}", m.content)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_prompt(
    bot: &Bot,
    history_text: &str,
    user_content: &str,
    hint: Option<&str>,
    peers: &[(String, Option<String>)],
) -> String {
    let mut extra = String::new();
    if let Some(h) = hint.map(str::trim).filter(|s| !s.is_empty()) {
        extra.push_str(&format!("\n本轮发言侧重：{h}\n"));
    }
    if !peers.is_empty() {
        let lines: Vec<String> = peers
            .iter()
            .map(|(name, h)| match h.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
                Some(hh) => format!("- {name}（侧重：{hh}）"),
                None => format!("- {name}"),
            })
            .collect();
        extra.push_str(&format!(
            "\n同轮还有这些成员会发言（请避免重复他们的观点，从你自己的角色角度补充）：\n{}\n",
            lines.join("\n")
        ));
    }

    format!(
        "你正在一个多人群聊里扮演角色「{nick}」。\n\
身份设定：{persona}\n\
硬性规则：\n\
1. 只输出你作为该角色要发到群里的一句/几句回复正文；\n\
2. 不要输出角色名、前缀、引号、思考过程或对本指令的复述；\n\
3. 不要冒充其他成员，不要解释你是 AI；\n\
4. 若此轮你不想说话（与话题无关、懒得回、角色设定如此等），只输出一行：NO_REPLY\n\
   不要输出「不回话」「沉默」等说明文字，系统会直接跳过，群里不会出现任何消息。\n\
{extra}\n\
近期群聊记录：\n\
{history}\n\
\n\
用户刚说：{user}",
        nick = bot.nickname,
        persona = bot.persona,
        extra = extra,
        history = if history_text.is_empty() {
            "（暂无历史）"
        } else {
            history_text
        },
        user = user_content
    )
}

fn build_route_prompt(bots: &[Bot], history_text: &str, user_content: &str) -> String {
    let roster: String = bots
        .iter()
        .map(|b| {
            let persona = if b.persona.chars().count() > 120 {
                format!("{}…", b.persona.chars().take(120).collect::<String>())
            } else {
                b.persona.clone()
            };
            format!("- id={} | 昵称={} | 设定={}", b.id, b.nickname, persona)
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "你是群聊发言调度器。根据用户这句话和近期上下文，决定本轮最适合开口的机器人。\n\
目标：\n\
1. 优先选最相关的 1 人；只有话题明显需要多视角时才选 2–{max} 人；\n\
2. 不要为了热闹让所有人都回；避免多人说相似内容；\n\
3. 若几乎无人相关，仍选最不违和的 1 人；\n\
4. 为每位入选者给一句简短 hint（他们应从哪个角度说，彼此角度要不同）。\n\
\n\
只输出一段 JSON，不要 markdown，不要解释。格式：\n\
{{\"speakers\":[{{\"id\":\"机器人id\",\"hint\":\"角度\"}}]}}\n\
\n\
可选机器人：\n\
{roster}\n\
\n\
近期群聊：\n\
{history}\n\
\n\
用户刚说：{user}",
        max = MAX_SPEAKERS,
        roster = roster,
        history = if history_text.is_empty() {
            "（暂无历史）"
        } else {
            history_text
        },
        user = user_content
    )
}

fn extract_json_object(text: &str) -> Option<String> {
    let t = text.trim();
    let start = t.find('{')?;
    let end = t.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(t[start..=end].to_string())
}

fn resolve_bot<'a>(bots: &'a [Bot], id: &str, nickname: &str) -> Option<&'a Bot> {
    let id = id.trim();
    let nickname = nickname.trim();
    if !id.is_empty() {
        if let Some(b) = bots.iter().find(|b| b.id == id) {
            return Some(b);
        }
    }
    if !nickname.is_empty() {
        if let Some(b) = bots.iter().find(|b| b.nickname == nickname) {
            return Some(b);
        }
        if let Some(b) = bots
            .iter()
            .find(|b| b.nickname.contains(nickname) || nickname.contains(&b.nickname))
        {
            return Some(b);
        }
    }
    None
}

fn parse_route_selection(raw: &str, bots: &[Bot]) -> Vec<SelectedBot> {
    let Some(json) = extract_json_object(raw) else {
        return Vec::new();
    };
    let Ok(decision) = serde_json::from_str::<RouteDecision>(&json) else {
        return Vec::new();
    };

    let mut selected = Vec::new();
    let mut seen = HashSet::new();

    for sp in &decision.speakers {
        let Some(bot) = resolve_bot(bots, &sp.id, &sp.nickname) else {
            continue;
        };
        if !seen.insert(bot.id.clone()) {
            continue;
        }
        let hint = sp.hint.trim().to_string();
        selected.push(SelectedBot {
            bot: bot.clone(),
            hint: if hint.is_empty() { None } else { Some(hint) },
        });
        if selected.len() >= MAX_SPEAKERS {
            break;
        }
    }

    if selected.is_empty() {
        for id in &decision.ids {
            let Some(bot) = resolve_bot(bots, id, "") else {
                continue;
            };
            if !seen.insert(bot.id.clone()) {
                continue;
            }
            selected.push(SelectedBot {
                bot: bot.clone(),
                hint: None,
            });
            if selected.len() >= MAX_SPEAKERS {
                break;
            }
        }
    }

    selected
}

/// Pick who should speak this turn. Falls back to a single bot if routing fails.
async fn select_speakers(bots: &[Bot], history_text: &str, user_content: &str) -> Vec<SelectedBot> {
    if bots.is_empty() {
        return Vec::new();
    }
    if bots.len() == 1 {
        return vec![SelectedBot {
            bot: bots[0].clone(),
            hint: None,
        }];
    }

    let prompt = build_route_prompt(bots, history_text, user_content);
    let raw = match run_cursor_agent_text(&prompt, None, None).await {
        Ok(r) => r.text,
        Err(_) => match run_cursor_agent_stream(&prompt, None, None, |_| {}, |_| {}).await {
            Ok(r) => r.text,
            Err(_) => String::new(),
        },
    };

    let mut selected = parse_route_selection(&raw, bots);
    if selected.is_empty() {
        // Safe fallback: one speaker only, avoids everyone echoing the same point.
        selected.push(SelectedBot {
            bot: bots[0].clone(),
            hint: None,
        });
    }
    selected
}

/// True when the model chose to stay silent (or produced a no-reply placeholder).
fn is_no_reply(text: &str, nickname: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return true;
    }

    let compact: String = t.chars().filter(|c| !c.is_whitespace()).collect();
    let stripped = compact
        .trim_matches(|c: char| {
            matches!(
                c,
                '（' | '(' | '【' | '[' | '）' | ')' | '】' | ']' | '"' | '\'' | '“' | '”' | '*'
            )
        })
        .to_string();

    let lower = stripped.to_ascii_lowercase();
    if lower == "no_reply" || lower == "noreply" {
        return true;
    }

    const PHRASES: &[&str] = &["不回话", "不回复", "无需回复", "沉默", "跳过", "选择不回话"];
    if PHRASES.iter().any(|p| stripped == *p) {
        return true;
    }

    let named = format!("{nickname}不回话");
    if stripped == named || stripped == format!("{nickname}选择不回话") {
        return true;
    }

    // Short “xxx不回话” style placeholders only.
    stripped.ends_with("不回话") && stripped.chars().count() <= nickname.chars().count() + 10
}

fn typing_delay() -> Duration {
    let span = TYPING_DELAY_MAX_MS - TYPING_DELAY_MIN_MS;
    let jitter = (Uuid::new_v4().as_u128() % (span as u128 + 1)) as u64;
    Duration::from_millis(TYPING_DELAY_MIN_MS + jitter)
}

pub async fn handle_user_message(
    app: AppHandle,
    store: Arc<SharedStore>,
    group_id: String,
    content: String,
) -> Result<(), String> {
    let text = content.trim().to_string();
    if text.is_empty() {
        return Err("消息不能为空".into());
    }

    let (user_msg, bots, history_text) = {
        let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
        let user_msg = store.append_message(Message {
            id: Uuid::new_v4().to_string(),
            group_id: group_id.clone(),
            sender_type: "user".into(),
            sender_id: None,
            bot_id: None,
            nickname: "我".into(),
            avatar: Some("#1e3a5f".into()),
            content: text.clone(),
            created_at: Utc::now().to_rfc3339(),
            status: "done".into(),
        })?;

        let bots = store.list_bots(Some(&group_id))?;
        let history: Vec<Message> = store
            .list_messages(&group_id, HISTORY_LIMIT + 1)?
            .into_iter()
            .filter(|m| m.id != user_msg.id)
            .collect();
        let history_text = format_history(&history);
        (user_msg, bots, history_text)
    };

    emit_event(
        &app,
        ChatEvent {
            event_type: "message".into(),
            group_id: Some(group_id.clone()),
            message: Some(user_msg),
            bot_id: None,
            message_id: None,
            delta: None,
            content: None,
            error: None,
            action: None,
        },
    );

    if bots.is_empty() {
        return Ok(());
    }

    // Route first (silent): decide who should speak based on context.
    let speakers = select_speakers(&bots, &history_text, &text).await;
    let peer_meta: Vec<(String, Option<String>)> = speakers
        .iter()
        .map(|s| (s.bot.nickname.clone(), s.hint.clone()))
        .collect();

    let mut tasks = Vec::new();
    for selected in speakers {
        let app2 = app.clone();
        let store2 = Arc::clone(&store);
        let gid = group_id.clone();
        let hist = history_text.clone();
        let user = text.clone();
        let peers: Vec<(String, Option<String>)> = peer_meta
            .iter()
            .filter(|(n, _)| n != &selected.bot.nickname)
            .cloned()
            .collect();
        tasks.push(tokio::spawn(async move {
            reply_as_bot(app2, store2, gid, selected, hist, user, peers).await
        }));
    }

    for task in tasks {
        let _ = task.await;
    }
    Ok(())
}

async fn reply_as_bot(
    app: AppHandle,
    store: Arc<SharedStore>,
    group_id: String,
    selected: SelectedBot,
    history_text: String,
    user_content: String,
    peers: Vec<(String, Option<String>)>,
) -> Result<Option<Message>, String> {
    let bot = selected.bot;
    let prompt = build_prompt(
        &bot,
        &history_text,
        &user_content,
        selected.hint.as_deref(),
        &peers,
    );
    let store_meta = Arc::clone(&store);
    let bot_id_meta = bot.id.clone();

    // 1) Generate silently in the background — no UI events yet.
    let stream_result = run_cursor_agent_stream(
        &prompt,
        bot.model.as_deref(),
        bot.cursor_chat_id.as_deref(),
        |_delta| {},
        |chat_id| {
            if let Ok(store) = store_meta.lock() {
                let _ = store.set_bot_chat_id(&bot_id_meta, chat_id);
            }
        },
    )
    .await;

    let result = match stream_result {
        Ok(r) => r,
        Err(stream_err) => match run_cursor_agent_text(
            &prompt,
            bot.model.as_deref(),
            bot.cursor_chat_id.as_deref(),
        )
        .await
        {
            Ok(r) => r,
            Err(_) => {
                let err_msg = format!("（回复失败：{stream_err}）");
                let error_message = {
                    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
                    store.append_message(Message {
                        id: Uuid::new_v4().to_string(),
                        group_id: group_id.clone(),
                        sender_type: "bot".into(),
                        sender_id: None,
                        bot_id: Some(bot.id.clone()),
                        nickname: bot.nickname.clone(),
                        avatar: Some(bot.avatar.clone()),
                        content: err_msg,
                        created_at: Utc::now().to_rfc3339(),
                        status: "error".into(),
                    })?
                };
                emit_event(
                    &app,
                    ChatEvent {
                        event_type: "bot_error".into(),
                        group_id: Some(group_id),
                        message: Some(error_message.clone()),
                        bot_id: Some(bot.id),
                        message_id: Some(error_message.id.clone()),
                        delta: None,
                        content: None,
                        error: Some(stream_err),
                        action: None,
                    },
                );
                return Ok(Some(error_message));
            }
        },
    };

    if let Some(chat_id) = &result.chat_id {
        if let Ok(store) = store.lock() {
            let _ = store.set_bot_chat_id(&bot.id, chat_id);
        }
    }

    let final_text = result.text.trim().to_string();
    // Stay silent: no typing indicator, no chat bubble.
    if is_no_reply(&final_text, &bot.nickname) {
        return Ok(None);
    }

    let message_id = Uuid::new_v4().to_string();
    let bot_id = bot.id.clone();
    let created_at = Utc::now().to_rfc3339();

    // 2) Reply is ready — show typing for 1–2s (ephemeral, not persisted), then send.
    let typing = Message {
        id: message_id.clone(),
        group_id: group_id.clone(),
        sender_type: "bot".into(),
        sender_id: None,
        bot_id: Some(bot_id.clone()),
        nickname: bot.nickname.clone(),
        avatar: Some(bot.avatar.clone()),
        content: String::new(),
        created_at: created_at.clone(),
        status: "streaming".into(),
    };

    emit_event(
        &app,
        ChatEvent {
            event_type: "bot_typing".into(),
            group_id: Some(group_id.clone()),
            message: Some(typing.clone()),
            bot_id: Some(bot_id.clone()),
            message_id: Some(message_id.clone()),
            delta: None,
            content: None,
            error: None,
            action: None,
        },
    );
    emit_event(
        &app,
        ChatEvent {
            event_type: "message".into(),
            group_id: Some(group_id.clone()),
            message: Some(typing),
            bot_id: None,
            message_id: None,
            delta: None,
            content: None,
            error: None,
            action: None,
        },
    );

    tokio::time::sleep(typing_delay()).await;

    let done = {
        let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
        store.append_message(Message {
            id: message_id.clone(),
            group_id: group_id.clone(),
            sender_type: "bot".into(),
            sender_id: None,
            bot_id: Some(bot_id.clone()),
            nickname: bot.nickname.clone(),
            avatar: Some(bot.avatar.clone()),
            content: final_text,
            created_at,
            status: "done".into(),
        })?
    };

    emit_event(
        &app,
        ChatEvent {
            event_type: "bot_done".into(),
            group_id: Some(group_id),
            message: Some(done.clone()),
            bot_id: Some(bot_id),
            message_id: Some(message_id),
            delta: None,
            content: None,
            error: None,
            action: None,
        },
    );

    Ok(Some(done))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_bots() -> Vec<Bot> {
        vec![
            Bot {
                id: "a".into(),
                group_id: "g".into(),
                nickname: "产品经理".into(),
                avatar: "#1".into(),
                persona: "产品".into(),
                model: None,
                cursor_chat_id: None,
            },
            Bot {
                id: "b".into(),
                group_id: "g".into(),
                nickname: "工程师".into(),
                avatar: "#2".into(),
                persona: "工程".into(),
                model: None,
                cursor_chat_id: None,
            },
        ]
    }

    #[test]
    fn parses_speakers_json() {
        let bots = sample_bots();
        let raw = r#"{"speakers":[{"id":"b","hint":"谈可行性"}]}"#;
        let sel = parse_route_selection(raw, &bots);
        assert_eq!(sel.len(), 1);
        assert_eq!(sel[0].bot.id, "b");
        assert_eq!(sel[0].hint.as_deref(), Some("谈可行性"));
    }

    #[test]
    fn parses_ids_fallback() {
        let bots = sample_bots();
        let raw = r#"这里是结果 {"ids":["a","b","x"]} 完毕"#;
        let sel = parse_route_selection(raw, &bots);
        assert_eq!(sel.len(), 2);
    }
}
