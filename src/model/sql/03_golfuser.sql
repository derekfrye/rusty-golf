CREATE TABLE golfuser (
    user_id BIGSERIAL NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );
    --alter table golfuser alter column name set data type text;
