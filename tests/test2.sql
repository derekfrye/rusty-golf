insert into event (name, espn_id) values ('U.S. Open 2024', 401580355);

insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player1'), (select golfer_id from golfer where name = 'Cameron Smith');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player2'), (select golfer_id from golfer where name = 'Bryson DeChambeau');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player3'), (select golfer_id from golfer where name = 'Xander Schauffele');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player4'), (select golfer_id from golfer where name = 'Scottie Scheffler');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player5'), (select golfer_id from golfer where name = 'Brooks Koepka');


insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player1'), (select golfer_id from golfer where name = 'Sahith Theegala');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player2'), (select golfer_id from golfer where  name = 'Collin Morikawa');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player3'), (select golfer_id from golfer  where name = 'Max Homa');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player4'), (select golfer_id from golfer where name = 'Rory McIlroy');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player5'), (select golfer_id from golfer where name = 'Matt Fitzpatrick');


insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player1'), (select golfer_id from golfer where name = 'Russell Henley');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player2'), (select golfer_id from golfer where name = 'Jason Day');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player3'), (select golfer_id from golfer where name = 'Viktor Hovland');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player4'), (select golfer_id from golfer where name = 'Ludvig Åberg');
insert into event_user_player (event_id, user_id, golfer_id)select (select event_id from event where espn_id = 401580355), (select user_id from bettor where name = 'Player5'), (select golfer_id from golfer where name = 'Justin Thomas');
