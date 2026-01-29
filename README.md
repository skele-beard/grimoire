# Grimoire Password Manager

A secure, modern password manager built with simplicity and security at its core.

## Overview

Grimoire is a password management solution designed to help you securely store and manage your passwords, credentials, and sensitive information. With a focus on user privacy and strong encryption, Grimoire keeps your data safe while remaining easy to use.
<img width="3750" height="1970" alt="grimoire_login" src="https://github.com/user-attachments/assets/cc2f00e0-8679-4620-9b91-5985d9ee3c59" />
<img width="3760" height="1986" alt="grimoire_main_view" src="https://github.com/user-attachments/assets/2d98a486-b49b-45bb-b871-a2e437d73f6a" />
<img width="2882" height="898" alt="grimoire_edit_view" src="https://github.com/user-attachments/assets/9cefbb1d-21fe-4111-83c7-d433feaa99e0" />


## Features

- **Secure Encryption**: All passwords are encrypted using AES-256-GCM, a trusted and standard scheme.
- **Master Password**: Single master password to access all your stored credentials
- **Password Generation**: Built-in strong password generator for creating secure passwords
- **Cross-Platform**: Works seamlessly across different devices and operating systems
- **Local Storage**: Your data stays on your device, no phoning home.

## Installation

```bash
# Clone the repository
git clone https://github.com/skele-beard/grimoire.git

# Navigate to the project directory
cd grimoire

# Build the binary
cargo build --release

# Run the application
./target/release/grimoire
```

## Security

- Master password is never stored - only a secure hash (Argon2id) is kept
- All password data is encrypted at rest
- No telemetry or data collection

## Requirements

- cargo

## Disclaimer

While Grimoire implements strong security measures, this is ultimately a hobbyist project. Use at your own discretion.

---

**Never forget your master password - it cannot be recovered!**
