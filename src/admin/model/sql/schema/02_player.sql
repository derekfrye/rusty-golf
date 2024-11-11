CREATE TABLE player (
    -- drop table player cascade
    player_id BIGSERIAL NOT NULL PRIMARY KEY,
    espn_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );



/*
SELECT espn_id,
    COUNT(*)
FROM player
GROUP BY espn_id
HAVING COUNT(*) > 1;
*/