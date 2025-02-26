use std::collections::HashMap;
use uuid::Uuid;
use crate::crypto::{ChatKey, CryptoError};

#[derive(Debug, Default)]
pub struct ChatKeyStore {
    chat_keys: HashMap<Uuid, HashMap<Uuid, Vec<u8>>>,
}

impl ChatKeyStore {
    pub fn new() -> Self {
        Self {
            chat_keys: HashMap::new(),
        }
    }

    pub fn add_chat_key(&mut self, chat_id: Uuid, user_id: Uuid, encrypted_key: Vec<u8>) {
        self.chat_keys
            .entry(chat_id)
            .or_default()
            .insert(user_id, encrypted_key);
    }

    pub fn get_chat_key(&self, chat_id: Uuid, user_id: Uuid) -> Option<&Vec<u8>> {
        self.chat_keys
            .get(&chat_id)
            .and_then(|users| users.get(&user_id))
    }

    pub fn remove_chat_key(&mut self, chat_id: Uuid, user_id: Uuid) {
        if let Some(users) = self.chat_keys.get_mut(&chat_id) {
            users.remove(&user_id);
            if users.is_empty() {
                self.chat_keys.remove(&chat_id);
            }
        }
    }

    pub fn clear_chat(&mut self, chat_id: Uuid) {
        self.chat_keys.remove(&chat_id);
    }
}