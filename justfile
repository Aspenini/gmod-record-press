set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

default:
    @just --list

# Install JS dependencies
install:
    bun install

# Run the desktop app
dev: install
    bun run tauri dev

# Release app and installer
build: install
    # linuxdeploy's bundled strip cannot handle RELR (.relr.dyn) on modern glibc.
    {{ if os() == "linux" { "NO_STRIP=1" } else { "" } }} bun run tauri build

# Rust tests
test:
    cargo test --manifest-path src-tauri/Cargo.toml

# Remove Rust build output
clean:
    cargo clean --manifest-path src-tauri/Cargo.toml
