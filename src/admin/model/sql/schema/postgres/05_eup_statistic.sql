CREATE TABLE IF NOT EXISTS eup_statistic (
    -- drop table eup_statistic
    eup_stat_id BIGSERIAL NOT NULL PRIMARY KEY,
    eup_id BIGINT NOT NULL REFERENCES event_user_player(eup_id),
    round INT NOT NULL,
    statistic_type TEXT NOT NULL,
    intval INT,
    timeval TIMESTAMP,
    last_refresh_ts TIMESTAMP,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );
