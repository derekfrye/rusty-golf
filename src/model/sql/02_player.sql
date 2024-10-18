CREATE TABLE player (
    -- drop table player cascade
    player_id BIGSERIAL NOT NULL PRIMARY KEY,
    espn_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );

--alter table player alter column name set data type text;
ALTER TABLE player ADD CONSTRAINT unq_name UNIQUE (name);

ALTER TABLE player ADD CONSTRAINT unq_espn_id UNIQUE (espn_id);

/*
SELECT espn_id,
    COUNT(*)
FROM player
GROUP BY espn_id
HAVING COUNT(*) > 1;
*/