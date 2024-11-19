
SELECT 'event' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'event')
union
SELECT 'golfstatistic' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'golfstatistic')
union
SELECT 'player' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'player')
union
SELECT 'golf_user' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'golfuser')
union
SELECT 'event_user_player' as tbl,EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'event_user_player')
union
SELECT 'eup_statistic' as tbl, EXISTS (SELECT FROM information_schema.tables WHERE table_schema = 'public' AND table_name = 'eup_statistic');
