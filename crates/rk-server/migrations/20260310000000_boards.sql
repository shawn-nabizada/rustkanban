-- Boards table
CREATE TABLE boards (
    uuid UUID PRIMARY KEY,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    position INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMP NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    deleted BOOLEAN NOT NULL DEFAULT FALSE,
    deleted_at TIMESTAMP
);

-- Enforce unique board names per user (only among active boards)
CREATE UNIQUE INDEX idx_boards_user_name_active ON boards (user_id, name) WHERE deleted = FALSE;

-- Performance index
CREATE INDEX idx_boards_user ON boards (user_id);

-- Add board_uuid to tasks (nullable initially for migration)
ALTER TABLE tasks ADD COLUMN board_uuid UUID REFERENCES boards(uuid);

-- Create default "Personal" board for each user who has tasks
INSERT INTO boards (uuid, user_id, name, position, created_at, updated_at)
SELECT gen_random_uuid(), user_id, 'Personal', 0, NOW(), NOW()
FROM (SELECT DISTINCT user_id FROM tasks) AS users_with_tasks;

-- Also create for users without tasks but who have accounts
INSERT INTO boards (uuid, user_id, name, position, created_at, updated_at)
SELECT gen_random_uuid(), id, 'Personal', 0, NOW(), NOW()
FROM users
WHERE id NOT IN (SELECT DISTINCT user_id FROM boards);

-- Backfill existing tasks to their user's Personal board
UPDATE tasks SET board_uuid = (
    SELECT uuid FROM boards WHERE boards.user_id = tasks.user_id AND boards.name = 'Personal' LIMIT 1
);

-- Now make it NOT NULL
ALTER TABLE tasks ALTER COLUMN board_uuid SET NOT NULL;
