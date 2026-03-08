# RustKanban Sync — Design Document

**Date:** 2026-03-08
**Status:** Approved

## Goal

Add cross-machine sync to RustKanban so a single user can access their kanban board from multiple devices. The app remains fully functional offline without an account — sync is purely opt-in.

## Scope

- **Now:** Single-user personal sync across machines
- **Later:** Multi-user team sharing, web-based kanban view

## Architecture

### Overview

Monolith Rust server (Axum) that handles OAuth, sync API, and serves a simple website. Communicates with a PostgreSQL database. The TUI client syncs via REST API calls.

```
┌──────────────┐         HTTPS          ┌──────────────────┐
│   rk client  │ ◄────────────────────► │   rk-server      │
│   (TUI)      │   Bearer token auth    │   (Axum)         │
│              │                        │                  │
│  SQLite      │                        │  PostgreSQL      │
│  (local)     │                        │  (hosted)        │
└──────────────┘                        └──────────────────┘
                                              │
                                        GitHub OAuth
                                              │
                                        ┌─────▼──────┐
                                        │  GitHub    │
                                        │  API       │
                                        └────────────┘
```

### Project Structure — Cargo Workspace

```
rustkanban/
├── Cargo.toml              (workspace root)
├── crates/
│   ├── rk-client/          (existing TUI app, moved from root)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── rk-server/          (Axum server)
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   └── templates/      (HTML pages)
│   └── rk-shared/          (shared sync types)
│       ├── Cargo.toml
│       └── src/
├── LICENSE
├── README.md
└── ...
```

**Why a workspace:**
- `rk-shared` gives one definition of the sync payload — no deserialization mismatches
- Each crate has its own dependencies (TUI doesn't pull in Axum, server doesn't pull in ratatui)
- One `cargo build` builds everything

**Migration impact:** Moving the existing code from the repo root into `crates/rk-client/` requires updating:
- `[[bin]] name = "rk"` moves to `crates/rk-client/Cargo.toml`
- `.github/workflows/ci.yml` — `cargo test/clippy/fmt` work unchanged (Cargo workspace runs all members)
- `.github/workflows/release.yml` — cross-compilation needs `-p rk-client` flag
- `cargo install --path .` → `cargo install --path crates/rk-client`
- `HomebrewFormula/rk.rb` — update build/install commands
- `aur/PKGBUILD` — update build/install commands
- `install.sh` — no change (downloads pre-built binary)
- `CLAUDE.md` — update build instructions
- `demo.tape` — no change (runs `rk` binary which is unchanged)

## Shared Types (`rk-shared`)

```rust
#[derive(Serialize, Deserialize)]
pub struct SyncPayload {
    pub tasks: Vec<SyncTask>,
    pub tags: Vec<SyncTag>,
    pub last_synced_at: Option<String>,  // ISO 8601
}

#[derive(Serialize, Deserialize)]
pub struct SyncTask {
    pub uuid: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_priority")]
    pub priority: String,
    #[serde(default = "default_column")]
    pub column: String,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,      // tag UUIDs
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SyncTag {
    pub uuid: String,
    pub name: String,
    pub updated_at: String,
    #[serde(default)]
    pub deleted: bool,
}

#[derive(Serialize, Deserialize)]
pub struct SyncResponse {
    pub tasks: Vec<SyncTask>,
    pub tags: Vec<SyncTag>,
    #[serde(default)]
    pub tag_uuid_mappings: HashMap<String, String>,  // client_uuid → server_uuid (for deduped tags)
    pub synced_at: String,
}
```

**Wire compatibility:** All optional or future-added fields use `#[serde(default)]` so old payloads missing these fields deserialize without error. Unknown fields from newer versions are silently ignored by serde's default behavior. This enables rolling upgrades where client and server are at different versions.

### Key Decisions

- **UUIDs** instead of autoincrement IDs — two machines can't share `id: 1`. Local autoincrement `id` stays for internal use (cursor positions, undo stack). UUID is for sync only.
- **Soft deletes** — `deleted: true` instead of `DELETE FROM`. Enables propagating deletions across devices. This applies to all delete operations including "Clear Done" (`Shift+D`) — bulk deletes must soft-delete to propagate via sync.
- **`last_synced_at`** — client sends this on pull so server only returns changes since last sync. If `last_synced_at` is `null` (first sync ever), the server returns all records and the client sends all records — a full sync.
- **Strings for priority/column** — keeps shared crate free of app-specific enums. Parsing at the edges.
- **Preferences do NOT sync** — sort mode and other preferences are per-device. A user may want different settings on their work laptop vs home desktop.
- **Tag deduplication** — tags are matched by `(user_id, name)` on the server, not by UUID alone. If two devices create a tag with the same name but different UUIDs, the server merges them into one and returns a UUID mapping so clients can update their local references. The server `tags` table has a `UNIQUE(user_id, name)` constraint (where `deleted = false`).

