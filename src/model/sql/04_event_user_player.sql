CREATE TABLE event_user_player (
    -- drop table event_user_player cascade
    eup_id BIGSERIAL NOT NULL PRIMARY KEY,
    event_id BIGINT NOT NULL REFERENCES event(event_id),
    user_id BIGINT NOT NULL REFERENCES golfuser(user_id),
    player_id BIGINT NOT NULL REFERENCES player(player_id),
    last_refresh_ts TIMESTAMP,
    ins_ts TIMESTAMP NOT NULL DEFAULT now()
    );

ALTER TABLE event_user_player ADD CONSTRAINT unq_event_id_user_id_player_id UNIQUE (
    event_id,
    user_id,
    player_id
    );

-- delete from event_user_player where event_id = 3
-- SELECT *
-- FROM event_user_player;
