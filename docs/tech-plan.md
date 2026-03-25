# RustKanban Tech Plan (Reverse-Engineered)

> **Generated:** 2026-03-24 | **Updated:** 2026-03-24
> **Source:** Codebase analysis of RustKanban v0.2.0
> **Status:** Reverse-engineered from existing code, not a forward-looking plan

---

## Stack Summary

| Layer | Technology | Version | Purpose |
|-------|-----------|---------|---------|
| **Language** | Rust | 2021 edition | All crates |
| **TUI Framework** | ratatui | 0.29 | Terminal rendering |
| **Terminal I/O** | crossterm | 0.28 | Input events, mouse capture, raw mode |
| **Client DB** | rusqlite (bundled SQLite) | 0.32 | Local task storage |
| **Server Framework** | Axum | 0.8 | HTTP/REST API |
| **Async Runtime** | Tokio | 1.x (full) | Server async I/O |
| **Server DB** | PostgreSQL 16 + sqlx | 0.8 | Server data persistence |
| **Auth** | GitHub OAuth (oauth2 crate) | 5.x | User identity |
| **Sessions** | tower-sessions + sqlx-store | 0.14 | Web session management |
| **HTTP Client** | ureq | 3.x | Client sync requests |
| **Frontend** | Svelte 4 + Vite 5 | — | Web SPA |
| **Frontend DnD** | svelte-dnd-action | — | Drag-and-drop |
| **Frontend Anim** | GSAP | — | Animations |
| **CLI** | clap (derive) | 4.5 | Argument parsing |
| **Serialization** | serde + serde_json | 1.x | JSON everywhere |
| **Config** | toml | 0.8 | Theme/keybinding files |
| **Dates** | chrono | 0.4 | Timestamps, due dates |
| **UUIDs** | uuid (v4) | 1.x | Entity identity |
| **Hashing** | sha2 | 0.10 | Token hashing |
| **Rate Limiting** | governor | 0.8 | IP-based request rate limiting |
| **Directories** | dirs | 6.0 | XDG paths |
| **Build** | Cargo workspace | — | Multi-crate build |
| **Task Runner** | just | — | Dev workflow commands |
| **Containerization** | Docker + docker-compose | — | Server deployment |
| **CI/CD** | GitHub Actions | — | Build, test, release, deploy |
| **Hosting** | DigitalOcean (SSH deploy) | — | Production server |

---

## Architecture Overview

### Crate Structure

```
RustKanban (Cargo workspace)
├── crates/rk-client    → Binary: rk (TUI + CLI)
├── crates/rk-server    → Binary: rk-server (Axum web server)
└── crates/rk-shared    → Library: sync protocol types
```

### Component Diagram

```
┌─────────────────────────────────────────────────────────┐
│                     User's Terminal                      │
│  ┌───────────────────────────────────────────────────┐  │
│  │                  rk-client (TUI)                   │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────────────┐  │  │
│  │  │ handler  │→│   app    │→│     ui/          │  │  │
│  │  │ (input)  │ │ (state)  │ │   (rendering)    │  │  │
│  │  └──────────┘ └────┬─────┘ └──────────────────┘  │  │
│  │                     │                              │  │
│  │              ┌──────┴──────┐                       │  │
│  │              │    db.rs    │                       │  │
│  │              │  (SQLite)   │                       │  │
│  │              └──────┬──────┘                       │  │
│  │                     │                              │  │
│  │         ~/.local/share/rustkanban/kanban.db        │  │
│  └───────────────┬───────────────────────────────────┘  │
│                  │ (opt-in sync via ureq)                │
└──────────────────┼──────────────────────────────────────┘
                   │ HTTPS (Bearer token)
                   ▼
┌──────────────────────────────────────────────────────────┐
│                   rk-server (Axum)                        │
│  ┌────────────┐ ┌────────────┐ ┌──────────────────────┐ │
│  │  routes/   │ │   auth/    │ │    session/          │ │
│  │  sync.rs   │ │  session   │ │  tower-sessions      │ │
│  │  web.rs    │ │  tokens    │ │  (PostgreSQL store)  │ │
│  │  shares.rs │ │            │ │                      │ │
│  └──────┬─────┘ └────────────┘ └──────────────────────┘ │
│         │                                                 │
│    ┌────┴────┐      ┌────────────────────────┐           │
│    │  sqlx   │──────│   PostgreSQL 16        │           │
│    │ (async) │      │   (users, devices,     │           │
│    └─────────┘      │    tasks, tags, boards,│           │
│                     │    shares, sessions)   │           │
│    ┌────────────────────────────────┐         └──────────┘│
│    │  frontend/ (Svelte SPA)       │                     │
│    │  Served as static files at /  │                     │
│    └────────────────────────────────┘                     │
└──────────────────────────────────────────────────────────┘
```

