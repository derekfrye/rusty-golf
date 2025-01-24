SELECT 'event' AS tbl, EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'event') AS ex
UNION
SELECT 'player' AS tbl, EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'golfer') AS ex
UNION
SELECT 'golf_user' AS tbl, EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'bettor') AS ex
UNION
SELECT 'event_user_player' AS tbl, EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'event_user_player') AS ex
UNION
SELECT 'eup_statistic' AS tbl, EXISTS (SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'eup_statistic') AS ex;
