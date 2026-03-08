-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY,
    github_id BIGINT UNIQUE NOT NULL,
    username TEXT NOT NULL,
    email TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Devices
CREATE TABLE devices (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    last_synced_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    stale BOOLEAN NOT NULL DEFAULT FALSE
);

-- Auth tokens (SHA-256 hashed)
CREATE TABLE auth_tokens (
    token_hash TEXT PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    expires_at TIMESTAMP,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Tasks
CREATE TABLE tasks (
    uuid UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    priority TEXT NOT NULL DEFAULT 'Medium',
    column_name TEXT NOT NULL DEFAULT 'todo',
    due_date DATE,
    created_at TIMESTAMP NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP
);

-- Tags
CREATE TABLE tags (
    uuid UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    updated_at TIMESTAMP NOT NULL,
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP
);

-- Enforce unique tag names per user (only among active tags)
CREATE UNIQUE INDEX idx_tags_user_name_active ON tags (user_id, name) WHERE deleted = FALSE;

-- Task-tag associations
CREATE TABLE task_tags (
    task_uuid UUID NOT NULL REFERENCES tasks(uuid) ON DELETE CASCADE,
    tag_uuid UUID NOT NULL REFERENCES tags(uuid) ON DELETE CASCADE,
    PRIMARY KEY (task_uuid, tag_uuid)
);

-- Performance indexes
CREATE INDEX idx_tasks_user_updated ON tasks (user_id, updated_at);
CREATE INDEX idx_tags_user_updated ON tags (user_id, updated_at);
CREATE INDEX idx_devices_user ON devices (user_id);
CREATE INDEX idx_auth_tokens_user ON auth_tokens (user_id);