### Key Architectural Patterns

| Pattern | Where | Description |
|---------|-------|-------------|
| **State machine** | `AppMode` enum | 13 states drive which handler + UI overlay is active. All mode transitions are explicit. |
| **DB-first mutation** | `app/mod.rs` + `db.rs` | All data changes write to SQLite first, then call `reload_tasks()`. No in-memory divergence from DB. |
| **Event loop** | `main.rs` | `render → poll(100ms) → handle → tick → repeat`. Single-threaded, no async in client. |
| **Soft deletes** | All entities | `deleted` flag + `deleted_at` timestamp. Required for sync protocol. |
| **Last-write-wins** | Sync protocol | Conflict resolution by comparing `updated_at` timestamps. Simple but lossy. |
| **UUID identity** | All entities | v4 UUIDs for cross-device identity. Auto-generated on insert. |
| **Configurable input** | `KeyMap` | Action enum → KeyEvent mapping. Two contexts (Board, Modal). TOML config. |
| **Theme system** | `Theme` struct | 16 named colors. TOML config. In-app editing with presets. |
| **Shared types** | `rk-shared` crate | Sync protocol types shared between client and server. Avoids drift. |
| **Rate limiting** | `rate_limit.rs` | IP-based rate limiting via `governor`. Two tiers: strict (30/min) for auth/shared routes, standard (120/min) global. |
| **CSRF protection** | `csrf.rs` | Requires `X-Requested-With` header on mutation requests. Skips Bearer-token requests. Blocks cross-origin form submissions. |

---

## Data Model

### Client (SQLite)

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    boards    │     │    tasks     │     │    tags      │
├──────────────┤     ├──────────────┤     ├──────────────┤
│ id (PK)      │◄────│ board_id (FK)│     │ id (PK)      │
│ uuid         │     │ id (PK)      │     │ uuid         │
│ name         │     │ uuid         │     │ name         │
│ position     │     │ title        │     │ updated_at   │
│ created_at   │     │ description  │     │ deleted      │
│ updated_at   │     │ priority     │     │ deleted_at   │
│ deleted      │     │ column_name  │     └──────┬───────┘
│ deleted_at   │     │ due_date     │            │
└──────────────┘     │ created_at   │     ┌──────┴───────┐
                     │ updated_at   │     │  task_tags   │
                     │ deleted      │     ├──────────────┤
                     │ deleted_at   │◄────│ task_id (FK) │
                     └──────────────┘     │ tag_id (FK)  │
                                          └──────────────┘

