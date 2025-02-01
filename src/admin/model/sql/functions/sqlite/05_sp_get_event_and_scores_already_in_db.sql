SELECT min(ins_ts) as ins_ts
FROM eup_statistic
WHERE event_espn_id = ?1;

        -- old query below, i guess i was thinking i would just ck if the event exists? Odd reasoning, over-complicated.
        -- SELECT CASE 
        --         -- has the event been configured
        --         WHEN EXISTS (
        --                 SELECT 1
        --                 FROM event
        --                 WHERE espn_id = ?1
        --                 )
        --             THEN CASE 
        --                     WHEN EXISTS (
        --                             -- is there at least one score logged in the db
        --                             SELECT 1
        --                             FROM eup_statistic
        --                             WHERE event_espn_id = (
        --                                     SELECT espn_id
        --                                     FROM event
        --                                     WHERE espn_id = ?1
        --                                     )
        --                             )
        --                         THEN 1
        --                     ELSE 0
        --                     END
        --         ELSE 0
        --         END;
