# Grimoire Password Manager

A secure, modern password manager built with simplicity and security at its core.

## Overview

Grimoire is a password management solution designed to help you securely store and manage your passwords, credentials, and sensitive information. With a focus on user privacy and strong encryption, Grimoire keeps your data safe while remaining easy to use.

## Features

- **Secure Encryption**: All passwords are encrypted using AES256, a trusted and standard scheme.
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

- Master password is never stored - only a secure hash is kept
- All password data is encrypted at rest
- No telemetry or data collection

## Requirements

- cargo

## License

[GPL]

## Disclaimer

While Grimoire implements strong security measures, this is ultimately a hobbyist project. Use at your own discretion.

---

**Never forget your master password - it cannot be recovered!**
