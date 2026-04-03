-- Add deleted_at column to users table for soft delete support
ALTER TABLE users ADD COLUMN deleted_at TIMESTAMP WITH TIME ZONE DEFAULT NULL;

-- Create index on deleted_at for efficient filtering of deleted users
CREATE INDEX idx_users_deleted_at ON users(deleted_at) WHERE deleted_at IS NOT NULL;

-- Add comment explaining the soft delete pattern
COMMENT ON COLUMN users.deleted_at IS 'Timestamp when user was soft deleted. NULL means user is active.';
