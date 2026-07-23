use crate::models::{
    Bot, CreateBotInput, Group, GroupDetail, Message, UpdateBotInput, UpdateUserProfileInput,
    UserProfile,
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use uuid::Uuid;

pub struct Store {
    conn: Connection,
    root: PathBuf,
}

impl Store {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&root).map_err(|e| e.to_string())?;
        let db_path = root.join("ai_bot.sqlite");
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            CREATE TABLE IF NOT EXISTS groups (
                id TEXT PRIMARY KEY NOT NULL,
                name TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS bots (
                id TEXT PRIMARY KEY NOT NULL,
                group_id TEXT NOT NULL,
                nickname TEXT NOT NULL,
                avatar TEXT NOT NULL,
                persona TEXT NOT NULL,
                model TEXT,
                cursor_chat_id TEXT,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY NOT NULL,
                group_id TEXT NOT NULL,
                sender_type TEXT NOT NULL,
                sender_id TEXT,
                bot_id TEXT,
                nickname TEXT NOT NULL,
                avatar TEXT,
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                status TEXT NOT NULL,
                FOREIGN KEY (group_id) REFERENCES groups(id) ON DELETE CASCADE
            );
            CREATE INDEX IF NOT EXISTS idx_bots_group ON bots(group_id);
            CREATE INDEX IF NOT EXISTS idx_messages_group_created
                ON messages(group_id, created_at);
            CREATE TABLE IF NOT EXISTS user_profile (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                nickname TEXT NOT NULL,
                avatar TEXT NOT NULL
            );
            INSERT OR IGNORE INTO user_profile (id, nickname, avatar)
                VALUES (1, '我', '#1e3a5f');
            ",
        )
        .map_err(|e| e.to_string())?;

        let store = Self { conn, root };
        store.migrate_from_json_if_needed()?;
        Ok(store)
    }

    fn migrate_from_json_if_needed(&self) -> Result<(), String> {
        let groups_json = self.root.join("groups.json");
        if !groups_json.exists() {
            return Ok(());
        }
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM groups", [], |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if count > 0 {
            return Ok(());
        }

        let groups: Vec<Group> = read_json_file(&groups_json, vec![]);
        let bots: Vec<Bot> = read_json_file(&self.root.join("bots.json"), vec![]);

        for g in &groups {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO groups (id, name, created_at) VALUES (?1, ?2, ?3)",
                    params![g.id, g.name, g.created_at],
                )
                .map_err(|e| e.to_string())?;
        }
        for b in &bots {
            self.conn
                .execute(
                    "INSERT OR IGNORE INTO bots
                     (id, group_id, nickname, avatar, persona, model, cursor_chat_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        b.id,
                        b.group_id,
                        b.nickname,
                        b.avatar,
                        b.persona,
                        b.model,
                        b.cursor_chat_id
                    ],
                )
                .map_err(|e| e.to_string())?;
        }
        for g in &groups {
            let messages: Vec<Message> =
                read_json_file(&self.root.join("messages").join(format!("{}.json", g.id)), vec![]);
            for m in messages {
                self.conn
                    .execute(
                        "INSERT OR IGNORE INTO messages
                         (id, group_id, sender_type, sender_id, bot_id, nickname, avatar, content, created_at, status)
                         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                        params![
                            m.id,
                            m.group_id,
                            m.sender_type,
                            m.sender_id,
                            m.bot_id,
                            m.nickname,
                            m.avatar,
                            m.content,
                            m.created_at,
                            m.status
                        ],
                    )
                    .map_err(|e| e.to_string())?;
            }
        }

        let backup = self.root.join("json_backup");
        let _ = fs::create_dir_all(&backup);
        let _ = fs::rename(&groups_json, backup.join("groups.json"));
        let _ = fs::rename(self.root.join("bots.json"), backup.join("bots.json"));
        let messages_dir = self.root.join("messages");
        if messages_dir.exists() {
            let _ = fs::rename(&messages_dir, backup.join("messages"));
        }
        Ok(())
    }

    pub fn list_groups(&self) -> Result<Vec<Group>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, name, created_at FROM groups ORDER BY created_at ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut groups = Vec::new();
        for row in rows {
            let (id, name, created_at) = row.map_err(|e| e.to_string())?;
            let bot_ids = self.bot_ids_for_group(&id)?;
            groups.push(Group {
                id,
                name,
                created_at,
                bot_ids,
            });
        }
        Ok(groups)
    }

    fn bot_ids_for_group(&self, group_id: &str) -> Result<Vec<String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM bots WHERE group_id = ?1 ORDER BY rowid ASC")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![group_id], |row| row.get(0))
            .map_err(|e| e.to_string())?;
        let mut ids = Vec::new();
        for row in rows {
            ids.push(row.map_err(|e| e.to_string())?);
        }
        Ok(ids)
    }

    pub fn get_group(&self, group_id: &str) -> Result<Option<Group>, String> {
        let row = self
            .conn
            .query_row(
                "SELECT id, name, created_at FROM groups WHERE id = ?1",
                params![group_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                    ))
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;

        Ok(match row {
            Some((id, name, created_at)) => Some(Group {
                bot_ids: self.bot_ids_for_group(&id)?,
                id,
                name,
                created_at,
            }),
            None => None,
        })
    }

    pub fn create_group(&self, name: &str) -> Result<Group, String> {
        let group = Group {
            id: Uuid::new_v4().to_string(),
            name: {
                let t = name.trim();
                if t.is_empty() {
                    "新群聊".to_string()
                } else {
                    t.to_string()
                }
            },
            created_at: Utc::now().to_rfc3339(),
            bot_ids: vec![],
        };
        self.conn
            .execute(
                "INSERT INTO groups (id, name, created_at) VALUES (?1, ?2, ?3)",
                params![group.id, group.name, group.created_at],
            )
            .map_err(|e| e.to_string())?;
        Ok(group)
    }

    pub fn update_group(&self, group_id: &str, name: &str) -> Result<Group, String> {
        let t = name.trim();
        if !t.is_empty() {
            let n = self
                .conn
                .execute(
                    "UPDATE groups SET name = ?1 WHERE id = ?2",
                    params![t, group_id],
                )
                .map_err(|e| e.to_string())?;
            if n == 0 {
                return Err("群不存在".into());
            }
        }
        self.get_group(group_id)?
            .ok_or_else(|| "群不存在".to_string())
    }

    pub fn delete_group(&self, group_id: &str) -> Result<(), String> {
        let n = self
            .conn
            .execute("DELETE FROM groups WHERE id = ?1", params![group_id])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("群不存在".into());
        }
        Ok(())
    }

    pub fn list_bots(&self, group_id: Option<&str>) -> Result<Vec<Bot>, String> {
        let mut bots = Vec::new();
        if let Some(gid) = group_id {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, group_id, nickname, avatar, persona, model, cursor_chat_id
                     FROM bots WHERE group_id = ?1 ORDER BY rowid ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map(params![gid], map_bot_row)
                .map_err(|e| e.to_string())?;
            for row in rows {
                bots.push(row.map_err(|e| e.to_string())?);
            }
        } else {
            let mut stmt = self
                .conn
                .prepare(
                    "SELECT id, group_id, nickname, avatar, persona, model, cursor_chat_id
                     FROM bots ORDER BY rowid ASC",
                )
                .map_err(|e| e.to_string())?;
            let rows = stmt
                .query_map([], map_bot_row)
                .map_err(|e| e.to_string())?;
            for row in rows {
                bots.push(row.map_err(|e| e.to_string())?);
            }
        }
        Ok(bots)
    }

    pub fn get_bot(&self, bot_id: &str) -> Result<Option<Bot>, String> {
        self.conn
            .query_row(
                "SELECT id, group_id, nickname, avatar, persona, model, cursor_chat_id
                 FROM bots WHERE id = ?1",
                params![bot_id],
                map_bot_row,
            )
            .optional()
            .map_err(|e| e.to_string())
    }

    pub fn create_bot(&self, group_id: &str, input: CreateBotInput) -> Result<Bot, String> {
        if self.get_group(group_id)?.is_none() {
            return Err("群不存在".into());
        }
        let bot = Bot {
            id: Uuid::new_v4().to_string(),
            group_id: group_id.to_string(),
            nickname: input
                .nickname
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "机器人".into()),
            avatar: input
                .avatar
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "#0d9488".into()),
            persona: input
                .persona
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "你是一个友好的助手。".into()),
            model: input.model.and_then(|s| {
                let t = s.trim().to_string();
                if t.is_empty() {
                    None
                } else {
                    Some(t)
                }
            }),
            cursor_chat_id: None,
        };
        self.conn
            .execute(
                "INSERT INTO bots
                 (id, group_id, nickname, avatar, persona, model, cursor_chat_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    bot.id,
                    bot.group_id,
                    bot.nickname,
                    bot.avatar,
                    bot.persona,
                    bot.model,
                    bot.cursor_chat_id
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(bot)
    }

    pub fn update_bot(&self, bot_id: &str, input: UpdateBotInput) -> Result<Bot, String> {
        let mut bot = self
            .get_bot(bot_id)?
            .ok_or_else(|| "机器人不存在".to_string())?;
        if let Some(n) = input.nickname {
            let t = n.trim();
            if !t.is_empty() {
                bot.nickname = t.to_string();
            }
        }
        if let Some(a) = input.avatar {
            bot.avatar = a;
        }
        if let Some(p) = input.persona {
            let t = p.trim();
            if !t.is_empty() {
                bot.persona = t.to_string();
            }
        }
        if input.clear_model == Some(true) {
            bot.model = None;
        } else if let Some(model) = input.model {
            let t = model.trim().to_string();
            bot.model = if t.is_empty() { None } else { Some(t) };
        }
        self.conn
            .execute(
                "UPDATE bots SET nickname = ?1, avatar = ?2, persona = ?3, model = ?4
                 WHERE id = ?5",
                params![bot.nickname, bot.avatar, bot.persona, bot.model, bot.id],
            )
            .map_err(|e| e.to_string())?;
        Ok(bot)
    }

    pub fn set_bot_chat_id(&self, bot_id: &str, chat_id: &str) -> Result<(), String> {
        let n = self
            .conn
            .execute(
                "UPDATE bots SET cursor_chat_id = ?1 WHERE id = ?2",
                params![chat_id, bot_id],
            )
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("机器人不存在".into());
        }
        Ok(())
    }

    pub fn delete_bot(&self, bot_id: &str) -> Result<(), String> {
        let n = self
            .conn
            .execute("DELETE FROM bots WHERE id = ?1", params![bot_id])
            .map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("机器人不存在".into());
        }
        Ok(())
    }

    pub fn list_messages(&self, group_id: &str, limit: usize) -> Result<Vec<Message>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, group_id, sender_type, sender_id, bot_id, nickname, avatar, content, created_at, status
                 FROM messages
                 WHERE group_id = ?1
                 ORDER BY created_at ASC, rowid ASC",
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![group_id], map_message_row)
            .map_err(|e| e.to_string())?;
        let mut messages = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| e.to_string())?);
        }
        if messages.len() > limit {
            messages = messages.split_off(messages.len() - limit);
        }
        Ok(messages)
    }

    pub fn append_message(&self, mut message: Message) -> Result<Message, String> {
        if message.id.is_empty() {
            message.id = Uuid::new_v4().to_string();
        }
        if message.created_at.is_empty() {
            message.created_at = Utc::now().to_rfc3339();
        }
        self.conn
            .execute(
                "INSERT INTO messages
                 (id, group_id, sender_type, sender_id, bot_id, nickname, avatar, content, created_at, status)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    message.id,
                    message.group_id,
                    message.sender_type,
                    message.sender_id,
                    message.bot_id,
                    message.nickname,
                    message.avatar,
                    message.content,
                    message.created_at,
                    message.status
                ],
            )
            .map_err(|e| e.to_string())?;
        Ok(message)
    }

    pub fn get_message(&self, message_id: &str) -> Result<Option<Message>, String> {
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, group_id, sender_type, sender_id, bot_id, nickname, avatar,
                        content, created_at, status
                 FROM messages WHERE id = ?1",
            )
            .map_err(|e| e.to_string())?;
        let mut rows = stmt
            .query_map(params![message_id], map_message_row)
            .map_err(|e| e.to_string())?;
        match rows.next() {
            Some(Ok(m)) => Ok(Some(m)),
            Some(Err(e)) => Err(e.to_string()),
            None => Ok(None),
        }
    }

    /// Mark a user message as recalled and delete bot replies that followed in this turn.
    pub fn recall_user_message(
        &self,
        group_id: &str,
        message_id: &str,
    ) -> Result<(Message, Vec<String>), String> {
        let msg = self
            .get_message(message_id)?
            .ok_or_else(|| "消息不存在".to_string())?;
        if msg.group_id != group_id {
            return Err("消息不存在".into());
        }
        if msg.sender_type != "user" {
            return Err("只能撤回自己的消息".into());
        }
        if msg.status == "recalled" {
            return Ok((msg, vec![]));
        }

        self.conn
            .execute(
                "UPDATE messages SET status = 'recalled', content = '' WHERE id = ?1",
                params![message_id],
            )
            .map_err(|e| e.to_string())?;

        // Collect messages after this one until the next user message.
        let all = self.list_messages(group_id, 500)?;
        let mut removing = Vec::new();
        let mut after = false;
        for m in &all {
            if m.id == message_id {
                after = true;
                continue;
            }
            if !after {
                continue;
            }
            if m.sender_type == "user" {
                break;
            }
            if m.sender_type == "bot" {
                removing.push(m.id.clone());
            }
        }

        for id in &removing {
            self.conn
                .execute("DELETE FROM messages WHERE id = ?1", params![id])
                .map_err(|e| e.to_string())?;
        }

        let recalled = self
            .get_message(message_id)?
            .ok_or_else(|| "消息不存在".to_string())?;
        Ok((recalled, removing))
    }

    pub fn get_user_profile(&self) -> Result<UserProfile, String> {
        let row = self
            .conn
            .query_row(
                "SELECT nickname, avatar FROM user_profile WHERE id = 1",
                [],
                |row| {
                    Ok(UserProfile {
                        nickname: row.get(0)?,
                        avatar: row.get(1)?,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;
        Ok(row.unwrap_or(UserProfile {
            nickname: "我".into(),
            avatar: "#1e3a5f".into(),
        }))
    }

    pub fn update_user_profile(&self, input: UpdateUserProfileInput) -> Result<UserProfile, String> {
        let mut profile = self.get_user_profile()?;
        if let Some(n) = input.nickname {
            let n = n.trim().to_string();
            if n.is_empty() {
                return Err("昵称不能为空".into());
            }
            if n.chars().count() > 24 {
                return Err("昵称太长".into());
            }
            profile.nickname = n;
        }
        if let Some(a) = input.avatar {
            let a = a.trim().to_string();
            if a.is_empty() {
                return Err("头像不能为空".into());
            }
            profile.avatar = a;
        }
        self.conn
            .execute(
                "INSERT INTO user_profile (id, nickname, avatar) VALUES (1, ?1, ?2)
                 ON CONFLICT(id) DO UPDATE SET nickname = excluded.nickname, avatar = excluded.avatar",
                params![profile.nickname, profile.avatar],
            )
            .map_err(|e| e.to_string())?;
        Ok(profile)
    }

    pub fn list_group_details(&self) -> Result<Vec<GroupDetail>, String> {
        let groups = self.list_groups()?;
        let mut out = Vec::with_capacity(groups.len());
        for group in groups {
            let bots = self.list_bots(Some(&group.id))?;
            out.push(GroupDetail { group, bots });
        }
        Ok(out)
    }
}

fn map_bot_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Bot> {
    Ok(Bot {
        id: row.get(0)?,
        group_id: row.get(1)?,
        nickname: row.get(2)?,
        avatar: row.get(3)?,
        persona: row.get(4)?,
        model: row.get(5)?,
        cursor_chat_id: row.get(6)?,
    })
}

fn map_message_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        group_id: row.get(1)?,
        sender_type: row.get(2)?,
        sender_id: row.get(3)?,
        bot_id: row.get(4)?,
        nickname: row.get(5)?,
        avatar: row.get(6)?,
        content: row.get(7)?,
        created_at: row.get(8)?,
        status: row.get(9)?,
    })
}

