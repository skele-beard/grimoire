use crate::secret;

use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
};
use crossterm::event::KeyCode;
use rand::distr::{Distribution, Uniform};
use rand::prelude::*;
use rand_argon_compatible::rngs::OsRng as OsRng08;
use secret::{EncryptedSecret, Pair, Secret};
use std::fs;
use std::path::PathBuf;

pub enum CurrentScreen {
    Main,
    Editing,
    New,
    Login,
    Init,
}

pub enum CurrentlyEditing {
    Name,
    Key,
    Value,
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
    pub scratch: String,

    key: [u8; 32],
}

#[allow(clippy::single_match)]
impl App {
    pub fn new(password_attempt: &str) -> App {
        let mut app = App {
            secrets: Vec::new(),
            secret_scratch_content: Vec::new(),
            current_screen: CurrentScreen::Login,
            currently_selected_secret_idx: None,
            currently_editing: None,
            master_password_file: PathBuf::from(
                "/home/chandler/grimoire/password_store/master.txt",
            ),
            password_store: PathBuf::from("/home/chandler/grimoire/password_store/"),
            secrets_per_row: 3,
            name_input: String::from(""),
            key_input: String::new(),
            value_input: String::new(),
            scratch: String::new(),
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
            let _ = self.populate_secrets(key);

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
            //.hash_password(self.scratch.as_bytes(), &salt)
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

    fn populate_secrets(&mut self, key: [u8; 32]) -> std::io::Result<()> {
        for entry in fs::read_dir(self.password_store.clone())? {
            let entry = entry?;
            let path = entry.path();
            if path == self.master_password_file {
                continue;
            }
            let secret: Secret = EncryptedSecret::decrypt(key, path);
            self.secrets.push(secret);
        }
        Ok(())
    }

    pub fn add_pair(&mut self) {
        let pair = Pair {
            key: self.key_input.clone(),
            value: self.value_input.clone(),
        };
        self.secret_scratch_content.push(pair);
    }

    pub fn save_secret(&mut self) {
        let secret = Secret::new(&self.name_input, self.secret_scratch_content.clone());
        secret.save(self.key, self.password_store.clone());
        self.secrets.push(secret);
    }

    // The only reason this method needs to exist is if the name is changed - we don't want the old
    // secret lingering around
    pub fn update_secret(&mut self) {
        //Delete secret
        match self.currently_selected_secret_idx {
            Some(current_idx) => {
                if current_idx < self.secrets.len() {
                    let secret = self.secrets.remove(current_idx);
                    let filepath = self.password_store.clone();
                    secret.delete(filepath);
                }
            }
            _ => {}
        }
        //
        //Resave with new values
        self.save_secret()
    }

    pub fn increment_currently_editing(&mut self) {
        match self.currently_editing {
            None => self.currently_editing = Some(CurrentlyEditing::Name),
            Some(CurrentlyEditing::Name) => self.currently_editing = Some(CurrentlyEditing::Key),
            Some(CurrentlyEditing::Key) => self.currently_editing = Some(CurrentlyEditing::Value),
            Some(CurrentlyEditing::Value) => self.currently_editing = Some(CurrentlyEditing::Name),
        }
    }

    pub fn decrement_currently_editing(&mut self) {
        match self.currently_editing {
            None => self.currently_editing = Some(CurrentlyEditing::Value),
            Some(CurrentlyEditing::Name) => self.currently_editing = Some(CurrentlyEditing::Value),
            Some(CurrentlyEditing::Key) => self.currently_editing = Some(CurrentlyEditing::Name),
            Some(CurrentlyEditing::Value) => self.currently_editing = Some(CurrentlyEditing::Key),
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
    }

    pub fn populate_input_fields_from_secret(&mut self) {
        match self.currently_selected_secret_idx {
            Some(current_idx) => {
                if let Some(secret) = self.secrets.get(current_idx) {
                    self.name_input = String::from(secret.get_name());
                    self.secret_scratch_content = secret.get_contents();
                }
            }
            _ => {}
        }
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
}