┌──────────────┐
│ preferences  │  (key-value store)
├──────────────┤
│ key (PK)     │  sort_mode, focused_column,
│ value        │  active_board_uuid, schema_version
└──────────────┘
```

**Schema migrations:** v1 (initial) → v2 (backfill UUIDs) → v3 (add boards, backfill "Personal" board). Auto-run on startup.

### Server (PostgreSQL)

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│    users     │     │   devices    │     │ auth_tokens  │
├──────────────┤     ├──────────────┤     ├──────────────┤
│ id (UUID PK) │◄────│ user_id (FK) │◄────│ device_id FK │
│ github_id    │     │ id (UUID PK) │     │ token_hash PK│
│ username     │     │ name         │     │ user_id (FK) │
│ email        │     │ last_synced  │     │ expires_at   │
│ created_at   │     │ stale        │     │ created_at   │
└──────┬───────┘     │ created_at   │     └──────────────┘
       │             └──────────────┘
       │
       │  ┌──────────────┐     ┌──────────────┐
       ├──│ api_tokens   │     │    boards    │
       │  ├──────────────┤     ├──────────────┤
       │  │ id (UUID PK) │     │ uuid (PK)    │
       │  │ user_id (FK) │     │ user_id (FK) │◄──┐
       │  │ token_hash   │     │ name         │   │
       │  │ label        │     │ position     │   │
       │  │ last_used_at │     │ created_at   │   │
       │  │ expires_at   │     │ updated_at   │   │
       │  └──────────────┘     │ deleted      │   │
       │                       └──────┬───────┘   │
       │                              │            │
       │  ┌──────────────┐     ┌──────┴───────┐   │
       ├──│    tasks     │     │ board_shares │   │
       │  ├──────────────┤     ├──────────────┤   │
       │  │ uuid (PK)    │     │ id (UUID PK) │   │
       │  │ user_id (FK) │     │ board_uuid FK│───┘
       │  │ board_uuid FK│     │ token        │
       │  │ title        │     │ permission   │  view | edit
       │  │ description  │     │ created_by   │
       │  │ priority     │     │ created_at   │
       │  │ column_name  │     └──────────────┘
       │  │ due_date     │
       │  │ created_at   │     ┌──────────────┐
       │  │ updated_at   │     │  task_tags   │
       │  │ deleted      │◄────│ task_uuid FK │
       │  │ deleted_at   │     │ tag_uuid FK  │──┐
       │  └──────────────┘     └──────────────┘  │
       │                                          │
       │  ┌──────────────┐                        │
       └──│    tags      │◄───────────────────────┘
          ├──────────────┤
          │ uuid (PK)    │
          │ user_id (FK) │
          │ name         │
          │ updated_at   │
          │ deleted      │
          │ deleted_at   │
          └──────────────┘
```

**Indexes:**
- `tasks`: (user_id, updated_at) for sync delta queries
- `tags`: UNIQUE (user_id, name) WHERE deleted = FALSE
- `boards`: UNIQUE (user_id, name) WHERE deleted = FALSE

**Server migrations:** 4 files (initial → api_tokens → boards → board_shares). Auto-run via sqlx on startup.

---

## API Surface

### Authentication

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/login` | None | Initiate GitHub OAuth (query params: redirect_port, device_name, headless) |
| GET | `/auth/callback` | None | GitHub OAuth callback (exchanges code for token) |
| GET | `/login/success` | None | Post-login redirect page |
| GET | `/auth/logout` | Session | Logout (clear session) |

### Sync (Bearer Token — device auth)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/sync/pull` | Bearer | Fetch changes since `last_synced_at` (or full snapshot if stale) |
| POST | `/api/v1/sync/push` | Bearer | Push local changes with conflict resolution |
| POST | `/api/v1/sync` | Bearer | Combined pull + push in one round trip |

### Web CRUD (Session or API Token)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/me` | Session/API | Current user profile |
| GET | `/api/v1/boards` | Session/API | List all boards |
| GET | `/api/v1/boards/{uuid}` | Session/API | Get board with tasks |
| POST | `/api/v1/tasks` | Session/API | Create task |
| PATCH | `/api/v1/tasks/{uuid}` | Session/API | Update task |
| DELETE | `/api/v1/tasks/{uuid}` | Session/API | Delete task |
| POST | `/api/v1/tags` | Session/API | Create tag |
| PATCH | `/api/v1/tags/{uuid}` | Session/API | Update tag |
| DELETE | `/api/v1/tags/{uuid}` | Session/API | Delete tag |

