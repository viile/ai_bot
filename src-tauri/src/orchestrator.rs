use crate::cursor::{run_cursor_agent_stream, run_cursor_agent_text};
use crate::models::{Bot, ChatEvent, Message};
use crate::store::SharedStore;
use crate::turns::{cancelled, TurnRegistry};
use chrono::Utc;
use serde::Deserialize;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

const HISTORY_LIMIT: usize = 30;
/// Prefer at most this many speakers per turn (user reply or crosstalk round).
const MAX_SPEAKERS: usize = 3;
/// Bot-to-bot crosstalk rounds after the initial reply.
const CHAIN_ROUNDS_MIN: usize = 1;
const CHAIN_ROUNDS_MAX: usize = 3;
/// Typing indicator duration after reply is ready (ms).
const TYPING_DELAY_MIN_MS: u64 = 1000;
const TYPING_DELAY_MAX_MS: u64 = 2000;

#[derive(Clone)]
enum TurnKind {
    /// Respond to the human user.
    User { content: String },
    /// Continue a deeper dialogue with another bot's latest line.
    CrossTalk {
        from_name: String,
        from_content: String,
    },
}

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
        .filter(|m| m.status != "recalled")
        .filter(|m| m.status != "error" || !m.content.is_empty())
        .map(|m| format!("[{}]: {}", m.nickname, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_prompt(
    bot: &Bot,
    history_text: &str,
    turn: &TurnKind,
    hint: Option<&str>,
    peers: &[(String, Option<String>)],
    user_nickname: &str,
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

    let turn_block = match turn {
        TurnKind::User { content } => format!(
            "真人用户「{user}」刚说：{content}\n\
若要专门对他说话，正文以「@{user}」开头；他 @ 了你时请直接回应。",
            user = user_nickname,
            content = content
        ),
        TurnKind::CrossTalk {
            from_name,
            from_content,
        } => format!(
            "这是群内成员互聊回合。\n\
「{from_name}」刚说：{from_content}\n\
请你以自己的角色接话：可以追问、补充、温和反驳或把话题往深处聊一两句。\n\
硬性格式：正文必须以「@{from_name}」开头（@ 后紧跟对方昵称，再空格写内容）。\n\
若要转而跟真人用户说话，也可改用「@{user}」开头。\n\
要求：像真人在群里插话；不要复读用户最早的原话；除 @ 点名外不要再加角色名前缀。",
            from_name = from_name,
            from_content = from_content,
            user = user_nickname
        ),
    };

    format!(
        "你正在一个多人群聊里扮演角色「{nick}」。\n\
身份设定：{persona}\n\
群里的真人用户昵称是「{user}」（点名他时必须写成 @{user}）。\n\
硬性规则：\n\
1. 只输出你作为该角色要发到群里的一句/几句回复正文；\n\
2. 不要输出角色名、前缀、引号、思考过程或对本指令的复述；\n\
3. 不要冒充其他成员，不要解释你是 AI；\n\
4. 若此轮你不想说话（与话题无关、懒得回、角色设定如此等），只输出一行：NO_REPLY\n\
   不要输出「不回话」「沉默」等说明文字，系统会直接跳过，群里不会出现任何消息；\n\
5. 点名成员或真人用户时使用「@昵称」格式，昵称必须与群内一致。\n\
{extra}\n\
近期群聊记录：\n\
{history}\n\
\n\
{turn_block}",
        nick = bot.nickname,
        persona = bot.persona,
        user = user_nickname,
        extra = extra,
        history = if history_text.is_empty() {
            "（暂无历史）"
        } else {
            history_text
        },
        turn_block = turn_block
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
        "你是群聊发言调度器。根据用户这句话和近期上下文，决定本轮谁开口。\n\
目标：\n\
1. 选出 1–{max} 人；人数请带随机性，不要总是只选 1 人——相关角色有几位就尽量让几位开口（仍不超过 {max}）；\n\
2. 避免让所有人都回；入选者角度要不同，避免说相似内容；\n\
3. 若几乎无人相关，至少选 1 人；\n\
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

/// True when the message @everyone / @所有人 / @all.
fn mentions_everyone(text: &str) -> bool {
    const CN: &[&str] = &["所有人", "全体成员"];
    for token in CN {
        if at_token_present(text, token) {
            return true;
        }
    }
    at_token_present(&text.to_ascii_lowercase(), "all")
}

fn at_token_present(text: &str, token: &str) -> bool {
    let needle = format!("@{token}");
    let mut from = 0;
    while let Some(rel) = text[from..].find(&needle) {
        let start = from + rel;
        let end = start + needle.len();
        let ok_after = text[end..]
            .chars()
            .next()
            .map(|c| {
                c.is_whitespace()
                    || matches!(
                        c,
                        ',' | '.' | '!' | '?' | ':' | ';' | '，' | '。' | '！' | '？' | '；' | '：'
                    )
            })
            .unwrap_or(true);
        if ok_after {
            return true;
        }
        from = start + 1;
    }
    false
}

/// Bots explicitly @-mentioned in the message (longest nickname match, stable order).
/// `@所有人` / `@all` expands to every bot in the group.
fn resolve_mentioned_bots(text: &str, bots: &[Bot]) -> Vec<Bot> {
    if mentions_everyone(text) {
        return bots.to_vec();
    }

    let mut order: Vec<&Bot> = bots.iter().collect();
    order.sort_by(|a, b| {
        b.nickname
            .chars()
            .count()
            .cmp(&a.nickname.chars().count())
            .then_with(|| a.nickname.cmp(&b.nickname))
    });

    let mut found = Vec::new();
    let mut seen = HashSet::new();
    for bot in order {
        let needle = format!("@{}", bot.nickname);
        if text.contains(&needle) && seen.insert(bot.id.clone()) {
            found.push(bot.clone());
        }
    }
    found
}

/// Ensure crosstalk replies start with `@Nickname `.
fn ensure_at_mention(text: &str, name: &str) -> String {
    let t = text.trim();
    if t.is_empty() {
        return t.to_string();
    }
    let prefix = format!("@{name}");
    if t.starts_with(&prefix) {
        return t.to_string();
    }
    // Already @someone else — still force the intended target at the front.
    format!("{prefix} {t}")
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

/// Pick who should speak this turn. @mentions force those bots; otherwise route + diversify.
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

    let mentioned = resolve_mentioned_bots(user_content, bots);
    if !mentioned.is_empty() {
        let everyone = mentions_everyone(user_content);
        return mentioned
            .into_iter()
            .map(|bot| SelectedBot {
                bot,
                hint: Some(if everyone {
                    "用户 @所有人，请结合自己的身份各说各话，避免内容雷同".into()
                } else {
                    "用户 @ 了你，请直接回应他的话".into()
                }),
            })
            .collect();
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
        let count = random_speaker_count(bots.len());
        let pool: Vec<&Bot> = bots.iter().collect();
        for bot in pick_random_bots(&pool, count) {
            selected.push(SelectedBot {
                bot: bot.clone(),
                hint: None,
            });
        }
    }
    diversify_selection(bots, selected)
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

fn rand_index(len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    (Uuid::new_v4().as_u128() % len as u128) as usize
}

fn rand_range_inclusive(min: usize, max: usize) -> usize {
    if max <= min {
        return min;
    }
    min + rand_index(max - min + 1)
}

/// Fisher–Yates shuffle using Uuid entropy.
fn shuffle_in_place<T>(items: &mut [T]) {
    for i in (1..items.len()).rev() {
        let j = rand_index(i + 1);
        items.swap(i, j);
    }
}

/// Random speaker count for a turn: at least 1, up to min(available, MAX_SPEAKERS).
/// When several bots are available, bias away from always picking exactly 1.
fn random_speaker_count(available: usize) -> usize {
    let max = available.min(MAX_SPEAKERS);
    if max <= 1 {
        return 1;
    }
    // 1..=max, but re-roll once if we got 1 and max>=2 (≈75% chance of 2+)
    let mut n = rand_range_inclusive(1, max);
    if n == 1 && max >= 2 && rand_index(4) != 0 {
        n = rand_range_inclusive(2, max);
    }
    n
}

fn pick_random_bots<'a>(pool: &[&'a Bot], count: usize) -> Vec<&'a Bot> {
    if pool.is_empty() || count == 0 {
        return Vec::new();
    }
    let mut idxs: Vec<usize> = (0..pool.len()).collect();
    shuffle_in_place(&mut idxs);
    idxs.into_iter()
        .take(count.min(pool.len()))
        .map(|i| pool[i])
        .collect()
}

/// Expand / trim a routed selection so each turn has a random multi-speaker feel.
fn diversify_selection(bots: &[Bot], mut selected: Vec<SelectedBot>) -> Vec<SelectedBot> {
    if bots.is_empty() {
        return selected;
    }
    let target = random_speaker_count(bots.len());

    if selected.len() > target {
        shuffle_in_place(&mut selected);
        selected.truncate(target);
        return selected;
    }

    if selected.len() < target {
        let chosen: HashSet<String> = selected.iter().map(|s| s.bot.id.clone()).collect();
        let rest: Vec<&Bot> = bots.iter().filter(|b| !chosen.contains(&b.id)).collect();
        let need = target - selected.len();
        for bot in pick_random_bots(&rest, need) {
            selected.push(SelectedBot {
                bot: bot.clone(),
                hint: Some("从你自己的角色角度补充一句，避免重复别人".into()),
            });
        }
    }

    shuffle_in_place(&mut selected);
    selected
}

fn load_history_text(store: &SharedStore, group_id: &str) -> Result<String, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let msgs = store.list_messages(group_id, HISTORY_LIMIT)?;
    Ok(format_history(&msgs))
}

fn latest_bot_message(store: &SharedStore, group_id: &str) -> Result<Option<Message>, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let msgs = store.list_messages(group_id, HISTORY_LIMIT)?;
    Ok(msgs
        .into_iter()
        .rev()
        .find(|m| m.sender_type == "bot" && m.status == "done" && !m.content.is_empty()))
}

