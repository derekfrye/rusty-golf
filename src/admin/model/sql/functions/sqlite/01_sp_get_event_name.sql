SELECT e.name AS eventname, ins_ts
FROM event AS e
WHERE e.espn_id = ?;
