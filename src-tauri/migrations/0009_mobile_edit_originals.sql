-- Official edited originals for the native Android client.
--
-- This is additive and deliberately separate from mobile_assets: the PWA keeps
-- using generated edit_display resources, while the Android client can download
-- the formal edit file (PNG/JPEG/WebP) only after an explicit request.

CREATE TABLE IF NOT EXISTS mobile_edit_originals (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL CHECK (entity_type IN ('film_photo', 'digital_photo')),
    entity_sync_id TEXT NOT NULL,
    source_path TEXT NOT NULL,
    mime_type TEXT NOT NULL CHECK (mime_type IN ('image/png', 'image/jpeg', 'image/webp')),
    byte_size INTEGER,
    sha256 TEXT,
    source_mtime INTEGER,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE (entity_type, entity_sync_id)
);

CREATE INDEX IF NOT EXISTS idx_mobile_edit_originals_entity
    ON mobile_edit_originals(entity_type, entity_sync_id);
