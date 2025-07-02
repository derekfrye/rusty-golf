SELECT e.name AS eventname, ins_ts, score_view_step_factor
FROM event AS e
WHERE e.espn_id = ?1;
