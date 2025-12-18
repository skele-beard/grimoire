use crate::secret;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use cli_clipboard::{ClipboardContext, ClipboardProvider};
use crossterm::event::KeyCode;
use dirs::config_dir;
use rand::distr::{Distribution, Uniform};
use rand::prelude::*;
use rand_argon_compatible::rngs::OsRng as OsRng08;
use secret::{EncryptedSecret, Pair, Secret};
use serde::Deserialize;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Config {
    master_password_file: PathBuf,
    password_store: PathBuf,
}

impl Config {
    fn load() -> Self {
        let config_path = config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("grimoire/config.toml");

        let content = match fs::read_to_string(&config_path) {
            Ok(s) if !s.trim().is_empty() => s,
            Ok(_) | Err(_) => {
                eprintln!("Error reading config, using defaults");
                String::new()
            }
        };

        if !content.trim().is_empty() {
            if let Ok(cfg) = toml::from_str(&content) {
                return cfg;
            } else {
                eprintln!("Invalid config format, using defaults");
            }
        }

        let base_dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
        Self {
            master_password_file: base_dir.join("grimoire/password_store/master_password"),
            password_store: base_dir.join("grimoire/password_store.json"),
        }
    }
}

pub enum CurrentScreen {
    Main,
    Searching,
    Editing,
    New,
    Login,
    Init,
}

#[derive(Clone)]
pub enum CurrentlyEditing {
    Name,
    Key(usize),
    Value(usize),
}

pub struct App {
    pub secrets: Vec<Secret>,
    pub current_screen: CurrentScreen,
    pub currently_editing: Option<CurrentlyEditing>,
    pub currently_selected_secret_idx: Option<usize>,
    pub master_password_file: PathBuf,
    pub password_store: PathBuf,
    pub secrets_per_row: usize,
    pub name_input: String,
    pub key_input: String,
    pub value_input: String,
    pub secret_scratch_content: Vec<Pair>,
    pub search_buffer: VecDeque<usize>,
    pub scratch: String,
    pub unlocked: bool,
    pub clipboard: ClipboardContext,
    key: [u8; 32],
}

#[allow(clippy::single_match)]
impl App {
    pub fn new() -> App {
        let config = Config::load();
        let mut app = App {
            secrets: Vec::new(),
            secret_scratch_content: Vec::new(),
            search_buffer: VecDeque::new(),
            current_screen: CurrentScreen::Login,
            currently_selected_secret_idx: None,
            currently_editing: None,
            master_password_file: config.master_password_file,
            password_store: config.password_store,
            secrets_per_row: 3,
            name_input: String::from(""),
            key_input: String::new(),
            value_input: String::new(),
            scratch: String::new(),
            unlocked: false,
            clipboard: ClipboardContext::new().unwrap(),
            key: [0u8; 32],
        };
        // init the master_password and secret store
        app.init();
        app
    }

