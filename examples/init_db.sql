create database golf;
create user golfuser with password 'tempPasswordWillbeReplacedIn!AdminPanel';
grant connect on database golf to golfuser;
\c golf;
grant select, insert, update, delete on all tables in schema public to golfuser;
grant create on schema public to golfuser;
\q