/// After the user-turn replies, randomly pull other bots into a short deep dialogue.
async fn run_crosstalk_rounds(
    app: AppHandle,
    store: Arc<SharedStore>,
    turns: Arc<TurnRegistry>,
    group_id: String,
    bots: &[Bot],
    initial_speaker_ids: &HashSet<String>,
    cancel: Arc<AtomicBool>,
    user_nickname: String,
) {
    if bots.len() < 2 || cancelled(&cancel) {
        return;
    }

    let Some(mut last) = latest_bot_message(&store, &group_id).ok().flatten() else {
        return;
    };

    let rounds = rand_range_inclusive(CHAIN_ROUNDS_MIN, CHAIN_ROUNDS_MAX);
    let mut spoke: HashSet<String> = initial_speaker_ids.clone();
    if let Some(id) = last.bot_id.clone() {
        spoke.insert(id);
    }

    for _ in 0..rounds {
        if cancelled(&cancel) {
            break;
        }
        let last_id = last.bot_id.clone().unwrap_or_default();
        let others: Vec<&Bot> = bots.iter().filter(|b| b.id != last_id).collect();
        if others.is_empty() {
            break;
        }

        // Prefer bots who haven't spoken yet this turn; else any other bot.
        let fresh: Vec<&Bot> = others
            .iter()
            .copied()
            .filter(|b| !spoke.contains(&b.id))
            .collect();
        let pool = if fresh.is_empty() { others } else { fresh };
        let count = random_speaker_count(pool.len());
        let picks = pick_random_bots(&pool, count);
        if picks.is_empty() {
            break;
        }

        let history_text = match load_history_text(&store, &group_id) {
            Ok(h) => h,
            Err(_) => break,
        };

        let peer_meta: Vec<(String, Option<String>)> = picks
            .iter()
            .map(|b| {
                (
                    b.nickname.clone(),
                    Some(format!("针对「{}」刚才的话接话，角度要和同伴不同", last.nickname)),
                )
            })
            .collect();

        let from_name = last.nickname.clone();
        let from_content = last.content.clone();

        let mut tasks = Vec::new();
        for pick in &picks {
            if cancelled(&cancel) {
                break;
            }
            let app2 = app.clone();
            let store2 = Arc::clone(&store);
            let cancel2 = Arc::clone(&cancel);
            let unick = user_nickname.clone();
            let gid = group_id.clone();
            let hist = history_text.clone();
            let turn = TurnKind::CrossTalk {
                from_name: from_name.clone(),
                from_content: from_content.clone(),
            };
            let peers: Vec<(String, Option<String>)> = peer_meta
                .iter()
                .filter(|(n, _)| n != &pick.nickname)
                .cloned()
                .collect();
            let selected = SelectedBot {
                bot: (*pick).clone(),
                hint: Some(format!(
                    "针对「{from_name}」刚才的话深入接话；同轮还有别人，不要说一样的话"
                )),
            };
            spoke.insert(pick.id.clone());
            let handle = tokio::spawn(async move {
                reply_as_bot(app2, store2, gid, selected, hist, turn, peers, cancel2, unick).await
            });
            turns.register_abort(&group_id, handle.abort_handle());
            tasks.push(handle);
        }

        let mut any_spoke = false;
        for task in tasks {
            match task.await {
                Ok(Ok(Some(msg))) => {
                    any_spoke = true;
                    last = msg;
                }
                _ => {}
            }
        }
        if cancelled(&cancel) {
            break;
        }
        if !any_spoke {
            continue;
        }

        // Prefer the chronologically latest bot line as the next trigger.
        if let Ok(Some(latest)) = latest_bot_message(&store, &group_id) {
            last = latest;
        }

        let gap = 350 + (Uuid::new_v4().as_u128() % 450) as u64;
        tokio::time::sleep(Duration::from_millis(gap)).await;
    }
}

