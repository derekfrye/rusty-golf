--alter table player alter column name set data type text;
ALTER TABLE player ADD CONSTRAINT unq_name UNIQUE (name);