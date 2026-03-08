# RustKanban development tasks

# Build the workspace
build:
    cargo build

# Run tests (optionally pass test name: just test my_test)
test *args:
    cargo test {{args}}

# Run clippy lints (must pass with zero warnings)
lint:
    cargo clippy -- -D warnings

# Format code
fmt:
    cargo fmt

# Run all checks (mirrors CI: format + lint + test)
check:
    cargo fmt -- --check
    cargo clippy -- -D warnings
    cargo test

# Install the TUI client locally as `rk`
install:
    cargo install --path crates/rk-client

# Create a release (updates versions, commits, tags)
release version:
    ./scripts/release.sh {{version}}

# Start local dev server (PostgreSQL + Axum on port 3000)
dev:
    docker compose up --build

# Regenerate the demo GIF (requires vhs: https://github.com/charmbracelet/vhs)
demo:
    vhs demo.tape

# Preview the man page
manpage:
    cargo run --package rustkanban -- manpage | man -l -

# Run server tests
test-server:
    cargo test --package rk-server

# Clean build artifacts
clean:
    cargo clean
