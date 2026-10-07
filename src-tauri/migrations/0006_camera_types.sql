ALTER TABLE cameras ADD COLUMN camera_type TEXT NOT NULL DEFAULT 'film'
    CHECK (camera_type IN ('film', 'digital'));

ALTER TABLE cameras ADD COLUMN sensor_format TEXT;
