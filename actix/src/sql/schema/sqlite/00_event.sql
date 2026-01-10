CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    espn_id INTEGER NOT NULL,
    year INT NOT NULL,
    name TEXT NOT NULL,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    score_view_step_factor real not null default 3.0, --deprecated
    refresh_from_espn INTEGER not null DEFAULT 1,
    end_date TEXT,
    UNIQUE (espn_id)
);
