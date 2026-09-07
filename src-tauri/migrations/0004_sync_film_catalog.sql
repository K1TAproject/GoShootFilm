CREATE TABLE IF NOT EXISTS app_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

UPDATE film_stocks
SET target_status = 'unshot'
WHERE target_status = 'untested';
