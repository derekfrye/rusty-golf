insert into event (name, espn_id) values ('U.S. Open 2024', 401580355);

insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player1'), (select player_id from player where name = 'Cameron Smith');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player2'), (select player_id from player where name = 'Bryson DeChambeau');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player3'), (select player_id from player where name = 'Xander Schauffele');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player4'), (select player_id from player where name = 'Scottie Scheffler');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player5'), (select player_id from player where name = 'Brooks Koepka');


insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player1'), (select player_id from player where name = 'Sahith Theegala');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player2'), (select player_id from player where  name = 'Collin Morikawa');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player3'), (select player_id from player  where name = 'Max Homa');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player4'), (select player_id from player where name = 'Rory McIlroy');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player5'), (select player_id from player where name = 'Matt Fitzpatrick');


insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player1'), (select player_id from player where name = 'Russell Henley');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player2'), (select player_id from player where name = 'Jason Day');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player3'), (select player_id from player where name = 'Viktor Hovland');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player4'), (select player_id from player where name = 'Ludvig Åberg');
insert into event_user_player (event_id, user_id, player_id)select (select event_id from event where espn_id = 401580355), (select user_id from golfuser where name = 'Player5'), (select player_id from player where name = 'Justin Thomas');