## Server Design (`rk-server`)

### Routes

```
Auth:
  GET  /                           → homepage (static HTML)
  GET  /login                      → redirect to GitHub OAuth
  GET  /auth/callback              → OAuth callback, set session
  POST /auth/logout                → clear session

Account:
  GET  /account                    → account settings page
  GET  /account/devices            → list devices + last synced
  POST /account/devices/:id/revoke → remove a device
  POST /account/delete             → delete account + all data

Health:
  GET  /health                     → returns 200 OK (for deployment monitoring)

API (Bearer token auth):
  POST /api/v1/sync/pull           → changes since client's last_synced_at
  POST /api/v1/sync/push           → receive client's changes
  POST /api/v1/sync                → pull + push in one round trip
```

### PostgreSQL Schema

```sql
CREATE TABLE users (
    id          UUID PRIMARY KEY,
    github_id   BIGINT UNIQUE NOT NULL,
    username    TEXT NOT NULL,
    email       TEXT,
    created_at  TIMESTAMP NOT NULL
);

CREATE TABLE devices (
    id              UUID PRIMARY KEY,
    user_id         UUID REFERENCES users(id) ON DELETE CASCADE,
    name            TEXT NOT NULL,
    last_synced_at  TIMESTAMP,
    created_at      TIMESTAMP NOT NULL,
    stale           BOOLEAN DEFAULT FALSE
);

CREATE TABLE auth_tokens (
    token_hash  TEXT PRIMARY KEY,
    user_id     UUID REFERENCES users(id) ON DELETE CASCADE,
    device_id   UUID REFERENCES devices(id) ON DELETE CASCADE,
    expires_at  TIMESTAMP,
    created_at  TIMESTAMP NOT NULL
);

CREATE TABLE tasks (
    uuid        UUID PRIMARY KEY,
    user_id     UUID REFERENCES users(id) ON DELETE CASCADE,
    title       TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority    TEXT NOT NULL DEFAULT 'Medium',
    column_name TEXT NOT NULL DEFAULT 'todo',
    due_date    DATE,
    created_at  TIMESTAMP NOT NULL,
    updated_at  TIMESTAMP NOT NULL,
    deleted     BOOLEAN DEFAULT FALSE,
    deleted_at  TIMESTAMP
);

CREATE TABLE tags (
    uuid        UUID PRIMARY KEY,
    user_id     UUID REFERENCES users(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    updated_at  TIMESTAMP NOT NULL,
    deleted     BOOLEAN DEFAULT FALSE,
    deleted_at  TIMESTAMP
);

-- Partial unique index: enforce unique tag names per user (only among non-deleted tags)
CREATE UNIQUE INDEX idx_tags_user_name_active ON tags (user_id, name) WHERE deleted = FALSE;

CREATE TABLE task_tags (
    task_uuid   UUID REFERENCES tasks(uuid) ON DELETE CASCADE,
    tag_uuid    UUID REFERENCES tags(uuid) ON DELETE CASCADE,
    PRIMARY KEY (task_uuid, tag_uuid)
);
```

### Server Configuration

The server reads configuration from environment variables:

```
DATABASE_URL=postgres://user:pass@host/rustkanban   # Postgres connection
GITHUB_CLIENT_ID=...                                 # OAuth app ID
GITHUB_CLIENT_SECRET=...                             # OAuth app secret
SERVER_URL=https://sync.rustkanban.com               # Public URL (for OAuth redirect_uri)
SESSION_SECRET=...                                   # Cookie signing key (32+ random bytes, hex)
PORT=3000                                            # Listen port (default: 3000)
```

For local development, these can be set in a `.env` file (loaded via `dotenvy`). In production, set via the hosting platform's environment config.

### Database Migrations

Server-side Postgres migrations are managed with `sqlx migrate` (compile-time checked SQL). Migration files live in `crates/rk-server/migrations/`. Migrations run automatically on server startup.

### Sync Processing Order