pub async fn handle_user_message(
    app: AppHandle,
    store: Arc<SharedStore>,
    turns: Arc<TurnRegistry>,
    group_id: String,
    content: String,
) -> Result<(), String> {
    let text = content.trim().to_string();
    if text.is_empty() {
        return Err("消息不能为空".into());
    }

    let (user_msg, bots, history_text, user_profile) = {
        let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
        let profile = store.get_user_profile()?;
        let user_msg = store.append_message(Message {
            id: Uuid::new_v4().to_string(),
            group_id: group_id.clone(),
            sender_type: "user".into(),
            sender_id: None,
            bot_id: None,
            nickname: profile.nickname.clone(),
            avatar: Some(profile.avatar.clone()),
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
        (user_msg, bots, history_text, profile)
    };

    let user_message_id = user_msg.id.clone();
    let cancel = turns.begin(&group_id, user_message_id.clone());

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
            removed_ids: None,
        },
    );

    if bots.is_empty() {
        turns.finish(&group_id, &user_message_id);
        return Ok(());
    }

    if cancelled(&cancel) {
        turns.finish(&group_id, &user_message_id);
        return Ok(());
    }

    // Route first (silent): decide who should speak based on context.
    let speakers = select_speakers(&bots, &history_text, &text).await;
    if cancelled(&cancel) {
        turns.finish(&group_id, &user_message_id);
        return Ok(());
    }

    let peer_meta: Vec<(String, Option<String>)> = speakers
        .iter()
        .map(|s| (s.bot.nickname.clone(), s.hint.clone()))
        .collect();
    let mut initial_speaker_ids: HashSet<String> = HashSet::new();
    for s in &speakers {
        initial_speaker_ids.insert(s.bot.id.clone());
    }

    let user_nick = user_profile.nickname.clone();
    let mut tasks = Vec::new();
    for selected in speakers {
        if cancelled(&cancel) {
            break;
        }
        let app2 = app.clone();
        let store2 = Arc::clone(&store);
        let cancel2 = Arc::clone(&cancel);
        let gid = group_id.clone();
        let hist = history_text.clone();
        let unick = user_nick.clone();
        let turn = TurnKind::User {
            content: text.clone(),
        };
        let peers: Vec<(String, Option<String>)> = peer_meta
            .iter()
            .filter(|(n, _)| n != &selected.bot.nickname)
            .cloned()
            .collect();
        let handle = tokio::spawn(async move {
            reply_as_bot(app2, store2, gid, selected, hist, turn, peers, cancel2, unick).await
        });
        turns.register_abort(&group_id, handle.abort_handle());
        tasks.push(handle);
    }

    for task in tasks {
        if let Ok(Ok(Some(msg))) = task.await {
            if let Some(id) = msg.bot_id {
                initial_speaker_ids.insert(id);
            }
        }
    }

    if !cancelled(&cancel) {
        // Phase D: random other bots continue a short deep dialogue (sequential).
        run_crosstalk_rounds(
            app,
            store,
            Arc::clone(&turns),
            group_id.clone(),
            &bots,
            &initial_speaker_ids,
            cancel,
            user_nick,
        )
        .await;
    }

    turns.finish(&group_id, &user_message_id);
    Ok(())
}

