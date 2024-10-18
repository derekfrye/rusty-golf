CREATE TABLE -- drop table event cascade
    event (
    event_id BIGSERIAL NOT NULL PRIMARY KEY,
    espn_id BIGINT NOT NULL,
    name TEXT NOT NULL,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );
    --alter table event alter column name set data type text;
