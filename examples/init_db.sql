CREATE TABLE IF NOT EXISTS event (
    event_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    espn_id INTEGER NOT NULL,
    year INT NOT NULL,
    name TEXT NOT NULL,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    score_view_step_factor real not null default 3.0,

    UNIQUE (espn_id)
);


CREATE TABLE IF NOT EXISTS eup_statistic (
    eup_stat_id INTEGER NOT NULL PRIMARY KEY,
    event_espn_id INT NOT NULL REFERENCES event(espn_id),
    golfer_espn_id INT NOT NULL REFERENCES golfer(espn_id),
    eup_id INT NOT NULL REFERENCES event_user_player(eup_id),
    grp INT NOT NULL,
    
    rounds JSON NOT NULL,
    round_scores JSON NOT NULL,
    tee_times JSON NOT NULL,
    holes_completed_by_round JSON NOT NULL,
    line_scores JSON NOT NULL,
    total_score INT NOT NULL,
    upd_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (golfer_espn_id, eup_id)
    );

CREATE TABLE IF NOT EXISTS golfer (
    -- drop table player cascade
    golfer_id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
    espn_id integer NOT NULL UNIQUE,
    name TEXT NOT NULL UNIQUE, -- i don't think its critical this is unique, program doesn't require it i don't think, but doing this just for extra data safety
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );


CREATE TABLE IF NOT EXISTS bettor (
    user_id integer NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    --alter table golfuser alter column name set data type text;

CREATE TABLE IF NOT EXISTS event_user_player (
    -- drop table event_user_player cascade
    eup_id INTEGER NOT NULL PRIMARY KEY,
    event_id INTEGER NOT NULL REFERENCES event(event_id),
    user_id INTEGER NOT NULL REFERENCES bettor(user_id),
    golfer_id INTEGER NOT NULL REFERENCES golfer(golfer_id),
    last_refresh_ts DATETIME,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,

    UNIQUE (event_id, user_id, golfer_id)
    );