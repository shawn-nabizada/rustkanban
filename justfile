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

# Start local dev server (PostgreSQL + Axum on port 3000, everything in Docker)
dev:
    docker compose up --build

# Start just the PostgreSQL database for local development
dev-db:
    docker compose up -d db
    @echo "Waiting for PostgreSQL..."
    @until docker compose exec -T db pg_isready -U rustkanban -q 2>/dev/null; do sleep 1; done
    @echo "PostgreSQL is ready (localhost:5433)"

# Regenerate the demo GIF (requires vhs: https://github.com/charmbracelet/vhs)
demo:
    vhs demo.tape

# Preview the man page
manpage:
    cargo run --package rustkanban -- manpage | man -l -

# Run server tests
test-server:
    cargo test --package rk-server

# Install frontend dependencies
setup-frontend:
    cd crates/rk-server/frontend && npm install

# Build the frontend (Svelte SPA)
build-frontend:
    cd crates/rk-server/frontend && npm run build

# Start PostgreSQL + Axum + Vite for web development (single command)
# Requires GitHub OAuth configured in .env (see crates/rk-server/.env.example)
dev-web:
    #!/usr/bin/env bash
    set -e

    # Start PostgreSQL if not running
    docker compose up -d db
    echo "Waiting for PostgreSQL..."
    until docker compose exec -T db pg_isready -U rustkanban -q 2>/dev/null; do
        sleep 1
    done
    echo "PostgreSQL is ready."

    # Install frontend deps if needed
    if [ ! -d crates/rk-server/frontend/node_modules ]; then
        echo "Installing frontend dependencies..."
        (cd crates/rk-server/frontend && npm install)
    fi

    # Set DATABASE_URL for local dev (port 5433 = Docker-mapped PostgreSQL)
    export DATABASE_URL="${DATABASE_URL:-postgres://rustkanban:rustkanban@localhost:5433/rustkanban}"
    export SESSION_SECRET="${SESSION_SECRET:-dev-secret-change-me-in-production}"
    export SERVER_URL="${SERVER_URL:-http://localhost:3000}"
    export PORT="${PORT:-3000}"

    echo ""
    echo "  Web UI (HMR):  http://localhost:5173/app"
    echo "  API server:    http://localhost:3000"
    echo "  Press Ctrl+C to stop both servers"
    echo ""

    # Start Vite dev server in background (HMR + proxy to Axum)
    (cd crates/rk-server/frontend && npm run dev) &
    VITE_PID=$!
    trap "kill $VITE_PID 2>/dev/null; echo 'Stopped.'" EXIT

    # Start Axum server in foreground
    cargo run -p rk-server

# Clean build artifacts
clean:
    cargo clean
