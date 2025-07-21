SELECT e.name AS eventname, ins_ts, score_view_step_factor, refresh_from_espn
FROM event AS e
WHERE e.espn_id = ?1;
