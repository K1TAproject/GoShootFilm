CREATE TABLE equipment_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    category TEXT NOT NULL CHECK (category IN ('lens', 'other')),
    subtype TEXT,
    brand TEXT NOT NULL,
    model TEXT NOT NULL,
    mount TEXT,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'inactive')),
    purchase_date TEXT,
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE digital_albums (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    title TEXT NOT NULL,
    camera_id INTEGER,
    shot_date TEXT,
    city TEXT,
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (camera_id) REFERENCES cameras(id) ON DELETE SET NULL
);

CREATE TABLE digital_photos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    album_id INTEGER NOT NULL,
    pairing_key TEXT NOT NULL COLLATE NOCASE,
    raw_path TEXT,
    edit_path TEXT,
    is_favorite INTEGER NOT NULL DEFAULT 0,
    note TEXT,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (album_id) REFERENCES digital_albums(id) ON DELETE CASCADE,
    UNIQUE (album_id, pairing_key),
    CHECK (raw_path IS NOT NULL OR edit_path IS NOT NULL)
);

CREATE INDEX idx_equipment_items_category ON equipment_items(category);
CREATE INDEX idx_digital_albums_camera ON digital_albums(camera_id);
CREATE INDEX idx_digital_photos_album ON digital_photos(album_id);