### Account Management (Session only)

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/api/v1/account/devices` | Session | List devices |
| PATCH | `/api/v1/account/devices/{id}` | Session | Rename device |
| DELETE | `/api/v1/account/devices/{id}` | Session | Revoke device |
| GET | `/api/v1/account/tokens` | Session | List API tokens |
| POST | `/api/v1/account/tokens` | Session | Create API token |
| DELETE | `/api/v1/account/tokens/{id}` | Session | Revoke API token |
| GET | `/api/v1/account/export` | Session | Export all data as JSON |
| DELETE | `/api/v1/account` | Session | Delete account (cascade) |
| POST | `/api/v1/auth/logout` | Session | Logout + clear session |

### Board Sharing

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| POST | `/api/v1/boards/{uuid}/shares` | Session | Create share link (view or edit) |
| GET | `/api/v1/boards/{uuid}/shares` | Session | List share links for board |
| DELETE | `/api/v1/shares/{id}` | Session | Revoke share link |
| GET | `/api/v1/shared/{token}` | None | Get shared board (guest access) |
| POST | `/api/v1/shared/{token}/tasks` | None | Create task in shared board (edit permission) |
| PATCH | `/api/v1/shared/{token}/tasks/{uuid}` | None | Update task in shared board |
| DELETE | `/api/v1/shared/{token}/tasks/{uuid}` | None | Delete task in shared board |

### Infrastructure

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | `/health` | None | Health check |

---

## Infrastructure & CI/CD

### Environments

| Environment | Database | Server | Frontend |
|-------------|----------|--------|----------|
| **Development** | PostgreSQL 16 via docker-compose (port 5433) | `cargo run` or `just dev` (port 3000) | Vite dev server (port 5173, proxies API to 3000) |
| **Production** | PostgreSQL 16 in Docker on DigitalOcean | Docker container (3-stage build: Node.js frontend → Rust → Debian runtime) | Built from source in Docker, served by Axum as static files |

### CI/CD Pipeline (GitHub Actions)

**ci.yml** — On every PR and push to main:
1. `cargo build` — compile all crates
2. `cargo clippy -- -D warnings` — lint with zero warnings
3. `cargo fmt -- --check` — verify formatting
4. `cargo test` — run all tests

**release.yml** — On tag `v*` or manual dispatch:
1. Resolve version from tag
2. Build for 5 targets: Linux x86_64, Linux ARM64, macOS x86_64, macOS ARM64, Windows x86_64
3. Publish `rk-shared` then `rk` to crates.io
4. Create GitHub release with binaries + checksums

**deploy.yml** — On push to main (or server changes):
1. SSH into DigitalOcean
2. `git pull` on production server
3. `docker compose up -d --build --force-recreate server`

### Docker

- **Server Dockerfile**: 3-stage build (Node.js 22 frontend build → Rust builder → Debian Bookworm slim runtime). Frontend `dist/` is no longer committed to git — built from source during Docker build.
- **docker-compose.yml**: PostgreSQL 16 + server service
- PostgreSQL health check before server starts
- 10 MB request body limit
- Migrations auto-run on server startup

### Monitoring & Observability

- **Logging**: `tracing` + `tracing-subscriber` (configurable via `RUST_LOG` env var)
- **Health check**: `GET /health` endpoint
- **No metrics/alerting**: No Prometheus, Grafana, or error tracking (Sentry, etc.)
- **No structured logging**: Uses default tracing format

---

## Architecture Decision Records (Inferred)

### ADR-1: Rust for Both Client and Server

**Decision:** Use Rust for the TUI client, sync server, and shared protocol types.
**Context:** Need a fast TUI with low startup time and a reliable server.
**Rationale:** Single language across the stack. Shared crate (`rk-shared`) prevents protocol drift. Rust's performance is ideal for TUI responsiveness.
**Trade-offs:** Higher development friction than Python/Node for web server. Compile times. Smaller ecosystem for web frameworks.
**Reversibility:** Low — entire codebase is Rust.

### ADR-2: SQLite (Client) + PostgreSQL (Server)

**Decision:** Use SQLite for local client storage and PostgreSQL for the sync server.
**Context:** Client needs offline-first storage. Server needs concurrent multi-user access.
**Rationale:** SQLite is embedded (no install needed), fast for single-user access, and portable. PostgreSQL handles concurrent sync requests and complex queries.
**Trade-offs:** Two different SQL dialects to maintain. Schema migrations are separate systems (manual SQLite vs sqlx PostgreSQL).
**Reversibility:** Medium — would require rewriting DB layers.

### ADR-3: Last-Write-Wins Conflict Resolution

**Decision:** Resolve sync conflicts by comparing `updated_at` timestamps; latest write wins.
**Context:** Need a sync strategy that's simple and predictable.
**Rationale:** Avoids complex CRDT or OT implementations. Users understand "latest change wins." Good enough for single-user multi-device (low conflict rate).
**Trade-offs:** Can lose edits if two devices modify the same task. No field-level merge. No conflict notification to user.
**Reversibility:** Medium — could add CRDT layer later without changing storage.

### ADR-4: GitHub OAuth Only

**Decision:** Only support GitHub as an identity provider.
**Context:** Need authentication for sync without managing passwords.
**Rationale:** Target audience is developers who all have GitHub accounts. Eliminates password storage, reset flows, and email verification. Trusted identity provider.
**Trade-offs:** Locks out non-GitHub users. Single point of failure for auth. GitHub API rate limits.
**Reversibility:** Medium — OAuth2 crate supports other providers; would need UI/flow changes.

### ADR-5: Svelte SPA for Web Frontend

**Decision:** Use Svelte 4 with Vite and client-side routing for the web interface.
**Context:** Need a web UI for browser-based access and board sharing.
**Rationale:** Svelte is lightweight, fast, and produces small bundles. SPA routing avoids server-side rendering complexity. Frontend is built from source during Docker build (Node.js stage).
**Trade-offs:** No SSR (affects SEO, but irrelevant for an app). SPA routing requires Axum fallback handler. Docker build adds a Node.js stage.
**Reversibility:** High — frontend is decoupled; could replace with any framework.

### ADR-6: Soft Deletes Everywhere

**Decision:** Use `deleted` flag + `deleted_at` timestamp instead of hard deletes.
**Context:** Sync protocol needs to propagate deletions to other devices.
**Rationale:** Other devices need to know an entity was deleted (otherwise they'd re-create it on next push). Server purge job cleans up after all devices have synced.
**Trade-offs:** Queries must always filter `WHERE deleted = FALSE`. Database grows without purging. More complex than hard deletes.
**Reversibility:** Low — removing soft deletes would break sync protocol.

### ADR-7: 3 Hardcoded Columns

**Decision:** Fix kanban columns to Todo, In Progress, Done.
**Context:** Simplicity vs flexibility trade-off.
**Rationale:** Covers 95% of personal kanban use cases. Avoids column management UI complexity. Simpler sync (column is a string enum, not a foreign key).
**Trade-offs:** No custom columns. Can't add "Blocked" or "Review" columns. May limit adoption by teams.
**Reversibility:** Medium — would need schema change, UI rework, and sync protocol update.

---

## Gap Analysis

### Critical

All critical issues have been resolved.

| Issue | Status | Resolution |
|-------|--------|------------|
| ~~No rate limiting on API~~ | **FIXED** | Added `governor`-based IP rate limiting: 30 req/min (strict) on auth + shared board routes, 120 req/min (standard) globally. See `rate_limit.rs`. |
| ~~No CSRF protection on shared board mutations~~ | **FIXED** | Added `csrf.rs` middleware requiring `X-Requested-With` header on all mutation requests. Skips Bearer-token requests. Frontend `api.js` updated. |

### High

| Issue | Status | Evidence / Impact |
|-------|--------|-------------------|
| **No monitoring or alerting** | OPEN | No Prometheus metrics, no Sentry/error tracking, no alerting rules. Only basic `tracing` logging. Production issues go undetected until users report them. |
| ~~Committed frontend build artifacts~~ | **FIXED** | `dist/` removed from git, added to `.gitignore`. Dockerfile updated with Node.js build stage. |
| ~~Server integration tests are minimal~~ | **FIXED** | Added 15 integration tests: task CRUD, tag CRUD, board sharing, sync pull/push, CSRF rejection, ownership enforcement. |

### Medium

| Issue | Status | Evidence / Impact |
|-------|--------|-------------------|
| **Client SQLite migrations are manual** | OPEN | No migration framework. Schema versions tracked in preferences table. Migration code is inline in `db.rs`. |
| **No structured error codes in API** | OPEN | Server returns `AppError` with HTTP status + message string. No error code enum for clients. |
| ~~`app.rs` is 2000+ lines~~ | **FIXED** | Split into 7 modules: `app/mod.rs` (787 lines), `modal.rs`, `options.rs`, `tags.rs`, `boards.rs`, `search.rs`, `text_utils.rs`. |
| **No input sanitization on shared board writes** | OPEN | Shared board task creation/update accepts raw input. Svelte auto-escapes, mitigating most XSS risk. |
| **Tag limit of 15 is enforced server-side but not communicated in client** | OPEN | Client has no UI indication of the limit. Server returns a generic error. |
| **No connection pooling configuration exposed** | OPEN | sqlx uses defaults (max_connections=5 configured in main.rs). No min/idle configuration. |
| **Dependabot configured but no auto-merge** | OPEN | Dependencies get PRs but require manual review. |

### Low

| Issue | Evidence | Impact |
|-------|----------|--------|
| **No man page for server** | Only client has `rk manpage`. Server has no CLI documentation. | Minor — server is deployed, not used interactively. |
| **No database backup strategy documented** | No backup script, cron job, or documentation for PostgreSQL backups. | Data loss risk in production. |
| **`update.rs` uses GitHub API without auth** | Rate limited to 60 requests/hour per IP for unauthenticated requests. | Could fail for users behind shared IPs (NAT, offices). |
| **No graceful shutdown for server** | No signal handler for SIGTERM/SIGINT. | In-flight sync requests may be dropped during deployment. Docker stop has a 10s grace period by default, but pending transactions could be lost. |
| **Pre-commit hook not auto-installed** | Requires manual `git config core.hooksPath .githooks`. | Contributors may push unformatted/unlinted code. |

---

## Recommendations (Prioritized)

### P0 — Address Before Next Release

1. ~~**Add rate limiting**~~ — **DONE.** `governor`-based IP rate limiting with strict (30/min) and standard (120/min) tiers.
2. ~~**Add CSRF protection**~~ — **DONE.** `X-Requested-With` header requirement on mutation requests.

### P1 — Address Soon

3. ~~**Remove `dist/` from git**~~ — **DONE.** Added to `.gitignore`, Dockerfile builds frontend from source.
4. **Add basic monitoring**: health check improvements (include DB status), structured JSON logging, and consider a simple error tracking integration.
5. ~~**Increase server test coverage**~~ — **DONE.** Added 15 integration tests covering CRUD, sharing, sync, CSRF.
6. **Communicate server limits in client**: show tag/task/board count and limits in the UI.

### P2 — Technical Debt

7. ~~**Split `app.rs`**~~ — **DONE.** Split into 7 modules under `src/app/`.
8. **Add structured error codes** to API responses for better client-side error handling.
9. **Adopt a migration framework** for client SQLite (e.g., `refinery` or custom versioned system with rollback).
10. **Configure sqlx connection pool** explicitly for production workloads.
11. **Document database backup strategy** and add a backup script.
12. **Add graceful shutdown** to the server with in-flight request draining.

### P3 — Nice to Have

13. **Auto-install pre-commit hooks** (check in `.git/hooks/` or use a setup script).
14. **Add GitHub token to update checker** or switch to authenticated requests.
15. **Add Dependabot auto-merge** for minor/patch updates with green CI.

---

## Directory Structure

```
RustKanban/
├── Cargo.toml                    # Workspace root
├── Cargo.lock
├── CLAUDE.md                     # Dev guide
├── README.md                     # User-facing docs
├── CHANGELOG.md                  # Release notes
├── LICENSE                       # BSL 1.1
├── justfile                      # Task runner
├── docker-compose.yml            # Dev/prod containers
├── install.sh / release.sh       # Scripts
├── demo.tape / demo.gif          # VHS recording
├── aur/PKGBUILD                  # Arch Linux package
├── HomebrewFormula/rk.rb          # Homebrew formula
├── .github/
│   ├── workflows/
│   │   ├── ci.yml                # Format + lint + test
│   │   ├── release.yml           # Multi-platform build + publish
│   │   └── deploy.yml            # DigitalOcean SSH deploy
│   └── dependabot.yml
├── .githooks/
│   └── pre-commit                # fmt + clippy check
├── crates/
│   ├── rk-client/src/            # TUI application
│   │   ├── main.rs               # CLI entry, event loop
│   │   ├── app/                  # App state + business logic (split into modules)
│   │   │   ├── mod.rs            # App struct, core methods, navigation, selection
│   │   │   ├── modal.rs          # Task create/edit modal + text input
│   │   │   ├── options.rs        # Keybinding + theme configuration
│   │   │   ├── tags.rs           # Tag management
│   │   │   ├── boards.rs         # Board management
│   │   │   ├── search.rs         # Search and filter
│   │   │   └── text_utils.rs     # UTF-8 helpers, wrapping, visual height
│   │   ├── db.rs                 # SQLite CRUD + migrations
│   │   ├── model.rs              # Task, Tag, Board, Priority, Column
│   │   ├── handler.rs            # Input dispatch per AppMode
│   │   ├── event.rs              # Crossterm event polling
│   │   ├── sync.rs               # Pull/push/combined sync
│   │   ├── auth.rs               # OAuth + credentials
│   │   ├── export.rs             # JSON export/import
│   │   ├── keybindings.rs        # Configurable key mappings
│   │   ├── theme.rs              # Color theme system
│   │   ├── update.rs             # Self-update via GitHub API
│   │   └── ui/                   # All rendering
│   │       ├── mod.rs            # Layout entry point
│   │       ├── board.rs          # Kanban columns
│   │       ├── modal.rs          # Task create/edit form
│   │       ├── detail.rs         # Task detail overlay
│   │       ├── sort_menu.rs      # Sort + tag filter
│   │       ├── tag_screen.rs     # Tag management
│   │       ├── search_bar.rs     # Search input
│   │       ├── delete_confirm.rs # Confirmation dialogs
│   │       ├── tab_bar.rs        # Board tabs
│   │       ├── board_mgmt.rs     # Board CRUD
│   │       ├── options.rs        # Keybinding + theme editor
│   │       └── help_bar.rs       # Help overlay
│   ├── rk-server/
│   │   ├── src/
│   │   │   ├── main.rs           # Server startup, route tree, middleware
│   │   │   ├── lib.rs            # Constants, module exports
│   │   │   ├── config.rs         # Env config
│   │   │   ├── auth.rs           # Token hashing, AuthUser extractor
│   │   │   ├── csrf.rs           # CSRF protection middleware
│   │   │   ├── rate_limit.rs     # IP-based rate limiting (governor)
│   │   │   ├── session.rs        # Session key constant
│   │   │   ├── error.rs          # AppError
│   │   │   ├── purge.rs          # Background cleanup
│   │   │   └── routes/
│   │   │       ├── mod.rs        # Route tree
│   │   │       ├── auth.rs       # GitHub OAuth
│   │   │       ├── sync.rs       # Pull/push/combined
│   │   │       ├── web.rs        # Task/tag CRUD
│   │   │       ├── web_account.rs # Device/token mgmt
│   │   │       ├── shares.rs     # Board sharing
│   │   │       └── pages.rs      # Login redirects
│   │   ├── migrations/           # PostgreSQL (sqlx)
│   │   ├── frontend/             # Svelte SPA
│   │   ├── Dockerfile
│   │   └── .env.example
│   └── rk-shared/src/
│       └── lib.rs                # Sync protocol types
└── docs/
    ├── product-spec.md           # This file's companion
    ├── tech-plan.md              # This file
    ├── USE_CASES.md
    ├── SYNC.md
    └── plans/                    # Historical design docs
```

---

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|------------|
| **Data loss from sync conflicts** | Medium | High | Last-write-wins is simple but can lose edits. Consider adding conflict detection/notification. |
| **Share token leakage** | Medium | Medium | Tokens in URLs can leak via referrer headers, browser history, server logs. Consider short-lived tokens or revocation UI improvements. |
| **Single server (no redundancy)** | High | High | DigitalOcean droplet is a single point of failure. No load balancer, no replica. Downtime during deploys. |
| **GitHub OAuth dependency** | Low | High | If GitHub is down or revokes the OAuth app, no one can log in or sync. No fallback auth. |
| **SQLite schema migration failures** | Low | Medium | Manual migrations could fail silently or corrupt data. No rollback mechanism. |
| **Scaling beyond single VPS** | Medium | Medium | No horizontal scaling story. PostgreSQL on same machine as app server. Would need architecture changes for growth. |
