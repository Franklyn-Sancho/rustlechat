use rand::RngCore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EncryptedMessage {
    pub encrypted_content: Vec<u8>,
    pub nonce: Vec<u8>,
    pub key_fingerprint: String,  // Adicione este campo
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PublicKey(pub String);

#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateKey(pub String);


#[derive(Clone)]
pub struct ChatKey(pub Vec<u8>);

impl ChatKey {
    pub fn generate() -> Self {
        let mut key = vec![0u8; 32];
        rand::thread_rng().fill_bytes(&mut key);
        ChatKey(key)
    }
}