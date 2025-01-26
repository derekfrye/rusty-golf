SELECT CASE 
        -- has the event been configured
        WHEN EXISTS (
                SELECT 1
                FROM event
                WHERE espn_id = ?1
                )
            THEN CASE 
                    WHEN EXISTS (
                            -- is there at least one score logged in the db
                            SELECT 1
                            FROM eup_statistic
                            WHERE event_espn_id = (
                                    SELECT espn_id
                                    FROM event
                                    WHERE espn_id = ?1
                                    )
                            )
                        THEN 1
                    ELSE 0
                    END
        ELSE 0
        END;
