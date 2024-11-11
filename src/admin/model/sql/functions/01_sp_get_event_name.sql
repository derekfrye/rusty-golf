CREATE
    OR REPLACE FUNCTION sp_get_event_name (event_id INT)
RETURNS TABLE (eventname TEXT) AS $$

BEGIN
    RETURN QUERY

    SELECT e.name AS eventname
    FROM event AS e
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
