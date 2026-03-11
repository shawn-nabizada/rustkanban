CREATE TABLE board_shares (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_uuid UUID NOT NULL REFERENCES boards(uuid) ON DELETE CASCADE,
    token TEXT NOT NULL UNIQUE,
    permission TEXT NOT NULL DEFAULT 'view',
    created_by UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_board_shares_board ON board_shares(board_uuid);
CREATE UNIQUE INDEX idx_board_shares_token ON board_shares(token);