When the server processes a push, it must handle records in dependency order within a single database transaction (rollback on any failure):

1. **Tags first** — upsert all tags (with name deduplication), build `tag_uuid_mappings`
2. **Remap task tag references** — for each `SyncTask.tags`, replace any client tag UUIDs with their server equivalents using the mappings from step 1. For example, if client sends task with `tags: ["uuid-A"]` and `uuid-A` was deduped to `uuid-B`, the task's tags become `["uuid-B"]` before insertion.
3. **Tasks second** — upsert all tasks (with remapped tag UUIDs)
4. **Task-tag associations last** — replace `task_tags` for each pushed task (delete existing, insert new). Uses the remapped UUIDs.

This prevents FK violations and ensures tag deduplication is applied consistently to task-tag associations.

### Tag Name Deduplication

When a pushed tag has the same `(user_id, name)` as an existing server tag but a different UUID:
1. Server keeps the existing tag (older UUID wins)
2. Server returns a `tag_uuid_mappings` field in `SyncResponse`: `{ "client_uuid": "server_uuid" }`
3. Client updates its local tag UUID and all `task_tags` references to match
4. This ensures all devices converge on the same UUID for each tag name

### Device Tracking & Purge

- Devices marked stale if `last_synced_at` older than 90 days
- Stale devices excluded from purge calculations
- When a stale device syncs again, it does a full pull instead of delta, then is re-activated (`stale = false`, `last_synced_at` updated to now)
- Purge job runs as a background tokio task (spawned on server startup, runs every 24 hours). For each user: hard-deletes records where `deleted = true AND deleted_at` is older than the oldest non-stale device's `last_synced_at`. If a user has no non-stale devices, skip purge for that user (their data is preserved until a device syncs again).

### Account/Devices Page

- Shows GitHub username/avatar
- Table of devices: name, last synced time, status (active/stale)
- "Revoke" button per device (deletes device + its auth token)
- Danger zone: "Delete account" with confirmation (wipes all server-side data)

## Client Changes (`rk-client`)

### SQLite Schema Migration

Migrations are versioned using a `schema_version` key in the `preferences` table. The current schema (before sync) is version 1. Each migration checks the current version and runs only if needed.

**Migration to version 2 (sync support):**
```sql
ALTER TABLE tasks ADD COLUMN uuid TEXT UNIQUE;
ALTER TABLE tags ADD COLUMN uuid TEXT UNIQUE;
ALTER TABLE tasks ADD COLUMN deleted INTEGER DEFAULT 0;
ALTER TABLE tasks ADD COLUMN deleted_at TEXT;
ALTER TABLE tags ADD COLUMN deleted INTEGER DEFAULT 0;
ALTER TABLE tags ADD COLUMN deleted_at TEXT;
-- Backfill existing rows with generated UUIDs (application code, not SQL)
-- UPDATE preferences SET value = '2' WHERE key = 'schema_version';
```

This pattern allows future migrations (version 3, 4, etc.) to be added incrementally. On startup, the app checks `schema_version` and runs any pending migrations in order.

Local autoincrement `id` stays for internal use. Board queries add `WHERE deleted = 0`.

