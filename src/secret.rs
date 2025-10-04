use aes_gcm::{
    Aes256Gcm, Key, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use base64::{Engine as _, engine::general_purpose};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Secret {
    name: String,
    username: String,
    password: String,
    last_modified: DateTime<Local>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedSecret {
    nonce: [u8; 12],
    ciphertext: String,
}

impl Secret {
    pub fn new(name: &str, username: &str, password: &str) -> Secret {
        Secret {
            name: String::from(name),
            username: String::from(username),
            password: String::from(password),
            last_modified: Local::now(),
        }
    }

    pub fn save(&self, key: [u8; 32], mut filepath: PathBuf) -> Result<(), std::io::Error> {
        let aes_key = Key::<Aes256Gcm>::from_slice(&key).clone();
        let cipher = Aes256Gcm::new(&aes_key);
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, &*self.to_json().as_ref()).unwrap();

        //write to disk
        let json = json!({
            "ciphertext": general_purpose::STANDARD.encode(&ciphertext),
            "nonce": *nonce
        });

        filepath.push(&self.name.as_str());
        filepath.set_extension(".json");

        fs::write(filepath, json.to_string())
    }

    pub fn delete(&self, mut filepath: PathBuf) -> Result<(), std::io::Error> {
        filepath.push(&self.name.as_str());
        filepath.set_extension(".json");
        fs::remove_file(filepath)
    }

    pub fn print(&self) {
        println!("{:?}", self);
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

    pub fn get_username(&self) -> &str {
        self.username.as_str()
    }

    pub fn get_password(&self) -> &str {
        self.password.as_str()
    }

    pub fn get_last_modified(&self) -> DateTime<Local> {
        self.last_modified
    }
}

impl EncryptedSecret {
    pub fn decrypt(key: [u8; 32], name: PathBuf) -> Secret {
        let contents = fs::read_to_string(name); //deserialize here, the problem is that you need to deserialize twice because of the nonce
        match contents {
            Ok(text) => {
                let contents: EncryptedSecret = serde_json::from_str(text.as_str()).unwrap();
                //println!("{:?}", contents);
                let ciphertext = general_purpose::STANDARD
                    .decode(&contents.ciphertext)
                    .unwrap();
                let aes_key = Key::<Aes256Gcm>::from_slice(&key).clone();
                let cipher = Aes256Gcm::new(&aes_key);
                let nonce = Nonce::from_slice(&contents.nonce);
                let plaintext = cipher
                    .decrypt(&nonce, ciphertext.as_slice())
                    .expect("Couldn't decrypt");
                Secret::from_json(String::from_utf8(plaintext).unwrap())
            }
            Err(e) => panic!("{}", e),
        }
    }
}
