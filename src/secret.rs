use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pair {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Secret {
    name: String,
    contents: Vec<Pair>,
    last_modified: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedSecret {
    nonce: [u8; 12],
    ciphertext: String,
}

impl Secret {
    pub fn new(name: &str, contents: Vec<Pair>) -> Secret {
        Secret {
            name: String::from(name),
            contents,
            last_modified: Local::now(),
        }
    }

    pub fn encrypt(&self, key: [u8; 32]) -> EncryptedSecret {
        let aes_key = Key::<Aes256Gcm>::from_slice(&key).clone();
        let cipher = Aes256Gcm::new(&aes_key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, &*self.to_json().as_ref()).unwrap();
        let encoded_ciphertext = general_purpose::STANDARD.encode(&ciphertext);
        EncryptedSecret {
            nonce: nonce.into(),
            ciphertext: encoded_ciphertext,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn from_json(json: String) -> Secret {
        serde_json::from_str(json.as_str()).unwrap()
    }

    pub fn get_name(&self) -> &str {
        self.name.as_str()
    }

    pub fn get_contents(&self) -> Vec<Pair> {
        self.contents.clone()
    }
}

impl EncryptedSecret {
    pub fn decrypt(&self, key: [u8; 32]) -> Secret {
        let ciphertext = general_purpose::STANDARD.decode(&self.ciphertext).unwrap();
        let aes_key = Key::<Aes256Gcm>::from_slice(&key);
        let cipher = Aes256Gcm::new(aes_key);
        let nonce = Nonce::from_slice(&self.nonce);
        let plaintext = cipher
            .decrypt(nonce, ciphertext.as_slice())
            .expect("Couldn't decrypt, secret was malformed - potentially tampered with");
        Secret::from_json(String::from_utf8(plaintext).unwrap())
    }
}
