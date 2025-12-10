# Installation

Currently, Aegis is distributed as source code. To install it, you need to compile the project locally using **Rust**.

## Prerequisites

Before proceeding, ensure you have the following installed on your machine:

- `Git`: To clone the repository. Download Git
- `Rust` & `Cargo`: To build the language. The easiest way to install Rust is via rustup.
    - Linux / macOS: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - Windows: Download `rustup-init.exe` from rust-lang.org.

## Step-by-Step Guide
### 1. Clone the Repository

Open your terminal and clone the Aegis source code:

```bash
git clone https://github.com/AegisProgrammingLanguage/AegisProgrammingLanguage.git
cd aegis
```

### 2. Build and Install

Use `cargo` to compile the project in release mode and install the binary globally on your system.

```bash
cargo install --path .
```

This process might take a minute or two as it downloads dependencies (like `tokio`, `reqwest`, etc.) and optimizes the build.

### 3. Verify Installation

Once the compilation is complete, `cargo` places the aegis binary in your generic Cargo bin folder (usually `~/.cargo/bin`). This folder should be in your system `PATH` automatically if you installed Rust via `rustup`.

To verify that Aegis is correctly installed, run:

```bash
aegis --version
```

You should see an output similar to:

```
aegis 0.2.0
```

## Troubleshooting

"Command not found: aegis"

If the installation finished successfully but the command is not found:
- Ensure `~/.cargo/bin` (Linux/Mac) or `%USERPROFILE%\.cargo\bin` (Windows) is in your system **PATH**.
- Restart your terminal session to refresh the environment variables.

## Build Errors on Linux

Since Aegis depends on `openssl` (via the HTTP module) and sometimes system libraries, you might need to install development packages on Linux (Ubuntu/Debian example):

```bash
sudo apt update
sudo apt install build-essential pkg-config libssl-dev
```

## Updating Aegis

To update to the latest version of Aegis, pull the latest changes from Git and force a re-installation:

```bash
git pull origin master
cargo install --path . --force
```
