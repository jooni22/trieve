-- This file should undo anything in `up.sql`
ALTER TABLE chunk_metadata DROP COLUMN IF EXISTS location;