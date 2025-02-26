use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Key, Nonce,
};
use rand::{RngCore, rngs::OsRng};
use crate::crypto::{CryptoError, EncryptedMessage, ChatKey};

#[derive(Clone)]
pub struct MessageCrypto {
    cipher: Aes256Gcm,
    key_fingerprint: String,
}

impl MessageCrypto {
    /// Creates a new MessageCrypto instance with the given chat key
    /// 
    /// # Arguments
    /// * `key` - Chat key
    /// 
    /// # Returns
    /// * MessageCrypto instance or error
    pub fn new(key: &ChatKey) -> Result<Self, CryptoError> {
        if key.0.len() != 32 {
            return Err(CryptoError::InvalidKey(format!(
                "Invalid key length: {}, expected 32",
                key.0.len()
            )));
        }
        
        // Create a fingerprint of the key for tracking
        let key_fingerprint = format!("{:x}", md5::compute(&key.0));
        let key = Key::<Aes256Gcm>::from_slice(&key.0);
        let cipher = Aes256Gcm::new(key);
        
        Ok(Self {
            cipher,
            key_fingerprint
        })
    }
    
    /// Generates a new random chat key
    /// 
    /// # Returns
    /// * New ChatKey instance
    pub fn generate_chat_key() -> ChatKey {
        let mut key = vec![0u8; 32];
        OsRng.fill_bytes(&mut key);
        ChatKey(key)
    }
    
    /// Encrypts a message
    /// 
    /// # Arguments
    /// * `message` - Plain text message
    /// 
    /// # Returns
    /// * Encrypted message or error
    pub fn encrypt(&self, message: &str) -> Result<EncryptedMessage, CryptoError> {
        // Generate random nonce
        let mut nonce_bytes = vec![0u8; 12];
        rand::thread_rng().fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        // Encrypt the message
        let encrypted = self.cipher
            .encrypt(nonce, message.as_bytes())
            .map_err(|e| {
                CryptoError::EncryptionError(e.to_string())
            })?;
        
        Ok(EncryptedMessage {
            encrypted_content: encrypted,
            nonce: nonce_bytes,
            key_fingerprint: self.key_fingerprint.clone(),
        })
    }
    
    /// Decrypts an encrypted message
    /// 
    /// # Arguments
    /// * `encrypted` - Encrypted message
    /// 
    /// # Returns
    /// * Decrypted message as string or error
    pub fn decrypt(&self, encrypted: &EncryptedMessage) -> Result<String, CryptoError> {
        if encrypted.nonce.len() != 12 {
            return Err(CryptoError::InvalidData(format!(
                "Invalid nonce length: {}, expected 12",
                encrypted.nonce.len()
            )));
        }
        
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let decrypted = self.cipher
            .decrypt(nonce, encrypted.encrypted_content.as_slice())
            .map_err(|e| {
                CryptoError::DecryptionError(e.to_string())
            })?;
        
        let decrypted_message = String::from_utf8(decrypted)
            .map_err(|e| CryptoError::InvalidData(e.to_string()))?;
        
        Ok(decrypted_message)
    }
}