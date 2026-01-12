use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Deserialize, Serialize)]
pub enum BrowserType {
    Firefox,
    Chrome,
}

#[derive(Deserialize, Serialize)]
pub struct Config {
    pub master_password_file: PathBuf,
    pub password_store: PathBuf,
    pub extension: bool,
    pub browser: Option<BrowserType>,
    pub secrets_per_row: usize,
    pub password_generator_length: u8,
    pub password_generator_symbols: bool,
}

impl Config {
    pub fn load() -> Self {
        // ############## Update this function to auto-create the native messaging manifest based
        // on the extension flag and the browser type. ################
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
            if let Ok(cfg) = toml::from_str::<Config>(&content) {
                let _ = cfg.install_native_messaging();
                return cfg;
            } else {
                eprintln!("Invalid config format, using defaults");
            }
        }

        // if we reach this point, we need to create the config file and use defaults
        let base_dir = config_dir().unwrap_or_else(|| PathBuf::from("."));
        let config = Self {
            master_password_file: base_dir.join("grimoire/password_store/master_password"),
            password_store: base_dir.join("grimoire/password_store.json"),
            extension: false,
            browser: None,
            secrets_per_row: 3,
            password_generator_length: 16,
            password_generator_symbols: true,
        };

        let config_string = toml::to_string(&config).unwrap();
        // ####### update this to only write if there was no config rather than overwriting and
        // invalid config.
        let _ = fs::write(config_path, config_string);
        config
    }

    fn install_native_messaging(&self) -> Result<(), Box<dyn Error>> {
        let binary_path = std::env::current_exe()?;

        #[cfg(windows)]
        let forwarder_path = binary_path.parent().unwrap().join("grimoire-forwarder.exe");

        #[cfg(not(windows))]
        let forwarder_path = binary_path.parent().unwrap().join("grimoire-forwarder");

        match self.browser {
            Some(BrowserType::Firefox) => {
                #[cfg(target_os = "linux")]
                let manifest_dir = dirs::home_dir()
                    .unwrap()
                    .join(".mozilla/native-messaging-hosts");

                #[cfg(target_os = "macos")]
                let manifest_dir = dirs::home_dir()
                    .unwrap()
                    .join("Library/Application Support/Mozilla/NativeMessagingHosts");

                #[cfg(target_os = "windows")]
                let manifest_dir = dirs::config_dir()
                    .unwrap()
                    .join("Mozilla\\NativeMessagingHosts");

                std::fs::create_dir_all(&manifest_dir)?;

                // Windows paths need escaped backslashes in JSON
                #[cfg(windows)]
                let path_str = forwarder_path.display().to_string().replace("\\", "\\\\");

                #[cfg(not(windows))]
                let path_str = forwarder_path.display().to_string();

                let manifest_content = format!(
                    r#"{{
  "name": "com.grimoire.native",
  "description": "Grimoire Password Manager",
  "path": "{}",
  "type": "stdio",
  "allowed_extensions": ["grimoire@yourdomain.com"]
}}"#,
                    path_str
                );

                let manifest_path = manifest_dir.join("com.grimoire.native.json");
                std::fs::write(&manifest_path, manifest_content)?;
            }
            Some(BrowserType::Chrome) => {
                #[cfg(target_os = "linux")]
                let chrome_dir = dirs::home_dir()
                    .unwrap()
                    .join(".config/google-chrome/NativeMessagingHosts");

                #[cfg(target_os = "macos")]
                let chrome_dir = dirs::home_dir()
                    .unwrap()
                    .join("Library/Application Support/Google/Chrome/NativeMessagingHosts");

                #[cfg(target_os = "windows")]
                let chrome_dir = dirs::config_dir()
                    .unwrap()
                    .join("Google\\Chrome\\NativeMessagingHosts");

                std::fs::create_dir_all(&chrome_dir)?;

                // Windows paths need escaped backslashes in JSON
                #[cfg(windows)]
                let path_str = forwarder_path.display().to_string().replace("\\", "\\\\");

                #[cfg(not(windows))]
                let path_str = forwarder_path.display().to_string();

                let manifest_content = format!(
                    r#"{{
  "name": "com.grimoire.native",
  "description": "Grimoire Password Manager",
  "path": "{}",
  "type": "stdio",
  "allowed_origins": [
    "chrome-extension://grimoire@yourdomain.com/"
  ]
}}"#,
                    path_str
                );

                let chrome_manifest = chrome_dir.join("com.grimoire.native.json");
                std::fs::write(&chrome_manifest, &manifest_content)?;
            }
            None => eprintln!(
                "Extension is enabled but no browser type is recognized. Please set the browser variable to \"Firefox\" or \"Chrome\" in your config."
            ),
        }
        Ok(())
    }
}
