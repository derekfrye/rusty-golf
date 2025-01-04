SELECT grp,
    golfername,
    playername,
    eup_id,
    espn_id
FROM (
    SELECT ROW_NUMBER() OVER (
            PARTITION BY g.name ORDER BY eup.eup_id
            ) AS grp,
        g.name AS playername,
        p.name AS golfername,
        eup.eup_id,
        p.espn_id
    FROM player AS p
    JOIN event_user_player AS eup ON p.player_id = eup.player_id
    JOIN event AS e ON eup.event_id = e.event_id
    JOIN golfuser AS g ON g.user_id = eup.user_id
    WHERE e.espn_id = ?
    ) AS t
ORDER BY grp,
    eup_id;