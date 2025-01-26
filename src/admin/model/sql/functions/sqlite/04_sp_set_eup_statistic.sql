INSERT INTO eup_statistic (
    event_espn_id,
    eup_id,
    grp,
    rounds,
    round_scores,
    tee_times,
    holes_completed_by_round,
    line_scores
    )
VALUES (
    ?1,
    ?2,
    ?3,
    ?4,
    ?5,
    ?6,
    ?7,
    ?8
    ) ON CONFLICT(event_espn_id, eup_id) DO

UPDATE
SET grp = EXCLUDED.grp,
    rounds = EXCLUDED.rounds,
    round_scores = EXCLUDED.round_scores,
    tee_times = EXCLUDED.tee_times,
    holes_completed_by_round = EXCLUDED.holes_completed_by_round,
    line_scores = EXCLUDED.line_scores;