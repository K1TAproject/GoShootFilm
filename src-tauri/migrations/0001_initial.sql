CREATE TABLE IF NOT EXISTS cameras (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    brand TEXT NOT NULL,
    model TEXT NOT NULL,
    format TEXT DEFAULT '135',
    purchase_date TEXT DEFAULT NULL,
    status TEXT DEFAULT 'active',
    note TEXT DEFAULT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS film_stocks (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    brand TEXT NOT NULL,
    name TEXT NOT NULL,
    iso INTEGER NOT NULL,
    type TEXT NOT NULL,
    target_status TEXT DEFAULT 'untested',
    note TEXT DEFAULT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(brand, name)
);

CREATE TABLE IF NOT EXISTS rolls (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    camera_id INTEGER NOT NULL,
    film_stock_id INTEGER NOT NULL,
    roll_index INTEGER NOT NULL,
    shot_month TEXT DEFAULT NULL,
    city TEXT DEFAULT NULL,
    note TEXT DEFAULT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (camera_id) REFERENCES cameras(id) ON DELETE CASCADE,
    FOREIGN KEY (film_stock_id) REFERENCES film_stocks(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS photos (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    roll_id INTEGER NOT NULL,
    frame_number INTEGER DEFAULT NULL,
    lab_scan_path TEXT DEFAULT NULL,
    edit_scan_path TEXT DEFAULT NULL,
    is_favorite INTEGER DEFAULT 0,
    note TEXT DEFAULT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (roll_id) REFERENCES rolls(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_rolls_camera_id ON rolls(camera_id);
CREATE INDEX IF NOT EXISTS idx_rolls_film_stock_id ON rolls(film_stock_id);
CREATE INDEX IF NOT EXISTS idx_rolls_shot_month ON rolls(shot_month);
CREATE INDEX IF NOT EXISTS idx_photos_roll_id ON photos(roll_id);
CREATE INDEX IF NOT EXISTS idx_photos_roll_frame ON photos(roll_id, frame_number);

INSERT OR IGNORE INTO film_stocks (id, brand, name, iso, type, target_status, created_at) VALUES
    (1, 'Foma', 'Fomapan 100 Classic', 100, 'B&W', 'shot', '2026-08-30 08:30:09'),
    (2, 'FUJIFILM', 'Fujicolor 100', 100, 'Color Negative', 'unshot', '2026-08-30 08:36:28'),
    (3, 'Kodak', 'Professional Ektachrome E100', 100, 'Slide', 'shot', '2026-08-30 08:36:28'),
    (4, 'Kodak', 'Professional PORTRA 160', 160, 'Color Negative', 'shot', '2026-08-30 08:36:28'),
    (5, 'Wolfen', 'NC500 Color Negative Film', 500, 'Color Negative', 'unshot', '2026-08-30 08:36:28'),
    (6, 'kodak', 'Gold 200', 200, 'Color Negative', 'shot', '2026-08-31 08:50:00');
