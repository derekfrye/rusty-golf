CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    espn_id INTEGER NOT NULL,
    year INT NOT NULL,
    name TEXT NOT NULL,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);
