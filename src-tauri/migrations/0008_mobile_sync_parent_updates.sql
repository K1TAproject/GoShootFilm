-- Keep parent summaries in sync when a child row changes.
--
-- Roll payloads include camera/film display names, cover information and a
-- photo count. Album payloads include camera display names, cover information
-- and raw/edit/photo counts. Bumping the parent revision in the same trigger
-- avoids a second desktop-side reconciliation pass and lets the phone replace
-- the old summary atomically.

CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_parent_insert
AFTER INSERT ON photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll' AND local_id = NEW.roll_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = NEW.roll_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_parent_update
AFTER UPDATE ON photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll' AND local_id = NEW.roll_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = NEW.roll_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_photo_parent_delete
AFTER DELETE ON photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll' AND local_id = OLD.roll_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'roll' AND local_id = OLD.roll_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_parent_insert
AFTER INSERT ON digital_photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_album' AND local_id = NEW.album_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = NEW.album_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_parent_update
AFTER UPDATE ON digital_photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_album' AND local_id = NEW.album_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = NEW.album_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_digital_photo_parent_delete
AFTER DELETE ON digital_photos
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_album' AND local_id = OLD.album_id;
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities WHERE entity_type = 'digital_album' AND local_id = OLD.album_id;
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_camera_parent_update
AFTER UPDATE ON cameras
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll'
      AND local_id IN (SELECT id FROM rolls WHERE camera_id = NEW.id);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities
    WHERE entity_type = 'roll'
      AND local_id IN (SELECT id FROM rolls WHERE camera_id = NEW.id);

    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'digital_album'
      AND local_id IN (SELECT id FROM digital_albums WHERE camera_id = NEW.id);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'digital_album', sync_id, revision, 0, updated_at
    FROM sync_entities
    WHERE entity_type = 'digital_album'
      AND local_id IN (SELECT id FROM digital_albums WHERE camera_id = NEW.id);
END;

CREATE TRIGGER IF NOT EXISTS trg_sync_film_parent_update
AFTER UPDATE ON film_stocks
BEGIN
    UPDATE sync_entities
    SET revision = revision + 1,
        updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
        deleted = 0
    WHERE entity_type = 'roll'
      AND local_id IN (SELECT id FROM rolls WHERE film_stock_id = NEW.id);
    INSERT INTO sync_changes (entity_type, sync_id, revision, deleted, changed_at)
    SELECT 'roll', sync_id, revision, 0, updated_at
    FROM sync_entities
    WHERE entity_type = 'roll'
      AND local_id IN (SELECT id FROM rolls WHERE film_stock_id = NEW.id);
END;