**Soft delete scope:** Soft deletes are always used regardless of login state. This keeps one code path for all users. For non-logged-in users, a local cleanup runs on startup: hard-deletes any soft-deleted records older than 30 days (since they'll never be synced). For logged-in users, local cleanup is skipped — the server purge handles it after sync propagation.

**Query split:** The existing `load_tasks()` / `load_tags()` functions are updated to add `WHERE deleted = 0` (used by board rendering, sorting, filtering, export). New functions `load_all_tasks()` / `load_all_tags()` return unfiltered results including soft-deleted records (used by sync push to send deletions to the server).

### New Modules

**`src/sync.rs`** — sync logic:
```rust
pub struct SyncClient {
    base_url: String,
    token: String,
    device_id: String,
}

impl SyncClient {
    pub fn pull(&self, conn: &Connection) -> Result<SyncResult>
    pub fn push(&self, conn: &Connection) -> Result<SyncResult>
    pub fn sync(&self, conn: &Connection) -> Result<SyncResult>
}
```

**`src/auth.rs`** — login flow:
```rust
pub fn login(base_url: &str) -> Result<Credentials>  // localhost callback
pub fn logout() -> Result<()>                         // delete credentials
pub fn load_credentials() -> Option<Credentials>      // read from disk
```

### Credentials File

`~/.config/rustkanban/credentials.json`:
```json
{
    "token": "rk_abc123...",
    "device_id": "550e8400-...",
    "device_name": "snabby-thinkpad",
    "server_url": "https://sync.rustkanban.com",
    "last_synced_at": "2026-03-08T12:00:00Z"
}
```

File permissions: `0600` (owner read/write only).

### New CLI Subcommands

```
rk login                          → opens browser, completes OAuth, saves credentials
rk login --server <url>           → use a custom server (default: https://sync.rustkanban.com)
rk login --device-name <name>     → override device name (default: system hostname)
rk logout                         → pushes unsynced changes, then deletes credentials file
rk sync                           → manual pull + push outside the TUI
rk status                         → shows login state (see output format below)
rk reset                          → (existing) now warns if logged in, see below
```

**`rk reset` behavior with sync:** If the user is logged in, `rk reset` warns: "You are logged in to sync. This will only reset local data — synced tasks will reappear on next sync. To also delete server data, use your account page. Continue? (Y/N)". This makes the scope of the reset clear. `rk reset` never touches server data — account deletion is only available via the website's danger zone.

**`rk logout` behavior:** Before deleting credentials, `rk logout` attempts a final push to sync any unsynced local changes. If the push fails (network error, expired token), it warns: "Could not push unsynced changes. Local data is preserved — log in again to sync. Continue logout? (Y/N)". Local data is never deleted by logout.

**Already logged in:** if `credentials.json` exists, `rk login` prints "Already logged in as device '{name}'. Run `rk logout` first to switch accounts or re-authenticate." and exits. This prevents accidentally creating duplicate device entries.

**Device naming:** defaults to the system hostname (e.g. "snabby-thinkpad"). Can be overridden with `--device-name` during login. Stored in `credentials.json`.

**Server URL:** defaults to the official hosted instance. Self-hosters can specify `--server` during login. Stored in `credentials.json`.

**`rk status` output:**
```
# When logged in:
Logged in as "snabby-thinkpad"
Server:      sync.rustkanban.com
Last synced: 2 minutes ago
Tasks:       42 local (3 unsynced)

# When not logged in:
Not logged in. Run `rk login` to enable sync.
```

### TUI Changes

- **Startup:** if logged in, pull from server before entering event loop (prints "Syncing..." to stdout, blocks briefly — acceptable since the TUI hasn't rendered yet)
- **Quit:** if logged in, push to server after leaving event loop (blocks briefly — acceptable since the TUI has already closed)
- **Keybinding:** `Ctrl+R` for manual sync — blocks the event loop during the HTTP request. The status bar shows "Syncing..." and the UI freezes briefly. This is acceptable for v1 since payloads are small (sub-second for typical boards). If this becomes a UX problem, a background thread can be added later.
- **After sync:** calls `reload_tasks()` to refresh the in-memory `App` state from the updated SQLite DB. Without this, the board would show stale pre-sync data.
- **Status bar:** shows sync state when logged in — "Synced 2m ago" / "Offline" / "Syncing..."
- **Not logged in:** status bar shows nothing sync-related, app works exactly as today
- **Undo stack:** cleared after any sync operation. Undo entries reference local state that may have been overwritten by the sync, so stale entries could cause confusion.

### Soft Delete Behavior Changes

The switch from hard deletes to soft deletes affects several existing features:

- **Delete task (`D` → `Y`):** changes from `DELETE FROM tasks` to `UPDATE tasks SET deleted=1, deleted_at=now()`. Board queries filter with `WHERE deleted = 0`.
- **Clear Done (`Shift+D`):** same change — soft-deletes all done tasks instead of hard-deleting. Still NOT undoable (no undo entries pushed), but deletions now propagate via sync.
- **Undo delete:** changes from `insert_task_full()` (re-inserting) to flipping `deleted = 0, deleted_at = NULL` on the existing row. Preserves the UUID.
- **Tag deletion:** soft-deletes the tag. The cascade removal from `task_tags` should also be handled as a logical operation rather than relying on SQL `ON DELETE CASCADE` (since the tag row isn't actually deleted).
- **Local cleanup:** after a successful push where the server acknowledges soft-deleted records, the client can optionally hard-delete them locally to save space. Not required — local SQLite size is negligible.

### Export/Import Changes

The export format must be updated to include UUIDs (version 2). Without UUIDs, importing the same export on two machines creates different UUIDs for the same tasks, causing duplicates after sync.

```json
{
    "version": 2,
    "tasks": [{ "uuid": "...", "title": "...", ... }],
    "tags": [{ "uuid": "...", "name": "..." }]
}
```

- **Export:** includes `uuid` for every task and tag
- **Import:** if a task/tag has a `uuid` that already exists locally, skip it (idempotent). If the `uuid` is new, insert with that UUID. If no `uuid` field (version 1 import), generate new UUIDs (backward compatible)
- **`insert_task()` and `insert_tag()`** must be updated to generate UUIDs for new rows (using `uuid` crate)

## Sync Algorithm

### Pull (server → client)

1. Client sends `POST /api/v1/sync/pull` with `{ last_synced_at }`
2. Server returns changes:
   - If `last_synced_at` is `null` (first sync): return ALL records for the user
   - If device is stale (no sync in 90+ days): return ALL records (full re-sync)
   - Otherwise: return tasks/tags where `updated_at > last_synced_at` (including soft-deleted)
3. Client merges in dependency order (tags first, then tasks):
   **Tags:**
   - UUID not in local DB → insert
   - UUID in local DB, server `updated_at` > local → overwrite
   - UUID in local DB, local `updated_at` >= server → keep local
   - Server says `deleted=true` → soft-delete locally
   **Tasks** (processed after tags so tag FKs exist):
   - Same merge rules as tags
   - After upserting task, replace its local `task_tags` with the server's tag list
4. Client updates local `last_synced_at`
5. Client clears undo stack

### Push (client → server)

1. Client sends `POST /api/v1/sync/push`:
   - If `last_synced_at` is `null` (first sync): send ALL local records
   - Otherwise: send records where `updated_at > last_synced_at`
2. Server processes in order: tags (with name dedup) → tasks → task_tags
3. Server merges with last-write-wins logic
4. Server returns any records where it kept its own version, plus `tag_uuid_mappings` for any deduplicated tags
5. Client applies returned records and UUID mappings
6. Server updates `device.last_synced_at`

### Combined Sync

Available via `POST /api/v1/sync` (single round trip), `Ctrl+R` in TUI, or `rk sync` CLI.

**Request:** client sends `SyncPayload` (its changed records + `last_synced_at`).

**Server processing (within a single transaction):**
1. **Pull phase:** gather all server records where `updated_at > client's last_synced_at`
2. **Push phase:** process the client's payload (tags → remap → tasks → task_tags) with LWW merge
3. **Build response:** the `SyncResponse` contains:
   - `tasks`: all tasks where the server's version differs from the client's — either tasks the client didn't have (from pull), or tasks where the server rejected the client's push (server had newer `updated_at`)
   - `tags`: same logic as tasks
   - `tag_uuid_mappings`: any UUIDs that were remapped during dedup
   - `synced_at`: server timestamp for the client to store as its new `last_synced_at`

**Client processing:** apply the entire response — insert/overwrite returned records, remap local tag UUIDs, update `last_synced_at`, call `reload_tasks()`, clear undo stack.

### Error Handling

The client distinguishes between error types and shows appropriate messages:

| HTTP Status | Meaning | Client Message |
|---|---|---|
| Network error | Server unreachable | "Sync failed — working offline" |
| 401 Unauthorized | Token expired or revoked | "Session expired — run `rk login` to re-authenticate" |
| 403 Forbidden | Account deleted or banned | "Account not found — run `rk logout` to clear credentials" |
| 409 Conflict | Version mismatch | "Sync conflict — try again" |
| 5xx | Server error | "Server error — try again later" |

**API error response format:**
```json
{
    "error": "token_expired",
    "message": "Your session has expired. Please run `rk login` to re-authenticate."
}
```

### Offline Handling

- Network failure → flash "Sync failed — working offline"
- App continues normally, local SQLite is source of truth
- Push failure on quit → "Changes saved locally, will sync next time"
- Auth errors on quit → push silently skipped, data safe in local SQLite
- No retries, no background threads

### Conflict Resolution

**v1:** Last-write-wins at the task level based on `updated_at` timestamp.

**Clock skew:** LWW trusts client `updated_at` timestamps. If one machine's clock is significantly off, its changes may incorrectly win or lose. This is acceptable for single-user sync — the user's own machines typically have NTP-synced clocks. If this becomes a problem, the server can assign a `server_updated_at` on push and use that for conflict resolution instead.

**Future (multi-user):** Upgrade to field-level merge. Track `updated_at` per field. If two users edit different fields of the same task, merge both. Same-field conflict → show both versions, let user pick.

## Authentication

### Login Flow (Localhost Callback)

1. `rk login` starts a temporary HTTP server on a random localhost port (uses port 0 for OS-assigned port to avoid conflicts)
2. Opens browser to `{server_url}/login?redirect_port={port}&device_name={hostname}`
3. Server stores `redirect_port` and `device_name` in a short-lived session, redirects to GitHub OAuth with `state` parameter (CSRF protection)
4. User authorizes on GitHub
5. GitHub redirects to server's `/auth/callback`
6. Server exchanges code for GitHub token, fetches user profile, creates/finds user
7. Server creates a new device record (using the `device_name` from step 2), generates `rk_` Bearer token, stores SHA-256 hash of the token in `auth_tokens`
8. Server redirects to `http://localhost:{port}/callback?token=rk_...&device_id=...`
9. CLI captures token and device_id, saves to credentials file, shuts down local server
10. Browser shows "You can close this tab"

**Timeout:** The CLI's localhost server times out after 5 minutes if no callback is received. Prints "Login timed out. Run `rk login` to try again." and exits. The CLI shows "Waiting for authentication... (Ctrl+C to cancel)" while waiting.

**Headless/SSH fallback:** If no browser is available (detected via `open`/`xdg-open` failure), `rk login` enters headless mode:
1. CLI constructs the login URL **without** `redirect_port` (e.g. `{server_url}/login?device_name={hostname}&mode=headless`)
2. CLI prints: "Open this URL in any browser:\n  {url}\nThen paste the token here:"
3. The server detects headless mode (no `redirect_port`), completes OAuth normally, but instead of redirecting to localhost, displays the token and device_id on a completion page as copyable text
4. User copies the token, pastes into CLI
5. CLI saves credentials

Alternatively, `rk login --token <token> --device-id <id> --server <url>` allows fully manual credential entry without any browser interaction.

### GitHub OAuth

- Only request minimal scopes: `read:user`, `user:email`
- GitHub token used once to get profile, then discarded
- CSRF protection via `state` parameter

## Security

- **Token storage:** SHA-256 hash server-side, raw token client-side
- **Token expiry:** 90 days sliding (refreshed on each sync)
- **Credentials file:** `0600` permissions
- **API scoping:** every query filtered by `user_id`
- **Rate limiting:** on auth endpoints
- **Request body limit:** 10MB max on sync API endpoints (far exceeds any realistic kanban board)
- **HTTPS only** in production
- **No JWT, no refresh tokens** — simple bearer token model

### Two Auth Mechanisms

The server uses two independent auth mechanisms:

1. **Bearer tokens** (for CLI API calls): stored as SHA-256 hash in `auth_tokens` table, sent via `Authorization: Bearer rk_...` header. One token per device. Revoking a device deletes its token.

2. **Session cookies** (for website pages): set during OAuth callback, stored server-side (e.g. `tower-sessions` with Postgres backend or in-memory). Used for `/account` pages. Independent of Bearer tokens — revoking a device's Bearer token doesn't log you out of the website, and clearing cookies doesn't invalidate CLI sessions.

Both are scoped to the same `user_id`. The OAuth callback creates both: a session cookie for the browser redirect chain, and a Bearer token for the CLI (sent via the localhost redirect).

## Website

Three server-rendered HTML pages using Askama/Tera templates:

1. **Homepage (`/`)** — hero, features, install instructions, "Login with GitHub" in nav
2. **Account (`/account`)** — GitHub username/avatar, devices table with revoke, danger zone with delete account
3. **Login** — just a redirect to GitHub, no page needed

Minimal CSS (Pico CSS or similar). No JS framework. Vanilla JS only for delete confirmation modal.

## Hosting

To be decided. Options by cost:
- **Fly.io free tier** — small VM + 1GB Postgres (good for v1)
- **Railway** — similar free/cheap tier
- **Hetzner VPS** — ~EUR 4/mo at scale
- **Neon** — free tier serverless Postgres

The server is a standard Axum binary + Postgres — deployable anywhere.

## Dependencies

### rk-shared
- `serde`, `serde_json` — serialization

### rk-client (new additions to existing deps)
- `uuid` (v4) — UUID generation for tasks/tags
- `ureq` — blocking HTTP client (minimal deps, fits single-threaded model; avoids pulling in tokio/hyper via reqwest)
- `open` — cross-platform "open URL in browser" for login flow

### rk-server
- `axum` — web framework
- `tokio` — async runtime
- `sqlx` (postgres, runtime-tokio, tls-rustls) — Postgres driver with compile-time checked queries
- `tower`, `tower-http` — middleware (CORS, rate limiting, compression)
- `tower-sessions`, `tower-sessions-sqlx-store` — session management for web pages
- `oauth2` — GitHub OAuth flow
- `sha2` — token hashing
- `uuid` (v4) — UUID generation
- `dotenvy` — .env file loading
- `askama` — compile-time HTML templates
- `chrono` — timestamp handling
- `tracing`, `tracing-subscriber` — structured logging

## Validation & Limits

Text field and resource limits, enforced on both client (TUI input) and server (API validation):

| Field | Max Length | Notes |
|---|---|---|
| Task title | 500 chars | Truncated in board view anyway |
| Task description | 5,000 chars | Multiline, displayed in detail view |
| Tag name | 50 chars | Short labels |
| Device name | 100 chars | Hostname or user-provided |

Server-side resource limits per user:

| Resource | Limit | Notes |
|---|---|---|
| Tasks | 200 | Including soft-deleted |
| Tags | 15 | Including soft-deleted |
| Tags per task | 10 | |
| Devices | 5 | Active + stale |

The server returns `422 Unprocessable Entity` with a descriptive error if any limit is exceeded. The client validates text lengths in the TUI modal (prevents typing beyond the limit) and in CLI import. The client does not enforce resource count limits — those are server-side only.

## Known Limitations (v1)

- **Delete resurrection:** If machine A edits a task (`updated_at = T2`) and machine B deletes it (`updated_at = T1`, where T1 < T2), LWW says the edit wins. The task "comes back from the dead" on machine B. This is inherent to last-write-wins. For single-user sync this is unlikely (you wouldn't edit and delete the same task on different machines). For future multi-user, field-level merge would handle this better.
- **Clock skew:** see Conflict Resolution section.
- **No real-time sync:** changes only propagate on startup, quit, or manual `Ctrl+R`.
- **Sync blocks the UI:** `Ctrl+R` freezes the TUI briefly during the HTTP request.

## Testing Strategy

### rk-shared
- Unit tests for `#[serde(default)]` deserialization — verify old payloads missing new fields deserialize correctly
- Round-trip tests: serialize → deserialize for all shared types

### rk-client
- **Sync logic (`sync.rs`):** test against mock HTTP responses (use `ureq` test helpers or a trait-based HTTP abstraction). Cover: pull merge (insert, overwrite, keep-local, delete), push payload construction, UUID remapping from `tag_uuid_mappings`, `last_synced_at` tracking
- **Auth (`auth.rs`):** test credential file read/write, permission checks, `--token` manual entry parsing
- **SQLite migration:** test v1→v2 migration on an in-memory DB: verify UUID backfill, `deleted` column defaults, `WHERE deleted=0` filtering
- **Soft delete:** test that `load_tasks()` excludes deleted, `load_all_tasks()` includes deleted, undo-delete flips the flag back
- **Export/import v2:** test UUID round-trip, version 1 backward compatibility (generates UUIDs on import)
- **Existing tests:** all current tests continue to pass — soft delete is transparent to board logic since `load_tasks()` filters

### rk-server
- **Integration tests:** spin up a test Postgres (via `sqlx::test` or testcontainers), run full sync round-trips: push from device A, pull from device B, verify LWW resolution, tag deduplication, UUID remapping
- **Auth tests:** mock GitHub OAuth responses, verify token creation, session handling, device registration
- **Purge tests:** create stale devices, soft-deleted records, run purge, verify correct records are hard-deleted
- **Validation tests:** exceed each limit (200 tasks, 15 tags, 5 devices), verify 422 responses
- **Edge cases:** first sync (null `last_synced_at`), stale device re-sync, concurrent pushes from two devices, empty payloads

### End-to-End
- Script that runs two `rk-client` instances against a local `rk-server`, creates/edits/deletes tasks on both, syncs, and verifies convergence. Not automated in CI initially — manual verification during development.

## Non-Goals (v1)

- Multi-user / team boards
- Web-based kanban view (read or read-write)
- Real-time sync / WebSockets
- CRDTs or operational transform
- Multiple OAuth providers
- Mobile app