fn read_json_file<T: serde::de::DeserializeOwned>(path: &Path, fallback: T) -> T {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

pub type SharedStore = Mutex<Store>;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CreateBotInput;
    use std::env;

    #[test]
    fn sqlite_roundtrip() {
        let dir = env::temp_dir().join(format!("ai_bot_test_{}", Uuid::new_v4()));
        let store = Store::new(dir.clone()).unwrap();
        let g = store.create_group("测试群").unwrap();
        let bot = store
            .create_bot(
                &g.id,
                CreateBotInput {
                    nickname: Some("小周".into()),
                    avatar: Some("#0d9488".into()),
                    persona: Some("三十多岁的产品经理。".into()),
                    model: None,
                },
            )
            .unwrap();
        store
            .append_message(Message {
                id: Uuid::new_v4().to_string(),
                group_id: g.id.clone(),
                sender_type: "user".into(),
                sender_id: None,
                bot_id: None,
                nickname: "我".into(),
                avatar: None,
                content: "你好".into(),
                created_at: Utc::now().to_rfc3339(),
                status: "done".into(),
            })
            .unwrap();

        let store2 = Store::new(dir.clone()).unwrap();
        let groups = store2.list_group_details().unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].group.name, "测试群");
        assert_eq!(groups[0].bots[0].nickname, bot.nickname);
        assert_eq!(groups[0].bots[0].persona, "三十多岁的产品经理。");
        let msgs = store2.list_messages(&g.id, 10).unwrap();
        assert_eq!(msgs.len(), 1);
        assert_eq!(msgs[0].content, "你好");
        let _ = fs::remove_dir_all(dir);
    }
}
