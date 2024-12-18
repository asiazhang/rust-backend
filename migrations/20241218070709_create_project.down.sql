-- Add migration script here
drop table if exists hm.projects;

drop schema if exists hm;

drop extension if exists pg_trgm;