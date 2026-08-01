-- Add hidden flag for conversations
-- Tracks agent-originated sessions that should be hidden from normal session selectors
-- Default 0 keeps legacy behavior and preserves backward compatibility.

ALTER TABLE conversations ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0;