    pub fn authenticate(
        &mut self,
        master_password: &str,
    ) -> Result<bool, argon2::password_hash::Error> {
        // read stored hash
        let hash = fs::read_to_string(&self.master_password_file).expect("should have read file");
        let parsed_hash = PasswordHash::new(&hash)?;

        // verify the password
        if Argon2::default()
            .verify_password(master_password.as_bytes(), &parsed_hash)
            .is_ok()
        {
            // derive key from password + salt
            let salt = self.get_salt();
            let mut key = [0u8; 32];
            Argon2::default()
                .hash_password_into(master_password.as_bytes(), &salt, &mut key)
                .unwrap();

            // store and populate
            self.key = key;
            self.unlocked = true;
            let _ = self.populate_secrets();

            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn get_salt(&self) -> [u8; 16] {
        let hash = fs::read_to_string(&self.master_password_file).expect("Should have read file");
        let hash_obj = PasswordHash::new(&hash).unwrap();
        match hash_obj.salt {
            Some(salt) => {
                let mut buf = [0u8; 16];
                salt.decode_b64(&mut buf).expect("Invalid salt");
                buf
            }
            None => panic!("No salt"),
        }
    }

    fn generate_password(length: u8, symbols: bool) -> String {
        let distr = Uniform::try_from(33..127).unwrap();
        let mut rng = rand::rng();
        let mut password = String::new();
        if symbols {
            while password.len() as u8 != length {
                password.push(distr.sample(&mut rng) as u8 as char);
            }
        }
        if !symbols {
            while password.len() as u8 != length {
                password.push(rng.sample(rand::distr::Alphanumeric) as char);
            }
        }
        password
    }

    pub fn set_master_password(&mut self) {
        let password = self.scratch.clone();
        if let Some(parent) = &self.master_password_file.parent() {
            fs::create_dir_all(parent).expect("Couldn't create parent directories");
        }
        let salt = SaltString::generate(&mut OsRng08);
        let hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap();

        let mut text = String::new();
        text.push_str(hash.to_string().as_str());

        match fs::write(&self.master_password_file, text) {
            Ok(_) => (),
            Err(e) => panic!("{}", e),
        }

        let mut key = [0u8; 32];
        let salt = self.get_salt();
        Argon2::default()
            .hash_password_into(password.as_bytes(), &salt, &mut key)
            .unwrap();

        // store and populate
        self.key = key;
    }

    fn init(&mut self) {
        let contents = fs::read_to_string(&self.master_password_file);
        match contents {
            Ok(text) => {
                if text.is_empty() {
                    self.current_screen = CurrentScreen::Init;
                }
            }
            _ => {
                self.current_screen = CurrentScreen::Init;
            }
        }
    }

    fn populate_secrets(&mut self) -> std::io::Result<()> {
        let file_contents = fs::read_to_string(&self.password_store).unwrap();
        let encrypted_secrets: Vec<EncryptedSecret> = serde_json::from_str(&file_contents).unwrap();
        self.secrets = encrypted_secrets
            .iter()
            .map(|es| es.decrypt(self.key))
            .collect();
        Ok(())
    }

    /// Find credentials for a given domain
    /// Returns (username, password) if found
    pub fn get_credentials_for_domain(&self, domain: &str) -> Option<(String, String)> {
        // Normalize the domain (remove protocol, www, etc.)
        let normalized_domain = domain
            .trim()
            .to_lowercase()
            .replace("https://", "")
            .replace("http://", "")
            .replace("www.", "")
            .replace(".com", "")
            .split('/')
            .next()
            .unwrap_or("")
            .to_string();

        // Search through secrets for a match
        for secret in &self.secrets {
            let secret_name = secret.get_name().to_lowercase();

            // Check if the secret name contains the domain
            if secret_name.contains(&normalized_domain) {
                let contents = secret.get_contents();

                // Look for username and password fields
                let mut username = None;
                let mut password = None;

                for pair in contents {
                    let key_lower = pair.key.to_lowercase();
                    if key_lower == "username" || key_lower == "user" || key_lower == "email" {
                        username = Some(pair.value.clone());
                    } else if key_lower == "password" || key_lower == "pass" {
                        password = Some(pair.value.clone());
                    }
                }

                // If we found both, return them
                if let (Some(u), Some(p)) = (username, password) {
                    return Some((u, p));
                }
            }
        }

        None
    }

    pub fn search_secrets(&mut self) {
        let input = &self.scratch;
        self.search_buffer = self
            .secrets
            .iter()
            .enumerate()
            .filter(|(_, secret)| secret.get_name().contains(input)) // if get_name() is a method, use secret.get_name().contains(input)
            .map(|(i, _)| i)
            .collect();
        if !self.search_buffer.is_empty() {
            self.currently_selected_secret_idx =
                Some(*self.search_buffer.front().expect("Will never be empty"));
        }
    }

    pub fn increment_search_buffer(&mut self) {
        if !self.search_buffer.is_empty() {
            let first_element = self.search_buffer.pop_front().expect("Will never be empty");
            self.search_buffer.push_back(first_element);
            self.currently_selected_secret_idx =
                Some(*self.search_buffer.front().expect("Will never be empty"));
        }
    }

    pub fn add_pair(&mut self) {
        let pair = Pair {
            key: self.key_input.clone(),
            value: self.value_input.clone(),
        };
        if !pair.key.is_empty() {
            self.secret_scratch_content.push(pair);
        }
    }

    pub fn delete_pair(&mut self) {
        match self.currently_editing {
            Some(CurrentlyEditing::Key(idx)) | Some(CurrentlyEditing::Value(idx)) => {
                if idx < self.secret_scratch_content.len() {
                    self.secret_scratch_content.remove(idx);
                    self.update_secret();
                }
            }
            _ => (),
        }
    }

    pub fn save_secret(&mut self) {
        if !&self.name_input.is_empty() {
            let secret = Secret::new(&self.name_input, self.secret_scratch_content.clone());
            self.secrets.push(secret);
            self.write_secrets_to_disk();
        }
    }

    pub fn write_secrets_to_disk(&mut self) {
        let encrypted_secrets: Vec<EncryptedSecret> = self
            .secrets
            .iter()
            .map(|secret| secret.encrypt(self.key))
            .collect();
        let file_content = serde_json::to_string(&encrypted_secrets).unwrap();
        let _ = fs::write(&self.password_store, file_content);
    }

    pub fn delete_secret(&mut self) {
        match self.currently_selected_secret_idx {
            Some(current_idx) => {
                let _ = self.secrets.remove(current_idx);
                self.write_secrets_to_disk();
            }
            None => (),
        }
    }

    pub fn load_secret(&mut self) {
        match self.currently_selected_secret_idx {
            Some(current_idx) => {
                if let Some(secret) = self.secrets.get(current_idx) {
                    self.name_input = String::from(secret.get_name());
                    self.secret_scratch_content = secret.get_contents();
                }
            }
            _ => (),
        }
    }

    pub fn update_secret(&mut self) {
        //Delete secret
        self.delete_secret();
        //Resave with new values
        self.save_secret()
    }

    pub fn increment_currently_editing(&mut self) {
        match self.currently_editing {
            None => self.currently_editing = Some(CurrentlyEditing::Name),
            Some(CurrentlyEditing::Name) => self.currently_editing = Some(CurrentlyEditing::Key(0)),
            Some(CurrentlyEditing::Key(idx)) => {
                self.currently_editing = Some(CurrentlyEditing::Value(idx))
            }
            Some(CurrentlyEditing::Value(idx)) => {
                if idx == self.secret_scratch_content.len() {
                    self.currently_editing = Some(CurrentlyEditing::Name)
                } else {
                    self.currently_editing = Some(CurrentlyEditing::Key(idx + 1))
                }
            }
        }
    }

    pub fn decrement_currently_editing(&mut self) {
        match self.currently_editing {
            None => {
                self.currently_editing = Some(CurrentlyEditing::Value(
                    self.secret_scratch_content.len() - 1,
                ))
            }
            Some(CurrentlyEditing::Name) => {
                self.currently_editing =
                    Some(CurrentlyEditing::Value(self.secret_scratch_content.len()))
            }
            Some(CurrentlyEditing::Key(idx)) => {
                if idx == 0 {
                    self.currently_editing = Some(CurrentlyEditing::Name)
                } else {
                    self.currently_editing = Some(CurrentlyEditing::Value(idx - 1))
                }
            }
            Some(CurrentlyEditing::Value(idx)) => {
                self.currently_editing = Some(CurrentlyEditing::Key(idx))
            }
        }
    }

    pub fn clear_input_fields(&mut self) {
        self.currently_selected_secret_idx = None;
        self.currently_editing = None;
        self.name_input.clear();
        self.key_input.clear();
        self.value_input.clear();
        self.secret_scratch_content.clear();
        self.scratch.clear();
        self.search_buffer.clear();
    }

    pub fn clear_key_value_fields(&mut self) {
        self.key_input.clear();
        self.value_input.clear();
    }

    pub fn select_new_secret(&mut self, input: KeyCode) {
        let len = self.secrets.len();
        if len == 0 {
            return;
        }

        self.currently_selected_secret_idx = Some(match self.currently_selected_secret_idx {
            None => 0,
            Some(current_idx) => match input {
                KeyCode::Left => {
                    if current_idx == 0 {
                        len - 1
                    } else {
                        current_idx - 1
                    }
                }
                KeyCode::Right => (current_idx + 1) % len,
                KeyCode::Down => (current_idx + self.secrets_per_row) % len,
                KeyCode::Up => {
                    if current_idx < self.secrets_per_row {
                        // wrap to bottom row
                        (len + current_idx).saturating_sub(self.secrets_per_row) % len
                    } else {
                        current_idx - self.secrets_per_row
                    }
                }
                _ => current_idx,
            },
        });
    }

    pub fn select_new_pair(&mut self, input: KeyCode) {
        let len = self.secret_scratch_content.len();
        let current = self
            .currently_editing
            .clone()
            .unwrap_or(CurrentlyEditing::Name);

        let next = match current {
            CurrentlyEditing::Name => match input {
                KeyCode::Down => CurrentlyEditing::Key(0),
                KeyCode::Up => CurrentlyEditing::Key(len),
                _ => CurrentlyEditing::Name,
            },
            CurrentlyEditing::Key(idx) | CurrentlyEditing::Value(idx) => match input {
                KeyCode::Down => {
                    if idx == len {
                        CurrentlyEditing::Name
                    } else {
                        CurrentlyEditing::Key(idx + 1)
                    }
                }
                KeyCode::Up => {
                    if idx == 0 {
                        CurrentlyEditing::Name
                    } else {
                        CurrentlyEditing::Key(idx - 1)
                    }
                }
                KeyCode::Left => CurrentlyEditing::Key(idx),
                KeyCode::Right => CurrentlyEditing::Value(idx),
                _ => CurrentlyEditing::Key(idx),
            },
        };

        self.currently_editing = Some(next);
    }
}
