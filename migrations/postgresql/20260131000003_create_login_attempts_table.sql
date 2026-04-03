-- Create login_attempts table
CREATE TABLE login_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) NOT NULL,
    ip_address INET NOT NULL,
    success BOOLEAN NOT NULL,
    attempted_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Create index on email and attempted_at for rate limiting
CREATE INDEX idx_login_attempts_email_attempted_at ON login_attempts(email, attempted_at);

-- Create index on ip_address and attempted_at for rate limiting
CREATE INDEX idx_login_attempts_ip_attempted_at ON login_attempts(ip_address, attempted_at);
