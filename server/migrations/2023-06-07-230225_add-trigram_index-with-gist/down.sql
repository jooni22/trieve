-- This file should undo anything in `up.sql`
DROP INDEX IF EXISTS idx_gist;
DROP EXTENSION IF EXISTS pg_trgm;