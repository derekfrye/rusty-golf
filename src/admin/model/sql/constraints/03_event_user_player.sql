ALTER TABLE event_user_player ADD CONSTRAINT unq_event_id_user_id_player_id UNIQUE (
    event_id,
    user_id,
    player_id
    );