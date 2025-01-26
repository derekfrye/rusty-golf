SELECT golfer_espn_id,
    es.eup_id,
    grp,
    rounds,
    round_scores,
    tee_times,
    holes_completed_by_round,
    line_scores,
    g.name AS golfername,
    b.name as bettorname,
    es.total_score
FROM eup_statistic AS es
JOIN golfer AS g ON es.golfer_espn_id = g.espn_id
join event_user_player as eup on es.eup_id = eup.eup_id
join bettor as b on b.user_id = eup.user_id
WHERE es.event_espn_id = ?1;
