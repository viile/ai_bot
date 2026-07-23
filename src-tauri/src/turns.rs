use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::AbortHandle;

struct TurnState {
    user_message_id: String,
    cancel: Arc<AtomicBool>,
    aborts: Vec<AbortHandle>,
}

/// Tracks the in-flight reply turn per group so recall can stop generation.
#[derive(Default)]
pub struct TurnRegistry {
    inner: Mutex<HashMap<String, TurnState>>,
}

impl TurnRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new turn for this group (cancels any previous turn first).
    pub fn begin(&self, group_id: &str, user_message_id: String) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(prev) = map.remove(group_id) {
            prev.cancel.store(true, Ordering::SeqCst);
            for h in prev.aborts {
                h.abort();
            }
        }
        map.insert(
            group_id.to_string(),
            TurnState {
                user_message_id,
                cancel: Arc::clone(&cancel),
                aborts: Vec::new(),
            },
        );
        cancel
    }

    pub fn register_abort(&self, group_id: &str, handle: AbortHandle) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = map.get_mut(group_id) {
            state.aborts.push(handle);
        }
    }

    /// Cancel only if the active turn belongs to this user message.
    pub fn cancel_for_message(&self, group_id: &str, message_id: &str) -> bool {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let Some(state) = map.get(group_id) else {
            return false;
        };
        if state.user_message_id != message_id {
            return false;
        }
        let state = map.remove(group_id).unwrap();
        state.cancel.store(true, Ordering::SeqCst);
        for h in state.aborts {
            h.abort();
        }
        true
    }

    pub fn finish(&self, group_id: &str, user_message_id: &str) {
        let mut map = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if map
            .get(group_id)
            .map(|s| s.user_message_id == user_message_id)
            .unwrap_or(false)
        {
            map.remove(group_id);
        }
    }
}

pub fn cancelled(flag: &Arc<AtomicBool>) -> bool {
    flag.load(Ordering::SeqCst)
}
