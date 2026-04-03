-- Create sessions table
CREATE TABLE sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_token VARCHAR(255) UNIQUE NOT NULL,
    ip_address INET,
    user_agent TEXT,
    expires_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index on session_token for faster lookups
CREATE INDEX idx_sessions_session_token ON sessions(session_token);

-- Create index on user_id for faster user session lookups
CREATE INDEX idx_sessions_user_id ON sessions(user_id);

-- Create index on expires_at for session cleanup
CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
