CREATE
    OR REPLACE FUNCTION sp_get_scores (eup_id bigINT)
RETURNS TABLE (
        round int
        , statistic_type text
        , intval int
        , timeval timestamp
        , last_refresh_ts timestamp
        ) AS $$

BEGIN
    RETURN QUERY

    SELECT 
        round
        , statistic_type
        , intval 
        , NULL
        last_refresh_ts
    FROM eup_statistic AS es
    -- eup_id is unique for an event, golfer, and better
    -- eup_id comes to the interface via sp_get_player_names
    WHERE es.eup_id = $1 and statistic_type = 'score'
    UNION
    SELECT 
        round
        , statistic_type
        , NULL 
        , timeval
        last_refresh_ts
    FROM eup_statistic AS es
    WHERE es.eup_id = $1 and statistic_type = 'teetime'
    UNION
    SELECT 
        round
        , statistic_type
        , intval 
        , NULL
        last_refresh_ts
    FROM eup_statistic AS es
    WHERE es.eup_id = $1 and statistic_type = 'holescompleted';
END;$$

LANGUAGE plpgsql;
    /*

SELECT 'DROP FUNCTION ' || oid::regprocedure || ';'
FROM   pg_proc
WHERE  proname = 'sp_get_scores'  -- name without schema-qualification
AND    pg_function_is_visible(oid);  -- restrict to current search_path

DROP FUNCTION sp_get_scores(bigint,bigint,integer);
*/
