SELECT grp,
    golfername,
    bettorrname,
    eup_id,
    espn_id
FROM (
    SELECT ROW_NUMBER() OVER (
            PARTITION BY b.name ORDER BY eup.eup_id
            ) AS grp,
        b.name AS bettorname,
        g.name AS golfername,
        eup.eup_id,
        p.espn_id
    FROM golfer AS g
    JOIN event_user_player AS eup ON g.golfer_id = eup.player_id
    JOIN event AS e ON eup.event_id = e.event_id
    JOIN bettor AS b ON b.user_id = eup.user_id
    WHERE e.espn_id = ?1
    ) AS t
ORDER BY grp,
    eup_id;