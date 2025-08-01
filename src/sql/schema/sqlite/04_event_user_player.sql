CREATE TABLE IF NOT EXISTS event_user_player (
    -- drop table event_user_player cascade
    eup_id INTEGER NOT NULL PRIMARY KEY,
    event_id INTEGER NOT NULL REFERENCES event(event_id),
    user_id INTEGER NOT NULL REFERENCES bettor(user_id),
    golfer_id INTEGER NOT NULL REFERENCES golfer(golfer_id),
    last_refresh_ts DATETIME,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    score_view_step_factor REAL DEFAULT 3.0,

    UNIQUE (event_id, user_id, golfer_id)
    );



-- delete from event_user_player where event_id = 3
-- SELECT *
-- FROM event_user_player;
