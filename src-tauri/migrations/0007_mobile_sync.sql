-- Mobile / LAN sync foundation.
--
-- Existing auto-increment IDs remain local-only. Every syncable row gets a
-- stable, opaque sync_id plus a revision and tombstone so a phone can apply
-- incremental changes without depending on desktop row IDs.
--
-- This migration only adds new tables and triggers. Existing columns,
-- indexes, import rules and gallery paths are untouched.

CREATE TABLE IF NOT EXISTS sync_entities (
    entity_type TEXT NOT NULL,
    local_id INTEGER NOT NULL,
    sync_id TEXT NOT NULL UNIQUE,
    revision INTEGER NOT NULL DEFAULT 1,
    updated_at TEXT NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (entity_type, local_id)
);

CREATE INDEX IF NOT EXISTS idx_sync_entities_sync_id ON sync_entities(sync_id);
CREATE INDEX IF NOT EXISTS idx_sync_entities_type_revision ON sync_entities(entity_type, revision);

CREATE TABLE IF NOT EXISTS sync_changes (
    seq INTEGER PRIMARY KEY AUTOINCREMENT,
    entity_type TEXT NOT NULL,
    sync_id TEXT NOT NULL,
    revision INTEGER NOT NULL,
    deleted INTEGER NOT NULL DEFAULT 0,
    changed_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_changes_seq ON sync_changes(seq);
CREATE INDEX IF NOT EXISTS idx_sync_changes_sync_id ON sync_changes(sync_id);

CREATE TABLE IF NOT EXISTS mobile_devices (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    platform TEXT NOT NULL DEFAULT 'android',
    credential_hash TEXT NOT NULL UNIQUE,
    paired_at TEXT NOT NULL,
    last_sync_at TEXT,
    revoked_at TEXT
);

CREATE TABLE IF NOT EXISTS mobile_pair_requests (
    id TEXT PRIMARY KEY,
    device_name TEXT NOT NULL,
    platform TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'approved', 'denied', 'expired')),
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    approved_device_id TEXT,
    approved_at TEXT,
    credential_delivered_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_mobile_pair_requests_status
    ON mobile_pair_requests(status, created_at);

CREATE TABLE IF NOT EXISTS mobile_assets (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_sync_id TEXT NOT NULL,
    version TEXT NOT NULL CHECK (version IN ('lab', 'edit', 'raw')),
    asset_type TEXT NOT NULL CHECK (asset_type IN ('thumbnail', 'edit_display', 'original_preview')),
    relative_path TEXT NOT NULL UNIQUE,
    source_path TEXT NOT NULL,
    source_size INTEGER,
    source_mtime INTEGER,
    sha256 TEXT,
    byte_size INTEGER,
    width INTEGER,
    height INTEGER,
    generated_at TEXT,
    created_at TEXT NOT NULL,
    UNIQUE (entity_type, entity_sync_id, version, asset_type)
);

CREATE INDEX IF NOT EXISTS idx_mobile_assets_entity
    ON mobile_assets(entity_type, entity_sync_id);
CREATE INDEX IF NOT EXISTS idx_mobile_assets_generated
    ON mobile_assets(generated_at);

-- Backfill stable IDs for every existing row. The sync_id is opaque and is
-- never exposed as a desktop ID or path.
INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'camera', id, lower(hex(randomblob(16))), 1,
       strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0
