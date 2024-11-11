CREATE
    OR REPLACE FUNCTION sp_get_player_names (event_id INT)
RETURNS TABLE (
        grp bigint
        , playername TEXT
        , golfername TEXT
        , eup_id BIGINT
        , espn_id BIGINT
        ) AS $$

BEGIN
    RETURN QUERY

    SELECT 
        ROW_NUMBER() OVER (PARTITION BY g.name ORDER BY eup.eup_id) AS grp
        , g.name AS playername
        , p.name as golfername
        , eup.eup_id
        , p.espn_id
    FROM player AS p
    JOIN event_user_player AS eup
        ON p.player_id = eup.player_id
    JOIN event AS e
        ON eup.event_id = e.event_id
    JOIN golfuser AS g
        ON g.user_id = eup.user_id
    WHERE e.espn_id = $1;
END;$$

LANGUAGE plpgsql;
    /*

SELECT 'DROP FUNCTION ' || oid::regprocedure || ';'
FROM   pg_proc
WHERE  proname = 'sp_get_player_names'  -- name without schema-qualification
AND    pg_function_is_visible(oid);  -- restrict to current search_path

DROP FUNCTION sp_get_player_names(integer);
*/
