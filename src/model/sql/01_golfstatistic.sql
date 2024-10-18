CREATE TABLE golfstatistic (
    -- drop table golfstatistic
    stat_id BIGSERIAL NOT NULL PRIMARY KEY,
    statistic_type TEXT NOT NULL,
    intval INT,
    timeval TIMESTAMP,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );
    -- insert into golfstatistic (statistic_type)
