CREATE TABLE IF NOT EXISTS golfer (
    -- drop table player cascade
    golfer_id INTEGER NOT NULL PRIMARY KEY,
    espn_id integer NOT NULL UNIQUE,
    name TEXT NOT NULL,
    ins_ts DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
    );



/*
SELECT espn_id,
    COUNT(*)
FROM player
GROUP BY espn_id
HAVING COUNT(*) > 1;
*/