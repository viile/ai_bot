mod cursor;
mod models;
mod orchestrator;
mod store;
mod turns;

use models::{CreateBotInput, GroupDetail, Message, UpdateBotInput, UpdateUserProfileInput};
use store::{SharedStore, Store};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager, State};
use turns::TurnRegistry;

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    // Prefer project-local ./data in dev for easy inspection; fall back to app data.
    let local = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../data");
    if local.exists() || cfg!(debug_assertions) {
        std::fs::create_dir_all(&local).map_err(|e| e.to_string())?;
        return Ok(local.canonicalize().unwrap_or(local));
    }
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("data");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

#[tauri::command]
async fn get_status() -> models::CursorStatus {
    cursor::get_status().await
}

#[tauri::command]
fn list_groups(store: State<'_, Arc<SharedStore>>) -> Result<Vec<GroupDetail>, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.list_group_details()
}

#[tauri::command]
fn create_group(
    store: State<'_, Arc<SharedStore>>,
    name: String,
) -> Result<GroupDetail, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let group = store.create_group(&name)?;
    Ok(GroupDetail {
        group,
        bots: vec![],
    })
}

#[tauri::command]
fn update_group(
    store: State<'_, Arc<SharedStore>>,
    id: String,
    name: String,
) -> Result<GroupDetail, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let group = store.update_group(&id, &name)?;
    let bots = store.list_bots(Some(&id))?;
    Ok(GroupDetail { group, bots })
}

#[tauri::command]
fn delete_group(store: State<'_, Arc<SharedStore>>, id: String) -> Result<(), String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.delete_group(&id)
}

#[tauri::command]
fn list_messages(
    store: State<'_, Arc<SharedStore>>,
    group_id: String,
) -> Result<Vec<Message>, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.list_messages(&group_id, 500)
}

#[tauri::command]
fn create_bot(
    store: State<'_, Arc<SharedStore>>,
    group_id: String,
    input: CreateBotInput,
) -> Result<models::Bot, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.create_bot(&group_id, input)
}

#[tauri::command]
fn update_bot(
    store: State<'_, Arc<SharedStore>>,
    group_id: String,
    bot_id: String,
    input: UpdateBotInput,
) -> Result<models::Bot, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let bot = store
        .get_bot(&bot_id)?
        .ok_or_else(|| "机器人不存在".to_string())?;
    if bot.group_id != group_id {
        return Err("机器人不存在".into());
    }
    store.update_bot(&bot_id, input)
}

#[tauri::command]
fn delete_bot(
    store: State<'_, Arc<SharedStore>>,
    group_id: String,
    bot_id: String,
) -> Result<(), String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    let bot = store
        .get_bot(&bot_id)?
        .ok_or_else(|| "机器人不存在".to_string())?;
    if bot.group_id != group_id {
        return Err("机器人不存在".into());
    }
    store.delete_bot(&bot_id)
}

#[tauri::command]
fn get_user_profile(store: State<'_, Arc<SharedStore>>) -> Result<models::UserProfile, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.get_user_profile()
}

#[tauri::command]
fn update_user_profile(
    store: State<'_, Arc<SharedStore>>,
    input: UpdateUserProfileInput,
) -> Result<models::UserProfile, String> {
    let store = store.lock().map_err(|_| "存储锁定失败".to_string())?;
    store.update_user_profile(input)
}

#[tauri::command]
async fn send_message(
    app: AppHandle,
    store: State<'_, Arc<SharedStore>>,
    turns: State<'_, Arc<TurnRegistry>>,
    group_id: String,
    content: String,
) -> Result<(), String> {
    let store = Arc::clone(&store);
    let turns = Arc::clone(&turns);
    orchestrator::handle_user_message(app, store, turns, group_id, content).await
}

#[tauri::command]
fn recall_message(
    app: AppHandle,
    store: State<'_, Arc<SharedStore>>,
    turns: State<'_, Arc<TurnRegistry>>,
    group_id: String,
    message_id: String,
) -> Result<Message, String> {
    orchestrator::recall_message(&app, &store, &turns, group_id, message_id)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = data_dir(app.handle())?;
            let store = Arc::new(Mutex::new(Store::new(dir)?));
            let turns = Arc::new(TurnRegistry::new());
            app.manage(store);
            app.manage(turns);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            list_groups,
            create_group,
            update_group,
            delete_group,
            list_messages,
            create_bot,
            update_bot,
            delete_bot,
            get_user_profile,
            update_user_profile,
            send_message,
            recall_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
