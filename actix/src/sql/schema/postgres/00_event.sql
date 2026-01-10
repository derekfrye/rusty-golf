CREATE TABLE IF NOT EXISTS event (
    event_id SERIAL PRIMARY KEY,
    espn_id INTEGER NOT NULL,
    year INT NOT NULL,
    name TEXT NOT NULL,
    ins_ts TIMESTAMP NOT NULL DEFAULT now(),
    score_view_step_factor real not null default 3.0,
    refresh_from_espn INTEGER not null DEFAULT 1,
    end_date TIMESTAMP,

    UNIQUE (espn_id)
);
