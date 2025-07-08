CREATE TABLE eup_statistic_hx (
    hx_id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_espn_id int not null,
    golfer_espn_id int not null,
    eup_id int not null,
    grp int not null,
    rounds json not null,
    round_scores json not null,
    tee_times json not null,
    holes_completed_by_round json not null,
    line_scores json not null,
    total_score INTEGER not null,
    ins_ts datetime not null,
    hx_ts TEXT DEFAULT CURRENT_TIMESTAMP
);