/// Recall a user message and abort any in-flight replies for that turn.
pub fn recall_message(
    app: &AppHandle,
    store: &SharedStore,
    turns: &TurnRegistry,
    group_id: String,
    message_id: String,
) -> Result<Message, String> {
    // Stop generation first so no more bubbles land after recall.
    turns.cancel_for_message(&group_id, &message_id);

    let (recalled, removed_ids) = {
        let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
        store.recall_user_message(&group_id, &message_id)?
    };

    emit_event(
        app,
        ChatEvent {
            event_type: "message_recalled".into(),
            group_id: Some(group_id),
            message: Some(recalled.clone()),
            bot_id: None,
            message_id: Some(message_id),
            delta: None,
            content: None,
            error: None,
            action: Some("stop_replies".into()),
            removed_ids: Some(removed_ids),
        },
    );
    Ok(recalled)
}

async fn reply_as_bot(
    app: AppHandle,
    store: Arc<SharedStore>,
    group_id: String,
    selected: SelectedBot,
    history_text: String,
    turn: TurnKind,
    peers: Vec<(String, Option<String>)>,
    cancel: Arc<AtomicBool>,
    user_nickname: String,
) -> Result<Option<Message>, String> {
    if cancelled(&cancel) {
        return Ok(None);
    }
    let bot = selected.bot;
    let prompt = build_prompt(
        &bot,
        &history_text,
        &turn,
        selected.hint.as_deref(),
        &peers,
        &user_nickname,
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

    if cancelled(&cancel) {
        return Ok(None);
    }

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
                if cancelled(&cancel) {
                    return Ok(None);
                }
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
                        removed_ids: None,
                    },
                );
                return Ok(Some(error_message));
            }
        },
    };

    if cancelled(&cancel) {
        return Ok(None);
    }

    if let Some(chat_id) = &result.chat_id {
        if let Ok(store) = store.lock() {
            let _ = store.set_bot_chat_id(&bot.id, chat_id);
        }
    }

    let final_text = {
        let raw = result.text.trim().to_string();
        // Stay silent: no typing indicator, no chat bubble.
        if is_no_reply(&raw, &bot.nickname) {
            return Ok(None);
        }
        match &turn {
            TurnKind::CrossTalk { from_name, .. } => ensure_at_mention(&raw, from_name),
            TurnKind::User { .. } => raw,
        }
    };

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

    if cancelled(&cancel) {
        return Ok(None);
    }

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
            removed_ids: None,
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
            removed_ids: None,
        },
    );

    tokio::time::sleep(typing_delay()).await;

    if cancelled(&cancel) {
        // Drop the ephemeral typing bubble on the client.
        emit_event(
            &app,
            ChatEvent {
                event_type: "message_removed".into(),
                group_id: Some(group_id),
                message: None,
                bot_id: Some(bot_id),
                message_id: Some(message_id.clone()),
                delta: None,
                content: None,
                error: None,
                action: Some("stop_replies".into()),
                removed_ids: Some(vec![message_id]),
            },
        );
        return Ok(None);
    }

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
            removed_ids: None,
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

    #[test]
    fn resolves_at_mentions_longest_first() {
        let bots = sample_bots();
        let hit = resolve_mentioned_bots("@工程师 帮看下，顺便也问问产品", &bots);
        // sample only has 产品经理 / 工程师 — "产品" alone shouldn't false-match 产品经理 without @
        assert_eq!(hit.len(), 1);
        assert_eq!(hit[0].nickname, "工程师");

        let both = resolve_mentioned_bots("@产品经理 @工程师 一起看", &bots);
        assert_eq!(both.len(), 2);
    }

    #[test]
    fn resolves_at_all() {
        let bots = sample_bots();
        let all = resolve_mentioned_bots("@所有人 报到", &bots);
        assert_eq!(all.len(), 2);
        let all2 = resolve_mentioned_bots("大家 @all 看一下", &bots);
        assert_eq!(all2.len(), 2);
    }

    #[test]
    fn ensures_at_prefix() {
        assert_eq!(ensure_at_mention("你好", "小王"), "@小王 你好");
        assert_eq!(ensure_at_mention("@小王 你好", "小王"), "@小王 你好");
    }
}
