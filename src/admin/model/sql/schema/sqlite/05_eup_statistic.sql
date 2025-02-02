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