FROM cameras;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'equipment', id, lower(hex(randomblob(16))), 1,
       COALESCE(updated_at, created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM equipment_items;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'film', id, lower(hex(randomblob(16))), 1,
       COALESCE(created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM film_stocks;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'roll', id, lower(hex(randomblob(16))), 1,
       COALESCE(created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM rolls;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'film_photo', id, lower(hex(randomblob(16))), 1,
       COALESCE(created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM photos;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'digital_album', id, lower(hex(randomblob(16))), 1,
       COALESCE(updated_at, created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM digital_albums;

INSERT OR IGNORE INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
SELECT 'digital_photo', id, lower(hex(randomblob(16))), 1,
       COALESCE(created_at, strftime('%Y-%m-%dT%H:%M:%fZ', 'now')), 0
FROM digital_photos;

-- A fresh install or an upgraded library needs one initial change per row so
-- the first phone sync can start from cursor 0.
INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
SELECT entity_type, sync_id, revision, 0, updated_at
FROM sync_entities;

-- Cameras -----------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_camera_insert
AFTER INSERT ON cameras
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('camera', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'camera', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'camera' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_camera_update
AFTER UPDATE ON cameras
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'camera' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'camera', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'camera' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_camera_delete
AFTER DELETE ON cameras
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'camera' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'camera', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'camera' AND local_id = OLD.id;
END;

-- Equipment ---------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_equipment_insert
AFTER INSERT ON equipment_items
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('equipment', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'equipment', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'equipment' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_equipment_update
AFTER UPDATE OF category, subtype, brand, model, mount, status, purchase_date, note ON equipment_items
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'equipment' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'equipment', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'equipment' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_equipment_delete
AFTER DELETE ON equipment_items
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'equipment' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'equipment', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'equipment' AND local_id = OLD.id;
END;

-- Films -------------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_film_insert
AFTER INSERT ON film_stocks
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('film', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'film' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_update
AFTER UPDATE OF brand, name, iso, type, target_status, note ON film_stocks
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'film' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'film' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_delete
AFTER DELETE ON film_stocks
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'film' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'film' AND local_id = OLD.id;
END;

-- Rolls -------------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_roll_insert
AFTER INSERT ON rolls
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('roll', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_roll_update
AFTER UPDATE OF camera_id, film_stock_id, roll_index, shot_month, city, note ON rolls
WHEN OLD.camera_id IS NOT NEW.camera_id
  OR OLD.film_stock_id IS NOT NEW.film_stock_id
  OR OLD.roll_index IS NOT NEW.roll_index
  OR OLD.shot_month IS NOT NEW.shot_month
  OR OLD.city IS NOT NEW.city
  OR OLD.note IS NOT NEW.note
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_roll_delete
AFTER DELETE ON rolls
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'roll' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = OLD.id;
END;

-- Film photos -------------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_insert
AFTER INSERT ON photos
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('film_photo', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film_photo', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'film_photo' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_update
AFTER UPDATE OF frame_number, lab_scan_path, edit_scan_path, is_favorite ON photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'film_photo' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film_photo', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'film_photo' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_delete
AFTER DELETE ON photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'film_photo' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'film_photo', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'film_photo' AND local_id = OLD.id;
END;

-- Digital albums ----------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_digital_album_insert
AFTER INSERT ON digital_albums
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('digital_album', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_album_update
AFTER UPDATE OF title, camera_id, shot_date, city, note ON digital_albums
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_album' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_album_delete
AFTER DELETE ON digital_albums
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'digital_album' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = OLD.id;
END;

-- Digital photos ----------------------------------------------------------
CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_insert
AFTER INSERT ON digital_photos
BEGIN
    INSERT INTO sync_entities (entity_type, local_id, sync_id, revision, updated_at, deleted)
    VALUES ('digital_photo', NEW.id, lower(hex(randomblob(16))), 1,
            strftime('%Y-%m-%dT%H:%M:%fZ', 'now'), 0);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_photo', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_photo' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_update
AFTER UPDATE OF pairing_key, raw_path, edit_path, is_favorite ON digital_photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_photo' AND local_id = NEW.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_photo', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_photo' AND local_id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_delete
AFTER DELETE ON digital_photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 1
    WHERE entity_type = 'digital_photo' AND local_id = OLD.id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_photo', sync_id, revision, 1, updated_at
    FROM sync_entities WHERE entity_type = 'digital_photo' AND local_id = OLD.id;
END;
