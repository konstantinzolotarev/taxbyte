-- SQLite TEXT columns are already nullable by default.
-- This migration exists for consistency with the PostgreSQL migration.
-- No schema change needed.
SELECT 1;
