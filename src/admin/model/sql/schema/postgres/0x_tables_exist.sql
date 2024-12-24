
SELECT 'event' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'event') as ex
union
SELECT 'golfstatistic' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'golfstatistic') as ex
union
SELECT 'player' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'player') as ex
union
SELECT 'golf_user' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'golfuser') as ex
union
SELECT 'event_user_player' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'event_user_player') as ex
union
SELECT 'eup_statistic' as tbl, EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'eup_statistic') as ex;
